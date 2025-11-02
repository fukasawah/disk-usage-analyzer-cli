//! Unit tests for CLI argument parsing extensions
#[cfg(test)]
mod tests {
	use dua::cli::args::{parse_args, Command};

	fn make_args(raw: &[&str]) -> Vec<String> {
		raw.iter().map(|s| s.to_string()).collect()
	}

	#[test]
	fn parse_scan_with_legacy_traversal_flag() {
		let argv = make_args(&[
			"dua",
			"scan",
			"/tmp/work",
			"--snapshot",
			"out.parquet",
			"--legacy-traversal",
		]);

		let parsed = parse_args(&argv).expect("parse scan args");
		let Command::Scan(scan) = parsed.command else {
			panic!("expected scan command");
		};

		assert!(scan.legacy_traversal);
		assert!(scan.strategy_override.is_none());
	}

	#[test]
	fn parse_scan_with_strategy_override() {
		let argv = make_args(&[
			"dua",
			"scan",
			"/tmp/work",
			"--snapshot",
			"out.parquet",
			"--strategy",
			"posix",
		]);

		let parsed = parse_args(&argv).expect("parse scan args");
		let Command::Scan(scan) = parsed.command else {
			panic!("expected scan command");
		};

		assert_eq!(scan.strategy_override.as_deref(), Some("posix"));
		assert!(!scan.legacy_traversal);
	}

	#[test]
	fn parse_scan_with_conflicting_strategy_flags() {
		let argv = make_args(&[
			"dua",
			"scan",
			"/tmp/work",
			"--legacy-traversal",
			"--strategy",
			"windows",
		]);

		let parsed = parse_args(&argv).expect("parse scan args");
		let Command::Scan(scan) = parsed.command else {
			panic!("expected scan command");
		};

		assert!(scan.legacy_traversal);
		assert_eq!(scan.strategy_override.as_deref(), Some("windows"));
	}

	#[test]
	fn strategy_flag_requires_value() {
		let argv = make_args(&["dua", "scan", "/tmp/work", "--strategy"]);
		let err = parse_args(&argv).expect_err("strategy flag without value should fail");
		assert!(err.contains("--strategy requires a value"));
	}

	#[test]
	fn progress_interval_flag_sets_override() {
		let argv = make_args(&[
			"dua",
			"scan",
			"/tmp/work",
			"--progress-interval",
			"5",
		]);

		let parsed = parse_args(&argv).expect("parse scan args");
		let Command::Scan(scan) = parsed.command else {
			panic!("expected scan command");
		};

		assert_eq!(scan.progress_interval_secs, Some(5));
	}

	#[test]
	fn progress_interval_requires_positive_value() {
		let argv = make_args(&["dua", "scan", "/tmp/work", "--progress-interval", "0"]);
		let err = parse_args(&argv)
			.expect_err("progress interval of zero should be rejected");
		assert!(err.contains("greater than zero"));

		let missing_value = make_args(&["dua", "scan", "/tmp/work", "--progress-interval"]);
		let err = parse_args(&missing_value)
			.expect_err("progress interval flag without value should fail");
		assert!(err.contains("--progress-interval requires a value"));
	}
}
