//! Schedules the execution of systems.

mod condition;
mod config;
mod label;

use rustc_hash::FxHashMap;

use crate::prelude::World;
use crate::world::UnsafeWorldCell;

pub use self::config::*;
pub use self::label::*;

/// Schedules systems to run sequentially or in parallel without data conflicts.
#[derive(Debug)]
pub struct Schedule {
    steps: Vec<ScheduleStep>,
}

/// Steps that can be run by a [`Schedule`].
#[derive(Debug)]
pub enum ScheduleStep {
    /// Runs the systems in parallel.
    Systems(Vec<SystemConfig>),
    /// Runs the systems sequentially on the main thread.
    LocalSystems(Vec<SystemConfig>),
    /// Runs the systems sequentially.
    ExclusiveSystems(Vec<SystemConfig>),
    /// Prevents future systems from running in parallel with previous ones.
    Barrier,
}

/// Enables creating a [`Schedule`] using the builder pattern.
pub struct ScheduleBuilder {
    sets: FxHashMap<Box<dyn ScheduleLabel>, Vec<SimpleScheduleStep>>,
    order: Vec<Box<dyn ScheduleLabel>>,
}

enum SimpleScheduleStep {
    System(SystemConfig),
    LocalSystem(SystemConfig),
    ExclusiveSystem(SystemConfig),
    Barrier,
}

impl Schedule {
    /// Enables creating a schedule using the builder pattern.
    pub fn builder() -> ScheduleBuilder {
        Default::default()
    }

    /// Consumes the schedule and returns the steps comprising it.
    pub fn into_steps(self) -> Vec<ScheduleStep> {
        self.steps
    }

    /// Initializes all systems in the schedule.
    pub fn initialize(&mut self, world: &mut World) {
        for step in &mut self.steps {
            match step {
                ScheduleStep::Systems(systems) => {
                    for system in systems {
                        system.initialize(world);
                    }
                }
                ScheduleStep::LocalSystems(systems) => {
                    for system in systems {
                        system.initialize(world);
                    }
                }
                ScheduleStep::ExclusiveSystems(systems) => {
                    for system in systems {
                        system.initialize(world);
                    }
                }
                ScheduleStep::Barrier => {}
            }
        }
    }

    /// Runs the systems. Calls [`maintain`](World::maintain) after each barrier and right before
    /// the function returns.
    pub fn run(&mut self, world: &mut World) {
        self.run_seq(world);
    }

    /// Runs the systems sequentially.
    pub fn run_seq(&mut self, world: &mut World) {
        self.run_generic(world, |systems, world| {
            for system in systems {
                unsafe { system.run_unsafe(world) };
            }
        });
    }

    /// Runs the systems in parallel.
    pub fn run_par(&mut self, world: &mut World) {
        use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};
        self.run_generic(world, |systems, world| {
            if systems.len() > 1 {
                systems.par_iter_mut().for_each(|system| {
                    unsafe {
                        system.run_unsafe(world);
                    }
                });
            } else {
                unsafe { systems[0].run_unsafe(world) };
            }
        });
    }

    fn run_generic(
        &mut self,
        world: &mut World,
        mut system_runner: impl FnMut(&mut [SystemConfig], UnsafeWorldCell) + Send,
    ) {
        for step in &mut self.steps {
            match step {
                ScheduleStep::Systems(systems) => {
                    system_runner(systems, world.as_unsafe_world_cell());
                    for system in systems {
                        system.apply_deferred(world);
                    }
                }
                ScheduleStep::LocalSystems(systems) => {
                    for system in systems {
                        world.maintain();
                        system.run(world);
                    }
                }
                ScheduleStep::ExclusiveSystems(systems) => {
                    for system in systems {
                        world.maintain();
                        system.run(world);
                    }
                }
                ScheduleStep::Barrier => world.maintain(),
            }
        }

        world.maintain();
    }
}

impl Default for ScheduleBuilder {
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

impl ScheduleBuilder {
    /// Adds a system to the schedule.
    pub fn add_system<M>(
        &mut self,
        label: impl ScheduleLabel,
        system: impl IntoConfig<M>,
    ) -> &mut Self {
        let system = system.into_config();
        let step = match (system.is_exclusive(), system.is_thread_local()) {
            (true, _) => SimpleScheduleStep::ExclusiveSystem(system),
            (_, true) => SimpleScheduleStep::LocalSystem(system),
            (_, _) => SimpleScheduleStep::System(system),
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

    fn entry(&mut self, label: Box<dyn ScheduleLabel>) -> &mut Vec<SimpleScheduleStep> {
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
    pub fn build(&mut self) -> Schedule {
        self.build_with_max_threads(usize::MAX)
    }

    /// Builds the schedule allowing at most `max_threads` systems to run in parallel.
    pub fn build_with_max_threads(&mut self, max_threads: usize) -> Schedule {
        let mut steps = Vec::<ScheduleStep>::new();

        fn step_to_non_conflicting_systems<'a>(
            step: &'a mut ScheduleStep,
            system: &SystemConfig,
        ) -> Option<&'a mut Vec<SystemConfig>> {
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
                        SimpleScheduleStep::LocalSystem(system) => {
                            match steps.last_mut() {
                                Some(ScheduleStep::LocalSystems(systems)) => systems.push(system),
                                _ => steps.push(ScheduleStep::LocalSystems(vec![system])),
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
