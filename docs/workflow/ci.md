# Workflow: quality gates, CI, and scheduled campaigns

> Read when: choosing which gate proves a change, wiring or re-tiering CI jobs, or scheduling mutation/fuzz campaigns.

## The gates

Run the **narrowest gate that proves your change** before any commit; the full sweep runs at merge/push (below).

| Gate                                | Proves                                                                                            |
| ----------------------------------- | ------------------------------------------------------------------------------------------------- |
| `mise run treefmt:check`            | formatting + lint across docs/config (pre-commit)                                                 |
| `mise run docs:conflict-markers`    | no unresolved Git conflict markers                                                                |
| `mise run docs:manifest-drift`      | `docs/gandr/MANIFEST.yml` BLAKE3 hashes match registered docs                                     |
| `mise run docs:reference-integrity` | corpus cross-references (edges, ADR refs, section refs) resolve                                   |
| `mise run test:doc-gates`           | the Rust documentation-gate regression suite                                                      |
| `mise run test:options-policy`      | Agda OPTIONS-policy regression tests + live Rust sweep                                            |
| `mise run test:soundness-oracles`   | Rust soundness-oracle companion gate                                                              |
| `mise run cargo:clippy`             | the current strict Clippy scope (pass/fail only — triage via aifix, [scripting.md](scripting.md)) |
| `mise run cargo:dylint`             | pinned upstream and project-local semantic rules over the current remediated scope                |
| `mise run cargo:nextest`            | the current remediated Rust test scope                                                            |
| `mise run agda:check`               | metatheory strict root + OPTIONS policy sweep ([agda.md](agda.md))                                |

## CI of record, verified locally

`.github/workflows/ci.yml` is the CI gate of record.
Local landing verification uses the Rust driver's fixed task plans; `act` remains pinned in `mise.toml` only as a manual GitHub Actions parity-debugging aid, not as the merge or push gate.
While the repository is private, the comprehensive local push tier is the routine pre-publication verifier; once public, push-triggered GitHub CI becomes the routine remote verifier.

Coverage is judged **per production file**, never by an aggregate crate percentage (`docs/HAZARDS.md`, the 94%-crate/72%-file incident; ADR-71).
`mise run cargo:llvm-cov` writes `coverage/llvm-cov-summary.json`; `mise run coverage:check` requires the tracked-file set to match `coverage/floors.toml` exactly and rejects any file below its recorded floor.
The current target and ordinary new-file seed are **80%** (temporary override `wyrd-hp7g`); historical floors below the target remain in force.
A new or renamed production file may start lower only when its exact path has a non-empty reason under `[new_file_exemptions]` and its `[files]` floor equals its measured baseline capped by the target.
After a genuine improvement, `mise run coverage:ratchet` raises floors, seeds ordinary new keys, and preserves active exemption metadata; an exemption retires when its floor reaches the target.
History comparison permits a floor to fall only when the explicit policy target falls, and then only floors above the new target may clamp exactly to it; lower historical floors stay monotonic.
It normalizes an absent/all-zero push base to `HEAD^`, distinguishes a genuinely absent base policy from an unreadable base object, and otherwise fails closed.

## Gate tiers — what runs when (`wyrd-hkd6`, `wyrd-9ai5`)

Every check has one canonical `mise` task body; CI and local hooks invoke that same task rather than reimplementing its commands.
The Rust workflow driver owns the fixed ordering and fails at the first nonzero task:

* **Merge tier** (`.config/wt.toml` `[pre-merge]`, `mise run gate:merge`): ordered `grammar:test`, `cargo:build`, `cargo:clippy`, `cargo:dylint`, `cargo:nextest`, `treefmt:check`, and `wrkflw` — the deterministic checks a normal diff can realistically break.
* **Push tier** (`prek.toml` pre-push, `cargo run --quiet -p wyrd-rust-gates -- workflow push`): fixed order `core:check`, `grammar:test`, `cargo:build`, `cargo:clippy`, `cargo:dylint`, `cargo:nextest`, `treefmt:check`, `wrkflw`, `cargo:doc-check`, `docs:conflict-markers`, `docs:manifest-drift`, `docs:reference-integrity`, `test:soundness-oracles`, `test:doc-gates`, `test:page-balance`, `test:graph-gates`, `test:dep-graph`, `cargo:no-panic`, and `cargo:careful-nextest` (`coverage:check` is temporarily disabled while the failed-refactor remediation leaves rewritten crates below their recorded floors; the coverage restoration pass re-enables it).
  The old `(tree, job)` Act stamp files are retired.
  The native success cache is repository-scoped in the host temporary directory and admits a hit only for an exact key covering schema, tier/task, `HEAD` commit and tree, submodule state, workflow plan, active tool versions, tracked workflow/toolchain configuration, and — on push — origin fetch/push URLs, branch, upstream ref/commit, and merge base.
  Dirty or untracked work, missing identity, a corrupt/schema-mismatched cache, and cache I/O uncertainty all run the task normally.
  Only successful tasks whose identity is unchanged after execution are recorded; failures never write a proof.
  Merge and push plans are serialized across linked worktrees by one host-global lock derived from the canonical Git common directory.
  Act, fuzz, mutation, publication, push, ratchet, and release task families are never cacheable.
  The weight is **deliberate**, and it fixes when pushing happens: a remote push is an _arc-boundary event_ — push `main` after a full arc of work has merged (a track branch landing via `wt merge`, a completed governance or tooling change), never per-commit or mid-arc to save progress.

During the phased Dylint landing, these task names stay canonical and strict over every enabled workspace member; the P0-disabled front-end cluster is recorded in [rust.md](rust.md#dylint-adoption-and-residual-ledger).
P0 `wyrd-9c32` owns restoration of those workspace members and the remaining lint scopes.
Scope changes must update `Cargo.toml`, the canonical `mise` task, the Rust workflow contract tests, and that ledger together; removing a gate or downgrading `-D warnings` is not a valid phase boundary.

Direct `act` runs are manual parity investigations outside this cache and lock contract.
A new CI job does not automatically join the push tier: give its real work a stable `mise` task, then add that task deliberately to the Rust plan only when it is host-compatible and belongs on the landing path.

## Scheduled campaigns are on NEITHER tier

A campaign whose cost is unbounded-ish and whose signal is a sweep ("given more time, does anything fall over") does not belong on the landing path; it belongs on a clock — and the clock is concrete: `.github/workflows/scheduled-campaigns.yml`, with schedule and typed `repository_dispatch` triggers only (no branch-selectable dispatch, push, or pull-request path).
**Moving a campaign off a gate without landing it on a schedule is a coverage regression wearing a speedup's clothes** — the schedule must exist first.

* The **weekly** cron (`17 3 * * 1`, UTC) resolves its lower bound through the Rust `maintenance-range` command: an explicit dispatch `from` wins, otherwise the last fully successful runner-local watermark wins, otherwise the newest first-parent commit at least eight days old is selected.
  Every candidate must resolve and be an ancestor of `HEAD`.
  It then invokes `mise run maintenance:weekly --from <base> --to HEAD` — bounded 300-second AFL++ campaigns over all five Rust targets plus changed-code mutation over exactly that range.
  Only after the complete campaign succeeds does `mise run maintenance:advance` atomically move the watermark to `HEAD`, so a missed interval cannot disappear.
* The **monthly** cron (`29 4 1 * *`, UTC) invokes `mise run maintenance:monthly` — the full contained mutation sweep; the `monthly-maintenance` dispatch event invokes the same named campaign from the default branch.

Both jobs require the self-hosted macOS/ARM64 runner carrying the `wyrd-maintenance` label (microsandbox needs Apple-Silicon nested virtualization); an unavailable runner stays visibly queued, and the Rust mutants driver fails closed when its snapshot or cache image is missing.
Timeouts are 90 minutes weekly and nine hours monthly, reserving report-copy and teardown grace beyond cargo-mutants' own ceilings; an observed mutation report must copy out or the campaign fails, and publication restores the prior report on final-rename failure.
An always-run cleanup step calls `mise run mutants:clean` before the jobs upload `mutants.out/`, recoverable `mutants.out.{next,previous}/`, and `fuzz/artifacts/` with 90-day retention.

* **Mutation testing** — `mise run mutants:*` only; every run is contained in an ephemeral microVM (`docs/HAZARDS.md` owns the safety story — the danger is the unmutated baseline's real host effects, not the mutants).
  Discipline: [mutation-adequacy.md](mutation-adequacy.md).
* **Fuzzing** — `fuzz/` is an independent AFL++ workspace with five targets (`lower`, `parse`, `check`, `parity`, `gates`), seeded from their committed corpora; deterministic seed replay via `mise run fuzz:rust-smoke`, unbounded per-target via `mise run fuzz:<target>`, and the bounded sequential sweep via `mise run fuzz:weekly`.
  The tree-sitter grammar's own cheap fuzz (`aube run fuzz:smoke`) **stays** on the gate inside `aube-build-grammar`.
