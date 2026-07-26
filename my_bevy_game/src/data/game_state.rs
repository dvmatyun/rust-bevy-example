use bevy::prelude::*;

/// Top-level game flow states.
///
/// `MainMenu` is the default. The menu transitions to one of the two
/// playing states; there is currently no transition back (restart the
/// app to return to the menu).
#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum GameState {
    #[default]
    MainMenu,
    PlayingSingle,
    PlayingMultiplayer,
}
