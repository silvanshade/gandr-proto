# Sharing lands in the engine, behind a trace the kernel checks

> Read when: writing the value domain, the normalizer, the conversion path, or anything that adds a node tag to the export format.

**Explicit sharing is adopted for the conversion path (owner decision, 2026-08-14).** The duplication strategy — atomic duplication under a spinal full-laziness policy, in structural-λ-calculus form with reduction at a distance — lands **in the engine, outside the trusted base, behind a trace the kernel replays**.
The kernel's stored term language and the export format are **not** changed by this decision; a stored form carrying sharing is a separate, deferred decision with its own trigger.

**Nothing in this record is built.** It states what the buildout is now committed to and what each step owes.

## The one fact the whole decision turns on

**Conversion lives inside the kernel, so a reduction strategy is trusted surface — and every theorem that would certify this sharing discipline is simply-typed.**

The deep-inference results the discipline rests on — β linearity, strong normalization and confluence of the sharing reductions, preservation of strong normalization, subject reduction — are proved over a typing system for conjunction–implication intuitionistic propositional logic. gandr's kernel is not propositional: its value-type vocabulary already carries universes at canonical levels and explicit lifts, and dependent formers arrive at the next kernel subset.
**No fragment of gandr's kernel is typed by those results, and extending them through type dependency is research rather than engineering.**

**The trace dissolves the problem instead of solving it.** A strategy whose output the kernel replays is not trusted, and a theorem certifying it is not required — the same discipline the export path already runs, where a maximal-sharing writer is an untrusted optimization caught by the reader's re-encode-compare rather than believed.
**So the trace precedes the strategy.
That ordering is forced, not chosen, and it is the single load-bearing constraint this record sets.**

## What the decision does not touch, and why

**Reduction forms are never stored.** A declaration's content is built into the environment arena behind a watermark; the checker's intermediates allocate past that mark and are truncated after the verdict, on rejection and on success alike.
The arena an artifact re-encodes is admitted content, not the working set that produced it.

Two consequences bind directly on implementation:

- **Sharing-bearing reduction syntax costs no format change.** Phantom-abstractions, distributors, and covers exist only during duplication and are never serialized; a term in sharing normal form carries none of them.
- **The stored form is implicated only by transport** — letting an artifact carry producer-computed sharing so a consumer need not recompute it.
  That is a question about cross-artifact re-check cost, not about conversion, and it is deferred below with its price.

## The staged path

Four steps, ordered.
**`share-adopt-rung-03` before `share-adopt-rung-04` is forced, not preferred**, by the argument above: conversion is in-kernel, so a strategy is trusted surface, and every theorem that would certify this one is simply-typed.
A reader who takes the ordering as a preference will reorder it, and reordering it puts an uncertified strategy inside the trusted base.
The rest of the ordering is convenience.

### share-adopt-rung-01 — sharing syntax in the value domain

The value domain gains sharing-bearing reduction syntax: closures with reduction at a distance, in structural-λ-calculus form.
Nothing is serialized.

- **Costs**: nothing on the trusted surface.
  This is part of writing a domain that does not exist yet.
- **Buys**: the substrate every later step needs, and the ability to state a duplication policy at all.

### share-adopt-rung-02 — the duplication policy is a named parameter

**The value domain names its duplication policy as a parameter from day one: which part of a shared abstraction is copied when it is forced into a redex stays policy, never a hardcoded whole-value clone.** No policy is installed at this step; the seat is built.

- **Costs**: nothing, while the domain is being written.
  Retrofitting it later costs the domain's shape.
- **Buys**: a policy chosen on measurement rather than at design time, and changed without a rewrite.

This is the same non-foreclosure move as the ratified closure-plane fence — closures stay abstract first-order pairs, the differential compares at the value and readback level rather than at intermediate-language shape — and it is binding on every conversion-path change from now.

### share-adopt-rung-03 — the trace seam

**Conversion emits a trace of its non-obvious decisions, and admission checks the trace by replay rather than trusting the strategy that produced it.** The trace's grain is the _decision_ — which side unfolded, which definition, where a fast path fired — not the reduction sequence.

- **Costs**: a trace vocabulary in the admitted surface, a replay checker, and the differential that makes it landable — from-scratch conversion equals trace-checked conversion, on every artifact.
  This is the largest cost in the path.
- **Buys**: the strategy leaves the trusted base, which is what makes `share-adopt-rung-04` legal; a checked trace is cacheable; and the result is certificate-shaped, matching the replayed-not-trusted discipline the rest of the system already runs.

**This step is worth building even if the strategy question were withdrawn**, because it is what lets any conversion strategy be replaced without re-arguing the kernel's trust.

### share-adopt-rung-04 — spinal duplication

Atomic duplication under a spinal full-laziness policy, installed into `share-adopt-rung-02`'s parameter and running inside `share-adopt-rung-03`'s trace.

- **Costs**: an allocation budget for reduction, which does not exist — the export reader's amplification budgets bound _decoded_ work and say nothing about how much a reduction allocates past the watermark.
  And a stated discharge of the open-terms confluence gap below.
- **Buys**: duplication granularity finer than call-by-need's, and a smaller search for the conversion checker.

**Two limits belong on this step so they are not discovered during implementation.**

- **Spinal full laziness and ordinary full laziness are both β-optimal for weak reduction**, so the finer granularity pays only under _strong_ reduction — where conversion goes, but not where weak-head unfolding goes.
- **The spinal calculus proves no complexity or cost theorem at all.** Its results are confluence, preservation of strong normalization, and strong normalization of the sharing reductions.
  The performance gain is argued, never measured, which is exactly what `share-adopt-rung-02`'s parameter exists to keep measurable.

## The obligations this decision creates

### share-adopt-obligation-01 — the reduction allocation axis

**The export reader's amplification budgets bound decoded work, not reduction work.** A reduction that duplicates atomically allocates past the admission watermark under no cap.
The standing requirement that the next conversion design pass state its fast-path posture against the no-hash-consing constraint is **discharged by this record**; the budget itself is owed at `share-adopt-rung-04`.
A later reader meeting the unbudgeted axis should find it owned rather than reopen it.

**The table-entry cap stays a live constraint.** It carries roughly a third again over the deepest artifact the kernel legitimately round-trips, it is already the first budget slated to rise, and replay checkpointing pushes on it from a second direction.
A third consumer is a reason to raise it deliberately rather than to meet it at a boundary golden.

### share-adopt-obligation-02 — the theorems are closed-and-weak, the checker is open-and-strong

**This is the adoption's central cost, and it is a scope mismatch rather than a build cost.**

| result                                   | scope                                                                                               |
| ---------------------------------------- | --------------------------------------------------------------------------------------------------- |
| sharing-reduction strong normalization   | **unrestricted** — all terms, untyped, by a syntactic lexicographic measure                         |
| sharing-reduction confluence             | **closed terms**, stated in the prose before the theorem and in the author's thesis likewise        |
| whole-calculus confluence, β and sharing | statement unqualified, **proof routes through the same readback lemmas**, so effective scope closed |
| preservation of strong normalization     | **unrestricted** — arbitrary strongly normalizing λ-terms                                           |
| β-optimality of the spine granularity    | **weak reduction only**; on closed terms weak reduction coincides with closed reduction             |

**The restriction has a named reason.** The 2013 ancestor states it inside its proposition and gives the ground: to exclude terms with **free variables held inside a weakening**.
The proof method is denotational — sharing normal forms are put in bijection with λ-terms and confluence follows from uniqueness — and weakening-held free variables break that correspondence's injectivity.

**No source claims non-confluence on open terms.** The restriction marks where the denotational proof method stops, not where the property fails.
This is an obligation to discharge, not a wall.

**Which terms the engine reduces: both, at two layers, and the split is the discharge.**

- **The evaluation layer runs closed configurations, machine-style.** The L machine is environment-extension only, resolves bound names through its value environment and a free name in force position through the prelude, and carries its state as a whole configuration.
  **The proved results cover this layer.**
- **The conversion driver runs open subterms.** Strong reduction is reduction under free variables: readback applies a closure to a fresh variable and normalizes the body, neutral heads are the ordinary case, and a dependent checker works under binders.
  Closedness holds of an admitted declaration, never of the subterms conversion descends into.
  **This layer is not covered.**

**The three routes, priced.**

| route                                                                                | costs to build                                                                                          | forecloses                                                                                                                       | leaves unproved                                                                            |
| ------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| restrict the sharing layer to **closed configurations** — the weak-head machine only | nothing not already planned; the normalizer is already weak-head and spine-local with readback above it | **the spine granularity where it was supposed to pay** — weak reduction is where full and spinal full laziness are already equal | nothing; inside the proved scope                                                           |
| fix a **deterministic sharing schedule** in the open/strong layer                    | a scheduling discipline plus the differential that pins it; no new theory                               | re-scheduling as a free optimization — a schedule change becomes a behaviour change needing its own differential                 | that a different schedule would agree, which matters only where replay orders must commute |
| **prove the open-term extension**                                                    | research; the work item is a canonical treatment of weakening-held free variables                       | nothing                                                                                                                          | nothing, if it lands                                                                       |

**The recommended discharge is the first two composed**: keep sharing inside the weak-head machine where configurations are closed and the results apply, and make the strong-reduction layer's sharing schedule deterministic.
**In the open-and-strong layer the adoption's foundation is therefore the deterministic schedule, not the paper's confluence theorem, and this record says so rather than inheriting a closed-terms result into an open-terms engine.** The price is that the spine granularity's advantage is deferred to whichever layer later earns an open-terms result.

**A fourth route stays in reserve and is why the mismatch is survivable at all.** Behind the trace the strategy is untrusted, so a schedule-dependent engine costs _completeness_, never _soundness_: a differently scheduled run may fail to find a proof another run finds and can never certify a false one.
That is the existing three-valued verdict discipline — holds, refuted, declined-within-budget — under which a schedule-unlucky run reports declined.

### share-adopt-obligation-03 — the no-hash-consing constraint needs sharpening

The kernel's conversion-plane constraint is stated as: sharing is _preserved_ and never _created_, no interning table or pointer-keyed memo enters the trusted base, and a fast path is admissible only when it decides reflexivity.

**Atomic duplication creates sharing during reduction, and that does not violate the constraint — but the wording reads as though it does.** The prohibited object is a **table**: something that manufactures identity, after which an identity-equality fast path decides more than reflexivity.
Sharing-bearing _syntax_ creates nodes, which are term constructors, and no equality verdict is ever read off them; the early-out keeps deciding exactly reflexive pairs.

**The constraint's statement is to be rewritten at the next kernel-vocabulary edit** to distinguish sharing-bearing syntax, which is admissible, from an identity-manufacturing table, which is not.

## The deferred decision: sharing in the stored form

**Deferred, not declined.
Its trigger is transport**, and nothing in the four steps above waits on it.

**Reserved node-tag range: `0x18..=0x1F`.** The unified subterm table's tags are contiguous from `0x00`, and `0x17` is taken by the sealing rung's abstract-type former.

| tags          | former                                                                                                                                                                                     |
| ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `0x18..=0x1B` | the sharing former, **one per family** — value, computation, value type, computation type — so polarity stays recoverable from the tag alone, which the child-checking discipline requires |
| `0x1C..=0x1F` | held: an explicit-weakening form if erasure ever becomes explicit, and second-generation sharing variants                                                                                  |

**Distributors and phantom-abstractions consume no tags at all, and that is what keeps this reservation small.** Both are reduction artifacts: a phantom-abstraction becomes an ordinary abstraction when its distributor is eliminated, and a term in sharing normal form — which is what a stored term is — carries neither.
**Only the sharing former is ever serialized**, so the whole duplication machinery costs four tags rather than a family of them.

The price, if the trigger fires:

- **Additive tags rather than a version bump.** Existing assignments keep their meaning and an older reader refuses an unknown tag precisely instead of mis-parsing.
- **A decoder generalized to variable-arity children.** The strictly-earlier child invariant, the post-order first-completion ordering, the single-forward-scan expanded-work computation, and the whole-artifact re-encode-compare all survive unchanged — each depends on children being strictly earlier, not on arity being fixed.
  The declaration segment's count-then-entries idiom is the shape to reuse.
- **A canonicalization-soundness obligation, which is the real cost and is a proof obligation rather than a coding one.** The sharing form is taken modulo two permutation quotients: within a sharing's bound vector and a cover, and between independent closures in an environment.
  Both must be given a content-determined canonical order or byte-determinism fails.
  Order a vector and a cover by first occurrence in the body; order an environment's closures by the earliest first occurrence among their bound variables, with the content address breaking ties.
  Both are computable in the forward scan the expanded-work budget already runs, and neither depends on decode history.

**One thing the price does not include.** Nameless de Bruijn survives sharing: a sharing binds a vector, so the index widens to a binder-distance-and-position pair, and α-equivalence stays syntactic identity.
**Linear variables do not force names.**

## What this record does not decide

- **Whether the stored form ever carries sharing.** Deferred above, with its trigger and its price.
- **Whether the ratified six-step definitional-equality pipeline survives.** It is a fixed heuristic strategy of the kind the current state of the art argues against, and that comparison did not exist when it was ratified.
  The fork is tracked at `gandr-cck3`, with the counter-evidence about the alternative recorded on it so it is not reopened as a recommendation.
- **What the trace's vocabulary is.** `share-adopt-rung-03`'s design pass owns it; this record fixes only that the grain is the decision rather than the reduction sequence.
- **Whether optimal reduction is ever on the table.** It is the spinal line's own stated future work, and nothing here assumes it.

## References

- David Sherratt, Willem Heijltjes, Tom Gundersen, Michel Parigot, "Spinal Atomic Lambda-Calculus", _Foundations of Software Science and Computation Structures_ (FoSSaCS 2020), LNCS 12077, pp. 582–601, `doi:10.1007/978-3-030-45231-5_30` — the sharing discipline, its typing system, and its theorem set.
- Tom Gundersen, Willem Heijltjes, Michel Parigot, "Atomic Lambda-Calculus: A Typed Lambda-Calculus with Explicit Sharing", _28th Annual ACM/IEEE Symposium on Logic in Computer Science_ (LICS 2013), pp. 311–320 — the calculus the spinal line extends.
- Nathanaëlle Courant, Xavier Leroy, "A Lazy, Concurrent Convertibility Checker", _Proceedings of the ACM on Programming Languages_ 10 (POPL), Article 53, January 2026, `doi:10.1145/3776695` — the checker architecture, the complexity framing over an abstract graph-reduction structure, and the trace seam.
  Its trace is proposed rather than implemented, and its mechanization establishes partial correctness only.
- Klaus Aehlig, Felix Joachimski, "Continuous Normalization for the Lambda-Calculus and Gödel's T", _Annals of Pure and Applied Logic_ 133 (2005), pp. 39–71, `doi:10.1016/j.apal.2004.10.003` — a normalizer whose repetition constructors denote the reduction sequence, and the candidate canonical form for the trace.
  **A continuous normal form individuates more finely than convertibility: two convertible terms can carry different repetition sequences, so it serves as a trace and never as a conversion normal form.**
- Thibaut Balabonski, "Weak Optimality, and the Meaning of Sharing", _International Conference on Functional Programming_ (ICFP 2013), pp. 263–274, `doi:10.1145/2500365.2500606` — the weak-reduction β-optimality result that bounds what the spine granularity buys.
- Alessio Guglielmi, Tom Gundersen, "Normalisation Control in Deep Inference via Atomic Flows", _Logical Methods in Computer Science_ 4 (1:9), 2008, pp. 1–36, `doi:10.2168/LMCS-4(1:9)2008` — atomic flows, for the certificate side rather than for this decision.
