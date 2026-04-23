# M1 Restore-Context Seam

## Why this change exists

`M1` exists to add the smallest durable boundary needed for later startup
restoration without guessing from scan history or session-only UI state. The
approved spec and architecture require a local, additive restore-context record
before the shell can safely resolve a non-`Overview` startup workspace.

## What changed

- `app-db` now creates and maintains a singleton `workspace_restore_context`
  table with only the approved durable fields:
  `schema_version`, `last_workspace`, `last_opened_scan_id`, and `updated_at`.
- `HistoryStore` now exposes `load_workspace_restore_context()` and
  `save_workspace_restore_context(...)`.
- `commands::shell` now exposes `get_workspace_restore_context` and
  `save_workspace_restore_context`.
- The TypeScript client and shared types now include the matching restore-context
  contract so later shell milestones can consume it without reopening the bridge.
- Existing workflow test doubles now implement the new client methods so the
  frontend still builds cleanly.

## Important constraints preserved

- Invalid or unsupported restore-context rows are treated as `no restore
  context`, matching the approved fail-safe compatibility rule.
- No visible shell, startup resolver, or auto-switch behavior changed in this
  milestone.
- No duplicate-analysis result persistence, cleanup-preview persistence, or
  backend workflow behavior was added.

## Verification summary

- `cargo test --manifest-path src-tauri/Cargo.toml -p app-db workspace_restore_context`
- `cargo test --manifest-path src-tauri/Cargo.toml workspace_restore_context_command_boundary`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `npm run build`
