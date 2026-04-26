//! Visible on-screen joystick (bottom-left).
//!
//! Two UI nodes:
//! - **Base** — large translucent ring at a fixed bottom-left offset.
//!   Visibility tracks `Settings.joystick_enabled`.
//! - **Knob** — smaller circle inside the base. Position follows
//!   `JoystickState.knob_offset` (written by
//!   `client_sim::joystick::gather_joystick_input`).
//!
//! The knob is positioned absolutely within the base so we can place
//! it precisely from the centre regardless of flex layout.

use bevy::prelude::*;

use crate::data::{JoystickState, Settings, joystick_layout};

#[derive(Component)]
pub(crate) struct JoystickBase;

#[derive(Component)]
pub(crate) struct JoystickKnob;

/// Where the centre of the knob sits when the joystick is idle —
/// constant across frames, expressed as a `left`/`top` inside the
/// base. (base_radius - knob_radius) places the knob centred.
const KNOB_REST_OFFSET: f32 =
    joystick_layout::BASE_RADIUS - joystick_layout::KNOB_RADIUS;

const BASE_DIAMETER: f32 = joystick_layout::BASE_RADIUS * 2.0;
const KNOB_DIAMETER: f32 = joystick_layout::KNOB_RADIUS * 2.0;

pub(crate) fn spawn_joystick(mut commands: Commands) {
    commands
        .spawn((
            JoystickBase,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(joystick_layout::CENTER_X_FROM_LEFT - joystick_layout::BASE_RADIUS),
                bottom: Val::Px(joystick_layout::CENTER_Y_FROM_BOTTOM - joystick_layout::BASE_RADIUS),
                width: Val::Px(BASE_DIAMETER),
                height: Val::Px(BASE_DIAMETER),
                border_radius: BorderRadius::all(Val::Px(joystick_layout::BASE_RADIUS)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.30)),
            // Initial visibility set by `update_joystick_visibility`
            // on the first Update.
        ))
        .with_child((
            JoystickKnob,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(KNOB_REST_OFFSET),
                top: Val::Px(KNOB_REST_OFFSET),
                width: Val::Px(KNOB_DIAMETER),
                height: Val::Px(KNOB_DIAMETER),
                border_radius: BorderRadius::all(Val::Px(joystick_layout::KNOB_RADIUS)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.95, 0.95, 0.95, 0.55)),
        ));
}

pub(crate) fn update_joystick_visibility(
    settings: Res<Settings>,
    mut q: Query<&mut Visibility, With<JoystickBase>>,
) {
    let target = if settings.joystick_enabled {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for mut v in &mut q {
        if *v != target {
            *v = target;
        }
    }
}

pub(crate) fn update_joystick_knob(
    state: Res<JoystickState>,
    mut q: Query<&mut Node, With<JoystickKnob>>,
) {
    for mut node in &mut q {
        // Knob's `left`/`top` are relative to base's top-left.
        // KNOB_REST_OFFSET centres it; add (in screen-px space) the
        // x and y deflection. JoystickState.knob_offset is already in
        // screen pixels with y growing downward, matching `top`.
        node.left = Val::Px(KNOB_REST_OFFSET + state.knob_offset.x);
        node.top = Val::Px(KNOB_REST_OFFSET + state.knob_offset.y);
    }
}
