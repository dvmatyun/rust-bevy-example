//! Camera-occlusion fade.
//!
//! Any entity tagged with `Occludable` (typically walls and roofs)
//! whose AABB sits between the camera and the player is smoothly
//! faded to a low alpha so the player stays visible.
//!
//! Implementation:
//! - Per `Occludable` entity we run a line-segment ↔ AABB test
//!   (camera.translation → player.translation).
//! - If the segment passes through the AABB, target alpha = 0.25;
//!   otherwise 1.0.
//! - The entity's `current_alpha` is exponentially smoothed toward
//!   the target so transitions are continuous.
//! - Each occludable spawns with its OWN `StandardMaterial` handle
//!   (see `render/buildings.rs::spawn_part`) so adjusting alpha on
//!   one wall doesn't bleed onto its neighbours.
//!
//! World-AABB approximation: we treat the entity's `Transform.scale`
//! as the size of a unit-cube mesh, so half-extents = `scale / 2`.
//! This is correct for the floor-slab and wall geometry built in
//! `render/buildings.rs`. Other shapes would need their own bounds.

use bevy::prelude::*;

use crate::data::{GameCamera, Player};

const OCCLUDED_ALPHA: f32 = 0.25;
/// Higher = snappier transition; lower = floaty fade.
const FADE_RATE: f32 = 8.0;

#[derive(Component)]
pub(crate) struct Occludable {
    pub current_alpha: f32,
}

impl Default for Occludable {
    fn default() -> Self {
        Self { current_alpha: 1.0 }
    }
}

pub(crate) fn fade_occluders(
    time: Res<Time>,
    cam_q: Query<&Transform, (With<GameCamera>, Without<Occludable>, Without<Player>)>,
    player_q: Query<&Transform, (With<Player>, Without<Occludable>)>,
    mut occluders: Query<
        (
            &GlobalTransform,
            &MeshMaterial3d<StandardMaterial>,
            &mut Occludable,
        ),
    >,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(cam_tf) = cam_q.iter().next() else { return };
    let Some(player_tf) = player_q.iter().next() else { return };

    let dt = time.delta_secs();
    // Frame-rate-independent exponential smoothing.
    let alpha_lerp = 1.0 - (-FADE_RATE * dt).exp();

    for (gt, mat_handle, mut occ) in &mut occluders {
        // World position is the global transform's translation; the
        // scale field carries the unit-cube extents we used at spawn.
        // (`Transform.scale` * 1.0 = world dimensions for a unit
        // cuboid; halved gives the AABB extent.)
        let (scale, _, translation) = gt.to_scale_rotation_translation();
        let half = scale.abs() * 0.5;

        let intersects = line_segment_aabb_intersect(
            cam_tf.translation,
            player_tf.translation,
            translation,
            half,
        );
        let target = if intersects { OCCLUDED_ALPHA } else { 1.0 };
        occ.current_alpha += (target - occ.current_alpha) * alpha_lerp;

        if let Some(mut mat) = materials.get_mut(&mat_handle.0) {
            mat.base_color.set_alpha(occ.current_alpha);
        }
    }
}

/// Slab-test line-segment ↔ AABB intersection. Returns true iff the
/// segment `p0 → p1` passes through the box defined by `center ± half`.
fn line_segment_aabb_intersect(p0: Vec3, p1: Vec3, center: Vec3, half: Vec3) -> bool {
    let dir = p1 - p0;
    let aabb_min = center - half;
    let aabb_max = center + half;

    let mut t_min = 0.0_f32;
    let mut t_max = 1.0_f32;

    // Slab-test along each of the 3 axes.
    let axes = [
        (dir.x, p0.x, aabb_min.x, aabb_max.x),
        (dir.y, p0.y, aabb_min.y, aabb_max.y),
        (dir.z, p0.z, aabb_min.z, aabb_max.z),
    ];
    for (d, p, mn, mx) in axes {
        if d.abs() < 1e-6 {
            // Segment is parallel to this axis. It can only hit the
            // box if the start point is already inside the slab.
            if p < mn || p > mx {
                return false;
            }
        } else {
            let t1 = (mn - p) / d;
            let t2 = (mx - p) / d;
            let t_near = t1.min(t2);
            let t_far = t1.max(t2);
            t_min = t_min.max(t_near);
            t_max = t_max.min(t_far);
            if t_min > t_max {
                return false;
            }
        }
    }
    true
}
