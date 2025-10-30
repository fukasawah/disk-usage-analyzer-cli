ios/ or android/
# Implementation Plan: Disk Usage CLI MVP

**Branch**: `001-disk-usage-cli` | **Date**: 2025-10-30 | **Spec**: `/workspaces/rs-disk-usage/specs/001-disk-usage-cli/spec.md`
**Input**: Feature specification from `/specs/001-disk-usage-cli/spec.md`

Note: This plan follows the speckit workflow. Phase 0 research and Phase 1 design artifacts are generated alongside this plan.

## Summary

Goal: Provide a fast, memory-efficient Rust CLI to summarize disk usage for a target directory, list immediate children with aggregate sizes/counts, support drill-down, and optionally persist/load snapshots in Parquet. Default size basis is physical; symlinks/junctions are not followed; traversal stays within the same filesystem. Human-readable output is default; a minimal JSON mode is included to comply with the repository constitution.

Approach: Single, standard-library–first implementation. Use std::fs read_dir recursion with careful error handling; streaming aggregation to compute per-directory totals without retaining all entries in memory. Parquet snapshot writer/reader is REQUIRED (per spec) using a modern, actively maintained crate. Platform-specific modules compute physical size (Unix via MetadataExt blocks; Windows via WinAPI). Hardlink handling defaults to deduplication by (device,inode), configurable. CLI designed for stable flags and exit codes.

## Technical Context

**Language/Version**: Rust 1.77+ (Edition 2021)  
**Primary Dependencies**:
- JSON: `serde` + `serde_json` (constitution requires stable JSON mode; robust escaping > hand-rolled)
- Snapshot I/O: `parquet` (Apache Arrow Rust Parquet implementation)
- Windows-only: `windows-sys` (or `windows`) for physical size and file ID; compiled conditionally via cfg(target_os = "windows")

Everything else via standard library (CLI parsing with std::env::args, traversal with std::fs, custom human-size formatting, custom error enum). No dual implementations, no optional feature-gated rewrites.
**Storage**: File-based snapshots in Parquet (.parquet) containing entries and meta tables (REQUIRED)  
**Testing**: `cargo test` with minimal helpers (prefer std); integration tests for CLI flows; snapshot round-trip tests. If CLI test helpers are added, choose actively maintained, minimal crates only.
**Target Platform**: Linux and Windows (MacOS best-effort)  
**Project Type**: Single binary crate (src/{cli,services,models,lib}), plus tests/{unit,integration,contract}  
**Performance Goals**: 
- Repo baseline: fast traversal for ~100k files (p95 ≤ 5s) as architecture target
- Feature spec: p95 ≤ 10 minutes for 1M files on SSD-class hardware; peak RSS ≤ 1GB
**Constraints**: 
- Default physical size basis; option to switch to logical
- No symlink/junction following; do not cross filesystem boundary
- Human-readable output by default; JSON mode available and stable
**Scale/Scope**: Up to 10M files; snapshot sizes can be large—streaming writers and bounded row groups are required

Unknowns to resolve (extracted for Phase 0):
1) Windows physical size API choice and behavior for compressed/sparse files
2) Windows filesystem boundary detection method with acceptable overhead
3) Parquet crate selection and schema details (single file multi-table vs flat schema) — resolved below
4) Hardlink dedup strategy data structure sizing under 10M files
5) Minimal JSON schema fields for MVP while keeping future compatibility

### Dependency footprint (final, MVP)

Required:
- `serde`, `serde_json` — correctness and safety for JSON mode per constitution
- `parquet` — snapshot is part of MVP for fast re-view without re-scan (spec requirement)

Conditional (only on Windows targets via cfg):
- `windows-sys` (or `windows`) — physical size and stable file identity

Standard-library implementation elsewhere:
- Traversal (std::fs), CLI parsing (std::env), sorting/paging (std), size formatting (custom), time as UNIX epoch seconds (std::time)

## Constitution Check

Pre-design gate evaluation:

- Code Quality: Plan defines modules (cli, services/traversal, services/aggregate, io/snapshot, models). Public APIs will use Result types; no use of `unwrap` in critical paths. Each module will include a top-level doc comment for contract and invariants. PASS (planned).
- Testing: Unit tests for aggregation, size-basis conversions, and hardlink handling. Integration tests for CLI (--help, scan, drill-down, JSON). Snapshot round-trip tests. Coverage target ≥ 90% lines on changed code; deterministic fixtures. PASS (planned).
- UX Consistency: CLI flags, exit codes, and JSON schema are specified below; human-readable default with `--json` available. PASS.
- Performance: SLOs stated; traversal/aggregation logic will include a benchmark plan on fixtures; streaming to avoid O(n^2) or unbounded memory. PASS (planned).

Re-check scheduled after Phase 1 design with finalized JSON schema and contracts.

## Project Structure

### Documentation (this feature)

```text
/workspaces/rs-disk-usage/specs/001-disk-usage-cli/
├── plan.md              # This file (speckit.plan output)
├── research.md          # Phase 0 (unknowns resolved, decisions logged)
├── data-model.md        # Phase 1 (entities, validation, transitions)
├── quickstart.md        # Phase 1 (usage and examples)
└── contracts/
    └── openapi.yaml     # Phase 1 (API contracts for future service/CLI parity)
```

### Source Code (repository root)

```text
src/
├── cli/                 # clap command/flag parsing and presentation
├── models/              # DirectoryEntry, SnapshotMeta, enums, DTOs (serde)
├── services/
│   ├── traverse.rs      # traversal + filtering (symlink policy, fs boundary)
│   ├── aggregate.rs     # streaming aggregation per directory
│   └── size.rs          # logical/physical size computation (platform-specific)
├── io/
│   └── snapshot.rs      # parquet write/read; schema mapping
└── lib.rs               # public facade and module docs

tests/
├── unit/
├── integration/
└── contract/
```

**Structure Decision**: Single binary crate with clear module boundaries to preserve testability and performance-focused iteration. Tests split per constitution (unit/integration/contract). Future renderer backends (HTML/SVG) can be added without impacting traversal core.

## CLI Contract (MVP + JSON mode)

Command: `dux` (working name)

Subcommands:
- `scan <PATH>`: Traverse and print immediate children summary; optional snapshot save
- `drill <PATH> <SUBDIR>`: Re-run summary using SUBDIR as new root (or `scan --root <SUBDIR>` variant)
- `view --from-snapshot <FILE>`: Print summary from snapshot without re-scan

Common Flags:
- `--basis <physical|logical>` (default: physical)
- `--max-depth <N>` (default: 1 for listing; >1 only for snapshot building)
- `--top <K>` (default: 10)
- `--sort <size|files|dirs>` (default: size)
- `--snapshot <FILE.parquet>` (write)
- `--from-snapshot <FILE.parquet>` (read)
- `--json` (machine-readable output to stdout)
- `--quiet` / `--verbose` (diagnostics to stderr)
- `--no-progress` (suppress progress)

Exit Codes:
- 0 success; 2 invalid input; 3 partial failures (some entries unreadable); 4 I/O/system errors.

JSON Schema (summary array items):
- path (string), size_bytes (u64), file_count (u32), dir_count (u32), depth (u16), basis (enum), errors (optional array)

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|---------------------------------------|
| Spec said "text only" but constitution requires `--json` | Constitution UX rule is non-negotiable | Deferring JSON would fail the gate; adding minimal JSON is low-risk and forward-compatible |
