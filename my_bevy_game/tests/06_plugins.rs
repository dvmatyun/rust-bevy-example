//! Stage 6: plugins.
//!
//! A `Plugin` is anything implementing `fn build(&self, app: &mut App)`.
//! Plugins are how a feature registers all its systems / resources /
//! messages in one place. They compose: `app.add_plugins((P1, P2, P3))`.
//!
//! This stage is also your first taste of *generics*: you'll write a
//! plugin parameterised by a component type.

use bevy::prelude::*;
use std::marker::PhantomData;

// =====================================================================
// TASK 6a — write a non-generic plugin.
//
// 1. Define `Counter(pub u32)` as a Resource (with `Default`).
// 2. Define `CounterPlugin` (a unit struct: `pub struct CounterPlugin;`).
// 3. Implement `Plugin` for it: in `build`, init `Counter` and add a
//    system that increments it each Update.
//
// HINTS:
//   - Plugin trait:
//
//       impl Plugin for CounterPlugin {
//           fn build(&self, app: &mut App) {
//               app.init_resource::<Counter>()
//                  .add_systems(Update, tick_counter);
//           }
//       }
// =====================================================================

#[derive(Resource, Default)]
pub struct Counter(pub u32);

fn tick_counter(_c: ResMut<Counter>) {
    // TODO: increment
}

// TODO: define `CounterPlugin` and `impl Plugin` for it.

#[test]
fn task_6a_plugin_increments() {
    let mut app = App::new();
    // app.add_plugins(CounterPlugin);
    app.update();
    app.update();
    app.update();
    // let n = app.world().resource::<Counter>().0;
    // assert_eq!(n, 3);
}

// =====================================================================
// TASK 6b — write a *generic* plugin.
//
// The replication system in this codebase has plugins like
//
//   pub struct ServerReplicatePlugin<T: Replicated>(PhantomData<T>);
//
// Generic plugins are the cleanest way to "register one set of systems
// per component type". Build a tiny analog:
//
// 1. Define a marker trait `Tracked: Component`.
// 2. Define `TrackerPlugin<T: Tracked>(PhantomData<T>)`.
// 3. In `impl<T: Tracked> Plugin for TrackerPlugin<T>`, register a
//    system that counts how many `Added<T>` entities appear, storing
//    the count in `TrackedCount<T>`.
//
// HINTS:
//   - The PhantomData is necessary because Rust requires the type
//     parameter to appear in the struct body. PhantomData<T> is a
//     zero-sized "I logically reference T".
//   - `Default for TrackerPlugin<T>` needs to be implemented manually
//     (or use `#[derive(Default)]` — note that derive auto-imposes
//     `T: Default` which we don't want; manual impl is cleaner).
//
//       impl<T: Tracked> Default for TrackerPlugin<T> {
//           fn default() -> Self { Self(PhantomData) }
//       }
//
// READ: `src/server/replication.rs` — `ServerReplicatePlugin<T>` is
// the production version of this exercise.
// =====================================================================

#[derive(Resource)]
pub struct TrackedCount<T> {
    pub count: u32,
    _marker: PhantomData<T>,
}

impl<T> Default for TrackedCount<T> {
    fn default() -> Self {
        Self {
            count: 0,
            _marker: PhantomData,
        }
    }
}

#[test]
fn task_6b_generic_plugin() {
    // TODO once you've written TrackerPlugin:
    //
    // #[derive(Component)]
    // struct Foo;
    // impl Tracked for Foo {}
    //
    // let mut app = App::new();
    // app.add_plugins(TrackerPlugin::<Foo>::default());
    // app.world_mut().spawn(Foo);
    // app.world_mut().spawn(Foo);
    // app.update();
    // let count = app.world().resource::<TrackedCount<Foo>>().count;
    // assert_eq!(count, 2);
}
