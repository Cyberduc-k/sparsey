use std::any::Any;
use std::hash::Hash;

/// A strongly-typed class of labels used to identify a [`Schedule`].
pub trait ScheduleLabel: Any {
    /// The name of this label.
    fn name(&self) -> &'static str;
    /// Clone this trait object.
    fn dyn_clone(&self) -> Box<dyn ScheduleLabel>;
}

impl PartialEq for dyn ScheduleLabel {
    fn eq(&self, other: &Self) -> bool {
        self.type_id() == other.type_id()
    }
}

impl Eq for dyn ScheduleLabel {}

impl Hash for dyn ScheduleLabel {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.type_id().hash(state);
    }
}

impl std::fmt::Debug for dyn ScheduleLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.name().fmt(f)
    }
}

/// Used to define a new [`ScheduleLabel`].
#[macro_export]
macro_rules! define_schedule_label {
    ($($(#[$attr:meta])* $vis:vis struct $label_name:ident;)*) => {
        $(
            $(#[$attr])*
            #[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
            $vis struct $label_name;

            impl $crate::schedule::ScheduleLabel for $label_name {
                fn name(&self) -> &'static str {
                    std::any::type_name::<$label_name>()
                }

                fn dyn_clone(&self) -> Box<dyn $crate::schedule::ScheduleLabel> {
                    Box::new(self.clone())
                }
            }
        )*
    };
}

define_schedule_label! {
    /// Identifies systems that should be executed first.
    pub struct First;

    /// Identifies systems that should execute before the main update loop.
    pub struct PreUpdate;

    /// Identifies systems that should be run as part of the main update loop.
    pub struct Update;

    /// Identifies systems that should run after the main update loop.
    pub struct PostUpdate;

    /// Identifies systems that should run last.
    pub struct Last;
}
