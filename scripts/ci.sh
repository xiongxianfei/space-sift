#!/usr/bin/env bash
set -euo pipefail

run_via_windows_shell() {
  if [ -n "${COMSPEC:-}" ]; then
    "$COMSPEC" /d /c "$@"
    return
  fi

  if command -v cmd.exe >/dev/null 2>&1; then
    cmd.exe /d /c "$@"
    return
  fi

  if command -v cmd >/dev/null 2>&1; then
    cmd /d /c "$@"
    return
  fi

  return 1
}

run_npm() {
  if command -v npm >/dev/null 2>&1; then
    npm "$@"
    return
  fi

  if run_via_windows_shell npm "$@"; then
    return
  fi

  echo "npm is not available on PATH" >&2
  exit 127
}

run_cargo() {
  if command -v cargo >/dev/null 2>&1; then
    cargo "$@"
    return
  fi

  if run_via_windows_shell cargo "$@"; then
    return
  fi

  echo "cargo is not available on PATH" >&2
  exit 127
}

run_npm ci
run_npm run lint
run_npm run test
run_npm run build
run_cargo check --manifest-path src-tauri/Cargo.toml
