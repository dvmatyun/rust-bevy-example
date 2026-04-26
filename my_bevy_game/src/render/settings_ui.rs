//! Settings panel: gear button (top-right) toggles a panel of -/value/+
//! controls that mutate the `Settings` resource at runtime.
//!
//! UI-internal types (`SettingsField`, `SettingsAdjust`,
//! `SettingsValueLabel`, `SettingsPanel`, `SettingsButton`,
//! `SettingsPanelState`) live here because nothing outside this module
//! needs them.

use bevy::prelude::*;

use crate::data::Settings;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SettingsField {
    LerpSpeed,
    OrbitSpeed,
    Distance,
    Height,
    Lookahead,
    VelSmooth,
}

#[derive(Component)]
pub(crate) struct SettingsAdjust {
    pub field: SettingsField,
    pub delta: f32,
}

#[derive(Component)]
pub(crate) struct SettingsValueLabel(pub SettingsField);

/// Boolean settings — same row pattern as numeric, but the button is a
/// single click-to-toggle pill instead of a -/value/+ triple.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SettingsBoolField {
    JoystickEnabled,
}

#[derive(Component)]
pub(crate) struct SettingsBoolToggle(pub SettingsBoolField);

#[derive(Component)]
pub(crate) struct SettingsBoolLabel(pub SettingsBoolField);

pub(crate) fn spawn_settings_ui(mut commands: Commands) {
    let safe_top = Val::Px(48.0);
    let safe_side = Val::Px(20.0);

    // Gear button (always visible, top-right)
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

    // Panel (initially hidden, sits below the gear).
    // Sized via left+right margins instead of fixed width so it
    // adapts to small phone screens. `max_width` keeps it from
    // stretching to absurd widths on desktop.
    commands
        .spawn((
            SettingsPanel,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(110.0),
                left: safe_side,
                right: safe_side,
                max_width: Val::Px(360.0),
                padding: UiRect::all(Val::Px(12.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                ..default()
            },
            BackgroundColor(PANEL_BG),
            Visibility::Hidden,
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new("Settings"),
                TextFont {
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            spawn_setting_row(panel, "Smooth", SettingsField::LerpSpeed, 0.5);
            spawn_setting_row(panel, "Orbit", SettingsField::OrbitSpeed, 0.25);
            spawn_setting_row(panel, "Distance", SettingsField::Distance, 1.0);
            spawn_setting_row(panel, "Height", SettingsField::Height, 1.0);
            spawn_setting_row(panel, "Lookahead", SettingsField::Lookahead, 0.05);
            spawn_setting_row(panel, "VelSmooth", SettingsField::VelSmooth, 0.5);
            spawn_bool_row(panel, "Joystick", SettingsBoolField::JoystickEnabled);
        });
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
            // Label
            row.spawn(Node {
                width: Val::Px(80.0),
                ..default()
            })
            .with_child((
                Text::new(format!("{label}:")),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));

            // Toggle pill — touch-friendly (44×28).
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
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
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
            // Label
            row.spawn(Node {
                width: Val::Px(80.0),
                ..default()
            })
            .with_child((
                Text::new(format!("{label}:")),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));

            // - button
            spawn_adjust_button(
                row,
                "-",
                SettingsAdjust {
                    field,
                    delta: -step,
                },
            );

            // value label
            row.spawn(Node {
                width: Val::Px(50.0),
                justify_content: JustifyContent::Center,
                ..default()
            })
            .with_child((
                Text::new("--"),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::WHITE),
                SettingsValueLabel(field),
            ));

            // + button
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
            TextFont {
                font_size: FontSize::Px(16.0),
                ..default()
            },
            TextColor(Color::WHITE),
        ));
}

/// Toggle the panel when the gear button is clicked.
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

/// Apply +/- presses to the `Settings` resource, clamped to sane ranges.
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
                }
            }
            Interaction::Hovered => *bg = BackgroundColor(BTN_HOVER),
            Interaction::None => *bg = BackgroundColor(BTN_NORMAL),
        }
    }
}

/// Mirror the current `Settings` values into the value labels.
pub(crate) fn update_settings_labels(
    settings: Res<Settings>,
    mut numeric: Query<(&SettingsValueLabel, &mut Text), Without<SettingsBoolLabel>>,
    mut bools: Query<(&SettingsBoolLabel, &mut Text), Without<SettingsValueLabel>>,
) {
    for (label, mut text) in &mut numeric {
        let s = match label.0 {
            SettingsField::LerpSpeed => format!("{:.1}", settings.camera_lerp_speed),
            SettingsField::OrbitSpeed => format!("{:.2}", settings.camera_orbit_speed),
            SettingsField::Distance => format!("{:.0}", settings.camera_distance),
            SettingsField::Height => format!("{:.0}", settings.camera_height),
            SettingsField::Lookahead => format!("{:.2}", settings.camera_lookahead),
            SettingsField::VelSmooth => format!("{:.1}", settings.camera_velocity_smoothing),
        };
        *text = Text::new(s);
    }
    for (label, mut text) in &mut bools {
        let v = match label.0 {
            SettingsBoolField::JoystickEnabled => settings.joystick_enabled,
        };
        *text = Text::new(if v { "on" } else { "off" });
    }
}

/// Click-to-toggle handler for boolean settings.
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
                }
            }
            Interaction::Hovered => *bg = BackgroundColor(BTN_HOVER),
            Interaction::None => *bg = BackgroundColor(BTN_NORMAL),
        }
    }
}
