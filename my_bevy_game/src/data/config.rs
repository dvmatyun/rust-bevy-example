//! World tuning constants and the terrain heightmap resource.

use bevy::prelude::*;

/// Tunables for world generation and player physics. Static during play.
#[derive(Resource, Clone, Copy, Debug)]
pub struct WorldConfig {
    /// World is `size x size` voxel columns, centred on origin.
    pub size: i32,
    /// Smaller = larger features.
    pub noise_scale: f32,
    /// Peak height in blocks.
    pub height_scale: f32,
    pub player_speed: f32,
    pub player_half_height: f32,
}

impl Default for WorldConfig {
    fn default() -> Self {
        Self {
            size: 64,
            noise_scale: 0.08,
            height_scale: 9.0,
            player_speed: 8.0,
            player_half_height: 0.7,
        }
    }
}

/// User-tunable preferences. Edited at runtime via the settings UI.
///
/// Lives in `data/` because it's read by `client_sim` (camera framing,
/// orbit speed) and `render` (camera lerp), and written by `render`
/// (settings UI buttons).
#[derive(Resource, Clone, Copy, Debug)]
pub struct Settings {
    /// How quickly the rendered camera tweens toward `DesiredCameraView`.
    /// Higher = snappier; lower = floatier inertia. Range \[1.0, 20.0\].
    pub camera_lerp_speed: f32,
    /// Q/E camera orbit speed in radians/second. Range \[0.5, 5.0\].
    pub camera_orbit_speed: f32,
    /// Horizontal distance from player to camera. Range \[5.0, 30.0\].
    pub camera_distance: f32,
    /// Camera height above player. Range \[3.0, 30.0\].
    pub camera_height: f32,
    /// **Predictive lookahead in seconds.** The camera focal point is
    /// `player + smoothed_velocity * lookahead`, so during steady
    /// movement the camera leads the player by `velocity * lookahead`,
    /// cancelling the natural lag of the focus lerp. Setting this to
    /// `1 / camera_lerp_speed` (≈ 0.25 with default speed 4.0) keeps
    /// the player exactly centred at constant speed. Range
    /// \[0.0, 1.0\] — 0 reverts to "always behind" lag.
    pub camera_lookahead: f32,
    /// **How fast the velocity estimate adapts** (rate constant for
    /// the exponential filter on instantaneous player velocity). Higher
    /// = prediction snaps to direction changes immediately (responsive
    /// but jittery on noisy motion); lower = prediction lags
    /// direction changes (stable but the "drag behind" lasts longer
    /// after starting/stopping). Range \[1.0, 20.0\].
    pub camera_velocity_smoothing: f32,
    /// Whether the on-screen joystick (bottom-left) is shown and
    /// processes touch / mouse input. Defaults to true on mobile,
    /// false on desktop.
    pub joystick_enabled: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            camera_lerp_speed: 4.0,
            camera_orbit_speed: 2.0,
            camera_distance: 14.0,
            camera_height: 14.0,
            camera_lookahead: 0.25,
            camera_velocity_smoothing: 4.0,
            joystick_enabled: cfg!(any(target_os = "android", target_os = "ios")),
        }
    }
}

/// Screen-space layout for the on-screen joystick. Constants live here
/// so both `client_sim` (for hit-testing) and `render` (for placing UI
/// nodes) reference the exact same numbers — no drift.
pub mod joystick_layout {
    use bevy::math::Vec2;

    /// Distance from the left window edge to the joystick centre (px).
    pub const CENTER_X_FROM_LEFT: f32 = 100.0;
    /// Distance from the bottom window edge to the joystick centre (px).
    pub const CENTER_Y_FROM_BOTTOM: f32 = 130.0;
    /// Visible base ring radius (px) — also the maximum knob deflection.
    pub const BASE_RADIUS: f32 = 70.0;
    /// Visible knob radius (px).
    pub const KNOB_RADIUS: f32 = 26.0;

    /// Joystick centre in window pixel coordinates (origin = top-left,
    /// y grows downward — matches `Window::cursor_position()` and
    /// `Touch::position()`).
    pub fn screen_center(window_height: f32) -> Vec2 {
        Vec2::new(CENTER_X_FROM_LEFT, window_height - CENTER_Y_FROM_BOTTOM)
    }

    /// True iff `point` lies within the joystick's base circle.
    pub fn contains(point: Vec2, window_height: f32) -> bool {
        (point - screen_center(window_height)).length() <= BASE_RADIUS
    }
}

/// Surface height per (x, z) world cell, populated by the server during
/// startup. Read by gameplay (snap-to-ground) and arbitrary tools.
#[derive(Resource, Default)]
pub struct TerrainHeights {
    pub size: i32,
    /// Row-major: `cells[(z + size/2) * size + (x + size/2)]`.
    pub cells: Vec<i32>,
}

impl TerrainHeights {
    pub fn at(&self, x: i32, z: i32) -> i32 {
        if self.cells.is_empty() {
            return 0;
        }
        let half = self.size / 2;
        let lx = (x + half).clamp(0, self.size - 1) as usize;
        let lz = (z + half).clamp(0, self.size - 1) as usize;
        self.cells[lz * self.size as usize + lx]
    }

    /// World-space Y of the top of the surface block at this XZ.
    pub fn ground_y(&self, world_x: f32, world_z: f32) -> f32 {
        let xi = world_x.round() as i32;
        let zi = world_z.round() as i32;
        // Block centred at y=h, top face at y=h+0.5
        self.at(xi, zi) as f32 + 0.5
    }

    /// March a ray against the heightmap and return the first
    /// world-space surface intersection, if any.
    ///
    /// Used by click-to-move: projecting the click onto a flat y=0
    /// plane biases the target toward the camera's far side when the
    /// player clicks on a hill. Marching the actual surface lands the
    /// target where the user pointed.
    pub fn raycast(&self, ray: Ray3d, max_distance: f32) -> Option<Vec3> {
        if self.cells.is_empty() {
            return None;
        }
        let origin = ray.origin;
        let dir = *ray.direction;
        // Camera underground / starting inside a hill — no useful hit.
        let mut prev_above = origin.y - self.ground_y(origin.x, origin.z);
        if prev_above < 0.0 {
            return None;
        }
        const STEP: f32 = 0.5; // half a block — fine enough for orbiting cameras
        let mut t = 0.0;
        while t < max_distance {
            t += STEP;
            let p = origin + dir * t;
            let above = p.y - self.ground_y(p.x, p.z);
            if above <= 0.0 {
                // Crossed the surface between the previous sample
                // (above ≥ 0) and this one (below ≤ 0). Linearly
                // interpolate to estimate the crossing point.
                let denom = prev_above - above;
                let frac = if denom.abs() > 1e-6 { prev_above / denom } else { 0.0 };
                let hit_t = t - STEP + frac * STEP;
                return Some(origin + dir * hit_t);
            }
            prev_above = above;
        }
        None
    }
}
