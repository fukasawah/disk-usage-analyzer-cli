# rs-disk-usage Development Guidelines

Auto-generated from all feature plans. Last updated: 2025-10-30

## Active Technologies
- Rust 1.77+ (Edition 2024) - already established in MVP (002-optimization-improvements)
- Parquet snapshots (existing) - will optimize feature flags (002-optimization-improvements)
- Rust 1.90 (Edition 2024) + `rayon` (parallel traversal), `windows` crate for Win32 APIs, platform-specific POSIX bindings (`rustix`) for Linux/macOS directory iteration (003-optimize-traversal)
- N/A (filesystem metadata only) (003-optimize-traversal)
- Rust 1.90+ (Edition 2024) (001-disk-usage-cli)

## Project Structure

```text
src/
tests/
```

## Commands

cargo fmt
cargo clippy
cargo test
cargo check --target x86_64-pc-windows-gnu
cargo build --target x86_64-pc-windows-gnu

## Code Style

Rust 1.90+ (Edition 2024): Follow standard conventions

## Recent Changes
- 003-optimize-traversal: Added Rust 1.90 (Edition 2024) + `rayon` (parallel traversal), `windows` crate for Win32 APIs, platform-specific POSIX bindings (`rustix`) for Linux/macOS directory iteration
- 002-optimization-improvements: Added Rust 1.77+ (Edition 2024) - already established in MVP
- 001-disk-usage-cli: Added Rust 1.90+ (Edition 2024)

<!-- MANUAL ADDITIONS START -->
## Platform Build Checks
- Validate cross-platform builds locally: at minimum run `cargo check --target x86_64-pc-windows-gnu` and `cargo build --target x86_64-pc-windows-gnu` before submitting changes.
- Ensure CI coverage for Windows MSVC stays green; investigate any cross-target regressions immediately.

## Release Checklist
- Use `cargo set-version --locked <new-version>` to bump the crate version and keep `Cargo.lock` in sync.
- Run `cargo test` (and any targeted release tests) before tagging.
- Update changelog or release notes if applicable.
- Commit with a message like `chore: release v<new-version>` and tag the same version.
<!-- MANUAL ADDITIONS END -->
