# Data Model: Traversal Performance Optimization

## Entities

### ScanSession
- **Description**: Represents a single invocation of the disk-usage CLI with optimized traversal enabled.
- **Fields**:
  - `session_id` (UUID): Correlates logs, progress, and final totals.
  - `target_path` (Path): Absolute or relative directory scanned.
  - `filesystem_type` (Enum: NTFS, APFS, ext4, XFS, Other): Detected filesystem kind.
  - `strategy` (Enum: optimized, legacy): Traversal implementation selected.
  - `start_time` (Timestamp): UTC time the scan began.
  - `end_time` (Timestamp): UTC time the scan completed or aborted.
  - `duration_ms` (Integer): Total runtime in milliseconds.
  - `total_files` (Integer): Count of files encountered (post-filter).
  - `total_dirs` (Integer): Count of directories visited.
  - `total_bytes` (Integer): Aggregated logical size in bytes.
  - `skipped_entries` (Integer): Number of items skipped due to permissions or errors.
  - `error_summary` (List<ScanError>): Aggregated errors grouped by errno/code.
  - `progress_snapshots` (List<ProgressSnapshot>): Ordered progress telemetry captured for long scans.
- **Relationships**:
  - Has many `ProgressSnapshot` records.
  - Has many `ScanError` entries.

### ProgressSnapshot
- **Description**: Periodic snapshot of traversal progress for long-running scans.
- **Fields**:
  - `session_id` (UUID): Foreign key to `ScanSession`.
  - `timestamp` (Timestamp): Time the snapshot was emitted.
  - `processed_entries` (Integer): Files/directories processed so far.
  - `processed_bytes` (Integer): Total bytes attributed to processed files.
  - `estimated_completion_ratio` (Float 0–1): Optional heuristic completion percentage.
  - `recent_throughput_bytes_per_sec` (Integer): Moving average over the last interval.

### ScanError
- **Description**: Captures recoverable issues encountered during traversal.
- **Fields**:
  - `session_id` (UUID): Foreign key to `ScanSession`.
  - `path` (Path): Item that triggered the error, if known.
  - `error_code` (String): Platform-specific error identifier (e.g., `ERROR_ACCESS_DENIED`, `EACCES`).
  - `severity` (Enum: warning, critical): Indicates whether traversal continued.
  - `occurred_at` (Timestamp): Time of occurrence.

### TraversalStrategy
- **Description**: Encapsulates filesystem-specific traversal behavior.
- **Fields**:
  - `strategy_id` (String): Identifier (e.g., `ntfs-large-fetch`, `posix-openat`).
  - `supported_filesystems` (List<String>): Filesystem types served by this strategy.
  - `syscalls` (List<String>): Primary OS interfaces invoked.
  - `parallelism_model` (Enum: single-thread, rayon-work-stealing): Concurrency approach.
  - `prefetch_hint` (Boolean): Whether the strategy leverages kernel prefetch mechanisms.
  - `fallback_strategy` (String): Strategy to switch to upon failure.

## Validation Rules
- `end_time` MUST be ≥ `start_time`; `duration_ms` derives from these timestamps.
- `strategy` MUST default to `optimized`; `legacy` is only allowed if explicitly requested by the user or upon automatic fallback.
- Accuracy guardrail: `total_bytes` variance between strategies for the same dataset MUST be ≤ max(1% of legacy total, 10 MB).
- Progress snapshots MUST be strictly increasing in `timestamp` and `processed_entries`.
- `skipped_entries` MUST equal the sum of `ScanError` entries marked as recoverable.

## State Transitions (ScanSession)
1. **Initialized** → `start_time` recorded, strategy selected.
2. **Running** → Progress snapshots emitted, totals accumulated per thread.
3. **Completing** → Aggregations reduced, parity check against legacy (sampled) executed.
4. **Completed** → Final totals published, `end_time` recorded.
5. **Aborted** → If user cancels or fatal error occurs, `status` flagged, partial totals preserved.
