//! Camera view-model.
//!
//! Two pieces of smoothing live here so the camera always stays on its
//! orbit arc and never lerps in a straight line through 3D space:
//!
//! 1. **Orbit smoothing** — `target_azimuth` (set by Q/E intents) is
//!    smoothed into `smoothed_azimuth` per frame.
//! 2. **Focus smoothing** — `CameraFocus` lerps toward the player's
//!    current position so player movement is felt as inertia rather
//!    than a hard-locked follow.
//!
//! `compute_camera_view` then derives `DesiredCameraView` from these
//! smoothed inputs. Because the camera is reconstructed from the arc
//! every frame, it is always on the correct radius from the focus.
//!
//! Render layer simply assigns `DesiredCameraView` to the `Camera3d`
//! transform — no further smoothing.

use bevy::prelude::*;

use crate::data::{CameraOrbitIntent, DesiredCameraView, Player, Settings, WorldConfig};

/// Client-only state: target and smoothed azimuth around the player.
/// 0 = camera on +Z; π/2 = camera on +X (positive azimuth = clockwise
/// when viewed from above).
#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct CameraOrbit {
    pub target_azimuth: f32,
    pub smoothed_azimuth: f32,
}

/// Smoothed focal point with predictive lookahead.
///
/// `pos` lerps toward `player.translation + smoothed_velocity *
/// camera_lookahead`. The `smoothed_velocity` (XZ only) is itself an
/// exponentially-smoothed estimate of frame-to-frame player motion.
/// Combined, the camera lags briefly when the player starts moving
/// (velocity estimate is still ramping up) but settles centred on
/// the player during steady motion.
#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct CameraFocus {
    pub pos: Vec3,
    /// Smoothed estimate of player velocity in the XZ plane (units/sec).
    pub smoothed_velocity: Vec2,
    /// Last-frame player position, used to estimate instantaneous
    /// velocity. `None` until the second frame.
    pub last_player_pos: Option<Vec3>,
}

/// Q/E intents nudge `target_azimuth`.
pub fn apply_camera_orbit(
    mut events: MessageReader<CameraOrbitIntent>,
    settings: Res<Settings>,
    time: Res<Time>,
    mut orbit: ResMut<CameraOrbit>,
) {
    let mut total = 0.0;
    for e in events.read() {
        total += e.delta;
    }
    if total != 0.0 {
        orbit.target_azimuth += total * settings.camera_orbit_speed * time.delta_secs();
    }
}

/// Frame-rate-independent exponential smoothing of `smoothed_azimuth`
/// toward `target_azimuth`, taking the shortest angular path.
pub fn smooth_camera_orbit(
    time: Res<Time>,
    settings: Res<Settings>,
    mut orbit: ResMut<CameraOrbit>,
) {
    let alpha = 1.0 - (-settings.camera_lerp_speed * time.delta_secs()).exp();
    orbit.smoothed_azimuth =
        lerp_angle_shortest(orbit.smoothed_azimuth, orbit.target_azimuth, alpha);
}

/// Smooth the focal point toward `player.translation + smoothed_velocity
/// * lookahead`. The lookahead term predicts the player's future
/// position and cancels the lag of the focus lerp during steady
/// motion (camera stays centred). When the player suddenly starts /
/// stops / changes direction, the velocity estimate adapts at rate
/// `camera_velocity_smoothing`, so the camera briefly lags and then
/// catches up — exactly the requested behaviour.
pub fn smooth_camera_focus(
    time: Res<Time>,
    settings: Res<Settings>,
    config: Res<WorldConfig>,
    player: Query<&Transform, With<Player>>,
    mut focus: ResMut<CameraFocus>,
) {
    let Ok(p) = player.single() else {
        return;
    };
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }

    // Frame-to-frame velocity estimate (XZ plane only — Y comes from
    // snap_to_ground and isn't a meaningful gameplay velocity).
    let instantaneous = match focus.last_player_pos {
        Some(prev) => Vec2::new(
            (p.translation.x - prev.x) / dt,
            (p.translation.z - prev.z) / dt,
        ),
        None => Vec2::ZERO,
    };
    focus.last_player_pos = Some(p.translation);

    // Cap to a sane multiple of player speed to absorb teleports /
    // ground-snap jumps without shooting the camera off into space.
    let max_speed = config.player_speed * 2.0;
    let clamped = if instantaneous.length() > max_speed {
        instantaneous.normalize() * max_speed
    } else {
        instantaneous
    };

    // Exponential filter on velocity.
    let vel_alpha = 1.0 - (-settings.camera_velocity_smoothing * dt).exp();
    focus.smoothed_velocity = focus.smoothed_velocity.lerp(clamped, vel_alpha);

    // Predicted look-at point: where the player will be in
    // `lookahead` seconds at current smoothed velocity.
    let lookahead = focus.smoothed_velocity * settings.camera_lookahead;
    let target = p.translation + Vec3::new(lookahead.x, 0.0, lookahead.y);

    // Tween the focus toward the predicted target.
    let focus_alpha = 1.0 - (-settings.camera_lerp_speed * dt).exp();
    focus.pos = focus.pos.lerp(target, focus_alpha);
}

/// Build `DesiredCameraView` from smoothed orbit + focus.
pub fn compute_camera_view(
    settings: Res<Settings>,
    orbit: Res<CameraOrbit>,
    focus: Res<CameraFocus>,
    mut view: ResMut<DesiredCameraView>,
) {
    let a = orbit.smoothed_azimuth;
    let dist = settings.camera_distance;
    let offset = Vec3::new(a.sin() * dist, settings.camera_height, a.cos() * dist);
    view.position = focus.pos + offset;
    view.look_at = focus.pos;
}

/// Lerp current → target along the shortest angular path (handles
/// 2π wrap-around correctly).
///
/// 📘 Naïve `lerp(0.1, 6.2, 0.5)` would interpolate "the long way"
/// around the circle (almost 6 radians). We want the short way (about
/// 0.2 radians the *other* direction). The `% TAU` and the bracketing
/// to `(-π, π]` produce the signed shortest difference.
fn lerp_angle_shortest(current: f32, target: f32, alpha: f32) -> f32 {
    // 📘 `use` statements can be local to a function — these
    // constants are only needed here, so we don't pollute the file
    // with a top-level import.
    use std::f32::consts::{PI, TAU};
    let mut diff = (target - current) % TAU;
    if diff > PI {
        diff -= TAU;
    } else if diff < -PI {
        diff += TAU;
    }
    current + diff * alpha
}
