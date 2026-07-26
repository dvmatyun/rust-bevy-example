//! Owns the `Camera3d` entity (or entities in split-screen).
//! Snaps each camera's `Transform` to its `DesiredCameraView` component,
//! which `client_sim::camera_view::compute_camera_view` writes every frame.
//!
//! All inertia / smoothing happens in `client_sim` (orbit angle and focal
//! point are smoothed there). Doing the lerp here would cause "cuts across
//! the arc" when orbiting fast.

use bevy::prelude::*;
use bevy::camera::Viewport;

use crate::client_sim::{CameraFocus, CameraOrbit};
use crate::data::{DesiredCameraView, GameCamera, GameEntity, PlayerSlot};

/// Spawn one camera for single-player.
pub fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        GameCamera,
        Camera3d::default(),
        CameraOrbit::default(),
        CameraFocus::default(),
        DesiredCameraView::default(),
        PlayerSlot(0),
        GameEntity,
        Transform::default(),
        #[cfg(target_os = "android")]
        Msaa::Off,
    ));
}

/// Spawn two side-by-side cameras for the split-screen multiplayer sim.
pub fn spawn_camera_multiplayer(mut commands: Commands, windows: Query<&Window>) {
    let (w, h) = windows
        .single()
        .map(|win| (win.physical_width(), win.physical_height()))
        .unwrap_or((1280, 720));

    let half_w = w / 2;
    for slot in 0u8..2 {
        let x_offset = u32::from(slot) * half_w;
        commands.spawn((
            GameCamera,
            Camera3d::default(),
            Camera {
                viewport: Some(Viewport {
                    physical_position: UVec2::new(x_offset, 0),
                    physical_size: UVec2::new(half_w, h),
                    ..default()
                }),
                order: slot as isize,
                ..default()
            },
            CameraOrbit::default(),
            CameraFocus::default(),
            DesiredCameraView::default(),
            PlayerSlot(slot),
            GameEntity,
            Transform::default(),
            #[cfg(target_os = "android")]
            Msaa::Off,
        ));
    }
}

/// Snap each camera's `Transform` to its `DesiredCameraView` component.
pub fn update_camera_transform(
    mut q: Query<(&DesiredCameraView, &mut Transform), With<GameCamera>>,
) {
    for (view, mut tf) in &mut q {
        tf.translation = view.position;
        tf.rotation = Transform::from_translation(view.position)
            .looking_at(view.look_at, Vec3::Y)
            .rotation;
    }
}
