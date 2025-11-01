# Feature Specification: Traversal Performance Optimization

**Feature Branch**: `001-optimize-traversal`  
**Created**: 2025-11-01  
**Status**: Draft  
**Input**: User description: "処理スピードを改善してほしい。愚直にトラバースしているが、これをもっと効率化できないか？もしくはファイルシステムに特化した実装ができないか？特にWindows(NTFS)はフォルダを右クリック→プロパティで表示したときのサイズは9万ファイルで3秒程度で算出できるので、これぐらい早くなってほしい。"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Rapid local audit (Priority: P1)

Windows power users rely on the CLI to understand the footprint of large project folders and expect results within a few seconds, comparable to Explorer's folder properties dialog.

**Why this priority**: Delivering near-instant scans addresses the core performance pain point and unblocks time-critical clean-up workflows.

**Independent Test**: Execute the CLI against a 90k-file NTFS directory on baseline hardware and confirm the run completes under the defined SLO while reporting accurate totals.

**Acceptance Scenarios**:

1. **Given** a local NTFS directory containing 90k files on baseline SSD hardware, **When** the user runs the default scan command, **Then** the CLI returns aggregated size metrics and exits within 3 seconds.
2. **Given** the same directory scanned twice within 10 minutes, **When** the user reruns the command, **Then** the CLI produces totals within 1% of the initial run and completes within 2 seconds.

---

### User Story 2 - Filesystem-aware strategy (Priority: P2)

Administrators working across Windows and Linux want the tool to automatically select the fastest traversal strategy per filesystem without manual tuning, while still exposing controls when needed.

**Why this priority**: Intelligent defaults reduce cognitive load and ensures the speed gains benefit every supported platform.

**Independent Test**: Validate that running the CLI on NTFS, APFS, and ext4 volumes auto-selects the documented strategy and surfaces an override flag for auditing.

**Acceptance Scenarios**:

1. **Given** supported Windows, macOS, and Linux environments, **When** the user runs the default command on each filesystem, **Then** the CLI reports which strategy was applied, respects user overrides, and finishes within 3 seconds for 90k-file directories on local storage.

---

### User Story 3 - Predictable progress on slower media (Priority: P3)

Operators scanning network shares or spinning disks need trustworthy progress indicators and predictable completion, even when the SLO cannot be met.

**Why this priority**: Transparency builds confidence and keeps the tool usable for less-performant storage while optimizations are tuned.

**Independent Test**: Simulate scans on high-latency or throttled volumes and verify progress feedback frequency, cancellation handling, and final accuracy.

**Acceptance Scenarios**:

1. **Given** a network-mounted directory that cannot meet the 3-second SLO, **When** the user launches a scan, **Then** the CLI emits progress updates at least every 2 seconds, honors cancellation, and returns totals within 2% accuracy of a baseline scan.

---

### Edge Cases

- Directories exceeding 1 million entries must provide progress feedback and avoid running out of memory while computing aggregates.
- Paths containing unreadable or locked files must skip them gracefully, surface a summary of omissions, and continue aggregating accessible data.
- Junctions, symbolic links, and reparse points must not cause infinite traversal loops and should respect existing symlink handling policies.
- Network shares with intermittent connectivity must resume or abort cleanly without corrupting partial results or hanging the CLI.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The CLI MUST complete aggregated size calculations for local directories containing up to 100k entries on baseline SSD hardware within 3 seconds at the 95th percentile.
- **FR-002**: The CLI MUST detect the underlying filesystem type (e.g., NTFS, APFS, ext4) and automatically select the documented traversal strategy for that type.
- **FR-003**: The CLI MUST provide a user-visible flag to disable optimized traversal and revert to legacy behavior for troubleshooting and parity checks.
- **FR-004**: The CLI MUST ensure optimized and legacy traversal modes produce total size outputs within the greater of 1% or 10 MB difference for identical datasets.
- **FR-005**: The CLI MUST emit structured progress updates (stderr or JSON) at least every 2 seconds whenever a scan is projected to exceed the 3-second SLO.

### Non-Functional Requirements *(mandatory)*

- **NFR-Perf**: Establish benchmark datasets per platform and document baseline hardware. p95 completion for 90k-file local scans must be ≤ 3 seconds; 1M-file scans must finish within 30 seconds while staying below 75% CPU utilization per core. Release binaries must remain ≤ 6 MB by disabling unused crate features and applying post-build stripping.
- **NFR-UX/CLI**: Document new or changed flags, default behaviors, and progress messaging for both human-readable and `--json` outputs. Preserve existing exit codes and ensure overrides are discoverable via `--help`.
- **NFR-CodeQuality**: Maintain module boundaries between traversal strategies and shared aggregation logic, add structured telemetry hooks for performance measurement, and avoid panics in production paths.
- **NFR-Testing**: Add platform-specific integration tests covering optimized strategies, regression tests to compare legacy vs optimized totals, and ensure modified modules maintain ≥ 90% line coverage.

### Key Entities *(include if feature involves data)*

- **Scan Session**: Represents a single CLI invocation, including target path, filesystem type, selected strategy, start/end timestamps, and aggregated results.
- **Traversal Strategy**: Encapsulates the rules and optimizations applied per filesystem, including eligibility criteria, required capabilities, and expected performance envelope.

## Assumptions

- Baseline hardware refers to a quad-core desktop CPU released within the last 3 years and NVMe SSD storage; final benchmarks will publish precise specs.
- Windows Explorer folder size measurements are treated as the user-perceived performance baseline for NTFS volumes.
- Network latency and external I/O throttling are outside the 3-second SLO but must still deliver accurate results with progress feedback.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 95% of local scans against directories ≤ 100k entries on supported filesystems complete within 3 seconds on documented baseline hardware.
- **SC-002**: 90% of large scans (up to 1M entries) complete within 30 seconds while maintaining accuracy within 1% of baseline totals.
- **SC-003**: At least 80% of surveyed Windows users report perceived parity or improvement versus Explorer for folder size calculations.
- **SC-004**: Support requests tagged "slow scan" decrease by 30% within one release cycle after launch.

## Clarifications

### Session 2025-11-01

- Q: 最適化後のリリースバイナリサイズの上限はどこまで許容されますか？ → A: Option A — リリースバイナリを常に 6 MB 以下に抑える
