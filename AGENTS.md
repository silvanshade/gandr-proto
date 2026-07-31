# Agent Guidance

## Start here

* `PLAN.html` — the approved reboot roadmap and current project wayfinder.
* `docs/gandr/spec/README.md` — the entry point to the language specification corpus (overview, metatheory, implementation, proof-engineering tracks).
* `docs/WORKFLOW.md` — the workflow routing layer.
  Read it first, then open only the task-scoped file it points to under `docs/workflow/`.

This file is a thin orientation adapter.
Substantive process guidance belongs in the routed workflow documents; reference it instead of restating it here.

## Working posture

* **Surgical changes; structural evaluation first.** Make the task-scoped change without unrelated rewrites, but first ask whether the change should extract shared functionality, prune duplication, or draw a module boundary.
  Act when that remains in scope; otherwise file a tracker item.
  Surface the opportunity either way.
* **Leave touched areas better.** Report undocumented engineering improvements and hazards; noticing and staying silent is the failure mode.
* **State uncertainty.** Say when current code or documentation does not prove a claim, and report unexpected harness, tool, or configuration failures immediately.
* **A refutation is the most expensive claim available here, and it binds only with the owner's sign-off.** Before recording that something does not apply, is not needed, cannot be done, or is the wrong structure, read [`docs/workflow/review.md`](docs/workflow/review.md) §"Declining is a claim too" and §"Refutations bind only with owner sign-off" — **whatever the task**, not only when reviewing.
  A wrong acceptance is loud and self-limiting; a wrong rejection is silent, compounding, and has been this project's recurring failure.
  The one-line test: _is the reason a fact about the machinery, or a fact about us?_ A reason that names one of our own properties is a representation question, not a refutation.
* **Verify behavior.** A green build is not proof.
  Exercise the changed behavior and add or update tests when behavior changes.
* **Gates prove structure, not meaning.** Substantial or publishable-stakes changes require an independent adversarial review before landing.
  Follow [`docs/workflow/review.md`](docs/workflow/review.md).

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
* `mise run gate:merge` is part of “done and verified,” not a substitute for exercising the changed behavior.
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

Store contributor-concern artifacts in a separate, versioned notes repository outside this tree; this repository has no in-repo `notes/` directory or stranded-notes guard.
Distill any project-relevant conclusion into the appropriate design or decision record, and leave contributor context in the notes location.

Commit messages are enforced by `commitlint`; `.commitlintrc.mts` is authoritative.

* Use `<type>(<scope>): <subject>`, with required lower-case type and scope.
* Choose the scope from the closed `GANDR_SCOPES` vocabulary.
* Keep the header and every body line at or below 100 characters.
  Separate header, body, and footer with blank lines; omit a trailing period from the subject.
* Agent co-author trailers must match the canonical registry byte-for-byte.
  Session trailers are prohibited.
* **Never begin a body line with `word:`.** The parser reads any such line as the start of the footer, so the prose above it stops being the body and `footer-leading-blank` rejects the message.
  This bites on ordinary sentences — `Note: …`, `Caveat: …`, `Exception: …` — and the error names the footer rather than the line that caused it, so it reads as unrelated.
  Reword (`One caveat is that …`) or move the colon off the line start.
* Inspect `.commitlintrc.mts` before inventing a type, scope, or trailer.

The `no-machine-local-paths` hook and commitlint are lexical backstops; classification remains the rule.

## Specification, corpus, and research

* `docs/gandr/spec/README.md` is the specification entry point; the four track documents under it are the design record.
  The validated `.gfd` corpus and its `gandr-workflow-docs` pipeline are parked (crates commented out of the workspace); `docs/spec/refs.yml` is derived and must be regenerated, never edited by hand, when that pipeline returns.
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
If a “small” follow-up starts requiring real building, land the cheap increment and record the remainder.

A finished task leaves durable state: tracker items updated and synchronized, decisions in the authoritative project record, working-tree changes committed to publishable standard, and stale memory revised.

At session close:

* complete the lifecycle in [`docs/workflow/worktrees.md`](docs/workflow/worktrees.md), [`docs/workflow/ci.md`](docs/workflow/ci.md), and [`docs/workflow/tracker.md`](docs/workflow/tracker.md);
* keep history local while no remote exists; once one exists, push reviewed, signed work to `main` at arc boundaries rather than per commit;
* apply the consolidated residuals-bead rule from the tracker workflow, marking inapplicable faces explicitly.
