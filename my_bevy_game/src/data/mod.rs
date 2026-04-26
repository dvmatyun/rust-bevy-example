//! Layer 1 — pure data. Shared types used by every other layer.
//!
//! Rules:
//! - No systems beyond `add_message` / `insert_resource` / `configure_sets`.
//! - No render types (Mesh3d, Camera3d, Color, Node, ...).
//! - No platform-specific code.
//!
//! See `docsai/architecture.md` for the full architecture description.

mod components;
mod config;
mod intents;
pub mod replication;

pub use components::*;
pub use config::*;
pub use config::joystick_layout;
pub use intents::*;
// Re-exports of the public replication API. Some are not used inside
// the crate yet — they're for game code (and future networking).
#[allow(unused_imports)]
pub use replication::{
    PeerId, Replicate, ReplicateTo, Replicated, ReplicationDelta, ReplicationOutbox,
    ReplicationSet, ServerEntity,
};

use bevy::prelude::*;

/// Top-level system set used to enforce strict per-frame ordering across
/// plugin boundaries:
///
///   Input → GameLogic → ViewModel → RenderApply
///
/// Each plugin assigns its `Update` systems to one of these sets via
/// `.in_set(AppSet::...)`. The chain is configured below.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum AppSet {
    /// `client_sim` reads input, emits intents.
    Input,
    /// `server` consumes intents, mutates game state.
    GameLogic,
    /// `client_sim` derives view-model state from game state.
    ViewModel,
    /// `render` applies view-model + game state to actual visuals.
    RenderApply,
}

/// Registers shared messages and inserts default resources.
///
/// Add **before** `ServerPlugin`, `ClientSimPlugin`, `RenderPlugin`.
pub struct DataPlugin;

impl Plugin for DataPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<MoveIntent>()
            .add_message::<CameraOrbitIntent>()
            .add_message::<ClickMoveIntent>()
            .insert_resource(WorldConfig::default())
            .insert_resource(Settings::default())
            .insert_resource(DesiredCameraView::default())
            .insert_resource(JoystickState::default())
            .insert_resource(MoveTarget::default())
            .insert_resource(TerrainHeights::default())
            .configure_sets(
                Update,
                (
                    AppSet::Input,
                    AppSet::GameLogic,
                    AppSet::ViewModel,
                    AppSet::RenderApply,
                )
                    .chain(),
            );
    }
}
