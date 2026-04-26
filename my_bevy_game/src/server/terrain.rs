//! Terrain generation. Pure game-logic: no rendering.
//!
//! Spawns logical `(TerrainBlock, Transform)` entities for each surface
//! cell and populates the `TerrainHeights` resource. The render layer
//! later attaches `Mesh3d`/`MeshMaterial3d` via `Added<TerrainBlock>`.

use bevy::prelude::*;

use crate::data::{Biome, TerrainBlock, TerrainHeights, WorldConfig};

pub fn setup_terrain(
    mut commands: Commands,
    config: Res<WorldConfig>,
    mut heights: ResMut<TerrainHeights>,
) {
    heights.size = config.size;
    heights.cells = vec![0; (config.size * config.size) as usize];

    let half = config.size / 2;
    for z in 0..config.size {
        for x in 0..config.size {
            let wx = x - half;
            let wz = z - half;
            let h = terrain_height_at(wx, wz, &config);
            heights.cells[(z * config.size + x) as usize] = h;

            commands.spawn((
                TerrainBlock {
                    biome: Biome::from_height(h),
                },
                Transform::from_xyz(wx as f32, h as f32, wz as f32),
            ));
        }
    }
}

pub fn terrain_height_at(x: i32, z: i32, config: &WorldConfig) -> i32 {
    let n = fbm(x as f32 * config.noise_scale, z as f32 * config.noise_scale);
    ((n - 0.3) * config.height_scale).round() as i32
}

// === Hash-based 2D value noise (deterministic, no extra crate deps) =========

fn hash2(x: i32, z: i32) -> f32 {
    let mut h = (x as u32)
        .wrapping_mul(374761393)
        .wrapping_add((z as u32).wrapping_mul(668265263));
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    h ^= h >> 16;
    (h as f32) / (u32::MAX as f32)
}

fn smoothstep(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn value_noise(x: f32, z: f32) -> f32 {
    let xi = x.floor() as i32;
    let zi = z.floor() as i32;
    let u = smoothstep(x - xi as f32);
    let v = smoothstep(z - zi as f32);
    let n00 = hash2(xi, zi);
    let n10 = hash2(xi + 1, zi);
    let n01 = hash2(xi, zi + 1);
    let n11 = hash2(xi + 1, zi + 1);
    let a = n00 * (1.0 - u) + n10 * u;
    let b = n01 * (1.0 - u) + n11 * u;
    a * (1.0 - v) + b * v
}

/// Fractional Brownian motion: 4 octaves of value noise summed with
/// halving amplitude and doubling frequency.
fn fbm(x: f32, z: f32) -> f32 {
    let mut total = 0.0;
    let mut amp = 1.0;
    let mut freq = 1.0;
    let mut max_amp = 0.0;
    for _ in 0..4 {
        total += value_noise(x * freq, z * freq) * amp;
        max_amp += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    total / max_amp
}
