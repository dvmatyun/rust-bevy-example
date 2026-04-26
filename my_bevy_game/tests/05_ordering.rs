//! Stage 5: system ordering.
//!
//! When two systems write the same data, the order matters. Bevy lets
//! you express order with `.before()`, `.after()`, `.chain()`, and
//! `SystemSet` labels.

use bevy::prelude::*;

#[derive(Resource, Default)]
struct Log(pub Vec<&'static str>);

// =====================================================================
// TASK 5a — force "alpha runs before beta".
//
// The two systems below append their name to the log. By default the
// scheduler is free to run them in either order (or in parallel) since
// they touch no overlapping data. Force a deterministic order so the
// test passes.
//
// HINTS:
//   - `app.add_systems(Update, (alpha, beta).chain());` runs them in
//     listed order.
//   - `app.add_systems(Update, alpha.before(beta));` is equivalent for
//     two systems.
// =====================================================================

fn alpha(mut log: ResMut<Log>) {
    log.0.push("alpha");
}

fn beta(mut log: ResMut<Log>) {
    log.0.push("beta");
}

#[test]
fn task_5a_alpha_before_beta() {
    let mut app = App::new();
    app.init_resource::<Log>();
    // TODO: register alpha & beta with explicit ordering.
    app.update();
    let log = app.world().resource::<Log>();
    // assert_eq!(log.0, vec!["alpha", "beta"]);
    let _ = log;
}

// =====================================================================
// TASK 5b — define a SystemSet that groups systems.
//
// 1. Define an enum `MySet { Collect, Apply }` with the right derives:
//
//      #[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
//      enum MySet { Collect, Apply }
//
// 2. Configure `(MySet::Collect, MySet::Apply).chain()` in `Update`.
// 3. Put `gather` in `Collect` and `commit` in `Apply` via `.in_set(...)`.
// 4. Verify the log order.
//
// READ: `src/data/mod.rs` — search for `AppSet`. Same pattern.
// =====================================================================

fn gather(mut log: ResMut<Log>) {
    log.0.push("gather");
}

fn commit(mut log: ResMut<Log>) {
    log.0.push("commit");
}

#[test]
fn task_5b_set_chain() {
    let mut app = App::new();
    app.init_resource::<Log>();
    // TODO
    app.update();
    let log = app.world().resource::<Log>();
    // assert_eq!(log.0, vec!["gather", "commit"]);
    let _ = log;
}
