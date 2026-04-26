//! Sticky click-to-move target.
//!
//! When the user clicks/taps, `render::handle_click_input` raycasts and
//! emits `ClickMoveIntent`. We store the latest target XZ in
//! `MoveTarget`. Each frame `auto_move_to_target` emits a `MoveIntent`
//! toward the target until the player is close enough (then the target
//! clears).
//!
//! Keyboard input clears `MoveTarget` (see `input::gather_move_input`),
//! so WASD always wins.

use bevy::prelude::*;

use crate::data::{ClickMoveIntent, MoveIntent, MoveTarget, Player};

const ARRIVAL_RADIUS: f32 = 0.4;

pub fn apply_click_target(
    mut events: MessageReader<ClickMoveIntent>,
    mut target: ResMut<MoveTarget>,
) {
    // Latest click wins.
    for e in events.read() {
        target.0 = Some(e.target);
    }
}

pub fn auto_move_to_target(
    player: Query<&Transform, With<Player>>,
    mut target: ResMut<MoveTarget>,
    mut writer: MessageWriter<MoveIntent>,
) {
    let Some(t) = target.0 else {
        return;
    };
    let Ok(p) = player.single() else {
        return;
    };
    let p_xz = Vec2::new(p.translation.x, p.translation.z);
    let to_target = t - p_xz;
    if to_target.length() < ARRIVAL_RADIUS {
        target.0 = None;
        return;
    }
    writer.write(MoveIntent {
        direction: to_target.normalize(),
    });
}
