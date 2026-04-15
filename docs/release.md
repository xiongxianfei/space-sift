# Release Runbook

This repository now contains the repo-side pieces for Milestone 6 release
hardening: tag-driven GitHub release automation, signing gates, updater
artifact generation, and versioned `winget` manifests.

## Required repository values

The GitHub repository release workflow expects these GitHub values:

- `WINDOWS_CERTIFICATE`
  Base64-encoded `.pfx` certificate file used to code-sign Windows artifacts.
- `WINDOWS_CERTIFICATE_PASSWORD`
  Password for the `.pfx` file above.
- `TAURI_SIGNING_PRIVATE_KEY`
  Tauri updater signing private key for generated updater artifacts.
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
  Password for the Tauri updater signing private key.
- `TAURI_UPDATER_PUBLIC_KEY`
  Public key paired with the updater signing private key. Store this as a
  repository variable unless you have a reason to keep it secret.

The workflow fails before packaging if any of these are missing.

## Local readiness check

From the repository root, run:

```bash
bash scripts/release-verify.sh
```

This verifies:

- clean working tree
- version sync across `package.json`, `src-tauri/tauri.conf.json`,
  `src-tauri/Cargo.toml`, and `winget/manifests/`
- presence of the release runbook and release workflow
- updater-artifact and Windows bundle settings in `src-tauri/tauri.conf.json`
- release workflow references to Tauri packaging, generated release config, and
  signing inputs

## Local build notes

`npm run tauri build` can still be used locally for unsigned maintainer builds.
The checked-in signing script only signs when
`SPACE_SIFT_ENABLE_CODE_SIGNING=true` and the certificate environment variables
are present. Public releases must not rely on unsigned local builds.

To locally verify updater-artifact packaging, set:

- `TAURI_UPDATER_PUBLIC_KEY`
- `TAURI_SIGNING_PRIVATE_KEY`
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`

Then run:

```bash
npm run release:config
npm run tauri build -- --config src-tauri/tauri.release.conf.json
```

## Tag and publish flow

1. Ensure `main` is green and your local checkout is clean.
2. Run `bash scripts/release-verify.sh`.
3. Confirm the repository values above are present in GitHub.
4. Verify the app version in `package.json`, `src-tauri/Cargo.toml`, and
   `src-tauri/tauri.conf.json`.
5. Create and push the version tag:

```bash
git tag v0.1.0
git push origin v0.1.0
```

6. Wait for `.github/workflows/release.yml` to complete.
7. Review the draft GitHub Release and confirm the signed Windows assets.

## Expected asset path

The checked-in installer manifest assumes the default NSIS asset path:

`https://github.com/xiongxianfei/space-sift/releases/download/v<version>/Space%20Sift_<version>_x64-setup.exe`

If the real asset name differs, update the installer manifest before submission.

## Winget maintenance

The checked-in manifests live under:

`winget/manifests/x/xiongxianfei/SpaceSift/<version>/`

Files:

- `xiongxianfei.SpaceSift.yaml`
- `xiongxianfei.SpaceSift.locale.en-US.yaml`
- `xiongxianfei.SpaceSift.installer.yaml`

Before submitting a real release:

1. Replace `InstallerSha256: REPLACE_WITH_RELEASE_SHA256` with the actual
   SHA256 for the shipped installer.
2. Verify the `InstallerUrl` matches the GitHub Release asset name exactly.
3. Submit the manifest to the public `winget-pkgs` repository or update your
   existing package submission flow.

## What is still manual

- provisioning the real signing certificate and updater keys
- setting the real `TAURI_UPDATER_PUBLIC_KEY` repository variable
- cutting the actual release tag
- verifying the installer on a clean Windows 11 machine
- replacing the placeholder `winget` SHA256 with the real release hash
- submitting the public `winget` manifest
