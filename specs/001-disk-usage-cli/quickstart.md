# Quickstart

This guide shows how to build and use the Disk Usage CLI (dux).

## Build

Requires Rust 1.77+ and cargo.

```bash
cargo build --release
```

The binary will be available at `target/release/dux`.

## Installation

```bash
cargo install --path .
```

Or run directly with cargo:

```bash
cargo run --bin dux -- [COMMAND] [OPTIONS]
```

## Usage

### Basic Scan

Summarize a directory (top 10 by size, physical basis):

```bash
dux scan /path/to/root
```

Example output:
```
Directory: /path/to/root

Path                                                       Size    Files     Dirs
--------------------------------------------------------------------------------
/path/to/root/large_dir                                1.50 GB      150       10
/path/to/root/medium_dir                             500.00 MB       80        5
/path/to/root/small_dir                               50.00 MB       20        2
```

### JSON Output

Show JSON output for scripting:

```bash
dux scan /path/to/root --json --top 5
```

### Custom Sorting and Limiting

Sort by file count, show top 3:

```bash
dux scan /path/to/root --sort files --top 3
```

Sort options: `size` (default), `files`, `dirs`

### Size Basis

Use logical size instead of physical:

```bash
dux scan /path/to/root --basis logical
```

### Depth Limiting

Limit traversal depth:

```bash
dux scan /path/to/root --max-depth 2
```

### Snapshot Operations

Save a snapshot after scanning:

```bash
dux scan /path/to/root --snapshot snapshot.parquet
```

View from an existing snapshot (no re-scan):

```bash
dux view --from-snapshot snapshot.parquet
```

View snapshot with JSON output:

```bash
dux view --from-snapshot snapshot.parquet --json --top 5
```

### Drill-Down

Drill into a subdirectory:

```bash
dux drill /path/to/root /path/to/root/subdir --top 5
```

This is equivalent to running `dux scan /path/to/root/subdir` but provides a consistent workflow.

### Verbose Output

Enable diagnostic messages:

```bash
dux scan /path/to/root --verbose
```

## Exit Codes

- `0`: Success
- `2`: Invalid input (bad arguments, path doesn't exist, etc.)
- `3`: Partial failure (some files/directories couldn't be read, but scan completed)
- `4`: I/O or system error

## Notes

- **Symlinks**: Not followed by default (prevents infinite loops and crossing mount points)
- **Filesystem boundaries**: Traversal does not cross filesystem boundaries
- **Default size basis**: Physical (actual disk usage including block allocation)
- **Hardlink handling**: Deduplicated by default (each hardlinked file counted once)
- **Error handling**: Errors are recorded but don't stop traversal; partial results returned

## Examples

Scan your home directory:
```bash
dux scan ~
```

Find largest directories in /var with JSON output:
```bash
dux scan /var --top 20 --json > var-usage.json
```

Create and view snapshot:
```bash
dux scan /large/directory --snapshot /tmp/scan.parquet
# Later, view without re-scanning:
dux view --from-snapshot /tmp/scan.parquet --top 5
```

Drill down into specific subdirectory:
```bash
dux scan /home/user
# See that /home/user/Projects is large
dux drill /home/user /home/user/Projects --top 10
```
