#![allow(clippy::needless_lifetimes)]

use std::sync::Exclusive;

use crate::FromRegistry;

use super::{Local, LocalData};

/// Trait implemented by types that can be borrowed by systems during execution.
pub trait ExclusiveSystemParam<TRegistry> {
    /// The system parameter generic over the lifetimes `'w` and `'s`.
    type Item<'s>;

    /// The state used by this parameter.
    type State: LocalData;

    /// Create the initial state from the given `registry`.
    fn init_state(registry: &mut TRegistry) -> Self::State;

    /// Borrows data from the given `registry`.
    #[must_use]
    fn borrow<'s>(state: &'s mut Self::State) -> Self::Item<'s>;
}

/// A set of multiple [`ExclusiveSystemParam`].
pub trait ExclusiveSystemParamSet<TRegistry> {
    /// The system parameter set generic over the lifetimes `'w` and `'s`.
    type Item<'s>;

    /// The state used by this parameter set.
    type State: LocalData;

    /// Create the initial state from the given `registry`.
    fn init_state(registry: &mut TRegistry) -> Self::State;

    /// Borrows data from the given `registry`.
    #[must_use]
    fn borrow<'s>(state: &'s mut Self::State) -> Self::Item<'s>;
}

impl<T, TRegistry> ExclusiveSystemParam<TRegistry> for Local<'_, T>
where
    T: LocalData + FromRegistry<TRegistry>,
{
    type Item<'s> = Local<'s, T>;
    type State = Exclusive<T>;

    fn init_state(registry: &mut TRegistry) -> Self::State {
        Exclusive::new(T::from_registry(registry))
    }

    fn borrow<'s>(state: &'s mut Self::State) -> Self::Item<'s> {
        Local(state.get_mut())
    }
}

macro_rules! impl_exclusive_system_param_set {
    ($(($Param:ident $n:tt)),*) => {
        impl<TRegistry, $($Param),*> ExclusiveSystemParamSet<TRegistry> for ($($Param,)*)
        where
            $($Param: ExclusiveSystemParam<TRegistry>),*
        {
            type Item<'s> = ($($Param::Item<'s>,)*);
            type State = ($($Param::State,)*);

            #[allow(clippy::unused_unit, unused_variables)]
            fn init_state(registry: &mut TRegistry) -> Self::State {
                ($($Param::init_state(registry),)*)
            }

            #[allow(clippy::unused_unit, unused_variables)]
            fn borrow<'s>(state: &'s mut Self::State) -> Self::Item<'s> {
                ($($Param::borrow(&mut state.$n),)*)
            }
        }
    };
}

impl_exclusive_system_param_set!();
impl_exclusive_system_param_set!((A 0));
impl_exclusive_system_param_set!((A 0), (B 1));
impl_exclusive_system_param_set!((A 0), (B 1), (C 2));
