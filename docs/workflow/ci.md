# Workflow: quality gates and the local merge wall

> Read when: choosing which gate proves a change, understanding the merge wall, or asking where hosted CI and the scheduled campaigns went.

## The wall is local during the reboot

Hosted CI is **parked** for the gandr reboot (owner direction, `gandr-fcw`): there is no `.github/workflows/` tree, `act`/`wrkflw`/`zizmor` are not wired, and no git remote is configured yet.
The whole gate wall is therefore **local** — the `wt` merge hooks, the `prek` commit hooks, and the `mise` task bodies they invoke — and it stays that way until go-public, when the hosted surface is rebuilt.
`gandr-kk7` tracks restoring the CI workflow surface (and un-ignoring the parked `ci_contracts` live test that reads `.github/workflows/ci.yml`).

Every check has one canonical `mise` task body; hooks invoke that same task rather than reimplementing its commands, so local and (future) hosted runs cannot drift.

## The narrowest gate that proves your change

Run the **narrowest gate that proves your change** before any commit; the merge wall runs the composed sweep automatically.

| Gate                                | Proves                                                                                                |
| ----------------------------------- | ----------------------------------------------------------------------------------------------------- |
| `mise run treefmt:check`            | formatting + lint across docs/config/Rust (the pre-commit hook runs this)                             |
| `mise run docs:conflict-markers`    | no unresolved Git conflict markers (`scripts/check-conflict-markers.nu`)                              |
| `mise run docs:manifest-drift`      | `docs/gandr/MANIFEST.yml` BLAKE3 hashes match registered docs                                         |
| `mise run docs:reference-integrity` | corpus cross-references (edges, ADR refs, section refs) resolve                                       |
| `mise run cargo:clippy`             | the strict nightly Clippy scope (pass/fail only — triage via aifix, [scripting.md](scripting.md))     |
| `mise run cargo:nextest`            | the current Rust test scope                                                                           |
| `mise run cargo:doc-check`          | workspace rustdoc builds with intra-doc links resolved (private items, `-D warnings`, pinned nightly) |
| `mise run cargo:dylint:recursion`   | merge-tier project-local recursion contracts across Dylint-covered workspace targets                  |
| `mise run cargo:dylint`             | strict project-local plus pinned upstream rules across Dylint-covered workspace targets               |
| `mise run agda:check`               | metatheory strict root + OPTIONS policy sweep ([agda.md](agda.md))                                    |

The doc-gate battery beyond the table (`test:doc-gates`, `test:soundness-oracles`, `test:options-policy`, coverage, no-panic, cargo-careful) exists as tasks but is not on the current merge wall; several return with the subject they check as the reboot ports it.

## The merge wall: `mise run gate:merge`

`.config/wt.toml` `[pre-merge]` is the merge wall — any non-zero exit aborts `wt merge`:

* **`gate:merge`** — the composed merge check, ordered: `cargo:build`, `cargo:clippy`, `cargo:dylint:recursion`, `cargo:doc-check`, `cargo:nextest`, `treefmt:check`.
  The recursion task loads only `gandr-workflow-dylint` over the existing Dylint-covered package scope; `gandr-workflow-gates` and the driver itself remain excluded under `gandr-0ze` and `gandr-3yh`.
  The full upstream Dylint inventory remains on-demand and in the parked push tier.
  `cargo:doc-check` runs `cargo doc --workspace --features=full --no-deps --document-private-items` on the pinned nightly with `RUSTDOCFLAGS="-D warnings"`, so a broken or redundant intra-doc link cannot land silently; it documents the whole workspace including the nightly-only `gandr-workflow-dylint` driver (like `cargo:clippy`) and so carries no `--exclude` set.
  It sits between `cargo:dylint:recursion` and `cargo:nextest` because it is a compile-class static check whose failures are cheap and localized, and grouping it with the other static analyzers surfaces doc breakage before the more expensive test run.
  This is the deterministic set a normal diff can realistically break.
  Parked entries return with their prerequisites: `toolchain:pin-check` (the `gandr-fcw.13` pin-drift gate, rebuild tracked in `gandr-wvd.20`) and `grammar:test` (returns with the tree-sitter grammar port).
* **`beads`** (`bd dolt pull && bd dolt push`) — makes the branch's beads durable on DoltHub **before** the merge removes the worktree's Dolt clone; pull-then-push self-heals the sibling-push race ([tracker.md](tracker.md); `core/HAZARDS.md` H2).
* Parked pre-merge hooks: `adr-guard` (returns when `docs/adr/` exists) and `core-pin` (`mise run core:check`, returns when the agentic-dev core is vendored at `.agents/core`).
  See [worktrees.md](worktrees.md).

Every commit additionally passes the `prek` **pre-commit** hooks — `treefmt:check`, `docs:conflict-markers`, `docs:manifest-drift`, `docs:reference-integrity`, `no-machine-local-paths`, `cargo:fmt-check` — and the **commit-msg** `commitlint` hook.
`prek install` arms these once per clone in the primary checkout (`core/HAZARDS.md` H4).
One commitlint gotcha recurs: any commit-body line **beginning** `word:` is parsed as a git trailer, so a sentence or wrapped line that opens with e.g. `D1:` trips `footer-leading-blank` against the `Co-Authored-By` footer — keep colon-suffixed tokens off line starts in commit bodies.

## Parked: the push tier and scheduled campaigns

These are **parked** during the reboot and return at go-public; they are recorded here so the shape is not lost, not because they run today.

* **Push tier** — `cargo run -p gandr-workflow-gates -- workflow push` (the fixed push plan in `crates/workflow-gates`) is parked as a `prek` pre-push hook because it invokes tasks not yet in place (`core:check`, `grammar:test`, `cargo:dylint`, `wrkflw`).
  The live pre-push hooks are `commitlint` over the push range (`scripts/commitlint-range.nu`) and the signed-commits check (`scripts/check-signed-commits.nu`).
  A push is deliberately an **arc-boundary event**: push after a full arc of work has merged, never per-commit — but no remote exists yet, so pushes wait on go-public.
* **Coverage** is judged **per production file**, never by an aggregate crate percentage (the 94%-crate/72%-file incident; `ADR-71`).
  `mise run coverage:check` compares each tracked production file against its recorded per-file floor and rejects any that fall below; the ratchet raises floors after a genuine improvement.
  Enforcement is off the wall while the port leaves rewritten crates below their floors and the floor table is unseeded; the coverage restoration pass re-arms it.
* **Scheduled campaigns** (mutation and fuzz sweeps) belong on a clock, not the landing path — and the clock is the hosted `scheduled-campaigns.yml`, which does not exist yet.
  Moving a campaign off a gate without landing it on a schedule is a coverage regression wearing a speedup's clothes, so the campaigns stay defined but dormant until the hosted schedule returns.
  + **Mutation testing** — `mise run mutants:*` only; every run is contained in an ephemeral microVM (`docs/HAZARDS.md` owns the safety story).
    Discipline: [mutation-adequacy.md](mutation-adequacy.md).
  + **Fuzzing** — `fuzz/` is an independent AFL++ workspace; deterministic seed replay via `mise run fuzz:rust-smoke`, unbounded per-target via `mise run fuzz:<target>`.
