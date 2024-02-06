use crate::entity::{Comp, CompMut, Component, Entities};
use crate::prelude::{NonSend, NonSendMut, World};
use crate::resource::{NonSendResource, Res, ResMut, Resource};
use crate::util::TypeData;
use crate::world::UnsafeWorldCell;

use super::LocalData;

/// The kind of data that can be borrowed from a registry.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum SystemParamKind {
    /// View over all entities in an [`EntityStorage`](crate::entity::EntityStorage).
    Entities,
    /// Shared view over all components of a given type.
    Comp(TypeData),
    /// Exclusive view over all components of a given type.
    CompMut(TypeData),
    /// Shared view over a resource of a given type.
    Res(TypeData),
    /// Exclusive view over a resource of a given type.
    ResMut(TypeData),
    /// Shared view over a non-[`Send`] resource of a given type.
    NonSend(TypeData),
    /// Exclusive view over a non-[`Send`] resource of a given type.
    NonSendMut(TypeData),
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
            (Self::NonSend(r1), Self::NonSendMut(r2)) => r1 == r2,
            (Self::NonSendMut(r1), Self::NonSend(r2)) => r1 == r2,
            (Self::NonSendMut(r1), Self::NonSendMut(r2)) => r1 == r2,
            _ => false,
        }
    }
}

/// Trait implemented by types that can be borrowed by systems during execution.
pub trait SystemParam {
    /// Whether this paramter is [`Send`] and [`Sync`].
    const SEND: bool;

    /// The system parameter generic over the lifetimes `'w` and `'s`.
    type Item<'w, 's>: SystemParam<State = Self::State>;

    /// The state used by this parameter.
    type State: LocalData;

    /// Fills `kinds` with all parameter kinds used by this [`SystemParam`].
    #[inline]
    #[allow(unused_variables)]
    fn param_kinds(kinds: &mut Vec<SystemParamKind>) {}

    /// Create the initial state from the [`World`].
    fn init_state(world: &mut World) -> Self::State;

    /// Apply any deferred mutations to the [`World`].
    #[inline]
    #[allow(unused_variables)]
    fn apply(state: &mut Self::State, world: &mut World) {}

    /// Borrows data from the [`World`].
    #[must_use]
    unsafe fn borrow<'w, 's>(
        state: &'s mut Self::State,
        world: UnsafeWorldCell<'w>,
    ) -> Self::Item<'w, 's>;
}

/// Shorthand way of accessing the associated type [`SystemParam::Item`] for a given
/// [`SystemParam`].
pub type SystemParamItem<'w, 's, P> = <P as SystemParam>::Item<'w, 's>;

/// A [`SystemParam`] that only reads the [`World`].
pub unsafe trait ReadonlySystemParam: SystemParam {}

impl SystemParam for Entities<'_> {
    const SEND: bool = true;

    type Item<'w, 's> = Entities<'w>;
    type State = ();

    fn param_kinds(kinds: &mut Vec<SystemParamKind>) {
        kinds.push(SystemParamKind::Entities);
    }

    fn init_state(_: &mut World) -> Self::State {}

    unsafe fn borrow<'w, 's>(
        _: &'s mut Self::State,
        world: UnsafeWorldCell<'w>,
    ) -> Self::Item<'w, 's> {
        world.entities().borrow_entities()
    }
}

unsafe impl ReadonlySystemParam for Entities<'_> {}

impl<T> SystemParam for Comp<'_, T>
where
    T: Component,
{
    const SEND: bool = true;

    type Item<'w, 's> = Comp<'w, T>;
    type State = ();

    fn param_kinds(kinds: &mut Vec<SystemParamKind>) {
        kinds.push(SystemParamKind::Comp(TypeData::new::<T>()));
    }

    fn init_state(_: &mut World) -> Self::State {}

    unsafe fn borrow<'w, 's>(
        _: &'s mut Self::State,
        world: UnsafeWorldCell<'w>,
    ) -> Self::Item<'w, 's> {
        world.entities().borrow()
    }
}

unsafe impl<T: Component> ReadonlySystemParam for Comp<'_, T> {}

impl<T> SystemParam for CompMut<'_, T>
where
    T: Component,
{
    const SEND: bool = true;

    type Item<'w, 's> = CompMut<'w, T>;
    type State = ();

    fn param_kinds(kinds: &mut Vec<SystemParamKind>) {
        kinds.push(SystemParamKind::CompMut(TypeData::new::<T>()));
    }

    fn init_state(_: &mut World) -> Self::State {}

    unsafe fn borrow<'w, 's>(
        _: &'s mut Self::State,
        world: UnsafeWorldCell<'w>,
    ) -> Self::Item<'w, 's> {
        world.entities().borrow_mut()
    }
}

impl<T> SystemParam for Res<'_, T>
where
    T: Resource,
{
    const SEND: bool = true;

    type Item<'w, 's> = Res<'w, T>;
    type State = ();

    fn param_kinds(kinds: &mut Vec<SystemParamKind>) {
        kinds.push(SystemParamKind::Res(TypeData::new::<T>()));
    }

    fn init_state(_: &mut World) -> Self::State {}

    unsafe fn borrow<'w, 's>(
        _: &'s mut Self::State,
        world: UnsafeWorldCell<'w>,
    ) -> Self::Item<'w, 's> {
        world.resources().borrow()
    }
}

unsafe impl<T: Resource> ReadonlySystemParam for Res<'_, T> {}

impl<T> SystemParam for ResMut<'_, T>
where
    T: Resource,
{
    const SEND: bool = true;

    type Item<'w, 's> = ResMut<'w, T>;
    type State = ();

    fn param_kinds(kinds: &mut Vec<SystemParamKind>) {
        kinds.push(SystemParamKind::ResMut(TypeData::new::<T>()));
    }

    fn init_state(_: &mut World) -> Self::State {}

    unsafe fn borrow<'w, 's>(
        _: &'s mut Self::State,
        world: UnsafeWorldCell<'w>,
    ) -> Self::Item<'w, 's> {
        world.resources().borrow_mut()
    }
}

impl<T> SystemParam for Option<Res<'_, T>>
where
    T: Resource,
{
    const SEND: bool = true;

    type Item<'w, 's> = Option<Res<'w, T>>;
    type State = ();

    fn param_kinds(kinds: &mut Vec<SystemParamKind>) {
        kinds.push(SystemParamKind::Res(TypeData::new::<T>()));
    }

    fn init_state(_: &mut World) -> Self::State {}

    unsafe fn borrow<'w, 's>(
        _: &'s mut Self::State,
        world: UnsafeWorldCell<'w>,
    ) -> Self::Item<'w, 's> {
        world.resources().try_borrow()
    }
}

unsafe impl<T: Resource> ReadonlySystemParam for Option<Res<'_, T>> {}

impl<T> SystemParam for Option<ResMut<'_, T>>
where
    T: Resource,
{
    const SEND: bool = true;

    type Item<'w, 's> = Option<ResMut<'w, T>>;
    type State = ();

    fn param_kinds(kinds: &mut Vec<SystemParamKind>) {
        kinds.push(SystemParamKind::ResMut(TypeData::new::<T>()));
    }

    fn init_state(_: &mut World) -> Self::State {}

    unsafe fn borrow<'w, 's>(
        _: &'s mut Self::State,
        world: UnsafeWorldCell<'w>,
    ) -> Self::Item<'w, 's> {
        world.resources().try_borrow_mut()
    }
}

impl<T> SystemParam for NonSend<'_, T>
where
    T: NonSendResource,
{
    const SEND: bool = false;

    type Item<'w, 's> = NonSend<'w, T>;
    type State = ();

    fn param_kinds(kinds: &mut Vec<SystemParamKind>) {
        kinds.push(SystemParamKind::Res(TypeData::new_non_send::<T>()));
    }

    fn init_state(_: &mut World) -> Self::State {}

    unsafe fn borrow<'w, 's>(
        _: &'s mut Self::State,
        world: UnsafeWorldCell<'w>,
    ) -> Self::Item<'w, 's> {
        world.non_send_resources().borrow()
    }
}

unsafe impl<T: NonSendResource> ReadonlySystemParam for NonSend<'_, T> {}

impl<T> SystemParam for NonSendMut<'_, T>
where
    T: NonSendResource,
{
    const SEND: bool = false;

    type Item<'w, 's> = NonSendMut<'w, T>;
    type State = ();

    fn param_kinds(kinds: &mut Vec<SystemParamKind>) {
        kinds.push(SystemParamKind::NonSendMut(TypeData::new_non_send::<T>()));
    }

    fn init_state(_: &mut World) -> Self::State {}

    unsafe fn borrow<'w, 's>(
        _: &'s mut Self::State,
        world: UnsafeWorldCell<'w>,
    ) -> Self::Item<'w, 's> {
        world.non_send_resources().borrow_mut()
    }
}

impl<T> SystemParam for Option<NonSend<'_, T>>
where
    T: NonSendResource,
{
    const SEND: bool = false;

    type Item<'w, 's> = Option<NonSend<'w, T>>;
    type State = ();

    fn param_kinds(kinds: &mut Vec<SystemParamKind>) {
        kinds.push(SystemParamKind::NonSend(TypeData::new_non_send::<T>()));
    }

    fn init_state(_: &mut World) -> Self::State {}

    unsafe fn borrow<'w, 's>(
        _: &'s mut Self::State,
        world: UnsafeWorldCell<'w>,
    ) -> Self::Item<'w, 's> {
        world.non_send_resources().try_borrow()
    }
}

unsafe impl<T: NonSendResource> ReadonlySystemParam for Option<NonSend<'_, T>> {}

impl<T> SystemParam for Option<NonSendMut<'_, T>>
where
    T: NonSendResource,
{
    const SEND: bool = false;

    type Item<'w, 's> = Option<NonSendMut<'w, T>>;
    type State = ();

    fn param_kinds(kinds: &mut Vec<SystemParamKind>) {
        kinds.push(SystemParamKind::NonSendMut(TypeData::new_non_send::<T>()));
    }

    fn init_state(_: &mut World) -> Self::State {}

    unsafe fn borrow<'w, 's>(
        _: &'s mut Self::State,
        world: UnsafeWorldCell<'w>,
    ) -> Self::Item<'w, 's> {
        world.non_send_resources().try_borrow_mut()
    }
}

macro_rules! impl_system_param_set {
    ($(($Param:ident $n:tt)),*) => {
        impl<$($Param),*> SystemParam for ($($Param,)*)
        where
            $($Param: SystemParam),*
        {
            const SEND: bool = true $(&& $Param::SEND)*;

            type Item<'w, 's> = ($($Param::Item<'w, 's>,)*);
            type State = ($($Param::State,)*);

            #[allow(unused_variables)]
            fn param_kinds(kinds: &mut Vec<SystemParamKind>) {
                $($Param::param_kinds(kinds);)*
            }

            #[allow(clippy::unused_unit, unused_variables)]
            fn init_state(world: &mut World) -> Self::State {
                ($($Param::init_state(world),)*)
            }

            #[allow(unused_variables)]
            fn apply(state: &mut Self::State, world: &mut World) {
                $($Param::apply(&mut state.$n, world);)*
            }

            #[allow(clippy::unused_unit, unused_variables)]
            unsafe fn borrow<'w, 's>(state: &'s mut Self::State, world: UnsafeWorldCell<'w>) -> Self::Item<'w, 's> {
                ($($Param::borrow(&mut state.$n, world),)*)
            }
        }

        unsafe impl<$($Param),*> ReadonlySystemParam for ($($Param,)*)
        where
            $($Param: ReadonlySystemParam),*
        {}
    };
}

impl_system_param_set!();
impl_system_param_set!((A 0));
impl_system_param_set!((A 0), (B 1));
impl_system_param_set!((A 0), (B 1), (C 2));
