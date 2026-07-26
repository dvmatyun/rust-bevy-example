//! Mouse / touch click → world XZ raycast → `ClickMoveIntent`.
//!
//! This is the one input handler that lives in the render layer
//! instead of `client_sim`. Reason: unprojecting screen → world
//! requires the actual `Camera` component (projection matrix +
//! viewport). See `docsai/architecture.md` ("Decision log → Why
//! screen-space input lives in render").

use bevy::prelude::*;

use crate::data::{terrain_gen, ClickMoveIntent, GameCamera, Settings, WorldConfig, joystick_layout};

/// Maximum march distance in world units. Camera is typically ≤ 30
/// units away from any terrain cell, so 200 is a generous bound.
const MAX_RAY_DISTANCE: f32 = 200.0;

pub fn handle_click_input(
    mouse: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform), With<GameCamera>>,
    config: Res<WorldConfig>,
    settings: Res<Settings>,
    mut writer: MessageWriter<ClickMoveIntent>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let window_h = window.height();
    let in_joystick =
        |p: Vec2| settings.joystick_enabled && joystick_layout::contains(p, window_h);

    // Collect all "click points" from this frame, dropping any that
    // started inside the on-screen joystick (those are consumed by
    // `client_sim::joystick::gather_joystick_input`).
    let mut click_points: Vec<Vec2> = Vec::new();

    if mouse.just_pressed(MouseButton::Left) {
        if let Some(p) = window.cursor_position() {
            if !in_joystick(p) {
                click_points.push(p);
            }
        }
    }
    for t in touches.iter_just_pressed() {
        let p = t.position();
        if !in_joystick(p) {
            click_points.push(p);
        }
    }

    if click_points.is_empty() {
        return;
    }

    let Some((camera, cam_tf)) = cameras.iter().next() else {
        return;
    };

    for pos in click_points {
        let Ok(ray) = camera.viewport_to_world(cam_tf, pos) else {
            continue;
        };
        // Ray-march against the actual heightmap so a click on a hill
        // lands on the hill's surface (not far behind it on a flat
        // y = 0 projection).
        if let Some(hit) = terrain_gen::terrain_raycast(ray, MAX_RAY_DISTANCE, &config) {
            writer.write(ClickMoveIntent {
                target: Vec2::new(hit.x, hit.z),
            });
        }
    }
}
