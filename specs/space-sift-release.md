# Space Sift Release Hardening

## Status

- approved

## Goal and context

`Space Sift` needs a repeatable Windows release path before the MVP can be
treated as publicly shippable. This spec defines the repository-side contract
for Milestone 6: release metadata must stay in sync, the repo must contain a
real tag-driven GitHub Release workflow for signed Windows artifacts, and the
checked-in `winget` files must stay versioned with the application.

Related plan:
- `docs/plans/2026-04-15-space-sift-win11-mvp.md`

## Examples

### Example 1: maintainer verifies release readiness locally

Given a maintainer is preparing `v0.1.0`, when they run the documented release
verification command from a clean working tree, then the command confirms the
version is synchronized across the app metadata, release docs, workflow, and
checked-in `winget` manifests.

### Example 2: GitHub tag triggers Windows release packaging

Given a maintainer pushes a tag like `v0.1.0`, when the release workflow runs,
then it first runs repository release-readiness checks and then starts a
Windows packaging job that uses Tauri release tooling to upload artifacts to a
GitHub Release.

### Example 3: signing prerequisites are missing

Given a public tag build starts without the required signing secrets, when the
Windows release job reaches its signing preflight, then the workflow fails
before any unsigned public release artifacts are uploaded.

### Example 4: winget manifest version drift

Given the app version changes in `package.json` but the checked-in `winget`
manifests still point at an older version, when release verification runs, then
the verification fails and names the mismatched manifest path.

## Inputs and outputs

Inputs:
- release verification from the repository root
- a tag push like `v0.1.0`
- repository metadata files and release configuration files

Outputs:
- a passed or failed release-readiness result
- a Windows release workflow capable of producing signed artifacts
- checked-in `winget` manifest files that match the current app version
- maintainer-facing release documentation

## Requirements

- R1: `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, and
  the checked-in `winget` manifest set for the current version MUST agree on
  the release version string.
- R2: The repository MUST contain a tag-driven release workflow under
  `.github/workflows/` that triggers on tags matching `v*`.
- R3: The release workflow MUST run a release-readiness step before packaging
  artifacts.
- R4: The release workflow MUST build Windows artifacts on `windows-latest`
  using Tauri release tooling.
- R5: The release workflow MUST require both of these secret groups before
  publishing public artifacts:
  - a Windows code-signing certificate and password
  - a Tauri updater signing private key and password
- R6: If the signing prerequisites in R5 are missing, the public release job
  MUST fail before uploading unsigned artifacts.
- R7: The repository MUST generate a release-only Tauri config that enables
  updater artifact generation for release builds and injects the updater public
  key without forcing normal local builds to depend on release secrets.
- R8: `src-tauri/tauri.conf.json` MUST include Windows bundle settings that
  support a public installer path:
  - a human-readable publisher
  - a release homepage
  - a short description
  - `allowDowngrades` disabled
  - an explicit WebView2 install mode
- R9: The repository MUST include a maintained release runbook at
  `docs/release.md` covering:
  - required secrets and their purpose
  - the local release-readiness command
  - the tag-and-publish flow
  - how to update the checked-in `winget` manifests
  - what parts of the release process are still manual
- R10: The repository MUST include a checked-in `winget` manifest set for the
  current version under `winget/manifests/`.
- R11: The `winget` manifest set in R10 MUST include:
  - a version manifest
  - a default locale manifest
  - an installer manifest
- R12: The installer manifest in R11 MUST target the GitHub Release asset path
  for the current version and MUST clearly reserve a field for the final
  release SHA256.
- R13: `scripts/release-verify.sh` MUST fail on:
  - a dirty working tree
  - version drift across release metadata
  - a tag name that does not match the current version
  - missing release docs
  - missing or version-skewed `winget` manifests
  - a release workflow that does not reference Tauri release tooling or signing
    prerequisites

## Invariants

- Public releases are signed or blocked.
- Release metadata does not drift silently.
- `winget` files in the repository track the app version intentionally.

## Error handling and boundary behavior

- E1: Local unsigned builds for maintainer testing may still exist, but the
  public release workflow MUST fail closed without signing inputs and updater
  key material.
- E2: Release verification MUST report the first missing metadata or manifest
  dependency clearly enough that a maintainer can repair it without guessing.
- E3: A tag push for `vX.Y.Z` MUST be treated as invalid if the app metadata is
  still at a different version.

## Compatibility and migration

- C1: This release path targets Windows 11 for public artifacts.
- C2: The checked-in `winget` manifests may contain placeholder SHA256 values
  until a real release is cut, but the placeholders MUST be explicit and easy
  to replace during release preparation.
- C3: Future updater UI work may consume the generated updater artifacts, but
  this milestone only requires generating and publishing them.

## Observability expectations

- O1: The repository test suite MUST include coverage for release-config drift.
- O2: The release workflow file MUST be readable enough that a maintainer can
  identify the readiness job, the signing gate, and the Tauri packaging step.
- O3: The release runbook MUST identify the exact secret names and manifest
  paths used by the repository.

## Edge cases

- Edge 1: Version metadata matches across app files but the `winget` manifest
  directory name is stale.
- Edge 2: A release tag is pushed without signing secrets configured.
- Edge 3: The generated release-only Tauri config stops enabling updater
  artifacts or stops carrying the updater public key.
- Edge 4: The release workflow exists but regresses to a docs-only placeholder
  that never packages Windows installers.

## Non-goals

- Generating a real certificate
- Publishing a live GitHub Release from the local repository state
- Submitting a live `winget` pull request from this milestone
- Building macOS or Linux public installers
- Adding in-app updater UI

## Acceptance criteria

- A maintainer can run release verification locally and catch version or
  manifest drift before tagging.
- A reviewer can inspect the release workflow and see a real Windows packaging
  job with signing gates instead of the earlier placeholder.
- The repository contains version-aligned `winget` manifests and a concrete
  release runbook for the MVP.
