//! Visual feedback markers — purely cosmetic, do NOT influence gameplay.
//!
//! - **Tap echo** (`TapMarker`): a yellow sphere that briefly expands at
//!   the click target position, then disappears. One per `ClickMoveIntent`.
//! - **Walk-target marker** (`WalkTargetMarker`): a green disc on the
//!   ground at the active `MoveTarget`. Spawned when the target is set,
//!   moved when it changes, despawned when the player arrives.

use bevy::prelude::*;

use crate::data::{terrain_gen, ClickMoveIntent, MoveTarget, WorldConfig};

const TAP_TTL: f32 = 0.5;

#[derive(Component)]
pub(crate) struct TapMarker {
    ttl: f32,
    max_ttl: f32,
}

#[derive(Component)]
pub(crate) struct WalkTargetMarker;

#[derive(Resource)]
pub(crate) struct MarkerAssets {
    tap_mesh: Handle<Mesh>,
    tap_material: Handle<StandardMaterial>,
    walk_mesh: Handle<Mesh>,
    walk_material: Handle<StandardMaterial>,
}

pub(crate) fn register_marker_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(MarkerAssets {
        tap_mesh: meshes.add(Sphere::new(0.25)),
        tap_material: materials.add(StandardMaterial {
            base_color: Color::srgba(1.0, 0.95, 0.30, 0.85),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            ..default()
        }),
        // Flat disc that lies on the ground. Plane3d normal is +Y so the
        // visible face is up.
        walk_mesh: meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(0.45)).mesh().build()),
        walk_material: materials.add(StandardMaterial {
            base_color: Color::srgba(0.30, 1.0, 0.40, 0.55),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            cull_mode: None,
            ..default()
        }),
    });
}

/// Spawn a tap echo for every `ClickMoveIntent` this frame.
pub(crate) fn spawn_tap_markers(
    mut events: MessageReader<ClickMoveIntent>,
    mut commands: Commands,
    assets: Option<Res<MarkerAssets>>,
    config: Res<WorldConfig>,
) {
    let Some(assets) = assets else { return };
    for ev in events.read() {
        let target = ev.target;
        // Lift the sphere a bit above the surface so it isn't half-buried.
        let y = terrain_gen::terrain_top_y(target.x, target.y, &config) + 0.5;
        commands.spawn((
            TapMarker {
                ttl: TAP_TTL,
                max_ttl: TAP_TTL,
            },
            Mesh3d(assets.tap_mesh.clone()),
            MeshMaterial3d(assets.tap_material.clone()),
            Transform::from_xyz(target.x, y, target.y),
        ));
    }
}

/// Pulse the tap marker (grow + then fade by despawning when ttl hits 0).
pub(crate) fn tick_tap_markers(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut TapMarker, &mut Transform)>,
) {
    let dt = time.delta_secs();
    for (e, mut m, mut tf) in &mut q {
        m.ttl -= dt;
        if m.ttl <= 0.0 {
            commands.entity(e).despawn();
            continue;
        }
        let frac = m.ttl / m.max_ttl; // 1.0 at start → 0.0 at end
        // Quick pulse: scale 0.5 → 2.0 over the lifetime.
        let scale = 0.5 + (1.0 - frac) * 1.5;
        tf.scale = Vec3::splat(scale);
    }
}

/// Keep one `WalkTargetMarker` entity alive iff `MoveTarget` is `Some`.
/// On change it just repositions the existing entity instead of
/// despawn-and-respawn for cheaper churn.
pub(crate) fn sync_walk_target_marker(
    mut commands: Commands,
    target: Res<MoveTarget>,
    config: Res<WorldConfig>,
    assets: Option<Res<MarkerAssets>>,
    mut existing: Query<(Entity, &mut Transform), With<WalkTargetMarker>>,
) {
    let Some(assets) = assets else { return };
    match target.0 {
        Some(t) => {
            // Slightly above the surface so the disc is visible on top
            // of the terrain block.
            let y = terrain_gen::terrain_top_y(t.x, t.y, &config) + 0.06;
            let pos = Vec3::new(t.x, y, t.y);
            if let Ok((_, mut tf)) = existing.single_mut() {
                tf.translation = pos;
            } else {
                commands.spawn((
                    WalkTargetMarker,
                    Mesh3d(assets.walk_mesh.clone()),
                    MeshMaterial3d(assets.walk_material.clone()),
                    Transform::from_translation(pos),
                ));
            }
        }
        None => {
            for (e, _) in &existing {
                commands.entity(e).despawn();
            }
        }
    }
}
