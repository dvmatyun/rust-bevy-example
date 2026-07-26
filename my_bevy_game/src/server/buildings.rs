//! Sample buildings + stairs for the multi-storey demo.
//!
//! Replicated entities the player can stand on at multiple Y values
//! for the same XZ. Real games would load these from a RON map file
//! (see `docsai/world-and-graphics-plan.md`).
//!
//! Important: ground floor and stair `low_y` are computed dynamically
//! to clear the terrain peak — see SKILL rule #9 (avoiding
//! Z-fighting).

use bevy::prelude::*;

use crate::data::{terrain_gen, Building, BuildingSide, Stairs, WorldConfig};

/// World-space safety margin between terrain peak and building
/// ground floor. Larger = more visual space, but the building floats
/// above terrain by more.
const TERRAIN_CLEARANCE: f32 = 0.5;

/// Vertical separation between consecutive floors.
const FLOOR_SPACING: f32 = 3.0;

/// How many discrete steps in the entrance stair flight.
const STAIR_STEPS: u32 = 6;

/// How far south of the building the stair flight extends (Z units).
const STAIR_LENGTH: f32 = 3.0;

pub fn spawn_demo_buildings(mut commands: Commands, config: Res<WorldConfig>) {
    let footprint = Rect::new(3.0, 3.0, 8.0, 8.0);

    // Sample terrain at several points inside the footprint to find
    // the peak. The ground floor must clear it or its slab will Z-fight
    // with the terrain mesh — visible as flickering colours.
    let mut peak: f32 = 0.0;
    for &x in &[3.0_f32, 4.5, 5.5, 6.5, 8.0] {
        for &z in &[3.0_f32, 4.5, 5.5, 6.5, 8.0] {
            peak = peak.max(terrain_gen::terrain_top_y(x, z, &config));
        }
    }

    let ground = peak + TERRAIN_CLEARANCE;
    let floor_heights = vec![ground, ground + FLOOR_SPACING, ground + FLOOR_SPACING * 2.0];

    commands.spawn((
        Building {
            footprint,
            floor_heights,
            // Entrance opens onto the stairs (which sit at +Z of the
            // building footprint — the South side).
            entrance: BuildingSide::South,
        },
        Transform::default(),  // required by hierarchy (children are floor visuals)
        Visibility::default(), // children inherit visibility
    ));

    // Entrance stairs from terrain up to the building's ground floor.
    // Footprint sits just south of the building (z = 8 → 11).
    // Sample terrain at the far south end to anchor the bottom step
    // close to the actual ground; the stair render then bridges that
    // height up to the building's ground floor in equal steps.
    let stair_footprint = Rect::new(3.0, 8.0, 8.0, 8.0 + STAIR_LENGTH);
    let stair_low = terrain_gen::terrain_top_y(5.5, 8.0 + STAIR_LENGTH - 0.5, &config);

    commands.spawn((
        Stairs {
            footprint: stair_footprint,
            low_y: stair_low,
            high_y: ground, // top of stairs lines up exactly with the ground floor
            steps: STAIR_STEPS,
        },
        Transform::default(),
        Visibility::default(),
    ));
}
