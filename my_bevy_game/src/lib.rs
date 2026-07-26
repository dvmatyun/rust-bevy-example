//! Top-level app composition. The actual game is layered:
//!
//!   data → server → client_sim → render
//!
//! See `docsai/architecture.md` for the architecture rules and the
//! `architecture` skill at `.claude/skills/architecture/SKILL.md`.

// Modules are public so integration tests under `tests/` can reach
// `data::DataPlugin`, `server::replication::*`, etc. The architecture
// rules still apply — see `.claude/skills/architecture/SKILL.md`.
pub mod client_sim;
pub mod data;
pub mod render;
pub mod server;

use bevy::{prelude::*, window::{AppLifecycle, RequestRedraw, WindowOccluded}, winit::WinitSettings};
#[cfg(any(target_os = "android", target_os = "ios"))]
use bevy::window::WindowMode;

#[bevy_main]
pub fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(window_settings()),
                ..default()
            }),
            data::DataPlugin,
            server::ServerPlugin,
            client_sim::ClientSimPlugin,
            render::RenderPlugin,
        ))
        .insert_resource(winit_settings())
        .add_systems(Update, (handle_lifetime, handle_unocclude))
        .run();
}

fn handle_lifetime(mut events: MessageReader<AppLifecycle>) {
    for _e in events.read() {}
}

/// Force a redraw when the window returns from a minimised/occluded state.
/// Without this, wgpu holds on to a stale swapchain surface and the screen
/// stays black until the next input event triggers a repaint.
#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn handle_unocclude(
    mut events: MessageReader<WindowOccluded>,
    mut redraw: MessageWriter<RequestRedraw>,
) {
    for ev in events.read() {
        if !ev.occluded {
            redraw.write(RequestRedraw);
        }
    }
}

#[cfg(any(target_os = "android", target_os = "ios"))]
fn handle_unocclude() {}

fn winit_settings() -> WinitSettings {
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        WinitSettings::mobile()
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        WinitSettings::game()
    }
}

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
