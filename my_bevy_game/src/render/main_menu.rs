//! Main-menu screen: title + two big buttons.
//!
//! The menu uses Bevy UI (`Node`) directly; no `Camera3d` is active during
//! `MainMenu`, so a `Camera2d` is spawned alongside the UI root and
//! despawned with it on `OnExit(MainMenu)`.

use bevy::prelude::*;

use crate::data::GameState;

/// Marker for every entity spawned by the main menu so they are bulk-despawned
/// on `OnExit(MainMenu)`.
#[derive(Component)]
struct MenuEntity;

/// Tag for the two action buttons so the interaction system can identify them.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
enum MenuButton {
    PlaySingle,
    PlayMultiplayer,
}

pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::MainMenu), spawn_main_menu)
            .add_systems(OnExit(GameState::MainMenu), despawn_main_menu)
            .add_systems(
                Update,
                handle_menu_buttons.run_if(in_state(GameState::MainMenu)),
            );
    }
}

// ── Spawn ─────────────────────────────────────────────────────────────────────

fn spawn_main_menu(mut commands: Commands) {
    // A Camera2d is required for Bevy UI to render when there is no Camera3d.
    commands.spawn((Camera2d, MenuEntity));

    commands
        .spawn((
            MenuEntity,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(24.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.05, 0.05, 0.10)),
        ))
        .with_children(|parent| {
            // Title
            parent.spawn((
                Text::new("Dungeon Runner"),
                TextFont {
                    font_size: FontSize::Px(52.0),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.85, 0.4)),
                Node {
                    margin: UiRect::bottom(Val::Px(16.0)),
                    ..default()
                },
            ));

            // Buttons
            spawn_button(parent, MenuButton::PlaySingle, "Play Single");
            spawn_button(parent, MenuButton::PlayMultiplayer, "Play Multiplayer (Sim)");
        });
}

fn spawn_button(parent: &mut ChildSpawnerCommands, tag: MenuButton, label: &str) {
    parent
        .spawn((
            tag,
            Button,
            Node {
                width: Val::Px(320.0),
                height: Val::Px(64.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.15, 0.15, 0.25)),
            BorderColor::all(Color::srgb(0.5, 0.5, 0.7)),
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(label),
                TextFont {
                    font_size: FontSize::Px(28.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

// ── Interaction ───────────────────────────────────────────────────────────────

fn handle_menu_buttons(
    mut interaction_q: Query<
        (&Interaction, &MenuButton, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for (interaction, btn, mut bg) in &mut interaction_q {
        match interaction {
            Interaction::Pressed => {
                *bg = BackgroundColor(Color::srgb(0.30, 0.30, 0.50));
                match btn {
                    MenuButton::PlaySingle => {
                        next_state.set(GameState::PlayingSingle);
                    }
                    MenuButton::PlayMultiplayer => {
                        next_state.set(GameState::PlayingMultiplayer);
                    }
                }
            }
            Interaction::Hovered => {
                *bg = BackgroundColor(Color::srgb(0.22, 0.22, 0.38));
            }
            Interaction::None => {
                *bg = BackgroundColor(Color::srgb(0.15, 0.15, 0.25));
            }
        }
    }
}

// ── Cleanup ───────────────────────────────────────────────────────────────────

fn despawn_main_menu(mut commands: Commands, q: Query<Entity, With<MenuEntity>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}
