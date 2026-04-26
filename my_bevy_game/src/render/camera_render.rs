//! Owns the `Camera3d` entity. Snaps the actual `Transform` to whatever
//! `client_sim::camera_view::compute_camera_view` wrote into
//! `DesiredCameraView`.
//!
//! All inertia / smoothing happens in `client_sim` (orbit angle and
//! focal point are smoothed there, so the camera always stays on its
//! orbit arc around the player rather than lerping in a straight line
//! through 3D space). Doing the lerp here would cause "cuts across the
//! arc" when orbiting fast.

use bevy::prelude::*;

use crate::data::{DesiredCameraView, GameCamera};

pub fn spawn_camera(mut commands: Commands, view: Res<DesiredCameraView>) {
    commands.spawn((
        GameCamera,
        Camera3d::default(),
        Transform::from_translation(view.position).looking_at(view.look_at, Vec3::Y),
        #[cfg(target_os = "android")]
        Msaa::Off,
    ));
}

pub fn update_camera_transform(
    view: Res<DesiredCameraView>,
    mut q: Query<&mut Transform, With<GameCamera>>,
) {
    let Ok(mut tf) = q.single_mut() else {
        return;
    };
    tf.translation = view.position;
    let look = tf.looking_at(view.look_at, Vec3::Y);
    tf.rotation = look.rotation;
}
