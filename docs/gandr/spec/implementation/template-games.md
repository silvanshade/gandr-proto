# Template games — the cobordism apparatus, and the separation logic it defers

This document owns gandr's adoption of the template-games programme: which objects gandr takes now, which half it defers and on what machinery, the single pairing the whole transfer turns on, the theorems the adoption owes, and the gate that holds every tile-level transfer until two decidable facts about the landed shift machinery are settled.

It exists as its own component because the apparatus is **semantic-foundations technology rather than an importable presentation theorem**.
Neither source contains a statement gandr can cite to discharge an obligation it already carries, so everything on offer is construction, and the constructions are heavy — which means what adoption buys has to be stated with its price attached, at a length the commissioning ruling cannot hold.

* Status: **an adopted direction whose constructions are unbuilt; the tile-level gate has reported positive at small-scope-exhaustive grade.** The apparatus was adopted 2026-08-02 on a theorem-grade read of both sources; every rung below is unbuilt, and tile-level transfers are licensed at the gate's own grade by [[#template-games-spike-01]].
  The separation-logic half of the same line is **deferred on machinery rather than adopted or declined**, and it is carried, with the requirements it places on that machinery, at [[proposed/separation-logic]].
* **The read's confidence is carried and never upgraded.** Each claim about the sources names the statement it rests on; the statements the read took on structure rather than line by line, and the gandr-side facts it inferred rather than proved, are marked where they are used and collected at [[#Source and confidence]].
* The ruling that commissioned the read is the device decline and its named replacement direction at [[circuit-terms#The design questions]], recorded there as `circuit-terms-question-16`; the interchange-strength decision the corpus already meets this line at is [[../metatheory#Interchange, by layer]].
  Both are linked rather than restated, and nothing here re-opens either.

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
None of them exists in the tree, and none is a re-presentation of something gandr already has.

* **An asynchronous structure on the store transition graph**, whose tiles are the licensed shift commutations.
  This is the pairing everything else stands on, and it is the one with a checkable debt attached ([[#The tile pairing, and the three axioms everything gates on]]).
* **Footprints as a polarized, term-derived datum on cell applications** — the triple of cells rewritten, cells and wires matched but preserved, and internal wires freshly bound — with independence defined from that datum rather than declared on the alphabet ([[#Footprints are polarized, and that is what licenses more]]).
* **A template** $T = (T[0], T[1], η, μ)$ — an internal opcategory in the cospan bicategory of the store ambient, equivalently a monad (a Bénabou polyad in the indexed case) there [@mellies-stefanesco-2020-csl, thm 1.1, def 2.1].
  This forces the largest new commitment in the whole programme, because the inclusion $T[0] ⊆ T[1]$ is a distinction gandr's certificates do not currently draw ([[#The environment polarity a template forces]]).
* **Certificates re-presented as cobordisms** over that template — the double category written $"Cob"(T)$ in the source — with composition by pushout followed by relabel along $μ$ [ibid., eqns 18, 22], in a virtual variant, because gandr's composition declines where the source's is total ([[#Certificates as cobordisms, in a virtual variant]]).

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

* **Symmetry** — a tile between two length-2 paths is a tile in the other direction too.
  **Argued for gandr but not at the axiom's level**: the shift witness takes its overlap conjunct per ordered pair in both orders, which argues the _relation_ is symmetric, whereas the axiom quantifies over **squares**.
  The shared-target half is what the ordered-pair argument does not obviously give, so the claim is **restated at the square level by the spike rather than inherited**.
* **Determinism** — for length-2 paths sharing both endpoints, a tile from one path to two others forces those two to coincide.
  Note the quantifier: **all three paths share both endpoints**, and with symmetry this makes the relation a partial involution, so the residual of a step after an independent step is uniquely determined.
  **Neither claimed nor proved for gandr.**
* **The cube property** [ibid., sec III-A, eqn 36] — **and it is a biconditional, not a filling condition.** For two length-3 paths sharing both endpoints, the front-to-back sweep of tiles exists **if and only if** the back-to-front sweep does.
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

* **The axioms.** Decide determinism and the cube property for the shift quotient over the landed shift machinery and its position and overlap substrate.
  If both hold, every tile-level transfer from this line is licensed; if either fails, the failure witness is the decision.
* **The polarized-footprint prototype.** Decide whether the polarized independence test — rewritten versus matched-but-preserved, read off the match image — licenses a strictly larger commuting class than the incomparable-positions conjunct, **without weakening the decided guard**.
  The test is prototyped beside the guard's constructor and never in place of it.

**Three conditions on how the axiom half is run, each of which changes what a verdict means.**

* **Fix the index first.** The source's tile relation is indexed by edges with parallel edges permitted, and gandr's is derived from a peak term and an ordered pair of cell-and-position applications; a verdict states which indexing it is about ([[#template-games-question-06]]).
* **Test both sweeps of the cube.** The property is a biconditional, so a procedure that checks one direction has not checked the axiom.
* **Restate symmetry at the square level.** The inherited by-construction argument is about ordered pairs; the axiom is about squares, and the shared-target half is the part the argument does not obviously give.

**A determinism refutation is reported with its consequence and not only as a failure**: what is lost is inheritance of tile structure under sub-store restriction, and therefore finite completeness of the ambient, and therefore the availability of the template formalism at all.

**Scope fence.** The spike changes no guard, adopts no part of the line, and buys facts rather than structure.

**Small**, in the sense the corpus uses for a bounded decidable question over landed code, and it shares a substrate with the guard it sits beside — so running it against the shift machinery is cheaper than running it against a reconstruction.

Tracked as `gandr-ng9.11`, whose scope is exactly the two halves above.

**Reported, 2026-08-02, positive on both halves — at a grade every consumer inherits.** Determinism and the cube property hold for the shift quotient as built, at **small-scope-exhaustive-plus-structural-argument** grade over the spike's fixture family — evidence, not proof; a property-based generator over terms and position sets would raise the grade — with symmetry restated at the square level as the run conditions required.
The polarized half reported the strictly larger commuting class, with the containment direction asserted empty over the comparison table.
The record is on `gandr-ng9.11`; tile-level transfers are licensed **at that grade and no stronger**, and the criterion consequence is [[#template-games-criterion-01]].

## Footprints are polarized, and that is what licenses more

**A footprint in the source is attached to a transition — an instruction occurrence — never to a state and never to a syntactic position.** Three variants appear, all explicit.

* The **machine-state footprint** is a quadruple of read set, write set, lock set, and allocation set [@mellies-stefanesco-2020-csl, sec 3.2.1].
* The **lock footprint** is the pair of lock set and allocation set, with independence componentwise disjointness [ibid., sec 3.2.2].
  It is deliberately coarser, and the source says so: it is "more liberal … about which footprints commute", because the mismatch between the two footprint notions is exactly what makes a data race detectable.
* The **separated-state footprint** is stated to be literally the same quadruple as the machine-state one [ibid., sec 7.2].

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

* **It strengthens on the guard's first two conjuncts.** Footprint independence is one term-derived test that collapses the incomparable-positions conjunct (term level) and the trivial-cell-pair-overlap conjunct (alphabet level) into a single polarized-disjointness condition — and it licenses strictly more, because read-only overlap is permitted.
  This is the one concrete place the source lets gandr earn more than it does today.
* **It is silent on the third conjunct, and structurally so.** The source has no convexity hazard, because its transitions act on a flat partial map where no action can create a directed path that destroys another match's convexity.
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

**The virtual variant's coherence is in neither source.** Nothing in the read supplies the coherence conditions a virtual variant of that construction would satisfy, so the variant is a construction gandr owes rather than one it imports; the obligation is [[#template-games-rung-04]] and the open question is [[#template-games-question-04]].

**One unmatched step is worth naming, because it is a place gandr has less structure rather than more.** The relabel along $μ : T[2] → T[1]$ records **which side performed each half** of a composite. gandr composition erases nothing at that step — not because it is careful, but because it never made the distinction, which is the same absence the environment polarity names from the other direction.

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

## The theorems owed

Six obligations.
**None of them is quoted from the sources**: each is a gandr obligation that the sources' shape implies, and **none discharges an obligation gandr already carries** — the programme adds theorems before it removes any.

Each carries what it would newly warrant, because an obligation with no stated payoff cannot be prioritized against the ones beside it.

### template-games-rung-01

**The store-transition ambient is adhesive.**

Expected **free** by the presheaf-topos route the corpus already records for labelled directed hypergraphs, **provided** cobordism supports are built as store-transition systems and not as computads ([[#Cobordism supports are store-transition systems, not computads]]).

**What it would newly warrant.** The three central proofs of the soundness route become available at all: without an adhesive ambient, the strictness and fibration lemmas [@mellies-stefanesco-2020-csl, lem 10.2, lem 10.9] have no hypotheses to stand on, and [[#template-games-rung-06]] is unreachable.

**Adhesivity is not the only ambient condition the apparatus needs, and the two are owed separately.** Adhesivity is what the soundness proofs consume; **finite completeness** is what makes the template formalism exist over asynchronous graphs at all, and that one is bought by determinism rather than by the presheaf route [@mellies-2021-template-games, prop 4, prop 6, appendix A].
This rung covers the first; the second rides [[#template-games-rung-02]].

**Grade.** Satisfiability is **inferred, not proved** — the read did not establish that gandr's certificate supports can be presented in that ambient.

### template-games-rung-02

**The tile set satisfies the three asynchronous-graph axioms at the level they quantify over** [@mellies-2021-template-games, sec III-A, eqns 35, 36].

**The rung is three obligations rather than two, and the third is a restatement rather than a proof.** Determinism and the cube property are wholly unproved for gandr; symmetry is argued by construction but not at the level the axiom quantifies over, so it is restated at the square level rather than inherited ([[#The tile pairing, and the three axioms everything gates on]]).
All three are decidable questions about the landed shift witness and the cheapest real experiment in the programme — which is why they are also the gate ([[#template-games-spike-01]]).

**What it would newly warrant.**

* **Every tile-level transfer from this line**, and nothing transfers without them.
  This rung is a precondition rather than a prize: it licenses the other five to be attempted.
* **Finite completeness of the ambient, and with it the existence of the template formalism** [ibid., prop 4, prop 6, appendix A] — which is what makes sub-store restriction inherit tile structure, the operation the disconnection axis and the frame direction both need.
* **Interchange in the third dimension rather than mere whiskering.** Every asynchronous graph presents a sesquicategory whose 2-cells are permutation sequences [ibid., prop 7, appendix D-A]; it presents a **2-category** once 2-cells are reschedulings modulo the induced bijection on edge indices [ibid., prop 8, appendix D-B], and the cube property's two sweeps are what make that quotient class nonempty from either side.
  **That last step is a reading of appendix D-B and not a statement the source makes**, since the source imposes all three axioms throughout.
* **Standing beyond this epic.** The coherent-congruence line concedes that it supplies **no correctness criterion** for its congruences [@oliveira-vale-mellies-shao-koenig-stefanesco-2022-layered, sec 8], and these axioms are exactly the criterion it says it lacks — so a clean positive verdict is evidence for more than gandr's own quotient.
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

**Grade.** The claim that the source's relation has no convexity analogue is a **structural** verdict — the source's transitions act on a flat partial map — and rests on the independence definition [@mellies-stefanesco-2020-csl, sec 3.2.1] rather than on an argument the source makes.

### template-games-rung-04

**A double category of gandr certificates in the style of the source's cobordism construction, in a virtual variant.**

Virtual because gandr's composition declines and the source's construction requires totality [@mellies-stefanesco-2020-csl, thm 1.2, thm 2.2].

**What it would newly warrant.** Certificates would have a semantic model rather than an operational description, which is the precondition for [[#template-games-rung-05]] and [[#template-games-rung-06]] being theorems about a model instead of observations about how matching is implemented.

**Grade.** **The variant's coherence is in neither source.** The totality requirement is read from statements that carry no proof in the text.

### template-games-rung-05

**A lax-monoidal structure whose laxator is gandr's own interchange coercion.**

**What it would newly warrant.** It turns the "certificate composition is structurally lax" row of the interchange stratification from a design ruling into a theorem ([[../metatheory#Interchange, by layer]]).
The corpus currently holds that row on the deadlock argument and on the general failure of duoidal coherence; a laxator identified with the coercion would make it a property of a model.

**Grade.** The source's own lax-monoidal statement carries no proof in the text and is asserted from an imported, locator-pending definition [@mellies-stefanesco-2020-csl, thm 4.4]; the scope note at [[#The proof-scope of the derived Hoare inequality]] applies to anything built on it.

### template-games-rung-06

**The prize: an asynchronous-soundness analogue.**

A comparison map from the certified layer to the operational layer that is both a 1-fibration and a 2-fibration, after the source's soundness theorem [@mellies-stefanesco-2020-csl, thm 10.1] and its first half [ibid., thm 10.5].
**Its second half is the excluded one**: data-race freedom [ibid., thm 10.6] has no gandr statement to be about, so the analogue is the fibration structure and never the pair ([[#Where the fit fails]]).

**What it would newly warrant — and this is the reason the whole direction was pursued.**

* **Parallel replay soundness.** The 2-fibration half says every commutation the operational layer exhibits lifts to the certified layer, which is precisely the licence a parallel-replay scheduler needs in order to reorder. gandr today has the bracket oracle and the per-pair guard; it has **no theorem that a parallel schedule is the same transformation**, and the 2-fibration is that theorem's shape.
* **A compositional independence story for disconnected components.** The preservation results — the parallel product preserves strictness [ibid., lem E.2] and preserves 1-fibrations [ibid., lem 10.10] — are exactly the shape the disconnection axis needs, and exactly what the declined device line could not give.
  The caveat travels with them: they are stated for the **synchronizing** product, so the disjoint case is a gandr obligation rather than an import ([[#Where the fit fails]]).
* **A stated frame property.** "A certificate valid over a sub-store stays valid in a larger store with the enlargement inert" is structurally free in gandr's carrier today; the value of the import is that it becomes a **provable property of a semantic model** rather than an artifact of how matching is implemented — which is what matters at the moment the store grows disconnection and multi-output.

**Grade, and it is the sharpest cost in the programme.** The source's soundness theorem is proved by induction over a **proof system**, and gandr has no proof system in that sense — so this rung is a **re-derivation, not a citation**.
Four of the lemmas it would follow were read for shape and not verified line by line [ibid., lem 10.2, lem 10.9, lem 10.10, lem E.2], one is asserted without proof in the source [ibid., lem 10.8], and the soundness theorem itself is **attributed to earlier work and re-proved axiomatically** in the source read, so this source is not the primary source for the theorem.
That earlier work has no bibliography key yet and was not chased.

## Open questions, with dispositions

Each is anchored and cited by link.
Every one carries a disposition.

### template-games-question-01

**Which datum is gandr's footprint, and does the polarized reading survive contact with the guard?**

**Carried, with the polarized match image preferred and the guard fence binding.** The pair of cells the application rewrites and cells and wires it matches but preserves is the fit ([[#Footprints are polarized, and that is what licenses more]]), and the substrate exists as per-metavariable variance metadata.
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

## What stays out of scope

Explicitly, so that nothing here is later read as a partial adoption of the whole.

**The separation-logic half is not in this list, because it is deferred rather than cut, and it is owned elsewhere.** The separating conjunction, permissions, separated states, predicate-indexed colouring, and the Frame rule at the predicate level are carried as a proposal at [[proposed/separation-logic]], held against the heap and reference machinery gandr is committed to; nothing below re-opens that, and nothing below is waiting on machinery.

* **Locks, critical sections, and resource invariants**, together with the change-of-locks development and the lock indexing the separated-state model carries [@mellies-stefanesco-2020-csl, sec 8, sec E.2].
  The frame-rule interpretation stated over that indexing [ibid., sec 9.3] is deferred with the separation-logic half rather than cut, and what it would require is [[proposed/separation-logic#separation-logic-requirement-05]].
* **Data-race freedom and the stateless model**, because gandr has no race notion and the second half of the soundness theorem therefore has no gandr statement to be about [ibid., thm 10.6].
* **The error monad** [ibid., sec 6], because gandr's three-valued verdict discipline is a different and already-recorded device: a declined check leaves a certificate stuck rather than refuted.
* **Multiplicative-additive linear logic and star-autonomy** [@mellies-2021-template-games, thm 1, thm 4], because gandr is not building a linear-logic model.
* **Higher-order concurrent separation logic and its descendants — Iris and FCSL**, which the sources name as their own future work and which are not a gandr direction.
  Neither is cited at a statement here, and neither has a bibliography key.

**One further exclusion is a decline rather than a scope cut, and it is recorded so it is not re-proposed.** The lock component of the footprint has no gandr counterpart, and that is not a gap: filling it would mean inventing a shared-resource notion in order to import machinery whose payoff is a race theory gandr does not want.
It is a decline in the strong sense — the arrival of a heap does not revive it ([[proposed/separation-logic#What stays out of scope regardless]]).

## Cost, stated honestly

* **The environment polarity is not optional and is not small.** It is what makes the template's two-player split exist, and hence what makes cobordisms nontrivial at all.
* **Nothing discharges an existing obligation.** The whole programme adds theorems before it removes any, and that is a property of the direction rather than of this scoping.
* **The prize is a re-derivation, not a citation.** The soundness theorem is proved by induction over a proof system gandr does not have.
* **Composition partiality forces a virtual variant whose coherence is in neither source**, so the target of the adoption is a construction rather than an import.
* **Against all four, the direction is real.** The tile pairing is exact, the adhesivity precondition is satisfiable at the right layer, and the polarized fragment is a cheap experiment that either buys a strictly larger commuting class or settles the direction with a counterexample.

## Source and confidence

**Both sources were read at theorem grade for the template/cobordism half on 2026-08-02, from copies held locally, with identity checked from page 1 for each.**

The primary [@mellies-stefanesco-2020-csl] was read in full — fifty pages, body plus appendices.
The substrate [@mellies-2021-template-games] was read twice at different depths, and the grades differ by material rather than by statement.

* **The asynchronous-graph axioms and the Gray-tensor content are read in full at theorem grade**, body plus appendices, with the axiom statements, the finite-completeness chain, and the sesquicategory and 2-category presentations taken at their own numbers [@mellies-2021-template-games, sec III-A, eqns 35, 36; prop 4, prop 6, appendix A; prop 7, prop 8, appendices D-A and D-B].
  No verdict of the earlier pass is contradicted by that read.
* **Everything else in the substrate was read at substrate grade** for the machinery the primary stands on: sec I and secs III-V, plus the statements of prop 1-8, thm 1-4, and def 1-3, with the proofs of none of them.
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

* **Lack and Sobociński on adhesive categories**, which is where the primary's adhesivity hypotheses come from.
* **Day and Street on symmetric pseudomonoids in a monoidal bicategory**, at their sec 3, which supplies the complete definition the lax-monoidal statement is asserted from [@mellies-stefanesco-2020-csl, thm 4.4].
* **The earlier Melliès and Stefanesco asynchronous-soundness paper**, and this one matters most: the soundness theorem [ibid., thm 10.1] is **attributed to that earlier work and re-proved axiomatically** in the source read here, so the source read here is **not** the theorem's original home, and [[#template-games-rung-06]] would be re-deriving a statement whose primary source the corpus does not yet hold.

Also unchased, and named with the locator the primary gives so each can be resolved inside it: Bénabou for the polyad definition at his def 5.5.1, Garner for lax double functors, Mulry at his lem 2.20 together with Johnstone for the smash-product lifting, Bourke and Gurski for the Gray characterization, and Melliès' own earlier template-games papers.

**gandr-side facts not verified, two of them load-bearing.**

* **Whether the shift witness tile set is deterministic and satisfies the cube property.** Answered positive at small-scope-exhaustive grade ([[#template-games-spike-01]]); what stays open is the grade itself — a property-based pass or a proof would settle it — and every tile-level transfer inherits the verdict's grade meanwhile ([[#template-games-rung-02]]).
* **Whether the store-transition ambient gandr would need is adhesive.** Satisfiability was **inferred** from the corpus's own record that labelled directed hypergraphs are a presheaf topos; it was not proved that gandr's certificate supports can be presented in that ambient, and the argument fails if they must be computads — [[#template-games-rung-01]].
* **The three-conjunct guard was read from the shift module's own documentation rather than from its body.** Verified against the tree at write time for this document: `gandr-theory-computads`'s `shift` module exists and documents the three conjuncts and their order, `derive_shift_equivalence` is its constructor, `ShiftObstruction` is the typed refusal, the overlap conjunct is asked of the cell pair through `overlaps_between`, and the convexity conjunct is carried as a `ConvexityDischarge` datum whose two inhabitants are the left-connected-over-acyclic-target discharge and the re-check-required refusal.
  **What is verified is the interface and its documented contract, not that the body decides what the documentation says it decides.**
* **No bibliography key beyond the three cited sources was checked by the read.** The keys cited in this document were checked against `bibliography.yml` at write time.

**A third source is cited once, at one statement, and it carries its own grade.** This document cites [@oliveira-vale-mellies-shao-koenig-stefanesco-2022-layered] at one statement — the concession that the coherent-congruence line supplies no correctness criterion [ibid., sec 8] — and nothing here rests on any other statement of it.
The source's full read grade is split, and its record is [[proposed/separation-logic#Source and confidence]]: the concurrent-object-space and case-study material at theorem grade, the rest at triage grade; this document consumes only the concession, which the theorem-grade part covers.

**No recorded corpus claim was contradicted by either source.** Both interchange-strength characterizations at [[../metatheory#Interchange, by layer]] check out: the Gray and invertible level against the substrate's own deadlock-and-diagonals passage and its Gray-tensor statements [@mellies-2021-template-games, sec I, def 1, thm 4], and the lax level against the primary's lax-monoidal statement and its concluding section [@mellies-stefanesco-2020-csl, thm 4.4].
The one sharpening the read produced is the filling-system scope carried at [[#The proof-scope of the derived Hoare inequality]].
