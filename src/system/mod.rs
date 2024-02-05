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

pub use self::commands::*;
pub use self::deferred::*;
pub use self::exclusive_param::*;
pub use self::local::*;
pub use self::param::*;
pub use self::run::*;

/// Encapsulates a function that borrows data from a registry during execution.
pub trait System<TRegistry>: Send + Sync + 'static {
    /// The system's input.
    type In;

    /// The system's output.
    type Out;

    /// Returns the system's name.
    fn name(&self) -> Cow<'static, str>;

    /// The system parameters.
    fn param_kinds(&self) -> &[SystemParamKind];

    /// Runs the system with the given input and registry.
    unsafe fn run_unsafe(&mut self, input: Self::In, registry: &TRegistry) -> Self::Out;

    /// Runs the system with the given input and exclusive access to the registry.
    fn run(&mut self, input: Self::In, registry: &mut TRegistry) -> Self::Out {
        unsafe { self.run_unsafe(input, registry) }
    }

    /// Applies any [`Deferred`] system parmeters.
    fn apply_deferred(&mut self, registry: &mut TRegistry);

    /// Initialize the system state.
    fn initialize(&mut self, registry: &mut TRegistry);

    /// Returns whether this system has exclusive acces to the registry.
    fn is_exclusive(&self) -> bool;
}

/// [`System`] types that do not modify the registry when run.
pub unsafe trait ReadonlySystem<TRegistry>: System<TRegistry> {
    /// Runs this system with the given input and registry.
    fn run_readyonly(&mut self, input: Self::In, registry: &TRegistry) -> Self::Out {
        unsafe { self.run_unsafe(input, registry) }
    }
}

/// A convenience type alias for a boxed [`System`] trait object.
pub type BoxedSystem<TRegistry, In = (), Out = ()> = Box<dyn System<TRegistry, In = In, Out = Out>>;

/// Conversion trait to turn something into a [`System`].
pub trait IntoSystem<TRegistry, In, Out, Marker>: Sized {
    /// The type of [`System`] that this instance converts into.
    type System: System<TRegistry, In = In, Out = Out>;

    /// Turn this value into its corresponding [`System`].
    fn into_system(self) -> Self::System;
}

impl<T: System<TRegistry>, TRegistry> IntoSystem<TRegistry, T::In, T::Out, ()> for T {
    type System = T;

    fn into_system(self) -> Self::System {
        self
    }
}

/// Wrapper type to mark a [`SystemParam`] as an input.
pub struct In<In>(pub In);

impl<TRegistry, In, Out> std::fmt::Debug for dyn System<TRegistry, In = In, Out = Out>
where
    TRegistry: 'static,
    In: 'static,
    Out: 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name())
    }
}

impl<TRegistry, In, Out> std::fmt::Debug for dyn ReadonlySystem<TRegistry, In = In, Out = Out>
where
    TRegistry: 'static,
    In: 'static,
    Out: 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name())
    }
}
