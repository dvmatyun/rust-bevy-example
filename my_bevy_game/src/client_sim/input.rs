//! Translates raw player input into game / view intents.
//!
//! Allowed: `ButtonInput<KeyCode>`, `Touches`. NOT allowed: writing to
//! game-state Transforms directly — emit a `MoveIntent` (server) or
//! `CameraOrbitIntent` (client view) instead.

use bevy::prelude::*;

use crate::client_sim::camera_view::CameraOrbit;
use crate::data::{CameraOrbitIntent, MoveIntent, MoveTarget};

/// WASD / arrow keys in *camera-relative* directions:
///   W = away from camera ("forward")
///   S = toward camera ("back")
///   A = left of camera   D = right of camera
///
/// Pressing any movement key cancels an outstanding click-to-move
/// target so WASD always wins.
pub fn gather_move_input(
    // 📘 ButtonInput<KeyCode> is a *resource* tracking which keys
    // are currently pressed / just-pressed / just-released. Bevy
    // updates it from the OS input events each frame.
    keys: Res<ButtonInput<KeyCode>>,
    orbit: Res<CameraOrbit>,
    mut click_target: ResMut<MoveTarget>,
    // 📘 MessageWriter<T> is the "send" half of the messaging system.
    // `.write(T)` enqueues an event for any reader (here:
    // `apply_movement` in the server).
    mut writer: MessageWriter<MoveIntent>,
) {
    // Camera-local input: forward = +Y_local, right = +X_local.
    let mut local = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        local.y += 1.0;
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        local.y -= 1.0;
    }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        local.x -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        local.x += 1.0;
    }
    if local == Vec2::ZERO {
        return;
    }

    // Keyboard cancels click-to-move.
    click_target.0 = None;

    let local = local.normalize();
    let a = orbit.smoothed_azimuth;
    // 📘 The math below converts "what the user pressed in screen
    // space" into "world XZ direction the player should move".
    //
    // Camera position (offset from focus) = `(sin a, _, cos a) * dist`.
    // So the *direction from camera to player* (in XZ) is the
    // negative: `(-sin a, -cos a)`. That's "forward" (away from camera,
    // i.e. away from the screen).
    let forward = Vec2::new(-a.sin(), -a.cos());
    // "Right" is forward rotated 90° clockwise (when viewed from above).
    // The 2D rotation matrix for -π/2 applied to forward gives this.
    let right = Vec2::new(a.cos(), -a.sin());
    // Linear combination: project the local input onto world axes.
    let world = local.x * right + local.y * forward;
    writer.write(MoveIntent {
        direction: world,
    });
}

/// Q/E rotates the camera around the player. Q = counter-clockwise,
/// E = clockwise (looking from above).
pub fn gather_camera_orbit_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut writer: MessageWriter<CameraOrbitIntent>,
) {
    let mut delta = 0.0;
    if keys.pressed(KeyCode::KeyQ) {
        delta -= 1.0;
    }
    if keys.pressed(KeyCode::KeyE) {
        delta += 1.0;
    }
    if delta != 0.0 {
        writer.write(CameraOrbitIntent { delta });
    }
}
