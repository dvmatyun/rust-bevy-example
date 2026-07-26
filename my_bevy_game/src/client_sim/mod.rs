//! Layer 3 — client-side simulation. Translates input into intents,
//! computes view-model state.
//!
//! Allowed: `ButtonInput`, `Touches`, reading game-state Transforms,
//! writing view-model resources (`MoveTarget`).
//!
//! Forbidden (same as server): all render types. Game-state mutation
//! goes through intents.
//!
//! Headless-testable with `MinimalPlugins + DataPlugin + ServerPlugin
//! + ClientSimPlugin`.

pub(crate) mod camera_view;
mod input;
mod joystick;
pub(crate) mod move_target;

use bevy::prelude::*;

use crate::data::{AppSet, GameState};

pub use camera_view::{CameraFocus, CameraOrbit};

pub struct ClientSimPlugin;

impl Plugin for ClientSimPlugin {
    fn build(&self, app: &mut App) {
        // CameraOrbit / CameraFocus are now *components* on GameCamera
        // entities, not global Resources. No insert_resource here.
        app
            // ── Input: all playing states ─────────────────────────────────
            .add_systems(
                Update,
                (
                    input::gather_camera_orbit_input_p1,
                    input::gather_teleport_input,
                    move_target::apply_click_target,
                    (
                        input::gather_move_input_p1,
                        joystick::gather_joystick_input,
                        move_target::auto_move_to_target,
                    )
                        .chain(),
                )
                    .in_set(AppSet::Input)
                    .run_if(
                        in_state(GameState::PlayingSingle)
                            .or_else(in_state(GameState::PlayingMultiplayer)),
                    ),
            )
            // ── Input: multiplayer-only (P2 controls) ─────────────────────
            .add_systems(
                Update,
                (
                    input::gather_move_input_p2,
                    input::gather_camera_orbit_input_p2,
                )
                    .in_set(AppSet::Input)
                    .run_if(in_state(GameState::PlayingMultiplayer)),
            )
            // ── ViewModel: both playing states ────────────────────────────
            .add_systems(
                Update,
                (
                    camera_view::apply_camera_orbit,
                    camera_view::smooth_camera_orbit,
                    camera_view::smooth_camera_focus,
                    camera_view::compute_camera_view,
                )
                    .chain()
                    .in_set(AppSet::ViewModel)
                    .run_if(
                        in_state(GameState::PlayingSingle)
                            .or_else(in_state(GameState::PlayingMultiplayer)),
                    ),
            );
    }
}
