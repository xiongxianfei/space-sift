use scan_core::{measure_scan_path, MeasuredScan, ScanFailure, ScanRequest};
use std::env;
use std::path::PathBuf;
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

    let mut failed = false;
    for run_index in 0..args.repeat {
        if args.repeat > 1 {
            println!("Run {}/{}", run_index + 1, args.repeat);
        }

        let mut request = ScanRequest::new(&args.path);
        request.top_items_limit = args.top_items_limit;
        let measured = measure_scan_path(&request, || false);
        print_measurement(&args.path, &measured);
        failed |= measured.result.is_err();

        if args.repeat > 1 && run_index + 1 < args.repeat {
            println!();
        }
    }

    if failed {
        process::exit(1);
    }
}

struct Args {
    path: PathBuf,
    repeat: usize,
    top_items_limit: usize,
}

fn parse_args(args: Vec<String>) -> Result<Args, String> {
    let mut path = None::<PathBuf>;
    let mut repeat = 1_usize;
    let mut top_items_limit = scan_core::DEFAULT_TOP_ITEMS_LIMIT;
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
            "--top-items-limit" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| String::from("missing value for --top-items-limit"))?;
                top_items_limit = value
                    .parse::<usize>()
                    .map_err(|_| format!("invalid top-items limit: {value}"))?;
                if top_items_limit == 0 {
                    return Err(String::from("--top-items-limit must be at least 1"));
                }
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
        path: path.ok_or_else(|| String::from("missing scan path"))?,
        repeat,
        top_items_limit,
    })
}

fn print_usage() {
    eprintln!(
        "usage: cargo run -p scan-core --example measure_scan -- <path> [--repeat N] [--top-items-limit N]"
    );
}

fn print_measurement(path: &PathBuf, measured: &MeasuredScan) {
    let status = match &measured.result {
        Ok(completed) => format!(
            "completed (scan_id={}, files={}, directories={})",
            completed.scan_id, completed.total_files, completed.total_directories
        ),
        Err(ScanFailure::Cancelled) => String::from("cancelled"),
        Err(ScanFailure::InvalidRoot { message }) => format!("invalid_root ({message})"),
        Err(ScanFailure::Internal { message }) => format!("internal_error ({message})"),
    };

    println!("path: {}", path.display());
    println!("status: {status}");
    println!("elapsed_ms: {}", measured.measurement.elapsed_millis);
    println!(
        "entries_per_second: {}",
        measured.measurement.entries_per_second
    );
    println!(
        "describe_path_calls: {}",
        measured.measurement.describe_path_calls
    );
    println!("read_dir_calls: {}", measured.measurement.read_dir_calls);
    println!(
        "progress_event_count: {}",
        measured.measurement.progress_event_count
    );
    println!(
        "cancellation_check_count: {}",
        measured.measurement.cancellation_check_count
    );
    println!(
        "cancel_to_stop_ms: {}",
        measured
            .measurement
            .cancel_to_stop_millis
            .map(|value| value.to_string())
            .unwrap_or_else(|| String::from("n/a"))
    );
    println!("files_discovered: {}", measured.measurement.files_discovered);
    println!(
        "directories_discovered: {}",
        measured.measurement.directories_discovered
    );
    println!("bytes_processed: {}", measured.measurement.bytes_processed);
    println!(
        "terminal_state: {:?}",
        measured.measurement.terminal_state
    );
}
