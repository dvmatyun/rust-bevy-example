//! Logical components — the shared vocabulary used by all layers.
//!
//! NO render components live here. Render-specific markers (e.g. mesh
//! attachments) belong in `render/`.
//!
//! 📘 Reading guide: every type below is a *plain Rust struct* with
//! one extra line of `#[derive(...)]` to opt into Bevy's ECS. The
//! derives generate trait implementations at compile time:
//! `Component` makes the type attachable to entities; `Serialize` /
//! `Deserialize` come from `serde` and let it cross the network.

use bevy::prelude::*;
// 📘 `serde` is the Rust serialization framework. The two derives
// below generate the conversion to / from a byte stream that the
// replication system needs.
use serde::{Deserialize, Serialize};

/// Marker for the player entity. The entity also carries `Transform`
/// (game position) and `Facing` but no mesh — the render layer attaches
/// the visual.
///
/// 📘 *Marker components* are zero-sized structs (`pub struct Player;`).
/// They store no data — they exist only as a *tag* you can filter on
/// in queries: `Query<&Transform, With<Player>>`. Cheap to spawn and
/// query because they take 0 bytes per entity.
#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug)]
pub struct Player;

/// Direction the player is "looking" on the XZ plane (unit vector).
/// Updated by the server when the player moves; used by the render
/// layer to pick face/side/back assets.
///
/// 📘 *Tuple struct*: `Facing(pub Vec2)` is shorthand for `Facing { 0:
/// Vec2 }`. Access the inner value with `.0`: `let v = facing.0;`. The
/// `pub` makes the inner field accessible from outside the module.
#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug)]
pub struct Facing(pub Vec2);

// 📘 Manual `Default` impl because we want a non-zero default
// (most types default to "all-zero" but a zero facing vector is
// unhelpful). Rust requires you to opt in to default behaviour by
// either deriving `Default` or implementing it by hand.
impl Default for Facing {
    fn default() -> Self {
        // Face -Z so the camera (at +Z by default) sees the player's back.
        Self(Vec2::new(0.0, -1.0))
    }
}

/// Marker for the main camera entity. The render layer adds `Camera3d`
/// to it; gameplay code only sees the marker + `Transform`.
#[derive(Component)]
pub struct GameCamera;

/// Marker for a terrain block entity, with the biome chosen by the
/// server. The render layer maps `Biome` → material.
#[derive(Component, Clone, Copy, Debug)]
pub struct TerrainBlock {
    pub biome: Biome,
}

/// Surface-material classification, derived from elevation.
///
/// 📘 An *enum* is a sum type — a value is exactly one of the listed
/// variants. Pattern-match it with `match biome { Biome::Water => …,
/// Biome::Sand => … }`. The derives give equality, copying and
/// debug-printing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Biome {
    Water,
    Sand,
    Grass,
    Rock,
    Snow,
}

impl Biome {
    pub fn from_height(h: i32) -> Self {
        if h <= 0 {
            Self::Water
        } else if h <= 1 {
            Self::Sand
        } else if h <= 4 {
            Self::Grass
        } else if h <= 6 {
            Self::Rock
        } else {
            Self::Snow
        }
    }
}
