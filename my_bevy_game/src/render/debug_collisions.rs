//! Debug visualisation of collision shapes as actual 3D mesh objects.
//!
//! Toggled by `Settings.debug_collisions` ("Show colliders" in the Physics
//! section). Each shape is a semi-transparent, unlit mesh:
//!
//!   - Cyan cylinder spawned as a child of the player entity.
//!   - Magenta cylinders spawned as children of each monster entity.
//!   - Yellow boxes spawned as world-space standalone entities, one per
//!     `Building::wall_segments()` entry (buildings are static).
//!
//! All debug meshes carry the `DebugCollider` marker. A single system
//! (`toggle_debug_collider_visibility`) flips their `Visibility` whenever
//! `Settings.debug_collisions` changes.

use bevy::prelude::*;

use crate::data::{Building, Monster, Player, Settings, WorldConfig};

const PLAYER_COLOR: Color = Color::srgba(0.20, 0.95, 1.0, 0.35);
const MONSTER_COLOR: Color = Color::srgba(1.0, 0.30, 0.85, 0.35);
const WALL_COLOR: Color = Color::srgba(1.0, 0.92, 0.20, 0.22);
const MONSTER_HALF_HEIGHT: f32 = 0.5;

/// Marks every entity that exists only as a collision-shape visualiser.
#[derive(Component)]
pub(crate) struct DebugCollider;

/// Shared materials for the three shape types — created once at startup.
#[derive(Resource)]
pub(crate) struct DebugColliderMats {
    player: Handle<StandardMaterial>,
    monster: Handle<StandardMaterial>,
    wall: Handle<StandardMaterial>,
}

// ── Startup ────────────────────────────────────────────────────────────────

pub(crate) fn register_debug_collider_mats(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mk = |color: Color| StandardMaterial {
        base_color: color,
        alpha_mode: AlphaMode::Blend,
        cull_mode: None, // show inside of shapes when camera enters them
        unlit: true,
        ..default()
    };
    commands.insert_resource(DebugColliderMats {
        player: materials.add(mk(PLAYER_COLOR)),
        monster: materials.add(mk(MONSTER_COLOR)),
        wall: materials.add(mk(WALL_COLOR)),
    });
}

// ── Per-entity attach systems ───────────────────────────────────────────────

pub(crate) fn attach_player_debug_collider(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    added: Query<Entity, Added<Player>>,
    settings: Res<Settings>,
    config: Res<WorldConfig>,
    mats: Res<DebugColliderMats>,
) {
    for entity in &added {
        let mesh = meshes.add(Cylinder {
            radius: settings.player_radius,
            half_height: config.player_half_height,
        });
        commands.entity(entity).with_children(|cb| {
            cb.spawn((
                DebugCollider,
                Mesh3d(mesh),
                MeshMaterial3d(mats.player.clone()),
                Transform::default(),
                debug_vis(&settings),
            ));
        });
    }
}

pub(crate) fn attach_monster_debug_collider(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    added: Query<Entity, Added<Monster>>,
    settings: Res<Settings>,
    mats: Res<DebugColliderMats>,
) {
    for entity in &added {
        let mesh = meshes.add(Cylinder {
            radius: settings.monster_radius,
            half_height: MONSTER_HALF_HEIGHT,
        });
        commands.entity(entity).with_children(|cb| {
            cb.spawn((
                DebugCollider,
                Mesh3d(mesh),
                MeshMaterial3d(mats.monster.clone()),
                Transform::default(),
                debug_vis(&settings),
            ));
        });
    }
}

/// Walls are world-space standalone entities (buildings are never transformed).
pub(crate) fn attach_building_debug_walls(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    added: Query<(Entity, &Building), Added<Building>>,
    settings: Res<Settings>,
    mats: Res<DebugColliderMats>,
) {
    for (_entity, building) in &added {
        let vis = debug_vis(&settings);
        for wall in building.wall_segments() {
            let mesh = meshes.add(Cuboid {
                half_size: wall.size * 0.5,
            });
            commands.spawn((
                DebugCollider,
                Mesh3d(mesh),
                MeshMaterial3d(mats.wall.clone()),
                Transform::from_translation(wall.center),
                vis,
            ));
        }
    }
}

// ── Visibility toggle ──────────────────────────────────────────────────────

/// Runs whenever `Settings` is mutated; flips all debug collider visibility.
pub(crate) fn toggle_debug_collider_visibility(
    settings: Res<Settings>,
    mut colliders: Query<&mut Visibility, With<DebugCollider>>,
) {
    if !settings.is_changed() {
        return;
    }
    let vis = debug_vis(&settings);
    for mut v in &mut colliders {
        *v = vis;
    }
}

fn debug_vis(settings: &Settings) -> Visibility {
    if settings.debug_collisions {
        Visibility::Visible
    } else {
        Visibility::Hidden
    }
}
