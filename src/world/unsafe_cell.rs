use std::cell::UnsafeCell;
use std::marker::PhantomData;

use crate::prelude::{EntityStorage, NonSendResourceStorage, ResourceStorage};

use super::World;

/// Variant of the [`World`] where resource and component access take `&self`, and the
/// responsibility to avoid aliasing violations are given to the caller instead of being checked at
/// compile-time by rust's borrow checker.
#[derive(Clone, Copy)]
pub struct UnsafeWorldCell<'w>(
    *const World,
    PhantomData<(&'w World, &'w UnsafeCell<World>)>,
);

unsafe impl Send for UnsafeWorldCell<'_> {}
unsafe impl Sync for UnsafeWorldCell<'_> {}

impl<'w> UnsafeWorldCell<'w> {
    #[inline]
    pub(crate) fn new(world: &'w World) -> Self {
        Self(world as *const World, PhantomData)
    }

    /// Gets a reference to the [`&World`](World) this [`UnsafeWorldCell`] belongs to. This can be
    /// used for arbitrary shared/readonly access.
    #[inline]
    pub unsafe fn world(self) -> &'w World {
        unsafe { self.unsafe_world() }
    }

    #[inline]
    unsafe fn unsafe_world(self) -> &'w World {
        &*self.0
    }

    /// Retrieves this world's [`EntityStorage`].
    #[inline]
    pub unsafe fn entities(self) -> &'w EntityStorage {
        &self.unsafe_world().entities
    }

    /// Retrieves this world's [`ResourceStorage`].
    #[inline]
    pub unsafe fn resources(self) -> &'w ResourceStorage {
        &self.unsafe_world().resources
    }

    /// Retrieves this world's [`NonSendResourceStorage`].
    #[inline]
    pub unsafe fn non_send_resources(self) -> &'w NonSendResourceStorage {
        &self.unsafe_world().non_send_resources
    }
}
