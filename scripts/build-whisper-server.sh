#!/usr/bin/env bash
# Compila whisper-server (whisper.cpp) come binario universale macOS (arm64 + x86_64)
# e lo posiziona in src-tauri/binaries/whisper-server, da dove Tauri lo bundla
# come risorsa dell'app (vedi tauri.conf.json → bundle.resources, docs/backend/stt.md).
#
# Statico (BUILD_SHARED_LIBS=OFF) e con Metal (GGML_METAL=ON): nessuna dylib esterna
# da bundlare, accelerazione GPU su Apple Silicon e Intel.
#
# Uso: ./scripts/build-whisper-server.sh [tag]
#   tag = tag/branch whisper.cpp da compilare (default: v1.7.4)
#
# Richiede: git, cmake, Xcode Command Line Tools (clang con supporto cross-arch macOS).

set -euo pipefail

WHISPER_TAG="${1:-v1.7.4}"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="$ROOT_DIR/src-tauri/binaries"
WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT

echo "==> Clono whisper.cpp @ $WHISPER_TAG in $WORK_DIR"
git clone --depth 1 --branch "$WHISPER_TAG" https://github.com/ggml-org/whisper.cpp.git "$WORK_DIR/whisper.cpp"
cd "$WORK_DIR/whisper.cpp"

CMAKE_FLAGS=(
  -DCMAKE_BUILD_TYPE=Release
  -DGGML_NATIVE=OFF
  -DGGML_METAL=ON
  -DBUILD_SHARED_LIBS=OFF
  -DWHISPER_BUILD_EXAMPLES=ON
  -DWHISPER_BUILD_SERVER=ON
)

for ARCH in arm64 x86_64; do
  echo "==> Compilo per $ARCH"
  cmake -B "build-$ARCH" -DCMAKE_OSX_ARCHITECTURES="$ARCH" "${CMAKE_FLAGS[@]}"
  cmake --build "build-$ARCH" --config Release --target whisper-server -j"$(sysctl -n hw.ncpu)"
done

mkdir -p "$OUT_DIR"
echo "==> Unisco arm64 + x86_64 in binario universale → $OUT_DIR/whisper-server"
lipo -create -output "$OUT_DIR/whisper-server" \
  "build-arm64/bin/whisper-server" \
  "build-x86_64/bin/whisper-server"
chmod +x "$OUT_DIR/whisper-server"

echo "==> Fatto:"
file "$OUT_DIR/whisper-server"
lipo -info "$OUT_DIR/whisper-server"
