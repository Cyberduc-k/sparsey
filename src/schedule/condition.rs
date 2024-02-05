use crate::prelude::IntoSystem;
use crate::system::ReadonlySystem;

pub type BoxedCondition<In = ()> = Box<dyn ReadonlySystem<In = In, Out = bool>>;

pub trait Condition<Marker, In = ()>:
    IntoSystem<In, bool, Marker, System = Self::ReadonlySystem>
{
    type ReadonlySystem: ReadonlySystem<In = In, Out = bool>;
}

impl<Marker, In, F> Condition<Marker, In> for F
where
    F: IntoSystem<In, bool, Marker>,
    F::System: ReadonlySystem,
{
    type ReadonlySystem = F::System;
}
