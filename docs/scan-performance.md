# Scan Performance Validation

This document records how to validate `Space Sift` scan performance without
changing the app contract.

## Guardrails

- Ordinary scans stay metadata-first.
- Ordinary scans do not hash files or read full file contents.
- Local fixed-volume roots use the optimized Win32 enumeration path.
- Removable, UNC or remote, and unsupported roots stay on the recursive
  fallback path.
- Validation must record when a path class or storage class is unavailable
  locally instead of pretending it was tested.

## Maintainer command

Run the `scan-core` example from the repo root:

```powershell
$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"
cargo run --release --manifest-path src-tauri/Cargo.toml -p scan-core --example measure_scan -- <path> --repeat 2
```

Useful options:

- `--repeat 2`: capture a first run and an immediate warm rerun.
- `--top-items-limit N`: keep the same traversal but change the retained top
  item count if a fixture needs a smaller result payload.

The example prints:

- `elapsed_ms`
- `entries_per_second`
- `describe_path_calls`
- `read_dir_calls`
- `progress_event_count`
- `cancellation_check_count`
- `files_discovered`
- `directories_discovered`
- `bytes_processed`
- `terminal_state`

Interpretation:

- `describe_path_calls` should stay near `1` on optimized local fixed-volume
  scans because the root still goes through the regular root validation path.
- Fallback scans will show much higher `describe_path_calls` because the
  recursive walker still describes each child path separately.
- Use the same tree for a local fixed-root run and a UNC fallback run when you
  want a correctness comparison, but prefer a stable tree for parity checks.

## 2026-04-16 validation snapshot

Environment:

- detected physical disks: `NVMe HFS001TEJ9X101N` and `NVMe ZHITAI TiPlus7100 2TB`
- available storage classes on this machine: local fixed-volume NVMe only
- slower storage class status: no HDD, removable, or slower non-NVMe class was
  available locally during this validation pass
- fallback path class used: UNC loopback through `\\localhost\D$`

Representative optimized local fixed-volume run:

```text
path: C:\Users\xiongxianfei\.gradle\caches
run 1 elapsed_ms: 19340
run 2 elapsed_ms: 5688
run 1 describe_path_calls: 1
run 2 describe_path_calls: 1
read_dir_calls: 167631
files_discovered: 253200
directories_discovered: 167631
```

Stable-tree fallback correctness comparison:

```text
local fixed path: D:\Data\20260415-space-sift
elapsed_ms: 86
describe_path_calls: 1
read_dir_calls: 3353
files_discovered: 25433
directories_discovered: 3353
bytes_processed: 11354184305

fallback UNC path: \\localhost\D$\Data\20260415-space-sift
elapsed_ms: 11706
describe_path_calls: 28786
read_dir_calls: 3353
files_discovered: 25433
directories_discovered: 3353
bytes_processed: 11354184305
```

Observed conclusions:

- The optimized local fixed-volume backend materially reduces explicit metadata
  follow-up work versus the fallback path.
- The fallback UNC path remains correct on a stable tree, but it is
  dramatically slower and should stay a fallback, not the preferred path.
- A live mutable cache tree is not a good parity fixture across backends
  because counts can drift while the tree is changing.
