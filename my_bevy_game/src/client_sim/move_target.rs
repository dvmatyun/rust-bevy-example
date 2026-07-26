//! Sticky click-to-move target (single-player only).
//!
//! When the user clicks/taps, `render::handle_click_input` raycasts and
//! emits `ClickMoveIntent`. We store the latest target XZ in
//! `MoveTarget`. Each frame `auto_move_to_target` emits a `MoveIntent`
//! toward the target until the player is close enough (then the target
//! clears).
//!
//! Both systems are gated to `GameState::PlayingSingle` in
//! `ClientSimPlugin`.

use bevy::prelude::*;

use crate::data::{ClickMoveIntent, MoveIntent, MoveTarget, Player, PlayerSlot};

const ARRIVAL_RADIUS: f32 = 0.4;

pub fn apply_click_target(
    mut events: MessageReader<ClickMoveIntent>,
    mut target: ResMut<MoveTarget>,
) {
    for e in events.read() {
        target.0 = Some(e.target);
    }
}

pub fn auto_move_to_target(
    players: Query<(&Transform, &PlayerSlot), With<Player>>,
    mut target: ResMut<MoveTarget>,
    mut writer: MessageWriter<MoveIntent>,
) {
    let Some(t) = target.0 else { return };

    // Only drive player-slot 0 (click-to-move is a single-player feature).
    let p_xz = players
        .iter()
        .find(|&(_, &PlayerSlot(s))| s == 0)
        .map(|(tf, _)| Vec2::new(tf.translation.x, tf.translation.z));
    let Some(p_xz) = p_xz else { return };

    let to_target = t - p_xz;
    if to_target.length() < ARRIVAL_RADIUS {
        target.0 = None;
        return;
    }
    writer.write(MoveIntent {
        direction: to_target.normalize(),
        player_slot: 0,
    });
}
