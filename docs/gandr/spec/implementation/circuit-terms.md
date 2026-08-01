# Circuit terms — computing with circuit-algebra terms

This document owns the circuit-term lane end to end: what it means for gandr to _compute_ with terms of the full circuit algebra, what the tree carries today, what must be built, and in what order.
It exists as its own component because the lane crosses every layer — the sequent IL, the L machine, the cell alphabet, the matching and normalization engines, the description universe, the checker, and the surface — and because the [[../metatheory#The substrate is the full circuit-algebra rung|generality ruling]] makes circuit structure a feature the rest of the language is expected to be designed _around_ rather than one that arrives late.

* Status: **design component; the substrate is audited and nothing is built.** Both ends of the multi-output special case exist and the middle is empty; the other three axes are not representable anywhere above the carrier.
  Every as-built claim below names the crate and symbol it was verified against.
* The **rewriting question is settled at theorem grade** as of 2026-08-01: the applicable instance is convex double-pushout rewriting with interfaces over monogamous acyclic hypergraphs, its fragment matches three of gandr's four axes exactly, and confluence there is decidable.
  What that pass opened is smaller and sharper than what it closed — a convexity hazard on a TCB-adjacent quotient, the wheel axis falling outside every published statement, and the fan-out obligation the retired asymmetry had hidden.
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

**The spider normal form is what makes the third row attractive.** Where a type supplies Frobenius, "are these two connected diagrams equal?" collapses to comparing a single many-to-many generator, which is a decision procedure gandr does not otherwise have for that fragment.
Getting it per type, on the types that genuinely have the structure, is a strictly better trade than either getting it everywhere or not at all.

**That row now has a construction rather than a hope, and it is the literature's own worked case.** A per-type supply of Frobenius structure, expressed _inside_ a Frobenius-free symmetric monoidal theory, is a pair of generators $\{μ : 2 → 1, δ : 1 → 2\}$ on that type with the Frobenius equations oriented as rules — which is the theory of **Frobenius semi-algebras**, the first case study of the correspondence paper.
Three facts transfer with it, each cited at its own statement:

* it is **terminating**, by a lexicographic reduction ordering that counts µ-trees and µ→δ paths [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-ii, thm 44];
* **acyclicity is load-bearing in that proof** — the authors state that if the two hyperedges of a Frobenius rule's left-hand side lay on a directed cycle, an infinite rewrite sequence is possible — so the supply and the wheel axis are **not independent**, and admitting wheels costs the termination argument rather than merely the representation;
* it is **not confluent** under naive critical-pair analysis, and it is that paper's own counterexample for why — see the convexity hazard under [[#The correspondence at gandr's own rung, at theorem grade]].

**The pattern's third ingredient — a preserved refuter — survives, and checking it is what makes this a supply rather than a loophole.** Because the structure is carried as _generators on one type_ rather than as an ambient assumption, a type that does not declare them simply has no such generator: a fan-in cell over it does not typecheck, and the diagnostic is at the declaration.
The invariant therefore stays falsifiable in the corpus's own sense — a program can still write the thing that lacks the structure and be told so — which is exactly what an ambient supply would have destroyed.
The witness plan below carries the refuter for both directions.

**The fourth row is settled, and the corpus's caution was right for a reason it had not yet named.** Full (co)unital Frobenius algebras **always induce a compact closed structure**, which is stated in that same case study as the reason the semi-algebra fragment is worth isolating at all [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-ii, sec. 5.1].
So a per-type _unital_ Frobenius supply is a per-type cup by construction, and the standing no-cup instruction applies to it verbatim.
A **nonunital** Frobenius supply — dropping the unit and the counit — carries no cup, and lands on the rung gandr's carrier already occupies for four independent reasons ([[../metatheory#Cellular data — descriptions, cells, and computads]]).
The disposition is therefore: **the cup row is retired in favour of the Frobenius row read nonunitally**; the negative-and-fractional-types line stays the worked account of what a cup would buy and cost [@chen-sabry-2021-negative-fractional], and nothing here proposes adding one.

## The substrate, layer by layer

Verified against the tree at the time of writing, symbol by symbol.

| layer                                                   | as built                                                                                                                                                 | verdict                                  |
| ------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------- |
| IL grammar (`core-sequent/src/il.rs`)                   | `ProducerNode::Ctor`, `ConsumerNode::Dtor`, `CommandNode::Prim`, and `CommandNode::Jump` each carry `cs: Box<[ConsumerId]>`                              | **representable** — no format break owed |
| typed-IL checker (`core-sequent/src/check.rs`)          | a `Dtor` with `cs.len() != 1` is rejected; the `Ctor`, `Prim`, and `Jump` consumer counts are walked but never counted                                   | one arity hard-coded, three unpoliced    |
| focusing (`core-sequent/src/focus.rs`)                  | every consumer-list construction site emits `Box::from([])` or a one-element list                                                                        | nothing builds it                        |
| L machine (`core-sequent/src/machine.rs`)               | `drive_prim` loads `cs.first()` and drops the rest; `CommandNode::Jump` yields `StuckReason::UnsupportedByReference`                                     | nothing runs it                          |
| cell alphabet (`theory-computads/src/pattern.rs`)       | `ConsPat::Op { op, args, ret: Box<Self> }` and `ConsPat::Frame { ctor, ret: Box<Self> }` — one return continuation **structurally**, not by a check      | **not representable**                    |
| the engines (`theory-computads`)                        | overlap enumeration, completion, normalization, composition, and tracelets are generic over `CellAlphabet` (`src/alphabet.rs`)                           | **no change owed** — the alphabet moves  |
| arity (`theory-levitation/src/arity.rs`)                | `BridgeArity { inputs, factors, source, dest, outputs }` over named `SortRef` ports, with `single_output` as the degenerate constructor                  | **built**                                |
| arity checking (`theory-levitation/src/wellformed.rs`)  | `check_arity` validates that the three maps compose, reporting `WfKind::ArityDoesNotCompose`                                                             | **built**                                |
| description table (`theory-levitation/src/desc.rs`)     | `OpDesc { name, arity, attrs }` sits in `DataDesc.ops`                                                                                                   | **built**                                |
| code universe (`theory-levitation/src/code.rs`)         | `Code::Var` is a bare recursive occurrence with no sort index                                                                                            | the index change is owed                 |
| surface grammar (`surface-grammar/src/surface/term.rs`) | `op_result()` accepts a single type or a named tuple `( ident : Type, … )`, kept local to the `op` member so it never collides with a parenthesized type | **built**                                |
| surface elaboration (`surface-engine/src/desc_elab.rs`) | `op_member` reads the ports and `bridge_arity` builds one monomial per output, each reading every input                                                  | **built**                                |
| desc → cells (`theory-computads/src/elaborate.rs`)      | `elaborate_data_desc` walks `desc.ctors` and `desc.cells`; **`desc.ops` is never read**, and the crate has no caller outside its own tests               | **the missing wire**                     |
| type interner (`core-checker/src/intern.rs`)            | the content key and the resolve/subtype API deliberately assume no arity-1 result                                                                        | no key-shape change owed later           |
| corpus                                                  | `examples/surface/data-operation-members.gandr` carries `NatDiv` with `op divmod(m, n) -> (q, r)` as a parse-only witness                                | witness exists, unpromoted               |

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
> The engine keeps enumerating critical pairs — that part was right all along — and gains three things it does not have: a definition of pre-critical pair that carries the interface, a convexity condition on matches, and a route choice (left-connected, or path joinability) that turns "the worklist drained" into a decidable claim once a termination argument exists.
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

**The convexity hazard is the finding that bears on gandr's soundness surface, not just its schedule.** Under convex rewriting, **two rule applications acting on disjoint sets of hyperedges can still block one another**: applying one can create a directed path that destroys the convexity of the other's match, so the second is no longer a legal rewrite [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-ii, ex 45].
The worked instance is the Frobenius-semi-algebra theory, and it is why that theory is terminating but not confluent.

> **gandr's shift equivalence is stated as "two adjacent cell applications at disjoint positions with trivial overlap commute", and disjointness of supports does not imply independence once matches must be convex.** The quotient is TCB-adjacent — it is what the `cells_equal` normal-form fast path decides — so this is a **guard obligation at the circuit rung**, not a scheduling note.
> Either the disjointness test is strengthened to a convexity-stable one, or the shift quotient is fenced to the fragment where convexity cannot be broken (left-connected left-hand sides are the published such fragment).
> Recorded as [[#circuit-terms-question-15|circuit-terms-question-15]] and [[#circuit-terms-spike-07|circuit-terms-spike-07]].

**What the confluence theory cannot give gandr is the wheel axis.** Acyclicity is a hypothesis of the ma-fragment, of convexity (the path relation is a relation on directed paths), and of the termination arguments; the correspondence paper is explicit that a Frobenius-free rewrite that would need to move a box past a redex requires "at least a traced symmetric monoidal structure" to be applied at all [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-iii, ex 5.2]. gandr's arity ruling has since made the two-sided closure — the trace — the **primitive** former ([[../metatheory#The arity interface, universe-style]]), so gandr's destination rung is traced symmetric monoidal without Frobenius, which is **outside** everything this line proves.
That gap is now the lane's largest cited unknown, and it is stated as such rather than assumed away.

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

gandr's circuit-body sketch wires by name sharing and has **no construct that says a name is internal to a block** — so every intermediate wire in a body is either accidentally part of the interface or needs a separate interface declaration.
The reversible-circuit language Ricercar supplies exactly the missing construct, and it is worth transcribing because three of its four parts transfer [@thomsen-kaarsgaard-soeken-2015-ricercar].

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
A generator is annotated with the devices it touches; a **resourceful trace** is a string diagram in which each device string appears at most once in each vertical section; the free effectful category over an effectful graph has resourceful traces as its morphisms, by an adjunction; and the **commuting tensor product** of free effectful categories combines two systems whose actions must commute while still exchanging resources.
The single-runtime construction is recovered as the one-device case, and ordinary Mazurkiewicz traces as the no-resources case.

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
**A repeated hole in a rule's left-hand side is precisely that case, and gandr admits one today.** Verified at source: `CellMeta::derive` in `theory-computads/src/sequent.rs` sets `linear: CellLinearity::from(lhs_count == 1)`, so a metavariable occurring twice on the left is **admitted and recorded as non-linear** rather than rejected, with `a_repeated_metavariable_is_nonlinear` exhibiting `⟨Pair(x; x) | α⟩`; and the substitution layer's matcher accepts a repeated occurrence by binding once and requiring agreement on the rebind (`subst.rs`, conflicting-rebind contract).
At the circuit rung that same pattern stops being free substitution and becomes a copy on a wire, and the leading implementation of gandr's own rung declines it explicitly.
This is the sharpest single consequence of the term-shaped-to-circuit-shaped move, and it is recorded as [[#circuit-terms-question-17|circuit-terms-question-17]].

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

**The crate boundary.** The proposal of record is a new **`theory-circuit-algebras`** beside the existing theory crates, owning the circuit-term representation, its interfaces and internal-wire binders, embedding-based matching, and diagram normal form — with `theory-computads` continuing to own cells, overlaps, completion, and tracelets over whatever alphabet it is given.
The seam is the `CellAlphabet` trait, which already exists and is already the place an alphabet is supplied, so the new crate would be an _inhabitant_ of that interface rather than a fork of the engines.
Deciding this before the alphabet grows is cheaper than after, and it is [[#The design questions|circuit-terms-question-12]].

## The design questions

Each is anchored and cited by link, never by position.
Every one carries a disposition.

1. **circuit-terms-question-01** — **does the alphabet grow in place, or does a second alphabet stand beside the first?** Growing the pattern type fires the compile-visible tripwire at every match site, which is what the pattern grammar's narrowness was designed for; a second `CellAlphabet` inhabitant leaves the landed one untouched at the price of two to maintain.
   **Carried**, and coupled to the crate question below: a new crate makes the second-inhabitant answer the natural one.
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
    **Carried, and cheapest settled before the alphabet grows** — but **narrower than it was**: the representation half is not new work, because the monogamous fragment's canonical representation is a port bijection and the Agda carrier already is one, so what the crate would own is interface bookkeeping, embedding-based matching with its convexity check, and diagram normal form.
13. **circuit-terms-question-13** — **does gandr want checked implicit coercions**, and is the circuit-term boundary one of their first customers?
    The observation is that a proof assistant's coercion mechanism is normally a bare insertion rule with no evidence attached, whereas gandr already plans a directed transformation family, a certificate layer, and named rewrite cells — so a coercion could be an **inhabitant of an existing evidence type** rather than new machinery, and mediating between primitive terms and circuit terms is the obvious motivating case.
    **Carried as a future direction, explicitly not scoped**, with its hazards named in the spike below.
14. **circuit-terms-question-14** — **is gandr's cell layer an effectful category?** Interchange holding only on a declared subclass of morphisms is the defining feature of premonoidal and effectful categories, and gandr's disjoint-positions reversal condition is that shape arrived at independently.
    **Closed in the affirmative for the single-spine layer, and the runtime object is what the spine becomes.** The identification is a theorem rather than an analogy [@roman-sobocinski-2025-premonoidal-string-diagrams, thm 3.14], the same source names call-by-push-value as the closest programming-language rendering, and there is a positive argument for representing the spine rather than keeping it structural: a structural spine is the side-table alternative the authors reject because it **loses locality of substitution** [ibid., rmk 2.8], which is exactly the property embedding-based matching needs.
    What does not close with it is the disconnection axis, which is [[#circuit-terms-question-16|circuit-terms-question-16]].
15. **circuit-terms-question-15** — **is gandr's disjointness test convexity-stable?** Under convex rewriting, two rule applications on **disjoint** sets of hyperedges can block one another, because one can create a directed path that destroys the other's convexity [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-ii, ex 45]. gandr's shift equivalence commutes adjacent applications at disjoint positions with trivial overlap, and that quotient is TCB-adjacent.
    **Carried, and it is a guard obligation rather than a design preference** — the two available answers are a strengthened disjointness test or a fence to the left-connected fragment, and neither may be assumed.
    [[#circuit-terms-spike-07|circuit-terms-spike-07]] decides it.
16. **circuit-terms-question-16** — **which of gandr's structures is a device, and which is a resource?** One runtime object means one sequentialization and makes every cell depend on every other [@earnshaw-sobocinski-2023-string-diagrammatic-trace-theory, prop 34], which is the opposite of what the disconnection axis is for.
    The structure that fits exists and is built: effectful categories over **several devices**, where a device is a shared resource whose sharing creates dependence and a resource is a typed thing actions consume and produce, with free effectful categories presented by **resourceful traces** and a **commuting tensor product** combining systems whose actions must commute [@earnshaw-nester-roman-2025-resourceful-traces].
    **Carried, as a mapping question rather than a gap.** The candidate reading is that gandr's structural spine is one device, disconnection is several, gandr's types are resources, and the interchange stratification is what the commuting tensor product constructs — and none of those four is checked.
    An earlier revision of this entry recorded the construction as unbuilt, on the 2023 paper's future-work note; the follow-up was in the library and the claim is corrected rather than carried.
17. **circuit-terms-question-17** — **what happens to non-linear patterns?** A repeated hole on a rule's left-hand side is free in a term-shaped store, because substitution copies; at the circuit rung it is a **copy on a wire**, which is a comonoid the type may not have.
    The leading implementation of gandr's own rung refuses exactly this case by name, raising a not-implemented error labelled "rewriting modulo Frobenius" as soon as a boundary vertex is used more than once [@chyp].
    **Carried, and it is the sharpest single consequence of the term-shaped-to-circuit-shaped move** — gandr admits non-linear patterns today (`CellMeta::derive`, `theory-computads/src/sequent.rs`, verified at source), so this is a live behaviour change and not a hypothetical.
18. **circuit-terms-question-18** — **is the fan-out obligation carried as a per-type supply, and at which layer?** The fan-out/fan-in asymmetry did not survive this pass, so copy owes the same treatment as merge. gandr already prices duplication on the **value** side, with grades and with `dup`/`drop` as ordinary computations; what has no answer is what the obligation means at the **cell** layer, where a repeated hole is the thing that would have to carry it.
    **Carried**, and coupled to [[#circuit-terms-question-17|circuit-terms-question-17]] — they are the same fact seen from the supply side and from the pattern side.
19. **circuit-terms-question-19** — **what covers the wheel axis?** Acyclicity is a hypothesis of the monogamous-acyclic fragment, of convex matching, and of the published termination arguments; gandr's arity ruling has made the trace primitive, so gandr's destination is traced symmetric monoidal without Frobenius, which no statement in the hypergraph-rewriting line reaches.
    **Carried, and it is now the lane's largest cited unknown** — the previous largest, whether the DPO instance applies at all, closed above.

## Spikes

### circuit-terms-spike-01

**Does the hypergraph DPO-with-interfaces instance apply to gandr's circuit cells?** Take one circuit cell shape, write it as a hypergraph with interface, and check three claims: that the interface is the coproduct of the cell's input and output ports, that gandr's rules have mono left legs so pushout complements are unique, and that a rewrite respecting the interface is the same relation as a gandr cell application.
**EXECUTED (2026-08-01), and the numbering is retained rather than reused.** Its three claims resolved as follows, against the sources rather than against a toy encoding.

* **The interface is the coproduct of the cell's input and output ports** — **confirmed as the setting's own definition**: a rule is $L ← i + j → R$ with $i + j$ discrete, and a hypergraph with interface is $G ← n + m$ for the ma-cospan $n → G ← m$ [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-iii, sec. 5.1].
* **gandr's rules have mono left legs, so pushout complements are unique** — **the wrong question at this rung, and it dissolves.** The Frobenius-free theory does not use plain pushout complements; it uses **boundary complements**, which are unique whenever they exist, explicitly including rules that are not left-linear [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-ii, prop 31]. gandr does **not** have mono left legs in general — non-linear patterns are admitted and their linearity recorded as derived metadata — so the claim as posed would have failed; the uniqueness gandr needs comes from elsewhere and is unconditional.
* **A rewrite respecting the interface is the same relation as a gandr cell application** — **confirmed for the correspondence and not yet for gandr.** The correspondence is an iff for arbitrary rewriting systems over arbitrary (including coloured) symmetric monoidal theories, at [ibid., thm 35 and thm 39], **provided the rewrite is convex**; whether gandr's cell application is convex is not a fact about the literature and is now [[#circuit-terms-question-15|circuit-terms-question-15]].

**The verdict is that the corpus's scoping is re-scoped, not contradicted**, and the re-scoping is written at [[#The correspondence at gandr's own rung, at theorem grade]].

### circuit-terms-spike-02

**Measure the enumerator blowup.** Instantiate the overlap enumerator on a toy circuit alphabet and count the overlap families against the single-output baseline, so the enumerator-cost question is answered by measurement rather than prediction.
**Small**, and worth running before the alphabet grows, because the number is an input to whether ordered ports are enough.

### circuit-terms-spike-03

**RE-SCOPED (2026-08-01) to the warrant, because the construction is settled.** The original question — is the fan-in obligation expressible as a per-type supply — is answered in the affirmative under [[#Per-type supply as a general decline-relaxation pattern]], with the construction, its termination theorem, and its no-cup condition all cited.
What remains is one thing and it is a warrant question: **does the sound-and-complete correspondence survive a _mixed_ signature**, where some colours supply the structure and others do not?
The published coloured statements quantify Frobenius over every colour, so nothing yet covers the mixed case; check whether the encoding of a supply as ordinary generators plus rules keeps the whole theory inside the monogamous acyclic fragment, and whether the resulting system is left-connected (which would make its confluence decidable by the cheap route) or needs path joinability.
**Small**, and it is now the gate on the second row of the supply table rather than on the first.

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

* Does an analogous pair exist over gandr's cell fragment once patterns are circuit-shaped, or does the cut-rooted left-hand-side discipline already exclude it?
* If it exists, is the repair a strengthened disjointness predicate (disjoint **and** no new path created between the other match's boundary), or a fence of the shift quotient to left-connected left-hand sides?
* Either way, what does the `cells_equal` fast path have to check before it may commute two applications, and does that check stay cheap enough to keep the fast path a fast path?

**Small to write, and it is the one item in this lane that touches a TCB-adjacent surface**, so it runs before the alphabet grows rather than after.
The honest default until it runs is that the shift quotient is warranted only on the fragment where matches cannot be non-convex, which today is the whole cell store and after the alphabet change is not.

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

| finding                                                                                                                                                                            | routes to                                                                                                             |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| Mixed-variance recursion schemes via Mendler dialgebras and two-sided fibrations [@spahn-2025-mendler-dialgebras]                                                                  | the (co)recursion surface and the reflection face — gandr already rules that mixed variance is the dinaturality shape |
| Row types plus Mendler-style extensible histomorphisms, with recursive calls at larger types [@hubers-ingle-marmaduke-morris-2025-extensible-recursive]                            | the (co)recursion surface and the module/row lane                                                                     |
| Differential 2-rigs, with a category of coloured species as the free one on a generator [@loregian-trimble-2023-differential-2-rigs]                                               | the metatheory track — the arity interface is already species-shaped                                                  |
| Generalized automata and coalgebras in Joyal's category of species [@loregian-2024-automata-species]                                                                               | `theory-nominal-automata`, which has no automaton yet, and the metatheory track                                       |
| A lambda-calculus generalization encoding higher-order store, I/O and non-determinism while staying confluent and simply typeable [@barrett-heijltjes-mccusker-2023-fmc-semantics] | the effects lane — the interesting claim for gandr is confluence _with_ effects                                       |
| An explicit compositional separation of control flow from data flow [@arellanes-2026-control-data-separation]                                                                      | calibration only — it is the separation gandr's ports already make                                                    |
| Effectful Mealy machines, with bisimilarity characterized via uniform feedback [@bonchi-dilavore-roman-2026-effectful-mealy]                                                       | the interchange stratification and the feedback rung                                                                  |
| Resourceful traces and the commuting tensor product of free effectful categories [@earnshaw-nester-roman-2025-resourceful-traces]                                                  | this lane after all — see [[#circuit-terms-question-16\|circuit-terms-question-16]]                                   |

## The corpus witness plan

| witness                                                         | what it pins                                                                          |
| --------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| a many-out `op` member that elaborates and runs                 | the Π-layer, promoted from the existing parse-only witness                            |
| a bridge arity whose maps do not compose                        | the `WfKind::ArityDoesNotCompose` decline, at the declaration table                   |
| two disjoint redexes in one body                                | the declined-horizontal-composition guard, writable once bodies exist                 |
| a fan-in cell whose target carries no commutative monoid        | the aggregation obligation, named at the declaration rather than implied by a picture |
| a fan-out cell whose source carries no cocommutative comonoid   | the dual obligation, which the retired asymmetry had hidden                           |
| a body with an unbound internal wire                            | the internal-wire binder, and the disjointness check shaped after `rwf`               |
| a wheel with no delay                                           | the wheel guard, which is a **new** guard and owes a witness once it exists           |
| a multi-consumer command the typed-IL checker admits or refuses | the tag-declared arity, replacing the hard-coded one                                  |
| two disjoint redexes where applying one destroys the other      | the convexity hazard, and whatever [[#circuit-terms-spike-07]] decides the guard is   |

Four of these are owed to the binding-guards inventory independently; this lane is where they stop being unwritable.

## Source and confidence

* **Every as-built row was verified against this tree at the time of writing**, with the crate and symbol named at the claim.
  The load-bearing negative claims — that no construction site emits more than one consumer, that the machine reads only the first, and that the description table's operations are never read — were checked at their sites.
* **The hypergraph-rewriting results are now read at theorem grade** and are cited at their own numbered statements: the monogamous-acyclic characterisation, boundary-complement uniqueness, the sound-and-complete convex correspondence including its coloured form, the Frobenius-semi-algebra termination proof and the disjoint-but-blocking counterexample [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-ii]; local confluence for DPO with interfaces, its computability conditions and decidability corollary, the left-connected route, and the path-joinability route with its converse [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-iii].
  The premonoidal and trace-theory results are likewise cited at their statements.
* **The remaining literature findings come from a triage sweep, not from close readings**, and are marked accordingly: abstracts and section maps, with targeted section-level reads for the supply, ancilla-scope, and reversible-term-rewriting results.
  Anything a rung depends on is filed as a spike rather than treated as established.
* **Two claims of an earlier revision did not survive and are corrected in place rather than dropped**: that fan-out is free while fan-in is not (contradicted by gandr's own monogamous carrier, by `dup`/`drop` on the surface, and by the monogamy condition of the correspondence), and that the mono-left-leg condition is what buys uniqueness at gandr's rung (superseded by boundary complements, which are unique unconditionally).
* **The readings of the implementations are this pass's own**, taken from source at the checkouts to hand, and are engineering observations rather than claims those projects make: the port-bijection representation and its stated reason [@sobocinski-wilson-zanasi-2019-cartographer], the matcher invariants, the convexity post-check, the language's refusal to name a wire and the Frobenius not-implemented path [@chyp], the spider-label wiring map [@discopy], the predicate-plus-surgery rewrite shape and the per-step semantic differential [@pyzx], and the hash-consed diagram representation with normal-form conditions as well-formedness invariants [@homotopy-rs].
* **One neighbouring result was already absorbed and carries a standing ruling**: the planar string-diagram normalization work is cited by the metatheory track with the ruling that its quotient must **not** be substituted for the symmetric one [@delpeuch-vicary-2022-normalization].
* **The hypergraph reading of the closest Agda encoding is this pass's own**, and is a fork-identification rather than a claim that thesis makes about gandr [@altenmuller-2026-string-diagrams].
