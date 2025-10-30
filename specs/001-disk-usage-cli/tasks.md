# Tasks: Disk Usage CLI MVP

**Input**: Design documents from `/specs/001-disk-usage-cli/`
**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: Tests are REQUIRED per constitution. Each user story includes unit tests and, when applicable, integration/CLI tests and JSON schema checks.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- [P]: Can run in parallel (different files, no dependencies)
- [Story]: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- Single project: `src/`, `tests/` at repository root
- Paths below follow the structure defined in plan.md

---

## Phase 1: Setup (Shared Infrastructure)

Purpose: Project initialization and basic structure

- [X] T001 Create Rust binary crate with modules: initialize Cargo manifest at `/workspaces/rs-disk-usage/Cargo.toml`
- [X] T002 Add dependencies in `/workspaces/rs-disk-usage/Cargo.toml`: `serde`, `serde_json`, `parquet`, and conditional `windows-sys` for Windows targets
- [X] T003 [P] Create module structure per plan in `src/`: `src/lib.rs`, `src/cli/mod.rs`, `src/models/mod.rs`, `src/services/traverse.rs`, `src/services/aggregate.rs`, `src/services/size.rs`, `src/io/snapshot.rs`
- [X] T004 [P] Scaffold binary entry point at `src/bin/dux.rs` with basic `main()` and `--help` placeholder
- [X] T005 [P] Create test directories with placeholder files: `tests/unit/mod.rs`, `tests/integration/mod.rs`, `tests/contract/mod.rs`
- [X] T006 Configure formatting and linting: add `/workspaces/rs-disk-usage/rustfmt.toml` and `/workspaces/rs-disk-usage/.cargo/config.toml` (set clippy lint levels)
- [X] T007 Add `.gitignore` entries for Rust target and snapshot files at `/workspaces/rs-disk-usage/.gitignore`

Checkpoint: Repo builds with cargo build and runs cargo clippy cleanly (no new warnings in scaffolding). ✓ COMPLETE

---

## Phase 2: Foundational (Blocking Prerequisites)

Purpose: Core infrastructure that MUST be complete before ANY user story can be implemented

⚠️ CRITICAL: No user story work can begin until this phase is complete

- [X] T008 Implement core data models in `src/models/mod.rs` with serde derives: `DirectoryEntry`, `SnapshotMeta`, `ErrorItem`
- [X] T009 [P] Implement size basis computation (Unix) in `src/services/size.rs` using `MetadataExt` blocks (physical) and `len()` (logical)
- [X] T010 [P] Implement size basis computation (Windows, cfg) in `src/services/size.rs` using `GetCompressedFileSizeW` via `windows-sys`
- [X] T011 Implement filesystem boundary detection (Unix) in `src/services/traverse.rs` using `MetadataExt::dev`
- [X] T012 [P] Implement filesystem boundary detection (Windows, cfg) in `src/services/traverse.rs` comparing starting volume vs entry volume
- [X] T013 Implement hardlink deduplication policy (default: dedupe) with `(dev,inode)` tracking in `src/services/traverse.rs` (Unix) and equivalent on Windows
- [X] T014 Implement traversal without following symlinks in `src/services/traverse.rs` using `std::fs::read_dir` and `symlink_metadata`
- [X] T015 Implement streaming aggregation primitives in `src/services/aggregate.rs` to compute per-directory totals post-order without retaining entire tree
- [X] T016 Define public facade in `src/lib.rs` with documented contracts: `scan_summary(root, opts) -> Result<Summary, Error>` and types
- [X] T017 [P] Implement error handling strategy and custom error enum in `src/lib.rs` and reuse in services
- [X] T018 [P] Implement human-readable size formatter helper in `src/services/format.rs`
- [X] T019 Add logging/progress hooks (no-op by default) and stderr diagnostics toggles in `src/lib.rs`
- [X] T020 Create minimal developer fixtures under `tests/fixtures/` for deterministic directory structures

Checkpoint: Foundation ready - user story implementation can now begin in parallel ✓ COMPLETE

---

## Phase 3: User Story 1 - ディレクトリ使用量の把握 (Priority: P1) 🎯 MVP

Goal: Summarize immediate children of a root directory with aggregate size/counts and present sorted output. Default basis is physical.

Independent Test: Against a known fixture, verify sizes and counts match expected values; invalid path returns error message and non-zero exit code.

### Tests for User Story 1

- [X] T021 [P] [US1] Unit tests for aggregation and size basis conversions in `tests/unit/aggregate_tests.rs`
- [X] T022 [P] [US1] Unit tests for traversal filtering (no symlink follow, same FS) in `tests/unit/traverse_tests.rs`
- [X] T023 [P] [US1] CLI/Integration test for `dux scan <PATH>` happy path in `tests/integration/test_scan.rs`
- [X] T024 [P] [US1] CLI/Integration test for invalid path and exit codes in `tests/integration/test_errors.rs`
- [X] T025 [P] [US1] JSON output smoke test for `--json` shape (fields only) in `tests/contract/test_json_shape.rs`

### Implementation for User Story 1

- [X] T026 [P] [US1] Implement CLI arg parsing for `scan <PATH>` with flags `--basis`, `--top`, `--sort`, `--json` in `src/bin/dux.rs`
- [X] T027 [US1] Wire CLI to lib facade `scan_summary` and print human-readable output in `src/bin/dux.rs`
- [X] T028 [US1] Implement JSON output branch using `serde_json` in `src/bin/dux.rs`
- [X] T029 [US1] Implement sorting and top-K limiting in `src/services/aggregate.rs`
- [X] T030 [US1] Implement exit codes mapping (0 ok; 2 invalid input; 3 partial failures; 4 I/O/system) in `src/bin/dux.rs`
- [X] T031 [US1] Ensure stderr diagnostics for errors and keep stdout clean for content/JSON in `src/bin/dux.rs`

Checkpoint: User Story 1 fully functional and independently testable ✓ COMPLETE

---

## Phase 4: User Story 2 - ドリルダウンによる深掘り (Priority: P2)

Goal: Allow re-running the summary using a specified subdirectory as the new root; support depth limits.

Independent Test: Re-aggregation for a subdirectory matches running scan with that subdirectory as the root; depth limit caps traversal accordingly.

### Tests for User Story 2

- [X] T032 [P] [US2] Unit tests for depth limiting behavior in `tests/unit/depth_tests.rs`
- [X] T033 [P] [US2] Integration test for `dux drill <ROOT> <SUBDIR>` equivalence to `scan <SUBDIR>` in `tests/integration/test_drill.rs`

### Implementation for User Story 2

- [X] T034 [P] [US2] Extend CLI to add `drill <ROOT> <SUBDIR>` subcommand and `--max-depth` in `src/bin/dux.rs`
- [X] T035 [US2] Add re-rooting logic and depth limiting in `src/lib.rs` and `src/services/traverse.rs`
- [X] T036 [US2] Ensure sorting and top-K still apply to drill output in `src/services/aggregate.rs`

Checkpoint: User Stories 1 AND 2 work independently ✓ COMPLETE

---

## Phase 5: User Story 3 - スナップショットの保存と閲覧 (Priority: P2)

Goal: Save traversal results to Parquet and view summaries from snapshots without re-scanning.

Independent Test: Save→load→display reproduces the same results as original scan on the same dataset; no re-traversal I/O occurs during view.

### Tests for User Story 3

- [X] T037 [P] [US3] Snapshot write/read round-trip test with fixtures in `tests/integration/test_snapshot_roundtrip.rs`
- [X] T038 [P] [US3] JSON output from `view --from-snapshot` matches schema fields in `tests/contract/test_snapshot_json.rs`
- [X] T039 [US3] Error handling test for invalid/corrupt snapshot file in `tests/integration/test_snapshot_errors.rs`

### Implementation for User Story 3

- [X] T040 [P] [US3] Implement Parquet schema and writer in `src/io/snapshot.rs` per data-model tables (entries/meta/errors)
- [X] T041 [P] [US3] Implement Parquet reader and filtering/sorting utilities in `src/io/snapshot.rs`
- [X] T042 [US3] Extend `scan` to accept `--snapshot <FILE.parquet>` and write snapshot in `src/bin/dux.rs`
- [X] T043 [US3] Add `view --from-snapshot <FILE.parquet>` subcommand in `src/bin/dux.rs`

Checkpoint: Snapshot workflows validated and independently testable ✓ COMPLETE

---

## Phase 6: User Story 4 - 大規模環境での安定動作 (Priority: P3)

Goal: Ensure processing completes without crashes on very large trees (up to ~10M files), with bounded memory and resilience to intermittent errors.

Independent Test: On a synthesized large fixture, traversal completes; memory and runtime stay within targets; intermittent errors are reported without aborting.

### Tests for User Story 4

- [X] T044 [P] [US4] Performance smoke test harness (bounded runtime) in `tests/integration/test_perf_smoke.rs`
- [X] T045 [US4] Resilience test: simulate permission errors and file churn; ensure non-zero error count but overall completion in `tests/integration/test_resilience.rs`

### Implementation for User Story 4

- [X] T046 [P] [US4] Add error aggregation and representative examples in Summary (limit count) in `src/lib.rs`
- [X] T047 [US4] Optimize traversal memory footprint (small LRU for parent accumulation) in `src/services/aggregate.rs`
- [X] T048 [US4] Implement optional cap/fallback for hardlink dedupe structure to avoid unbounded growth in `src/services/traverse.rs`
- [X] T049 [US4] Add `--no-progress` and `--verbose` toggles with lightweight progress estimates in `src/bin/dux.rs`

Checkpoint: All user stories independently functional; stability validated ✓ COMPLETE

---

## Phase N: Polish & Cross-Cutting Concerns

Purpose: Improvements that affect multiple user stories

- [X] T050 [P] Documentation updates in `specs/001-disk-usage-cli/quickstart.md` and root `README.md`
- [X] T051 Code cleanup and refactoring passes across `src/` modules (no behavior change)
- [X] T052 Performance profiling notes and TODOs captured in `specs/001-disk-usage-cli/research.md`
- [X] T053 [P] Additional unit tests to reach coverage targets in `tests/unit/`
- [X] T054 Security/robustness hardening for path handling and output encoding (audit) across `src/`
- [X] T055 Run quickstart.md validation by executing documented commands locally; adjust examples in `specs/001-disk-usage-cli/quickstart.md`

✓ ALL PHASES COMPLETE

---

## Dependencies & Execution Order

### Phase Dependencies

- Setup (Phase 1): No dependencies - can start immediately
- Foundational (Phase 2): Depends on Setup completion - BLOCKS all user stories
- User Stories (Phase 3+): All depend on Foundational phase completion
  - User stories can proceed in parallel (if staffed)
  - Or sequentially in priority order (P1 → P2 → P3)
- Polish (Final Phase): Depends on all desired user stories being complete

### User Story Dependencies

- User Story 1 (P1): Can start after Foundational; no dependency on other stories
- User Story 2 (P2): Can start after Foundational; independently testable from US1
- User Story 3 (P2): Can start after Foundational; independently testable from US1/US2
- User Story 4 (P3): Can start after Foundational; independently testable (perf-focused)

### Within Each User Story

- Tests MUST be written and FAIL before implementation
- Models before services
- Services before CLI
- Core implementation before integration
- Story complete before moving to next priority

### Parallel Opportunities

- All [P] tasks in Setup and Foundational can run in parallel
- Once Foundational completes, all user stories can start in parallel
- Within stories, [P] tests and module implementations can be parallelized (different files)

---

## Parallel Example: User Story 1

```bash
# Launch all tests for User Story 1 together (example):
Task: "Unit tests for aggregation and size basis conversions in tests/unit/aggregate_tests.rs"
Task: "CLI/Integration test for dux scan in tests/integration/test_scan.rs"
Task: "JSON output smoke test for --json in tests/contract/test_json_shape.rs"

# Launch independent modules for User Story 1 together:
Task: "Implement sorting and top-K in src/services/aggregate.rs"
Task: "Implement CLI arg parsing in src/bin/dux.rs"
```

## Parallel Example: User Story 2

```bash
# Launch tests for User Story 2 together (example):
Task: "Unit tests for depth limiting behavior in tests/unit/depth_tests.rs"
Task: "Integration test for drill equivalence in tests/integration/test_drill.rs"

# Launch independent modules for User Story 2 together:
Task: "Extend CLI with drill and --max-depth in src/bin/dux.rs"
Task: "Add re-rooting and depth limiting in src/services/traverse.rs"
```

## Parallel Example: User Story 3

```bash
# Launch tests for User Story 3 together (example):
Task: "Snapshot round-trip test in tests/integration/test_snapshot_roundtrip.rs"
Task: "Snapshot JSON schema test in tests/contract/test_snapshot_json.rs"

# Launch independent modules for User Story 3 together:
Task: "Implement Parquet writer in src/io/snapshot.rs"
Task: "Implement Parquet reader in src/io/snapshot.rs"
```

## Parallel Example: User Story 4

```bash
# Launch tests for User Story 4 together (example):
Task: "Performance smoke test harness in tests/integration/test_perf_smoke.rs"
Task: "Resilience test with permission errors in tests/integration/test_resilience.rs"

# Launch independent modules for User Story 4 together:
Task: "Error aggregation in src/lib.rs"
Task: "LRU optimization in src/services/aggregate.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL - blocks all stories)
3. Complete Phase 3: User Story 1
4. STOP and VALIDATE: Test User Story 1 independently (unit + integration + JSON)
5. Demo the CLI

### Incremental Delivery

1. Complete Setup + Foundational → Foundation ready
2. Add User Story 1 → Test independently → Demo (MVP!)
3. Add User Story 2 → Test independently → Demo
4. Add User Story 3 → Test independently → Demo
5. Add User Story 4 → Test independently → Document perf/stability

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Once Foundational is done:
   - Developer A: User Story 1
   - Developer B: User Story 2
   - Developer C: User Story 3
   - Developer D: User Story 4 (perf)
3. Stories complete and integrate independently

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story is independently completable and testable
- Verify tests fail before implementing
- Commit after each task or logical group
- Avoid: vague tasks, same file conflicts, cross-story dependencies that break independence
