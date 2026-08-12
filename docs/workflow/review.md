# Workflow: adversarial review and research verdicts

> Read when: preparing a substantial change for landing, running or interpreting an adversarial pass, closing a research-question task — **or before recording that anything does not apply, is not needed, or cannot be done.** That last trigger is not scoped to review work: §"Declining is a claim too" and §"Refutations bind only with owner sign-off" are standing rules for every task in this tree, and a refutation binds only with the owner's sign-off.

## When and how

The gates catch **structural** faults (formatting, hash drift, dangling references), not **semantic** ones — a change can pass every gate while quietly distorting a claim or missing an instance nobody thought to grep for.
So any substantial or publishable-stakes change (an absorption, a multi-document edit, a curation pass, anything touching the corpus) gets an **independent adversarial review before landing/pushing and before bead closeout**: An immutable checkpoint may exist solely as isolated-review input, but it is not landed or pushed.

- **Independent** — a reviewer separate from the author, given the changed files (and the source they derive from), not the author's rationale.
- **Adversarial** — prompted to find faults: "what is wrong, missing, or distorted here?"
- **Multi-lens** — distinct lenses for distinct failure modes: fidelity to source, policy/leakage compliance, cross-reference integrity, fresh-reader coherence, context-economy ([docs.md](docs.md)).
- **Demonstrability** — apply the full canonical checklist in [corpus.md](corpus.md): surfaced means same-change runnable model and pathological examples, harness assertions, and coverage-map registration; internal-only means named runnable crate fixtures exercised by named crate tests/harness assertions, intended future programs, and the exact corpus-promotion blocker.
  The manual must not present conformance-only support as user syntax.

Scale to the change: a one-line fix needs only the gates; a cross-cutting corpus pass earns several lenses.
Triage findings like drift findings: fix should-fix in the same change, file the rest as beads — never silent; surviving findings are residuals, folded into the consolidated closeout bead ([tracker.md](tracker.md) §“Feature landing and residual closeout”).

**Isolation.** Mutating review agents run only through the Worktrunk-owned lane ([worktrees.md](worktrees.md) §"Mutating sub-agents") — a Bash-capable agent sharing the live tree has corrupted uncommitted work before.
An immutable checkpoint commit is the normal input to an isolated reviewer; a strictly read-only reviewer may inspect uncommitted state.
For governance docs already on `main`, reviewers stay read-only and the orchestrator applies fixes on `main` before push.

## Documentation fidelity review

Documentation is the one artifact class with **no natural adversary for omission**: a dropped function fails to compile and a dropped case fails a test, but a dropped paragraph fails nothing — `docs:check` validates structure (IDs, terms, cites), so a component that silently sheds half its source's implementation-grade content passes every gate.
Omission is invisible in the artifact itself; it is only visible in a diff against the sources.
Accordingly (owner decision `gandr-fid.0`, 2026-07-21), **every change to `docs/gandr/spec/` gets a two-axis adversarial review**, not just substantial ones:

1. **Correctness axis** — the standard lenses above: are the claims that appear accurate, cited, current?
2. **Fidelity axis** — the reviewer receives the change's **declared source set** (the wyrd files, research sweep, ledger entries, or session decisions it draws from) and adversarially hunts for what was dropped, compressed, or de-linked, stanced as "prove that load-bearing detail was lost."

The fidelity instrument is the **content-class inventory**: for each class — decision/summary tables; grammars; typing rules; type/code signatures; architecture (crate/module homes); algorithms; staging plans with gates; corpus-example plans; open questions; dependency tables; precise citations (theorem numbers, section anchors) — record retained / compressed-lossy / dropped against the source.
**Gate: zero dropped load-bearing classes.** Compression is acceptable only when the compressed form still lets an implementer proceed without the source; when density is the problem, spread out, explain, and link — never drop ([docs.md](docs.md) §"Documentation economy").

Preconditions and boundaries:

- **A doc change with no declared source set cannot be fidelity-reviewed** — declaring sources is part of authoring, not an optional courtesy.
  For net-new components the commissioning bead names the sources.
  The research corpus that used to serve as the absorption source-set registry has left this repository, so absorption work declares its sources in the change itself or in the bead that commissioned it — the obligation to declare them is what mattered, and it did not leave with the directory.
- Reboot-era operating notes stating "no adversarial reviewers" (the PLAN-assembly posture) **do not apply to documentation authoring**; this section supersedes them for that class (`gandr-fid.0`).
- The merge discipline for re-absorption: reboot truth wins on status and naming; the source wins on payload the reboot copy dropped.

## Absorption and reboot passes

Migrations and reboots are the highest-volume absorption work there is, and the 2026-07-31 spec-reboot review measured their characteristic failures: one open question settled by assertion, a red gate never run, works cited with no entries, entries never cited, and a long tail of silently dropped detail.
The rules below are what a migration owes beyond the per-document procedure of [specs.md](specs.md).

- **The disposition ledger.** Every open item in a source — an open question, spike, obligation, falsifier, pending read — gets exactly one disposition, recorded where a reader meets it: **carried**; **declined with a reversal condition**; **parked with a reason**; or **retired with a tombstone saying why**.
  An item that vanishes without a disposition is a defect; omission is invisible in the artifact and only visible against the source.
- **A refutation needs the same sign-off in a migration as anywhere else.** A settlement claim for something the source left open ("its consumers no longer spend it") is a refutation; it binds only with owner sign-off, and until then it is recorded as declined with its reversal condition.
  The test is the standing one: is the reason a fact about the machinery, or a fact about us?
- **Registration is part of authoring.** An unregistered corpus document is a fatal drift-gate finding, so "whether to register" is not a decision the author may leave open.
  When authoring directly on `main`, run the docs gates before committing — the pre-commit hook does not watch documentation paths.
- **References are payload.** Every literature claim carries a key at first mention; every key resolves; the bibliography holds no entry the corpus never cites and no cited work lacks an entry.
  An unnamed work ("a published mechanization", "the leading implementation") is named, or the claim is marked locator-pending at the claim.
- **As-built claims are verified against the tree at write time**, with the module or symbol named.
  "Verified against the crate" without the verification is a finding, and counts are stated with their counting convention.
- **Record, per source, which part was read.** A held, cited source can still have its headline theorem unconsumed; the failure has recurred.
- **Persist the working reports.** Scout and inventory reports a fold rests on are preserved with the migration log; a disposition table without its reports is not auditable, and the next sweep must not have to re-derive them.
- **State the pass structure up front.** If a first pass will be re-swept, the log says so where it reports the first pass's folds; "folded" and "swept" must never be ambiguous.
- **Clarification is a separate pass, and it lands.** Fidelity asks whether anything is dropped, mis-stated, or unsupported; clarification re-reads for confusion — claims true but reading as their opposite, terms before definition, cryptic compressions, misleading attributions — and fixes in place.

## Interpreting findings — challenged, not refuted

Adversarial findings are **inputs, not verdicts**, and two kinds bind differently:

- **Binding** — a _fabrication_, a _factual/citation error_, or a genuine _category error_: fixed before the reviewed checkpoint lands/pushes and before bead closeout, not negotiable.
- **Challenged, not refuted** — a _strategic, feasibility, or redundancy_ judgement: recorded as **challenged** — de-emphasised in the search space, never dismissed as refuted/dead.
  As the design converges, genuinely-unhelpful negatives self-filter; dismissing early risks discarding a branch a later reframing would revive.

The adversary is never the final say.
Even a literature impossibility proof gets its applicability checked (does its hypothesis hold for _our_ object?) before binding.
When reporting, present challenges as things to engineer around, bracketed — never leading with the adversarial conclusion as settled.

**Every adversary pass writes a human-inspectable report artifact** — reasoning, citations, severity, each finding's binding-vs-challenged disposition — kept in the reviewing contributor's private workspace outside this tree, citable from the owning bead — for the maintainer, as an `ss-` identifier ([tracker.md](tracker.md)); the deliverable cites it briefly, never inlines it.
The point is auditability: a later reader sees _why_ a branch was challenged.

## Declining is a claim too — the counterfactual test

Declines that later had to be overridden have taken **two shapes**, not one.
Both are fluent, both name something true, and both quietly exercise the owner's authority without the owner seeing the option.

### Shape 1 — the static decline: "doesn't apply, because our setting is X"

The previous section says an impossibility result gets its applicability checked before it binds.
That check is **not enough on its own**, because it is usually run statically: _does the hypothesis hold for our object, as our object currently stands?_ That question holds fixed the thing most likely to be wrong.
In a design still being built the setting is a variable too — and usually the cheaper one to move — so the static question is under-specified and its default resolution is biased toward rejection.

**Adversarial stance amplifies this rather than catching it.** An adversary's success condition is finding a defeater, and under a static reading a mismatch with the current setting _is_ one.
The remedy is **not** to soften the adversarial pass; uncritical acceptance is the worse failure and fills the tree with machinery that never carries weight.
The remedy is to make the **decline** an adversarial target in its own right: having found the mismatch, attack your own rejection.

**The tell, which is checkable by reading.** A decline whose stated reason is a fact about _us_ rather than a fact about _the machinery_ — "doesn't apply because our setting is X", "the wrong theory for our case", "that's just what our foundational commitment costs".
Each names one of our properties; none asks whether that property should hold.
When a rejection's reason is load-bearing vocabulary from our own design, it reads as settled and stops being questioned — which is exactly when it needs to be.

### Shape 2 — the solution-weighted decline: "yes, we could, but…"

The more common one, and harder to spot because it _concedes_ applicability and then loses on cost.
It arises whenever the analysis is framed **solution-first** — here is a candidate change, now evaluate it — because that framing is structurally rigged:

- costs are **concrete, enumerable and immediate** (migration, churn, risk, the unknowns any systematic change drags in);
- benefits are **diffuse, speculative and deferred**;
- and the evaluator is usually mid-task, so the change reads as an _interruption_ whose cost they bear personally while the benefit accrues to someone later.

Every one of those pushes the same way.
A cost/benefit run at the moment of encounter is therefore not a neutral instrument, and "yes, but" is what it outputs by default.

**The remedy is an opportunist lead**, which is a change to what the work _produces_, not merely to its tone.
When reviewing any other technical system, the point is **extraction**: what can be learned here, and could our current approach be improved by knowing it?
Report it that way round — _"while researching X I found a potential improvement to Y, worth considering under these conditions"_ — and only then run the cost/benefit.

This matters for a reason beyond framing.
**A decline made by the finder is invisible; a decline made by the owner is a decision.** Both shapes have the finder silently deciding, which is why the errors are undetectable.
An opportunist lead does not make declines rarer — it makes them the owner's, and it leaves an inventory behind that survives the decline and is reusable when the surrounding design moves.

### Calibrate to the artifact class — this tree is speculative

The conservative register is the **default professional one**, so it arrives unexamined and needs disarming explicitly:

- **A speculative research artifact** — this tree — has no shipped consumers, high option value, and cheap recoverable errors.
  Surface aggressively; the bar for _raising_ an opportunity is near zero, and "yes, but" needs to survive the questions below before it binds.
- **A production artifact with real stakeholders** has genuine switching costs, and stability is itself a feature.
  There "yes, but" is frequently the correct answer.

Do not import the second posture into the first by habit.

### Before a decline binds, answer four questions

Not "does this apply", which is static, but:

1. **_Should_ it apply?** If our setting were free to be whatever it ideally is, is this the right fit?
   Ask this **first**, before compatibility.
   Weakest of the four and the one that rots into a rubber stamp — nearly anything sounds like it should apply in the abstract — so it opens the analysis and never closes it.
2. **What exact delta would make it apply?** Name it concretely.
   A decline with no nameable delta is an intuition, not a finding; and symmetrically, an _acceptance_ whose delta nobody has costed is optimism.
3. **What does that delta cost, and what kind of change is it?** Distinguish a **representation** change — usually cheap, especially early — from a **commitment** change, which is expensive.
   _A decline that rests on a representation choice is not a decline._ Questions 2 and 3 carry the weight; if the output is "yes, and we would have to change everything", that is a sign this one was answered by vibe rather than named.
4. **What would it unlock — or eliminate?** Both directions.
   Retiring machinery is a payoff too, and often the larger one — and it is the benefit most often missed, because eliminations do not present themselves as features.

**Why the test tilts this way: the two errors are not symmetric.** A wrong acceptance is loud and self-limiting — the machinery fails to carry weight and you find out.
A wrong rejection is silent and compounding — it produces no signal at any later point, and the design ossifies around the gap.
Rejection errors are undetectable by construction, so they have to be made expensive on purpose.

**Record the delta, not just the verdict.** "Challenged, not refuted" only works if a later sweep can re-open a branch cheaply instead of re-deriving it from scratch.
The delta and its cost are what make that possible, and they belong in the adversary report artifact alongside the finding.

## Refutations bind only with owner sign-off

A decline sets something aside; a **refutation** closes it — "does not apply", "is not needed", "cannot be done", "is the wrong structure", "is a category error".
That is the strongest claim available here, and it is the most expensive one this project has made.

**The evidence is a pattern, not an anecdote.** The machinery that turned out to be load-bearing — virtual double categories, the tracelet algebra, the circuit algebra — was in each case ruled out or passed over first, and recovered only after owner pushback and re-analysis.
None of those recoveries needed new information.
Each needed one unexamined premise unpacked.
So the failure mode is not ignorance; it is that a refutation, once written down fluently, stops being read as a claim at all.

Three rules follow, and the third is the instrument.

### 1. An agent proposes a refutation; it never lands one

Until the owner signs off, the finding is recorded as **challenged** with its delta (§"Before a decline binds") — never as refuted, dead, or ruled out, in the deliverable, the tracker and the design record alike.
Sign-off is a decision the owner makes, so the report must give them something to decide on: state what would have to be true for the answer to flip, not only why it is currently no.

### 2. Skepticism scales with how established the target is

Refuting something the tree has already established — a landed finding, a documented decision, a named structure, a source previously judged relevant — carries a **heavier** burden than declining something novel, not a lighter one.
The intuition runs the other way ("we already looked at this"), which is exactly why it needs saying.
**The burden is on the refutation.** An established claim is not disturbed by an argument that merely sounds tidier than it does.

### 3. Enumerate the premises, and tag each one

A refutation is only as strong as its weakest premise, and the recurring failure is that nobody lists them.
So list them, and tag each:

| the premise is…                                                                                       | tag         | what it can carry                                    |
| ----------------------------------------------------------------------------------------------------- | ----------- | ---------------------------------------------------- |
| a fact about **the machinery** — a theorem's hypothesis, a definition's shape, a published result     | `machinery` | a refutation                                         |
| a fact about **us** — our current representation, presentation, naming, ambient, or design commitment | `ours`      | **nothing.** It makes this a representation question |

**Gate: a refutation resting on any `ours` premise is not a refutation.** Report it as the opportunist lead it actually is — _"this would apply if we changed X"_ — and hand the decision over.
This is the four-questions test made mechanical, and unlike the questions it is checkable by a reader who was not present for the reasoning.

**Two worked instances, both from 2026-07-28, both caught by the owner rather than the author.** "The pair is not duoidal — grafting is composition, not a tensor" rested on _the objects are the profiles_, which is `ours`.
"The dimension-wise certification cannot be the reasoning layer's hypothesis" rested on _every `Set`-level structure presents through the discrete setoid on the identity type_, which is `ours`.
Both read as facts about the machinery; both were facts about a representation we chose and are free to change.

### When the adversarial pass is required, and what it must be given

Not every turn and not every change — that dilutes it to a ritual.
It fires on:

- **closeout, before handoff** — the consolidated pass over what the session is about to leave behind;
- **every reversal** of a landed finding, decision, or characterization;
- **every first-time characterization claim** — "X **is** a Y" — because a name is a claim (`docs/gandr/spec/proof-engineering.md` §"Terminology follows the ladder, and a name may not assert an unchecked correspondence"), and a naming claim fails in precisely the way a reversal does.
  A reversal-only trigger misses these: one of the two instances above was a naming claim, not a reversal.

**Ask the owner before running one.** The pass costs real budget, and the owner may already know the answer.
That consent rule is this pass's alone — the landing review of §"When and how" fires on its own trigger and needs no asking; the two are different instruments, not one rule stated twice.

**Give the reviewer the code and the primary source — never the author's write-up.** This is the "independent" bullet above, and it is not a formality: both instances above are invisible in the author's summary and visible in the Agda signatures plus one section of the source.
A reviewer handed the rationale ratifies its frame, and the frame is what was wrong.

**One standing lens, cheap enough to run without an agent:** _is this a fact about us, or a fact about the machinery?_ It fires on both instances above and on all three historical ones.

## Where a sign-off request goes

This document generates most of the project's owner decisions — a refutation needs sign-off, a decline needs the owner to see the option, an adversarial pass needs asking before it runs.
None of them is posed as a compressed inline batch in chat.

Every one goes on the **owner-decision queue**, which lives on the tracker: a queue bead per topic, one comment per question, answered by comment.
The mechanism, the identifier rule, and the closeout discipline are [`tracker.md`](tracker.md) §"The owner-decision queue"; the binding statement is `AGENTS.md` §"The owner-decision queue".

## A recalled citation that turns out to be on-target is still unverified

The corpus discipline says to verify a source's **identity**, not merely its presence.
This section is the case that discipline is weakest against, because it does not look like a failure at all.

**The shape.** A citation is produced from recall rather than from a bibliography — author, title, sometimes a venue, all stated with the fluency of something read.
Later the paper is obtained, and it is **real and on-subject**.
The referent was right; the citation was invented.

**Why that is the dangerous case rather than the lucky one.** Being on-target reads as confirmation, so the entry is quietly promoted from recalled to checked without anyone checking anything, and the wrong author, title or year rides downstream on the strength of the subject matching.
A citation that pointed at nothing would have been caught the moment someone looked for it.

**The tell, and it is checkable.** A citation transcribed from a real bibliography preserves the authorship, because the author list is the part you copy.
**Recall gets the topic right and the metadata wrong** — a wrong title _and_ a wrong author count on a paper that nonetheless exists is the signature, and it means recall, not a transcription slip.

Three rules follow:

- **A not-held list cannot come from a filename sweep**, since you cannot sweep for what is absent.
  Whenever one is written, say where it came from — a named source's bibliography, or recall.
  An unattributed list of things to obtain is recall until proven otherwise.
- **Obtaining the paper verifies the paper, never the citation.** Read the title page and correct the entry in place; do not let "it turned out to exist" stand in for having checked it.
- **Do not manufacture the provenance afterwards.** If the source of an association is unknown, record it as unknown.
  Reconstructing a plausible origin for a citation is the same failure one level up, and it is harder to catch because it explains the evidence.

Worked instance, 2026-07-28: a duoidal-bibliography entry recorded from recall as a solo-authored paper under a title matching nothing.
The paper it pointed at is real, on-subject and now held — with three authors and a different title.
It was cited in neither of the two held sources most likely to carry it, one of which predates it outright.

## Research-question tasks — deliver the outlook

A task investigating a research question delivers, alongside findings, a brief **outlook**: **evidence for** vs **evidence against** as short bullets, plus a one-line **Net** (decisive, not hedged).
The adversary artifact checks claims against sources, not feasibility — the outlook is stated separately.
Distinguish "the framing is solid" from "the specific vehicle is high-risk"; separate what is established from what is open/conjectural; name the most-likely dead-ends explicitly.
