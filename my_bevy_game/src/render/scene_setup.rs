//! Static scene setup: lights, sky color, ambient.

use bevy::prelude::*;

pub fn setup_scene(mut commands: Commands) {
    commands.insert_resource(ClearColor(Color::srgb(0.55, 0.75, 0.92)));

    commands.insert_resource(GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 250.0,
        ..default()
    });

    commands.spawn((
        DirectionalLight {
            illuminance: 6000.0,
            #[cfg(not(target_os = "android"))]
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(40.0, 80.0, 40.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
