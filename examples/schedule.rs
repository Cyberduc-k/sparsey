//! Schedule systems

use sparsey::prelude::*;
use sparsey::schedule::Update;

#[derive(Clone, Copy, Debug)]
struct Position(i32, i32);

#[derive(Clone, Copy, Debug)]
struct Velocity(i32, i32);

#[derive(Clone, Copy, Debug)]
struct Frozen;

fn update_velocities(mut velocities: CompMut<Velocity>, frozens: Comp<Frozen>) {
    println!("[Update velocities]");

    (&mut velocities)
        .include(&frozens)
        .for_each_with_entity(|(entity, velocity)| {
            println!("{:?} is frozen. Set its velocity to (0, 0)", entity);

            *velocity = Velocity(0, 0);
        });

    println!();
}

fn update_positions(mut positions: CompMut<Position>, velocities: Comp<Velocity>) {
    println!("[Update positions]");

    (&mut positions, &velocities).for_each_with_entity(|(entities, (position, velocity))| {
        position.0 += velocity.0;
        position.1 += velocity.1;

        println!("{:?}, {:?}, {:?}", entities, *position, velocity);
    });

    println!();
}

fn exclusive(world: &mut World) {
    println!("[Exclusive system]");
    println!("{}", world.entities().len());
    println!();
}

fn main() {
    let mut world = World::default();
    world.register::<Position>();
    world.register::<Velocity>();
    world.register::<Frozen>();

    world.create((Position(0, 0), Velocity(1, 1)));
    world.create((Position(0, 0), Velocity(2, 2)));
    world.create((Position(0, 0), Velocity(3, 3), Frozen));

    let mut schedule = Schedule::builder()
        .add_system(Update, exclusive)
        .add_system(Update, update_velocities)
        .add_system(Update, update_positions)
        .build();

    println!("Schedule: {schedule:#?}");
    println!();
    schedule.initialize(&mut world);

    for _ in 0..3 {
        schedule.run(&mut world);
    }
}
