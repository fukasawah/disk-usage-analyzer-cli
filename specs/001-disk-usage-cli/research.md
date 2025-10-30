# Research (Phase 0)

This document consolidates unknowns and decisions needed for the Disk Usage CLI MVP.

## Unknowns and Decisions

### 1) Windows physical size API
- Decision: Use `GetCompressedFileSizeW` via `windows` crate for physical size on NTFS/FAT; fall back to `metadata.len()` when unavailable. Document that sparse/compressed files report on-disk clusters.
- Rationale: Mirrors du-like behavior for physical accounting; pure-Rust interop through `windows` crate.
- Alternatives considered: Using `fsutil` subprocess (rejected: portability, perf), always logical size (rejected: UX expectation is physical by default).

### 2) Filesystem boundary detection on Windows
- Decision: Compare volume serial number using `GetVolumePathNameW` + `GetVolumeInformationW`, cache the root volume for the starting path.
- Rationale: Robustly detects crossing between volumes; minimal per-entry overhead with cached check.
- Alternatives: Path prefix heuristics (rejected: brittle with junctions), skip boundary check (rejected: spec requires boundary enforcement).

### 3) Traversal engine choice
- Decision: Start with `jwalk` for parallel traversal with bounded memory; provide feature flag to switch to `walkdir` + `rayon` if compatibility issues arise.
- Rationale: `jwalk` offers out-of-the-box parallelism and dir-entry metadata prefetch; proven in large repositories.
- Alternatives: Custom thread pool over `walkdir` (higher maintenance), single-threaded (too slow for 1M+ files).

### 4) Parquet crate and schema shape (REQUIRED)
- Decision: Use `parquet` crate (apache arrow-rs, actively maintained). Single .parquet file with two logical tables in separate row groups distinguished by a `table` discriminator column: `entries`, `meta`, and optionally `errors`.
- Rationale: Snapshot再表示を高速にするMVP要件に合致。列指向により読み込み時のフィルタ/列選択が高速・省メモリ。実運用での堅牢性が高い。
- Alternatives: JSONL/CSV/独自バイナリは再表示の速度・容量面で不利（今回は不採用）。

### 5) Hardlink dedup strategy
- Decision: Track seen (dev,inode) pairs using `hashbrown::HashSet<(u64,u64)>` with load-factor tuning; cap memory with optional bloom filter when count grows beyond threshold; configurable policy `dedupe|count`.
- Rationale: Meets FR-009 with predictable memory usage; HashSet overhead ~32B/entry is acceptable up to tens of millions with optional fallback.
- Alternatives: OS-specific link count decrements (complex and error-prone), ignore dedup (violates default behavior).

### 6) Minimal JSON schema fields (MVP)
- Decision: For summary listing items output: `{ path, size_bytes, file_count, dir_count, depth, basis }` and top-level `{ root, started_at, finished_at, errors }`.
- Rationale: Satisfies constitution's JSON requirement while keeping forward-compatibility.
- Alternatives: Omit metadata (hurts scripting), over-detailed schema (rigid, more churn later).

## Best Practices Collected
- Use `same-file` to compare inodes and handle hardlinks safely.
- Avoid following symlinks; for Unix use `lstat` equivalents to inspect without dereferencing.
- Stream aggregation: compute directory totals in a post-order fashion without retaining all children in memory; use small LRU for recently visited parents.
- Progress and logging: default off or minimal, enable via `--verbose`; keep stdout clean for machine outputs.
- Testing on large trees: synthesize fixtures with varying depth and symlink/hardlink patterns; gate test runtime to stay deterministic and fast.

## Dependency Minimization Notes（最小＋一貫実装）

- 実装ポリシー: 標準ライブラリ＋最小限の近代的・メンテされているcrateのみ。
- 必須crate: `serde`/`serde_json`（JSON）、`parquet`（スナップショット）。
- Windowsのみ: `windows-sys`（物理サイズ/ファイルID）。Unixはstd拡張で代替。
- 走査/集計/CLI/整形はstdで自前実装。並列化はMVPでは行わず（性能要件に抵触しない範囲で最適化）。
- 時刻はUNIX epoch整数を出力（RFC 3339に固執せず依存削減）。

性能注記:
- 再表示速度: Parquetは列指向読み出しにより上位K件抽出・並べ替えなどが高速。JSONL等に対し有意な差あり。
- 初回走査: std実装＋ストリーミング集計で10^6ファイル級でもメモリは抑制可能。並列化は将来の最適化候補として検討。

## Open Items (tracked)
- Confirm Windows compressed size behavior parity with `du -A` equivalents.
- Measure `jwalk` vs `walkdir+rayon` on 1M files; switch if regressions observed.
