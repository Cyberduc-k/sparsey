use std::borrow::Cow;
use std::marker::PhantomData;

use crate::prelude::World;
use crate::world::UnsafeWorldCell;

use super::exclusive_param::{ExclusiveSystemParam, ExclusiveSystemParamSet};
use super::{In, IntoSystem, System};

pub struct ExclusiveSystem<F, Marker>
where
    F: ExclusiveSystemParamFunction<Marker>,
{
    func: F,
    param_state: Option<<F::Params as ExclusiveSystemParamSet>::State>,
    marker: PhantomData<fn() -> Marker>,
}

#[doc(hidden)]
pub struct IsExclusiveSystem;

impl<Marker, F> IntoSystem<F::In, F::Out, (IsExclusiveSystem, Marker)> for F
where
    Marker: 'static,
    F: ExclusiveSystemParamFunction<Marker>,
{
    type System = ExclusiveSystem<F, Marker>;

    fn into_system(self) -> Self::System {
        ExclusiveSystem {
            func: self,
            param_state: None,
            marker: PhantomData,
        }
    }
}

impl<Marker, F> System for ExclusiveSystem<F, Marker>
where
    Marker: 'static,
    F: ExclusiveSystemParamFunction<Marker>,
{
    type In = F::In;
    type Out = F::Out;

    #[inline]
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed(std::any::type_name::<F>())
    }

    #[inline]
    fn param_kinds(&self) -> &[super::SystemParamKind] {
        &[]
    }

    #[inline]
    unsafe fn run_unsafe(&mut self, _: Self::In, _: UnsafeWorldCell) -> Self::Out {
        panic!("cannot run exclusive systems with a shared registry");
    }

    fn run(&mut self, input: Self::In, world: &mut World) -> Self::Out {
        let state = self
            .param_state
            .as_mut()
            .expect("param_state not initialized");
        let params = F::Params::borrow(state);
        self.func.run(world, input, params)
    }

    #[inline]
    fn apply_deferred(&mut self, _: &mut World) {}

    #[inline]
    fn initialize(&mut self, registry: &mut World) {
        self.param_state = Some(F::Params::init_state(registry));
    }

    #[inline]
    fn is_exclusive(&self) -> bool {
        true
    }

    #[inline]
    fn is_thread_local(&self) -> bool {
        true
    }
}

pub trait ExclusiveSystemParamFunction<Marker>: Send + Sync + 'static {
    type In;
    type Out;
    type Params: ExclusiveSystemParamSet;

    fn run(
        &mut self,
        world: &mut World,
        input: Self::In,
        param: <Self::Params as ExclusiveSystemParamSet>::Item<'_>,
    ) -> Self::Out;
}

macro_rules! impl_exclusive_system_function {
    ($(($Param:ident $n:tt)),*) => {
        #[allow(non_snake_case)]
        impl<Out, Func, $($Param),*> ExclusiveSystemParamFunction<fn($($Param),*) -> Out> for Func
        where
            Out: 'static,
            Func: Send + Sync + 'static,
            for<'a> &'a mut Func:
                FnMut(&mut World, $($Param),*) -> Out +
                FnMut(&mut World, $(<$Param as ExclusiveSystemParam>::Item<'_>),*) -> Out,
            $($Param: ExclusiveSystemParam),*
        {
            type In = ();
            type Out = Out;
            type Params = ($($Param,)*);

            #[inline]
            #[allow(unused_variables)]
            fn run(&mut self, world: &mut World, _: (), param: <Self::Params as ExclusiveSystemParamSet>::Item<'_>) -> Out {
                #[allow(clippy::too_many_arguments)]
                fn call_inner<Out, $($Param),*>(
                    mut f: impl FnMut(&mut World, $($Param),*) -> Out,
                    world: &mut World,
                    $($Param: $Param,)*
                ) -> Out {
                    f(world, $($Param,)*)
                }
                call_inner(self, world, $(param.$n),*)
            }
        }

        #[allow(non_snake_case)]
        impl<Input, Out, Func, $($Param),*> ExclusiveSystemParamFunction<fn(In<Input>, $($Param),*) -> Out> for Func
        where
            Out: 'static,
            Func: Send + Sync + 'static,
            for<'a> &'a mut Func:
                FnMut(&mut World, In<Input>, $($Param),*) -> Out +
                FnMut(&mut World, In<Input>, $(<$Param as ExclusiveSystemParam>::Item<'_>),*) -> Out,
            $($Param: ExclusiveSystemParam),*
        {
            type In = Input;
            type Out = Out;
            type Params = ($($Param,)*);

            #[inline]
            #[allow(unused_variables)]
            fn run(&mut self, world: &mut World, input: Input, param: <Self::Params as ExclusiveSystemParamSet>::Item<'_>) -> Out {
                #[allow(clippy::too_many_arguments)]
                fn call_inner<Input, Out, $($Param),*>(
                    mut f: impl FnMut(&mut World, In<Input>, $($Param),*) -> Out,
                    world: &mut World,
                    input: In<Input>,
                    $($Param: $Param,)*
                ) -> Out {
                    f(world, input, $($Param),*)
                }
                call_inner(self, world, In(input), $(param.$n),*)
            }
        }
    };
}

impl_exclusive_system_function!();
impl_exclusive_system_function!((A 0));
impl_exclusive_system_function!((A 0), (B 1));
impl_exclusive_system_function!((A 0), (B 1), (C 2));
