# Workflow: worktrees, merging, and agent isolation

> Read when: creating or removing a worktree, merging a branch, launching a mutating sub-agent, or debugging a `wt` hook.
> Base lifecycle (`wt switch --create` / `wt merge --no-squash main` / `wt remove`, the squash gotcha, hook approvals, governance-docs-on-main): `.agents/core/core/WORKFLOW.md` §"Worktrees and merging".
> This file is the gandr delta.
> **Standing rule, whatever the task:** before recording that something does not apply, is not needed, or cannot be done, read [review.md](review.md) §"Declining is a claim too" and §"Refutations bind only with owner sign-off" — a refutation binds only with the owner's sign-off.

## Layout and trust

* **Worktree location.** Worktrees live under one per-repo sibling directory (`../gandr-worktrees/<branch>`) via the `worktree-path` template in your **user** worktrunk config — never nested inside the repo (a nested worktree pulls the main `mise.toml` in as a duplicate parent layer and shows up as untracked content in main's tree).
* **Trust.** Require mise v2026.7.5+; trust the main checkout once and let linked worktrees inherit it.
  Never bootstrap trust from Worktrunk or agent preflight hooks (core H15).
  Snapshot harnesses keep native CoW/reflink isolation and trust their stable root once in global mise config (OMP: `{{ env.HOME }}/.omp/wt`).
* **Fresh worktree setup.** Run `mise run setup` before the first commit — it installs the node dependencies the treefmt pre-commit and commitlint push-range hooks need, plus the pinned Rust toolchains (the old `--no-verify` bypass is obsolete and agent briefs must never instruct it).

## Hooks (`.config/wt.toml`)

* `[[pre-start]]`: `copy-ignored` (fail-open ignored-state reflink; excludes `.beads/**` — the shared-tracker topology guard, [tracker.md](tracker.md) §"Source of truth and sync" — plus `target/**` and friends) and `beads-pull` (`bd dolt pull || true` — cross-machine freshness of the shared database; worktree-to-worktree visibility needs no pull).
  The template's `mise setup` and submodule init/warmup pre-start steps are parked while the reboot bootstraps — they reference state this repo does not have yet and re-grow with the pieces they serve.
* **`[pre-merge]` is the local wall** — any non-zero exit aborts the merge: `mise run gate:merge` (the composed merge check, [ci.md](ci.md)) and `beads` (`bd dolt pull && bd dolt push` — make the branch's beads durable on DoltHub once gates are green; pull-then-push self-heals the race with other pushers).
  Parked pending their prerequisites: `adr-guard` (ADRs land on `main` only, core H5 — returns when `docs/adr/` exists) and `core-pin` (`mise run core:check`, the read-only vendored core at its pin — returns when the agentic-dev core is vendored at `.agents/core`).
* `[post-merge]`: `beads-pull` in the primary — with the shared per-machine database this is cross-machine freshness only; the merged branch's beads were already visible locally the moment they were written.

Contributor notes live in the sibling notes repository, named `../<repo-name>-notes` beside the primary checkout (here `gandr-notes`; earlier sessions used the predecessor's `wyrd-notes`, now holding a pointer), so worktree lifecycle operations cannot strand them — the historical in-repo gitignored `notes/` and its `notes-guard` gate are retired.

## Mutating sub-agents: the Worktrunk-owned lane

A sub-agent with Bash/git access shares the live working tree unless given its own path — a stray `git checkout`/formatter/merge can revert uncommitted work (core H7; realized in the predecessor project during the record-rung adversarial pass).
The lane:

1. the orchestrator **commits visible state first**;
2. pre-creates the worktree: `wt switch --create <branch> --base=@ --no-cd`;
3. launches the agent at the returned path with harness-native worktree creation disabled;
4. the agent commits only on its assigned branch;
5. the orchestrator integrates with `wt merge --no-squash <target>`.

A harness that cannot target the path keeps its native isolation backend and stable globally trusted root; it never emulates Worktrunk hooks or runs `mise trust` as preflight (core H15).
Read-only no-command agents may share a tree.
Hooks are user-preapproved with `wt config approvals add`; agents never pass blanket `--yes`.

## Sub-agent briefs: tool routing is the orchestrator's job

Ambient tooling guidance does not reach tool-restricted agent types, and a detailed brief displaces whatever does — so the brief itself must carry the routing.
Before spawning any sub-agent:

1. **Verify tool availability for the target agent type.** MCP tools exist only where the agent type exposes them (many types are Bash-only); a CLI fallback counts only if confirmed invocable (e.g. `codegraph --help`) from that agent's environment.
   **If availability cannot be verified, stop and get help from the owner before launching the task** (owner rule, 2026-07-12) — this failure is routinely caught too late to respond to.
2. **Route tools explicitly in the brief**: which tool for which step, MCP or CLI form, and any indexed tree or workspace the tool must be pointed at.
3. **Prefer the structure-aware tools** the task's data already has an index for (codegraph for any `.codegraph`-indexed tree — including foreign checkouts; sem/weave for diffs and merges) over grep/read loops, and say so in the brief.

Realized failure (2026-07-12): a boundary-analysis sub-agent ran grep/Read over a codegraph-indexed tree because the brief dropped the routing and the agent type had no MCP exposure; the gap surfaced only after the report landed.

## End-of-work lifecycle: no branch or worktree left in an unclear state

Work routinely finishes while its branch and worktree linger, and nobody can later tell whether they hold anything unlanded (owner rule, 2026-07-19).
The finishing session disposes of what it created, in the same session the work concludes:

1. **Integrate or record.** Deliverable branches merge to `main` once approved; a branch that deliberately does not merge gets its supersession/decline rationale recorded (tracker comment or doc) before deletion.
2. **Remove the worktree** with a plain `git worktree remove` / `wt remove` — never `--force` on the first attempt: a refusal means uncommitted state, which is triaged (commit, salvage to a bead, or deliberately discard with the rationale written down), not clobbered.
3. **Delete the branch** with `git branch -d`; `-D` only for a branch whose content is formally superseded, with the rationale recorded where the supersession was decided.
4. **If disposal cannot happen yet** (awaiting review, unmerged residue, blocked integration), **file a residual bead** naming the branch/worktree, its exact state, and the condition under which it becomes safe to remove.
   The bead is the handoff — a branch's status must never live only in a session's memory.
5. **Automation-spawned worktrees and branches** (workflow `wf_*`, agent `agent-*`) are disposed by the orchestrating session at task close.
   A later session that finds strays treats them as triage under rule 2: verify clean, then remove; anything dirty gets a bead before any deletion.

Realized instance (2026-07-19): 36 stale worktrees and 44 dead branches had accumulated across one research cycle because sessions ended without disposal; the cleanup consumed a closeout session that rules 1–5 make unnecessary.

## ADRs and governance docs

The governance-doc carve-out (`.agents/core/core/WORKFLOW.md` §"Governance docs land on main") covers gandr's `AGENTS.md`, `CLAUDE.md`, `CONTRIBUTING.md`, `docs/{WORKFLOW,KNOWLEDGE,HAZARDS}.md`, the `docs/workflow/` sub-files, and `docs/adr/` — these land on `main` directly, one commit each.
ADRs are per-decision files (`docs/adr/NNNN-slug.md`), so parallel branches no longer collide on content or manifest hash; the residual risk is a **number race** (two branches minting the same next number), which is why record-on-main-first stands as policy.
`docs/adr/` does not exist yet — the reboot bootstrap mints no new ADRs (owner direction, `gandr-fcw`; decisions live in the approved `PLAN.html` and the wayfinder tracker until a decision log is deliberately re-introduced) — so the `adr-guard` `[pre-merge]` gate is parked and re-arms with the directory.
