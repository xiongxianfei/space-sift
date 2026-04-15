# Space Sift MVP Foundation

## Status

- approved

## Goal and context

`Space Sift` is a Windows 11 desktop app for big-file discovery, duplicate
cleanup, and safe space reclamation. This spec defines the contract for the
Milestone 1 foundation release: a bootable branded desktop shell that explains
the product direction and safety model without claiming unfinished scan or
cleanup features already work.

Related plan:
- `docs/plans/2026-04-15-space-sift-win11-mvp.md`

## Examples

### Example 1: first launch

Given a contributor runs the documented desktop development command on a
supported Windows 11 machine, when the app starts, then a single `Space Sift`
window opens and the landing screen explains the product purpose.

### Example 2: safety-first messaging

Given a user opens the landing screen, when they review the primary content,
then they can see that the product is designed to keep the normal UI
unprivileged and to prefer the Recycle Bin before permanent deletion.

### Example 3: unfinished features are not misrepresented

Given scan, duplicate discovery, and cleanup execution are not yet implemented,
when the user views the landing screen, then any visible entry points for those
capabilities are clearly marked as planned and are not executable destructive
actions.

## Inputs and outputs

Inputs:
- app launch through the documented desktop development command
- user viewing the initial landing screen

Outputs:
- a branded desktop window
- explanatory content about the planned feature set
- non-destructive UI affordances for not-yet-built capabilities

## Requirements

- R1: On supported Windows 11 development machines, the documented desktop
  launch command MUST open a single application window titled `Space Sift`.
- R2: The initial landing screen MUST identify `Space Sift` as a disk-space
  recovery tool focused on big-file discovery, duplicate cleanup, and safe
  cleanup workflows.
- R3: The initial landing screen MUST communicate both of these safety
  principles:
  - the normal app UI runs without administrator elevation
  - Recycle Bin deletion is the default safety path before permanent deletion
- R4: The landing screen MUST present the planned capability areas for:
  - large-file discovery
  - duplicate detection
  - cleanup rules
- R5: If a capability is not implemented yet, the UI MUST label it as planned,
  coming soon, or otherwise unavailable. The UI MUST NOT imply that scan or
  deletion workflows already work in Milestone 1.
- R6: Any visible action controls related to not-yet-implemented destructive or
  scan flows MUST be disabled or otherwise non-executable in Milestone 1.
- R7: Launching the Milestone 1 shell MUST NOT require the contributor to run
  the entire app as administrator.
- R8: The landing screen MUST remain readable without network access and MUST
  not depend on a cloud account or remote API to render its initial content.

## Invariants

- The Milestone 1 app is informational and non-destructive.
- The initial screen does not mutate the filesystem.
- The initial screen does not require sign-in or internet access.

## Error handling and boundary behavior

- E1: If future capability areas are shown before implementation, they MUST
  stay visibly unavailable rather than failing after a user click.
- E2: If the app is launched in an environment without admin rights, the normal
  landing screen MUST still be usable.
- E3: The initial experience MUST avoid hard-coded messaging that promises the
  NTFS fast path is already available in v1.

## Compatibility and migration

- C1: This Milestone 1 foundation targets Windows 11 only.
- C2: Later milestones may replace the landing-screen placeholders with working
  features, but they SHOULD preserve the explicit safety model introduced here.

## Observability expectations

- O1: The project MUST provide a desktop development command that maintainers
  can run locally for the Milestone 1 shell.
- O2: The frontend test suite MUST cover the branded landing screen and the
  disabled/planned-state behavior for unfinished capabilities.

## Edge cases

- Edge 1: Offline launch still shows the landing content.
- Edge 2: A non-admin user session still launches the landing shell.
- Edge 3: Placeholder actions cannot trigger a scan or delete path.

## Non-goals

- Implementing real scan execution
- Implementing duplicate detection
- Implementing cleanup execution
- Implementing an NTFS metadata fast path
- Prompting for elevation during normal app startup

## Acceptance criteria

- A reviewer can launch the Milestone 1 shell and see `Space Sift` branding,
  product-purpose messaging, and the two core safety principles.
- A reviewer can see planned capability areas for large-file discovery,
  duplicate detection, and cleanup without being able to trigger destructive or
  misleading unfinished actions.
- Automated frontend tests cover the initial landing screen contract.
