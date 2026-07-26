//! Translates raw player input into game / view intents.
//!
//! In `PlayingSingle` mode only P1 systems run and intents are written
//! directly to `MoveIntent`.
//!
//! In `PlayingMultiplayer` mode both P1 and P2 systems run and movement
//! intents are written to `BufferedMoveIntent` so the network-simulation
//! layer can delay them ~100 ms before the server sees them.

use bevy::prelude::*;

use crate::client_sim::camera_view::CameraOrbit;
use crate::data::{
    BufferedMoveIntent, CameraOrbitIntent, GameCamera, GameState, MoveIntent, MoveTarget,
    PlayerSlot, TeleportIntent,
};

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Returns the smoothed azimuth of the camera for the given `slot`,
/// or 0.0 if no camera exists yet (first frame of a new game session).
fn camera_azimuth(
    cameras: &Query<(&CameraOrbit, &PlayerSlot), With<GameCamera>>,
    slot: u8,
) -> f32 {
    cameras
        .iter()
        .find(|&(_, &PlayerSlot(s))| s == slot)
        .map(|(orbit, _)| orbit.smoothed_azimuth)
        .unwrap_or(0.0)
}

/// Convert a local (screen-relative) 2D input vector and a camera
/// azimuth into a world-XZ direction.
fn local_to_world(local: Vec2, azimuth: f32) -> Vec2 {
    let forward = Vec2::new(-azimuth.sin(), -azimuth.cos());
    let right = Vec2::new(azimuth.cos(), -azimuth.sin());
    local.x * right + local.y * forward
}

// ── Player-1 systems (WASD / Q-E / T) ───────────────────────────────────────

/// WASD / arrow keys → camera-relative world direction for player 1.
///
/// In `PlayingSingle`, emits `MoveIntent` directly.
/// In `PlayingMultiplayer`, emits `BufferedMoveIntent` so the delay
/// simulation can introduce ~100 ms of latency.
pub fn gather_move_input_p1(
    keys: Res<ButtonInput<KeyCode>>,
    cameras: Query<(&CameraOrbit, &PlayerSlot), With<GameCamera>>,
    state: Res<State<GameState>>,
    mut click_target: ResMut<MoveTarget>,
    mut direct: MessageWriter<MoveIntent>,
    mut buffered: MessageWriter<BufferedMoveIntent>,
) {
    let mut local = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyW) {
        local.y += 1.0;
    }
    if keys.pressed(KeyCode::KeyS) {
        local.y -= 1.0;
    }
    if keys.pressed(KeyCode::KeyA) {
        local.x -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) {
        local.x += 1.0;
    }
    if local == Vec2::ZERO {
        return;
    }
    click_target.0 = None;
    let world = local_to_world(local.normalize(), camera_azimuth(&cameras, 0));
    if *state.get() == GameState::PlayingMultiplayer {
        buffered.write(BufferedMoveIntent { direction: world, player_slot: 0 });
    } else {
        direct.write(MoveIntent { direction: world, player_slot: 0 });
    }
}

/// Player-2 movement: arrow keys → camera-relative world direction.
/// Always writes `BufferedMoveIntent` (only active in multiplayer).
pub fn gather_move_input_p2(
    keys: Res<ButtonInput<KeyCode>>,
    cameras: Query<(&CameraOrbit, &PlayerSlot), With<GameCamera>>,
    mut buffered: MessageWriter<BufferedMoveIntent>,
) {
    let mut local = Vec2::ZERO;
    if keys.pressed(KeyCode::ArrowUp) {
        local.y += 1.0;
    }
    if keys.pressed(KeyCode::ArrowDown) {
        local.y -= 1.0;
    }
    if keys.pressed(KeyCode::ArrowLeft) {
        local.x -= 1.0;
    }
    if keys.pressed(KeyCode::ArrowRight) {
        local.x += 1.0;
    }
    if local == Vec2::ZERO {
        return;
    }
    let world = local_to_world(local.normalize(), camera_azimuth(&cameras, 1));
    buffered.write(BufferedMoveIntent { direction: world, player_slot: 1 });
}

/// Q / E orbit player-1's camera. Never buffered (local view operation).
pub fn gather_camera_orbit_input_p1(
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
        writer.write(CameraOrbitIntent { delta, player_slot: 0 });
    }
}

/// U / O orbit player-2's camera. Only active in multiplayer.
pub fn gather_camera_orbit_input_p2(
    keys: Res<ButtonInput<KeyCode>>,
    mut writer: MessageWriter<CameraOrbitIntent>,
) {
    let mut delta = 0.0;
    if keys.pressed(KeyCode::KeyU) {
        delta -= 1.0;
    }
    if keys.pressed(KeyCode::KeyO) {
        delta += 1.0;
    }
    if delta != 0.0 {
        writer.write(CameraOrbitIntent { delta, player_slot: 1 });
    }
}

/// T = teleport up 3 units, Shift+T = down. Debug only; player 0.
pub fn gather_teleport_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut writer: MessageWriter<TeleportIntent>,
) {
    if keys.just_pressed(KeyCode::KeyT) {
        let down = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
        let delta_y = if down { -3.0 } else { 3.0 };
        writer.write(TeleportIntent { delta_y, player_slot: 0 });
    }
}
