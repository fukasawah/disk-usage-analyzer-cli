//! CLI argument parsing

#[derive(Debug, Clone)]
pub struct CliArgs {
    pub command: Command,
}

#[derive(Debug, Clone)]
pub enum Command {
    Scan(ScanArgs),
    Drill(DrillArgs),
    View(ViewArgs),
}

#[derive(Debug, Clone)]
pub struct ScanArgs {
    pub path: String,
    pub basis: String,
    pub snapshot: Option<String>,
    pub max_depth: Option<u16>,
    pub verbose: bool,
    pub quiet: bool,
}

#[derive(Debug, Clone)]
pub struct DrillArgs {
    pub root: String,
    pub subdir: String,
    pub basis: String,
    pub top: usize,
    pub sort: String,
    pub json: bool,
    pub max_depth: Option<u16>,
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
            verbose: false,
            quiet: false,
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
        "drill" => {
            let drill_args = parse_drill_args(&args[2..])?;
            Command::Drill(drill_args)
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
                scan_args.basis = args[i].clone();
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
            "--verbose" => {
                scan_args.verbose = true;
            }
            "--quiet" => {
                scan_args.quiet = true;
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

fn parse_drill_args(args: &[String]) -> Result<DrillArgs, String> {
    let mut root = String::new();
    let mut subdir = String::new();
    let mut basis = "physical".to_string();
    let mut top = 10;
    let mut sort = "size".to_string();
    let mut json = false;
    let mut max_depth = None;
    let mut i = 0;

    while i < args.len() {
        match args[i].as_str() {
            "--basis" => {
                i += 1;
                if i >= args.len() {
                    return Err("--basis requires a value".to_string());
                }
                basis = args[i].clone();
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
                sort = args[i].clone();
            }
            "--json" => {
                json = true;
            }
            "--max-depth" => {
                i += 1;
                if i >= args.len() {
                    return Err("--max-depth requires a value".to_string());
                }
                max_depth = Some(
                    args[i]
                        .parse()
                        .map_err(|_| "--max-depth must be a number".to_string())?,
                );
            }
            arg if !arg.starts_with("--") => {
                if root.is_empty() {
                    root = arg.to_string();
                } else if subdir.is_empty() {
                    subdir = arg.to_string();
                } else {
                    return Err(format!("Unexpected argument: {arg}"));
                }
            }
            _ => return Err(format!("Unknown option: {}", args[i])),
        }
        i += 1;
    }

    if root.is_empty() {
        return Err("Missing required argument: ROOT".to_string());
    }
    if subdir.is_empty() {
        return Err("Missing required argument: SUBDIR".to_string());
    }

    Ok(DrillArgs {
        root,
        subdir,
        basis,
        top,
        sort,
        json,
        max_depth,
    })
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
                sort = args[i].clone();
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
