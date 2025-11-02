# Quickstart: Traversal Performance Optimization

## Prerequisites
- Rust toolchain 1.90+ installed (via `rustup +1.90.0`)
- Windows 10+/Server 2019+, macOS 13+, or Linux kernel 5.10+ with NVMe SSD for baseline benchmarks
- Example dataset with ~100k files (e.g., unpacked Chromium source tree)

## Build the CLI
```bash
cargo build --release
```

## Run an optimized scan
```bash
target/release/dua C:\path\to\dataset
```
- Detects NTFS automatically and enables the large-fetch traversal strategy
- Emits progress snapshots to stderr every ~2 seconds once runtime exceeds the 3-second SLO

## Compare with legacy traversal
```bash
target/release/dua --legacy-traversal C:\path\to\dataset
```
- Provides baseline numbers for parity verification
- Expect totals within 1% or 10 MB of the optimized run

## Capture JSON output with progress events
```bash
target/release/dua scan /data/projects --progress-interval 2 --snapshot projects.parquet
dua view projects.parquet --json
```
- The scan command emits throttled stderr progress updates at the requested cadence
- The JSON view output includes recorded progress snapshots for downstream tooling

## Tune progress cadence
```bash
target/release/dua scan /mnt/archive --progress-interval 5 --snapshot archive.parquet
```
- Increase the interval for noisy remote shares or decrease it for granular telemetry
- Progress snapshots stored in the Parquet file reflect the requested cadence

## Validate performance
```bash
hyperfine \
  -w 2 \
  'target/release/dua C:\\path\\to\\dataset' \
  'target/release/dua --legacy-traversal C:\\path\\to\\dataset'
```
- Ensure optimized traversal achieves ≤3s p95 runtime on NTFS
- Document results under `specs/003-optimize-traversal/research.md`

## Run regression tests
```bash
cargo test --features integration traversal_fast_path
```
- Executes parity and performance-guard tests added for the optimized strategies
- Confirm all new tests pass before submitting a PR
