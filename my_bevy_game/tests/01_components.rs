//! Stage 1: components and entities.
//!
//! Goal of this file: introduce the Bevy ECS triangle —
//! World ↔ Entity ↔ Component.
//!
//! Run all tasks: `cargo test --test 01_components`
//! Run one task:  `cargo test --test 01_components -- task_1b`

use bevy::prelude::*;

// =====================================================================
// TASK 1a — define a component.
//
// Fill in the body of `Health` so that it stores a single `i32`
// representing current hit points. Hint: see `Facing(pub Vec2)` in
// `src/data/components.rs` for the tuple-struct pattern.
//
// Replace the placeholder below with:
//
//   #[derive(Component)]
//   pub struct Health(pub i32);
//
// Don't forget the `#[derive(Component)]` — Bevy needs it to know
// this type can be attached to entities.
// =====================================================================

// PLACEHOLDER — replace with your real Health.
#[derive(Component, Clone, Copy)]
pub struct Health(pub i32);

// =====================================================================
// TASK 1b — spawn entities and read them back.
//
// Below is a setup system that should spawn 3 entities with `Health`
// components: 10, 50, 100. Then `task_1b` runs the App once and
// asserts that the `HighestHp` resource contains 100.
//
// Tasks:
//   1. Implement `spawn_three_units` to spawn three entities, each
//      with a Health component (values 10, 50, 100).
//   2. Implement `count_units` to query &Health, find the max value,
//      and write it into `hp.0`.
//
// HINTS:
//   - Spawn with `commands.spawn(Health(10));`
//   - `Query<&Health>` iterates with `for h in &q { ... }` (where
//     `h: &Health`) — accessing the inner i32 is `h.0`.
//   - Use `q.iter().map(|h| h.0).max().unwrap_or(0)` to get the highest.
//
// READ:
//   - https://bevyengine.org/learn/quick-start/getting-started/ecs/
//   - The Rust Book chapter 4 (ownership) — explains `&` references.
// =====================================================================

#[derive(Resource, Default)]
struct HighestHp(i32);

fn spawn_three_units(_commands: Commands) {
    // TODO: spawn three entities with Health(10), Health(50), Health(100).
}

fn count_units(_q: Query<&Health>, _hp: ResMut<HighestHp>) {
    // TODO: query &Health, find the max value, write it to hp.0.
    // (Without this implementation, hp.0 stays at 0 and the test below
    // fails with a clear assertion error.)
}

#[test]
fn task_1b_highest_hp() {
    let mut app = App::new();
    app.insert_resource(HighestHp::default());
    app.add_systems(Startup, spawn_three_units);
    app.add_systems(Update, count_units);
    app.update(); // run Startup + one Update tick
    app.update(); // a second tick to be safe (Startup → Update settle)

    let highest = app.world().resource::<HighestHp>().0;
    assert_eq!(
        highest, 100,
        "expected the 100-HP unit to win; got {highest}. \
         Did you spawn three entities AND find the max?"
    );
}
