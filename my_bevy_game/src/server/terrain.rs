//! Terrain generation glue.
//!
//! After moving the noise + height functions to
//! `data::terrain_gen`, the server itself doesn't have to do any
//! per-cell setup at startup — heights are computed on demand
//! everywhere they're needed (player snap-to-ground, building
//! placement, click raycast, render chunk mesh build). This file is
//! kept as a tiny shim so `server/mod.rs` can still register a setup
//! system in case future game logic needs to do one-shot terrain
//! work (caves, spawn-point selection, …).

use bevy::prelude::*;

pub fn setup_terrain() {
    // No-op for now; heights are procedural and queried on demand.
}
