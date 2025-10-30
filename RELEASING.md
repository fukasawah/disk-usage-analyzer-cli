# Release Process

## How to Release

1. **Update version in `Cargo.toml`**:
   ```toml
   [package]
   version = "0.2.0"  # Update this
   ```

2. **Update `RELEASE_NOTES.md`** with changes

3. **Commit changes**:
   ```bash
   git add Cargo.toml RELEASE_NOTES.md
   git commit -m "chore: bump version to 0.2.0"
   git push origin main
   ```

4. **Create and push tag**:
   ```bash
   git tag v0.2.0
   git push origin v0.2.0
   ```

5. **GitHub Actions will automatically**:
   - Run tests on all platforms
   - Build binaries for:
     - Linux x86_64 (glibc and musl)
     - Linux ARM64
     - Windows x86_64
     - macOS x86_64 and ARM64
   - Create GitHub Release
   - Upload all binaries and checksums

6. **Verify the release** at: `https://github.com/fukasawah/dua/releases`

## Testing Locally Before Release

```bash
# Test on current platform
cargo test --release

# Test musl build (Linux)
cargo build --release --target x86_64-unknown-linux-musl
cargo test --release --target x86_64-unknown-linux-musl

# Check binary size
ls -lh target/release/dua
ls -lh target/x86_64-unknown-linux-musl/release/dua
```

## Platform Support Policy

### ✅ Tier 1: Fully Supported
- **Linux x86_64** (glibc and musl)
- **Windows x86_64** (MSVC)

These platforms are:
- Tested in CI on every commit
- Guaranteed to work
- Maintainer actively uses

### ⚠️ Tier 2: Best Effort
- **Linux ARM64** (aarch64)
- **macOS** (Intel and Apple Silicon)

These platforms are:
- Built and tested in CI
- Should work, but not actively used by maintainer
- Community contributions welcome for issues
- No guarantee of timely fixes

## Troubleshooting

### Release workflow fails

1. Check [Actions tab](https://github.com/fukasawah/dua/actions)
2. Review failed job logs
3. Common issues:
   - Dependency update conflicts
   - Test failures on specific platform
   - GitHub API rate limits

### Binary size exceeds threshold

If CI warns about binary size >10MB:
1. Review `cargo bloat --release` output
2. Check for new dependencies
3. Verify release profile settings in `Cargo.toml`
