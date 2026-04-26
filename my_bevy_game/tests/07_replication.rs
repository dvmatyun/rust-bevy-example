//! Stage 7: opt-in replication.
//!
//! By the end of this file you'll have:
//!   - declared a custom `Replicated` component,
//!   - tagged an entity with `Replicate`,
//!   - watched a `ReplicationDelta` appear in the outbox.
//!
//! All without writing any networking code — the replication
//! infrastructure does the work for you.

use bevy::prelude::*;
use my_bevy_game::data::{
    DataPlugin, Replicate, ReplicationDelta, ReplicationOutbox,
};
use my_bevy_game::server::replication::{
    ServerReplicatePlugin, ServerReplicationPlugin,
};
use serde::{Deserialize, Serialize};

// =====================================================================
// TASK 7a — define a Replicated component.
//
// 1. Define `Score(pub u32)` with the right derives. You need
//    `Component` (so it can attach to an entity), `Serialize` and
//    `Deserialize` (so it can be packed for the wire), `Clone`,
//    `Copy`, `Debug`.
//
//      #[derive(Component, Serialize, Deserialize, Clone, Copy, Debug)]
//      pub struct Score(pub u32);
//
// 2. Implement `Replicated` for it. Pick a `TYPE_ID` that doesn't
//    collide with the built-in ones in `data/replication.rs::type_ids`
//    (PLAYER=1, FACING=2, TRANSFORM=3). Use 1000 to be safe.
//
//      use my_bevy_game::data::Replicated;
//      impl Replicated for Score { const TYPE_ID: u16 = 1000; }
//
// 3. Register `ServerReplicatePlugin::<Score>::default()` in the test.
// =====================================================================

// TODO: define Score + impl Replicated.

#[test]
fn task_7a_score_replication() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        DataPlugin,
        ServerReplicationPlugin,
        // ServerReplicatePlugin::<Score>::default(),
    ));

    // Spawn a replicated entity with a Score.
    // app.world_mut().spawn((Replicate, Score(42)));

    // First Update: spawn applies via Commands.
    app.update();
    // Second Update: PostUpdate runs replication collectors.
    app.update();

    // Inspect the outbox. We expect at least one Spawn delta and one
    // Upsert delta for Score.
    // let outbox = app.world().resource::<ReplicationOutbox>();
    // let has_spawn = outbox.deltas.iter().any(|d| matches!(d, ReplicationDelta::Spawn { .. }));
    // let has_upsert = outbox.deltas.iter().any(|d|
    //     matches!(d, ReplicationDelta::Upsert { type_id: 1000, .. })
    // );
    // assert!(has_spawn, "expected a Spawn delta in the outbox");
    // assert!(has_upsert, "expected an Upsert<Score> delta in the outbox");

    let _ = (Replicate,); // dummy reference so the import is used
}

// =====================================================================
// TASK 7b — verify mutation produces an Upsert.
//
// 1. After the spawn, mutate the Score on the entity (set to 99).
// 2. Run another tick.
// 3. Check the outbox now contains an Upsert delta carrying the new
//    value (decode the bytes with bincode + serde to confirm).
//
// HINTS:
//   - `bincode::serde::decode_from_slice::<Score, _>(&bytes,
//     bincode::config::standard())` to decode.
//   - The outbox is **cleared every frame** (see `reset_outbox`).
//     Inspect it AFTER the tick that mutated, but BEFORE another tick.
// =====================================================================

// =====================================================================
// TASK 7c — entity without `Replicate` does NOT replicate.
//
// Spawn an entity with `Score(7)` but WITHOUT `Replicate`. Run a tick.
// Check the outbox is empty for that entity (no spawn, no upsert).
// This is the "two opt-ins" rule: component-type AND entity-marker
// are both required.
// =====================================================================
