# Workflow: issue tracking (beads)

> Read when: creating, closing, or triaging beads; wiring dependencies; auditing tracker state.
> Base discipline: `.agents/core/core/WORKFLOW.md` §"Issue tracking".
> This file is the gandr delta.
> **Standing rule, whatever the task:** before recording that something does not apply, is not needed, or cannot be done, read [review.md](review.md) §"Declining is a claim too" and §"Refutations bind only with owner sign-off" — a refutation binds only with the owner's sign-off.

## Source of truth and sync

Issues live in **one shared Dolt database per machine** (beads, prefix `gandr-`): the primary checkout and every worktree resolve the same database through the git common directory, so worktrees carry **no gitignored `.beads` state of their own** (the `wt` copy-ignored step excludes `.beads/**`; topology record: `gandr-fid.15`).
A bead written from any checkout is immediately visible in all of them — no pull between worktrees.
That immediacy is about **visibility**, and it does not extend to the remote: because every checkout advances the same branch, worktrees still contend with each other when pushing.
The database syncs **out-of-band from git** to DoltHub [`silvanshade/gandr-beads`](https://www.dolthub.com/repositories/silvanshade/gandr-beads) (the `origin` Dolt remote), the only off-machine copy:

* **push after every write** — `bd dolt push` immediately after `bd create`/`update`/`dep`/`close`;
* **pull before relying on reads** — `bd dolt pull` at session start and before triage (the `wt` `beads-pull` hooks and the `SessionStart` hook mechanize this); this guards **cross-machine** staleness, not worktree-to-worktree staleness;
* trust the remote over a local clone when they disagree;
* **expect a rejected push, and do not escalate** — see below;
* **read remotes with `bd dolt remote list`** — `bd dolt show` reports `Remotes: (none)` even when `origin` is configured and pushes are arriving (`gandr-mib`);
* **server lifecycle is owner-controlled** — agents never run `bd dolt stop`/`start` or restart the Dolt server unprompted: a mid-session kill is what forked per-worktree clones and double-recorded a closeout on 2026-07-22 (`gandr-fid.15`).

### A rejected push is contention, not breakage

The shared database has **one branch and many writers**: every checkout, and every agent working in one, commits to the same branch, and any of them may push first.
So "updates were rejected because the tip of your current branch is behind" is the **ordinary outcome of concurrency**, not a symptom of a broken topology.
Measured 2026-08-02: 367 tracker commits in one day, with two sessions' commits interleaved four times inside a twelve-second window.

The remedy is three steps, and the middle one is the one that gets skipped:

```text
bd dolt pull      # take the other writer's commits
bd dolt commit    # commit your own pending working-set change
bd dolt push
```

Two things about that sequence are worth knowing before they are met.

**"Pull complete" followed by a still-rejected push is a normal pair, not a contradiction.** Under active contention another writer can land between your pull and your push; repeat the sequence rather than concluding the pull failed.

**`bd dolt commit` is not redundant.** Most `bd` writes commit as they go, but a write can leave a change in the working set, and a push only ever pushes commits — so a push can be rejected while the work sits uncommitted and invisible to it.

**Do not reach for the conflict-repair recipe for this.** That recipe (`gandr-fid.15`) exists for genuine merge conflicts, is a graph-wide operation, and is owner territory requiring a backup first.
Contention is not conflict; treating one as the other is how a routine race becomes an incident.

**Topology invariant** (check when in doubt): `bd dolt status` from every checkout reports the **same** PID/port/data-dir (the primary checkout's `.beads/dolt`), and a probe bead created in one checkout is instantly visible from another without any pull.
A worktree whose `.beads` holds gitignored database state (`dolt/`, `embeddeddolt/`, `metadata.json`, `dolt-server.*`) is misconfigured: prove nothing local-only exists vs `origin/main`, then quarantine that state (recipe in `gandr-fid.15`).

`.beads/*.jsonl` is a gitignored local-only export read by `bv`, never committed.

## Filing and lifetime policy (anti-drift)

The 2026-07-12 triage deleted ~600 of 845 beads; these rules exist so that never recurs.

* **A bead is a work item, not a memory.** File a bead only for work someone will plausibly execute within the current or next program of work.
  Insight belongs in an ADR (decision), the corpus (design), or the notes repo (contributor context) — never parked in the tracker.
* **Consolidation-first.** Filing what would be the third bead on one topic means the topic needs ONE topic bead instead — extend or replace; never accumulate siblings an agent must reassemble.
  Residual seams discovered during work go into the owning topic bead (a `bd comment` or a description line), not as new dangling beads, unless immediately schedulable.
* **Research expires.** A research question gets one bead whose deliverable is a decision (adopt/defer, recorded as an ADR); when the pass completes, the bead closes and residual leads go to the single project watchlist bead.
  A lead not promoted to a decision by the next triage sweep is deleted — it can be re-asked if it ever matters again.
* **Triage sweep cadence.** At each sweep (weekly obligation): active beads pruned toward ≤20; off-trajectory items older than a week deleted or folded into topic beads; closed beads older than ~2 weeks purged; deferred topic beads reviewed for reactivation.
* **Epics need children or a close date.** An epic with no active children is either done (close it) or a label (fold it into a topic bead).

## Conventions

* **Small graph, current graph.** The tracker is a working set, not an archive: ≲20 active beads, consolidated by topic.
  Prefer one compact bead that carries a topic's current state over several partial beads an agent must reassemble.
  Prune aggressively at triage; a dropped question can be re-asked later if it ever matters again (owner posture, 2026-07-12).
* **Reconstruction is the filing acid test.** A contributor must be able to reconstruct and execute the full task from the bead plus standing project guidance alone.
  Include the intended outcome, boundaries and non-goals, governing decisions and provenance, affected interfaces, dependencies, acceptance and verification evidence, and authoritative references needed to act safely.
  Do not depend on private notes, session history, branch state, or an unstated conversation.
  Point to stable tracked project records instead of duplicating them; if the contract cannot remain compact, use one topic bead with executable children rather than omitting context.

### Titles and metadata

* **Normalize titles as `<scope>: <subject>`.** Use one or more comma-delimited members of the closed `GANDR_SCOPES` vocabulary in `.commitlintrc.mts`; sort multiple scopes lexicographically, with no spaces between them.
  The bead type remains structured metadata and is not repeated in the title.
  Keep ordinary words lower-case, preserving case only inside a backtick-delimited literal whose spelling is case-sensitive.
  The subject is action-oriented and names the outcome, decision, or question—not implementation steps, status, dates, branch names, bead IDs, or provenance unless one is essential to distinguish the work.
  Titles have no trailing period, must fit the commitlint 100-character ceiling, and should usually fit within 72 characters.
* **Choose the precise work type.** Use `decision` for an explicit choice or ratified architecture record; `spike` for a timeboxed investigation that reduces uncertainty before commitment; `story` for a user-perspective capability; and `milestone` only for a zero-work completion marker over related issues.
  Use `epic`, `feature`, `bug`, `task`, or `chore` when their ordinary meanings fit better; do not force an underused type onto mismatched work.
* **Choose metadata before filing.** Before every `bd create`, run `bd label list-all` and `bd types list-all`, then select the most appropriate existing labels and work type.
  Do not invent a near-synonym for convenience.
  If nothing fits well—or the bead would not surface reliably in a targeted search—consult the user before adding a metadata category.
  With explicitly granted full autonomy, create a category when appropriate and report every metadata-category addition or change at closeout.
  When searching a label family, enumerate the matching labels by prefix and query every applicable exact label.
* **Retain full research citations in the bead.** For every relevant paper, record its title, authors, year, and the best-fitting unique locator—prefer a DOI, then an arXiv or HAL identifier, then a stable URL.
  Cite standards, repositories, issues, and other research sources with equivalent identifying detail and a resolvable locator; a corpus path or filename alone is not a research citation.
  Never leave the only citation in session context or a notes-repository report.
  Put references known at filing in the description.
  References discovered later are the sole exception to the comment-only update rule: add them directly to the bead's standing body, normally in a compact `## References` section in `notes`, so the bead alone retains its canonical bibliography.

### Safe graph and field updates

* **Back up before graph-wide operations.** Before bulk triage, normalization, relabeling, dependency rewrites, or any other graph-wide mutation, create and sync a Dolt-native `bd backup` to a durable location outside the project tree.
  Verify the backup status and retain the backup until the operation is complete, synchronized, and verified safe to roll forward without it.
  Follow the end-to-end [beads graph sweep workflow](beads-graph-sweep.xml) for baseline capture, read-only classification, deterministic mutation, conservation checks, and reporting.
* **Progress additions are comments only, except references.** After filing, append progress, evidence, and closeout chronology with `bd comment`; never accumulate them by amending `notes`, `description`, `design`, or `acceptance_criteria`.
  Edit those standing fields only to correct the bead's authoritative current contract or to add and maintain its canonical research references.
* Beads cite corpus paths (`docs/gandr/spec/…`, `docs/adr/…`) so an agent lands with context.
* Every doc-drift finding files a bead (`docs/KNOWLEDGE.md` phase 1) — drift produces work items, not silent warnings.
* Dependencies via `bd dep add <child> <parent>`; **after any dep change regenerate the passive export** (`bd export -o .beads/issues.jsonl`) so `bv` sees the edge — it reads the export, not Dolt.
  Trust `bd show`/`bd blocked` over `bv` when they disagree.
* `bd list` hides closed issues — use `--all` / `bd show <id>` when auditing done-ness (core/HAZARDS.md H3).
  JSON listing commands paginate; pass `-n 0` (or use `bd export`) for complete sets.
* **Large text fields wedge the database** (owner-confirmed 2026-07-19; formerly a hazards-doc entry, now standing workflow guidance).
  Keep `notes`, `description`, `design`, and `acceptance_criteria` compact: standing directions plus a short pointer at most.
  Field updates REPLACE content and can clobber prior context; comments append safely.

## The owner-decision queue

> Adopted (owner, 2026-08-02) after compressed in-chat decision batches proved hard to answer: items were hard to tell apart, hard to answer individually, and easy to lose.
> Re-homed onto the tracker the same day, for the reason below.

Decisions, sign-offs, and adjudications the owner must take are **queued on the tracker**, not posed as inline batches in chat and not collected in a shared document.

**Why the tracker rather than a queue document, stated once so it is not re-litigated.** A shared queue document has a single mutable identifier space, a single file, and no query surface, so every concurrent writer is a collision waiting to happen — and the first one arrived within a day of adoption, when two sessions independently minted the same `owner-q-016` for unrelated questions.
The tracker already answers all three: bead identifiers are allocated centrally and cannot collide, comments append instead of conflicting, and one label makes the whole open queue a single query.
This is the same failure the reference discipline exists to prevent, met from the tooling side: **a colliding identifier is worse than none, because it reads as precise**.

### Where a queue lives

* **A bead that needs decisions gets a queue bead as its child** (`bd create … --parent <bead>`), so the queue's identifier is a suffix of the work it serves and the link is structural rather than remembered.
* **An epic gets one queue bead for the whole epic**, never one per child.
  A question raised while working a child goes on the epic's queue bead, and names the child it concerns.
* The queue bead's type is `decision` and it carries the **`human`** label, so `bd list --label human --status open` is the standing view of everything waiting on the owner.
  `human` marks _awaiting the owner_ and is what separates a queue bead from an ordinary `decision` bead an agent will research and record.
* Title it `<scope>: decision queue for <topic>` so it reads as a container rather than as a decision someone is about to take.

### Posing a question

**One question is one comment**, and it opens with its identifier.

**The identifier is `<queue-bead-id>-question-NN`**, zero-padded, numbered within that bead, and stable: retiring a question leaves its number unused rather than renumbering the rest.

**The prefix is always the hosting queue bead's own identifier — never the identifier of a bead the question is about.** A question may concern another bead entirely, or half of one and half of another; it still takes the prefix of the bead it _lives on_, and its body names whatever else it concerns.
That is what makes the identifier self-locating: `gandr-fid.14.7-question-03` is in `gandr-fid.14.7`, always, and a reader who meets it in a commit message or another bead knows exactly where to look.

**Each question is self-contained**, to the same standard as a bead's own reconstruction test: the context, a plain-terms explanation, a **concrete statement of what the decision changes**, the options, and the agent's recommendation with its reason.
An owner should be able to answer from the comment alone, without session history.

### Answering, and what an agent may not do

**The owner answers with a comment** on the same bead, leading with the same identifier.

**An agent never writes the owner's answer**, never records a ruling the owner did not give, and never converts silence into consent.
An unanswered question stays unanswered; if the work cannot proceed without it, that is a blocked bead, not a licence to decide.

### Closeout

* The **ruling of record lands in the authoritative artifact** — the corpus document, the decision record, the code, or the owning bead's standing contract — never only in the comment stream.
* A follow-up comment records the execution and names where the ruling landed, so the queue bead reads as an audit trail rather than a second source of truth.
* A child bead's queue closes when its parent's decisions are taken and executed; an **epic's queue bead lives as long as the epic** and closes with it.
* The comment stream may be cited, but citing it never substitutes for the ruling's home.

## Feature landing and residual closeout

The base rule still governs: close a bead only when its full recorded scope is done and verified; make residual scope epic-shaped; file follow-ups `discovered-from` the parent; sweep related memory before closing. gandr adds one **feature-landing workflow** so executable evidence, manual work, mutation campaigns, and other residuals cannot drift into separate conventions.

**The merge gate is part of "done and verified"**: before a task is considered finished — and before its residuals bead is filed — the work must pass `mise run gate:merge` in the landing worktree (the same gate `wt merge` runs pre-merge).
Passing narrower gates (build, scoped clippy, nextest) is not sufficient: the merge gate is where project-wide lints catch what narrow verification misses — realized instance 2026-07-22: the recursion-documentation dylint fired only under `gate:merge`, after scoped clippy and nextest were already green.

### Demonstrability lands with the feature

* **Surfaced language feature.** The implementing change includes runnable `gandr-corpus` model **and** pathological examples, harness assertions, and coverage-map registration ([corpus.md](corpus.md); `ADR-84`).
  These examples are landing evidence, never residual work.
  A syntax-first change includes its parse-gated `surface/` witness.
  The semantics-graduation change promotes that witness to runnable `model/`, adds runnable pathological coverage, harness assertions, and coverage-map registration in that same change.
* **Internal engine feature with no language surface.** The implementing change includes named runnable crate fixtures exercised by named crate tests or harness assertions and mirroring the intended future gandr programs.
  Its closeout residuals bead names those future programs and the exact surface blocker that enables promotion.
  The manual face says plainly that no user syntax exists yet; conformance-only support is not presented as a runnable feature.

A reviewer and a user must be able to answer “what can I run to see this work?” from the landing itself.
“The crate tests pass” is sufficient only when the named internal fixtures are exercised by those passing tests and the future program shape and promotion blocker are both recorded.

### One residuals bead, four faces

Closing work that **touches the language surface**, **adds or substantially refactors production Rust**, or **leaves any known residual** files or updates one consolidated residuals bead.
Use four explicit faces, marking a face `not applicable` rather than silently omitting it:

1. **MANUAL OBLIGATION.** Every language-surface addition or behavior change names the matching `docs/manual/` update.
   An internal-only feature instead records the absence of user syntax and what must land before the manual may present one.
2. **MUTATION CAMPAIGN.** New or substantially refactored production Rust names a scheduled standalone campaign over the commit range and intended scope ([mutation-adequacy.md](mutation-adequacy.md) §“Campaign lifecycle”).
   The residuals bead retains its required `discovered-from` provenance, but no campaign or residual creates a blocking dependency back to completed implementation.
3. **OTHER RESIDUALS AND FUTURE DIRECTIONS.** The consolidated residuals bead is the closeout index of record: record every known residual in this face.
   When one active topic/work bead already owns execution, link that one item instead of duplicating it; never rely only on a closed parent and never create additional residual siblings.
4. **CORPUS OBLIGATION.** For a surfaced feature, record the model/pathological witnesses, harness assertions, and coverage-map registration that already landed; this is closure evidence, not deferred example work.
   For an internal-only feature, record the named exercised crate fixtures, intended future corpus programs, and blocker whose completion makes promotion possible.

| Change shape                 | Manual face                               | Mutation face        | Other face                    | Corpus face                                    |
| ---------------------------- | ----------------------------------------- | -------------------- | ----------------------------- | ---------------------------------------------- |
| Surfaced language feature    | matching manual update                    | apply Rust trigger   | index every residual or `N/A` | landed runnable treatment                      |
| Internal engine feature      | “no user syntax” + promotion prerequisite | apply Rust trigger   | index every residual or `N/A` | exercised fixtures + future programs + blocker |
| Tooling-only production Rust | `N/A`                                     | campaign is required | index every residual or `N/A` | `N/A`                                          |

Where none of the three triggers applies, file nothing.

After dependency changes, regenerate the passive export as described above.

## Agent-attribution metadata

Bead closures carry agent attribution, stamped at close via `bd update --set-metadata`: `model=<model-id>`, `runner=<runner>`, `agent_tokens=<n>`.
Unstamped closures surface as "unattributed" in the metrics report (`mise run metrics:agents`, lands with the code tooling), so coverage gaps stay visible.
