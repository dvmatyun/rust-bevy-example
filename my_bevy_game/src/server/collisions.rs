//! Collision response for the player and monsters.
//!
//! Two stages run sequentially each frame:
//!   1. **Wall push-out** — every body is pushed out of any
//!      `Building::wall_segments()` AABB on the XZ plane.
//!   2. **Body push-out** — pairwise cylinder ↔ cylinder push-apart
//!      between every player/monster body that overlaps another.
//!
//! Both stages are gated by `Settings.collisions_enabled` and use the
//! `player_radius` / `monster_radius` from `Settings` so the user can
//! tune them at runtime.
//!
//! Body shape: vertical cylinder. Radius from settings, height from
//! `WorldConfig.player_half_height` (player) or a fixed value
//! (monsters — they're sprite-sized).
//!
//! Pair iteration is O(n²) — fine for the demo (a few hundred
//! monsters max). For a real game with thousands of bodies we'd swap
//! in a uniform spatial grid keyed by integer `(x, z)` cell.

use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use crate::data::{Building, Monster, Player, Settings, WallSegment, WorldConfig};

/// Approximate vertical extent of a monster body (used for the wall &
/// body Y-overlap tests). Matches the half-height in `monsters_view`.
const MONSTER_HALF_HEIGHT: f32 = 0.5;

/// Resolve player ↔ wall collisions on the XZ plane. Runs after
/// movement and before `snap_to_ground` (which only edits Y).
pub fn resolve_player_wall_collisions(
    settings: Res<Settings>,
    config: Res<WorldConfig>,
    buildings: Query<&Building>,
    mut player_q: Query<&mut Transform, (With<Player>, Without<Monster>)>,
) {
    if !settings.collisions_enabled {
        return;
    }
    let walls = collect_walls(&buildings);
    if walls.is_empty() {
        return;
    }
    for mut tf in &mut player_q {
        push_out_walls(&mut tf, settings.player_radius, config.player_half_height, &walls);
    }
}

/// Resolve monster ↔ wall collisions. Runs after `wander_monsters`.
pub fn resolve_monster_wall_collisions(
    settings: Res<Settings>,
    buildings: Query<&Building>,
    mut monster_q: Query<&mut Transform, With<Monster>>,
) {
    if !settings.collisions_enabled {
        return;
    }
    let walls = collect_walls(&buildings);
    if walls.is_empty() {
        return;
    }
    for mut tf in &mut monster_q {
        push_out_walls(&mut tf, settings.monster_radius, MONSTER_HALF_HEIGHT, &walls);
    }
}

/// Pairwise body push-apart between the player and every monster
/// (and monsters with each other). Player has infinite mass — only
/// the monster moves on player ↔ monster overlap. Monster ↔ monster
/// splits the push 50/50.
pub fn resolve_body_body_collisions(
    settings: Res<Settings>,
    config: Res<WorldConfig>,
    mut bodies: Query<
        (Entity, &mut Transform, Option<&Player>, Option<&Monster>),
        Or<(With<Player>, With<Monster>)>,
    >,
) {
    if !settings.collisions_enabled {
        return;
    }

    // Snapshot positions + per-body mass / radius / height.
    struct Body {
        entity: Entity,
        pos: Vec3,
        radius: f32,
        half_h: f32,
        is_player: bool,
    }

    let mut snapshot: Vec<Body> = Vec::new();
    for (entity, tf, player, monster) in &bodies {
        let (radius, half_h, is_player) = if player.is_some() {
            (settings.player_radius, config.player_half_height, true)
        } else if monster.is_some() {
            (settings.monster_radius, MONSTER_HALF_HEIGHT, false)
        } else {
            continue;
        };
        snapshot.push(Body {
            entity,
            pos: tf.translation,
            radius,
            half_h,
            is_player,
        });
    }

    // Pairwise cylinder push-apart on the XZ plane.
    for i in 0..snapshot.len() {
        let (left, right) = snapshot.split_at_mut(i + 1);
        let a = &mut left[i];
        for b in right.iter_mut() {
            // Y overlap (cylinder height test).
            if (a.pos.y - b.pos.y).abs() >= a.half_h + b.half_h {
                continue;
            }
            let dx = b.pos.x - a.pos.x;
            let dz = b.pos.z - a.pos.z;
            let dist_sq = dx * dx + dz * dz;
            let min_dist = a.radius + b.radius;
            if dist_sq >= min_dist * min_dist {
                continue;
            }
            let dist = dist_sq.sqrt();
            // Bodies exactly on the same XZ — pick an arbitrary axis
            // so they slide apart rather than NaN out.
            let (nx, nz) = if dist < 1e-4 {
                (1.0_f32, 0.0_f32)
            } else {
                (dx / dist, dz / dist)
            };
            let overlap = min_dist - dist;

            // Player has infinite mass: monsters bounce off, player
            // doesn't budge. Monster ↔ monster splits 50/50.
            let (a_share, b_share) = match (a.is_player, b.is_player) {
                (true, false) => (0.0, 1.0),
                (false, true) => (1.0, 0.0),
                _ => (0.5, 0.5),
            };

            a.pos.x -= nx * overlap * a_share;
            a.pos.z -= nz * overlap * a_share;
            b.pos.x += nx * overlap * b_share;
            b.pos.z += nz * overlap * b_share;
        }
    }

    // Write back keyed by Entity — robust to any iteration order.
    let updates: HashMap<Entity, Vec3> =
        snapshot.iter().map(|b| (b.entity, b.pos)).collect();
    for (entity, mut tf, _, _) in &mut bodies {
        if let Some(new_pos) = updates.get(&entity) {
            tf.translation.x = new_pos.x;
            tf.translation.z = new_pos.z;
        }
    }
}

fn collect_walls(buildings: &Query<&Building>) -> Vec<WallSegment> {
    let mut out = Vec::new();
    for b in buildings {
        out.extend(b.wall_segments());
    }
    out
}

/// For each wall whose Y-range overlaps the body, push the body out
/// of the wall along the XZ axis with the smaller penetration depth.
/// This is "good enough" for our slow-walking gameplay; for fast
/// motion a swept test would be more correct.
fn push_out_walls(tf: &mut Transform, radius: f32, half_height: f32, walls: &[WallSegment]) {
    for w in walls {
        let half = w.size * 0.5;
        let wall_min_y = w.center.y - half.y;
        let wall_max_y = w.center.y + half.y;
        let body_min_y = tf.translation.y - half_height;
        let body_max_y = tf.translation.y + half_height;
        if body_max_y <= wall_min_y || body_min_y >= wall_max_y {
            continue;
        }

        let dx = tf.translation.x - w.center.x;
        let dz = tf.translation.z - w.center.z;
        let overlap_x = (half.x + radius) - dx.abs();
        let overlap_z = (half.z + radius) - dz.abs();
        if overlap_x <= 0.0 || overlap_z <= 0.0 {
            continue; // already outside on at least one axis
        }

        // Push along the axis with smaller penetration. If the body
        // is exactly on the wall axis (signum = 0) pick +1 to break
        // the tie deterministically.
        if overlap_x < overlap_z {
            let s = if dx == 0.0 { 1.0 } else { dx.signum() };
            tf.translation.x += overlap_x * s;
        } else {
            let s = if dz == 0.0 { 1.0 } else { dz.signum() };
            tf.translation.z += overlap_z * s;
        }
    }
}
