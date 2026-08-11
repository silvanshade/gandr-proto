# Workflow: authoring the specification corpus

> Read when: creating, editing, migrating, or re-absorbing any document in `docs/gandr/spec/`; the posture also governs `docs/research/` records.
> Base practice: `docs/gandr/spec/README.md` (the corpus's own conventions), [review.md](review.md) §"Documentation fidelity review" (the mandatory review) and §"Absorption and reboot passes" (the migration discipline), [docs.md](docs.md) (the economy posture).
> **Standing rule, whatever the task:** before recording that something does not apply, is not needed, or cannot be done, read [review.md](review.md) §"Declining is a claim too" and §"Refutations bind only with owner sign-off" — a refutation binds only with the owner's sign-off.

## The corpus is migrating out — no new specification documents

**Owner direction, 2026-08-07, bannered in `docs/gandr/spec/README.md`: do not add new specification documents to this corpus, and prefer editing over growing what is here.** The corpus's deep structure — design, reasoning, open questions, specifications in motion — is moving to the maintainer's research workspace, which is now its primary home; this repository keeps code, tests, and outward-facing user documentation.
What this repository still receives from a design is a **thin decision record** stating the outcome and what it binds — never a new corpus document.
Dispatched work follows the same rule by default: a dispatch into this repository builds.
Everything below continues to govern edits to the existing corpus while the migration runs.

## The priority order

1. **Technical precision and exhaustive detail** — implementation details, normative signatures, proof obligations, theorem numbers, gate conditions — are the single most important content to retain across these documents.
2. Natural prose and conversational tone are an aspiration beyond that.
   When the two axes conflict, the first wins, always.

## The absorption bar

Absorbing a source document — a pre-reboot spec, a research sweep, a session decision record — is **superset-transfer, never summarization** (owner decision `gandr-fid.0`): the source is the floor, not the ceiling.
The acceptance test: **an implementing agent must be able to build the component from the corpus document alone, without opening the source tree.** Every load-bearing artifact lives in a payload block, and every payload block gets an introducing prose paragraph.

## The register is timeless

Corpus prose is written for the technical manual the corpus becomes: it says what the design **is and does**, forward-looking, never how the design was reached.
Absorption is not a chronicle — superseded verdicts, “what changed” narratives, and session archaeology (who ran which pass, when, on what steer) are decision-record and bead content, and they live there, not in the component.
Load-bearing rationale is unaffected: the reasons a decision binds stay inlined in current terms, as they always have.
What leaves is the retrospective _framing_ — “earlier passes filed this as X”, “two things changed since”, “this component absorbs the Y session” — replaced by the positive statement the framing was carrying.
Status carries the time dimension: the roadmap documents sequence the future; the guards and hazard documents carry the superseded claims that must stay visibly superseded; everything else states the present.
Provenance that is itself content-honesty — a claim resting on a transcript, a delegated close-read, or an unverified locator — stays marked at the claim, because that is about the source, not the session.
Decision and rung tags are semantic names, section anchors, or tracker ids — never letter serials (`M1`, `F3.19`) in titles or prose.
The test: a reader meeting the corpus as a manual learns the design without learning its biography.

## The per-document procedure

1. **Declare the source set before writing.** Absorption or migration: the source files, named in the migration log that rides the work.
   Net-new: the commissioning bead plus the realized code the document must stay faithful to.
   A change with no declared source set cannot be fidelity-reviewed, and therefore cannot land.
2. **Read the whole source set; build the content-class inventory.** For each class — decision/summary tables; grammars; typing rules; type/code signatures; architecture (crate/module homes); algorithms; staging plans with gates; corpus-example plans; open questions; dependency tables; precise citations (theorem numbers, section anchors) — record what exists and where it will live in the corpus.
   Omission is invisible in the artifact; the inventory is how the author sees it before the reviewer has to.
3. **Write to the corpus's structure** — the four tracks, one main document plus focused sub-documents and a roadmap per track — in the two-register weave: payload lives in tables, grammars, code, and registers; prose airs out (one load-bearing idea per paragraph, emphasis on the spine).
   Density is answered by spreading out, explaining, and linking — never by dropping.
   Re-absorption is a **merge**: reboot truth wins on status and naming; the source wins on payload.
   **Every open item in a source — an open question, spike, obligation, falsifier, pending read — gets exactly one disposition, recorded where a reader meets it: carried; declined with a reversal condition; parked with a reason; or retired with a tombstone saying why.** A settlement claim for something the source left open is a refutation and follows the standing rule above.
   Record numbers stay banned; their load-bearing rationale is inlined restated in current terms.
4. **Code and signature discipline.** Quote code **byte-exact** against its home — the Agda module or the crate, named at the block (visibility, attributes, field names and types; mark excerpts as excerpts).
   A "quoted" block that silently drops a keyword is a factual error, not an excerpt convention.
   When naming fixtures, name where the test actually lives.
5. **References and citations.** Every external work is cited by key from `docs/gandr/spec/bibliography.yml` at first mention; every key resolves; the bibliography holds no entry the corpus never cites and no cited work lacks an entry.
   Claims resting on an unverified locator say so at the claim.
   Decisions, commitments, obligations, and findings are referred to by meaningful names and linked anchors, never by bare codes whose referent lives in a retired document.
6. **As-built claims are verified against the tree at write time**, with the module or symbol named at the claim.
   "Verified against the crate" without the verification performed is a defect; counts (files, commands, members) are stated with their counting convention or their path.
7. **Gates: `mise run docs:manifest-drift` and `mise run docs:reference-integrity` green, `treefmt` clean.** Registered documents: the manifest hash rides the same commit — an unregistered corpus document is a fatal drift-gate finding, so **registration is part of authoring, not a later decision**.
   When authoring directly on `main`, run the docs gates before committing; the pre-commit hook does not watch documentation paths.
8. **Mandatory two-axis review** ([review.md](review.md) §"Documentation fidelity review"): an independent read-only reviewer, given the changed files and the declared source set (not the author's rationale), stanced adversarially ("prove load-bearing detail was lost"), recording the per-class retained/compressed/dropped inventory.
   Gate: zero dropped load-bearing classes.
   Binding findings are fixed before landing; the artifact goes to the notes `adversary/` register; the commit message cites the review.
   **Scope (owner ruling, 2026-08-09, `gandr-rhu0-answer-01`): the two-axis review binds on authoring passes — absorption, migration, net-new, re-absorption.** A **claims repair** — correcting a status, symbol, or spelling the tree refutes, with the tree as the declared evidence — instead takes an **independent adversarial verification per item**: after writing, re-derive each corrected claim from the tree as if reviewing a stranger's change, and record what was checked in the landing's report.
   The stance earned the ruling by catching a live code defect on the first repair it ran against.
9. **Commit per arc** (`docs(spec): …`), classified publishable (no machine-local paths, no session forensics), with the canonical co-author trailer.
10. **Tracker and ledger.** Bead notes updated and pushed; the migration log rides the same arc.
11. **Report.** A per-document completion report carries the deliverables _and_ the process findings — workflow-guidance gaps, tool problems, lessons — so the guidance improves with every document.

## The clarification pass

Fidelity and clarity are different properties and are checked in different passes.
The fidelity pass asks: is anything dropped, mis-stated, or unsupported?
The clarification pass re-reads for confusion, and fixes in place:

- **claims that are true but read as their opposite** — a title that names the programme, not the property; a "not X" where the emphasis lands on X;
- **terms used before definition**, and jargon with no gloss on first use;
- **unnamed works** — "a published mechanization", "the leading implementation" — which must be named or the claim marked locator-pending;
- **cryptic compressions** — "paid in three currencies" without the three currencies named, "the opposite direction" without the direction stated;
- **attributions that mislead** — a citation whose displayed claim is not what the cited work says.

A confusion found is fixed where it stands, never logged for later.

## The loss signature (what the audits measured)

The 2026-07-21 audits measured ~50–70% retention on first-pass absorptions; the recurring drops: decision tables prosified; type/trait/enum signatures dropped; crate/module layouts dropped; corpus-example plans dropped; open-questions sections dropped; decision-trail rationale erased along with the (correctly banned) record numbers.
The 2026-07-31 reboot review added the migration-shaped instances of the same signature: **open items vanishing without a disposition** (an open question of the source reappearing nowhere, or reappearing as a settled claim); **works referenced with no bibliography entry, and entries the corpus never cites**; **"verified against the crate" claims that were never verified**; **spike tables referencing named spikes that do not exist**; and **a red gate nobody ran** — the manifest registration left as "a deliberate decision to take" while the gate treated the state as fatal.
The counter is the procedure above: dispositions, the content-class inventory, the reference discipline, and the gates.

## The prose-pacing doctrine

Prose in the corpus is **aired out**: one load-bearing idea per paragraph, with paragraph breaks taken freely.
**Bold and italics mark the load-bearing ideas** — a reader skimming only the emphasized text recovers the document's spine; that is what makes the structure scannable.
Density belongs in payload blocks (tables, grammars, code, registers); prose carries one idea at a time.
A paragraph that needs two load-bearing ideas gets split; a paragraph whose idea needs emphasis gets it; emphasis is the concept marker, so roughly one emphasized idea per paragraph is the shape of the target, not a quota.

### Identifiers are informative, prefixed, and linkable — never a bare letter and a number

**No uninformative single-letter naming schemes.** Anything referenced anywhere that matters — a spike, an open question, a stage, an obligation, a decision — gets an identifier whose prefix abbreviates what it _is_ or the topic it belongs to, followed by a zero-padded number: `meta-spike-04`, `meta-question-19`.
A bare `S1` or `P1` names nothing, so a reader who meets it outside its home document cannot tell what it refers to, or even which document to open.

**This is not hypothetical, and the corpus has already paid for it.** `S1` meant two unrelated things at once: a spike in the metatheory roadmap and "the trusted S1 core" in the implementation track — the second of which was never defined anywhere, in three uses.
Collisions like that are invisible until someone cites one and means the other.

Three rules follow, and the third is what makes the first two worth the edit:

- **Prefix by topic, number by position.** The prefix is the disambiguator; the number is only an index within it.
  Numbering is **stable** — retiring an item leaves its number unused rather than renumbering the rest, because renumbering silently invalidates every reference taken before it.
- **Give the identifier an anchor.** An identifier nobody can link to is a search string.
  Prefer a heading per item (`### meta-spike-04`), which Obsidian resolves as `[[metatheory/roadmap#meta-spike-04]]`; a table cannot be linked into row by row, so a list of items that get cited individually should not be a table.
  Where a heading per item is too heavy, lead the item with the bolded identifier and link to the section.
- **Cite by link, not by code.** `[[roadmap#Open questions|meta-question-19]]` survives a document being reorganised and tells the reader where to go; `open question 19` does neither.

Retired schemes are the exception and stay exactly as they were: the concordance in the guards ledger exists to decode old notes, so the codes in its left-hand column are data, not usage.

### A period is a sentence end, so do not spend one on an abbreviation

`rumdl` reflows corpus prose with `reflow-mode = "semantic-line-breaks"`, which puts one sentence per line by splitting at a period followed by a space.
**It cannot know that an abbreviating period is not a sentence end, and it should not have to** — the ambiguity is in the prose, not in the tool.
So the abbreviating period is what gives way.

The symptom is worth knowing because it does not point at its cause: the text is fine as written, `treefmt` reflows it, and what you notice afterwards is a line shattered mid-clause with the fragment stranded below.
The fix belongs in the sentence, never in a formatter exemption.

**The trigger is narrower than "any abbreviation", and knowing which one saves rewriting prose that was never at risk.** Measured against the pinned `rumdl`: a split needs a period, a space, and then a **capital letter**.

| written         | reflowed | why                 |
| --------------- | -------- | ------------------- |
| `Prop. I.2.5.7` | splits   | capital `I` follows |
| `Def. B.2`      | splits   | capital `B` follows |
| `Fig. 3`        | survives | a digit follows     |
| `i.e. the …`    | survives | lowercase follows   |

So the hazard is the abbreviation standing before a capitalised token, which in this corpus is almost always a **locator with a letter in it** — and the Latin abbreviations are safe in practice, because what follows them is normally lowercase.

- **Where the punctuation introduces or separates, use a colon.** `Note:`, `Caveat:`, `Definition B.2:` — a colon is never a sentence end, so nothing has to be disambiguated and nothing depends on what follows.
- **Where the period only abbreviates a locator, drop it.** The corpus already reads `Prop 5.13`, `Thm 4.10`, `Def 3.1.1`; keep locators in that form and the capital after them stops mattering.

### Two more formatter rules that rewrite structure rather than reporting it

Both were hit authoring the 2026-08-02 absorptions, both cost a round trip, and both are invisible in the source you wrote.

**A paragraph that follows a table and contains a `|` is read as an orphaned table row.** The pipe does not have to be a table pipe — a wikilink alias is enough, so `[[#some-anchor|some-anchor]]` in the first paragraph after a table trips `MD075` and the formatter refuses the file.
The fix is in the link, not the table: drop a redundant alias (an anchor whose display text repeats the anchor needs none), or escape the pipe.

**Heading levels are increment-enforced, and the formatter silently corrects rather than failing.** `MD001` forbids skipping a level, so a block of identifier headings under a `##` section must be `###` — writing `####` there does not error, it gets rewritten to `###` on the next format, and a reviewer reading the pre-format text will report a nesting problem that no longer exists.
Check the post-format file before acting on any structural finding about headings.

**And one neighbouring trap, because it fires on the same edit and reports somewhere else entirely.** `§` is reserved for _corpus_ anchors: `docs:reference-integrity` reads `§N.N` as a section reference and fails it as dangling when no corpus document defines it.
An external source's own sections are written out — "appendix B.11", "Example 3.6" — and the gate then leaves them alone.

Explicitly _not_ the target: syllable- and complex-word readability scores (Flesch/Fog and friends).
This is a mathematics corpus — complex words are not the enemy (owner direction, 2026-07-23); density, pacing, concept placement at paragraph boundaries, and flow are.
When prose-measurement tooling exists for the corpus again, it measures and locates, and never gates: a coverage gate makes authors write to the instrument and kills the essay.

## Pointers

- `docs/gandr/spec/README.md` — the corpus's own conventions (format, math, citations, anchors, status, dispositions).
- `docs/gandr/MANIFEST.yml` — the corpus registry; the `docs:manifest-drift` gate watches it.
- [review.md](review.md) — the review doctrine and finding dispositions.
- [docs.md](docs.md) — documentation economy, scoped to _which documents exist_, never fidelity.
