# Environment for cross-building Rust + llama.cpp (C++/cmake) to Android.
# `source` it; it only exports variables (idempotent). Used by
# scripts/android.sh. Needs: Android Studio, the SDK with an NDK and the
# SDK's cmake (SDK Manager > SDK Tools > CMake), `rustup target add
# aarch64-linux-android`.

export JAVA_HOME="/Applications/Android Studio.app/Contents/jbr/Contents/Home"
export ANDROID_HOME="${ANDROID_HOME:-$HOME/Library/Android/sdk}"
export ANDROID_SDK_ROOT="$ANDROID_HOME"
# Newest installed NDK.
NDK_HOME="$(ls -d "$ANDROID_HOME"/ndk/*/ 2>/dev/null | sort -V | tail -1)"
export NDK_HOME="${NDK_HOME%/}"
export ANDROID_NDK_HOME="$NDK_HOME" ANDROID_NDK_ROOT="$NDK_HOME" ANDROID_NDK="$NDK_HOME"

_nk_api=28
_nk_llvm="$NDK_HOME/toolchains/llvm/prebuilt/darwin-x86_64"
_nk_cmake="$(ls -d "$ANDROID_HOME"/cmake/*/bin 2>/dev/null | sort -V | tail -1)"
# (The NDK clang is NOT put on PATH: that would also hijack host builds.)
# PATH gets the SDK cmake once; sourcing twice is a no-op.
if [ -n "$_nk_cmake" ]; then
  case ":$PATH:" in *":$_nk_cmake:"*) ;; *) export PATH="$_nk_cmake:$PATH" ;; esac
fi

# cc-rs / cargo
export CC_aarch64_linux_android="$_nk_llvm/bin/aarch64-linux-android${_nk_api}-clang"
export CXX_aarch64_linux_android="$_nk_llvm/bin/aarch64-linux-android${_nk_api}-clang++"
export AR_aarch64_linux_android="$_nk_llvm/bin/llvm-ar"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$CC_aarch64_linux_android"
# cmake (llama.cpp): the NDK's toolchain file; the cmake crate reads these.
export CMAKE_TOOLCHAIN_FILE_aarch64_linux_android="$NDK_HOME/build/cmake/android.toolchain.cmake"
export ANDROID_PLATFORM="android-${_nk_api}"
export ANDROID_ABI="arm64-v8a"
# Use the SDK's ninja if present (the NDK toolchain file likes it).
if [ -x "$_nk_cmake/ninja" ]; then export CMAKE_GENERATOR="Ninja"; fi
unset _nk_api _nk_llvm _nk_cmake
# llama.cpp CPU kernels: without this the build is plain armv8-a (no int8
# dot-product / matrix-multiply), several times slower for Q4 models. The
# Galaxy S25 (Snapdragon 8 Elite, Oryon) has dotprod and i8mm. Do NOT source
# this for a phone older than ~2021 (Cortex-A76 and older would crash).
export GGML_CPU_ARM_ARCH="armv8.6-a+dotprod+i8mm"
