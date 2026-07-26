//! Player game logic: spawn, movement, ground collision, facing.
//!
//! Reads `MoveIntent` and `TeleportIntent` events. Mutates only
//! `Transform` and `Facing`. No rendering.
//!
//! 📘 Reading guide: each `pub fn` here is a *Bevy system*. Bevy
//! decides automatically when to run it based on the *system
//! parameters* — the function arguments. `Res<T>` = read-only resource
//! borrow, `ResMut<T>` = exclusive resource borrow, `Query<...>` =
//! the entities matching a filter, `MessageReader<T>` = incoming
//! messages. Bevy injects all of these for you.

use bevy::prelude::*;

use crate::data::{
    terrain_gen, Building, ClientTransform, Facing, GameEntity, MoveIntent, Player, PlayerSlot,
    Replicate, Stairs, TeleportIntent, WorldConfig,
};

pub fn spawn_player_single(mut commands: Commands, config: Res<WorldConfig>) {
    let tf = Transform::from_xyz(0.0, config.height_scale + 5.0, 0.0);
    commands.spawn((
        Player,
        PlayerSlot(0),
        Facing::default(),
        tf,
        ClientTransform(tf),
        GameEntity,
        Replicate,
    ));
}

pub fn spawn_player_multiplayer(mut commands: Commands, config: Res<WorldConfig>) {
    for slot in 0u8..2 {
        let x = if slot == 0 { -2.0 } else { 2.0 };
        let tf = Transform::from_xyz(x, config.height_scale + 5.0, 0.0);
        commands.spawn((
            Player,
            PlayerSlot(slot),
            Facing::default(),
            tf,
            ClientTransform(tf),
            GameEntity,
            Replicate,
        ));
    }
}

/// Sums all `MoveIntent`s this frame per player slot, advances each
/// player on the XZ plane, and updates `Facing`.
pub fn apply_movement(
    mut events: MessageReader<MoveIntent>,
    config: Res<WorldConfig>,
    time: Res<Time>,
    mut q: Query<(&mut Transform, &mut Facing, &PlayerSlot), With<Player>>,
) {
    let mut totals = [Vec2::ZERO; 2];
    for e in events.read() {
        totals[e.player_slot.min(1) as usize] += e.direction;
    }
    let dt = time.delta_secs();
    let bound = (config.world_radius_cells - 1) as f32;
    for (mut tf, mut facing, &PlayerSlot(slot)) in &mut q {
        let total = totals[slot.min(1) as usize];
        if total == Vec2::ZERO {
            continue;
        }
        let dir = total.normalize();
        tf.translation.x =
            (tf.translation.x + dir.x * config.player_speed * dt).clamp(-bound, bound);
        tf.translation.z =
            (tf.translation.z + dir.y * config.player_speed * dt).clamp(-bound, bound);
        facing.0 = dir;
    }
}

/// Debug: nudge a player vertically by slot. Real movement up
/// requires a physics engine — see `docsai/world-and-graphics-plan.md`.
pub fn apply_teleport(
    mut events: MessageReader<TeleportIntent>,
    mut q: Query<(&mut Transform, &PlayerSlot), With<Player>>,
) {
    let mut totals = [0.0f32; 2];
    for e in events.read() {
        totals[e.player_slot.min(1) as usize] += e.delta_y;
    }
    for (mut tf, &PlayerSlot(slot)) in &mut q {
        let total = totals[slot.min(1) as usize];
        if total != 0.0 {
            tf.translation.y += total;
        }
    }
}

/// Snap the player Y to the highest walkable surface at-or-below
/// their current Y. Surfaces are:
///   - the terrain heightmap (always present, the base ground),
///   - the floor of any `Building` whose footprint contains the
///     player's XZ,
///   - the surface of any `Stairs` whose footprint contains the
///     player's XZ (one Y per discrete step, computed from the
///     stair's slope).
///
/// "At-or-below current Y" lets the player walk between floors — if
/// you teleport up to Y=6, you snap onto the 5.5 floor; if you walk
/// off the edge of that floor onto terrain, you snap down to the
/// terrain.
pub fn snap_to_ground(
    config: Res<WorldConfig>,
    buildings: Query<&Building>,
    stairs_q: Query<&Stairs>,
    mut q: Query<&mut Transform, With<Player>>,
) {
    let half_h = config.player_half_height;
    // Tolerance > half_h so once we've snapped to a floor (player.y =
    // floor_y + half_h), the next frame's "≤ player.y + tolerance"
    // check still includes that floor.
    let stand_tolerance = half_h + 0.1;

    for mut tf in &mut q {
        let p_xz = Vec2::new(tf.translation.x, tf.translation.z);
        let mut best_y = terrain_gen::terrain_top_y(p_xz.x, p_xz.y, &config);

        // Building floors.
        for b in &buildings {
            if !b.footprint.contains(p_xz) {
                continue;
            }
            for &floor_y in &b.floor_heights {
                if floor_y <= tf.translation.y + stand_tolerance && floor_y > best_y {
                    best_y = floor_y;
                }
            }
        }

        // Stair steps.
        for s in &stairs_q {
            if let Some(surface_y) = s.surface_y_at(p_xz) {
                if surface_y <= tf.translation.y + stand_tolerance && surface_y > best_y {
                    best_y = surface_y;
                }
            }
        }

        tf.translation.y = best_y + half_h;
    }
}
