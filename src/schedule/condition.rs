use crate::prelude::IntoSystem;
use crate::system::ReadonlySystem;

pub type BoxedCondition<TRegistry, In = ()> =
    Box<dyn ReadonlySystem<TRegistry, In = In, Out = bool>>;

pub trait Condition<TRegistry, Marker, In = ()>:
    IntoSystem<TRegistry, In, bool, Marker, System = Self::ReadonlySystem>
{
    type ReadonlySystem: ReadonlySystem<TRegistry, In = In, Out = bool>;
}

impl<TRegistry, Marker, In, F> Condition<TRegistry, Marker, In> for F
where
    F: IntoSystem<TRegistry, In, bool, Marker>,
    F::System: ReadonlySystem<TRegistry>,
{
    type ReadonlySystem = F::System;
}
