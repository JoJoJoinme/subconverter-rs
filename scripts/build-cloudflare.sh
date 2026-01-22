#!/bin/bash
set -e

# Start stopwatch
BUILD_START_TIME=$SECONDS

echo "Building Subconverter for Cloudflare Workers..."

# Check if wasm-pack is installed
if ! command -v wasm-pack &> /dev/null; then
    echo "wasm-pack not found. Installing..."
    curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
fi

# Paths
ROOT_DIR=$(pwd)
OUT_DIR="$ROOT_DIR/cloudflare/pkg"

# Build WASM
echo "Running wasm-pack build with cloudflare feature..."
# Ensure the output directory exists
mkdir -p cloudflare
# --target web is important for Cloudflare Workers (ES modules)
# --features cloudflare enables the Cloudflare-specific bindings
wasm-pack build --target web --out-dir "$OUT_DIR" --release --features cloudflare

echo "Build complete! Output in $OUT_DIR"

# Calculate and print build time
BUILD_END_TIME=$SECONDS
BUILD_DURATION=$((BUILD_END_TIME - BUILD_START_TIME))
echo "Total build time: $((BUILD_DURATION / 60)) minutes and $((BUILD_DURATION % 60)) seconds"
