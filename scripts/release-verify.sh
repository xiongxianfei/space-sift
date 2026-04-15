#!/usr/bin/env bash
set -euo pipefail

if [[ -n "$(git status --porcelain)" ]]; then
  echo "Release verification failed: working tree is dirty."
  exit 1
fi

package_version="$(node -e "console.log(JSON.parse(require('fs').readFileSync('package.json', 'utf8')).version)")"
tauri_version="$(node -e "console.log(JSON.parse(require('fs').readFileSync('src-tauri/tauri.conf.json', 'utf8')).version)")"
cargo_version="$(grep '^version = ' src-tauri/Cargo.toml | head -n1 | sed -E 's/version = \"([^\"]+)\"/\1/')"

if [[ "$package_version" != "$tauri_version" || "$package_version" != "$cargo_version" ]]; then
  echo "Release verification failed: version mismatch."
  echo "package.json: $package_version"
  echo "src-tauri/tauri.conf.json: $tauri_version"
  echo "src-tauri/Cargo.toml: $cargo_version"
  exit 1
fi

if [[ -n "${GITHUB_REF_NAME:-}" && ! "${GITHUB_REF_NAME}" =~ ^v ]]; then
  echo "Release verification failed: GitHub tag must start with v."
  exit 1
fi

echo "Release verification passed."
echo "Version: $package_version"
echo "Reminder: public releases must use signed Windows artifacts."
