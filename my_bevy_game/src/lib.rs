//! Minimal 3D scene — stripped down for Android Mali-G77 compatibility.

use bevy::{
    prelude::*,
    window::{AppLifecycle, WindowMode},
    winit::WinitSettings,
};

#[bevy_main]
pub fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    resizable: false,
                    mode: WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
                    ..default()
                }),
                ..default()
            }),
        )
        .insert_resource(WinitSettings::mobile())
        .add_systems(Startup, setup_scene)
        .add_systems(Update, (button_handler, handle_lifetime))
        .run();
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // ground
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(5.0, 5.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.1, 0.2, 0.1))),
    ));
    // cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(materials.add(Color::srgb(0.5, 0.4, 0.3))),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));
    // light — NO shadows on Android (causes segfault on Mali)
    commands.spawn((
        PointLight {
            intensity: 1_000_000.0,
            #[cfg(not(target_os = "android"))]
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));
    // camera — MSAA off on Android (causes Vulkan panic on Mali)
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        #[cfg(target_os = "android")]
        Msaa::Off,
    ));

    // simple button
    commands
        .spawn((
            Button,
            Node {
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                position_type: PositionType::Absolute,
                left: Val::Px(50.0),
                right: Val::Px(50.0),
                bottom: Val::Px(50.0),
                ..default()
            },
        ))
        .with_child((
            Text::new("Test Button"),
            TextFont { font_size: FontSize::Px(30.0), ..default() },
            TextColor::BLACK,
        ));
}

fn button_handler(
    mut q: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<Button>)>,
) {
    for (interaction, mut bg) in &mut q {
        *bg = match interaction {
            Interaction::Pressed => Color::srgb(0.2, 0.2, 0.8).into(),
            Interaction::Hovered => Color::srgb(0.5, 0.5, 0.5).into(),
            Interaction::None => Color::WHITE.into(),
        };
    }
}

fn handle_lifetime(mut events: MessageReader<AppLifecycle>) {
    for _e in events.read() {}
}
