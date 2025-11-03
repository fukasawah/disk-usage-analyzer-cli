//! Disk Usage CLI (dua) - Main binary entry point

use dua::cli::args::{Command, parse_args};
use dua::cli::output::format_json;
use dua::models::ProgressSnapshot;
use dua::services::aggregate::{SortBy, get_immediate_children, sort_and_limit};
use dua::services::format::format_size;
use dua::{ScanOptions, SizeBasis, StrategyKind};
use std::process;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

fn main() {
    // Initialize logger (controlled by RUST_LOG environment variable)
    // Example: RUST_LOG=debug dua scan /path
    env_logger::init();

    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_help();
        return;
    }

    match args[1].as_str() {
        "--help" | "-h" => {
            print_help();
            return;
        }
        "--version" | "-v" => {
            print_version();
            return;
        }
        _ => {}
    }

    // Parse arguments
    let cli_args = match parse_args(&args) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Error: {e}");
            eprintln!("Use --help for usage information");
            process::exit(2);
        }
    };

    // Execute command
    let exit_code = match &cli_args.command {
        Command::Scan(scan_args) => handle_scan(scan_args),
        Command::View(view_args) => handle_view(view_args),
    };

    process::exit(exit_code);
}

#[allow(clippy::too_many_lines)]
fn handle_scan(args: &dua::cli::args::ScanArgs) -> i32 {
    // Snapshot is required for scan
    let snapshot_path = if let Some(ref path) = args.snapshot {
        path.clone()
    } else {
        eprintln!("Error: --snapshot is required for scan command");
        eprintln!("Example: dua scan /usr --snapshot usr.parquet");
        return 2;
    };

    // Parse basis
    let basis = match args.basis.as_str() {
        "physical" => SizeBasis::Physical,
        "logical" => SizeBasis::Logical,
        _ => {
            eprintln!("Invalid basis: {}. Use 'physical' or 'logical'", args.basis);
            return 2;
        }
    };

    // Create scan options
    let mut opts = ScanOptions {
        basis,
        max_depth: args.max_depth,
        ..ScanOptions::default()
    };

    if let Some(label) = args.strategy_override.as_deref() {
        match StrategyKind::from_str(label) {
            Ok(kind) => opts.strategy_override = Some(kind),
            Err(err) => {
                eprintln!("Error: {err}");
                return 2;
            }
        }
    } else if args.legacy_traversal {
        opts.strategy_override = Some(StrategyKind::Legacy);
    }

    let interval_override = if let Some(interval_secs) = args.progress_interval_secs {
        opts.progress_interval = Duration::from_secs(interval_secs);
        true
    } else {
        false
    };

    if interval_override {
        opts.progress_byte_trigger = u64::MAX;
    }

    if !args.quiet {
        opts.progress_notifier = Some(Arc::new(move |snapshot: &ProgressSnapshot| {
            #[allow(clippy::cast_precision_loss)]
            let elapsed_secs = snapshot.timestamp_ms as f64 / 1000.0;
            let processed = format_size(snapshot.processed_bytes);
            let entries = snapshot.processed_entries;
            let throughput_suffix = snapshot
                .recent_throughput_bytes_per_sec
                .map(|bps| format!(", throughput ~{}/s", format_size(bps)))
                .unwrap_or_default();
            let completion_suffix = snapshot
                .estimated_completion_ratio
                .map(|r| format!(", completion {:.0}%", r * 100.0))
                .unwrap_or_default();

            eprintln!(
                "[{elapsed_secs:6.1}s] {entries} entries, {processed} processed{throughput_suffix}{completion_suffix}",
            );
        }));

        eprintln!("Scanning: {}", args.path);
    }

    let summary = match dua::scan_summary(&args.path, &opts) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: {e}");
            return match e {
                dua::Error::InvalidInput(_) => 2,
                dua::Error::PartialFailure { .. } => 3,
                _ => 4,
            };
        }
    };

    if !args.quiet {
        eprintln!("Found {} entries", summary.entries.len());
        eprintln!("Saving snapshot to: {snapshot_path}");
    }

    // Create snapshot metadata
    let meta = dua::models::SnapshotMeta {
        scan_root: summary.root.clone(),
        started_at: format!("{:?}", summary.started_at),
        finished_at: format!("{:?}", summary.finished_at),
        size_basis: args.basis.clone(),
        hardlink_policy: "dedupe".to_string(),
        excludes: vec![],
        strategy: summary.strategy.to_string(),
    };

    // Save snapshot
    if let Err(e) =
        dua::io::snapshot::write_snapshot(&snapshot_path, &meta, &summary.entries, &summary.errors)
    {
        eprintln!("Error: Failed to save snapshot: {e}");
        return 4;
    }

    if !args.quiet {
        eprintln!(
            "Snapshot saved: {} ({} entries)",
            snapshot_path,
            summary.entries.len()
        );
    }

    // Return appropriate exit code
    if summary.errors.is_empty() {
        0 // Success
    } else {
        3 // Partial failure
    }
}

fn handle_view(args: &dua::cli::args::ViewArgs) -> i32 {
    // Parse sort
    let sort_by = match args.sort.as_str() {
        "size" => SortBy::Size,
        "files" => SortBy::Files,
        "dirs" => SortBy::Dirs,
        _ => {
            eprintln!("Invalid sort: {}. Use 'size'", args.sort);
            return 2;
        }
    };

    // Read snapshot
    let (meta, all_entries, errors) = match dua::io::snapshot::read_snapshot(&args.from_snapshot) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Error reading snapshot: {e}");
            return 4;
        }
    };

    // Determine root path and depth for filtering
    let (display_root, parent_depth) = if let Some(ref drill_path) = args.path {
        // Find the entry for this path to get its depth
        let entry = all_entries.iter().find(|e| e.path == *drill_path);
        if let Some(e) = entry {
            (drill_path.clone(), e.depth)
        } else {
            eprintln!("Error: Path '{drill_path}' not found in snapshot");
            return 2;
        }
    } else {
        (meta.scan_root.clone(), 0)
    };

    let strategy = StrategyKind::from_str(&meta.strategy).unwrap_or(StrategyKind::Legacy);

    // Get immediate children of the target path
    let mut entries = get_immediate_children(&all_entries, &display_root, parent_depth);

    // Sort and limit
    entries = sort_and_limit(entries, sort_by, Some(args.top));

    // Create a summary-like structure for output
    let summary = dua::Summary {
        root: display_root,
        entries: vec![], // Not used for view
        errors,
        started_at: std::time::SystemTime::UNIX_EPOCH, // Placeholder
        finished_at: std::time::SystemTime::UNIX_EPOCH, // Placeholder
        strategy,
        progress: Vec::new(),
    };

    // Output
    if args.json {
        let json = format_json(&summary, &entries);
        println!("{json}");
    } else {
        use dua::cli::output::{AdaptivePreviewStrategy, format_text_with_all_entries};
        format_text_with_all_entries(
            &summary,
            &entries,
            &all_entries,
            &AdaptivePreviewStrategy::default(),
        );
    }

    0
}

fn print_help() {
    println!("Disk Usage CLI (dua) - Analyze disk usage for directory trees");
    println!();
    println!("USAGE:");
    println!("    dua scan <PATH> --snapshot <FILE> [OPTIONS]");
    println!("    dua view <SNAPSHOT> [OPTIONS]");
    println!();
    println!("COMMANDS:");
    println!("    scan      Traverse a path, aggregate usage, and persist a snapshot");
    println!("    view      Read a snapshot and display aggregates instantly");
    println!();
    println!("GLOBAL OPTIONS:");
    println!("    -h, --help                 Show this help message");
    println!("    -v, --version              Show version information");
    println!();
    println!("SCAN OPTIONS:");
    println!("    --snapshot <FILE>         Save results to a Parquet snapshot (required)");
    println!("    --basis <TYPE>            Size basis: physical (default) or logical");
    println!("    --max-depth <N>           Limit traversal depth (default: unlimited)");
    println!("    --legacy-traversal        Force the legacy traversal backend");
    println!(
        "    --strategy <NAME>         Override strategy: windows|posix|legacy (aliases: ntfs, unix)"
    );
    println!("    --progress-interval <S>   Emit progress updates every S seconds (default: 2)");
    println!("    --quiet                   Suppress non-error output");
    println!();
    println!("VIEW OPTIONS:");
    println!("    --path <SUBDIR>           Focus on a path inside the snapshot");
    println!("    --top <K>                 Show top K entries (default: 10)");
    println!("    --sort <FIELD>            Sort by size|files|dirs (default: size)");
    println!("    --json                    Emit machine-readable output");
    println!();
    println!("WORKFLOW:");
    println!("    1. Capture snapshot:  dua scan /usr --snapshot /tmp/usr.parquet");
    println!("    2. Inspect quickly:   dua view /tmp/usr.parquet --sort files");
    println!("    3. Deep dive:         dua view /tmp/usr.parquet --path /usr/share --top 20");
    println!();
    println!("EXAMPLES:");
    println!("    dua scan /home --progress-interval 1 --snapshot home.parquet");
    println!("    dua scan /data --strategy posix --snapshot data.parquet");
    println!("    dua view home.parquet --path /home/user/Downloads --json");
}

fn print_version() {
    const VERSION: &str = env!("CARGO_PKG_VERSION");
    const GIT_HASH: &str = env!("GIT_HASH");
    const GIT_DATE: &str = env!("GIT_DATE");
    const BUILD_TARGET: &str = env!("BUILD_TARGET");

    println!("dua {VERSION}");
    println!("Commit: {GIT_HASH} ({GIT_DATE})");
    println!("Target: {BUILD_TARGET}");

    #[cfg(debug_assertions)]
    println!("Build: debug");
    #[cfg(not(debug_assertions))]
    println!("Build: release");
}
