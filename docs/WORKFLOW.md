# Workflow

How work moves through this project, for humans and agents.
This is the **routing layer**: `AGENTS.md` carries what binds on every entry path, and the depth lives in task-scoped sub-workflow files under `docs/workflow/`.
Every rule is stated in this repository — a pointer to a tree that is not checked out here is a rule nobody reads.

> **Do not read every sub-workflow up front.** Each one names the tasks it serves; load the one your task matches, when it matches.
> This file plus `AGENTS.md` is enough orientation for most work.
>
> **One exception, because task-scoping is what defeated it.** [workflow/review.md](workflow/review.md) §"Declining is a claim too" and §"Refutations bind only with owner sign-off" are **standing rules for every task**, not review-task guidance.
> They fire whenever you are about to record that something does not apply, is not needed, cannot be done, or is the wrong structure — which happens most often in the middle of ordinary implementation work, where nothing would otherwise route you there.
> **A refutation binds only with the owner's sign-off**, and the recurring cost in this project has been refutations that were never read as claims.

## Source of truth

- **Code**: `main`, with the `origin` remote receiving **signed** pushes only, at arc boundaries rather than per commit.
- **Work**: beads (prefix `gandr-`) in a local Dolt database syncing out-of-band from git to DoltHub — push after every write, pull before reads ([workflow/tracker.md](workflow/tracker.md)).
- **Design**: the specification corpus (cited as `spec:`, held outside this repository) is authoritative over every other document in this tree; programme ordering lives in `gandr-e08j`, and `gandr-fcw` retains the reboot rationale while the per-file `docs/adr/` log remains deferred until a decision log is deliberately re-introduced.
  Deep design context also arrives **dispatched** from the maintainer's private research workspace (`AGENTS.md` §"Dispatched work"); what this repository relies on is restated here, and beads may cite that context as `ss-` identifiers.
- **Contributor notes** (session plans, handoffs, research digests, adversary reports): each contributor's own private workspace, outside this tree (`AGENTS.md` §"Commits and publishable history").

## The sub-workflow files

| Read                                                             | When your task involves                                                                                                                   |
| ---------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| [workflow/tracker.md](workflow/tracker.md)                       | creating/closing/triaging beads, dependency edges, tracker audits, owner decisions and the escalation bright line                         |
| [workflow/beads-graph-sweep.xml](workflow/beads-graph-sweep.xml) | graph-wide bead classification, normalization, redundancy, citation sweeps                                                                |
| [workflow/worktrees.md](workflow/worktrees.md)                   | worktree lifecycle, merging, `wt` hooks, mutating sub-agents, governance docs                                                             |
| [workflow/ci.md](workflow/ci.md)                                 | choosing gates, the local merge wall, coverage, fuzzing, the parked CI surface                                                            |
| [workflow/rust.md](workflow/rust.md)                             | writing or reviewing Rust; `# Contract` / `# Adequacy` conventions                                                                        |
| [workflow/debugging.md](workflow/debugging.md)                   | debugging workspace binaries with LLDB                                                                                                    |
| [workflow/mutation-adequacy.md](workflow/mutation-adequacy.md)   | adequacy hypotheses, survivor triage, mutation campaigns                                                                                  |
| [workflow/soundness.md](workflow/soundness.md)                   | checker/machine/subtype/effect/grade changes, coherence oracles                                                                           |
| [workflow/corpus.md](workflow/corpus.md)                         | surface-corpus examples, the feature landing rule                                                                                         |
| `spec:proof-engineering.md` (held outside this repository)       | designing Agda structures: representation, characterization, reasoning style, namespacing — the doctrine, not the workflow                |
| [workflow/scripting.md](workflow/scripting.md)                   | project scripts (Nushell/TS), reading diagnostics (aifix)                                                                                 |
| [workflow/review.md](workflow/review.md)                         | adversarial review, interpreting adversary findings, research outlooks — **and, standing for every task, declining or refuting anything** |
| [workflow/specs.md](workflow/specs.md)                           | identifier, citation, and prose-form rules surviving the corpus migration (the `spec:` corpus itself has left)                            |
| [workflow/docs.md](workflow/docs.md)                             | adding/restructuring docs, formatter posture, math-dense Markdown                                                                         |

## Quality gates, in one breath

**Discover the task surface before invoking any project tooling — `mise tasks --local`.** A `mise` task is the stable entry point, and it usually carries environment the bare binary does not.
`treefmt`, `treefmt:check` and `cargo:fmt` all export the pinned nightly, because `rustfmt.toml` relies on nightly-only options that stable `rustfmt` **silently ignores** rather than rejecting.
So a bare `treefmt` reformats the whole tree against the wrong style, reports success, and leaves a diff no gate asked for.
The hazard shape is general: it applies to every tool whose task body sets a toolchain, a variable, or a config path, so reach for the task rather than the binary even when the bare invocation looks equivalent.
Narrow, file-scoped checks (`rumdl check <file>`, `typos <file>`) stay useful while iterating — but the tree-wide verdict is the task's.

Run the **narrowest gate that proves your change** before any commit; the merge wall (`gate:merge`) runs the composed sweep automatically ([workflow/ci.md](workflow/ci.md); the push tier and hosted CI are parked during the reboot).
Docs: `treefmt:check`, `docs:conflict-markers`, `test:doc-gates`.
Rust: `cargo:clippy` (pass/fail only — triage via aifix), `cargo:nextest`.
The design corpus and its BLAKE3 registry have left this repository, so registration is no longer part of authoring here and the drift and reference-integrity gates retired with them.

## Standing principles (the short forms)

- **Modularity-first**: before modifying, evaluate structure — extract on touch, act or schedule, always surface (`AGENTS.md` §"Working posture"; precedents in [workflow/rust.md](workflow/rust.md)).
- **Formatters and linters are best-effort**: never satisfy a tool at the cost of artifact fidelity ([workflow/docs.md](workflow/docs.md)).
- **External research artifacts are reference-only**: read and cite published work; never vendor, port, or depend on companion artifacts, regardless of license.
- **Graduation principle (dogfood the stack)**: when a major component ships, evaluate it as a replacement for the ad-hoc tooling that preceded it and file beads for the graduations it enables — the project's own layers are the intended substrate for the tooling around the project, so interim tooling keeps its formats substrate-agnostic.
- **Adversarial review before substantial landings**; findings are challenged-not-refuted unless factual ([workflow/review.md](workflow/review.md)).
- **Documentation economy — prefer forgetting over hoarding**: accumulation is a named project killer; keep surveys and session context out of the tree ([workflow/docs.md](workflow/docs.md)).
  Economy governs **which documents exist**, never the fidelity of load-bearing content: spreading out, explaining, and linking is the sanctioned response to density — dropping is not (`gandr-fid.0`).
- **Documentation authoring gets a mandatory fidelity review**: every change that absorbs, migrates, or rewrites load-bearing content is adversarially diffed against its declared source set for dropped content, not just checked for correctness ([workflow/review.md](workflow/review.md) §"Documentation fidelity review").

## Where things are decided

| Question                           | Answer lives in                                                                                                     |
| ---------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| What is the design?                | the specification corpus, cited as `spec:` and held outside this repository (`AGENTS.md`)                           |
| Why was it decided?                | the specification corpus (`spec:`) and `gandr-fcw` wayfinder tracker (`docs/adr/` deferred)                         |
| Who decides, and where?            | the agent by default; escalations go outboard ([workflow/tracker.md](workflow/tracker.md))                          |
| What's next, in what order?        | the tracker's programme epic `gandr-e08j` (the buildout wayfinder; see `AGENTS.md` §"Start here")                   |
| How are docs kept trustworthy?     | the format wall ([workflow/ci.md](workflow/ci.md)) + the fidelity review ([workflow/review.md](workflow/review.md)) |
| What work is open right now?       | `bv --robot-triage`                                                                                                 |
| What can go wrong (failure modes)? | the hazards recorded inline in the workflow file that owns each surface                                             |
| How does work move through here?   | this document + `docs/workflow/`                                                                                    |
