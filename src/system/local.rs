use std::fmt;
use std::ops::{Deref, DerefMut};
use std::sync::Exclusive;

use crate::util::TypeData;
use crate::FromRegistry;

use super::{ReadonlySystemParam, SystemParam, SystemParamKind};

/// State local to a system.
#[derive(PartialEq, Eq, Hash)]
pub struct Local<'s, T>(pub(crate) &'s mut T);

/// Marker trait for types that can be used as system state.
pub trait LocalData: Send + Sync + 'static {}

impl<T: Send + Sync + 'static> LocalData for T {}

impl<T, TRegistry> SystemParam<TRegistry> for Local<'_, T>
where
    T: LocalData + FromRegistry<TRegistry>,
{
    const KIND: SystemParamKind = SystemParamKind::State(TypeData::new::<T>());

    type Item<'w, 's> = Local<'s, T> where TRegistry: 'w;
    type State = Exclusive<T>;

    fn init_state(registry: &mut TRegistry) -> Self::State {
        Exclusive::new(T::from_registry(registry))
    }

    fn borrow<'w, 's>(state: &'s mut Self::State, _: &'w TRegistry) -> Self::Item<'w, 's> {
        Local(state.get_mut())
    }
}

unsafe impl<T, TRegistry> ReadonlySystemParam<TRegistry> for Local<'_, T> where
    T: LocalData + FromRegistry<TRegistry>
{
}

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