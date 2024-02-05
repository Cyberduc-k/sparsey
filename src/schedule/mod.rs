//! Schedules the execution of systems.

mod condition;
mod config;
mod label;

use rustc_hash::FxHashMap;

pub use self::config::*;
pub use self::label::*;

use crate::Registry;

/// Schedules systems to run sequentially or in parallel without data conflicts.
pub struct Schedule<TRegistry> {
    steps: Vec<ScheduleStep<TRegistry>>,
}

/// Steps that can be run by a [`Schedule`].
pub enum ScheduleStep<TRegistry> {
    /// Runs the systems in parallel.
    Systems(Vec<SystemConfig<TRegistry>>),
    /// Runs the systems sequentially.
    ExclusiveSystems(Vec<SystemConfig<TRegistry>>),
    /// Prevents future systems from running in parallel with previous ones.
    Barrier,
}

/// Enables creating a [`Schedule`] using the builder pattern.
pub struct ScheduleBuilder<TRegistry> {
    sets: FxHashMap<Box<dyn ScheduleLabel>, Vec<SimpleScheduleStep<TRegistry>>>,
    order: Vec<Box<dyn ScheduleLabel>>,
}

enum SimpleScheduleStep<TRegistry> {
    System(SystemConfig<TRegistry>),
    ExclusiveSystem(SystemConfig<TRegistry>),
    Barrier,
}

impl<TRegistry: 'static> Schedule<TRegistry> {
    /// Enables creating a schedule using the builder pattern.
    pub fn builder() -> ScheduleBuilder<TRegistry> {
        Default::default()
    }

    /// Consumes the schedule and returns the steps comprising it.
    pub fn into_steps(self) -> Vec<ScheduleStep<TRegistry>> {
        self.steps
    }

    /// Initializes all systems in the schedule.
    pub fn initialize(&mut self, registry: &mut TRegistry) {
        for step in &mut self.steps {
            match step {
                ScheduleStep::Systems(systems) => {
                    for system in systems {
                        system.initialize(registry);
                    }
                }
                ScheduleStep::ExclusiveSystems(systems) => {
                    for system in systems {
                        system.initialize(registry);
                    }
                }
                ScheduleStep::Barrier => {}
            }
        }
    }
}

impl<TRegistry: Registry + 'static> Schedule<TRegistry> {
    /// Runs the systems. Calls `Registry::maintain` after each barrier and right before the
    /// function returns.
    pub fn run(&mut self, registry: &mut TRegistry) {
        self.run_seq(registry);
    }

    /// Runs the systems sequentially.
    pub fn run_seq(&mut self, registry: &mut TRegistry) {
        self.run_generic(registry, |systems, registry| {
            for system in systems {
                unsafe { system.run_unsafe(registry) };
            }
        });
    }

    fn run_generic(
        &mut self,
        registry: &mut TRegistry,
        mut system_runner: impl FnMut(&mut [SystemConfig<TRegistry>], &TRegistry) + Send,
    ) {
        for step in &mut self.steps {
            match step {
                ScheduleStep::Systems(systems) => {
                    system_runner(systems, registry);
                    for system in systems {
                        system.apply_deferred(registry);
                    }
                }
                ScheduleStep::ExclusiveSystems(systems) => {
                    for system in systems {
                        registry.maintain();
                        system.run(registry);
                    }
                }
                ScheduleStep::Barrier => registry.maintain(),
            }
        }

        registry.maintain();
    }
}

impl<TRegistry: 'static> Default for ScheduleBuilder<TRegistry> {
    fn default() -> Self {
        let mut builder = Self {
            sets: Default::default(),
            order: Default::default(),
        };

        builder.add_label(First);
        builder.add_label(PreUpdate);
        builder.add_label(Update);
        builder.add_label(PostUpdate);
        builder.add_label(Last);
        builder
    }
}

impl<TRegistry: 'static> ScheduleBuilder<TRegistry> {
    /// Adds a system to the schedule.
    pub fn add_system<M>(
        &mut self,
        label: impl ScheduleLabel,
        system: impl IntoConfig<TRegistry, M>,
    ) -> &mut Self {
        let system = system.into_config();
        let step = match system.is_exclusive() {
            false => SimpleScheduleStep::System(system),
            true => SimpleScheduleStep::ExclusiveSystem(system),
        };
        self.entry(Box::new(label)).push(step);
        self
    }

    /// Adds a barrier, preventing future systems from running parallel with the previously added
    /// systems.
    pub fn add_barrier(&mut self, label: impl ScheduleLabel) -> &mut Self {
        self.entry(Box::new(label))
            .push(SimpleScheduleStep::Barrier);
        self
    }

    /// Add a new label to identify a set of systems.
    pub fn add_label(&mut self, label: impl ScheduleLabel) -> &mut Self {
        self.label_entry(label);
        self
    }

    /// Add a new label before `before` to identify a set of systems.
    pub fn add_label_before(
        &mut self,
        label: impl ScheduleLabel,
        before: impl ScheduleLabel,
    ) -> &mut Self {
        let idx = self.label_entry(label);
        if let Some(mut before_idx) = self.label_idx(&before) {
            if before_idx > idx {
                before_idx -= 1;
            }
            let label = self.order.remove(idx);
            self.order.insert(before_idx, label);
        }
        self
    }

    /// Add a new label after `after` to identify a set of systems.
    pub fn add_label_after(
        &mut self,
        label: impl ScheduleLabel,
        after: impl ScheduleLabel,
    ) -> &mut Self {
        let idx = self.label_entry(label);
        if let Some(mut after_idx) = self.label_idx(&after) {
            if after_idx > idx {
                after_idx += 1;
            }
            let label = self.order.remove(idx);
            self.order.insert(after_idx, label);
        }
        self
    }

    fn entry(&mut self, label: Box<dyn ScheduleLabel>) -> &mut Vec<SimpleScheduleStep<TRegistry>> {
        self.sets.entry(label.dyn_clone()).or_insert_with(|| {
            if !self.order.contains(&label) {
                self.order.push(label);
            }
            Default::default()
        })
    }

    fn label_entry(&mut self, label: impl ScheduleLabel) -> usize {
        match self.label_idx(&label) {
            Some(idx) => idx,
            None => {
                self.order.push(Box::new(label));
                self.order.len() - 1
            }
        }
    }

    fn label_idx(&self, label: &dyn ScheduleLabel) -> Option<usize> {
        self.order.iter().position(|id| **id == *label)
    }

    // /// Appends the steps from the given `ScheduleBuilder` to the current schedule.
    // pub fn append(&mut self, other: &mut ScheduleBuilder<TRegistry>) -> &mut Self {
    //     for (label, mut set) in other.sets.drain() {
    //         self.entry(label).append(&mut set);
    //     }
    //     self
    // }

    /// Builds the schedule.
    pub fn build(&mut self) -> Schedule<TRegistry> {
        self.build_with_max_threads(usize::MAX)
    }

    /// Builds the schedule allowing at most `max_threads` systems to run in parallel.
    pub fn build_with_max_threads(&mut self, max_threads: usize) -> Schedule<TRegistry> {
        let mut steps = Vec::<ScheduleStep<TRegistry>>::new();

        fn step_to_non_conflicting_systems<'a, TRegistry: 'static>(
            step: &'a mut ScheduleStep<TRegistry>,
            system: &SystemConfig<TRegistry>,
        ) -> Option<&'a mut Vec<SystemConfig<TRegistry>>> {
            match step {
                ScheduleStep::Systems(systems) => {
                    let system_conflict = systems
                        .iter()
                        .flat_map(|s| s.param_kinds())
                        .any(|p1| system.param_kinds().iter().any(|p2| p1.conflicts_with(*p2)));

                    if system_conflict {
                        None
                    } else {
                        Some(systems)
                    }
                }
                _ => None,
            }
        }

        for label in &self.order {
            if let Some(set) = self.sets.get_mut(label) {
                if matches!(steps.last(), Some(ScheduleStep::Systems(_))) {
                    steps.push(ScheduleStep::Barrier);
                }

                for step in set.drain(..) {
                    match step {
                        SimpleScheduleStep::System(system) => {
                            let systems = steps
                                .iter_mut()
                                .rev()
                                .map_while(|step| step_to_non_conflicting_systems(step, &system))
                                .filter(|systems| systems.len() < max_threads)
                                .last();

                            match systems {
                                Some(systems) => systems.push(system),
                                None => steps.push(ScheduleStep::Systems(vec![system])),
                            }
                        }
                        SimpleScheduleStep::ExclusiveSystem(system) => {
                            match steps.last_mut() {
                                Some(ScheduleStep::ExclusiveSystems(systems)) => {
                                    systems.push(system)
                                }
                                _ => steps.push(ScheduleStep::ExclusiveSystems(vec![system])),
                            }
                        }
                        SimpleScheduleStep::Barrier => {
                            if matches!(steps.last(), Some(ScheduleStep::Systems(_))) {
                                steps.push(ScheduleStep::Barrier);
                            }
                        }
                    }
                }
            }
        }

        Schedule { steps }
    }
}

impl<TRegistry: 'static> std::fmt::Debug for Schedule<TRegistry> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Schedule")
            .field("steps", &self.steps)
            .finish()
    }
}

impl<TRegistry: 'static> std::fmt::Debug for ScheduleStep<TRegistry> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Systems(arg0) => f.debug_tuple("Systems").field(arg0).finish(),
            Self::ExclusiveSystems(arg0) => f.debug_tuple("ExclusiveSystems").field(arg0).finish(),
            Self::Barrier => write!(f, "Barrier"),
        }
    }
}
