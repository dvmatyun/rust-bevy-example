//! Voxel terrain demo: Minecraft-style heightmap world with a billboard
//! player rectangle controlled by WASD.
//!
//! Architecture: each domain owns a `Plugin`. Terrain generation uses
//! hash-based 2D value noise with FBM (no extra crate dependencies).

use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
    window::AppLifecycle,
    winit::WinitSettings,
};
#[cfg(any(target_os = "android", target_os = "ios"))]
use bevy::window::WindowMode;

// === World tuning ===========================================================
const WORLD_SIZE: i32 = 64;       // 64x64 columns of voxel blocks
const NOISE_SCALE: f32 = 0.08;    // smaller = larger features
const HEIGHT_SCALE: f32 = 9.0;    // peak height in blocks
const PLAYER_SPEED: f32 = 8.0;
const PLAYER_HALF_HEIGHT: f32 = 0.7;
const CAMERA_OFFSET: Vec3 = Vec3::new(0.0, 14.0, 14.0);

// ============================================================================
// Components
// ============================================================================

#[derive(Component)]
struct Player;

#[derive(Component)]
struct GameCamera;

#[derive(Component)]
struct TerrainBlock;

#[derive(Component)]
struct HudText;

// ============================================================================
// Resources
// ============================================================================

/// Precomputed surface height for every world column.
#[derive(Resource)]
struct TerrainHeights {
    /// Row-major: heights[(z + half) * size + (x + half)]
    cells: Vec<i32>,
}

impl TerrainHeights {
    fn at(&self, x: i32, z: i32) -> i32 {
        let half = WORLD_SIZE / 2;
        let lx = (x + half).clamp(0, WORLD_SIZE - 1) as usize;
        let lz = (z + half).clamp(0, WORLD_SIZE - 1) as usize;
        self.cells[lz * WORLD_SIZE as usize + lx]
    }

    /// World-space Y of the top of the surface block at this XZ.
    fn ground_y(&self, world_x: f32, world_z: f32) -> f32 {
        let xi = world_x.round() as i32;
        let zi = world_z.round() as i32;
        // Block centred at y=h, top face at y=h+0.5
        self.at(xi, zi) as f32 + 0.5
    }
}

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

// ============================================================================
// Plugins
// ============================================================================

struct TerrainPlugin;
impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(Color::srgb(0.55, 0.75, 0.92)))
            .insert_resource(GlobalAmbientLight {
                color: Color::WHITE,
                brightness: 250.0,
                ..default()
            })
            .add_systems(Startup, setup_terrain);
    }
}

struct PlayerPlugin;
impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_player)
            .add_systems(
                Update,
                (player_input, snap_to_ground, billboard_player).chain(),
            );
    }
}

struct CameraFollowPlugin;
impl Plugin for CameraFollowPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera)
            .add_systems(Update, follow_player);
    }
}

struct HudPlugin;
impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin {
            max_history_length: 120,
            ..default()
        })
        .insert_resource(FrameTimeHistory::new(120))
        .add_systems(Startup, spawn_hud)
        .add_systems(Update, (record_frame_time, update_hud));
    }
}

// ============================================================================
// Terrain
// ============================================================================

fn setup_terrain(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Sun
    commands.spawn((
        DirectionalLight {
            illuminance: 6000.0,
            #[cfg(not(target_os = "android"))]
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(40.0, 80.0, 40.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    let cube_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));

    // Height-band materials (Minecraft-ish biomes by elevation)
    let mat_water = materials.add(Color::srgb(0.20, 0.40, 0.85));
    let mat_sand = materials.add(Color::srgb(0.85, 0.78, 0.45));
    let mat_grass = materials.add(Color::srgb(0.30, 0.65, 0.25));
    let mat_rock = materials.add(Color::srgb(0.45, 0.40, 0.35));
    let mat_snow = materials.add(Color::srgb(0.95, 0.95, 0.97));

    let half = WORLD_SIZE / 2;
    let mut cells = vec![0i32; (WORLD_SIZE * WORLD_SIZE) as usize];

    for z in 0..WORLD_SIZE {
        for x in 0..WORLD_SIZE {
            let wx = x - half;
            let wz = z - half;
            let h = terrain_height_at(wx, wz);
            cells[(z * WORLD_SIZE + x) as usize] = h;

            let mat = if h <= 0 {
                mat_water.clone()
            } else if h <= 1 {
                mat_sand.clone()
            } else if h <= 4 {
                mat_grass.clone()
            } else if h <= 6 {
                mat_rock.clone()
            } else {
                mat_snow.clone()
            };

            commands.spawn((
                Mesh3d(cube_mesh.clone()),
                MeshMaterial3d(mat),
                Transform::from_xyz(wx as f32, h as f32, wz as f32),
                TerrainBlock,
            ));
        }
    }

    commands.insert_resource(TerrainHeights { cells });
}

fn terrain_height_at(x: i32, z: i32) -> i32 {
    // FBM in [0,1] roughly; centre and scale to a signed integer height.
    let n = fbm(x as f32 * NOISE_SCALE, z as f32 * NOISE_SCALE);
    ((n - 0.3) * HEIGHT_SCALE).round() as i32
}

// === Hash-based value noise (deterministic, no extra deps) ==================

fn hash2(x: i32, z: i32) -> f32 {
    let mut h = (x as u32)
        .wrapping_mul(374761393)
        .wrapping_add((z as u32).wrapping_mul(668265263));
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    h ^= h >> 16;
    (h as f32) / (u32::MAX as f32)
}

fn smoothstep(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn value_noise(x: f32, z: f32) -> f32 {
    let xi = x.floor() as i32;
    let zi = z.floor() as i32;
    let u = smoothstep(x - xi as f32);
    let v = smoothstep(z - zi as f32);
    let n00 = hash2(xi, zi);
    let n10 = hash2(xi + 1, zi);
    let n01 = hash2(xi, zi + 1);
    let n11 = hash2(xi + 1, zi + 1);
    let a = n00 * (1.0 - u) + n10 * u;
    let b = n01 * (1.0 - u) + n11 * u;
    a * (1.0 - v) + b * v
}

/// Fractional Brownian motion: stack 4 octaves of value noise.
fn fbm(x: f32, z: f32) -> f32 {
    let mut total = 0.0;
    let mut amp = 1.0;
    let mut freq = 1.0;
    let mut max_amp = 0.0;
    for _ in 0..4 {
        total += value_noise(x * freq, z * freq) * amp;
        max_amp += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    total / max_amp
}

// ============================================================================
// Player
// ============================================================================

fn spawn_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Thin upright slab — reads as a 2D rectangle in the 3D world.
    // `unlit` keeps the colour vivid regardless of lighting.
    let mesh = meshes.add(Cuboid::new(0.8, 1.4, 0.05));
    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.30, 0.20),
        unlit: true,
        cull_mode: None, // visible from both sides
        ..default()
    });

    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        // Spawn high; snap_to_ground places it on the surface next frame.
        Transform::from_xyz(0.0, HEIGHT_SCALE + 5.0, 0.0),
        Player,
    ));
}

fn player_input(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut q: Query<&mut Transform, With<Player>>,
) {
    let mut dir = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        dir.z -= 1.0;
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        dir.z += 1.0;
    }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        dir.x -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        dir.x += 1.0;
    }
    if dir == Vec3::ZERO {
        return;
    }
    let dir = dir.normalize();
    let bound = (WORLD_SIZE / 2 - 1) as f32;
    let dt = time.delta_secs();
    for mut tf in &mut q {
        tf.translation.x =
            (tf.translation.x + dir.x * PLAYER_SPEED * dt).clamp(-bound, bound);
        tf.translation.z =
            (tf.translation.z + dir.z * PLAYER_SPEED * dt).clamp(-bound, bound);
    }
}

fn snap_to_ground(
    terrain: Option<Res<TerrainHeights>>,
    mut q: Query<&mut Transform, With<Player>>,
) {
    let Some(terrain) = terrain else { return };
    for mut tf in &mut q {
        tf.translation.y =
            terrain.ground_y(tf.translation.x, tf.translation.z) + PLAYER_HALF_HEIGHT;
    }
}

/// Rotate the player slab around Y so its flat face is always toward the
/// camera — the "2D sprite in a 3D world" effect.
fn billboard_player(
    cam_q: Query<&Transform, (With<GameCamera>, Without<Player>)>,
    mut player_q: Query<&mut Transform, With<Player>>,
) {
    let Ok(cam_tf) = cam_q.single() else { return };
    for mut tf in &mut player_q {
        let to_cam = cam_tf.translation - tf.translation;
        let yaw = to_cam.x.atan2(to_cam.z);
        tf.rotation = Quat::from_rotation_y(yaw);
    }
}

// ============================================================================
// Camera
// ============================================================================

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(CAMERA_OFFSET).looking_at(Vec3::ZERO, Vec3::Y),
        GameCamera,
        #[cfg(target_os = "android")]
        Msaa::Off,
    ));
}

/// Smooth third-person follow: lerp toward `player + offset`, then look at
/// the player.
fn follow_player(
    time: Res<Time>,
    player_q: Query<&Transform, (With<Player>, Without<GameCamera>)>,
    mut cam_q: Query<&mut Transform, With<GameCamera>>,
) {
    let Ok(player_tf) = player_q.single() else {
        return;
    };
    let Ok(mut cam_tf) = cam_q.single_mut() else {
        return;
    };
    let target = player_tf.translation + CAMERA_OFFSET;
    let t = (time.delta_secs() * 6.0).min(1.0);
    cam_tf.translation = cam_tf.translation.lerp(target, t);
    let look = cam_tf.looking_at(player_tf.translation, Vec3::Y);
    cam_tf.rotation = look.rotation;
}

// ============================================================================
// HUD
// ============================================================================

fn spawn_hud(mut commands: Commands) {
    let safe_top = Val::Px(48.0);
    let safe_side = Val::Px(20.0);

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
}

fn record_frame_time(time: Res<Time>, mut history: ResMut<FrameTimeHistory>) {
    history.push(time.delta_secs(), 120);
}

fn update_hud(
    diagnostics: Res<DiagnosticsStore>,
    history: Res<FrameTimeHistory>,
    mut q: Query<&mut Text, With<HudText>>,
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
    let avg = history.avg_fps();
    let one_pct = history.one_pct_low_fps();
    for mut text in &mut q {
        *text = Text::new(format!(
            "FPS {fps:.0}  Avg {avg:.0}  1%Low {one_pct:.0}  |  {frame_ms:.2}ms  |  WASD to move"
        ));
    }
}

// ============================================================================
// Lifecycle / window settings
// ============================================================================

fn handle_lifetime(mut events: MessageReader<AppLifecycle>) {
    for _e in events.read() {}
}

fn winit_settings() -> WinitSettings {
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        WinitSettings::mobile()
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        WinitSettings::game()
    }
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

// ============================================================================
// Entry point
// ============================================================================

#[bevy_main]
pub fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(window_settings()),
                ..default()
            }),
            TerrainPlugin,
            CameraFollowPlugin,
            PlayerPlugin,
            HudPlugin,
        ))
        .insert_resource(winit_settings())
        .add_systems(Update, handle_lifetime)
        .run();
}
