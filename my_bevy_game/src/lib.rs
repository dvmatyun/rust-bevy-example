use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
};

// Golden angle gives the most uniform angular spread when adding cubes one at a time.
const GOLDEN_ANGLE: f32 = 2.399_963;

// Button normal/hover/press colours
const BTN_NORMAL: Color = Color::srgb(0.20, 0.20, 0.40);
const BTN_HOVER: Color = Color::srgb(0.28, 0.28, 0.52);
const BTN_PRESS: Color = Color::srgb(0.35, 0.35, 0.65);

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

#[derive(Component)]
struct OrbitingCube {
    radius: f32,
    speed: f32,
    angle_offset: f32,
}

#[derive(Component)]
enum ButtonAction {
    AddOne,
    SubOne,
    AddTen,
    SubTen,
    AddHundred,
    SubHundred,
}

#[derive(Component)]
struct HudText;

#[derive(Component)]
struct CubeCountLabel;

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

#[derive(Resource)]
struct CubeSettings {
    radius: f32,
    speed: f32,
}

#[derive(Resource)]
struct CubeCount {
    target: i32,
    /// Monotonically increasing index — used for unique angle offsets and colours.
    next_index: u32,
}

#[derive(Resource)]
struct CubeMesh(Handle<Mesh>);

/// Circular buffer of recent raw frame times (seconds) for percentile stats.
#[derive(Resource)]
struct FrameTimeHistory {
    times: std::collections::VecDeque<f32>,
}

impl FrameTimeHistory {
    fn new(capacity: usize) -> Self {
        Self {
            times: std::collections::VecDeque::with_capacity(capacity),
        }
    }

    fn push(&mut self, dt: f32, capacity: usize) {
        self.times.push_back(dt);
        while self.times.len() > capacity {
            self.times.pop_front();
        }
    }

    fn avg_fps(&self) -> f32 {
        if self.times.is_empty() {
            return 0.0;
        }
        let mean = self.times.iter().sum::<f32>() / self.times.len() as f32;
        if mean > 0.0 { 1.0 / mean } else { 0.0 }
    }

    /// 1% low: average FPS of the worst 1% of frames.
    fn one_pct_low_fps(&self) -> f32 {
        if self.times.is_empty() {
            return 0.0;
        }
        let mut sorted: Vec<f32> = self.times.iter().copied().collect();
        sorted.sort_by(|a: &f32, b: &f32| b.total_cmp(a)); // descending (slowest first)
        let count = ((sorted.len() as f32 * 0.01).ceil() as usize).max(1);
        let worst_mean = sorted[..count].iter().sum::<f32>() / count as f32;
        if worst_mean > 0.0 { 1.0 / worst_mean } else { 0.0 }
    }
}

// ---------------------------------------------------------------------------
// Colour palette
// ---------------------------------------------------------------------------

fn cube_color(index: usize) -> Color {
    match index % 8 {
        0 => Color::srgb(1.0, 0.3, 0.3),
        1 => Color::srgb(1.0, 0.7, 0.2),
        2 => Color::srgb(0.3, 1.0, 0.3),
        3 => Color::srgb(0.2, 0.8, 1.0),
        4 => Color::srgb(0.6, 0.3, 1.0),
        5 => Color::srgb(1.0, 0.3, 0.9),
        6 => Color::srgb(1.0, 0.6, 0.1),
        _ => Color::srgb(0.5, 1.0, 0.5),
    }
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 8.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Directional light
    commands.spawn((
        DirectionalLight {
            illuminance: 3000.0,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Ground plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(12.0, 12.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.25, 0.45, 0.25))),
    ));

    // Shared cube mesh handle stored as a resource so sync_cube_count can reuse it.
    commands.insert_resource(CubeMesh(meshes.add(Cuboid::new(0.6, 0.6, 0.6))));

    // ── UI ──────────────────────────────────────────────────────────────────
    // Root: full-screen flex-column; top = HUD, bottom = button bar.
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::SpaceBetween,
            ..default()
        })
        .with_children(|root| {
            // ── Top HUD ─────────────────────────────────────────────────────
            root.spawn(Node {
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            })
            .with_children(|top| {
                top.spawn(Node {
                    padding: UiRect::all(Val::Px(8.0)),
                    border_radius: BorderRadius::all(Val::Px(6.0)),
                    ..default()
                })
                .insert(BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)))
                .with_children(|panel| {
                    panel
                        .spawn(Text::new("FPS: --  |  Frame: --ms  |  Cubes: 0"))
                        .insert(TextFont {
                            font_size: 15.0,
                            ..default()
                        })
                        .insert(TextColor(Color::WHITE))
                        .insert(HudText);
                });
            });

            // ── Bottom button bar ────────────────────────────────────────────
            root.spawn(Node {
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(12.0)),
                ..default()
            })
            .with_children(|bottom| {
                bottom
                    .spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(6.0),
                        align_items: AlignItems::Center,
                        padding: UiRect::all(Val::Px(10.0)),
                        border_radius: BorderRadius::all(Val::Px(8.0)),
                        ..default()
                    })
                    .insert(BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)))
                    .with_children(|bar| {
                        for (label, action) in [
                            ("-100", ButtonAction::SubHundred),
                            (" -10", ButtonAction::SubTen),
                            ("  -1", ButtonAction::SubOne),
                        ] {
                            bar.spawn(Button)
                                .insert(Node {
                                    width: Val::Px(50.0),
                                    height: Val::Px(34.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    border: UiRect::all(Val::Px(1.0)),
                                    border_radius: BorderRadius::all(Val::Px(4.0)),
                                    ..default()
                                })
                                .insert(BackgroundColor(BTN_NORMAL))
                                .insert(BorderColor::all(Color::srgb(0.4, 0.4, 0.7)))
                                .insert(action)
                                .with_children(|btn| {
                                    btn.spawn(Text::new(label))
                                        .insert(TextFont { font_size: 15.0, ..default() })
                                        .insert(TextColor(Color::WHITE));
                                });
                        }

                        // Centre label showing target cube count
                        bar.spawn(Node {
                            width: Val::Px(82.0),
                            justify_content: JustifyContent::Center,
                            ..default()
                        })
                        .with_children(|lbl| {
                            lbl.spawn(Text::new("Cubes: 6"))
                                .insert(TextFont { font_size: 14.0, ..default() })
                                .insert(TextColor(Color::srgb(0.85, 0.85, 0.85)))
                                .insert(CubeCountLabel);
                        });

                        for (label, action) in [
                            ("+1  ", ButtonAction::AddOne),
                            ("+10 ", ButtonAction::AddTen),
                            ("+100", ButtonAction::AddHundred),
                        ] {
                            bar.spawn(Button)
                                .insert(Node {
                                    width: Val::Px(50.0),
                                    height: Val::Px(34.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    border: UiRect::all(Val::Px(1.0)),
                                    border_radius: BorderRadius::all(Val::Px(4.0)),
                                    ..default()
                                })
                                .insert(BackgroundColor(BTN_NORMAL))
                                .insert(BorderColor::all(Color::srgb(0.4, 0.4, 0.7)))
                                .insert(action)
                                .with_children(|btn| {
                                    btn.spawn(Text::new(label))
                                        .insert(TextFont { font_size: 15.0, ..default() })
                                        .insert(TextColor(Color::WHITE));
                                });
                        }
                    });
            });
        });
}


fn orbit_cubes(time: Res<Time>, mut query: Query<(&OrbitingCube, &mut Transform)>) {
    for (orbit, mut transform) in &mut query {
        let angle = time.elapsed_secs() * orbit.speed + orbit.angle_offset;
        transform.translation =
            Vec3::new(angle.cos() * orbit.radius, 0.3, angle.sin() * orbit.radius);
        transform.rotation = Quat::from_rotation_y(angle * 2.0);
    }
}

/// Spawns or despawns cubes to match `CubeCount::target`.
fn sync_cube_count(
    mut commands: Commands,
    cube_mesh: Res<CubeMesh>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cube_count: ResMut<CubeCount>,
    cubes: Query<Entity, With<OrbitingCube>>,
    settings: Res<CubeSettings>,
) {
    let current = cubes.iter().count() as i32;
    let target = cube_count.target;

    if current == target {
        return;
    }

    if current < target {
        for _ in 0..(target - current) {
            let idx = cube_count.next_index;
            cube_count.next_index += 1;
            commands.spawn((
                Mesh3d(cube_mesh.0.clone()),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: cube_color(idx as usize),
                    ..default()
                })),
                Transform::default(),
                OrbitingCube {
                    radius: settings.radius,
                    speed: settings.speed,
                    angle_offset: idx as f32 * GOLDEN_ANGLE,
                },
            ));
        }
    } else {
        let to_remove = (current - target) as usize;
        for entity in cubes.iter().take(to_remove) {
            commands.entity(entity).despawn();
        }
    }
}

/// Handles button presses and hover highlight.
fn button_system(
    mut interaction_query: Query<
        (&Interaction, &ButtonAction, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
    mut cube_count: ResMut<CubeCount>,
) {
    for (interaction, action, mut bg) in &mut interaction_query {
        match interaction {
            Interaction::Pressed => {
                *bg = BackgroundColor(BTN_PRESS);
                let delta = match action {
                    ButtonAction::AddOne => 1,
                    ButtonAction::SubOne => -1,
                    ButtonAction::AddTen => 10,
                    ButtonAction::SubTen => -10,
                    ButtonAction::AddHundred => 100,
                    ButtonAction::SubHundred => -100,
                };
                cube_count.target = (cube_count.target + delta).max(0);
            }
            Interaction::Hovered => *bg = BackgroundColor(BTN_HOVER),
            Interaction::None => *bg = BackgroundColor(BTN_NORMAL),
        }
    }
}

/// Records the current frame time into `FrameTimeHistory` every frame.
fn record_frame_time(time: Res<Time>, mut history: ResMut<FrameTimeHistory>) {
    history.push(time.delta_secs(), 120);
}

/// Updates the HUD overlay and the cube-count label every frame.
fn update_hud(
    diagnostics: Res<DiagnosticsStore>,
    history: Res<FrameTimeHistory>,
    mut hud_query: Query<&mut Text, With<HudText>>,
    mut label_query: Query<&mut Text, (With<CubeCountLabel>, Without<HudText>)>,
    cubes: Query<(), With<OrbitingCube>>,
    cube_count: Res<CubeCount>,
) {
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);
    let frame_ms = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0)
        * 1000.0;

    let avg_fps = history.avg_fps();
    let one_pct_low = history.one_pct_low_fps();
    let cube_actual = cubes.iter().count();

    for mut text in &mut hud_query {
        *text = Text::new(format!(
            "FPS {fps:.0}  Avg {avg_fps:.0}  1%Low {one_pct_low:.0}  |  {frame_ms:.2}ms  |  Cubes {cube_actual}"
        ));
    }
    for mut text in &mut label_query {
        *text = Text::new(format!("Cubes: {}", cube_count.target));
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[bevy_main]
pub fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            FrameTimeDiagnosticsPlugin {
                // 120 samples ≈ 0.5 s at 240 Hz — enough for a meaningful 1% low
                max_history_length: 120,
                ..default()
            },
        ))
        .insert_resource(CubeSettings {
            radius: 3.0,
            speed: 1.2,
        })
        .insert_resource(CubeCount {
            target: 6,
            next_index: 0,
        })
        .insert_resource(FrameTimeHistory::new(120))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (orbit_cubes, sync_cube_count, button_system, record_frame_time, update_hud),
        )
        .run();
}
