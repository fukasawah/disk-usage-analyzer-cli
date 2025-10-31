# dua - Disk Usage Analyzer

[![CI](https://github.com/fukasawah/dua/actions/workflows/ci.yml/badge.svg)](https://github.com/fukasawah/dua/actions)
[![Release](https://github.com/fukasawah/dua/actions/workflows/release.yml/badge.svg)](https://github.com/fukasawah/dua/releases)

Fast command-line tool to analyze disk usage and find space hogs. Written in Rust.

## Features

- **Fast**: Efficiently scan large directory trees
- **Snapshot support**: Save scan results to Parquet format for instant re-analysis
- **Drill down**: View subdirectories without re-scanning
- **JSON output**: Machine-readable format for scripting
- **Safe**: Doesn't follow symlinks or cross filesystem boundaries

## Installation

Download pre-built binaries from [Releases](https://github.com/fukasawah/dua/releases):

**Supported Platforms** (tested in CI):
- Linux x86_64 (glibc): `dua-v*-linux-x86_64`
- Linux x86_64 (musl, static): `dua-v*-linux-x86_64-musl` **← Recommended**
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

Drill down into a subdirectory:
```bash
dua view usage.parquet --path /path/to/directory/subdir
```

JSON output for scripting:
```bash
dua view usage.parquet --json
```

## Building from Source

Requirements: Rust 1.77+

```bash
cargo build --release
# Binary: target/release/dua
```

Static Linux binary (recommended, works on any Linux):
```bash
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
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

### Check Code Quality

Format code:
```bash
cargo fmt
```

Run linter:
```bash
cargo clippy --all-targets --all-features
```

Check formatting without modifying files:
```bash
cargo fmt -- --check
```

### Before Committing

Run these commands to ensure code quality (pre-commit hooks do this automatically):

```bash
# Format code
cargo fmt

# Run linter with all warnings as errors
cargo clippy --all-targets --all-features -- -D warnings

# Run tests
cargo test --release
```

Or run all at once:
```bash
cargo fmt && cargo clippy --all-targets --all-features -- -D warnings && cargo test --release
```

### Measure Test Coverage

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
