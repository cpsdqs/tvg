#!/bin/bash
set -euxo pipefail

cd "$(dirname "$0")"
cd tvg-wasm
cargo build --release --target wasm32-unknown-unknown
cd ..
wasm-bindgen --target web --out-dir tvg-wasm-out ../target/wasm32-unknown-unknown/release/tvg_wasm.wasm

if [[ "$#" -gt 0 ]]; then
  if [[ "$1" == "serve" ]]; then
    python3 -m http.server
  fi
fi
