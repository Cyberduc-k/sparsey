use std::ops::{Deref, DerefMut};
use std::sync::Exclusive;

use crate::prelude::{FromWorld, World};
use crate::world::UnsafeWorldCell;

use super::{ReadonlySystemParam, SystemParam};

/// Types that can be used with [`Deferred<T>`] in systems.
pub trait SystemBuffer: Send + 'static {
    /// Applies any deferred mutations to the [`World`].
    fn apply(&mut self, world: &mut World);
}

/// A [`SystemParam`] that stores a buffer to defer world mutations.
pub struct Deferred<'a, T: SystemBuffer>(&'a mut T);

impl<T: SystemBuffer> Deferred<'_, T> {
    /// Returns a [`Deferred<T>`] with a smaller lifetime.
    pub fn reborrow(&mut self) -> Deferred<T> {
        Deferred(self.0)
    }
}

impl<'a, T: SystemBuffer> Deref for Deferred<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<'a, T: SystemBuffer> DerefMut for Deferred<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}

impl<T> SystemParam for Deferred<'_, T>
where
    T: SystemBuffer + FromWorld,
{
    const SEND: bool = true;

    type Item<'w, 's> = Deferred<'s, T>;
    type State = Exclusive<T>;

    fn init_state(world: &mut World) -> Self::State {
        Exclusive::new(T::from_world(world))
    }

    fn apply(state: &mut Self::State, world: &mut World) {
        state.get_mut().apply(world);
    }

    unsafe fn borrow<'w, 's>(
        state: &'s mut Self::State,
        _: UnsafeWorldCell<'w>,
    ) -> Self::Item<'w, 's> {
        Deferred(state.get_mut())
    }
}

unsafe impl<T> ReadonlySystemParam for Deferred<'_, T> where T: SystemBuffer + FromWorld {}
