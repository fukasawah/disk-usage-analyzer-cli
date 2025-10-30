# Quickstart: Optimization & Quality Improvements

**Feature**: 002-optimization-improvements  
**Audience**: Developers working on build optimization, platform support, and test quality

## Overview

This feature improves the MVP with:
- **Windows build support** (MSVC toolchain)
- **Binary size reduction** (13MB → <7MB target)
- **Test coverage measurement** (≥80% core modules)
- **Test organization** (clarify naming)
- **Parquet optimization** (minimal features)

## Prerequisites

- Rust 1.77+ (Edition 2024)
- Cargo (comes with Rust)
- Git
- **Windows-specific**: Visual Studio 2019+ with MSVC toolchain (for Windows development)

## Development Setup

### 1. Install Development Tools

**Coverage Tool** (Linux/macOS/Windows):
```bash
cargo install cargo-llvm-cov
```

**Binary Size Analysis**:
```bash
cargo install cargo-bloat
```

### 2. Clone and Build

```bash
git clone <repo-url>
cd rs-disk-usage
git checkout 002-optimization-improvements

# Initial build (unoptimized)
cargo build --release
ls -lh target/release/dux  # Linux/macOS: ~13MB baseline
```

**Windows**:
```cmd
cargo build --release
dir target\release\dux.exe  # Windows: ~13MB baseline
```

## Build Optimization

### Release Profile Configuration

The optimized release profile is configured in `Cargo.toml`:

```toml
[profile.release]
opt-level = "s"        # Optimize for size (try "z" for more aggressive)
lto = "fat"            # Link-time optimization (maximum)
codegen-units = 1      # Single codegen unit for better optimization
strip = true           # Strip debug symbols
```

**Build Time**: Expect 60-90 seconds (2-3x slower than default, acceptable per constitution).

### Size Analysis

**Before optimization**:
```bash
cargo bloat --release -n 20
```

**After optimization**:
```bash
# Clean rebuild with optimized profile
cargo clean
cargo build --release
cargo bloat --release -n 20

# Compare sizes
ls -lh target/release/dux
```

**Target**: <7MB (acceptable), <5MB (stretch goal).

## Test Coverage

### Measuring Coverage

**Full coverage report** (terminal):
```bash
cargo llvm-cov --all-features --workspace
```

**HTML report** (detailed, visual):
```bash
cargo llvm-cov --html --open
```

**CI-friendly format** (lcov):
```bash
cargo llvm-cov --lcov --output-path coverage.lcov
```

### Coverage Targets

Per spec (SC-008):
- **Core modules** (services/*, io/*): ≥80% line coverage
- **CLI layer**: ≥70% (UI code has some untestable paths)
- **Models**: ≥90% (simple structures)

**Check specific module**:
```bash
cargo llvm-cov --html --open
# Open browser, navigate to src/services/ or src/io/ to see detailed line coverage
```

## Windows Development

### Building on Windows

**Install MSVC** (Visual Studio 2019+ Build Tools):
- Download from https://visualstudio.microsoft.com/downloads/
- Select "Desktop development with C++" workload
- Or use `vs_buildtools.exe --add Microsoft.VisualStudio.Workload.VCTools`

**Add Rust MSVC target**:
```cmd
rustup target add x86_64-pc-windows-msvc
```

**Build**:
```cmd
cargo build --release --target x86_64-pc-windows-msvc
dir target\x86_64-pc-windows-msvc\release\dux.exe
```

### Windows-Specific Tests

**Minimum viable tests** (from research.md):

1. **Build test**: Verify compilation succeeds (above command)
2. **Scan test**: Small directory scan
   ```cmd
   .\target\release\dux.exe scan C:\Windows\System32\drivers --snapshot test.parquet
   ```
3. **View test**: Read snapshot
   ```cmd
   .\target\release\dux.exe view test.parquet --top 5
   ```

**Note**: Some unit tests may require Windows-specific fixtures. Use `#[cfg(windows)]` guards if needed.

### Known Limitations (Windows)

- **Filesystem boundary detection**: Currently disabled (returns constant).
  - Impact: Scanning multiple drives will traverse all (no boundary stop).
  - Future work: Implement GetVolumePathNameW for proper drive comparison.
- **Junction points**: Treated as regular directories (not symlinks).
  - Behavior: Junctions are traversed, but size tracking works correctly.

## Testing Workflow

### Run All Tests

```bash
cargo test --all-features
```

**Expected**: 24 tests passing (from MVP baseline).

### Run Specific Test Suites

**Integration tests**:
```bash
cargo test --test test_scan
cargo test --test test_view_drill_down  # (renamed from test_drill)
cargo test --test test_snapshot_roundtrip
```

**Unit tests**:
```bash
cargo test --lib
```

**Performance smoke test**:
```bash
cargo test --test test_perf_smoke -- --ignored
```

### Test Naming Convention

Per research.md Decision 4.3:
- **Format**: `test_<feature>_<aspect>.rs`
- **Examples**:
  - `test_scan.rs` - Scan command functionality
  - `test_view_drill_down.rs` - View with --path (drill-down)
  - `test_snapshot_roundtrip.rs` - Snapshot save/load cycle

**Renamed**: `test_drill.rs` → `test_view_drill_down.rs` (clarifies purpose).

## Parquet Optimization

### Minimal Features Configuration

**Cargo.toml**:
```toml
parquet = { version = "53.2", default-features = false, features = ["arrow", "snap"] }
arrow-array = "53.4"
arrow-schema = "53.4"
```

**Rationale** (from research.md Decision 5.1):
- `arrow`: Required for Arrow integration (core functionality)
- `snap`: Snappy compression (lightweight, keeps snapshots small)
- Disabled: async, brotli, flate2, lz4, zstd (unused, save ~1-1.5MB)

**Backward compatibility**: Existing snapshots remain readable (compression is metadata).

### Verify Parquet Changes

```bash
# Rebuild with minimal features
cargo clean
cargo build --release

# Test with existing snapshot
cargo test --test test_snapshot_roundtrip

# Check binary size
cargo bloat --release | grep parquet
```

## Performance Validation

### Before/After Benchmarks

**Baseline** (before optimization):
```bash
# Create test dataset (10K files)
mkdir -p /tmp/bench-fixture
# ... (generate 10K files)

# Benchmark
time cargo run --release -- scan /tmp/bench-fixture --snapshot bench.parquet
time cargo run --release -- view bench.parquet --top 100
```

**Optimized**:
```bash
cargo clean
cargo build --release  # With optimized profile

time target/release/dux scan /tmp/bench-fixture --snapshot bench-opt.parquet
time target/release/dux view bench-opt.parquet --top 100
```

**Acceptance** (from research.md Decision 2.3):
- Scan: <5% regression
- View: <10% regression
- Binary size: <7MB (primary goal)

## Troubleshooting

### Build Time Too Long (>2 minutes)

If LTO takes too long:
```toml
# Try thin LTO instead of fat
lto = "thin"  # ~20% size reduction vs ~30% for fat
```

### Binary Still >7MB

1. **Try aggressive size optimization**:
   ```toml
   opt-level = "z"  # Instead of "s"
   ```
2. **Audit dependencies**:
   ```bash
   cargo bloat --release -n 50
   # Look for unexpected large contributors
   ```
3. **Check Parquet features**:
   ```bash
   cargo tree -e features | grep parquet
   # Verify only arrow+snap enabled
   ```

### Coverage Tool Fails

**Windows-specific**:
- Ensure MSVC toolchain installed (cargo-llvm-cov requires LLVM)
- Use nightly if stable fails: `cargo +nightly llvm-cov`

**Linux/macOS**:
- Install LLVM: `apt install llvm` (Linux) or `brew install llvm` (macOS)

### Tests Fail on Windows

**Path issues**:
- Use `PathBuf` instead of hardcoded `/` or `\`
- Display: `path.display()` auto-converts to backslashes on Windows

**File permissions**:
- Windows test fixtures may require elevated permissions for symlinks
- Use regular files for most tests, skip symlink tests if unsupported

## Next Steps

After setup:
1. ✅ Verify baseline: `cargo build --release` → 13MB
2. ✅ Apply optimizations: Update `Cargo.toml` with release profile
3. ✅ Measure: `cargo bloat --release`, check binary size
4. ✅ Test: `cargo test --all-features` → 24 tests passing
5. ✅ Coverage: `cargo llvm-cov --workspace` → report ≥80% core modules
6. ✅ Windows: Build and run 3 minimum viable tests
7. ✅ Commit: Update Cargo.toml, tests, documentation

## Reference

- **Spec**: `specs/002-optimization-improvements/spec.md`
- **Research**: `specs/002-optimization-improvements/research.md`
- **Plan**: `specs/002-optimization-improvements/plan.md`
- **Constitution**: `.specify/constitution.md` (quality gates)
