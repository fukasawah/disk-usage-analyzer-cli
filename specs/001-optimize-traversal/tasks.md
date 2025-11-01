---
description: "Task list for traversal optimization implementation"
---

# Tasks: Traversal Performance Optimization

**Input**: Design documents from `/specs/001-optimize-traversal/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, quickstart.md, contracts/openapi.yaml

**Tests**: Tests are REQUIRED per constitution. Each user story includes unit + integration coverage, parity checks between legacy and optimized modes, JSON schema assertions, and performance/size benchmarks to confirm ≤3 s SLO and ≤6 MB release binary.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Baseline toolchain and benchmarking guardrails

- [ ] T001 Create Rust toolchain pin for 1.90.0 in `rust-toolchain.toml`
- [ ] T002 Update `scripts/benchmark.sh` to record optimized vs legacy timings and fail if `target/release/dua` exceeds 6 MB

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core traversal scaffolding required by all stories

- [ ] T003 Add `rayon`, `windows`, and `rustix` dependencies with minimal features in `Cargo.toml`
- [ ] T004 Scaffold traversal dispatcher in `src/services/traverse/mod.rs` with documented invariants
- [ ] T005 Define `TraversalStrategy` trait and shared contracts in `src/services/traverse/strategy.rs`
- [ ] T006 Create progress throttling stub in `src/services/traverse/progress.rs`
- [ ] T007 Update concurrent aggregation helpers in `src/services/aggregate.rs` for threaded reductions
- [ ] T008 Expose new traversal module exports from `src/lib.rs`

**Checkpoint**: Foundation ready – user story implementation can now begin.

---

## Phase 3: User Story 1 – Rapid local audit (Priority: P1) 🎯 MVP

**Goal**: Deliver NTFS-optimized traversal that meets the 3 s p95 SLO while preserving output parity.

**Independent Test**: Run `scripts/benchmark.sh` against the 90k-file NTFS dataset and verify optimized run completes ≤3 s with totals within ≤1% or 10 MB of `--legacy-traversal`.

### Tests for User Story 1

- [ ] T009 [P] [US1] Add NTFS benchmark regression in `tests/integration/test_perf_smoke.rs` enforcing ≤3 s p95
- [ ] T010 [P] [US1] Add optimized vs legacy parity integration in `tests/integration/test_scan.rs`
- [ ] T011 [US1] Extend aggregator concurrency assertions in `tests/unit/aggregate_tests.rs` for threaded folds

### Implementation for User Story 1

- [ ] T012 [US1] Implement Win32 large-fetch traversal pipeline in `src/services/traverse/windows.rs`
- [ ] T013 [US1] Register NTFS strategy selection in `src/services/traverse/mod.rs`

**Checkpoint**: Windows optimized traversal meets performance and parity requirements.

---

## Phase 4: User Story 2 – Filesystem-aware strategy (Priority: P2)

**Goal**: Auto-select fastest traversal per filesystem and expose operator overrides.

**Independent Test**: Run CLI on NTFS, APFS, and ext4 fixtures to confirm strategy reporting, override flag behavior, and matching totals between strategies.

### Tests for User Story 2

- [ ] T014 [P] [US2] Cover strategy detection permutations in `tests/unit/traverse_tests.rs`
- [ ] T015 [P] [US2] Add CLI flag parsing tests in `tests/unit/cli_args_tests.rs`
- [ ] T016 [P] [US2] Ensure JSON snapshot strategy field contract in `tests/contract/test_snapshot_json.rs`

### Implementation for User Story 2

- [ ] T017 [US2] Implement filesystem detection helper in `src/services/traverse/detect.rs`
- [ ] T018 [US2] Implement POSIX `openat` traversal with batching in `src/services/traverse/posix.rs`
- [ ] T019 [US2] Wire override and legacy flags in `src/cli/args.rs`
- [ ] T020 [US2] Surface strategy selection in verbose output via `src/cli/output.rs`
- [ ] T021 [US2] Embed strategy metadata in progress snapshots within `src/io/snapshot.rs`

**Checkpoint**: CLI auto-detects strategies, honors overrides, and reports selected backend.

---

## Phase 5: User Story 3 – Predictable progress on slower media (Priority: P3)

**Goal**: Provide throttled, trustworthy progress updates and cancellation handling when scans exceed the SLO.

**Independent Test**: Run slow-media fixture to confirm progress events emit every ≤2 s (stderr and JSON) and totals remain accurate within 2%.

### Tests for User Story 3

- [ ] T022 [P] [US3] Add slow-media progress cadence test in `tests/integration/test_resilience.rs`
- [ ] T023 [P] [US3] Extend progress JSON schema assertions in `tests/contract/test_snapshot_json.rs`

### Implementation for User Story 3

- [ ] T024 [US3] Finalize byte/time-based throttling logic in `src/services/traverse/progress.rs`
- [ ] T025 [US3] Emit throttled progress snapshots from traversal loops in `src/services/traverse/windows.rs`
- [ ] T026 [US3] Emit throttled progress snapshots from traversal loops in `src/services/traverse/posix.rs`
- [ ] T027 [US3] Add `--progress-interval` flag handling and plumbing in `src/cli/args.rs`
- [ ] T028 [US3] Pass progress configuration through dispatcher in `src/services/traverse/mod.rs`

**Checkpoint**: Progress reporting is predictable, cancellable, and consistent across outputs.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Documentation, validation, and evidence capture across stories

- [ ] T029 [P] Refresh CLI documentation with new flags and strategy behavior in `README.md`
- [ ] T030 [P] Update developer quickstart with progress flag usage in `specs/001-optimize-traversal/quickstart.md`
- [ ] T031 Record benchmark + binary size evidence in `specs/001-optimize-traversal/research.md`
- [ ] T032 Summarize final success metrics and clarifications in `specs/001-optimize-traversal/spec.md`
- [ ] T033 Run and document final `cargo fmt`, `cargo clippy -D warnings`, `cargo test`, and `scripts/benchmark.sh` results in `specs/001-optimize-traversal/tasks.md`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 → Phase 2**: Toolchain and benchmark guards must exist before scaffolding traversal modules.
- **Phase 2 → Phases 3–5**: Dispatcher, strategy trait, and aggregation support are prerequisites for all user stories.
- **Phases 3–5**: Execute in priority order (US1 → US2 → US3) or in parallel once Phase 2 completes, provided shared files are coordinated.
- **Phase 6**: Execute after targeted user stories finish.

### User Story Dependencies

- **US1**: Depends on foundational modules; no other story prerequisites.
- **US2**: Depends on US1 dispatcher wiring to extend detection and overrides.
- **US3**: Depends on US1/US2 implementations to emit progress from both strategies.

### Within Each User Story

1. Tests (T009/T014/T022) should be authored first to fail.
2. Implement strategy logic (T012/T017/T018/T024–T028).
3. Integrate CLI/reporting updates (T019–T021, T027–T028).
4. Re-run tests/benchmarks before proceeding to next story.

### Parallel Opportunities

- [P] tasks in Setup (T002) and Polish (T029–T030) can run independently.
- Within US1, T009 and T010 can be developed concurrently.
- Within US2, T014–T016 can proceed in parallel while implementation tasks begin once dispatcher hooks land.
- Within US3, T022 and T023 can execute concurrently before wiring progress emission.

---

## Parallel Example: User Story 2

```bash
# Parallel code/test tasks after foundational work
Task T014: Expand strategy detection cases in tests/unit/traverse_tests.rs
Task T015: Add CLI override parsing coverage in tests/unit/cli_args_tests.rs
Task T016: Update contract snapshot JSON assertions
```

---

## Implementation Strategy

### MVP First (User Story 1)

1. Complete Phases 1–2.
2. Land US1 (T009–T013) and validate benchmarks ≤3 s with parity guard.
3. Ship MVP to stakeholders for feedback.

### Incremental Delivery

1. Extend to US2 (T014–T021) for cross-platform strategy awareness and operator overrides.
2. Add US3 (T022–T028) to improve transparency on slow media.
3. Finish with Phase 6 polish tasks capturing documentation and evidence.

### Parallel Team Strategy

- Developer A: Focus on US1 implementation (T009–T013).
- Developer B: Build detection + POSIX traversal for US2 (T014–T021).
- Developer C: Own progress enhancements for US3 (T022–T028).
- Share benchmarks + documentation via Phase 6 tasks once stories converge.
