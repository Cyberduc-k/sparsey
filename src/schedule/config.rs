use std::ops::{Deref, DerefMut};

use crate::prelude::{IntoSystem, System, World};
use crate::system::BoxedSystem;
use crate::world::UnsafeWorldCell;

use super::condition::{BoxedCondition, Condition};

/// Stores configuration for a single system.
#[derive(Debug)]
pub struct SystemConfig {
    system: BoxedSystem,
    conditions: Vec<BoxedCondition>,
    is_initialized: bool,
}

/// Types that can convert into a [`SystemConfig`].
pub trait IntoConfig<Marker>: Sized {
    /// Convert into a [`SystemConfig`].
    fn into_config(self) -> SystemConfig;

    /// Run the system only if the [`Condition`] is `true`.
    fn run_if<M>(self, condition: impl Condition<M>) -> SystemConfig {
        self.into_config().run_if(condition)
    }
}

#[doc(hidden)]
pub struct IsSystemConfig;

impl IntoConfig<IsSystemConfig> for SystemConfig {
    #[inline]
    fn into_config(self) -> SystemConfig {
        self
    }
}

impl<T, Marker> IntoConfig<Marker> for T
where
    T: IntoSystem<(), (), Marker>,
{
    fn into_config(self) -> SystemConfig {
        SystemConfig {
            system: Box::new(self.into_system()),
            conditions: vec![],
            is_initialized: false,
        }
    }
}

impl Deref for SystemConfig {
    type Target = dyn System<In = (), Out = ()>;

    fn deref(&self) -> &Self::Target {
        &*self.system
    }
}

impl DerefMut for SystemConfig {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.system
    }
}

impl SystemConfig {
    /// Run the system only if the [`Condition`] is `true`.
    pub fn run_if<M>(mut self, condition: impl Condition<M>) -> Self {
        self.conditions.push(Box::new(condition.into_system()));
        self
    }

    /// Initialize the system state.
    pub fn initialize(&mut self, world: &mut World) {
        if !self.is_initialized {
            self.system.initialize(world);
            for condition in &mut self.conditions {
                condition.initialize(world);
            }
            self.is_initialized = true;
        }
    }

    /// Runs the system with the given [`World`].
    pub unsafe fn run_unsafe(&mut self, world: UnsafeWorldCell) -> bool {
        if self.should_run(world.world()) {
            self.system.run_unsafe((), world);
            true
        } else {
            false
        }
    }

    /// Runs the system with exclusive access to the [`World`].
    pub fn run(&mut self, world: &mut World) -> bool {
        if self.should_run(world) {
            self.system.run((), world);
            true
        } else {
            false
        }
    }

    /// Returns `true` if all conditions are `true`.
    pub fn should_run(&mut self, world: &World) -> bool {
        self.conditions
            .iter_mut()
            .all(|c| c.run_readonly((), world))
    }
}
