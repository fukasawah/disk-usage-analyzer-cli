//! CLI argument parsing

#[derive(Debug, Clone)]
pub struct CliArgs {
    pub command: Command,
}

#[derive(Debug, Clone)]
pub enum Command {
    Scan(ScanArgs),
    View(ViewArgs),
}

#[derive(Debug, Clone)]
pub struct ScanArgs {
    pub path: String,
    pub basis: String,
    pub snapshot: Option<String>,
    pub max_depth: Option<u16>,
    pub quiet: bool,
    pub legacy_traversal: bool,
    pub strategy_override: Option<String>,
    pub progress_interval_secs: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct ViewArgs {
    pub from_snapshot: String,
    pub path: Option<String>,
    pub top: usize,
    pub sort: String,
    pub json: bool,
}

impl Default for ScanArgs {
    fn default() -> Self {
        Self {
            path: String::new(),
            basis: "physical".to_string(),
            snapshot: None,
            max_depth: None,
            quiet: false,
            legacy_traversal: false,
            strategy_override: None,
            progress_interval_secs: None,
        }
    }
}

/// Parse command line arguments
pub fn parse_args(args: &[String]) -> Result<CliArgs, String> {
    if args.len() < 2 {
        return Err("No command specified".to_string());
    }

    let command = match args[1].as_str() {
        "scan" => {
            let scan_args = parse_scan_args(&args[2..])?;
            Command::Scan(scan_args)
        }
        "view" => {
            let view_args = parse_view_args(&args[2..])?;
            Command::View(view_args)
        }
        _ => return Err(format!("Unknown command: {}", args[1])),
    };

    Ok(CliArgs { command })
}

fn parse_scan_args(args: &[String]) -> Result<ScanArgs, String> {
    let mut scan_args = ScanArgs::default();
    let mut i = 0;

    while i < args.len() {
        match args[i].as_str() {
            "--basis" => {
                i += 1;
                if i >= args.len() {
                    return Err("--basis requires a value".to_string());
                }
                scan_args.basis.clone_from(&args[i]);
            }
            "--snapshot" => {
                i += 1;
                if i >= args.len() {
                    return Err("--snapshot requires a file path".to_string());
                }
                scan_args.snapshot = Some(args[i].clone());
            }
            "--max-depth" => {
                i += 1;
                if i >= args.len() {
                    return Err("--max-depth requires a value".to_string());
                }
                scan_args.max_depth = Some(
                    args[i]
                        .parse()
                        .map_err(|_| "--max-depth must be a number".to_string())?,
                );
            }
            "--quiet" => {
                scan_args.quiet = true;
            }
            "--legacy-traversal" => {
                scan_args.legacy_traversal = true;
            }
            "--strategy" => {
                i += 1;
                if i >= args.len() {
                    return Err("--strategy requires a value".to_string());
                }
                scan_args.strategy_override = Some(args[i].clone());
            }
            "--progress-interval" => {
                i += 1;
                if i >= args.len() {
                    return Err("--progress-interval requires a value".to_string());
                }
                let secs: u64 = args[i]
                    .parse()
                    .map_err(|_| "--progress-interval must be a positive integer".to_string())?;
                if secs == 0 {
                    return Err("--progress-interval must be greater than zero".to_string());
                }
                scan_args.progress_interval_secs = Some(secs);
            }
            arg if !arg.starts_with("--") => {
                if scan_args.path.is_empty() {
                    scan_args.path = arg.to_string();
                } else {
                    return Err(format!("Unexpected argument: {arg}"));
                }
            }
            _ => return Err(format!("Unknown option: {}", args[i])),
        }
        i += 1;
    }

    if scan_args.path.is_empty() {
        return Err("Missing required argument: PATH".to_string());
    }

    Ok(scan_args)
}

fn parse_view_args(args: &[String]) -> Result<ViewArgs, String> {
    let mut from_snapshot = String::new();
    let mut path = None;
    let mut top = 10;
    let mut sort = "size".to_string();
    let mut json = false;
    let mut i = 0;

    while i < args.len() {
        match args[i].as_str() {
            "--path" => {
                i += 1;
                if i >= args.len() {
                    return Err("--path requires a value".to_string());
                }
                path = Some(args[i].clone());
            }
            "--top" => {
                i += 1;
                if i >= args.len() {
                    return Err("--top requires a value".to_string());
                }
                top = args[i]
                    .parse()
                    .map_err(|_| "--top must be a number".to_string())?;
            }
            "--sort" => {
                i += 1;
                if i >= args.len() {
                    return Err("--sort requires a value".to_string());
                }
                sort.clone_from(&args[i]);
            }
            "--json" => {
                json = true;
            }
            arg if !arg.starts_with("--") => {
                if from_snapshot.is_empty() {
                    from_snapshot = arg.to_string();
                } else {
                    return Err(format!("Unexpected argument: {arg}"));
                }
            }
            _ => return Err(format!("Unknown option: {}", args[i])),
        }
        i += 1;
    }

    if from_snapshot.is_empty() {
        return Err("Missing required argument: SNAPSHOT_FILE".to_string());
    }

    Ok(ViewArgs {
        from_snapshot,
        path,
        top,
        sort,
        json,
    })
}
