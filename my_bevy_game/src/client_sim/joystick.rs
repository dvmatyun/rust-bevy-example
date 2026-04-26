//! On-screen joystick input.
//!
//! Reads raw `Touches` (mobile) or `MouseButton::Left` + cursor
//! position (desktop). When a press lands inside the joystick base, we
//! treat that pointer as "driving" the joystick: emit a `MoveIntent`
//! camera-rotated like keyboard, and update `JoystickState` so the
//! render layer can draw the knob deflection.
//!
//! Disabled when `Settings.joystick_enabled = false` — the system
//! becomes a cheap no-op that only resets the state once.

use bevy::prelude::*;

use crate::client_sim::camera_view::CameraOrbit;
use crate::data::{JoystickState, MoveIntent, MoveTarget, Settings, joystick_layout};

/// Below this fraction of full deflection we don't emit a MoveIntent
/// (visual deflection still updates so the user sees feedback).
const DEAD_ZONE: f32 = 0.18;

pub fn gather_joystick_input(
    settings: Res<Settings>,
    windows: Query<&Window>,
    touches: Res<Touches>,
    mouse: Res<ButtonInput<MouseButton>>,
    orbit: Res<CameraOrbit>,
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
    let Ok(window) = windows.single() else {
        return;
    };
    let h = window.height();
    let center = joystick_layout::screen_center(h);
    let radius = joystick_layout::BASE_RADIUS;

    // A "driving" pointer is one whose CURRENT position is within
    // ~2× the base radius of the joystick centre. That generous bound
    // means dragging the finger outside the base ring still controls
    // the joystick (deflection is clamped at base radius).
    let active_pos: Option<Vec2> = touches
        .iter()
        .map(|t| t.position())
        .find(|p| (*p - center).length() <= radius * 2.0)
        .or_else(|| {
            mouse.pressed(MouseButton::Left).then(|| ()).and_then(|_| {
                window.cursor_position().filter(|p| (*p - center).length() <= radius * 2.0)
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
    let knob_offset = if dist > radius {
        delta.normalize() * radius
    } else {
        delta
    };
    state.knob_offset = knob_offset;
    state.active = true;

    // Joystick takes precedence over click-to-move.
    click_target.0 = None;

    // Screen-space → camera-local (forward = screen-up because screen y
    // grows downward). Then rotate by camera azimuth like keyboard.
    let local = Vec2::new(knob_offset.x / radius, -knob_offset.y / radius);
    if local.length() < DEAD_ZONE {
        return;
    }
    let a = orbit.smoothed_azimuth;
    let forward = Vec2::new(-a.sin(), -a.cos());
    let right = Vec2::new(a.cos(), -a.sin());
    let world = local.x * right + local.y * forward;
    if world.length_squared() > 0.0 {
        writer.write(MoveIntent {
            direction: world.normalize(),
        });
    }
}
