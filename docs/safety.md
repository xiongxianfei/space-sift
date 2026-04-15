# Space Sift Safety Notes

`Space Sift` is a cleanup tool, so the product contract is intentionally more
conservative than a typical disk browser.

## Current guarantees

- The normal desktop UI stays unprivileged by default.
- Cleanup is preview-first. The frontend never submits arbitrary delete paths
  directly to the backend execution command.
- The default cleanup mode is `recycle`, which attempts to move files to the
  Windows Recycle Bin.
- Permanent delete is available only through a separate explicit confirmation
  toggle.
- Cleanup is file-only in this milestone. Directories are not removed.
- Cleanup candidates stay scoped to the loaded scan root and the stored scan
  file-entry payload.
- Protected Windows paths fail closed in the current milestone instead of
  elevating the whole app.

## Current cleanup sources

The built-in rule catalog is intentionally narrow:

- `temp-folder-files`
- `download-partials`

The cleanup preview can also include duplicate delete candidates derived from
the current duplicate keep/delete selection.

## Revalidation rules

Before executing a cleanup action, the backend revalidates that each file:

- still exists
- is still a regular file
- still matches the previewed size and metadata snapshot
- is still within the current scan root

If any check fails, that file is skipped and reported as a failed cleanup
entry rather than being deleted.

## User expectations

- Run a fresh scan after cleanup. The stored scan result is stale once files
  have moved or been deleted.
- Treat permanent delete as an advanced path, not a routine workflow.
- Do not expect protected-path cleanup to elevate automatically in this
  milestone.
