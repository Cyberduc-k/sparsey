//! Run systems conditionally.

use sparsey::prelude::*;

#[derive(Default)]
struct Flag(bool);

fn stateful_system(mut counter: Local<usize>, mut flag: ResMut<Flag>) {
    *counter += 1;
    println!("Current counter: {counter}");
    if *counter == 2 {
        flag.0 = false;
    }
}

fn stateful_condition(flag: Res<Flag>) -> bool {
    flag.0
}

fn main() {
    let mut world = World::default();
    let mut schedule = Schedule::default();
    schedule.add_system(stateful_system.run_if(stateful_condition));
    schedule.initialize(&mut world);
    world.insert_resource(Flag(true));

    for _ in 0..3 {
        schedule.run(&mut world);
    }
}
