# Template games — the cobordism apparatus, and the separation logic that does not come with it

This document owns gandr's adoption of the template-games programme: which objects gandr takes, which half it declines at the door and why, the single pairing the whole transfer turns on, the theorems the adoption owes, and the gate that holds every tile-level transfer until two decidable facts about the landed shift machinery are settled.

It exists as its own component because the apparatus is **semantic-foundations technology rather than an importable presentation theorem**.
Neither source contains a statement gandr can cite to discharge an obligation it already carries, so everything on offer is construction, and the constructions are heavy — which means what adoption buys has to be stated with its price attached, at a length the commissioning ruling cannot hold.

* Status: **an adopted direction with nothing built, and one gate before any tile-level result transfers.** The apparatus was adopted 2026-08-02 on a theorem-grade read of both sources; every rung below is unbuilt, and the transfer is held behind [[#template-games-spike-01]].
* **The read's confidence is carried and never upgraded.** Each claim about the sources names the statement it rests on; the statements the read took on structure rather than line by line, and the gandr-side facts it inferred rather than proved, are marked where they are used and collected at [[#Source and confidence]].
* The ruling that commissioned the read is the device decline and its named replacement direction at [[circuit-terms#The design questions]], recorded there as `circuit-terms-question-16`; the interchange-strength decision the corpus already meets this line at is [[../metatheory#Interchange, by layer]].
  Both are linked rather than restated, and nothing here re-opens either.

## The adopted object, and the declined half

**gandr adopts the template/cobordism apparatus**: the machinery of [@mellies-stefanesco-2020-csl] secs 1-5 and sec 10, standing on the asynchronous template games of [@mellies-2021-template-games].

**gandr declines the concurrent separation logic riding on it, at the door**: separated states, the separating conjunction over a permission monoid, permissions themselves, locks, critical sections, resource invariants, and the data-race half of soundness.

**The reason is one reason and it is about the carrier rather than about taste: the CSL half does not survive contact with gandr's store.** The separation product is defined by domain union with permission multiplication [@mellies-stefanesco-2020-csl, sec 7.1], so it presupposes that a state is a partial function from addresses to value-and-permission pairs over a partial cancellative commutative monoid. gandr's store is a directed hypergraph: there is no address set, no partial-map structure, and no permission monoid.
Every statement about the separating conjunction — the separated-state definition [ibid., def 7.1], the predicate semantics, and the Frame rule — is therefore unavailable without inventing that structure first, and inventing it is not a small move.

**Naming the candidate correctly matters, because the commissioning ruling named it the other way round.** The entry at [[circuit-terms#The design questions]] calls the replacement direction "the separation-logic line", and the read's verdict is that the separation logic is precisely the part that does not come.
The correction is recorded here rather than applied silently to that entry, because the entry's ruling — that the device mapping is declined and that this line is its replacement — stands unchanged; only the name of what arrives has moved.

## What adoption builds

Four constructions, in the order they depend on one another.
None of them exists in the tree, and none is a re-presentation of something gandr already has.

* **An asynchronous structure on the store transition graph**, whose tiles are the licensed shift commutations.
  This is the pairing everything else stands on, and it is the one with a checkable debt attached ([[#The tile pairing, and the two axioms everything gates on]]).
* **Footprints as a polarized, term-derived datum on cell applications** — the triple of cells rewritten, cells and wires matched but preserved, and internal wires freshly bound — with independence defined from that datum rather than declared on the alphabet ([[#Footprints are polarized, and that is what licenses more]]).
* **A template**, an internal opcategory in the cospan bicategory of the store ambient [@mellies-stefanesco-2020-csl, thm 1.1, def 2.1].
  This forces the largest new commitment in the whole programme, because the template's two-player split is a distinction gandr's certificates do not currently draw ([[#The environment polarity a template forces]]).
* **Certificates re-presented as cobordisms** over that template, with composition by pushout followed by relabel [ibid., eqns 18, 22] — in a virtual variant, because gandr's composition declines where the source's is total ([[#Certificates as cobordisms, in a virtual variant]]).

## The tile pairing, and the two axioms everything gates on

**The single most decision-relevant pairing is the tile, and it is exact in both directions.**

| source object                           | statement                                                                                             | gandr counterpart                                                        |
| --------------------------------------- | ----------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| the asynchronous-graph permutation tile | [@mellies-stefanesco-2020-csl, sec 3.1], with the axioms at [@mellies-2021-template-games, sec III-A] | the landed shift-equivalence witness (`gandr-theory-computads`, `shift`) |
| path equivalence modulo tiles           | [@mellies-stefanesco-2020-csl, sec 3.1]                                                               | the shift quotient on certificates                                       |

**An asynchronous graph must satisfy three axioms, and gandr has one of them** [@mellies-2021-template-games, sec III-A].

* **Symmetry** of the permutation tiles — **held by construction.** The shift witness takes its overlap conjunct per ordered pair in both orders, which is what the symmetry axiom asks for.
* **Determinism** — two tiles out of the same corner with the same first leg have the same second leg — **neither claimed nor proved for gandr.**
* **The cube property** — **neither claimed nor proved for gandr.**

**So the gate is these two axioms, and it is a hard one: nothing transfers at tile level before it is answered.** Every downstream result in the source consumes tiles; none produces one, so a tile-level statement borrowed before the axioms hold would be borrowed against a structure gandr has not been shown to have.
The gate's executing scope is [[#template-games-spike-01]], and no rung of [[#The theorems owed]] may be started against a tile-level premise until that spike reports.

**This is also the cheapest experiment in the programme**, which is why it is the gate rather than a deferred obligation: both axioms are decidable questions about a landed artifact, not construction programmes, and a failure witness settles the direction as usefully as a proof.

## The gating spike

### template-games-spike-01

**Prove or refute determinism and the cube property for the shift quotient as built, and test the polarized-footprint fragment beside the decided guard.**

The spike's two halves are the two cheapest facts the adoption decision needs.

* **The axioms.** Decide determinism and the cube property for the shift quotient over the landed shift machinery and its position and overlap substrate.
  If both hold, every tile-level transfer from this line is licensed; if either fails, the failure witness is the decision.
* **The polarized-footprint prototype.** Decide whether the polarized independence test — rewritten versus matched-but-preserved, read off the match image — licenses a strictly larger commuting class than the incomparable-positions conjunct, **without weakening the decided guard**.
  The test is prototyped beside the guard's constructor and never in place of it.

**Scope fence.** The spike changes no guard, adopts no part of the line, and buys facts rather than structure.

**Small**, in the sense the corpus uses for a bounded decidable question over landed code, and it shares a substrate with the guard it sits beside — so running it against the shift machinery is cheaper than running it against a reconstruction.

Tracked as `gandr-ng9.11`, whose scope is exactly the two halves above.

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

| candidate                                                               | verdict                                                                                                                                                                                                                                                                        |
| ----------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **the polarized match image** — rewritten against matched-but-preserved | **the fit.** The only candidate reproducing the read/write asymmetry, which is the source's content rather than its packaging; gandr already ships the polarization substrate as per-metavariable variance metadata                                                            |
| the plain match image (the support)                                     | corresponds only in the degenerate case where every access is a write. This is what the decided guard's first two conjuncts test today, and it is strictly weaker: it refuses commutations the polarized reading licenses                                                      |
| a port set or boundary interface                                        | **wrong level, instructively so.** Footprints range over the machine's addresses, not over the transition's own interface; reading a port set as the footprint makes "shares a wire" mean "dependent", when a wire two applications only read is a permitted read/read overlap |
| the wiring-read support, closed along wires                             | **not the source's notion at any point.** Footprints are shallow and nothing takes a transitive closure; a closure-shaped footprint makes nearly every pair dependent in a connected diagram                                                                                   |

**Component by component, the mapping is uneven and the unevenness is informative.** The read and write sets map to the polarized match image.
The allocation set — plain disjointness there — maps **plausibly** to internal-wire freshness, which the corpus witness plan already carries as the internal-wire binder.
The lock set has **no gandr counterpart whatsoever**, and that absence is not a gap to be filled: it is the load-bearing edge of the declined half.

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
"Forced" means the pairing can be made but only by adding structure gandr does not have; "absent" means there is no counterpart and the row is a decline rather than a gap.

| source object                                                                       | statement                                                                                    | gandr counterpart                                                                         | mark                                                                                                                                                                                                                                     |
| ----------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| asynchronous-graph permutation tile                                                 | [@mellies-stefanesco-2020-csl, sec 3.1]; axioms at [@mellies-2021-template-games, sec III-A] | the landed shift-equivalence witness                                                      | **exact**                                                                                                                                                                                                                                |
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
| separated state, permission monoid, separating conjunction                          | [@mellies-stefanesco-2020-csl, def 7.1, sec 7.1]                                             | none                                                                                      | **absent** — the declined half                                                                                                                                                                                                           |
| machine state, as stack, heap, and locks                                            | [ibid., sec 3.2.1]                                                                           | the cell store                                                                            | **fails at the carrier** ([[#Where the fit fails]])                                                                                                                                                                                      |

## Where the fit fails

Four failures, each stated as precisely as the holdings above.
The first is the reason the CSL half is declined at the door; the remaining three are debts adoption incurs.

1. **The store structure the separating conjunction needs is structure gandr does not have.** Stated at [[#The adopted object, and the declined half]] and not restated here; it is a failure of the carrier, not of the mapping.
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

**A template is an internal opcategory in the cospan bicategory over the store ambient** [@mellies-stefanesco-2020-csl, thm 1.1, def 2.1, sec 2.2], and its content for gandr is one inclusion: the template's object of ambient steps sits inside its object of all steps [ibid., sec 3.3].

**That inclusion is the split between "steps the ambient may perform" and "steps this certificate performs", and gandr certificates are closed-world today.** A gandr certificate records a derivation against an append-only store; nothing in it distinguishes a step the certificate is responsible for from a step the environment took while the certificate was in flight, because the environment is not represented at all.

**This is the largest new commitment in the whole programme, and it is not optional.** The inclusion is what makes the two-player structure exist, and the two-player structure is what makes a cobordism over the template carry more information than its support.
Without it the cobordisms are trivial and the apparatus buys nothing.

**It is also not small.** Representing an environment polarity touches what a certificate _is_, not merely how it is checked, and no part of the corpus currently reserves a slot for it.
The design question is [[#template-games-question-02]].

## Certificates as cobordisms, in a virtual variant

**A certificate would be re-presented as a cobordism over the template**: a support with an input boundary, an output boundary, and a labelling into the template [@mellies-stefanesco-2020-csl, eqn 21], with composition by pushout followed by relabel along the template's multiplication [ibid., eqns 18, 22].

**The pairing is plausible and it is the best structural one the read found**, because gandr's tracelets already have exactly that shape: a derivation with two boundaries, meaningful only against the store it was minted against.

**What does not transfer is totality, and that is what forces the variant.** The source's construction requires pushouts in the ambient precisely so that horizontal composition is total [ibid., thm 1.2, thm 2.2], while gandr's certificate composition declines on the directed band when the acyclicity gate fails, carrying the variable-flow cycle as its diagnostic.
A double category whose horizontal composition declines is a **virtual** double category, and the target of the adoption is therefore a virtual variant of the source's construction.

**The virtual variant's coherence is in neither source.** Nothing in the read supplies the coherence conditions a virtual variant of that construction would satisfy, so the variant is a construction gandr owes rather than one it imports; the obligation is [[#template-games-rung-04]] and the open question is [[#template-games-question-04]].

**One unmatched step is worth naming, because it is a place gandr has less structure rather than more.** The relabel along the template's multiplication records **which side performed each half** of a composite. gandr composition erases nothing at that step — not because it is careful, but because it never made the distinction, which is the same absence the environment polarity names from the other direction.

## The proof-scope of the derived Hoare inequality

**The corpus's claim stands, and this section exists to keep it scoped rather than to weaken it.**

The interchange-strength decision records concurrent separation logic as the level at which interchange is a **non-invertible lax coercion**, with the Hoare inequality **derived rather than postulated** [[../metatheory#Interchange, by layer]].

**That claim is correct, and the read verified what it rests on.** It rests on the lax-monoidal structure of the cobordism double category [@mellies-stefanesco-2020-csl, thm 4.4] — a colimit commuting with a limit up to a non-reversible coercion — and that statement is **unconditional**.
The statement carries no proof in the source text and is asserted from lax-monoidality of the cobordism construction together with the Day and Street theory of symmetric pseudomonoids, which is a reading grade rather than a defect, and it is marked here because the corpus leans on it.

**What is conditional is a different derivation, for a different composition, and the corpus does not currently claim it.** The Hoare inequality for the **generalized** composition is derived only under a hypothesis relating the filling of a parallel pair to the parallel pair of fillings [ibid., prop 5.1], and that hypothesis is verified in the source's appendix B for the **code** templates only.

**The source states explicitly that the hypothesis is not necessarily satisfied for the template of separated states** [ibid., sec 5], with the reason given: a two-player separated state decomposes into a three-player one in several ways.

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

**Grade.** Satisfiability is **inferred, not proved** — the read did not establish that gandr's certificate supports can be presented in that ambient.

### template-games-rung-02

**The tile set is deterministic and satisfies the cube property** [@mellies-2021-template-games, sec III-A].

Symmetry is already had by construction.
These two are unproved for gandr, they are decidable questions about the landed shift witness, and they are the cheapest real experiment in the programme — which is why they are also the gate ([[#template-games-spike-01]]).

**What it would newly warrant.** Every tile-level transfer from this line, and nothing transfers without them.
This rung is a precondition rather than a prize: it licenses the other five to be attempted.

**Grade.** **Not verified, and load-bearing.** Both properties are unproved for gandr and both are preconditions for any tile-level transfer.

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

A comparison map from the certified layer to the operational layer that is both a 1-fibration and a 2-fibration, after the source's soundness theorem and its two halves [@mellies-stefanesco-2020-csl, thm 10.1, thm 10.5, thm 10.6].

**What it would newly warrant — and this is the reason the whole direction was pursued.**

* **Parallel replay soundness.** The 2-fibration half says every commutation the operational layer exhibits lifts to the certified layer, which is precisely the licence a parallel-replay scheduler needs in order to reorder. gandr today has the bracket oracle and the per-pair guard; it has **no theorem that a parallel schedule is the same transformation**, and the 2-fibration is that theorem's shape.
* **A compositional independence story for disconnected components.** The preservation results — the parallel product preserves strictness [ibid., lem E.2] and preserves 1-fibrations [ibid., lem 10.10] — are exactly the shape the disconnection axis needs, and exactly what the declined device line could not give.
  The caveat travels with them: they are stated for the **synchronizing** product, so the disjoint case is a gandr obligation rather than an import ([[#Where the fit fails]]).
* **A stated frame property.** "A certificate valid over a sub-store stays valid in a larger store with the enlargement inert" is structurally free in gandr's carrier today; the value of the import is that it becomes a **provable property of a semantic model** rather than an artifact of how matching is implemented — which is what matters at the moment the store grows disconnection and multi-output.

**Grade, and it is the sharpest cost in the programme.** The source's soundness theorem is proved by induction over a **proof system**, and gandr has no proof system in that sense — so this rung is a **re-derivation, not a citation**.
Three of the lemmas it would follow were read for shape and not verified line by line [ibid., lem 10.2, lem 10.9, lem 10.10, lem E.2], one is asserted without proof in the source [ibid., lem 10.8], and the soundness theorem itself is **attributed to earlier work and re-proved axiomatically** in the source read, so this source is not the primary source for the theorem.
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

## What stays out of scope

Explicitly, so that nothing here is later read as a partial adoption of the whole.

* **Separation logic proper**: the separating conjunction, permissions, separated states, predicate-indexed colouring, and the Frame rule as an inference rule.
* **Locks, critical sections, and resource invariants**, together with the change-of-locks development and the frame-rule interpretation that stands on it [@mellies-stefanesco-2020-csl, sec 8, sec 9.3, sec E.2].
* **Data-race freedom and the stateless model**, because gandr has no race notion and the second half of the soundness theorem therefore has no gandr statement to be about [ibid., thm 10.6].
* **The error monad** [ibid., sec 6], because gandr's three-valued verdict discipline is a different and already-recorded device: a declined check leaves a certificate stuck rather than refuted.
* **Multiplicative-additive linear logic and star-autonomy** [@mellies-2021-template-games, thm 1, thm 4], because gandr is not building a linear-logic model.
* **Higher-order concurrent separation logic and its descendants**, which are the sources' own future work and not a gandr direction.

**One further exclusion is a decline rather than a scope cut, and it is recorded so it is not re-proposed.** The lock component of the footprint has no gandr counterpart, and that is not a gap: filling it would mean inventing a shared-resource notion in order to import machinery whose payoff is a race theory gandr does not want.

## Cost, stated honestly

* **The environment polarity is not optional and is not small.** It is what makes the template's two-player split exist, and hence what makes cobordisms nontrivial at all.
* **Nothing discharges an existing obligation.** The whole programme adds theorems before it removes any, and that is a property of the direction rather than of this scoping.
* **The prize is a re-derivation, not a citation.** The soundness theorem is proved by induction over a proof system gandr does not have.
* **Composition partiality forces a virtual variant whose coherence is in neither source**, so the target of the adoption is a construction rather than an import.
* **Against all four, the direction is real.** The tile pairing is exact, the adhesivity precondition is satisfiable at the right layer, and the polarized fragment is a cheap experiment that either buys a strictly larger commuting class or settles the direction with a counterexample.

## Source and confidence

**Both sources were read at theorem grade for the template/cobordism half on 2026-08-02, from copies held locally, with identity checked from page 1 for each.**

The primary [@mellies-stefanesco-2020-csl] was read in full — fifty pages, body plus appendices.
The substrate [@mellies-2021-template-games] was read for the machinery the primary stands on: sec I, secs III-V, and the statements of the numbered propositions and theorems.

**What was read but not verified line by line, in the primary.**

| statement                                                           | grade                                                                                                 |
| ------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| thm 1.1, thm 1.2, thm 2.2, thm 4.4                                  | **carry no proof in the source text**; thm 4.4 is asserted from lax-monoidality plus Day-Street       |
| lem 4.2, lem 10.8                                                   | **asserted without proof** in the source                                                              |
| lem 10.2, lem 10.3, lem 10.7, lem 10.9, lem 10.10, lem E.2, lem E.5 | proofs read **for shape, not for correctness** — each is a diagram chase followed rather than checked |
| appendix B's map construction                                       | read as a case analysis and **not checked**                                                           |

**What was not read line by line, in the substrate** [@mellies-2021-template-games]: sec VI and its theorem, sec VII and its definition of the template as a monad, and the appendices.
The statements of the numbered propositions, theorems, and definitions were read; **the proofs of none of them were**.

**Imported results were not chased.** Three of them bear on claims above, so they are named rather than gestured at, and each is **locator-pending**: none has a bibliography key yet, and this document cites none of them at a statement.

* **Lack and Sobociński on adhesive categories**, which is where the primary's adhesivity hypotheses come from.
* **Day and Street on symmetric pseudomonoids in a monoidal bicategory**, which supplies the complete definition the lax-monoidal statement is asserted from [@mellies-stefanesco-2020-csl, thm 4.4].
* **The earlier Melliès and Stefanesco asynchronous-soundness paper**, and this one matters most: the soundness theorem [ibid., thm 10.1] is **attributed to that earlier work and re-proved axiomatically** in the source read here, so the source read here is **not** the theorem's original home, and [[#template-games-rung-06]] would be re-deriving a statement whose primary source the corpus does not yet hold.

Also unchased, and named for completeness: Bénabou for the polyad definition, Garner for lax double functors, Mulry and Johnstone for the smash-product lifting, Bourke and Gurski for the Gray characterization, and Melliès' own earlier template-games papers.

**gandr-side facts not verified, two of them load-bearing.**

* **Whether the shift witness tile set is deterministic and satisfies the cube property.** Both unproved, both preconditions for any tile-level transfer, both decidable — [[#template-games-rung-02]] and [[#template-games-spike-01]].
* **Whether the store-transition ambient gandr would need is adhesive.** Satisfiability was **inferred** from the corpus's own record that labelled directed hypergraphs are a presheaf topos; it was not proved that gandr's certificate supports can be presented in that ambient, and the argument fails if they must be computads — [[#template-games-rung-01]].
* **The three-conjunct guard was read from the shift module's own documentation rather than from its body.** Verified against the tree at write time for this document: `gandr-theory-computads`'s `shift` module exists and documents the three conjuncts and their order, `derive_shift_equivalence` is its constructor, `ShiftObstruction` is the typed refusal, the overlap conjunct is asked of the cell pair through `overlaps_between`, and the convexity conjunct is carried as a `ConvexityDischarge` datum whose two inhabitants are the left-connected-over-acyclic-target discharge and the re-check-required refusal.
  **What is verified is the interface and its documented contract, not that the body decides what the documentation says it decides.**
* **No bibliography key beyond the two named sources was checked by the read.** The keys cited in this document were checked against `bibliography.yml` at write time.

**No recorded corpus claim was contradicted by either source.** Both interchange-strength characterizations at [[../metatheory#Interchange, by layer]] check out: the Gray and invertible level against the substrate's own deadlock-and-diagonals passage and its Gray-tensor statements [@mellies-2021-template-games, sec I, def 1, thm 4], and the lax level against the primary's lax-monoidal statement and its concluding section [@mellies-stefanesco-2020-csl, thm 4.4].
The one sharpening the read produced is the filling-system scope carried at [[#The proof-scope of the derived Hoare inequality]].
