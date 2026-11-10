#!/usr/bin/env bash
# Runs a Tauri Android command with the toolchain environment set up, e.g.
#   scripts/android.sh build --target aarch64 --debug --apk
#   scripts/android.sh dev
# Defaults match a standard Android Studio install on macOS; override by exporting them.
set -euo pipefail
export JAVA_HOME="${JAVA_HOME:-/Applications/Android Studio.app/Contents/jbr/Contents/Home}"
export ANDROID_HOME="${ANDROID_HOME:-$HOME/Library/Android/sdk}"
if [[ -z "${NDK_HOME:-}" ]]; then
  NDK_HOME="$(ls -d "$ANDROID_HOME"/ndk/* | sort -V | tail -1)"
fi
export NDK_HOME
[[ -f "$HOME/.cargo/env" ]] && source "$HOME/.cargo/env"
cd "$(dirname "$0")/.."
# Native toolchain exports (llama.cpp / NDK cross-compile) live in a sibling file.
# shellcheck disable=SC1091
[[ -f scripts/android-env.sh ]] && source scripts/android-env.sh
exec pnpm tauri android "$@"
