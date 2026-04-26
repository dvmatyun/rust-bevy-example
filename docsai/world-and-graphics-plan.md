# World & graphics plan

Design notes for upgrading the world from "voxel cubes + procedural
heightmap" to a Valheim-like low-poly look with hand-authored
content, destructibility, and multi-storey buildings. Written as
recommendations + trade-offs, not a step-by-step for one specific
feature.

## TL;DR — recommended order

1. **Visual upgrade first** — switch from voxel cubes to a triangulated
   flat-shaded heightmap mesh + vertex colours. Biggest visual
   improvement, smallest architectural change. `TerrainHeights` stays
   as the source of truth.
2. **Hybrid maps** — keep procedural terrain, add a RON file describing
   hand-placed entities (buildings, NPCs, props). Reuses the existing
   ECS spawning path.
3. **Voxel destruction** — add a `TerrainMutation` event the server
   applies to `TerrainHeights`; render rebuilds the affected mesh
   chunk via `Changed<TerrainHeights>`.
4. **Trees / rocks as destructibles** — spawn them as entities with
   `Health + Replicate`; despawn on death. First non-terrain
   destructibles.
5. **Multi-storey buildings** — last. Forces the biggest change
   (`snap_to_ground` becomes "find highest walkable surface near me",
   not "look up height in `TerrainHeights`") and likely pulls in a
   physics engine.

## 1. Valheim-style low-poly terrain

Three independent visual ingredients:

1. **Smooth surface, not voxels** — use a triangulated heightmap mesh
   (each cell = 2 triangles). The mesh is generated once per chunk
   from `TerrainHeights`.
2. **Flat shading** (the "faceted" look) — each triangle has its own
   normal. Achieved by *not sharing vertices*: every triangle gets 3
   unique vertices, each carrying the face normal. Triples vertex
   count, but eliminates per-vertex normal averaging and the
   smoothed-out look.
3. **Per-vertex colours per biome** (instead of textures) — apply
   `Mesh::ATTRIBUTE_COLOR` so each vertex carries its biome colour.
   One `StandardMaterial { base_color: WHITE, .. }` with the vertex
   colours doing the work. No texture loading, easy to tweak.

### Concrete migration

Replace [server/terrain.rs](../my_bevy_game/src/server/terrain.rs)'s
"spawn one Cuboid per cell" loop with a chunk-mesh build:

```rust
// Per chunk:
let mut positions: Vec<[f32; 3]> = Vec::with_capacity(cells * 2 * 3);
let mut normals:   Vec<[f32; 3]> = Vec::with_capacity(cells * 2 * 3);
let mut colors:    Vec<[f32; 4]> = Vec::with_capacity(cells * 2 * 3);
for z in 0..CHUNK_SIZE {
    for x in 0..CHUNK_SIZE {
        // Heights at the four corners
        let h00 = heights.at(x, z) as f32;
        let h10 = heights.at(x + 1, z) as f32;
        let h01 = heights.at(x, z + 1) as f32;
        let h11 = heights.at(x + 1, z + 1) as f32;

        // Two triangles, each with its own face normal + uniform colour
        push_triangle(&mut positions, &mut normals, &mut colors,
            [(x, h00, z), (x+1, h10, z), (x, h01, z+1)],
            biome_color(h00));
        push_triangle(&mut positions, &mut normals, &mut colors,
            [(x+1, h10, z), (x+1, h11, z+1), (x, h01, z+1)],
            biome_color(h10));
    }
}
let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, default());
mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL,   normals);
mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR,    colors);
let mesh_handle = meshes.add(mesh);
```

Render side: a single `Mesh3d(mesh_handle)` per chunk. **Massively
fewer entities** than the current ~4096 per-cell cubes — should also
be much faster.

### Polish (after the mesh switch)

- **DirectionalLight + `shadow_maps_enabled = true`** on desktop (we
  already have this; `cfg!(android)` disables shadows for Mali).
- **Atmospheric fog**: `bevy_render`'s `FogSettings` component on the
  camera entity. Distance-based linear or exponential fog gives the
  Valheim "moody horizon" look in ~5 lines.
- **Tonemapper**: already enabled via the `tonemapping_luts` feature.
  Try `Tonemapping::TonyMcMapface` or `BlenderFilmic` on the camera.
- **Bloom + SSAO**: `bevy_core_pipeline::bloom::Bloom`, `bevy_pbr::ssao`
  components. Big visual upgrade, modest perf cost.

### Time estimate

- Day 1: chunk-mesh generator + render integration. Game looks
  immediately different.
- Day 2: vertex colours per biome + flat-shade refinement (slope
  shading, beach blending).
- Day 3: fog + tonemapper polish.

## 2. Best way to make 3D maps

Three approaches:

| Approach | Pros | Cons |
|---|---|---|
| **Procedural** (what you have) | Fast iteration, infinite variation | Maps feel samey; no story-driven content |
| **Authored** (Blender / level editor) | Tight artistic control | Slow; needs a workflow / editor |
| **Hybrid** (procedural + hand-placed POIs) | Best of both | Two pipelines |

**Recommendation: hybrid.** Keep the noise-driven heightmap as the
canvas; layer in hand-placed entities (NPCs, buildings, item caches)
loaded from a file.

### Suggested format: RON

[RON](https://docs.rs/ron) is the Rust-flavoured JSON. `serde` derives
work for free, and it's far more readable than JSON for nested
structures.

```ron
// assets/maps/starter_island.ron
World(
    spawn_points: [ (x: 0.0, z: 0.0), (x: 5.0, z: -3.0) ],
    buildings: [
        Building(
            kind: "lodge",
            position: (x: 12.0, z: -8.0),
            rotation_deg: 45.0,
        ),
        Building(kind: "well", position: (x: -2.0, z: 4.0), rotation_deg: 0.0),
    ],
    npcs: [
        Npc(kind: "merchant", position: (x: 11.0, z: -6.0)),
    ],
)
```

Load at startup with `bevy_asset` + a small custom `AssetLoader`.
Each entry maps to a spawn function that creates the right entity
bundle.

### Editor (defer)

When the file gets too painful to edit by hand, build a dev-only
overlay using `bevy_egui` — drag entities around in-game, save back
to RON. Don't build this until you actually have ~50+ placed
entities; it's a multi-day project on its own.

## 3. Destructible environment

Three classes:

| Approach | Examples | Cost / fit |
|---|---|---|
| **Voxel destruction** | Minecraft, Teardown | Trivial if your world is data. Perfect fit for our heightmap. |
| **Mesh splitting** | Red Faction Guerrilla | Geometric cuts at runtime. Hard. Needs physics. |
| **Pre-fractured** | Battlefield buildings | Designer pre-builds the debris pieces. Cheap to *use*, expensive to *author*. |

**Recommendation: voxel-style heightmap mutation.** Maps onto our
existing data model directly.

### Design

Add an event in `data/intents.rs`:

```rust
#[derive(Message, Clone, Debug)]
pub struct TerrainMutation {
    pub x: i32,
    pub z: i32,
    pub delta_h: i32, // negative = dig, positive = raise
}
```

A new server system in `server/terrain.rs` consumes the event:

```rust
fn apply_terrain_mutations(
    mut events: MessageReader<TerrainMutation>,
    mut heights: ResMut<TerrainHeights>,
) {
    for ev in events.read() {
        // Mutate the cell. ResMut triggers Changed<TerrainHeights>
        // which the render layer's chunk-rebuild system picks up.
        let half = heights.size / 2;
        let lx = (ev.x + half).clamp(0, heights.size - 1) as usize;
        let lz = (ev.z + half).clamp(0, heights.size - 1) as usize;
        let idx = lz * heights.size as usize + lx;
        heights.cells[idx] = (heights.cells[idx] + ev.delta_h).max(0);
    }
}
```

Render side: when `heights.is_changed()`, find which chunk(s) the
mutated cells fall in and rebuild only those meshes. Bevy's `Changed<
Resource>` doesn't tell you *which cells* changed — easiest is to
maintain a `HashSet<ChunkCoord>` of dirty chunks that the mutation
system fills, the render system drains.

### Replication

`TerrainMutation` is itself a perfect Replicated message — it's
small (12 bytes) and self-contained. The server emits it, the
network sends it to clients, the client's own `apply_terrain_mutations`
applies the same change to the client's `TerrainHeights`. Render then
rebuilds the chunk identically on both sides.

(Sending the entire `TerrainHeights` resource as a Replicated
component is also an option but wasteful — most cells never change.)

### Trees / rocks (entity-based destructibles)

Separate from terrain. Each tree is an entity with `Mesh3d`, `Health`,
and `Replicate`. When `Health.current ≤ 0`, despawn (and optionally
spawn debris particles). The replication infrastructure already
handles spawn / despawn deltas — no new code.

## 4. Multi-storey buildings (same XY, different Z)

The fundamental constraint of a heightmap: **one Y per (X, Z)**. To
walk on multiple levels, abandon "the world IS a heightmap" and treat
it as "the world is a base heightmap *plus* mesh-collision entities".

| Approach | Notes |
|---|---|
| **Floor-as-entity** | Buildings are separate entities with their own collision. Player Y is determined by which surface (terrain or floor) is "under" them. Skyrim, Valheim. |
| **Voxel grid** | Full 3D voxel storage. Expensive (Minecraft ships ~100KB/chunk compressed). Total flexibility. |
| **Layered heightmap** | N stacked heightmaps. Limited to N levels, doesn't handle complex shapes. Avoid. |

**Recommendation: floor-as-entity + eventually a physics engine.**

### Design

```rust
// data/components.rs
#[derive(Component)]
pub struct Building {
    pub footprint: Rect2d,         // XZ bounds
    pub floors: Vec<FloorPlane>,    // ordered low → high
}

pub struct FloorPlane {
    pub y: f32,
    pub walkable: WalkableArea,    // polygon or rect inside footprint
}
```

Refactor `server::snap_to_ground`:

```rust
fn snap_to_ground(
    heights: Res<TerrainHeights>,
    buildings: Query<&Building>,
    config: Res<WorldConfig>,
    mut q: Query<&mut Transform, With<Player>>,
) {
    for mut tf in &mut q {
        let p = tf.translation;
        let mut best_y = heights.ground_y(p.x, p.z);
        for b in &buildings {
            if !b.footprint.contains(Vec2::new(p.x, p.z)) {
                continue;
            }
            for floor in &b.floors {
                // Pick the highest floor that's at-or-below the
                // player's current Y (so jumping up to next floor
                // doesn't tunnel down through the current one).
                if floor.y <= p.y + 0.5 && floor.y > best_y && floor.walkable.contains(p) {
                    best_y = floor.y;
                }
            }
        }
        tf.translation.y = best_y + config.player_half_height;
    }
}
```

This works for static, non-overlapping buildings and is enough for
houses, towers, bridges. Good for ~80% of the use cases.

### When you outgrow the simple approach

Add a physics engine for proper mesh collisions:

- **`bevy_rapier3d`** — wraps the well-known Rapier physics engine.
  Mature, battle-tested. ~1MB extra binary.
- **`avian3d`** (formerly `bevy_xpbd_3d`) — pure-Rust, position-based
  dynamics. Bevy-native, growing fast.

Either lets you assign mesh colliders to building entities and a
character controller to the player. The collision response automatically
handles "you're walking on a sloped roof" or "you fell through a hole".

This is the *biggest* architectural change of the four — defer it
until you have enough buildings to justify it.

## Cross-cutting considerations

### Replication

All four upgrades fit our existing replication infrastructure:

- **Visual mesh upgrade** — purely client-side, no replication impact.
- **Hand-placed entities** — server spawns them with `Replicate`;
  components ride the existing pipeline.
- **Terrain mutations** — `TerrainMutation` is a Message, replicate
  it directly via the message wire.
- **Buildings** — `Building` is just a `Replicated` component;
  spawn/despawn deltas already work.

### Performance budgets

Rough budget on the test phone (Mali-G77, mobile CPU):

- Terrain mesh: 64×64 chunk × 2 triangles × 3 verts = 24,576 verts.
  Trivial for any GPU.
- Building meshes: target < 1,000 verts each, 50 buildings on screen
  → 50K verts. Fine.
- Trees / rocks: instance them via `bevy_render`'s
  `InstancedMaterialPlugin` pattern. 5,000 trees at 200 verts each
  via instancing → essentially free.

### Save / load

Once we have terrain mutations + entities placed via RON:

- Map file = procedural seed + RON-defined entities + replay log of
  terrain mutations
- Load = generate from seed → spawn RON entities → apply mutations
- Save = serialise the entity world (Bevy's `bevy_scene` does this)
  + the mutation log

Bevy's `bevy_scene` (with `serialize` feature, which we already have)
handles the entity-world half. The mutation log is just a
`Vec<TerrainMutation>` you keep in a resource.
