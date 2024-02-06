use std::fmt;
use std::ops::{Deref, DerefMut};
use std::sync::Exclusive;

use crate::prelude::{FromWorld, World};
use crate::world::UnsafeWorldCell;

use super::{ReadonlySystemParam, SystemParam};

/// State local to a system.
#[derive(PartialEq, Eq, Hash)]
pub struct Local<'s, T>(pub(crate) &'s mut T);

/// Marker trait for types that can be used as system state.
pub trait LocalData: Send + Sync + 'static {}

impl<T: Send + Sync + 'static> LocalData for T {}

impl<T> SystemParam for Local<'_, T>
where
    T: LocalData + FromWorld,
{
    const SEND: bool = true;

    type Item<'w, 's> = Local<'s, T>;
    type State = Exclusive<T>;

    fn init_state(world: &mut World) -> Self::State {
        Exclusive::new(T::from_world(world))
    }

    unsafe fn borrow<'w, 's>(
        state: &'s mut Self::State,
        _: UnsafeWorldCell<'w>,
    ) -> Self::Item<'w, 's> {
        Local(state.get_mut())
    }
}

unsafe impl<T> ReadonlySystemParam for Local<'_, T> where T: LocalData + FromWorld {}

impl<'s, T> Deref for Local<'s, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<'s, T> DerefMut for Local<'s, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}

impl<T: fmt::Debug> fmt::Debug for Local<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<T: fmt::Display> fmt::Display for Local<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
