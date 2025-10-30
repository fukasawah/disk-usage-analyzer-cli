<!--
Sync Impact Report

- Version change: 0.0.0 → 1.0.0
- Modified principles: N/A (template → concrete)
- Added sections: Quality Gates; Development Workflow
- Removed sections: Principle 5 placeholder removed to match requested 4 principles
- Templates requiring updates:
	- ✅ .specify/templates/plan-template.md (Constitution Check gates aligned)
	- ✅ .specify/templates/tasks-template.md (tests now REQUIRED; added perf/quality cues)
	- ✅ .specify/templates/spec-template.md (added mandatory Non-Functional Requirements section aligned with constitution)
- Follow-up TODOs:
	- TODO(RATIFICATION_DATE): Provide original adoption date for historical record
	- Optional: Define hardware reference profile for performance SLO validation (CPU, RAM, disk)
-->

# rs-disk-usage Constitution

## Core Principles

### I. Code Quality Discipline
The codebase MUST be easy to reason about, idiomatic, and safe.

- Rust edition: 2021 or later; code MUST pass `cargo fmt --check` with default style.
- Linting: `cargo clippy` MUST be clean with `-D warnings` for the default workspace.
- Safety: Avoid `unwrap()`/`expect()` in production paths; if used, a comment MUST justify why it
	cannot fail. Prefer `Result`/`Option` handling and explicit error types.
- Modularity: Public APIs MUST be small, documented, and follow single-responsibility. Any new
	module MUST include a top-level doc comment explaining its contract and invariants.
- Observability: Errors MUST include actionable context; logs SHOULD be structured when logging is
	present; CLI stderr is reserved for diagnostics.

Rationale: High code quality lowers maintenance cost, enables safe refactors, and improves
reliability for a utility likely to run on large directory trees.

### II. Testing Standards and Coverage
Testing is non‑negotiable and MUST gate merges.

- Unit tests: All public functions and critical branches MUST be covered by unit tests.
- Integration/CLI tests: Primary flows (e.g., scanning a directory, JSON output) MUST have
	integration tests using sample fixtures. Snapshot tests MAY be used for stable human output.
- Coverage: Target ≥ 90% lines and ≥ 80% branches on changed code. If coverage tooling is not
	available, equivalent evidence (e.g., annotated test map) MUST be provided. Waivers require
	explicit, time‑boxed justification in the PR.
- Determinism: Tests MUST be fast (< 2 minutes total on CI), deterministic, and independent.

Rationale: A disk-usage tool touches filesystem edge cases; robust tests prevent regressions and
allow safe iteration.

### III. Consistent User Experience (CLI and Output)
User experience MUST be predictable and consistent across releases.

- CLI contracts: Provide `--help`, stable flags, and semantic exit codes (0 success; non‑zero
	categorized failures). Breaking changes to flags require a MAJOR version bump or a deprecated
	grace period with warnings.
- Output modes: Human‑readable by default; a `--json` mode MUST be available with a stable schema.
	Errors and warnings go to stderr; machine‑readable output goes to stdout only.
- UX ergonomics: Support `--version`, `--quiet`, and `--verbose`. Defaults MUST be safe and
	unsurprising; progress output, if any, MUST be suppressible.
- Documentation: README/`--help` MUST include examples for common tasks and JSON schema notes.

Rationale: Consistent UX enables scripting, automation, and user trust.

### IV. Performance and Efficiency Targets
The tool MUST be performant and resource‑efficient for typical workloads.

- Baseline SLOs (subject to tuning with evidence):
	- p95 end‑to‑end time to summarize a directory with ~100k files ≤ 5s on a reasonable Linux
		machine; peak RSS ≤ 200MB.
	- No runtime regression > 10% p95 or > 20% p99 across minor/patch releases without a waiver.
- Profiling: Changes that alter traversal/aggregation logic MUST include a before/after benchmark or
	trace on representative data.
- Scalability: The tool MUST avoid O(n^2) behavior on common operations; streaming/iterator patterns
	SHOULD be used for large trees.

Rationale: Users invoke disk-usage tools interactively and in CI; responsiveness matters.

## Quality Gates
The following gates MUST pass before merge and on release builds:

- Build: `cargo build --release` PASS for all targets in CI matrix.
- Formatting/Linting: `cargo fmt --check` PASS; `cargo clippy -D warnings` PASS.
- Tests: `cargo test` PASS; evidence of coverage meeting targets or an approved, time‑boxed waiver.
- UX checks: `--help` example and CLI contract verified; JSON schema compatibility test PASS.
- Performance: No benchmark regression beyond thresholds; new features include SLO notes or an
	explicit “N/A with justification”.

## Development Workflow
Lightweight, evidence‑driven workflow that enforces the gates above.

- Branching: Conventional branch names (e.g., `feat/*`, `fix/*`). Commits use Conventional Commits.
- Review: Every PR requires at least one reviewer; reviewers MUST verify Quality Gates.
- Documentation: Update `--help`, examples, and changelog when user‑visible behavior changes.
- Exceptions: Temporary waivers MUST document scope, reason, owner, and expiry date.

## Governance
This constitution defines non‑negotiable standards for this repository.

- Amendments: Proposed via PR including a rationale and impact analysis. Provide migration guidance
	if principles change. Approved amendments update version and date below.
- Versioning policy for this document:
	- MAJOR: Remove/redefine principles in a backward‑incompatible way.
	- MINOR: Add a new principle or materially expand guidance.
	- PATCH: Clarifications, wording, or non‑semantic refinements.
- Compliance: PR authors and reviewers are responsible for ensuring adherence; periodic audits MAY
	be scheduled.

**Version**: 1.0.0 | **Ratified**: TODO(RATIFICATION_DATE): Provide original adoption date | **Last Amended**: 2025-10-30

# [PROJECT_NAME] Constitution
<!-- Example: Spec Constitution, TaskFlow Constitution, etc. -->

## Core Principles

### [PRINCIPLE_1_NAME]
<!-- Example: I. Library-First -->
[PRINCIPLE_1_DESCRIPTION]
<!-- Example: Every feature starts as a standalone library; Libraries must be self-contained, independently testable, documented; Clear purpose required - no organizational-only libraries -->

### [PRINCIPLE_2_NAME]
<!-- Example: II. CLI Interface -->
[PRINCIPLE_2_DESCRIPTION]
<!-- Example: Every library exposes functionality via CLI; Text in/out protocol: stdin/args → stdout, errors → stderr; Support JSON + human-readable formats -->

### [PRINCIPLE_3_NAME]
<!-- Example: III. Test-First (NON-NEGOTIABLE) -->
[PRINCIPLE_3_DESCRIPTION]
<!-- Example: TDD mandatory: Tests written → User approved → Tests fail → Then implement; Red-Green-Refactor cycle strictly enforced -->

### [PRINCIPLE_4_NAME]
<!-- Example: IV. Integration Testing -->
[PRINCIPLE_4_DESCRIPTION]
<!-- Example: Focus areas requiring integration tests: New library contract tests, Contract changes, Inter-service communication, Shared schemas -->

### [PRINCIPLE_5_NAME]
<!-- Example: V. Observability, VI. Versioning & Breaking Changes, VII. Simplicity -->
[PRINCIPLE_5_DESCRIPTION]
<!-- Example: Text I/O ensures debuggability; Structured logging required; Or: MAJOR.MINOR.BUILD format; Or: Start simple, YAGNI principles -->

## [SECTION_2_NAME]
<!-- Example: Additional Constraints, Security Requirements, Performance Standards, etc. -->

[SECTION_2_CONTENT]
<!-- Example: Technology stack requirements, compliance standards, deployment policies, etc. -->

## [SECTION_3_NAME]
<!-- Example: Development Workflow, Review Process, Quality Gates, etc. -->

[SECTION_3_CONTENT]
<!-- Example: Code review requirements, testing gates, deployment approval process, etc. -->

## Governance
<!-- Example: Constitution supersedes all other practices; Amendments require documentation, approval, migration plan -->

[GOVERNANCE_RULES]
<!-- Example: All PRs/reviews must verify compliance; Complexity must be justified; Use [GUIDANCE_FILE] for runtime development guidance -->

**Version**: [CONSTITUTION_VERSION] | **Ratified**: [RATIFICATION_DATE] | **Last Amended**: [LAST_AMENDED_DATE]
<!-- Example: Version: 2.1.1 | Ratified: 2025-06-13 | Last Amended: 2025-07-16 -->
