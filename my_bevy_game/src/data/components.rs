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

/// Marker for a terrain *chunk* mesh entity. The world is divided
/// into `CHUNK_SIZE`² cell chunks; render keeps the ones within
/// `Settings.render_distance_chunks` of the camera loaded.
///
/// `coord` is the chunk's grid index (0,0 covers cells x∈[0..32),
/// z∈[0..32)). World position is `coord * CHUNK_SIZE` cells.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TerrainChunk {
    pub coord: IVec2,
}

/// Roaming AI agent. Has `Transform`, `Wander`, and a kind index for
/// visual variation. Rendered as a billboarded sprite.
#[derive(Component, Clone, Copy, Debug)]
pub struct Monster {
    pub kind: u8,
}

/// Simple wander state: move in `direction` at `speed` for
/// `time_remaining` seconds, then pick a new direction.
#[derive(Component, Clone, Copy, Debug)]
pub struct Wander {
    pub direction: Vec2, // XZ unit vector
    pub time_remaining: f32,
    pub speed: f32,
}

impl Default for Wander {
    fn default() -> Self {
        Self {
            direction: Vec2::new(1.0, 0.0),
            time_remaining: 0.0, // expires immediately → picks new direction on first tick
            speed: 2.0,
        }
    }
}

/// A multi-storey building. The world is a heightmap (one Y per XZ),
/// so anything that lets the player walk at *different* Y values for
/// the same XZ has to live on a separate entity that
/// `snap_to_ground` consults.
///
/// `footprint` — XZ bounds in world units (`Vec2.x = world X`,
/// `Vec2.y = world Z`).
/// `floor_heights` — Y-coordinates of walkable floor surfaces,
/// ordered low → high. Each value is "the Y the player should stand
/// at" when they're on that floor.
/// `entrance` — which side of the footprint has a door cut in the
/// ground-story wall (the entrance gap is centred on that side).
#[derive(Component, Clone, Debug)]
pub struct Building {
    pub footprint: Rect,
    pub floor_heights: Vec<f32>,
    pub entrance: BuildingSide,
}

/// Side of a `Building` footprint, named by the world-axis edge it
/// sits on. North = -Z edge, South = +Z, East = +X, West = -X.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BuildingSide {
    North,
    #[default]
    South,
    East,
    West,
}

/// One axis-aligned wall block. Used by both the render layer (to
/// spawn a cuboid mesh) and the server (collision push-out). Centre
/// is the world-space midpoint; size is the full extent (not half).
#[derive(Clone, Copy, Debug)]
pub struct WallSegment {
    pub center: Vec3,
    pub size: Vec3,
}

/// Building geometry constants — shared between render and collision
/// so the mesh and the AABB never disagree.
pub const FLOOR_THICKNESS: f32 = 0.2;
pub const WALL_THICKNESS: f32 = 0.15;
/// Vertical air-gap between the wall mesh and the floor/ceiling slab
/// it sits between, to avoid coplanar Z-fighting (see SKILL #9).
pub const WALL_GAP: f32 = 0.01;
/// Half-width of the entrance gap cut into the ground-story wall on
/// `Building.entrance`.
pub const ENTRANCE_HALF_WIDTH: f32 = 0.9;

impl Building {
    /// Wall AABBs for every story. The ground-story wall on the
    /// `entrance` side is split into two pieces with an opening.
    /// Corners belong to the East/West walls; North/South walls are
    /// shortened on each end by `WALL_THICKNESS` so they don't overlap
    /// (overlap would cause Z-fighting at the corner).
    pub fn wall_segments(&self) -> Vec<WallSegment> {
        let mut out = Vec::new();
        let center = self.footprint.center();
        let size_xz = self.footprint.size();
        let footprint_min = self.footprint.min;
        let footprint_max = self.footprint.max;

        let ew_len_z = size_xz.y;
        let ns_len_x = (size_xz.x - 2.0 * WALL_THICKNESS).max(0.0);

        for (story_idx, window) in self.floor_heights.windows(2).enumerate() {
            let lower = window[0];
            let upper = window[1];
            let upper_slab_bottom = upper - FLOOR_THICKNESS;
            if upper_slab_bottom <= lower + 2.0 * WALL_GAP {
                continue;
            }
            let wall_bottom = lower + WALL_GAP;
            let wall_top = upper_slab_bottom - WALL_GAP;
            let h = wall_top - wall_bottom;
            let cy = (wall_bottom + wall_top) * 0.5;
            let is_ground = story_idx == 0;

            // North (-Z edge): spans X, inset on both ends.
            push_wall(
                &mut out,
                Vec3::new(center.x, cy, footprint_min.y),
                Vec3::new(ns_len_x, h, WALL_THICKNESS),
                /*split_along_x=*/ true,
                is_ground && self.entrance == BuildingSide::North,
            );
            // South (+Z edge).
            push_wall(
                &mut out,
                Vec3::new(center.x, cy, footprint_max.y),
                Vec3::new(ns_len_x, h, WALL_THICKNESS),
                true,
                is_ground && self.entrance == BuildingSide::South,
            );
            // West (-X edge): full Z length, owns NW & SW corners.
            push_wall(
                &mut out,
                Vec3::new(footprint_min.x, cy, center.y),
                Vec3::new(WALL_THICKNESS, h, ew_len_z),
                false,
                is_ground && self.entrance == BuildingSide::West,
            );
            // East (+X edge): owns NE & SE corners.
            push_wall(
                &mut out,
                Vec3::new(footprint_max.x, cy, center.y),
                Vec3::new(WALL_THICKNESS, h, ew_len_z),
                false,
                is_ground && self.entrance == BuildingSide::East,
            );
        }

        out
    }
}

fn push_wall(
    out: &mut Vec<WallSegment>,
    center: Vec3,
    size: Vec3,
    split_along_x: bool,
    has_entrance: bool,
) {
    if !has_entrance {
        out.push(WallSegment { center, size });
        return;
    }
    let total_len = if split_along_x { size.x } else { size.z };
    let segment_len = total_len * 0.5 - ENTRANCE_HALF_WIDTH;
    if segment_len <= 0.05 {
        return; // entrance ≥ wall — nothing left to render.
    }
    let offset = ENTRANCE_HALF_WIDTH + segment_len * 0.5;
    for sign in [-1.0_f32, 1.0] {
        let (piece_center, piece_size) = if split_along_x {
            (
                Vec3::new(center.x + sign * offset, center.y, center.z),
                Vec3::new(segment_len, size.y, size.z),
            )
        } else {
            (
                Vec3::new(center.x, center.y, center.z + sign * offset),
                Vec3::new(size.x, size.y, segment_len),
            )
        };
        out.push(WallSegment {
            center: piece_center,
            size: piece_size,
        });
    }
}

/// A discrete-step staircase running along the Z axis.
///
/// The footprint encloses the whole flight on the XZ plane.
/// `low_y` is the surface at `footprint.max.y` (south end);
/// `high_y` is the surface at `footprint.min.y` (north end). So
/// walking *north* (decreasing Z) goes UP, *south* goes DOWN.
/// `steps` is the count of discrete equal-height step blocks.
///
/// Use `surface_y_at(xz)` to query the walkable Y at a point
/// inside the footprint.
#[derive(Component, Clone, Debug)]
pub struct Stairs {
    pub footprint: Rect,
    pub low_y: f32,
    pub high_y: f32,
    pub steps: u32,
}

impl Stairs {
    /// Walkable surface Y at this XZ, or `None` if outside the footprint.
    ///
    /// The surface is interpolated *linearly* along the stair's Z axis
    /// (a continuous ramp) even though the visual mesh is discrete
    /// blocks. This avoids the "jump" the player would otherwise feel
    /// crossing each step boundary; the visual stays low-poly while
    /// motion is smooth.
    pub fn surface_y_at(&self, xz: Vec2) -> Option<f32> {
        if self.steps == 0 || !self.footprint.contains(xz) {
            return None;
        }
        let z_range = self.footprint.max.y - self.footprint.min.y;
        if z_range <= 0.0 {
            return None;
        }
        // 0 at top edge (high_y), 1 at bottom edge (low_y).
        let frac_from_top = ((xz.y - self.footprint.min.y) / z_range).clamp(0.0, 1.0);
        Some(self.high_y + (self.low_y - self.high_y) * frac_from_top)
    }
}

/// The player's transform as seen by each client — delayed by the
/// server→client leg of the simulated network.
///
/// In `PlayingMultiplayer` this is updated 100 ms ±20 ms after the
/// server writes the real `Transform`, mirroring how a position
/// snapshot would travel over the wire.  In `PlayingSingle` it is
/// kept in immediate sync with `Transform` by a passthrough system.
///
/// Camera smoothing and billboard orientation read this component so
/// that both the camera lag AND the visual position reflect the full
/// 200 ms round-trip latency.  Collision detection and server logic
/// continue to use the authoritative `Transform`.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct ClientTransform(pub Transform);

/// Identifies which player slot an entity belongs to.
///
/// 0 = first player (WASD / Q-E), 1 = second player (IJKL / U-O).
/// Present on `Player` entities and `GameCamera` entities so camera
/// and billboard systems can route per-player without a global query.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PlayerSlot(pub u8);

/// Marks every entity spawned during an active game session.
/// The render plugin despawns all of them on `OnExit(Playing*)` so
/// transitioning back to the main menu starts clean.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct GameEntity;

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
