//! Procedural terrain noise + heightmap queries.
//!
//! All terrain heights are computed *on demand* from the noise
//! function — there is no central `TerrainHeights` resource. This
//! lets the game support arbitrarily large worlds (10× / 100× /
//! 10000× scale) without any per-cell memory cost.
//!
//! Heights are continuous floats. The base shape comes from 4-octave
//! FBM at `WorldConfig.noise_scale`; on top of that we add a
//! high-frequency low-amplitude "bump" layer so the surface has
//! pebble-scale variation instead of being a smooth sheet — Valheim's
//! rolling-hills look.
//!
//! - `terrain_surface_y_cell(x, z, &cfg)` → continuous Y at integer
//!   cell coordinates. Used by the chunk-mesh builder, which samples
//!   the four corners of every cell and stitches them.
//! - `terrain_top_y(world_x, world_z, &cfg)` → bilinearly-interpolated
//!   surface Y at floating-point world coords (smooth between cells).
//! - `terrain_raycast(ray, max_dist, &cfg)` → ray-march against the
//!   heightmap and return the world-space intersection if any.

use bevy::prelude::*;

use super::config::WorldConfig;

// === Public queries =========================================================

/// Continuous surface Y at the integer cell coordinate `(x, z)`.
pub fn terrain_surface_y_cell(x: i32, z: i32, config: &WorldConfig) -> f32 {
    base_height_at(x as f32, z as f32, config) + bump_at(x as f32, z as f32)
}

/// Surface Y at arbitrary world coordinates, bilinearly interpolated
/// between the four surrounding integer cells. Smooth movement &
/// monster placement use this so they don't snap on cell boundaries.
pub fn terrain_top_y(world_x: f32, world_z: f32, config: &WorldConfig) -> f32 {
    let xf = world_x.floor();
    let zf = world_z.floor();
    let xi = xf as i32;
    let zi = zf as i32;
    let tx = world_x - xf;
    let tz = world_z - zf;
    let h00 = terrain_surface_y_cell(xi, zi, config);
    let h10 = terrain_surface_y_cell(xi + 1, zi, config);
    let h01 = terrain_surface_y_cell(xi, zi + 1, config);
    let h11 = terrain_surface_y_cell(xi + 1, zi + 1, config);
    let a = h00 * (1.0 - tx) + h10 * tx;
    let b = h01 * (1.0 - tx) + h11 * tx;
    a * (1.0 - tz) + b * tz
}

/// March a ray against the procedural terrain. Returns the first
/// world-space surface hit, or `None` if no hit within `max_distance`.
pub fn terrain_raycast(ray: Ray3d, max_distance: f32, config: &WorldConfig) -> Option<Vec3> {
    let origin = ray.origin;
    let dir = *ray.direction;
    let mut prev_above = origin.y - terrain_top_y(origin.x, origin.z, config);
    if prev_above < 0.0 {
        return None;
    }
    const STEP: f32 = 0.5;
    let mut t = 0.0;
    while t < max_distance {
        t += STEP;
        let p = origin + dir * t;
        let above = p.y - terrain_top_y(p.x, p.z, config);
        if above <= 0.0 {
            let denom = prev_above - above;
            let frac = if denom.abs() > 1e-6 {
                prev_above / denom
            } else {
                0.0
            };
            let hit_t = t - STEP + frac * STEP;
            return Some(origin + dir * hit_t);
        }
        prev_above = above;
    }
    None
}

// === Internal: layered noise ===============================================

/// Large-scale rolling hills — the base elevation field.
fn base_height_at(x: f32, z: f32, config: &WorldConfig) -> f32 {
    let n = fbm(x * config.noise_scale, z * config.noise_scale);
    (n - 0.3) * config.height_scale
}

/// High-frequency, low-amplitude pebble noise added on top of the
/// base height so flat-ish areas still have visible variation.
fn bump_at(x: f32, z: f32) -> f32 {
    // Two octaves of fine noise, ±0.35 units total.
    let a = value_noise(x * 0.6, z * 0.6) - 0.5;
    let b = value_noise(x * 1.7 + 13.0, z * 1.7 + 7.0) - 0.5;
    a * 0.45 + b * 0.25
}

// === Hash-based 2D value noise (deterministic, no extra deps) ==============

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

/// Fractional Brownian motion: 4 octaves of value noise.
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
