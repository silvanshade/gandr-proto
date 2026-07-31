# The gandr language specification

This directory is the specification corpus for the gandr language: the project entrypoint for any session or contributor that needs to know what gandr _is_, what has been decided, what is built, and what remains.

It is organized as four tracks.
Each track has one main document — thorough enough that no load-bearing detail is missable — plus a subdirectory of focused sub-documents and a detailed roadmap.
The main documents describe what _currently is the case_ and summarize what remains; the roadmaps carry the detailed remaining work.

## The tracks

| track                 | main document         | subdirectory         | what it owns                                                                                                                                                                                      |
| --------------------- | --------------------- | -------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Overview**          | [[overview]]          | —                    | what the language is about, in brief, and a map of the other tracks                                                                                                                               |
| **Metatheory**        | [[metatheory]]        | `metatheory/`        | the mathematics specific to gandr's semantic model: the circuit-algebra substrate, the sequent kernel's metatheory, the type system, identity and univalence, the doctrine and certificate layers |
| **Implementation**    | [[implementation]]    | `implementation/`    | the Rust implementation: crates, the kernel IL, the rewriting and completion engines, storage, surfaces                                                                                           |
| **Proof engineering** | [[proof-engineering]] | `proof-engineering/` | the Agda development discipline: how structures are represented and organized, independent of gandr-specific content                                                                              |

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
  Where a named decision is load-bearing it has a heading in the owning track document.
* **Status.** The specs describe the current state and accepted directions.
  Historical narrative is out of scope except where a superseded claim must stay visibly superseded to prevent relitigation; those live in each track's `guards` or `hazards` sub-documents.
* **Dispositions.** Every open item imported from a source — an open question, spike, obligation, falsifier, pending read — carries exactly one disposition where a reader meets it: carried; declined with a reversal condition (in `guards`); parked with a reason (in `roadmap`); or retired with a tombstone (in `guards`).
  Nothing open vanishes silently; a settlement claim for something a source left open is a refutation and follows `docs/workflow/review.md`.

## Roadmaps

Each track links a roadmap sub-document (for example [[metatheory/roadmap]]) carrying the detailed description of what actually remains to be done — spikes, obligations, open questions, falsifiers — while the main documents keep only summaries.
