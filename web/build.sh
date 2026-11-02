#!/usr/bin/env bash
# Builds the standalone play page into site/: the play page, the game compiled to
# WebAssembly, and the network weights. The release workflow publishes site/ to
# GitHub Pages. Needs the wasm32-unknown-unknown target and wasm-bindgen-cli at
# the version web/Cargo.toml pins.
#
# Preview locally:  bash web/build.sh && py -3.12 -m http.server -d site 8767
set -euo pipefail
cd "$(dirname "$0")/.."

cargo build -p loonn-web --target wasm32-unknown-unknown --profile web
rm -rf site
mkdir -p site
wasm-bindgen --target web --no-typescript --out-dir site/pkg target/wasm32-unknown-unknown/web/loonn_web.wasm
cp play/index.html play/logo.png play/favicon.png site/
cp web/worker.js web/model.bin site/
echo "Built site/ ($(du -sh site | cut -f1))"
