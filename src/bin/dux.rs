//! Disk Usage CLI (dux) - Main binary entry point

use rs_disk_usage::services::aggregate::{get_immediate_children, sort_and_limit, SortBy};
use rs_disk_usage::cli::args::{parse_args, Command};
use rs_disk_usage::cli::output::{format_text, format_json};
use rs_disk_usage::{ScanOptions, SizeBasis, HardlinkPolicy};
use std::process;

fn main() {
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
    let exit_code = match cli_args.command {
        Command::Scan(scan_args) => handle_scan(scan_args),
        Command::Drill(drill_args) => handle_drill(drill_args),
        Command::View(view_args) => handle_view(view_args),
    };
    
    process::exit(exit_code);
}

fn handle_scan(args: rs_disk_usage::cli::args::ScanArgs) -> i32 {
    // Snapshot is required for scan
    let snapshot_path = match args.snapshot {
        Some(path) => path,
        None => {
            eprintln!("Error: --snapshot is required for scan command");
            eprintln!("Example: dux scan /usr --snapshot usr.parquet");
            return 2;
        }
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
    let opts = ScanOptions {
        basis,
        max_depth: args.max_depth,
        hardlink_policy: HardlinkPolicy::Dedupe,
        follow_symlinks: false,
        cross_filesystem: false,
    };
    
    // Perform scan
    if args.verbose || !args.quiet {
        eprintln!("Scanning: {}", args.path);
    }
    
    let summary = match rs_disk_usage::scan_summary(&args.path, &opts) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: {e}");
            return match e {
                rs_disk_usage::Error::InvalidInput(_) => 2,
                rs_disk_usage::Error::PartialFailure { .. } => 3,
                _ => 4,
            };
        }
    };
    
    if args.verbose || !args.quiet {
        eprintln!("Found {} entries", summary.entries.len());
        eprintln!("Saving snapshot to: {}", snapshot_path);
    }
    
    // Create snapshot metadata
    let meta = rs_disk_usage::models::SnapshotMeta {
        scan_root: summary.root.clone(),
        started_at: format!("{:?}", summary.started_at),
        finished_at: format!("{:?}", summary.finished_at),
        size_basis: args.basis.clone(),
        hardlink_policy: "dedupe".to_string(),
        excludes: vec![],
    };
    
    // Save snapshot
    if let Err(e) = rs_disk_usage::io::snapshot::write_snapshot(
        &snapshot_path,
        &meta,
        &summary.entries,
        &summary.errors,
    ) {
        eprintln!("Error: Failed to save snapshot: {e}");
        return 4;
    }
    
    if args.verbose || !args.quiet {
        eprintln!("Snapshot saved: {} ({} entries)", snapshot_path, summary.entries.len());
    }
    
    // Return appropriate exit code
    if !summary.errors.is_empty() {
        3 // Partial failure
    } else {
        0 // Success
    }
}

fn handle_drill(args: rs_disk_usage::cli::args::DrillArgs) -> i32 {
    // Parse basis
    let basis = match args.basis.as_str() {
        "physical" => SizeBasis::Physical,
        "logical" => SizeBasis::Logical,
        _ => {
            eprintln!("Invalid basis: {}. Use 'physical' or 'logical'", args.basis);
            return 2;
        }
    };
    
    // Parse sort
    let sort_by = match args.sort.as_str() {
        "size" => SortBy::Size,
        "files" => SortBy::Files,
        "dirs" => SortBy::Dirs,
        _ => {
            eprintln!("Invalid sort: {}. Use 'size', 'files', or 'dirs'", args.sort);
            return 2;
        }
    };
    
    // Verify that subdir is within root (or use subdir directly)
    // For drill, we simply use the subdir as the new root
    let scan_path = &args.subdir;
    
    // Create scan options
    let opts = ScanOptions {
        basis,
        max_depth: args.max_depth,
        hardlink_policy: HardlinkPolicy::Dedupe,
        follow_symlinks: false,
        cross_filesystem: false,
    };
    
    // Perform scan on the subdirectory
    let summary = match rs_disk_usage::scan_summary(scan_path, &opts) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: {e}");
            return match e {
                rs_disk_usage::Error::InvalidInput(_) => 2,
                rs_disk_usage::Error::PartialFailure { .. } => 3,
                _ => 4,
            };
        }
    };
    
    // Get immediate children
    let mut children = get_immediate_children(&summary.entries, scan_path, 0);
    
    // Sort and limit
    children = sort_and_limit(children, sort_by, Some(args.top));
    
    // Output
    if args.json {
        let json = format_json(&summary, &children);
        println!("{json}");
    } else {
        format_text(&summary, &children);
    }
    
    // Return appropriate exit code
    if !summary.errors.is_empty() {
        3 // Partial failure
    } else {
        0 // Success
    }
}

fn handle_view(args: rs_disk_usage::cli::args::ViewArgs) -> i32 {
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
    let (meta, all_entries, errors) = match rs_disk_usage::io::snapshot::read_snapshot(&args.from_snapshot) {
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
        match entry {
            Some(e) => (drill_path.clone(), e.depth),
            None => {
                eprintln!("Error: Path '{}' not found in snapshot", drill_path);
                return 2;
            }
        }
    } else {
        (meta.scan_root.clone(), 0)
    };
    
    // Get immediate children of the target path
    let mut entries = get_immediate_children(&all_entries, &display_root, parent_depth);
    
    // Sort and limit
    entries = sort_and_limit(entries, sort_by, Some(args.top));
    
    // Create a summary-like structure for output
    let summary = rs_disk_usage::Summary {
        root: display_root,
        entries: vec![],  // Not used for view
        errors,
        started_at: std::time::SystemTime::UNIX_EPOCH,  // Placeholder
        finished_at: std::time::SystemTime::UNIX_EPOCH,  // Placeholder
    };
    
    // Output
    if args.json {
        let json = format_json(&summary, &entries);
        println!("{json}");
    } else {
        use rs_disk_usage::cli::output::{format_text_with_all_entries, AdaptivePreviewStrategy};
        format_text_with_all_entries(&summary, &entries, &all_entries, &AdaptivePreviewStrategy::default());
    }
    
    0
}

fn print_help() {
    println!("Disk Usage CLI (dux) - Analyze disk usage for directory trees");
    println!();
    println!("USAGE:");
    println!("    dux <COMMAND> [OPTIONS]");
    println!();
    println!("COMMANDS:");
    println!("    scan <PATH>              Scan filesystem and save snapshot");
    println!("    view <SNAPSHOT>          View saved snapshot");
    println!("    drill <SNAPSHOT> <PATH>  Drill down into subdirectory");
    println!();
    println!("WORKFLOW:");
    println!("    1. Scan once (slow):     dux scan /usr --snapshot /tmp/usr.parquet");
    println!("    2. View anytime (fast):  dux view /tmp/usr.parquet");
    println!("    3. Drill down (fast):    dux view /tmp/usr.parquet --path /usr/share");
    println!();
    println!("GLOBAL OPTIONS:");
    println!("    --help, -h               Show this help message");
    println!("    --version, -v            Show version information");
    println!();
    println!("SCAN OPTIONS:");
    println!("    --snapshot <FILE>        Save snapshot to Parquet file (required)");
    println!("    --basis <TYPE>           Size basis: physical (default) or logical");
    println!("    --max-depth <N>          Maximum traversal depth (unlimited by default)");
    println!("    --verbose                Show progress during scan");
    println!();
    println!("VIEW/DRILL OPTIONS:");
    println!("    --path <SUBDIR>          Focus on subdirectory (drill down)");
    println!("    --top <K>                Show top K entries (default: 10)");
    println!("    --sort <FIELD>           Sort by: size (default)");
    println!("    --json                   Output in JSON format");
    println!();
    println!("EXAMPLES:");
    println!("    dux scan /home --snapshot home.parquet");
    println!("    dux view home.parquet");
    println!("    dux view home.parquet --path /home/user/Downloads --top 20");
}

fn print_version() {
    println!("dux {}", env!("CARGO_PKG_VERSION"));
}
