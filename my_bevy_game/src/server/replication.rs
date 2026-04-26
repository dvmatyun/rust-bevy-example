//! Server-side replication plumbing.
//!
//! This module is the only piece that actually walks the world to
//! find changes and pack them into bytes. Game logic doesn't know it
//! exists — change detection (`Added<T>`, `Changed<T>`,
//! `RemovedComponents<T>`) does the work.
//!
//! ## How to opt in
//!
//! ```ignore
//! // top-level App:
//! app.add_plugins((
//!     ServerReplicationPlugin,                            // infra (outbox, sets, flush)
//!     ServerReplicatePlugin::<Player>::default(),         // per-type collectors
//!     ServerReplicatePlugin::<Facing>::default(),
//!     ServerReplicatePlugin::<Transform>::default(),
//! ));
//!
//! // game code:
//! commands.spawn((Player, Facing::default(), Transform::default(), Replicate));
//! //                                                                 ^^^^^^^^^
//! //                                            this entity now replicates.
//! ```
//!
//! ## What gets sent
//!
//! Each `PostUpdate` frame, in order:
//! 1. **`Spawn`** for entities that just gained `Replicate`.
//! 2. **`Upsert`** for any `Added<T>` or `Changed<T>` (where T is
//!    registered) on a `Replicate`-tagged entity.
//! 3. **`Remove`** for any `RemovedComponents<T>` while the entity
//!    is still replicate-tagged (= component removed but entity alive).
//! 4. **`Despawn`** for entities that lost `Replicate` (or were
//!    despawned, which removes `Replicate` along with everything else).
//!
//! Optional `ReplicateTo { peers }` on the entity narrows the audience
//! — absent means broadcast.

use bevy::prelude::*;
// 📘 PhantomData<T> is a zero-sized type that "logically holds a T"
// without actually storing one. We need it because Rust requires
// every type parameter on a struct to be used in the struct body.
// `ServerReplicatePlugin<T>` doesn't carry a `T` value at runtime —
// `T` is only used to pick which systems to register at compile time
// — so PhantomData fills that requirement at zero cost.
use std::marker::PhantomData;

use crate::data::{
    Replicate, ReplicateTo, Replicated, ReplicationDelta, ReplicationOutbox,
    ReplicationSet,
};

// ============================================================================
// Top-level plugin (infrastructure)
// ============================================================================

/// Owns the outbox, configures the `PostUpdate` set chain, runs the
/// generic spawn/despawn/flush systems. Add per-type collectors with
/// `ServerReplicatePlugin::<T>`.
pub struct ServerReplicationPlugin;

impl Plugin for ServerReplicationPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ReplicationOutbox::default())
            .configure_sets(
                PostUpdate,
                (
                    ReplicationSet::Reset,
                    ReplicationSet::Spawned,
                    ReplicationSet::Components,
                    ReplicationSet::Despawned,
                    ReplicationSet::Flush,
                )
                    .chain(),
            )
            .add_systems(PostUpdate, reset_outbox.in_set(ReplicationSet::Reset))
            .add_systems(PostUpdate, collect_spawned.in_set(ReplicationSet::Spawned))
            .add_systems(PostUpdate, collect_despawned.in_set(ReplicationSet::Despawned))
            .add_systems(PostUpdate, flush_outbox.in_set(ReplicationSet::Flush));
    }
}

// ============================================================================
// Per-type plugin (one per registered T)
// ============================================================================

/// Registers collectors for a single `Replicated` component type.
pub struct ServerReplicatePlugin<T: Replicated>(PhantomData<T>);

impl<T: Replicated> Default for ServerReplicatePlugin<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: Replicated> Plugin for ServerReplicatePlugin<T> {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (collect_added::<T>, collect_changed::<T>, collect_removed::<T>)
                .in_set(ReplicationSet::Components),
        );
    }
}

// ============================================================================
// Systems
// ============================================================================

fn reset_outbox(mut outbox: ResMut<ReplicationOutbox>) {
    outbox.clear();
}

/// Detect newly-replicated entities (Replicate marker just added).
fn collect_spawned(
    mut outbox: ResMut<ReplicationOutbox>,
    new: Query<(Entity, Option<&ReplicateTo>), Added<Replicate>>,
) {
    for (e, audience) in &new {
        outbox.push(
            ReplicationDelta::Spawn { entity: e.into() },
            audience.map(|a| a.peers.clone()),
        );
    }
}

/// Detect entities that lost the Replicate marker (or were despawned
/// with it). Audience info is gone with the entity, so we broadcast.
fn collect_despawned(
    mut outbox: ResMut<ReplicationOutbox>,
    mut removed: RemovedComponents<Replicate>,
) {
    for e in removed.read() {
        outbox.push(ReplicationDelta::Despawn { entity: e.into() }, None);
    }
}

/// Per-type: pack newly-inserted T components.
///
/// 📘 `fn name<T: Replicated>` is a *generic function*: it's
/// monomorphised at compile time into one copy per type T it's
/// instantiated with. The `Replicated` trait bound restricts T to
/// types that implement that trait (so we can call `T::TYPE_ID`).
fn collect_added<T: Replicated>(
    mut outbox: ResMut<ReplicationOutbox>,
    // 📘 Filter combination: `(With<Replicate>, Added<T>)` means
    // "must have Replicate marker AND T was added since this system
    // last ran". `Added<T>` is a *change-detection filter* — it uses
    // Bevy's per-component change ticks, evaluated *per archetype*
    // for performance.
    q: Query<(Entity, &T, Option<&ReplicateTo>), (With<Replicate>, Added<T>)>,
) {
    for (e, value, audience) in &q {
        let Ok(bytes) = bincode::serde::encode_to_vec(value, bincode::config::standard()) else {
            warn!("replication: failed to encode added {}", std::any::type_name::<T>());
            continue;
        };
        outbox.push(
            ReplicationDelta::Upsert {
                entity: e.into(),
                type_id: T::TYPE_ID,
                bytes,
            },
            audience.map(|a| a.peers.clone()),
        );
    }
}

/// Per-type: pack changed (but not just-added) T components.
/// `Ref<T>` lets us check `is_added`/`is_changed` to dedupe with
/// `collect_added`.
///
/// 📘 `Ref<T>` is a *smart pointer* that wraps a component ref AND
/// the change-tick metadata. It dereferences to `&T` (so
/// `&*value` gives a plain reference) and exposes `is_added()`,
/// `is_changed()`, `last_changed()`. Using `Ref<T>` means we can
/// query EVERY entity with T (not just the changed ones) and decide
/// per-entity what to do, which is exactly the dedupe pattern
/// `collect_added` uses.
fn collect_changed<T: Replicated>(
    mut outbox: ResMut<ReplicationOutbox>,
    q: Query<(Entity, Ref<T>, Option<&ReplicateTo>), With<Replicate>>,
) {
    for (e, value, audience) in &q {
        // 📘 If the component was JUST added (this same frame) we
        // skip — `collect_added` handled it. If it wasn't changed
        // at all, also skip. Net effect: send one Upsert per
        // (entity, T) per frame, regardless of how many systems
        // mutated T.
        if !value.is_changed() || value.is_added() {
            continue;
        }
        let Ok(bytes) = bincode::serde::encode_to_vec(&*value, bincode::config::standard())
        else {
            warn!("replication: failed to encode changed {}", std::any::type_name::<T>());
            continue;
        };
        outbox.push(
            ReplicationDelta::Upsert {
                entity: e.into(),
                type_id: T::TYPE_ID,
                bytes,
            },
            audience.map(|a| a.peers.clone()),
        );
    }
}

/// Per-type: emit `Remove` deltas only when the component was
/// removed but the entity is still replicate-tagged (entity-wide
/// despawns are handled by `collect_despawned`).
fn collect_removed<T: Replicated>(
    mut outbox: ResMut<ReplicationOutbox>,
    mut removed: RemovedComponents<T>,
    still_replicated: Query<(), With<Replicate>>,
) {
    for e in removed.read() {
        if still_replicated.get(e).is_ok() {
            outbox.push(
                ReplicationDelta::Remove {
                    entity: e.into(),
                    type_id: T::TYPE_ID,
                },
                None,
            );
        }
    }
}

/// Stage-1 sink: log a one-line summary at most once per second so
/// you can verify replication is firing without flooding the
/// console. A real transport replaces this body with an enqueue.
fn flush_outbox(
    time: Res<Time>,
    mut throttle: Local<f32>,
    outbox: Res<ReplicationOutbox>,
) {
    if outbox.is_empty() {
        return;
    }
    *throttle += time.delta_secs();
    if *throttle < 1.0 {
        return;
    }
    *throttle = 0.0;

    let mut spawn = 0;
    let mut upsert = 0;
    let mut remove = 0;
    let mut despawn = 0;
    for d in &outbox.deltas {
        match d {
            ReplicationDelta::Spawn { .. } => spawn += 1,
            ReplicationDelta::Upsert { .. } => upsert += 1,
            ReplicationDelta::Remove { .. } => remove += 1,
            ReplicationDelta::Despawn { .. } => despawn += 1,
        }
    }
    //info!(
    //    "[replication] frame: spawn={spawn} upsert={upsert} remove={remove} despawn={despawn}"
    //);
}
