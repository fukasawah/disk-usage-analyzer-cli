# Feature Specification: Optimization & Quality Improvements# Feature Specification: [FEATURE NAME]



**Feature Branch**: `002-optimization-improvements`  **Feature Branch**: `[###-feature-name]`  

**Created**: 2025-10-30  **Created**: [DATE]  

**Status**: Draft  **Status**: Draft  

**Input**: User description: "改善フェーズを進めたい。windowsビルドを行いたい。ビルドサイズが大きい(13MB)ので何がひっ迫しているのか確認して削減してほしい。またparquetで使うフィーチャを絞ると削減できるかどうかなど調べてほしい。テストにdrillを含むテスト名があり機能するか疑わしいのでテストを整理してほしい。テストのカバレッジを取るなどしてほしい。"**Input**: User description: "$ARGUMENTS"



## User Scenarios & Testing *(mandatory)*## User Scenarios & Testing *(mandatory)*



### User Story 1 - Windows Platform Support (Priority: P1)<!--

  IMPORTANT: User stories should be PRIORITIZED as user journeys ordered by importance.

As a Windows user, I need to build and run the dux binary natively on Windows so that I can analyze disk usage on my Windows filesystem without using WSL or Linux VMs.  Each user story/journey must be INDEPENDENTLY TESTABLE - meaning if you implement just ONE of them,

  you should still have a viable MVP (Minimum Viable Product) that delivers value.

**Why this priority**: Expands platform support to Windows, which is a major desktop OS. This is critical for user adoption and makes the tool truly cross-platform as originally envisioned in the spec.  

  Assign priorities (P1, P2, P3, etc.) to each story, where P1 is the most critical.

**Independent Test**: Can be fully tested by building on Windows (using `cargo build --target x86_64-pc-windows-msvc`), running basic commands (`dux scan C:\Users --snapshot test.parquet`, `dux view test.parquet`), and verifying output correctness against known directory structures.  Think of each story as a standalone slice of functionality that can be:

  - Developed independently

**Acceptance Scenarios**:  - Tested independently

  - Deployed independently

1. **Given** a Windows 10/11 system with Rust toolchain installed, **When** user runs `cargo build --release`, **Then** binary compiles successfully without errors  - Demonstrated to users independently

2. **Given** a Windows system, **When** user runs `dux scan C:\Users --snapshot test.parquet`, **Then** filesystem is scanned correctly with physical size computation using GetCompressedFileSizeW-->

3. **Given** a saved snapshot on Windows, **When** user runs `dux view test.parquet`, **Then** results display correctly with Windows path separators

4. **Given** a Windows directory with compressed files, **When** scanning with `--basis physical`, **Then** compressed sizes are reported accurately### User Story 1 - [Brief Title] (Priority: P1)



---[Describe this user journey in plain language]



### User Story 2 - Binary Size Reduction (Priority: P2)**Why this priority**: [Explain the value and why it has this priority level]



As a developer or user, I need the dux binary to be smaller (target: <5MB) so that it's faster to download, deploy, and distribute, especially for CI/CD pipelines and containerized environments.**Independent Test**: [Describe how this can be tested independently - e.g., "Can be fully tested by [specific action] and delivers [specific value]"]



**Why this priority**: Current 13MB binary size is large for a CLI tool. Reducing it improves distribution speed, reduces storage costs, and makes the tool more lightweight. This is important for adoption but not critical for functionality.**Acceptance Scenarios**:



**Independent Test**: Can be fully tested by building with optimizations (`cargo build --release`), measuring binary size with `ls -lh target/release/dux`, analyzing size contributors with `cargo bloat`, and verifying all functionality still works after optimization changes.1. **Given** [initial state], **When** [action], **Then** [expected outcome]

2. **Given** [initial state], **When** [action], **Then** [expected outcome]

**Acceptance Scenarios**:

---

1. **Given** release build configuration, **When** analyzing binary with `cargo bloat --release`, **Then** top size contributors are identified and documented

2. **Given** Parquet feature flags refined, **When** building with minimal features, **Then** binary size reduces by at least 30% while maintaining required functionality### User Story 2 - [Brief Title] (Priority: P2)

3. **Given** LTO and optimization settings configured, **When** building release binary, **Then** binary size is under 7MB (stretch goal: under 5MB)

4. **Given** optimized binary, **When** running full test suite, **Then** all 24 tests pass without regression[Describe this user journey in plain language]



---**Why this priority**: [Explain the value and why it has this priority level]



### User Story 3 - Test Organization & Quality (Priority: P2)**Independent Test**: [Describe how this can be tested independently]



As a developer maintaining the codebase, I need tests to be well-organized, correctly named, and covering all critical paths so that I can confidently refactor code and catch regressions early.**Acceptance Scenarios**:



**Why this priority**: Test quality directly impacts code maintainability and confidence in changes. Cleaning up test organization and improving coverage prevents future bugs. This is important for long-term health but doesn't add user-facing features.1. **Given** [initial state], **When** [action], **Then** [expected outcome]



**Independent Test**: Can be fully tested by reviewing test file structure, running test suite with `cargo test`, verifying all test names match their functionality, and generating coverage reports with `cargo tarpaulin` or `cargo llvm-cov`.---



**Acceptance Scenarios**:### User Story 3 - [Brief Title] (Priority: P3)



1. **Given** current test suite, **When** reviewing test file names and test function names, **Then** all tests with "drill" in name either test drill functionality or are renamed appropriately[Describe this user journey in plain language]

2. **Given** test organization, **When** running `cargo test`, **Then** test output clearly groups unit/integration/contract tests

3. **Given** test suite, **When** generating coverage report, **Then** code coverage is measured and documented (target: ≥80% line coverage for core modules)**Why this priority**: [Explain the value and why it has this priority level]

4. **Given** coverage report, **When** reviewing untested code paths, **Then** critical paths (error handling, edge cases) are identified and new tests added

5. **Given** test improvements, **When** running full test suite, **Then** all tests pass and new tests catch at least one previously untested edge case**Independent Test**: [Describe how this can be tested independently]



---**Acceptance Scenarios**:



### User Story 4 - Parquet Dependency Optimization (Priority: P3)1. **Given** [initial state], **When** [action], **Then** [expected outcome]



As a developer, I need to understand which Parquet features are actually used and configure the dependency to use only required features so that compile times and binary size are minimized.---



**Why this priority**: Parquet is a large dependency that may include unused features. Optimizing it can reduce build times and binary size. This is a "nice to have" optimization that supports Story 2 but isn't critical on its own.[Add more user stories as needed, each with an assigned priority]



**Independent Test**: Can be fully tested by auditing Parquet API usage in `src/io/snapshot.rs`, creating a feature-minimal Cargo.toml configuration, building with reduced features, and verifying snapshot read/write still works correctly.### Edge Cases



**Acceptance Scenarios**:<!--

  ACTION REQUIRED: The content in this section represents placeholders.

1. **Given** current Parquet usage, **When** auditing `src/io/snapshot.rs`, **Then** all used Parquet APIs and features are documented  Fill them out with the right edge cases.

2. **Given** identified required features, **When** configuring `Cargo.toml` with `default-features = false` and explicit feature list, **Then** binary still builds successfully-->

3. **Given** optimized Parquet configuration, **When** running snapshot tests (`test_snapshot_roundtrip`, `test_snapshot_errors`), **Then** all tests pass without modification

4. **Given** minimal Parquet features, **When** comparing binary size, **Then** size reduction is documented (even if minimal)- What happens when [boundary condition]?

- How does system handle [error scenario]?

---

## Requirements *(mandatory)*

### Edge Cases

<!--

- What happens when building on Windows without `windows-sys` features configured? (Should fail with clear error)  ACTION REQUIRED: The content in this section represents placeholders.

- How does binary size reduction affect runtime performance? (Should not regress scan/view performance)  Fill them out with the right functional requirements.

- What if test coverage tool (tarpaulin/llvm-cov) isn't installed? (Document installation instructions)-->

- How do we handle tests that legitimately test drill functionality vs. tests with misleading names? (Rename misleading tests, keep legitimate ones)

- What if Parquet minimal features break snapshot compatibility? (Validate with existing snapshots)### Functional Requirements



## Requirements *(mandatory)*- **FR-001**: System MUST [specific capability, e.g., "allow users to create accounts"]

- **FR-002**: System MUST [specific capability, e.g., "validate email addresses"]  

### Functional Requirements- **FR-003**: Users MUST be able to [key interaction, e.g., "reset their password"]

- **FR-004**: System MUST [data requirement, e.g., "persist user preferences"]

#### Windows Build Support- **FR-005**: System MUST [behavior, e.g., "log all security events"]



- **FR-001**: System MUST build successfully on Windows using MSVC toolchain (x86_64-pc-windows-msvc target)*Example of marking unclear requirements:*

- **FR-002**: System MUST use platform-specific physical size computation on Windows (GetCompressedFileSizeW via windows-sys)

- **FR-003**: System MUST handle Windows path separators (\) correctly in all path operations and output- **FR-006**: System MUST authenticate users via [NEEDS CLARIFICATION: auth method not specified - email/password, SSO, OAuth?]

- **FR-004**: System MUST support Windows-specific filesystem features (NTFS compression, alternate data streams) when computing physical sizes- **FR-007**: System MUST retain user data for [NEEDS CLARIFICATION: retention period not specified]

- **FR-005**: All existing CLI commands (scan, view) MUST work correctly on Windows with Windows paths (e.g., `C:\Users\...`)

### Non-Functional Requirements *(mandatory)*

#### Binary Size Optimization

- **NFR-Perf**: Define performance SLOs for this feature (e.g., p95 runtime, memory ceiling). Include

- **FR-006**: Build configuration MUST enable LTO (Link Time Optimization) for release builds  any required benchmarks or profiling to validate changes that affect traversal/aggregation logic.

- **FR-007**: System MUST document binary size before and after optimization with `cargo bloat` analysis- **NFR-UX/CLI**: Specify CLI flags, exit codes, and `--json` schema changes (if any). Note any

- **FR-008**: System MUST use minimal Parquet feature flags (disable unused compression codecs, async runtime features)  deprecations and compatibility plans.

- **FR-009**: System MUST maintain all existing functionality after size optimizations (validated by existing test suite)- **NFR-CodeQuality**: Note any constraints impacting code structure (module boundaries, public API

- **FR-010**: Build configuration MUST strip debug symbols from release binaries  contracts, error handling without `unwrap` in production paths) and observability expectations.

- **NFR-Testing**: List required unit and integration/CLI tests, determinism requirements, and

#### Test Organization  planned coverage level for changed code (target ≥ 90% lines unless justified).



- **FR-011**: All test files and functions MUST have names that accurately describe what they test### Key Entities *(include if feature involves data)*

- **FR-012**: Tests for removed or non-existent functionality (e.g., standalone `drill` command if deprecated) MUST be removed or updated

- **FR-013**: Test file structure MUST follow convention: `tests/unit/`, `tests/integration/`, `tests/contract/`- **[Entity 1]**: [What it represents, key attributes without implementation]

- **FR-014**: Each test function MUST have a clear assertion that validates expected behavior- **[Entity 2]**: [What it represents, relationships to other entities]



#### Test Coverage## Success Criteria *(mandatory)*



- **FR-015**: System MUST provide test coverage measurement using cargo-tarpaulin or cargo-llvm-cov<!--

- **FR-016**: Coverage report MUST identify uncovered lines in core modules (traverse, aggregate, snapshot, output)  ACTION REQUIRED: Define measurable success criteria.

- **FR-017**: System MUST achieve minimum 80% line coverage for core business logic modules (src/services/*, src/io/*)  These must be technology-agnostic and measurable.

- **FR-018**: Coverage report MUST be generated in both terminal output and HTML format for review-->



#### Parquet Optimization### Measurable Outcomes



- **FR-019**: System MUST audit and document all Parquet APIs and features actually used in `src/io/snapshot.rs`- **SC-001**: [Measurable metric, e.g., "Users can complete account creation in under 2 minutes"]

- **FR-020**: Cargo.toml MUST specify explicit Parquet features with `default-features = false`- **SC-002**: [Measurable metric, e.g., "System handles 1000 concurrent users without degradation"]

- **FR-021**: System MUST validate snapshot read/write functionality after Parquet optimization (run snapshot tests)- **SC-003**: [User satisfaction metric, e.g., "90% of users successfully complete primary task on first attempt"]

- **FR-022**: Documentation MUST note any Parquet features that cannot be disabled and why- **SC-004**: [Business metric, e.g., "Reduce support tickets related to [X] by 50%"]


### Non-Functional Requirements *(mandatory)*

- **NFR-Perf**: Binary size reduction optimizations MUST NOT regress runtime performance:
  - Scan performance: No more than 5% slowdown for 10K file benchmark
  - View performance: No more than 10% slowdown for large snapshot display
  - Include benchmark comparisons before/after optimization

- **NFR-UX/CLI**: No CLI changes required for this feature. All existing commands maintain same interface:
  - Windows paths displayed with backslashes in output
  - Exit codes remain unchanged
  - JSON schema unchanged

- **NFR-CodeQuality**: 
  - Windows-specific code MUST be properly gated with `#[cfg(windows)]`
  - No `unwrap()` in production code paths (use proper error handling)
  - Document platform-specific behavior in module docstrings
  - Build configuration changes MUST be documented in README or BUILD.md

- **NFR-Testing**:
  - All 24 existing tests MUST continue to pass on Linux after changes
  - Windows platform tests MUST be added (minimum: scan, view, physical size computation)
  - CI/CD SHOULD include Windows builds if possible (document if not feasible)
  - Test coverage target: ≥80% for src/services/* and src/io/*
  - Coverage report MUST be generated in CI pipeline

- **NFR-Build**:
  - Release builds MUST use `lto = true` and `codegen-units = 1`
  - Strip debug symbols with `strip = true` in release profile
  - Document build time impact of LTO (acceptable if <2x increase)

### Key Entities

Not applicable - this feature focuses on build optimization and testing infrastructure, not data modeling.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Binary successfully builds on Windows 10/11 with MSVC toolchain without errors
- **SC-002**: Binary size reduces from 13MB to under 7MB for release builds (stretch: under 5MB)
- **SC-003**: All 24 existing tests pass on both Linux and Windows platforms
- **SC-004**: Test coverage for core modules (src/services/*, src/io/*) reaches ≥80% line coverage
- **SC-005**: Zero tests with misleading names (e.g., "drill" tests that don't test drill functionality)
- **SC-006**: Build time for release binary remains under 3 minutes on standard CI hardware
- **SC-007**: Scan performance does not regress by more than 5% (measured with 10K file benchmark)
- **SC-008**: Coverage report generation completes successfully and produces HTML output

## Assumptions *(optional)*

- Rust toolchain 1.77+ is available on Windows with MSVC target installed
- Developers have access to Windows machines for testing (or can use Windows CI runners)
- `cargo-bloat`, `cargo-tarpaulin` (Linux) or `cargo-llvm-cov` (cross-platform) can be installed
- Current Parquet usage is limited to basic read/write operations (schema definition, record batch I/O)
- LTO and optimization settings are acceptable trade-offs for binary size vs. build time
- Windows physical size computation via GetCompressedFileSizeW is already implemented (verify in code)

## Dependencies

- Existing: `windows-sys` crate (already in Cargo.toml, verify correct features enabled)
- New: `cargo-bloat` (development tool for size analysis)
- New: `cargo-tarpaulin` (Linux) or `cargo-llvm-cov` (cross-platform) for coverage
- Existing: All 24 tests must pass before and after changes
- Build: MSVC toolchain on Windows (not a code dependency, but required for Windows builds)

## Scope

### In Scope

1. **Windows Build Support**:
   - Cross-platform compilation for x86_64-pc-windows-msvc
   - Windows path handling and display
   - Windows-specific physical size computation verification
   - Basic Windows integration tests

2. **Binary Size Reduction**:
   - LTO and codegen-units optimization
   - Debug symbol stripping
   - Parquet feature optimization
   - cargo bloat analysis and documentation

3. **Test Quality**:
   - Audit and fix test names
   - Remove/update tests for deprecated functionality
   - Add missing edge case tests
   - Improve test organization

4. **Test Coverage**:
   - Set up coverage tooling (tarpaulin or llvm-cov)
   - Generate coverage reports
   - Document coverage metrics
   - Identify and prioritize untested code paths

### Out of Scope

- Changing CLI interface or adding new commands
- Performance optimization beyond preventing regressions
- Adding Windows-specific features (e.g., VSS snapshots, Windows Search integration)
- Comprehensive Windows hardlink tracking (already noted as future work in MVP)
- ARM64 Windows support (focus on x86_64 only)
- Automated Windows CI pipeline setup (document but don't implement if infra unavailable)
- Coverage enforcement in CI (generate reports but don't fail builds on low coverage yet)

## Clarifications Needed

None - all requirements are clear based on the user's description. Reasonable defaults have been applied:

- **Binary size target**: 7MB (acceptable), 5MB (stretch) - industry standard for CLI tools
- **Coverage target**: 80% line coverage - standard for production code
- **Performance regression tolerance**: 5% for scans, 10% for views - acceptable for size optimization
- **Windows test scope**: Basic integration tests (scan, view) - comprehensive enough to validate platform support

## Notes

- This feature represents the "polish" phase after MVP completion
- Focus on quality, maintainability, and platform support rather than new features
- Binary size analysis may reveal opportunities for future optimization
- Windows build testing may require access to Windows hardware or CI runners
- Test coverage measurement will guide future test improvements
- Parquet optimization findings should be documented even if size reduction is minimal
