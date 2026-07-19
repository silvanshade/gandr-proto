# Workflow: adversarial review and research verdicts

> Read when: preparing a substantial change for landing, running or interpreting an adversarial pass, or closing a research-question task.
> Base practice: `.agents/core/core/WORKFLOW.md` §"Adversarial review of substantial changes".

## When and how

The gates catch **structural** faults (formatting, hash drift, dangling references), not **semantic** ones — a change can pass every gate while quietly distorting a claim or missing an instance nobody thought to grep for.
So any substantial or publishable-stakes change (an absorption, a multi-document edit, a curation pass, anything touching the corpus) gets an **independent adversarial review before landing/pushing and before bead closeout**: This is wyrd's timing override of the shared core's “before committing” shorthand: an immutable checkpoint may exist solely as isolated-review input, but it is not landed or pushed.

* **Independent** — a reviewer separate from the author, given the changed files (and the source they derive from), not the author's rationale.
* **Adversarial** — prompted to find faults: "what is wrong, missing, or distorted here?"
* **Multi-lens** — distinct lenses for distinct failure modes: fidelity to source, policy/leakage compliance, cross-reference integrity, fresh-reader coherence, context-economy ([docs.md](docs.md)).
* **Demonstrability** — apply the full canonical checklist in [corpus.md](corpus.md): surfaced means same-change runnable model and pathological examples, harness assertions, and coverage-map registration; internal-only means named runnable crate fixtures exercised by named crate tests/harness assertions, intended future programs, and the exact corpus-promotion blocker.
  The manual must not present conformance-only support as user syntax.

Scale to the change: a one-line fix needs only the gates; a cross-cutting corpus pass earns several lenses.
Triage findings like drift findings: fix should-fix in the same change, file the rest as beads — never silent; surviving findings are residuals, folded into the consolidated closeout bead ([tracker.md](tracker.md) §“Feature landing and residual closeout”).

**Isolation.** Mutating review agents run only through the Worktrunk-owned lane ([worktrees.md](worktrees.md) §"Mutating sub-agents") — a Bash-capable agent sharing the live tree has corrupted uncommitted work before (core H7).
An immutable checkpoint commit is the normal input to an isolated reviewer; a strictly read-only reviewer may inspect uncommitted state.
For governance docs already on `main`, reviewers stay read-only and the orchestrator applies fixes on `main` before push.

## Interpreting findings — challenged, not refuted

Adversarial findings are **inputs, not verdicts**, and two kinds bind differently:

* **Binding** — a _fabrication_, a _factual/citation error_, or a genuine _category error_: fixed before the reviewed checkpoint lands/pushes and before bead closeout, not negotiable.
* **Challenged, not refuted** — a _strategic, feasibility, or redundancy_ judgement: recorded as **challenged** — de-emphasised in the search space, never dismissed as refuted/dead.
  As the design converges, genuinely-unhelpful negatives self-filter; dismissing early risks discarding a branch a later reframing would revive.

The adversary is never the final say.
Even a literature impossibility proof gets its applicability checked (does its hypothesis hold for _our_ object?) before binding.
When reporting, present challenges as things to engineer around, bracketed — never leading with the adversarial conclusion as settled.

**Every adversary pass writes a human-inspectable report artifact** — reasoning, citations, severity, each finding's binding-vs-challenged disposition — under `adversary/` in the sibling `wyrd-notes` repository; the deliverable cites it briefly, never inlines it.
The point is auditability: a later reader sees _why_ a branch was challenged.

## Research-question tasks — deliver the outlook

A task investigating a research question delivers, alongside findings, a brief **outlook**: **evidence for** vs **evidence against** as short bullets, plus a one-line **Net** (decisive, not hedged).
The adversary artifact checks claims against sources, not feasibility — the outlook is stated separately.
Distinguish "the framing is solid" from "the specific vehicle is high-risk"; separate what is established from what is open/conjectural; name the most-likely dead-ends explicitly.
