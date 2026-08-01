# Durable computation — reversibility, replication, and computations that outlive a process

This document owns the durable-computation direction: computations whose state survives the process that produced it, can be run backwards, and can be replicated and reconciled across machines.
It is a **feature direction with a design basis and no scoped work**, opened because a literature sweep supplied the reference points the direction had previously lacked.

* Status: **direction, with sources.** Nothing here is scheduled, and nothing in the current build-out depends on it.
* The direction is stated here rather than in the roadmap because it is a coherent feature with its own vocabulary and its own literature, and because several pieces of it are **already built for other reasons** — recording that coincidence is most of the value of the document.

## What the direction is

Four properties, which the literature treats separately and which gandr would want together:

* **Durable** — a computation's state is a persisted artifact, not process memory, so a computation can be suspended, moved, and resumed, and its history is inspectable after the fact.
* **Reversible** — every step has a defined inverse, so a computation can be run backwards to any earlier point rather than only restarted from a checkpoint.
* **Concurrent and distributed** — independent parts proceed independently, and "undo" means undoing an action **only once its consequences have been undone**, which is the causal-consistency condition rather than a global clock.
* **Reconcilable** — divergent replicas of the same computation merge without a coordinator, which is the replicated-data-type condition.

The reason to state them together is that each one alone is a known engineering problem with known solutions, and the combination is what a language could plausibly make _ordinary_ rather than expert work.

## Why gandr is unusually well positioned

The claim to check, stated as a claim rather than a result: **gandr may already have most of the substrate, built for unrelated reasons.**

| the direction needs                                 | gandr already has                                                                                                                                            |
| --------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| persistent, verifiable computation state            | the content-addressed storage stack, with canonicalize-before-address, a manifest-committed artifact identity, and history-independence as a tested property |
| mergeable replicated structure                      | prolly trees, whose whole reason for existing is structural sharing and cheap diff/merge over ordered content                                                |
| a step relation with first-class continuations      | the polarized sequent IL, where consumers are first-class data and a machine state _is_ a command                                                            |
| a notion of "the same computation, differently run" | replay-equivalence as certificate identity, which is already the project's answer to when two derivations are the same                                       |
| an inverse for a step                               | the certificate algebra's invertibility flag, and the two-mode composition boundary that routes on it                                                        |
| a place to put the evidence                         | tracelets, which are already _recorded, replayable derivations_ rather than logs                                                                             |

**The sharpest version of the observation**: gandr's rewriting engine already records which cell fired where and replays rather than trusts.
A replayable forward derivation is most of a reversible one — what is missing is not the record but the **inverse of each step and the guarantee that the record is sufficient to compute it**, which is precisely what the reversible-rewriting literature studies.

Two honest cautions on the same claim.

* **Replay is not reversal.** gandr replays a derivation forward from a recorded peak; running it backwards needs each step to have a computable inverse, and the engine's invertibility is a **provenance stipulation** for derived-by-completion cells rather than a checked property.
* **The identity layer is a gate, not a helper, until it lands.** The interesting version of this direction — where "the same computation reached two ways" is a path, and reconciliation is transport — needs the univalence machinery.
  Before that it is engineering; after it, it is the language's own vocabulary.

## The reference points the sweep supplied

Collected from the reversible-computing literature, which is where this direction's prior art lives.
Each entry says what it supplies; none is adopted.

### Categorical models of reversible computation

* **Join inverse categories** give reversible recursion, a †-trace, and algebraic ω-compactness, answering how recursive definitions work when every morphism is a partial isomorphism [@kaarsgaard-axelsen-gluck-2017-join-inverse-recursion].
* **Join inverse rig categories** are proposed as _the_ categorical model of reversible computing, covering several reversible languages at once [@kaarsgaard-rennela-2021-join-inverse-rig].
* **Reversible pattern-matching needs structure beyond that** — the paper derives what must be added to model the case analysis a reversible functional language actually uses [@chardonnet-lemonnier-valiron-2021-reversible-pattern-matching].
  This is the entry most directly relevant to gandr, whose eliminators are exactly case analysis and copatterns.
* **The historical and categorical arc**, from Landauer and Bennett through rig categories to reversible effects, is surveyed with the connections to type isomorphisms, permutations, and univalent universes made explicit [@carette-heunen-kaarsgaard-sabry-2024-compositional-reversible].

### Reversibility as a program transformation

* **Reversible term rewriting** extends rewriting conservatively so each forward step is undoable, and supplies **injectivization** and **inversion** as transformations that make a standard rewrite system reversible [@nishida-palacios-vidal-2017-reversible-term-rewriting].
  Its trace-shrinking work is the part gandr's engine would consume first.
* **Injectivization, reversibilization, inverse interpretation, and program inversion** are surveyed as a coherent metaprogramming toolkit rather than one-off tricks [@gluck-yokoyama-2023-reversible-pl-perspective].
* **Logically reversible abstract machines** convert systematically into programs of a reversible language of type isomorphisms, including self-interpreters [@james-sabry-2014-isomorphic-interpreters].
  If gandr ever wants a reversible evaluator, this is the recipe shape.

### Reversibility for concurrency, which is where "durable" actually bites

* **Causal-consistent reversibility** is the operative notion: an action may be undone exactly when its consequences have been undone.
  The higher-order π-calculus development proves causal consistency and relates its causality to the standard causal semantics [@lanese-mezzina-stefani-2016-reversible-hopi].
* **A general technique derives a causal-consistent reversible extension from a forward reduction semantics** in a specific format, with causality based on resources consumed and produced, and the properties proved once for the whole family rather than per calculus [@lanese-medic-2020-uncontrolled-reversible].
  This is the most reusable item in the group and the natural first read.
* **The space overhead of reversibility is measured, not guessed**: making an abstract machine reversible costs at most linear space in the number of execution steps, and the bound is tight — some programs cannot be made reversible without storing a commensurate amount [@lienhardt-lanese-mezzina-stefani-2012-reversible-machine].
  Any durable-computation design owes a budget, and this is the shape of the answer.
* **The COST-action survey** indexes the field across models, automata, Petri nets, and process calculi [@aman-et-al-2020-foundations-reversible].

### Material history as a compositional structure

The sweep's most on-point find for the **durable** half rather than the reversible half: a monoidal category of **open transition systems that generate material history as transitions unfold**, where the history generated by a composite is composed of the histories generated by its components, and the construction is parameterized by a resource theory [@nester-2022-situated-transition-systems].

That is a compositional audit trail — the thing a durable computation needs and that a log is not, because a log does not compose.
It is the closest published structure to "the record of what happened is itself an algebraic object with the same composition as the thing it records", which is what gandr's tracelets are reaching for one layer down.

### The univalence connection

* **Reversible programs are paths**: a reversible language of type isomorphisms corresponds to a formally presented univalent universe, with combinators as 1-paths and combinator optimizations as **2-paths** [@carette-chen-choudhury-sabry-2017-reversible-univalent].
* **The correspondence is sound and complete at both levels**, with the language presented by the free symmetric rig groupoid [@choudhury-karwowski-sabry-2022-symmetries-reversible].

This is why the direction is filed as more than a feature.
If a durable computation's history is a path and its reconciliation is transport, then the identity layer gandr is building for other reasons is the same machinery, and "undo" stops being a runtime facility bolted onto a language and becomes a term former with a type.

## Open questions

1. **durable-computation-question-01** — **is a tracelet already a reversible-computation trace?** It records which cell fired where and replays by re-matching; the reversible-rewriting line records enough to run backwards.
   **Carried**; the answer decides whether this direction reuses the certificate layer or needs a second record.
2. **durable-computation-question-02** — **what is the unit of durability?** A command, a cut, a certificate, or an artifact.
   **Carried**; the storage stack's grain is the artifact and the engine's grain is the cell, and they do not obviously agree.
3. **durable-computation-question-03** — **does causal consistency have a home in the sequent IL?** Causality-by-resources-consumed-and-produced is stated for reduction semantics in a specific format, and a polarized cut is a resource interaction.
   **Carried**, and it is the specific question the general-derivation technique would be read to answer.
4. **durable-computation-question-04** — **do prolly trees carry the replication story, or only the storage story?** They are in the tree for content-addressed storage; whether they are the replicated data structure a distributed durable computation would merge on is a separate claim.
   **Carried, and unverified** — the coincidence is suggestive and is not an argument.
5. **durable-computation-question-05** — **what does reversibility look like at the surface-engine interface?** The engine already drives an editor-facing incremental pipeline with checkpoints and an append-only emission log where rollback is truncation, which is a reversible structure serving a non-reversible purpose.
   **Carried**; this is the cheapest place the direction could show a user-visible result.
6. **durable-computation-question-06** — **what is the space budget?** The measured bound is linear in execution steps and tight.
   **Carried**; a direction that cannot state its overhead is not designed.

## Relationship to the rest of the corpus

* The **certificate layer** owns replay-equivalence and the invertibility flag; this direction consumes them and must not restate them.
* The **identity layer** owns paths and transport; the univalence connection above is a reason to keep this direction's vocabulary aligned with it rather than inventing a parallel one.
* The **storage stack** owns content addressing, prolly trees, and artifact identity; the durability claims here are claims about _reuse_, and each is marked unverified until checked.
* [[circuit-terms]] routed this literature here rather than dropping it; the two lanes share only their sources.

## Source and confidence

* **This is a direction, not a design.** No item above has been read beyond triage grade, and the gandr-side claims are explicitly claims: the substrate table says what gandr has, not that it suffices.
* The **space-overhead result and the causal-consistency condition are quoted from abstracts**, and both are the kind of statement that should be re-read at its theorem before anything is built on it.
* The **prolly-tree replication claim is the weakest link** and is marked as such: structural sharing and cheap merge are why the structure exists, and that is not the same as being a replicated data type with a defined merge semantics.
