# Dart / Flutter source-of-truth reference

The Dart game **We Are Vikings** is the original source of the gameplay
logic we're porting to Bevy. When implementing or reviewing migrated
systems, consult these files for the canonical algorithm, formulas,
and edge-case handling.

## Repository

`C:\Repositories\we_are_vikings\`

The relevant subtree is:

```
C:\Repositories\we_are_vikings\packages\ecs_aim\lib\src\base_ecs\
├── movement\               # MoveComponent, velocity / displacement / push systems
├── collisions\             # HitboxComponent, AABB/SAT, quadtree, push resolution
├── game_actions\           # action lifecycle (start/in-progress/completed), damage calc
│   ├── damage_calculator.dart       ← damage / dodge formulas
│   ├── formular\game_context.dart   ← memoised per-action context
│   └── mixins\                      ← per-effect resolvers (impact, push, award)
├── units\
│   ├── components\unit_component.dart   # composite stat container
│   ├── stats\                           # HealthStat, ArmorStat, DamageStatDouble, …
│   └── effects\                         # EffectsContainerComponent, EffectsSystem
├── animations_server\
├── input_events\
├── player_data\
└── spawn\
```

Plus the network-payload helpers (`PayloadAim`, `IComponentEcsSerializable`)
which we **do not port** — Bevy's change detection + our auto-replication
plugin replace them.

## How to read it for porting

For each subsystem we port, the survey at
[`docsai/architecture.md` → "Network replication"](architecture.md) and
the architecture skill explain the destination. The Dart files give
the source-of-truth algorithms, including:

- exact damage formula (armour vs. defence, hit/dodge rolls, minimum
  damage clamp)
- collision push blend (charge factor, brace, depth-based strength)
- effect ordering (flat → percent → flag → special)
- action lifecycle phases and what each phase consumes / produces

## Anti-patterns to **avoid** when porting

The Dart code is shaped by the Dart runtime (no zero-cost generics,
GC pressure on per-component allocations). Several patterns are
work-arounds that **don't translate**:

1. **Int32List-packed components.** Dart packs many fields into a
   single typed array indexed by position constants (e.g. status-effect
   property at index 0, value at index 2). In Rust use **strongly-typed
   structs / enums**; serde does the layout for the wire format.

2. **Magic property IDs / type IDs at runtime.** Dart dispatches on
   integer property IDs (`EffectPropertiesEcs.foodGenerationDt`,
   `propertyId == HP`). In Rust use **per-stat newtype components**
   (`Health`, `Armor`, `MoveSpeed`) and let the type system dispatch.
   Status-effect kinds become **marker components** (`Stunned`,
   `Frozen`) so movement / action systems filter with
   `Without<Stunned>` instead of bit-checking flags.

3. **Manual `reportComponentToClient(...)` calls.** Replaced entirely
   by `Replicate` marker + `ServerReplicatePlugin::<T>` — change
   detection drives the wire.

4. **`GameContext` memoisation cache** (one dodge roll per action).
   In Bevy: each action is one `Message`. Multiple consumers reading
   the same `Message` get the same data — use that instead of a
   side-channel cache. If a roll genuinely needs to be cached, store
   the result on the action `Message` itself.

5. **Mixins for per-effect resolvers.** Use a normal trait + per-type
   plugin pattern (same shape as `ServerReplicatePlugin<T>`).
