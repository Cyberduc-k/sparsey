use crate::entity::{Comp, CompMut, Component, Entities};
use crate::prelude::{EntityStorage, ResourceStorage};
use crate::resource::{Res, ResMut, Resource};
use crate::util::TypeData;
use crate::World;

use super::LocalData;

/// The kind of data that can be borrowed from a registry.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum SystemParamKind {
    /// View over all entities in an [`EntityStorage`](crate::entity::EntityStorage).
    Entities,
    /// State local to a system.
    State(TypeData),
    /// Shared view over all components of a given type.
    Comp(TypeData),
    /// Exclusive view over all components of a given type.
    CompMut(TypeData),
    /// Shared view over a resource of a given type.
    Res(TypeData),
    /// Exclusive view over a resource of a given type.
    ResMut(TypeData),
}

impl SystemParamKind {
    /// Returns whether two system parameter kinds conflict, thus preventing two systems from
    /// running in parallel.
    #[inline]
    #[must_use]
    pub fn conflicts_with(self, other: Self) -> bool {
        #[allow(clippy::match_same_arms)]
        match (self, other) {
            (Self::Comp(c1), Self::CompMut(c2)) => c1 == c2,
            (Self::CompMut(c1), Self::Comp(c2)) => c1 == c2,
            (Self::CompMut(c1), Self::CompMut(c2)) => c1 == c2,
            (Self::Res(r1), Self::ResMut(r2)) => r1 == r2,
            (Self::ResMut(r1), Self::Res(r2)) => r1 == r2,
            (Self::ResMut(r1), Self::ResMut(r2)) => r1 == r2,
            _ => false,
        }
    }
}

/// Trait implemented by types that can be borrowed by systems during execution.
pub trait SystemParam<TRegistry> {
    /// The kind of system parameter.
    const KIND: SystemParamKind;

    /// The system parameter generic over the lifetimes `'w` and `'s`.
    type Item<'w, 's>
    where
        TRegistry: 'w;

    /// The state used by this parameter.
    type State: LocalData;

    /// Create the initial state from the given `registry`.
    fn init_state(registry: &mut TRegistry) -> Self::State;

    /// Borrows data from the given `registry`.
    #[must_use]
    fn borrow<'w, 's>(state: &'s mut Self::State, registry: &'w TRegistry) -> Self::Item<'w, 's>;

    /// Apply any deferred mutations to the given `registry`.
    #[allow(unused_variables)]
    fn apply(state: &mut Self::State, registry: &mut TRegistry) {}
}

/// A set of multiple [`SystemParam`].
pub trait SystemParamSet<TRegistry> {
    /// The kinds of system parameters.
    const KINDS: &'static [SystemParamKind];

    /// The system parameter set generic over the lifetimes `'w` and `'s`.
    type Item<'w, 's>
    where
        TRegistry: 'w;

    /// The state used by this parameter set.
    type State: LocalData;

    /// Create the initial state from the given `registry`.
    fn init_state(registry: &mut TRegistry) -> Self::State;

    /// Borrows data from the given `registry`.
    #[must_use]
    fn borrow<'w, 's>(state: &'s mut Self::State, registry: &'w TRegistry) -> Self::Item<'w, 's>;

    /// Apply any deferred mutations to the given `registry`
    fn apply(state: &mut Self::State, registry: &mut TRegistry);
}

/// A [`SystemParam`] that only reads the given registry.
pub unsafe trait ReadonlySystemParam<TRegistry>: SystemParam<TRegistry> {}

/// A [`SystemParamSet`] that only reads the given registry.
pub unsafe trait ReadonlySystemParamSet<TRegistry>: SystemParamSet<TRegistry> {}

impl SystemParam<World> for Entities<'_> {
    const KIND: SystemParamKind = SystemParamKind::Entities;

    type Item<'w, 's> = Entities<'w>;
    type State = ();

    fn init_state(_: &mut World) -> Self::State {}

    fn borrow<'w, 's>(_: &'s mut Self::State, world: &'w World) -> Self::Item<'w, 's> {
        world.entities.borrow_entities()
    }
}

impl SystemParam<EntityStorage> for Entities<'_> {
    const KIND: SystemParamKind = SystemParamKind::Entities;

    type Item<'w, 's> = Entities<'w>;
    type State = ();

    fn init_state(_: &mut EntityStorage) -> Self::State {}

    fn borrow<'w, 's>(_: &'s mut Self::State, entities: &'w EntityStorage) -> Self::Item<'w, 's> {
        entities.borrow_entities()
    }
}

unsafe impl ReadonlySystemParam<World> for Entities<'_> {}
unsafe impl ReadonlySystemParam<EntityStorage> for Entities<'_> {}

impl<T> SystemParam<World> for Comp<'_, T>
where
    T: Component,
{
    const KIND: SystemParamKind = SystemParamKind::Comp(TypeData::new::<T>());

    type Item<'w, 's> = Comp<'w, T>;
    type State = ();

    fn init_state(_: &mut World) -> Self::State {}

    fn borrow<'w, 's>(_: &'s mut Self::State, world: &'w World) -> Self::Item<'w, 's> {
        world.entities.borrow()
    }
}

impl<T> SystemParam<EntityStorage> for Comp<'_, T>
where
    T: Component,
{
    const KIND: SystemParamKind = SystemParamKind::Comp(TypeData::new::<T>());

    type Item<'w, 's> = Comp<'w, T>;
    type State = ();

    fn init_state(_: &mut EntityStorage) -> Self::State {}

    fn borrow<'w, 's>(_: &'s mut Self::State, entities: &'w EntityStorage) -> Self::Item<'w, 's> {
        entities.borrow()
    }
}

unsafe impl<T: Component> ReadonlySystemParam<World> for Comp<'_, T> {}
unsafe impl<T: Component> ReadonlySystemParam<EntityStorage> for Comp<'_, T> {}

impl<T> SystemParam<World> for CompMut<'_, T>
where
    T: Component,
{
    const KIND: SystemParamKind = SystemParamKind::CompMut(TypeData::new::<T>());

    type Item<'w, 's> = CompMut<'w, T>;
    type State = ();

    fn init_state(_: &mut World) -> Self::State {}

    fn borrow<'w, 's>(_: &'s mut Self::State, world: &'w World) -> Self::Item<'w, 's> {
        world.entities.borrow_mut()
    }
}

impl<T> SystemParam<EntityStorage> for CompMut<'_, T>
where
    T: Component,
{
    const KIND: SystemParamKind = SystemParamKind::CompMut(TypeData::new::<T>());

    type Item<'w, 's> = CompMut<'w, T>;
    type State = ();

    fn init_state(_: &mut EntityStorage) -> Self::State {}

    fn borrow<'w, 's>(_: &'s mut Self::State, entities: &'w EntityStorage) -> Self::Item<'w, 's> {
        entities.borrow_mut()
    }
}

impl<T> SystemParam<World> for Res<'_, T>
where
    T: Resource,
{
    const KIND: SystemParamKind = SystemParamKind::Res(TypeData::new::<T>());

    type Item<'w, 's> = Res<'w, T>;
    type State = ();

    fn init_state(_: &mut World) -> Self::State {}

    fn borrow<'w, 's>(_: &'s mut Self::State, world: &'w World) -> Self::Item<'w, 's> {
        world.resources.borrow()
    }
}

impl<T> SystemParam<ResourceStorage> for Res<'_, T>
where
    T: Resource,
{
    const KIND: SystemParamKind = SystemParamKind::Res(TypeData::new::<T>());

    type Item<'w, 's> = Res<'w, T>;
    type State = ();

    fn init_state(_: &mut ResourceStorage) -> Self::State {}

    fn borrow<'w, 's>(
        _: &'s mut Self::State,
        resources: &'w ResourceStorage,
    ) -> Self::Item<'w, 's> {
        resources.borrow()
    }
}

unsafe impl<T: Resource> ReadonlySystemParam<World> for Res<'_, T> {}
unsafe impl<T: Resource> ReadonlySystemParam<ResourceStorage> for Res<'_, T> {}

impl<T> SystemParam<World> for ResMut<'_, T>
where
    T: Resource,
{
    const KIND: SystemParamKind = SystemParamKind::ResMut(TypeData::new::<T>());

    type Item<'w, 's> = ResMut<'w, T>;
    type State = ();

    fn init_state(_: &mut World) -> Self::State {}

    fn borrow<'w, 's>(_: &'s mut Self::State, world: &'w World) -> Self::Item<'w, 's> {
        world.resources.borrow_mut()
    }
}

impl<T> SystemParam<ResourceStorage> for ResMut<'_, T>
where
    T: Resource,
{
    const KIND: SystemParamKind = SystemParamKind::ResMut(TypeData::new::<T>());

    type Item<'w, 's> = ResMut<'w, T>;
    type State = ();

    fn init_state(_: &mut ResourceStorage) -> Self::State {}

    fn borrow<'w, 's>(
        _: &'s mut Self::State,
        resources: &'w ResourceStorage,
    ) -> Self::Item<'w, 's> {
        resources.borrow_mut()
    }
}

impl<T> SystemParam<World> for Option<Res<'_, T>>
where
    T: Resource,
{
    const KIND: SystemParamKind = SystemParamKind::Res(TypeData::new::<T>());

    type Item<'w, 's> = Option<Res<'w, T>>;
    type State = ();

    fn init_state(_: &mut World) -> Self::State {}

    fn borrow<'w, 's>(_: &'s mut Self::State, world: &'w World) -> Self::Item<'w, 's> {
        world.resources.try_borrow()
    }
}

impl<T> SystemParam<ResourceStorage> for Option<Res<'_, T>>
where
    T: Resource,
{
    const KIND: SystemParamKind = SystemParamKind::Res(TypeData::new::<T>());

    type Item<'w, 's> = Option<Res<'w, T>>;
    type State = ();

    fn init_state(_: &mut ResourceStorage) -> Self::State {}

    fn borrow<'w, 's>(
        _: &'s mut Self::State,
        resources: &'w ResourceStorage,
    ) -> Self::Item<'w, 's> {
        resources.try_borrow()
    }
}

unsafe impl<T: Resource> ReadonlySystemParam<World> for Option<Res<'_, T>> {}
unsafe impl<T: Resource> ReadonlySystemParam<ResourceStorage> for Option<Res<'_, T>> {}

impl<T> SystemParam<World> for Option<ResMut<'_, T>>
where
    T: Resource,
{
    const KIND: SystemParamKind = SystemParamKind::ResMut(TypeData::new::<T>());

    type Item<'w, 's> = Option<ResMut<'w, T>>;
    type State = ();

    fn init_state(_: &mut World) -> Self::State {}

    fn borrow<'w, 's>(_: &'s mut Self::State, world: &'w World) -> Self::Item<'w, 's> {
        world.resources.try_borrow_mut()
    }
}

impl<T> SystemParam<ResourceStorage> for Option<ResMut<'_, T>>
where
    T: Resource,
{
    const KIND: SystemParamKind = SystemParamKind::ResMut(TypeData::new::<T>());

    type Item<'w, 's> = Option<ResMut<'w, T>>;
    type State = ();

    fn init_state(_: &mut ResourceStorage) -> Self::State {}

    fn borrow<'w, 's>(
        _: &'s mut Self::State,
        resources: &'w ResourceStorage,
    ) -> Self::Item<'w, 's> {
        resources.try_borrow_mut()
    }
}

macro_rules! impl_system_param_set {
    ($(($Param:ident $n:tt)),*) => {
        impl<TRegistry, $($Param),*> SystemParamSet<TRegistry> for ($($Param,)*)
        where
            $($Param: SystemParam<TRegistry>),*
        {
            const KINDS: &'static [SystemParamKind] = &[$($Param::KIND),*];

            type Item<'w, 's> = ($($Param::Item<'w, 's>,)*) where TRegistry: 'w;
            type State = ($($Param::State,)*);

            #[allow(clippy::unused_unit, unused_variables)]
            fn init_state(registry: &mut TRegistry) -> Self::State {
                ($($Param::init_state(registry),)*)
            }

            #[allow(clippy::unused_unit, unused_variables)]
            fn borrow<'w, 's>(state: &'s mut Self::State, registry: &'w TRegistry) -> Self::Item<'w, 's> {
                ($($Param::borrow(&mut state.$n, registry),)*)
            }

            #[allow(unused_variables)]
            fn apply(state: &mut Self::State, registry: &mut TRegistry) {
                $($Param::apply(&mut state.$n, registry);)*
            }
        }

        unsafe impl<TRegistry, $($Param),*> ReadonlySystemParamSet<TRegistry> for ($($Param,)*)
        where
            $($Param: ReadonlySystemParam<TRegistry>),*
        {}
    };
}

impl_system_param_set!();
impl_system_param_set!((A 0));
impl_system_param_set!((A 0), (B 1));
impl_system_param_set!((A 0), (B 1), (C 2));
