//! A simple asteroid-dodge game built with the Bevy game engine.
//!
//! Controls:
//!   - Left / Right arrow keys (or A / D) to move the player ship.
//!   - Space to restart after game over.

use bevy::prelude::*;
use rand::Rng;

// ── Window / world constants ──────────────────────────────────────────────────
const WINDOW_WIDTH: f32 = 800.0;
const WINDOW_HEIGHT: f32 = 600.0;
const HALF_W: f32 = WINDOW_WIDTH / 2.0;
const HALF_H: f32 = WINDOW_HEIGHT / 2.0;

// ── Player constants ──────────────────────────────────────────────────────────
const PLAYER_SPEED: f32 = 400.0;
const PLAYER_W: f32 = 60.0;
const PLAYER_H: f32 = 20.0;
const PLAYER_START_Y: f32 = -HALF_H + 50.0;

// ── Enemy (asteroid) constants ────────────────────────────────────────────────
const ENEMY_SIZE: f32 = 30.0;
const ENEMY_SPEED_BASE: f32 = 160.0;
/// Asteroids get faster as your score climbs.
const ENEMY_SPEED_SCALE: f32 = 4.0;
const SPAWN_INTERVAL: f32 = 1.0;

// ── Game state ────────────────────────────────────────────────────────────────
#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
enum AppState {
    #[default]
    Playing,
    GameOver,
}

// ── Components ────────────────────────────────────────────────────────────────
#[derive(Component)]
struct Player;

#[derive(Component)]
struct Enemy;

#[derive(Component)]
struct ScoreText;

#[derive(Component)]
struct GameOverOverlay;

// ── Resources ─────────────────────────────────────────────────────────────────
#[derive(Resource, Default)]
struct Score(u32);

#[derive(Resource)]
struct SpawnTimer(Timer);

#[derive(Resource)]
struct ScoreTimer(Timer);

// ── Entry point ───────────────────────────────────────────────────────────────
fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Asteroid Dodge – Bevy".to_string(),
                    resolution: (WINDOW_WIDTH, WINDOW_HEIGHT).into(),
                    resizable: false,
                    ..default()
                }),
                ..default()
            }),
        )
        .init_state::<AppState>()
        .init_resource::<Score>()
        .insert_resource(SpawnTimer(Timer::from_seconds(
            SPAWN_INTERVAL,
            TimerMode::Repeating,
        )))
        .insert_resource(ScoreTimer(Timer::from_seconds(1.0, TimerMode::Repeating)))
        .insert_resource(ClearColor(Color::srgb(0.08, 0.08, 0.15)))
        .add_systems(Startup, setup)
        // ── Playing systems ───────────────────────────────────────────────────
        .add_systems(
            Update,
            (
                move_player,
                spawn_enemies,
                move_enemies,
                tick_score,
                check_collisions,
            )
                .run_if(in_state(AppState::Playing)),
        )
        // ── GameOver systems ──────────────────────────────────────────────────
        .add_systems(OnEnter(AppState::GameOver), show_game_over)
        .add_systems(
            Update,
            handle_restart.run_if(in_state(AppState::GameOver)),
        )
        .add_systems(OnExit(AppState::GameOver), cleanup_game_over)
        .run();
}

// ── Setup ─────────────────────────────────────────────────────────────────────
fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());

    // Player ship (bright blue rectangle).
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::srgb(0.25, 0.55, 1.0),
                custom_size: Some(Vec2::new(PLAYER_W, PLAYER_H)),
                ..default()
            },
            transform: Transform::from_xyz(0.0, PLAYER_START_Y, 0.0),
            ..default()
        },
        Player,
    ));

    // HUD score label (top-left).
    commands.spawn((
        TextBundle::from_section(
            "Score: 0",
            TextStyle {
                font_size: 28.0,
                color: Color::srgb(0.9, 0.9, 0.9),
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(14.0),
            ..default()
        }),
        ScoreText,
    ));

    // Small control-hint at the top-right.
    commands.spawn(
        TextBundle::from_section(
            "← / → to move",
            TextStyle {
                font_size: 18.0,
                color: Color::srgba(0.7, 0.7, 0.7, 0.7),
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            right: Val::Px(14.0),
            ..default()
        }),
    );
}

// ── Playing systems ───────────────────────────────────────────────────────────

fn move_player(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Transform, With<Player>>,
) {
    for mut transform in &mut query {
        let mut direction = 0.0_f32;
        if keys.pressed(KeyCode::ArrowLeft) || keys.pressed(KeyCode::KeyA) {
            direction -= 1.0;
        }
        if keys.pressed(KeyCode::ArrowRight) || keys.pressed(KeyCode::KeyD) {
            direction += 1.0;
        }
        transform.translation.x += direction * PLAYER_SPEED * time.delta_seconds();
        transform.translation.x = transform
            .translation
            .x
            .clamp(-HALF_W + PLAYER_W / 2.0, HALF_W - PLAYER_W / 2.0);
    }
}

fn spawn_enemies(
    mut commands: Commands,
    time: Res<Time>,
    mut timer: ResMut<SpawnTimer>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        let mut rng = rand::thread_rng();
        let x = rng.gen_range((-HALF_W + ENEMY_SIZE)..(HALF_W - ENEMY_SIZE));
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: Color::srgb(1.0, 0.35, 0.2),
                    custom_size: Some(Vec2::splat(ENEMY_SIZE)),
                    ..default()
                },
                transform: Transform::from_xyz(x, HALF_H + ENEMY_SIZE, 0.0),
                ..default()
            },
            Enemy,
        ));
    }
}

fn move_enemies(
    time: Res<Time>,
    score: Res<Score>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform), With<Enemy>>,
) {
    let speed = ENEMY_SPEED_BASE + score.0 as f32 * ENEMY_SPEED_SCALE;
    for (entity, mut transform) in &mut query {
        transform.translation.y -= speed * time.delta_seconds();
        if transform.translation.y < -HALF_H - ENEMY_SIZE {
            commands.entity(entity).despawn();
        }
    }
}

fn tick_score(
    time: Res<Time>,
    mut timer: ResMut<ScoreTimer>,
    mut score: ResMut<Score>,
    mut query: Query<&mut Text, With<ScoreText>>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        score.0 += 1;
        for mut text in &mut query {
            text.sections[0].value = format!("Score: {}", score.0);
        }
    }
}

fn check_collisions(
    player_query: Query<&Transform, With<Player>>,
    enemy_query: Query<&Transform, With<Enemy>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    let Ok(player_tf) = player_query.get_single() else {
        return;
    };
    let p = player_tf.translation.truncate();

    for enemy_tf in &enemy_query {
        let e = enemy_tf.translation.truncate();
        let diff = (p - e).abs();
        // AABB collision check.
        if diff.x < (PLAYER_W / 2.0 + ENEMY_SIZE / 2.0)
            && diff.y < (PLAYER_H / 2.0 + ENEMY_SIZE / 2.0)
        {
            next_state.set(AppState::GameOver);
            return;
        }
    }
}

// ── GameOver systems ──────────────────────────────────────────────────────────

fn show_game_over(mut commands: Commands, score: Res<Score>) {
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(16.0),
                    ..default()
                },
                background_color: Color::srgba(0.0, 0.0, 0.0, 0.65).into(),
                ..default()
            },
            GameOverOverlay,
        ))
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                "GAME OVER",
                TextStyle {
                    font_size: 72.0,
                    color: Color::srgb(1.0, 0.25, 0.2),
                    ..default()
                },
            ));
            parent.spawn(TextBundle::from_section(
                format!("Final Score: {}", score.0),
                TextStyle {
                    font_size: 40.0,
                    color: Color::WHITE,
                    ..default()
                },
            ));
            parent.spawn(TextBundle::from_section(
                "Press SPACE to play again",
                TextStyle {
                    font_size: 24.0,
                    color: Color::srgb(0.75, 0.75, 0.75),
                    ..default()
                },
            ));
        });
}

fn handle_restart(
    keys: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<AppState>>,
    mut commands: Commands,
    enemy_query: Query<Entity, With<Enemy>>,
    mut player_query: Query<&mut Transform, With<Player>>,
    mut score: ResMut<Score>,
    mut spawn_timer: ResMut<SpawnTimer>,
    mut score_timer: ResMut<ScoreTimer>,
    mut score_text_query: Query<&mut Text, With<ScoreText>>,
) {
    if keys.just_pressed(KeyCode::Space) {
        // Reset counters.
        score.0 = 0;
        spawn_timer.0.reset();
        score_timer.0.reset();

        // Clear all existing asteroids.
        for entity in &enemy_query {
            commands.entity(entity).despawn();
        }

        // Return player to starting position.
        for mut tf in &mut player_query {
            tf.translation = Vec3::new(0.0, PLAYER_START_Y, 0.0);
        }

        // Reset HUD text.
        for mut text in &mut score_text_query {
            text.sections[0].value = "Score: 0".to_string();
        }

        next_state.set(AppState::Playing);
    }
}

fn cleanup_game_over(mut commands: Commands, query: Query<Entity, With<GameOverOverlay>>) {
    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }
}
