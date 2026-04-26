//! Stage 4: change detection.
//!
//! Bevy stores per-component "ticks". The query filters `Added<T>`,
//! `Changed<T>`, and the system param `RemovedComponents<T>` let
//! systems react only to actual changes.

use bevy::prelude::*;

#[derive(Component, Default)]
struct Counter(pub u32);

#[derive(Resource, Default)]
struct Observed {
    added: u32,
    changed: u32,
}

// =====================================================================
// TASK 4a — implement two systems:
//
//   - `count_additions` should add `q.iter().count() as u32` to
//     `obs.added` for every entity newly added with `Counter`.
//   - `count_changes` should add `q.iter().count() as u32` to
//     `obs.changed` for every entity whose `Counter` changed (this
//     includes additions).
//
// HINTS:
//   - Filter form: `Query<(), Added<Counter>>` or
//     `Query<(), Changed<Counter>>`. The `()` says "we don't fetch
//     anything beyond the filter".
//   - Use `q.iter().count()` to get the size each frame.
// =====================================================================

fn count_additions(_q: Query<(), Added<Counter>>, mut _obs: ResMut<Observed>) {
    // TODO
}

fn count_changes(_q: Query<(), Changed<Counter>>, mut _obs: ResMut<Observed>) {
    // TODO
}

#[test]
fn task_4a_change_detection() {
    let mut app = App::new();
    app.init_resource::<Observed>();
    app.add_systems(Update, (count_additions, count_changes));

    // Spawn one entity.
    app.world_mut().spawn(Counter(0));
    app.update();

    // Mutate it (mutate triggers Changed; not Added).
    let entity = app
        .world_mut()
        .query::<Entity>()
        .iter(app.world())
        .next()
        .expect("expected at least one entity");
    app.world_mut()
        .entity_mut(entity)
        .get_mut::<Counter>()
        .expect("entity should have a Counter")
        .0 = 99;
    app.update();

    let obs = app.world().resource::<Observed>();
    // After two updates and one mutation:
    //   added   should be 1 (the spawn was first frame)
    //   changed should be 2 (added is also "changed", plus the mutation)
    assert_eq!(obs.added, 1, "expected 1 addition; got {}", obs.added);
    assert_eq!(obs.changed, 2, "expected 2 changes; got {}", obs.changed);
}

// =====================================================================
// TASK 4b — `Ref<T>` and the difference between `is_added` / `is_changed`.
//
// Replace `Query<(), Changed<Counter>>` with `Query<Ref<Counter>>`
// and inside the loop count only entries where
// `value.is_changed() && !value.is_added()` ("changed but NOT just
// added"). This is the dedupe trick used in
// `src/server/replication.rs::collect_changed`.
//
// Verify the count is 1 (the mutation only) instead of 2.
// =====================================================================
