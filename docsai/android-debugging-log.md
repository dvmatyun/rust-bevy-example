# Android Debugging Log

Complete list of problems encountered and fixed while getting Bevy running on Android (Xiaomi 21081111RG, Mali-G77 MC9, Android 14).

---

## 1. `android-game-activity` vs `android-native-activity`

**Symptom:** `undefined symbol: ANativeActivity_onCreate` on app launch.

**Cause:** `cargo-apk` generates an AndroidManifest pointing to `NativeActivity`, which expects `ANativeActivity_onCreate`. The `android-game-activity` feature exports a different entry point (`GameActivity`).

**Fix:** Use `android-native-activity` feature instead of `android-game-activity` in Cargo.toml. `GameActivity` requires a Gradle-based build, not `cargo-apk`.

---

## 2. `staticlib` in crate-type prevents `libc++_shared.so` bundling

**Symptom:** `dlopen failed: cannot locate symbol "__cxa_pure_virtual"` on app launch.

**Cause:** Adding `"staticlib"` to `crate-type` in Cargo.toml changed how cargo-apk links the C++ runtime, preventing `libc++_shared.so` from being bundled into the APK.

**Fix:** Use `crate-type = ["cdylib", "rlib"]` only. `cdylib` is for Android `.so`, `rlib` is for desktop binary linkage. Do NOT include `staticlib` (it's for iOS and should only be added when building for iOS on a Mac).

---

## 3. `android_shared_stdcxx` feature needed for C++ runtime

**Symptom:** Same `__cxa_pure_virtual` crash even with correct crate-type.

**Cause:** When `bevy_audio` is enabled, Bevy pulls in Oboe (a C++ audio library) which requires `libc++_shared.so`. Without the `android_shared_stdcxx` feature, the shared C++ runtime isn't bundled.

**Fix:** Add `"android_shared_stdcxx"` to Bevy features in Cargo.toml when `bevy_audio` is enabled. Note: this feature does NOT exist in Bevy 0.19-dev (not needed there because the dependency chain changed).

---

## 4. Missing renderer features cause purple screen on ALL platforms

**Symptom:** Purple/magenta screen for 1-2 frames, then crash. Happens on Windows too, not just Android.

**Cause:** Stripping Bevy features to reduce APK size removed `tonemapping_luts`, `hdr`, `ktx2`, and `zstd_rust`. Without these, the PBR tone mapping pipeline cannot initialize — it needs pre-baked LUT textures that are compiled into the binary via `include_bytes!`.

**Fix:** Always include these four features together:
```toml
"tonemapping_luts",
"hdr",
"ktx2",
"zstd_rust",  # called "zstd" in Bevy main branch, "zstd_rust" in 0.18.1
```

**Also note:**
- `bevy_diagnostic` and `bevy_input` are NOT Cargo features — they're always included. Don't add them to the feature list.
- In Bevy 0.18.1 the feature is `zstd_rust`. In Bevy 0.19-dev it's also `zstd_rust` (the official example uses `zstd` but that's an alias in the source tree).

---

## 5. `strip = "symbols"` breaks Android

**Symptom:** `java.lang.UnsatisfiedLinkError: Unable to load native library` — the `.so` loads but `android_main` symbol is missing.

**Cause:** The release profile had `strip = "symbols"` which removes ALL symbols including exported ones like `android_main` and `ANativeActivity_onCreate`.

**Fix:** Use `strip = "debuginfo"` instead of `strip = "symbols"` in the workspace `[profile.release]`. This removes debug info (saves space) but keeps exported symbols intact.

---

## 6. Shadows segfault on Android

**Symptom:** SIGSEGV after rendering starts on some Android devices.

**Cause:** Shadow map rendering causes segfaults on certain Android GPU drivers (known Bevy issue https://github.com/bevyengine/bevy/issues/8214).

**Fix:** Disable shadows on Android with a cfg guard:
```rust
commands.spawn((
    PointLight {
        intensity: 1_000_000.0,
        #[cfg(not(target_os = "android"))]
        shadows_enabled: true,
        ..default()
    },
    Transform::from_xyz(4.0, 8.0, 4.0),
));
```

---

## 7. MSAA panics on Android

**Symptom:** Vulkan command errors / panics during rendering on Android.

**Cause:** MSAA resolve passes trigger bugs in some Android Vulkan drivers (known Bevy issue https://github.com/bevyengine/bevy/issues/8229).

**Fix:** Disable MSAA on Android with a cfg guard on the camera:
```rust
commands.spawn((
    Camera3d::default(),
    Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    #[cfg(target_os = "android")]
    Msaa::Off,
));
```

---

## 8. Bevy 0.18.1 wgpu Vulkan SIGSEGV on Mali-G77

**Symptom:** `Fatal signal 11 (SIGSEGV)` in `wgpu_hal::vulkan::command::CommandEncoder::begin_encoding` during `queue_submit`. Fault address pattern: `0x1000000xx` (null handle + offset). Crashes on every launch, ~0.5s after window creation.

**Device:** Mali-G77 MC9, driver `v1.r32p1-01eac0`, Vulkan backend.

**Cause:** Bug in the wgpu version shipped with Bevy 0.18.1. The Vulkan command encoder receives a null/invalid device handle on this GPU driver.

**Attempted workarounds (NONE worked):**
- Removing `multi_threaded` (single-threaded mode)
- `Msaa::Off`
- Disabling shadows
- `Backends::GL` (OpenGL ES) — returned "Unable to find a GPU"
- Minimal feature set
- `WinitSettings::mobile()`

**Fix:** Switch from `bevy = "0.18.1"` to local Bevy 0.19-dev (`path = "C:/Repositories/Rust/bevy"`) which ships a newer wgpu that fixes this Mali driver bug. The Vulkan SIGSEGV disappears completely with 0.19-dev.

---

## 9. Bevy 0.19-dev PBR cluster bindings crash

**Symptom:** `thread panicked: called Option::unwrap() on a None value` at `bevy_pbr/src/render/mesh_view_bindings.rs:727`.

**Cause:** The cluster index list buffer was empty because of problem #10 below. The `.unwrap()` on `clusterable_object_index_lists_binding()` (which returns `Option`) panicked because the buffer hadn't been allocated.

**Initial workaround:** Changed `.unwrap()` to `match` with `continue` — this prevented the crash but also skipped all view rendering, so nothing was visible.

**Real fix:** Fix problem #10 below, which makes the buffer valid so `.unwrap()` succeeds.

---

## 10. CPU clustering with Storage buffers — root cause of invisible scene

**Symptom:** App runs without crashing (after fix #9) but nothing renders — black/empty screen. Error log spam: `"Shouldn't be pushing a clusterable object index from CPU when GPU clustering is in use"`.

**Cause:** The Mali-G77 supports storage buffers but does NOT support GPU compute clustering. Bevy's initialization:
1. Checks `supports_storage_buffers` → true (Mali-G77 has them)
2. Creates `ViewClusterBindings` with `Storage` variant buffers
3. Checks `gpu_clustering_supported` → false (no compute shader support)
4. Falls back to CPU clustering code
5. CPU clustering calls `push_raw_index()` which hits the `Storage` arm
6. The `Storage` arm just logged `error!(...)` instead of writing data
7. Empty buffer → `binding()` returns `None` → PBR pipeline can't create bind groups → nothing renders

**Fix:** Patched `push_raw_index()` in `bevy/crates/bevy_pbr/src/cluster/mod.rs` (line ~660):

```rust
// BEFORE (broken):
ViewClusterBuffers::Storage { .. } => {
    error!("Shouldn't be pushing a clusterable object index from CPU when GPU clustering is in use");
}

// AFTER (fixed):
ViewClusterBuffers::Storage {
    clusterable_object_index_lists,
    ..
} => {
    // CPU clustering fallback with storage buffers — allocate space so
    // the buffer exists and binding() returns Some.
    clusterable_object_index_lists.add();
}
```

This allocates buffer space (via `UninitBufferVec::add()`) so the GPU buffer gets created during `write_buffer()`, making `binding()` return `Some`, allowing the PBR pipeline to proceed.

---

## Summary of final working configuration

- **Bevy:** Local 0.19-dev from `C:/Repositories/Rust/bevy` (with patches above)
- **crate-type:** `["cdylib", "rlib"]`
- **Android feature:** `android-native-activity`
- **Required renderer features:** `tonemapping_luts`, `hdr`, `ktx2`, `zstd_rust`
- **Release profile:** `strip = "debuginfo"` (NOT `"symbols"`)
- **Runtime guards:** `Msaa::Off` and `shadows_enabled: false` on Android via `#[cfg]`
- **Build command:** `cargo apk build -p my_bevy_game --lib`
- **Test device:** Xiaomi 21081111RG, Mali-G77 MC9, Android 14
