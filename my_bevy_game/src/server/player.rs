//! Player game logic: spawn, movement, ground collision, facing.
//!
//! Reads `MoveIntent` events. Mutates only `Transform` and `Facing`.
//! No rendering.
//!
//! 📘 Reading guide: each `pub fn` here is a *Bevy system*. Bevy
//! decides automatically when to run it based on the *system
//! parameters* — the function arguments. `Res<T>` = read-only resource
//! borrow, `ResMut<T>` = exclusive resource borrow, `Query<...>` =
//! the entities matching a filter, `MessageReader<T>` = incoming
//! messages. Bevy injects all of these for you.

use bevy::prelude::*;

use crate::data::{Facing, MoveIntent, Player, Replicate, TerrainHeights, WorldConfig};

pub fn spawn_player(mut commands: Commands, config: Res<WorldConfig>) {
    // 📘 `commands.spawn((A, B, C, ...))` creates a new entity with
    // the listed components attached. The tuple is a *Bundle* —
    // anything that's `Component` or another Bundle works.
    commands.spawn((
        Player,
        Facing::default(),
        // Spawn high; `snap_to_ground` corrects it on the first Update.
        Transform::from_xyz(0.0, config.height_scale + 5.0, 0.0),
        Replicate, // server-authoritative entity → ships to all clients
    ));
}

/// Sums all `MoveIntent`s this frame, advances the player on the XZ
/// plane, and updates `Facing` to the direction of movement.
pub fn apply_movement(
    // 📘 MessageReader iterates events written this frame (and the
    // last one, since events are double-buffered).
    mut events: MessageReader<MoveIntent>,
    config: Res<WorldConfig>,
    time: Res<Time>,
    // 📘 Query<(&mut T, &mut U), With<X>> = "for every entity with
    // X marker, give me mutable borrows of T and U". The tuple says
    // which components to fetch; the second tuple slot (here `With<Player>`)
    // is the *filter* — it doesn't fetch anything, just narrows results.
    mut q: Query<(&mut Transform, &mut Facing), With<Player>>,
) {
    // 📘 Vec2::ZERO is a const initialiser: const ZERO: Vec2 = Vec2::new(0.0, 0.0).
    let mut total = Vec2::ZERO;
    for e in events.read() {
        total += e.direction;
    }
    if total == Vec2::ZERO {
        // 📘 Early return is idiomatic when there's nothing to do —
        // skips the rest without an `else` branch.
        return;
    }
    let dir = total.normalize();
    let dt = time.delta_secs();
    let bound = (config.size / 2 - 1) as f32;
    // 📘 `for (mut tf, mut facing) in &mut q` iterates the query
    // *mutably*. The `&mut q` (vs `&q`) is what makes the borrows
    // mutable. Each loop iteration destructures the tuple into two
    // bindings.
    for (mut tf, mut facing) in &mut q {
        tf.translation.x =
            (tf.translation.x + dir.x * config.player_speed * dt).clamp(-bound, bound);
        tf.translation.z =
            (tf.translation.z + dir.y * config.player_speed * dt).clamp(-bound, bound);
        facing.0 = dir;
    }
}

/// Pins the player Y to the surface of the terrain column under it.
pub fn snap_to_ground(
    heights: Res<TerrainHeights>,
    config: Res<WorldConfig>,
    mut q: Query<&mut Transform, With<Player>>,
) {
    if heights.cells.is_empty() {
        return;
    }
    for mut tf in &mut q {
        tf.translation.y =
            heights.ground_y(tf.translation.x, tf.translation.z) + config.player_half_height;
    }
}
