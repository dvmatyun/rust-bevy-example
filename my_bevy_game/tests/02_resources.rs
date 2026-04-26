//! Stage 2: resources.
//!
//! A resource is a singleton — exactly one per `App`. Use it for
//! shared state that doesn't belong to any entity (game timer, RNG,
//! settings, …).

use bevy::prelude::*;

// =====================================================================
// TASK 2a — define a resource.
//
// Replace the placeholder body if you like, but this one is OK as-is:
//
//   #[derive(Resource, Default)]
//   pub struct Score(pub u32);
//
// (`Default` lets you write `Score::default()` and lets Bevy
// auto-init the resource via `init_resource::<Score>()`.)
// =====================================================================

#[derive(Resource, Default)]
pub struct Score(pub u32);

// =====================================================================
// TASK 2b — increment the resource each frame.
//
// Implement `tick_score` so that each Update advances `Score` by 1.
//
// HINTS:
//   - System param: `mut score: ResMut<Score>` (mutable resource access).
//   - Increment with `score.0 += 1;`.
// =====================================================================

fn tick_score(mut _score: ResMut<Score>) {
    // TODO: increment _score.0
}

#[test]
fn task_2b_score_advances() {
    let mut app = App::new();
    app.init_resource::<Score>();
    app.add_systems(Update, tick_score);
    app.update();
    app.update();
    app.update();
    let s = app.world().resource::<Score>().0;
    assert_eq!(s, 3, "expected 3 ticks; got {s}. Did you increment?");
}

// =====================================================================
// TASK 2c — read a resource and write a NEW one.
//
// 1. Define `ScoreSquared(pub u32)` (with `Resource, Default`).
// 2. Write a system `square_score(score: Res<Score>, mut sq:
//    ResMut<ScoreSquared>)` that sets `sq.0 = score.0 * score.0`.
// 3. Wire both `tick_score` and `square_score` into the test below.
// 4. Run enough ticks for both to fire and assert the value.
//
// HINTS:
//   - `Res<T>` is a shared (read-only) borrow.
//   - `ResMut<T>` is exclusive — Bevy ensures only one system at a time
//     mutates it.
// =====================================================================

#[test]
fn task_2c_square_score() {
    // TODO: implement and wire.
}
