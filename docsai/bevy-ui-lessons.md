# Bevy 0.19-dev Pitfalls — Comprehensive Cheat-Sheet

A running log of every issue we ran into and how to fix it. Skim the
table of contents before adding Bevy code or touching dependencies.

## Contents

- [Cargo features (the biggest gotcha)](#cargo-features-the-biggest-gotcha)
- [Bevy 0.18 → 0.19 API renames](#bevy-018--019-api-renames)
- [Resource vs Component renames](#resource-vs-component-renames)
- [UI spawn patterns and types](#ui-spawn-patterns-and-types)
- [Camera, sprites and 2D-in-3D tricks](#camera-sprites-and-2d-in-3d-tricks)
- [Platform-specific code (cfg guards)](#platform-specific-code-cfg-guards)
- [Windows-specific build issues](#windows-specific-build-issues)
- [Camera smoothing pitfalls](#camera-smoothing-pitfalls)

---

## Cargo features (the biggest gotcha)

When using `default-features = false`, Bevy 0.19-dev splits rendering
across many crates. Missing one yields a **silent failure**: app
crashes with exit code 1 and **no error logs** on the console.

### `bevy_ui_render` is mandatory for visible UI

`bevy_ui` provides only ECS types (`Node`, `Button`, `Text`, ...).
**`bevy_ui_render` is the crate that actually draws them.** Without it,
UI nodes exist in the world but never appear on screen — and the
process exits with code 1 silently.

`bevy_ui_render` pulls in `bevy_sprite_render` automatically, but
you also need `bevy_sprite` and `bevy_text` + `default_font`.

### Minimum features for 3D + UI in 0.19-dev

```toml
[dependencies.bevy]
path = "C:/Repositories/Rust/bevy"
default-features = false
features = [
  # 3D rendering
  "bevy_pbr", "bevy_core_pipeline", "bevy_render",
  "bevy_camera", "bevy_light", "bevy_mesh", "bevy_image", "bevy_shader",
  # UI (BOTH required!)
  "bevy_ui", "bevy_ui_render",
  # UI dependencies
  "bevy_sprite", "bevy_sprite_render", "bevy_text", "default_font",
  # Renderer initialisation (skip → purple screen / crash)
  "tonemapping_luts", "hdr", "ktx2", "zstd_rust",
  # Window / input
  "bevy_winit", "bevy_window",
  # Threading
  "multi_threaded",
  # Image format support (PNG for textures/font atlas)
  "png",
  # Web (harmless on native)
  "webgl2",
  # Mobile (only the Android one matters; iOS uses defaults)
  "android-native-activity",   # NOT android-game-activity
  "touch",
]
```

### Feature dependency chain

```
bevy_ui_render
  → bevy_sprite_render        (sprite-based UI quads)
  → bevy_ui  (ECS types)
  → bevy_sprite               (sprite types)
  → bevy_text + default_font  (text rendering)
```

### Required for renderer init

`tonemapping_luts` + `hdr` + `ktx2` + `zstd_rust` together — without
all four, the renderer fails to initialise and you get a purple screen
or a crash on every platform.

### `bevy_diagnostic` and `bevy_input` are NOT features

They're always included; trying to add them to `features` is an error.

### Android: use `android-native-activity` (not `android-game-activity`)

`cargo-apk` only supports `NativeActivity`. The other variant fails
to load at runtime.

---

## Bevy 0.18 → 0.19 API renames

| 0.18                | 0.19-dev               | Where seen                         |
|---------------------|------------------------|------------------------------------|
| `shadows_enabled`   | `shadow_maps_enabled`  | `PointLight`, `DirectionalLight`   |
| `ChildBuilder`      | `ChildSpawnerCommands` | `with_children` closure parameter  |
| `font_size: 15.0`   | `font_size: FontSize::Px(15.0)` | `TextFont` struct field   |
| `AmbientLight` resource | `GlobalAmbientLight` resource | `commands.insert_resource(...)` |
| `Event` derive      | `Message` derive       | events / messages                  |
| `EventReader`       | `MessageReader`        | system parameter                   |
| `EventWriter`       | `MessageWriter`        | system parameter                   |
| `app.add_event::<T>()` | `app.add_message::<T>()` | plugin builder                  |
| `event_reader.send(...)` | `message_writer.write(...)` | emit                          |

### `AmbientLight` is now a Component, `GlobalAmbientLight` is the resource

```rust
// 0.18: AmbientLight was inserted as a Resource
commands.insert_resource(AmbientLight { color: ..., brightness: 250.0 });

// 0.19-dev: insert GlobalAmbientLight as Resource;
//          AmbientLight is now a per-camera Component
commands.insert_resource(GlobalAmbientLight {
    color: Color::WHITE,
    brightness: 250.0,
    ..default()
});
```

Trying to `insert_resource(AmbientLight {...})` in 0.19-dev gives:
`AmbientLight is not a Resource`.

### `zstd_rust` (not `zstd`) in both 0.18.1 and 0.19-dev

Naming is the same in both — but the bare `zstd` feature does not exist.

---

## Resource vs Component renames

Several types changed between Resource ↔ Component in 0.19-dev. If
you see "T is not a Resource" or "T is not a Component" errors,
check the docs for the current shape:

- `AmbientLight` — Component (per camera). `GlobalAmbientLight` is the world-wide resource.
- `Camera3d`, `Mesh3d`, `MeshMaterial3d` — tuple-struct **Components** wrapping handles. Use `.0` to access the handle: `mesh.0 = new_handle.clone()`.
- `Text`, `TextFont`, `TextColor`, `TextLayout` — **separate** Components. Spawn them as a tuple alongside `Node`/`Button`.

---

## UI spawn patterns and types

### Both spawn styles work — pick what's clearest

Tuple spawn + `with_child` (best for leaf nodes):
```rust
commands.spawn((
    Button,
    Node { ..default() },
    BackgroundColor(Color::WHITE),
)).with_child((
    Text::new("Click me"),
    TextFont { font_size: FontSize::Px(20.0), ..default() },
    TextColor(Color::BLACK),
));
```

Chained `.spawn(...).insert(...)` + nested `with_children` (better for
deep hierarchies):
```rust
commands.spawn(Node { ..default() })
    .insert(BackgroundColor(Color::BLACK))
    .with_children(|parent| {
        parent.spawn(Text::new("Hello"))
            .insert(TextFont { font_size: FontSize::Px(15.0), ..default() })
            .insert(TextColor(Color::WHITE));
    });
```

### `with_children` closures take `&mut ChildSpawnerCommands` in 0.19-dev

When extracting a helper function:
```rust
fn spawn_button(parent: &mut ChildSpawnerCommands, label: &str) { ... }
```

### `Camera3d`/`Mesh3d`/`MeshMaterial3d` are tuple structs

```rust
// Spawn:
commands.spawn((
    Mesh3d(meshes.add(Cuboid::default())),
    MeshMaterial3d(materials.add(Color::WHITE)),
    Transform::default(),
));

// Replace at runtime via `.0`:
fn swap(mut q: Query<&mut Mesh3d>) {
    for mut m in &mut q { m.0 = new_handle.clone(); }
}
```

### MeshMaterial3d is generic over the material type

`MeshMaterial3d<StandardMaterial>` (or your custom material). Queries
need the type parameter:

```rust
mut q: Query<&mut MeshMaterial3d<StandardMaterial>, With<Player>>,
```

---

## Camera, sprites and 2D-in-3D tricks

### A `Camera3d` is enough — no `IsDefaultUiCamera` needed if there's only one camera

When you have a single `Camera3d`, UI rendering attaches to it
automatically (highest-order camera targeting the primary window
becomes the default UI camera). You only need `IsDefaultUiCamera` when
disambiguating between multiple cameras.

### 2D sprite in a 3D world — billboard via `Transform.rotation`

For a flat coloured rectangle that always faces the camera:
```rust
let mesh = meshes.add(Cuboid::new(0.8, 1.4, 0.05)); // thin slab
let material = materials.add(StandardMaterial {
    base_color: Color::srgb(0.95, 0.30, 0.20),
    unlit: true,                 // ignore lighting → consistent colour
    cull_mode: None,             // visible from both faces
    ..default()
});
```

Each frame, rotate the slab to face the camera (yaw only):
```rust
let to_cam = cam_tf.translation - tf.translation;
let yaw = to_cam.x.atan2(to_cam.z);
tf.rotation = Quat::from_rotation_y(yaw);
```

### Multi-asset face/side/back to convey orientation

Bevy 0.19-dev billboards always face the camera, so the player slab
"shows the same texture" regardless of facing. To imply orientation,
keep multiple `(Mesh3d, MeshMaterial3d)` pairs (different widths +
colours) and swap them based on `dot(facing, player_to_camera)`. See
`render/player_view.rs` for a 5-bucket implementation
(face / front-side / side / back-side / back).

---

## Platform-specific code (cfg guards)

Use `#[cfg(target_os = ...)]` rather than runtime checks — they're
zero-cost and won't accidentally compile in features for the wrong OS.

```rust
// Shadows segfault on some Android GPUs
#[cfg(not(target_os = "android"))]
shadow_maps_enabled: true,

// MSAA causes Vulkan panics on Android Mali devices
#[cfg(target_os = "android")]
Msaa::Off,

// Mobile-only window settings (rotation gesture, status-bar hide)
#[cfg(any(target_os = "android", target_os = "ios"))]
{
    Window {
        mode: WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
        recognize_rotation_gesture: true,
        prefers_home_indicator_hidden: true,
        prefers_status_bar_hidden: true,
        ..default()
    }
}
```

### `WindowMode::BorderlessFullscreen` only on mobile

On desktop, default windowed mode renders correctly; borderless
fullscreen had bugs in our setup that caused UI to be invisible.

### `WinitSettings::mobile()` only on mobile

`WinitSettings::mobile()` switches the run loop to "ReactiveLowPower"
mode — only redraws on input events (saves battery). On desktop this
freezes the FPS counter and any animation. Use `WinitSettings::game()`
on desktop:

```rust
fn winit_settings() -> WinitSettings {
    #[cfg(any(target_os = "android", target_os = "ios"))]
    { WinitSettings::mobile() }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    { WinitSettings::game() }
}
```

### Conditional imports

When `WindowMode` is only used in mobile cfg blocks, gate the import:

```rust
#[cfg(any(target_os = "android", target_os = "ios"))]
use bevy::window::WindowMode;
```

Otherwise you get an unused-import warning on Windows.

---

## Windows-specific build issues

### `windows` crate version conflict (E0277 in `wgpu-hal`)

Symptom: `cargo build` fails with errors about `ID3D12Heap` type
mismatches inside `wgpu-hal`. Cause: `gpu-allocator` (a `wgpu-hal`
dependency) is pinned to `windows v0.61` in `Cargo.lock`, but
`wgpu-hal` requires `v0.62`. Two `windows` crates in the graph =
incompatible types.

**Fix once per repository:**
```bash
cargo update windows@0.61.3 --precise 0.62.2
```

`gpu-allocator`'s semver range is `>=0.53, <=0.62`, so 0.62 satisfies
both crates. Verify there's only one `windows` version with:

```bash
cargo tree -i windows@0.62.2
```

### rust-analyzer holds the previous `.exe` open → linker fails

Symptom: after a successful run, the next `cargo build` says:

```
error: failed to remove file `target/debug/my_bevy_game_bin.exe`
Caused by: The process cannot access the file because it is being used by another process.
```

Cause: rust-analyzer keeps an open handle on the produced binary even
after the process exits.

**Fixes (in order of preference):**

1. `Ctrl+Shift+P → "rust-analyzer: Restart server"`
2. `Ctrl+Shift+P → "Developer: Reload Window"`
3. Add to workspace `Cargo.toml`:
   ```toml
   [profile.dev]
   incremental = false
   ```
   Slightly slower incremental builds, eliminates the lock entirely.

### `strip = "symbols"` in release profile breaks Android

`strip = "symbols"` strips the `android_main` export, which `cargo-apk`
needs to find. Use `strip = "debuginfo"` instead.

### Stopping a running game — VS Code task

Closing the Bevy window normally exits the process (Bevy's default
`ExitCondition::OnAllClosed`). If a task terminal still shows
"running":

- `Ctrl+C` in the task terminal — cleanest.
- `Ctrl+Shift+P → "Tasks: Terminate Task"` — kill the terminal process.
- The "Windows: Stop game" task added to `.vscode/tasks.json`
  force-kills by name.

---

## Mobile-friendly UI rules

Every overlay must work on small phone screens (target ≥ 360 × 640 px).
Same rules as in [.claude/skills/architecture/SKILL.md](../.claude/skills/architecture/SKILL.md)
rule #8, repeated here for the lessons doc:

- **Never overlap UI elements.** The settings gear (top-right) and the
  FPS HUD (top, full-width) collided in the first iteration: HUD text
  was hidden behind the gear. Fix: HUD's `right` margin must reserve
  space for the gear. Use a `gear_clearance` constant (`20 + 48 + 12 =
  80 px`) and bake it into the HUD `right` value.
- **Panels use `left`+`right` margins, not fixed `width`.** A fixed
  `Val::Px(260.0)` panel sticks out or wastes space. Pattern:
  ```rust
  Node {
      left: safe_side,
      right: safe_side,
      max_width: Val::Px(360.0),  // cap on desktop
      ...
  }
  ```
- **Touch-target sizes**: 44 × 44 px or larger for anything tappable.
  The gear is 48 × 48; the ± step buttons in the settings panel are
  28 × 28 (borderline — only used at desktop-comfortable sizes).
- **Safe-area padding constants** (defined once and reused):
  ```rust
  let safe_top    = Val::Px(48.0);   // status bar + notch
  let safe_bottom = Val::Px(66.0);   // home indicator + nav bar
  let safe_side   = Val::Px(20.0);   // minimum side margin
  ```
- **Wrap, don't overflow**: long button rows use
  `flex_wrap: FlexWrap::Wrap` with `column_gap`/`row_gap`.
- **Mental model**: when unsure, pretend the screen is 360 × 640 and
  check that every interactive element is reachable and every label
  is fully visible (not behind another widget).

## Camera smoothing pitfalls

### Frame-rate-independent exponential smoothing

Don't use `lerp(current, target, fixed_alpha)` — the speed becomes
frame-rate dependent. Use:

```rust
let alpha = 1.0 - (-k * dt).exp();  // k = rate constant per second
current = current.lerp(target, alpha);
```

`k` is "how many e-foldings per second" (higher = snappier). Same
formula for `slerp` on quaternions.

### Linear position lerp ≠ orbital arc

Lerping the camera **position** in 3D space cuts across the inside of
its orbit circle when the user pans/orbits fast. The camera visibly
"goes off centre" before snapping back.

**Fix:** smooth the **angle** (azimuth) and the **focal point**, not
the world-space camera position. Reconstruct the camera position each
frame from the smoothed angle: `cam = focus + (sin(a)·d, h, cos(a)·d)`.
Render layer just snaps to the result; smoothing already happened in
the view-model.

### Angular interpolation must take the shortest path

A naïve lerp of two angles can rotate the long way around when crossing
the ±π boundary. Take the wrapped diff:

```rust
fn lerp_angle_shortest(current: f32, target: f32, alpha: f32) -> f32 {
    use std::f32::consts::{PI, TAU};
    let mut diff = (target - current) % TAU;
    if diff > PI { diff -= TAU; }
    else if diff < -PI { diff += TAU; }
    current + diff * alpha
}
```

### `Camera::viewport_to_world` for click-to-world raycast

```rust
// Inside a system with `Query<(&Camera, &GlobalTransform)>`:
let ray = camera.viewport_to_world(cam_global_tf, cursor_pos)?;
// Intersect with horizontal plane y = GROUND_Y:
let t = (GROUND_Y - ray.origin.y) / ray.direction.y;
if t > 0.0 {
    let hit = ray.origin + *ray.direction * t;
}
```

`viewport_to_world` requires `GlobalTransform` (not `Transform`) and
returns `Result<Ray3d, ViewportConversionError>`. `ray.direction` is a
`Dir3` — deref with `*`.
