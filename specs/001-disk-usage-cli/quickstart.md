# Quickstart (Phase 1)

This guide shows how to run the Disk Usage CLI once implemented.

## Build
- Requires Rust 1.77+ and cargo.

## Usage (planned CLI)

- Summarize a directory (top 10 by size, physical basis):
  dux scan /path/to/root

- Show JSON output for scripting:
  dux scan /path/to/root --json --top 5

- Save a snapshot after scanning:
  dux scan /path/to/root --snapshot snapshot.parquet

- View from an existing snapshot (no re-scan):
  dux view --from-snapshot snapshot.parquet --json

- Drill into a subdirectory:
  dux drill /path/to/root /path/to/root/var --top 5

## Notes
- Symlinks/junctions are not followed by default.
- Traversal does not cross filesystem boundaries.
- Default size basis is physical; use --basis logical for logical sizes.
