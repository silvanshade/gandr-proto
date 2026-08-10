# The gandr language specification

> **This corpus is migrating out of this repository.** gandr's specifications are moving to the project's research vault, which is now their primary home for deep structure — design, reasoning, open questions, and specifications while they move.
> This repository keeps its code, its tests, and its outward-facing (user-level) documentation.
> **Do not add new specification documents to this corpus, and prefer editing over growing what is here.** New design content is authored in the vault through its own write path; what this repository still needs from a design is a thin decision record stating the outcome and what it binds, not a specification document. (Owner direction, 2026-08-07; the migration is deliberate and in progress.)
>
> Removed documents, with the page that absorbed each:
>
> + `implementation/proposed/elaboration-schedulers.md` — absorbed by "unification and coercion".
> + `implementation/proposed/display-provenance.md` — absorbed by "dub-table problem".
> + `implementation/proposed/fuss-free-universes.md` — absorbed by "fuss-free universes".
> + `implementation/proposed/pattern-unifier.md` — absorbed by "predictable fragment unifier".
> + `implementation/proposed/strengthening.md` — absorbed by "strengthening via unification".
> + `implementation/proposed/ornaments.md` — absorbed by "ornaments over the description universe".
> + `implementation/proposed/interactive-surface.md` — absorbed by "interactive and toolchain surface".
> + `surface-language/proposed/theories.md` — absorbed by "theories and extension".

This directory is the specification corpus for the gandr language: the project entrypoint for any session or contributor that needs to know what gandr _is_, what has been decided, what is built, and what remains.

It is organized as five tracks.
Each track has one main document — thorough enough that no load-bearing detail is missable — plus a subdirectory of focused sub-documents and a detailed roadmap.
The main documents describe what _currently is the case_ and summarize what remains; the roadmaps carry the detailed remaining work.

## The tracks

| track                 | main document         | subdirectory         | what it owns                                                                                                                                                                                                        |
| --------------------- | --------------------- | -------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Overview**          | [[overview]]          | —                    | what the language is about, in brief, and a map of the other tracks                                                                                                                                                 |
| **Metatheory**        | [[metatheory]]        | `metatheory/`        | the mathematics specific to gandr's semantic model: the circuit-algebra substrate, the sequent kernel's metatheory, the type system, identity and univalence, the doctrine and certificate layers                   |
| **Implementation**    | [[implementation]]    | `implementation/`    | the Rust implementation: crates, the kernel IL, the rewriting and completion engines, storage, surfaces                                                                                                             |
| **Proof engineering** | [[proof-engineering]] | `proof-engineering/` | the Agda development discipline: how structures are represented and organized, independent of gandr-specific content                                                                                                |
| **Surface language**  | [[surface-language]]  | `surface-language/`  | the surface language: the grammar's design (precedence-bounded grammars, the molder/melder pipeline, obligations), the declaration forms and their reserved slots, the shell fragment, and the vocabulary decisions |

The boundary between metatheory and proof engineering requires judgement: content belongs to **metatheory** when it is both mathematical _and_ specific to gandr's semantic model (the carrier, the site, the kernel calculus, gandr's univalence statements); it belongs to **proof engineering** when it concerns how mathematics is mechanized here in general (the ∞-graph substrate, the familial representation principle, coherence-cost policy), even when the examples are gandr modules.

## Conventions

* **Format.** Documents are Obsidian-flavoured Markdown.
  Internal links are wikilinks (`[[metatheory]]`, `[[metatheory/roadmap|the metatheory roadmap]]`); every section that other documents point at carries a stable heading so the link target is concrete.
* **Mathematics.** All mathematical notation is Typst, rendered by obsidian-typst-mate: inline math in `$…$`, display math in `$$…$$`, and full Typst blocks (used for commutative diagrams) in ```typst fenced code blocks.
  Diagrams use fletcher (`#import "@preview/fletcher:0.5.8"`); Obsidian resolves package dependencies automatically.
* **Citations.** Every external work is cited by its key in [[bibliography|bibliography.yml]] (Hayagriva format), written inline as `[@key]`.
  The bibliography is local to this directory: entries used by the specs are copied here from the contributor's research library, so the specs are self-contained.
  Claims that rest on an unverified locator say so at the claim.
* **No bare letter-number references.** Decisions, commitments, obligations, and findings are referred to by meaningful names and linked anchors, never by bare codes like "M1" or "F3.19" whose referent lives in a retired document.
  Where an item genuinely needs an identifier, the identifier carries a topic prefix and a zero-padded number — `meta-spike-04`, `meta-question-19` — is anchored so it can be linked into, and is cited by that link.
  See [`docs/workflow/specs.md`](../../workflow/specs.md) for the rule and what a bare letter cost this corpus.
  Where a named decision is load-bearing it has a heading in the owning track document.
* **`proposed/` is where a design with no implementation lives.** The corpus register is timeless: a document says what _is_ the case.
  A design that nothing yet realizes cannot honour that register without either overstating itself or hedging every sentence, so each track may carry a `proposed/` subdirectory, and a document's presence there _is_ the statement that no crate or module realizes it.
  A `proposed/` document still owes everything else the corpus owes — dispositions, citations by key, anchored identifiers, and the two-axis review — and it still names, precisely, which of its premises are built and which are not.
  It graduates into the track proper when an implementation lands, at which point its as-built claims are verified against the tree like any other document's.
* **Status.** The specs describe the current state and accepted directions.
  Historical narrative is out of scope except where a superseded claim must stay visibly superseded to prevent relitigation; those live in each track's `guards` or `hazards` sub-documents.
* **Dispositions.** Every open item imported from a source — an open question, spike, obligation, falsifier, pending read — carries exactly one disposition where a reader meets it: carried; declined with a reversal condition (in `guards`); parked with a reason (in `roadmap`); or retired with a tombstone (in `guards`).
  Nothing open vanishes silently; a settlement claim for something a source left open is a refutation and follows `docs/workflow/review.md`.
* **A decline carries a revisit obligation.** Many of this corpus's declines were adopted because the design had reached a point where it was not known how to go further while keeping the properties it needs — and several have since been dissolved by literature that was newer, or simply never found the first time.
  So meeting a recorded decline obliges the reader to ask **is this still necessary, and if so why**, before treating it as settled.
  A decline that can answer with a current reason is a rule; one that cannot is a defect, and the reversal conditions exist to make the difference cheap to see.
  This does not weaken the refutation discipline: reopening a decline is a claim like any other and is argued, not assumed.

## Roadmaps

Each track links a roadmap sub-document (for example [[metatheory/roadmap]]) carrying the detailed description of what actually remains to be done — spikes, obligations, open questions, falsifiers — while the main documents keep only summaries.
