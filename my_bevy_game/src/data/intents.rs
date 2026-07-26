//! Cross-layer messages and view-model resources/components.
//!
//! - **Intents** flow upward: client_sim emits, server consumes.
//! - **View-model** types flow downward: client_sim writes, render reads.

use bevy::prelude::*;

/// "The player wants to move this frame."
///
/// `direction.x` → world +X axis, `direction.y` → world +Z axis.
/// Magnitude is in `[0, 1]` (already normalised by the emitter).
/// `player_slot` — 0 = first player, 1 = second player.
#[derive(Message, Clone, Copy, Debug)]
pub struct MoveIntent {
    pub direction: Vec2,
    pub player_slot: u8,
}

/// Like `MoveIntent` but queued by the multiplayer network-simulation
/// layer before being released after the configured ping delay.
///
/// In `PlayingSingle` mode this type is never emitted; `MoveIntent` is
/// written directly.  In `PlayingMultiplayer` the input system writes
/// `BufferedMoveIntent`; `server::net_sim` reads it, waits ~100 ms,
/// then writes `MoveIntent`.
#[derive(Message, Clone, Copy, Debug)]
pub struct BufferedMoveIntent {
    pub direction: Vec2,
    pub player_slot: u8,
}

/// "The player wants to orbit the camera this frame."
///
/// `delta` is a signed multiplier (typically -1.0 or +1.0). The actual
/// orbit speed is taken from `Settings.camera_orbit_speed`.
/// Camera orbit is NOT buffered through `NetSim` — it is a local view
/// operation, applied immediately regardless of game mode.
#[derive(Message, Clone, Copy, Debug)]
pub struct CameraOrbitIntent {
    pub delta: f32,
    /// Which camera to orbit (0 = P1 camera, 1 = P2 camera).
    pub player_slot: u8,
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

/// Debug-only: nudge the player Y by `delta_y` units.
///
/// Used to climb between building floors during testing — proper
/// movement up requires a physics engine / character controller
/// (planned in `docsai/world-and-graphics-plan.md`).
#[derive(Message, Clone, Copy, Debug)]
pub struct TeleportIntent {
    pub delta_y: f32,
    /// Always 0 (player 0 only — debug feature).
    pub player_slot: u8,
}

/// Settings UI requested spawning N more monsters near the player.
#[derive(Message, Clone, Copy, Debug)]
pub struct SpawnMonstersIntent {
    pub count: u32,
}

/// Settings UI requested removing all roaming monsters.
#[derive(Message, Clone, Copy, Debug)]
pub struct DespawnMonstersIntent;

// ── View-model ──────────────────────────────────────────────────────────────

/// Where the camera *should* be this frame. Stored as a **Component**
/// on each `GameCamera` entity (not a Resource) so split-screen can
/// have two independent desired views.
///
/// `client_sim::camera_view::compute_camera_view` writes this.
/// `render::camera_render::update_camera_transform` reads it.
#[derive(Component, Clone, Copy, Debug, Default)]
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
