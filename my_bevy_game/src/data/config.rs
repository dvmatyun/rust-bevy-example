//! World tuning constants and user-tunable settings.

use bevy::prelude::*;

/// Tunables for world generation and player physics. Static during play.
///
/// Note: `world_radius_cells` is the *world boundary* — beyond it the
/// player is clamped. Larger = bigger explorable world. The value is
/// also used by chunk loading to know when to stop spawning chunks.
#[derive(Resource, Clone, Copy, Debug)]
pub struct WorldConfig {
    /// Half-extent of the world in cells. Total cells = `(2 * radius)^2`.
    pub world_radius_cells: i32,
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
            world_radius_cells: 32, // 64x64 cells default
            noise_scale: 0.08,
            height_scale: 9.0,
            player_speed: 8.0,
            player_half_height: 0.7,
        }
    }
}

/// User-tunable preferences. Edited at runtime via the settings UI.
#[derive(Resource, Clone, Copy, Debug)]
pub struct Settings {
    // ── Camera ────────────────────────────────────────────────────────
    pub camera_lerp_speed: f32,
    pub camera_orbit_speed: f32,
    pub camera_distance: f32,
    pub camera_height: f32,
    pub camera_lookahead: f32,
    pub camera_velocity_smoothing: f32,
    // ── Input ─────────────────────────────────────────────────────────
    pub joystick_enabled: bool,
    // ── Graphics ──────────────────────────────────────────────────────
    /// Enable real-time directional shadows. Adds significant cost on
    /// mobile (Mali GPUs in particular — see android-debugging-log).
    pub shadows_enabled: bool,
    /// Enable distance fog (atmospheric haze hiding chunk pop-in).
    pub fog_enabled: bool,
    /// How many chunks (each `CHUNK_SIZE`²) around the camera to keep
    /// loaded. Doubles as the LOD radius.
    pub render_distance_chunks: i32,
    // ── Physics ───────────────────────────────────────────────────────
    /// Master toggle for all collision response (walls + bodies). Off
    /// reverts to the previous "ghost everything" behaviour.
    pub collisions_enabled: bool,
    /// Cylinder radius used for the player ↔ wall and player ↔ body
    /// collision tests. ~0.4 ≈ shoulder width at our scale.
    pub player_radius: f32,
    /// Cylinder radius for monster ↔ wall and ↔ body tests.
    pub monster_radius: f32,
    /// Draw wall AABBs and body cylinders as gizmo wireframes so the
    /// player can see exactly what the collision system sees.
    pub debug_collisions: bool,
    // ── World ─────────────────────────────────────────────────────────
    /// Sun-direction azimuth (degrees, 0 = sun in +X direction).
    pub sun_azimuth_deg: f32,
    /// Sun-direction elevation above horizon (degrees, 90 = directly above).
    pub sun_elevation_deg: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            // Camera
            camera_lerp_speed: 4.0,
            camera_orbit_speed: 2.0,
            camera_distance: 14.0,
            camera_height: 14.0,
            camera_lookahead: 0.25,
            camera_velocity_smoothing: 4.0,
            // Input
            joystick_enabled: cfg!(any(target_os = "android", target_os = "ios")),
            // Graphics
            shadows_enabled: !cfg!(target_os = "android"), // off on Mali
            fog_enabled: true,
            render_distance_chunks: 4,
            // Physics
            collisions_enabled: true,
            player_radius: 0.4,
            monster_radius: 0.45,
            debug_collisions: false,
            // Sun
            sun_azimuth_deg: 60.0,
            sun_elevation_deg: 60.0,
        }
    }
}

// ── Joystick screen layout (shared by client_sim hit-test + render UI) ──
pub mod joystick_layout {
    use bevy::math::Vec2;
    pub const CENTER_X_FROM_LEFT: f32 = 100.0;
    pub const CENTER_Y_FROM_BOTTOM: f32 = 130.0;
    pub const BASE_RADIUS: f32 = 70.0;
    pub const KNOB_RADIUS: f32 = 26.0;

    pub fn screen_center(window_height: f32) -> Vec2 {
        Vec2::new(CENTER_X_FROM_LEFT, window_height - CENTER_Y_FROM_BOTTOM)
    }
    pub fn contains(point: Vec2, window_height: f32) -> bool {
        (point - screen_center(window_height)).length() <= BASE_RADIUS
    }
}

/// Cell-grid chunk size for terrain rendering. Each chunk is one
/// mesh; chunks within `Settings.render_distance_chunks` of the camera
/// are kept loaded.
pub const CHUNK_SIZE: i32 = 32;
