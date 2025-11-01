# Research: Traversal Performance Optimization

## Decision 1: Windows traversal primitive
- **Decision**: Use Win32 `FindFirstFileExW` with `FIND_FIRST_EX_LARGE_FETCH` plus `GetFileInformationByHandleEx` for metadata, accessed via the `windows` crate.
- **Rationale**: `FindFirstFileExW` supports large directory prefetching and avoids the allocation overhead of `std::fs::read_dir`. Pairing it with `GetFileInformationByHandleEx` yields fast access to file size and attributes without reopening handles, mirroring the behavior of Explorer.
- **Alternatives considered**:
  - Using `std::fs::read_dir`: simpler but relies on CRT shims and shows 2–3× slower enumeration on NTFS under benchmark.
  - Reading the NTFS USN journal: provides incremental updates but requires admin privileges and does not align with one-shot CLI scenarios.
  - `IFileSystemImage` COM APIs: optimized for optical media authoring rather than raw directory scans.

## Decision 2: POSIX traversal primitive
- **Decision**: Use `rustix::fs::openat` with `Dir::read` to batch `getdents64` calls, coupled with `fstatat` for metadata, and reuse file descriptors for subdirectories.
- **Rationale**: `rustix` exposes efficient `openat`/`getdents` wrappers with fewer allocations than `std::fs`, and keeps path resolution inside the kernel, improving cache locality on ext4/APFS. Reusing the parent directory file descriptor cuts repeated `open` calls on deep trees.
- **Alternatives considered**:
  - `walkdir` crate: easy but allocates new `PathBuf` per entry and cannot exploit `openat`.
  - GNU `fts`: not idiomatic in Rust and lacks Windows support.
  - Custom `libc` bindings: higher maintenance burden versus `rustix`'s safe abstraction.

## Decision 3: Parallelization strategy
- **Decision**: Adopt a work-stealing task queue built on `rayon`, seeding directories breadth-first and processing file batches per thread with per-thread accumulators.
- **Rationale**: `rayon` integrates with existing codebase, offers adaptive worker counts, and simplifies CPU pinning. Per-thread accumulation minimizes contention; final results reduce via `fold`.
- **Alternatives considered**:
  - Manual thread pool: more control but higher complexity for dynamic load balancing.
  - Async runtime: directory traversal is syscall-bound; async introduces overhead without IO multiplexing benefits.
  - Single-threaded optimizations only: insufficient to hit the 3-second SLO on spinning disks.

## Decision 4: Progress signaling
- **Decision**: Emit progress snapshots through the existing `ProgressReporter` abstraction (stderr stream) every 1M bytes processed or every 2 seconds, whichever comes first, and extend `--json` mode with periodic progress objects gated by an opt-in flag.
- **Rationale**: Reusing the existing progress hooks avoids breaking UX while meeting the requirement for predictable updates on slow media. Byte-based and time-based triggers cover both large-file and many-file trees.
- **Alternatives considered**:
  - Adding a dedicated progress thread: risks contention with the traversal queue and complicates shutdown.
  - Quiet-by-default progress: fails the transparency requirement for slow scans.
  - Printing per-directory updates: too chatty for large trees and hard to parse.

## Decision 5: Legacy parity guardrail
- **Decision**: Keep the legacy traversal behind a `--legacy-traversal` flag and wire automated regression tests that diff totals between legacy and optimized runs within 1% or 10 MB.
- **Rationale**: Provides instant fallback for troubleshooting and regression detection, aligning with spec FR-004. Regression tests protect against platform-specific discrepancies introduced by new APIs.
- **Alternatives considered**:
  - Removing legacy code entirely: risky until new path matures across all filesystems.
  - Silent feature flag: lacks operator control and hinders A/B comparisons.
  - Config file toggle: heavier UX with no added benefit over CLI flag.
