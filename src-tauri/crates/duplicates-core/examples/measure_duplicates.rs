use duplicates_core::{
    measure_duplicate_analysis, CachedHashes, DuplicateAnalysisFailure,
    DuplicateAnalysisMeasurement, DuplicateAnalysisRequest, DuplicateAnalysisState,
    DuplicateCandidate, HashCache, HashCacheKey, MeasuredDuplicateAnalysis,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

fn main() {
    let args = match parse_args(env::args().skip(1).collect()) {
        Ok(args) => args,
        Err(message) => {
            eprintln!("{message}");
            print_usage();
            process::exit(2);
        }
    };

    let candidates = match collect_candidates(&args.path) {
        Ok(candidates) => candidates,
        Err(message) => {
            eprintln!("{message}");
            process::exit(2);
        }
    };

    let cache = MemoryHashCache::default();
    let mut failed = false;
    for run_index in 0..args.repeat {
        if args.repeat > 1 {
            println!("Run {}/{}", run_index + 1, args.repeat);
        }

        let mut request = DuplicateAnalysisRequest::new(
            format!("measure-duplicates-{}", run_index + 1),
            args.path.display().to_string(),
            candidates.clone(),
        );
        request.partial_hash_bytes = args.partial_hash_bytes;

        let measured = measure_duplicate_analysis(&cache, &request, || false);
        print_measurement(&args.path, candidates.len(), &measured);
        failed |= measured.result.is_err();

        if args.repeat > 1 && run_index + 1 < args.repeat {
            println!();
        }
    }

    if failed {
        process::exit(1);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Args {
    path: PathBuf,
    repeat: usize,
    partial_hash_bytes: usize,
}

fn parse_args(args: Vec<String>) -> Result<Args, String> {
    let mut path = None::<PathBuf>;
    let mut repeat = 1_usize;
    let mut partial_hash_bytes = duplicates_core::DEFAULT_PARTIAL_HASH_BYTES;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--repeat" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| String::from("missing value for --repeat"))?;
                repeat = value
                    .parse::<usize>()
                    .map_err(|_| format!("invalid repeat count: {value}"))?;
                if repeat == 0 {
                    return Err(String::from("--repeat must be at least 1"));
                }
            }
            "--partial-hash-bytes" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| String::from("missing value for --partial-hash-bytes"))?;
                partial_hash_bytes = value
                    .parse::<usize>()
                    .map_err(|_| format!("invalid partial-hash-bytes value: {value}"))?;
            }
            value if value.starts_with("--") => {
                return Err(format!("unknown flag: {value}"));
            }
            value => {
                if path.is_some() {
                    return Err(format!("unexpected extra path argument: {value}"));
                }
                path = Some(PathBuf::from(value));
            }
        }

        index += 1;
    }

    Ok(Args {
        path: path.ok_or_else(|| String::from("missing duplicate-analysis path"))?,
        repeat,
        partial_hash_bytes,
    })
}

fn print_usage() {
    eprintln!(
        "usage: cargo run -p duplicates-core --example measure_duplicates -- <path> [--repeat N] [--partial-hash-bytes N]"
    );
}

fn collect_candidates(path: &Path) -> Result<Vec<DuplicateCandidate>, String> {
    let metadata =
        fs::symlink_metadata(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;

    if metadata.is_file() {
        return Ok(vec![DuplicateCandidate {
            path: path.display().to_string(),
            size_bytes: metadata.len(),
        }]);
    }

    if !metadata.is_dir() {
        return Err(format!(
            "path is neither a regular file nor a directory: {}",
            path.display()
        ));
    }

    let mut candidates = Vec::new();
    let mut directories = vec![path.to_path_buf()];

    while let Some(current) = directories.pop() {
        let entries = fs::read_dir(&current)
            .map_err(|error| format!("failed to enumerate {}: {error}", current.display()))?;
        for entry in entries {
            let entry = entry.map_err(|error| {
                format!("failed to read directory entry under {}: {error}", current.display())
            })?;
            let entry_path = entry.path();
            let entry_metadata = fs::symlink_metadata(&entry_path)
                .map_err(|error| format!("failed to read {}: {error}", entry_path.display()))?;

            if entry_metadata.is_dir() {
                directories.push(entry_path);
            } else if entry_metadata.is_file() {
                candidates.push(DuplicateCandidate {
                    path: entry_path.display().to_string(),
                    size_bytes: entry_metadata.len(),
                });
            }
        }
    }

    candidates.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(candidates)
}

fn print_measurement(path: &Path, candidate_count: usize, measured: &MeasuredDuplicateAnalysis) {
    let status = match &measured.result {
        Ok(completed) => format!(
            "completed (analysis_id={}, groups={}, issues={})",
            completed.analysis_id,
            completed.groups.len(),
            completed.issues.len()
        ),
        Err(DuplicateAnalysisFailure::Cancelled) => String::from("cancelled"),
        Err(DuplicateAnalysisFailure::InvalidRequest { message }) => {
            format!("invalid_request ({message})")
        }
        Err(DuplicateAnalysisFailure::Internal { message }) => {
            format!("internal_error ({message})")
        }
    };

    println!("path: {}", path.display());
    println!("candidate_count: {candidate_count}");
    println!("status: {status}");
    print_measurement_fields(&measured.measurement);
}

fn print_measurement_fields(measurement: &DuplicateAnalysisMeasurement) {
    println!("elapsed_ms: {}", measurement.elapsed_millis);
    println!(
        "validated_candidate_count: {}",
        measurement.validated_candidate_count
    );
    println!(
        "partial_hash_candidate_count: {}",
        measurement.partial_hash_candidate_count
    );
    println!(
        "full_hash_candidate_count: {}",
        measurement.full_hash_candidate_count
    );
    println!("cache_lookup_count: {}", measurement.cache_lookup_count);
    println!("cache_hit_count: {}", measurement.cache_hit_count);
    println!("cache_miss_count: {}", measurement.cache_miss_count);
    println!("cache_write_count: {}", measurement.cache_write_count);
    println!(
        "partial_cache_lookup_count: {}",
        measurement.partial_cache_lookup_count
    );
    println!(
        "partial_cache_hit_count: {}",
        measurement.partial_cache_hit_count
    );
    println!(
        "partial_cache_miss_count: {}",
        measurement.partial_cache_miss_count
    );
    println!(
        "partial_cache_write_count: {}",
        measurement.partial_cache_write_count
    );
    println!(
        "full_cache_lookup_count: {}",
        measurement.full_cache_lookup_count
    );
    println!("full_cache_hit_count: {}", measurement.full_cache_hit_count);
    println!("full_cache_miss_count: {}", measurement.full_cache_miss_count);
    println!(
        "full_cache_write_count: {}",
        measurement.full_cache_write_count
    );
    println!(
        "partial_hash_bytes_read: {}",
        measurement.partial_hash_bytes_read
    );
    println!("full_hash_bytes_read: {}", measurement.full_hash_bytes_read);
    println!(
        "progress_event_count: {}",
        measurement.progress_event_count
    );
    println!(
        "cancel_to_stop_ms: {}",
        measurement
            .cancel_to_stop_millis
            .map(|value| value.to_string())
            .unwrap_or_else(|| String::from("n/a"))
    );
    println!(
        "terminal_state: {}",
        duplicate_analysis_state_label(&measurement.terminal_state)
    );
}

fn duplicate_analysis_state_label(state: &DuplicateAnalysisState) -> &'static str {
    match state {
        DuplicateAnalysisState::Idle => "idle",
        DuplicateAnalysisState::Running => "running",
        DuplicateAnalysisState::Completed => "completed",
        DuplicateAnalysisState::Cancelled => "cancelled",
        DuplicateAnalysisState::Failed => "failed",
    }
}

#[derive(Default)]
struct MemoryHashCache {
    entries: RefCell<HashMap<HashCacheKey, CachedHashes>>,
}

impl HashCache for MemoryHashCache {
    fn get_cached_hashes(
        &self,
        key: &HashCacheKey,
    ) -> Result<Option<CachedHashes>, DuplicateAnalysisFailure> {
        Ok(self.entries.borrow().get(key).cloned())
    }

    fn save_partial_hash(
        &self,
        key: &HashCacheKey,
        partial_hash: &str,
    ) -> Result<(), DuplicateAnalysisFailure> {
        let mut entries = self.entries.borrow_mut();
        let entry = entries.entry(key.clone()).or_insert(CachedHashes {
            partial_hash: None,
            full_hash: None,
        });
        entry.partial_hash = Some(partial_hash.to_string());
        Ok(())
    }

    fn save_full_hash(
        &self,
        key: &HashCacheKey,
        full_hash: &str,
    ) -> Result<(), DuplicateAnalysisFailure> {
        let mut entries = self.entries.borrow_mut();
        let entry = entries.entry(key.clone()).or_insert(CachedHashes {
            partial_hash: None,
            full_hash: None,
        });
        entry.full_hash = Some(full_hash.to_string());
        Ok(())
    }
}
