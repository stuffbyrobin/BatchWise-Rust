#!/usr/bin/env bash
# Build the brewing-physics WASM bundle and emit JS/TS bindings into the frontend.
set -euo pipefail

cd "$(dirname "$0")"

# Output into the React app (the BatchWise repo, a sibling checkout). Override
# with OUT_DIR=… for a different layout.
OUT_DIR="${OUT_DIR:-../../BatchWise/frontend/src/lib/physics/wasm}"
CRATE_NAME="batchwise_physics_wasm"

echo "→ compiling to wasm32-unknown-unknown (release)…"
cargo build --release --target wasm32-unknown-unknown

echo "→ generating bindings (--target web) into ${OUT_DIR}…"
mkdir -p "${OUT_DIR}"
wasm-bindgen \
  "target/wasm32-unknown-unknown/release/${CRATE_NAME}.wasm" \
  --out-dir "${OUT_DIR}" \
  --target web

echo "✓ done. Artifacts in ${OUT_DIR}:"
ls -1 "${OUT_DIR}"
