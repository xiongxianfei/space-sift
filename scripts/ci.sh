#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ps_script="${script_dir}/ci.ps1"

for shell_cmd in pwsh pwsh.exe powershell powershell.exe; do
  if command -v "${shell_cmd}" >/dev/null 2>&1; then
    exec "${shell_cmd}" -NoLogo -NoProfile -ExecutionPolicy Bypass -File "${ps_script}"
  fi
done

echo "scripts/ci.sh is a compatibility wrapper. Use PowerShell and run 'powershell -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts/ci.ps1'." >&2
exit 127
