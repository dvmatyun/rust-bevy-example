# Rust Bevy Example — Project Guide

## Target Platforms
This project targets **all four platforms** and must be kept working on all of them at all times:
- **Windows** — primary dev platform, `cargo run`
- **Web (WASM)** — `trunk serve`, requires `bevy/webgl2` feature
- **Android** — `cargo-apk` + NDK
- **iOS** — Xcode + Apple signing

## Project Structure
- `my_bevy_game/` — standalone Bevy game (the main game, all platforms)
- `bevy_ffi/` — headless Bevy rendered to a pixel buffer, exposed as a C FFI cdylib for embedding (not in workspace currently)
- `flutter_rust_wrap/` — Flutter Windows app that embeds `bevy_ffi` via a texture plugin

## Build Commands
- **Windows:** `cargo run -p my_bevy_game` / `cargo run -p my_bevy_game --release`
- **Android APK:** `ANDROID_HOME=... NDK_HOME=... cargo apk build -p my_bevy_game --lib`
- **Web:** `cd my_bevy_game && trunk serve` (requires `trunk` installed)
- **iOS:** `cargo build -p my_bevy_game --target aarch64-apple-ios --release` then Xcode

## Android Setup (already done)
- NDK at `C:/Users/worc1/AppData/Local/Android/Sdk/ndk/28.0.12916984`
- Rust target `aarch64-linux-android` installed
- `cargo-apk` installed
- Always use `--lib` flag: `cargo apk build -p my_bevy_game --lib` (avoids bin/cdylib conflict panic)

## Rules for Claude
- **Record every milestone achieved** in the Milestones section below.
- Before adding platform-specific code, use `#[cfg(...)]` guards — never break other platforms.
- Keep `my_bevy_game` as the canonical game implementation; `bevy_ffi` is a thin headless wrapper around the same logic.
- Prefer Bevy idiomatic patterns: ECS components/resources/systems over global state.

## Milestones
- [x] Basic Bevy ECS demo — Person/Name components, query systems
- [x] 3D scene — 6 colored orbiting cubes, ground plane, directional light, camera
- [x] Flutter Windows embedding — `bevy_ffi` cdylib, `bevy_texture_plugin` C++ Flutter plugin, MethodChannel bridge (`init`, `setParams`, `getStats`, `shutdown`), Flutter `Texture` widget display
- [x] `bevy_ffi` compiles cleanly against Bevy 0.18 — fixed all API changes (`RenderTarget` as component, `RenderSystems`, `TexelCopyBufferInfo`, `MessageWriter<AppExit>`, `#[unsafe(no_mangle)]`, `PollType`)
- [x] In-game UI — +1/-1/+10/-10/+100/-100 cube count buttons, FPS/frame-time/avg/1%low HUD overlay
- [x] Multi-platform build setup — `webgl2` feature, `android-game-activity` feature, `index.html` for Trunk, `[lib] cdylib+rlib`, `#[bevy_main]`, Android APK verified building (`cargo apk build --lib`)
