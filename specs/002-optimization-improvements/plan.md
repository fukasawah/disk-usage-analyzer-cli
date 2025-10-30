# Implementation Plan: Optimization & Quality Improvements

**Branch**: `002-optimization-improvements` | **Date**: 2025-10-30 | **Spec**: [spec.md](spec.md)  
**Input**: Feature specification from `/specs/002-optimization-improvements/spec.md`

**Note**: This plan follows the speckit workflow. Phase 0 research and Phase 1 design artifacts are generated alongside this plan.

## Summary

**Goal**: Enhance rs-disk-usage CLI with Windows platform support, reduce binary size from 13MB to <7MB (stretch: <5MB), improve test organization and coverage (target: ≥80%), and optimize Parquet dependency configuration for minimal footprint.

**Approach**: 
1. **Windows Support**: Verify and enhance existing Windows-specific code (GetCompressedFileSizeW via windows-sys), add Windows path handling tests, ensure MSVC build succeeds
2. **Binary Size Reduction**: Enable LTO, strip debug symbols, analyze with cargo-bloat, optimize Parquet features (disable unused compression codecs and async features), document size contributors
3. **Test Quality**: Audit all test names (especially "drill" tests), remove/update tests for deprecated functionality, measure coverage with cargo-tarpaulin (Linux) or cargo-llvm-cov (cross-platform), add tests for uncovered critical paths
4. **Performance Validation**: Ensure optimizations don't regress scan/view performance beyond 5%/10% tolerance

This is a polish/quality phase building on the MVP (001-disk-usage-cli). No new user-facing features; focus on maintainability, cross-platform support, and distribution efficiency.

## Technical Context

**Language/Version**: Rust 1.77+ (Edition 2024) - already established in MVP  
**Primary Dependencies**: 
- Existing: `serde`, `serde_json`, `parquet`, `arrow-array`, `arrow-schema`, `windows-sys` (conditional)
- New (dev tools): `cargo-bloat` (binary size analysis), `cargo-tarpaulin` or `cargo-llvm-cov` (coverage measurement)

**Storage**: Parquet snapshots (existing) - will optimize feature flags  
**Testing**: `cargo test` (existing 24 tests) + coverage tooling + new Windows tests  
**Target Platform**: 
- Current: Linux (primary)
- Adding: Windows 10/11 with MSVC toolchain (x86_64-pc-windows-msvc)
- Best-effort: macOS (Unix variant, should work)

**Project Type**: Single binary crate (established in MVP)  
**Performance Goals**: 
- No regression: Scan performance ≤5% slower, view performance ≤10% slower (measured against MVP baseline)
- Binary size: 13MB → <7MB (acceptable), <5MB (stretch)
- Build time: <3 minutes on standard CI hardware (with LTO acceptable)

**Constraints**: 
- All 24 existing tests must continue to pass on Linux
- Windows tests must validate basic functionality (scan, view, physical size)
- No breaking changes to CLI interface
- JSON schema unchanged

**Scale/Scope**: 
- Build optimization: One-time analysis and configuration
- Test coverage: 80% target for src/services/*, src/io/* modules
- Windows platform: Basic integration tests (not exhaustive Windows feature testing)

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

Required gates based on repository constitution:

### Pre-Design Gate Evaluation

- **Code Quality**: 
  - ✅ Plan maintains modular structure (no new modules, refining existing)
  - ✅ Windows-specific code will use `#[cfg(windows)]` gates
  - ✅ No new unsafe code or `unwrap()` in production paths
  - ✅ Build configuration changes will be documented
  - **PASS (planned)**

- **Testing**: 
  - ✅ Plan includes test audit and cleanup (FR-011 to FR-014)
  - ✅ Coverage target ≥80% for core modules (FR-017)
  - ✅ New Windows tests will be deterministic and fast
  - ✅ All 24 existing tests must pass (non-regression requirement)
  - **PASS (planned)**

- **UX Consistency**: 
  - ✅ No CLI changes (NFR-UX/CLI explicitly states no interface changes)
  - ✅ Windows paths displayed with backslashes (FR-003)
  - ✅ Exit codes unchanged
  - ✅ JSON schema unchanged
  - **PASS**

- **Performance**: 
  - ✅ SLOs clearly stated: <5% scan regression, <10% view regression (NFR-Perf)
  - ✅ Benchmark plan: 10K file benchmark before/after optimization
  - ✅ Binary size reduction is explicit goal with measurement (cargo bloat)
  - ✅ Build time impact documented (LTO acceptable if <2x increase)
  - **PASS (planned)**

### Post-Design Re-check

Will be performed after Phase 1 design (research.md, data-model.md, contracts) to validate:
- No new complexity introduced
- Test plan covers all changes
- Performance benchmarks defined
- Documentation updated

## Project Structure

### Documentation (this feature)

```text
specs/002-optimization-improvements/
├── plan.md              # This file (speckit.plan output)
├── research.md          # Phase 0 (Windows APIs, Parquet features, coverage tools, size optimization strategies)
├── data-model.md        # Phase 1 (N/A for this feature - no new data entities)
├── quickstart.md        # Phase 1 (Updated build instructions for Windows, coverage, optimization)
└── contracts/
    └── openapi.yaml     # Phase 1 (N/A for this feature - no API changes)
```

**Note**: data-model.md and contracts/openapi.yaml may be minimal or N/A since this feature doesn't introduce new data entities or APIs.

### Source Code (repository root)

**No new source structure** - this feature refines existing MVP code:

```text
src/
├── cli/                 # Existing - no changes except possible Windows path display tweaks
├── models/              # Existing - no changes
├── services/
│   ├── traverse.rs      # Existing - verify Windows filesystem boundary detection
│   ├── aggregate.rs     # Existing - no changes
│   ├── size.rs          # Existing - verify Windows GetCompressedFileSizeW implementation
│   └── format.rs        # Existing - possible Windows path display adjustment
├── io/
│   └── snapshot.rs      # Existing - audit for Parquet feature optimization
└── lib.rs               # Existing - no changes

tests/
├── unit/                # Existing - audit and refine test names
├── integration/         # Existing - audit "drill" tests, add Windows tests
└── contract/            # Existing - verify JSON schema unchanged

# New: Build configuration changes
Cargo.toml               # Update: LTO, strip, minimal Parquet features
.cargo/config.toml       # Possible: Windows-specific build settings
```

### New/Modified Files for this Feature

1. **Cargo.toml**:
   - Add `[profile.release]` with `lto = true`, `codegen-units = 1`, `strip = true`
   - Refine `parquet` dependency: `default-features = false, features = [...]` (minimal set)
   - Possibly: `opt-level = "z"` or `"s"` for size optimization

2. **Build/Documentation**:
   - `BUILD.md` or `README.md`: Document Windows build instructions, coverage tooling, size optimization
   - `.github/workflows/ci.yml` (if exists): Add Windows build, coverage reporting

3. **Tests**:
   - New: `tests/integration/test_windows_platform.rs` (or similar) for Windows-specific tests
   - Modified: Rename/update tests with misleading names (e.g., drill tests)
   - New: `tests/coverage.sh` script for generating coverage reports

4. **Dev Tools** (not checked in, documented):
   - `cargo install cargo-bloat`
   - `cargo install cargo-tarpaulin` (Linux) or `cargo install cargo-llvm-cov` (cross-platform)

**Structure Decision**: This feature maintains the single binary crate structure established in MVP (001-disk-usage-cli). No new modules or reorganization needed. Focus is on configuration tuning (Cargo.toml), test quality (test file refinements), and validation (Windows platform tests, coverage measurement).

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

**No violations** - this feature operates within existing architecture and constitution requirements:
- No new unsafe code or unwrap usage
- Testing requirements exceeded (adding coverage measurement)
- UX consistency maintained (no interface changes)
- Performance explicitly validated (regression testing)

This table is intentionally empty.

## Phase 0: Research (Unknowns to Resolve)

The following unknowns must be researched and documented in `research.md`:

### 1. Windows Build & Platform Support

**Research Tasks**:
- Verify existing `windows-sys` usage in `src/services/size.rs` for GetCompressedFileSizeW
- Document Windows MSVC toolchain setup (rustup target add x86_64-pc-windows-msvc)
- Research Windows filesystem boundary detection (if not already implemented)
- Identify Windows-specific path handling requirements (backslash normalization)
- Research Windows hardlink tracking requirements (currently placeholder - out of scope but document limitations)

**Decision Points**:
- Is GetCompressedFileSizeW implementation correct for NTFS compressed/sparse files?
- Do we need special handling for Windows junction points vs symlinks?
- What Windows-specific tests are minimum viable (scan, view, size computation)?

### 2. Binary Size Optimization

**Research Tasks**:
- Run `cargo bloat --release` on current 13MB binary to identify top contributors
- Research Parquet crate features and determine which are unused:
  - Compression codecs: snappy, gzip, brotli, zstd, lz4 (which are needed?)
  - Async runtime features (likely unused for synchronous file I/O)
  - Arrow IPC features (likely unused)
- Research LTO (Link Time Optimization) impact:
  - Build time increase (acceptable if <2x)
  - Binary size reduction (expected 20-40%)
- Research strip options: `strip = true` vs `strip = "debuginfo"` vs `strip = "symbols"`
- Research `opt-level` options: `"3"` (default) vs `"z"` (size) vs `"s"` (speed+size)

**Decision Points**:
- What is acceptable build time increase for LTO? (Constitution: <3min total)
- Which Parquet features can be safely disabled without breaking snapshot read/write?
- Should we use `opt-level = "z"` for maximum size reduction or stick with `"3"`?
- Is `codegen-units = 1` necessary or does `lto = "fat"` suffice?

### 3. Test Coverage Tooling

**Research Tasks**:
- Compare cargo-tarpaulin (Linux only, mature) vs cargo-llvm-cov (cross-platform, newer)
- Document installation and usage for chosen tool
- Research coverage report formats: terminal output, HTML, lcov
- Research integration with CI (GitHub Actions coverage reporting)
- Research exclusion patterns for generated code, tests themselves

**Decision Points**:
- Which coverage tool to standardize on? (Recommendation: cargo-llvm-cov for cross-platform)
- What coverage metrics to track? (Line coverage primary, branch coverage secondary)
- Should coverage be enforced in CI? (Spec says generate reports, not enforce)

### 4. Test Organization & Naming

**Research Tasks**:
- Audit all tests with "drill" in name:
  - `tests/integration/test_drill.rs` - does this test actual drill functionality or view --path?
  - Any other misleading test names?
- Document test naming conventions (should match functionality tested)
- Research test organization best practices for Rust projects

**Decision Points**:
- Is "drill" command still relevant? (MVP integrated drill into view --path)
- Should test_drill.rs be renamed to test_drill_down.rs or test_view_with_path.rs?
- Are there other tests with misleading names?

### 5. Parquet Usage Audit

**Research Tasks**:
- Review `src/io/snapshot.rs` to document all Parquet APIs used:
  - Schema definition (arrow_schema)
  - Record batch creation (arrow_array)
  - Writer (parquet::arrow::ArrowWriter)
  - Reader (parquet::arrow::arrow_reader)
- Identify which Parquet features these APIs require
- Research Parquet default features and how to minimize them

**Decision Points**:
- Can we use `parquet = { version = "53.2", default-features = false, features = ["arrow", "..."] }`?
- What is the minimal viable feature set for our usage?
- Will disabling compression codecs affect existing snapshots? (Need backward compatibility test)

**Output**: `research.md` with all decisions documented, alternatives considered, and rationales provided.

## Phase 1: Design & Contracts

**Prerequisites**: `research.md` complete with all unknowns resolved

### 1. Data Model (`data-model.md`)

**Status**: **N/A** - This feature does not introduce new data entities.

**Existing Data Model** (from MVP, unchanged):
- `DirectoryEntry`: path, parent_path, depth, size_bytes, file_count, dir_count
- `SnapshotMeta`: scan_root, started_at, finished_at, size_basis, hardlink_policy, excludes
- `ErrorItem`: path, code, message

**No changes required** for this feature. Document in data-model.md that existing structures are sufficient.

### 2. API Contracts (`contracts/openapi.yaml`)

**Status**: **N/A** - This feature does not change CLI interface or add new APIs.

**Existing CLI Contract** (from MVP, unchanged):
- `dux scan <PATH> --snapshot <FILE>` - no changes
- `dux view <SNAPSHOT> [--path <SUBDIR>]` - no changes
- `dux drill <ROOT> <SUBDIR>` - deprecated but not removed in this feature
- Exit codes: 0 (success), 2 (invalid input), 3 (partial failure), 4 (I/O error)
- JSON schema: unchanged

**No OpenAPI/GraphQL schema needed** - CLI tool only. Document in contracts/ that CLI interface is stable.

### 3. Quickstart Guide (`quickstart.md`)

Update quickstart to include:

**New Sections**:
1. **Windows Build Instructions**:
   ```bash
   # On Windows with MSVC toolchain
   rustup target add x86_64-pc-windows-msvc
   cargo build --release --target x86_64-pc-windows-msvc
   ```

2. **Binary Size Optimization** (for developers):
   ```bash
   # Analyze binary size
   cargo install cargo-bloat
   cargo bloat --release -n 20

   # Build with optimizations (already in Cargo.toml)
   cargo build --release
   # Results in ~7MB binary (down from 13MB)
   ```

3. **Test Coverage** (for developers):
   ```bash
   # Install coverage tool
   cargo install cargo-llvm-cov

   # Generate coverage report
   cargo llvm-cov --html --open
   # View coverage in browser

   # Or terminal output
   cargo llvm-cov --all-features --workspace
   ```

4. **Windows Usage Examples**:
   ```cmd
   REM Windows command prompt
   dux scan C:\Users --snapshot users.parquet
   dux view users.parquet --top 10
   dux view users.parquet --path "C:\Users\Username\Documents"
   ```

**Updated Sections**:
- Build section: Add Windows-specific notes
- Testing section: Add coverage measurement instructions
- Development section: Add size analysis and optimization notes

### 4. Agent Context Update

Run `.specify/scripts/bash/update-agent-context.sh copilot` to update `.github/copilot-instructions.md` with:
- Windows platform support (x86_64-pc-windows-msvc)
- New dev tools: cargo-bloat, cargo-llvm-cov/cargo-tarpaulin
- Build optimization settings: LTO, strip, minimal features
- Test coverage requirements: ≥80% for core modules

**Output**: Updated `.github/copilot-instructions.md` with optimization and Windows support context.

## Phase 2: Task Breakdown (Not Generated by `/speckit.plan`)

**Note**: Task breakdown is performed by the `/speckit.tasks` command, which generates `tasks.md`.

The `/speckit.tasks` command will create prioritized tasks covering:
1. Windows platform support (P1)
2. Binary size optimization (P2)
3. Test organization and coverage (P2)
4. Parquet dependency optimization (P3)

Each task will include:
- Acceptance criteria from spec
- File paths to modify
- Testing requirements
- Dependencies on other tasks

## Unknowns & Clarifications

All unknowns identified in Technical Context are addressed in Phase 0 research:
1. Windows physical size API correctness → Research Task 1
2. Windows filesystem boundary detection → Research Task 1
3. Parquet feature minimization → Research Task 2 & 5
4. Coverage tool selection → Research Task 3
5. Test naming audit → Research Task 4

**No additional clarifications needed** - all requirements are clear from spec.

## Success Metrics

This plan will be considered successful when:

1. **Research Complete** (Phase 0):
   - All 5 research tasks documented in `research.md`
   - Decisions made with rationales
   - No remaining "NEEDS CLARIFICATION" markers

2. **Design Complete** (Phase 1):
   - `data-model.md` confirms existing structures sufficient
   - `contracts/` documents stable CLI interface
   - `quickstart.md` updated with Windows, coverage, and optimization instructions
   - Agent context updated with new tools and settings

3. **Constitution Gates Pass** (Re-check after Phase 1):
   - Code quality: Windows cfg gates documented, no new complexity
   - Testing: Coverage plan clear, Windows tests specified
   - UX: No interface changes confirmed
   - Performance: Benchmark plan documented

4. **Ready for Implementation** (`/speckit.tasks`):
   - Clear understanding of Windows build requirements
   - Parquet feature optimization strategy defined
   - Test coverage tooling selected and documented
   - Size optimization approach validated (LTO + features + strip)

**Current Status**: Ready to proceed with Phase 0 research.
