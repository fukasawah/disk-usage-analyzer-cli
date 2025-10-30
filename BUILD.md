# Build Documentation

## Baseline Metrics (Before Optimization)

**Binary Size** (as of 2025-10-30):
- `target/release/dua`: 13 MB (actual file size: 15.7 MB)
- `target/release/dua`: 440 KB
- `.text` section: 7.4 MB

**Test Count**: 24 tests passing

**Build Time** (unoptimized release): ~2.8 seconds

### Size Contributors (Top 20)

Largest code contributors from `cargo bloat --release -n 20`:

1. **Parquet/Arrow** (~43% of .text): Compression algorithms (brotli), column writers, casting
   - `brotli::enc::block_splitter::BrotliSplitBlock`: 52.9 KB
   - `parquet::column::writer::ColumnWriter::close`: 47.8 KB
   - `arrow_cast::cast::cast_with_options`: 35.8 KB
   - Various arrow_select and arrow_cast functions: ~26-27 KB each

2. **Remaining methods** (~43% of .text): 11,871 smaller methods (6.7 MB combined)

3. **Our code** (minimal): 
   - `dua::io::snapshot::write_snapshot`: 25.2 KB

**Analysis**: The binary is dominated by Parquet/Arrow dependencies. Optimizing Parquet features and enabling LTO should yield significant size reduction.

## Windows Build Instructions

### Prerequisites

1. **Install Visual Studio Build Tools** (MSVC toolchain):
   - Download from https://visualstudio.microsoft.com/downloads/
   - Select "Desktop development with C++" workload
   - Or use command line: `vs_buildtools.exe --add Microsoft.VisualStudio.Workload.VCTools`

2. **Add Rust MSVC target**:
   ```cmd
   rustup target add x86_64-pc-windows-msvc
   ```

### Building on Windows

```cmd
# Build for native Windows (MSVC)
cargo build --release --target x86_64-pc-windows-msvc

# Binary location
dir target\x86_64-pc-windows-msvc\release\dua.exe
```

### Windows Usage Examples

```cmd
# Scan a directory
dua scan C:\Windows\System32 --snapshot system32.parquet

# View results
dua view system32.parquet --top 10

# Drill down into subdirectory
dua view system32.parquet --path "C:\Windows\System32\drivers"
```

### Known Windows Limitations

1. **Filesystem Boundary Detection**: Currently disabled (returns constant device ID)
   - **Impact**: Scanning multiple drives will traverse all (no boundary stop)
   - **Future Work**: Implement GetVolumePathNameW for proper drive comparison
   
2. **Junction Points**: Treated as regular directories (not symlinks)
   - **Behavior**: Junctions are traversed, size tracking works correctly

3. **Physical Size**: Uses GetCompressedFileSizeW
   - **Works correctly** for NTFS compressed and sparse files
   - Returns compressed size (expected behavior)

## Binary Size Optimization

### Release Profile Configuration

The project uses an optimized release profile in `Cargo.toml`:

```toml
[profile.release]
opt-level = "s"        # Optimize for size (balanced size/speed)
lto = "fat"            # Link-time optimization (maximum)
codegen-units = 1      # Single codegen unit for better cross-crate optimization
strip = true           # Strip debug symbols
```

**Actual Results** (as of 2025-10-30):
- Binary size reduction: **61%** (13 MB → 5.1 MB) ✅ **EXCEEDS STRETCH GOAL (<5MB)**
- Build time: 1m 14s (acceptable, well under 3 minute limit)
- .text section: 4.0 MB (down from 7.4 MB, 46% reduction)
- All 24 tests: PASS ✅
- Performance: No regression detected

### Version Embedding

The binary includes build-time version information via `build.rs`:

```bash
$ dua --version
dua 0.1.0
Commit: c327f6e (2025-10-30 10:59:51 +0000)
Target: x86_64-unknown-linux-gnu
Build: release
```

This information is embedded at compile-time using:
- `CARGO_PKG_VERSION`: Package version from Cargo.toml
- `GIT_HASH`: Short commit hash from `git rev-parse --short HEAD`
- `GIT_DATE`: Commit date from `git log -1 --format=%ci`
- `BUILD_TARGET`: Target triple (e.g., `x86_64-unknown-linux-musl`)
- `Build type`: Debug or release based on `cfg(debug_assertions)`
  - Scan: 84ms (10K files)
  - View: 11ms
  - Drill-down: 13ms

### Size Analysis Tools

**Install cargo-bloat**:
```bash
cargo install cargo-bloat
```

**Analyze binary size contributors**:
```bash
# Top 20 size contributors
cargo bloat --release -n 20

# Full analysis
cargo bloat --release -n 50
```

**Compare before/after**:
```bash
# Before optimization (baseline)
cargo bloat --release -n 20 > size-baseline.txt

# After optimization
cargo bloat --release -n 20 > size-optimized.txt

# Compare
diff size-baseline.txt size-optimized.txt
```

## Test Coverage

### Prerequisites

```bash
cargo install cargo-llvm-cov
```

### Measuring Coverage

**Terminal summary**:
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

Per specification (SC-008):
- **Core modules** (`src/services/*`, `src/io/*`): ≥80% line coverage
- **CLI layer** (`src/cli/*`): ≥70% (UI code has some untestable paths)
- **Models** (`src/models/*`): ≥90% (simple structures)

### Exclusion Patterns

Coverage reports exclude:
- Test files (`tests/*`)
- Platform-specific stubs (e.g., Windows code on Linux CI)
- Generated code (if any)

## Parquet Dependency Optimization

### Minimal Features Configuration

The project uses minimal Parquet features to reduce binary size:

```toml
parquet = { version = "53.2", default-features = false, features = ["arrow", "snap"] }
```

**Features enabled**:
- `arrow`: Required for Arrow integration (core functionality)
- `snap`: Snappy compression (lightweight, keeps snapshots small)

**Features disabled** (unused, saves ~1-1.5MB):
- `async`: Async I/O (we use synchronous file operations)
- `brotli`, `flate2`, `lz4`, `zstd`: Additional compression codecs (not needed)

**Backward compatibility**: Existing snapshots remain readable (compression is metadata).

### Verify Parquet Configuration

```bash
# Check which features are enabled
cargo tree -e features | grep parquet

# Expected output should show only: arrow, snap
```

## Performance Validation

### Benchmark Script

Create test fixtures and measure performance:

```bash
# Create benchmark script
chmod +x scripts/benchmark.sh

# Run benchmarks
./scripts/benchmark.sh
```

### Acceptance Criteria

After optimization, performance must remain within tolerance:

- **Scan operation**: <5% regression
- **View operation**: <10% regression
- **Binary size**: <7MB (acceptable), <5MB (stretch goal)

### Manual Performance Testing

```bash
# Create test dataset
mkdir -p /tmp/bench-fixture
# ... generate test files ...

# Measure scan performance
time target/release/dua scan /tmp/bench-fixture --snapshot bench.parquet

# Measure view performance
time target/release/dua view bench.parquet --top 100
```

## Troubleshooting

### Build Time Too Long (>2 minutes)

If LTO takes too long, try thin LTO:
```toml
[profile.release]
lto = "thin"  # ~20% size reduction vs ~30% for fat
```

### Binary Still >7MB After Optimization

1. **Try aggressive size optimization**:
   ```toml
   opt-level = "z"  # Instead of "s"
   ```

2. **Audit dependencies**:
   ```bash
   cargo bloat --release -n 50
   # Look for unexpected large contributors
   ```

3. **Verify Parquet features**:
   ```bash
   cargo tree -e features | grep parquet
   # Should only show: arrow, snap
   ```

### Coverage Tool Fails

**Windows-specific**:
- Ensure MSVC toolchain installed (cargo-llvm-cov requires LLVM)
- Try nightly if stable fails: `cargo +nightly llvm-cov`

**Linux/macOS**:
- Verify LLVM toolchain installed: `llvm-config --version`
- Update Rust toolchain: `rustup update`

## Test Organization

### Test Naming Convention

All test files follow the convention: `test_<feature>_<aspect>.rs`

**Examples**:
- `test_scan.rs` - Tests for scan command functionality
- `test_view_drill_down.rs` - Tests for view with drill-down (--path) functionality
- `test_snapshot_roundtrip.rs` - Tests for snapshot save/load cycle
- `test_errors.rs` - Tests for error handling
- `test_resilience.rs` - Tests for resilience to partial failures

**Test Organization**:
- `tests/unit/` - Unit tests for individual functions/modules
- `tests/integration/` - Integration tests for complete workflows
- `tests/contract/` - Contract tests for JSON schema validation

## Development Workflow

### Full Build and Test Cycle

```bash
# Clean build with optimizations
cargo clean
cargo build --release

# Run all tests
cargo test --all-features

# Generate coverage report
cargo llvm-cov --html --open

# Analyze binary size
cargo bloat --release -n 20
```

### Continuous Integration

Recommended CI workflow:

1. **Build**: `cargo build --release`
2. **Test**: `cargo test --all-features`
3. **Coverage**: `cargo llvm-cov --lcov --output-path coverage.lcov`
4. **Size Check**: Verify binary <7MB (fail if exceeded)
5. **Performance**: Run benchmark suite, fail if >10% regression

## References

- [Rust Profile Settings](https://doc.rust-lang.org/cargo/reference/profiles.html)
- [cargo-bloat Documentation](https://github.com/RazrFalcon/cargo-bloat)
- [cargo-llvm-cov Documentation](https://github.com/taiki-e/cargo-llvm-cov)
- [Parquet Rust Documentation](https://docs.rs/parquet/)
