# Bevy 0.19-dev UI Overlay — Lessons Learned

## The Critical Feature: `bevy_ui_render`

In Bevy 0.19-dev, UI rendering is split into two separate crates/features:
- **`bevy_ui`** — provides ECS types: `Node`, `Button`, `Text`, `BackgroundColor`, `Interaction`, etc.
- **`bevy_ui_render`** — actually **draws** UI nodes on screen. Depends on `bevy_sprite_render`.

**Without `bevy_ui_render`, UI nodes exist in the ECS but are completely invisible.** No errors, no warnings — just silent failure. This is the #1 gotcha when using `default-features = false`.

### Minimum feature set for UI overlay on 3D scene (Bevy 0.19-dev)

```toml
[dependencies.bevy]
path = "C:/Repositories/Rust/bevy"
default-features = false
features = [
  # 3D rendering
  "bevy_pbr",
  "bevy_core_pipeline",
  "bevy_render",
  "bevy_camera",
  "bevy_light",
  "bevy_mesh",
  "bevy_image",
  "bevy_shader",
  # UI (both required!)
  "bevy_ui",
  "bevy_ui_render",
  # UI dependencies
  "bevy_sprite",
  "bevy_sprite_render",
  "bevy_text",
  "default_font",
  # ... other features as needed
]
```

### Feature dependency chain
```
bevy_ui_render
  -> bevy_sprite_render (renders sprite-based UI quads)
  -> bevy_ui (ECS types)
  -> bevy_sprite (sprite types)
  -> bevy_text (text layout)
```

## WinitSettings: Desktop vs Mobile

`WinitSettings::mobile()` suppresses continuous rendering — it only redraws on user input. This means:
- On desktop: UI appears frozen, animations don't play, FPS counter never updates
- On mobile: saves battery, but user must touch screen to trigger first render

**Solution:** Use `WinitSettings::game()` on desktop, `WinitSettings::mobile()` only on actual mobile:
```rust
fn winit_settings() -> WinitSettings {
    #[cfg(any(target_os = "android", target_os = "ios"))]
    { WinitSettings::mobile() }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    { WinitSettings::game() }
}
```

## Window Mode: Desktop vs Mobile

`WindowMode::BorderlessFullscreen` should only be used on mobile. On desktop, use the default windowed mode:
```rust
fn window_settings() -> Window {
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        Window {
            resizable: false,
            mode: WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
            recognize_rotation_gesture: true,
            prefers_home_indicator_hidden: true,
            prefers_status_bar_hidden: true,
            ..default()
        }
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        Window {
            title: "My Bevy Game".to_string(),
            ..default()
        }
    }
}
```

## UI Spawn Patterns (Bevy 0.19-dev)

Both patterns work in 0.19-dev:

### Tuple spawn + `with_child` (preferred for leaf nodes)
```rust
commands
    .spawn((
        Button,
        Node { justify_content: JustifyContent::Center, ..default() },
        BackgroundColor(Color::WHITE),
    ))
    .with_child((
        Text::new("Click me"),
        TextFont { font_size: FontSize::Px(20.0), ..default() },
        TextColor(Color::BLACK),
    ));
```

### `spawn().insert()` + `with_children` (works for nested layouts)
```rust
commands.spawn(Node { ..default() })
    .insert(BackgroundColor(Color::BLACK))
    .with_children(|parent| {
        parent.spawn(Text::new("Hello"))
            .insert(TextFont { font_size: FontSize::Px(15.0), ..default() })
            .insert(TextColor(Color::WHITE));
    });
```

### `ChildSpawnerCommands` type
In 0.19-dev, the `with_children` closure receives `&mut ChildSpawnerCommands` (was `ChildBuilder` in older versions). Use this type when extracting helper functions:
```rust
fn spawn_button(parent: &mut ChildSpawnerCommands, label: &str) { ... }
```

## `FontSize::Px()` wrapper
Bevy 0.19-dev requires `FontSize::Px(30.0)` for `TextFont.font_size`. Plain `f32` like `30.0` no longer compiles (was valid in 0.18.1).

## `shadows_enabled` renamed to `shadow_maps_enabled`
In Bevy 0.19-dev, `PointLight.shadows_enabled` was renamed to `shadow_maps_enabled`.

## `windows` crate version conflict
If `cargo check` fails with `wgpu-hal` errors about `ID3D12Heap` type mismatches, it's a lockfile issue: `gpu-allocator` gets pinned to `windows v0.61` while `wgpu-hal` needs `v0.62`. Fix:
```bash
cargo update windows@0.61.3 --precise 0.62.2
```
This works because `gpu-allocator`'s semver range is `>=0.53, <=0.62`.
