//! State example.

use sparsey::prelude::*;

fn stateful_system(mut counter: Local<usize>) {
    *counter += 1;
    println!("Current counter: {counter}");
}

fn main() {
    let mut world = World::default();
    let mut system = stateful_system.into_system();
    system.initialize(&mut world);

    for _ in 0..3 {
        system.run((), &mut world);
    }
}
