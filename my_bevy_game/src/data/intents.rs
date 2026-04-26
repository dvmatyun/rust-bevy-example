//! Cross-layer messages and view-model resources.
//!
//! - **Intents** flow upward: client_sim emits, server consumes.
//! - **View-model** resources flow downward: client_sim writes, render reads.

use bevy::prelude::*;

/// "The player wants to move this frame."
///
/// `direction.x` → world +X axis, `direction.y` → world +Z axis.
/// Magnitude is in `[0, 1]` (already normalised by the emitter).
#[derive(Message, Clone, Copy, Debug)]
pub struct MoveIntent {
    pub direction: Vec2,
}

/// "The player wants to orbit the camera this frame."
///
/// `delta` is a signed multiplier (typically -1.0 or +1.0). The actual
/// orbit speed is taken from `Settings.camera_orbit_speed`.
#[derive(Message, Clone, Copy, Debug)]
pub struct CameraOrbitIntent {
    pub delta: f32,
}

/// "The player wants to walk to this XZ point."
///
/// Emitted by the render layer after a successful screen-to-world
/// raycast on a mouse click or touch tap (since unprojection requires
/// the active `Camera`). Consumed by `client_sim` which sets a sticky
/// `MoveTarget` and emits `MoveIntent`s toward it each frame until the
/// player arrives or keyboard input cancels it.
#[derive(Message, Clone, Copy, Debug)]
pub struct ClickMoveIntent {
    pub target: Vec2,
}

/// View-model: where the camera *should* be this frame. The render layer
/// reads this and tweens its actual `Camera3d.Transform` toward it.
#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct DesiredCameraView {
    pub position: Vec3,
    pub look_at: Vec3,
}

/// View-model for the on-screen joystick. Written by
/// `client_sim::joystick::gather_joystick_input`, read by
/// `render::joystick_ui` to place the visible knob.
#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct JoystickState {
    /// Knob offset from base centre, in screen pixels. (0, 0) = idle.
    pub knob_offset: Vec2,
    /// Whether a finger / mouse is currently driving the joystick.
    pub active: bool,
}

/// Sticky click-to-move target. `Some(xz)` means the player is auto-
/// walking toward this point until close enough; `None` means no
/// active click target.
///
/// Written by `client_sim::move_target` (consumes `ClickMoveIntent`,
/// clears on arrival or keyboard / joystick input). Read by `render`
/// to draw the walk-target marker.
#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct MoveTarget(pub Option<Vec2>);
