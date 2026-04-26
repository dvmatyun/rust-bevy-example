# my_bevy_game — Layered Architecture

The game is a single Bevy `App` whose plugins form a strict 4-layer
pipeline. Each layer can only depend on the layers below it.

```
┌─────────────────────────────────────────────────────┐
│                   data                              │  Layer 1
│   components, resources, intents (no systems)       │
└─────────────────────────────────────────────────────┘
                       ▲ types only
┌──────────────────────┴──────────────────────────────┐
│                  server                             │  Layer 2
│   game logic. Source of truth.                      │
│   Reads intents → mutates Transform / state.        │
│   NO render references.                             │
└─────────────────────────────────────────────────────┘
                       ▲ Transform, Resources
┌──────────────────────┴──────────────────────────────┐
│                client_sim                           │  Layer 3
│   input mapping → intents.                          │
│   game state → view-model (DesiredCameraView).      │
│   NO render references.                             │
└─────────────────────────────────────────────────────┘
                       ▲ DesiredCameraView, Transform
┌──────────────────────┴──────────────────────────────┐
│                  render                             │  Layer 4
│   Mesh3d, Camera3d, StandardMaterial, Node, Text.   │
│   The only layer allowed to touch render types.     │
└─────────────────────────────────────────────────────┘
```

## Why 4 layers?

The split between **server** and **client_sim** is what makes the game
networked-ready and unit-testable. The split between **client_sim** and
**render** is what makes simulation programmatic — you can run the
client logic headlessly in a test, drive it with synthetic input, and
assert on the resulting `DesiredCameraView` and `Transform` without any
GPU.

In a future networked build:

- `server` runs on the server process.
- `client_sim + render` runs on each client process.
- The link between them — currently shared ECS resources/events — is
  replaced by a message channel. The interface (intents up, snapshots
  down) stays the same.

## Layer details

### 1. `data/` — shared types

Pure declarations. The vocabulary every other layer imports.

| File | Contents |
|------|---------|
| `components.rs` | `Player`, `GameCamera`, `TerrainBlock`, `Biome` |
| `config.rs`     | `WorldConfig`, `TerrainHeights` |
| `intents.rs`    | `MoveIntent` (event), `DesiredCameraView` (resource) |

Allowed: `bevy_ecs`, `bevy_math`, `bevy_app` traits.
Forbidden: systems beyond `add_message` / `insert_resource`; render
types; anything platform-specific.

### 2. `server/` — game logic (source of truth)

| File | Systems |
|------|---------|
| `terrain.rs` | `setup_terrain` (Startup): generates heightmap via FBM noise, spawns `(TerrainBlock, Transform)` entities, populates `TerrainHeights`. |
| `player.rs`  | `spawn_player` (Startup), `apply_movement` (Update, consumes `MoveIntent`), `snap_to_ground` (Update, uses `TerrainHeights`). |

**Forbidden imports:** `Mesh3d`, `MeshMaterial3d`, `Camera3d`, `Color`,
`Node`, `Text`, `BackgroundColor`, `StandardMaterial`, `Image`,
`Window`, `ButtonInput`, `Touches`, `KeyCode`, `MouseButton`. The
server doesn't know about screens.

**Allowed:** `Transform`, `Time`, all of `bevy_ecs`/`bevy_app`/`bevy_math`,
reading custom resources from `data/`, consuming `MoveIntent`.

**Why this matters:** the server can be exercised by a unit test that
adds only `MinimalPlugins + DataPlugin + ServerPlugin`. No window, no
GPU.

### 3. `client_sim/` — client-side simulation

| File | Systems |
|------|---------|
| `input.rs` | `gather_input` — reads `ButtonInput<KeyCode>` → emits `MoveIntent`. |
| `camera_view.rs` | `compute_camera_view` — reads `Player.Transform` → writes `DesiredCameraView`. |

This layer is **the** place that translates platform input into game
intents. It also computes any view-model state the render layer needs.

**Forbidden imports:** the same render types as server. The new rule
here: do NOT mutate gameplay state directly — emit an intent, the
server handles it.

**Allowed:** input resources (`ButtonInput`, `Touches`, `MouseButton`),
reading `Transform` of game entities, writing view-model resources.

**Why this matters:** tests can synthesise input by emitting
`MoveIntent` events directly and inspect `DesiredCameraView` to assert
camera framing. No render, no window.

### 4. `render/` — visuals & UI

| File | Responsibility |
|------|----------------|
| `scene_setup.rs` | Sun, sky color, ambient light. |
| `terrain_view.rs` | `TerrainAssets` resource (mesh + materials). `attach_terrain_visuals` adds `Mesh3d`/`MeshMaterial3d` to entities marked `TerrainBlock` via `Added<TerrainBlock>`. |
| `player_view.rs` | `PlayerAssets`. `attach_player_visuals`. `billboard_player` (rotates player to face camera). |
| `camera_render.rs` | Owns the `Camera3d` entity. Reads `DesiredCameraView`, lerps the actual `Transform` toward it. |
| `hud.rs` | FPS overlay (`Node`, `Text`, `BackgroundColor`). |

This is the only layer that imports `Mesh3d`, `Camera3d`,
`StandardMaterial`, etc.

## Per-frame data flow

```
   keyboard               mouse / touch click
        │                         │
        ▼                         ▼
 client_sim::gather_move_input   render::handle_click_input
   (rotates by smoothed_azimuth)   (Camera::viewport_to_world,
   ──MoveIntent──┐                  intersect ground plane)
                 │                  ──ClickMoveIntent──┐
                 │                                     ▼
                 │                  client_sim::apply_click_target
                 │                    (set sticky MoveTarget)
                 │                                     │
                 │                                     ▼
                 │                  client_sim::auto_move_to_target
                 │ <────────────── MoveIntent ─────────┘
                 ▼
        server::apply_movement
          (mutates Player.Transform + Facing)
                 │
                 ▼
        server::snap_to_ground
                 │
                 ▼
        client_sim::apply_camera_orbit       (Q/E intent → target_azimuth)
        client_sim::smooth_camera_orbit      (smooth angle, shortest path)
        client_sim::smooth_camera_focus      (smooth focal point → player)
        client_sim::compute_camera_view      (DesiredCameraView from smooth state)
                 │
                 ▼
        render::update_camera_transform      (snap; smoothing already done)
        render::update_player_face_view      (front/side/back asset by dot)
        render::billboard_player             (face the camera)
                 │
                 ▼
              (renderer draws frame)
```

**Key smoothing rule:** all camera inertia is computed in `client_sim`
on the *angle* and *focal point*, not on the rendered position. This
guarantees the camera always stays on its orbit arc around the player
instead of cutting across in straight 3D lines when orbiting.

## Camera & input design notes

### Smoothing happens in `client_sim` on **angles** and **focal points**

Naïve linear lerp of camera **position** through 3D space cuts across
the inside of the orbit arc when orbiting fast — the camera visibly
"goes off centre". Instead we smooth the orbit **azimuth** and the
**focal point**:

- `CameraOrbit { target_azimuth, smoothed_azimuth }` — exponential
  smoothing on the angle, with shortest-path wrap-around handling.
- `CameraFocus { pos }` — exponential smoothing toward the player
  position.
- `compute_camera_view` derives `DesiredCameraView` from these every
  frame: `pos = focus + (sin(a)·d, h, cos(a)·d)`. The camera always
  lies on the orbit circle around the (smoothed) focus.

`Settings.camera_lerp_speed` is the rate constant `k` in the
frame-rate-independent formula `alpha = 1 - exp(-k·dt)`.

The render layer is a pure assignment — no further smoothing — to avoid
double-smoothing artefacts.

### WASD is camera-relative

`gather_move_input` reads `CameraOrbit.smoothed_azimuth` and rotates the
local input vector into world space:

- forward (W) = `(-sin a, -cos a)` — away from camera on XZ
- right (D)   = `(cos a, -sin a)` — perpendicular, screen-right

So W always moves "into the screen" regardless of how the player has
orbited the camera.

### Click-to-move uses the render layer for unprojection

`render::handle_click_input` reads mouse / touch events, performs a
`Camera::viewport_to_world` raycast, intersects the y = 0 plane, and
emits `ClickMoveIntent` (a `data/`-defined message). `client_sim`
consumes it into a sticky `MoveTarget` resource;
`auto_move_to_target` emits a `MoveIntent` toward the target each
frame until the player is within 0.4 units. Pressing any WASD key
clears the target so keyboard always wins.

This is the only input handler in the render layer — see "Decision
log → Why does the render layer emit one input intent" for the
rationale.

## Network replication

> **Philosophy:** game logic should never think about the network. Components
> opt in via a trait + a per-type plugin; the framework discovers all
> changes via Bevy's change detection and packs them into a delta buffer.

### Two opt-ins

1. **The component** implements `Replicated` (`Component + Serialize +
   DeserializeOwned`) with a stable `TYPE_ID`. Done in
   `data/replication.rs`.
2. **The entity** carries a `Replicate` marker. Without it, no
   replicated component on that entity is sent.

```rust
// data/components.rs — the component opts in
#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug)]
pub struct Facing(pub Vec2);

// data/replication.rs — register the wire id
impl Replicated for Facing { const TYPE_ID: u16 = type_ids::FACING; }

// server/mod.rs — register the per-type plugin
.add_plugins(ServerReplicatePlugin::<Facing>::default())

// server/player.rs — the entity opts in
commands.spawn((Player, Facing::default(), Transform::default(), Replicate));
```

### Audience targeting (the "to whom")

- **Default** = broadcast to every peer.
- Attach `ReplicateTo { peers: vec![PeerId; N] }` to an entity to
  scope visibility (rooms, areas of interest).

### How it works (per `PostUpdate` frame)

```
ReplicationSet::Reset      → drop last frame's outbox
ReplicationSet::Spawned    → Added<Replicate>           → Spawn  delta
ReplicationSet::Components → Added<T>                   → Upsert delta
                             Changed<T> (not Added)     → Upsert delta
                             RemovedComponents<T>       → Remove delta
ReplicationSet::Despawned  → RemovedComponents<Replicate> → Despawn delta
ReplicationSet::Flush      → drain ReplicationOutbox to transport
```

`Spawned` runs before `Components` so a fresh entity exists on the
receiver before its component values arrive. `Despawned` runs after
`Components` so `Remove` deltas for in-flight component removals
reach peers before the entity dies on their side.

### Why this is "the most optimised way" in Bevy (vs ecs_aim)

Bevy's primitives mean we don't write boilerplate for any of:

| Concern | Bevy primitive | What we'd write by hand otherwise |
|---|---|---|
| "What changed this frame" | `Ref<T>::is_changed()` ticks | per-component dirty flag bookkeeping |
| "What got added"           | `Added<T>` filter             | observer / on_insert hook |
| "What got removed"         | `RemovedComponents<T>`         | transient `RemoveComponentFromEntity` event-component (ecs_aim's pattern) |
| "Send only deltas, never full state" | the above three together | manual snapshot diff |
| Scheduling order           | `SystemSet` chain in `PostUpdate` | an explicit framework pass |
| Per-type registration      | generic `Plugin<T>`           | a registry / type-id table |

The collector uses `Ref<T>::is_changed() && !is_added()` to dedupe the
same-frame "Added → Changed" double-fire — one delta per component per
frame, carrying the latest value.

### What's deferred

- **Transport** — Stage-1 flush logs a one-line summary to stdout.
  Replace `flush_outbox` with a transport push when networking lands.
- **Receiver / `ServerEntity → LocalEntity` mapping** — needed only
  on the client side (separate App / process). The wire types (`ServerEntity`,
  `ReplicationDelta`) are already serde-friendly.
- **Authority / prediction** — server-authoritative is the default
  assumption. Client-side prediction is a separate layer above this.

## Decision log

### Why does the render layer emit one input intent (`ClickMoveIntent`)?

Screen → world unprojection requires the active `Camera` component
(projection matrix + viewport). Camera lives in `render/`, so the
mouse-click / touch-tap handler that turns a screen point into a
world point also lives there (`render/click_input.rs`). It emits
`ClickMoveIntent` (defined in `data/`) which `client_sim` consumes
into a sticky `MoveTarget`.

Pure key-and-touch-position input (no projection) still lives in
`client_sim/input.rs`. The rule is: **input that needs the camera is
allowed in `render/` and emits intents downward**.

### Why is `Player.Transform.rotation` written by the render layer?

The billboard system in `render/player_view.rs` mutates
`Player.Transform.rotation` so the slab faces the camera. This is the
only render-layer write to a logical entity's `Transform` and is a
deliberate exception:

- The rotation is purely visual; gameplay never reads it.
- WASD movement uses world-axis directions, not player-relative.

If a future feature needs rotation as game state (e.g. facing-direction
attacks), refactor to spawn a child `PlayerVisual` entity in the render
layer and rotate the child instead.

### Why a single `App` rather than `SubApp`s?

`SubApp` would give a fully isolated server world but adds complexity:
duplicated component registries, manual data shuttling. Plugin-level
separation enforced by the rules above provides 95% of the benefit at
20% of the complexity. When networking lands, the layer boundary is
already where the network channel goes.

### Why not separate crates?

Each layer could be its own crate, which would let the compiler enforce
the import bans. For a single-developer hobby project the friction
(workspace boilerplate, slower compile in CI, awkward shared types)
outweighs the benefit. The `architecture` skill plus a grep-based
verification command provide most of the same guarantees.

## Adding a feature: where does it go?

| Feature | Layer(s) |
|--------|----------|
| Add `Health` component | `data` |
| Damage when stepping on lava biome | `server` |
| New input gesture (e.g. jump button) | `data` (intent) + `client_sim/input.rs` (emit) + `server` (handle) |
| Health bar UI | `render/hud.rs` |
| Camera shake on damage | `client_sim` (compute shake offset → modify `DesiredCameraView`) |
| Multiplayer | replace the in-process intents/Transform sharing with a network channel between `server` and `client_sim` |

## Testing recipe

```rust
// tests/movement.rs
use bevy::prelude::*;
use my_bevy_game::{data::*, server::*, client_sim::*};

#[test]
fn move_intent_advances_player() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, DataPlugin, ServerPlugin, ClientSimPlugin));
    app.update(); // run Startup

    // Synthesise an intent.
    app.world_mut().send_message(MoveIntent { direction: Vec2::new(1.0, 0.0) });
    app.update(); // run Update

    let player_tf = app.world_mut()
        .query_filtered::<&Transform, With<Player>>()
        .single(app.world())
        .unwrap();
    assert!(player_tf.translation.x > 0.0);
}
```

The only Bevy plugin needed is `MinimalPlugins` — no window, no GPU,
deterministic.

## Verification grep

To check no render types leaked into lower layers:

```bash
grep -nrE "Mesh3d|MeshMaterial3d|Camera3d|StandardMaterial|BackgroundColor|TextColor|TextFont|Node[^a-zA-Z_]|TextLayout" \
  my_bevy_game/src/data \
  my_bevy_game/src/server \
  my_bevy_game/src/client_sim
```

Should return zero matches. The `architecture` skill runs this check.
