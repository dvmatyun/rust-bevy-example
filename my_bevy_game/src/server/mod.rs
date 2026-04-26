//! Layer 2 — server (game logic, source of truth).
//!
//! Allowed: `bevy_ecs`, `bevy_app`, `bevy_math`, `Transform`, `Time`,
//! reading `Res<TerrainHeights>` etc., consuming intents from `data/`.
//!
//! Forbidden: `Mesh3d`, `MeshMaterial3d`, `Camera3d`, `Color`, `Node`,
//! `Text`, `BackgroundColor`, `StandardMaterial`, `Image`, `Window`,
//! direct keyboard/touch input. (Those belong in `client_sim/` or
//! `render/`.)
//!
//! The server can be run headless by adding `MinimalPlugins + DataPlugin
//! + ServerPlugin` — useful for unit tests and (eventually) running as
//! a dedicated server process.

mod player;
pub mod replication;
mod terrain;

use bevy::prelude::*;

use crate::data::{AppSet, Facing, Player};
use replication::{ServerReplicatePlugin, ServerReplicationPlugin};

pub struct ServerPlugin;

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ServerReplicationPlugin,                       // outbox + flush
            ServerReplicatePlugin::<Player>::default(),    // type opt-ins
            ServerReplicatePlugin::<Facing>::default(),
            ServerReplicatePlugin::<Transform>::default(),
        ))
        .add_systems(
            Startup,
            (terrain::setup_terrain, player::spawn_player).chain(),
        )
        .add_systems(
            Update,
            (player::apply_movement, player::snap_to_ground)
                .chain()
                .in_set(AppSet::GameLogic),
        );
    }
}
