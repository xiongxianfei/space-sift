# Space Sift Release Hardening Test Spec

This test spec maps `specs/space-sift-release.md` to concrete repository tests
and release-readiness checks.

## Test cases

| Test ID | Spec item | Test type | Scenario | Expected result |
| --- | --- | --- | --- | --- |
| T1 | R1, R10, R11, Edge 1 | repository config | Read `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, and the checked-in `winget` manifests | All declared versions match and the expected manifest files exist under the current version directory |
| T2 | R2, R3, R4, R5, R6, R13, Edge 2, Edge 4 | repository config | Read `.github/workflows/release.yml` | The workflow triggers on `v*`, contains a readiness phase, a Windows packaging job, Tauri release tooling, and an explicit signing-secret gate |
| T3 | R7, R8, Edge 3 | repository config | Read `src-tauri/tauri.conf.json` and `scripts/write-tauri-release-config.mjs` | The base Tauri config keeps the Windows bundle metadata/settings required for public release and the release-config writer injects updater artifacts plus the updater public key for release packaging |
| T4 | R9, O3 | repository config | Read `docs/release.md` | The runbook documents the secret names, release-readiness command, tag flow, and `winget` manifest maintenance path |
| T5 | R12 | repository config | Read the installer manifest | The installer URL points at the GitHub Releases path for the current version and the SHA256 field is explicitly marked for replacement during release prep |
| T6 | R13, E2, E3 | script smoke | Run `bash scripts/release-verify.sh` from a clean working tree | The script passes only when versions, docs, workflow, tag semantics, and `winget` metadata are aligned |

## Coverage by requirement

| Requirement | Covered by |
| --- | --- |
| R1 | T1, T6 |
| R2 | T2 |
| R3 | T2 |
| R4 | T2 |
| R5 | T2 |
| R6 | T2 |
| R7 | T2, T3, T6 |
| R8 | T3 |
| R9 | T4, T6 |
| R10 | T1, T6 |
| R11 | T1 |
| R12 | T5 |
| R13 | T2, T6 |
| O1 | T1, T2, T3, T4, T5 |
| O2 | T2 |
| O3 | T4 |

## Fixtures and scenarios

- Repository-config tests may read checked-in text files directly.
- The release verification smoke test should run from the repository root on a
  clean working tree.
- The `winget` fixture path should use the current version declared in
  `package.json`.

## What not to test

- Live certificate import
- Live GitHub Release uploads
- Live `winget` submission
- End-user updater UI behavior

## Gaps and follow-up

- A real public release still requires repository secrets and a real tag.
- Manual installer smoke testing on a clean Windows 11 VM remains part of the
  broader milestone acceptance pass even after repository-config checks pass.
