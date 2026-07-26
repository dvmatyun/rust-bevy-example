//! Roaming monsters: spawn / despawn / wander logic.
//!
//! All monsters share a single component shape (`Monster + Wander +
//! Transform + Facing`). The `kind` byte selects which sprite the
//! render layer uses. Wander is a simple "move in a direction for a
//! random number of seconds, then pick a new direction" — bouncing
//! off the world boundary.
//!
//! Spawning is intent-driven (`SpawnMonstersIntent` from the
//! settings UI). Positions are random within a radius around the
//! player so a fresh batch is always visible.

use bevy::prelude::*;

use crate::data::{
    terrain_gen, DespawnMonstersIntent, Facing, Monster, Player, SpawnMonstersIntent, Wander,
    WorldConfig,
};

const MONSTER_HALF_HEIGHT: f32 = 0.5;
const MIN_WANDER_TIME: f32 = 1.5;
const MAX_WANDER_TIME: f32 = 4.5;
const SPAWN_RADIUS: f32 = 25.0; // around the player

/// Process spawn-monsters requests from the settings UI.
pub fn apply_spawn_monsters(
    mut events: MessageReader<SpawnMonstersIntent>,
    mut commands: Commands,
    config: Res<WorldConfig>,
    player_q: Query<&Transform, With<Player>>,
    time: Res<Time>,
    mut rng_state: Local<u32>,
) {
    if *rng_state == 0 {
        *rng_state = 0xCAFEBABE;
    }
    let player_pos = player_q.single().map(|t| t.translation).unwrap_or(Vec3::ZERO);

    for ev in events.read() {
        // Salt the RNG with the wall-clock so successive presses don't
        // produce identical layouts.
        *rng_state ^= time.elapsed_secs().to_bits();

        let world_bound = (config.world_radius_cells - 1) as f32;
        for _ in 0..ev.count {
            let angle = pcg_f32(&mut *rng_state) * std::f32::consts::TAU;
            let dist = pcg_f32(&mut *rng_state) * SPAWN_RADIUS;
            let x = (player_pos.x + angle.cos() * dist).clamp(-world_bound, world_bound);
            let z = (player_pos.z + angle.sin() * dist).clamp(-world_bound, world_bound);
            let y = terrain_gen::terrain_top_y(x, z, &config) + MONSTER_HALF_HEIGHT;

            let kind = (pcg_step(&mut *rng_state) % 3) as u8;
            let dir_angle = pcg_f32(&mut *rng_state) * std::f32::consts::TAU;
            let speed = 1.5 + pcg_f32(&mut *rng_state) * 1.5;

            commands.spawn((
                Monster { kind },
                Wander {
                    direction: Vec2::new(dir_angle.cos(), dir_angle.sin()),
                    time_remaining:
                        MIN_WANDER_TIME + pcg_f32(&mut *rng_state) * (MAX_WANDER_TIME - MIN_WANDER_TIME),
                    speed,
                },
                Facing(Vec2::new(dir_angle.cos(), dir_angle.sin())),
                Transform::from_xyz(x, y, z),
            ));
        }
    }
}

/// Despawn every monster on the map.
pub fn apply_despawn_all(
    mut events: MessageReader<DespawnMonstersIntent>,
    mut commands: Commands,
    monsters: Query<Entity, With<Monster>>,
) {
    if events.read().next().is_none() {
        return;
    }
    for e in &monsters {
        commands.entity(e).despawn();
    }
}

/// Advance every monster: integrate position, bounce off world
/// bounds, snap Y to terrain, decrement / refresh wander timer.
pub fn wander_monsters(
    time: Res<Time>,
    config: Res<WorldConfig>,
    mut q: Query<(&mut Transform, &mut Wander, &mut Facing), With<Monster>>,
    mut rng_state: Local<u32>,
) {
    if *rng_state == 0 {
        *rng_state = 0xDEADBEEF;
    }
    let dt = time.delta_secs();
    let bound = (config.world_radius_cells - 1) as f32;

    for (mut tf, mut w, mut facing) in &mut q {
        // Refresh direction when timer expires.
        w.time_remaining -= dt;
        if w.time_remaining <= 0.0 {
            let angle = pcg_f32(&mut *rng_state) * std::f32::consts::TAU;
            w.direction = Vec2::new(angle.cos(), angle.sin());
            w.time_remaining =
                MIN_WANDER_TIME + pcg_f32(&mut *rng_state) * (MAX_WANDER_TIME - MIN_WANDER_TIME);
        }

        // Integrate XZ.
        let dx = w.direction.x * w.speed * dt;
        let dz = w.direction.y * w.speed * dt;
        let mut nx = tf.translation.x + dx;
        let mut nz = tf.translation.z + dz;

        // Bounce off world edges.
        if nx.abs() > bound {
            w.direction.x = -w.direction.x;
            nx = nx.clamp(-bound, bound);
        }
        if nz.abs() > bound {
            w.direction.y = -w.direction.y;
            nz = nz.clamp(-bound, bound);
        }
        tf.translation.x = nx;
        tf.translation.z = nz;
        tf.translation.y = terrain_gen::terrain_top_y(nx, nz, &config) + MONSTER_HALF_HEIGHT;
        facing.0 = w.direction;
    }
}

// === Tiny PCG-style PRNG ====================================================

fn pcg_step(state: &mut u32) -> u32 {
    *state = state
        .wrapping_mul(747796405)
        .wrapping_add(2891336453);
    let word = ((*state >> ((*state >> 28).wrapping_add(4))) ^ *state).wrapping_mul(277803737);
    (word >> 22) ^ word
}

fn pcg_f32(state: &mut u32) -> f32 {
    pcg_step(state) as f32 / u32::MAX as f32
}
