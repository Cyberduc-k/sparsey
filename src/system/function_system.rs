use std::borrow::Cow;
use std::marker::PhantomData;

use super::{
    In, IntoSystem, ReadonlySystem, ReadonlySystemParamSet, System, SystemParam, SystemParamKind,
    SystemParamSet,
};
use crate::{EntityStorage, ResourceStorage, World};

pub struct FunctionSystem<Marker, TRegistry, F>
where
    F: SystemParamFunction<Marker, TRegistry>,
{
    func: F,
    param_state: Option<<F::Params as SystemParamSet<TRegistry>>::State>,
    marker: PhantomData<fn(&TRegistry) -> Marker>,
}

#[doc(hidden)]
pub struct IsFunctionSystem;

impl<TRegistry, Marker, F> IntoSystem<TRegistry, F::In, F::Out, (IsFunctionSystem, Marker)> for F
where
    TRegistry: 'static,
    Marker: 'static,
    F: SystemParamFunction<Marker, TRegistry>,
{
    type System = FunctionSystem<Marker, TRegistry, F>;

    fn into_system(self) -> Self::System {
        FunctionSystem {
            func: self,
            param_state: None,
            marker: PhantomData,
        }
    }
}

impl<TRegistry, Marker, F> System<TRegistry> for FunctionSystem<Marker, TRegistry, F>
where
    TRegistry: 'static,
    Marker: 'static,
    F: SystemParamFunction<Marker, TRegistry>,
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

    unsafe fn run_unsafe(&mut self, input: Self::In, registry: &TRegistry) -> Self::Out {
        let state = self
            .param_state
            .as_mut()
            .expect("param_state not initialized");
        let params = F::Params::borrow(state, registry);
        self.func.run(input, params)
    }

    fn apply_deferred(&mut self, registry: &mut TRegistry) {
        let state = self
            .param_state
            .as_mut()
            .expect("param_state not initialized");
        F::Params::apply(state, registry);
    }

    #[inline]
    fn initialize(&mut self, registry: &mut TRegistry) {
        self.param_state = Some(F::Params::init_state(registry));
    }

    #[inline]
    fn is_exclusive(&self) -> bool {
        false
    }
}

unsafe impl<TRegistry, Marker, F> ReadonlySystem<TRegistry> for FunctionSystem<Marker, TRegistry, F>
where
    TRegistry: 'static,
    Marker: 'static,
    F: SystemParamFunction<Marker, TRegistry>,
    F::Params: ReadonlySystemParamSet<TRegistry>,
{
}

pub trait SystemParamFunction<Marker, TRegistry>: Send + Sync + 'static {
    type In;
    type Out;
    type Params: SystemParamSet<TRegistry>;

    fn run(
        &mut self,
        input: Self::In,
        param: <Self::Params as SystemParamSet<TRegistry>>::Item<'_, '_>,
    ) -> Self::Out;
}

macro_rules! impl_system_function {
    ($(($Param:ident $n:tt)),*) => {
        impl_system_function_in!(world: World; $(($Param $n)),*);
        impl_system_function_in!(entities: EntityStorage; $(($Param $n)),*);
        impl_system_function_in!(resources: ResourceStorage; $(($Param $n)),*);
    };
}

macro_rules! impl_system_function_in {
    ($registry:ident: $Registry:ty; $(($Param:ident $n:tt)),*) => {
        #[allow(non_snake_case)]
        impl<Out, Func, $($Param),*> SystemParamFunction<fn($($Param),*) -> Out, $Registry> for Func
        where
            Out: 'static,
            Func: Send + Sync + 'static,
            for<'a> &'a mut Func:
                FnMut($($Param),*) -> Out +
                FnMut($(<$Param as SystemParam<$Registry>>::Item<'_, '_>),*) -> Out,
            $($Param: SystemParam<$Registry>),*
        {
            type In = ();
            type Out = Out;
            type Params = ($($Param,)*);

            #[inline]
            #[allow(unused_variables)]
            fn run(&mut self, _: (), param: <Self::Params as SystemParamSet<$Registry>>::Item<'_, '_>) -> Out {
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
        impl<Input, Out, Func, $($Param),*> SystemParamFunction<fn(In<Input>, $($Param),*) -> Out, $Registry> for Func
        where
            Out: 'static,
            Func: Send + Sync + 'static,
            for<'a> &'a mut Func:
                FnMut(In<Input>, $($Param),*) -> Out +
                FnMut(In<Input>, $(<$Param as SystemParam<$Registry>>::Item<'_, '_>),*) -> Out,
            $($Param: SystemParam<$Registry>),*
        {
            type In = Input;
            type Out = Out;
            type Params = ($($Param,)*);

            #[inline]
            #[allow(unused_variables)]
            fn run(&mut self, input: Input, param: <Self::Params as SystemParamSet<$Registry>>::Item<'_, '_>) -> Out {
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
