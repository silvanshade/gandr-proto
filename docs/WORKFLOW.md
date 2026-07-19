# Workflow

How work moves through this project, for humans and agents.
This is the **routing layer**: the base operating doctrine lives in the shared core (`.agents/core/core/{PRINCIPLES,WORKFLOW,HAZARDS,PUBLISHABLE-HISTORY}.md`), and the wyrd-specific depth lives in task-scoped sub-workflow files under `docs/workflow/`.

> **Do not read every sub-workflow up front.** Each one names the tasks it serves; load the one your task matches, when it matches.
> This file plus `AGENTS.md` is enough orientation for most work.

## Source of truth

* **Code**: `origin` (github.com/silvanshade/wyrd); `main` receives signed pushes only.
* **Work**: beads (prefix `wyrd-`) in a local Dolt database syncing out-of-band from git to DoltHub — push after every write, pull before reads ([workflow/tracker.md](workflow/tracker.md)).
* **Design**: `docs/gandr/` is authoritative over every other document (`docs/KNOWLEDGE.md` §Authority); decisions live one-per-file in `docs/adr/`.
* **Contributor notes** (session plans, handoffs, research digests, adversary reports): the sibling `wyrd-notes` repository — a separate local git repo beside this one, never part of this tree.

## The sub-workflow files

| Read                                                           | When your task involves                                                       |
| -------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| [workflow/tracker.md](workflow/tracker.md)                     | creating/closing/triaging beads, dependency edges, tracker audits             |
| [workflow/worktrees.md](workflow/worktrees.md)                 | worktree lifecycle, merging, `wt` hooks, mutating sub-agents, governance docs |
| [workflow/ci.md](workflow/ci.md)                               | choosing gates, CI jobs and tiers, coverage, fuzzing, campaign scheduling     |
| [workflow/rust.md](workflow/rust.md)                           | writing or reviewing Rust; `# Contract` / `# Adequacy` conventions            |
| [workflow/mutation-adequacy.md](workflow/mutation-adequacy.md) | adequacy hypotheses, survivor triage, mutation campaigns                      |
| [workflow/soundness.md](workflow/soundness.md)                 | checker/machine/subtype/effect/grade changes, coherence oracles               |
| [workflow/corpus.md](workflow/corpus.md)                       | gandr-corpus examples, the feature landing rule, `gandr-pro`                  |
| [workflow/agda.md](workflow/agda.md)                           | anything under `metatheory/`                                                  |
| [workflow/scripting.md](workflow/scripting.md)                 | project scripts (Nushell/TS), reading diagnostics (aifix)                     |
| [workflow/review.md](workflow/review.md)                       | adversarial review, interpreting adversary findings, research outlooks        |
| [workflow/docs.md](workflow/docs.md)                           | adding/restructuring docs, formatter posture, math-dense Markdown             |

## Quality gates, in one breath

Run the **narrowest gate that proves your change** before any commit; the merge and push tiers run the full sweep automatically ([workflow/ci.md](workflow/ci.md)).
Docs: `treefmt:check`, `docs:conflict-markers`, `docs:manifest-drift`, `docs:reference-integrity`, `test:doc-gates`.
Rust: `cargo:clippy` (pass/fail only — triage via aifix), `cargo:nextest`.
Agda: `agda:check`.
Editing a registered corpus doc updates its `docs/gandr/MANIFEST.yml` b3sum in the same commit.

## Standing principles (the short forms)

* **Modularity-first**: before modifying, evaluate structure — extract on touch, act or schedule, always surface (`.agents/core/core/PRINCIPLES.md` §"Working posture"; wyrd precedents in [workflow/rust.md](workflow/rust.md)).
* **Formatters and linters are best-effort**: never satisfy a tool at the cost of artifact fidelity ([workflow/docs.md](workflow/docs.md)).
* **External research artifacts are reference-only**: read and cite published work; never vendor, port, or depend on companion artifacts, regardless of license.
  Agda dependencies additionally need maintainer sign-off first ([workflow/agda.md](workflow/agda.md)).
* **Graduation principle (dogfood the stack)**: when a major component ships, evaluate it as a replacement for the ad-hoc tooling that preceded it and file beads for the graduations it enables — the project's own layers are the intended substrate for the tooling around the project, so interim tooling keeps its formats substrate-agnostic.
* **Adversarial review before substantial landings**; findings are challenged-not-refuted unless factual ([workflow/review.md](workflow/review.md)).
* **Documentation economy — prefer forgetting over hoarding**: accumulation is a named project killer; distill decisions into ADRs, keep surveys and session context in the notes repo ([workflow/docs.md](workflow/docs.md)).

## Where things are decided

| Question                           | Answer lives in                                    |
| ---------------------------------- | -------------------------------------------------- |
| What is the design?                | `docs/gandr/` (authoritative)                      |
| Why was it decided?                | `docs/adr/` (one record per decision)              |
| What's next, in what order?        | `docs/gandr/VISION.md` §6                          |
| How are docs kept trustworthy?     | `docs/KNOWLEDGE.md`                                |
| What work is open right now?       | `bv --robot-triage`                                |
| What can go wrong (failure modes)? | `docs/HAZARDS.md` + `.agents/core/core/HAZARDS.md` |
| How does work move through here?   | this document + `docs/workflow/`                   |
