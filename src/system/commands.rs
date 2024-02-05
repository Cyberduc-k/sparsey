use std::mem::MaybeUninit;
use std::ptr::NonNull;
use std::sync::Exclusive;

use crate::entity::ComponentSet;
use crate::prelude::{Entities, Entity, World};
use crate::resource::Resource;
use crate::world::UnsafeWorldCell;

use super::{Deferred, SystemBuffer, SystemParam, SystemParamKind};

/// A [`World`] mutation.
pub trait Command: Send + 'static {
    /// Applies this command.
    fn apply(self, world: &mut World);
}

/// A [`Command`] queue to perform changes to the [`World`].
pub struct Commands<'w, 's> {
    queue: Deferred<'s, CommandQueue>,
    entities: Entities<'w>,
}

/// Densely and efficiently stores a queue of types implementing [`Command`].
#[derive(Default)]
pub struct CommandQueue {
    bytes: Vec<MaybeUninit<u8>>,
}

impl SystemBuffer for CommandQueue {
    fn apply(&mut self, world: &mut World) {
        self.apply_or_drop_queud(Some(world));
    }
}

impl SystemParam for Commands<'_, '_> {
    const KIND: SystemParamKind = SystemParamKind::Entities;
    const SEND: bool = true;

    type Item<'w, 's> = Commands<'w, 's>;
    type State = Exclusive<CommandQueue>;

    fn init_state(_: &mut World) -> Self::State {
        Default::default()
    }

    unsafe fn borrow<'w, 's>(
        state: &'s mut Self::State,
        world: UnsafeWorldCell<'w>,
    ) -> Self::Item<'w, 's> {
        Commands {
            queue: Deferred::borrow(state, world),
            entities: world.entities().borrow_entities(),
        }
    }
}

impl Commands<'_, '_> {
    /// Creates a new entity without requiring exclusive access to the storage. The entity is not
    /// added to the storage until [`maintain`](Self::maintain) is called.
    ///
    /// Returns the newly created entity.
    pub fn create_atomic(&self) -> Entity {
        self.entities.create_atomic()
    }

    /// Creates a new entity with the given `components`.
    ///
    /// Returns the newly created entity.
    pub fn create<C>(&mut self, components: C) -> Entity
    where
        C: ComponentSet + Send + 'static,
    {
        let entity = self.create_atomic();
        self.insert(entity, components);
        entity
    }

    /// Creates new entities with the components produced by the iterator.
    pub fn extend<C, I>(&mut self, components: I)
    where
        C: ComponentSet + Send + 'static,
        I: IntoIterator<Item = C> + Send + 'static,
    {
        self.queue.push(move |world: &mut World| {
            C::extend(&mut world.entities, components);
        });
    }

    /// Adds the given `components` to `entity` if `entity` is present in the storage.
    pub fn insert<C>(&mut self, entity: Entity, components: C)
    where
        C: ComponentSet + Send + 'static,
    {
        self.queue.push(move |world: &mut World| {
            world.entities.insert(entity, components);
        });
    }

    /// Removes components from the given `entity`.
    pub fn delete<C>(&mut self, entity: Entity)
    where
        C: ComponentSet + Send + 'static,
    {
        self.queue.push(move |world: &mut World| {
            C::delete(&mut world.entities, entity);
        });
    }

    /// Removes the given `entity` and its components from the storage.
    pub fn destroy(&mut self, entity: Entity) {
        self.queue.push(move |world: &mut World| {
            world.entities.destroy(entity);
        });
    }

    /// Insert a resource into the storage.
    pub fn insert_resource<R: Resource>(&mut self, resource: R) {
        self.queue.push(move |world: &mut World| {
            world.resources.insert(resource);
        });
    }

    /// Insert a resource into the storage.
    pub fn remove_resource<R: Resource>(&mut self) {
        self.queue.push(move |world: &mut World| {
            world.resources.remove::<R>();
        });
    }
}

impl<F> Command for F
where
    F: FnOnce(&mut World) + Send + 'static,
{
    fn apply(self, world: &mut World) {
        self(world);
    }
}

unsafe impl Send for CommandQueue {}
unsafe impl Sync for CommandQueue {}

struct CommandMeta {
    consume_command_and_get_size:
        unsafe fn(value: NonNull<u8>, world: &mut Option<&mut World>) -> usize,
}

impl CommandQueue {
    /// Push a [`Command`] onto the queue.
    #[inline]
    pub fn push<C: Command>(&mut self, command: C) {
        #[repr(C, packed)]
        struct Packed<T: Command> {
            meta: CommandMeta,
            command: T,
        }

        let meta = CommandMeta {
            consume_command_and_get_size: |command, world| {
                let command: C = unsafe { command.cast::<C>().as_ptr().read_unaligned() };
                match world {
                    Some(world) => command.apply(world),
                    None => drop(command),
                }
                std::mem::size_of::<C>()
            },
        };

        let old_len = self.bytes.len();
        self.bytes.reserve(std::mem::size_of::<Packed<C>>());
        let ptr = unsafe { self.bytes.as_mut_ptr().add(old_len) };

        unsafe {
            ptr.cast::<Packed<C>>()
                .write_unaligned(Packed { meta, command });
        }

        unsafe {
            self.bytes
                .set_len(old_len + std::mem::size_of::<Packed<C>>());
        }
    }

    /// Take all commands from `other` and append them to `self`, leaving `other` empty.
    pub fn append(&mut self, other: &mut CommandQueue) {
        self.bytes.append(&mut other.bytes);
    }

    #[inline]
    fn apply_or_drop_queud(&mut self, mut world: Option<&mut World>) {
        let bytes_range = self.bytes.as_mut_ptr_range();
        let mut cursor = bytes_range.start;
        unsafe { self.bytes.set_len(0) };

        while cursor < bytes_range.end {
            let meta = unsafe { cursor.cast::<CommandMeta>().read_unaligned() };
            cursor = unsafe { cursor.add(std::mem::size_of::<CommandMeta>()) };
            let cmd = unsafe { NonNull::<u8>::new_unchecked(cursor.cast()) };
            let size = unsafe { (meta.consume_command_and_get_size)(cmd, &mut world) };
            cursor = unsafe { cursor.add(size) };
        }
    }
}

impl Drop for CommandQueue {
    fn drop(&mut self) {
        self.apply_or_drop_queud(None);
    }
}
