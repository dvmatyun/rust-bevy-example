//! Network replication primitives.
//!
//! ## Philosophy
//!
//! Game-logic code never thinks about networking. To replicate a
//! component, two opt-ins are required and that's it:
//!
//! 1. **The component opts in** by implementing `Replicated` (a
//!    blanket bound: `Component + Serialize + DeserializeOwned`).
//!    There's a stable `TYPE_ID` per replicated type to identify it
//!    on the wire.
//! 2. **The entity opts in** by carrying a `Replicate` marker
//!    component. Without it, no replicated component on that entity
//!    is sent. Camera, HUD nodes, terrain blocks etc. stay
//!    client-local automatically.
//!
//! After that, change detection does the work:
//!
//! - `Added<T>` → `Upsert` delta on first insert
//! - `Changed<T>` (excluding the same-frame Added) → `Upsert` delta
//!   carrying the new value
//! - `RemovedComponents<T>` (entity still alive) → `Remove` delta
//! - `Added<Replicate>` / `RemovedComponents<Replicate>` → `Spawn` /
//!   `Despawn` deltas, surfaced once per entity-lifetime change
//!
//! All deltas land in the `ReplicationOutbox` resource. The flush
//! system at the end of `PostUpdate` drains it (currently to logs;
//! later: to a transport).
//!
//! ## "What and to whom" — opt-in shape
//!
//! - **What**: register `ServerReplicatePlugin::<T>::default()` for
//!   each `T: Replicated` you want sent. Components without a
//!   plugin registration are never sent, even if they're on a
//!   `Replicate`-tagged entity.
//! - **To whom**: by default, every replicate-tagged entity is
//!   broadcast to every peer. Attach `ReplicateTo { peers }` to
//!   restrict to a subset (room semantics).
//!
//! See `docsai/architecture.md` → "Network replication" for the
//! full design rationale.

use bevy::prelude::*;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

/// Components that opt in to replication. Every implementor needs
/// a stable wire id (don't reuse if a type is removed) and serde
/// support so the collector can pack it into bytes.
///
/// `Component` is required for ECS storage; `Serialize +
/// DeserializeOwned` for wire format; nothing else.
pub trait Replicated: Component + Serialize + DeserializeOwned {
    const TYPE_ID: u16;
}

/// Stable per-component wire ids. Adding a new replicated type =
/// add a constant here + `impl Replicated for Foo { const TYPE_ID
/// = type_ids::FOO; }`. Don't reuse retired ids.
pub mod type_ids {
    pub const PLAYER: u16 = 1;
    pub const FACING: u16 = 2;
    pub const TRANSFORM: u16 = 3;
}

/// Mark an entity as eligible for replication. Without this, no
/// replicated component on the entity is sent.
#[derive(Component, Default, Clone, Copy, Debug)]
pub struct Replicate;

/// Stable peer / connection identity. When a real transport lands
/// this is the connection slot; today it's just an opaque integer.
pub type PeerId = u32;

/// Optional per-entity audience. Absent ⇒ broadcast to all peers.
/// Present ⇒ only the listed peers receive deltas for this entity.
#[derive(Component, Clone, Default, Debug)]
pub struct ReplicateTo {
    pub peers: Vec<PeerId>,
}

/// Server-authoritative entity id on the wire — built from
/// `Entity::to_bits()`. The client maintains a
/// `HashMap<ServerEntity, Entity>` to map to local entities.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ServerEntity(pub u64);

impl From<Entity> for ServerEntity {
    fn from(e: Entity) -> Self {
        Self(e.to_bits())
    }
}

/// One unit of replication state-change. The receiver applies these
/// in arrival order; the collector emits them in dependency order
/// (Spawn → Upsert → Remove → Despawn within a frame).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ReplicationDelta {
    Spawn {
        entity: ServerEntity,
    },
    /// Insert-or-update. The receiver doesn't need to distinguish.
    Upsert {
        entity: ServerEntity,
        type_id: u16,
        bytes: Vec<u8>,
    },
    Remove {
        entity: ServerEntity,
        type_id: u16,
    },
    Despawn {
        entity: ServerEntity,
    },
}

/// Per-frame outbound queue. Collectors push into `deltas` and
/// `audiences` (parallel arrays); flush drains both.
#[derive(Resource, Default)]
pub struct ReplicationOutbox {
    pub deltas: Vec<ReplicationDelta>,
    /// Audience for each delta (parallel array). `None` = broadcast.
    pub audiences: Vec<Option<Vec<PeerId>>>,
}

impl ReplicationOutbox {
    pub fn push(&mut self, delta: ReplicationDelta, audience: Option<Vec<PeerId>>) {
        self.deltas.push(delta);
        self.audiences.push(audience);
    }

    pub fn clear(&mut self) {
        self.deltas.clear();
        self.audiences.clear();
    }

    #[allow(dead_code)] // public API surface for future transport
    pub fn len(&self) -> usize {
        self.deltas.len()
    }

    pub fn is_empty(&self) -> bool {
        self.deltas.is_empty()
    }
}

/// Replication runs late in `PostUpdate`. Sub-sets are chained so
/// the receiver gets a coherent ordering:
///
///   Reset → Spawned → Components → Despawned → Flush
///
/// `Spawned` runs first so a freshly spawned entity exists on the
/// receiver before its components arrive. `Despawned` runs after the
/// component collectors so a within-frame "remove component then
/// despawn" doesn't generate a stray `Remove` for a dead entity.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum ReplicationSet {
    Reset,
    Spawned,
    Components,
    Despawned,
    Flush,
}

// ============================================================================
// Replicated impls for our game's components and Bevy built-ins.
// Centralised here so type-id collisions are easy to spot.
// ============================================================================

use super::components::{Facing, Player};

impl Replicated for Player {
    const TYPE_ID: u16 = type_ids::PLAYER;
}

impl Replicated for Facing {
    const TYPE_ID: u16 = type_ids::FACING;
}

impl Replicated for Transform {
    const TYPE_ID: u16 = type_ids::TRANSFORM;
}
