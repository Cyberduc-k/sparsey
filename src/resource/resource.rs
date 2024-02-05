use std::any::Any;

/// Trait implemented by all types that can be stored in
/// [`ResourceStorage`](crate::resource::ResourceStorage).
pub trait Resource: NonSendResource + Send + Sync {}

/// Trait implemented by all types that can be stored in
/// [`NonSendResourceStorage`](crate::resource::NonSendResourceStorage).
pub trait NonSendResource: 'static {
    /// Upcasts `self`.
    #[must_use]
    fn into_any(self: Box<Self>) -> Box<dyn Any>;

    /// Upcasts `self`.
    #[must_use]
    fn as_any(&self) -> &dyn Any;

    /// Upcasts `self`.
    #[must_use]
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T> Resource for T where T: NonSendResource + Send + Sync {}

impl<T> NonSendResource for T
where
    T: 'static,
{
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl dyn Resource {
    /// Returns whether the resource is of type `T`.
    #[must_use]
    pub fn is<T>(&self) -> bool
    where
        T: Resource,
    {
        self.as_any().is::<T>()
    }

    /// Tries to downcast `self` to a resource of type `T`.
    pub fn downcast<T>(self: Box<Self>) -> Result<Box<T>, Box<Self>>
    where
        T: Resource,
    {
        if self.is::<T>() {
            Ok(self.into_any().downcast().unwrap())
        } else {
            Err(self)
        }
    }

    /// Tries to downcast `self` to a resource of type `T`.
    #[must_use]
    pub fn downcast_ref<T>(&self) -> Option<&T>
    where
        T: Resource,
    {
        self.as_any().downcast_ref()
    }

    /// Tries to downcast `self` to a resource of type `T`.
    #[must_use]
    pub fn downcast_mut<T>(&mut self) -> Option<&mut T>
    where
        T: Resource,
    {
        self.as_any_mut().downcast_mut()
    }
}

impl dyn NonSendResource {
    /// Returns whether the resource is of type `T`.
    #[must_use]
    pub fn is<T>(&self) -> bool
    where
        T: NonSendResource,
    {
        self.as_any().is::<T>()
    }

    /// Tries to downcast `self` to a resource of type `T`.
    pub fn downcast<T>(self: Box<Self>) -> Result<Box<T>, Box<Self>>
    where
        T: NonSendResource,
    {
        if self.is::<T>() {
            Ok(self.into_any().downcast().unwrap())
        } else {
            Err(self)
        }
    }

    /// Tries to downcast `self` to a resource of type `T`.
    #[must_use]
    pub fn downcast_ref<T>(&self) -> Option<&T>
    where
        T: NonSendResource,
    {
        self.as_any().downcast_ref()
    }

    /// Tries to downcast `self` to a resource of type `T`.
    #[must_use]
    pub fn downcast_mut<T>(&mut self) -> Option<&mut T>
    where
        T: NonSendResource,
    {
        self.as_any_mut().downcast_mut()
    }
}
