//! Camera view-model — per-camera orbit and focus state.
//!
//! `CameraOrbit` and `CameraFocus` are **components** attached to each
//! `GameCamera` entity rather than global resources.  This lets single-
//! player (one camera) and split-screen multiplayer (two cameras) share
//! exactly the same systems: every system just iterates over cameras.
//!
//! Two pieces of smoothing ensure the camera stays on its arc:
//!
//! 1. **Orbit smoothing** — `target_azimuth` (set by Q/E intents) is
//!    smoothed into `smoothed_azimuth` per frame.
//! 2. **Focus smoothing** — `CameraFocus` lerps toward the matching
//!    player's position so movement feels like inertia.
//!
//! `compute_camera_view` then derives `DesiredCameraView` (also a
//! component on each camera) from these smoothed values.  The render
//! layer snaps `Camera3d.Transform` to `DesiredCameraView` — no further
//! smoothing there.

use bevy::prelude::*;

use crate::data::{CameraOrbitIntent, ClientTransform, DesiredCameraView, GameCamera, Player, PlayerSlot, Settings, WorldConfig};

/// Per-camera orbit state.  0 rad = camera on +Z, π/2 = camera on +X.
/// Positive azimuth = clockwise when viewed from above.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct CameraOrbit {
    pub target_azimuth: f32,
    pub smoothed_azimuth: f32,
}

/// Per-camera focus with predictive lookahead and velocity smoothing.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct CameraFocus {
    pub pos: Vec3,
    pub smoothed_velocity: Vec2,
    pub last_player_pos: Option<Vec3>,
}

// ── Systems ─────────────────────────────────────────────────────────────────

/// Q/E (P1) or U/O (P2) intents nudge the matching camera's
/// `target_azimuth`.
pub fn apply_camera_orbit(
    mut events: MessageReader<CameraOrbitIntent>,
    settings: Res<Settings>,
    time: Res<Time>,
    mut cameras: Query<(&mut CameraOrbit, &PlayerSlot), With<GameCamera>>,
) {
    // Accumulate per-slot delta across all intents this frame.
    let mut deltas = [0.0f32; 2];
    for e in events.read() {
        let idx = e.player_slot.min(1) as usize;
        deltas[idx] += e.delta;
    }
    for (mut orbit, &PlayerSlot(slot)) in &mut cameras {
        let d = deltas[slot.min(1) as usize];
        if d != 0.0 {
            orbit.target_azimuth += d * settings.camera_orbit_speed * time.delta_secs();
        }
    }
}

/// Frame-rate-independent exponential smoothing of each camera's
/// `smoothed_azimuth` toward `target_azimuth` (shortest angular path).
pub fn smooth_camera_orbit(
    time: Res<Time>,
    settings: Res<Settings>,
    mut cameras: Query<&mut CameraOrbit, With<GameCamera>>,
) {
    let alpha = 1.0 - (-settings.camera_lerp_speed * time.delta_secs()).exp();
    for mut orbit in &mut cameras {
        orbit.smoothed_azimuth =
            lerp_angle_shortest(orbit.smoothed_azimuth, orbit.target_azimuth, alpha);
    }
}

/// Smooth each camera's focal point toward its matching player's
/// `ClientTransform` — the network-delayed position.  This ensures
/// the camera follows the same lagged position the billboard renders
/// at, giving the full 200 ms round-trip feel in multiplayer.
pub fn smooth_camera_focus(
    time: Res<Time>,
    settings: Res<Settings>,
    config: Res<WorldConfig>,
    players: Query<(&ClientTransform, &PlayerSlot), With<Player>>,
    mut cameras: Query<(&mut CameraFocus, &PlayerSlot), With<GameCamera>>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    let max_speed = config.player_speed * 2.0;

    for (mut focus, &PlayerSlot(cam_slot)) in &mut cameras {
        let player_translation = players
            .iter()
            .find(|&(_, &PlayerSlot(s))| s == cam_slot)
            .or_else(|| players.iter().next())
            .map(|(ct, _)| ct.0.translation);
        let Some(p) = player_translation else {
            continue;
        };

        let instantaneous = match focus.last_player_pos {
            Some(prev) => Vec2::new(
                (p.x - prev.x) / dt,
                (p.z - prev.z) / dt,
            ),
            None => Vec2::ZERO,
        };
        focus.last_player_pos = Some(p);

        let clamped = if instantaneous.length() > max_speed {
            instantaneous.normalize() * max_speed
        } else {
            instantaneous
        };

        let vel_alpha = 1.0 - (-settings.camera_velocity_smoothing * dt).exp();
        focus.smoothed_velocity = focus.smoothed_velocity.lerp(clamped, vel_alpha);

        let lookahead = focus.smoothed_velocity * settings.camera_lookahead;
        let target = p + Vec3::new(lookahead.x, 0.0, lookahead.y);

        let focus_alpha = 1.0 - (-settings.camera_lerp_speed * dt).exp();
        focus.pos = focus.pos.lerp(target, focus_alpha);
    }
}

/// Build each camera's `DesiredCameraView` from its smoothed orbit + focus.
pub fn compute_camera_view(
    settings: Res<Settings>,
    mut cameras: Query<(&CameraOrbit, &CameraFocus, &mut DesiredCameraView), With<GameCamera>>,
) {
    for (orbit, focus, mut view) in &mut cameras {
        let a = orbit.smoothed_azimuth;
        let dist = settings.camera_distance;
        let offset = Vec3::new(a.sin() * dist, settings.camera_height, a.cos() * dist);
        view.position = focus.pos + offset;
        view.look_at = focus.pos;
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Lerp current → target along the shortest angular path.
fn lerp_angle_shortest(current: f32, target: f32, alpha: f32) -> f32 {
    use std::f32::consts::{PI, TAU};
    let mut diff = (target - current) % TAU;
    if diff > PI {
        diff -= TAU;
    } else if diff < -PI {
        diff += TAU;
    }
    current + diff * alpha
}
