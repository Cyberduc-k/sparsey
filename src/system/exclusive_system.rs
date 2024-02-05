use std::borrow::Cow;
use std::marker::PhantomData;

use super::exclusive_param::{ExclusiveSystemParam, ExclusiveSystemParamSet};
use super::{In, IntoSystem, System};
use crate::{EntityStorage, ResourceStorage, World};

pub struct ExclusiveSystem<Marker, TRegistry, F>
where
    F: ExclusiveSystemParamFunction<Marker, TRegistry>,
{
    func: F,
    param_state: Option<<F::Params as ExclusiveSystemParamSet<TRegistry>>::State>,
    marker: PhantomData<fn(&mut TRegistry) -> Marker>,
}

#[doc(hidden)]
pub struct IsExclusiveSystem;

impl<TRegistry, Marker, F> IntoSystem<TRegistry, F::In, F::Out, (IsExclusiveSystem, Marker)> for F
where
    TRegistry: 'static,
    Marker: 'static,
    F: ExclusiveSystemParamFunction<Marker, TRegistry>,
{
    type System = ExclusiveSystem<Marker, TRegistry, F>;

    fn into_system(self) -> Self::System {
        ExclusiveSystem {
            func: self,
            param_state: None,
            marker: PhantomData,
        }
    }
}

impl<TRegistry, Marker, F> System<TRegistry> for ExclusiveSystem<Marker, TRegistry, F>
where
    TRegistry: 'static,
    Marker: 'static,
    F: ExclusiveSystemParamFunction<Marker, TRegistry>,
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
    unsafe fn run_unsafe(&mut self, _: Self::In, _: &TRegistry) -> Self::Out {
        panic!("cannot run exclusive systems with a shared registry");
    }

    fn run(&mut self, input: Self::In, registry: &mut TRegistry) -> Self::Out {
        let state = self
            .param_state
            .as_mut()
            .expect("param_state not initialized");
        let params = F::Params::borrow(state);
        self.func.run(registry, input, params)
    }

    #[inline]
    fn apply_deferred(&mut self, _: &mut TRegistry) {}

    #[inline]
    fn initialize(&mut self, registry: &mut TRegistry) {
        self.param_state = Some(F::Params::init_state(registry));
    }

    #[inline]
    fn is_exclusive(&self) -> bool {
        true
    }
}

pub trait ExclusiveSystemParamFunction<Marker, TRegistry>: Send + Sync + 'static {
    type In;
    type Out;
    type Params: ExclusiveSystemParamSet<TRegistry>;

    fn run(
        &mut self,
        registry: &mut TRegistry,
        input: Self::In,
        param: <Self::Params as ExclusiveSystemParamSet<TRegistry>>::Item<'_>,
    ) -> Self::Out;
}

macro_rules! impl_exclusive_system_function {
    ($(($Param:ident $n:tt)),*) => {
        impl_exclusive_system_function_in!(world: World; $(($Param $n)),*);
        impl_exclusive_system_function_in!(entities: EntityStorage; $(($Param $n)),*);
        impl_exclusive_system_function_in!(resources: ResourceStorage; $(($Param $n)),*);
    };
}

macro_rules! impl_exclusive_system_function_in {
    ($registry:ident: $Registry:ty; $(($Param:ident $n:tt)),*) => {
        #[allow(non_snake_case)]
        impl<Out, Func, $($Param),*> ExclusiveSystemParamFunction<fn($($Param),*) -> Out, $Registry> for Func
        where
            Out: 'static,
            Func: Send + Sync + 'static,
            for<'a> &'a mut Func:
                FnMut(&mut $Registry, $($Param),*) -> Out +
                FnMut(&mut $Registry, $(<$Param as ExclusiveSystemParam<$Registry>>::Item<'_>),*) -> Out,
            $($Param: ExclusiveSystemParam<$Registry>),*
        {
            type In = ();
            type Out = Out;
            type Params = ($($Param,)*);

            #[inline]
            #[allow(unused_variables)]
            fn run(&mut self, $registry: &mut $Registry, _: (), param: <Self::Params as ExclusiveSystemParamSet<$Registry>>::Item<'_>) -> Out {
                #[allow(clippy::too_many_arguments)]
                fn call_inner<Out, $($Param),*>(
                    mut f: impl FnMut(&mut $Registry, $($Param),*) -> Out,
                    $registry: &mut $Registry,
                    $($Param: $Param,)*
                ) -> Out {
                    f($registry, $($Param,)*)
                }
                call_inner(self, $registry, $(param.$n),*)
            }
        }

        #[allow(non_snake_case)]
        impl<Input, Out, Func, $($Param),*> ExclusiveSystemParamFunction<fn(In<Input>, $($Param),*) -> Out, $Registry> for Func
        where
            Out: 'static,
            Func: Send + Sync + 'static,
            for<'a> &'a mut Func:
                FnMut(&mut $Registry, In<Input>, $($Param),*) -> Out +
                FnMut(&mut $Registry, In<Input>, $(<$Param as ExclusiveSystemParam<$Registry>>::Item<'_>),*) -> Out,
            $($Param: ExclusiveSystemParam<$Registry>),*
        {
            type In = Input;
            type Out = Out;
            type Params = ($($Param,)*);

            #[inline]
            #[allow(unused_variables)]
            fn run(&mut self, $registry: &mut $Registry, input: Input, param: <Self::Params as ExclusiveSystemParamSet<$Registry>>::Item<'_>) -> Out {
                #[allow(clippy::too_many_arguments)]
                fn call_inner<Input, Out, $($Param),*>(
                    mut f: impl FnMut(&mut $Registry, In<Input>, $($Param),*) -> Out,
                    $registry: &mut $Registry,
                    input: In<Input>,
                    $($Param: $Param,)*
                ) -> Out {
                    f($registry, input, $($Param),*)
                }
                call_inner(self, $registry, In(input), $(param.$n),*)
            }
        }
    };
}

impl_exclusive_system_function!();
impl_exclusive_system_function!((A 0));
impl_exclusive_system_function!((A 0), (B 1));
impl_exclusive_system_function!((A 0), (B 1), (C 2));
