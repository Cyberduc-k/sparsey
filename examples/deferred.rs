//! Deferred example.

use sparsey::prelude::*;
use sparsey::system::SystemBuffer;

#[derive(Default)]
struct Alarm(bool);

#[derive(Default)]
struct AlarmFlag(bool);

impl AlarmFlag {
    pub fn flag(&mut self) {
        self.0 = true;
    }
}

impl SystemBuffer for AlarmFlag {
    type Registry = World;

    fn apply(&mut self, world: &mut World) {
        if self.0 {
            world.resources.borrow_mut::<Alarm>().0 = true;
            self.0 = true;
        }
    }
}

fn deferred_system(mut counter: Local<usize>, mut alarm: Deferred<AlarmFlag>) {
    *counter += 1;
    if *counter == 3 {
        alarm.flag();
    }
}

fn main() {
    let mut world = World::default();
    let mut system = deferred_system.into_system();
    system.initialize(&mut world);
    world.resources.insert(Alarm::default());

    for _ in 0..3 {
        system.run((), &mut world);
    }

    system.apply_deferred(&mut world);
}
