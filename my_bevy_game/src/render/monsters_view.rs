//! Roaming-monster visuals.
//!
//! Each `Monster` becomes a billboarded sprite quad. Three "kinds"
//! get three different procedurally-generated 16×16 textures:
//!   - 0 = red wolfish (pointy ears, beady eyes)
//!   - 1 = green frogish (round, big eyes)
//!   - 2 = blue ghostish (wavy bottom, large pupil)
//!
//! Textures are baked once at startup into `MonsterAssets`. Each
//! monster spawn-attach picks the matching mesh+material handle.

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::data::{GameCamera, Monster, Player};

const SPRITE_SIZE: u32 = 16;

#[derive(Resource)]
pub(crate) struct MonsterAssets {
    /// Same mesh for every monster (a flat 0.8 × 1.2 quad facing +Z).
    /// Billboard system rotates each monster's `Transform` per frame.
    mesh: Handle<Mesh>,
    /// One material per kind, each backed by a unique procedural texture.
    materials: [Handle<StandardMaterial>; 3],
}

pub(crate) fn register_monster_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    // Flat quad. We use a Cuboid with a tiny Z extent so it has solid
    // depth-write behaviour — alpha-mask is enough for the cutout.
    let mesh = meshes.add(Cuboid::new(0.8, 1.2, 0.05));

    let mat_handles: [Handle<StandardMaterial>; 3] = std::array::from_fn(|kind| {
        let img = generate_monster_sprite(kind as u8);
        let img_handle = images.add(img);
        materials.add(StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: Some(img_handle),
            unlit: true,
            // Mask cutout: pixels with alpha > 0.5 are opaque, the
            // rest are completely discarded. No Z-sort issues.
            alpha_mode: AlphaMode::Mask(0.5),
            cull_mode: None, // visible from both sides
            ..default()
        })
    });

    commands.insert_resource(MonsterAssets {
        mesh,
        materials: mat_handles,
    });
}

/// Attach a `Mesh3d`/`MeshMaterial3d` to every newly-spawned monster.
pub(crate) fn attach_monster_visuals(
    mut commands: Commands,
    assets: Option<Res<MonsterAssets>>,
    new_monsters: Query<(Entity, &Monster), Added<Monster>>,
) {
    let Some(assets) = assets else { return };
    for (entity, monster) in &new_monsters {
        let kind = (monster.kind as usize).min(assets.materials.len() - 1);
        commands.entity(entity).insert((
            Mesh3d(assets.mesh.clone()),
            MeshMaterial3d(assets.materials[kind].clone()),
        ));
    }
}

/// Yaw-only rotate every monster to face the camera (same trick as
/// the player slab).
pub(crate) fn billboard_monsters(
    cam_q: Query<&Transform, (With<GameCamera>, Without<Monster>, Without<Player>)>,
    mut monster_q: Query<&mut Transform, With<Monster>>,
) {
    let Ok(cam_tf) = cam_q.single() else { return };
    for mut tf in &mut monster_q {
        let to_cam = cam_tf.translation - tf.translation;
        let yaw = to_cam.x.atan2(to_cam.z);
        tf.rotation = Quat::from_rotation_y(yaw);
    }
}

// === Procedural sprite generation ==========================================

fn generate_monster_sprite(kind: u8) -> Image {
    let s = SPRITE_SIZE as usize;
    let mut data = vec![0u8; s * s * 4];

    // Body colour, eye colour, secondary highlight differ per kind.
    let (body, eye) = match kind {
        0 => ([200u8, 60, 50, 255], [255u8, 255, 255, 255]),  // red wolf
        1 => ([60u8, 180, 80, 255], [10u8, 10, 10, 255]),     // green frog
        _ => ([90u8, 140, 220, 255], [255u8, 240, 200, 255]), // blue ghost
    };

    let put = |data: &mut [u8], x: usize, y: usize, c: [u8; 4]| {
        let idx = (y * s + x) * 4;
        data[idx..idx + 4].copy_from_slice(&c);
    };

    // Body shape: roughly elliptical body; head smaller on top.
    for y in 0..s {
        for x in 0..s {
            let cx = (x as f32 - 7.5) / 5.0;       // body half-width 5
            let cy_body = (y as f32 - 11.5) / 4.0; // body lower half
            let cy_head = (y as f32 - 4.5) / 3.0;  // head upper half

            let in_body = cx * cx + cy_body * cy_body <= 1.0 && y >= 7;
            let in_head = cx * cx + cy_head * cy_head <= 1.0 && y < 8;

            if in_body || in_head {
                put(&mut data, x, y, body);
            }
        }
    }

    // Per-kind decorations.
    match kind {
        0 => {
            // Pointy ears (triangles at top)
            for dy in 0..3 {
                let span = 3 - dy;
                for dx in 0..span {
                    put(&mut data, 3 + dx, dy, body);
                    put(&mut data, 12 - dx, dy, body);
                }
            }
            // Eyes
            put(&mut data, 5, 4, eye);
            put(&mut data, 10, 4, eye);
        }
        1 => {
            // Big bulging eyes (2x2)
            for dy in 0..2 {
                for dx in 0..2 {
                    put(&mut data, 4 + dx, 3 + dy, eye);
                    put(&mut data, 10 + dx, 3 + dy, eye);
                }
            }
        }
        _ => {
            // Wavy bottom (transparent notches)
            for x in (0..s).step_by(4) {
                put(&mut data, x, 14, [0, 0, 0, 0]);
                put(&mut data, x, 15, [0, 0, 0, 0]);
                put(&mut data, x + 1, 15, [0, 0, 0, 0]);
            }
            // Big single eye
            for dy in 0..3 {
                for dx in 0..3 {
                    put(&mut data, 6 + dx, 4 + dy, eye);
                }
            }
            put(&mut data, 7, 5, [10, 10, 10, 255]); // pupil
        }
    }

    let mut img = Image::new(
        Extent3d {
            width: SPRITE_SIZE,
            height: SPRITE_SIZE,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    // Keep crisp pixels — no linear filtering blurring our 16×16 art.
    img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor::nearest());
    img
}
