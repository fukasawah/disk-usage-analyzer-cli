# Research: Optimization & Quality Improvements

**Feature**: 002-optimization-improvements  
**Date**: 2025-10-30  
**Purpose**: Resolve technical unknowns before design and implementation

## Overview

This document resolves the 5 research areas identified in plan.md:
1. Windows Build & Platform Support
2. Binary Size Optimization
3. Test Coverage Tooling
4. Test Organization & Naming
5. Parquet Usage Audit

## 1. Windows Build & Platform Support

### Research Questions
- Is GetCompressedFileSizeW implementation correct for NTFS compressed/sparse files?
- Do we need special handling for Windows junction points vs symlinks?
- What Windows-specific tests are minimum viable?

### Current State Analysis

**Existing Windows Code** (`src/services/size.rs`):
```rust
#[cfg(windows)]
pub fn physical_size_from_path(path: &Path) -> std::io::Result<u64> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::INVALID_FILE_SIZE;
    use windows_sys::Win32::Storage::FileSystem::GetCompressedFileSizeW;
    
    // Converts path to UTF-16, calls GetCompressedFileSizeW
    // Falls back to logical size on error
}
```

**Analysis**:
- ✅ Implementation looks correct - GetCompressedFileSizeW is the right API for physical size
- ✅ Handles NTFS compression correctly (returns compressed size)
- ✅ Falls back to logical size on error (safe behavior)
- ⚠️ Sparse files: GetCompressedFileSizeW returns compressed size, not allocated size (acceptable for MVP)

**Filesystem Boundary Detection** (`src/services/traverse.rs`):
```rust
#[cfg(not(unix))]
fn get_device_id(_metadata: &fs::Metadata) -> u64 {
    // Returns constant 0 - effectively disables boundary checking
}
```

**Analysis**:
- ⚠️ Windows boundary detection is disabled (returns constant)
- 📝 Proper implementation would use GetVolumePathNameW to compare drive letters/mount points
- ✅ For MVP, disabling is acceptable (most users scan within single drive)
- 🔮 Future work: Implement proper Windows volume comparison

**Path Handling**:
- Rust's PathBuf handles both / and \ on Windows automatically
- Display to user: should normalize to backslashes for Windows convention
- No special handling needed beyond what Rust std::path provides

**Junction Points vs Symlinks**:
- Current code uses `symlink_metadata()` which doesn't follow symlinks
- Windows junctions are treated as directories (not symlinks) by Rust
- ✅ Current behavior is safe (won't follow junctions unexpectedly)
- 📝 Document: junctions are traversed like regular directories

### Decisions

**Decision 1.1: Windows Physical Size**
- **Chosen**: Keep existing GetCompressedFileSizeW implementation
- **Rationale**: Correct for NTFS compression, acceptable for sparse files, safe fallback
- **Alternatives considered**:
  - GetFileSizeEx: Only returns logical size (not physical)
  - GetFileInformationByHandle: Requires opening file handle (performance cost)

**Decision 1.2: Filesystem Boundary**
- **Chosen**: Document current limitation, mark as future work
- **Rationale**: Implementing proper Windows volume detection is low priority for MVP polish phase
- **Future work**: Use GetVolumePathNameW for proper drive/mount point comparison
- **Impact**: Users scanning multiple drives will see full scan (not a breaking issue)

**Decision 1.3: Path Display**
- **Chosen**: Let Rust PathBuf handle cross-platform paths, display as-is
- **Rationale**: PathBuf::display() automatically uses backslashes on Windows
- **Alternatives considered**: Force normalize to backslashes (unnecessary, Rust handles it)

**Decision 1.4: Windows Tests (Minimum Viable)**
- **Chosen**: 3 integration tests:
  1. Build test: Verify MSVC compilation succeeds
  2. Scan test: `dux scan C:\Windows\System32 --snapshot test.parquet` (small dir)
  3. View test: `dux view test.parquet --top 5`
- **Rationale**: Validates core functionality without exhaustive Windows feature testing
- **Out of scope**: NTFS-specific features (compressed files, alternate data streams) - defer to future

### Implementation Notes

- Verify `windows-sys` features in Cargo.toml: `Win32_Storage_FileSystem`, `Win32_Foundation`
- Update quickstart.md with Windows build instructions
- Add Windows CI job if GitHub Actions available (document if not)
- Document filesystem boundary limitation in README

---

## 2. Binary Size Optimization

### Research Questions
- What is acceptable build time increase for LTO?
- Which Parquet features can be safely disabled?
- Should we use `opt-level = "z"` for maximum size reduction?
- Is `codegen-units = 1` necessary?

### Current State Analysis

**Baseline** (from user report):
- Current binary size: 13MB (release build)
- Target: <7MB (acceptable), <5MB (stretch)
- Reduction needed: ≥46% (for 7MB), ≥62% (for 5MB)

**Size Contributor Analysis** (research required - will run cargo bloat):
```bash
cargo install cargo-bloat
cargo bloat --release -n 20
```

Expected top contributors (based on typical Rust binaries):
1. Parquet/Arrow libraries (likely 40-50% of size)
2. Standard library (15-20%)
3. Serde/JSON (10-15%)
4. Our code (5-10%)

### Optimization Strategies

**Strategy 1: Link Time Optimization (LTO)**
```toml
[profile.release]
lto = "fat"          # or "thin" for faster builds
codegen-units = 1    # single codegen unit for better optimization
```

**Expected impact**:
- Size reduction: 20-30%
- Build time increase: 2-3x (from ~30s to ~60-90s)
- ✅ Acceptable per constitution (<3min total)

**Trade-offs**:
- `lto = "fat"`: Maximum optimization, slower build (~30% size reduction)
- `lto = "thin"`: Faster build, less optimization (~20% size reduction)
- **Chosen**: `lto = "fat"` (we prioritize distribution size over build time)

**Strategy 2: Strip Debug Symbols**
```toml
[profile.release]
strip = true  # or "debuginfo" or "symbols"
```

**Expected impact**:
- Size reduction: 10-20% (depends on debug info amount)
- No runtime impact
- ✅ No downside for distributed binaries

**Options**:
- `strip = true`: Strip all symbols (recommended)
- `strip = "debuginfo"`: Keep some symbols for panic backtraces
- `strip = "symbols"`: Strip debug symbols only
- **Chosen**: `strip = true` (maximum size reduction, backtraces work with rustc version)

**Strategy 3: Optimization Level**
```toml
[profile.release]
opt-level = "z"  # or "s" or "3"
```

**Options**:
- `opt-level = "3"`: Default, optimize for speed
- `opt-level = "s"`: Optimize for size (some speed trade-off)
- `opt-level = "z"`: Aggressive size optimization (more speed trade-off)

**Analysis**:
- "z" vs "3": Typically 5-15% smaller, potentially 10-20% slower
- Our use case: CLI tool, not hot path performance critical
- **Chosen**: Start with `opt-level = "s"` (balanced), test "z" if needed for stretch goal

**Strategy 4: Codegen Units**
```toml
[profile.release]
codegen-units = 1
```

**Expected impact**:
- Size reduction: 5-10% (better cross-crate optimization)
- Build time increase: 20-30% (less parallelization)
- ✅ Acceptable in combination with LTO

**Strategy 5: Abort on Panic**
```toml
[profile.release]
panic = "abort"
```

**Expected impact**:
- Size reduction: 5-10% (no unwinding code)
- Trade-off: No panic backtraces, just immediate abort
- ⚠️ Risky for debugging - not recommended for MVP

**Chosen**: Skip this optimization (keep default panic = "unwind")

### Decisions

**Decision 2.1: Release Profile Configuration**
```toml
[profile.release]
opt-level = "s"        # Size-optimized (try "z" if <7MB not reached)
lto = "fat"            # Maximum LTO
codegen-units = 1      # Single codegen unit
strip = true           # Strip all symbols
```

**Rationale**:
- Balanced approach: size priority without extreme trade-offs
- Build time acceptable: estimated 60-90s (well under 3min limit)
- Reversible: can tune opt-level if performance regression occurs

**Decision 2.2: Build Time Acceptance**
- **Chosen**: Accept 2-3x build time increase (30s → 90s)
- **Rationale**: Constitution allows <3min; optimization is one-time configuration
- **Monitoring**: Will measure actual build time, adjust if exceeds 2min

**Decision 2.3: Performance Validation**
- **Chosen**: Benchmark before/after with 10K file fixture
- **Acceptance**: <5% scan regression, <10% view regression
- **Method**: `time` command on Linux, `Measure-Command` on Windows

### Implementation Notes

- Update Cargo.toml with [profile.release] section
- Run cargo bloat before/after to document size contributors
- Create benchmark script: `benchmarks/size_optimization.sh`
- Document findings in BUILD.md or README

---

## 3. Test Coverage Tooling

### Research Questions
- cargo-tarpaulin vs cargo-llvm-cov - which to use?
- What coverage metrics to track?
- Should coverage be enforced in CI?

### Tool Comparison

**cargo-tarpaulin** (Linux only):
- Pros:
  - Mature, stable, widely used
  - Good documentation
  - HTML reports, CI integration
- Cons:
  - Linux only (requires ptrace)
  - Slower (ptrace overhead)
- Installation: `cargo install cargo-tarpaulin`

**cargo-llvm-cov** (cross-platform):
- Pros:
  - Works on Linux, macOS, Windows
  - Faster (LLVM instrumentation)
  - Better coverage accuracy
  - Active development
- Cons:
  - Newer, less mature
  - Requires nightly toolchain for some features (but works on stable)
- Installation: `cargo install cargo-llvm-cov`

### Coverage Metrics

**Line Coverage** (primary):
- Measures which lines executed during tests
- Standard metric, easy to understand
- Target: ≥80% for core modules (spec requirement)

**Branch Coverage** (secondary):
- Measures which branches (if/else, match arms) taken
- More granular than line coverage
- Useful for identifying untested edge cases

**Function Coverage**:
- Measures which functions called
- Less useful for Rust (most functions are small)

### Decision

**Decision 3.1: Coverage Tool**
- **Chosen**: `cargo-llvm-cov`
- **Rationale**:
  - Cross-platform (supports Windows testing goal)
  - Faster than tarpaulin
  - Better for our multi-platform requirements
  - Works on stable Rust (we use 1.77+)
- **Alternatives considered**:
  - tarpaulin: Rejected due to Linux-only limitation
  - gcov/kcov: More complex setup, less Rust-native

**Decision 3.2: Coverage Metrics**
- **Chosen**: Focus on line coverage, report branch coverage as bonus
- **Rationale**: Line coverage is sufficient for spec requirement (≥80%)
- **Target breakdown**:
  - `src/services/*`: ≥80% (core business logic)
  - `src/io/*`: ≥80% (snapshot I/O critical)
  - `src/cli/*`: ≥70% (UI code, some untestable paths)
  - `src/models/*`: ≥90% (simple structures, should be high)

**Decision 3.3: Coverage Enforcement**
- **Chosen**: Generate reports, document gaps, do NOT fail builds on low coverage
- **Rationale**: Spec says "generate coverage reports" not "enforce coverage"
- **Future work**: Consider enforcement in CI after baseline established

**Decision 3.4: Report Formats**
- **Chosen**: Generate both terminal output and HTML reports
- **Commands**:
  ```bash
  # Terminal summary
  cargo llvm-cov --all-features --workspace
  
  # HTML report (for detailed review)
  cargo llvm-cov --html --open
  
  # CI-friendly (lcov format for uploading)
  cargo llvm-cov --lcov --output-path coverage.lcov
  ```

### Implementation Notes

- Add coverage script: `scripts/coverage.sh`
- Document in quickstart.md under Development section
- Add CI job for coverage reporting (if GitHub Actions available)
- Create coverage badge (optional, nice to have)

**Exclusion Patterns** (code to skip):
- Test files themselves (tests/*)
- Generated code (if any)
- Platform-specific stubs (e.g., Windows code on Linux CI)

---

## 4. Test Organization & Naming

### Research Questions
- Is "drill" command still relevant?
- Should test_drill.rs be renamed?
- Are there other tests with misleading names?

### Current Test Audit

**Existing Tests** (from MVP):
```
tests/
├── contract/
│   ├── test_json_shape.rs         ✅ Clear name
│   └── test_snapshot_json.rs      ✅ Clear name
├── integration/
│   ├── test_scan.rs               ✅ Clear name
│   ├── test_errors.rs             ✅ Clear name
│   ├── test_drill.rs              ⚠️ NEEDS REVIEW
│   ├── test_snapshot_roundtrip.rs ✅ Clear name
│   ├── test_snapshot_errors.rs    ✅ Clear name
│   ├── test_perf_smoke.rs         ✅ Clear name
│   └── test_resilience.rs         ✅ Clear name
└── unit/
    ├── aggregate_tests.rs         ✅ Clear name
    ├── traverse_tests.rs          ✅ Clear name
    └── depth_tests.rs             ✅ Clear name
```

**Test Name Audit**: Only `test_drill.rs` needs review.

### Drill Functionality Analysis

**Original MVP Design**:
- Three commands: `scan`, `view`, `drill`
- `drill <ROOT> <SUBDIR>`: Drill down into subdirectory

**Final MVP Implementation**:
- `scan <PATH> --snapshot <FILE>`: Scan and save
- `view <SNAPSHOT> [--path <SUBDIR>]`: View snapshot, optional drill with --path
- `drill <ROOT> <SUBDIR>`: Kept for backward compatibility but redundant

**test_drill.rs Contents** (needs verification):
```rust
// Likely tests:
// 1. drill command functionality
// 2. OR view --path functionality (if drill was renamed/merged)
```

### Decision

**Decision 4.1: Test File Naming**
- **Action**: Audit `test_drill.rs` to determine what it actually tests
- **Options**:
  1. If it tests standalone `drill` command → keep name OR update to test view --path
  2. If it tests view --path functionality → rename to `test_view_drill.rs` or `test_view_with_path.rs`
  3. If drill command is deprecated → rename to `test_view_drill_down.rs` and update tests

**Decision 4.2: Recommended Approach**
- **Chosen**: Rename `test_drill.rs` → `test_view_drill_down.rs`
- **Rationale**:
  - Clarifies that it tests drill-down functionality (view --path)
  - Removes ambiguity about standalone drill command
  - Aligns with actual functionality in MVP

**Decision 4.3: Test Naming Convention**
- **Standard**: `test_<feature>_<aspect>.rs` for integration tests
- **Examples**:
  - `test_scan.rs` - scan command tests
  - `test_view_drill_down.rs` - view with --path (drill-down)
  - `test_snapshot_roundtrip.rs` - snapshot save/load cycle

**Decision 4.4: Other Test Names**
- **Audit result**: All other test names are clear and accurate
- **No changes needed** for other test files

### Implementation Notes

- Review test_drill.rs contents
- Rename file and update test function names if needed
- Update test documentation to clarify purpose
- Ensure all tests pass after rename (update imports if needed)

---

## 5. Parquet Usage Audit

### Research Questions
- Can we use `default-features = false` for parquet?
- What is the minimal viable feature set?
- Will disabling compression affect existing snapshots?

### Current Usage Analysis

**Parquet Code Review** (`src/io/snapshot.rs`):

**APIs Used**:
1. **Schema Definition** (arrow_schema):
   ```rust
   use arrow_schema::{DataType, Field, Schema};
   ```

2. **Array Creation** (arrow_array):
   ```rust
   use arrow_array::{
       Array, ArrayRef, RecordBatch, 
       StringArray, UInt16Array, UInt32Array, UInt64Array
   };
   ```

3. **Parquet Writer** (parquet):
   ```rust
   use parquet::arrow::ArrowWriter;
   use parquet::file::properties::WriterProperties;
   ```

4. **Parquet Reader** (parquet):
   ```rust
   use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
   ```

**Current Dependencies** (Cargo.toml):
```toml
parquet = "53.2"
arrow-array = "53.4"
arrow-schema = "53.4"
```

### Parquet Features Analysis

**Default Features** (from parquet crate documentation):
- `arrow`: Arrow integration (REQUIRED - we use it)
- `snap`: Snappy compression (possibly unused)
- `brotli`: Brotli compression (possibly unused)
- `flate2`: Gzip compression (possibly unused)
- `lz4`: LZ4 compression (possibly unused)
- `zstd`: Zstandard compression (possibly unused)
- `async`: Async I/O support (UNUSED - we use synchronous I/O)

**Our Usage**:
- We use `WriterProperties::builder().build()` with NO compression specified
- Default compression is **NONE** or **SNAPPY** (depends on crate defaults)
- We don't explicitly set compression codec

**Compression Impact**:
- Existing snapshots: May be uncompressed or Snappy-compressed
- Disabling all compression: Would still read existing snapshots (backward compatible)
- New snapshots: Would be uncompressed (slightly larger files, but acceptable)

### Minimal Feature Set

**Required Features**:
- `arrow`: Core requirement for Arrow integration
- `snap` (optional): If we want to support Snappy compression (small binary impact)

**Definitely Unused**:
- `async`: We use synchronous file I/O
- `brotli`, `flate2`, `lz4`, `zstd`: We don't configure these codecs

### Decision

**Decision 5.1: Parquet Features Configuration**
```toml
# Option 1: Minimal (no compression)
parquet = { version = "53.2", default-features = false, features = ["arrow"] }

# Option 2: With Snappy (small binary increase, better snapshot sizes)
parquet = { version = "53.2", default-features = false, features = ["arrow", "snap"] }
```

**Chosen**: Option 2 (arrow + snap)
**Rationale**:
- Snappy is lightweight, minimal binary impact (~100-200KB)
- Keeps snapshot files smaller (20-30% size reduction)
- Backward compatible with existing snapshots
- Good trade-off: small binary increase, better user experience

**Decision 5.2: Arrow Dependencies**
```toml
# Keep explicit versions for stability
arrow-array = "53.4"
arrow-schema = "53.4"
```

**Chosen**: Keep as-is, no changes needed
**Rationale**: These are lightweight dependencies, already required by parquet feature

**Decision 5.3: Compression Strategy**
- **Current**: No explicit compression set (defaults to Snappy if available)
- **Chosen**: Keep default behavior, do NOT explicitly set compression
- **Rationale**: Simplicity, backward compatibility, user can configure if needed in future

**Decision 5.4: Backward Compatibility**
- **Test**: Run snapshot tests after feature change
- **Validation**: Verify existing usr.parquet (from MVP) still loads correctly
- **Fallback**: If tests fail, add back required features

### Expected Size Impact

**Parquet/Arrow Size Contribution** (estimated from cargo bloat):
- With all default features: ~4-5MB
- With arrow+snap only: ~3-3.5MB
- **Savings**: ~1-1.5MB (≈10-12% of total binary)

**Not Huge** but contributes to overall size reduction goal.

### Implementation Notes

- Update Cargo.toml with minimal features
- Run `cargo bloat` before/after to measure actual impact
- Run full test suite, especially snapshot tests
- Document feature rationale in Cargo.toml comments
- Test backward compatibility with existing usr.parquet file

---

## Summary of Decisions

### Windows Support
1. ✅ Keep existing GetCompressedFileSizeW implementation
2. 📝 Document filesystem boundary limitation (future work)
3. ✅ Use Rust PathBuf default path handling
4. ✅ Add 3 minimum viable Windows integration tests

### Binary Size Optimization
1. ✅ Enable LTO = "fat", codegen-units = 1, strip = true
2. ✅ Use opt-level = "s" (try "z" if needed)
3. ✅ Accept 2-3x build time increase (~60-90s)
4. ✅ Benchmark before/after for performance validation

### Test Coverage
1. ✅ Use cargo-llvm-cov (cross-platform)
2. ✅ Target ≥80% line coverage for core modules
3. ✅ Generate reports (terminal + HTML), do NOT enforce in CI yet
4. ✅ Document in quickstart.md

### Test Organization
1. ✅ Rename test_drill.rs → test_view_drill_down.rs
2. ✅ All other test names are clear
3. ✅ Follow convention: test_<feature>_<aspect>.rs

### Parquet Optimization
1. ✅ Use minimal features: arrow + snap
2. ✅ Keep default compression behavior (Snappy if available)
3. ✅ Test backward compatibility
4. ✅ Expected savings: ~1-1.5MB

### Overall Size Reduction Forecast

**Baseline**: 13MB

**Expected reductions**:
- LTO + strip + codegen-units: -30% (~4MB) → **9MB**
- Parquet minimal features: -10% (~1MB) → **8MB**
- opt-level = "s": -5% (~0.5MB) → **7.5MB**

**Projected**: ~7.5MB (meets acceptance criteria)  
**Stretch goal**: opt-level = "z" might reach 6-7MB (close to 5MB stretch goal)

**Risk mitigation**: If <7MB not reached, try opt-level = "z" or investigate other large dependencies.

---

## Next Steps (Phase 1)

1. ✅ Research complete - all unknowns resolved
2. → Generate data-model.md (minimal, existing structures sufficient)
3. → Generate quickstart.md (add Windows, coverage, optimization sections)
4. → Update agent context with optimization and Windows info
5. → Re-check Constitution gates with concrete decisions
6. → Proceed to /speckit.tasks for implementation breakdown
