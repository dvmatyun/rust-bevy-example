//! Static scene setup: sky color, ambient, the directional sun.
//! The sun's shadow + direction are controllable at runtime via
//! `Settings`; see `apply_sun_settings` below.

use bevy::prelude::*;

use crate::data::Settings;

/// Marker for the sun's directional light entity so we can find it
/// each frame in `apply_sun_settings`.
#[derive(Component)]
pub(crate) struct SunLight;

pub fn setup_scene(mut commands: Commands) {
    commands.insert_resource(ClearColor(Color::srgb(0.55, 0.75, 0.92)));

    commands.insert_resource(GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 200.0,
        ..default()
    });

    commands.spawn((
        SunLight,
        DirectionalLight {
            illuminance: 8000.0,
            // Shadow toggle is applied each frame from Settings; the
            // initial value here is just whatever — `apply_sun_settings`
            // overwrites it on Update.
            #[cfg(not(target_os = "android"))]
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(40.0, 80.0, 40.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// Each frame: re-aim the sun based on `Settings.sun_azimuth_deg` /
/// `sun_elevation_deg`, and toggle shadow maps from
/// `Settings.shadows_enabled`.
pub(crate) fn apply_sun_settings(
    settings: Res<Settings>,
    mut sun_q: Query<(&mut Transform, &mut DirectionalLight), With<SunLight>>,
) {
    let Ok((mut tf, mut light)) = sun_q.single_mut() else {
        return;
    };

    // Convert azimuth/elevation (degrees) to a unit direction vector.
    // Azimuth is measured clockwise from +X (looking down +Y).
    let az = settings.sun_azimuth_deg.to_radians();
    let el = settings.sun_elevation_deg.to_radians();
    let cos_e = el.cos();
    let dir = Vec3::new(az.cos() * cos_e, el.sin().max(0.05), az.sin() * cos_e);

    // Position the sun "high" along its direction; it's a
    // *directional* light so absolute position only matters for the
    // shadow camera. 80 units up is enough for our scale.
    tf.translation = dir * 80.0;
    *tf = tf.looking_at(Vec3::ZERO, Vec3::Y);

    light.shadow_maps_enabled = settings.shadows_enabled;
}
