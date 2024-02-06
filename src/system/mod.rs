//! Hadles functions that borrow data from a registry during execution.

mod commands;
mod deferred;
mod exclusive_param;
mod exclusive_system;
mod function_system;
mod local;
mod param;
mod run;

use std::borrow::Cow;

use crate::world::{UnsafeWorldCell, World};

pub use self::commands::*;
pub use self::deferred::*;
pub use self::exclusive_param::*;
pub use self::exclusive_system::*;
pub use self::function_system::*;
pub use self::local::*;
pub use self::param::*;
pub use self::run::*;

/// Encapsulates a function that borrows data from a registry during execution.
pub trait System: Send + Sync + 'static {
    /// The system's input.
    type In;

    /// The system's output.
    type Out;

    /// Returns the system's name.
    fn name(&self) -> Cow<'static, str>;

    /// The system parameters.
    fn param_kinds(&self) -> &[SystemParamKind];

    /// Runs the system with the given input and [`World`].
    unsafe fn run_unsafe(&mut self, input: Self::In, world: UnsafeWorldCell) -> Self::Out;

    /// Runs the system with the given input and exclusive access to the [`World`].
    fn run(&mut self, input: Self::In, world: &mut World) -> Self::Out {
        unsafe { self.run_unsafe(input, world.as_unsafe_world_cell()) }
    }

    /// Applies any [`Deferred`] system parmeters.
    fn apply_deferred(&mut self, world: &mut World);

    /// Initialize the system state.
    fn initialize(&mut self, registry: &mut World);

    /// Returns whether this system has exclusive acces to the [`World`].
    fn is_exclusive(&self) -> bool;

    /// Returns whether this system must run on the main thread.
    fn is_thread_local(&self) -> bool;
}

/// [`System`] types that do not modify the registry when run.
pub unsafe trait ReadonlySystem: System {
    /// Runs this system with the given input and [`World`].
    fn run_readonly(&mut self, input: Self::In, world: &World) -> Self::Out {
        unsafe { self.run_unsafe(input, world.as_unsafe_world_cell()) }
    }
}

/// A convenience type alias for a boxed [`System`] trait object.
pub type BoxedSystem<In = (), Out = ()> = Box<dyn System<In = In, Out = Out>>;

/// Conversion trait to turn something into a [`System`].
pub trait IntoSystem<In, Out, Marker>: Sized {
    /// The type of [`System`] that this instance converts into.
    type System: System<In = In, Out = Out>;

    /// Turn this value into its corresponding [`System`].
    fn into_system(self) -> Self::System;
}

impl<T: System> IntoSystem<T::In, T::Out, ()> for T {
    type System = T;

    fn into_system(self) -> Self::System {
        self
    }
}

/// Wrapper type to mark a [`SystemParam`] as an input.
pub struct In<In>(pub In);

impl<In, Out> std::fmt::Debug for dyn System<In = In, Out = Out>
where
    In: 'static,
    Out: 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name())
    }
}

impl<In, Out> std::fmt::Debug for dyn ReadonlySystem<In = In, Out = Out>
where
    In: 'static,
    Out: 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name())
    }
}
