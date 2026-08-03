# Separation logic over the template apparatus — what the heap machinery must supply, and what can be built before it

**Proposed.
Nothing this document proposes is adopted, and the construction it describes does not exist.** It carries the separation-logic half of the template-games line — separated states, the separating conjunction over a permission monoid, and the Frame rule at the predicate level — as a proposal held against heap and reference machinery gandr is committed to and has not built.

It exists because the apparatus half and the logic half arrive on different schedules, and the corpus needs the difference recorded as a schedule rather than as a verdict.
[[../template-games]] owns what imports now: the template and cobordism apparatus, the tile pairing, the polarized footprint, and the six theorems the adoption owes.
This document owns what waits, what it waits _on_, and — the reason it is worth writing before anything is built — **what the waiting half requires of the machinery it waits on**, so the memory model is designed against those requirements the first time rather than revisited after it lands.

* Status: **proposed, and deferred on committed machinery.** The deferral is not a decline and not a scope cut; it is an import order.
  The gating substrate is [[#The gating substrate, and what the deferral is deferred on]], and every requirement it must satisfy is at [[#The requirements ledger]].
* **The deferral is about the carrier's schedule, never about the mathematics.** Every fit fact the read established stands unchanged and is cited to its own statement number here as it is in the component document; what this document changes is the disposition, from unavailable to owed-later.
* **The read's confidence is carried and never upgraded.** Each claim about the sources names the statement it rests on, and the grades are collected at [[#Source and confidence]].
* **One fragment in the tree is pointed at, and it is inert by design.** The polarized-footprint prototype sits beside the decided shift guard and is consumed by nothing; it is described, with its verification, at [[#What can be built before the machinery lands]].

## The gating substrate, and what the deferral is deferred on

**gandr is committed to a real heap and memory model.** The commitment is a design direction with no implementation, which is exactly what a `proposed/` document is for, and naming it precisely is what makes the deferral checkable rather than an indefinite postponement.

**The closest in-tree artifact of that direction is the mode and reference calculus** [[../../surface-language/proposed/modes-and-references]], and it is this document's gating substrate.
Three of its recorded positions decide whether the separation-logic half can be stated at all.

* **References, mutable cells, borrowing, regions, and access modes are neither designed nor built**, and there is no internal value-representation, layout, or address model.
* **The central open decision is what a shared borrow mechanically is** ([[../../surface-language/proposed/modes-and-references#mode-decision-05]]), and its recommendation names a freeze-region or fractional-permission discipline — the fractional half of which is the nearest candidate carrier for a permission structure.
* **Grades are not that carrier and cannot be made into it.** A grade counts uses along a run; a permission divides ownership among simultaneous holders, and the two are not interdefinable.
  Reusing the sealed grade semiring as the permission monoid answers a different question, which is the mode calculus's own central finding.

**The value-semantics floor defers the same machinery to the same place**, so the two documents agree on where the store comes from: references and cells need a store, a region or lifetime discipline, and aliasing control, and [[../../surface-language/value-semantics]] defers all three to the mode calculus rather than fixing them itself.

**And the migration is not finished, which bounds how much of the substrate can be read off the corpus today.** The graded operation signatures the mode calculus depends on are recorded in the proposal document itself precisely because the type-system document that should own them has not been migrated; a requirement stated here against "the type discipline" is therefore stated against a design in transit, not against a settled document.

**So the deferral has a concrete condition, and it is the only one.** The separation-logic half becomes statable when the mode and reference calculus fixes a heap with addresses, a permission or ownership structure over it, and a discipline that says which transitions read and which write.
Until then this document is a requirements ledger and a buildable-now list, and nothing in it is a licence.

## What the import gives once the machinery lands

**Four things arrive together, and they arrive in the order below because each consumes the one above it.**

**Separated states over the heap.** A state splits, and the split is what makes local reasoning mean anything: the source's separated state decomposes three ways [@mellies-stefanesco-2020-csl, def 7.1], with one component playing the frame.
For gandr this is the object that would let a certificate be stated over a part of the store while remaining a statement about the whole.

**The separating conjunction.** The separation product is defined by domain union with permission multiplication [ibid., sec 7.1], and it is the operation the whole logic is named for.
Its payoff for gandr is a compositional statement of ownership that the current carrier can express only structurally, by two derivations touching disjoint positions.

**The Frame rule at the predicate level.** It is not a primitive in the source's model: it is interpreted as the parallel product of a proof's interpretation against an identity cobordism [ibid., sec 9.3], so all of its force comes from the parallel product — a pullback followed by a relabel [ibid., eqns 28-29] — together with the lax-monoidal structure of the cobordism double category [ibid., thm 4.4].
What that buys gandr is not the enlargement-inert property, which is structurally free in gandr's carrier today; it is that the property becomes **a provable property of a semantic model** rather than an artifact of how matching is implemented ([[../template-games#template-games-rung-06]]).

**And the fourth is already half-banked.** The polarized footprint is the separation-logic half's own independence notion, and it is the one piece of it that could be prototyped without a heap, because it reads off a transition rather than off a state.
The source's separated-state footprint is stated to be literally the same quadruple as the machine-state one [ibid., sec 7.2], so the datum the prototype already computes is the datum the separated-state model would consume — with the two components gandr has no counterpart for, `lock` and `mem`, absent for different reasons and dispositioned differently ([[#separation-logic-requirement-06]] and [[#What stays out of scope regardless]]).

**One caveat travels with the whole group and is recorded here rather than discovered later.** The Hoare inequality for the **generalized** composition is derived only under a hypothesis on the filling system, verified for the source's code templates and stated explicitly as not necessarily satisfied for the template of separated states [ibid., prop 5.1, sec 5].
So the half this document defers is precisely the half where that hypothesis is unestablished, and a later pass that leans on the conditional derivation inherits it ([[../template-games#The proof-scope of the derived Hoare inequality]]).

## The requirements ledger

**This is the section the deferral exists for.** Each row states one construct of the separation-logic half, the machinery it requires at the statement that demands it, and what breaks or weakens if the memory model lands without it.
The rows are written to be consumed by the mode-and-reference lane, and each has a stable anchored identifier so a decision there can cite one without quoting it.

**Numbering is stable**: retiring a row leaves its number unused.
Rows 01 through 05 are the chain the separating conjunction and the Frame rule stand on, taken in dependency order; rows 06 and 07 are the two footprint components whose demands are independent of that chain; row 08 is the fork the memory model's own shape decides.

**Nothing in a row is a licence and nothing in one is adopted.** A row is a constraint the machinery would have to satisfy for the deferred half to become statable, stated now so that satisfying it costs nothing extra and failing to satisfy it is a visible choice rather than an accident.

### separation-logic-requirement-01

**An addressed, partial heap the separation product can take a domain union over.**

_The construct._ The separation product $σ * σ'$ [@mellies-stefanesco-2020-csl, sec 7.1].

_What it requires._ A state that is a **partial function from addresses to value-and-permission pairs**.
Both halves are load-bearing: there must be an address set, and the map must be partial, because the product is a union of domains and a total store has no domain to union.

_What breaks without it._ Everything below.
A memory model whose store is reachable only by structural position, or whose store is total, gives the separating conjunction no definition at all, and every statement about it inherits the failure.

_Where gandr stands today._ The store is a directed hypergraph with no address set and no partial-map structure, which is the fit failure the component document records ([[../template-games#Where the fit fails]]).
That is a fact about the carrier as built, and this row is what would change it.

### separation-logic-requirement-02

**A permission structure that is a partial cancellative commutative monoid.**

_The construct._ The permission arithmetic the separation product multiplies over [ibid., sec 7.1].

_What it requires._ Partiality (not every pair of permissions composes), commutativity, and cancellativity.
The candidate carrier is whatever the mode calculus chooses for shared-XOR-exclusive access, and its own recommendation names fractional permissions as one of the two mechanisms in scope ([[../../surface-language/proposed/modes-and-references#mode-decision-05]]).

_What weakens without cancellativity._ A frame is no longer determined by a whole and a part, so a rule that quantifies over "the rest of the state" quantifies over something the state does not pin down.
**That consequence is a structural reading of the definition and not a lemma quoted from the source**, which asserts the monoid conditions at the definition rather than deriving their uses.

_The hazard to avoid, stated because it is the cheap wrong answer._ The sealed grade semiring is not a permission monoid.
Grades bound how many times a thunk is forced; permissions divide ownership among concurrent holders, and the mode calculus's central finding is exactly that the two axes do not substitute for one another.

### separation-logic-requirement-03

**A state that splits with an owned role and a framed role.**

_The construct._ The separated state [ibid., def 7.1] — the Code's part, the per-lock resource map, and the Frame's part, with the source's own gloss at the definition naming the first as the memory owned by the Code and the third as the part owned by the Frame or the Environment [ibid., sec 7.1]; the Frame rule's interpretation consumes the split [ibid., sec 9.3].

_What gandr needs of it, and what it does not._ The triple's middle component is the lock-resource family, and this document cuts locks permanently ([[#What stays out of scope regardless]]), so the lock-free residue of the definition is a **two-role** split — owned against framed — and that residue, not the triple, is what this row asks the memory model to supply.
The demand is decomposability of the heap into those two distinguished roles rather than merely into disjoint pieces — and, from the apparatus side, the environment polarity a template forces, because the split between steps the ambient may perform and steps this certificate performs is what makes the frame role mean anything ([[../template-games#The environment polarity a template forces]]).

_What breaks without it._ The Frame rule has nothing to quantify over.
A heap that splits with no owned-against-framed role distinction supports disjointness but not framing, which is a weaker property than the one this half is imported for.

### separation-logic-requirement-04

**A predicate domain over the heap, and the template formalism to index it by.**

_The construct._ The predicate semantics and the predicate-indexed colouring, over the template the source writes for separated states — the predicate grammar and its satisfaction relation [ibid., sec 7.1], and the predicate indexing of the internal category [ibid., sec 7.3].

_What it requires._ Two things from two different directions.
From the memory model: a domain of assertions about heaps, closed under the separating conjunction, which requires rows 01 through 03 first.
From the apparatus: the template formalism itself, whose availability over asynchronous graphs rides finite completeness of the ambient, which is bought by the tile axioms rather than by the presheaf route ([[../template-games#template-games-rung-02]]).

_The concrete content, carried so the consuming lane can proceed without the source._ The grammar's separating fragment is `emp`, the separating conjunction `P ∗ Q`, the permissioned points-to `v ↦^p w`, and variable ownership `own_p(x)`; satisfaction of the separating conjunction is by state split — `σ ⊨ P ∗ Q` exactly when `σ = σ₁ ∗ σ₂` with `σ₁ ⊨ P` and `σ₂ ⊨ Q` [ibid., sec 7.1].
And the rule this half is imported for, stated once: from `Γ ⊢ {P} C {Q}` infer `Γ ⊢ {P ∗ R} C {Q ∗ R}` [ibid., fig 1].

_What weakens without the apparatus half._ The predicates can still be written, but they index nothing, so the logic has assertions and no model — which is the position the corpus would be in if the heap landed before the tile-level gate reported.

_A scope that travels with this row specifically._ The filling-system hypothesis is the one the source states may fail for the separated-states template [ibid., prop 5.1, sec 5], so a proof layer built on this row inherits an unverified hypothesis and owes its verification for whatever template gandr's proof layer becomes.

### separation-logic-requirement-05

**The cobordism construction, in gandr's virtual variant, before the Frame rule is a rule about anything.**

_The construct._ The Frame rule as the parallel product against an identity cobordism [ibid., sec 9.3], with its force supplied by the parallel product [ibid., eqns 28-29] and the lax-monoidal structure [ibid., thm 4.4].

_What it requires._ Rows 01 through 04, plus the cobordism double category itself — which gandr can only have in a **virtual** variant, because the source's construction requires the ambient to have pushouts precisely so that horizontal composition is total [ibid., thm 1.2, thm 2.2] while gandr's certificate composition declines on the acyclicity gate ([[../template-games#template-games-rung-04]]).

_What is forfeited if the heap lands without this._ Nothing that gandr has today, and that is the honest statement: the enlargement-inert property remains structurally free in the carrier.
What is forfeited is the upgrade — the property never becomes provable about a model, and the compositional independence story for disconnected components stays out of reach ([[../template-games#template-games-rung-06]]).

_The variant's coherence is in neither source_, so this row is a construction gandr owes rather than one it imports, and the open question is [[../template-games#template-games-question-04]].

### separation-logic-requirement-06

**Allocation and deallocation events, with freshness visible per transition.**

_The construct._ The `mem` component of the footprint quadruple — the addresses a transition allocates or deallocates — whose independence condition is plain disjointness [ibid., sec 3.2.1].

_What it requires._ A notion of freshly allocated or deallocated address that a transition carries, so that two transitions can be compared on their allocation sets without consulting the rest of the state.

_What breaks if addresses are recycled invisibly._ Plain disjointness stops being decidable per transition, and the conjunct has to be replaced by a reachability test over the heap — which is the connectivity-closed footprint the read ruled out at the wrong level, and which makes nearly every pair dependent in a connected structure ([[../template-games#Footprints are polarized, and that is what licenses more]]).

_Where gandr stands today._ The candidate counterpart is internal-wire freshness, marked **plausible** and not established, and the binder that would carry it does not exist: no form closes a cycle and no internal-wire binder is built ([[../circuit-terms]]).
So this row is owed from both ends at once.

### separation-logic-requirement-07

**Access classified as read against write, per transition, and never flattened.**

_The construct._ The `rd` and `wr` components and the four-condition independence relation over them [ibid., sec 3.2.1], of which only two conditions are plain disjointness.

_What it requires._ That the memory model expose, for each transition, which locations it reads and which it writes, as two sets and not one.
Two footprints may share read locations freely; only a write against anything collides.

_What weakens if the heap reports only touched locations._ The polarization collapses to the plain-support reading, which is the Mazurkiewicz trace-monoid reading the corpus already records — coarsest-safe and strictly weaker, refusing exactly the read-only overlaps the polarized reading licenses ([[../../metatheory#The certificate algebra]]).
That would give back the one place the read found where the source lets gandr earn more than it does today.

_This row is a preservation requirement rather than an acquisition one._ gandr already has the polarized datum at the term level ([[#What can be built before the machinery lands]]); what row 07 asks is that the heap interface not lose it.

### separation-logic-requirement-08

**A decision, at the memory model's own design time, on whether there is a global heap at all.**

_The construct._ The parallel product, which in the source is a **synchronizing** pullback over a shared object [ibid., eqns 28-29] — there is no disjoint parallel product anywhere in it, and the degenerate case where the shared object is trivial is not stated.

_What it requires._ Not a feature but a fork, and the memory model is what settles it: either gandr introduces a global ambient heap and recovers disconnection as a decomposition of it, taking the source's route and its preservation results with it, or it keeps disconnection primitive with region-local heaps and owes the disjoint case itself.

_What each branch costs._ The first is a real design commitment and not a free reading, because gandr's disconnection is today the absence of a shared state rather than a decomposition of one.
The second leaves the preservation results owed in the disjoint case, since the source states them only for the synchronizing product.

_Neither branch is free and nothing here chooses._ The fork is the component document's open question [[../template-games#template-games-question-03]], recorded as a requirement row because the choice is made by the machinery this document waits on, not by the apparatus it rides.

## What can be built before the machinery lands

**The point of this section is that the deferral should cost as little as possible.** What follows is machinery-independent — it reads off transitions and terms rather than off a heap — so building it now does not pre-commit the memory model and does not have to be revisited when the memory model arrives.

| item                                                             | grade                                                                                         |
| ---------------------------------------------------------------- | --------------------------------------------------------------------------------------------- |
| the polarized footprint, as a transition-attached datum          | **built**, and inert by design                                                                |
| the read-against-write independence verdict                      | **built**, and measured against the decided guard                                             |
| the frame as the excluded class of the match image               | **built**, as a classification and not as a rule                                              |
| the polarization substrate on cells                              | **built**, and shipped before this line was read                                              |
| the footprint's generic shape, for a later heap-carrying carrier | **interface only** — the generic exists and no carrier with a heap exists to instantiate it   |
| the allocation component of the footprint                        | **not buildable now** — the binder it would read off does not exist                           |
| a permission carrier chosen ahead of the mode calculus           | **deliberately not built**, because building it here would pre-empt the decision that owns it |

Each row is stated in full below, because a grade with no evidence behind it is a claim rather than a record.

**The polarized footprint is built, and nothing consumes it.** `gandr-theory-computads`'s `footprint` module carries `MatchFootprint`, which splits the addresses a redex covers into `written`, `read`, and `framed`; `match_footprint` reads that datum off one `CellApp` firing in one term; and `footprint_independence` decides the two read-and-write conjuncts, returning `Independent`, `WriteWrite`, or `ReadWrite` with the colliding address.
The module's own status is that it is a prototype consumed by nothing, with `derive_shift_equivalence` remaining the only licence for a shift — which is the guard fence the component document binds it to ([[../template-games#template-games-question-01]]).
**Verified against the tree at write time**, at the named module and symbols.

**The measurement the prototype was built for has been taken, and it is a differential rather than an adoption.** The integration suite `footprint` compares the polarized test against the decided guard on a fixture family, and its four-cell comparison is the deliverable: pairs licensed by both, pairs licensed by the polarized test only, pairs licensed by the guard only — asserted **empty** over the whole table, which is the containment a "strictly larger" claim needs — and pairs licensed by neither.
Two of the polarized-only rows are refusals the guard reports at a metavariable position the overlap enumerator counts as a composition seam, and **exactly one of the two is caused by that gap rather than by polarization**: the suite pins the hole-seam pair's whole overlap family as bare metavariables throughout and exhibits a surviving genuine composition seam in the schematic pair's, so repairing the enumerator would stop the guard refusing the hole-seam pair while the schematic pair stays refused either way.
And the schematic row's extra licence is not the read/write polarization itself but a third source its own fixture names: the guard's overlap conjunct is cell-keyed where the polarized test is instance-keyed, so cells that overlap schematically are disjoint at this instance — a licence plain support-disjointness would also grant.
**This document's first revision recorded both rows as the enumerator's**, which the suite's fixtures did not establish, because the schematic row asserted only the refusing overlap's kind; the correction is recorded here rather than applied quietly, since a measurement that conflates the enumerator's gap with polarization's win is worthless.

**The frame rule's structural content is buildable now, and it is already there — as a classification, not as a rule.** The prototype's third class is exactly the addresses a firing neither reads nor writes, and excluding them from the footprint is the whole of the frame read structurally: a transition does not collide at a location it carries through without inspecting, however deeply that location sits inside its match image.
**This is not the Frame rule**, which is a statement about predicates and needs rows 01 through 05; it is the reason the Frame rule will have something to be about when those rows are met.

**The polarization substrate on cells predates this line and needs nothing from it.** A cell carries derived per-metavariable variance and linearity metadata, identified by hole name across the faces, which is what makes a polarized reading of a match image possible at all ([[../../metatheory#Interchange, by layer]]).

**The footprint's generic shape is buildable now only in interface, and the qualification is the honest part.** `MatchFootprint` and the independence verdict are generic over the cell alphabet, with addresses being the alphabet's own position type, so a later carrier that addresses a heap instantiates the same shapes rather than re-inventing them.
What does not transfer for free is three modelling choices the prototype makes for term positions and would have to re-decide for heap addresses: the spine above a redex is excluded from the footprint; the write test is node-local rather than subtree-local; and addresses are absolute term positions, which **move** in this carrier where the source's do not.
The module names the last of these as the concrete shape of the debt an adoption takes on — a residual map on positions, which `CellApp` has no room for — and the suite pins it with a fixture where a relocated frame is refused conservatively rather than licensed wrongly.

**The allocation component is not buildable now, and the reason is a missing construct rather than a missing decision.** The `mem` conjunct needs a freshness event to read off, gandr's candidate is the internal-wire binder, and no internal-wire binder exists.
That is [[#separation-logic-requirement-06]] seen from the build side.

**And one thing is deliberately not built, which is as much a part of not-revisiting as the rest.** A permission carrier invented here, ahead of the mode calculus, would pre-empt the decision that owns it and would almost certainly be the grade semiring wearing a different name.
The requirements ledger exists so that the calculus can choose a carrier _knowing_ what the separation-logic half will ask of it, which is a different and better thing than choosing one now.

## What stays out of scope regardless

**These are cuts and not deferrals: nothing the memory model becomes brings them back**, and they are recorded here so a later pass does not re-propose them on the strength of the heap having landed.

**Locks, critical sections, and resource invariants.** They appear in both footprints [@mellies-stefanesco-2020-csl, sec 3.2.1, sec 3.2.2], in the machine model's acquire and release transitions, throughout the change-of-locks development [ibid., sec 8], in the indexing of the separated-state model, and in three of the appendix lemmas [ibid., lem E.3, lem E.4, lem E.5].
Deleting them does not leave a smaller theorem, which is why this is a cut rather than a simplification.
**The lock component of the footprint has no gandr counterpart and that is not a gap to be filled**: filling it would mean inventing a shared-resource notion in order to import machinery whose payoff is a race theory gandr does not want.

**Data-race freedom and the stateless model.** A race is **defined** by the tile mismatch between the machine-state model and the stateless one [ibid., sec 3.2.2], and the second half of the soundness theorem is that freedom [ibid., thm 10.6]. gandr has no race notion, so that half has no gandr statement to be about, and the prize the apparatus is adopted for is the first half and the fibration structure and never the pair.

**The lock-indexed apparatus the Frame rule's interpretation is stated over.** The interpretation itself is deferred with the rest of the logic ([[#separation-logic-requirement-05]]); what is cut is the indexing it is stated against — the change-of-locks development and the appendix material that stands on it [ibid., sec 8, sec E.2] — together with the freely generated lock-graph bicategory that development introduces, whose at-most-one-invertible-2-cell choice the corpus already cites for a different purpose [ibid., sec 8].
**That the interpretation survives the removal of its lock indexing is a structural reading, marked as such and load-bearing for this boundary**: the source states the interpretation in the cobordism double category of separated states parameterized by the lock context [ibid., sec 9.3], and it is this reading — instantiate at the empty lock context — that separates this cut from requirement-05's deferral.
The lock-free instance is therefore a construction gandr owes alongside the virtual variant, and it shares the component document's open question ([[../template-games#template-games-question-04]]).

**Sequential-consistency-shaped correctness criteria from the layered line.** That line concedes it supplies **no correctness criterion** for its coherent congruences, and the substitute it names is that any congruence which is a subrelation of the equivalence up to sequential consistency is coherent [@oliveira-vale-mellies-shao-koenig-stefanesco-2022-layered, sec 8].
A criterion that rests on a sequential-consistency assumption is not a criterion gandr can take, because gandr's quotient is defined over a store with no such assumption to lean on; the criterion gandr wants is the asynchronous-graph axioms, which is the same concession read the other way ([[../template-games#template-games-rung-02]]).

**And the lock-shaped half of that line's case studies.** Its protected-object construction fixes a lock interface with a sequential specification and surrounds each method body with acquire and release [ibid., sec 7.1], and its worked certification is a ticket lock [ibid., sec 7.2].
The transferable content is the shape of the argument — an equational theory on traces, if it preserves happens-before, can carry a concurrency argument that would otherwise need a program logic — and **not** the lock, which lands in the same cut as the primary source's.

## Source and confidence

**Written against the same two primary sources as [[../template-games]], plus one third source cited for two statements**, and every grade below is carried from the read rather than re-established here.

The primary [@mellies-stefanesco-2020-csl] was read in full — fifty pages, body plus appendices — at theorem grade for the template/cobordism half, as the component document scopes it, and the substrate [@mellies-2021-template-games] at the grades the component document records.
Nothing in this document rests on a statement of either source that the component document does not already carry at the same number.

**The third source carries a split grade, and the split matters for which of its statements this document may lean on.** For [@oliveira-vale-mellies-shao-koenig-stefanesco-2022-layered], read in the extended-technical-report form the managed library holds [@oliveira-vale-mellies-shao-koenig-stefanesco-2022-layered-tr], the concurrent-object-space and case-study material was read at theorem grade — the coherent-congruence definitions, the concurrent object space, the disjoint tensor and its symmetric monoidal consequence, the protected-object construction, and the ticket-lock certification.
**That grade has since widened rather than shifted**: the object-based construction, the layered model, the nondeterminism section, and the correspondence section were read at theorem grade in a later pass, and the report's concurrent appendix was read with its proofs — the record is at [[../template-games#Source and confidence]].
What remains below theorem grade is the related-work and conclusion material, read at map grade, and the two statements this document cites [ibid., sec 7.1, sec 7.2, sec 8] fall inside the theorem-grade part under either read.

**What is verified against the tree at write time.** The buildable-now section's built rows only, at the named module and symbols: `gandr-theory-computads`'s `footprint` module and its `MatchFootprint`, `match_footprint`, `footprint_independence`, and `FootprintIndependence`; the guard constructor `derive_shift_equivalence` it sits beside; and the integration suite `footprint` whose four-cell comparison and hole-seam attribution are described above.
**What is verified is the module's public interface and its own documented contract**, together with the suite's stated fixture structure — not that any body decides what its documentation says it decides.

**What is not verified, and is load-bearing for the ledger.** No requirement row has been checked against an implementation, because there is no implementation to check it against: each row is derived from the source's definition at its own statement number and from the corpus's record of what gandr's carrier is.
A row's "what breaks" clause is therefore a structural consequence of the definition rather than a failure anyone has exhibited, and where the consequence is a reading rather than a quoted lemma the row says so at the claim.

**Two readings are flagged rather than buried.** That cancellativity is what determines a frame from a whole and a part ([[#separation-logic-requirement-02]]) is a structural reading of the monoid conditions; the source asserts the conditions at the definition and does not derive their uses at a numbered statement this read can cite.
And that the Frame rule's interpretation survives the removal of its lock indexing ([[#What stays out of scope regardless]]) is the reading that places this document's boundary between the cut and the deferral; the lock-free instance is a construction gandr owes, not a statement the source makes.

**No recorded corpus claim is contradicted by this document.** The fit facts, marks, and statement numbers are the component document's, unchanged; only the disposition of the separation-logic half differs, and that difference is the owner ruling this document exists to record.
