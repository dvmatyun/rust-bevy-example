//! Building visuals: floor slabs + walls.
//!
//! Per `Building` entity, we attach as children:
//!   - one floor slab per `floor_heights` entry (player walks on it)
//!   - wall segments returned by `Building::wall_segments()`, which
//!     handles entrance gaps and the wall-to-wall corner inset that
//!     prevents Z-fighting at building corners (SKILL #9).
//!
//! Each wall gets its OWN `StandardMaterial` handle so the
//! occlusion-fade system can change one wall's alpha without
//! affecting any other. Floor slabs share a single material —
//! they're below the player and never occlude.
//!
//! All surfaces use a procedural plank/grain texture so they read as
//! 3D volumes instead of flat painted blocks (Valheim-style). The
//! texture is sampled with nearest filtering for the chunky-pixel
//! look that matches our 16×16 monster sprites.
//!
//! Roofs are intentionally absent (the top floor is an open
//! terrace). See `docsai/world-and-graphics-plan.md`.

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::data::{Building, FLOOR_THICKNESS};
use crate::render::occlusion::Occludable;

const FLOOR_COLOR: Color = Color::srgb(0.55, 0.40, 0.25);
const WALL_COLOR: Color = Color::srgb(0.78, 0.62, 0.42);

#[derive(Resource)]
pub(crate) struct BuildingAssets {
    /// Unit cuboid (1×1×1); we scale per-instance via `Transform.scale`.
    unit_cube: Handle<Mesh>,
    /// Shared floor material (floors are never occluders, so per-
    /// instance materials would just waste memory).
    floor_material: Handle<StandardMaterial>,
    /// Shared procedural wood/plank texture used by floors, walls,
    /// and stairs. Each material tints it with its own `base_color`.
    pub(crate) plank_texture: Handle<Image>,
}

pub(crate) fn register_building_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let plank_texture = images.add(generate_plank_texture());
    commands.insert_resource(BuildingAssets {
        unit_cube: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        floor_material: materials.add(StandardMaterial {
            base_color: FLOOR_COLOR,
            base_color_texture: Some(plank_texture.clone()),
            perceptual_roughness: 0.85,
            ..default()
        }),
        plank_texture,
    });
}

pub(crate) fn attach_building_visuals(
    mut commands: Commands,
    assets: Option<Res<BuildingAssets>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    new_buildings: Query<(Entity, &Building), Added<Building>>,
) {
    let Some(assets) = assets else { return };
    let unit_cube = assets.unit_cube.clone();
    let floor_material = assets.floor_material.clone();
    let plank_texture = assets.plank_texture.clone();

    for (entity, building) in &new_buildings {
        let center = building.footprint.center();
        let size = building.footprint.size();

        commands.entity(entity).with_children(|parent| {
            // ── Floor slabs ────────────────────────────────────────
            for &floor_y in &building.floor_heights {
                parent.spawn((
                    Mesh3d(unit_cube.clone()),
                    MeshMaterial3d(floor_material.clone()),
                    Transform::from_xyz(center.x, floor_y - FLOOR_THICKNESS / 2.0, center.y)
                        .with_scale(Vec3::new(size.x, FLOOR_THICKNESS, size.y)),
                ));
            }

            // ── Walls (one mesh per WallSegment from data layer) ──
            for wall in building.wall_segments() {
                // Per-wall material so occlusion alpha is per-wall.
                let wall_mat = materials.add(StandardMaterial {
                    base_color: WALL_COLOR,
                    base_color_texture: Some(plank_texture.clone()),
                    perceptual_roughness: 0.85,
                    // Blend mode is required for smooth alpha
                    // transitions; cull none so the wall is
                    // visible from inside too.
                    alpha_mode: AlphaMode::Blend,
                    cull_mode: None,
                    ..default()
                });
                parent.spawn((
                    Mesh3d(unit_cube.clone()),
                    MeshMaterial3d(wall_mat),
                    Transform::from_xyz(wall.center.x, wall.center.y, wall.center.z)
                        .with_scale(wall.size),
                    Occludable::default(),
                ));
            }
        });
    }
}

// === Procedural plank/grain texture ========================================

/// 64×64 grayscale-ish wood-grain texture: a few horizontal planks,
/// each filled with high-frequency longitudinal "grain" noise. Tinted
/// at material time, so the same atlas serves walls and floors.
fn generate_plank_texture() -> Image {
    const SIZE: u32 = 64;
    const PLANKS: u32 = 4; // horizontal planks across the texture
    let s = SIZE as usize;
    let mut data = vec![0u8; s * s * 4];

    let plank_height = SIZE / PLANKS;

    for y in 0..SIZE {
        // Per-plank base brightness so consecutive planks look
        // distinct (real wood has natural colour variation).
        let plank_idx = y / plank_height;
        let plank_brightness = 0.85 + 0.15 * hash_f(plank_idx, 0xA1);

        // Dark seam between planks.
        let in_seam = (y % plank_height) == 0;

        for x in 0..SIZE {
            // Longitudinal grain — high-freq noise stretched along X
            // so it looks like wood fibres running plank-length.
            let grain = noise(x as f32 * 0.35, y as f32 * 1.4 + plank_idx as f32 * 7.3);
            // Fine speckle for surface texture.
            let speckle = noise(x as f32 * 1.2, y as f32 * 1.2 + 13.0);
            let mut v = plank_brightness * (0.78 + 0.18 * grain + 0.06 * speckle);
            if in_seam {
                v *= 0.45;
            }
            let v = v.clamp(0.0, 1.0);
            let c = (v * 255.0) as u8;
            let idx = (y as usize * s + x as usize) * 4;
            data[idx] = c;
            data[idx + 1] = c;
            data[idx + 2] = c;
            data[idx + 3] = 255;
        }
    }

    let mut img = Image::new(
        Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    // Nearest sampling keeps the chunky-pixel look (Valheim/Minecraft
    // territory) instead of blurring the grain.
    img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor::nearest());
    img
}

fn hash_f(x: u32, salt: u32) -> f32 {
    let mut h = x.wrapping_mul(2654435761).wrapping_add(salt.wrapping_mul(40503));
    h ^= h >> 13;
    h = h.wrapping_mul(0x5bd1e995);
    h ^= h >> 15;
    (h as f32) / (u32::MAX as f32)
}

fn noise(x: f32, y: f32) -> f32 {
    let xi = x.floor() as i32;
    let yi = y.floor() as i32;
    let tx = x - xi as f32;
    let ty = y - yi as f32;
    let u = tx * tx * (3.0 - 2.0 * tx);
    let v = ty * ty * (3.0 - 2.0 * ty);
    let h00 = hash_f((xi as u32).wrapping_mul(73856093) ^ (yi as u32).wrapping_mul(19349663), 1);
    let h10 = hash_f(((xi + 1) as u32).wrapping_mul(73856093) ^ (yi as u32).wrapping_mul(19349663), 1);
    let h01 = hash_f((xi as u32).wrapping_mul(73856093) ^ ((yi + 1) as u32).wrapping_mul(19349663), 1);
    let h11 = hash_f(((xi + 1) as u32).wrapping_mul(73856093) ^ ((yi + 1) as u32).wrapping_mul(19349663), 1);
    let a = h00 * (1.0 - u) + h10 * u;
    let b = h01 * (1.0 - u) + h11 * u;
    a * (1.0 - v) + b * v
}
