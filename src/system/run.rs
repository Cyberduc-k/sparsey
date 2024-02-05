use crate::entity::EntityStorage;
use crate::resource::ResourceStorage;
use crate::World;

use super::{IntoSystem, System};

/// Trait used to run a system immediately on a registry.
pub trait Run: Sized {
    /// Runs a system and applies its deferred parameters.
    fn run<T: IntoSystem<Self, (), Out, Marker>, Out, Marker>(&mut self, system: T) -> Out {
        self.run_with((), system)
    }

    /// Runs a system with given input and applies its deferred parameters.
    fn run_with<T: IntoSystem<Self, In, Out, Marker>, In, Out, Marker>(
        &mut self,
        input: In,
        system: T,
    ) -> Out;
}

impl Run for World {
    fn run_with<T: IntoSystem<Self, In, Out, Marker>, In, Out, Marker>(
        &mut self,
        input: In,
        system: T,
    ) -> Out {
        let mut system: T::System = system.into_system();
        system.initialize(self);
        let out = system.run(input, self);
        system.apply_deferred(self);
        out
    }
}

impl Run for EntityStorage {
    fn run_with<T: IntoSystem<Self, In, Out, Marker>, In, Out, Marker>(
        &mut self,
        input: In,
        system: T,
    ) -> Out {
        let mut system: T::System = system.into_system();
        system.initialize(self);
        let out = system.run(input, self);
        system.apply_deferred(self);
        out
    }
}

impl Run for ResourceStorage {
    fn run_with<T: IntoSystem<Self, In, Out, Marker>, In, Out, Marker>(
        &mut self,
        input: In,
        system: T,
    ) -> Out {
        let mut system: T::System = system.into_system();
        system.initialize(self);
        let out = system.run(input, self);
        system.apply_deferred(self);
        out
    }
}
