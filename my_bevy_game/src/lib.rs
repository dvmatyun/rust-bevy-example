//! 3D scene with orbiting cubes, FPS HUD, and cube count controls.
//! Android Mali-G77 and iOS compatible.

use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
    window::{AppLifecycle, WindowMode},
    winit::WinitSettings,
};

const GOLDEN_ANGLE: f32 = 2.399_963;

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
    /// Inclination from the XZ plane — tilts the orbit so cubes move through Y.
    inclination: f32,
}

#[derive(Component)]
enum ButtonAction {
    Add(i32),
    Sub(i32),
}

#[derive(Component)]
struct HudText;

#[derive(Component)]
struct CubeCountLabel;

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

#[derive(Resource)]
struct CubeCount {
    target: i32,
    next_index: u32,
}

#[derive(Resource)]
struct CubeMesh(Handle<Mesh>);

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

    fn one_pct_low_fps(&self) -> f32 {
        if self.times.is_empty() {
            return 0.0;
        }
        let mut sorted: Vec<f32> = self.times.iter().copied().collect();
        sorted.sort_by(|a, b| b.total_cmp(a));
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

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Ground plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(12.0, 12.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.25, 0.45, 0.25))),
    ));

    // Light — NO shadows on Android (segfault on Mali)
    commands.spawn((
        PointLight {
            intensity: 1_000_000.0,
            #[cfg(not(target_os = "android"))]
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));

    // Camera — MSAA off on Android (Vulkan panic on Mali)
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 8.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
        #[cfg(target_os = "android")]
        Msaa::Off,
    ));

    // Shared cube mesh
    commands.insert_resource(CubeMesh(meshes.add(Cuboid::new(0.6, 0.6, 0.6))));

    // ── UI ───────────────────────────────────────────────────────────────
    // Safe-area padding: Bevy 0.19-dev has no safe-area API, so we use
    // generous fixed insets that cover status bars, notches, and home
    // indicators on common devices.
    let safe_top = Val::Px(48.0);    // status bar + notch
    let safe_bottom = Val::Px(66.0);  // home indicator / nav bar + 32px extra
    let safe_side = Val::Px(20.0);    // minimum side padding

    // HUD text (top, respects safe area, constrained to screen width)
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: safe_side,
                right: safe_side,
                top: safe_top,
                padding: UiRect::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
        ))
        .with_child((
            Text::new("FPS: --"),
            TextFont {
                font_size: FontSize::Px(16.0),
                ..default()
            },
            TextColor(Color::WHITE),
            HudText,
        ));

    // Button bar (bottom, wraps to next line if it doesn't fit)
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: safe_bottom,
                left: safe_side,
                right: safe_side,
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                column_gap: Val::Px(6.0),
                row_gap: Val::Px(6.0),
                ..default()
            },
        ))
        .with_children(|bar| {
            for (label, action) in [
                ("-1000", ButtonAction::Sub(1000)),
                ("-50", ButtonAction::Sub(50)),
                ("-1", ButtonAction::Sub(1)),
            ] {
                spawn_button(bar, label, action);
            }

            // Cube count label
            bar.spawn((
                Node {
                    min_width: Val::Px(90.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
            ))
            .with_child((
                Text::new("Cubes: 6"),
                TextFont {
                    font_size: FontSize::Px(18.0),
                    ..default()
                },
                TextColor(Color::WHITE),
                CubeCountLabel,
            ));

            for (label, action) in [
                ("+1", ButtonAction::Add(1)),
                ("+50", ButtonAction::Add(50)),
                ("+1000", ButtonAction::Add(1000)),
            ] {
                spawn_button(bar, label, action);
            }
        });
}

fn spawn_button(parent: &mut ChildSpawnerCommands, label: &str, action: ButtonAction) {
    parent
        .spawn((
            Button,
            Node {
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(BTN_NORMAL),
            action,
        ))
        .with_child((
            Text::new(label),
            TextFont {
                font_size: FontSize::Px(18.0),
                ..default()
            },
            TextColor(Color::WHITE),
        ));
}

fn orbit_cubes(time: Res<Time>, mut query: Query<(&OrbitingCube, &mut Transform)>) {
    for (orbit, mut transform) in &mut query {
        let angle = time.elapsed_secs() * orbit.speed + orbit.angle_offset;
        // Spherical orbit: inclination tilts the circular path so each cube
        // sweeps a unique great-circle on a sphere, using all three axes.
        let x = angle.cos() * orbit.radius;
        let flat_z = angle.sin() * orbit.radius;
        let y = flat_z * orbit.inclination.sin() + 0.5;
        let z = flat_z * orbit.inclination.cos();
        transform.translation = Vec3::new(x, y.abs(), z);
        transform.rotation = Quat::from_rotation_y(angle * 2.0);
    }
}

fn sync_cube_count(
    mut commands: Commands,
    cube_mesh: Res<CubeMesh>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cube_count: ResMut<CubeCount>,
    cubes: Query<Entity, With<OrbitingCube>>,
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
            let radius = 3.0 + (idx as f32 * 0.05).sin() * 1.5;
            let speed = 1.0 + (idx as f32 * 0.3).cos() * 0.5;
            // Golden-angle based inclination gives each cube a unique orbital tilt
            let inclination = (idx as f32 * GOLDEN_ANGLE * 0.7).sin() * 1.2;
            commands.spawn((
                Mesh3d(cube_mesh.0.clone()),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: cube_color(idx as usize),
                    ..default()
                })),
                Transform::default(),
                OrbitingCube {
                    radius,
                    speed,
                    angle_offset: idx as f32 * GOLDEN_ANGLE,
                    inclination,
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
                    ButtonAction::Add(n) => *n,
                    ButtonAction::Sub(n) => -(*n),
                };
                cube_count.target = (cube_count.target + delta).max(0);
            }
            Interaction::Hovered => *bg = BackgroundColor(BTN_HOVER),
            Interaction::None => *bg = BackgroundColor(BTN_NORMAL),
        }
    }
}

fn record_frame_time(time: Res<Time>, mut history: ResMut<FrameTimeHistory>) {
    history.push(time.delta_secs(), 120);
}

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

/// Touch camera controls (works on both Android and iOS):
/// - 1-finger drag: orbit camera around the origin
/// - 2-finger pinch: zoom in/out (distance between fingers)
fn touch_camera(
    touches: Res<Touches>,
    mut camera_transform: Single<&mut Transform, With<Camera3d>>,
    mut last_orbit_pos: Local<Option<Vec2>>,
    mut last_pinch_dist: Local<Option<f32>>,
) {
    let pressed: Vec<&bevy::input::touch::Touch> = touches.iter().collect();

    match pressed.len() {
        1 => {
            // Single finger: orbit
            *last_pinch_dist = None;
            let finger = pressed[0];
            let pos = finger.position();
            if let Some(last) = *last_orbit_pos {
                let delta = pos - last;
                if delta.length() > 0.5 {
                    let dist = camera_transform.translation.length();
                    let (mut azimuth, mut elevation) =
                        camera_spherical(&camera_transform);
                    azimuth -= delta.x * 0.008;
                    elevation = (elevation - delta.y * 0.005).clamp(0.15, 1.4);
                    camera_transform.translation = Vec3::new(
                        dist * elevation.sin() * azimuth.sin(),
                        dist * elevation.cos(),
                        dist * elevation.sin() * azimuth.cos(),
                    );
                    **camera_transform =
                        camera_transform.looking_at(Vec3::ZERO, Vec3::Y);
                }
            }
            *last_orbit_pos = Some(pos);
        }
        2 => {
            // Two fingers: pinch to zoom
            *last_orbit_pos = None;
            let a = pressed[0].position();
            let b = pressed[1].position();
            let current_dist = a.distance(b);

            if let Some(prev_dist) = *last_pinch_dist {
                if prev_dist > 1.0 {
                    let scale = prev_dist / current_dist; // >1 = zoom out, <1 = zoom in
                    let cam_dist = camera_transform.translation.length();
                    let new_dist = (cam_dist * scale).clamp(3.0, 30.0);
                    camera_transform.translation =
                        camera_transform.translation.normalize() * new_dist;
                }
            }
            *last_pinch_dist = Some(current_dist);
        }
        _ => {
            // No fingers or 3+: reset tracking
            *last_orbit_pos = None;
            *last_pinch_dist = None;
        }
    }
}

/// Extract spherical angles (azimuth, elevation) from camera position.
fn camera_spherical(transform: &Transform) -> (f32, f32) {
    let p = transform.translation;
    let dist = p.length();
    let elevation = (p.y / dist).acos();
    let azimuth = p.x.atan2(p.z);
    (azimuth, elevation)
}

fn handle_lifetime(mut events: MessageReader<AppLifecycle>) {
    for _e in events.read() {}
}

fn winit_settings() -> WinitSettings {
    #[cfg(any(target_os = "android", target_os = "ios"))]
    { WinitSettings::mobile() }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    { WinitSettings::game() }
}

fn window_settings() -> Window {
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        Window {
            resizable: false,
            mode: WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
            recognize_rotation_gesture: true,
            prefers_home_indicator_hidden: true,
            prefers_status_bar_hidden: true,
            ..default()
        }
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        Window {
            title: "My Bevy Game".to_string(),
            ..default()
        }
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[bevy_main]
pub fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(window_settings()),
                ..default()
            }),
            FrameTimeDiagnosticsPlugin {
                max_history_length: 120,
                ..default()
            },
        ))
        .insert_resource(winit_settings())
        .insert_resource(CubeCount {
            target: 6,
            next_index: 0,
        })
        .insert_resource(FrameTimeHistory::new(120))
        .add_systems(Startup, setup_scene)
        .add_systems(
            Update,
            (
                orbit_cubes,
                sync_cube_count,
                button_system,
                record_frame_time,
                update_hud,
                touch_camera,
                handle_lifetime,
            ),
        )
        .run();
}
