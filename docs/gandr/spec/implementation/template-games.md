# Template games — the cobordism apparatus, and the separation logic it defers

This document owns gandr's adoption of the template-games programme: which objects gandr takes now, which half it defers and on what machinery, the single pairing the whole transfer turns on, the theorems the adoption owes, and the gate that holds every tile-level transfer until two decidable facts about the landed shift machinery are settled.

It exists as its own component because the apparatus is **semantic-foundations technology rather than an importable presentation theorem**.
Neither adopted-line source contains a statement gandr can cite to discharge an obligation it already carries, so everything on offer is construction, and the constructions are heavy — which means what adoption buys has to be stated with its price attached, at a length the commissioning ruling cannot hold.

- Status: **an adopted direction whose constructions are unbuilt but for one inert prototype; the tile-level gate has reported positive at small-scope-exhaustive grade.** The apparatus was adopted 2026-08-02 on a theorem-grade read of both sources; every rung below is unbuilt, the polarized-footprint fragment is prototyped in the tree and consumed by nothing ([[#What adoption builds]]), and tile-level transfers are licensed at the gate's own grade by [[#template-games-spike-01]].
  The separation-logic half of the same line is **deferred on machinery rather than adopted or declined**, and it is carried, with the requirements it places on that machinery, at [[proposed/separation-logic]].
- **The read's confidence is carried and never upgraded.** Each claim about the sources names the statement it rests on; the statements the read took on structure rather than line by line, and the gandr-side facts it inferred rather than proved, are marked where they are used and collected at [[#Source and confidence]].
- The ruling that commissioned the read is the device decline and its named replacement direction at [[circuit-terms#The design questions]], recorded there as `circuit-terms-question-16`; the interchange-strength decision the corpus already meets this line at is [[../metatheory#Interchange, by layer]].
  Both are linked rather than restated, and nothing here re-opens either.
- **Two further sources are read against this line rather than adopted from it, and each is cited at its own statements.** The differential-linear-logic template model [@mellies-2019-template-games-dll] carries a span-composition hazard the cobordism route inherits, together with the localization that fixes it ([[#The span hazard the cobordism route inherits]]); the layered object-based line [@oliveira-vale-mellies-shao-koenig-stefanesco-2022-layered], with the extended technical report that carries its concurrent proofs [@oliveira-vale-mellies-shao-koenig-stefanesco-2022-layered-tr], is read as a **third carrier** for the disconnection axis and for the quotient ([[#The layered line, read as a third carrier]]).
  Neither is adopted; the second is argued against as a wholesale carrier and imitated only as a shape ([[#What stays out of scope]]).

## The adopted object, and the half that waits on the memory model

**gandr adopts the template/cobordism apparatus now**: the machinery of [@mellies-stefanesco-2020-csl] secs 1-5 and sec 10, standing on the asynchronous template games of [@mellies-2021-template-games].

**gandr defers the concurrent separation logic riding on it**: separated states, the separating conjunction over a permission monoid, permissions themselves, and the Frame rule at the predicate level.
That half is deferred on machinery gandr is committed to and has not built, and it is carried as a proposal — with the requirements it places on that machinery, and the fragments that can be built ahead of it — at [[proposed/separation-logic]].

**Two items of the same neighbourhood are cut rather than deferred, and the difference is not cosmetic**: locks, critical sections, and resource invariants, and the data-race half of soundness.
Nothing the memory model becomes brings those back ([[#What stays out of scope]]).

**The import order is set by the carrier as it stands today, and it is an order rather than a verdict.** The separation product is defined by domain union with permission multiplication [@mellies-stefanesco-2020-csl, sec 7.1], so it presupposes that a state is a partial function from addresses to value-and-permission pairs over a partial cancellative commutative monoid. gandr's store is a directed hypergraph: there is no address set, no partial-map structure, and no permission monoid.
Every statement about the separating conjunction — the separated-state definition [ibid., def 7.1], the predicate semantics, and the Frame rule — therefore waits on the heap and reference machinery gandr is committed to and has not yet built, and what each of them will require of that machinery is recorded at [[proposed/separation-logic#The requirements ledger]] so the build-out meets those requirements the first time.

**Naming the candidate correctly matters, because the commissioning ruling named the line after the half that arrives last.** The entry at [[circuit-terms#The design questions]] calls the replacement direction "the separation-logic line", and the read's verdict is that the separation logic is the part that arrives later, behind the apparatus that carries it.
The correction is recorded here rather than applied silently to that entry, because the entry's ruling — that the device mapping is declined and that this line is its replacement — stands unchanged; only the order in which the line's two halves arrive has moved.

## What adoption builds

Four constructions, in the order they depend on one another.
**One of them has a fragment in the tree and the other three have nothing**; none of the four is a re-presentation of something gandr already has.

- **An asynchronous structure on the store transition graph**, whose tiles are the licensed shift commutations.
  This is the pairing everything else stands on, and it is the one with a checkable debt attached ([[#The tile pairing, and the three axioms everything gates on]]).
- **Footprints as a polarized, term-derived datum on cell applications** — the triple of cells rewritten, cells and wires matched but preserved, and internal wires freshly bound — with independence defined from that datum rather than declared on the alphabet ([[#Footprints are polarized, and that is what licenses more]]).
  **This is the one construction with any tree presence, and what is there is two of its three components, prototyped and inert.** `gandr-theory-computads`'s `footprint` module carries `MatchFootprint`, splitting the positions a redex covers into `written`, `read`, and `framed` with only the first two entering the footprint; `match_footprint` derives it from one `CellApp` firing in one term and `footprint_independence` decides the two read-against-write conjuncts.
  Nothing in the tree calls into it and `derive_shift_equivalence` stays the only licence for a shift, so the guard fence holds as stated ([[#Footprints are polarized, and that is what licenses more]]).
  The freshness component is absent because the internal-wire binder it would read a per-transition event off does not exist ([[proposed/separation-logic#separation-logic-requirement-06]]), and the prototype addresses **term positions** rather than a store, so the carrier half of the shape is interface only.
- **A template** $T = (T[0], T[1], η, μ)$ — an internal opcategory in the cospan bicategory of the store ambient, equivalently a monad (a Bénabou polyad in the indexed case) there [@mellies-stefanesco-2020-csl, thm 1.1, def 2.1].
  This forces the largest new commitment in the whole programme, because the inclusion $T[0] ⊆ T[1]$ is a distinction gandr's certificates do not currently draw ([[#The environment polarity a template forces]]).
- **Certificates re-presented as cobordisms** over that template — the double category written $"Cob"(T)$ in the source — with composition by pushout followed by relabel along $μ$ [ibid., eqns 18, 22], in a virtual variant, because gandr's composition declines where the source's is total ([[#Certificates as cobordisms, in a virtual variant]]).

## The tile pairing, and the three axioms everything gates on

**The single most decision-relevant pairing is the tile, and it comes with a checkable debt.** The correspondence marks the pairing **exact** in both directions; the axioms it would be exact under, and the index it would be exact at, are both open below.

| source object                           | statement                                                                                             | gandr counterpart                                                        |
| --------------------------------------- | ----------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| the asynchronous-graph permutation tile | [@mellies-stefanesco-2020-csl, sec 3.1], with the axioms at [@mellies-2021-template-games, sec III-A] | the earned shift-equivalence witness (`gandr-theory-computads`, `shift`) |
| path equivalence modulo tiles           | [@mellies-stefanesco-2020-csl, sec 3.1]                                                               | the shift quotient on certificates                                       |

**What a tile is quantified over has to be fixed before an axiom check means anything, and the source's choice is not the obvious one.** A **square** is a pair of length-2 paths sharing **both** a source and a target, and the tile relation is a set of such squares [@mellies-2021-template-games, sec III-A, eqn 35].

**The carrier permits parallel edges, deliberately, so the relation is indexed by edges and never by source-label-target triples** [ibid., sec I].
The source declines the traditional at-most-one-transition-per-label simplification in as many words.

**gandr's step identity is a different datum, and the pairing owes a re-index before an axiom check means anything.** As built, a gandr transition is a `CellApp` — a cell identifier together with a position — applied to a peak term, and the shift witness is derived from a peak plus an ordered pair of those (`gandr-theory-computads`, `shift`).
So two applications of one cell at two positions in one term are distinct even when they reach the same target, which is the behaviour the source's edge indexing has and its triple indexing does not.
**Whether that makes gandr's relation edge-indexed in the source's sense turns on whether the position counts as part of the label, and that is unsettled**, so a verdict has to say which indexing it was taken at ([[#template-games-question-06]]).

**An asynchronous graph must satisfy three axioms, and gandr has none of them at the level the axioms quantify over** [@mellies-2021-template-games, sec III-A].

- **Symmetry** — a tile between two length-2 paths is a tile in the other direction too.
  **Argued for gandr but not at the axiom's level**: the shift witness takes its overlap conjunct per ordered pair in both orders, which argues the _relation_ is symmetric, whereas the axiom quantifies over **squares**.
  The shared-target half is what the ordered-pair argument does not obviously give, so the claim is **restated at the square level by the spike rather than inherited**.
- **Determinism** — for length-2 paths sharing both endpoints, a tile from one path to two others forces those two to coincide.
  Note the quantifier: **all three paths share both endpoints**, and with symmetry this makes the relation a partial involution, so the residual of a step after an independent step is uniquely determined.
  **Neither claimed nor proved for gandr.**
- **The cube property** [ibid., sec III-A, eqn 36] — **and it is a biconditional, not a filling condition.** For two length-3 paths sharing both endpoints, the front-to-back sweep of tiles exists **if and only if** the back-to-front sweep does.
  A decision procedure therefore tests **both** directions, and a witness refuting either direction refutes the axiom.
  **Neither claimed nor proved for gandr.**

**What determinism buys is not a coherence nicety, and stating the loss correctly is what makes the gate worth its cost.** Determinism is what makes the ambient category of asynchronous graphs **finitely complete**: products are componentwise, equalizers are the work [ibid., prop 4, sec III-C], and the cube property of an equalizer subgraph follows from determinism of the target [ibid., prop 6, appendix A].
The chain runs: determinism, then tiles are inherited by equalizer subgraphs, then the ambient has all finite limits, then the template formalism — an internal category in a category with finite limits [ibid., sec I] — is available over asynchronous graphs **at all**.

**So the concrete gandr consequence of a determinism failure is that sub-store restriction does not inherit tile structure** — which is exactly the operation the disconnection axis and the frame direction both need, and it is checkable on the same witness.

**The gate is therefore these axioms, and it is a hard one: nothing transfers at tile level before it is answered.** Every downstream result in the source consumes tiles; none produces one, so a tile-level statement borrowed before the axioms hold would be borrowed against a structure gandr has not been shown to have.
The gate's executing scope is [[#template-games-spike-01]], and no rung of [[#The theorems owed]] may be started against a tile-level premise until that spike reports.
The spike has reported, positive at small-scope-exhaustive grade, so rungs may start — carrying the verdict's grade, not the source's.

**This is also the cheapest experiment in the programme**, which is why it is the gate rather than a deferred obligation: the two unproved axioms are decidable questions about a landed artifact, not construction programmes, and a failure witness settles the direction as usefully as a proof.

## The gating spike

### template-games-spike-01

**Prove or refute determinism and the cube property for the shift quotient as built, and test the polarized-footprint fragment beside the decided guard.**

The spike's two halves are the two cheapest facts the adoption decision needs.

- **The axioms.** Decide determinism and the cube property for the shift quotient over the landed shift machinery and its position and overlap substrate.
  If both hold, every tile-level transfer from this line is licensed; if either fails, the failure witness is the decision.
- **The polarized-footprint prototype.** Decide whether the polarized independence test — rewritten versus matched-but-preserved, read off the match image — licenses a strictly larger commuting class than the incomparable-positions conjunct, **without weakening the decided guard**.
  The test is prototyped beside the guard's constructor and never in place of it.

**Three conditions on how the axiom half is run, each of which changes what a verdict means.**

- **Fix the index first.** The source's tile relation is indexed by edges with parallel edges permitted, and gandr's is derived from a peak term and an ordered pair of cell-and-position applications; a verdict states which indexing it is about ([[#template-games-question-06]]).
- **Test both sweeps of the cube.** The property is a biconditional, so a procedure that checks one direction has not checked the axiom.
- **Restate symmetry at the square level.** The inherited by-construction argument is about ordered pairs; the axiom is about squares, and the shared-target half is the part the argument does not obviously give.

**A determinism refutation is reported with its consequence and not only as a failure**: what is lost is inheritance of tile structure under sub-store restriction, and therefore finite completeness of the ambient, and therefore the availability of the template formalism at all.

**Scope fence.** The spike changes no guard, adopts no part of the line, and buys facts rather than structure.

**Small**, in the sense the corpus uses for a bounded decidable question over landed code, and it shares a substrate with the guard it sits beside — so running it against the shift machinery is cheaper than running it against a reconstruction.

Tracked as `gandr-ng9.11`, whose scope is exactly the two halves above.

**Reported, 2026-08-02, positive on both halves — at a grade every consumer inherits.** Determinism and the cube property hold for the shift quotient as built, at **small-scope-exhaustive-plus-structural-argument** grade over the spike's fixture family — evidence, not proof; a property-based generator over terms and position sets would raise the grade — with symmetry restated at the square level as the run conditions required.
The polarized half reported the strictly larger commuting class, with the containment direction asserted empty over the comparison table.
The record is on `gandr-ng9.11`; tile-level transfers are licensed **at that grade and no stronger**, and the criterion consequence is [[#template-games-criterion-01]].

## Footprints are polarized, and that is what licenses more

**A footprint in the source is attached to a transition — an instruction occurrence — never to a state and never to a syntactic position.** Three variants appear, all explicit.

- The **machine-state footprint** is a quadruple of read set, write set, lock set, and allocation set [@mellies-stefanesco-2020-csl, sec 3.2.1].
- The **lock footprint** is the pair of lock set and allocation set, with independence componentwise disjointness [ibid., sec 3.2.2].
  It is deliberately coarser, and the source says so: it is "more liberal … about which footprints commute", because the mismatch between the two footprint notions is exactly what makes a data race detectable.
- The **separated-state footprint** is stated to be literally the same quadruple as the machine-state one [ibid., sec 7.2].

**Independence is four conditions, and only two of them are plain disjointness** [ibid., sec 3.2.1].

```text
(rd(p) ∪ wr(p)) ∩ wr(q) = ∅          lock(p) ∩ lock(q) = ∅
(rd(q) ∪ wr(q)) ∩ wr(p) = ∅          mem(p) ∩ mem(q) = ∅
```

**The load-bearing fact is that independence is polarized rather than disjoint.** Two footprints may share read locations freely; only a write against anything collides.

**And independence is derived from the action's semantics, never declared on the alphabet** — which is precisely the property the declined device line failed on.
Two occurrences of the same instruction at different addresses are independent here, where in the declined framework no generator is orthogonal to itself and two occurrences of one cell therefore never interchange.

**The polarized match image is gandr's footprint, and it is the only candidate that reproduces the asymmetry.** The four candidates the read ranked, and what separates them, are two axes: polarized versus flat, and shallow versus closed along connectivity.
The source is polarized and shallow.

| candidate                                                               | verdict                                                                                                                                                                                                                                                                                                                                                                                       |
| ----------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **the polarized match image** — rewritten against matched-but-preserved | **the fit.** The only candidate reproducing the read/write asymmetry, which is the source's content rather than its packaging; gandr already ships the polarization substrate as per-metavariable variance metadata ([[../metatheory#Interchange, by layer]])                                                                                                                                 |
| the plain match image (the support)                                     | corresponds only in the degenerate case where every access is a write. This is what the decided guard's first two conjuncts test today, and **it is the Mazurkiewicz trace-monoid reading the corpus already records** ([[../metatheory#The certificate algebra]]) — **coarsest-safe**, and strictly weaker: it refuses commutations the polarized reading licenses                           |
| a port set or boundary interface                                        | **wrong level, instructively so.** Footprints range over the machine's addresses, not over the transition's own interface; reading a port set as the footprint makes "shares a wire" mean "dependent", when a wire two applications only read is a permitted read/read overlap. **That is the disconnection axis's characteristic error, and the source's polarization is what rules it out** |
| the wiring-read support, closed along wires                             | **not the source's notion at any point.** Footprints are shallow and nothing takes a transitive closure; a closure-shaped footprint makes nearly every pair dependent in a connected diagram                                                                                                                                                                                                  |

**Component by component, the mapping is uneven and the unevenness is informative.** The read and write sets map to the polarized match image.
The allocation set — plain disjointness there — maps **plausibly** to internal-wire freshness, which the corpus witness plan already carries as the internal-wire binder.
The lock set has **no gandr counterpart whatsoever**, and that absence is not a gap to be filled: it is the load-bearing edge of the part that stays out whatever the memory model becomes ([[proposed/separation-logic#What stays out of scope regardless]]).

**A third carrier corroborates the polarized direction, and the corroboration is a worked argument rather than an analogy.** The layered line's ticket-lock congruence licenses commutation by **read-passivity on a shared location**: its rules swap a read past an increment on the _same_ object and swap two reads of it, and the swaps are justified by preservation of the happens-before order rather than by disjointness of support [@oliveira-vale-mellies-shao-koenig-stefanesco-2022-layered, sec 7.2].
That is the licence the polarized footprint promises and the incomparable-positions conjunct refuses, obtained on a carrier that shares nothing with either source of this line ([[#The layered line, read as a third carrier]]).
It is **evidence for the direction and not a contribution to [[#template-games-rung-03]]**: that source computes no footprint and derives no independence relation from an action's semantics ([[#Recorded absences]]).

**The guard fence, stated so it cannot be read as a licence.** The polarized test is **prototyped beside the decided guard and never replaces it unproven**.
The decided guard is the three-conjunct one recorded at [[circuit-terms#circuit-terms-spike-07]] — incomparable positions, trivial cell-pair overlap, and each match image still convex in the other's reduct — and a polarized independence test that licensed more without a proof that it licenses only what the guard would have licensed would be a silent weakening of a TCB-adjacent quotient.
What the prototype may do is exhibit the larger class; what it may not do is decide a commutation the guard refuses.
The disposition is [[#template-games-question-01]].

## What the frame rule derives, and what it does not

**The Frame rule is not a primitive in the source's model.** It is interpreted as the parallel product of a proof's interpretation against an identity cobordism [@mellies-stefanesco-2020-csl, sec 9.3], so all its force is the parallel product's — a pullback followed by a relabel [ibid., eqns 28-29] — together with the lax-monoidal structure of the cobordism double category [ibid., thm 4.4].

**Against the per-pair shift commutation the decided guard earns, the frame rule is orthogonal, and the derivation runs the other way.** In the source, commutation is **input**, not output: the tile set of the machine-state model is _defined_ as the squares whose footprints are independent [ibid., sec 3.2.1].
Every downstream statement consumes and transports tiles and none produces one [ibid., prop 5.1, thm 4.4, lem 10.8, lem 10.10], and there is no statement of the form "these two steps commute because of the frame rule".
So the frame rule cannot discharge the shift guard.

**The honest comparison is tile-condition against guard, and there the verdict splits.**

- **It strengthens on the guard's first two conjuncts.** Footprint independence is one term-derived test that collapses the incomparable-positions conjunct (term level) and the trivial-cell-pair-overlap conjunct (alphabet level) into a single polarized-disjointness condition — and it licenses strictly more, because read-only overlap is permitted.
  This is the one concrete place the source lets gandr earn more than it does today.
- **It is silent on the third conjunct, and structurally so.** The source has no convexity hazard, because its transitions act on a flat partial map where no action can create a directed path that destroys another match's convexity.
  Its independence relation therefore has no analogue of gandr's convexity conjunct and offers nothing toward discharging it.
  **The conjunct that is gandr's hard one is the conjunct the source never had to state**, which is why it must be added to the tile condition or discharged separately ([[#template-games-rung-03]]).

**Against per-component disconnection independence, the frame rule speaks — but what it says is a soundness frame rather than a commutation licence, and its separation is logical rather than structural.** The parallel product is a pullback over a shared template [ibid., eqns 28-29]; the two components share the whole machine state, and each sees the other's moves as its own Frame moves [ibid., sec 4.5].
Separation is then enforced by the separating conjunction on predicates and by the three-way split of a separated state [ibid., def 7.1].

**Separation there is a decomposition of a shared global state; gandr's disconnection is the absence of a shared state.** Adopting the line as written would mean introducing a global ambient and then recovering disconnection as a decomposition of it — a real and unobvious design commitment rather than a free reading, and the open question [[#template-games-question-03]].

**What the frame rule buys gandr today is therefore close to nothing.** "A certificate valid over a sub-store stays valid in a larger store with the enlargement inert" is structurally free in gandr's carrier, which is why the source has to work to state it at all.
What is genuinely new is the composite result downstream of it [ibid., thm 10.1, thm 10.5, thm 10.6], and that is the prize ([[#template-games-rung-06]]).

## What the semantic model attaches to

Each source object is paired against its gandr counterpart and marked **exact**, **plausible**, **forced**, or **absent**.
"Forced" means the pairing can be made but only by adding structure gandr does not have; "absent" means there is no counterpart today, and the row says what kind of absence it is.

| source object                                                                       | statement                                                                                    | gandr counterpart                                                                         | mark                                                                                                                                                                                                                                     |
| ----------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| asynchronous-graph permutation tile                                                 | [@mellies-stefanesco-2020-csl, sec 3.1]; axioms at [@mellies-2021-template-games, sec III-A] | the earned shift-equivalence witness                                                      | **exact**                                                                                                                                                                                                                                |
| path equivalence modulo tiles                                                       | [@mellies-stefanesco-2020-csl, sec 3.1]                                                      | the shift quotient on certificates                                                        | **exact**                                                                                                                                                                                                                                |
| a cobordism, as support with input and output boundaries and a template labelling   | [ibid., eqn 21]                                                                              | a tracelet or certificate: input boundary, output boundary, derivation as support         | **plausible**, and the best structural pairing in the read                                                                                                                                                                               |
| composition as pushout followed by relabel                                          | [ibid., eqns 18, 22]                                                                         | certificate composition                                                                   | **plausible**; the relabel step is unmatched — the template's multiplication records which side performed each half, and gandr erases nothing because it never drew the distinction                                                      |
| a template, as internal opcategory in the cospan bicategory                         | [ibid., thm 1.1, def 2.1, sec 2.2]                                                           | none                                                                                      | **forced** — gandr's ambient duoidal category of interfaces is not a template; the nearest object is "cell alphabet plus type discipline", an analogy rather than a structure                                                            |
| a game, as an object with a labelling into the template's objects                   | [ibid., eqn 20, sec 2.3]                                                                     | a typed store boundary                                                                    | **plausible**                                                                                                                                                                                                                            |
| the two-player split between ambient steps and this program's steps                 | [ibid., sec 3.3]                                                                             | none — gandr certificates are closed-world                                                | **absent**, and it is not optional ([[#The environment polarity a template forces]])                                                                                                                                                     |
| simulation, as a map of supports over the template                                  | [ibid., sec 1, sec 2.3]                                                                      | replay-equivalence                                                                        | **forced** — a simulation is a map and replay-equivalence forgets it; the corpus already records gandr's identity as strictly coarser than the induced-bijection quotient, so simulations are strictly finer than anything gandr carries |
| lax-monoidal structure on the cobordism double category, laxator the Hoare coercion | [ibid., thm 4.4]                                                                             | the "certificate composition is structurally lax" level of the interchange stratification | **plausible** — and this is where that level would acquire a theorem instead of a design ruling ([[#template-games-rung-05]])                                                                                                            |
| Gray tensor and invertible reshuffling                                              | [@mellies-2021-template-games, def 1, thm 4]                                                 | the invertible top level of the interchange stratification                                | **exact**                                                                                                                                                                                                                                |
| the interchange stratification as such                                              | —                                                                                            | —                                                                                         | **absent on the source side**: the pair supplies two of gandr's four levels and does not stratify                                                                                                                                        |
| separated state, permission monoid, separating conjunction                          | [@mellies-stefanesco-2020-csl, def 7.1, sec 7.1]                                             | none                                                                                      | **absent, and deferred rather than cut** — the gating machinery and what it must supply are at [[proposed/separation-logic]]                                                                                                             |
| machine state, as stack, heap, and locks                                            | [ibid., sec 3.2.1]                                                                           | the cell store                                                                            | **fails at the carrier** ([[#Where the fit fails]]); the heap half of the mismatch is what the deferral waits on and the lock half is cut                                                                                                |

## Where the fit fails

Four failures, each stated as precisely as the holdings above.
The first is the reason the separation-logic half waits on machinery rather than arriving with the apparatus; the remaining three are debts adoption incurs.

1. **The store structure the separating conjunction needs is structure gandr does not have yet.** Stated at [[#The adopted object, and the half that waits on the memory model]] and not restated here; it is a fact about the carrier as built rather than about the mapping, and its disposition is deferral on the heap and reference machinery ([[proposed/separation-logic]]) rather than decline.
2. **Locks are load-bearing rather than incidental, so deleting them does not leave a smaller theorem.** They appear in both footprints [@mellies-stefanesco-2020-csl, sec 3.2.1, sec 3.2.2], in the machine model's acquire and release transitions, throughout the change-of-locks development [ibid., sec 8], in the indexing of the separated-state model, and in three of the appendix lemmas [ibid., lem E.3, lem E.4, lem E.5].
   The stateless model exists only to project onto locks and allocations, and the second half of soundness is data-race freedom, where a race is **defined** by the tile mismatch between the machine-state model and the stateless one [ibid., sec 3.2.2, thm 10.6]. gandr has no race notion, so that half of the soundness theorem has no gandr statement to be about — which is why the prize below is the first half and the fibration structure, never the pair.
3. **There is no disjoint parallel product anywhere in the source.** The parallel product is a synchronizing pullback over a shared object [ibid., eqns 28-29], and gandr's disconnection is disjoint by construction.
   The source therefore **cannot be cited to justify** gandr's per-component independence: the degenerate case where the shared object is trivial is not stated.
   The disjoint case is a gandr obligation and not an import.
4. **Composition totality.** The cobordism double category requires the ambient to have pushouts precisely so that horizontal composition is total [ibid., thm 1.2, thm 2.2]. gandr's certificate composition is deliberately **partial** — acyclicity-gated, declining with the cycle as its diagnostic.
   A double category whose horizontal composition declines is not that construction, so the target has to be a virtual variant; the mismatch is unstated in the source and the debt is real ([[#Certificates as cobordisms, in a virtual variant]]).

**The two theorems the totality requirement is read from carry no proof in the source text** [ibid., thm 1.2, thm 2.2], so that requirement is taken from their statements rather than from an argument.

## Cobordism supports are store-transition systems, not computads

**The obstruction the read went looking for is absent, and the reason it is absent is a constraint that must be recorded with the adoption.**

Three central proofs turn on the ambient being adhesive and on representables being tiny [@mellies-stefanesco-2020-csl, lem 10.2, lem 10.9, def E.1, together with the strictness cube at eqn 38], and the source's own ambient of asynchronous graphs is adhesive because it is a presheaf category [ibid., rmk 3.2].

The corpus already records the matching fact one layer down: labelled directed hypergraphs form a presheaf topos and are therefore adhesive, with the same ambient hypotheses stated at [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-iii, asm 3.1] and the consequence recorded at [[circuit-terms#The hypergraph correspondence is the applicable rewriting instance once cells stop being trees]].

**So the adhesivity precondition is satisfiable at the layer where cobordism supports would live — and the constraint is that they must live there.** A support is a transition system over stores; it is **not** a computad.
The non-presheaf hazard the corpus carries [@makkai-zawadowski-2008-computads] bites at the computad layer, which is not where this construction puts its supports.

**Building the supports as computads would forfeit the whole soundness route**, because the three proofs above would lose their ambient.
The constraint is therefore normative for the adoption and not an implementation preference: **cobordism supports are built as store-transition systems.**

**The satisfiability is inferred and not proved, and the inference is marked here rather than in a footnote.** What the read established is that the corpus's own presheaf-topos record makes an adhesive ambient available at that layer.
What it did **not** establish is that gandr's certificate supports can be presented in that ambient, and the argument fails outright if they must be computads.
That is [[#template-games-rung-01]].

## The ambient hypothesis is level-dependent

**The two adopted-line sources sit at different interchange strengths, and they therefore require different ambient hypotheses; the obligation list as first drafted treated them as one substrate.** Adhesivity is the hypothesis of the **cospan-and-pushout** level — the primary's level, where the ambient of asynchronous graphs is adhesive because it is a presheaf category [@mellies-stefanesco-2020-csl, rmk 3.2] and horizontal composition is total because the ambient has pushouts [ibid., thm 1.2, thm 2.2].
It is **not** the hypothesis the Gray level needs, and the substrate says so in its own words.

**At the Gray level the ambient is monoidal and not cartesian, and the replacement hypothesis is named rather than gestured at.** Because the category of small 2-categories under the Gray tensor is monoidal rather than cartesian, the template formalism is upgraded from a category with finite limits to **a monoidal category with coreflexive equalizers, preserved componentwise by the tensor** [@mellies-2021-template-games, sec I].
The replacement machinery is four statements, and the first of them is proved rather than assumed.

- **The precondition holds for the Gray tensor**: it preserves coreflexive equalizers componentwise [ibid., prop 5, sec IV-B, eqn 41, appendix C].
- **Horizontal composition becomes a limit rather than a colimit.** The horizontal composite of bicomodules is the **equalizer** of a coreflexive pair [ibid., sec VI-B, def 4], and the componentwise-preservation assumption is what makes that composition associative [ibid., thm 2].
- **A template becomes a monad in a weak double category of comodules** rather than an internal opcategory in cospans: an internal category in the monoidal ambient is a monad in $"Comod"(S)$ [ibid., def 5, sec VII-A].
- **The translation out of the asynchronous-graph world does not preserve cartesian products, and does preserve equalizers** [ibid., sec V-B] — which is the load-bearing sentence, because it is what makes the upgrade available at all.

**So the ambient work is owed twice rather than once, and the two obligations are not substitutes.** Adhesivity and finite completeness buy the cospan-and-pushout formalism [@mellies-stefanesco-2020-csl, rmk 3.2] together with [@mellies-2021-template-games, prop 4]; coreflexive equalizers preserved componentwise buy the Gray one.
This reshapes [[#template-games-rung-01]], [[#template-games-rung-04]], and [[#template-games-rung-05]], and it is recorded as a **level-dependence** rather than as a replacement: neither hypothesis corrects the other, and a rung that names one without naming its level has named half a precondition.

**Grade.** Whether a store-transition ambient gandr would need has coreflexive equalizers preserved componentwise by whatever tensor gandr would put on it is **not established**.
What is established is that this is the precondition the source uses, and proves, for its own ambient.

## What the Gray level costs, and the reason behind a decision the corpus already took

**The deadlock defect the corpus records is caused by a simultaneous-move 1-cell, not by strictness in the abstract.** The concurrent template's tensor is the **cartesian product** of the underlying categories [@mellies-2021-template-games, sec I, eqns 23, 24], and the two orders of playing one move on each side are identified there with the **diagonal map** of that product [ibid., eqns 26-28] — so the categorical interpretation believes, wrongly, that two strategies could resolve a deadlock by playing both moves synchronously.
The Gray tensor's fix is a presentation fact: 1-cells are freely generated by the two families that move one side at a time, so **there is no diagonal to be resolved into**, and the two orders stay distinct 1-cells joined by an invertible commutation [ibid., appendix B, eqns 56, 57, 60, 61].

**This is the semantic reason behind a decision the corpus states without one.** The boundary language declines two simultaneous rewrite arguments because that spelling denotes horizontal composition ([[../metatheory#Interchange, by layer]]); the simultaneous-move generator is exactly what fabricates the deadlock resolution, so the corpus's refusal and the Gray tensor's design are the same decision taken twice.
Nothing here re-opens that ruling — this supplies its reason.

**The price of the top interchange level is exactly stated, and it is smaller than it looks.** The comparison 2-functor from the Gray tensor to the cartesian product is **locally fully faithful** [ibid., sec IV-A, eqns 38, 39]: a 2-cell of the Gray tensor is exactly a pair of component 2-cells, so the whole price is extra 1-cells plus one invertible connecting 2-cell per pair, and **no new 2-cell data whatsoever**.
Presented by generators and relations, the invertible interchanger costs **four relation families** — functoriality of the two one-sided injections, invertibility of the commutation, its naturality on both sides, and its coherence [ibid., appendix B, eqns 62-65] — and that is the complete obligation list if gandr ever presents its own interchange witness that way.

**And the two adopted-line sources exhibit two strengths of one slot, which is what [[#template-games-rung-05]] has to name.** The comparison carrying the shuffle tensor to the Gray tensor is an **isomorphism** [ibid., sec V-B, sec VIII], while the same slot in the primary is a **non-invertible lax coercion** [@mellies-stefanesco-2020-csl, thm 4.4].
That is the corpus's own interchange stratification exhibited inside a single programme ([[../metatheory#Interchange, by layer]]), and a laxator result that does not say which level it proves is a result about an unnamed structure.

## The environment polarity a template forces

**A template $T = (T[0], T[1], η, μ)$ is an internal opcategory in the cospan bicategory over the store ambient** [@mellies-stefanesco-2020-csl, thm 1.1, def 2.1, sec 2.2], and its content for gandr is one inclusion, $T[0] ⊆ T[1]$: the object of ambient steps sits inside the object of all steps [ibid., sec 3.3].

**That inclusion is the split between "steps the ambient may perform" and "steps this certificate performs", and gandr certificates are closed-world today.** A gandr certificate records a derivation against an append-only store; nothing in it distinguishes a step the certificate is responsible for from a step the environment took while the certificate was in flight, because the environment is not represented at all.

**This is the largest new commitment in the whole programme, and it is not optional.** The inclusion is what makes the two-player structure exist, and the two-player structure is what makes a cobordism over the template carry more information than its support.
Without it the cobordisms are trivial and the apparatus buys nothing.

**It is also not small.** Representing an environment polarity touches what a certificate _is_, not merely how it is checked, and no part of the corpus currently reserves a slot for it.
The design question is [[#template-games-question-02]].

## Certificates as cobordisms, in a virtual variant

**A certificate would be re-presented as a cobordism over the template** — an object of the double category the source writes $"Cob"(T)$: a support with an input boundary, an output boundary, and a labelling into the template [@mellies-stefanesco-2020-csl, eqn 21], with composition by pushout followed by relabel along the template's multiplication $μ$ [ibid., eqns 18, 22].

**The pairing is plausible and it is the best structural one the read found**, because gandr's tracelets already have exactly that shape: a derivation with two boundaries, meaningful only against the store it was minted against.

**What does not transfer is totality, and that is what forces the variant.** The source's construction requires pushouts in the ambient precisely so that horizontal composition is total [ibid., thm 1.2, thm 2.2], while gandr's certificate composition declines on the directed band when the acyclicity gate fails, carrying the variable-flow cycle as its diagnostic.
A double category whose horizontal composition declines is a **virtual** double category, and the target of the adoption is therefore a virtual variant of the source's construction.
**The gate itself is not bespoke to certificates**: it is one call into the shared graph substrate's cycle witness, the same implementation that rejects precedence cycles at grammar build ([[graph-substrate#The certificate and relations lanes]]).

**The virtual variant's coherence is in neither source.** Nothing in the read supplies the coherence conditions a virtual variant of that construction would satisfy, so the variant is a construction gandr owes rather than one it imports; the obligation is [[#template-games-rung-04]] and the open question is [[#template-games-question-04]].

**One unmatched step is worth naming, because it is a place gandr has less structure rather than more.** The relabel along $μ : T[2] → T[1]$ records **which side performed each half** of a composite. gandr composition erases nothing at that step — not because it is careful, but because it never made the distinction, which is the same absence the environment polarity names from the other direction.

## The span hazard the cobordism route inherits

**Composing spans by ordinary pullback does not preserve their 2-cells, and the source that records it calls the phenomenon troublesome and remarkable.** In the games bicategory of [@mellies-2019-template-games-dll, sec III-A] the 2-cells are **not preserved by pullbacks of spans**: a natural isomorphism between functors is not transported to a pair of reversible 2-cells in the span bicategory at all, but to a pair of **cospans of simulations**, through a cylinder category [ibid., sec III-B, eqns 26, 27].
**An earlier revision of this section said that certificates composed as spans over a store ambient would inherit exactly that**; that sentence is corrected below rather than left standing, because the construction this document adopts is the span construction's dual and the primary's silence is a consequence of that rather than an oversight ([[#The verdict on the transfer, and the delta that would reverse it]]).

**The fix is a localization, and it is what prices the double-category rung.** Replace each hom-category by its **homotopy category**, localized at the weak equivalences, and compose by **homotopy pullbacks** [ibid., sec III-C, sec III-D, eqn 28].
When the model structure is right proper the ordinary pullback along a fibration already computes the homotopy pullback [ibid., sec IV-A], which is why every structural map of that source's template carries a fibrancy or fibration condition.
So the price of composing certificate spans **and keeping their 2-cells** is a fibrancy discipline on the template plus a localization at replay-equivalences.

**The shape is already in gandr's carrier, which is what makes the hazard worth recording rather than merely noting.** Certificate identity is itself a localization at replay-equivalence — the identity forgets the derivation and keeps the boundary, and the corpus records it as strictly coarser than the induced-bijection quotient ([[../metatheory#The certificate algebra]]) — so the localization the fix asks for is not a new construction but an identity gandr already has.
What follows for the obligation is that [[#template-games-rung-04]] must say **which** of two targets it means: the cobordism double category in a 1-category, or the same construction up to homotopy.

**Grade, and this was the weakest transfer in the document.** The non-preservation is proved for spans in a 2-category, and whether gandr's certificate 2-cells live in a 2-categorical setting in the required sense was not established when the transfer was recorded.
That premise has since been checked against both sources and against the tree, and the check is the next section.

## The verdict on the transfer, and the delta that would reverse it

**The transfer as this document stated it does not hold, and the reason is a fact about the two sources rather than a fact about gandr.** The stated form is corrected here; a second, dual form of the hazard survives the correction, is **challenged rather than refuted**, and is handed to the owner with its delta ([[#template-games-question-07]]).

**The hazard is a statement about the span construction, and the construction this document adopts is that construction's dual.** In the differential source a strategy $σ : A ⊸ B$ **is a span** $A ← S → B$ with a labelling $λ_σ : S → ⊙[1]$, and strategies compose by the **pullback** $S ×_B T$ [@mellies-2019-template-games-dll, sec II-B, eqns 19, 20, 21].
In the primary a cobordism $σ : A ⊸ B$ **is a cospan** $A → S ← B$ with the same shape of labelling, and cobordisms compose by the **pushout** $S +_B T$ followed by the relabel along $μ$ [@mellies-stefanesco-2020-csl, eqns 21, 22, thm 1.2].
**The primary states the relation between the two in as many words**, and it is a duality rather than a variation: both bicategories are constructed as a bicategory sliced above a formal monad living **either in the span bicategory, for games, or in the cospan bicategory, for cobordisms**, and the differential source's internal category is "dualized" into an internal opcategory of machine states here [ibid., sec 1.2].
So "certificates composed as spans" named a shape the adoption does not take at the level this document commits to, and the dual statement — that pushouts of cospans fail to preserve 2-cells — is **not one either source makes**.

**The hazard also needs an ambient that has 2-cells at all, and the source says so itself.** It is stated for "the important case $S = "Cat"$", where the ambient is a 2-category and not merely a category, and the source records in the next section that the phenomenon "remains invisible" in the construction of the games bicategory **because that construction relies only on the categorical structure of $S$** [@mellies-2019-template-games-dll, sec III-A, sec III-B].
The cobordism construction's hypotheses are 1-categorical throughout — a category with pushouts, nothing more [@mellies-stefanesco-2020-csl, thm 1.2, thm 2.2] — so the same sentence applies to it unchanged.

**The remaining reasons are facts about gandr rather than about the machinery, and they are marked as such because they cannot carry a refutation** (the premise-tagging rule of [`docs/workflow/review.md`](../../../workflow/review.md)).
The ambient this document commits gandr to is the store-transition layer, whose adhesivity warrant is that it is a presheaf topos — a 1-categorical claim ([[#Cobordism supports are store-transition systems, not computads]]).
The differential source introduces the localization **in order to interpret the exponential modality**, which this document excludes ([[#Recorded absences]]).
And gandr's certificate layer has no third dimension for the hazard to be about, which the next paragraph states at the symbol.

**The certificate layer was read at the symbol rather than from its documentation.** A certificate is a peak, two recorded paths out of it, and a join both reach (`gandr-theory-computads`, `tracelet`); composition is a **graft** of the two paths under the strict-equality seam "`a`'s join is `b`'s peak" — unconditional on the invertible lane and acyclicity-gated on the directed one (`gandr-theory-computads`, `compose`) — and **no operation anywhere in the tree composes certificates by a limit**.
What sits between two certificates is not a morphism but a two-valued relation: replay-equivalence identifies every pair sharing a boundary that both replay, so the certificate layer as built is locally posetal in the strongest available sense, carrying **at most one certificate per boundary** up to its own identity.
**The one place the tree does take pullbacks is a dimension below the certificates**: unification computes pattern pullbacks and cell intersection is componentwise — the tight and cell layers of the compositional-rewriting double-category suite (`gandr-theory-virtual-doctrines`, the crDC suite) — where certificates are the 2-cells _over_ the pullback and never its legs.

**Naming where gandr's genuine 2-cells do live is what keeps this from being the claim that gandr has none.** Every asynchronous graph presents a 2-category whose 2-cells are reschedulings modulo the induced bijection on edge indices [@mellies-2021-template-games, prop 8, appendix D-B], and gandr's tile set satisfies the axioms at the grade [[#template-games-spike-01]] reports.
Those 2-cells are exactly what certificate identity forgets: the corpus already records replay-equivalence as strictly coarser, forgetting the induced permutation and never comparing the two paths, with the coarse choice held as a design choice and not a theorem ([[../metatheory#The certificate algebra]]).
**So the hazard is not vacuous for want of 2-cells in gandr.** It is unreachable because the layer that has them is not the layer where certificates compose, and the layer where they compose has quotiented them away — and over the shipped sequent alphabet the tile set's extension is empty besides, so the 2-cells are exhibited today only over the nesting alphabet the axiom suite fixes.

**The delta is two changes and neither alone suffices.** Certificate composition would have to become a **limit** rather than the seam graft or the pushout — which is instantiated in shape at exactly one level of this programme, the Gray level, where the horizontal composite is the equalizer of a coreflexive pair ([[#The ambient hypothesis is level-dependent]]) — **and** certificate identity would have to become finer than replay-equivalence, at least as fine as the induced-bijection quotient the corpus declines, so that there are 2-cells for the limit to fail to preserve.
A construction making only the first leaves nothing to preserve; one making only the second leaves no limit to fail.

**The cost, and the kind of change each half is.** The first half is a **commitment** change, because it is the Gray level's ambient hypothesis and that ambient is owed separately from the adhesive one ([[#The ambient hypothesis is level-dependent]]); the second reverses a landed design ruling whose reversal condition the corpus already states.
**What signing the challenge off would eliminate** is the localization price at the cospan-and-pushout level, which stops being owed; **what it would not touch** is [[#template-games-rung-04]]'s obligation to name its target level, which this document already records as standing either way.

## The proof-scope of the derived Hoare inequality

**The corpus's claim stands, and this section exists to keep it scoped rather than to weaken it.**

The interchange-strength decision records concurrent separation logic as the level at which interchange is a **non-invertible lax coercion**, with the Hoare inequality **derived rather than postulated** [[../metatheory#Interchange, by layer]].

**That claim is correct, and the read verified what it rests on.** It rests on the lax-monoidal structure of the cobordism double category [@mellies-stefanesco-2020-csl, thm 4.4] — a colimit commuting with a limit up to a non-reversible coercion — and that statement is **unconditional**.
The statement carries no proof in the source text and is asserted from lax-monoidality of the cobordism construction together with the Day and Street theory of symmetric pseudomonoids — an import this corpus holds **locator-pending**, with no bibliography key ([[#Source and confidence]]) — which is a reading grade rather than a defect, and it is marked here because the corpus leans on it.

**What is conditional is a different derivation, for a different composition, and the corpus does not currently claim it.** The Hoare inequality for the **generalized** composition is derived only under a hypothesis on the filling system [ibid., prop 5.1], and the direction of that hypothesis is the whole of its content.

```text
fill(l ∥ l', m ∥ m')  ⟶  fill(l, m) ∥ fill(l', m')
```

**The hypothesis is verified in the source's appendix B for the code templates only** — the two the source calls $S$ and $L$.

**The source states explicitly that the hypothesis is not necessarily satisfied for the template it calls $"Sep"$, the template of separated states** [ibid., sec 5], with the reason given: a two-player separated state decomposes into a three-player one in several ways.

**So the scope is: unconditional for the lax-monoidal half, verified for the code templates, and explicitly not established for the proof template through the filling system.** A later pass that leans harder on the derivation — for instance one that wants the inequality for a generalized composition rather than for the laxator — inherits the hypothesis and owes its verification.
The disposition is [[#template-games-question-05]].

**The appendix B verification was read as a case analysis and was not checked**, so the "verified for the code templates" half is carried at the source's word.

## The layered line, read as a third carrier

**A third source is read against this line rather than adopted from it, and what earns it a place here is that its carrier is disjoint from both of this line's.** The layered object-based line models each layer as an object-based game over coherence spaces [@oliveira-vale-mellies-shao-koenig-stefanesco-2022-layered], with the proofs of its concurrent fragment in its extended technical report [@oliveira-vale-mellies-shao-koenig-stefanesco-2022-layered-tr].
It supplies no statement gandr can cite either.
What it does supply is four decisions this line leaves open: what an object would have to be, whether that carrier could be adopted wholesale, where the quotient's cost actually falls, and whether a **disjoint** parallel product exists anywhere in the neighbourhood.

**What the carrier buys is one hinge rather than three.** A regular map is determined by a linear map, hence by its action on one overlay event at a time, so a per-method table is the whole implementation [@oliveira-vale-mellies-shao-koenig-stefanesco-2022-layered, thm 2.7, ex 2.8]; a regular map **is** a coalgebra morphism between free coalgebras [ibid., sec 6.3], which is the hinge, because relaxing "free" to "concurrent object space" is precisely what later buys the parallel product; and composition is total and associative [ibid., def 2.9, def 5.2].

**What it costs is four clauses of definitions rather than four design preferences.** The realization of an overlay event may not depend on where in the trace it occurs beyond what its own underlay segment records, and no overlay event may be realized by a non-contiguous underlay fragment [ibid., def 2.6]; a linear map may not identify two coherent inputs [ibid., def 2.3]; and **there is no parallel product at all before the quotient** — the category of regular maps has no tensor, which that source attributes rather than proves [ibid., sec 6].

**Certified refinement does not bear on gandr's certificate composition, and the reason is the level rather than totality.** Certification there is a **property of a morphism over a total base** — a map of functorial refinement systems [ibid., def 3.14, def 3.16, def 5.3, sec 5.3] — and it composes by a bare existential chain with no side condition, at every one of its three levels [ibid., def 2.9, def 3.8, sec 6.5].
In gandr the certificate **is** the morphism, and its composition is partial, acyclicity-gated, and declines with the cycle as its diagnostic ([[../metatheory#The certificate algebra]]).
A refinement system's refinements compose whenever the base does, and gandr has no total base for them to sit over — so that framework can neither license nor shape the virtual variant [[#template-games-rung-04]] owes.

**And the carrier is provably blind to the distinction gandr's third verdict is.** The functor from layers to regular maps is **full but not faithful**, and the source states the cause: two strategies differing only on partial behaviours have the same image, because the functor captures exactly the complete behaviours [ibid., prop 5.4, sec 5.3]. gandr's third verdict is declined-within-budget, which is a partial behaviour ([[../metatheory#The certificate algebra]]).
**That is a reason against adopting the carrier rather than a cost of adopting it**, and it is recorded as such at [[#What stays out of scope]].

**One transferable rule carries the source's own impossibility proof: an identification a map cannot make, a quotient of its source can.** For two interleavings of two independent increments there is **no linear map** sending both to one sequence, because the linear-map condition would force the two interleavings equal [ibid., sec 6, def 2.3], and the whole concurrent-object-space apparatus exists to route around that.
The gandr consequence is a placement ruling rather than a hazard: replay-equivalence deliberately identifies two confluence certificates that join by different routes, so on this carrier that identification could not be the action of a map — it would have to be a congruence on the source, which is where the corpus already puts the shift quotient ([[../metatheory#The certificate algebra]]).

## What an object would have to be, and which gandr datum could be one

**That source's unit of sharing is an object, and it is four requirements rather than an intuition** [@oliveira-vale-mellies-shao-koenig-stefanesco-2022-layered, sec 2.1, def 3.1, sec 6, sec 7.1].
An object is **named in the alphabet** before anything else happens; it carries an **operation interface**; its **state is encapsulated**, so no state appears in any move; and it is **used sequentially inside while being unconditionally independent outside** — a token of one object and a token of another are always coherent [ibid., sec 6] and [@oliveira-vale-mellies-shao-koenig-stefanesco-2022-layered-tr, lem C.5].
In one line: a **maximal unit of sequential dependence**, named statically, carrying an interface and hidden state.
**The four requirements are the source's; compressing them into that line is the read's.**

**The requirement has teeth, and they are the declined device mapping's teeth one level up.** Naming plus unconditional cross-coherence means **independence is declared on the alphabet**, which is verbatim the defect that closed the device mapping ([[circuit-terms#circuit-terms-question-16]]).
So no gandr candidate counts until it is shown to escape that, and there are exactly two escape conditions — they are also the axes the ranking is taken on: the naming must be **read off the substrate** rather than declared, and the partition must be **stable** under the rewrites the certificate performs.

| candidate                                      | verdict                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| ---------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **the connected component of the store graph** | **the best fit, and the only candidate whose disjointness gandr proves rather than declares.** The merger's incidence theorem gives both directions, so a merge of two connected shapes has exactly two components and they are the operands ([[../metatheory#The carrier, as landed]]) — the source's unconditional cross-coherence, obtained structurally. It is named only **per certificate**, it has no operation vocabulary and no hidden state, and **stability fails**: a rewrite that connects two components fuses them |
| a store region cut out by a hole or interface  | **the right shape for what an object _is_, and not a rival.** A region with a boundary interface has a signature and hidden wiring, and the corpus already carries the construction ([[../metatheory#Holes]]); but two regions sharing a boundary wire are not independent, so unconditional cross-coherence holds exactly at the empty-interface case — which is the row above. Adopt it as that row's description                                                                                                               |
| a wire, port set, or boundary occurrence       | **right datum, wrong level: this is the footprint, not the object.** It has no operations and no state, and making a shared wire a shared object makes "shares a wire" mean "dependent", when a wire two applications only read is the permitted read/read overlap the polarized direction exists to license ([[#Footprints are polarized, and that is what licenses more]])                                                                                                                                                      |
| a type — a palette colour, a sort              | **refuted, and already refuted in the corpus**: gandr's types are resources ([[circuit-terms#circuit-terms-question-16]]), and one cell application touches several colours at once, so no token is indexed by exactly one type and the naming requirement cannot even be stated                                                                                                                                                                                                                                                  |
| a cell — a generator of the cell alphabet      | **a category error, and the same one already ruled on.** A cell is an **operation of a signature** [@oliveira-vale-mellies-shao-koenig-stefanesco-2022-layered, def 3.1] — what a token is built from, not what tokens are indexed by — so making cells the objects would index every application by its rule and make two occurrences of one cell never independent, which is exactly the commutation the shift equivalence earns per pair                                                                                       |

**No candidate wins both axes, so the recommendation is a composite, and it is carried as an open question rather than as a ruling** ([[#template-games-question-08]]): name objects **per certificate** off the start position's component decomposition, take the region reading as the description of what those objects are, and make "preserves the decomposition" a **certificate-level side condition** rather than a morphism.

**A cross-cutting caution travels with all of it.** That source carries **two** indexings — one by **agent**, who executes an operation, and one by **object**, what is touched [ibid., sec 6, sec 7.1] — and its ticket-lock congruence needs both at once [ibid., sec 7.2]. gandr's parallel-replay direction is an **agent** axis and the disconnection axis is an **object** axis; merging them would repeat, on this source, the game-polarity-against-footprint-polarity collision this line fences off ([[#Recorded absences]]).

## What the third carrier decides about the quotient

**The quotient's two conditions cost gandr very differently, and only one of them is expensive.** A concurrent object space is the object space quotiented by a **coherent congruence** — an equivalence contained in the coherence relation and closed under two-sided contexts [@oliveira-vale-mellies-shao-koenig-stefanesco-2022-layered, sec 6.3, def 6.2].

- **Coherence is cheap, and arguably free.** Related traces are permutations of one another, so their first difference is a swapped pair of **distinct** operations, and two distinct operations are always coherent under the standard coherence relation [ibid., ex 2.2, sec 5.2].
  **This is a reading and not a source claim** — the source never states it — and it depends on gandr's tokens being operation-and-result pairs, which the read did not check against gandr's own encoding.
- **Congruence is the expensive half, and it lands on a fence the corpus already has.** Two-sided context closure demands that independence be a **context-free** property of the two adjacent tokens [@oliveira-vale-mellies-shao-koenig-stefanesco-2022-layered-tr, prop C.1, prop C.2, prop C.3], and gandr's third guard conjunct — each match image still convex in the other's reduct — is store-dependent.
  **The verdict is conditional and the condition is already recorded**: on today's alphabet that conjunct is discharged outright on a store certified left-connected over an acyclic target ([[circuit-terms#circuit-terms-question-15]]), so gandr's shift relation is context-free today and would be a congruence, and it stops being one at exactly the moment the alphabet change lifts that fence.
  Same fence and same trigger as that entry, which is corroboration that the fence is load-bearing for more than the guard.
  **That the demand is context-freeness is a reading of the proof's shape**, not a requirement the source states.

**A behaviour there is a clique, and the clique is forced by determinism at the proof.** An object is a clique of the object space [@oliveira-vale-mellies-shao-koenig-stefanesco-2022-layered, def 2.4] and an object strategy's even-length plays are one [ibid., prop 5.5]; the mechanism is that in a deterministic strategy the largest common prefix of two even-length plays is even-length [@oliveira-vale-mellies-shao-koenig-stefanesco-2022-layered-tr, prop B.6, lem B.1], with determinism the strategy condition [@oliveira-vale-mellies-shao-koenig-stefanesco-2022-layered, def 3.11(3)].
**And that source's own nondeterminism section does not supply the accommodation**: a non-deterministic layer interface is a **set of deterministic** strategies, upward closed under the refinement order, with certification quantified over the set [ibid., def 4.1, def 4.2].
That is **specification** nondeterminism — which deterministic object you turned out to have — and not branching within a run.
The distinction is recorded because that section reads, from its title alone, like the accommodation gandr needs.

**The conflict this creates survives at the level of the strategy and is refuted at the level of the single run.** The verdict on `gandr-9os.10` settled the gandr side: an individual certificate's **replay** takes no choices at all — the recorded step list is the control flow, with no tie-break, no index choice, and no fallback re-search — while the certificate **family** over a peak is genuinely nondeterministic, because the overlap enumerator emits one entry per unifier and never collapses them and the completion loop chooses.
Determinism in the source is a property of a **strategy**, a set of plays [ibid., def 3.11(3)], which is exactly the level at which the conflict survives; the single-run reading is refuted.
**What the surviving conflict actually names is that gandr has no canonical step list per boundary** — replay-equivalence identifies certificates with different step lists deliberately ([[../metatheory#The certificate algebra]]) — and that, rather than intra-run branching, is what a clique requirement would need.

**Grade.** The replay half is **structural and exhaustive over the replay path, given a fixed store**, at the verdict's own grade and no stronger.
It carries one named gap that is **not** nondeterminism: a recorded step names its cell by store index, so replay is a function of certificate and store rather than of certificate and cell multiset.
That gap is recorded on `gandr-9os.10` rather than here, because it is a referential weakness of the certificate datum and not a fact about this line.

## The disjoint parallel product the disconnection axis wanted

**The disconnection axis has a shape to imitate after all, and it is the one thing this read found that the primary source cannot give.** The primary has **no disjoint parallel product** — its parallel product is a synchronizing pullback over a shared object ([[#Where the fit fails]]) — whereas the layered line has one: for two concurrent object spaces the tensor **is** a concurrent object space, that category is symmetric monoidal, and the certified category inherits the product through the interleaving morphism [@oliveira-vale-mellies-shao-koenig-stefanesco-2022-layered, eqn 9, prop 6.5, prop 6.6, sec 6.5].
The proofs live in the technical report and were read there [@oliveira-vale-mellies-shao-koenig-stefanesco-2022-layered-tr, prop C.4, lem C.5, cor C.6, prop C.7, prop C.8].

**Its hypotheses are exact, and there are three of them plus nothing.**

- **Disjoint alphabets with unconditional cross-coherence.** The carrier is the "with": tokens are the disjoint sum, and a token of one side is always coherent with a token of the other [@oliveira-vale-mellies-shao-koenig-stefanesco-2022-layered, sec 6] and [@oliveira-vale-mellies-shao-koenig-stefanesco-2022-layered-tr, lem C.5].
- **Independence between components defined by projection, not derived.** Two traces are related exactly when their projections are, so every swap of adjacent tokens from different components is in the tensored relation automatically — no side condition, no footprint, no test [@oliveira-vale-mellies-shao-koenig-stefanesco-2022-layered, sec 6.4].
- **Each component's relation is a coherent congruence on its own component** [@oliveira-vale-mellies-shao-koenig-stefanesco-2022-layered-tr, prop C.4].
- **Nothing else** — no finiteness, no well-foundedness, no acyclicity, and no determinism beyond what the carrier already has.
  The read checked those proofs for a smuggled hypothesis and found none.

**One of those hypotheses is the single place this source's characteristic weakness is not gandr's.** Where the source **assumes** disjoint token sets with unconditional cross-coherence, gandr **proves** the corresponding fact: the merger's incidence theorem gives both directions, so a merge of two connected shapes has exactly two components and they are the operands — disconnection is what the substrate says rather than what the engine arranges ([[../metatheory#The carrier, as landed]]).
The projection hypothesis survives too, and exactly where it is needed: across genuinely disconnected components the guard's third conjunct is vacuous, because an application confined to one component cannot create a directed path in the other and therefore cannot destroy the other's convexity.
**That last step is an argument about gandr's convexity hazard rather than a claim about the source**, and the fence of the previous section bites only on the within-component relations.

**The third hypothesis is an obligation this line already owes, arriving from the other side.** Each component's relation being a coherent congruence is the layered line's analogue of the asynchronous-graph axioms, and that line concedes it has **no correctness criterion** for its congruences [@oliveira-vale-mellies-shao-koenig-stefanesco-2022-layered, sec 8] — the concession [[#template-games-criterion-01]] records the shift-quotient axioms as answering.

**And the caveat is the device mapping's objection recurring.** The tensored relation is defined on a **fixed** decomposition, and the certified concurrent category has no morphism whose source and target decompose differently, so **a rewrite that fuses two previously disconnected components has no image in the construction** [@oliveira-vale-mellies-shao-koenig-stefanesco-2022-layered-tr, prop C.4, prop C.7] together with [@oliveira-vale-mellies-shao-koenig-stefanesco-2022-layered, sec 6.5].
The product therefore licenses per-component independence only for **decomposition-preserving** certificates — the same certificate-level side condition the granularity ranking reaches from the other direction ([[#template-games-question-08]]).

**What it would license, priced in three.**

- **A stated theorem where the corpus has a design ruling.** For disjoint components the true-concurrency presentation and the interleaving-modulo-swap presentation are the same object, by an isomorphism of coalgebras [ibid., eqn 9] and [@oliveira-vale-mellies-shao-koenig-stefanesco-2022-layered-tr, prop C.7].
  The corpus already rules that on polarized or disjoint boundaries sequential and parallel composition coincide ([[../metatheory#Interchange, by layer]]); this is that ruling as a theorem, on a third carrier, with its hypothesis named.
- **The discriminating hypothesis of the interchange stratification is sharing, not strength.** The simultaneous-move presentation is a **theorem** for disjoint components here and a **defect** for interacting ones at the Gray level ([[#What the Gray level costs, and the reason behind a decision the corpus already took]]), and the difference is that disjoint tokens are unconditionally coherent, so nothing is left to deadlock.
  That clause is what makes the corpus's stratification a stratification rather than a menu, and the corpus does not carry it.
  **The comparison is the read's**: neither source compares itself to the other in those terms, and the layered documents never mention deadlock at all ([[#Recorded absences]]).
- **A compositional certified product with a genuinely local proof obligation.** Certify each component's certificate against its own component in isolation and obtain the composite by the product — the shape the source demonstrates when a purely local sequential refinement condition discharges a concurrent global one [@oliveira-vale-mellies-shao-koenig-stefanesco-2022-layered, sec 6.5, sec 7.1].
  This is what [[circuit-terms#circuit-terms-question-16]] declined the device mapping for lacking, and what [[#Where the fit fails]] records the primary as not supplying.

**Priced honestly, it is a shape to imitate and not a theorem to cite.** The carrier is coherence spaces where gandr's is a labelled directed hypergraph; the product inherits the missing correctness criterion; and it inherits the stability caveat above.
Its own symmetric-monoidal statement is carried on its statement rather than on a proof, in either document ([[#Source and confidence]]).

## The theorems owed

Six obligations.
**None of them is quoted from the sources**: each is a gandr obligation that the sources' shape implies, and **none discharges an obligation gandr already carries** — the programme adds theorems before it removes any.

Each carries what it would newly warrant, because an obligation with no stated payoff cannot be prioritized against the ones beside it.

**The six are cited by link and never by ordinal, and their identifiers are the rung anchors below.** A reading of this line proposed minting them as `template-games-theorem-01` through `template-games-theorem-06` so that a mapping taken against them would survive being quoted elsewhere; the six already carry stable anchored identifiers — [[#template-games-rung-01]] through [[#template-games-rung-06]], in that order and with those numbers — so the proposal is satisfied by the anchors that exist rather than by a second name for each.
**Nothing here carries two identifiers**, because a colliding reference reads as precise and is worse than no reference at all; anything citing the proposed names resolves them onto the rung anchors one for one, and the numbering is stable in both directions.

### template-games-rung-01

**The store-transition ambient is adhesive.**

Expected **free** by the presheaf-topos route the corpus already records for labelled directed hypergraphs, **provided** cobordism supports are built as store-transition systems and not as computads ([[#Cobordism supports are store-transition systems, not computads]]).

**What it would newly warrant.** The three central proofs of the soundness route become available at all: without an adhesive ambient, the strictness and fibration lemmas [@mellies-stefanesco-2020-csl, lem 10.2, lem 10.9] have no hypotheses to stand on, and [[#template-games-rung-06]] is unreachable.

**Adhesivity is not the only ambient condition the apparatus needs, and the two are owed separately.** Adhesivity is what the soundness proofs consume; **finite completeness** is what makes the template formalism exist over asynchronous graphs at all, and that one is bought by determinism rather than by the presheaf route [@mellies-2021-template-games, prop 4, prop 6, appendix A].
This rung covers the first; the second rides [[#template-games-rung-02]].

**And adhesivity is the hypothesis of one level rather than of the apparatus.** The Gray level wants a **monoidal, not cartesian** ambient with coreflexive equalizers preserved componentwise by the tensor, and horizontal composition by equalizer rather than by pushout ([[#The ambient hypothesis is level-dependent]]).
So this rung is the **cospan-and-pushout** level's ambient obligation; the Gray level's is a second, separate obligation, and neither discharges the other.
A verdict on this rung therefore names its level, and the reshaping is a statement of level-dependence rather than a substitution of one hypothesis for the other.

**Grade.** Satisfiability is **inferred, not proved** — the read did not establish that gandr's certificate supports can be presented in that ambient.
Nor was it established that the ambient gandr would need satisfies the **Gray** level's condition; what is established there is only that the condition is the one the source uses and proves for its own ambient.

### template-games-rung-02

**The tile set satisfies the three asynchronous-graph axioms at the level they quantify over** [@mellies-2021-template-games, sec III-A, eqns 35, 36].

**The rung is three obligations rather than two, and the third is a restatement rather than a proof.** Determinism and the cube property are wholly unproved for gandr; symmetry is argued by construction but not at the level the axiom quantifies over, so it is restated at the square level rather than inherited ([[#The tile pairing, and the three axioms everything gates on]]).
All three are decidable questions about the landed shift witness and the cheapest real experiment in the programme — which is why they are also the gate ([[#template-games-spike-01]]).

**What it would newly warrant.**

- **Every tile-level transfer from this line**, and nothing transfers without them.
  This rung is a precondition rather than a prize: it licenses the other five to be attempted.
- **Finite completeness of the ambient, and with it the existence of the template formalism** [ibid., prop 4, prop 6, appendix A] — which is what makes sub-store restriction inherit tile structure, the operation the disconnection axis and the frame direction both need.
- **Interchange in the third dimension rather than mere whiskering.** Every asynchronous graph presents a sesquicategory whose 2-cells are permutation sequences [ibid., prop 7, appendix D-A]; it presents a **2-category** once 2-cells are reschedulings modulo the induced bijection on edge indices [ibid., prop 8, appendix D-B], and the cube property's two sweeps are what make that quotient class nonempty from either side.
  **That last step is a reading of appendix D-B and not a statement the source makes**, since the source imposes all three axioms throughout.
- **A presentation by generators and relations, with exactly two relation families.** The 2-category is freely generated by the graph's vertices and edges in dimensions 0 and 1 and by one 2-cell per permutation tile in dimension 2, modulo two families of equations and no others [ibid., sec V-A, eqns 45, 46]: a tile followed by its symmetric inverse is the identity reshuffling, and two tile sequences realizing the same order reversal on three indices are equal.
  That is the cleanest statement of what gandr's shift-equivalence 2-cells would present if the axioms hold.
  **Which axiom underwrites which family is the same reading and carries the same mark**: symmetry is what makes the inverse tile exist for the first family and the cube property is what makes both sweeps available for the second, and the source states neither implication.
- **Standing beyond this epic.** The coherent-congruence line concedes that it supplies **no correctness criterion** for its congruences [@oliveira-vale-mellies-shao-koenig-stefanesco-2022-layered, sec 8], and these axioms are exactly the criterion it says it lacks — so a clean positive verdict is evidence for more than gandr's own quotient.
  The verdict landed positive, and the record is [[#template-games-criterion-01]].

**Grade.** **Landed positive at small-scope-exhaustive grade** ([[#template-games-spike-01]]): determinism and the cube property hold for the shift quotient as built, at small-scope-exhaustive-plus-structural-argument grade — evidence, not proof — with symmetry restated at the square level.
The axioms themselves are read at theorem grade in the original; every tile-level transfer this rung licenses inherits the verdict's grade, not the source's.

### template-games-criterion-01

**The shift-quotient axioms are the correctness criterion the coherent-congruence line concedes it lacks — recorded at the verdict's grade and citable at no more than that grade.**

The layered line's coherent congruences are the shift quotient's strictly-coarser, structurally-incomparable cousin, and that line concedes it supplies **no correctness criterion** for them [@oliveira-vale-mellies-shao-koenig-stefanesco-2022-layered, sec 8].
The asynchronous-graph axioms — symmetry, determinism, the cube property — are exactly the criterion it names as missing, and they are decidable over gandr's landed shift machinery, which is what made the question cheap to settle.

**The record.** With [[#template-games-spike-01]] landed positive, gandr's shift quotient **satisfies that criterion at small-scope-exhaustive-plus-structural-argument grade**: determinism and the cube property hold over the spike's fixture family — evidence, not proof — and symmetry is restated at the square level.
A property-based pass or a proof would strengthen the claim; until one lands, no consumer may cite this anchor as more than its grade, and a failure witness found later revokes the claim rather than qualifying it.

### template-games-rung-03

**Footprint independence implies the decided guard's licence, with the gap named.**

The gap is stated rather than hoped over: **the source's independence relation has no convexity conjunct**, so gandr's third conjunct must be added to the tile condition or discharged separately.
It does not come along.

**What it would newly warrant.** A **strictly larger commuting class**: polarized footprint independence licenses commutation across read-only overlaps, which the incomparable-positions conjunct refuses.
This is the only place the read found where the source lets gandr earn more than it does today, and it is the cheapest fragment to adopt because it touches the guard and nothing else.

**Neither comparison source contributes to this rung, and both absences are verified rather than assumed** ([[#Recorded absences]]).
What the layered line supplies instead is corroboration of the _direction_ from a third carrier — a worked concurrency argument licensing commutation by read-passivity on a shared location ([[#Footprints are polarized, and that is what licenses more]]) — which is evidence for the choice of datum and not a contribution to the theorem.

**Grade.** The claim that the source's relation has no convexity analogue is a **structural** verdict — the source's transitions act on a flat partial map — and rests on the independence definition [@mellies-stefanesco-2020-csl, sec 3.2.1] rather than on an argument the source makes.

### template-games-rung-04

**A double category of gandr certificates in the style of the source's cobordism construction, in a virtual variant.**

Virtual because gandr's composition declines and the source's construction requires totality [@mellies-stefanesco-2020-csl, thm 1.2, thm 2.2].

**What it would newly warrant.** Certificates would have a semantic model rather than an operational description, which is the precondition for [[#template-games-rung-05]] and [[#template-games-rung-06]] being theorems about a model instead of observations about how matching is implemented.

**The rung is level-dependent, and its failure mode differs by level.** At the cospan-and-pushout level horizontal composition is a pushout; at the Gray level it is the **equalizer of a coreflexive pair**, with the template a monad in a weak double category of comodules rather than an internal opcategory in cospans ([[#The ambient hypothesis is level-dependent]]).
A limit that fails to exist is not a pushout that fails to exist, so **the virtual-variant argument has to be made once per level** rather than once for the apparatus.

**And the rung names its target on a second axis as well.** Ordinary pullback composition of spans does not preserve their 2-cells, and the recorded fix is a localization at weak equivalences with composition by homotopy pullback ([[#The span hazard the cobordism route inherits]]) — so this rung states whether it targets the cobordism double category in a 1-category or the same construction up to homotopy.
That hazard does not apply to this rung's cospan-and-pushout level, because it is a statement about spans composed by pullback and this level composes cospans by pushout; whether a dual of it applies at the Gray level is the challenge handed to the owner ([[#template-games-question-07]]), and the naming obligation stands either way.

**Grade.** **The variant's coherence is in neither source.** The totality requirement is read from statements that carry no proof in the text.

### template-games-rung-05

**A lax-monoidal structure whose laxator is gandr's own interchange coercion.**

**What it would newly warrant.** It turns the "certificate composition is structurally lax" row of the interchange stratification from a design ruling into a theorem ([[../metatheory#Interchange, by layer]]).
The corpus currently holds that row on the deadlock argument and on the general failure of duoidal coherence; a laxator identified with the coercion would make it a property of a model.

**The rung must name the level it proves, and the two sources price the same slot differently.** The comparison carrying the shuffle tensor to the Gray tensor is an **isomorphism** [@mellies-2021-template-games, sec V-B, sec VIII], while the same slot in the primary is a **non-invertible lax coercion** [@mellies-stefanesco-2020-csl, thm 4.4].
So a laxator result that does not say which level it is about is a result about an unnamed structure ([[#What the Gray level costs, and the reason behind a decision the corpus already took]]), and the interchange stratification the rung would turn into a theorem is exactly the thing that distinguishes them ([[../metatheory#Interchange, by layer]]).

**Grade.** The source's own lax-monoidal statement carries no proof in the text and is asserted from an imported, locator-pending definition [@mellies-stefanesco-2020-csl, thm 4.4]; the scope note at [[#The proof-scope of the derived Hoare inequality]] applies to anything built on it.

### template-games-rung-06

**The prize: an asynchronous-soundness analogue.**

A comparison map from the certified layer to the operational layer that is both a 1-fibration and a 2-fibration, after the source's soundness theorem [@mellies-stefanesco-2020-csl, thm 10.1] and its first half [ibid., thm 10.5].
**Its second half is the excluded one**: data-race freedom [ibid., thm 10.6] has no gandr statement to be about, so the analogue is the fibration structure and never the pair ([[#Where the fit fails]]).

**What it would newly warrant — and this is the reason the whole direction was pursued.**

- **Parallel replay soundness.** The 2-fibration half says every commutation the operational layer exhibits lifts to the certified layer, which is precisely the licence a parallel-replay scheduler needs in order to reorder. gandr today has the bracket oracle and the per-pair guard; it has **no theorem that a parallel schedule is the same transformation**, and the 2-fibration is that theorem's shape.
- **A compositional independence story for disconnected components.** The preservation results — the parallel product preserves strictness [ibid., lem E.2] and preserves 1-fibrations [ibid., lem 10.10] — are exactly the shape the disconnection axis needs, and exactly what the declined device line could not give.
  The caveat travels with them: they are stated for the **synchronizing** product, so the disjoint case is a gandr obligation rather than an import ([[#Where the fit fails]]).
- **A stated frame property.** "A certificate valid over a sub-store stays valid in a larger store with the enlargement inert" is structurally free in gandr's carrier today; the value of the import is that it becomes a **provable property of a semantic model** rather than an artifact of how matching is implemented — which is what matters at the moment the store grows disconnection and multi-output.

**Neither comparison source contributes to this rung, and the absence was checked rather than assumed** ([[#Recorded absences]]): no fibration, no soundness statement, and no comparison map with either property occurs in the Gray substrate or anywhere in the layered line's two documents.
The prize lives entirely in the primary, which is what makes this rung a re-derivation against a single source.

**Grade, and it is the sharpest cost in the programme.** The source's soundness theorem is proved by induction over a **proof system**, and gandr has no proof system in that sense — so this rung is a **re-derivation, not a citation**.
Four of the lemmas it would follow were read for shape and not verified line by line [ibid., lem 10.2, lem 10.9, lem 10.10, lem E.2], one is asserted without proof in the source [ibid., lem 10.8], and the soundness theorem itself is **attributed to earlier work and re-proved axiomatically** in the source read, so this source is not the primary source for the theorem.
That earlier work has no bibliography key yet and was not chased.

## Recorded absences

**Each entry here is a verdict rather than an omission, and each is recorded so that no rung carries a false expectation.** The two comparison sources were searched for the material the obligation list would want from them; what follows is what is not there.

**From the Gray substrate** [@mellies-2021-template-games], read in full.

- **Nothing toward the fibration prize.** No fibration, soundness, or comparison statement occurs anywhere in it; the prize lives entirely in the primary [@mellies-stefanesco-2020-csl, thm 10.1, thm 10.5, thm 10.6], and [[#template-games-rung-06]] gains nothing here.
- **Nothing direct toward footprints.** Footprints do not occur; the nearest machinery is the polarity-as-comonoid statement [@mellies-2021-template-games, prop 3] and the source and target maps on its four generators [ibid., eqn 22], which are about scheduling polarity.
  Recorded as "no contribution" rather than left implied ([[#template-games-rung-03]]).

**From the layered line** [@oliveira-vale-mellies-shao-koenig-stefanesco-2022-layered] and its technical report, by full-text search of both documents.

- **Fibrations.** The word occurs in neither document; there is no soundness theorem, no lifting property, and no comparison map with one — the full-but-not-faithful functor [ibid., prop 5.4] is not a fibration.
  [[#template-games-rung-06]] gains nothing.
- **Footprints and derived independence.** The word does not occur.
  Independence first appears as a **generating rule of a congruence** [ibid., sec 6.2] and is never computed from an action's semantics, which is the property [[#template-games-rung-03]] is about.
  That rung gains nothing.
- **The ambient hypothesis.** "Adhesive" does not occur, and there is no ambient category with limit or colimit hypotheses at all — the carrier is fixed and concrete.
  [[#template-games-rung-01]] gains nothing, and the level-dependence recorded at [[#The ambient hypothesis is level-dependent]] is untouched by it.
- **The asynchronous-graph axioms.** "Cube property" does not occur and there is no analogue of determinism of tiles; coherence plus context closure is what replaces them there ([[#What the third carrier decides about the quotient]]).
  [[#template-games-rung-02]] gains nothing.
- **Deadlock.** The word does not occur, so the reading of why the simultaneous-move identification is safe for disjoint components is the read's own and not that source's comparison ([[#The disjoint parallel product the disconnection axis wanted]]).

**One absence is a fence rather than a gap, and it is re-affirmed rather than merely restated.** No polarity of any kind occurs in the layered line's object-based material; the only polarity in either of its documents is **game** polarity, in the technical report's game-semantics appendix, and the only polarity in the Gray substrate is the Player-and-Opponent polarity of moves and positions [@mellies-2021-template-games, sec VII].
Game polarity and footprint polarity share a word and nothing else, and **nothing in this document identifies them**: the footprint polarization's only source is the primary [@mellies-stefanesco-2020-csl, sec 3.2.1].
The collision would be worse than silence, because it would read as a pairing.

**And one absence is a decline the read made of itself.** Nothing in this document claims the differential-linear-logic exponential bears on this line.
The exponential-as-copy machinery, the linear-non-linear adjunction, and that source's differential soundness theorem [@mellies-2019-template-games-dll, sec VI] touch no rung — gandr is not building a linear-logic model, and multiplicative-additive linear logic is already out of scope ([[#What stays out of scope]]).
This document cites that source only for the span hazard and its localization ([[#The span hazard the cobordism route inherits]]), and once for a cross-line input that is not this component's obligation ([[#Cross-line inputs, and where they belong]]).

## Cross-line inputs, and where they belong

**Two findings of this read bear on decisions taken elsewhere, and they are recorded here only so they are not lost with the read.** Neither is an obligation of this component, neither is cited by anything above, and both corroborate rulings already landed rather than re-opening them.

- **Where copy and merge are the two adjoint avatars of one structure, the interaction law is bialgebra — forced, free, and pseudo.** In the differential-linear-logic template model the comonoid and the monoid on one carrier are the right and left adjoints of a single monoidal structure, so they carry a **bimonoid** rather than a Frobenius structure, up to invertible 2-cell, and the copy comonoid is symmetric rather than cocommutative [@mellies-2019-template-games-dll, sec I-B, sec I-D, eqn 5].
  **It does not decide anything**, because gandr's carrier has no adjoint pairing to force the law; what it contributes is a published precedent with its **precondition named**, alongside the rewriting-cost argument the owner ruling already rests on ([[circuit-terms#circuit-terms-question-20]]).
  **The transferable rule is the read's**: the source exhibits the mechanism additively and multiplicatively and never states it as a rule.
- **A Gray comonoid structure on the free 2-category over a set is the same data as a polarity function to a two-element set** [@mellies-2021-template-games, prop 3, eqn 31].
  That is a published instance of a comonoid **read off an existing two-valued predicate** rather than posited as a new former — an equivalence of data rather than an analogy, which is the shape of the ruling that a cell-layer copy obligation is read off the grade discipline ([[circuit-terms#circuit-terms-question-18]]).
  Its force and its limit are the same fact: it is stated for the **free** structure on a set.

## Open questions, with dispositions

Each is anchored and cited by link.
Every one carries a disposition.

### template-games-question-01

**Which datum is gandr's footprint, and does the polarized reading survive contact with the guard?**

**Carried, with the polarized match image preferred and the guard fence binding.** The pair of cells the application rewrites and cells and wires it matches but preserves is the fit ([[#Footprints are polarized, and that is what licenses more]]), and a polarization substrate exists on cells as per-metavariable variance metadata.
**What that substrate is evidence for is narrowed (owner ruling, 2026-08-08); the disposition itself is unchanged.** A variance-driven polarized reading is one design the substrate would support, and it is not the reading the prototype built, which consults no cell metadata and reads its three classes off the firing instance by grafting and probe perturbation instead ([[proposed/separation-logic#What can be built before the machinery lands]]).
Two sub-questions travel with it and neither is answered here: whether the allocation component's counterpart really is internal-wire freshness (marked **plausible**, not established), and whether the polarized test's larger commuting class is a superset of the decided guard's rather than merely a different one.
The fence is the disposition's operative half: the test is prototyped beside the guard and **never replaces it unproven**.
The prototype is [[#template-games-spike-01]].

### template-games-question-02

**How does gandr represent the environment polarity a template forces?**

**Carried, and it is the largest new commitment in the programme.** gandr certificates are closed-world; the template's inclusion of ambient steps into all steps is what makes cobordisms nontrivial, so the polarity is not optional ([[#The environment polarity a template forces]]).
Nothing here proposes a representation.
What the question needs first is a decision about _where_ the polarity lives — in the certificate, in the store it is minted against, or in the boundary — because that choice decides whether existing certificates are re-presentable or superseded.

### template-games-question-03

**Does gandr introduce a global ambient and recover disconnection as a decomposition of it, or keep disconnection primitive and owe the disjoint case itself?**

**Carried, unanswered, and it is a fork rather than a detail.** Separation in the source is a decomposition of a **shared** global state, while gandr's disconnection is the **absence** of a shared state ([[#What the frame rule derives, and what it does not]]).
Taking the source's route means introducing the ambient and then recovering disconnection inside it, which is a real design commitment.
Keeping disconnection primitive means the preservation results of [[#template-games-rung-06]] are owed in the disjoint case, because the source states them only for the synchronizing product.
Neither branch is free and the read does not choose between them.

### template-games-question-04

**What are the coherence conditions of the virtual variant?**

**Carried, and it is a genuine hole rather than a lookup.** Composition partiality forces a virtual variant of the cobordism double category, and **the virtual variant's coherence is not in either source** ([[#Certificates as cobordisms, in a virtual variant]]).
The corpus's existing virtual-honesty posture is the natural place to look first — the reflection dictionary is already virtual, with loose composites existing only as an overlap-indexed seam family — but whether that reading supplies the coherence a cobordism variant needs is unestablished.

### template-games-question-05

**Is the Hoare inequality derived for a proof template through the filling system, or only for the code templates?**

**Answered against the sources, and carried as a scope rather than as a defect.** The corpus's derived-not-postulated claim rests on the unconditional lax-monoidal statement and stands ([[#The proof-scope of the derived Hoare inequality]]).
The **generalized** composition's inequality holds under a filling hypothesis verified for the code templates only, and the source states explicitly that the hypothesis is not necessarily satisfied for the separated-states template.
The disposition is that the scope is recorded now, before anything leans on the conditional half; a pass that does lean on it inherits the hypothesis and owes its verification for whatever template gandr's proof layer becomes.

### template-games-question-06

**At which index is gandr's tile relation stated, and does the pairing survive the source's?**

**Carried, and it is a precondition on the gate rather than a consequence of it.** The source's carrier permits parallel edges deliberately, so its tile relation is indexed by **edges** and never by source-label-target triples [@mellies-2021-template-games, sec I], while gandr's witness is derived from a peak term and an ordered pair of cell-and-position applications.
Those need not be the same quantification, and an axiom verdict taken at the wrong one is a verdict about a different relation.
The disposition is that the re-index is settled **before** the axiom half of [[#template-games-spike-01]] runs, and that any verdict names the indexing it was taken at.

**What is established and what is not.** The gandr side is verified at the symbol: a step is a `CellApp` over a peak, so position is part of what individuates a transition (`gandr-theory-computads`, `shift` and `rewrite`).
**What is not established is the translation** — whether the cell identifier alone is the source's label, with position part of the edge, or whether the pair is the label — and nothing here decides it.

### template-games-question-07

**Does the span 2-cell hazard bite in gandr's setting, or is it vacuous there?**

**Answered against the sources for the stated form, and challenged rather than refuted for the dual form; the second half needs owner sign-off and not a further read.** The verdict, its premises and its delta are at [[#The verdict on the transfer, and the delta that would reverse it]].

**The stated form is settled on the machinery.** The non-preservation is a statement about spans composed by pullback in the games bicategory [@mellies-2019-template-games-dll, sec II-B, sec III-A], while this document's adoption target is the cobordism construction — cospans composed by pushout — and the primary itself names the two as slices above a formal monad in the span and the cospan bicategory respectively [@mellies-stefanesco-2020-csl, eqns 21, 22, thm 1.2, sec 1.2].
The hazard's further hypothesis, an ambient carrying 2-cells, is one the source explicitly says the construction does without [@mellies-2019-template-games-dll, sec III-B].

**The dual form is what is challenged, and it rests on premises about gandr rather than about the machinery**, so it is handed over rather than closed: gandr's ambient is committed 1-categorical, certificate composition is a seam graft with no limit in it, and certificate identity collapses each boundary's certificates to one.
Neither source states a pushout analogue of the phenomenon, so nothing here is quoting one.

**Three halves of the disposition, and the third is new.** The hazard is not quoted as a gandr fact in either form; [[#template-games-rung-04]] states which target it means — the cobordism double category in a 1-category, or the same construction up to homotopy — either way, because that naming obligation does not wait on this answer; and the **delta** is recorded so a later sweep re-opens the branch instead of re-deriving it, namely a limit-shaped composition at the Gray level together with a certificate identity finer than replay-equivalence.

### template-games-question-08

**Does gandr name objects per certificate off the start position's component decomposition, and is decomposition-preservation a certificate-level side condition?**

**Carried, with the composite recommendation preferred and neither half ruled.** The connected component is the only granularity candidate whose disjointness gandr **proves** rather than declares, and the region-and-splice reading is the only one that supplies an interface and hidden state, so the recommendation is to use the second as the description of the first ([[#What an object would have to be, and which gandr datum could be one]]).

**Two things stop that from being a ruling, and both are cheap to settle in the wrong order.** A component decomposition would have to be computable at a certificate's start position, and the corpus records `Connected` as an undirected predicate on shapes without recording that a decomposition operation exists or is cheap — **unverified, and load-bearing for the whole proposal**.
And a rewrite that fuses two components has no image in the construction that motivates the proposal, so "preserves the decomposition" would be a **certificate-level side condition** rather than a structural fact ([[#The disjoint parallel product the disconnection axis wanted]]).

**The fence is that this question does not re-open [[#template-games-question-03]].** Naming objects per certificate is a granularity proposal inside gandr's existing carrier; whether gandr introduces a global ambient at all is the separate fork, and nothing here decides it.

## What stays out of scope

Explicitly, so that nothing here is later read as a partial adoption of the whole.

**The separation-logic half is not in this list, because it is deferred rather than cut, and it is owned elsewhere.** The separating conjunction, permissions, separated states, predicate-indexed colouring, and the Frame rule at the predicate level are carried as a proposal at [[proposed/separation-logic]], held against the heap and reference machinery gandr is committed to; nothing below re-opens that, and nothing below is waiting on machinery.

- **Locks, critical sections, and resource invariants**, together with the change-of-locks development and the lock indexing the separated-state model carries [@mellies-stefanesco-2020-csl, sec 8, sec E.2].
  The frame-rule interpretation stated over that indexing [ibid., sec 9.3] is deferred with the separation-logic half rather than cut, and what it would require is [[proposed/separation-logic#separation-logic-requirement-05]].
- **Data-race freedom and the stateless model**, because gandr has no race notion and the second half of the soundness theorem therefore has no gandr statement to be about [ibid., thm 10.6].
- **The error monad** [ibid., sec 6], because gandr's three-valued verdict discipline is a different and already-recorded device: a declined check leaves a certificate stuck rather than refuted.
- **Multiplicative-additive linear logic and star-autonomy** [@mellies-2021-template-games, thm 1, thm 4], because gandr is not building a linear-logic model.
- **Higher-order concurrent separation logic and its descendants — Iris and FCSL**, which the sources name as their own future work and which are not a gandr direction.
  Neither is cited at a statement here, and neither has a bibliography key.
- **The layered line's coherence-space carrier, as gandr's certificate carrier.** The functor from layers to regular maps is full but **not faithful**, and the cause is stated in the source: strategies differing only on partial behaviours have the same image [@oliveira-vale-mellies-shao-koenig-stefanesco-2022-layered, prop 5.4, sec 5.3]. gandr's declined-within-budget verdict is exactly such a partial behaviour, so adopting that carrier wholesale would **collapse the third verdict** rather than cost something to keep — a reason against the carrier and not a price of it.
  **What is out of scope is the wholesale carrier and not the line**: its disjoint parallel product is imitated as a shape ([[#The disjoint parallel product the disconnection axis wanted]]), its concession about correctness criteria is cited at [[#template-games-criterion-01]], and its object granularity is the material of [[#template-games-question-08]].

**One further exclusion is a decline rather than a scope cut, and it is recorded so it is not re-proposed.** The lock component of the footprint has no gandr counterpart, and that is not a gap: filling it would mean inventing a shared-resource notion in order to import machinery whose payoff is a race theory gandr does not want.
It is a decline in the strong sense — the arrival of a heap does not revive it ([[proposed/separation-logic#What stays out of scope regardless]]).

## Cost, stated honestly

- **The environment polarity is not optional and is not small.** It is what makes the template's two-player split exist, and hence what makes cobordisms nontrivial at all.
- **Nothing discharges an existing obligation.** The whole programme adds theorems before it removes any, and that is a property of the direction rather than of this scoping.
- **The prize is a re-derivation, not a citation.** The soundness theorem is proved by induction over a proof system gandr does not have.
- **Composition partiality forces a virtual variant whose coherence is in neither source**, so the target of the adoption is a construction rather than an import.
- **The ambient hypothesis is level-dependent, so the ambient work is owed twice.** Adhesivity buys the cospan-and-pushout level and coreflexive equalizers preserved componentwise buy the Gray one, and neither discharges the other ([[#The ambient hypothesis is level-dependent]]).
  The same level-dependence splits the double-category rung and the laxator rung, so three of the six obligations are owed per level rather than once.
- **Against all five, the direction is real.** The tile pairing is exact, the adhesivity precondition is satisfiable at the right layer, and the polarized fragment was the cheap experiment the direction turned on — it ran, and it reported the strictly larger commuting class at the spike's own grade ([[#template-games-spike-01]]).

## Source and confidence

**Four sources are cited in this document, and the two adopted-line sources carry a different grade from the two read against them.**

**Both adopted-line sources were read at theorem grade for the template/cobordism half on 2026-08-02, from copies held locally, with identity checked from page 1 for each.**

The primary [@mellies-stefanesco-2020-csl] was read in full — fifty pages, body plus appendices.
The substrate [@mellies-2021-template-games] was read twice at different depths, and the grades differ by material rather than by statement.

- **The asynchronous-graph axioms and the Gray-tensor content are read in full at theorem grade**, body plus appendices, with the axiom statements, the finite-completeness chain, and the sesquicategory and 2-category presentations taken at their own numbers [@mellies-2021-template-games, sec III-A, eqns 35, 36; prop 4, prop 6, appendix A; prop 7, prop 8, appendices D-A and D-B].
  No verdict of the earlier pass is contradicted by that read.
- **Everything else in the substrate was read at substrate grade** for the machinery the primary stands on: sec I and secs III-V, plus the statements of prop 1-8, thm 1-4, and def 1-3, with the proofs of none of them.
  The material that pass did not reach line by line is sec VI (the comodule construction and thm 2), sec VII (the template as a monad in comodules, def 5), and appendices A-E — of which appendices A and D have since been read as arguments at the theorem-grade pass above.

**What was read but not verified line by line, in the primary.**

| statement                                                           | grade                                                                                                 |
| ------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| thm 1.1, thm 1.2, thm 2.2, thm 4.4                                  | **carry no proof in the source text**; thm 4.4 is asserted from lax-monoidality plus Day and Street   |
| lem 4.2, lem 10.8                                                   | **asserted without proof** in the source                                                              |
| lem 10.2, lem 10.3, lem 10.7, lem 10.9, lem 10.10, lem E.2, lem E.5 | proofs read **for shape, not for correctness** — each is a diagram chase followed rather than checked |
| appendix B's map construction                                       | read as a case analysis and **not checked**                                                           |

**What is unverified in the substrate even at the theorem-grade pass** [@mellies-2021-template-games]: thm 1, thm 3, and thm 4 carry no proof in the text, and thm 3 is explicitly deferred to a recipe in a work that was not chased; thm 2 is called folklore by the author and is likewise unproved there.
Appendix C's coreflexive-equalizer preservation proof was followed only for structure, and appendix B's coherence relations were **not** checked against an independent presentation of the Gray tensor.
The finite-limit argument of appendix A, including the proof that gives the equalizer subgraph its cube property, and the reshuffling construction of appendix D were read as arguments and followed.

**Imported results were not chased.** Three of them bear on claims above, so they are named rather than gestured at, and each is **locator-pending**: none has a bibliography key yet, and this document cites none of them at a statement.

- **Lack and Sobociński on adhesive categories**, which is where the primary's adhesivity hypotheses come from.
- **Day and Street on symmetric pseudomonoids in a monoidal bicategory**, at their sec 3, which supplies the complete definition the lax-monoidal statement is asserted from [@mellies-stefanesco-2020-csl, thm 4.4].
- **The earlier Melliès and Stefanesco asynchronous-soundness paper**, and this one matters most: the soundness theorem [ibid., thm 10.1] is **attributed to that earlier work and re-proved axiomatically** in the source read here, so the source read here is **not** the theorem's original home, and [[#template-games-rung-06]] would be re-deriving a statement whose primary source the corpus does not yet hold.

Also unchased, and named with the locator the primary gives so each can be resolved inside it: Bénabou for the polyad definition at his def 5.5.1, Garner for lax double functors, Mulry at his lem 2.20 together with Johnstone for the smash-product lifting, Bourke and Gurski for the Gray characterization, and Melliès' own earlier template-games papers.

**gandr-side facts not verified, two of them load-bearing.**

- **Whether the shift witness tile set is deterministic and satisfies the cube property.** Answered positive at small-scope-exhaustive grade ([[#template-games-spike-01]]); what stays open is the grade itself — a property-based pass or a proof would settle it — and every tile-level transfer inherits the verdict's grade meanwhile ([[#template-games-rung-02]]).
- **Whether the store-transition ambient gandr would need is adhesive.** Satisfiability was **inferred** from the corpus's own record that labelled directed hypergraphs are a presheaf topos; it was not proved that gandr's certificate supports can be presented in that ambient, and the argument fails if they must be computads — [[#template-games-rung-01]].
- **The three-conjunct guard was read from the shift module's own documentation rather than from its body.** Verified against the tree at write time for this document: `gandr-theory-computads`'s `shift` module exists and documents the three conjuncts and their order, `derive_shift_equivalence` is its constructor, `ShiftObstruction` is the typed refusal, the overlap conjunct is asked of the cell pair through `overlaps_between`, and the convexity conjunct is carried as a `ConvexityDischarge` datum whose two inhabitants are the left-connected-over-acyclic-target discharge and the re-check-required refusal.
  **What is verified is the interface and its documented contract, not that the body decides what the documentation says it decides.**
- **The polarized-footprint prototype was read the same way, at its module documentation and its symbols rather than at its bodies.** Verified against the tree at write time: the `footprint` module of the same crate exists, `MatchFootprint` carries the `written`, `read`, and `framed` splits over the alphabet's own position type, `match_footprint` reads it off one `CellApp` firing, `footprint_independence` returns a `FootprintIndependence` whose three inhabitants are independence and the two collisions, and the module states that nothing calls into it.
  **What is verified is again the interface and what the module says it decides.**
- **No bibliography key beyond the four cited sources was checked by the read.** All four keys cited in this document were checked against `bibliography.yml` at write time, including the technical report's own key, which is a separate entry because the report is a separate artifact.

**The differential-linear-logic template model is cited for two things and read in full, and it carries almost no proofs of its own.** [@mellies-2019-template-games-dll] was read in full — cover, thirteen-page body, and its appendix — with identity checked from page 1.
This document cites it for the span 2-cell hazard and its localization ([[#The span hazard the cobordism route inherits]]) and once as a cross-line input ([[#Cross-line inputs, and where they belong]]), and for nothing else.
**Its properties A-F are assumptions rather than results, and its differential soundness theorem is stated with its proof only indicated**, so anything taken from that source is taken on its statements; the imported works behind those statements were not chased and none has a bibliography key.

**The layered line carries a split grade, and the split decides which of its statements this document may lean on.** [@oliveira-vale-mellies-shao-koenig-stefanesco-2022-layered] was read at theorem grade across its object-based construction, its layered model, its nondeterminism section, its correspondence section, and its concurrent-object-space and case-study material, with identity checked from page 1; its extended technical report [@oliveira-vale-mellies-shao-koenig-stefanesco-2022-layered-tr] was acquired, identity-checked from page 1, and its appendix C read **in full with proofs** — which matters, because that appendix is the only place the parallel-product proofs exist, and the two documents are distinct artifacts, the report carrying appendices the conference version omits.
Its appendix B was read for statements with two proofs followed, and its game-semantics appendix was read as statements only, without checking the report's own claim that the presentation is equivalent to the conference version's.

**Three of that line's statements are taken on their statements rather than on proofs**, and each is flagged where it is used: the symmetric-monoidal statement for the concurrent category has **no proof in either document** — the isomorphism it rests on is proved and was read, and the remaining ingredient is cited to imported work that was not chased; the representation theorem for regular maps is attributed and unproved there; and the claim that the pre-quotient category has no tensor products is an attribution in a parenthesis.

**Four claims about the layered line in this document are the read's reasoning rather than source statements, and each is marked at the claim**: that coherence is cheap for gandr because two distinct operations are always coherent; that congruence forces independence to be context-free; that the third guard conjunct is vacuous across components; and the whole of the sharing-not-strength reading of the interchange stratification.
The one-line definition of an object is likewise a compression of the source's four requirements and not a definition the source gives.

**One further gandr-side fact is unverified and load-bearing for the layered material**: whether gandr can compute a component decomposition at a certificate's start position cheaply, which the granularity recommendation assumes ([[#template-games-question-08]]).
A second one was carried here and is now discharged — whether gandr's certificate 2-cells live in the setting the span hazard needs — read at the symbol over the certificate, composition and virtual-double-category modules ([[#The verdict on the transfer, and the delta that would reverse it]]).
**No gandr source code was read for the layered material**: its gandr-side statements are taken from the corpus as written, except for the replay determinism claim, which is carried at the grade of the verdict on `gandr-9os.10` — structural and exhaustive over the replay path, given a fixed store ([[#What the third carrier decides about the quotient]]).

**No recorded corpus claim was contradicted by any of the four sources.** Both interchange-strength characterizations at [[../metatheory#Interchange, by layer]] check out: the Gray and invertible level against the substrate's own deadlock-and-diagonals passage and its Gray-tensor statements [@mellies-2021-template-games, sec I, def 1, thm 4], and the lax level against the primary's lax-monoidal statement and its concluding section [@mellies-stefanesco-2020-csl, thm 4.4].
Three sharpenings were produced rather than corrections: the filling-system scope carried at [[#The proof-scope of the derived Hoare inequality]]; the level-dependence of the ambient hypothesis ([[#The ambient hypothesis is level-dependent]]), which refines an obligation rather than refuting a claim; and the discriminating hypothesis the interchange stratification does not carry ([[#The disjoint parallel product the disconnection axis wanted]]).
