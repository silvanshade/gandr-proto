# Agent Guidance

## Start here

* **The buildout wayfinder is the tracker's programme epic `gandr-e08j`** (repo: unify the buildout path around the absorption programme): the current wave, the lane adjacencies, and the triage state live there.
* `PLAN.html` — the approved reboot roadmap, retained as a historical record; it is no longer the current wayfinder.
* `docs/gandr/spec/README.md` — the language specification corpus, **which is migrating out of this repository** (its banner carries the rule: do not add specification documents).
* `docs/WORKFLOW.md` — the workflow routing layer.
  Read it first, then open only the task-scoped file it points to under `docs/workflow/`.

This file is a thin orientation adapter.
Substantive process guidance belongs in the routed workflow documents; reference it instead of restating it here.
There are two deliberate exceptions, both restated here because they bind on every entry path into this repository and on every artifact class in it: the reference discipline, and the rule that a description is never evidence.

## Unambiguous reference — identifiers and citations

**Both rules below bind in every context, not only in the specification corpus**: tracked documents, code comments and rustdoc, commit messages, tracker items, review artifacts, plans, notes, and chat.

The reason is one reason, and it is about what happens to text _after_ it is written.
Documents come to cite other documents; fragments get quoted, excerpted, summarized, and carried into contexts their author never saw.
A reference that resolves only inside its home document arrives somewhere else meaning nothing — or, worse, meaning something else.
**A colliding reference is worse than no reference at all, because it reads as precise.** So the test is never "is this clear here"; it is "does this still resolve, and resolve uniquely, when it arrives somewhere else with no context attached".

**No bare letter-number identifiers.** Anything referred to anywhere that matters — a decision, commitment, obligation, finding, spike, open question, stage, rung, phase — carries an identifier whose prefix abbreviates what it _is_ or the topic it belongs to, followed by a zero-padded number: `meta-spike-04`, `meta-question-19`.
Never `M1`, `S1`, `P1`, `H2`, `D11`, `F3.19`.
Numbering is **stable**: retiring an item leaves its number unused rather than renumbering the rest, because renumbering silently invalidates every reference already taken against it.
Give each identifier an anchor and cite it **by link, not by code** — `[[metatheory/roadmap#meta-question-19]]`, never "open question 19".

This is not hypothetical, and this project has already paid for it: `S1` named a metatheory spike and an undefined "trusted S1 core" at the same time, and the collision stayed invisible until someone cited one and meant the other.
Retired schemes are the sole exception and stay exactly as they are — the concordance in the guards ledger exists to decode old notes, so the codes in its left-hand column are data, not usage.

**No ambiguous citations, ever.** A reference to external work is one of exactly two things: a key from a durable reference register — `[@key]` resolving in `docs/gandr/spec/bibliography.yml` (Hayagriva), or an equivalent BibTeX register — or the full title with its authors and its most accurate stable identifier (DOI, ISBN, arXiv id, HAL id, and the like).
"The tagless-final paper", "the leading implementation", "a published mechanization", and a bare author-year with no register entry are each unusable by the next reader and unverifiable by the next reviewer.
A claim resting on an unverified locator says so at the claim.

The full statement of the identifier rule, with the anchoring and linking conventions, is [`docs/workflow/specs.md`](docs/workflow/specs.md) §"Identifiers are informative, prefixed, and linkable — never a bare letter and a number"; the corpus's citation convention is [`docs/gandr/spec/README.md`](docs/gandr/spec/README.md).

## Description is a hypothesis, not evidence

**Any text describing an artifact is a guide to what to expect, and never proof of what is.** This binds on every describing surface in this tree without exception: Rust doc comments and rustdoc, Agda comments and module headers, per-crate status files, corpus example prose, the specification corpus, tracker items, and commit messages.
Read it to know where to look and what shape to expect.
**Then verify against the thing itself** — the definition, the rule, the test, the tree.

**Why this earns its own section next to the reference rule above.** The two failures are opposites in how they announce themselves.
A dangling reference is self-reporting: follow it, find nothing, and you know.
**A description that is fluent and wrong reports nothing at all.** It reads as settled, it is quoted onward as settled, and detecting it costs exactly as much as opening the implementation — which is why nobody does, and why the error survives every reader until one happens to look.

Three obligations follow, and the second and third are what stop the drift accumulating:

* **Verify before relying.** Every as-built claim you write, and every claim you rest a decision on, is checked against the definition rather than the prose beside it.
* **Report every conflict to the owner.** When the description and the artifact disagree, say so — in the response, not only in a tracker item.
  Silence here is how a contradiction becomes load-bearing.
* **Correct what is obviously wrong, and report every correction.** Fixing it silently is only half the job: an unreported correction leaves the owner unable to see the rate, and the rate is the thing that needs to come down.

Route what you find by failure mode, because the two want different fixes: a reference that resolves to nothing goes to `gandr-mf8`; a statement that is wrong, stale, or ambiguous where the reference does resolve goes to `gandr-q5c`.

**This project has already paid for it.** `core-checker`'s `grade` module says its `Dup` and `Drop` rules are "Stage 2", meaning _which rules of the calculus they belong to_ — and a 2026-08-02 absorption read that as _not yet built_, wrote it into a landed specification document, and it survived to an independent review before anyone opened `checker.rs`, where both rules are implemented and `Dup` demonstrably enforces its grade sum.
The prose was accurate about its own subject and wrong about the question being asked of it.
That is the normal case, not an unlucky one.

## Working posture

* **Surgical changes; structural evaluation first.** Make the task-scoped change without unrelated rewrites, but first ask whether the change should extract shared functionality, prune duplication, or draw a module boundary.
  Act when that remains in scope; otherwise file a tracker item.
  Surface the opportunity either way.
* **Leave touched areas better.** Report undocumented engineering improvements and hazards; noticing and staying silent is the failure mode.
  Descriptions that disagree with what they describe are the highest-frequency instance — see §"Description is a hypothesis, not evidence" for the routing and the reporting obligation.
* **State uncertainty.** Say when current code or documentation does not prove a claim, and report unexpected harness, tool, or configuration failures immediately.
* **A refutation is the most expensive claim available here, and it binds only with the owner's sign-off.** Before recording that something does not apply, is not needed, cannot be done, or is the wrong structure, read [`docs/workflow/review.md`](docs/workflow/review.md) §"Declining is a claim too" and §"Refutations bind only with owner sign-off" — **whatever the task**, not only when reviewing.
  A wrong acceptance is loud and self-limiting; a wrong rejection is silent, compounding, and has been this project's recurring failure.
  The one-line test: _is the reason a fact about the machinery, or a fact about us?_ A reason that names one of our own properties is a representation question, not a refutation.
* **Verify behavior.** A green build is not proof.
  Exercise the changed behavior and add or update tests when behavior changes.
* **Gates prove structure, not meaning.** Substantial or publishable-stakes changes require an independent adversarial review before landing.
  Follow [`docs/workflow/review.md`](docs/workflow/review.md).

## Dispatched work

Work here often arrives **dispatched from the maintainer's private research workspace**: a brief naming the design context, the target bead, and the landing path.

* **This repository's contract binds on everything done here, whatever the launch point.** A brief supplements the routed workflow documents; it never replaces them.
* **Read the brief's design context first**, before writing anything, and treat it as the reference discipline treats any source: verify as-built claims against this tree, never against the brief's prose alone.
* **The only reference to that private context this repository admits is an `ss-` bead identifier, cited in a bead** ([`docs/workflow/tracker.md`](docs/workflow/tracker.md) §"The `ss-` identifier is the one permitted reference to the maintainer's private research context").
  Tracked content — code, tests, documentation, commit messages — carries no such reference, and every landing must stand on its own: a contributor without access to that workspace must be able to understand it from this repository alone.
* Dispatched work lands through this repository's own worktree lifecycle and merge wall, exactly as native work does.

## The owner-decision queue

Decisions, sign-offs, and adjudications the owner must take are **queued on the tracker, not posed inline and not collected in a document**: a queue bead per topic, one self-contained question per comment identified `<queue-bead-id>-question-NN`, answered by comment under the matching `-answer-NN`.
**File the question before raising it, and never ask leave to file one** — an unfiled question exists only in a transcript, which nothing can search or cite.
If a ruling nonetheless arrives without a filed question, file the question retroactively and record the ruling as an agent-authored transcription comment before the work proceeds.
Every ruling of record lands in the authoritative project artifact; the comment stream is the deliberation record, never the ruling's home.
The full discipline, including the identifier and numbering rules, is [`docs/workflow/tracker.md`](docs/workflow/tracker.md) §"The owner-decision queue".

## Work tracking

Beads is the issue tracker, with prefix `gandr-`.
[`docs/workflow/tracker.md`](docs/workflow/tracker.md) owns the shared-Dolt topology, cross-machine synchronization, server-lifecycle boundary, graph-backup protocol, title and metadata conventions, comment-only progress updates, research-reference retention, audit conventions, feature-landing evidence, and residual closeout.

* Push after every tracker write; pull before relying on tracker reads.
* Never stop, start, or restart the Dolt server unless the owner asks.
* Use neither TodoWrite nor Markdown TODO lists nor an ad hoc `MEMORY.md`.

## Worktrees, agents, and merging

Follow [`docs/workflow/worktrees.md`](docs/workflow/worktrees.md) for the complete lifecycle.

* Do file-modifying work in a `wt` worktree and keep the primary checkout on `main`; ask before creating one when the task's track is unclear.
  The routing and governance documents named by that workflow may land directly on `main`.
* Run `mise run setup` in every fresh worktree before its first commit.
  Never bypass hooks, emulate Worktrunk hooks, or bootstrap `mise trust` from agent preflight.
* Mutating automated agents use the Worktrunk-owned lane: commit visible state, pre-create the worktree, launch the agent there without harness-native worktree creation, require commits only on its branch, then integrate with `wt merge --no-squash`.
* Prefer the structure-aware path: `codegraph` for scope and blast radius, `sem` for divergence and conflict risk, `weave` for merge resolution, and `wt merge` for rebase, gates, and landing.
* Dispose of completed worktrees and branches in the same session.
  If safe disposal is blocked, record the exact state and removal condition in a bead.

## Quality gates and review

[`docs/workflow/ci.md`](docs/workflow/ci.md) owns the current gate inventory and merge wall.

* Run the narrowest gate that proves the change before committing.
* `mise run gate:merge` is part of "done and verified," not a substitute for exercising the changed behavior.
* Scale independent review to the change.
  One-line fixes need the gates; substantial, cross-cutting, corpus, and publishable-stakes changes follow [`docs/workflow/review.md`](docs/workflow/review.md).

## Commits and publishable history

Assume everything committed here will eventually be public.
Before adding tracked content or a commit message, classify it:

* **Project-concern** — design, decisions and rationale, specifications, code, tests, tooling configuration, and honest professional provenance.
  This includes noting that a revision corrects an earlier draft or that a design was refined through dialogue with language models.
  Write it to publishable standard.
* **Contributor-concern** — machine-local paths and hostnames, private artifacts, session and model forensics, harness mechanics, and salvage narratives.
  Keep it out of tracked content and commit messages.

**Acid test:** would the content be wrong or useless for a contributor on different hardware?
If so, it is contributor-concern.

Contributor-concern artifacts live **outside this tree**, in the contributor's own private workspace.
The maintainer's is the private research workspace whose work items may be cited from beads as `ss-` identifiers ([`docs/workflow/tracker.md`](docs/workflow/tracker.md)); session plans, handoffs, research digests, and adversary reports go there, and any project-relevant conclusion is distilled into the appropriate design or decision record here.
The historical sibling notes repositories (`../gandr-notes`, and `../wyrd-notes` before it) are read-only archives, not destinations for new material.

Commit messages are enforced by `commitlint`; `.commitlintrc.mts` is authoritative.

* Use `<type>(<scope>): <subject>`, with required lower-case type and scope.
* Choose the scope from the closed `GANDR_SCOPES` vocabulary.
* Keep the header and every body line at or below 100 characters.
  Separate header, body, and footer with blank lines; omit a trailing period from the subject.
* Agent co-author trailers must match the canonical registry byte-for-byte.
  Session trailers are prohibited.
* **One agent, one name, every surface.** The name an agent signs a commit with is the name it acts under everywhere an author or actor is recorded — tracker writes above all.
  It is the registry entry's name part, byte-for-byte, without the address: `Claude Opus 5 (1M context)`, never a shortened, harness, or session variant.
  The mechanics for the tracker are [`docs/workflow/tracker.md`](docs/workflow/tracker.md) §"Agent-attribution metadata".
* A body line may begin with a colon-suffixed word.
  The stock rule that misread such prose as a footer is replaced by a trailer-aware rule keyed on real trailer tokens, so a trailer still needs its leading blank line and ordinary prose no longer trips it.
* Inspect `.commitlintrc.mts` before inventing a type, scope, or trailer.

The `no-machine-local-paths` hook and commitlint are lexical backstops; classification remains the rule.

## Specification, corpus, and research

* `docs/gandr/spec/README.md` is the specification entry point; the four track documents under it are the design record, and `docs/gandr/spec/bibliography.yml` is the register they cite.
  The prose document-class tool `gandr-workflow-docs` is parked (crate commented out of the workspace), so the tracked `.xml` documents carry no active gate.
  Follow [`docs/workflow/docs.md`](docs/workflow/docs.md).
* Every surfaced language feature lands its complete executable corpus treatment in the same change.
  Syntax-only work lands a parse-gated `surface/` witness; internal-only work lands named exercised fixtures and an explicit promotion blocker.
  Follow [`docs/workflow/corpus.md`](docs/workflow/corpus.md).
* External research artifacts are references for understanding only.
  Never vendor, port, or depend on companion artifacts, regardless of license.

## Rust, automation, and diagnostics

* Read [`docs/workflow/rust.md`](docs/workflow/rust.md) before writing or reviewing Rust.
  It owns the no-partial-functions policy, checked arithmetic, typed errors, contract documentation, and production-versus-test lint posture.
* New automation or a new script starts behind a named `mise` task, which remains the stable entry point.
  Adapt or consolidate an existing task before adding another.
* When modifying the task surface, simplify redundancies that are in scope and surface larger cleanup opportunities.
  Report any future hazard or security concern noticed along the way: immediately if it bears on current work, otherwise at closeout.
  This opportunistic duty does not require a separate audit.
* Adding a task requires consulting the user first unless the user has granted full autonomy for the work.
* If implementation outgrows a small task body, move the logic into an appropriate `workflow-*` crate instead of adding a standalone script.
  Creating such a crate also requires consultation unless full autonomy has been granted.
* Report every task addition, removal, or material change—and every new workflow crate—at closeout.
* Enumerate compiler, linter, and language-server diagnostics through aifix, with its CLI only as fallback.
  `mise run cargo:clippy` is a pass/fail gate, not a diagnostic-enumeration interface.

## Completion and durable state

Fresh sessions are preferred for significant work, and re-orienting later has real cost.
Finish cheap follow-through while the context is loaded: one-line documentation repairs, hazard records, and small tracker updates should not force a later session to re-orient.
Defer only genuinely significant work.
If a "small" follow-up starts requiring real building, land the cheap increment and record the remainder.

A finished task leaves durable state: tracker items updated and synchronized, decisions in the authoritative project record, working-tree changes committed to publishable standard, and stale memory revised.

At session close:

* complete the lifecycle in [`docs/workflow/worktrees.md`](docs/workflow/worktrees.md), [`docs/workflow/ci.md`](docs/workflow/ci.md), and [`docs/workflow/tracker.md`](docs/workflow/tracker.md);
* keep history local while no remote exists; once one exists, push reviewed, signed work to `main` at arc boundaries rather than per commit;
* apply the consolidated residuals-bead rule from the tracker workflow, marking inapplicable faces explicitly.
