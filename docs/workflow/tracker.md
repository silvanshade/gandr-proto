# Workflow: issue tracking (beads)

> Read when: creating, closing, or triaging beads; wiring dependencies; auditing tracker state.
> This file is the full statement of the tracker discipline.
> **Every tracker write carries your actor.** `bd create`/`update`/`comment`/`close` (and any other writing subcommand) take `--actor '<name>'`, where the name is your commit-trailer registry entry's name part, byte-for-byte from `.commitlintrc.mts`, with the address dropped — otherwise `bd` records the write under the owner's identity.
> Full mechanics and rationale: §"Agent-attribution metadata" below.
> `bd prime`'s hook output does not mention `--actor`; the obligation stands regardless.
> **Standing rule, whatever the task:** before recording that something does not apply, is not needed, or cannot be done, read [review.md](review.md) §"Declining is a claim too" and §"Refutations bind only with owner sign-off" — a refutation binds only with the owner's sign-off.

## Source of truth and sync

Issues live in **one shared Dolt database per machine** (beads, prefix `gandr-`): the primary checkout and every worktree resolve the same database through the git common directory, so worktrees carry **no gitignored `.beads` state of their own** (the `wt` copy-ignored step excludes `.beads/` and `.beads/**`, both forms, for the reason recorded beside that config; topology record: `gandr-fid.15`).
A bead written from any checkout is immediately visible in all of them — no pull between worktrees.
That immediacy is about **visibility**, and it does not extend to the remote: because every checkout advances the same branch, worktrees still contend with each other when pushing.
The database syncs **out-of-band from git** to DoltHub [`silvanshade/gandr-beads`](https://www.dolthub.com/repositories/silvanshade/gandr-beads) (the `origin` Dolt remote), the only off-machine copy:

- **push after every write** — `bd dolt push` immediately after `bd create`/`update`/`dep`/`close`;
- **pull before relying on reads** — `bd dolt pull` at session start and before triage (the `wt` `beads-pull` hooks and the `SessionStart` hook mechanize this); this guards **cross-machine** staleness, not worktree-to-worktree staleness;
- trust the remote over a local clone when they disagree;
- **expect a rejected push, and do not escalate** — see below;
- **read remotes with `bd dolt remote list`** — `bd dolt show` reports `Remotes: (none)` even when `origin` is configured and pushes are arriving (`gandr-mib`);
- **server lifecycle is owner-controlled** — agents never run `bd dolt stop`/`start` or restart the Dolt server unprompted: a mid-session kill is what forked per-worktree clones and double-recorded a closeout on 2026-07-22 (`gandr-fid.15`).

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

**`bd` resolves its workspace from the working directory, and a shell's working directory outlives the command that set it.** A `cd` into another project's checkout — to read a source tree, to check a prior project's tracker — silently retargets **every later `bd` call in that shell** at that project's database.
The symptom is not an error: it is `bd show <id>` reporting "no issue found" for a bead that plainly exists, which reads as data loss and invites exactly the wrong response.
**Before believing a bead has vanished, check the working directory**, then re-run from the intended checkout.
Disjoint id prefixes are what keep this from being worse than confusing: a `gandr-` id can never match in a `wyrd-` database, so a mistargeted write fails rather than landing somewhere unintended.
That is a property of the prefixes, not a safeguard anyone designed, so do not lean on it — pass an absolute `cd` in the same command as the `bd` call when a session has been reading another tree.

## Filing and lifetime policy (anti-drift)

The 2026-07-12 triage deleted ~600 of 845 beads; these rules exist so that never recurs.

- **A bead is a work item, not a memory.** File a bead only for work someone will plausibly execute within the current or next program of work.
  Insight belongs in an ADR (decision), the corpus (design), or the notes repo (contributor context) — never parked in the tracker.
- **Consolidation-first.** Filing what would be the third bead on one topic means the topic needs ONE topic bead instead — extend or replace; never accumulate siblings an agent must reassemble.
  Residual seams discovered during work go into the owning topic bead (a `bd comment` or a description line), not as new dangling beads, unless immediately schedulable.
- **Research expires.** A research question gets one bead whose deliverable is a decision (adopt/defer, recorded as an ADR); when the pass completes, the bead closes and residual leads go to the single project watchlist bead.
  A lead not promoted to a decision by the next triage sweep is deleted — it can be re-asked if it ever matters again.
- **Triage sweep cadence.** At each sweep (weekly obligation): active beads pruned toward ≤20; off-trajectory items older than a week deleted or folded into topic beads; deferred topic beads reviewed for reactivation.
- **Closed beads are never purged** (owner ruling, 2026-08-07, retiring the former purge-after-~2-weeks clause).
  Closeout comments, question/answer identifiers, and historical integration records are cited from documents and other beads, and a purge orphans every citation; the accumulation problem the purge aimed at belongs to the pruning of _open_ beads above.
- **Epics need children or a close date.** An epic with no active children is either done (close it) or a label (fold it into a topic bead).

## Conventions

- **Small graph, current graph.** The tracker is a working set, not an archive: ≲20 active beads, consolidated by topic.
  Prefer one compact bead that carries a topic's current state over several partial beads an agent must reassemble.
  Prune aggressively at triage; a dropped question can be re-asked later if it ever matters again (owner posture, 2026-07-12).
- **Reconstruction is the filing acid test.** A contributor must be able to reconstruct and execute the full task from the bead plus standing project guidance alone.
  Include the intended outcome, boundaries and non-goals, governing decisions and provenance, affected interfaces, dependencies, acceptance and verification evidence, and authoritative references needed to act safely.
  Do not depend on private notes, session history, branch state, or an unstated conversation.
  Point to stable tracked project records instead of duplicating them; if the contract cannot remain compact, use one topic bead with executable children rather than omitting context.

### Titles and metadata

- **Normalize titles as `<scope>: <subject>`.** Use one or more comma-delimited members of the closed `GANDR_SCOPES` vocabulary in `.commitlintrc.mts`; sort multiple scopes lexicographically, with no spaces between them.
  The bead type remains structured metadata and is not repeated in the title.
  Keep ordinary words lower-case, preserving case only inside a backtick-delimited literal whose spelling is case-sensitive.
  The subject is action-oriented and names the outcome, decision, or question—not implementation steps, status, dates, branch names, bead IDs, or provenance unless one is essential to distinguish the work.
  Titles have no trailing period, must fit the commitlint 100-character ceiling, and should usually fit within 72 characters.
- **Choose the precise work type.** Use `decision` for an explicit choice or ratified architecture record; `spike` for a timeboxed investigation that reduces uncertainty before commitment; `story` for a user-perspective capability; and `milestone` only for a zero-work completion marker over related issues.
  Use `epic`, `feature`, `bug`, `task`, or `chore` when their ordinary meanings fit better; do not force an underused type onto mismatched work.
- **Choose metadata before filing.** Before every `bd create`, run `bd label list-all` and `bd types list-all`, then select the most appropriate existing labels and work type.
  Do not invent a near-synonym for convenience.
  If nothing fits well—or the bead would not surface reliably in a targeted search—consult the user before adding a metadata category.
  With explicitly granted full autonomy, create a category when appropriate and report every metadata-category addition or change at closeout.
  When searching a label family, enumerate the matching labels by prefix and query every applicable exact label.
- **Retain full research citations in the bead.** For every relevant paper, record its title, authors, year, and the best-fitting unique locator—prefer a DOI, then an arXiv or HAL identifier, then a stable URL.
  Cite standards, repositories, issues, and other research sources with equivalent identifying detail and a resolvable locator; a corpus path or filename alone is not a research citation.
  Never leave the only citation in session context or a notes-repository report.
  Put references known at filing in the description.
  References discovered later are one of the two exceptions to the comment-only update rule (the other being a ledger bead, below): add them directly to the bead's standing body, normally in a compact `## References` section in `notes`, so the bead alone retains its canonical bibliography.

### The `ss-` identifier is the one permitted reference to the maintainer's private research context

A bead may cite an identifier with the `ss-` prefix: a work item in the maintainer's private research tracker, where much of this project's design context lives (owner ruling, 2026-08-06).
The identifier is deliberately the **only** permitted form of that reference: it locates precisely for the maintainer, it is opaque to everyone else — no path, no document name, no topology — and it is not relative to any machine, so it acts as a firewall between provenance and dependence.
Two boundaries keep it that way.
**Tracked content never carries it** — code, tests, documentation, and commit messages name no `ss-` identifiers and no other reference to that private context, and load-bearing design context is restated in this repository's own artifacts; a bead whose task cannot be reconstructed without resolving an `ss-` reference fails the reconstruction test above.
**It is provenance, never a dependency** — cite it for where a design came from, not in place of stating what the design is.

### Safe graph and field updates

- **Back up before graph-wide operations.** Before bulk triage, normalization, relabeling, dependency rewrites, or any other graph-wide mutation, create and sync a Dolt-native `bd backup` to a durable location outside the project tree.
  Verify the backup status and retain the backup until the operation is complete, synchronized, and verified safe to roll forward without it.
  Follow the end-to-end [beads graph sweep workflow](beads-graph-sweep.xml) for baseline capture, read-only classification, deterministic mutation, conservation checks, and reporting.
- **Progress additions are comments only, except references and ledger beads.** After filing, append progress, evidence, and closeout chronology with `bd comment`; never accumulate them by amending `notes`, `description`, `design`, or `acceptance_criteria`.
  Edit those standing fields only to correct the bead's authoritative current contract, to add and maintain its canonical research references, or to maintain a **ledger bead** as defined below.
- **A ledger bead is edited in place, and that is the point of it** (owner ruling, 2026-08-02).
  A ledger bead's substance is a **register whose current state is the deliverable** — one row per tracked thing, kept correct — rather than a task with a history.
  A register split across a comment stream is not a register: a reader would have to reassemble the present from a chronology, which is exactly the fragmentation it exists to prevent.
  So its `design` field is edited in place, and a change that affects a row updates that row in the same change.
  Comments on a ledger bead are for anything that is **not** a row.
- **What makes a bead a ledger bead, so this does not become a licence to amend anything.** It says so in its own description, it is a register rather than a task, and its rows are maintained by _other_ work rather than by progress on itself.
  The specification absorption ledger (`gandr-fid.11`) is the standing instance: one row per pre-reboot source, updated by whichever absorption touches that source.
  Everything else keeps the comment-only rule, and "this bead accumulates state" is not on its own a reason to claim the exception — most beads accumulate state, which is what comments are for.
- Beads cite corpus paths (`spec:…`) so an agent lands with context.
- Every doc-drift finding files a bead — drift produces work items, not silent warnings.
- Dependencies via `bd dep add <child> <parent>`; **after any dep change regenerate the passive export** (`bd export -o .beads/issues.jsonl`) so `bv` sees the edge — it reads the export, not Dolt.
  Trust `bd show`/`bd blocked` over `bv` when they disagree.
- `bd list` hides closed issues — use `--all` / `bd show <id>` when auditing done-ness.
  JSON listing commands paginate; pass `-n 0` (or use `bd export`) for complete sets.
- **Large text fields wedge the database** (owner-confirmed 2026-07-19; formerly a hazards-doc entry, now standing workflow guidance).
  Keep `notes`, `description`, `design`, and `acceptance_criteria` compact: standing directions plus a short pointer at most.
  Field updates REPLACE content and can clobber prior context; comments append safely.
- **Tracker text is never hard-wrapped** (owner ruling, 2026-08-02).
  `bd show` renders descriptions and comments inside its own fixed-width box and re-wraps every source line to that width, so text authored with hard line breaks at any other width double-wraps into stranded one-word fragments and becomes unreadable — the failure was observed across an entire bead's comment stream before the cause was found.
  Write one line per paragraph and one line per list item, however long, with blank lines between blocks, and let the renderer do all the wrapping; this applies equally to text passed inline and via `--file`, and worker briefs must carry the rule.
  A wide markdown table is unreadable in that box regardless of wrapping — in tracker text prefer one line per row (`row — verdict — grade` style) and keep real tables in corpus documents.

### Landings are named without mutable commit SHAs

**A local commit SHA is never the durable identity or proof of a landing in a bead.** This applies to standing fields, comments, close reasons, fold reports, handoffs, and review records.
A worker handoff records the repository, owning bead, source branch, ordered commit subjects, and verification.
After landing, the integrator records the destination ref, repeats that tuple, notes any subject rewrite, and says how to observe the result.
The branch may then be deleted; the record survives a rebase, re-sign, or history rewrite.

A SHA may appear only as an explicitly time-bounded operational handle or when the SHA itself is the forensic subject.
State whether it is pre-integration, post-integration, or a snapshot, and carry the durable identity beside it.
It never substitutes for target-ref reachability or an observable artifact.
Object lookup alone is not validation: a discarded pre-rebase object often remains readable until reflog expiry or garbage collection.
Any checker tests reachability from a live branch or tag, not merely whether the object resolves.

Existing comments are append-only.
Correct a historical hash map with a new comment when a reader would otherwise rely on it; do not rewrite the old comment or build prose-rewrite machinery around it.
Machine-owned tables may track old-to-new mappings only when the same mechanism updates and validates them.

## Decisions: act by default, escalate at the bright line

**An agent decides by default and keeps building.** Work never pauses on a choice this section does not send to the owner, and an agent that cannot decide asks whoever dispatched the work rather than filing anything.

This replaces a queue — a `decision` bead per topic, one question per comment, answered by comment — that ran here from 2026-08-02 and was retired by owner ruling on 2026-08-12.
Four things went wrong with it, and each was structural rather than a lapse: hosting process documentation in beads was the founding mistake; a choice-shaped contract manufactured volume, because anything can be posed as a question; the queue bottlenecked buildout, because work waited on a batch nobody had reached; and bead prose grew into walls unreadable by the one person who most needs to read them.

### The bright line

A choice reaches the owner when it is one of exactly three things.

| Escalate                                                                                                                 | The failure it closes                                                |
| ------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------- |
| **Expensive to reverse** — non-regenerable deletions, public-facing names, cross-repository contracts, schema migrations | the cost of being wrong falls on someone who cannot undo it          |
| **In conflict with an explicit prior owner ruling**, where the conflict survives the judgment below                      | an agent overturning a ruling by deciding again, quietly             |
| **A decline of something the owner explicitly asked for**                                                                | a request answered by not doing it, with the refusal never surfacing |

**Nothing else escalates.** Being unsure is not a fourth line, and neither is expensive-to-do, contested-between-agents, or interesting-to-ask.

**The conflict case inherits the first case's bar; it is not a line of its own.** An apparent conflict is judged before it is escalated: where the owner's recent directives point in a newer direction that plausibly overrides the older ruling, and following that direction is cheap to reverse, follow it and keep building, with the record naming the ruling it supersedes and what would restore it.
Escalate only when the conflict survives that judgment **and** resolving it either way is expensive to reverse.
Read without the qualification, this case is a hatch every surface disagreement fits through.

**A refutation is not exempt from the line — it sits on it.** A refutation is a decline in the strongest form this project admits, and [`review.md`](review.md) §"Refutations bind only with owner sign-off" is what routes it here.

### Everything else: choose, record, keep building

Choose by the project's choice ethic — the most principled option theoretically, the most interesting frontier-wise, or the most performant, ideally a novel synthesis of all three; **never the easiest for effort's sake alone**.

**One choice is one compact comment on the bead the work belongs to**, carrying three things and nothing else:

- the choice, stated as what is now true;
- the viable alternatives, named in a clause each;
- **the reversal condition** — what would trigger revisiting it.

**No retroactive questions.** A decision already taken is a record, so it is written as a record; filing the question it would have been is the manufactured volume this model exists to end.

### The rare escalation goes outboard

**The standing decision surface is held in the maintainer's private research workspace, not in this repository** (owner ruling, 2026-08-12).
One escalation is one concise entry there: the decision line, short options, **the recommendation first**, and context carried as links into real documentation rather than restated.
The surface is named by what it is rather than by a path, for the reason the `spec:` corpus is: a pointer into a tree that is not checked out here is not a reference this repository can hold.

**The order is transactional, and the order is the point.** The owner answers on that surface.
The agent executes the ruling.
The record lands in the owning bead **here**.
**Only then is the entry deleted** — so every stable identifier ends its life with exactly one durable home, and no ruling is ever in flight between two of them.

Reaching the surface is a crossing, not a tracked artifact: an agent working here raises the escalation through whoever dispatched the work, under `AGENTS.md` §"Dispatched work".
The permitted reference back is an `ss-` identifier cited in a bead, and nothing else.

**`human` marks a work bead carrying an open escalation.** It is rare by construction, it is not a queue, and it is never a container for questions.

### What binds however a ruling arrives

- **An agent never writes the owner's answer**, never records a ruling the owner did not give, and never reads silence as consent.
  An unanswered escalation stays unanswered; if the work cannot proceed without it, that is a blocked bead, not a licence to decide.
- **A ruling given in conversation is recorded the same way** — as a record in the owning bead, with no question filed first.
- **Record the ruling, not the option label.** An answer that selects an option often sharpens it in the same breath, and the sharpening is the load-bearing part: "option b" plus "and its value is X, not Y" changes what the work _is_, not merely when it happens.
  The option letter is provenance.
- **The ruling of record lands in the authoritative artifact** — the corpus document, the decision record, the code, or the owning bead's standing contract — never only in a comment stream.
  A comment may be cited; citing it never substitutes for the ruling's home.

### Reading the retired queues

The `decision` beads carrying `human` hold question and answer comments under the retired scheme, and those streams stay readable rather than being rewritten.

**The identifier is `<queue-bead-id>-question-NN`**, zero-padded, numbered within the hosting bead, answered by `-answer-NN`.
**The prefix is always the bead the comment _lives on_**, never the bead it concerns — which is what makes `gandr-fid.14.7-question-03` self-locating for a reader meeting it in a commit message or another bead.
Numbering was stable and stays stable: a retired number is left unused rather than renumbering the rest.

Nothing new is filed under the scheme.

## Feature landing and residual closeout

The base rule still governs: close a bead only when its full recorded scope is done and verified; make residual scope epic-shaped; file follow-ups `discovered-from` the parent; sweep related memory before closing. gandr adds one **feature-landing workflow** so executable evidence, manual work, mutation campaigns, and other residuals cannot drift into separate conventions.

**The merge gate is part of "done and verified"**: before a task is considered finished — and before its residuals bead is filed — the work must pass `mise run gate:merge` in the landing worktree (the same gate `wt merge` runs pre-merge).
Passing narrower gates (build, scoped clippy, nextest) is not sufficient: the merge gate is where project-wide lints catch what narrow verification misses — realized instance 2026-07-22: the recursion-documentation dylint fired only under `gate:merge`, after scoped clippy and nextest were already green.

### Demonstrability lands with the feature

- **Surfaced language feature.** The implementing change includes runnable `gandr-corpus` model **and** pathological examples, harness assertions, and coverage-map registration ([corpus.md](corpus.md)).
  These examples are landing evidence, never residual work.
  A syntax-first change includes its parse-gated `surface/` witness.
  The semantics-graduation change promotes that witness to runnable `model/`, adds runnable pathological coverage, harness assertions, and coverage-map registration in that same change.
- **Internal engine feature with no language surface.** The implementing change includes named runnable crate fixtures exercised by named crate tests or harness assertions and mirroring the intended future gandr programs.
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

### An agent acts under its commit-trailer name, on every tracker write

**Binding, no exceptions.** The name an agent signs commits with is the name it files, comments, updates, and closes under.
`bd` takes it as `--actor`, a global flag on every subcommand:

```sh
bd create … --actor 'Claude Opus 5 (1M context)'
bd comment <id> --file … --actor 'Claude Opus 5 (1M context)'
bd close <id> --reason … --actor 'Claude Opus 5 (1M context)'
```

**The string is the canonical trailer's name part, byte-for-byte, with the address dropped** — `Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>` gives the actor `Claude Opus 5 (1M context)`.
The registry in `.commitlintrc.mts` is authoritative for the spelling, exactly as it is for commits; a shortened, harness-supplied, or session-specific variant is a different agent as far as the record is concerned, which is the whole failure this rule closes.

**Why it is required rather than nice.** Without `--actor`, `bd` falls back to `$BEADS_ACTOR`, then `git user.name`, then `$USER` — so every agent writes under the **owner's** name, and the tracker records the one identity that is certainly wrong.
Comment streams and audit trails then read as if the owner asked the questions, made the corrections, and closed the beads.
Several agents share this repository and this tracker at once — the reason a rejected push is contention rather than breakage, above — so "who wrote this" is a live question during a session and not merely an archival one.

**Do not set `BEADS_ACTOR` globally.** A machine-wide value pins one model name across every session and every model on that machine, which reintroduces the same wrong-identity failure one level up.
Pass `--actor` per invocation, or let a per-session harness set the variable if one can.

**Two things this is not.** It is not `--assignee`, which records who will _do_ the work and is normally the owner or unset.
And it is not retroactive: the audit trail is append-only, so writes already made under the fallback name stay as they are, and a correction goes in a comment rather than in a rewrite.

### Closure metadata

Bead closures also carry agent attribution as structured metadata, stamped at close via `bd update --set-metadata`: `model=<model-id>`, `runner=<runner>`, `agent_tokens=<n>`.
A metrics report over these stamps is planned with the code tooling and does not exist yet; until it lands, the stamps are the record and nothing surfaces the gaps.
The actor rule above and this metadata answer different questions — _who acted_ against _what ran_ — and both are recorded.
