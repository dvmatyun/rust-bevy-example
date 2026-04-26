//! FPS overlay. Pure render concern.

use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
};

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin {
            max_history_length: 120,
            ..default()
        })
        .insert_resource(FrameTimeHistory::new(120))
        .add_systems(Startup, spawn_hud)
        .add_systems(Update, (record_frame_time, update_hud));
    }
}

#[derive(Component)]
struct HudText;

#[derive(Resource)]
struct FrameTimeHistory {
    times: std::collections::VecDeque<f32>,
}

impl FrameTimeHistory {
    fn new(capacity: usize) -> Self {
        Self {
            times: std::collections::VecDeque::with_capacity(capacity),
        }
    }

    fn push(&mut self, dt: f32, capacity: usize) {
        self.times.push_back(dt);
        while self.times.len() > capacity {
            self.times.pop_front();
        }
    }

    fn avg_fps(&self) -> f32 {
        if self.times.is_empty() {
            return 0.0;
        }
        let mean = self.times.iter().sum::<f32>() / self.times.len() as f32;
        if mean > 0.0 { 1.0 / mean } else { 0.0 }
    }

    fn one_pct_low_fps(&self) -> f32 {
        if self.times.is_empty() {
            return 0.0;
        }
        let mut sorted: Vec<f32> = self.times.iter().copied().collect();
        sorted.sort_by(|a, b| b.total_cmp(a));
        let count = ((sorted.len() as f32 * 0.01).ceil() as usize).max(1);
        let worst_mean = sorted[..count].iter().sum::<f32>() / count as f32;
        if worst_mean > 0.0 { 1.0 / worst_mean } else { 0.0 }
    }
}

fn spawn_hud(mut commands: Commands) {
    let safe_top = Val::Px(48.0);
    let safe_side = Val::Px(20.0);
    // Reserve the top-right corner for the settings gear (48px button
    // at right: 20px). HUD ends 80px before the right edge so the
    // gear doesn't sit on top of FPS text on narrow screens.
    let gear_clearance = Val::Px(20.0 + 48.0 + 12.0);

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: safe_side,
                right: gear_clearance,
                top: safe_top,
                padding: UiRect::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
        ))
        .with_child((
            Text::new("FPS: --"),
            TextFont {
                font_size: FontSize::Px(14.0),
                ..default()
            },
            TextColor(Color::WHITE),
            HudText,
        ));
}

fn record_frame_time(time: Res<Time>, mut history: ResMut<FrameTimeHistory>) {
    history.push(time.delta_secs(), 120);
}

fn update_hud(
    diagnostics: Res<DiagnosticsStore>,
    history: Res<FrameTimeHistory>,
    mut q: Query<&mut Text, With<HudText>>,
) {
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);
    let frame_ms = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0)
        * 1000.0;
    let avg = history.avg_fps();
    let one_pct = history.one_pct_low_fps();
    for mut text in &mut q {
        *text = Text::new(format!(
            "FPS {fps:.0}  Avg {avg:.0}  1%Low {one_pct:.0}  |  {frame_ms:.2}ms  |  WASD to move"
        ));
    }
}
