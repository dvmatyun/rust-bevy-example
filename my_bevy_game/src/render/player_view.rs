//! Player rendering: a thin upright slab (2D rectangle in 3D world)
//! that always faces the camera (billboard).
//!
//! Three visual variants are pre-built — front / side / back — and the
//! current one is chosen each frame by comparing the player's `Facing`
//! to the camera direction. This makes the sprite "show its face" or
//! "show its back" depending on where the camera is around the player.
//!
//! Note on the billboard: it writes `Player.Transform.rotation`. This
//! is a deliberate exception (rotation is purely visual; gameplay does
//! not read it). Documented in `docsai/architecture.md`.

use bevy::prelude::*;

use crate::data::{Facing, GameCamera, Player, PlayerSlot};

struct FaceAsset {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
}

/// Five view buckets covering 360° around the player. Widths narrow
/// toward the side view (silhouette effect) and colour darkens from
/// face to back.
#[derive(Resource)]
pub(crate) struct PlayerAssets {
    /// dot > 0.85 — camera nearly head-on with player.
    front: FaceAsset,
    /// 0.4 < dot ≤ 0.85 — camera between front and side.
    front_side: FaceAsset,
    /// |dot| ≤ 0.4 — camera roughly perpendicular to facing.
    side: FaceAsset,
    /// -0.85 ≤ dot < -0.4 — camera between side and back.
    back_side: FaceAsset,
    /// dot < -0.85 — camera behind the player.
    back: FaceAsset,
}

pub fn register_player_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut make = |w: f32, color: Color| FaceAsset {
        mesh: meshes.add(Cuboid::new(w, 1.4, 0.05)),
        material: materials.add(StandardMaterial {
            base_color: color,
            unlit: true,
            cull_mode: None, // visible from both sides
            ..default()
        }),
    };
    commands.insert_resource(PlayerAssets {
        front: make(0.80, Color::srgb(1.00, 0.55, 0.55)), // light pink — face
        front_side: make(0.60, Color::srgb(0.95, 0.42, 0.40)),
        side: make(0.30, Color::srgb(0.90, 0.30, 0.20)), // narrow profile
        back_side: make(0.60, Color::srgb(0.65, 0.22, 0.14)),
        back: make(0.80, Color::srgb(0.45, 0.15, 0.10)), // dark — back of head
    });
}

pub fn attach_player_visuals(
    mut commands: Commands,
    assets: Option<Res<PlayerAssets>>,
    new_players: Query<Entity, Added<Player>>,
) {
    let Some(assets) = assets else {
        return;
    };
    for entity in &new_players {
        commands.entity(entity).insert((
            Mesh3d(assets.front.mesh.clone()),
            MeshMaterial3d(assets.front.material.clone()),
        ));
    }
}

/// Picks front / side / back asset per frame based on the angle between
/// the player's `Facing` and the player→camera vector (XZ plane).
/// Each player matches by `PlayerSlot`; falls back to slot 0's camera.
pub fn update_player_face_view(
    cam_q: Query<(&Transform, &PlayerSlot), (With<GameCamera>, Without<Player>)>,
    assets: Option<Res<PlayerAssets>>,
    mut player_q: Query<
        (
            &Transform,
            &Facing,
            &PlayerSlot,
            &mut Mesh3d,
            &mut MeshMaterial3d<StandardMaterial>,
        ),
        With<Player>,
    >,
) {
    let Some(assets) = assets else { return };
    if cam_q.is_empty() { return };

    for (tf, facing, &PlayerSlot(p_slot), mut mesh, mut mat) in &mut player_q {
        // Use the matching camera, fall back to the first available.
        let cam_tf = cam_q
            .iter()
            .find(|&(_, &PlayerSlot(s))| s == p_slot)
            .or_else(|| cam_q.iter().next())
            .map(|(t, _)| t);
        let Some(cam_tf) = cam_tf else { continue };

        let pc = (cam_tf.translation - tf.translation).xz();
        if pc.length_squared() < 1e-4 { continue }
        let pc = pc.normalize();
        let dot = facing.0.dot(pc);
        let asset = if dot > 0.85 {
            &assets.front
        } else if dot > 0.40 {
            &assets.front_side
        } else if dot >= -0.40 {
            &assets.side
        } else if dot >= -0.85 {
            &assets.back_side
        } else {
            &assets.back
        };
        mesh.0 = asset.mesh.clone();
        mat.0 = asset.material.clone();
    }
}

pub fn billboard_player(
    cam_q: Query<(&Transform, &PlayerSlot), (With<GameCamera>, Without<Player>)>,
    mut player_q: Query<(&mut Transform, &PlayerSlot), With<Player>>,
) {
    if cam_q.is_empty() { return }

    for (mut tf, &PlayerSlot(p_slot)) in &mut player_q {
        let cam_tf = cam_q
            .iter()
            .find(|&(_, &PlayerSlot(s))| s == p_slot)
            .or_else(|| cam_q.iter().next())
            .map(|(t, _)| t);
        let Some(cam_tf) = cam_tf else { continue };
        let to_cam = cam_tf.translation - tf.translation;
        let yaw = to_cam.x.atan2(to_cam.z);
        tf.rotation = Quat::from_rotation_y(yaw);
    }
}
