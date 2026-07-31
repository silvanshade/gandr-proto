# Workflow

How work moves through this project, for humans and agents.
This is the **routing layer**: the base operating doctrine lives in the shared core (`.agents/core/core/{PRINCIPLES,WORKFLOW,HAZARDS,PUBLISHABLE-HISTORY}.md`), and the gandr-specific depth lives in task-scoped sub-workflow files under `docs/workflow/`.

> **Do not read every sub-workflow up front.** Each one names the tasks it serves; load the one your task matches, when it matches.
> This file plus `AGENTS.md` is enough orientation for most work.
>
> **One exception, because task-scoping is what defeated it.** [workflow/review.md](workflow/review.md) §"Declining is a claim too" and §"Refutations bind only with owner sign-off" are **standing rules for every task**, not review-task guidance.
> They fire whenever you are about to record that something does not apply, is not needed, cannot be done, or is the wrong structure — which happens most often in the middle of ordinary implementation work, where nothing would otherwise route you there.
> **A refutation binds only with the owner's sign-off**, and the recurring cost in this project has been refutations that were never read as claims.

## Source of truth

* **Code**: local `main` only — no git remote is configured during the reboot bootstrap; once the gandr remote lands, `main` receives **signed** pushes only.
* **Work**: beads (prefix `gandr-`) in a local Dolt database syncing out-of-band from git to DoltHub — push after every write, pull before reads ([workflow/tracker.md](workflow/tracker.md)).
* **Design**: `docs/gandr/` is authoritative over every other document (`docs/KNOWLEDGE.md` §Authority); during the reboot bootstrap decisions live in the approved `PLAN.html` and the `gandr-fcw` wayfinder tracker — the per-file `docs/adr/` log is deferred until a decision log is deliberately re-introduced (owner direction, `gandr-fcw`).
* **Contributor notes** (session plans, handoffs, research digests, adversary reports): the sibling `wyrd-notes` repository — a separate local git repo beside this one, never part of this tree.

## The sub-workflow files

| Read                                                             | When your task involves                                                                                                                   |
| ---------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| [workflow/tracker.md](workflow/tracker.md)                       | creating/closing/triaging beads, dependency edges, tracker audits                                                                         |
| [workflow/beads-graph-sweep.xml](workflow/beads-graph-sweep.xml) | graph-wide bead classification, normalization, redundancy, citation sweeps                                                                |
| [workflow/worktrees.md](workflow/worktrees.md)                   | worktree lifecycle, merging, `wt` hooks, mutating sub-agents, governance docs                                                             |
| [workflow/ci.md](workflow/ci.md)                                 | choosing gates, the local merge wall, coverage, fuzzing, the parked CI surface                                                            |
| [workflow/rust.md](workflow/rust.md)                             | writing or reviewing Rust; `# Contract` / `# Adequacy` conventions                                                                        |
| [workflow/mutation-adequacy.md](workflow/mutation-adequacy.md)   | adequacy hypotheses, survivor triage, mutation campaigns                                                                                  |
| [workflow/soundness.md](workflow/soundness.md)                   | checker/machine/subtype/effect/grade changes, coherence oracles                                                                           |
| [workflow/corpus.md](workflow/corpus.md)                         | gandr-corpus examples, the feature landing rule, `gandr-pro`                                                                              |
| [workflow/agda.md](workflow/agda.md)                             | anything under `metatheory/`                                                                                                              |
| [workflow/scripting.md](workflow/scripting.md)                   | project scripts (Nushell/TS), reading diagnostics (aifix)                                                                                 |
| [workflow/review.md](workflow/review.md)                         | adversarial review, interpreting adversary findings, research outlooks — **and, standing for every task, declining or refuting anything** |
| [workflow/specs.md](workflow/specs.md)                           | authoring/editing the `docs/gandr/spec` corpus, re-absorptions, doc fidelity                                                              |
| [workflow/docs.md](workflow/docs.md)                             | adding/restructuring docs, formatter posture, math-dense Markdown                                                                         |

## Quality gates, in one breath

**Discover the task surface before invoking any project tooling — `mise tasks --local`.** A `mise` task is the stable entry point, and it usually carries environment the bare binary does not.
`treefmt`, `treefmt:check` and `cargo:fmt` all export the pinned nightly, because `rustfmt.toml` relies on nightly-only options that stable `rustfmt` **silently ignores** rather than rejecting.
So a bare `treefmt` reformats the whole tree against the wrong style, reports success, and leaves a diff no gate asked for.
The hazard shape is general: it applies to every tool whose task body sets a toolchain, a variable, or a config path, so reach for the task rather than the binary even when the bare invocation looks equivalent.
Narrow, file-scoped checks (`rumdl check <file>`, `typos <file>`) stay useful while iterating — but the tree-wide verdict is the task's.

Run the **narrowest gate that proves your change** before any commit; the merge wall (`gate:merge`) runs the composed sweep automatically ([workflow/ci.md](workflow/ci.md); the push tier and hosted CI are parked during the reboot).
Docs: `treefmt:check`, `docs:conflict-markers`, `docs:manifest-drift`, `docs:reference-integrity`, `test:doc-gates`.
Rust: `cargo:clippy` (pass/fail only — triage via aifix), `cargo:nextest`.
Agda: `agda:check`.
Adding a doc under `docs/gandr/` registers it in `docs/gandr/MANIFEST.yml` with a b3sum and at least one edge, and editing a registered corpus doc updates that b3sum — both in the same commit, and `docs:manifest-drift` is what catches either omission.

## Standing principles (the short forms)

* **Modularity-first**: before modifying, evaluate structure — extract on touch, act or schedule, always surface (`.agents/core/core/PRINCIPLES.md` §"Working posture"; precedents in [workflow/rust.md](workflow/rust.md)).
* **Formatters and linters are best-effort**: never satisfy a tool at the cost of artifact fidelity ([workflow/docs.md](workflow/docs.md)).
* **External research artifacts are reference-only**: read and cite published work; never vendor, port, or depend on companion artifacts, regardless of license.
  Agda dependencies additionally need maintainer sign-off first ([workflow/agda.md](workflow/agda.md)).
* **Graduation principle (dogfood the stack)**: when a major component ships, evaluate it as a replacement for the ad-hoc tooling that preceded it and file beads for the graduations it enables — the project's own layers are the intended substrate for the tooling around the project, so interim tooling keeps its formats substrate-agnostic.
* **Adversarial review before substantial landings**; findings are challenged-not-refuted unless factual ([workflow/review.md](workflow/review.md)).
* **Documentation economy — prefer forgetting over hoarding**: accumulation is a named project killer; distill decisions into ADRs, keep surveys and session context in the notes repo ([workflow/docs.md](workflow/docs.md)).
  Economy governs **which documents exist**, never the fidelity of load-bearing content: spreading out, explaining, and linking is the sanctioned response to density — dropping is not (`gandr-fid.0`).
* **Documentation authoring gets a mandatory fidelity review**: every `docs/gandr/spec/` / `docs/research/` change is adversarially diffed against its declared source set for dropped content, not just checked for correctness ([workflow/review.md](workflow/review.md) §"Documentation fidelity review").

## Where things are decided

| Question                           | Answer lives in                                                            |
| ---------------------------------- | -------------------------------------------------------------------------- |
| What is the design?                | `docs/gandr/` (authoritative)                                              |
| Why was it decided?                | `gandr-fcw` wayfinder tracker + `PLAN.html` (reboot; `docs/adr/` deferred) |
| What's next, in what order?        | `docs/gandr/VISION.md` §6                                                  |
| How are docs kept trustworthy?     | `docs/KNOWLEDGE.md`                                                        |
| What work is open right now?       | `bv --robot-triage`                                                        |
| What can go wrong (failure modes)? | `docs/HAZARDS.md` + `.agents/core/core/HAZARDS.md`                         |
| How does work move through here?   | this document + `docs/workflow/`                                           |
