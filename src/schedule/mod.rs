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
    sets: FxHashMap<Box<dyn ScheduleLabel>, ScheduleSet>,
    order: Vec<Box<dyn ScheduleLabel>>,
    max_threads: usize,
}

/// A set of [`ScheduleStep`]s.
#[derive(Default, Debug)]
pub struct ScheduleSet {
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

impl Default for Schedule {
    fn default() -> Self {
        let mut schedule = Self {
            sets: Default::default(),
            order: Default::default(),
            max_threads: usize::MAX,
        };

        schedule.add_label(First);
        schedule.add_label(PreUpdate);
        schedule.add_label(Update);
        schedule.add_label(PostUpdate);
        schedule.add_label(Last);
        schedule
    }
}

impl Schedule {
    /// Set the `max_threads` of this [`Schedule`].
    pub fn with_max_threads(mut self, max_threads: usize) -> Self {
        self.max_threads = max_threads;
        self
    }

    /// Consumes the schedule and returns the sets comprising it.
    pub fn into_sets(self) -> FxHashMap<Box<dyn ScheduleLabel>, ScheduleSet> {
        self.sets
    }

    /// Returns the schedule order.
    pub fn order(&self) -> &[Box<dyn ScheduleLabel>] {
        &self.order
    }

    /// Initializes all systems in the schedule.
    pub fn initialize(&mut self, world: &mut World) {
        for set in self.sets.values_mut() {
            set.initialize(world);
        }
    }

    /// Runs the systems. Calls [`maintain`](World::maintain) after each barrier and right before
    /// the function returns.
    pub fn run(&mut self, world: &mut World) {
        if self.max_threads == 0 {
            self.run_seq(world);
        } else {
            self.run_par(world);
        }
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
        for label in &self.order {
            if let Some(set) = self.sets.get_mut(label) {
                set.run_generic(world, &mut system_runner);
            }
        }
    }
}

impl ScheduleSet {
    /// Initializes all systems in the set.
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

impl Schedule {
    /// Adds a system to the schedule.
    pub fn add_system<M>(
        &mut self,
        label: impl ScheduleLabel,
        system: impl IntoConfig<M>,
    ) -> &mut Self {
        let max_threads = self.max_threads;
        let system = system.into_config();
        let set = self.entry(Box::new(label));
        if system.is_exclusive() {
            set.add_exclusive_system(system);
        } else if system.is_thread_local() {
            set.add_local_system(system);
        } else {
            set.add_system(system, max_threads);
        }
        self
    }

    /// Adds a barrier, preventing future systems from running parallel with the previously added
    /// systems.
    pub fn add_barrier(&mut self, label: impl ScheduleLabel) -> &mut Self {
        self.entry(Box::new(label))
            .steps
            .push(ScheduleStep::Barrier);
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
            if after_idx < idx {
                after_idx += 1;
            }
            let label = self.order.remove(idx);
            self.order.insert(after_idx, label);
        }
        self
    }

    fn entry(&mut self, label: Box<dyn ScheduleLabel>) -> &mut ScheduleSet {
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
}

impl ScheduleSet {
    fn add_system(&mut self, system: SystemConfig, max_threads: usize) {
        if matches!(self.steps.last(), Some(ScheduleStep::Systems(_))) {
            self.steps.push(ScheduleStep::Barrier);
        }

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

        let systems = self
            .steps
            .iter_mut()
            .rev()
            .map_while(|step| step_to_non_conflicting_systems(step, &system))
            .filter(|systems| systems.len() < max_threads)
            .last();

        match systems {
            Some(systems) => systems.push(system),
            None => self.steps.push(ScheduleStep::Systems(vec![system])),
        }
    }

    fn add_local_system(&mut self, system: SystemConfig) {
        if matches!(self.steps.last(), Some(ScheduleStep::Systems(_))) {
            self.steps.push(ScheduleStep::Barrier);
        }
        match self.steps.last_mut() {
            Some(ScheduleStep::LocalSystems(systems)) => systems.push(system),
            _ => self.steps.push(ScheduleStep::LocalSystems(vec![system])),
        }
    }

    fn add_exclusive_system(&mut self, system: SystemConfig) {
        if matches!(self.steps.last(), Some(ScheduleStep::Systems(_))) {
            self.steps.push(ScheduleStep::Barrier);
        }
        match self.steps.last_mut() {
            Some(ScheduleStep::ExclusiveSystems(systems)) => systems.push(system),
            _ => {
                self.steps
                    .push(ScheduleStep::ExclusiveSystems(vec![system]))
            }
        }
    }
}
