//! On-screen joystick input (single-player / mobile only).
//!
//! Reads raw `Touches` (mobile) or `MouseButton::Left` + cursor position
//! (desktop). When a press lands inside the joystick base, emits a
//! `MoveIntent` for player 0 rotated by camera azimuth (same maths as
//! keyboard input).
//!
//! Gated to `GameState::PlayingSingle` in `ClientSimPlugin`.

use bevy::prelude::*;

use crate::client_sim::camera_view::CameraOrbit;
use crate::data::{GameCamera, JoystickState, MoveIntent, MoveTarget, PlayerSlot, Settings, joystick_layout};

const DEAD_ZONE: f32 = 0.18;

pub fn gather_joystick_input(
    settings: Res<Settings>,
    windows: Query<&Window>,
    touches: Res<Touches>,
    mouse: Res<ButtonInput<MouseButton>>,
    cameras: Query<(&CameraOrbit, &PlayerSlot), With<GameCamera>>,
    mut state: ResMut<JoystickState>,
    mut click_target: ResMut<MoveTarget>,
    mut writer: MessageWriter<MoveIntent>,
) {
    if !settings.joystick_enabled {
        if state.active || state.knob_offset != Vec2::ZERO {
            *state = JoystickState::default();
        }
        return;
    }
    let Ok(window) = windows.single() else { return };
    let h = window.height();
    let center = joystick_layout::screen_center(h);
    let radius = joystick_layout::BASE_RADIUS;

    let active_pos: Option<Vec2> = touches
        .iter()
        .map(|t| t.position())
        .find(|p| (*p - center).length() <= radius * 2.0)
        .or_else(|| {
            mouse.pressed(MouseButton::Left).then(|| ()).and_then(|_| {
                window
                    .cursor_position()
                    .filter(|p| (*p - center).length() <= radius * 2.0)
            })
        });

    let Some(pos) = active_pos else {
        if state.active || state.knob_offset != Vec2::ZERO {
            *state = JoystickState::default();
        }
        return;
    };

    let delta = pos - center;
    let dist = delta.length();
    let knob_offset = if dist > radius { delta.normalize() * radius } else { delta };
    state.knob_offset = knob_offset;
    state.active = true;
    click_target.0 = None;

    let local = Vec2::new(knob_offset.x / radius, -knob_offset.y / radius);
    if local.length() < DEAD_ZONE {
        return;
    }

    let azimuth = cameras
        .iter()
        .find(|&(_, &PlayerSlot(s))| s == 0)
        .map(|(orbit, _)| orbit.smoothed_azimuth)
        .unwrap_or(0.0);
    let forward = Vec2::new(-azimuth.sin(), -azimuth.cos());
    let right = Vec2::new(azimuth.cos(), -azimuth.sin());
    let world = local.x * right + local.y * forward;
    if world.length_squared() > 0.0 {
        writer.write(MoveIntent { direction: world.normalize(), player_slot: 0 });
    }
}
