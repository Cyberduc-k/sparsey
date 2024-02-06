use std::borrow::Cow;
use std::marker::PhantomData;

use crate::prelude::World;
use crate::world::UnsafeWorldCell;

use super::{
    In, IntoSystem, ReadonlySystem, ReadonlySystemParam, System, SystemParam, SystemParamItem,
    SystemParamKind,
};

/// This is a powerful and convenient tool for working with exclusive world access, allowing you to
/// fetch data from the [`World`] as if you were running a [`System`].
pub struct SystemState<Param: SystemParam + 'static> {
    param_state: Param::State,
}

impl<Param: SystemParam> SystemState<Param> {
    /// Creates a new [`SystemState`] with default state.
    pub fn new(world: &mut World) -> Self {
        Self {
            param_state: Param::init_state(world),
        }
    }

    /// Applies deferred mutations to the [`World`].
    pub fn apply(&mut self, world: &mut World) {
        Param::apply(&mut self.param_state, world);
    }

    /// Retrieve the [`SystemParam`] values. This can only be called when all parameters are
    /// read-only.
    pub fn get<'w, 's>(&'s mut self, world: &'w World) -> SystemParamItem<'w, 's, Param>
    where
        Param: ReadonlySystemParam,
    {
        unsafe { self.get_unchecked(world.as_unsafe_world_cell()) }
    }

    /// Retrieve the [`SystemParam`] values.
    pub fn get_mut<'w, 's>(&'s mut self, world: &'w mut World) -> SystemParamItem<'w, 's, Param> {
        unsafe { self.get_unchecked(world.as_unsafe_world_cell()) }
    }

    /// Retrieve the [`SystemParam`] values.
    pub unsafe fn get_unchecked<'w, 's>(
        &'s mut self,
        world: UnsafeWorldCell<'w>,
    ) -> SystemParamItem<'w, 's, Param> {
        Param::borrow(&mut self.param_state, world)
    }
}

/// The [`System`] counter part of an ordinary function.
pub struct FunctionSystem<F, Marker>
where
    F: SystemParamFunction<Marker>,
{
    func: F,
    param_state: Option<<F::Param as SystemParam>::State>,
    param_kinds: Vec<SystemParamKind>,
    marker: PhantomData<fn() -> Marker>,
}

#[doc(hidden)]
pub struct IsFunctionSystem;

impl<Marker, F> IntoSystem<F::In, F::Out, (IsFunctionSystem, Marker)> for F
where
    Marker: 'static,
    F: SystemParamFunction<Marker>,
{
    type System = FunctionSystem<F, Marker>;

    fn into_system(self) -> Self::System {
        let mut param_kinds = Vec::new();
        F::Param::param_kinds(&mut param_kinds);
        FunctionSystem {
            func: self,
            param_state: None,
            param_kinds,
            marker: PhantomData,
        }
    }
}

impl<Marker, F> System for FunctionSystem<F, Marker>
where
    Marker: 'static,
    F: SystemParamFunction<Marker>,
{
    type In = F::In;
    type Out = F::Out;

    #[inline]
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed(std::any::type_name::<F>())
    }

    #[inline]
    fn param_kinds(&self) -> &[SystemParamKind] {
        &self.param_kinds
    }

    unsafe fn run_unsafe(&mut self, input: Self::In, world: UnsafeWorldCell) -> Self::Out {
        let state = self
            .param_state
            .as_mut()
            .expect("param_state not initialized");
        let params = F::Param::borrow(state, world);
        self.func.run(input, params)
    }

    fn apply_deferred(&mut self, world: &mut World) {
        let state = self
            .param_state
            .as_mut()
            .expect("param_state not initialized");
        F::Param::apply(state, world);
    }

    #[inline]
    fn initialize(&mut self, world: &mut World) {
        self.param_state = Some(F::Param::init_state(world));
    }

    #[inline]
    fn is_exclusive(&self) -> bool {
        false
    }

    #[inline]
    fn is_thread_local(&self) -> bool {
        !F::Param::SEND
    }
}

unsafe impl<F, Marker> ReadonlySystem for FunctionSystem<F, Marker>
where
    Marker: 'static,
    F: SystemParamFunction<Marker>,
    F::Param: ReadonlySystemParam,
{
}

/// A trait implemented for all functions that can used as [`System`]s.
pub trait SystemParamFunction<Marker>: Send + Sync + 'static {
    /// The input type to this system.
    type In;
    /// The return type of this system.
    type Out;
    /// The [`SystemParam`]/s used by this system.
    type Param: SystemParam;

    /// Executes this system once.
    fn run(&mut self, input: Self::In, param: SystemParamItem<'_, '_, Self::Param>) -> Self::Out;
}

macro_rules! impl_system_function {
    ($(($Param:ident $n:tt)),*) => {
        #[allow(non_snake_case)]
        impl<Out, Func, $($Param),*> SystemParamFunction<fn($($Param),*) -> Out> for Func
        where
            Out: 'static,
            Func: Send + Sync + 'static,
            for<'a> &'a mut Func:
                FnMut($($Param),*) -> Out +
                FnMut($(<$Param as SystemParam>::Item<'_, '_>),*) -> Out,
            $($Param: SystemParam),*
        {
            type In = ();
            type Out = Out;
            type Param = ($($Param,)*);

            #[inline]
            #[allow(unused_variables)]
            fn run(&mut self, _: (), param: SystemParamItem<'_, '_, Self::Param>) -> Out {
                #[allow(clippy::too_many_arguments)]
                fn call_inner<Out, $($Param),*>(
                    mut f: impl FnMut($($Param),*) -> Out,
                    $($Param: $Param,)*
                ) -> Out {
                    f($($Param,)*)
                }
                call_inner(self, $(param.$n),*)
            }
        }

        #[allow(non_snake_case)]
        impl<Input, Out, Func, $($Param),*> SystemParamFunction<fn(In<Input>, $($Param),*) -> Out> for Func
        where
            Out: 'static,
            Func: Send + Sync + 'static,
            for<'a> &'a mut Func:
                FnMut(In<Input>, $($Param),*) -> Out +
                FnMut(In<Input>, $(<$Param as SystemParam>::Item<'_, '_>),*) -> Out,
            $($Param: SystemParam),*
        {
            type In = Input;
            type Out = Out;
            type Param = ($($Param,)*);

            #[inline]
            #[allow(unused_variables)]
            fn run(&mut self, input: Input, param: SystemParamItem<'_, '_, Self::Param>) -> Out {
                #[allow(clippy::too_many_arguments)]
                fn call_inner<Input, Out, $($Param),*>(
                    mut f: impl FnMut(In<Input>, $($Param),*) -> Out,
                    input: In<Input>,
                    $($Param: $Param,)*
                ) -> Out {
                    f(input, $($Param),*)
                }
                call_inner(self, In(input), $(param.$n),*)
            }
        }
    };
}

impl_system_function!();
impl_system_function!((A 0));
impl_system_function!((A 0), (B 1));
impl_system_function!((A 0), (B 1), (C 2));
