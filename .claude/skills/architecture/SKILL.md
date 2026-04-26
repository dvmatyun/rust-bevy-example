---
name: architecture
description: Use this skill when adding, modifying, or reviewing features in my_bevy_game. Enforces the 4-layer architecture (data → server → client_sim → render) and prevents render-layer leaks into game logic. Invoke proactively when the user asks to add gameplay, input, UI, or visual changes.
---

# 4-Layer Architecture Enforcement (my_bevy_game)

When working on `my_bevy_game/src/`, always respect the layered structure
documented in `docsai/architecture.md`. Read it first if you haven't in
this session.

## Layer summary

1. **`data/`** — pure types (components, resources, intents). No systems.
2. **`server/`** — game logic, source of truth. NO render references.
3. **`client_sim/`** — input → intents, view-model state. NO render
   references.
4. **`render/`** — `Mesh3d`, `Camera3d`, materials, UI. The only layer
   allowed to touch render types.

## Forbidden imports per layer

| Layer        | May NOT use                                                                                                            |
|--------------|------------------------------------------------------------------------------------------------------------------------|
| `data/`      | systems besides `add_message`/`insert_resource`; render types                                                          |
| `server/`    | `Mesh3d`, `MeshMaterial3d`, `Camera3d`, `Node`, `Text`, `Color`, `StandardMaterial`, `Image`, `BackgroundColor`, `ButtonInput`, `Touches`, `KeyCode`, `Window` |
| `client_sim/`| same render types as server. **May** read `ButtonInput` / `Touches` (it owns input mapping)                            |
| `render/`    | (no restrictions; this is where UI/render lives)                                                                        |

## Mandatory rules when adding code

1. **New shared type** → `data/`.
2. **New player action** → new `Message` in `data/intents.rs`, emitted
   by `client_sim/input.rs`, consumed by a `server/` system.
3. **New visual** → goes in `render/`. Use `Added<MarkerComponent>`
   queries to attach `Mesh3d`/`MeshMaterial3d` to entities created by
   lower layers.
4. **New camera behavior** → compute in `client_sim/`, apply in
   `render/`. **Camera smoothing always happens in client_sim** (on
   angles and focal points). Render just snaps to `DesiredCameraView`.
5. **No `Mesh3d`/`Camera3d`/`Color` outside `render/`**. Reject any
   change that introduces them in `data/`, `server/`, or `client_sim/`.
6. **System ordering** — use `AppSet` system sets to enforce
   `Input → GameLogic → ViewModel → RenderApply`. Plugins assign their
   `Update` systems to a set with `.in_set(AppSet::...)`. Use
   `.chain()` for in-set ordering when needed.
7. **New replicated component** → derive `Component + Serialize +
   Deserialize`, add a `TYPE_ID` constant in
   `data/replication.rs::type_ids`, `impl Replicated`, register
   `ServerReplicatePlugin::<T>::default()` in `server/mod.rs`. Game
   code stays untouched — change detection wires it up. See
   `docsai/architecture.md` → "Network replication".
8. **All UI must be mobile-friendly.** Every overlay, HUD, panel,
   button must be readable and operable on small phone screens
   (target ≥ 360 × 640 px). Concretely:
   - **No element ever sits on top of another.** The settings gear
     and the FPS HUD are both top-right candidates — the HUD's
     `right` margin must include space for the gear (currently
     `gear_clearance = 20 + 48 + 12 = 80 px`). Verify by reading
     the actual layout, not by trusting that values "look fine".
   - **Panels use `left`+`right` margins, not fixed `width`.** A
     fixed 260px panel sticks out or wastes space; `left:
     safe_side, right: safe_side, max_width: ...` adapts.
   - **Buttons ≥ 40×40 px** (touch target). The gear is 48×48; the
     ± step buttons are 28×28 which is borderline — if you add new
     buttons that need to be tappable, default to 44 px.
   - **Safe-area padding**: top 48px (status bar / notch), bottom
     34–66px (home indicator / nav bar), sides ≥ 20px. Defined
     once and reused; don't sprinkle different values.
   - **Wrap, don't overflow.** Long labels / button rows should use
     `flex_wrap: FlexWrap::Wrap` with `column_gap` + `row_gap`.
   - When unsure, pretend the screen is 360×640 and check that
     everything is still tappable and readable.

### Exception: screen-space input lives in `render/`

Mouse-click / touch-tap → world raycast requires the active `Camera`
(for `viewport_to_world`). The handler that emits `ClickMoveIntent`
therefore lives in `render/click_input.rs`. This is the only input
handler in render. **Pure key/touch-position input still lives in
`client_sim/input.rs`** — only handlers that need the projection are
allowed in render.

## Verification grep (run before reporting work done)

```bash
grep -nrE "Mesh3d|MeshMaterial3d|Camera3d|StandardMaterial|BackgroundColor|TextColor|TextFont|Node[^a-zA-Z_]|TextLayout" \
  my_bevy_game/src/data \
  my_bevy_game/src/server \
  my_bevy_game/src/client_sim
```

MUST return zero matches. If it doesn't, refactor before continuing.

Also run:

```bash
cargo check --bin my_bevy_game_bin
```

It must finish with no errors.

## When the user proposes something that violates this

Don't silently put rendering in the server. Explain which layer the
code should go in and refactor the plan before implementing. Examples:

- "Spawn a sword mesh when the player attacks" → split into:
  `data` (`Sword` component, `AttackIntent`), `server` (handle intent,
  spawn `(Sword, Transform)`), `render` (attach mesh via
  `Added<Sword>`).
- "Make the camera shake on damage" → `client_sim` computes shake
  offset, modifies `DesiredCameraView`. `render` already lerps toward
  it.
- "Show a HUD message when …" → server emits a `Message` (e.g.
  `Notification`); HUD in `render/hud.rs` reads it.

## Useful references during work

- `docsai/architecture.md` — full architecture description with data
  flow diagrams, decision log (Player rotation exception, screen-space
  input exception, single-App vs SubApp), camera/input design notes,
  and a unit-test recipe.
- `docsai/bevy-ui-lessons.md` — **comprehensive Bevy 0.19-dev pitfalls
  cheat-sheet**. Read before adding UI/render code or touching Cargo
  features. Covers feature flags, 0.18 → 0.19 API renames, UI spawn
  patterns, platform `cfg` guards, Windows build issues, and camera
  smoothing.
- `docsai/android-debugging-log.md` — Android-specific findings
  (Mali GPU bugs, NDK setup, APK packaging quirks).
- `CLAUDE.md` — Project guide (build commands, platform setup,
  milestones).

## Quick pitfall reminders (high-frequency hits)

Before reporting work done, sanity-check these against the lessons doc:

- **UI invisible / app exits with code 1 silently** → `bevy_ui_render`
  feature missing. Check `Cargo.toml`.
- **`AmbientLight is not a Resource`** → it's now `GlobalAmbientLight`
  in 0.19-dev (the resource name changed; `AmbientLight` is now a
  per-camera Component).
- **`shadows_enabled` field error** → renamed to
  `shadow_maps_enabled`.
- **`ChildBuilder` not found** → renamed to `ChildSpawnerCommands`.
- **`font_size: 30.0` doesn't compile** → wrap with `FontSize::Px(30.0)`.
- **`Event` / `EventReader` not found** → renamed to `Message` /
  `MessageReader`. Use `app.add_message::<T>()` and
  `MessageWriter::write(...)`.
- **`wgpu-hal` `ID3D12Heap` errors on Windows** → `cargo update windows@0.61.3 --precise 0.62.2`.
- **Linker fails "file in use" after running** → rust-analyzer is
  holding the exe; restart it (`Ctrl+Shift+P → rust-analyzer:
  Restart server`).
- **Camera "cuts through corner" when orbiting** → smoothing is on
  position instead of angle/focus; smoothing must live in
  `client_sim` on `azimuth` and `focus.pos`, render layer just snaps.
- **WASD doesn't follow rotated camera** → `gather_move_input` must
  rotate the local input vector by `CameraOrbit.smoothed_azimuth`.
