//! The world containing both entities and resources.

mod unsafe_cell;

pub use self::unsafe_cell::*;

use crate::entity::{Component, ComponentSet, EntityStorage, GroupLayout};
use crate::prelude::{Comp, CompMut, Entities, Entity, NonSend, NonSendMut, Res, ResMut};
use crate::resource::{NonSendResource, NonSendResourceStorage, Resource, ResourceStorage};

/// Storage for entities and resources.
#[derive(Default, Debug)]
pub struct World {
    /// Storage for entities.
    pub entities: EntityStorage,
    /// Storage for resources.
    pub resources: ResourceStorage,
    /// Storage for non-[`Send`] resources.
    pub non_send_resources: NonSendResourceStorage,
}

/// Create data from a [`World`].
pub trait FromWorld {
    /// Create data from the given [`World`].
    fn from_world(world: &mut World) -> Self;
}

impl<T: Default> FromWorld for T {
    fn from_world(_: &mut World) -> Self {
        T::default()
    }
}

impl World {
    /// Creates a new world with the given group layout.
    #[inline]
    #[must_use]
    pub fn new(layout: &GroupLayout) -> Self {
        Self {
            entities: EntityStorage::new(layout),
            resources: ResourceStorage::default(),
            non_send_resources: NonSendResourceStorage::default(),
        }
    }

    /// Creates a new [`UnsafeWorldCell`] view with only read access to everything.
    #[inline]
    pub fn as_unsafe_world_cell(&self) -> UnsafeWorldCell {
        UnsafeWorldCell::new(self)
    }

    /// Returns whether the world contains no entities and no resources.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty() && self.resources.is_empty() && self.non_send_resources.is_empty()
    }

    /// Removes all entities and all resources from the storage.
    #[inline]
    pub fn clear(&mut self) {
        self.entities.clear();
        self.resources.clear();
        self.non_send_resources.clear();
    }

    /// Removes all entities and all resources from the storage and resets the entity allocator.
    ///
    /// After this call, the storage is allowed to return previously allocated entities.
    #[inline]
    pub fn reset(&mut self) {
        self.entities.reset();
        self.resources.clear();
        self.non_send_resources.clear();
    }
}

impl World {
    /// Sets a new `GroupLayout`.
    ///
    /// This function iterates over all entities in the storage, so it is best called when the
    /// storage is empty.
    #[inline]
    pub fn set_layout(&mut self, layout: &GroupLayout) {
        self.entities.set_layout(layout);
    }
    /// Registers a new component type.
    ///
    /// Returns whether the component was newly registered.
    pub fn register<T>(&mut self) -> bool
    where
        T: Component,
    {
        self.entities.register::<T>()
    }

    /// Returns whether component type `T` is registered.
    #[must_use]
    pub fn is_registered<T>(&self) -> bool
    where
        T: Component,
    {
        self.entities.is_registered::<T>()
    }

    /// Creates a new entity with the given `components`.
    ///
    /// Returns the newly created entity.
    pub fn create<C>(&mut self, components: C) -> Entity
    where
        C: ComponentSet,
    {
        self.entities.create(components)
    }

    /// Creates new entities with the components produced by the iterator.
    ///
    /// Returns the newly created entities as a slice.
    pub fn extend<C, I>(&mut self, components: I) -> &[Entity]
    where
        C: ComponentSet,
        I: IntoIterator<Item = C>,
    {
        self.entities.extend(components)
    }

    /// Creates a new entity without requiring exclusive access to the storage. The entity is not
    /// added to the storage until [`maintain`](Self::maintain) is called.
    ///
    /// Returns the newly created entity.
    #[inline]
    pub fn create_atomic(&self) -> Entity {
        self.entities.create_atomic()
    }

    /// Adds the given `components` to `entity` if `entity` is present in the storage.
    ///
    /// Returns whether the components were successfully added.
    pub fn insert<C>(&mut self, entity: Entity, components: C) -> bool
    where
        C: ComponentSet,
    {
        self.entities.insert(entity, components)
    }

    /// Removes components from the given `entity`.
    ///
    /// Returns the components that were successfully removed.
    #[must_use = "Use `delete` to discard the components."]
    pub fn remove<C>(&mut self, entity: Entity) -> C::Remove
    where
        C: ComponentSet,
    {
        self.entities.remove::<C>(entity)
    }

    /// Removes components from the given `entity`.
    pub fn delete<C>(&mut self, entity: Entity)
    where
        C: ComponentSet,
    {
        self.entities.delete::<C>(entity)
    }

    /// Removes the given `entity` and its components from the storage.
    ///
    /// Returns whether the `entity` was present in the storage.
    #[inline]
    pub fn destroy(&mut self, entity: Entity) -> bool {
        self.entities.destroy(entity)
    }

    /// Adds the entities allocated with [`create_atomic`](Self::create_atomic) to the storage.
    #[inline]
    pub fn maintain(&mut self) {
        self.entities.maintain();
    }

    /// Returns wether `entity` is present in the storage.
    #[inline]
    #[must_use]
    pub fn contains(&self, entity: Entity) -> bool {
        self.entities.contains(entity)
    }

    /// Returns all entities in the storage as a slice.
    #[inline]
    #[must_use]
    pub fn entities(&self) -> &[Entity] {
        self.entities.entities()
    }

    /// Borrows a view over all entities in the storage.
    ///
    /// This view supports the creation of new entities without requiring exclusive access to the
    /// storage.
    #[inline]
    #[must_use]
    pub fn borrow_entities(&self) -> Entities {
        self.entities.borrow_entities()
    }

    /// Borrows a shared view over all components of type `T` in the storage.
    #[must_use]
    pub fn borrow<T>(&self) -> Comp<T>
    where
        T: Component,
    {
        self.entities.borrow()
    }

    /// Borrows an exclusive view over all components of type `T` in the storage.
    #[must_use]
    pub fn borrow_mut<T>(&self) -> CompMut<T>
    where
        T: Component,
    {
        self.entities.borrow_mut()
    }
}

impl World {
    /// Returns whether the storage contains a resource of type `T`.
    pub fn contains_resource<T>(&self) -> bool
    where
        T: Resource,
    {
        self.resources.contains::<T>()
    }

    /// Insert a new resource of type `T` into the storage.
    ///
    /// Returns the previous resource, if any.
    pub fn init_resource<T>(&mut self) -> Option<T>
    where
        T: Resource + FromWorld,
    {
        let resource = T::from_world(self);
        self.resources.insert(resource)
    }

    /// Insert a new resource of type `T` into the storage.
    ///
    /// Returns the previous resource, if any.
    pub fn insert_resource<T>(&mut self, resource: T) -> Option<T>
    where
        T: Resource,
    {
        self.resources.insert(resource)
    }

    /// Removes a resource of type `T` from the storage, if it exists.
    ///
    /// Returns the removed resource, if it was present.
    pub fn remove_resource<T>(&mut self) -> Option<T>
    where
        T: Resource,
    {
        self.resources.remove::<T>()
    }

    /// Borrows a resource of type `T` from the storage.
    #[must_use]
    pub fn resource<T>(&self) -> Res<T>
    where
        T: Resource,
    {
        self.resources.borrow::<T>()
    }

    /// Mutably borrows a resource of type `T` from the storage.
    #[must_use]
    pub fn resource_mut<T>(&self) -> ResMut<T>
    where
        T: Resource,
    {
        self.resources.borrow_mut::<T>()
    }

    /// Borrows a resource of type `T` from the storage, if it exists.
    #[must_use]
    pub fn try_resource<T>(&self) -> Option<Res<T>>
    where
        T: Resource,
    {
        self.resources.try_borrow::<T>()
    }

    /// Mutably borrows a resource of type `T` from the storage, if it exists.
    #[must_use]
    pub fn try_resource_mut<T>(&self) -> Option<ResMut<T>>
    where
        T: Resource,
    {
        self.resources.try_borrow_mut::<T>()
    }

    /// Gets a mutable reference to a resource of type `T`, if it exists.
    pub fn get_resource<T>(&mut self) -> Option<&mut T>
    where
        T: Resource,
    {
        self.resources.try_get_mut()
    }
}

impl World {
    /// Returns whether the storage contains a non-[`Send`] resource of type `T`.
    pub fn contains_non_send<T>(&self) -> bool
    where
        T: NonSendResource,
    {
        self.non_send_resources.contains::<T>()
    }

    /// Insert a new non-[`Send`] resource of type `T` into the storage.
    ///
    /// Returns the previous resource, if any.
    pub fn init_non_send<T>(&mut self) -> Option<T>
    where
        T: NonSendResource + FromWorld,
    {
        let resource = T::from_world(self);
        self.non_send_resources.insert(resource)
    }

    /// Insert a new non-[`Send`] resource of type `T` into the storage.
    ///
    /// Returns the previous resource, if any.
    pub fn insert_non_send<T>(&mut self, resource: T) -> Option<T>
    where
        T: NonSendResource,
    {
        self.non_send_resources.insert(resource)
    }

    /// Removes a resource of type `T` from the storage, if it exists.
    ///
    /// Returns the removed resource, if it was present.
    pub fn remove_non_send<T>(&mut self) -> Option<T>
    where
        T: NonSendResource,
    {
        self.non_send_resources.remove::<T>()
    }

    /// Borrows a non-[`Send`] resource of type `T` from the storage.
    #[must_use]
    pub fn non_send<T>(&self) -> NonSend<T>
    where
        T: NonSendResource,
    {
        self.non_send_resources.borrow::<T>()
    }

    /// Mutably borrows a non-[`Send`] resource of type `T` from the storage.
    #[must_use]
    pub fn non_send_mut<T>(&self) -> NonSendMut<T>
    where
        T: NonSendResource,
    {
        self.non_send_resources.borrow_mut::<T>()
    }
}
