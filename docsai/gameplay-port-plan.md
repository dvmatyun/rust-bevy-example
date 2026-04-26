# Gameplay-port plan: Dart → Bevy

Plan for porting the basic gameplay subsystems from the Dart `ecs_aim`
package to our Bevy 4-layer architecture. **Not yet implemented — this
is the design; subsystems will be added one at a time.**

## Source of truth

The canonical implementations of every formula and edge case live in
the Dart codebase, summarised in
[`docsai/dart-source-reference.md`](dart-source-reference.md):

```
C:\Repositories\we_are_vikings\packages\ecs_aim\lib\src\base_ecs\
  movement\
  collisions\
  game_actions\           (damage_calculator.dart, action lifecycle)
  units\stats\            (HealthStat, ArmorStat, DamageStatDouble, …)
  units\effects\
```

Re-read those files for the exact algorithm before implementing each
subsystem.

## Cross-cutting principles

These two override any literal translation of Dart structure:

1. **No magic IDs.** Every Dart `propertyId` becomes a Rust **type**.
   - `propertyId == HP` → component called `Health`.
   - `EffectPropertiesEcs.foodGenerationDt` → component called
     `FoodGenerationRate`.
   - Status effects (`flags & STUN`, `flags & FREEZE`, …) → marker
     components `Stunned`, `Frozen`, `Feared`. Movement / action
     systems then filter `Without<Stunned>` for free.
2. **No packed `Int32List`s.** Every Dart packed-array struct becomes a
   normal Rust struct; `serde + bincode` (already wired into the
   replication pipeline) produces the wire format. Cleaner code,
   smaller wire size (varint), no positional bugs.

## Replication wiring (already in place)

For every component marked **Replicate? ✓** in the tables below:

1. derive `Component + Serialize + Deserialize`,
2. add a `TYPE_ID` constant in `data/replication.rs::type_ids`,
3. `impl Replicated for X { const TYPE_ID = type_ids::X; }`,
4. add `ServerReplicatePlugin::<X>::default()` to `server/mod.rs`.

Server-only components (`Pushed`, `WalkSteps`, `SpatialIndex`,
`HasCollided`, etc.) do **not** opt in.

---

## 1. Movement (extend existing)

Today: `MoveIntent → Transform`. Dart: `MoveIntent → Velocity →
(acceleration ramp) → Transform`, plus separate knockback. Plan:

| New / changed | Layer | Replicate? | Notes |
|---|---|:---:|---|
| `Speed(f32)` | `data/components.rs` | ✓ | base movespeed, mutable by buffs |
| `Velocity(Vec2)` | `data/components.rs` | ✓ | XZ unit vector × current speed |
| `WalkSteps(u8)` | `data/components.rs` | server-only | Dart's `stepsCount`; 8-frame quadratic accel ramp |
| `Pushed { remaining_secs, total: Vec2, applied: Vec2, stop_on_collision: bool }` | `data/components.rs` | server-only | Dart's `PushOverTimeComponent` |
| `KnockbackEvent { entity, total, secs, stop_on_collision }` | `data/intents.rs` | message | emitted by collisions |
| `apply_move_intent` system | `server/movement.rs` |  | reads `MoveIntent`, sets `Velocity = direction * Speed`, updates `Facing` |
| `tick_walk_steps` system | `server/movement.rs` |  | accel ramp; skip `With<ActionInProgress>` |
| `apply_velocity` system | `server/movement.rs` |  | `Transform.translation += Velocity * dt * accel(WalkSteps)` |
| `apply_pushes` system | `server/movement.rs` |  | per-frame slice from `Pushed`; decrements timer |
| `apply_knockback_events` system | `server/movement.rs` |  | consumes `KnockbackEvent`, inserts/updates `Pushed` |

Order in `AppSet::GameLogic` (chained):

```
apply_move_intent
  → apply_knockback_events
  → tick_walk_steps
  → apply_velocity
  → apply_pushes
  → snap_to_ground          (existing)
```

Files: `data/{components.rs, intents.rs, replication.rs}`,
`server/{player.rs → split into movement.rs}`, `server/mod.rs`.

---

## 2. Damage / Health (new)

Direct port of `damage_calculator.dart` formulas. Use **one component
per stat** instead of a giant `UnitStats` container.

| Component | Purpose | Replicate? |
|---|---|:---:|
| `Health { current: i32, max: i32 }` | HP. clients render bars | ✓ |
| `Damage { min: f32, max: f32 }` | attack damage range | ✓ |
| `Armor(i32)` | % reduction via `100 / (100 + armor)` | ✓ |
| `Defense(i32)` | flat reduction | ✓ |
| `ArmorPenetration(f32)` | % of target armor ignored | ✓ |
| `DefensePenetration(i32)` | flat ignored | ✓ |
| `HitRate(f32)`, `FleeRate(f32)` | hit/dodge roll | ✓ |
| `Faction(u32)` | friend/foe filter | ✓ |
| `Dead` (marker) | replaces Dart `isDead` flag | ✓ |

**Status effects as marker components** (replaces
`UnitBoolEffectsStat` bitflags):

```rust
#[derive(Component, Serialize, Deserialize)] struct Stunned   { until_secs: f32 }
#[derive(Component, Serialize, Deserialize)] struct Frozen    { until_secs: f32 }
#[derive(Component, Serialize, Deserialize)] struct Feared    { until_secs: f32 }
#[derive(Component, Serialize, Deserialize)] struct Silenced  { until_secs: f32 }
```

Movement / action systems use `Without<Stunned>, Without<Frozen>` —
no bit-checking, compile-time enforced.

**Damage events:**

```rust
// data/intents.rs
#[derive(Message, Serialize, Deserialize, Clone)]
pub struct DamageEvent {
    pub source: Entity,
    pub target: Entity,
    pub kind: DamageKind,           // Physical, Magic, True, …
    pub raw_amount: Option<i32>,    // None → roll from attacker's Damage
    pub roll_hit: Option<bool>,     // populated by roll_hit_chance,
                                    // shared by all consumers (replaces
                                    // Dart's GameContext memoisation)
}

#[derive(Message)]
pub struct DeathEvent { pub entity: Entity }
```

**Server systems** in `server/damage.rs`:

| System | Order | Reads | Writes |
|---|---|---|---|
| `roll_hit_chance` | early | `DamageEvent`, attacker `HitRate`, target `FleeRate`, `Random` | tags event with `roll_hit = Some(...)` |
| `apply_damage` | after | tagged `DamageEvent`, attacker `{Damage, ArmorPen, DefensePen}`, target `{Armor, Defense, Health}`, `Random` | `Health.current` |
| `detect_death` | after | `Changed<Health>` | inserts `Dead` marker, emits `DeathEvent` |

Bevy's `MessageReader` semantics give "every consumer sees the same
event" — push and on-hit effects all read the same hit roll without a
`GameContext` cache.

**RNG:** `Random` resource in `data/`, wrapping
`rand_chacha::ChaCha8Rng` for determinism in headless tests.

Files: `data/{stats.rs, damage_events.rs, replication.rs}`,
`server/damage.rs`, `server/mod.rs`.

---

## 3. Collisions (new)

Port the Dart broad-phase grid + AABB/SAT narrow-phase + push-blend
resolution. ~300 lines of geometry; no full physics engine needed.

| Component | Purpose | Replicate? |
|---|---|:---:|
| `Hitbox { half_extents: Vec2, offset: Vec2 }` | AABB (position from `Transform`) | ✓ (clients can debug-draw) |
| `Faction(u32)` | reused from damage | ✓ |
| `CollisionLayer(u32)`, `CollisionMask(u32)` | bitmask filter (Dart's `isEnemyFractionCollision`) | ✓ |
| `Collider { allow_displacement, has_push_effect, can_be_displaced, remove_on_first_collision }` | typed Dart `HasCollisionInteractionFlag` | ✓ |
| `OnHitAction(Option<ActionId>)` | which action fires on collision | ✓ |
| `CollidesOnce` (marker) | Dart's `collidesOneTime` | server-only |
| `HasCollided` (marker, frame-scoped) | Dart's `hasCollided` flag | server-only |

**Resources:** `SpatialIndex` (uniform grid; quadtree is overkill for
our world). Server-only.

**Events:**

```rust
// data/intents.rs (or data/collision_events.rs)
#[derive(Message, Clone)]
pub struct CollisionEvent {
    pub a: Entity,
    pub b: Entity,
    pub normal: Vec2,   // unit vector from b → a
    pub depth: f32,     // penetration
}
```

**Server systems** in `server/collision.rs` (`AppSet::GameLogic`,
after movement integration):

| System | Order | Notes |
|---|---|---|
| `update_spatial_index` | first | `Changed<Transform>` for `Hitbox` entities → re-bucket |
| `detect_collisions` | next | broad (grid query) → narrow (AABB + faction/mask) → emit `CollisionEvent` |
| `clear_frame_flags` | last | reset `HasCollided` markers |

**Resolution** — three independent consumers of `CollisionEvent`:

1. **`apply_collision_push`** (`server/collision.rs`): emits
   `KnockbackEvent` (consumed by movement). Dart blend:
   - `charge_factor = max(0, move_dir · normal)`
   - `brace = 1 - charge_factor * 0.7`
   - `push_strength = min(depth * 2 + 4, 32) * brace`
2. **`trigger_on_hit_actions`** (`server/actions.rs`): if collider has
   `OnHitAction(Some(id))` → emit `ActionStartEvent`.
3. **`apply_collide_once_removal`**: entities with `CollidesOnce +
   remove_on_first_collision` get a `DespawnScheduled` marker.

Each resolver runs from the same `CollisionEvent` queue.

Files: `data/{collision.rs, replication.rs}`, `server/collision.rs`,
`server/spatial_index.rs`, `server/mod.rs`.

---

## Suggested implementation order

1. **Movement upgrade** — smallest delta; extends what we have.
   `Velocity + Speed + WalkSteps` accel ramp + `Pushed` /
   `KnockbackEvent`.
2. **Health + Damage events pipeline** (no targets/attacks yet).
   Hand-fire a `DamageEvent` on a key press to validate the chain.
3. **Collision detection only** (no resolvers). Watch
   `CollisionEvent`s appear in replication logs when two test cubes
   overlap.
4. **Wire collisions → push** (`KnockbackEvent` into movement). Cubes
   bump and slide.
5. **Wire collisions → damage** for "attack collider"-tagged entities.
6. **Status effects** (`Stunned`, `Frozen`, `Burning(damage_per_sec)`,
   …). Each gets its own component + ticker system. Movement / action
   filters consume them via `Without<...>`.
7. **Action system** (start / in-progress / completed Messages,
   cooldown component, resolver). Mirrors Dart 1:1 with marker
   components instead of integer property IDs.

## Anti-patterns (from Dart) to avoid in the port

Recap of [`docsai/dart-source-reference.md`](dart-source-reference.md)
"Anti-patterns" section, applied to gameplay porting:

- ❌ packed `Int32List`-with-positional-fields → ✅ normal Rust structs
  + serde
- ❌ integer `propertyId` runtime dispatch → ✅ per-stat newtype
  components (`Health`, `Armor`, …) and trait dispatch
- ❌ status-flag bitfields (`isStun`, `isFreeze`, …) → ✅ marker
  components + `With<...>` / `Without<...>` query filters
- ❌ manual `reportComponentToClient(...)` calls → ✅
  `Replicate` marker + `ServerReplicatePlugin::<T>` (already wired)
- ❌ `GameContext` memoisation cache → ✅ store one-shot rolls on the
  triggering `Message` so all consumers read the same value
- ❌ resolver mixins → ✅ trait + per-type plugin pattern (same shape
  as `ServerReplicatePlugin<T>`)
