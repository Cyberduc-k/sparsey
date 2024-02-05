use std::ops::{Deref, DerefMut};
use std::sync::Exclusive;

use crate::util::TypeData;
use crate::FromRegistry;

use super::{ReadonlySystemParam, SystemParam, SystemParamKind};

/// Types that can be used with [`Deferred<T>`] in systems.
pub trait SystemBuffer: Send + 'static {
    /// The registry the mutations should be applied to.
    type Registry;

    /// Applies any deferred mutations to the given `registry`.
    fn apply(&mut self, registry: &mut Self::Registry);
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

impl<T, TRegistry> SystemParam<TRegistry> for Deferred<'_, T>
where
    T: SystemBuffer<Registry = TRegistry> + FromRegistry<TRegistry>,
{
    const KIND: SystemParamKind = SystemParamKind::State(TypeData::new::<Exclusive<T>>());

    type Item<'w, 's> = Deferred<'s, T> where TRegistry: 'w;
    type State = Exclusive<T>;

    fn init_state(registry: &mut TRegistry) -> Self::State {
        Exclusive::new(T::from_registry(registry))
    }

    fn borrow<'w, 's>(state: &'s mut Self::State, _: &'w TRegistry) -> Self::Item<'w, 's> {
        Deferred(state.get_mut())
    }

    fn apply(state: &mut Self::State, registry: &mut TRegistry) {
        state.get_mut().apply(registry);
    }
}

unsafe impl<T, TRegistry> ReadonlySystemParam<TRegistry> for Deferred<'_, T> where
    T: SystemBuffer<Registry = TRegistry> + FromRegistry<TRegistry>
{
}
