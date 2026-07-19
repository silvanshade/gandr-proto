# Workflow: project tooling and diagnostics

> Read when: changing the project-local gate driver, a shared-core script call, a tooling regression surface, or compiler/linter diagnostic handling.
> Base discipline: `.agents/core/core/WORKFLOW.md` §"Scripting".

## Keep the ownership boundary explicit

Project-local gates and landing orchestration live in the Rust crate `crates/wyrd-rust-gates`.
Its stable human, hook, and CI entrypoints are the task names in `mise.toml`; task bodies route into `cargo run --quiet -p wyrd-rust-gates -- <subcommand>`.
Use a stable `mise run <task>` at callsites so command spelling, arguments, and prerequisites have one owner; invoke the CLI directly when implementing or diagnosing that boundary.

The vendored agentic-dev core is a separate ownership domain.
Calls under `.agents/core/scripts/*.nu` remain live Nushell — for example `docs:conflict-markers`, `core:check`, and the Worktrunk ADR guard — and are not candidates for project-local rewrites.
Format-specific helpers that remain outside the gate crate, such as the reference-manual builders under `docs/manual/tools/`, are likewise reached through their named tasks; they do not own project gate policy.
Do not revive retired project gate scripts or copy shared-core Nushell into the Rust crate.

## One typed CLI, domain-owned modules

`src/main.rs` owns only the typed command grammar, dispatch, stable exit classes, and process boundary.
Library modules own the work: contracts and CI policy, documentation gates, graph and source policy, project dependency policy, coverage, maintenance ranges, contained mutation, workflow planning, and shared process/filesystem support.
This keeps every operation callable through one binary without concentrating domain logic in a stringly dispatcher.

Prefer the narrow stable task that proves the change:

* `mise run docs:manifest-drift` and `mise run docs:reference-integrity` for corpus documentation;
* `mise run test:options-policy`, `mise run test:soundness-oracles`, `mise run test:graph-gates`, and `mise run test:dep-graph` for policy surfaces;
* `mise run coverage:check` and `mise run coverage:ratchet` for per-file coverage policy;
* the `mise run mutants:*` family for mutation modes, and `cargo run --quiet -p wyrd-rust-gates -- workflow {merge|push}` for fixed landing tiers.

Semantic violations are stable Rust `Finding` values; malformed input, I/O, subprocess, and containment failures are typed errors.
Both paths fail closed at the CLI boundary.

## Tests, benchmarks, and fuzzing

The crate's regression surface is Rust:

* `cargo nextest run -p wyrd-rust-gates` runs its composed integration suite; the stable `mise run test:*` tasks add live fixtures where the policy requires them.
* `cargo bench -p wyrd-rust-gates --bench commands` measures representative command surfaces without turning performance numbers into gate semantics.
* The feature-gated parser facade feeds the independent AFL++ `gates` target.
  `mise run fuzz:gates` runs that campaign, while `mise run fuzz:rust-smoke` deterministically replays every committed seed across all five Rust targets.

Keep pure parsing/planning separate from side effects so tests, the benchmark, and the fuzz target exercise the same decisions as the CLI.
Every fix needs a regression witness for its observable contract; do not recreate standalone Nushell test runners beside the Rust suite.

## Git and subprocesses are sanitized by construction

Hook runners can export `GIT_DIR`, `GIT_INDEX_FILE`, `GIT_WORK_TREE`, and object/ref overrides that redirect a child Git process away from its requested working directory.
Project Rust domains therefore pass exact argv vectors through the shared support boundary and request Git sanitization; repository-control variables are removed before spawn.
Shared-core Nushell continues to use `.agents/core/scripts/lib/git.nu` for the same reason.

Do not add shell-string interpolation or an ad hoc `Command` wrapper.
Use the shared status path when output is purely live, and the shared output path when a domain must parse stdout: stdout is streamed while a bounded semantic copy is retained, stderr remains live, and over-limit output is drained before returning a typed failure.
Extend that seam and its child-process fixtures when a new process contract is required.

## Diagnostics go through aifix

aifix is the diagnostic adapter, not the pass/fail gate.
For a project-wide Rust sweep, use the MCP batch surface with profile `auto`; discover profiles before selecting a narrower one, and use `aifix_pipeline` for diagnostics already captured in a structured stream.
It normalizes, deduplicates, and groups findings; replay applies only explicitly recorded project-local fixes.

Reserve `mise run cargo:clippy` (`-D warnings`) for the binary gate.
The `agda:check` task owns its `aifix batch agda` invocations and then calls the Rust OPTIONS-policy sweep.
Best-effort posture stands ([docs.md](docs.md)): aifix improves triage but never licenses degrading an artifact to silence a finding.
