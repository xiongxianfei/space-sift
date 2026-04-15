#!/usr/bin/env bash
set -euo pipefail

npm ci
npm run lint
npm run test
npm run build
cargo check --manifest-path src-tauri/Cargo.toml
