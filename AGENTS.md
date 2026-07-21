# Agent Guidance

## Map

* `PLAN.html` — the approved reboot roadmap and current project wayfinder.
* `docs/spec/index.xml` — the entry point to the status-attributed, tool-validated specification corpus.
* `docs/WORKFLOW.md` — the workflow **routing layer**: read it first, then open only the task-scoped sub-file it points to under `docs/workflow/` (tracker, worktrees, ci, scripting, rust, mutation-adequacy, soundness, corpus, agda, review, docs).

## Working posture

* **Surgical changes; structural evaluation first.** Make surgical changes to the task itself; do not sprawl into unrelated rewrites.
  But before modifying, evaluate structure: should this change extract shared functionality, prune duplication, or draw a module boundary?
  Act when in-scope and surgical, otherwise file a tracker item — always surface it.
  "Surgical" governs HOW you touch code; modularity-first governs WHETHER you first evaluate its structure.
* **Leave what you modify in as good or better shape than you found it.** Report engineering improvements or hazards you notice that are not yet documented — noticing and staying silent is the failure mode.
* **State uncertainty** when current code or docs do not prove a claim.
  Report unexpected harness/tool/config failures immediately.
* **Verify before completion.** A green build is not proof; exercise the behavior the change affects, and add or update tests for behavior changes.
* **Gates prove structure, not meaning.** For substantial or publishable-stakes changes, run an independent adversarial review before committing (`WORKFLOW.md` §"Adversarial review").

## Task completion — do small immediate follow-ups NOW

Fresh sessions are preferred for significant work, and re-orienting a new session has real cost.
That makes deferral _asymmetric_: a small step costs little to finish now but forces a full re-orient if pushed to "later".
So, inverted from the usual instinct:

* At the end of every task, scan for small steps that can be done immediately — a one-line doc fix, a hazard to record, a follow-up tracker item to file — and do them now, while the context is loaded.
* Defer only genuinely _significant_ work (anything that wants its own focused session).
* Stay honest about size: if a "small" step turns out to need real building, stop, land the genuinely cheap increment, and defer the significant remainder.

## Publishable history

Classify every change **project-concern vs contributor-concern** before committing — machine-local paths, host quirks, session/model forensics never enter tracked content or commit messages.
The doctrine and the acid test are `PUBLISHABLE-HISTORY.md`; the mechanical backstops are the `no-machine-local-paths` prek hook and the commitlint session-trailer ban (`fragments/commitlintrc.base.mts`).

## Durable state

Finished tasks produce durable artifacts: tracker items updated and synced, decisions in the ADR, working-tree changes committed to publishable standard, memory revised where the task made it stale.
The full discipline — including the tracker sync rules and the session-close checklist — is `WORKFLOW.md`.

## Publishable history — project-concern vs contributor-concern

Everything that enters a repo's git history is written assuming it will **eventually be public**.
Before adding content to tracked files or commit messages, classify it:

* **Project-concern** — the design, decisions and their rationale, specs, code, tests, tooling configuration, and honest professional provenance (e.g. "this revision corrects an earlier draft"; "the design genesis was iterative refinement through dialogue with various language models").
  This belongs in tracked files and commit messages, written to publishable standard.
* **Contributor-concern** — anything meaningful only to a particular contributor's machine, workflow, or process: machine-local paths and hostnames, private artifacts and their filenames, session and model forensics (which model produced a draft, what it hallucinated), harness and workflow mechanics, salvage/rebuild narratives.
  This never enters tracked content or commit messages.

**The acid test: would this content be wrong or useless for a second contributor on different hardware?** If yes, it is contributor-concern and stays out.

### Where contributor-concern material lives

An untracked location, by consumer choice:

* a gitignored `notes/` directory in-repo (the original pattern — pair it with the stranded-notes exposure, `HAZARDS.md` H1), or
* a **sister notes repo** (a separate, fully version-controlled local repository the project references only as "refer to local notes") — removes the H1 exposure by construction and is the recommended pattern for new consumers.

When contributor context is needed to understand a project decision, distill the project-relevant part into the decision record and keep the rest in the notes location.

### Mechanical backstops

* The `no-machine-local-paths` prek hook (`fragments/prek.base.toml`) rejects staged content containing machine-local home paths.
* The commitlint base (`fragments/commitlintrc.base.mts`) rejects session-link trailers (`Claude-Session:` and variants) — agent-harness forensics are contributor-concern — and requires agent co-author trailers to match the canonical registry byte-for-byte, so agent attribution (which IS project-concern, as honest provenance) stays uniform.

Backstops are lexical and incomplete; the classification is the rule, the hooks are seatbelts.

## Project delta (kept thin)

* **Tracker**: beads, prefix `gandr-`; sync rides DoltHub [`silvanshade/gandr-beads`](https://www.dolthub.com/repositories/silvanshade/gandr-beads) via `bd dolt push` / `pull` (out-of-band from git — push after every write, pull before reads; core/HAZARDS.md H2).
  No TodoWrite / markdown TODOs / ad-hoc `MEMORY.md`.
  `bv --robot-*` for triage (bare `bv` blocks the session); confirm with `bd show` (`bd list` hides closed — use `--all` when auditing, core/HAZARDS.md H3).
* **Gates** (run the narrowest that proves your change): `mise run treefmt:check`, `docs:conflict-markers`, `docs:manifest-drift` (MANIFEST.yml BLAKE3), `docs:reference-integrity`, `wrkflw` (workflow edits); Rust — `cargo:clippy` (pass/fail gate only), `cargo:nextest`; Agda — `agda:check`.
  These run in CI (the gate of record) and locally via `prek` once you `prek install` (once per clone, primary checkout; core/HAZARDS.md H4).
* **Specification corpus**: `docs/spec/index.xml` is the entry point to status-attributed XML components validated by `gandr-workflow-docs`; `docs/spec/refs.yml` is derived and must be regenerated rather than edited by hand (`docs/workflow/docs.md`).
* **Corpus treatment**: every new surfaced gandr feature lands runnable literate model examples, runnable pathological coverage, harness assertions, and coverage-map registration in the SAME change (`crates/gandr-corpus`; ADR-84 supersedes ADR-52 Decision B and Decision C's two-tree cardinality; `docs/workflow/corpus.md`).
  A syntax-only landing gets a parse-gated `surface/` witness; its semantics-graduation change promotes that witness and adds the full treatment in that same change.
  Internal-only work lands named fixtures exercised by named tests plus an explicit corpus-promotion blocker.
<!--* **gandr-pro skill**: load the `gandr-pro` skill BEFORE any gandr-related work — writing or reviewing `.gandr` programs, corpus examples, or reasoning about gandr semantics.
  Source of truth: `crates/gandr-corpus/skills/gandr-pro/SKILL.md` (ADR-52 Decision E), surfaced via `.claude/skills/` + `.omp/skills/` symlinks, maintained on the corpus-treatment train.-->
* **ADR log**: wyrd's is `docs/adr/` (b3sum-hashed, sequentially numbered — the hard ADR-on-main case; the `adr-guard` `[pre-merge]` gate rejects branch ADR edits, core/HAZARDS.md H5).
  Record an ADR on `main` first, then rebase.
* **Agda in its own commit**: keep `metatheory/**` (Agda) work in a separate commit from the Rust it mirrors (distinct artifact whose history may be reorganized; the `docs/gandr/**` dictionary face may ride with either).
* **Rust code** follows `docs/workflow/rust.md` — no partial functions (no indexing/slicing, no `unwrap`/`expect` in shipping code), checked arithmetic, typed errors over panics, a `# Contract` rustdoc block on nontrivial items.
  The `Cargo.toml` `[workspace.lints]` wall enforces the mechanical parts; test/bench relax it via one crate-level `#![cfg_attr(test, allow(...))]`, production never.
* **Diagnostics via aifix**: reading / enumerating compiler / lint / LSP diagnostics ALWAYS goes through the `aifix_*` MCP tools (CLI `aifix` fallback), never by parsing raw `cargo clippy` (a raw `-D warnings` run aborts at the first failing target and under-reports).
  `mise run cargo:clippy` is only the pass/fail gate.
  `docs/workflow/scripting.md` §"Diagnostics go through aifix".
* **Scripts**: typed only — Nushell for small/pipeline scripts, TypeScript (type-stripping Node, no build) for larger; no untyped `bash`/`sh`; a bare `any`/`unknown` needs explicit sign-off.
  `core/WORKFLOW.md` §Scripting; worked examples (doc-gate scripts, `std/assert` shadowing, nutest) in `docs/workflow/scripting.md`.
* **External research = reference only; Agda deps vetted**: research artifacts (companion code, mechanizations) are for understanding only — never vendored / ported / depended-on, any license; adding an **Agda** dependency needs maintainer sign-off first (stricter than the Rust/TS trees, where the finder skills give latitude).
  The sister **internal-univalence** library is in-house, not external: its engine is consumed as a pinned, read-only git submodule at `metatheory/upstream/internal-univalence` (`iu:check` guards the pin) per `docs/gandr/spec/proposal-metatheory-relaunch.md`, with the integration record in `metatheory/README.md` §"Upstream integration" and the house style in `docs/workflow/agda.md`.
  The reference-only rule and the Agda vetting bar are both in `docs/workflow/agda.md`.
* **Structure-aware triad, not raw git**: `wt merge` orchestrates (rebase + gate + land); the content analysis goes `codegraph` (scope / blast radius) → `sem` (what diverged / conflict risk, not `git diff`) → `weave` (merge / conflict resolution, not `git merge`).
* **Worktrees**: all file-modifying work in a `wt` worktree; the primary checkout stays on `main` (ask before creating one if the track is unclear).
  In every fresh worktree run `mise run setup` before the first commit — it populates the root and grammar-package node dependencies so the treefmt pre-commit and commitlint push-range hooks pass natively (the old sanctioned `--no-verify` bypass is obsolete, and agent briefs must instruct the setup task, never the bypass).
  Mutating automated agents use the orchestrator lane: commit visible state, pre-create `wt switch --create <branch> --base=@ --no-cd`, launch the agent at the returned path with harness-native worktree creation disabled, require the agent to commit only on its assigned branch, and integrate that branch into the owning branch with `wt merge --no-squash <target>`.
  Harnesses that cannot target that path keep native CoW/reflink isolation (`task.isolation.mode = "auto"`) and a stable root trusted once in global mise configuration (OMP: `{{ env.HOME }}/.omp/wt`); never emulate Worktrunk hooks or run `mise trust` as agent preflight (core H15).
  Read-only no-command agents may share a tree; project hooks are user-preapproved with `wt config approvals add`, agents never use blanket `--yes`, and Worktrunk fails loudly when noninteractive and unapproved.
  The governance-doc carve-out (core/WORKFLOW.md) covers wyrd's `AGENTS.md`, `CLAUDE.md`, `CONTRIBUTING.md`, `docs/{WORKFLOW,KNOWLEDGE,HAZARDS}.md`, `docs/workflow/`, and `docs/adr/` — those land on `main` directly.
* **Contributor notes**: contributor-concern material (session forensics, handoffs, machine-local scratch, salvage narratives) lives in the sibling `wyrd-notes` repository, not here — there is no in-repo `notes/` directory and no stranded-notes guard.
  Classify per `.agents/core/core/PUBLISHABLE-HISTORY.md` before every commit; when contributor context explains a project decision, distill the project-relevant part into `docs/adr/` and leave the rest in wyrd-notes.
* **Session close**: no git remote is configured during the reboot bootstrap — history is local-only until the gandr remote lands; once it does, reviewed work is committed and pushed to `main` with **signed** commits (conservative — push reviewed work, not speculative/unrequested).
  Pushes are **arc-boundary events**: push after a full arc of work has merged to `main` (typically a `wt merge` landing), never per-commit — the pre-push tier deliberately runs the complete act-CI simulation (minutes; `docs/workflow/ci.md` §"Gate tiers"), so batching an arc's commits into one push is the intended shape.
  Full lifecycle: `core/WORKFLOW.md` §"Session close".
  At closeout file the **residuals bead** — manual, mutation-adequacy, other-residual, and corpus faces folded into one, with `not applicable` explicit (`docs/workflow/tracker.md` §"Feature landing and residual closeout").
