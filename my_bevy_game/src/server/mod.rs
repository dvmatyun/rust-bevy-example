//! Layer 2 — server (game logic, source of truth).
//!
//! Allowed: `bevy_ecs`, `bevy_app`, `bevy_math`, `Transform`, `Time`,
//! reading shared resources, consuming intents from `data/`.
//!
//! Forbidden: render types (`Mesh3d`, `Camera3d`, `Color`, `Node`,
//! `BackgroundColor`, `StandardMaterial`, `Image`, `Window`); direct
//! keyboard/touch input.

mod buildings;
mod collisions;
mod monsters;
pub(crate) mod net_sim;
mod player;
pub mod replication;
mod terrain;

use bevy::prelude::*;

use crate::data::{AppSet, Facing, GameEntity, GameState, Player};
use net_sim::NetSimPlugin;
use replication::{ServerReplicatePlugin, ServerReplicationPlugin};

pub struct ServerPlugin;

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ServerReplicationPlugin,
            ServerReplicatePlugin::<Player>::default(),
            ServerReplicatePlugin::<Facing>::default(),
            ServerReplicatePlugin::<Transform>::default(),
            NetSimPlugin,
        ))
        // ── Single-player game session ────────────────────────────────────
        .add_systems(
            OnEnter(GameState::PlayingSingle),
            (
                terrain::setup_terrain,
                player::spawn_player_single,
                buildings::spawn_demo_buildings,
            )
                .chain(),
        )
        .add_systems(OnExit(GameState::PlayingSingle), cleanup_game_entities)
        // ── Multiplayer simulation session ────────────────────────────────
        .add_systems(
            OnEnter(GameState::PlayingMultiplayer),
            (
                terrain::setup_terrain,
                player::spawn_player_multiplayer,
                buildings::spawn_demo_buildings,
            )
                .chain(),
        )
        .add_systems(OnExit(GameState::PlayingMultiplayer), cleanup_game_entities)
        // ── Game logic (both playing states) ─────────────────────────────
        .add_systems(
            Update,
            (
                player::apply_movement,
                player::apply_teleport,
                monsters::apply_spawn_monsters,
                monsters::apply_despawn_all,
                monsters::wander_monsters,
                collisions::resolve_player_wall_collisions,
                collisions::resolve_monster_wall_collisions,
                collisions::resolve_body_body_collisions,
                player::snap_to_ground,
            )
                .chain()
                .in_set(AppSet::GameLogic)
                .run_if(
                    in_state(GameState::PlayingSingle)
                        .or_else(in_state(GameState::PlayingMultiplayer)),
                ),
        );
    }
}

fn cleanup_game_entities(mut commands: Commands, q: Query<Entity, With<GameEntity>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}
