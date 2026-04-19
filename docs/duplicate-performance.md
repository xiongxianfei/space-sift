# Duplicate Analysis Performance Validation

This document records how to validate duplicate-analysis performance without
weakening the duplicate-safety contract.

## Guardrails

- Only fully verified full-hash matches may be presented as duplicate groups.
- Local fixed-volume roots use the optimized bounded-worker policy.
- Removable and other non-primary roots stay on the conservative serial policy.
- Remote or UNC roots are skipped and reported instead of being content-hashed.
- Placeholder-style Windows file attributes are treated as unsafe for duplicate
  hashing and must be skipped or reported instead of silently hydrated.
- Validation must record when a path class or placeholder-sensitive fixture is
  unavailable locally instead of pretending it was tested.

## Maintainer command

Run the `duplicates-core` example from the repo root:

```powershell
$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"
cargo run --release --manifest-path src-tauri/Cargo.toml -p duplicates-core --example measure_duplicates -- <path> --repeat 2
```

Useful options:

- `--repeat 2`: capture a cold run and an immediate warm rerun against the same
  in-process cache.
- `--partial-hash-bytes N`: vary the staged partial-hash window if a benchmark
  fixture needs a different threshold for investigation.

The example prints:

- `elapsed_ms`
- `candidate_count`
- `validated_candidate_count`
- `partial_hash_candidate_count`
- `full_hash_candidate_count`
- `cache_lookup_count`
- `cache_hit_count`
- `cache_miss_count`
- `cache_write_count`
- `partial_hash_bytes_read`
- `full_hash_bytes_read`
- `progress_event_count`
- `cancel_to_stop_ms`
- `terminal_state`

Interpretation:

- A healthy warm rerun should keep the same duplicate-group and issue totals as
  the cold run while driving content reads and cache writes toward zero.
- Local fixed-volume runs should do the real hashing work only on the cold pass,
  then reuse cached partial and full hashes on the immediate warm rerun.
- Remote or UNC roots should report issues with zero validated/hash-stage
  candidates and zero content bytes read because the local-only contract blocks
  network-backed hashing.
- If placeholder-sensitive files are present, they should surface as issues or
  conservative skips, not as silent content reads.

## 2026-04-16 validation snapshot

Environment:

- primary large local fixed-volume fixture: `C:\Users\xiongxianfei\.gradle\caches`
- non-primary fallback fixture: `\\localhost\D$\Data\20260415-space-sift`
- available storage classes on this machine: local fixed-volume NVMe only
- placeholder-sensitive path status: `C:\Users\xiongxianfei\OneDrive` exists,
  but it only contained `desktop.ini`, so no real placeholder-backed duplicate
  candidate set was available for validation

Representative large local fixed-volume run:

```text
path: C:\Users\xiongxianfei\.gradle\caches
run 1 elapsed_ms: 24800
run 2 elapsed_ms: 7304
candidate_count: 254065
validated_candidate_count: 248308
partial_hash_candidate_count: 5423
full_hash_candidate_count: 248170
run 1 groups: 23167
run 2 groups: 23167
run 1 issues: 3410
run 2 issues: 3410
run 1 partial_hash_bytes_read: 355401728
run 1 full_hash_bytes_read: 8406353406
run 2 partial_hash_bytes_read: 0
run 2 full_hash_bytes_read: 0
run 1 cache_write_count: 250183
run 2 cache_write_count: 0
```

Representative non-primary UNC fallback run:

```text
path: \\localhost\D$\Data\20260415-space-sift
elapsed_ms: 13
candidate_count: 27130
groups: 0
issues: 21723
validated_candidate_count: 0
partial_hash_candidate_count: 0
full_hash_candidate_count: 0
cache_lookup_count: 0
cache_write_count: 0
partial_hash_bytes_read: 0
full_hash_bytes_read: 0
terminal_state: completed
```

Observed conclusions:

- The local fixed-volume path now benefits from bounded hashing concurrency plus
  staged cache reuse without weakening the full-hash trust boundary.
- The warm rerun on the large local fixture reused cached partial and full
  hashes successfully: duplicate groups and issues stayed stable while content
  reads dropped to zero.
- The remote or UNC fallback policy remains intentionally conservative: the
  example enumerated candidates but did not hash remote content, and it reported
  local-only issues instead.
- Placeholder-sensitive validation remains explicitly unverified on this
  machine because no usable placeholder-backed duplicate fixture was available.
