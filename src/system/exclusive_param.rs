#![allow(clippy::needless_lifetimes)]

use std::sync::Exclusive;

use crate::prelude::{FromWorld, World};

use super::{Local, LocalData};

/// Trait implemented by types that can be borrowed by systems during execution.
pub trait ExclusiveSystemParam {
    /// The system parameter generic over the lifetimes `'w` and `'s`.
    type Item<'s>: ExclusiveSystemParam<State = Self::State>;

    /// The state used by this parameter.
    type State: LocalData;

    /// Create the initial state from the [`World`].
    fn init_state(world: &mut World) -> Self::State;

    /// Borrows data from the given `registry`.
    #[must_use]
    fn borrow<'s>(state: &'s mut Self::State) -> Self::Item<'s>;
}

impl<T> ExclusiveSystemParam for Local<'_, T>
where
    T: LocalData + FromWorld,
{
    type Item<'s> = Local<'s, T>;
    type State = Exclusive<T>;

    fn init_state(world: &mut World) -> Self::State {
        Exclusive::new(T::from_world(world))
    }

    fn borrow<'s>(state: &'s mut Self::State) -> Self::Item<'s> {
        Local(state.get_mut())
    }
}

macro_rules! impl_exclusive_system_param_set {
    ($(($Param:ident $n:tt)),*) => {
        impl<$($Param),*> ExclusiveSystemParam for ($($Param,)*)
        where
            $($Param: ExclusiveSystemParam),*
        {
            type Item<'s> = ($($Param::Item<'s>,)*);
            type State = ($($Param::State,)*);

            #[allow(clippy::unused_unit, unused_variables)]
            fn init_state(world: &mut World) -> Self::State {
                ($($Param::init_state(world),)*)
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
