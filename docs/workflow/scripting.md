# Workflow: project tooling and diagnostics

> Read when: changing the project-local gate driver, a shared-core script call, a tooling regression surface, or compiler/linter diagnostic handling.
>
> **Legacy scripts:** the existing `scripts/` tree is obsolete and migration-bound.
> Do not add new scripts.
> Continue to follow this document when invoking, maintaining, or diagnosing existing scripts until they are replaced.
> **Standing rule, whatever the task:** before recording that something does not apply, is not needed, or cannot be done, read [review.md](review.md) §"Declining is a claim too" and §"Refutations bind only with owner sign-off" — a refutation binds only with the owner's sign-off.

## Keep the ownership boundary explicit

Project gate policy and landing orchestration live in the Rust crate `crates/workflow-gates` (package `gandr-workflow-gates`).
Its stable human, hook, and (future) CI entrypoints are the task names in `mise.toml`; task bodies route into `cargo run --quiet -p gandr-workflow-gates -- <subcommand>`.
Use a stable `mise run <task>` at callsites so command spelling, arguments, and prerequisites have one owner; invoke the CLI directly only when implementing or diagnosing that boundary.

**New scripting starts as a `mise` task body, and graduates to a `workflow-*` Rust crate if it outgrows one.** There is no third home.
A task body is the cheapest thing that already has a name, a callsite discipline, and a place in the gate wall; when one accumulates real logic, branching, or its own tests, it graduates into `gandr-workflow-gates` (or a sibling `workflow-*` crate) rather than growing in place.

Nushell is **retired for new work**.
The remaining `.nu` helpers below are legacy: keep them working, do not extend them, and prefer migrating one to a task body or the Rust crate whenever it is touched substantively.
The retirement is for measured reasons — Nushell startup cost showed up in gate latency, and its harness behaviour proved unreliable enough to distrust in hooks.
The long-run destination is that this scripting layer is written in gandr itself; until then the two homes above are the whole policy.

The legacy helpers, each reached through its named `mise` task or `prek` hook, never inlined:

- `scripts/check-conflict-markers.nu` — the `docs:conflict-markers` gate and its pre-commit hook;
- `scripts/check-machine-local-paths.nu` — the `no-machine-local-paths` pre-commit hook (publishable-history backstop);
- `scripts/commitlint-range.nu` and `scripts/check-signed-commits.nu` — the pre-push range checks;
- `scripts/lib/git.nu`, `scripts/lib/push-range.nu` — shared helpers.

The retired shared-core delegation (`core:check`, `core-init`, the Worktrunk ADR guard) is gone with the core itself; the checks that once delegated there run as the legacy helpers above.
Do not revive retired project gate scripts, and do not fold a typed helper into the Rust crate ad hoc — graduate one into `gandr-workflow-gates` only when it becomes gate policy.

## One typed CLI, domain-owned modules

`src/main.rs` owns only the typed command grammar, dispatch, stable exit classes, and process boundary.
Library modules own the work: contracts and CI policy, documentation gates, graph and source policy, project dependency policy, coverage, maintenance ranges, contained mutation, workflow planning, and shared process/filesystem support.
This keeps every operation callable through one binary without concentrating domain logic in a stringly dispatcher.

Prefer the narrow stable task that proves the change:

- `mise run test:soundness-oracles` and `mise run test:graph-gates` for policy surfaces;
- `mise run coverage:check` and `mise run coverage:ratchet` for per-file coverage policy;
- the fixed landing tiers — `mise run gate:merge` for the merge wall, and the `cargo run --quiet -p gandr-workflow-gates -- workflow push` plan (parked during the reboot, [ci.md](ci.md)) — plus the `mise run mutants:*` family for mutation modes.

Semantic violations are stable Rust `Finding` values; malformed input, I/O, subprocess, and containment failures are typed errors.
Both paths fail closed at the CLI boundary.

## Tests, benchmarks, and fuzzing

The crate's regression surface is Rust:

- `cargo nextest run -p gandr-workflow-gates` runs its composed integration suite; the stable `mise run test:*` tasks add live fixtures where the policy requires them.
- `cargo bench -p gandr-workflow-gates --bench commands` measures representative command surfaces without turning performance numbers into gate semantics.
- The feature-gated parser facade feeds the independent AFL++ `gates` target.
  `mise run fuzz:gates` runs that campaign, while `mise run fuzz:rust-smoke` deterministically replays every committed seed across all five Rust targets.

Keep pure parsing/planning separate from side effects so tests, the benchmark, and the fuzz target exercise the same decisions as the CLI.
Every fix needs a regression witness for its observable contract; do not recreate standalone Nushell test runners beside the Rust suite.

## Git and subprocesses are sanitized by construction

Hook runners can export `GIT_DIR`, `GIT_INDEX_FILE`, `GIT_WORK_TREE`, and object/ref overrides that redirect a child Git process away from its requested working directory.
Project Rust domains therefore pass exact argv vectors through the shared support boundary and request Git sanitization; repository-control variables are removed before spawn.
The project-local Nushell helpers use `scripts/lib/git.nu` for the same reason.

Do not add shell-string interpolation or an ad hoc `Command` wrapper.
Use the shared status path when output is purely live, and the shared output path when a domain must parse stdout: stdout is streamed while a bounded semantic copy is retained, stderr remains live, and over-limit output is drained before returning a typed failure.
Extend that seam and its child-process fixtures when a new process contract is required.

## Diagnostics go through aifix

aifix is the diagnostic adapter, not the pass/fail gate.
For a project-wide Rust sweep, use the MCP batch surface with profile `auto`; discover profiles before selecting a narrower one, and use `aifix_pipeline` for diagnostics already captured in a structured stream.
It normalizes, deduplicates, and groups findings; replay applies only explicitly recorded project-local fixes.

Reserve `mise run cargo:clippy` (`-D warnings`) for the binary gate.
Best-effort posture stands ([docs.md](docs.md)): aifix improves triage but never licenses degrading an artifact to silence a finding.
