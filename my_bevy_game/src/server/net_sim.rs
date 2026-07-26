//! In-memory network-delay simulation for `PlayingMultiplayer`.
//!
//! Two simulated legs, each with a random one-way delay of 100 ms ±20 ms:
//!
//!   Client → Server (input leg)
//!     `BufferedMoveIntent`s are queued in `NetSimBuffer`.  After the
//!     deadline passes, `flush_net_sim` writes `MoveIntent` so
//!     `server::player::apply_movement` sees it in `AppSet::GameLogic`.
//!
//!   Server → Client (position leg)
//!     After `GameLogic` each player's authoritative `Transform` is
//!     snapshotted into `PosSimBuffer`.  After the deadline passes,
//!     `flush_positions` copies the snapshot into `ClientTransform`
//!     on that entity.  The camera and billboard systems read
//!     `ClientTransform` so the visible position lags by 100 ms,
//!     giving a full 200 ms round-trip.
//!
//! In `PlayingSingle`, `sync_client_transform` immediately mirrors
//! `Transform → ClientTransform` so single-player feels instant.
//!
//! Camera-orbit intents are NOT buffered — they are local view
//! operations that should feel instant regardless of ping.

use std::collections::VecDeque;

use bevy::prelude::*;

use crate::data::{AppSet, BufferedMoveIntent, ClientTransform, GameState, MoveIntent, Player};

// One-way simulated latency: base ± half-spread (ms).
const BASE_MS: f64 = 100.0;
const SPREAD_MS: f64 = 20.0;

// ── Input-leg buffer ─────────────────────────────────────────────────────────

#[derive(Debug)]
struct PendingIntent {
    release_at: f64,
    intent: BufferedMoveIntent,
}

#[derive(Resource, Default)]
pub(crate) struct NetSimBuffer {
    queue: VecDeque<PendingIntent>,
    rng: u32,
}

impl NetSimBuffer {
    fn next_delay_secs(&mut self) -> f64 {
        self.rng = pcg_step(self.rng);
        let frac = (self.rng as f64) / (u32::MAX as f64 + 1.0);
        let ms = BASE_MS + (frac * 2.0 - 1.0) * SPREAD_MS;
        ms / 1000.0
    }
}

// ── Position-leg buffer ──────────────────────────────────────────────────────

#[derive(Debug)]
struct PendingPos {
    release_at: f64,
    entity: Entity,
    transform: Transform,
}

#[derive(Resource, Default)]
pub(crate) struct PosSimBuffer {
    queue: VecDeque<PendingPos>,
    rng: u32,
}

impl PosSimBuffer {
    fn next_delay_secs(&mut self) -> f64 {
        self.rng = pcg_step(self.rng);
        let frac = (self.rng as f64) / (u32::MAX as f64 + 1.0);
        let ms = BASE_MS + (frac * 2.0 - 1.0) * SPREAD_MS;
        ms / 1000.0
    }
}

fn pcg_step(state: u32) -> u32 {
    let s = state.wrapping_mul(747_796_405).wrapping_add(2_891_336_453);
    let word = ((s >> ((s >> 28).wrapping_add(4))) ^ s).wrapping_mul(277_803_737);
    (word >> 22) ^ word
}

// ── Plugin ───────────────────────────────────────────────────────────────────

pub struct NetSimPlugin;

impl Plugin for NetSimPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(NetSimBuffer {
            rng: 0xDEAD_CAFE,
            ..default()
        })
        .insert_resource(PosSimBuffer {
            rng: 0xBEEF_F00D,
            ..default()
        })
        // Input leg: runs at the start of Input so MoveIntents arrive in GameLogic.
        .add_systems(
            Update,
            (enqueue_buffered, flush_net_sim)
                .chain()
                .in_set(AppSet::Input)
                .run_if(in_state(GameState::PlayingMultiplayer)),
        )
        // Position leg (multiplayer): snapshot transforms after GameLogic,
        // flush into ClientTransform before ViewModel reads them.
        .add_systems(
            Update,
            (enqueue_positions, flush_positions)
                .chain()
                .after(AppSet::GameLogic)
                .before(AppSet::ViewModel)
                .run_if(in_state(GameState::PlayingMultiplayer)),
        )
        // Position leg (single-player): immediately mirror Transform → ClientTransform.
        .add_systems(
            Update,
            sync_client_transform
                .after(AppSet::GameLogic)
                .before(AppSet::ViewModel)
                .run_if(in_state(GameState::PlayingSingle)),
        );
    }
}

// ── Input-leg systems ─────────────────────────────────────────────────────────

fn enqueue_buffered(
    time: Res<Time>,
    mut events: MessageReader<BufferedMoveIntent>,
    mut buf: ResMut<NetSimBuffer>,
) {
    let now = time.elapsed_secs_f64();
    for e in events.read() {
        let delay = buf.next_delay_secs();
        buf.queue.push_back(PendingIntent {
            release_at: now + delay,
            intent: *e,
        });
    }
}

fn flush_net_sim(
    time: Res<Time>,
    mut buf: ResMut<NetSimBuffer>,
    mut writer: MessageWriter<MoveIntent>,
) {
    let now = time.elapsed_secs_f64();
    while let Some(front) = buf.queue.front() {
        if front.release_at > now {
            break;
        }
        let p = buf.queue.pop_front().unwrap();
        writer.write(MoveIntent {
            direction: p.intent.direction,
            player_slot: p.intent.player_slot,
        });
    }
}

// ── Position-leg systems ──────────────────────────────────────────────────────

/// After GameLogic, snapshot every player's current `Transform` into
/// the position-delay queue with a fresh 100 ms ±20 ms deadline.
pub(crate) fn enqueue_positions(
    time: Res<Time>,
    players: Query<(Entity, &Transform), With<Player>>,
    mut buf: ResMut<PosSimBuffer>,
) {
    let now = time.elapsed_secs_f64();
    for (entity, tf) in &players {
        let delay = buf.next_delay_secs();
        buf.queue.push_back(PendingPos {
            release_at: now + delay,
            entity,
            transform: *tf,
        });
    }
}

/// Drain position snapshots whose deadline has passed and write them
/// into `ClientTransform` so the camera / billboard see the delayed position.
pub(crate) fn flush_positions(
    time: Res<Time>,
    mut buf: ResMut<PosSimBuffer>,
    mut players: Query<&mut ClientTransform, With<Player>>,
) {
    let now = time.elapsed_secs_f64();
    // Drain only entries that are ready; the queue is approximately
    // time-ordered so we break on the first not-yet-ready entry.
    let mut i = 0;
    while i < buf.queue.len() {
        if buf.queue[i].release_at > now {
            i += 1;
            continue;
        }
        let p = buf.queue.remove(i).unwrap();
        if let Ok(mut ct) = players.get_mut(p.entity) {
            ct.0 = p.transform;
        }
        // Don't increment i; the next element shifted into position i.
    }
}

/// Single-player passthrough: immediately mirror `Transform → ClientTransform`
/// so there is zero added latency in solo play.
pub(crate) fn sync_client_transform(
    mut players: Query<(&Transform, &mut ClientTransform), With<Player>>,
) {
    for (tf, mut ct) in &mut players {
        ct.0 = *tf;
    }
}
