# rs-disk-usage Development Guidelines

Auto-generated from all feature plans. Last updated: 2025-10-30

## Active Technologies
- Rust 1.77+ (Edition 2024) - already established in MVP (002-optimization-improvements)
- Parquet snapshots (existing) - will optimize feature flags (002-optimization-improvements)
- Rust 1.90 (Edition 2024) + `rayon` (parallel traversal), `windows` crate for Win32 APIs, platform-specific POSIX bindings (`rustix`) for Linux/macOS directory iteration (001-optimize-traversal)
- N/A (filesystem metadata only) (001-optimize-traversal)

- Rust 1.90+ (Edition 2024) (001-disk-usage-cli)

## Project Structure

```text
src/
tests/
```

## Commands

cargo test
cargo clippy

## Code Style

Rust 1.90+ (Edition 2024): Follow standard conventions

## Recent Changes
- 001-optimize-traversal: Added Rust 1.90 (Edition 2024) + `rayon` (parallel traversal), `windows` crate for Win32 APIs, platform-specific POSIX bindings (`rustix`) for Linux/macOS directory iteration
- 002-optimization-improvements: Added Rust 1.77+ (Edition 2024) - already established in MVP

- 001-disk-usage-cli: Added Rust 1.90+ (Edition 2024)

<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->
