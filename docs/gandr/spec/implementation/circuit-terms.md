# Circuit terms — computing with circuit-algebra terms

This document owns the circuit-term lane end to end: what it means for gandr to _compute_ with terms of the full circuit algebra, what the tree carries today, what must be built, and in what order.
It exists as its own component because the lane crosses every layer — the sequent IL, the L machine, the cell alphabet, the matching and normalization engines, the description universe, the checker, and the surface — and because the [[../metatheory#The substrate is the full circuit-algebra rung|generality ruling]] makes circuit structure a feature the rest of the language is expected to be designed _around_ rather than one that arrives late.

* Status: **design component; the substrate is audited and nothing is built.** Both ends of the multi-output special case exist and the middle is empty; the other three axes are not representable anywhere above the carrier.
  Every as-built claim below names the crate and symbol it was verified against.
* The **rewriting question is settled at theorem grade** as of 2026-08-01: the applicable instance is convex double-pushout rewriting with interfaces over monogamous acyclic hypergraphs, its fragment matches three of gandr's four axes exactly, and confluence there is decidable.
  What that pass opened is smaller and sharper than what it closed — a convexity hazard on a TCB-adjacent quotient, the wheel axis falling outside every published statement, and the fan-out obligation the retired asymmetry had hidden.
  The first two are now answered in place: the quotient's guard gains a convexity re-check with a left-connected discharge, and the wheel axis is carried by matching the **cut-open** form alone, under the delay's own path extension.
* The carrier-side facts are landed and machine-checked in [[../metatheory/carrier]]; the surface half of the same question is the design sketch [[../surface-language/circuit-cells]], whose concrete syntax is deliberately unsettled and lands last.
* The mathematics of the arity is the metatheory track's [[../metatheory#Cellular data — descriptions, cells, and computads|bridge-diagram account]]; nothing here proposes changing the carrier.

## The scope, and why multi-out alone is the wrong frame

A first pass at this lane scoped it as **multi-out** — operations with more than one result.
That framing is a special case of the real task and it hides three of the four axes.

**Arity is the retired axis.** The guards ledger tombstones "restrict to dioperads, therefore give up many-out" with the reason that dioperads have the same colour set as properads, and that what the higher rungs add is **reconvergence, disconnection, and wheels** rather than arity.
A lane organized around "more than one return value" answers a closed question.

So the lane is: **make computing with circuit-algebra terms work across the whole language.** Its axes, with the carrier status the metatheory track establishes:

| axis              | on a diagram                                     | carrier                                                                                 | what the rest of the system lacks                          |
| ----------------- | ------------------------------------------------ | --------------------------------------------------------------------------------------- | ---------------------------------------------------------- |
| **multi-output**  | one cell emits to several destinations           | expressible; the bridge diagram is its arity                                            | the term face, the machine, and the cell alphabet          |
| **reconvergence** | two paths out of one cell rejoin at another      | expressible; excluded from a `Cell` only by simple connectivity                         | a term has one root, so a rejoin has no spelling           |
| **disconnection** | two sub-diagrams with no wire between them       | expressible **and proved** — a merge of two connected shapes has exactly two components | the block grammar has one spine                            |
| **wheels**        | an output feeds back, through cells, to an input | expressible; wheel-freeness is _derived_ from simple connectivity                       | no form closes a cycle, and no internal-wire binder exists |

Multi-output remains the entry point because it is the axis whose two ends are already built, and because the Π-layer half is free.
It is not the destination.

**Three further faces belong to this lane and have no home elsewhere**, because they are what "computing with" means beyond "representing":

* **matching** — what it means for a circuit pattern to match a circuit term once the pattern is not a tree;
* **normalization** — what a normal form _is_ for these terms, and whether it needs machinery of its own;
* **the crate boundary** — the proposal that this machinery lives in a new `theory-circuit-algebras` beside the existing theory crates, rather than growing inside `theory-computads`.

## Many-out, fan-out, fan-in, supply, and Frobenius, in plain terms

This vocabulary recurs below and in the cited literature, so it is stated once here with examples rather than assumed.
An earlier revision of this section stated a **fan-out/fan-in asymmetry that the sources and gandr's own carrier both contradict**; the corrected three-way split is below, and the correction is recorded rather than quietly applied because the retired claim was load-bearing for the aggregation split.

Three things get confused under two words, and the corpus needs all three apart.

* **Many-out** is one cell with several output _ports_, each carrying a different result to one destination.
* **Fan-out** is one _wire_ going to two places — the same value arriving twice.
* **Fan-in** is two wires arriving at one place.

**Many-out is free**, in the exact sense that it needs no structure on the type: a cell's several results are named by the arity's target map, and nothing has to be decided.
This is the Π-layer of the bridge diagram, and it is what "routing is free" correctly names.

**Fan-out and fan-in are the mirror images of each other, and they cost the same thing.** Two wires arriving at one place have to become one thing, and nothing in the wiring says how: the target must answer "what are these two contributions, together?", and that answer is a binary operation.
For the diagram to denote anything, the operation must not depend on which contribution the wiring happens to present first (**commutativity**) or on how a three-way fan-in is bracketed (**associativity**), and an empty fan-in must mean something (**a unit**) — which is exactly a **commutative monoid**.
Dually, one wire arriving at two places has to become two things, and that is a **cocommutative comonoid**: a copy with a discard.
Neither is wiring; both are generators with laws.

**gandr's carrier already says so, and this is the check that settles it.** `Match` pairs every source with exactly one partner and every sink is hit exactly once ([[../metatheory/carrier]], machine-checked) — so no wiring datum in the carrier can name two targets for one source, and fan-out is no more expressible there than fan-in.
The surface says so too: `dup(v)` and `drop(v)` are **ordinary computations**, not wiring ([[../surface-language#Expressions: every form]]), and grades exist precisely because duplication is priced.
The literature says so a third time: the image of the syntax under the Frobenius-free hypergraph correspondence is exactly the **monogamous** acyclic cospans — every node of in-degree and out-degree at most one [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-ii, thm 25; coloured, thm 27].

The retired asymmetry was a **cartesian** intuition: in a cartesian category every object carries a comonoid, so copying looks free while merging does not. gandr's cell layer is resource-sensitive — session channels, linear resources, affine capabilities, grades — which is precisely the setting where that intuition does not hold.
The consequence for the rest of this document is that **the per-type-supply pattern below has two rows where it had one**, and that the aggregation obligation has a dual the corpus had not named.

```text
// free: many-out. one cell, two distinct results, each to one destination.
*divmod(-m: Nat, -n: Nat, +q: Nat, +r: Nat)

// not free: one source, two destinations, the same value twice. a `copy` on
// Pipe must be a cocommutative comonoid.
*split(-p: Pipe, +l: Pipe, +r: Pipe)

// not free: two sources, one destination. an `append` on Pipe must be a
// commutative monoid, or the diagram does not denote anything.
*merge(-p: Pipe, -q: Pipe, +r: Pipe)
```

**A supply is that obligation promoted from a note to a structure.** Saying "`Pipe` has a commutative monoid" is a fact about one type.
Saying a category **supplies** a prop means every object carries that structure, compatibly with the monoidal product and unit [@fong-spivak-2020-supply].
The practical difference is what one no longer has to check: the ambient coherence isomorphisms — associators, unitors, braiding — are **automatically** homomorphisms for any supply, so the compatibility that would otherwise be re-verified at every cell comes for free, and the supply survives passage to the strictification, which matters because gandr's carrier is strict.
**The dividend is scoped to what the definition quantifies over** (owner sign-off, 2026-08-02): the source's supply places the structure at _every_ object, with a coupling condition across the tensor — so a **per-type** structure is not a supply in that sense, the dividend is not earned by it, and the mixed case owes its own homomorphism argument wherever it wants one ([[#circuit-terms-spike-03]]).

**Frobenius is the maximal version of that.** A _special commutative Frobenius monoid_ on an object is a commutative monoid (merge, and an empty-merge unit) **plus** its mirror image, a cocommutative comonoid (split, and a discard), satisfying laws that make the two interact so freely that **any connected diagram built from them with $n$ inputs and $m$ outputs collapses to a single $n → m$ generator** — the spider theorem [@coecke-duncan-2011-interacting-observables].
A **hypergraph category** is a symmetric monoidal category in which _every_ object is supplied with one [@fong-spivak-2019-hypergraph-categories].

That is what "supplies Frobenius" means, and why the phrase is load-bearing: in a hypergraph category, splitting, merging, initializing, and discarding a wire are available on **every** type, unconditionally, and connectivity is all that survives of a diagram.

**Practically, for gandr, the split means this.** Where a target carries the structure, fan-in is a lawful cell and the spider normal form applies.
Where it does not, a diagram that fans in is **not** a wiring question a picture can settle — it is an unmet obligation, and the surface should name it at the declaration rather than let the drawing imply it.
The whole aggregation split is that sentence.

## Why free fan-in is declined, and under what condition that is revisited

**A decline is a claim, and it carries a revisit obligation.** This project has repeatedly found that a restriction adopted because "we do not know how to extend the system while keeping the properties we need" was later dissolved by literature that was newer, or simply never found the first time.
So the standing discipline is that meeting a recorded decline obliges the reader to ask **is this still necessary, and if so why** — and that a decline which cannot answer with a current reason is a defect, not a rule.
The recorded reversal conditions exist to make that question cheap; this section states the current answer for the fan-in decline specifically, so a later pass re-opens it against reasons rather than against silence.

Ambient free fan-in is declined for three reasons, in order of force.

* **It would make structural an invariant that gandr deliberately made refutable.** The governing principle is stated once and binds everywhere: _an invariant can be structural or refutable, never both in one type_.
  If every object supplied Frobenius, "this diagram fans in lawfully" would be true by construction and the obligation would stop being checkable — the same failure mode as a wheel guard becoming unfalsifiable when no program can write a wheel.
* **It is not true of gandr's targets.** A session channel, a linear resource, and an affine capability do not carry a commutative monoid, and supplying one would silently license merging two channels or two capabilities into one.
  The obligation is not bureaucracy; the types genuinely differ in whether they have it.
* **It sits at the wrong rung.** gandr's carrier is the **nonunital (downward)** circuit-algebra rung: the wiring datum pairs every source with a partner, no constructor pairs two sinks, and consequently the nodeless loop is inexpressible and no scalar has to be assigned to a free loop.
  Frobenius structure brings cups with it, and the standing instruction is explicit — if a cup is ever added, three consequences go at once, and **a cup must not be added merely to make an operation total**.

**The reversal condition, stated so it is checkable.** The decline covers _ambient, unconditional_ supply.
It does not cover per-type supply, which is exactly what this lane wants: a target that has the structure gets the cell, and the supply notion is the right shape for recording that it does.
So the reversal condition is narrow — **a construction that makes "this type supplies the structure" a checked, per-type judgement rather than an ambient assumption** dissolves the decline without touching the rung, and that construction is what [[#circuit-terms-spike-03|circuit-terms-spike-03]] goes looking for.

## Per-type supply as a general decline-relaxation pattern

The fan-in decline dissolves not by admitting the structure everywhere but by making "this type has it" a **checked per-type judgement**.
That move is more general than the case that produced it, and it is worth stating as a pattern, because several of this corpus's declines have the same shape: _we cannot have X, because X ambient would destroy a property we need_.
Whenever a decline has that form, the question to ask before treating it as settled is whether the property survives when X is **supplied per type and checked** rather than assumed.

The pattern's ingredients, in the order they have to be established:

* a **carrier of the obligation** — what it means for one type to have the structure, expressible in gandr's own formers rather than as an ambient axiom;
* a **checked judgement** — the obligation is discharged at the declaration, with a decline and a diagnostic when it is not;
* a **preserved refuter** — the invariant stays falsifiable, because a program can still write the thing that lacks the structure and be told so.

Four standing declines are candidates for exactly this treatment, and each is recorded here as a candidate rather than as a proposal.

| decline                                   | what per-type supply would change                                                                                                                                                                                                                                                  |
| ----------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **ambient free fan-in** (the case above)  | fan-in cells become lawful exactly where the commutative monoid is supplied, and unlawful with a diagnostic elsewhere                                                                                                                                                              |
| **ambient free fan-out** (the case above) | copy cells become lawful exactly where the cocommutative comonoid is supplied. The row exists because the fan-out/fan-in asymmetry did not survive; grades and `dup`/`drop` are already the value-side answer, so the open half is what the obligation means at the **cell** layer |
| **Frobenius structure**                   | a type that supplies Frobenius gets the **spider normal form** — a decision procedure for connected diagrams — without every type getting split, merge, init, and discard                                                                                                          |
| **the cup**, and with it compact closure  | **retired into the row above rather than carried**: a per-type _unital_ Frobenius supply _is_ a per-type cup, and the nonunital supply is not                                                                                                                                      |

**The spider normal form is what makes the third row attractive.** Where a type supplies Frobenius, "are these two connected diagrams equal?" collapses to comparing a many-to-many generator, which is a decision procedure gandr does not otherwise have for that fragment.
The procedure is delivered through canonicalization at the representation, never by running the interaction rules to completion — the running route does not converge ([[#circuit-terms-question-20]]).
Getting it per type, on the types that genuinely have the structure, is a strictly better trade than either getting it everywhere or not at all.

> **The normal form is a triple, not a pair, and the corpus's statement of it is the special case.** For a general Frobenius algebra, any map $A^{⊗m} → A^{⊗n}$ presented by a connected diagram equals the standard diagram with $m$ in, $n$ out, **and $j$ beads**, where $j$ is the number of bounded connected components of the diagram's complement — its cycle rank [@majid-rietsch-2021-planar-spider, cor 2.4].
> A closed diagram, $m = n = 0$, is therefore a **scalar**, one per bead count.
> The familiar "collapses to a single $n → m$ generator" is what happens when the algebra is **special**, because speciality sets the bead to the unit and the count stops being observable.
> **gandr has already ruled against exactly that move at the carrier**: circles are counted rather than discarded, `Code a b = Shape a b × ℕ`, and the recorded reason is that discarding sets a value to `𝟙` where `𝟘` was available and leaves no term able to witness the other answer.
> So the bead count and gandr's circle count are **related but not identical, and the identification is corrected in place** (owner sign-off, 2026-08-02): the carrier's own `Circuit.selfloop` has cycle rank one while its `ℕ` is zero, so the invariant a per-type supply must compare is **(m, n, j)** with **j = β₁(Shape) + ℕ** — the shape's own cycle rank plus the carried count — never the bare `ℕ`.
> This sharpens [[#circuit-terms-spike-05|circuit-terms-spike-05]] from "is the collapse checkable" to "is the bead count computable on gandr's representation" — and it is, near-linearly, by union-find over the port bijection with the carried count added; the executed warrant spike works an example.
> Content addressing that interned on the bare `ℕ` would identify diagrams the triple separates, which is why the correction is load-bearing for the canonicalization route rather than cosmetic.
>
> **A planarity condition travels with that citation, and it is a condition rather than a bar.** The general theorem is proved for **planar** connected diagrams, and planarity is what buys the generality: it is how the result reaches **noncommutative** algebras and **asymmetric** Frobenius forms.
> A first reading says gandr must refuse it, because the parallel direction stays symmetric and ordering it "would be a silent catastrophe".
> **That reading is too quick, and the corpus refutes it two sections away**: the merger already **is** a planar tensor — the base is the free monoid on the colours, which is not commutative — and canonicalization is precisely "the passage from lists to multisets at the objects, which is what makes the Day convolution symmetric" ([[../metatheory#Interchange, by layer]]).
> So gandr does not run a symmetric representation.
> It runs a **planar representation with symmetry recovered at the quotient**, and `canon-sound` is the bridge.
>
> **The opposition is therefore not symmetric-versus-planar but _at which layer the order lives_**, which is the corpus's standing pattern rather than a new one: ordering is a section, never a planarization of the theory.
> A planar theorem is consumable at the representation layer through exactly that section — the shape being a canonical boundary permutation composed with the standard form, with the permutation canonicalized the way `Rigid`'s recipe already canonicalizes construction terms (permutations outermost, ordered monomials, unique minimal representative).
> What that would cost is one more `canon-sound` instance and the **monomial-to-monomial check**, which the Frobenius relations satisfy on their face — they equate single diagrams, not sums — and which the corpus requires be checked before anything leans on it.
> What it is **not** licensed to do is make the theory planar; the bracket oracle's symmetry is over **disjoint** primitives, and whether that direction separates cleanly from the within-component order the spider theorem uses is unestablished.
> Recorded as [[#circuit-terms-question-21|circuit-terms-question-21]], because the reason to want it is concrete: `append` on a `Pipe` is the aggregation section's own motivating example and it is **not commutative**.

**That row now has a construction rather than a hope, and it is the literature's own worked case.** A per-type supply of Frobenius structure, expressed _inside_ a Frobenius-free symmetric monoidal theory, is a pair of generators $\{μ : 2 → 1, δ : 1 → 2\}$ on that type with the Frobenius equations oriented as rules — which is the theory of **Frobenius semi-algebras**, the first case study of the correspondence paper.
Three facts transfer with it, each cited at its own statement:

* it is **terminating**, by a lexicographic reduction ordering that counts µ-trees and µ→δ paths [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-ii, thm 44];
* **acyclicity is load-bearing in that proof** — the authors state that if the two hyperedges of a Frobenius rule's left-hand side lay on a directed cycle, an infinite rewrite sequence is possible — so the supply and the wheel axis are **not independent**, and admitting wheels costs the termination argument rather than merely the representation;
* it is **not confluent** — unhedged, the source's own example exhibiting one diagram with two distinct normal forms [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-iii, ex 5.3] — so the spider form is **not delivered by running these rules**; the decision procedure arrives through canonicalization at the representation ([[#circuit-terms-question-21]]), and the convexity hazard under [[#The correspondence at gandr's own rung, at theorem grade]] is the same example wearing its other hat.

> **The fan-in and fan-out rows are not independent, and the table read as four independent rows hides a fork.** A type that supplies **both** a monoid and a comonoid must say how they interact, and there are two canonical answers with opposite rewriting behaviour.
> **Frobenius** — the spider law — makes connected diagrams **contract** to the standard form.
> **Bialgebra** — copy-then-merge — makes them **expand**: the correspondence paper's own bialgebra case study notes that repeated application of one of its rules "can significantly increase the number of hyperedges", and it needs a five-component lexicographic order to prove termination at all — four path- and hyperedge-counting orders, plus the one the Frobenius case already used [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-ii, thm 46].
> The two are the field's two canonical theories over the same generators, and they are not variants of one thing.
> **So the supply table owes a fifth entry that is not a row but a question**: when both are supplied on one type, which interaction law is supplied with them — and the answer changes whether that type's fragment shrinks or grows under rewriting.
> The structure in which both coexist coherently is a **Hopf-Frobenius algebra**, two Frobenius algebras and two Hopf algebras sharing structure maps [@collins-2024-hopf-frobenius].
> Its genericity result — that the conditions are minor — is a statement about the category of **vector spaces**, resting on integrals and the Larson–Sweedler theorem; gandr's carrier is combinatorial at `Set`, so **nothing about how easily the two coexist transfers**, and the fork stays a real decision rather than a formality.
> Recorded as [[#circuit-terms-question-20|circuit-terms-question-20]].

**The pattern's third ingredient — a preserved refuter — survives, and checking it is what makes this a supply rather than a loophole.** Because the structure is carried as _generators on one type_ rather than as an ambient assumption, a type that does not declare them simply has no such generator: a fan-in cell over it does not typecheck, and the diagnostic is at the declaration.
The invariant therefore stays falsifiable in the corpus's own sense — a program can still write the thing that lacks the structure and be told so — which is exactly what an ambient supply would have destroyed.
The witness plan below carries the refuter for both directions.

**The fourth row is settled, and the corpus's caution was right for a reason it had not yet named.** Full (co)unital Frobenius algebras **always induce a compact closed structure**, which is stated in that same case study as the reason the semi-algebra fragment is worth isolating at all [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-ii, sec. 5.1].
So a per-type _unital_ Frobenius supply is a per-type cup by construction, and the standing no-cup instruction applies to it verbatim.
A **nonunital** Frobenius supply — dropping the unit and the counit — carries no cup, and lands on the rung gandr's carrier already occupies for four independent reasons ([[../metatheory#Cellular data — descriptions, cells, and computads]]).
The disposition is therefore: **the cup row is retired in favour of the Frobenius row read nonunitally**; the negative-and-fractional-types line stays the worked account of what a cup would buy and cost [@chen-sabry-2021-negative-fractional], and nothing here proposes adding one.

## The substrate, layer by layer

Verified against the tree at the time of writing, symbol by symbol.

| layer                                                   | as built                                                                                                                                                                                                                                                                                           | verdict                                                                        |
| ------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------ |
| IL grammar (`core-sequent/src/il.rs`)                   | `ProducerNode::Ctor`, `ConsumerNode::Dtor`, `CommandNode::Prim`, and `CommandNode::Jump` each carry `cs: Box<[ConsumerId]>`                                                                                                                                                                        | **representable** — no format break owed                                       |
| typed-IL checker (`core-sequent/src/check.rs`)          | a `Dtor` with `cs.len() != 1` is rejected; the `Ctor`, `Prim`, and `Jump` consumer counts are walked but never counted                                                                                                                                                                             | one arity hard-coded, three unpoliced                                          |
| focusing (`core-sequent/src/focus.rs`)                  | every consumer-list construction site emits `Box::from([])` or a one-element list                                                                                                                                                                                                                  | nothing builds it                                                              |
| L machine (`core-sequent/src/machine.rs`)               | `drive_prim` loads `cs.first()` and drops the rest; `CommandNode::Jump` yields `StuckReason::UnsupportedByReference`                                                                                                                                                                               | nothing runs it                                                                |
| cell alphabet (`theory-computads/src/pattern.rs`)       | `ConsPat::Op { op, args, ret: Box<Self> }` and `ConsPat::Frame { ctor, ret: Box<Self> }` — one return continuation **structurally**, not by a check                                                                                                                                                | **not representable**                                                          |
| the engines (`theory-computads`)                        | overlap enumeration, completion, normalization, composition, and tracelets are generic over `CellAlphabet` (`src/alphabet.rs`)                                                                                                                                                                     | **no change owed** — the alphabet moves                                        |
| arity (`theory-levitation/src/arity.rs`)                | `BridgeArity { inputs, factors, source, dest, outputs }` over named `SortRef` ports, with `single_output` as the degenerate constructor                                                                                                                                                            | **built**                                                                      |
| arity checking (`theory-levitation/src/wellformed.rs`)  | `check_arity` validates that the three maps compose, reporting `WfKind::ArityDoesNotCompose`                                                                                                                                                                                                       | **built**                                                                      |
| description table (`theory-levitation/src/desc.rs`)     | `OpDesc { name, arity, attrs }` sits in `DataDesc.ops`                                                                                                                                                                                                                                             | **built**                                                                      |
| code universe (`theory-levitation/src/code.rs`)         | `Code::Var` is a bare recursive occurrence with no sort index                                                                                                                                                                                                                                      | the index change is owed                                                       |
| surface grammar (`surface-grammar/src/surface/term.rs`) | `op_result()` accepts a single type or a named tuple `( ident : Type, … )`, kept local to the `op` member so it never collides with a parenthesized type                                                                                                                                           | **built**                                                                      |
| surface elaboration (`surface-engine/src/desc_elab.rs`) | `op_member` reads the ports and `bridge_arity` builds one monomial per output, each reading every input                                                                                                                                                                                            | **built**                                                                      |
| desc → cells (`theory-computads/src/elaborate.rs`)      | `elaborate_data_desc` reads `desc.ctors`, `desc.ops`, and `desc.cells`: an `op` whose arity is the one-monomial, one-output shape is admitted, every other arity is declined, and a face applying a declined operation is declined with it. `gandr-surface-engine`'s `desc_cells.rs` is the caller | **wired at the single-output shape**; the many-out arity is declined, not held |
| type interner (`core-checker/src/intern.rs`)            | the content key and the resolve/subtype API deliberately assume no arity-1 result                                                                                                                                                                                                                  | no key-shape change owed later                                                 |
| corpus                                                  | `NatDiv`'s `op divmod(m, n) -> (q, r)` runs as `examples/pathological/desc/many-out-op-declined.gandr` — a well-formed description whose cell-layer decline is asserted — beside the single-output `examples/model/desc/desc-op-cells.gandr`                                                       | **promoted**; the decline is the witness, not execution                        |

The summary a reader should take away: **the sequent layer's support for many-out is exactly that a field is a `Vec`.** That is real and worth having, because no serialized format or node layout has to change.
It is also strictly weaker than support, and the cell alphabet — the layer where circuit structure would actually earn something — cannot express it at all.
The other three axes have no representation anywhere above the carrier.

## What the diagrammatic-rewriting literature supplies

The first subsection below was read **at theorem grade** and answers this lane's decisive question; everything after it is from a triage sweep, marked as such, and each entry names what it supplies and what it does not.

### The hypergraph correspondence is the applicable rewriting instance once cells stop being trees

The metatheory track records that gandr's double-pushout inheritance is nominal, with no pushout complement in code, because the term-rewriting double-category instance is the right shape for a **term-shaped** cell store and the graph-shaped double-pushout instances do not apply.
That scoping is correct today and **stops being correct at the moment the cell grammar admits reconvergence or many-out**, because the cells stop being term-shaped.

The literature has already built the replacement [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting]:

* string-diagram rewriting **modulo Frobenius structure** corresponds exactly to **double-pushout rewriting on hypergraphs**, proved sound and complete, and generalized to rewriting modulo _multiple_ Frobenius structures;
* labelled directed hypergraphs form a **presheaf topos and are therefore adhesive**, which is what makes DPO well-behaved there;
* the operative notion is **DPO with interfaces**, where a rewrite is taken relative to an interface that lets the diagram be glued into a larger, possibly unknown context — the interface decides which rewrites are applicable at all;
* pushout complements are **unique when the rule's left leg is mono**, and effectively enumerable when they are not.

That last point is the same phenomenon gandr already records from the virtual reading — non-linear overlaps fan out into families rather than a single fused rule — arriving independently from the DPO side, which is corroboration rather than a new constraint.
It is also **superseded at the Frobenius-free rung** by the boundary-complement result below, which restores uniqueness without the mono hypothesis.

### The correspondence at gandr's own rung, at theorem grade

The sequel drops the Frobenius assumption, and the third part supplies the confluence theory.
Both were read at theorem grade for this pass; the statements below are quoted or cited at their own numbers rather than paraphrased from abstracts.

> **The engine question, answered in one paragraph, because it is the thing this lane most needed decided.** gandr's rewriting engine does **not** become a hypergraph double-pushout engine, and it does **not** stay as it is.
> It becomes **a critical-pair engine that borrows the confluence procedure and keeps its own representation**, and the reason is that the two halves of the correspondence have very different prices for gandr.
> The representation half is nearly free and mostly already paid: the monogamous fragment's canonical representation is a bijection on ports, and gandr's carrier already is one, so adopting a node-carrying hypergraph would be re-representing what the tree has rather than acquiring something.
> The rewriting half is what gandr lacks and what the line supplies: **boundary complements** in place of pushout complements, **convex matching** in place of position-indexed matching, **pre-critical pairs with interfaces** in place of bare overlaps, and the two routes to decidable confluence.
> The engine keeps enumerating critical pairs — that part was right all along — and gains three things it does not have: a definition of pre-critical pair that carries the interface, a convexity condition on matches, and a route choice — left-connected, or path joinability — that turns "the worklist drained" into a decidable claim once a termination argument exists.
> That choice is **taken for the as-built alphabet and re-opened by its growth**, which is why it is stated below rather than left to the engine.
> **The one thing this verdict does not cover is the wheel axis**, and that is stated at the end of this section rather than buried.

**The Frobenius-free correspondence is exact, and it is exact on a fragment shaped like gandr's declines.** The image of the syntax under the cospan-of-hypergraphs encoding is precisely the **monogamous acyclic** cospans — no directed cycle, and every node of in-degree and out-degree at most one — and this holds for **coloured** PROPs as it does for one-sorted ones [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-ii, thm 25 and thm 27].
Reading gandr's four axes against that fragment is the sharpest calibration this pass found:

| gandr's axis      | inside the monogamous-acyclic fragment? | why                                                                                                                                       |
| ----------------- | --------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| **multi-output**  | **yes**, freely                         | a hyperedge has several ordered targets; each target node still has in-degree 1                                                           |
| **reconvergence** | **yes**, freely                         | two targets of one hyperedge may be two sources of another; degrees stay at 1                                                             |
| **disconnection** | **yes**, freely                         | the fragment imposes no connectivity condition                                                                                            |
| **fan-in**        | **no** — and that is monogamy           | in-degree 2 is exactly the combining fan-in this lane declines. **The decline is the fragment's own condition, arrived at independently** |
| **fan-out**       | **no** — and that is monogamy too       | out-degree 2 is the copy; the mirror image, priced the same, which is the correction recorded above                                       |
| **wheels**        | **no** — and that is acyclicity         | the axis this lane wants last is the one condition the Frobenius-free theory cannot drop (below)                                          |

So the correspondence covers three of gandr's four axes exactly, excludes the two aggregation moves gandr already declines, and excludes wheels.
**That is a much better fit than the corpus assumed**, and it means the applicable instance is not a distant target: it is the fragment gandr's carrier is already inside.

**Boundary complements retire the mono-left-leg condition.** The Frobenius-free theory replaces pushout complements with **boundary complements**, which additionally require the complement's own cospan to be monogamous, and these are **unique whenever they exist** — stated with the explicit remark that this "restore[es] uniqueness of pushout complements, even though we consider some rules which are not left-linear" [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-ii, prop 31].
The soundness-and-completeness statement built on them is an **iff for arbitrary rewriting systems**: $d ⇒_R e$ exactly when $[\![d]\!] ⇛_{[\![R]\!]} [\![e]\!]$ under **convex** DPO rewriting [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-ii, thm 35; coloured, thm 39].
The mono-left-leg claim the corpus carried is therefore true of the Frobenius rung and **not the condition that matters at gandr's**; what matters instead is convexity.

**Confluence is decidable, and the interface is what makes it so.** For DPO-with-interfaces the Knuth–Bendix property holds: joinability of all pre-critical pairs entails local confluence [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-iii, thm 3.1], and for a **computable terminating** such system confluence is decidable [ibid., cor 3.1].
Two conditions travel with it and both are load-bearing:

* the **ambient hypotheses** are an epi–mono factorisation system, binary coproducts, pushouts and pullbacks, adhesivity, and pushouts stable under pullbacks — which "hold in any presheaf category" and are closed under slice [ibid., asm 3.1];
* **computable** is defined, not assumed: pullbacks computable, the set of quotients of $L_i + L_j$ finite and computable for every rule pair, and every one-step rewrite of a given $G ← J$ enumerable [ibid., sec. 3].

**The empty interface is the undecidable case, and the analogy is exact.** The authors' own framing: hypergraphs with empty interface are "morally the graphical analogue of ground terms", Plump's undecidability result is about them, and **ground confluence is undecidable for both terms and graphs while confluence is decidable for both** [ibid., secs. 3 and 7].
This is the single most important sentence in the pass for gandr, because gandr's completion engine works on cell **patterns** and its overlaps carry seam data — which is to say gandr is already on the decidable side, and the honest caveat it ships is about budget rather than about undecidability.

**Without Frobenius there are two routes, and gandr must choose one.**

* **Left-connected systems** — left-linear, ma-rules, and every left-hand side **strongly connected** (a path from every input to every output).
  There the naive notion of critical pair is unchanged, local confluence follows from joinability of ma-pre-critical pairs, and confluence of a terminating system is decidable [ibid., def 5.6, thm 5.3, cor 5.1].
* **Convex critical-pair analysis via formal path extensions** — for systems that are not left-connected, joinability must be checked not only for the critical pair but for its **path extensions**: the signature is extended with three formal path generators, a critical pair is **path joinable** when it joins under every _maximal path relation_, and path joinability of all ma-pre-critical pairs entails local confluence, with a near-converse over the extended signature [ibid., def 5.7–5.10, thm 5.4, thm 5.5].

**gandr's cell left-hand sides are strongly connected as built, so the left-connected route is the one to take.** The verdict is an argument over the pattern grammar rather than a sweep of the fixtures, because the grammar admits nothing else.

A `CmdPat` has exactly one form, `Cut { pol, prod, cons }`, and its consumer half is a **linear spine**: `ConsPat::Op { op, args, ret: Box<Self> }` and `ConsPat::Frame { ctor, ret: Box<Self> }` each carry exactly one continuation, and `ProdPat` carries no consumer subterm at all, so the spine terminates in a single `ConsPat::Meta` or `ConsPat::Top` (`theory-computads/src/pattern.rs`).
A left-hand side therefore has **at most one output** — the terminal covariable — while its inputs are its producer metavariables, which can occur only in `prod`'s constructor tree or in an `Op` frame's `args`.
Every one of those positions feeds the spine: a producer subtree's value flows up to the cut and then down the spine, and an `Op` frame's argument flows into that frame and then down the spine below it.
So every input reaches the one output, and a spine ending in `ConsPat::Top` satisfies the condition vacuously on either reading of `★`.

**Strong connectedness in the sources' sense is a directed path from _every_ input to _every_ output**, stated for monogamous acyclic cospans and restated for ma-hypergraphs [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-ii, def 37; @bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-iii, def 5.6].
The spine argument discharges it for every expressible left-hand side, and the inventory confirms rather than establishes it: the elaborator's cut forms, `frame_defining_cell`'s `⟨v | K⁻(β)⟩`, and every rule fixture in `theory-computads` and `theory-virtual-doctrines` are of that one shape.

**Strong connectedness is one conjunct of left-connectedness, and the other two are owed rather than held — so the verdict selects the route without yet putting gandr on it.** Both definitions ask three things at once: the system is **left-linear**, every rule is an **ma-rule** on both sides, and every left-hand side is **strongly connected**.

* **Strong connectedness** is the conjunct discharged above, unconditionally, for every expressible left-hand side.
* **Left-linearity** is the [[#circuit-terms-question-17|circuit-terms-question-17]] ruling, whose consequence — `CellMeta::derive`'s `linear` field turning from derived metadata into a check with a diagnostic — is owed and not built, so a repeated hole is still admitted today (`theory-computads/src/sequent.rs`, fixture `a_repeated_metavariable_is_nonlinear`).
* **The ma-rule condition** fails for the same reason and on both sides: a repeated hole is a copy on a wire, which has out-degree two and is therefore not monogamous.
  On the left its fix is the linearity check; on the right it is the per-type comonoid of [[#circuit-terms-question-18|circuit-terms-question-18]].

**So the two remaining conjuncts have one cause and are build items rather than research questions**, which is the useful shape of the answer: the expensive route was never forced, and what stands between gandr and the cheap one is a diagnostic it has already ruled to emit.

**The consequence is the one the theorem advertises, and its hypothesis travels with it.** For a left-connected system the not-necessarily-convex DPO-with-interfaces relation coincides with syntactic rewriting in both directions, so a match is **automatically convex**, no convexity check is owed at match time, and the published match-enumeration bound drops by a factor of the target's size [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-ii, thm 38].
**That proof runs through acyclicity of the target**: a directed path from an output of the match back to an input would induce a directed cycle, and a monogamous acyclic hypergraph has none.
So this dissolves the convexity question **for the acyclic case only**, and says nothing about a re-closed delay loop, whose target is by construction not acyclic — and the answer there is that such a loop is **never the target**: the cut-open form is, and it is acyclic, so the theorem applies where the engine actually works ([[#circuit-terms-spike-08|circuit-terms-spike-08]]).

**The premise is a fact about gandr's current grammar rather than about the machinery, so the verdict does not survive the alphabet growth this lane exists to carry out.** Two of the four axes break it on their own, and the witness is concrete in each case.

* **Multi-output** is the substrate table's own missing row — `ret` is one continuation _structurally_, not by a check.
  The moment it becomes several, `⟨Succ(m) | divmod(n; add(k; α), β)⟩` is expressible and is **not** strongly connected: `k` feeds the `add` frame and so reaches `α`, and nothing carries it to `β`.
* **Disconnection** breaks it more bluntly.
  A left-hand side with two components and no wire between them has an input in one and an output in the other with no path between them, which is exactly what def 5.6 excludes; a `CmdPat` is one cut today, so no such side is writable.

**Reconvergence adds no boundary node and so cannot by itself create an unreached output, and wheels are excluded one condition earlier by the fragment's acyclicity** — which is what makes the fork below two-way rather than four-way.

**So the route selection has a design half, and it is not this section's to take.** Whether gandr's rules may carry disconnected or multi-output left-hand sides belongs with the alphabet decision ([[#circuit-terms-question-01|circuit-terms-question-01]]); if they may, def 5.6 excludes the system outright and **path joinability is the only published route left**, with its three formal path generators and its maximal path relations [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-iii, def 5.7–5.10, thm 5.4, thm 5.5].
The fork is recorded here and taken there, and nothing above may be read as deciding it.

> **One hypothesis of the argument is unsettled in the tree, and it is named here because two layers read it differently.** The argument counts a producer metavariable and a consumer metavariable as **two** interface nodes even when they wear the same name, which is what the matcher does: `Subst` keys its bindings by `MetaVar` and a `MetaVar` carries its `Cat` (`theory-computads/src/subst.rs`).
> `CellMeta::derive` keys by **name alone**, so it reports such a pair as one hole at two polarities and records `CellVariance::Mixed` (`theory-computads/src/sequent.rs`, fixture `a_hole_at_both_polarities_is_mixed`).
> Read the metadata's way, `⟨r | seam(; r)⟩` is a left-hand side whose single interface node is both the source and the target of the `seam` edge — a directed cycle, which leaves the monogamous acyclic fragment before strong connectedness is even asked.
> The verdict above rests on the matcher's reading, which is the operative one for rewriting; **which reading is the diagram's is owed an answer at the circuit rung**, and the shape is one `μ`/`μ̃` and cocase produce rather than an artificial fixture.

**The convexity hazard is the finding that bears on gandr's soundness surface, not just its schedule.** Under convex rewriting, **two rule applications acting on disjoint sets of hyperedges can still block one another**: applying one can create a directed path that destroys the convexity of the other's match, so the second is no longer a legal rewrite [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-ii, ex 45].
The worked instance is the Frobenius-semi-algebra theory, and it is why that theory is terminating but not confluent.

> **gandr's shift equivalence is stated as "two adjacent cell applications at disjoint positions with trivial overlap commute", and disjointness of supports does not imply independence once matches must be convex.** The quotient is TCB-adjacent — it is what the `cells_equal` normal-form fast path would decide, and that fast path is **not built**: `cells_equal` decides boundary equality plus two replays today ([[#circuit-terms-spike-07|circuit-terms-spike-07]] verifies this at the symbol) — so this is a **guard obligation at the circuit rung**, discharged before the fast path exists rather than a scheduling note.
> The disjointness test is strengthened to a convexity-stable one, and the fence to the fragment where convexity cannot be broken is kept **inside** that strengthening rather than as its alternative: left-connected left-hand sides are the published such fragment, gandr's as-built ones are inside it, and a store certified so discharges the strengthened conjunct instead of running it.
> Decided at [[#circuit-terms-question-15|circuit-terms-question-15]] and [[#circuit-terms-spike-07|circuit-terms-spike-07]].

**What the confluence theory cannot give gandr is the wheel axis.** Acyclicity is a hypothesis of the ma-fragment, of convexity (the path relation is a relation on directed paths), and of the termination arguments; the correspondence paper is explicit that a Frobenius-free rewrite that would need to move a box past a redex requires "at least a traced symmetric monoidal structure" to be applied at all [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-iii, ex 5.2]. gandr's arity ruling has since made the two-sided closure — the trace — the **primitive** former ([[../metatheory#The arity interface, universe-style]]), so a naive reading puts gandr's destination at traced symmetric monoidal without Frobenius, which is **outside** everything this line proves.
The next section says why that reading is the wrong one, and what the right one costs.

## Wheels, and which structure the cell layer takes

The corpus has carried three accounts of cycles at three layers without saying how they relate, so a reader asking "may a cell have a wheel" has had to reconstruct the answer.
This section states the relation and the ruling that follows.

| layer                                | what it says about a cycle                                                                                                                                              | status                       |
| ------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------- |
| **carrier** (`Shape`, `Match`)       | a wheel is representable; wheel-freeness is a **predicate**, and the operations do not preserve it                                                                      | landed, machine-checked      |
| **arity** (`Arity.sub`, the closure) | the **trace is the primitive former** — substitution's base case closes a block of sources against a block of sinks, and its degenerate case closes a vertexless circle | ruled; partly built          |
| **cell store and surface**           | a wheel needs a **delay guard**: a fed-back port must be delayed, and feedback categories are the adopted entry tier                                                    | design sketch, nothing built |

**These are three answers to three questions, and only the middle one is about equations.** The carrier's business is what is representable, and under the generality ruling it must represent more than gandr admits.
The arity layer's business is the combinatorics of graph substitution, where the closure is an operation on shapes and the circle count is bookkeeping; it makes no claim that a program may loop.
The cell layer's business is what the rewriting engine may assume, and that is where a choice has to be made.

**The choice is trace versus feedback, and it is a choice about which equations the engine may use.**

|                                   | **traced**                                                                                                   | **feedback** — the adopted tier                                                                                 |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------- |
| yanking, $"Tr"(σ) = "id"$         | holds — a bare loop collapses to a wire                                                                      | **fails** — a bare loop is a delay, not an identity                                                             |
| sliding (dinaturality)            | holds **unguarded** — a cell may be rotated freely around the loop                                           | holds only **across the guard**, or only for isomorphisms — either way the loop keeps a distinguished cut point |
| what matching becomes             | matching **modulo rotation**: a cyclic pattern has no first position, so the engine matches in a cyclic word | matching on a **directed acyclic graph**, unchanged                                                             |
| what the diagram normal form owes | a canonical rotation, on top of everything `canon` already owes                                              | nothing new                                                                                                     |
| what a wheel means operationally  | a fixpoint, computed                                                                                         | a stream: the same value one step later                                                                         |

**The two entry-tier sources axiomatize feedback differently, and the difference is exactly where sliding sits**, so the correspondence with the trace axioms is given at their own numbered statements rather than by the trace names alone [@katis-sabadini-walters-2002-feedback, def 2.2] [@dilavore-defelice-roman-2022-monoidal-streams, def 3.1].

| the trace axiom                               | Katis–Sabadini–Walters, Def 2.2                                                    | Di Lavore–de Felice–Román, Def 3.1                                                      |
| --------------------------------------------- | ---------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| naturality in the source (left tightening)    | **(i) naturality** — kept                                                          | **(A1) tightening** — kept, both sides in one axiom                                     |
| naturality in the target (right tightening)   | **(ii) naturality** — kept                                                         | **(A1) tightening** — kept                                                              |
| dinaturality in the loop object (**sliding**) | **(iii) weak naturality** — kept **only when the mediating map is an isomorphism** | **(A5) sliding** — kept in full, but the moved morphism appears **guarded** on one side |
| vanishing at the unit                         | **(iv) vanishing** — kept                                                          | **(A2) vanishing** — kept                                                               |
| vanishing at a tensor                         | **(v) vanishing** — kept                                                           | **(A3) joining** — kept                                                                 |
| superposing (strength)                        | **(vi) superposing** — kept                                                        | **(A4) strength** — kept                                                                |
| **yanking**                                   | **absent**                                                                         | **absent**                                                                              |

**Yanking is the only axiom that separates the two tiers, and that is a published result rather than a reading of one.** The free traced monoidal category on a category with feedback is the quotient obtained by adding **yanking alone**, and full sliding then _follows_ from the weak form in the presence of the remaining axioms [@katis-sabadini-walters-2002-feedback, prop 2.7].
The monoidal-streams line states the same boundary from the other side: a traced monoidal category is a feedback monoidal category guarded by the identity functor in which the loop over the symmetry is the identity [@dilavore-defelice-roman-2022-monoidal-streams, rmk 3.2].

**So the cell layer declines one axiom and not two**, and the corpus's earlier accounting — feedback drops yanking _and_ sliding — was right about the consequence and wrong about the structure.
Sliding without yanking is consistent and is what the monoidal-streams axiomatization takes; yanking without sliding is not available at all.
The guard is what keeps a distinguished cut point even where sliding holds in full, because the moved morphism is delayed on one side of the equation and not on the other.

**And the decline is reversible by construction, which is what makes its reversal condition cheap to honour.** Recovering the trace is a quotient of the feedback category rather than a different theory, so a construction that later needs the equations does not need the layer rebuilt around them.
The fixpoint reading is not lost either, and this is the point most easily mistaken for a cost: a **fixed-point semantics** of a category with feedback is defined as a monoidal functor to a compact closed category taking feedback to trace, so "a wheel is a computed fixpoint" is available as a _semantics of_ the feedback syntax rather than as the alternative to it [@katis-sabadini-walters-2002-feedback, def 2.8].

> **Ruled (owner, 2026-08-01): the cell layer takes feedback, and the trace is declined there.** The decline is **scoped to the cell layer and to it alone** — three fences follow — and it carries a reversal condition.

**The decisive reason is not that matching gets easier; it is that devices gandr already ships are stated over _positions in the diagram_, and the trace equations make a position ill-defined.**

> **An earlier draft of this passage overstated the case, and the overstatement is recorded rather than quietly fixed because it made the error this corpus is most prone to.** It claimed three devices break, on the grounds that "a cycle has no earliest position" and "a cycle has no critical path".
> Both read a cycle in the **diagram** as a cycle in the **derivation** — the layer conflation [[../metatheory#The representation is not the theory|the reading rule]] names, committed in the same session that promoted the rule.
> A cyclic diagram does not by itself make the causal order among rewrites cyclic, so the **bracket oracle's critical path is not broken by a trace and that bullet is withdrawn**.
> What survives is narrower and sound.

* **Deterministic normalization is the load-bearing one, and what sliding takes from it is a representative rather than the order.** Normalization is deterministic by "outermost position, then store insertion order", and _outermost_ presupposes a well-founded order on positions.
  Sliding moves a cell arbitrarily far around a loop, so no position is well-founded in the **closed** diagram — and a canonical rotation is what would restore one, at which point "outermost" is a decision again.
* **The canonical schedule pays the same coin at lower strength.** Its "earliest causal position" is causal order among rewrites, which survives a trace; but _position_ is position in the diagram, and under sliding it is defined only **up to rotation**.
  So the schedule needs a canonical rotation it does not have — one more `canon` obligation, not an undefined notion.
* **Those two are therefore one fact at two strengths, and the corpus's currency for a quotiented notion is a `canon` obligation.** Sliding quotients the position order rather than destroying it, which is a real price and not an impossibility — and that is what makes the decline a choice rather than a forced move.
* **Yanking is a distinct cost, though not an independent axiom.** It erases a step, so replay-equivalence would owe closure under an equation that deletes one from a record whose purpose is to have recorded it.
  Because full sliding follows from yanking, this price cannot be declined separately from the one above; taking the trace takes both [@katis-sabadini-walters-2002-feedback, prop 2.7].

Under feedback none of that arises: the delay is the tick boundary, position is well-founded in the cut-open representative, and the schedule needs no rotation.
The decline therefore rests on **three devices paying, two of them in the same coin** — a canonical rotation — which is still decisive against a structure gandr has no use for, but it is a smaller claim than the one first written, and the difference is the kind that matters.

**The second reason is coherence with the track's own temporal reading.** Identity here is a construction in time over an unfinished substrate, and the storage discipline is bounded sensitivity of the address map under local edits.
A delay is a temporal notion and a tick is a natural edit boundary; yanking is an atemporal statement — that a loop containing nothing _is_ nothing — and it is the one equation that erases a step from a record whose whole purpose is to have recorded the steps.

**The third reason is that the cut-open form is what the free construction already computes with, and rewriting it is sound with no trace axiom at all.** If every directed cycle in a body must contain at least one **delayed** port, then cutting every delayed port yields an acyclic diagram — removing an edge from every cycle leaves a directed acyclic graph, by definition.
A cyclic body $Γ → Δ$ with delayed ports $D$ cuts open to an acyclic body $Γ + D → Δ + D$, and **the cut points join the interface** — which is exactly the shape a double-pushout rewrite with interfaces already takes.

**The identification is of the cut-open form, and naming the right object is the whole of it.** A delay-guarded wheel is a cyclic hypergraph and is therefore not an ma-cospan; its **cut-open form** is the ma-cospan, and the delays' cut ends are that cospan's boundary.
Re-closing the delays leaves the fragment again.
This is [[../metatheory#The representation is not the theory|the reading rule]] applied one layer over — the acyclic presentation is a representation of the wheel, not the wheel — and every result of the correspondence applies to the presentation.

**That presentation is not a gandr device but the published normal form of the free feedback category, which is the strongest form this warrant could take.** An arrow of $"Circ"(C)$ from $A$ to $B$ **is** a pair $(α, U)$ with $α : A ⊗ U → B ⊗ U$ an arrow of the underlying monoidal category, taken up to isomorphism of the loop object $U$; feedback is $"fbk"_U (α, V) = (α, U ⊗ V)$, which moves the boundary between interface and loop object and leaves $α$ untouched; and $"Circ"(C)$ is the free category with feedback on $C$ [@katis-sabadini-walters-2002-feedback, def 2.4, prop 2.5 and prop 2.6].
The monoidal-streams line gives the guarded form of the same datum: a stateful morphism is a pair $(S, f)$ with $f : F S ⊗ X → S ⊗ Y$, the hom-set is the coend over the loop object of those morphisms, and that construction is the free feedback monoidal category over the guard [@dilavore-defelice-roman-2022-monoidal-streams, def 3.3, def 3.4 and thm 3.5].
**Cutting at the delays is therefore not an approximation of the theory; it is a representative of it** — and the axiom licensing every delayed port to be cut at once rather than one at a time is vanishing at a tensor, which is (v) in the one axiomatization and joining (A3) in the other.

**The soundness of rewriting the cut-open form and re-closing needs no trace axiom, and it is worth naming which fact carries it.** Feedback is defined as an **operation on hom-sets**, so equal cut-open morphisms have equal closures by congruence for a function — in **any** feedback category, with nothing about sliding or yanking invoked [@katis-sabadini-walters-2002-feedback, def 2.2] [@dilavore-defelice-roman-2022-monoidal-streams, def 3.1].
That is why declining those two equations costs the re-closure argument nothing.

**What the cut does cost is matching completeness, and the earlier claim that it costs the rewriting theory nothing was false as stated.** The cut removes a wire, so no rule whose left-hand side spans a delayed port can ever match the cut-open form: a delay-guarded loop carries a **permanently unmatchable seam**.
The correspondence's completeness half is an iff for rewriting the **cut-open** term [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-ii, thm 35 and thm 39], so completeness is inherited exactly there and not across the seam.
And congruence runs one way only — feedback is not injective, since $"Circ"$ identifies cut-open forms up to isomorphism of the loop object and the monoidal-streams hom-set identifies them up to dinaturality in it — so a rewriting theory complete on cut-open forms is complete on closed ones only up to that quotient.

**Whether the seam is a price or the point depends on the law, and the criterion is stateable.** A delay is a tick boundary under the temporal reading of identity, and the two equations that touch a tick are the two this ruling has just accounted for: **yanking erases one and is declined**, while **sliding crosses one and is kept only in the guarded form**, which is exactly what leaves a distinguished cut point.
For a law of either shape the seam is **the point**, and a law rewriting across it would reintroduce precisely what the decline was for.
For a law internal to one tick that merely happens to span a delayed port, the seam is a **price** — a real loss in matching completeness, paid to keep the tick.

**The criterion separating the two is upheld, and its test is not the one first proposed.** Asking whether the rule's left-hand side _factors through_ the guard is not evaluable on the cut-open form, because the guard **is** the cut and has been removed from it; the evaluable surrogate is whether the rule's left-hand side _mentions_ a delay, decided once at the declaration in time linear in that side.
The surrogate is well-founded rather than representation-dependent for the ruling's own reason — the only equation that could erase a delay is yanking, and yanking is the declined one — and [[#circuit-terms-spike-08|circuit-terms-spike-08]] carries the argument together with what the answer costs under each delay placement.

**Three fences, because this decline is easy to over-read and each over-reading would be a real error.**

* **It does not touch the carrier.** `Shape` still represents wheels, wheel-freeness is still a predicate the operations do not preserve, and every refuter stands.
* **It does not touch the arity layer, and must not be read as declining _its_ trace.** `Arity.sub`'s two-sided closure is an operation on shapes with its circles counted; it is not an equation the engine may use.
  The distinction is load-bearing beyond bookkeeping: the Int construction's licence — that a result about the compact-closed ambient reaches gandr **without a cup entering the carrier** — has as its hypothesis that the wiring category be **traced**, and that hypothesis is **answered by the arity ruling's decision, with the build still owed**: the closure is ruled and partly built, and no trace axiom is yet proved in the carrier ([[../metatheory#The rung, identified]]).
  Declining the trace at the cell layer leaves that hypothesis, and therefore that licence, untouched.
  Two different things are called "the trace" one layer apart, and conflating them would silently withdraw an import gandr depends on.
* **It does not answer the derivation dimension.** The metatheory records that what the wheel axis buys one dimension up is _cyclic derivation_ — the completion loop's fixpoints — and a cycle in the rewrite relation is not a delay-guarded loop in a diagram.
  This ruling is about wheels in the **term** dimension; the derivation dimension is a separate question and nothing here decides it.
  > **Ruled (owner, 2026-08-02): cyclic derivations are declined, with a reversal condition.** Stated in the certificate layer's own vocabulary: a certificate is a replayed path and replay-equivalence replays recorded paths, so a derivation that returns to its own start has no replayable representative — no finite record distinguishes its fixpoint from its unrolling.
  > The interaction with the engine is named rather than implied: the completion budget already declines-and-reports on an exhausted worklist, and the reduction order is plain node count, not substitution-stable — so no well-founded measure exists on which a returning derivation could be admitted as terminating evidence, and a fixpoint the completion loop encounters reports through the budget path instead of becoming a cyclic certificate.
  > The refusal is at representation, not at phenomenon.
  > The reversal condition: a consumer that needs the fixpoint as an **object** — a certificate whose meaning is the closure itself rather than any finite replay — with the concrete candidate being certificate composition at the completion boundary; if it arrives, the treatment is a new certificate kind with its own identity discipline, never a cycle admitted into replay-equivalence.
  > The obligation this creates for replay-equivalence is recorded where the certificate algebra is specified ([[../metatheory#The certificate algebra]]).

**The reversal condition, stated so it is checkable.** The decline reopens if a construction gandr needs requires sliding or yanking as an **equation** rather than as a convenience — the concrete candidate being the certificate layer itself, if composing two certificates ever turns out to demand rotating a cell around a loop.
It does **not** reopen merely because a program wants a loop; that is what the delay guard is for.

**Both conditions on the ruling hardening are now met, and what the reading changed is recorded rather than absorbed silently.** The feedback-category sources are read **in the original**, at the numbered statements cited above, and the axiom accounting is theirs rather than a reconstruction from a downstream implementation [@katis-sabadini-walters-2002-feedback] [@dilavore-defelice-roman-2022-monoidal-streams].
Two things moved: the decline is of **one** axiom rather than two, since full sliding follows from yanking; and the delay cut is not merely compatible with the published axioms but is their free construction's own normal form.
Nothing the ruling turns on moved, which is the outcome worth stating explicitly — a reading that confirms is as reportable as one that does not.

**The convexity behaviour of the cut is settled, and the gap closed in favour of the proposal at the price of one fence.** Convexity is a condition on paths **in the target**, and a match convex in the cut-open form does cease to be convex once the delays are re-closed, because a path runs round the loop — a two-cell body with one delayed back-edge exhibits it, and the direction is forced rather than incidental, since re-closure only adds edges and convexity is violated by a path existing.
So the cut-open test is strictly the more permissive of the two.

**What that costs is a rule about which diagram may be matched, not a repair to the convexity test, and the rule is the fence the ruling already implies.** The **cut-open form is the only legal matching target**; a match is never sought in, and convexity never evaluated against, the re-closed body — whose cyclicity puts it outside the hypothesis of every theorem that gives convexity its meaning.
Where a rule's reach must account for the loop, the condition is computed on the cut-open form under the delay's **path extension**, which is the published construction for abstracting exactly this kind of "does a path exist" question and, for a delay-guarded body, its easy case: the guard names the added edges, so the relation is computed by reachability rather than quantified over [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-iii, def 5.7 and def 5.8].
The witness, the repair's alternatives, the guard's cost, and the per-placement notes are at [[#circuit-terms-spike-08|circuit-terms-spike-08]].

**The per-type reading of the wheel axis is about a delay, not about Frobenius**, and saying why keeps the supply table honest — the answer below is that the wheel axis adds no row to that table at all.
The tempting analogy — fan-in is a commutative monoid, so a wheel is a Frobenius structure — **does not work**: the trace is bought by compact closure, compact closure comes from **(co)unital** Frobenius, and the unit is precisely what the nonunital rung omits.
A nonunital Frobenius supply therefore gives no loop at all, and a unital one is a cup by another name.
What a feedback structure needs is far weaker, and gandr already has its vocabulary: a **delay**.

**The delay is carried by the feedback binder, not by a row in the supply table, and the source settles which.** The monoidal-streams type theory adds a delay operator on types — extended to contexts pointwise — together with two formation rules: a delay rule taking a term in a context to the delayed term in the delayed context, and a feedback rule that binds the fed-back variable **at the delayed type in the premise** while the body produces it undelayed [@dilavore-defelice-roman-2022-monoidal-streams, sec. 8.2].
The waiting combinator is then derived from feedback rather than assumed.

**That guard is total on types, which is exactly why "this type supplies a delay" cannot be the discriminator.** Every type has a delayed form, so a supply keyed by type would hold everywhere and the wheel guard would become unfalsifiable — the failure mode this document already names for an ambient supply, arriving by a different route.
What discriminates is not the type but **how the fed-back occurrence is used**, and that is a condition on the binder.

**So the carrier already exists and needs no new former: it is the guardedness discipline of the (co)recursion surface, read at the binder instead of at the declaration.** The `>` sigil already obliges a marked call to sit under at least one copattern observation ([[../surface-language/recursion#The productivity ladder]]), and one observation is one tick; the feedback binder's rule is the same condition on the fed-back port's occurrence in the body.
The productivity discipline is therefore **not** promoted to a supply — it stays a checking rule and acquires one more site.

**The refuter the wheel guard owes is writable and fails at that rule.** A body closing a loop whose fed-back port occurs **bare**, not under an observation of its type, parses — the grammar admits the occurrence exactly as it admits an escaping self-reference — and the guardedness rung refuses it with a diagnostic naming the missing observation.
The corpus witness "a wheel with no delay" is satisfiable with no supply machinery at all; what it waits on is the binder.
The one true reading of "the type supplies it" survives as a consequence rather than a mechanism: a type that is not codata has no observation to sit under, so it cannot host a fed-back port.

> **Ruled (owner, 2026-08-02): the surface gains a type-level delay, and the typed discipline is required.** The feedback binder's typing rule is the monoidal-streams shape — the fed-back port sits at the delayed type in the premise and is produced undelayed — so the delay placement is the asymmetric, guarded one, and the guardedness discipline above remains what discharges the obligation.
> **The former's name and precise syntax are provisional** and the corpus writes a placeholder (`.d`) until three recorded collision risks are cleared: `Delay` as the partiality monad, should that lane open; the abstract-stone-duality direction; and the Lipschitz-condition mechanism, which may want the delay generalized to a graded modality.
> The surface record is [[../surface-language/circuit-cells#The block form, ruled]]; the cheaper no-former route the earlier recommendation named is superseded by this ruling.
> One consequence worth naming at the engine: the symmetric $Γ + D → Δ + D$ cut shape used below remains the placement-erased reading — forgetting the delay marking on the interface recovers it — so the spike results stated against it stand, with the seam test taking its typed form.

### The closest Agda encoding forks from gandr exactly here

The corpus carries the combinatorial string-diagram thesis as calibration — the closest existing Agda encoding, with three independent arrivals at gandr's decisions [@altenmuller-2026-string-diagrams].
Read for the hypergraph question, it forks from gandr at precisely this point, and the reason is instructive.

It **deliberately does not use hypergraphs.** In the hypergraph encoding, objects become vertices with at most two incident edges and morphisms become hyperedges; the thesis instead makes wires edges and boxes vertices, and equips the graph with a **rotation system** fixing the cyclic order of edges around each vertex — because its setting is **non-symmetric** monoidal categories, where the order of ports genuinely matters.

**gandr is on the other side of that fork.** Its symmetry ruling is explicit that ordering the parallel-component direction would be a silent catastrophe, so a rotation system is the one thing gandr must _not_ adopt, and the hypergraph encoding — which records connectivity and forgets order — is the side gandr belongs on.
The thesis is therefore calibration for the engineering and a **counter-model for the representation**, and saying which is which is the useful outcome of reading it.

Two further facts it supplies, both actionable:

* its related-work section is a ready-made map of the **interface** literature — hypergraphs with interfaces, cospans of hypergraphs with discrete feet, structured and decorated cospans, and contexts in monoidal categories — with the observation that discrete feet are **not enough to talk about graph embeddings**, which is why its own interface graphs carry edges;
* it reports that confluence for rewriting systems over hypergraphs-with-interfaces is **decidable**, which if it holds at gandr's rung bears directly on the completion engine's honest limits.

### Fan-in is a supply, not a per-cell side condition

The supply notion, its coherence theorem, and its survival of strictification are stated under [[#Many-out, fan-out, fan-in, supply, and Frobenius, in plain terms]] above [@fong-spivak-2020-supply].
The construction literature for building such categories is decorated cospans and decorated corelations, with corelations giving the black-boxing direction that discards interior structure [@fong-2015-decorated-cospans] [@fong-2017-decorated-corelations] [@fong-2016-thesis], and the universal-construction account presents these semantic categories as colimits of simpler ones, which is what makes complete axiomatisation tractable [@fong-zanasi-2018-universal-corelations].
Completeness of finite matrices for (dagger-)hypergraph categories is the semantic counterpart [@kissinger-2015-finite-matrices-hypergraph].

**The gandr-specific reading is the opposite of the usual one.** This literature is interested in categories where _every_ object supplies Frobenius; gandr wants the supply per type, checked, with the ambient category deliberately not supplying it everywhere.

### Internal wires are a binder, and there is a worked precedent

gandr's circuit-body sketch wires by name sharing and originally had **no construct that says a name is internal to a block** — so every intermediate wire in a body was either accidentally part of the interface or needed a separate interface declaration.
The ruled block form ([[../surface-language/circuit-cells#The block form, ruled]], 2026-08-02) closes half of that gap from the other side: the head declares the interface, so internal wires are the body names not in it, computed and checked by exactly the fold below; the explicit binder stays reserved for obligation-carrying wires, whose surface form is the `feed` statement.
The reversible-circuit language Ricercar supplies the construct and the fold, and it is worth transcribing because three of its four parts transfer [@thomsen-kaarsgaard-soeken-2015-ricercar].

Its **ancilla scope** `α x. A` binds an internal wire `x` over a circuit `A`, with fresh names minted on introduction and an obligation that `x` is constant at **both** ends of `A`.
Well-formedness is a fold over the syntax returning the set of used names:

```text
rwf(Id(x))  = {x}          rwf(A ; B)  = rwf(A) ∪ rwf(B)
rwf(Not(x)) = {x}          rwf(φ ⊳ A)  = dom(φ) ⊎ rwf(A)
                           rwf(α x. A) = rwf(A) \ {x}
```

with `⊎` the **disjoint union, undefined when the operands overlap** — so the whole check either returns a name set or fails, and the failure point is exactly a sharing violation.
Inversion commutes with the binder: `inv(α x. A) = α x. inv(A)`.

What transfers:

* **the binder itself** — an internal wire is a scoped name that leaves the interface, which is the construct gandr's circuit bodies lack and the thing that makes a body's interface computable rather than declared;
* **`rwf` as the shape of gandr's disjointness check** — the circuit-cells sketch says two redexes are disjoint iff they share no port name, and that making disjointness a name check rather than a reader's assertion is the trigger that reopens the horizontal-composition decline.
  `rwf` is a published, worked instance of that check: a syntax-directed fold whose only failure mode is non-disjointness;
* **the two-ended obligation** — the bound wire owes a condition at both ends, which is the same shape as the feedback rung's requirement that a fed-back port be delayed;
* **the honest limit, which transfers as a warning** — the authors state that reversible-well-formedness is necessary but **not sufficient** once ancillae are present, because whether the bound wire really holds the required value at both ends is a **semantic** check the syntactic fold cannot make.
  Read into gandr: the name-level disjointness check will not discharge the delay obligation, and expecting it to would be the error.

What does not transfer: its primitives (identity and not, with Boolean control), its permutation semantics over symmetric groups, and the physical ancilla-reuse motivation.
The construct is the import; the circuit model is not.

### Premonoidal and effectful categories are the published home of gandr's interchange stratification

gandr declines two simultaneous rewrite arguments because the diagram is unambiguous while its **sequentializations are not**, and it stratifies interchange by layer rather than assuming it.
That is not an idiosyncrasy: it is the defining feature of a **premonoidal** category — a monoidal category **without the interchange law** — and of an **effectful** category, which is a premonoidal category with a chosen monoidal subcategory of morphisms that _do_ interchange [@roman-sobocinski-2025-premonoidal-string-diagrams].

The correspondence to gandr's own position is close enough to be worth stating precisely. gandr's reversal condition for the horizontal-composition decline is "accept exactly on **disjoint positions**, where the two readings are shift-equal" — which is a chosen class of pairs that commute, sitting inside a structure where commuting is not general.
That is the effectful-category shape, arrived at independently.

What the source supplies beyond the vocabulary, read at theorem grade for this pass: string diagrams with an added **runtime object** are an internal language for effectful, premonoidal, and Freyd categories, and the statement is an isomorphism rather than an analogy — the free strict effectful category's homs are exactly the runtime-monoidal category's homs with the runtime adjoined on both sides, $\mathrm{EffString}(V,G)(A,B) ≅ \mathrm{String}_{\mathrm{Run}}(V,G)(R ⊗ A, R ⊗ B)$ [@roman-sobocinski-2025-premonoidal-string-diagrams, thm 3.14], with ordinary isotopy sound and complete on the result [ibid., cor 3.15] and the corresponding adjunction for premonoidal categories at [ibid., thm 5.5].
The runtime object is a wire threaded through every generator that has not been declared to interchange, which is exactly how a sequential spine is made visible inside a diagram — and gandr's single-spine cell grammar is that same spine, currently structural rather than represented.
The source's own related work names **call-by-push-value** as the closest translation of effectful categories into a programming language, so gandr's core calculus and this structure are the same object approached from two sides.

**Three details make this actionable rather than decorative, and one of them is an argument for representing the spine.**

* **Not representing it costs locality of substitution.** The authors consider the cheap alternative — keep a side table of which morphisms must not interchange — and reject it by a worked failure: knowing $f = g ; h$ no longer licenses substituting $g ; h$ for $f$ inside a larger diagram, because the relative order of $g$, $h$ and a neighbouring effectful $k$ becomes meaningful [ibid., rmk 2.8].
  Read into gandr: **a structural spine is exactly such a side table**, and locality of substitution is precisely the property gandr's matcher needs once a match is a sub-diagram embedding rather than a position.
* **The runtime is a linear resource in the Drinfeld centre.** It braids past every object, so its position in the interface does not matter (formalised as a _braid clique_), but it is neither copied nor discarded [ibid., def 3.8–3.10].
  A represented gandr spine would inherit both properties, and the second is the one that makes it a resource rather than a label.
* **One runtime means one sequentialization, and that is in direct tension with the disconnection axis.** The trace-theory development states it plainly: the runtime string appears **only once in each string diagram**, reflecting that premonoidal categories have no tensor product on morphisms, and the resulting endomorphism monoid is the **free** monoid on the generators — every order distinct [@earnshaw-sobocinski-2023-string-diagrammatic-trace-theory, prop 34].
  A single represented spine would therefore make every cell depend on every other, which is the opposite of what gandr's disconnection axis is for.

**The trace-theory line supplies the identification, and it names gandr's shift equivalence.** Mazurkiewicz trace languages are exactly symmetric monoidal languages over **monoidal distributed alphabets**, where each generator carries a set of locations and independence is disjointness of location sets [ibid., thm 22]; and the serialization square commutes — the free monoid on the generators (the premonoidal, one-runtime reading) quotients onto the trace monoid (the monoidal reading) by erasing the runtime [ibid., thm 35]. gandr's certificate normal form is a primitive multiset plus a canonical schedule, quotiented by "adjacent applications at disjoint positions commute"; **that is the trace monoid of the independence relation "disjoint support"**, and the canonical schedule is its normal form.
This is an identification worth carrying because it names what gandr built and supplies its literature, not because anything is adopted from it.

**The multi-runtime generalization is where gandr's disconnection axis lands, and it has since been built.** The 2023 paper proposes it as future work — generalize effectful categories to several runtimes so that actions may have input and output types rather than being atomic [@earnshaw-sobocinski-2023-string-diagrammatic-trace-theory, sec. 8] — and the follow-up carries it out [@earnshaw-nester-roman-2025-resourceful-traces].
Its move is to separate two things Mazurkiewicz's own gloss conflates: **devices**, the shared resources whose sharing creates dependence ("the printer", "the memory location"), and **resources** in the monoidal sense, the typed things actions consume and produce.
A generator is annotated with the devices it touches; the intended picture — the source's own intuition, with the diagrammatic account explicitly unformalized at several devices — is a string diagram in which each device string appears at most once in each vertical section; the free effectful category over an effectful graph has **resourceful traces** as its morphisms, by an adjunction; and the **commuting tensor product** of free effectful categories combines two systems whose actions must commute while still exchanging resources.
The single-runtime recovery as the one-device case is the source's prose remark rather than a numbered statement, and ordinary Mazurkiewicz traces arise as the no-resources case.

**What this changes for gandr is the question, not the answer.** The structure the disconnection axis wants exists, so the open item is no longer "does anyone have this" but a sharper mapping question: **which of gandr's structures is a device and which is a resource?** The single structural spine is one device; disconnection is several; gandr's types are resources; and the corpus's interchange stratification is what the commuting tensor product constructs.
Recorded as [[#circuit-terms-question-16|circuit-terms-question-16]], re-stated to the mapping.

### The spider theorem is the normal form fan-in cells want

Where a commutative Frobenius structure is present, any connected diagram of its generators with $n$ inputs and $m$ outputs equals the single $n → m$ generator [@coecke-duncan-2011-interacting-observables], and a production implementation takes this literally — a spider is **boxless**, introducing no generator, so many-in/many-out is the wiring map being allowed to repeat a label [@discopy].
This is the second independent argument that arity is the wrong axis, and it is the normal-form statement a fan-in cell would be checked against.

### Semi-strictification is a live candidate for the standing strictness warrant

The metatheory track's deepest open question is the strictness warrant at the circuit rung: the rectification licence lapsed with the rung change and nothing has replaced it.
A semi-strict algebraic model of $(∞, n)$-categories has since been proved equivalent to a weak non-algebraic one, with **algebraic units and composition of round pasting diagrams satisfying a strict form of associativity and interchange**, constructed combinatorially from regular directed complexes [@chanavat-hadzihasanovic-2025-semistrictification]; its companion extends the pasting theorem to directed complexes with **frame-acyclic** molecules and shows they coincide with regular polygraphs up to dimension three [@chanavat-2026-pasting-theorem].
Both are on gandr's axis — combinatorial, directed, acyclicity-conditioned, dimension-three-relevant.
Whether either discharges or reshapes the warrant is the metatheory track's question; this lane records the pointer and the reason.

### Completeness results are metatheory input, not implementation input

Completeness theorems prove that a rule set generates every equality valid in one chosen model, and gandr has no fixed semantic target, so the _form_ of the result does not transfer to the implementation.
That is a statement about **which track consumes them**, not a dismissal.
They are the field's worked examples of "a diagrammatic calculus, a semantics, and a proof that the rewriting is exactly right for it" — which is the shape of any adequacy claim gandr eventually makes for its own certificate layer, and worked examples are more useful there than stronger general statements.
The stabilizer and Clifford+T completeness results, and the ZW calculus the second routes through, are recorded for the metatheory track on that basis rather than carried here.

### The reversible line has its own home

The reversible-computing literature overlaps gandr on invertibility, and the sweep found that most of it belongs to a different feature entirely — durable, reversible, distributed computation — which is scoped in [[durable-computation]].
Two items bear on this lane specifically.

**Trace positions.** Reversible term rewriting extends rewriting so each forward step can be undone, then works to **remove positions from traces**, because positions are dynamic and carrying them requires expensive instrumentation [@nishida-palacios-vidal-2017-reversible-term-rewriting]. gandr records which cell fired where and not the matched substitution, deliberately, so replay must re-match — the same cost paid on purpose for a different reason.
The interesting fact is that the escape route is a **restriction class** (pure-constructor systems: basic left-hand sides, constructor right-hand sides) close in shape to gandr's own cell discipline, where a rule's left side is a cut between a constructor pattern and an operation frame.

**Compact closure by negative and fractional types.** A first-order reversible language of type isomorphisms extends to a compact closed category by adding a dual to sums and a dual to products, with negative types reversing execution flow and fractional types garbage-collecting [@chen-sabry-2021-negative-fractional].
This is the cup gandr's carrier declines, and it is therefore the best worked account of what admitting one would buy and cost — which the standing instruction asks for.

## What the implementations supply, read as artifacts

Four existing engines were read at source for this pass.
They are read as **engineering evidence about representation and search**, never as authorities on the mathematics, and each entry says what was checked.

**The monogamous fragment needs no node set at all, and gandr already has the representation.** Cartographer represents an open hypergraph as a **bijection between source ports and target ports** and drops the node set entirely, with the reason stated at the type: monogamy makes nodes redundant, because a node "is identified uniquely by the two Ports [it] connect[s]" [@sobocinski-wilson-zanasi-2019-cartographer]. gandr's `Match Γ Δ` — a source chooses a sink, every sink hit once — **is that structure**, arrived at independently ([[../metatheory/carrier]]).
The contrast with the Frobenius side is exactly one bit: DisCoPy, which supports spiders, represents the wiring as an arbitrary **function** from ports to a set of spider labels, so a label may be repeated and many-in/many-out costs no generator [@discopy].
**Bijection versus function is the whole representational content of the Frobenius/no-Frobenius choice**, which makes the per-type supply above concretely representable: a mixed wiring datum, a bijection on ports of non-supplying types and a function on ports of supplying types.
What this pass did **not** check is whether the published correspondence covers that mixed case; the coloured statements quantify Frobenius over every colour, so the mixed case is currently unwarranted rather than warranted.

**Matching a monogamous pattern branches once per connected component, and is deterministic after that.** Cartographer traverses the pattern's wires in an undirected depth-first order and, for each pattern wire, uses the already-matched endpoint hyperedge to _determine_ the context wire wherever one endpoint is fixed, falling back to nondeterministic choice only when neither is.
Chyp does the same job vertex-first, with three local invariants at each extension — type and size agreement, non-injectivity permitted **only at boundary vertices**, and, at an interior vertex, exact equality of in-degree and out-degree, which its comments state is what makes the DPO gluing conditions hold — plus one **global post-check**, convexity, implemented as "no path from the image of an output to the image of an input" [@chyp].
The seam data on the gandr side is therefore not a position but a **pair of partial bijections** — wire↔wire and edge↔edge — which is what the corpus's span-level seam data becomes when a match is an embedding.

**Chyp also answers the internal-wire question by refusing it.** Its declarative language never lets a wire be named: generators are declared by arity alone (`gen f : 2 -> 1`), diagrams are built from `;`, `*`, `id`, `id0` and `sw[…]`, and internal wires exist only as the positional seam of a sequential composition.
There is no ancilla scope, no name binding, and consequently no unbound-wire error class.
The cost is visible and admitted in its own documentation: permutation indices are **local to each swap**, so "splitting or combining swap maps will change some indices in general" — positional wiring is not compositional in its indices, which is the concrete price of the all-ordered choice gandr's [[#The design questions|circuit-terms-question-02]] weighs.
Its governing slogan, "**only connectivity matters**", is the diagram-normal-form half made into a user-visible rule, and its `refl` tactic decides it by **cospan isomorphism** rather than by rewriting — which is the split this document insists on, implemented.

**Chyp's DPO refuses a repeated boundary vertex by name, and the name is "Frobenius".** Its pushout-complement construction handles a boundary vertex of in-degree and out-degree one by splitting it, and raises `NotImplementedError("Rewriting modulo Frobenius not yet supported.")` as soon as a boundary vertex is used more than once on either side.
**A repeated hole in a rule's left-hand side was precisely that case, and gandr admitted one when this pass was written.** Verified at source at the time: `CellMeta::derive` in `theory-computads/src/sequent.rs` derived a `linear` flag and nothing consulted it as a gate, so a metavariable occurring twice on the left was **admitted and recorded as non-linear** rather than rejected, with `a_repeated_metavariable_is_nonlinear` exhibiting `⟨Pair(x; x) | α⟩`; and the substitution layer's matcher accepts a repeated occurrence by binding once and requiring agreement on the rebind (`subst.rs`, conflicting-rebind contract), which it still does, because matching is not the admission question.
At the circuit rung that same pattern stops being free substitution and becomes a copy on a wire, and the leading implementation of gandr's own rung declines it explicitly.
This is the sharpest single consequence of the term-shaped-to-circuit-shaped move, and it is recorded as [[#circuit-terms-question-17|circuit-terms-question-17]] — **since ruled and implemented**: the copy is now refused where a description's cells are admitted, and the derived metadata keeps recording rather than rejecting.

**The fastest engine in the family gets its speed by abandoning general matching.** PyZX's rewrites are not DPO: each is a `check_*` predicate on one or two vertices paired with an `unsafe_*` in-place graph surgery, driven by a fixed strategy pipeline (`full_reduce` as a loop over named simplification phases with per-rule ordering flags), counted by a rewrite-statistics object, and guarded by a debug mode that compares **tensor semantics after every step** to find the first divergence.
Two lessons transfer without adopting anything: a rule whose left-hand side is one or two generators does not need a matcher at all, and the correctness net for a strategy-driven normalizer is a differential against an independent semantics — which is the discipline gandr already runs between its two checker realizations.

**Diagram-level hash-consing is precedented; evaluator-level is still declined.** `homotopy-rs` hash-conses both diagrams and rewrites through a thread-local factory, with an $n$-diagram represented as a source $(n-1)$-diagram plus a list of cospans of rewrites, and structural equality on that pair.
Its well-formedness checker carries typed defect variants that include **normal-form conditions as invariants** — a cone may not be trivial, cones must be correctly ordered — rather than as a separate pass.
Neither observation conflicts with the performance track's "no global hash-consing in the evaluator": the consing there is on the diagram representation, which is gandr's storage layer, not its machine.

## Matching, normalization, and the crate boundary

The three faces that make this "computing with" rather than "representing".

**Matching.** gandr's one-sided matcher and two-sided unifier are written against a pattern language whose consumer side is a linear spine.
A circuit pattern is not a spine and not a tree: it has several roots, may reconverge, and may have components with no wire between them.
Matching therefore stops being a structural recursion and becomes a **sub-diagram embedding problem**, which is where the DPO line's matching-plus-boundary-complement formulation is the published answer.
Two things are now settled rather than open: the search shape is wire- or vertex-driven propagation with one nondeterministic seed per connected component of the pattern, and the span-level seam data becomes a pair of partial bijections rather than a position — both read off working implementations above.
What remains open is convexity, which is a **global** condition on the match and therefore the one part of the check that does not decompose along the pattern.

**Normalization.** Two normal-form questions must be kept apart, and conflating them is the hazard.

* _Diagram normal form_ — when do two circuit terms denote the same diagram?
  For the connected Frobenius case this is the spider collapse; in general it is a graph-isomorphism-flavoured question, and the corpus's own linear-time acyclicity test is a different and weaker check.
* _Rewriting normal form_ — the result of running the rewrite system to completion, which is what the certificate algebra already means by normalization.

The first is a property of the representation and is what content-addressing must intern on; the second is a property of the theory. gandr's `Rigid` device is where the first lands, and `Rigid.canon-sound` at the circuit rung is the standing obligation that owes it.
**Whether the first needs machinery of its own is the lane's largest unpriced question**, and it is what would justify a crate of its own.

**The first question is smaller than it looked, at least for the monogamous fragment.** Chyp decides it by cospan isomorphism and states it to users as "only connectivity matters"; and the isomorphism problem it decides is over the port bijection, not over an arbitrary labelled graph.
Two concrete shapes for a canonical linearization are now on the table, and they are the two extremes of one family rather than two guesses: orienting the cut equation one way makes normal forms **corolla decompositions** — pick a vertex, recurse into the components its removal leaves — and orienting it the other way makes them **edge decompositions** — pick an edge, recurse into the two components its removal leaves; both are exhibited for one syntax over unrooted trees, with the observation that the two orientations bracket a mixed style [@obradovic-2017-thesis, sec. 2.4.2].
The second is a spanning-tree traversal, which is the shape [[../metatheory/roadmap#meta-spike-09|meta-spike-09]] went looking for; recording both here is what that spike was owed on the `canon` side.

**The crate boundary — ruled (owner, 2026-08-02).** A new **`theory-circuit-algebras`** stands beside the existing theory crates at the narrowed boundary of [[#circuit-terms-question-12]]: it owns interface bookkeeping, embedding-based matching with its convexity check, and diagram normal form — not the representation, which is the carrier's port bijection and was already not new work, and not the engines: `theory-computads` continues to own cells, overlaps, completion, and tracelets over whatever alphabet it is given.
The seam is the `CellAlphabet` trait (`theory-computads/src/alphabet.rs`), which already exists and is already the place an alphabet is supplied — but with the alphabet ruled to grow in place at [[#circuit-terms-question-01]], the crate is **machinery over that seam, not a second inhabitant of it**, and the inhabitant phrasing of the earlier proposal is superseded.
Against [[../implementation#The crate map|the crate map]] it enters at tier 5, depending on `theory-computads` alone.
One consequence is recorded rather than designed here: if completion ever consumes embedding-based matching, it does so through a matcher seam supplied where the engine is instantiated, never by a downward dependency from `theory-computads` — establishing that seam is the build's own obligation, carried on its bead.

## The design questions

Each is anchored and cited by link, never by position.
Every one carries a disposition.

1. **circuit-terms-question-01** — **does the alphabet grow in place, or does a second alphabet stand beside the first?** Growing the pattern type fires the compile-visible tripwire at every match site, which is what the pattern grammar's narrowness was designed for; a second `CellAlphabet` inhabitant leaves the landed one untouched at the price of two to maintain.
   > **Ruled (owner, 2026-08-02): the alphabet grows in place, and no second inhabitant stands beside it.** The compile-visible tripwire is the reason rather than the cost: growing the pattern type confronts every match site at compile time, which is what the pattern grammar's narrowness was designed to do, while a second inhabitant would hide the migration behind the interface and leave two alphabets to maintain.
   > The coupling this entry carried is dissolved deliberately rather than followed: the crate of [[#circuit-terms-question-12]] lands as machinery over the `CellAlphabet` seam, so no second-inhabitant answer becomes natural with it.
   > The route consequence is selected knowingly rather than discovered later: growing in place is what eventually carries multi-output and disconnection into the one store, at which point the left-connectedness discharge stops being free and the strengthened guard of [[#circuit-terms-question-15]] is what carries — exactly the conditionality that entry records.
2. **circuit-terms-question-02** — **are a cell's ports ordered, named, or both?** Within-cell ordering costs nothing to adopt because no symmetry is present to give up, while ordering the **parallel-component** direction would be a silent catastrophe.
   **Carried, with ordered-plus-named preferred** — which is what `BridgeArity` already does.
3. **circuit-terms-question-03** — **what is a destination, operationally?** A store cell written before a single control transfer, or a covariable bound to a consumer the machine enters?
   The second sequentializes and re-raises interchange; the first is the destination-passing reading the metatheory ratifies.
   **Carried, and load-bearing** — the machine rung cannot be built without it.
4. **circuit-terms-question-04** — **the Σ-former at the multi-output face**, the metatheory track's own open question: the Σ-η direction is where fan-out bites, and premise-form statement is what keeps associative–commutative completion out of the rule layer.
   **Parked on the metatheory track, and a hard gate on the Σ-layer half.** The Π-layer half does not wait for it.
5. **circuit-terms-question-05** — **is the fan-in obligation carried as a per-type supply?** See the decline section above for what this would narrow rather than overturn.
   **Closed in the affirmative, with one residual.** The construction exists and is worked in the literature: a per-type supply expressed inside a Frobenius-free theory is a generator pair with the Frobenius equations as rules, terminating at [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-ii, thm 44], carrying no cup as long as the unit and counit are omitted, and representable as a mixed wiring datum (a bijection on non-supplying colours, a function on supplying ones).
   **The residual is the warrant, not the construction**: the published correspondence quantifies Frobenius over _every_ colour, so the mixed case is unwarranted rather than warranted, and [[#circuit-terms-spike-03|circuit-terms-spike-03]] is re-scoped to it while the dual obligation for fan-out becomes its own question below.
6. **circuit-terms-question-06** — **does the hypergraph DPO instance become the applicable one at this rung**, retiring the scoping that says graph-shaped double-pushout instances do not apply?
   **Closed, and the answer is qualified rather than flat.** The scoping is **retired for gandr's first three axes and stands for the fourth.** Convex DPO with interfaces over monogamous acyclic hypergraphs is sound and complete for arbitrary symmetric monoidal theories including coloured ones, and its fragment admits many-out, reconvergence and disconnection while excluding exactly the fan-in and fan-out gandr already declines; the acyclic hypothesis is what gandr's wheel axis will leave, and no published statement covers the traced case.
   The engineering consequence is stated at [[#The correspondence at gandr's own rung, at theorem grade]] and the residual is [[#circuit-terms-question-19|circuit-terms-question-19]].
7. **circuit-terms-question-07** — **how does the arity index the description universe?** Generalizing the recursive-occurrence code to a multiset of output sorts is a container, so the term face forces the **indexed** description universe.
   **Carried**, and shared with the higher-cells lane, which wants sort members for the same reason.
8. **circuit-terms-question-08** — **what does the enumerator cost once interfaces are circuit-shaped?** Non-linear interfaces fan out families and the measured multi-sum degeneracy ends, which the corpus already names as the trigger for revisiting full multi-globularity.
   **Carried as a scheduled consequence**, to be measured rather than predicted.
9. **circuit-terms-question-09** — **do circuit terms subsume products and sums, and should gandr keep both?** If a family of circuit formers covers tupling and casing, keeping eager products and sums beside them is redundancy the user pays for twice.
   **Carried, and the largest open design question in the lane.** Four sub-questions travel with it and none is answered here.
   + _Polarity_ — products and sums are **positive** value formers with a settled call-by-push-value story; a circuit term is an interface with input and output ports, and which side of the value/computation split it lands on is undecided.
     Ports carry polarity already, in the carrier's own orientation morphism, so the question is whether the two polarities are one notion at two layers or two notions wearing one word.
   + _Semantics_ — a subsuming family is indexed by its arity, so it is a family of formers rather than a former, and gandr's frozen core has no arity-indexed formers.
     The justification would have to be a levitated description, not a kernel addition — which is a point in favour, since the description layer is where arities already live.
   + _Pattern matching_ — case analysis on a sum is an eliminator with one arm per constructor; matching on a circuit term is embedding, and the two do not obviously unify.
     Whether copatterns are the bridge is the specific version of this to answer first.
   + _Presentation_ — if both are kept, the distinction has to be legible in syntax and in cost, and "these two things look the same and behave differently" is the failure mode the surface's design stance exists to prevent.
10. **circuit-terms-question-10** — **do circuit-shaped members exist in `codata` position?** `codata` blocks already parse `rule` members, and the higher-cells lane carries the same question for its respelled ladder.
    **Carried, inherited**; declining there is a legitimate answer but must be a decision, not an omission.
11. **circuit-terms-question-11** — **can trace positions be dropped on gandr's cell fragment**, as the reversible-rewriting line drops them on pure-constructor systems?
    **Carried**, and cheap to settle.
12. **circuit-terms-question-12** — **does this machinery want its own crate?** The proposal of record is `theory-circuit-algebras`, an inhabitant of the existing `CellAlphabet` interface rather than a fork of the engines.
    > **Ruled (owner, 2026-08-02): the machinery gets its own crate, `theory-circuit-algebras`, at the narrowed boundary.** It owns interface bookkeeping, embedding-based matching with its convexity check, and diagram normal form; the representation half was already not new work — the monogamous fragment's canonical representation is a port bijection and the Agda carrier already is one — and it stays where it is.
    > With [[#circuit-terms-question-01]] ruled grow-in-place, the crate is machinery over the `CellAlphabet` seam rather than an inhabitant of it; the boundary, its tier against the crate map, and the matcher-seam consequence are recorded at [[#Matching, normalization, and the crate boundary]].
13. **circuit-terms-question-13** — **does gandr want checked implicit coercions**, and is the circuit-term boundary one of their first customers?
    The observation is that a proof assistant's coercion mechanism is normally a bare insertion rule with no evidence attached, whereas gandr already plans a directed transformation family, a certificate layer, and named rewrite cells — so a coercion could be an **inhabitant of an existing evidence type** rather than new machinery, and mediating between primitive terms and circuit terms is the obvious motivating case.
    **Carried as a future direction, explicitly not scoped**, with its hazards named in the spike below.
14. **circuit-terms-question-14** — **is gandr's cell layer an effectful category?** Interchange holding only on a declared subclass of morphisms is the defining feature of premonoidal and effectful categories, and gandr's disjoint-positions reversal condition is that shape arrived at independently.
    **Closed in the affirmative for the single-spine layer, and the runtime object is what the spine becomes.** The identification is a theorem rather than an analogy [@roman-sobocinski-2025-premonoidal-string-diagrams, thm 3.14], the same source names call-by-push-value as the closest programming-language rendering, and there is a positive argument for representing the spine rather than keeping it structural: a structural spine is the side-table alternative the authors reject because it **loses locality of substitution** [ibid., rmk 2.8], which is exactly the property embedding-based matching needs.
    What does not close with it is the disconnection axis, which is [[#circuit-terms-question-16|circuit-terms-question-16]].
15. **circuit-terms-question-15** — **is gandr's disjointness test convexity-stable?** Under convex rewriting, two rule applications on **disjoint** sets of hyperedges can block one another, because one can create a directed path that destroys the other's convexity [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-ii, ex 45]. gandr's shift equivalence commutes adjacent applications at disjoint positions with trivial overlap, and that quotient is TCB-adjacent.
    **Closed, and it was a guard obligation rather than a design preference** — the two available answers were a strengthened disjointness test or a fence to the left-connected fragment, and the answer takes the first and keeps the second inside it.
    **The route is selected for the as-built alphabet, and it is conditional in two directions.** Every expressible cell left-hand side is **strongly connected**, because a `CmdPat` is one cut whose consumer half is a linear spine with a single terminal — so the **left-connected** route is the one to take, under which matches on an acyclic target are automatically convex, the notion of critical pair is unchanged, and confluence of a terminating system is decidable by the cheap route [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-ii, def 37 and thm 38] [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-iii, def 5.6, thm 5.3, cor 5.1].
    The forcing argument, the two remaining conjuncts, the two witnesses that break it, and the acyclicity fence are at [[#The correspondence at gandr's own rung, at theorem grade]].
    **What follows for this question is that the fence is free today and stops being free the moment multi-output or disconnection reaches the rule layer** — so the alphabet decision is a route decision as well, and the choice between the two answers was not settled by pointing at today's inventory.
    **The decided guard is that two applications commute when their positions are incomparable, their cell pair has trivial overlap, and each match image is still convex in the other's reduct** — with the third condition discharged outright, never run, on a store certified left-connected over an acyclic target.
    The as-built test the guard extends, the two arguments that no counterexample is expressible today, the cost accounting against the replay decision it accelerates, and the falsifier that would cost it its speed are at [[#circuit-terms-spike-07|circuit-terms-spike-07]].
16. **circuit-terms-question-16** — **which of gandr's structures is a device, and which is a resource?** One runtime object means one sequentialization and makes every cell depend on every other [@earnshaw-sobocinski-2023-string-diagrammatic-trace-theory, prop 34], which is the opposite of what the disconnection axis is for.
    The closest built structure is effectful categories over **several devices**, where a device is a shared resource whose sharing creates dependence and a resource is a typed thing actions consume and produce, with free effectful categories presented by **resourceful traces** and a **commuting tensor product** combining systems whose actions must commute [@earnshaw-nester-roman-2025-resourceful-traces].
    > **Ruled (owner, 2026-08-02): the device approach is declined as unfit for this rung, and the mapping closes refuted rather than unanswered.** The four legs were checked against the source at theorem grade.
    > The spine-as-one-device reading holds only as the source's prose remark; types-as-resources holds as an identification while the constructions stay planar — no symmetry, no copy, no discard.
    > Disconnection-as-several-devices fails structurally: independence there is declared on the alphabet and never read off connectivity — orthogonality is a relation on generators and no generator is orthogonal to itself, so two occurrences of one effectful cell never interchange however disjoint their positions, which is exactly the commutation gandr's shift equivalence earns per pair.
    > The stratification-as-commuting-tensor-product leg fails with the difference stated: that tensor partitions the alphabet, gandr's stratification partitions the term; and a rewrite connecting previously disconnected components would fuse devices, for which the framework has no morphism.
    > Capability-style concerns ride gandr's own type system rather than a device layer, and the axis keeps its wiring-read, per-term independence.
    > **The replacement direction is the separation-logic line over template games** [@mellies-stefanesco-2020-csl] [@mellies-2021-template-games]: separation as footprint disjointness with a frame rule is per-term independence of exactly the shape the axis needs, and the corpus already meets this line at the interchange-strength decision ([[../metatheory#Interchange, by layer]]), where the Hoare inequality arrives as a derived lax coercion — the earned-not-imposed posture the stratification already has.
    > **That read has since reported at theorem grade, and it moved the name of what arrives**: what survives contact with gandr's carrier is the template and cobordism apparatus, while the separation logic — separated states, the separating conjunction over a permission monoid, locks, and the data-race half of soundness — is declined at the door.
    > The direction is owned in detail by [[template-games]], which carries the tile pairing and the two axioms it gates on, the polarized footprint with its guard fence, the environment-polarity and virtual-cobordism obligations, the constraint that cobordism supports are store-transition systems, the six theorems owed, and what stays out of scope.
    > An earlier revision of this entry recorded the construction as unbuilt, on the 2023 paper's future-work note; the follow-up was in the library and the claim is corrected rather than carried.
17. **circuit-terms-question-17** — **what happens to non-linear patterns?** A repeated hole on a rule's left-hand side is free in a term-shaped store, because substitution copies; at the circuit rung it is a **copy on a wire**, which is a comonoid the type may not have.
    The leading implementation of gandr's own rung refuses exactly this case by name, raising a not-implemented error labelled "rewriting modulo Frobenius" as soon as a boundary vertex is used more than once [@chyp].
    > **Ruled (owner, 2026-08-01): cell patterns are linear, and the per-type comonoid is the named later generalization.** A metavariable occurring twice on a left-hand side is refused with a diagnostic naming the copy; a type supplying a cocommutative comonoid may host the copy explicitly, and that is [[#circuit-terms-question-18|circuit-terms-question-18]] rather than an omission.
    > **The ruling is free today, measured rather than assumed — and the measurement is restated here, because the first count was wrong in both directions.** There are **four** repeated-metavariable cut patterns in `theory-computads` and a **fifth** in `theory-virtual-doctrines`, and only one of the five is a copy the ruling costs anything.
    > `a_repeated_metavariable_is_nonlinear` exhibits `⟨Pair(x; x) | α⟩` — the one genuine copy, and a metadata fixture rather than a rule anyone depends on.
    > `a_hole_at_both_polarities_is_a_linear_seam` exhibits `⟨r | seam(; r)⟩`, which is **not a copy at all**: the name is worn once as a producer and once as a consumer, so it is one hole at two polarities — the dinaturality seam the composition gate reads, and a reachable shape rather than a fixture-only curiosity.
    > The substitution layer's `⟨x | add(Succ(Succ(x)); ⊤)⟩` is a **unification goal**, not a rule left-hand side, so no admission boundary ever sees it.
    > `fanout_family_is_a_multi_sum_not_a_single_rule` and, in `theory-virtual-doctrines`, `the_seam_family_is_the_overlap_indexed_multi_sum` each build `⟨Pair(y; y) | g(α)⟩` **deliberately**, to witness that composition at a shared seam is a _family_ of overlaps rather than a single fused rule; they are the concurrency contract's own witnesses and must keep witnessing it, which is a second reason the refusal cannot be a construction guard.
    > **Placement (owner, 2026-08-02): the refusal is an admission boundary, not a construction guard.** It runs where cells enter a store from a description — the elaboration path — and `CellMeta::derive` keeps computing metadata and rejects nothing.
    > The reason is the inventory above: multi-sum contract witnesses and unification goals are legitimate internal shapes, so what the ruling governs is which cells may be **admitted**, never which patterns may be **constructed**.
    > **The copy relation is per `(name, category)` pair, not per name (owner, 2026-08-02).** A hole at two polarities is a seam and stays admitted; variance still joins across polarities and still reports it `Mixed`.
    > Counting bare name occurrences conflated the two questions and reported the seam as non-linear, which would have made the boundary refuse the very shape two-mode certificate composition exists to consume — so the count was corrected in the same change that added the boundary.
    > **What it costs is idempotence and cancellation** — `and(x, x) ~> x`, `x - x ~> 0` — which stop being writable with a repeated hole.
    > They are respelled with the copy named rather than lost, exactly as a fan-in cell must name its monoid: the rule matches through the copying cell, and a type whose grade already licenses duplication is where such a rule lives.
    > The diagnostic says so rather than only reporting a rejection — it names the copied hole and points at the respelling and at the hosting generalization.
    > Two consequences ride with it: `CellMeta::derive`'s `linear` field is joined by a **check with a diagnostic** at the admission boundary rather than becoming one, and the corpus's globularity-above-the-base trigger must be restated, because it was conditioned on a measurement this ruling now prevents.
18. **circuit-terms-question-18** — **is the fan-out obligation carried as a per-type supply, and at which layer?** The fan-out/fan-in asymmetry did not survive this pass, so copy owes the same treatment as merge. gandr already prices duplication on the **value** side, with grades and with `dup`/`drop` as ordinary computations; what has no answer is what the obligation means at the **cell** layer, where a repeated hole is the thing that would have to carry it.
    **Carried as the named generalization of the linearity ruling above**, and it is the row that makes idempotence rules writable again on the types that genuinely support them.
    The sub-question to answer first is whether the cell-layer comonoid is a **new** supply or is read off the existing grade discipline, since a grade-ω binding already licenses duplication on the value side.
    > **Ruled (owner, 2026-08-02): read off the grade discipline, not a new former — a plan whose substrate is named rather than assumed.** The copy obligation at the cell layer is declared by the type's grade-side licence, mirroring the delay answer one entry down: a discipline read at its binder rather than a new supply former.
    > The as-built blocker is recorded rather than waved off: today a grade is a field of a thunk-typed binding, not a function of the type, so it cannot yet determine a per-colour predicate — the executed warrant spike's tracker record (2026-08-02) carries this with two further disanalogies.
    > Execution therefore rides the value-semantics incorporation ([[../surface-language/value-semantics]]), and the string model's copy-on-write intersection is tracked so neither lane decides copy pricing in passing.
19. **circuit-terms-question-19** — **does the cell layer take trace or feedback?** Restated 2026-08-01 from "what covers the wheel axis", which conflated three layers.
    **Ruled: feedback, with the trace declined at this layer only**, on three grounds — position order, the temporal reading of identity, and the delay cut keeping the correspondence applicable to the cut-open form.
    The account, the three fences and the reversal condition are [[#Wheels, and which structure the cell layer takes]].
    **The ruling is hardened against both sources in the original**, and the reading moved two things without disturbing the choice: the decline is of **one** axiom, yanking, since full sliding follows from it [@katis-sabadini-walters-2002-feedback, prop 2.7]; and the delay cut is the free feedback category's own normal form rather than an inference about it.
    **The delay question is answered and needs no former of its own**: the guard is total on types, so what carries "a delay is present" is the feedback **binder's** typing rule, discharged by the guardedness discipline the (co)recursion surface already runs.
    **The warrant is now discharged too, and it cost a fence rather than a repair.** Cutting at the delays does **not** preserve convexity under re-closure — a two-cell witness exhibits the path, and re-closure is more permissive by construction because it only adds edges — so the **cut-open form is the only legal matching target**, and the condition the engine computes there is the delay's own path extension, which is the known-relation case of the published path-joinability route rather than a gandr side condition.
    The seam criterion is upheld with its test replaced: whether a left-hand side **factors** through the guard is not evaluable once the guard has become the cut, and the evaluable surrogate is whether the rule **mentions** a delay, decided once at the declaration and stable precisely because yanking is the declined axiom ([[#circuit-terms-spike-08|circuit-terms-spike-08]]).
20. **circuit-terms-question-20** — **when a type supplies both fan-in and fan-out, which interaction law comes with them?** The two canonical answers over the same generators are **Frobenius**, under which connected diagrams contract to a standard form, and **bialgebra**, under which they expand — the correspondence paper's bialgebra case study needs a five-component lexicographic order to terminate, because one of its rules increases the hyperedge count.
    They are not variants of one structure, and the choice decides whether a supplied type's fragment shrinks or grows under rewriting.
    **Carried, and it is a fork the supply table hid** by listing the two directions as independent rows.
    The coherent both-at-once structure is Hopf-Frobenius [@collins-2024-hopf-frobenius], whose "the conditions are minor" result is a **vector-space** statement resting on integrals, so it does not transfer to a combinatorial carrier and the fork stays a real decision.
    > **Ruled (owner, 2026-08-02): bialgebra is the interaction law when both directions are supplied.** The two arms are not symmetric in rewriting cost, and the sources say so: the Frobenius interaction system is terminating but **not confluent** — the source's own example exhibits one diagram with two distinct normal forms [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-iii, ex 5.3] — and two of its rules structurally cannot be strongly connected, a strongly connected Frobenius left-hand side being equivalent to a directed cycle the fragment excludes; the bialgebra system is left-connected in the source's own words [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-ii, sec. 5.2], with the non-commutative variant verified left-connected and proved confluent [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-iii, sec. 6.1].
    > A single-direction supply is left-connected either way: the cheap confluence route is lost only by adding an interaction law, and only by the Frobenius one.
    > What this re-routes rather than loses: the spider normal form stays true as a theorem and stops being a rewrite-strategy deliverable — its decision procedure arrives through canonicalization at the representation ([[#circuit-terms-question-21]] and the execution ladder's rung-04), never by running the contraction rules.
    > The reversal condition: a lane that genuinely needs wire-contraction semantics as **equations**, which then pays for path joinability explicitly rather than by default.
21. **circuit-terms-question-21** — **can the noncommutative, asymmetric spider theorem be consumed at the representation layer rather than refused?** The reason to want it is concrete: `append` on a `Pipe` is the aggregation section's own motivating example and it is **not** commutative, so the standing commutativity requirement may be excluding the case that motivated the feature.
    The reason to think it possible is that gandr does not run a symmetric representation — the merger **is** a planar tensor, and canonicalization is already the passage that makes the convolution symmetric — so a planar theorem lands where gandr's order already lives, consumed through `canon-sound` rather than adopted into the theory.
    **Carried**, with three things to establish in order: that the within-component order the spider theorem uses separates from the cross-component symmetry the bracket oracle needs; that the monomial-to-monomial condition holds for the Frobenius relations, which on their face it does since they equate diagrams rather than sums; and what the resulting `canon-sound` instance costs.
    The failure mode to watch is the one the corpus names for the planar quotient generally: a substitution that "would read as a strengthening while narrowing what `cells_equal` accepts".

## Spikes

### circuit-terms-spike-01

**Does the hypergraph DPO-with-interfaces instance apply to gandr's circuit cells?** Take one circuit cell shape, write it as a hypergraph with interface, and check three claims: that the interface is the coproduct of the cell's input and output ports, that gandr's rules have mono left legs so pushout complements are unique, and that a rewrite respecting the interface is the same relation as a gandr cell application.
**EXECUTED (2026-08-01), and the numbering is retained rather than reused.** Its three claims resolved as follows, against the sources rather than against a toy encoding.

* **The interface is the coproduct of the cell's input and output ports** — **confirmed as the setting's own definition**: a rule is $L ← i + j → R$ with $i + j$ discrete, and a hypergraph with interface is $G ← n + m$ for the ma-cospan $n → G ← m$ [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-iii, sec. 5.1].
* **gandr's rules have mono left legs, so pushout complements are unique** — **the wrong question at this rung, and it dissolves.** The Frobenius-free theory does not use plain pushout complements; it uses **boundary complements**, which are unique whenever they exist, explicitly including rules that are not left-linear [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-ii, prop 31]. gandr did **not** have mono left legs in general when this spike ran — non-linear patterns were admitted and their linearity recorded as derived metadata — so the claim as posed would have failed; the uniqueness gandr needs comes from elsewhere and is unconditional, which is why the linearity ruling at [[#circuit-terms-question-17|circuit-terms-question-17]] leaves this finding standing even though it has since removed the admitted non-linear case.
* **A rewrite respecting the interface is the same relation as a gandr cell application** — **confirmed for the correspondence and not yet for gandr.** The correspondence is an iff for arbitrary rewriting systems over arbitrary (including coloured) symmetric monoidal theories, at [ibid., thm 35 and thm 39], **provided the rewrite is convex**; whether gandr's cell application is convex is not a fact about the literature and is now [[#circuit-terms-question-15|circuit-terms-question-15]].

**The verdict is that the corpus's scoping is re-scoped, not contradicted**, and the re-scoping is written at [[#The correspondence at gandr's own rung, at theorem grade]].

### circuit-terms-spike-02

**Measure the enumerator blowup.** Instantiate the overlap enumerator on a toy circuit alphabet and count the overlap families against the single-output baseline, so the enumerator-cost question is answered by measurement rather than prediction.
**Small**, and worth running before the alphabet grows, because the number is an input to whether ordered ports are enough.

### circuit-terms-spike-03

**RE-SCOPED (2026-08-01) to the warrant, because the construction is settled.** The original question — is the fan-in obligation expressible as a per-type supply — is answered in the affirmative under [[#Per-type supply as a general decline-relaxation pattern]], with the construction, its termination theorem, and its no-cup condition all cited.
What remains is one thing and it is a warrant question: **does the sound-and-complete correspondence survive a _mixed_ signature**, where some colours supply the structure and others do not?
The published coloured statements quantify Frobenius over every colour, so nothing yet covers the mixed case; check whether the encoding of a supply as ordinary generators plus rules keeps the whole theory inside the monogamous acyclic fragment, and whether the resulting system is left-connected (which would make its confluence decidable by the cheap route) or needs path joinability.
**The cut-rooted forcing argument does not reach that system**: a supply's generators are not cut patterns, so its rules' left-hand sides are checked against def 5.6 directly rather than inherited from the verdict at [[#The correspondence at gandr's own rung, at theorem grade]].
**Small**, and it is now the gate on the second row of the supply table rather than on the first.

**Executed (2026-08-02), and the warrant holds with its hazard re-aimed.** The mixed encoding is covered as instances: monogamy and acyclicity are colour-blind node and path conditions, and the correspondence theorems quantify over arbitrary coloured signatures [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-ii, def 12–13, def 18–20, thm 27, thm 39] — the every-colour Frobenius quantification this spike was re-scoped around belongs to the Frobenius correspondence, which the encoding route does not use, so the recorded hazard was aimed at the wrong correspondence.
The route cost is asymmetric and decided the fork at [[#circuit-terms-question-20]]; the spider triple's bead count is computable near-linearly on the representation by union-find over the port bijection, with the identification corrected in place at the supply table (j = β₁(Shape) + ℕ, owner sign-off 2026-08-02).
One lemma is genuinely missing and stays a conditional spike: whether confluence of a mixed system follows from confluence of each colour's subsystem — neither paper addresses modular composition, and it becomes work only if mixed systems ship.
The full record with statement numbers is the executed spike's tracker comment, 2026-08-02.

### circuit-terms-spike-04

**Is gandr's cell-visible pattern fragment position-removable?** Compare the pure-constructor restriction class of the reversible-rewriting line against gandr's cell pattern discipline and decide whether recorded positions are necessary or merely convenient.
**½ day.**

### circuit-terms-spike-05

**Does the spider normal form give fan-in cells a decision procedure?** Where a supply exists, check whether the connected-diagram collapse is checkable on gandr's representation, and how it relates to the corpus's linear-time acyclicity test.
**Small.** Feeds the diagram-normal-form half rather than the rewriting half.

### circuit-terms-spike-06

**What would a checked implicit coercion cost, and what would it be evidence of?** Write one coercion between a primitive term and a circuit term and answer three questions: what evidence type inhabits it; whether coherence — two coercion paths between the same pair agreeing — is a theorem, a certificate, or an unmet obligation; and whether insertion is decidable without search.
**Unmeasured, and deliberately fenced.** The hazards are named rather than solved: an insertion rule that fires silently is a readability surface and a soundness surface at once, and the corpus's standing position that variance is derived and never declared has coercion as its neighbouring temptation.

### circuit-terms-spike-07

**Is gandr's disjointness test convexity-stable, and if not, what is the fence?** Take the published counterexample — two rule applications on disjoint hyperedge sets where each destroys the other's convexity [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-ii, ex 45] — and answer three questions in order.
**EXECUTED (2026-08-01)**, and the answers open by saying what the test is, because the relation the corpus states and the equality the tree decides today are not the same object.

**The disjointness test is not built, and neither is the fast path it would guard.** Verified at source: `CellStoreVdc::cells_equal` decides domain equality, codomain equality, and `elaborations_replay_equivalent`, which on two certificates is `replay_equivalent` — equal peaks, equal joins, and **both tracelets replayed** against the store (`theory-virtual-doctrines/src/vdc.rs`; `theory-computads/src/tracelet.rs`).
That is boundary equality plus two replays, which is the decision the normal form was to accelerate rather than the normal form itself, so the shift quotient has no implementation that could be convexity-unstable, and the tractability witness that would carry the fast path's soundness certificate is recorded as not existing ([[roadmap]]).

**What the tree does carry is the two ingredients the test would be assembled from, and their shapes decide the answers below.** A **position** is `Pos(Box<[usize]>)`, a path of child indices into the pattern tree, and an application is a `CellApp { cell, at }` over one (`theory-computads/src/pattern.rs`; `theory-computads/src/rewrite.rs`); an **overlap** is a property of an ordered _cell pair_, computed by `enumerate_overlaps` at the cut seam and at command seams inside the left cell's right-hand side, never at a pair of positions in one term (`theory-computads/src/overlap.rs`).
So the test the quotient asks for reads, as built, as **two `Pos` paths of which neither is a prefix of the other**, conjoined with a cell-pair overlap lookup — and the two conjuncts live in different indexes, which is what the third answer turns on.

* Does an analogous pair exist over gandr's cell fragment once patterns are circuit-shaped, or does the cut-rooted left-hand-side discipline already exclude it?
  **No witness exists over the as-built alphabet, and the exclusion is over-determined: two independent arguments give it, at different strengths.** The first is the correspondence's own: every expressible left-hand side is strongly connected, so on an acyclic target **every** match is convex, unconditionally [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-ii, thm 38] — and no rewrite can destroy a property that no match is able to fail.
  That is stronger than "these two applications do not interfere": non-convexity is **unreachable** in the fragment, which is why the blocking pair cannot be assembled rather than merely failing to turn up.
  The second is a fact about the representation, and it makes the hazard vacuous before the theorem is invoked at all: a `Pos` addresses a subtree, two incomparable positions address **disjoint subtrees**, and the only route between disjoint subtrees of a term runs up to their common ancestor and back down — which is not a directed path from an output of an image back to an input of it.
  A term tree has no cross edges, and the counterexample is built out of one.
  **The two arguments are not interchangeable, and the difference is the disposition.** The first rests on thm 38 conjoined with a fact about today's grammar; the second rests on the representation alone.
  Both of those premises are premises about **us**, so this is a fence over the current alphabet and never a refutation of the hazard — which is the honest form for it to take, because the alphabet is precisely what this lane changes.
* If it exists, is the repair a strengthened disjointness predicate (disjoint **and** no new path created between the other match's boundary), or a fence of the shift quotient to left-connected left-hand sides — a fence that costs nothing today and costs exactly those two axes afterwards?
  **The strengthened predicate is selected, and the fence is retained inside it as a discharge rather than declined.** The two are not alternatives at one layer: the fence is a property of the **rule set**, checkable once per cell at insertion, while the predicate is a property of a **pair of applications in one term**, checkable only where that pair is commuted.
  The fence is declined as the primary answer for a reason about the fence rather than about gandr's taste for it: strong connectedness is broken by multi-output and by disconnection, with a witness recorded for each ([[#The correspondence at gandr's own rung, at theorem grade]]), so fencing the quotient to left-connected left-hand sides fences out the two axes this lane exists to add.
  **The predicate's correct statement is not "no new path" but the convexity re-check that phrase is reaching for**, and stating it that way is what makes it warranted rather than invented: two applications may commute when each match image is still **convex in the other's reduct**.
  The two phrasings agree immediately — the images are disjoint, so every path internal to each is untouched, and any new violation is a path that leaves an image and returns through the region the other application rewrote, which is exactly the output-to-input path a convex match forbids [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-ii, def 22 and def 33].
  **The fence then becomes the predicate's own fast path.** A store certified left-connected makes the re-check provably constant-true by thm 38, so it is skipped rather than run — and that certificate is the natural first inhabitant of the tractability witness the roadmap owes, which is where a TCB-adjacent fast path's soundness evidence was always going to live.
* Either way, what does the `cells_equal` fast path have to check before it may commute two applications, and does that check stay cheap enough to keep the fast path a fast path?
  **The guard is four conditions and only the third is new.** The positions are incomparable — neither `Pos` a prefix of the other; the cell pair has trivial overlap; **each match image is convex in the other's reduct**; and that third condition is discharged outright when the store carries a left-connectedness certificate and the target is acyclic.
  **The check is cheap per query, and the cost lands somewhere other than where the question expected it.** Convexity of one match is a directed reachability sweep from the image's outputs, $O(|G|)$ in the target, and it is what raises match enumeration from $O(|L| \cdot |G|)$ to $O(|L| \cdot |G|^{2})$ in the source's own sizes (Remark 36 of the same paper) — while the competitor it has to beat is the as-built decision, whose two replays run `rewrite_at` once per step and are therefore already $Ω(k \cdot |G|)$ for a $k$-step certificate (`theory-computads/src/tracelet.rs`).
  One guarded commutation thus costs what one replay step costs, and the fast path stays ahead by the factor the replay pays over the whole path.
  **What the repair does cost is the independence relation's index, and that is the finding worth carrying forward.** The overlap conjunct is keyed by cell pair and is cacheable — it is the overlap-support lookup the bracket oracle already shares across four consumers — while the convexity conjunct is keyed by _term and position_ and cannot be cached across terms at all.
  So the repair moves one conjunct of independence out of a static table and into a dynamic sweep, which is exactly why the left-connected discharge is load-bearing rather than an optimization: on that fragment the dynamic conjunct is constant and the static cache is complete again.
  **The falsifier, stated so the argument can lose.** The per-query cost is linear, but the number of queries a canonical schedule needs is not — a normal form reached by adjacent transpositions asks $O(k^{2})$ independence questions in the worst case, at which point the guard costs $Ω(k^{2} \cdot |G|)$ against a replay's $Ω(k \cdot |G|)$ and the fast path has stopped being one.
  That is a measurement rather than an argument, of the shape [[#circuit-terms-spike-02|circuit-terms-spike-02]] already carries but taken over schedules instead of overlaps, and it is named here rather than assumed away.

**The honest default is now stated positively rather than left pending.** The shift quotient is warranted on the fragment where no match can be made non-convex; that fragment is the whole cell store today, by either argument above; and after the alphabet change it is the sub-store whose left-hand sides stay strongly connected, with the convexity re-check standing in for the certificate everywhere else.

**As built (2026-08-02): the guard has its first consumer, and it is a constructor rather than a fast path.** `gandr-theory-computads`'s `shift::derive_shift_equivalence` decides the three conjuncts in the order stated above and refuses the pair — a typed obstruction, never a panic and never a silent identity — when any fails; the convexity conjunct is carried as a datum naming its warrant, answered per store by the alphabet, instead of being recomputed; and the independence question is asked of the cell pair alone, through an `overlaps_between` extracted from the store-wide enumerator without changing what it reports.
Two as-built facts arrived with it, and both bound where the guard bites.

* **The quotient's extension over the sequent alphabet is still empty, and now for a stated reason rather than for want of a predicate.** A `CmdPat` is one cut whose children are a producer and a consumer, so a term has exactly one command position and no two applications can ever be incomparable; the guard is a specification the alphabet has not reached, and it becomes live the moment an alphabet nests commands — which is what a circuit-shaped body does.
* **The overlap enumerator counts a metavariable position as a seam.** Over an alphabet whose every subterm is a command position, a cell whose right-hand side exposes a hole therefore overlaps every cell, so the trivial-overlap conjunct is strictly stricter than the critical-pair notion it is named after, which excludes variable positions.
  That is the enumerator's gap to close and not the guard's to work around, and it is why a two-redex fixture needs ground right-hand sides today.

### circuit-terms-spike-08

**Is the delay cut convexity-stable?** The wheel ruling is that the **cut-open form** of a delay-guarded cyclic body is an ma-cospan whose boundary carries the delays' cut ends, so cutting every delayed port turns a body $Γ → Δ$ into an acyclic $Γ + D → Δ + D$ that the whole correspondence covers.
Check the one gap that would sink it, in three steps.
**EXECUTED (2026-08-01), for the symmetric delay placement the corpus then assumed**, with the asymmetric placement's differences stated at the one answer that has any; **the placement has since been ruled asymmetric-typed** (owner, 2026-08-02, at [[#Wheels, and which structure the cell layer takes]]), which collapses those notes as marked below.
**The gap is real and it does not sink the proposal**, because what it costs is a fence on which diagram may be matched, not a repair to the convexity test.

* Take a body with one delayed back-edge and a match convex in the cut-open form; re-close the delay and check whether a path now runs from an output of the match to an input of it.
  **It can, and the witness is two cells and one delay.** Let the cut-open body have inputs $x$ and $d^{-}$, outputs $y$ and $d^{+}$, a hyperedge $f$ from $d^{-}$ to an internal node $p$, and a hyperedge $g$ from $x$ and $p$ to $y$ and $d^{+}$.
  Every node has in-degree and out-degree at most one and the two edges are ordered $f$ before $g$, so this is monogamous and acyclic; the match whose image is $f$ together with $g$ is convex there, because there is no third hyperedge for a path between the image's nodes to leave through.
  Re-closing the delay adds the edge from $d^{+}$ to $d^{-}$, and $d^{+}$ is an output of the image while $d^{-}$ is an input of it — so a directed path now runs from an output of the image back to an input of it through a hyperedge the image does not contain, which is exactly the failure a convex match is defined to exclude [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-ii, def 22 and def 33].
  **The error runs systematically in the permissive direction, and establishing that takes one line rather than a search.** Re-closure only ever adds edges, so the re-closed diagram's path set contains the cut-open one; convexity is violated by the _existence_ of a path; therefore convexity in the re-closed form implies convexity in the cut-open form and never the converse.
  The cut-open test is **strictly the more permissive of the two, by construction** — a fact about the two path sets rather than a property of any particular body.
  **The witness also separates two relations to the delay that the corpus had been carrying as one.** A left-hand side **spans** a delayed port when its image contains the delay edge, which the cut-open form makes impossible because that edge is not there; that is the unmatchable seam.
  A left-hand side **straddles** the cut when its image has a cut end among its outputs and a matching cut end among its inputs, which the cut-open form matches perfectly happily — and that is the class re-closure breaks.
  **So the seam does not protect convexity**: the cut makes one class unmatchable and leaves the complementary class matchable, and the second is the one this question is about.
* If it can, decide whether the repair is a convexity condition stated on the **re-closed** diagram (which the engine would then have to compute at match time) or a restriction on where a delay may sit relative to a redex.
  **Neither, and the reason the first is refused is the decisive one.** Convexity is _statable_ on a cyclic hypergraph — the definition quantifies over paths and asks nothing of acyclicity — but every theorem that makes convexity mean anything carries the monogamous acyclic hypothesis: the sound-and-complete correspondence, boundary-complement uniqueness, and the automatic-convexity theorem alike.
  A convexity condition computed on the re-closed diagram would therefore be a check with **no theorem behind it**, which is the shape of side condition this corpus exists to refuse.
  **What is warranted instead is the same condition computed on the cut-open form under the delay's own path extension, and that is published machinery rather than a gandr device.** The confluence paper extends the signature with three formal path generators precisely to abstract "whether certain paths exist" over the context a critical pair sits in, and a path extension is a mono adding vertices and path-labelled hyperedges that realize a prescribed relation between the boundary's outputs and inputs [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-iii, def 5.7 and def 5.8].
  The delay closure **induces** such a relation, and that is the construction's easy case: for a general system the maximal path relations are quantified over because the context is unknown, and there may be exponentially many [ibid., def 5.9 and def 5.10], whereas a delay-guarded body knows its context — the only paths re-closure adds are those routed through the edges $d^{+} → d^{-}$ for $d$ in the delay set, so the relation on a match's boundary is **computed** by reachability in the cut-open form rather than enumerated over.
  So the re-closure is not a new hazard needing new theory; it is the **known-path-relation** case of the path-joinability route, and that route's local-confluence theorem applies to it with its near-converse over the extended signature [ibid., thm 5.4 and thm 5.5].
  **The restriction on where a delay may sit survives as the cheap sufficient condition, not as the answer.** "No match may straddle the cut" is checkable against the image's boundary in $O(|L|)$ with no augmentation at all, and it is sound; it is also strictly stronger than needed, because it refuses a match touching two cut ends belonging to _different_ loops and therefore joined by no path whatever.
  A body with several independent delayed loops is the ordinary case for a stateful circuit, so the cheap condition loses exactly the matches the wheel axis was added to admit — which selects the path-extension condition on warrant and on completeness both, and leaves the restriction where the left-connectedness fence landed one spike over: **inside the answer, as the discharge that lets the sweep be skipped**.
  **One half of this is the owner's rather than the engine's, and it is flagged rather than taken.** The path-extension condition is invisible to a programmer — an engine check with a diagnostic at worst — while the straddling restriction is a surface-visible rule about where a rule may be written relative to a delay.
  Whether to expose it as such is a surface decision, and nothing here takes it.
* Either way, state whether the guard — every directed cycle contains a delayed port — stays a linear-time back-edge check, since that cheapness is most of the proposal's appeal.
  **It stays linear, and it is cheaper than that: it runs once at the cut and never again during rewriting.** On a body presented closed it is delete-the-delay-edges followed by a depth-first back-edge search, $O(|V| + |E|)$ — the shape of the linear-time acyclicity test the corpus already runs.
  **After the cut it stops being a check and becomes a consequence.** The cut-open form is acyclic, rewriting keeps it inside the monogamous acyclic fragment, and re-closure adds only delay edges — so every directed cycle of the re-closed body contains a delayed port automatically, and the guard is preserved by every step rather than re-established after each one.
  **What does need a rule is a step that changes the delay set**, because the cut is determined by that set and the cut ends are the interface: a rule adding or removing a delay moves the boundary between interface and loop object, which is an application of feedback itself rather than a rewrite with interfaces [@katis-sabadini-walters-2002-feedback, def 2.4 and prop 2.5].
  Such a rule is therefore not a cut-open rewriting step at all, and the engine owes it either a separate treatment or a refusal with a diagnostic.
  > **Ruled (owner, 2026-08-02): refusal.** A rule that adds, removes, or moves a delay is a program edit rather than a rewrite — under the asymmetric-typed placement it is a **type** change — and it is refused with a diagnostic naming the moved boundary, landing when the wheel rung first makes such a rule writable ([[#circuit-terms-rung-09]]).
  > The second-rewrite-relation treatment is declined with it: a whole relation with its own metatheory, for a use gandr does not have.
  > The reversal condition: a demonstrated retiming-shaped optimization need that cannot be expressed as an elaboration-time program transformation.

**And settle the seam question alongside it, because the two share a target.** Rewriting the cut-open form is sound with no trace axiom, feedback being an operation on hom-sets; what the cut costs is that no rule whose left-hand side spans a delayed port can match at all ([[#Wheels, and which structure the cell layer takes]]).
Decide whether the proposed criterion — the seam is **the point** for a law that would cross or erase a tick, and a **price** for a law internal to one tick — is checkable as stated, namely by whether the rule's left-hand side factors through the guard.

**The criterion is right and its stated test is not checkable, so the test is replaced rather than the criterion.** Factoring through the guard cannot be evaluated on the representation the engine matches against, because the guard **is** the cut: in the cut-open form the delay has been removed, leaving nothing for a left-hand side to factor through.

**The evaluable surrogate is an occurrence test, and it is better placed than the one it replaces.** Ask whether the rule's own left-hand side mentions a delay — decided at the declaration, in time linear in the left-hand side, once per rule rather than once per match — and it separates the two cases the criterion wants: a rule that mentions a delay is a rule about the tick boundary, for which the seam is the point and the refusal is the decline doing its work; a rule that mentions none is internal to one tick, for which the seam is a price whose exact content is that the rule cannot reach across a cut it never named.

**What makes the occurrence test well-founded rather than representation-dependent is the ruling itself.** A delay could only stop occurring if some equation erased it, and the one equation that erases a loop is yanking, which this layer declines; sliding — kept in full in one axiomatization and weakly in the other — moves a morphism around the loop without deleting the delay [@katis-sabadini-walters-2002-feedback, def 2.2 and prop 2.7] [@dilavore-defelice-roman-2022-monoidal-streams, def 3.1].
So delay occurrence is invariant under exactly the equations the cell layer keeps, and the surrogate is stable under them.
**Whether occurrence and factoring coincide is left open with its reason**: factoring quantifies over decompositions and occurrence does not, so the two agree on every case the criterion was written for without that agreement being a theorem here.

**The delay placement each answer is given for.** The witness, the permissive-direction argument, the path-extension repair, and the guard's linearity and self-preservation are all stated for the **symmetric** placement — the $Γ + D → Δ + D$ cut the corpus assumes, where the loop object appears undelayed on both sides [@katis-sabadini-walters-2002-feedback, def 2.4] — and none of them moves under the **asymmetric** placement, where the guard is a functor and the cut-open morphism is $F S ⊗ X → S ⊗ Y$ [@dilavore-defelice-roman-2022-monoidal-streams, def 3.3], because each is a statement about which directed paths exist and the two placements add the same edges.

**The seam criterion is the one answer that moves, and it moves in a direction worth the owner's attention.** Under the asymmetric placement the delay is carried by a **type**, so the occurrence test is a question the type checker already answers — a rule mentions the delay exactly when one of its ports sits at the delayed type — whereas under the symmetric placement the delay is a **generator** and the test inspects the rule's diagram content.
A smaller difference travels with it: the asymmetric cut gives the two ends of one delay **different types**, so no match can identify them, which the symmetric cut permits.

**The placement fork is closed (owner ruling, 2026-08-02): asymmetric-typed**, consistently with where the corpus's delay answer already leaned — the delay is carried by the feedback binder's typing rule and discharged by the guardedness discipline ([[#circuit-terms-question-19|circuit-terms-question-19]]), the monoidal-streams shape.
The collapse of the notes above: every placement-invariant answer stands as written; the symmetric cut shape stays valid as the placement-erased reading; and the seam criterion takes its **typed** form — a rule mentions the delay exactly when one of its ports sits at the delayed type, a question the type checker answers — which was the sharper of the two evaluable forms.
What the spike recorded as the fork's stake is thereby realized rather than mooted, and the delay former's name stays provisional per the ruling.

**Small**, and it shares its shape with [[#circuit-terms-spike-07|circuit-terms-spike-07]]: both ask whether a local independence test survives a global path condition, so running them together is cheaper than running either alone.

## The execution ladder

> **Ratified (owner, 2026-08-02).** The order the lane executes in, restated against the four axes after the multi-out ladder lapsed at the rescope.
> The retired multi-out-rung numbers stay unused, and this numbering is fresh, per the reference discipline.
> Each rung names what it needs and what it unblocks; the tracker's beads are filed against these anchors, and execution status lives in the tracker rather than here.
> The derivation dimension of the wheel axis is deliberately outside this ladder: it is ruled declined at [[#Wheels, and which structure the cell layer takes]] and reopens only by its recorded reversal condition.

1. **circuit-terms-rung-01** — **the cong2 corpus witness**: the reconvergence axis at dimension 2, alphabet-neutral by design, on the term-shaped store.
   Needs nothing outside its own sub-ladder (grammar, port fold, boundary check, elaboration, graduation, shift witness); unblocks the grammar for rung-02, the rule-member graduation gate, and the shift guard's surface consumer.
2. **circuit-terms-rung-02** — **the rewrite-face respelling**: the landed description-rule syntax and the corpus retire the old face arrow for the ruled one.
   Needs rung-01's grammar half; unblocks a uniform face spelling while the corpus is still small.
3. **circuit-terms-rung-03** — **mint `theory-circuit-algebras`** at the ruled narrowed boundary ([[#Matching, normalization, and the crate boundary]]).
   Needs the alphabet and crate rulings, both taken; unblocks rungs 04 and 05.
4. **circuit-terms-rung-04** — **diagram normal form**: the canonical linearization behind the `Rigid` device, what content addressing interns on.
   Needs rung-03; unblocks content addressing at the circuit rung and the canonicalization consumption route for the spider theorem.
5. **circuit-terms-rung-05** — **embedding-based matching behind the decided guard** ([[#circuit-terms-question-15]]).
   Needs rung-03; unblocks rungs 06 through 08 — nothing circuit-shaped matches without it.
6. **circuit-terms-rung-06** — **the tag-declared IL consumer arity**: the multi-output middle's intermediate-language half, owed independently of this arc.
   Needs nothing; unblocks rung-07 end to end.
7. **circuit-terms-rung-07** — **grow the alphabet in place, multi-output first** ([[#circuit-terms-question-01]]): the first axis through the whole language.
   The left-connectedness discharge stops being free at this rung and the strengthened guard is what carries.
   Needs rungs 05 and 06.
8. **circuit-terms-rung-08** — **disconnection**: lands after the declined device mapping's replacement direction reports ([[#circuit-terms-question-16]]), and carries the recorded route cost — a disconnected left-hand side forces path joinability, so the cheap-route claim gets its condition stated at this rung.
   Needs rung-07 and the disconnection independence story.
9. **circuit-terms-rung-09** — **wheels in the term dimension**: the delay former's spelling resolution, cut-open matching inside the recorded fences, the delay-path extension, and the delay-set refusal diagnostic ([[#circuit-terms-spike-08]]).
   Needs the delay placement and delay-set rulings, both taken; unblocks the first stateful corpus witness.

## Findings that route to other tracks

Recorded so nothing the sweep found vanishes, with the receiving track named.
None is scoped by this lane.

| finding                                                                                                                                                                                               | routes to                                                                                    |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| Semi-strictification with algebraic units and strict interchange [@chanavat-hadzihasanovic-2025-semistrictification]                                                                                  | the metatheory track's strictness-warrant question — a candidate re-warrant, not a discharge |
| The pasting theorem for frame-acyclic directed complexes [@chanavat-2026-pasting-theorem]                                                                                                             | the metatheory track's polygraph and acyclicity obligations                                  |
| Diagrammatic completeness results as worked adequacy arguments                                                                                                                                        | the metatheory track — the shape of an adequacy claim, not an implementation input           |
| Reversible programs as paths in a univalent universe, with combinator optimizations as 2-paths [@carette-chen-choudhury-sabry-2017-reversible-univalent]                                              | the identity layer, and [[durable-computation]]                                              |
| Π presented by the free symmetric rig groupoid, sound and complete at both levels [@choudhury-karwowski-sabry-2022-symmetries-reversible]                                                             | the identity layer and the certificate layer's 2-cell discipline                             |
| Join inverse categories and their rig extension as the model of reversible functional programming [@kaarsgaard-axelsen-gluck-2017-join-inverse-recursion] [@kaarsgaard-rennela-2021-join-inverse-rig] | [[durable-computation]]                                                                      |
| Pattern-matching needing structure beyond join inverse rig categories [@chardonnet-lemonnier-valiron-2021-reversible-pattern-matching]                                                                | the codata and case-tree lane, and [[durable-computation]]                                   |
| Negative and fractional types giving compact closure operationally [@chen-sabry-2021-negative-fractional]                                                                                             | the metatheory track's no-cup ruling — the worked cost of adding a cup                       |

A second group was found by the same sweep, is **not** circuit-terms material, and is routed here rather than folded, so that nothing found is lost and nothing unrelated is absorbed.
Each row is abstract-grade only; none has been read, and each is registered so the receiving track cites a key rather than a description.

| finding                                                                                                                                                                            | routes to                                                                                                                                                                                                                                    |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Mixed-variance recursion schemes via Mendler dialgebras and two-sided fibrations [@spahn-2025-mendler-dialgebras]                                                                  | the (co)recursion surface and the reflection face — gandr already rules that mixed variance is the dinaturality shape                                                                                                                        |
| Row types plus Mendler-style extensible histomorphisms, with recursive calls at larger types [@hubers-ingle-marmaduke-morris-2025-extensible-recursive]                            | the (co)recursion surface and the module/row lane                                                                                                                                                                                            |
| Differential 2-rigs, with a category of coloured species as the free one on a generator [@loregian-trimble-2023-differential-2-rigs]                                               | the metatheory track — the arity interface is already species-shaped                                                                                                                                                                         |
| Generalized automata and coalgebras in Joyal's category of species [@loregian-2024-automata-species]                                                                               | `theory-nominal-automata`, which has no automaton yet, and the metatheory track                                                                                                                                                              |
| A lambda-calculus generalization encoding higher-order store, I/O and non-determinism while staying confluent and simply typeable [@barrett-heijltjes-mccusker-2023-fmc-semantics] | the effects lane — the interesting claim for gandr is confluence _with_ effects                                                                                                                                                              |
| An explicit compositional separation of control flow from data flow [@arellanes-2026-control-data-separation]                                                                      | calibration only — it is the separation gandr's ports already make                                                                                                                                                                           |
| Effectful Mealy machines, with bisimilarity characterized via uniform feedback [@bonchi-dilavore-roman-2026-effectful-mealy]                                                       | the interchange stratification and the feedback rung                                                                                                                                                                                         |
| Resourceful traces and the commuting tensor product of free effectful categories [@earnshaw-nester-roman-2025-resourceful-traces]                                                  | this lane after all — see [[#circuit-terms-question-16\|circuit-terms-question-16]]                                                                                                                                                          |
| Colimits of quantum codes as a surgery operation, with pushouts in chain complexes of $F_2$-matrices [@cowtan-2024-code-surgery]                                                   | **nowhere — checked and routed out**, so the check is not repeated: its gluing lives in the linear, homological ambient the corpus already declines imports from, for the same reason it declines the modular-operad-as-modules presentation |

## The corpus witness plan

| witness                                                         | what it pins                                                                                        |
| --------------------------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| a many-out `op` member that elaborates and runs                 | the Π-layer; the decline half is landed, the running half waits on the alphabet                     |
| a bridge arity whose maps do not compose                        | the `WfKind::ArityDoesNotCompose` decline, at the declaration table                                 |
| two disjoint redexes in one body                                | the declined-horizontal-composition guard, writable once bodies exist                               |
| a fan-in cell whose target carries no commutative monoid        | the aggregation obligation, named at the declaration rather than implied by a picture               |
| a fan-out cell whose source carries no cocommutative comonoid   | the dual obligation, which the retired asymmetry had hidden                                         |
| a body with an unbound internal wire                            | the internal-wire binder, and the disjointness check shaped after `rwf`                             |
| a wheel with no delay                                           | the wheel guard, which is a **new** guard and owes a witness once it exists                         |
| a multi-consumer command the typed-IL checker admits or refuses | the tag-declared arity, replacing the hard-coded one                                                |
| two disjoint redexes where applying one destroys the other      | the convexity hazard, and the guard [[#circuit-terms-spike-07]] decided: the convexity re-check     |
| a match straddling a delay cut                                  | the cut-open-only matching fence, and the delay path extension [[#circuit-terms-spike-08]] selected |

Four of these are owed to the binding-guards inventory independently; this lane is where they stop being unwritable.

## Source and confidence

* **Every as-built row was verified against this tree at the time of writing**, with the crate and symbol named at the claim.
  The load-bearing negative claims — that no construction site emits more than one consumer, that the machine reads only the first, and that the description table's operations were never read — were checked at their sites.
  The last of the three has since been closed: `elaborate_data_desc` now reads `desc.ops`, admits the single-output shape, and declines the rest, which the desc → cells row records.
* **The independence machinery the two convexity spikes reason about was verified the same way**, and one of those checks is itself a finding: `cells_equal` decides boundary equality conjoined with two replays and carries **no** normal-form fast path, so the quotient the hazard threatens has no implementation yet (`theory-virtual-doctrines/src/vdc.rs`; `theory-computads/src/tracelet.rs`).
  A position is a child-index path and an overlap is a cell-pair property computed at seams, which is what makes the strengthened guard's two conjuncts land in different indexes (`theory-computads/src/pattern.rs`; `theory-computads/src/rewrite.rs`; `theory-computads/src/overlap.rs`).
* **Three further numbered statements were read in the original for those spikes** and are cited where they are used: the convex sub-hypergraph and convex match definitions, and the efficiency remark whose reachability sweep and enumeration bounds carry the cheapness argument [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-ii, def 22, def 33 and Remark 36]; and the path-relation, path-extension, maximal-path-relation and path-joinability definitions that make the delay closure an instance rather than a new device [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-iii, def 5.7–5.10].
* **The hypergraph-rewriting results are now read at theorem grade** and are cited at their own numbered statements: the monogamous-acyclic characterisation, boundary-complement uniqueness, the sound-and-complete convex correspondence including its coloured form, the left-connectedness definition and its automatic-convexity theorem, the Frobenius-semi-algebra termination proof and the disjoint-but-blocking counterexample [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-ii]; local confluence for DPO with interfaces, its computability conditions and decidability corollary, the left-connected route, and the path-joinability route with its converse [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-iii].
  The premonoidal and trace-theory results are likewise cited at their statements.
* **The feedback-category sources are read in the original rather than through an implementation of them**, and are cited at their own numbered statements: the feedback axioms and the free `Circ` construction with its universal property, the quotient by yanking that yields the free traced category, and fixed-point semantics [@katis-sabadini-walters-2002-feedback]; the guarded feedback axioms, the stateful-morphism construction and its freeness, the trace boundary, and the type theory's delay and feedback rules [@dilavore-defelice-roman-2022-monoidal-streams].
  Neither was held in the research library when the ruling was written, which is why it stood on an implementation's reading until now.
* **The remaining literature findings come from a triage sweep, not from close readings**, and are marked accordingly: abstracts and section maps, with targeted section-level reads for the supply, ancilla-scope, and reversible-term-rewriting results.
  Anything a rung depends on is filed as a spike rather than treated as established.
* **Two claims of an earlier revision did not survive and are corrected in place rather than dropped**: that fan-out is free while fan-in is not (contradicted by gandr's own monogamous carrier, by `dup`/`drop` on the surface, and by the monogamy condition of the correspondence), and that the mono-left-leg condition is what buys uniqueness at gandr's rung (superseded by boundary complements, which are unique unconditionally).
* **The readings of the implementations are this pass's own**, taken from source at the checkouts to hand, and are engineering observations rather than claims those projects make: the port-bijection representation and its stated reason [@sobocinski-wilson-zanasi-2019-cartographer], the matcher invariants, the convexity post-check, the language's refusal to name a wire and the Frobenius not-implemented path [@chyp], the spider-label wiring map [@discopy], the predicate-plus-surgery rewrite shape and the per-step semantic differential [@pyzx], and the hash-consed diagram representation with normal-form conditions as well-formedness invariants [@homotopy-rs].
* **One neighbouring result was already absorbed and carries a standing ruling**: the planar string-diagram normalization work is cited by the metatheory track with the ruling that its quotient must **not** be substituted for the symmetric one [@delpeuch-vicary-2022-normalization].
* **The hypergraph reading of the closest Agda encoding is this pass's own**, and is a fork-identification rather than a claim that thesis makes about gandr [@altenmuller-2026-string-diagrams].
