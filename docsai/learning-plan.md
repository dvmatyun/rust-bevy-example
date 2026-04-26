# Learning plan: this codebase, step by step

A guided tour through `my_bevy_game/` for a Rust newcomer. Each step has:

- **Read** — files / sections to open in this order
- **Understand** — the question you should be able to answer afterwards
- **Try** — a small concrete experiment (often: a stub test in `tests/`)
- **Hints** — what to look up if you get stuck

If you complete a step and the "Understand" question still feels fuzzy,
re-read the linked file and tweak one line at a time until the
behaviour change matches your mental model.

---

## Stage 0 — before you write any code

**Read**
1. [`README.md`](../README.md) and [`CLAUDE.md`](../CLAUDE.md) — what this
   project is and how to build it.
2. [`docsai/architecture.md`](architecture.md) — the **4-layer
   architecture** (`data → server → client_sim → render`). Look at the
   ASCII diagram and the per-frame data-flow section.
3. [`.claude/skills/architecture/SKILL.md`](../.claude/skills/architecture/SKILL.md)
   — the rules. Don't memorise; just see what's enforced.

**Understand**
- What lives in each of the 4 layers? Why is the split there?
- Which layer is allowed to spawn `Mesh3d`, `Camera3d`, `Color`?

**Try** — open `tests/00_smoke.rs`, run `cargo test --test 00_smoke`. It
should already pass. You're verifying your toolchain works.

---

## Stage 1 — Components: the simplest Bevy primitive

**Read**
1. [`src/data/components.rs`](../my_bevy_game/src/data/components.rs)

**Understand**
- A **component** in Bevy ECS is just a `struct` with `#[derive(Component)]`.
- A **marker component** (zero-sized struct like `Player;`) is a tag
  that has no data — just signals "this entity is a player".
- `Facing(pub Vec2)` is a *tuple struct* — like `Facing { 0: Vec2 }` but
  shorter syntax.

**Try** — `tests/01_components.rs`: create a `Health(i32)` component and
spawn an entity with it. Read the `App::new` boilerplate from the test.

**Hints**
- Rust Book: ["Defining and Instantiating Structs"](https://doc.rust-lang.org/book/ch05-01-defining-structs.html)
- Bevy book: [ECS basics](https://bevyengine.org/learn/quick-start/getting-started/ecs/)

---

## Stage 2 — Resources: global state for the App

**Read**
1. [`src/data/config.rs`](../my_bevy_game/src/data/config.rs) — `WorldConfig`,
   `Settings`, `TerrainHeights`. All `#[derive(Resource)]`.
2. [`src/data/intents.rs`](../my_bevy_game/src/data/intents.rs) —
   `DesiredCameraView`, `JoystickState`, `MoveTarget`. View-model
   resources.

**Understand**
- A **resource** is a singleton — exactly one instance per `App`.
   Components belong to entities; resources don't.
- `Settings::default()` is automatically inserted by `DataPlugin`.
- `Res<T>` (read-only) and `ResMut<T>` (read/write) are the system
  parameters that fetch resources.

**Try** — `tests/02_resources.rs`: insert a custom resource and read it
in a system.

**Hints**
- The trait `Default` provides `Default::default()` — a "make a
  reasonable starting value" function.
- Most numeric types (`i32`, `f32`, `bool`) implement `Default`
  automatically, returning 0 / 0.0 / false.

---

## Stage 3 — Systems: functions that do work

**Read**
1. [`src/server/player.rs`](../my_bevy_game/src/server/player.rs) — three
   short systems: `spawn_player`, `apply_movement`, `snap_to_ground`.
2. [`src/client_sim/input.rs`](../my_bevy_game/src/client_sim/input.rs) —
   `gather_move_input` and `gather_camera_orbit_input`.

**Understand**
- A **system** is a function whose parameters are all *system
  parameters* (`Res<T>`, `ResMut<T>`, `Query<...>`, `MessageReader<T>`,
  …). Bevy's scheduler injects them.
- `Query<&T>` borrows components shared. `Query<&mut T>` borrows
  exclusively. The scheduler runs systems with non-overlapping borrows
  in parallel for free.
- Filters like `With<Player>` constrain *which* entities are returned
  but don't fetch data.

**Try** — `tests/03_systems.rs`: write a system that spawns one entity,
then queries it back.

**Hints**
- Rust Book: ["What is Ownership?"](https://doc.rust-lang.org/book/ch04-01-what-is-ownership.html)
  and ["References and Borrowing"](https://doc.rust-lang.org/book/ch04-02-references-and-borrowing.html).
- Bevy book: [System parameters](https://bevyengine.org/learn/quick-start/getting-started/ecs/#system-parameters).

---

## Stage 4 — Messages (events): one-shot inter-system signals

**Read**
1. [`src/data/intents.rs`](../my_bevy_game/src/data/intents.rs) —
   `MoveIntent`, `CameraOrbitIntent`, `ClickMoveIntent`. All
   `#[derive(Message)]`.
2. The `MessageReader<T>` / `MessageWriter<T>` calls in
   `src/server/player.rs::apply_movement` and
   `src/client_sim/input.rs::gather_move_input`.

**Understand**
- Messages are **fire-and-forget signals** between systems. Multiple
  readers each see the same events.
- `app.add_message::<T>()` registers a message type. We do this in
  `DataPlugin`.
- A message lives for ~2 frames in a double-buffered queue. So a
  reader running after the writer in the same frame sees it.

**Try** — `tests/04_messages.rs`: emit a custom message from a writer
system, count it in a reader system.

**Hints**
- In Bevy 0.18 these were called `Event` / `EventReader` / `EventWriter`.
  In 0.19-dev they were renamed to `Message` / `MessageReader` /
  `MessageWriter`.
- See [docsai/bevy-ui-lessons.md → Bevy 0.18 → 0.19 API renames](bevy-ui-lessons.md).

---

## Stage 5 — System scheduling: ordering without hard-coding it

**Read**
1. [`src/data/mod.rs`](../my_bevy_game/src/data/mod.rs) — the `AppSet`
   enum and the `configure_sets(...)` chain in `DataPlugin`.
2. The `.in_set(AppSet::...)` calls in
   [`src/server/mod.rs`](../my_bevy_game/src/server/mod.rs),
   [`src/client_sim/mod.rs`](../my_bevy_game/src/client_sim/mod.rs),
   [`src/render/mod.rs`](../my_bevy_game/src/render/mod.rs).

**Understand**
- A `SystemSet` is a *label* attached to a system. Systems with the
  same label form a group.
- `configure_sets(Update, (A, B, C).chain())` says "in the `Update`
  schedule, all systems in set A run before all systems in set B,
  which run before all systems in set C".
- This decouples ordering from individual system identities — you can
  add a new system to set B without telling any other system about it.

**Try** — `tests/05_ordering.rs`: write two systems and force one to
run before the other using `.before(...)`.

**Hints**
- Bevy book: [Schedules and SystemSets](https://bevyengine.org/learn/quick-start/getting-started/plugins/).
- Rust Book: ["Defining an Enum"](https://doc.rust-lang.org/book/ch06-01-defining-an-enum.html).

---

## Stage 6 — Plugins: bundling related functionality

**Read**
1. [`src/lib.rs`](../my_bevy_game/src/lib.rs) — the top-level
   `add_plugins((...))` call.
2. [`src/server/mod.rs`](../my_bevy_game/src/server/mod.rs) — the
   `ServerPlugin` `impl Plugin for ServerPlugin`.
3. [`src/render/joystick_ui.rs`](../my_bevy_game/src/render/joystick_ui.rs)
   for a single-feature plugin.

**Understand**
- A **`Plugin`** is anything implementing `fn build(&self, app: &mut App)`.
- `App::add_plugins((P1, P2, P3))` calls each plugin's `build()` in
  order, allowing them to register their own systems / resources /
  messages.
- Plugins can be parameterised: `ServerReplicatePlugin::<T>` is generic
  over which component type to replicate.

**Try** — `tests/06_plugins.rs`: write a plugin that inserts a
resource and adds a system that increments it.

**Hints**
- Generic plugin types (`Plugin<T>`) need a `PhantomData<T>` field to
  satisfy "use the type parameter in the struct" — see
  `src/server/replication.rs`.

---

## Stage 7 — Change detection: react when data moves

**Read**
1. [`src/server/replication.rs`](../my_bevy_game/src/server/replication.rs)
   — `collect_added`, `collect_changed`, `collect_removed`.
2. The `Added<T>` filter in
   `src/render/terrain_view.rs::attach_terrain_visuals` and
   `src/render/player_view.rs::attach_player_visuals`.

**Understand**
- Bevy stores per-component **change ticks**. Every time a system
  mutates a component, the tick advances.
- The `Added<T>` filter matches entities where T was *added* (as in
  inserted) since the last time the system ran.
- The `Changed<T>` filter matches entities where T was *added or
  modified*.
- `Ref<T>` lets a system check `is_added()` / `is_changed()`
  manually — useful for dedupe (see `collect_changed`).

**Try** — `tests/07_change_detection.rs`: spawn an entity with a
component, then mutate it. Verify that a system reading `Changed<T>`
sees the change exactly once.

**Hints**
- Filters are *zero-cost* — they're tested per-archetype, not
  per-entity, so they're cheap.

---

## Stage 8 — Camera math: smoothing, look-at, raycast

**Read**
1. [`src/client_sim/camera_view.rs`](../my_bevy_game/src/client_sim/camera_view.rs) —
   exponential smoothing, predictive lookahead, shortest-angle lerp.
2. [`src/render/camera_render.rs`](../my_bevy_game/src/render/camera_render.rs) —
   how the smoothed values become the rendered `Camera3d.Transform`.
3. [`src/render/click_input.rs`](../my_bevy_game/src/render/click_input.rs) —
   `viewport_to_world` raycast.

**Understand**
- `1.0 - (-k * dt).exp()` is the frame-rate-independent smoothing
  factor `α`. Higher `k` = snappier.
- "Lookahead in time" is implemented by adding `velocity * lookahead`
  to the focal point, so the focus *predicts* where the player will
  be. The ramp-up time of the velocity estimate is the "OK to lag"
  window you feel right after starting to move.
- For click → world, project the screen point through the camera's
  inverse projection matrix (`Camera::viewport_to_world`), then
  intersect the ray with the terrain heightmap (`TerrainHeights::raycast`).

**Try** — change `camera_lookahead` in the settings panel from 0.0 to
1.0 and observe the camera shifting.

---

## Stage 9 — Rendering: meshes, materials, billboards

**Read**
1. [`src/render/terrain_view.rs`](../my_bevy_game/src/render/terrain_view.rs) —
   `TerrainAssets`, `attach_terrain_visuals`.
2. [`src/render/player_view.rs`](../my_bevy_game/src/render/player_view.rs) —
   five face-mesh variants, `billboard_player` (yaw-only rotation),
   `update_player_face_view` (pick mesh by dot product).
3. [`src/render/markers.rs`](../my_bevy_game/src/render/markers.rs) —
   simple cosmetic markers.

**Understand**
- `Mesh3d` and `MeshMaterial3d<StandardMaterial>` are tuple-struct
  components wrapping a `Handle<…>`. Multiple entities sharing the
  same handle share the same GPU resource (cheap).
- Billboarding = computing a yaw rotation each frame so a flat slab
  always faces the camera.
- "Pick the right mesh from a discrete set based on view angle" is a
  common trick — you've seen it in classic 2D-in-3D games (Doom).

**Try** — change the player face colours in `register_player_assets`,
rebuild on Windows, watch the colour wheel.

---

## Stage 10 — Replication: change-detection meets the wire

**Read**
1. [`src/data/replication.rs`](../my_bevy_game/src/data/replication.rs) —
   `Replicated` trait, `Replicate` marker, `ReplicationDelta`.
2. [`src/server/replication.rs`](../my_bevy_game/src/server/replication.rs) —
   how `Added<T>` / `Changed<T>` / `RemovedComponents<T>` drive the
   `ReplicationOutbox`.
3. [`docsai/architecture.md`](architecture.md) → "Network replication".

**Understand**
- Network replication and change detection are the *same problem*.
  Bevy's tick mechanism gives us "what changed since last tick" for
  free. We just serialise those changes into byte-deltas.
- The `Replicate` marker on an entity is the opt-in. Without it, no
  replicated component on the entity is sent.
- A per-type generic plugin (`ServerReplicatePlugin<T>`) registers
  collectors that target one specific component type.

**Try** — `tests/08_replication.rs`: register a custom `Replicated`
component, spawn an entity with `Replicate`, mutate it, assert the
outbox now contains your delta.

---

## Stage 11 — Going beyond: where to read next

- **Bevy internals**: how archetypes work, how systems are run in
  parallel based on access analysis. Read
  `C:/Repositories/Rust/bevy/crates/bevy_ecs/src/schedule/executor/multi_threaded.rs`
  with the survey from
  [docsai/architecture.md → Decision log](architecture.md).
- **The Dart source we're porting from** — see
  [`docsai/dart-source-reference.md`](dart-source-reference.md) and the
  port plan at [`docsai/gameplay-port-plan.md`](gameplay-port-plan.md).
  Pick the simplest unported subsystem (movement upgrade) and try it.
- **Broader Rust topics** (ownership, lifetimes, async, macros,
  unsafe) — see [`docsai/rust-learning-plan.md`](rust-learning-plan.md).

---

## Pacing

Don't sprint. A reasonable pace:

- Stages 0–3 in one sitting (~2 hours): components, resources, systems.
- Stages 4–6 in another sitting: messages, scheduling, plugins.
- Stages 7–10 over a week — these are where real understanding compounds.
- Stage 11 is open-ended.

If a stage's "Try" feels easy, scan the next one. If a stage feels
hard, go back to the previous one's source files and **change one
parameter at a time** to see what breaks.
