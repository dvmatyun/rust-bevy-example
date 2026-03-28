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
- **Windows:** `cargo run --bin my_bevy_game_bin` / `cargo run --bin my_bevy_game_bin --release`
- **Android APK (PowerShell):**
  ```powershell
  $env:ANDROID_HOME="C:/Users/worc1/AppData/Local/Android/Sdk"
  $env:NDK_HOME="C:/Users/worc1/AppData/Local/Android/Sdk/ndk/28.0.12916984"
  cargo apk build -p my_bevy_game --lib
  ```
- **Web:** `cd my_bevy_game && trunk serve` (requires `trunk` installed)
- **iOS:** `cargo build -p my_bevy_game --target aarch64-apple-ios --release` then Xcode

## Android Setup (already done)
- NDK at `C:/Users/worc1/AppData/Local/Android/Sdk/ndk/28.0.12916984`
- Rust target `aarch64-linux-android` installed
- `cargo-apk` installed
- Always use `--lib` flag: `cargo apk build -p my_bevy_game --lib` (avoids bin/cdylib conflict panic)
- Test device: **Xiaomi 21081111RG, Mali-G77 MC9, Android 14**

## Android Debugging Lessons (CRITICAL — do not repeat these mistakes)

> **Full detailed log with all 10 problems and fixes:** [docsai/android-debugging-log.md](docsai/android-debugging-log.md)

### Bevy version: must use local 0.19-dev, NOT published 0.18.1
- **Bevy 0.18.1 + Mali-G77 = SIGSEGV** in `wgpu_hal::vulkan::command::CommandEncoder::begin_encoding` during `queue_submit`. This is a wgpu Vulkan bug fixed in newer wgpu (shipped in Bevy 0.19-dev). **No workaround exists** — single-threaded, MSAA off, shadows off, GL backend — nothing helps.
- **Bevy 0.19-dev Vulkan works** on Mali-G77 but has a PBR cluster bindings bug (`unwrap()` on `None` at `mesh_view_bindings.rs:727`). We patched this locally with `continue` instead of `unwrap()` at `C:\Repositories\Rust\bevy\crates\bevy_pbr\src\render\mesh_view_bindings.rs`.
- The dependency uses `path = "C:/Repositories/Rust/bevy"` pointing to local Bevy 0.19-dev.

### Feature flags that matter
- `android-native-activity` — required by `cargo-apk` (NOT `android-game-activity`)
- `android_shared_stdcxx` — bundles `libc++_shared.so` into APK. **Required when `bevy_audio` is enabled** (Oboe is C++). Without it: `__cxa_pure_virtual` crash on load. NOT available in Bevy 0.19-dev (not needed without audio).
- `tonemapping_luts` + `hdr` + `ktx2` + `zstd_rust` — **required for the renderer to initialize**. Without these: purple screen → crash on all platforms.
- `webgl2` — harmless on native, needed for web.
- In Bevy 0.18.1: `zstd_rust` (NOT `zstd`). In Bevy 0.19-dev: also `zstd_rust`.
- `bevy_diagnostic` and `bevy_input` are NOT Cargo features — they're always included.
- `FontSize::Px(30.0)` is required in 0.19-dev; plain `30.0` works in 0.18.1.

### crate-type in Cargo.toml
- Use `["cdylib", "rlib"]` — cdylib for Android .so, rlib for desktop binary linkage.
- Do NOT include `"staticlib"` — it prevents cargo-apk from bundling `libc++_shared.so`.

### Android cfg guards (from official Bevy mobile example)
```rust
// Shadows segfault on some Android GPUs
#[cfg(not(target_os = "android"))]
shadows_enabled: true,

// MSAA causes Vulkan panics on Android
#[cfg(target_os = "android")]
Msaa::Off,
```

### Other Android findings
- `cargo-apk` is deprecated but still works. Only supports `NativeActivity`.
- `strip = "symbols"` in release profile breaks Android — strips `android_main` export. Use `strip = "debuginfo"` instead.
- Debug APK: ~110 MB. Release APK: ~18 MB.
- `WinitSettings::mobile()` — only redraws on input (saves battery). Touch screen to see rendering.
- GL backend (`Backends::GL`) does not work on this device ("Unable to find a GPU").
- `bevy_gilrs` always fails on Android ("Gilrs does not support current platform") — harmless.

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
- [x] Multi-platform build setup — `webgl2` feature, `android-native-activity` feature, `index.html` for Trunk, `[lib] cdylib+rlib`, `#[bevy_main]`, Android APK verified building (`cargo apk build --lib`)
- [x] Android APK running on Mali-G77 — patched Bevy 0.19-dev PBR cluster bindings, Vulkan rendering working, no SIGSEGV
