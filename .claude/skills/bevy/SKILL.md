---
name: bevy
description: Use this skill when writing or reviewing any Bevy ECS code — components, systems, queries, plugins, states, events, assets, rendering, scheduling, or performance. Covers Bevy 0.19-dev idioms used in this project.
---

# Bevy ECS Engineering Standards

Apply these rules whenever writing or reviewing Bevy code in `my_bevy_game/`.
This project uses **Bevy 0.19-dev** from a local path — always prefer 0.19 APIs.
Cross-reference the 4-layer architecture skill when placing new code in a layer.

---

## ECS Fundamentals

### Components = pure data, zero logic
```rust
#[derive(Component)]
struct Health { current: f32, max: f32 }

#[derive(Component)]
struct Velocity(Vec3);
```
- One concern per component. Avoid monolithic components — they waste memory across archetypes.
- Use marker (ZST) components for entity classification: `#[derive(Component)] struct Enemy;`
- `Bundle` groups components for atomic spawning; never spawn partial entities that require follow-up inserts.

### Systems = pure logic, operate on queries
```rust
fn movement(mut query: Query<(&mut Transform, &Velocity)>, time: Res<Time>) {
    for (mut transform, vel) in &mut query {
        transform.translation += vel.0 * time.delta_secs();
    }
}
```
- Keep systems single-purpose and short. If a system does two things, split it.
- Never store logic in components. Never store render types in server/client_sim layers.

### Resources = global singleton state
```rust
#[derive(Resource)]
struct Score(u32);
```
- Use `Res<T>` (read-only) wherever possible — allows parallel system execution.
- `ResMut<T>` blocks parallelisation; use it only when mutation is required.
- Never use `RefCell` or interior mutability inside resources.

---

## Query Best Practices

### Filter aggressively
```rust
// Good — only iterates enemies that are alive and just took damage
Query<(&mut Health, &Transform), (With<Enemy>, Without<Dead>, Changed<Health>)>
```

Useful filters:
| Filter | When to use |
|---|---|
| `With<T>` | Entity must have T (free, archetype-level) |
| `Without<T>` | Entity must lack T |
| `Changed<T>` | T was mutated this frame |
| `Added<T>` | T was inserted this frame |

### Avoid mutable conflicts — use `ParamSet`
```rust
fn resolve_damage(
    mut params: ParamSet<(
        Query<&mut Health, With<Player>>,
        Query<&mut Health, With<Enemy>>,
    )>,
) { ... }
```

### `get` / `get_mut` instead of `unwrap`
```rust
if let Ok(mut hp) = query.get_mut(entity) { hp.current -= damage; }
```

### Cache `QueryState` for hot paths
Pre-build `QueryState` in `Local<T>` resources when a query is constructed every frame but the result set is small.

---

## System Scheduling

### Canonical ordering in this project
```
input → MoveIntent (client_sim)
      → Transform mutation (server)
      → DesiredCameraView (client_sim)
      → Camera Transform (render)
```

### Ordering APIs
```rust
app.add_systems(Update, (
    handle_input,
    apply_movement.after(handle_input),
    update_camera.after(apply_movement),
));
```

### System sets for multi-system ordering
```rust
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
enum GameSet { Input, Logic, View }

app.configure_sets(Update, (GameSet::Input, GameSet::Logic, GameSet::View).chain());
```

### Run conditions — skip work cheaply
```rust
app.add_systems(Update, expensive_ai.run_if(in_state(GameState::Playing)));
app.add_systems(Update, react_to_death.run_if(resource_changed::<EnemyCount>()));
```

### Fixed timestep for physics / game logic
```rust
app.add_systems(FixedUpdate, physics_step);
```

---

## Change Detection

Prefer reactive patterns over polling:
```rust
fn on_health_changed(query: Query<(Entity, &Health), Changed<Health>>) { ... }
fn on_enemy_spawned(query: Query<Entity, Added<Enemy>>) { ... }
```

`RemovedComponents<T>` for cleanup:
```rust
fn cleanup_dead(mut removed: RemovedComponents<Health>, mut commands: Commands) {
    for entity in removed.read() { commands.entity(entity).despawn(); }
}
```

---

## Events

Use events for cross-system communication — never query another system's output directly.
```rust
#[derive(Event)]
struct DamageEvent { target: Entity, amount: f32 }

// sender
fn attack(mut events: EventWriter<DamageEvent>) {
    events.send(DamageEvent { target, amount: 10.0 });
}

// receiver
fn apply_damage(mut events: EventReader<DamageEvent>, mut query: Query<&mut Health>) {
    for ev in events.read() {
        if let Ok(mut hp) = query.get_mut(ev.target) { hp.current -= ev.amount; }
    }
}
```

In Bevy 0.19-dev, events use the **observer pattern** — prefer `Trigger<E>` observers for entity-targeted events when the receiver is always one entity.

---

## Plugin Architecture

Group related systems, resources, and components into a plugin per feature:
```rust
pub struct CombatPlugin;
impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<DamageEvent>()
           .init_resource::<CombatStats>()
           .add_systems(Update, (attack, apply_damage.after(attack)));
    }
}
```

- One plugin per file is a reasonable default.
- Plugins are the unit of discoverability — name them after features, not technical categories.

---

## States & Game Flow

```rust
#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
enum GameState { #[default] Loading, Playing, Paused, GameOver }

app.init_state::<GameState>();

// Spawn on enter, despawn on exit
app.add_systems(OnEnter(GameState::Playing), spawn_level);
app.add_systems(OnExit(GameState::Playing), despawn_level);
```

---

## Asset Loading

```rust
#[derive(Resource)]
struct GameAssets { player_mesh: Handle<Mesh>, font: Handle<Font> }

fn load_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(GameAssets {
        player_mesh: asset_server.load("models/player.glb"),
        font: asset_server.load("fonts/main.ttf"),
    });
}

// Gate play on load completion
fn check_loaded(
    assets: Res<GameAssets>,
    meshes: Res<Assets<Mesh>>,
    mut next: ResMut<NextState<GameState>>,
) {
    if meshes.contains(&assets.player_mesh) {
        next.set(GameState::Playing);
    }
}
```

---

## Rendering (render layer only)

- `Mesh3d` + `MeshMaterial3d<StandardMaterial>` — 0.19-dev API for 3D.
- `bevy_ui_render` feature is **mandatory** for visible UI — `bevy_ui` alone renders nothing.
- Use `FontSize::Px(30.0)` — plain `f32` was removed in 0.19-dev.
- `GlobalAmbientLight` — renamed from `AmbientLight` in 0.19-dev.
- `shadow_maps_enabled` — renamed from `shadows_enabled` in 0.19-dev.
- Never put `Mesh3d`, `Camera3d`, `Color`, `Node`, or any render type in `server/` or `client_sim/`.

---

## Entity Hierarchies

```rust
// Spawn parent with children
commands.spawn(ParentBundle::default()).with_children(|parent| {
    parent.spawn(ChildBundle::default());
});

// Walk hierarchy
fn propagate(
    parents: Query<(&Transform, &Children)>,
    mut children: Query<&mut Transform, Without<Children>>,
) { ... }
```

Use `ChildSpawnerCommands` — `ChildBuilder` was renamed in 0.19-dev.

---

## Performance

- **Archetype fragmentation**: adding/removing components frequently breaks cache locality. Prefer flag components or `Option<T>` over repeated add/remove.
- **`SparseSet` storage**: use `#[component(storage = "SparseSet")]` for components that are added/removed constantly (e.g., status effects). Use default table storage for stable components.
- **Parallel iteration**: `query.par_iter_mut()` for large entity counts when inner work is independent.
- **Avoid `.clone()` on queries** — query items are already references.
- Profile with `bevy_diagnostic` (`FrameTimeDiagnosticsPlugin`, `LogDiagnosticsPlugin`) before optimising.

---

## Anti-Patterns

| Pattern | Problem | Fix |
|---|---|---|
| Logic in components | Breaks ECS data/logic split | Move to systems |
| Querying all entities | Iterates unnecessary archetypes | Add `With`/`Without` filters |
| `unwrap()` on `query.get()` | Panics when entity is missing | Use `if let Ok(...)` |
| Mutable resource everywhere | Prevents parallelism | Prefer `Res<T>` |
| `RefCell` / `Mutex` in components | Defeats ECS ownership | Restructure data |
| Render types in server/client_sim | Breaks headless compilation | Keep in render layer |
| `cargo clean` mid-session | Destroys incremental cache | Only clean when linker errors |

---

## Build Tips (this project)

- Dev: `cargo run --bin my_bevy_game_bin` (Windows)
- Web: `cd my_bevy_game && trunk serve`
- Android: `cargo apk build -p my_bevy_game --lib` (always `--lib`)
- Feature `bevy/dynamic_linking` not used here (local path dep); incremental is already fast.
- `bevy_ui_render` + `bevy_sprite_render` + `tonemapping_luts` + `hdr` + `ktx2` + `zstd_rust` are all required — removing any causes crashes or invisible UI.
