use std::borrow::Cow;
use std::marker::PhantomData;

use crate::prelude::World;
use crate::world::UnsafeWorldCell;

use super::{
    In, IntoSystem, ReadonlySystem, ReadonlySystemParamSet, System, SystemParam, SystemParamKind,
    SystemParamSet,
};

pub struct FunctionSystem<F, Marker>
where
    F: SystemParamFunction<Marker>,
{
    func: F,
    param_state: Option<<F::Params as SystemParamSet>::State>,
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
        FunctionSystem {
            func: self,
            param_state: None,
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
        F::Params::KINDS
    }

    unsafe fn run_unsafe(&mut self, input: Self::In, world: UnsafeWorldCell) -> Self::Out {
        let state = self
            .param_state
            .as_mut()
            .expect("param_state not initialized");
        let params = F::Params::borrow(state, world);
        self.func.run(input, params)
    }

    fn apply_deferred(&mut self, world: &mut World) {
        let state = self
            .param_state
            .as_mut()
            .expect("param_state not initialized");
        F::Params::apply(state, world);
    }

    #[inline]
    fn initialize(&mut self, world: &mut World) {
        self.param_state = Some(F::Params::init_state(world));
    }

    #[inline]
    fn is_exclusive(&self) -> bool {
        false
    }

    #[inline]
    fn is_thread_local(&self) -> bool {
        !F::Params::SEND
    }
}

unsafe impl<F, Marker> ReadonlySystem for FunctionSystem<F, Marker>
where
    Marker: 'static,
    F: SystemParamFunction<Marker>,
    F::Params: ReadonlySystemParamSet,
{
}

pub trait SystemParamFunction<Marker>: Send + Sync + 'static {
    type In;
    type Out;
    type Params: SystemParamSet;

    fn run(
        &mut self,
        input: Self::In,
        param: <Self::Params as SystemParamSet>::Item<'_, '_>,
    ) -> Self::Out;
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
            type Params = ($($Param,)*);

            #[inline]
            #[allow(unused_variables)]
            fn run(&mut self, _: (), param: <Self::Params as SystemParamSet>::Item<'_, '_>) -> Out {
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
            type Params = ($($Param,)*);

            #[inline]
            #[allow(unused_variables)]
            fn run(&mut self, input: Input, param: <Self::Params as SystemParamSet>::Item<'_, '_>) -> Out {
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
