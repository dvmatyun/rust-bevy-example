//! Chunked terrain rendering.
//!
//! The world is divided into `CHUNK_SIZE`×`CHUNK_SIZE` cell chunks
//! (default 32×32). Each chunk is one flat-shaded mesh entity. Bevy
//! frustum-culls them automatically based on each entity's
//! `Aabb`/`GlobalTransform`, so off-screen chunks don't draw.
//!
//! Per frame `manage_terrain_chunks` looks at the camera position and:
//!   1. computes the desired set of chunks within
//!      `Settings.render_distance_chunks` of the camera, clamped to
//!      world bounds, and
//!   2. spawns missing chunks + despawns chunks that are no longer
//!      wanted.
//!
//! This means scaling the world to 100× or 10000× does **not**
//! generate 10⁴ × 10⁴ meshes — only the visible neighbourhood of
//! ~`(2*render_distance + 1)²` chunks ever exists at once.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::mesh::PrimitiveTopology;
use std::collections::HashSet;

use crate::data::{terrain_gen, Biome, GameCamera, Settings, TerrainChunk, WorldConfig, CHUNK_SIZE};

#[derive(Resource)]
pub(crate) struct TerrainAssets {
    /// One material reused by every chunk; vertex colours do the
    /// per-biome tinting.
    material: Handle<StandardMaterial>,
}

pub fn register_terrain_assets(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(TerrainAssets {
        material: materials.add(StandardMaterial {
            base_color: Color::WHITE,
            perceptual_roughness: 0.95,
            metallic: 0.0,
            ..default()
        }),
    });
}

pub fn manage_terrain_chunks(
    mut commands: Commands,
    config: Res<WorldConfig>,
    settings: Res<Settings>,
    assets: Option<Res<TerrainAssets>>,
    mut meshes: ResMut<Assets<Mesh>>,
    cam_q: Query<&Transform, With<GameCamera>>,
    existing: Query<(Entity, &TerrainChunk)>,
) {
    let Some(assets) = assets else { return };
    let Some(cam_tf) = cam_q.iter().next() else { return };

    let cam_chunk = IVec2::new(
        (cam_tf.translation.x / CHUNK_SIZE as f32).floor() as i32,
        (cam_tf.translation.z / CHUNK_SIZE as f32).floor() as i32,
    );
    let r = settings.render_distance_chunks.max(1);

    // Build the wanted set, clamped to world bounds.
    let mut wanted: HashSet<IVec2> = HashSet::new();
    let radius_cells = config.world_radius_cells;
    for dx in -r..=r {
        for dz in -r..=r {
            let coord = cam_chunk + IVec2::new(dx, dz);
            // Skip chunks completely outside the world rectangle.
            let chunk_min_x = coord.x * CHUNK_SIZE;
            let chunk_min_z = coord.y * CHUNK_SIZE;
            let chunk_max_x = chunk_min_x + CHUNK_SIZE;
            let chunk_max_z = chunk_min_z + CHUNK_SIZE;
            if chunk_max_x <= -radius_cells
                || chunk_min_x >= radius_cells
                || chunk_max_z <= -radius_cells
                || chunk_min_z >= radius_cells
            {
                continue;
            }
            wanted.insert(coord);
        }
    }

    // Despawn chunks no longer in `wanted`; track which ones already exist.
    let mut existing_coords: HashSet<IVec2> = HashSet::new();
    for (entity, chunk) in &existing {
        if wanted.contains(&chunk.coord) {
            existing_coords.insert(chunk.coord);
        } else {
            commands.entity(entity).despawn();
        }
    }

    // Spawn missing.
    for coord in wanted {
        if existing_coords.contains(&coord) {
            continue;
        }
        let mesh = build_chunk_mesh(coord, &config);
        let mesh_handle = meshes.add(mesh);
        commands.spawn((
            TerrainChunk { coord },
            Mesh3d(mesh_handle),
            MeshMaterial3d(assets.material.clone()),
            Transform::default(),
        ));
    }
}

/// Build a single chunk's mesh covering the cell range
/// `[coord*CHUNK_SIZE .. (coord+1)*CHUNK_SIZE)` along both axes.
fn build_chunk_mesh(coord: IVec2, config: &WorldConfig) -> Mesh {
    let radius = config.world_radius_cells;
    let cell_min_x = coord.x * CHUNK_SIZE;
    let cell_min_z = coord.y * CHUNK_SIZE;
    let cell_max_x = (cell_min_x + CHUNK_SIZE).min(radius);
    let cell_max_z = (cell_min_z + CHUNK_SIZE).min(radius);
    let cell_min_x = cell_min_x.max(-radius);
    let cell_min_z = cell_min_z.max(-radius);

    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut colors: Vec<[f32; 4]> = Vec::new();

    for z in cell_min_z..cell_max_z {
        for x in cell_min_x..cell_max_x {
            let h00 = terrain_gen::terrain_surface_y_cell(x, z, config);
            let h10 = terrain_gen::terrain_surface_y_cell(x + 1, z, config);
            let h01 = terrain_gen::terrain_surface_y_cell(x, z + 1, config);
            let h11 = terrain_gen::terrain_surface_y_cell(x + 1, z + 1, config);

            let p00 = Vec3::new(x as f32, h00, z as f32);
            let p10 = Vec3::new(x as f32 + 1.0, h10, z as f32);
            let p01 = Vec3::new(x as f32, h01, z as f32 + 1.0);
            let p11 = Vec3::new(x as f32 + 1.0, h11, z as f32 + 1.0);

            let avg_h = (h00 + h01 + h10 + h11) * 0.25;
            let cell_color = biome_color(avg_h, x, z);

            push_triangle(&mut positions, &mut normals, &mut colors, p00, p01, p10, cell_color);
            push_triangle(&mut positions, &mut normals, &mut colors, p10, p01, p11, cell_color);
        }
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh
}

fn push_triangle(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    a: Vec3, b: Vec3, c: Vec3,
    color: [f32; 4],
) {
    let n = (b - a).cross(c - a).normalize_or_zero();
    let n_arr = n.to_array();
    positions.push(a.to_array());
    positions.push(b.to_array());
    positions.push(c.to_array());
    normals.extend_from_slice(&[n_arr, n_arr, n_arr]);
    colors.extend_from_slice(&[color, color, color]);
}

fn cell_jitter(x: i32, z: i32) -> f32 {
    let mut h = (x as u32).wrapping_mul(73856093);
    h ^= (z as u32).wrapping_mul(19349663);
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    h ^= h >> 16;
    (h as f32) / (u32::MAX as f32)
}

fn biome_color(h: f32, x: i32, z: i32) -> [f32; 4] {
    let base = match Biome::from_height(h.round() as i32) {
        Biome::Water => Vec3::new(0.20, 0.40, 0.85),
        Biome::Sand => Vec3::new(0.85, 0.78, 0.45),
        Biome::Grass => Vec3::new(0.30, 0.65, 0.25),
        Biome::Rock => Vec3::new(0.45, 0.40, 0.35),
        Biome::Snow => Vec3::new(0.95, 0.95, 0.97),
    };
    let amt = (cell_jitter(x, z) - 0.5) * 0.20;
    let color = (base + Vec3::splat(amt)).max(Vec3::ZERO).min(Vec3::ONE);
    [color.x, color.y, color.z, 1.0]
}
