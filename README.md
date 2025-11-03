# dua - Disk Usage Analyzer

[![CI](https://github.com/fukasawah/dua/actions/workflows/ci.yml/badge.svg)](https://github.com/fukasawah/dua/actions)
[![Release](https://github.com/fukasawah/dua/actions/workflows/release.yml/badge.svg)](https://github.com/fukasawah/dua/releases)

Fast command-line tool to analyze disk usage and find space hogs. Written in Rust.

## Features

- **Fast**: Efficiently scan large directory trees with optimized NTFS/POSIX traversals
- **Filesystem aware**: Auto-detects the best traversal strategy and allows manual overrides
- **Predictable progress**: Emits throttled progress snapshots every few seconds on long scans
- **Snapshot support**: Save scan results to Parquet format for instant re-analysis
- **Drill down**: Focus on subdirectories from saved snapshots
- **JSON output**: Machine-readable format for scripting, including progress telemetry
- **Safe**: Doesn't follow symlinks or cross filesystem boundaries

## Installation

Download pre-built binaries from [Releases](https://github.com/fukasawah/dua/releases):

**Supported Platforms** (tested in CI):
- Linux x86_64 (glibc): `dua-v*-linux-x86_64`
- Linux x86_64 (musl, static): `dua-v*-linux-x86_64-musl`
- Windows x86_64: `dua-v*-windows-x86_64.exe`

**Unsupported** (builds provided but untested):
- Linux ARM64, macOS (Intel/Apple Silicon)

> **Note**: The maintainer does not have access to macOS. macOS builds are provided as-is without testing.

## Usage

Scan and save a snapshot:
```bash
dua scan /path/to/directory --snapshot usage.parquet
```

View results (instant, no re-scan):
```bash
dua view usage.parquet
```

Drill down into a subdirectory (no re-scan needed):
```bash
dua view usage.parquet --path /path/to/directory/subdir
```

JSON output for scripting:
```bash
dua view usage.parquet --json
```

### Strategy selection and overrides

- Optimized traversal is enabled by default and auto-detects the filesystem to pick the best backend (NTFS, POSIX, or legacy).
- Force the legacy fallback for troubleshooting with `--legacy-traversal`.
- Pin a specific optimized backend with `--strategy windows` or `--strategy posix` when testing platform behavior.

```bash
dua scan /data/projects --strategy posix --snapshot projects.parquet
dua scan /data/projects --legacy-traversal --snapshot projects.parquet
```

### Progress telemetry

- The CLI emits stderr progress snapshots roughly every 2 seconds by default once a scan exceeds the 3-second SLO.
- Adjust the cadence with `--progress-interval <seconds>` for slower media.
- Progress snapshots are captured in JSON output and stored in Parquet snapshots for later inspection.

```bash
dua scan /data/projects --progress-interval 1 --snapshot projects.parquet
```

## Building from Source

Requirements: Rust 1.90+

```bash
cargo build --release
# Binary: target/release/dua
```

Static Linux binary (recommended, works on any Linux):
```bash
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
```

### Cross-compiling for Windows (GNU toolchain)

Install the MinGW toolchain and Rust target:

```bash
sudo apt-get install -y mingw-w64
rustup target add x86_64-pc-windows-gnu
```

Then check and build the Windows binary locally (matches CI expectations):

```bash
cargo check --target x86_64-pc-windows-gnu
cargo build --target x86_64-pc-windows-gnu
```

## Development

### Setup Git Hooks

Enable pre-commit hooks (runs `fmt` and `clippy` before commit):
```bash
git config core.hooksPath hooks
```

### Run Tests

```bash
cargo test
```

### Development Workflow

Before committing, run these commands to ensure code quality (pre-commit hooks run these automatically):

```bash
# Format code
cargo fmt

# Run linter with all warnings as errors (same as CI/pre-commit)
cargo clippy --all-targets --all-features -- -D warnings

# Run tests
cargo test --release
```

Or run all at once:
```bash
cargo fmt && cargo clippy --all-targets --all-features -- -D warnings && cargo test --release

# Verify the Windows GNU cross-target (requires mingw-w64 as above)
cargo check --target x86_64-pc-windows-gnu && cargo build --target x86_64-pc-windows-gnu
```

Check formatting without modifying files:
```bash
cargo fmt -- --check

```bash
cargo install cargo-llvm-cov
cargo llvm-cov --html --open
```

## Releasing

1. Update version in `Cargo.toml`
2. Commit: `git commit -am "chore: bump version to X.Y.Z"`
3. Tag: `git tag vX.Y.Z`
4. Push: `git push origin main --tags`

GitHub Actions will automatically build and publish release binaries.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
