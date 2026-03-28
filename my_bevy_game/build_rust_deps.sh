#!/usr/bin/env bash

# Builds Rust dependencies for iOS — called by the Xcode "Rust" build phase.
# Based on Bevy's official mobile example and
# https://github.com/nicklockwood/iVersion/blob/master/RELEASE_NOTES.md

set -eux

PATH=$PATH:$HOME/.cargo/bin

PROFILE=debug
RELFLAG=
if [[ "$CONFIGURATION" != "Debug" ]]; then
    PROFILE=release
    RELFLAG=--release
fi

set -euvx

# Homebrew on Apple Silicon
export PATH="$PATH:/opt/homebrew/bin"

# Cargo output goes into Xcode's derived-data tree
export CARGO_TARGET_DIR="$DERIVED_FILE_DIR/cargo"

# Reset PATH so `cc` resolves to the system compiler (avoids "ld: library
# 'System' not found" from the Xcode-injected toolchain).
export PATH="/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin:$PATH"

IS_SIMULATOR=0
if [ "${LLVM_TARGET_TRIPLE_SUFFIX-}" = "-simulator" ]; then
  IS_SIMULATOR=1
fi

EXECUTABLES=
for arch in $ARCHS; do
  case "$arch" in
    x86_64)
      if [ $IS_SIMULATOR -eq 0 ]; then
        echo "Building for x86_64, but not a simulator build. What's going on?" >&2
        exit 2
      fi
      export CFLAGS_x86_64_apple_ios="-target x86_64-apple-ios"
      TARGET=x86_64-apple-ios
      ;;

    arm64)
      if [ $IS_SIMULATOR -eq 0 ]; then
        TARGET=aarch64-apple-ios
      else
        TARGET=aarch64-apple-ios-sim
      fi
      ;;
  esac

  cargo build $RELFLAG --target $TARGET --bin my_bevy_game_bin

  EXECUTABLES="$EXECUTABLES $DERIVED_FILE_DIR/cargo/$TARGET/$PROFILE/my_bevy_game_bin"
done

# Combine multi-arch binaries and place at the path Xcode expects
lipo -create -output "$TARGET_BUILD_DIR/$EXECUTABLE_PATH" $EXECUTABLES
