//! Layer 3 — client-side simulation. Translates input into intents,
//! computes view-model state.
//!
//! Allowed: `ButtonInput`, `Touches`, reading game-state Transforms,
//! writing view-model resources (`DesiredCameraView`, `CameraOrbit`,
//! `CameraFocus`, `MoveTarget`).
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

use crate::data::AppSet;

pub use camera_view::{CameraFocus, CameraOrbit};

pub struct ClientSimPlugin;

impl Plugin for ClientSimPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(CameraOrbit::default())
            .insert_resource(CameraFocus::default())
            // MoveTarget is registered as default by DataPlugin.
            // Input phase: collect all sources of MoveIntent before the
            // server consumes them. gather_move_input must run before
            // auto_move_to_target so keyboard-cancelled targets aren't
            // re-emitted in the same frame. gather_joystick_input
            // similarly cancels the click target when the joystick is
            // active, so it must also run before auto_move_to_target.
            .add_systems(
                Update,
                (
                    input::gather_camera_orbit_input,
                    move_target::apply_click_target,
                    (
                        input::gather_move_input,
                        joystick::gather_joystick_input,
                        move_target::auto_move_to_target,
                    )
                        .chain(),
                )
                    .in_set(AppSet::Input),
            )
            // ViewModel: orbit smoothing → focus smoothing → camera view.
            .add_systems(
                Update,
                (
                    camera_view::apply_camera_orbit,
                    camera_view::smooth_camera_orbit,
                    camera_view::smooth_camera_focus,
                    camera_view::compute_camera_view,
                )
                    .chain()
                    .in_set(AppSet::ViewModel),
            );
    }
}
