//! Terrain rendering: attaches `Mesh3d`/`MeshMaterial3d` to entities the
//! server spawned with `TerrainBlock + Transform`.

use bevy::prelude::*;

use crate::data::{Biome, TerrainBlock};

#[derive(Resource)]
pub(crate) struct TerrainAssets {
    cube: Handle<Mesh>,
    water: Handle<StandardMaterial>,
    sand: Handle<StandardMaterial>,
    grass: Handle<StandardMaterial>,
    rock: Handle<StandardMaterial>,
    snow: Handle<StandardMaterial>,
}

pub fn register_terrain_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(TerrainAssets {
        cube: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        water: materials.add(Color::srgb(0.20, 0.40, 0.85)),
        sand: materials.add(Color::srgb(0.85, 0.78, 0.45)),
        grass: materials.add(Color::srgb(0.30, 0.65, 0.25)),
        rock: materials.add(Color::srgb(0.45, 0.40, 0.35)),
        snow: materials.add(Color::srgb(0.95, 0.95, 0.97)),
    });
}

/// Adds visual components to terrain entities the server creates. Uses
/// `Added<TerrainBlock>` so it fires once per entity, however it was
/// spawned (Startup or later).
pub fn attach_terrain_visuals(
    mut commands: Commands,
    assets: Option<Res<TerrainAssets>>,
    new_blocks: Query<(Entity, &TerrainBlock), Added<TerrainBlock>>,
) {
    let Some(assets) = assets else {
        return;
    };
    for (entity, block) in &new_blocks {
        let mat = match block.biome {
            Biome::Water => assets.water.clone(),
            Biome::Sand => assets.sand.clone(),
            Biome::Grass => assets.grass.clone(),
            Biome::Rock => assets.rock.clone(),
            Biome::Snow => assets.snow.clone(),
        };
        commands
            .entity(entity)
            .insert((Mesh3d(assets.cube.clone()), MeshMaterial3d(mat)));
    }
}
