//! Settings panel: gear button (top-right) + scrollable list of
//! camera, graphics, and benchmark controls.
//!
//! Three control flavours:
//! - **Numeric +/-** — `SettingsField` variants. Labelled value
//!   between two buttons; clicking ± clamps within range.
//! - **Boolean toggle** — `SettingsBoolField` variants. One pill
//!   button showing on / off.
//! - **Action button** — `SettingsAction` variants. Click fires an
//!   intent (spawn N monsters, set world radius). Static text.
//!
//! Layout: the panel is a flex-row — scrollable content on the left
//! (flex-grow 1) and a 6-px scrollbar track on the right. The track
//! contains an absolutely-positioned thumb whose size and position are
//! updated every frame from `ScrollPosition` + `ComputedNode`.

use bevy::prelude::*;

use crate::data::{
    DespawnMonstersIntent, Monster, Settings, SpawnMonstersIntent, WorldConfig,
};

const BTN_NORMAL: Color = Color::srgb(0.20, 0.20, 0.40);
const BTN_HOVER: Color = Color::srgb(0.28, 0.28, 0.52);
const BTN_PRESS: Color = Color::srgb(0.35, 0.35, 0.65);
const PANEL_BG: Color = Color::srgba(0.0, 0.0, 0.0, 0.85);
const GEAR_BG: Color = Color::srgba(0.0, 0.0, 0.0, 0.7);

#[derive(Resource, Default)]
pub(crate) struct SettingsPanelState {
    pub open: bool,
}

#[derive(Component)]
pub(crate) struct SettingsButton;

#[derive(Component)]
pub(crate) struct SettingsPanel;

/// The inner scrollable column — carries `ScrollPosition` so Bevy tracks
/// mouse-wheel / touch-swipe scroll, and lets us read it for the thumb.
#[derive(Component)]
pub(crate) struct SettingsPanelContent;

/// The thin track on the right edge of the panel.
#[derive(Component)]
pub(crate) struct ScrollbarTrack;

/// The draggable-looking thumb inside the track.
#[derive(Component)]
pub(crate) struct ScrollbarThumb;

// ── Numeric fields ──────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SettingsField {
    LerpSpeed,
    OrbitSpeed,
    Distance,
    Height,
    Lookahead,
    VelSmooth,
    RenderDistance,
    SunAzimuth,
    SunElevation,
    PlayerRadius,
    MonsterRadius,
}

#[derive(Component)]
pub(crate) struct SettingsAdjust {
    pub field: SettingsField,
    pub delta: f32,
}

#[derive(Component)]
pub(crate) struct SettingsValueLabel(pub SettingsField);

// ── Boolean toggles ─────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SettingsBoolField {
    JoystickEnabled,
    ShadowsEnabled,
    FogEnabled,
    CollisionsEnabled,
    DebugCollisions,
}

#[derive(Component)]
pub(crate) struct SettingsBoolToggle(pub SettingsBoolField);

#[derive(Component)]
pub(crate) struct SettingsBoolLabel(pub SettingsBoolField);

// ── One-shot action buttons ─────────────────────────────────────────────

#[derive(Clone, Copy, Debug)]
pub(crate) enum SettingsAction {
    SpawnMonsters(u32),
    DespawnAllMonsters,
    SetWorldRadius(i32),
}

#[derive(Component)]
pub(crate) struct SettingsActionButton(pub SettingsAction);

/// Live label showing total monster count (alive entities).
#[derive(Component)]
pub(crate) struct MonsterCountLabel;

/// Live label showing the current world radius (cells).
#[derive(Component)]
pub(crate) struct WorldRadiusLabel;

// =====================================================================
// Spawn UI
// =====================================================================

pub(crate) fn spawn_settings_ui(mut commands: Commands) {
    let safe_top = Val::Px(48.0);
    let safe_side = Val::Px(20.0);

    // Gear button (always visible, top-right).
    commands
        .spawn((
            Button,
            SettingsButton,
            Node {
                position_type: PositionType::Absolute,
                top: safe_top,
                right: safe_side,
                width: Val::Px(48.0),
                height: Val::Px(48.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(GEAR_BG),
        ))
        .with_child((
            Text::new("S"),
            TextFont {
                font_size: FontSize::Px(22.0),
                ..default()
            },
            TextColor(Color::WHITE),
        ));

    // Outer panel: flex-row pinned by top + bottom + left + right so
    // it fills the viewport column without overflowing on any screen.
    commands
        .spawn((
            SettingsPanel,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(110.0),
                bottom: Val::Px(66.0), // iOS home-indicator safe area
                left: safe_side,
                right: safe_side,
                max_width: Val::Px(380.0),
                flex_direction: FlexDirection::Row,
                ..default()
            },
            BackgroundColor(PANEL_BG),
            Visibility::Hidden,
        ))
        .with_children(|panel| {
            // ── Scrollable content column ─────────────────────────────
            panel
                .spawn((
                    SettingsPanelContent,
                    Node {
                        flex_direction: FlexDirection::Column,
                        flex_grow: 1.0,
                        row_gap: Val::Px(6.0),
                        // 8 px right gap keeps text clear of scrollbar.
                        padding: UiRect {
                            left: Val::Px(12.0),
                            right: Val::Px(8.0),
                            top: Val::Px(12.0),
                            bottom: Val::Px(12.0),
                        },
                        overflow: Overflow::scroll_y(),
                        ..default()
                    },
                    ScrollPosition::default(),
                ))
                .with_children(|content| {
                    // World scale lives at the top so the resize buttons are
                    // visible without scrolling on small screens.
                    spawn_section_label(content, "── World scale ──");
                    spawn_world_scale_row(content);
                    spawn_world_radius_row(content);

                    spawn_section_label(content, "── Monsters ──");
                    spawn_monster_row(content);
                    spawn_monster_count_row(content);

                    spawn_section_label(content, "── Physics ──");
                    spawn_bool_row(content, "Collisions", SettingsBoolField::CollisionsEnabled);
                    spawn_bool_row(content, "Show colliders", SettingsBoolField::DebugCollisions);
                    spawn_setting_row(content, "Player r", SettingsField::PlayerRadius, 0.05);
                    spawn_setting_row(content, "Monster r", SettingsField::MonsterRadius, 0.05);

                    spawn_section_label(content, "── Camera ──");
                    spawn_setting_row(content, "Smooth", SettingsField::LerpSpeed, 0.5);
                    spawn_setting_row(content, "Orbit", SettingsField::OrbitSpeed, 0.25);
                    spawn_setting_row(content, "Distance", SettingsField::Distance, 1.0);
                    spawn_setting_row(content, "Height", SettingsField::Height, 1.0);
                    spawn_setting_row(content, "Lookahead", SettingsField::Lookahead, 0.05);
                    spawn_setting_row(content, "VelSmooth", SettingsField::VelSmooth, 0.5);

                    spawn_section_label(content, "── Input ──");
                    spawn_bool_row(content, "Joystick", SettingsBoolField::JoystickEnabled);

                    spawn_section_label(content, "── Graphics ──");
                    spawn_bool_row(content, "Shadows", SettingsBoolField::ShadowsEnabled);
                    spawn_bool_row(content, "Fog", SettingsBoolField::FogEnabled);
                    spawn_setting_row(content, "Render", SettingsField::RenderDistance, 1.0);
                    spawn_setting_row(content, "Sun az", SettingsField::SunAzimuth, 15.0);
                    spawn_setting_row(content, "Sun el", SettingsField::SunElevation, 5.0);
                });

            // ── Scrollbar track ───────────────────────────────────────
            panel
                .spawn((
                    ScrollbarTrack,
                    Node {
                        width: Val::Px(6.0),
                        // flex-col fills the full panel height automatically
                        ..default()
                    },
                    BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.08)),
                ))
                .with_children(|track| {
                    track.spawn((
                        ScrollbarThumb,
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(0.0),
                            width: Val::Px(6.0),
                            height: Val::Px(40.0), // updated every frame
                            top: Val::Px(0.0),     // updated every frame
                            border_radius: BorderRadius::all(Val::Px(3.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.55)),
                    ));
                });
        });
}

// ── Scrollbar update ────────────────────────────────────────────────────────

pub(crate) fn update_scrollbar(
    content_q: Query<
        (&ComputedNode, &ScrollPosition, &Children),
        With<SettingsPanelContent>,
    >,
    track_q: Query<&ComputedNode, With<ScrollbarTrack>>,
    children_q: Query<&ComputedNode>,
    mut thumb_q: Query<&mut Node, With<ScrollbarThumb>>,
) {
    let Ok((content_cn, scroll, content_children)) = content_q.single() else {
        return;
    };
    let Ok(track_cn) = track_q.single() else { return };
    let Ok(mut thumb) = thumb_q.single_mut() else { return };

    // `ComputedNode.size` is a Vec2 field (not a method).
    let viewport_h = content_cn.size.y;
    let track_h = track_cn.size.y;
    if viewport_h <= 0.0 || track_h <= 0.0 {
        return;
    }

    // Sum the computed heights of every direct child + row_gap between them.
    let item_count = content_children.len();
    let content_h: f32 = content_children
        .iter()
        .filter_map(|e| children_q.get(e).ok())
        .map(|cn: &ComputedNode| cn.size.y)
        .sum::<f32>()
        + 6.0 * item_count.saturating_sub(1) as f32  // row_gap contributions
        + 24.0; // top (12) + bottom (12) padding

    let max_scroll = (content_h - viewport_h).max(0.0);

    if max_scroll <= 0.0 {
        // All content fits — thumb fills track, no scrolling possible.
        thumb.height = Val::Px(track_h);
        thumb.top = Val::Px(0.0);
        return;
    }

    let scroll_ratio = (scroll.y / max_scroll).clamp(0.0, 1.0);
    let thumb_h = ((viewport_h / content_h) * track_h).max(24.0); // min 24 px
    let thumb_top = scroll_ratio * (track_h - thumb_h);

    thumb.height = Val::Px(thumb_h);
    thumb.top = Val::Px(thumb_top);
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn spawn_section_label(parent: &mut ChildSpawnerCommands, text: &str) {
    parent.spawn((
        Text::new(text),
        TextFont {
            font_size: FontSize::Px(13.0),
            ..default()
        },
        TextColor(Color::srgb(0.7, 0.7, 0.7)),
    ));
}

fn spawn_bool_row(parent: &mut ChildSpawnerCommands, label: &str, field: SettingsBoolField) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(6.0),
            ..default()
        })
        .with_children(|row| {
            row.spawn(Node {
                width: Val::Px(80.0),
                ..default()
            })
            .with_child((
                Text::new(format!("{label}:")),
                TextFont { font_size: FontSize::Px(14.0), ..default() },
                TextColor(Color::WHITE),
            ));
            row.spawn((
                Button,
                SettingsBoolToggle(field),
                Node {
                    min_width: Val::Px(60.0),
                    height: Val::Px(28.0),
                    padding: UiRect::axes(Val::Px(10.0), Val::Px(0.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(BTN_NORMAL),
            ))
            .with_child((
                Text::new("--"),
                TextFont { font_size: FontSize::Px(14.0), ..default() },
                TextColor(Color::WHITE),
                SettingsBoolLabel(field),
            ));
        });
}

fn spawn_setting_row(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    field: SettingsField,
    step: f32,
) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(6.0),
            ..default()
        })
        .with_children(|row| {
            row.spawn(Node {
                width: Val::Px(80.0),
                ..default()
            })
            .with_child((
                Text::new(format!("{label}:")),
                TextFont { font_size: FontSize::Px(14.0), ..default() },
                TextColor(Color::WHITE),
            ));
            spawn_adjust_button(row, "-", SettingsAdjust { field, delta: -step });
            row.spawn(Node {
                width: Val::Px(50.0),
                justify_content: JustifyContent::Center,
                ..default()
            })
            .with_child((
                Text::new("--"),
                TextFont { font_size: FontSize::Px(14.0), ..default() },
                TextColor(Color::WHITE),
                SettingsValueLabel(field),
            ));
            spawn_adjust_button(row, "+", SettingsAdjust { field, delta: step });
        });
}

fn spawn_adjust_button(parent: &mut ChildSpawnerCommands, label: &str, action: SettingsAdjust) {
    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(28.0),
                height: Val::Px(28.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(BTN_NORMAL),
            action,
        ))
        .with_child((
            Text::new(label),
            TextFont { font_size: FontSize::Px(16.0), ..default() },
            TextColor(Color::WHITE),
        ));
}

fn spawn_action_button(parent: &mut ChildSpawnerCommands, label: &str, action: SettingsAction) {
    parent
        .spawn((
            Button,
            Node {
                min_width: Val::Px(50.0),
                height: Val::Px(28.0),
                padding: UiRect::axes(Val::Px(8.0), Val::Px(0.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(BTN_NORMAL),
            SettingsActionButton(action),
        ))
        .with_child((
            Text::new(label),
            TextFont { font_size: FontSize::Px(13.0), ..default() },
            TextColor(Color::WHITE),
        ));
}

fn spawn_monster_row(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(4.0),
            flex_wrap: FlexWrap::Wrap,
            row_gap: Val::Px(4.0),
            ..default()
        })
        .with_children(|row| {
            spawn_action_button(row, "+1",   SettingsAction::SpawnMonsters(1));
            spawn_action_button(row, "+50",  SettingsAction::SpawnMonsters(50));
            spawn_action_button(row, "+500", SettingsAction::SpawnMonsters(500));
            spawn_action_button(row, "Clear", SettingsAction::DespawnAllMonsters);
        });
}

fn spawn_monster_count_row(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(6.0),
            ..default()
        })
        .with_children(|row| {
            row.spawn(Node {
                width: Val::Px(80.0),
                ..default()
            })
            .with_child((
                Text::new("Alive:"),
                TextFont { font_size: FontSize::Px(13.0), ..default() },
                TextColor(Color::srgb(0.7, 0.7, 0.7)),
            ));
            row.spawn((
                Text::new("0"),
                TextFont { font_size: FontSize::Px(14.0), ..default() },
                TextColor(Color::WHITE),
                MonsterCountLabel,
            ));
        });
}

fn spawn_world_scale_row(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(4.0),
            flex_wrap: FlexWrap::Wrap,
            row_gap: Val::Px(4.0),
            ..default()
        })
        .with_children(|row| {
            spawn_action_button(row, "1×",     SettingsAction::SetWorldRadius(32));
            spawn_action_button(row, "10×",    SettingsAction::SetWorldRadius(320));
            spawn_action_button(row, "100×",   SettingsAction::SetWorldRadius(3200));
            spawn_action_button(row, "10000×", SettingsAction::SetWorldRadius(320_000));
        });
}

fn spawn_world_radius_row(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(6.0),
            ..default()
        })
        .with_children(|row| {
            row.spawn(Node {
                width: Val::Px(80.0),
                ..default()
            })
            .with_child((
                Text::new("Radius:"),
                TextFont { font_size: FontSize::Px(13.0), ..default() },
                TextColor(Color::srgb(0.7, 0.7, 0.7)),
            ));
            row.spawn((
                Text::new("32 cells"),
                TextFont { font_size: FontSize::Px(13.0), ..default() },
                TextColor(Color::WHITE),
                WorldRadiusLabel,
            ));
        });
}

// =====================================================================
// Handlers
// =====================================================================

pub(crate) fn handle_gear_button(
    mut interactions: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<SettingsButton>),
    >,
    mut state: ResMut<SettingsPanelState>,
    mut panel_q: Query<&mut Visibility, With<SettingsPanel>>,
) {
    for (interaction, mut bg) in &mut interactions {
        match interaction {
            Interaction::Pressed => {
                *bg = BackgroundColor(GEAR_BG);
                state.open = !state.open;
                for mut v in &mut panel_q {
                    *v = if state.open {
                        Visibility::Visible
                    } else {
                        Visibility::Hidden
                    };
                }
            }
            Interaction::Hovered => *bg = BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.7)),
            Interaction::None => *bg = BackgroundColor(GEAR_BG),
        }
    }
}

pub(crate) fn handle_adjust_buttons(
    mut interactions: Query<
        (&Interaction, &SettingsAdjust, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
    mut settings: ResMut<Settings>,
) {
    for (interaction, action, mut bg) in &mut interactions {
        match interaction {
            Interaction::Pressed => {
                *bg = BackgroundColor(BTN_PRESS);
                match action.field {
                    SettingsField::LerpSpeed => {
                        settings.camera_lerp_speed =
                            (settings.camera_lerp_speed + action.delta).clamp(1.0, 20.0);
                    }
                    SettingsField::OrbitSpeed => {
                        settings.camera_orbit_speed =
                            (settings.camera_orbit_speed + action.delta).clamp(0.5, 5.0);
                    }
                    SettingsField::Distance => {
                        settings.camera_distance =
                            (settings.camera_distance + action.delta).clamp(5.0, 30.0);
                    }
                    SettingsField::Height => {
                        settings.camera_height =
                            (settings.camera_height + action.delta).clamp(3.0, 30.0);
                    }
                    SettingsField::Lookahead => {
                        settings.camera_lookahead =
                            (settings.camera_lookahead + action.delta).clamp(0.0, 1.0);
                    }
                    SettingsField::VelSmooth => {
                        settings.camera_velocity_smoothing =
                            (settings.camera_velocity_smoothing + action.delta).clamp(1.0, 20.0);
                    }
                    SettingsField::RenderDistance => {
                        settings.render_distance_chunks =
                            (settings.render_distance_chunks + action.delta as i32).clamp(1, 16);
                    }
                    SettingsField::SunAzimuth => {
                        settings.sun_azimuth_deg =
                            (settings.sun_azimuth_deg + action.delta).rem_euclid(360.0);
                    }
                    SettingsField::SunElevation => {
                        settings.sun_elevation_deg =
                            (settings.sun_elevation_deg + action.delta).clamp(5.0, 89.0);
                    }
                    SettingsField::PlayerRadius => {
                        settings.player_radius =
                            (settings.player_radius + action.delta).clamp(0.1, 2.0);
                    }
                    SettingsField::MonsterRadius => {
                        settings.monster_radius =
                            (settings.monster_radius + action.delta).clamp(0.1, 2.0);
                    }
                }
            }
            Interaction::Hovered => *bg = BackgroundColor(BTN_HOVER),
            Interaction::None => *bg = BackgroundColor(BTN_NORMAL),
        }
    }
}

pub(crate) fn handle_bool_toggles(
    mut interactions: Query<
        (&Interaction, &SettingsBoolToggle, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
    mut settings: ResMut<Settings>,
) {
    for (interaction, toggle, mut bg) in &mut interactions {
        match interaction {
            Interaction::Pressed => {
                *bg = BackgroundColor(BTN_PRESS);
                match toggle.0 {
                    SettingsBoolField::JoystickEnabled => {
                        settings.joystick_enabled = !settings.joystick_enabled;
                    }
                    SettingsBoolField::ShadowsEnabled => {
                        settings.shadows_enabled = !settings.shadows_enabled;
                    }
                    SettingsBoolField::FogEnabled => {
                        settings.fog_enabled = !settings.fog_enabled;
                    }
                    SettingsBoolField::CollisionsEnabled => {
                        settings.collisions_enabled = !settings.collisions_enabled;
                    }
                    SettingsBoolField::DebugCollisions => {
                        settings.debug_collisions = !settings.debug_collisions;
                    }
                }
            }
            Interaction::Hovered => *bg = BackgroundColor(BTN_HOVER),
            Interaction::None => *bg = BackgroundColor(BTN_NORMAL),
        }
    }
}

pub(crate) fn handle_action_buttons(
    mut interactions: Query<
        (&Interaction, &SettingsActionButton, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
    mut spawn_writer: MessageWriter<SpawnMonstersIntent>,
    mut despawn_writer: MessageWriter<DespawnMonstersIntent>,
    mut config: ResMut<WorldConfig>,
) {
    for (interaction, btn, mut bg) in &mut interactions {
        match interaction {
            Interaction::Pressed => {
                *bg = BackgroundColor(BTN_PRESS);
                match btn.0 {
                    SettingsAction::SpawnMonsters(n) => {
                        spawn_writer.write(SpawnMonstersIntent { count: n });
                    }
                    SettingsAction::DespawnAllMonsters => {
                        despawn_writer.write(DespawnMonstersIntent);
                    }
                    SettingsAction::SetWorldRadius(r) => {
                        config.world_radius_cells = r;
                    }
                }
            }
            Interaction::Hovered => *bg = BackgroundColor(BTN_HOVER),
            Interaction::None => *bg = BackgroundColor(BTN_NORMAL),
        }
    }
}

pub(crate) fn update_settings_labels(
    settings: Res<Settings>,
    config: Res<WorldConfig>,
    monsters: Query<(), With<Monster>>,
    mut numeric: Query<
        (&SettingsValueLabel, &mut Text),
        (Without<SettingsBoolLabel>, Without<MonsterCountLabel>, Without<WorldRadiusLabel>),
    >,
    mut bools: Query<
        (&SettingsBoolLabel, &mut Text),
        (Without<SettingsValueLabel>, Without<MonsterCountLabel>, Without<WorldRadiusLabel>),
    >,
    mut monster_count: Query<&mut Text, (With<MonsterCountLabel>, Without<SettingsValueLabel>, Without<SettingsBoolLabel>, Without<WorldRadiusLabel>)>,
    mut radius: Query<&mut Text, (With<WorldRadiusLabel>, Without<SettingsValueLabel>, Without<SettingsBoolLabel>, Without<MonsterCountLabel>)>,
) {
    for (label, mut text) in &mut numeric {
        let s = match label.0 {
            SettingsField::LerpSpeed => format!("{:.1}", settings.camera_lerp_speed),
            SettingsField::OrbitSpeed => format!("{:.2}", settings.camera_orbit_speed),
            SettingsField::Distance => format!("{:.0}", settings.camera_distance),
            SettingsField::Height => format!("{:.0}", settings.camera_height),
            SettingsField::Lookahead => format!("{:.2}", settings.camera_lookahead),
            SettingsField::VelSmooth => format!("{:.1}", settings.camera_velocity_smoothing),
            SettingsField::RenderDistance => format!("{}", settings.render_distance_chunks),
            SettingsField::SunAzimuth => format!("{:.0}°", settings.sun_azimuth_deg),
            SettingsField::SunElevation => format!("{:.0}°", settings.sun_elevation_deg),
            SettingsField::PlayerRadius => format!("{:.2}", settings.player_radius),
            SettingsField::MonsterRadius => format!("{:.2}", settings.monster_radius),
        };
        *text = Text::new(s);
    }
    for (label, mut text) in &mut bools {
        let v = match label.0 {
            SettingsBoolField::JoystickEnabled => settings.joystick_enabled,
            SettingsBoolField::ShadowsEnabled => settings.shadows_enabled,
            SettingsBoolField::FogEnabled => settings.fog_enabled,
            SettingsBoolField::CollisionsEnabled => settings.collisions_enabled,
            SettingsBoolField::DebugCollisions => settings.debug_collisions,
        };
        *text = Text::new(if v { "on" } else { "off" });
    }
    let count = monsters.iter().count();
    for mut t in &mut monster_count {
        *t = Text::new(count.to_string());
    }
    let r = config.world_radius_cells;
    for mut t in &mut radius {
        *t = Text::new(format!("{} cells ({}×)", r, (r / 32).max(1)));
    }
}
