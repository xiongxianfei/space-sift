#!/usr/bin/env bash
set -euo pipefail

fail() {
  echo "Release verification failed: $1"
  exit 1
}

if [[ -n "$(git status --porcelain)" ]]; then
  fail "working tree is dirty."
fi

package_version="$(node -e "console.log(JSON.parse(require('fs').readFileSync('package.json', 'utf8')).version)")"
tauri_version="$(node -e "console.log(JSON.parse(require('fs').readFileSync('src-tauri/tauri.conf.json', 'utf8')).version)")"
cargo_version="$(node -e "const cargo = require('fs').readFileSync('src-tauri/Cargo.toml', 'utf8'); const match = cargo.match(/^\\[package\\][\\s\\S]*?^version = \\\"([^\\\"]+)\\\"/m); if (!match) process.exit(1); console.log(match[1]);")"

winget_dir="winget/manifests/x/xiongxianfei/SpaceSift/${package_version}"
winget_version_manifest="${winget_dir}/xiongxianfei.SpaceSift.yaml"
winget_locale_manifest="${winget_dir}/xiongxianfei.SpaceSift.locale.en-US.yaml"
winget_installer_manifest="${winget_dir}/xiongxianfei.SpaceSift.installer.yaml"
release_doc="docs/release.md"
release_workflow=".github/workflows/release.yml"
release_config_writer="scripts/write-tauri-release-config.mjs"

if [[ "$package_version" != "$tauri_version" || "$package_version" != "$cargo_version" ]]; then
  echo "package.json: ${package_version}"
  echo "src-tauri/tauri.conf.json: ${tauri_version}"
  echo "src-tauri/Cargo.toml: ${cargo_version}"
  fail "version mismatch."
fi

for required_file in \
  "${winget_version_manifest}" \
  "${winget_locale_manifest}" \
  "${winget_installer_manifest}" \
  "${release_doc}" \
  "${release_workflow}" \
  "${release_config_writer}"
do
  [[ -f "${required_file}" ]] || fail "missing required release file: ${required_file}"
done

for manifest_file in \
  "${winget_version_manifest}" \
  "${winget_locale_manifest}" \
  "${winget_installer_manifest}"
do
  grep -Fq "PackageVersion: ${package_version}" "${manifest_file}" || \
    fail "version drift in ${manifest_file}"
done

grep -Fq '"allowDowngrades": false' src-tauri/tauri.conf.json || \
  fail "src-tauri/tauri.conf.json must disable installer downgrades."
grep -Fq '"type": "downloadBootstrapper"' src-tauri/tauri.conf.json || \
  fail "src-tauri/tauri.conf.json must declare a WebView2 install mode."
grep -Fq 'TAURI_UPDATER_PUBLIC_KEY' "${release_config_writer}" || \
  fail "release config writer must require TAURI_UPDATER_PUBLIC_KEY."
grep -Fq 'createUpdaterArtifacts: true' "${release_config_writer}" || \
  fail "release config writer must enable updater artifacts."
grep -Fq 'latest/download/latest.json' "${release_config_writer}" || \
  fail "release config writer must target the GitHub latest.json updater endpoint."

grep -Fq 'tauri-apps/tauri-action@v0' "${release_workflow}" || \
  fail "release workflow must use tauri release tooling."
grep -Fq 'windows-latest' "${release_workflow}" || \
  fail "release workflow must package on windows-latest."
grep -Fq 'WINDOWS_CERTIFICATE' "${release_workflow}" || \
  fail "release workflow must require the Windows certificate secret."
grep -Fq 'WINDOWS_CERTIFICATE_PASSWORD' "${release_workflow}" || \
  fail "release workflow must require the Windows certificate password secret."
grep -Fq 'TAURI_SIGNING_PRIVATE_KEY' "${release_workflow}" || \
  fail "release workflow must require the updater private key secret."
grep -Fq 'TAURI_SIGNING_PRIVATE_KEY_PASSWORD' "${release_workflow}" || \
  fail "release workflow must require the updater private key password secret."
grep -Fq 'TAURI_UPDATER_PUBLIC_KEY' "${release_workflow}" || \
  fail "release workflow must require the updater public key variable."
grep -Fq 'npm run release:config' "${release_workflow}" || \
  fail "release workflow must generate a release-only Tauri config."
grep -Fq 'src-tauri/tauri.release.conf.json' "${release_workflow}" || \
  fail "release workflow must build with the generated release config."
grep -Fq 'scripts/release-verify.sh' "${release_workflow}" || \
  fail "release workflow must run repository release verification first."

grep -Fq 'WINDOWS_CERTIFICATE' "${release_doc}" || \
  fail "docs/release.md must document the Windows certificate secret."
grep -Fq 'WINDOWS_CERTIFICATE_PASSWORD' "${release_doc}" || \
  fail "docs/release.md must document the Windows certificate password secret."
grep -Fq 'TAURI_SIGNING_PRIVATE_KEY' "${release_doc}" || \
  fail "docs/release.md must document the updater private key secret."
grep -Fq 'TAURI_SIGNING_PRIVATE_KEY_PASSWORD' "${release_doc}" || \
  fail "docs/release.md must document the updater private key password secret."
grep -Fq 'TAURI_UPDATER_PUBLIC_KEY' "${release_doc}" || \
  fail "docs/release.md must document the updater public key variable."
grep -Fq 'bash scripts/release-verify.sh' "${release_doc}" || \
  fail "docs/release.md must document the release verification command."
grep -Fq 'npm run release:config' "${release_doc}" || \
  fail "docs/release.md must document the release-config generation command."
grep -Fq 'winget/manifests' "${release_doc}" || \
  fail "docs/release.md must document the winget manifest path."

grep -Fq "InstallerUrl: https://github.com/xiongxianfei/space-sift/releases/download/v${package_version}/" "${winget_installer_manifest}" || \
  fail "winget installer manifest must point at the GitHub release path for v${package_version}."
grep -Fq 'InstallerSha256: REPLACE_WITH_RELEASE_SHA256' "${winget_installer_manifest}" || \
  fail "winget installer manifest must contain the explicit SHA256 replacement placeholder."

if [[ -n "${GITHUB_REF_NAME:-}" ]]; then
  expected_tag="v${package_version}"
  [[ "${GITHUB_REF_NAME}" == "${expected_tag}" ]] || \
    fail "tag ${GITHUB_REF_NAME} does not match package version ${package_version}."
fi

echo "Release verification passed."
echo "Version: ${package_version}"
echo "Reminder: public releases must use signed Windows artifacts and refreshed winget hashes."
