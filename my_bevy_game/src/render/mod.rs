//! Layer 4 — render & UI. The only layer allowed to spawn `Mesh3d`,
//! `Camera3d`, `StandardMaterial`, `Node`, `Text`, etc.
//!
//! Reads from lower layers via:
//! - `Transform` on logical entities (set by `server/`).
//! - `DesiredCameraView`, `Settings` resources (set/read by `client_sim/`).
//!
//! Render also owns one input handler (`click_input::handle_click_input`)
//! because screen → world raycasts require the actual `Camera`. See
//! `docsai/architecture.md` decision log.

mod camera_render;
mod click_input;
mod hud;
mod joystick_ui;
mod markers;
mod player_view;
mod scene_setup;
mod settings_ui;
mod terrain_view;

use bevy::prelude::*;

use crate::data::AppSet;

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(hud::HudPlugin)
            .insert_resource(settings_ui::SettingsPanelState::default())
            .add_systems(
                Startup,
                (
                    scene_setup::setup_scene,
                    terrain_view::register_terrain_assets,
                    player_view::register_player_assets,
                    camera_render::spawn_camera,
                    settings_ui::spawn_settings_ui,
                    joystick_ui::spawn_joystick,
                    markers::register_marker_assets,
                ),
            )
            // Input phase systems: produce intents that lower layers
            // will consume. The click raycast lives here (not in
            // client_sim) because it needs the active `Camera`.
            .add_systems(
                Update,
                (
                    click_input::handle_click_input,
                    settings_ui::handle_gear_button,
                    settings_ui::handle_adjust_buttons,
                    settings_ui::handle_bool_toggles,
                )
                    .in_set(AppSet::Input),
            )
            // RenderApply: visual updates that depend on game-state +
            // view-model already being current.
            .add_systems(
                Update,
                (
                    terrain_view::attach_terrain_visuals,
                    player_view::attach_player_visuals,
                    camera_render::update_camera_transform,
                    player_view::update_player_face_view,
                    player_view::billboard_player,
                    settings_ui::update_settings_labels,
                    joystick_ui::update_joystick_visibility,
                    joystick_ui::update_joystick_knob,
                    markers::spawn_tap_markers,
                    markers::tick_tap_markers,
                    markers::sync_walk_target_marker,
                )
                    .in_set(AppSet::RenderApply),
            );
    }
}
