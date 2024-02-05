use atomic_refcell::{AtomicRef, AtomicRefMut};
use std::fmt;
use std::ops::{Deref, DerefMut};

/// Shared borrow over a resource of type `T`.
pub struct Res<'a, T>(pub(crate) AtomicRef<'a, T>);

/// Exclusive borrow over a resource of type `T`.
pub struct ResMut<'a, T>(pub(crate) AtomicRefMut<'a, T>);

/// Shared borrower over a non-[`Send`] resource of type `T`.
pub struct NonSend<'a, T>(pub(crate) AtomicRef<'a, T>);

/// Exclusive borrower over a non-[`Send`] resource of type `T`.
pub struct NonSendMut<'a, T>(pub(crate) AtomicRefMut<'a, T>);

impl<T> DerefMut for ResMut<'_, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> DerefMut for NonSendMut<'_, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

macro_rules! impl_res_common {
    ($Res:ident) => {
        impl<T> Deref for $Res<'_, T> {
            type Target = T;

            #[must_use]
            fn deref(&self) -> &T {
                &self.0
            }
        }

        impl<T> fmt::Debug for $Res<'_, T>
        where
            T: fmt::Debug,
        {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }

        impl<T> fmt::Display for $Res<'_, T>
        where
            T: fmt::Display,
        {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

impl_res_common!(Res);
impl_res_common!(ResMut);
impl_res_common!(NonSend);
impl_res_common!(NonSendMut);
