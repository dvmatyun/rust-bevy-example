//! Stair visuals: each step is a full block from `low_y` up to its
//! own surface height. Stacked along the stair axis they form the
//! classic chunky low-poly staircase silhouette:
//!
//! ```text
//!                        |‾‾‾‾| step N-1 (top, surface = high_y)
//!                   |‾‾‾‾|    |
//!              |‾‾‾‾|    |    |
//!         |‾‾‾‾|    |    |    |
//!    |‾‾‾‾|    |    |    |    |
//! |‾‾‾‾|    |    |    |    |    |
//! |____|____|____|____|____|____|
//!  step 0 …                       (bottom, surface = low_y + step_h)
//! ```
//!
//! `register_stair_assets` runs after `register_building_assets`
//! (chained in `RenderPlugin`) so we can reuse the plank texture
//! generated there — same wood look across the building and its
//! entrance stairs.

use bevy::prelude::*;

use crate::data::Stairs;
use crate::render::buildings::BuildingAssets;

#[derive(Resource)]
pub(crate) struct StairAssets {
    /// Reused unit cuboid scaled per-step.
    unit_cube: Handle<Mesh>,
    /// Same brown as the building floors so the structure reads as
    /// one piece. Shared across all steps — they don't fade.
    material: Handle<StandardMaterial>,
}

pub(crate) fn register_stair_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    building_assets: Res<BuildingAssets>,
) {
    commands.insert_resource(StairAssets {
        unit_cube: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.55, 0.40, 0.25), // matches building FLOOR_COLOR
            base_color_texture: Some(building_assets.plank_texture.clone()),
            perceptual_roughness: 0.85,
            ..default()
        }),
    });
}

/// On every newly-spawned `Stairs`, attach one child block per step.
pub(crate) fn attach_stair_visuals(
    mut commands: Commands,
    assets: Option<Res<StairAssets>>,
    new_stairs: Query<(Entity, &Stairs), Added<Stairs>>,
) {
    let Some(assets) = assets else { return };
    let unit_cube = assets.unit_cube.clone();
    let material = assets.material.clone();

    for (entity, stairs) in &new_stairs {
        if stairs.steps == 0 {
            continue;
        }
        let center_x = (stairs.footprint.min.x + stairs.footprint.max.x) * 0.5;
        let size_x = stairs.footprint.max.x - stairs.footprint.min.x;
        let z_range = stairs.footprint.max.y - stairs.footprint.min.y;
        let step_size_z = z_range / stairs.steps as f32;
        let step_height = (stairs.high_y - stairs.low_y) / stairs.steps as f32;

        commands.entity(entity).with_children(|parent| {
            // Step k = 0 is the LOWEST step; it sits at the
            // footprint's max Z (south end). Step k = N-1 is the
            // highest, at the footprint's min Z (north, abutting the
            // building).
            for k in 0..stairs.steps {
                // Distance from the south edge: k=0 is the southmost,
                // k=N-1 the northmost.
                let z_center = stairs.footprint.max.y - (k as f32 + 0.5) * step_size_z;

                // Block height = (k+1) * step_height (rises from
                // low_y up to surface).
                let height = (k as f32 + 1.0) * step_height;
                let y_center = stairs.low_y + height * 0.5;

                parent.spawn((
                    Mesh3d(unit_cube.clone()),
                    MeshMaterial3d(material.clone()),
                    Transform::from_xyz(center_x, y_center, z_center)
                        .with_scale(Vec3::new(size_x, height, step_size_z)),
                ));
            }
        });
    }
}
