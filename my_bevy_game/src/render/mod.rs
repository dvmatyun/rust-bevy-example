//! Layer 4 — render & UI. The only layer allowed to spawn `Mesh3d`,
//! `Camera3d`, `StandardMaterial`, `Node`, `Text`, etc.

mod buildings;
mod camera_render;
mod click_input;
mod debug_collisions;
mod hud;
mod joystick_ui;
mod main_menu;
mod markers;
mod monsters_view;
mod occlusion;
mod player_view;
mod scene_setup;
mod settings_ui;
mod stairs;
mod terrain_view;

use bevy::prelude::*;

use crate::data::{AppSet, GameEntity, GameState};

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((hud::HudPlugin, main_menu::MainMenuPlugin))
            .insert_resource(settings_ui::SettingsPanelState::default())
            // ── Asset registration (stateless, runs once at Startup) ──────
            .add_systems(
                Startup,
                (
                    scene_setup::setup_scene,
                    terrain_view::register_terrain_assets,
                    player_view::register_player_assets,
                    markers::register_marker_assets,
                    monsters_view::register_monster_assets,
                    debug_collisions::register_debug_collider_mats,
                    (buildings::register_building_assets, stairs::register_stair_assets).chain(),
                ),
            )
            // ── Spawn game-world entities on state entry ──────────────────
            .add_systems(
                OnEnter(GameState::PlayingSingle),
                (
                    camera_render::spawn_camera,
                    settings_ui::spawn_settings_ui,
                    joystick_ui::spawn_joystick,
                ),
            )
            .add_systems(
                OnEnter(GameState::PlayingMultiplayer),
                camera_render::spawn_camera_multiplayer,
            )
            // ── Cleanup on exit ───────────────────────────────────────────
            .add_systems(OnExit(GameState::PlayingSingle), cleanup_game_entities)
            .add_systems(OnExit(GameState::PlayingMultiplayer), cleanup_game_entities)
            // ── Input phase ───────────────────────────────────────────────
            .add_systems(
                Update,
                (
                    click_input::handle_click_input
                        .run_if(in_state(GameState::PlayingSingle)),
                    settings_ui::handle_gear_button,
                    settings_ui::handle_adjust_buttons,
                    settings_ui::handle_bool_toggles,
                    settings_ui::handle_action_buttons,
                )
                    .in_set(AppSet::Input),
            )
            // ── RenderApply (split into two blocks; tuple limit = 16) ─────
            .add_systems(
                Update,
                (
                    terrain_view::manage_terrain_chunks,
                    buildings::attach_building_visuals,
                    stairs::attach_stair_visuals,
                    monsters_view::attach_monster_visuals,
                    monsters_view::billboard_monsters,
                    player_view::attach_player_visuals,
                    camera_render::update_camera_transform,
                    player_view::update_player_face_view,
                    player_view::billboard_player,
                )
                    .in_set(AppSet::RenderApply)
                    .run_if(
                        in_state(GameState::PlayingSingle)
                            .or_else(in_state(GameState::PlayingMultiplayer)),
                    ),
            )
            .add_systems(
                Update,
                (
                    settings_ui::update_settings_labels,
                    settings_ui::update_scrollbar,
                    joystick_ui::update_joystick_visibility,
                    joystick_ui::update_joystick_knob,
                    markers::spawn_tap_markers,
                    markers::tick_tap_markers,
                    markers::sync_walk_target_marker,
                    occlusion::fade_occluders,
                    scene_setup::apply_sun_settings,
                    debug_collisions::attach_player_debug_collider,
                    debug_collisions::attach_monster_debug_collider,
                    debug_collisions::attach_building_debug_walls,
                    debug_collisions::toggle_debug_collider_visibility,
                )
                    .in_set(AppSet::RenderApply)
                    .run_if(
                        in_state(GameState::PlayingSingle)
                            .or_else(in_state(GameState::PlayingMultiplayer)),
                    ),
            );
    }
}

fn cleanup_game_entities(mut commands: Commands, q: Query<Entity, With<GameEntity>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}
