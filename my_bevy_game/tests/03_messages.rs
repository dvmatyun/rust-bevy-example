//! Stage 3: Messages (events).
//!
//! Messages are fire-and-forget signals. Multiple readers see them.
//! In Bevy 0.19-dev: `#[derive(Message)]`, `MessageReader<T>`,
//! `MessageWriter<T>`, and `app.add_message::<T>()`.

use bevy::prelude::*;

// =====================================================================
// TASK 3a — define a message and the systems that produce / consume it.
//
// 1. The `Damage` message is provided below for you. Note the
//    `#[derive(Message)]` — that's the magic.
// 2. Implement `emit_damage` to write `Damage(7)` once per frame.
// 3. Implement `total_damage` to add every received damage value into
//    a `TotalDamage` resource.
//
// HINTS:
//   - Writer:  `mut writer: MessageWriter<Damage>; writer.write(Damage(7));`
//   - Reader:  `mut reader: MessageReader<Damage>; for d in reader.read() { … }`
//
// READ:
//   - `src/data/intents.rs` — see `MoveIntent` for a tiny example.
//   - `src/server/player.rs::apply_movement` — sums all incoming
//     MoveIntents in one frame, useful pattern.
// =====================================================================

#[derive(Message, Clone, Copy)]
pub struct Damage(pub u32);

#[derive(Resource, Default)]
struct TotalDamage(pub u32);

fn emit_damage(mut _writer: MessageWriter<Damage>) {
    // TODO: write Damage(7)
}

fn total_damage(mut _reader: MessageReader<Damage>, mut _total: ResMut<TotalDamage>) {
    // TODO: for each event, _total.0 += d.0
}

#[test]
fn task_3a_damage_accumulates() {
    let mut app = App::new();
    app.add_message::<Damage>()
        .init_resource::<TotalDamage>()
        .add_systems(Update, (emit_damage, total_damage).chain());

    // Three ticks should give us 3 × 7 = 21.
    app.update();
    app.update();
    app.update();

    let t = app.world().resource::<TotalDamage>().0;
    assert_eq!(t, 21, "expected 21 (=3*7); got {t}");
}

// =====================================================================
// TASK 3b — multiple readers see the same messages.
//
// Add a SECOND reader system, `audit_damage`, that records the total
// in a separate resource `AuditedTotal`. Verify both totals match.
//
// This proves: a message isn't "consumed" by the first reader; each
// reader has its own cursor over the buffer.
// =====================================================================

#[test]
fn task_3b_two_readers() {
    // TODO
}
