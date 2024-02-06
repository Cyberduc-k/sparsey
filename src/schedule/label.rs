use __labels::intern::Interned;

#[doc(hidden)]
pub use labels as __labels;

labels::define_label! {
    /// "A strongly-typed class of labels used to identify a [`Schedule`].
    ScheduleLabel
}

/// An [`Interned`] version of [`ScheduleLabel`].
pub type InternedScheduleLabel = Interned<dyn ScheduleLabel>;

/// Used to define a new [`ScheduleLabel`].
#[macro_export]
macro_rules! define_schedule_label {
    ($(#[$attr:meta])* $vis:vis $label_name:ident $(;)?) => {
        $(#[$attr])*
        #[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
        $vis struct $label_name;

        impl $crate::schedule::ScheduleLabel for $label_name {
            fn as_dyn_eq(&self) -> &dyn $crate::schedule::label::__labels::DynEq {
                self
            }

            fn as_dyn_hash(&self) -> &dyn $crate::schedule::label::__labels::DynHash {
                self
            }

            fn dyn_clone(&self) -> Box<dyn $crate::schedule::ScheduleLabel> {
                Box::new(self.clone())
            }
        }
    };
}
