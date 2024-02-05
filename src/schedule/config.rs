use std::ops::{Deref, DerefMut};

use crate::prelude::{IntoSystem, System};
use crate::system::BoxedSystem;

use super::condition::{BoxedCondition, Condition};

/// Stores configuration for a single system.
pub struct SystemConfig<TRegistry> {
    system: BoxedSystem<TRegistry>,
    conditions: Vec<BoxedCondition<TRegistry>>,
}

/// Types that can convert into a [`SystemConfig`].
pub trait IntoConfig<TRegistry: 'static, Marker>: Sized {
    /// Convert into a [`SystemConfig`].
    fn into_config(self) -> SystemConfig<TRegistry>;

    /// Run the system only if the [`Condition`] is `true`.
    fn run_if<M>(self, condition: impl Condition<TRegistry, M>) -> SystemConfig<TRegistry> {
        self.into_config().run_if(condition)
    }
}

impl<T, TRegistry: 'static, Marker> IntoConfig<TRegistry, Marker> for T
where
    T: IntoSystem<TRegistry, (), (), Marker>,
{
    fn into_config(self) -> SystemConfig<TRegistry> {
        SystemConfig {
            system: Box::new(self.into_system()),
            conditions: vec![],
        }
    }
}

impl<TRegistry> Deref for SystemConfig<TRegistry> {
    type Target = dyn System<TRegistry, In = (), Out = ()>;

    fn deref(&self) -> &Self::Target {
        &*self.system
    }
}

impl<TRegistry> DerefMut for SystemConfig<TRegistry> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.system
    }
}

impl<TRegistry: 'static> SystemConfig<TRegistry> {
    /// Run the system only if the [`Condition`] is `true`.
    pub fn run_if<M>(mut self, condition: impl Condition<TRegistry, M>) -> Self {
        self.conditions.push(Box::new(condition.into_system()));
        self
    }

    /// Initialize the system state.
    pub fn initialize(&mut self, registry: &mut TRegistry) {
        self.system.initialize(registry);
        for condition in &mut self.conditions {
            condition.initialize(registry);
        }
    }

    /// Runs the system with the given registry.
    pub unsafe fn run_unsafe(&mut self, registry: &TRegistry) -> bool {
        if self.should_run(registry) {
            self.system.run_unsafe((), registry);
            true
        } else {
            false
        }
    }

    /// Runs the system with exclusive access to the registry.
    pub fn run(&mut self, registry: &mut TRegistry) -> bool {
        if self.should_run(registry) {
            self.system.run((), registry);
            true
        } else {
            false
        }
    }

    /// Returns `true` if all conditions are `true`.
    pub fn should_run(&mut self, registry: &TRegistry) -> bool {
        self.conditions
            .iter_mut()
            .all(|c| c.run_readyonly((), registry))
    }
}

impl<TRegistry: 'static> std::fmt::Debug for SystemConfig<TRegistry> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SystemConfig")
            .field("system", &self.system)
            .field("conditions", &self.conditions)
            .finish()
    }
}
