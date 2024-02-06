//! Schedules the execution of systems.

mod condition;
mod config;
mod label;

use rustc_hash::FxHashMap;

use crate::define_schedule_label;
use crate::prelude::World;
use crate::world::UnsafeWorldCell;

pub use self::config::*;
pub use self::label::*;

/// A collection of [`Schedule`]s.
#[derive(Default, Debug)]
pub struct Schedules {
    schedules: FxHashMap<InternedScheduleLabel, Schedule>,
}

/// Schedules systems to run sequentially or in parallel without data conflicts.
#[derive(Debug)]
pub struct Schedule {
    label: InternedScheduleLabel,
    steps: Vec<ScheduleStep>,
    max_threads: usize,
    is_initialized: bool,
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

define_schedule_label!(DefaultSchedule);

impl Default for Schedule {
    fn default() -> Self {
        Self::new(DefaultSchedule)
    }
}

impl Schedules {
    /// Inserts a labeled schedule.
    pub fn insert(&mut self, schedule: Schedule) -> Option<Schedule> {
        self.schedules.insert(schedule.label, schedule)
    }

    /// Removes the schedule corresponding to the `label`.
    pub fn remove(&mut self, label: impl ScheduleLabel) -> Option<Schedule> {
        self.schedules.remove(&label.intern())
    }

    /// Returns if a schedule with the provided `label` exists.
    pub fn contains(&self, label: impl ScheduleLabel) -> bool {
        self.schedules.contains_key(&label.intern())
    }

    /// Returns a reference to the schedule associated with `label`, if it exists.
    pub fn get(&self, label: impl ScheduleLabel) -> Option<&Schedule> {
        self.schedules.get(&label.intern())
    }

    /// Returns a mutable reference to the schedule associated with `label`, if it exists.
    pub fn get_mut(&mut self, label: impl ScheduleLabel) -> Option<&mut Schedule> {
        self.schedules.get_mut(&label.intern())
    }
}

impl Schedule {
    /// Constructs an empty [`Schedule`].
    pub fn new(label: impl ScheduleLabel) -> Self {
        Self {
            label: label.intern(),
            steps: Default::default(),
            max_threads: usize::MAX,
            is_initialized: false,
        }
    }

    /// Get the [`InternedScheduleLabel`] for this [`Schedule`].
    pub fn label(&self) -> InternedScheduleLabel {
        self.label
    }

    /// Sets the maximum number of threads. Only affects the systems added after this.
    pub fn set_max_threads(&mut self, max_threads: usize) -> &mut Self {
        self.max_threads = max_threads;
        self
    }

    /// Initializes all systems in the set.
    pub fn initialize(&mut self, world: &mut World) {
        if !self.is_initialized {
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
    }

    /// Runs the systems. Calls [`maintain`](World::maintain) after each barrier and right before
    /// the function returns.
    pub fn run(&mut self, world: &mut World) {
        self.initialize(world);
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
    pub fn add_system<M>(&mut self, system: impl IntoConfig<M>) -> &mut Self {
        let system = system.into_config();
        if matches!(self.steps.last(), Some(ScheduleStep::Systems(_))) {
            self.steps.push(ScheduleStep::Barrier);
        }

        if system.is_exclusive() {
            match self.steps.last_mut() {
                Some(ScheduleStep::ExclusiveSystems(systems)) => systems.push(system),
                _ => {
                    self.steps
                        .push(ScheduleStep::ExclusiveSystems(vec![system]))
                }
            }
        } else if system.is_thread_local() {
            match self.steps.last_mut() {
                Some(ScheduleStep::LocalSystems(systems)) => systems.push(system),
                _ => self.steps.push(ScheduleStep::LocalSystems(vec![system])),
            }
        } else {
            let systems = self
                .steps
                .iter_mut()
                .rev()
                .map_while(|step| step_to_non_conflicting_systems(step, &system))
                .filter(|systems| systems.len() < self.max_threads)
                .last();

            match systems {
                Some(systems) => systems.push(system),
                None => self.steps.push(ScheduleStep::Systems(vec![system])),
            }
        }

        self.is_initialized = false;
        self
    }

    /// Adds a barrier, preventing future systems from running parallel with the previously added
    /// systems.
    pub fn add_barrier(&mut self) -> &mut Self {
        self.steps.push(ScheduleStep::Barrier);
        self
    }
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
