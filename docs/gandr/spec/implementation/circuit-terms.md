# Circuit terms — computing with circuit-algebra terms

This document owns the circuit-term lane end to end: what it means for gandr to _compute_ with terms of the full circuit algebra, what the tree carries today, what must be built, and in what order.
It exists as its own component because the lane crosses every layer — the sequent IL, the L machine, the cell alphabet, the matching and normalization engines, the description universe, the checker, and the surface — and because the [[../metatheory#The substrate is the full circuit-algebra rung|generality ruling]] makes circuit structure a feature the rest of the language is expected to be designed _around_ rather than one that arrives late.

* Status: **design component; the substrate is audited and nothing is built.** Both ends of the multi-output special case exist and the middle is empty; the other three axes are not representable anywhere above the carrier.
  Every as-built claim below names the crate and symbol it was verified against.
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

## Fan-in, supply, and Frobenius, in plain terms

This vocabulary recurs below and in the cited literature, so it is stated once here with examples rather than assumed.

**Fan-out** is one wire going to two places.
**Fan-in** is two wires arriving at one place.
On a diagram they look like mirror images, and they are not symmetric in what they cost.

Fan-out is **free**, in the exact sense that it needs no structure on the type: to send one value to two destinations the wiring map simply names two targets, and nothing has to be decided.
The corpus states this as _routing is free — non-combining multi-output is just the target map_.

Fan-in is **not free**, because two things arriving at one place have to become one thing, and nothing in the wiring says how.
Concretely: if two producers both feed one `Pipe` port, the target must answer "what are these two contributions, together?" — and that answer is a binary operation.
For the diagram to denote anything, the operation must not depend on which contribution the wiring happens to present first (**commutativity**) or on how a three-way fan-in is bracketed (**associativity**), and an empty fan-in must mean something (**a unit**).
A commutative, associative operation with a unit is exactly a **commutative monoid**, which is why the corpus says a combining fan-in cell is lawful only where its target carries one.

```text
// free: routing. one source, two destinations, nothing combined.
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

Three standing declines are candidates for exactly this treatment, and each is recorded here as a candidate rather than as a proposal.

| decline                                  | what per-type supply would change                                                                                                                                                                                                                                                                                                                                                 |
| ---------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **ambient free fan-in** (the case above) | fan-in cells become lawful exactly where the commutative monoid is supplied, and unlawful with a diagnostic elsewhere                                                                                                                                                                                                                                                             |
| **Frobenius structure**                  | a type that supplies Frobenius gets the **spider normal form** — a decision procedure for connected diagrams — without every type getting split, merge, init, and discard                                                                                                                                                                                                         |
| **the cup**, and with it compact closure | the standing instruction is that a cup must never be added to make an operation total, and that adding one brings three consequences at once. A per-type cup does not obviously satisfy that instruction — the no-cup consequences are stated about the **carrier**, not about a type — so this is the candidate that most needs the instruction re-read before anyone acts on it |

**The spider normal form is what makes the second row attractive.** Where a type supplies Frobenius, "are these two connected diagrams equal?" collapses to comparing a single many-to-many generator, which is a decision procedure gandr does not otherwise have for that fragment.
Getting it per type, on the types that genuinely have the structure, is a strictly better trade than either getting it everywhere or not at all.

**The third row is the one to be careful with**, and it is recorded with its hazard rather than its appeal.
The negative-and-fractional-types line shows what compact closure buys operationally and what it costs [@chen-sabry-2021-negative-fractional]; the corpus's no-cup consequences are stated at the carrier, and a per-type reading would have to establish that a type-level cup does not smuggle a carrier-level one.
That is a metatheory question, not an implementation one.

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

Findings from a triage sweep of the string-diagram and circuit-algebra implementations, which have been computing with these structures for a decade.
Each names what it supplies and what it does not.

### The hypergraph correspondence is the applicable rewriting instance once cells stop being trees

The metatheory track records that gandr's double-pushout inheritance is nominal, with no pushout complement in code, because the term-rewriting double-category instance is the right shape for a **term-shaped** cell store and the graph-shaped double-pushout instances do not apply.
That scoping is correct today and **stops being correct at the moment the cell grammar admits reconvergence or many-out**, because the cells stop being term-shaped.

The literature has already built the replacement [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting]:

* string-diagram rewriting **modulo Frobenius structure** corresponds exactly to **double-pushout rewriting on hypergraphs**, proved sound and complete, and generalized to rewriting modulo _multiple_ Frobenius structures;
* labelled directed hypergraphs form a **presheaf topos and are therefore adhesive**, which is what makes DPO well-behaved there;
* the operative notion is **DPO with interfaces**, where a rewrite is taken relative to an interface that lets the diagram be glued into a larger, possibly unknown context — the interface decides which rewrites are applicable at all;
* pushout complements are **unique when the rule's left leg is mono**, and effectively enumerable when they are not.

That last point is the same phenomenon gandr already records from the virtual reading — non-linear overlaps fan out into families rather than a single fused rule — arriving independently from the DPO side, which is corroboration rather than a new constraint.

**The caveat is the Frobenius assumption**, which is the free fan-in this lane declines ambiently.
The sequel that drops it — rewriting modulo _symmetric monoidal_ structure without Frobenius — is closer to gandr's rung and is the first item of the deeper sweep this finding triggers ([[../metatheory/roadmap]]).

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

The supply notion, its coherence theorem, and its survival of strictification are stated under [[#Fan-in, supply, and Frobenius, in plain terms]] above [@fong-spivak-2020-supply].
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

What the source supplies beyond the vocabulary: string diagrams with an added **runtime object** are an internal language for effectful, premonoidal, and Freyd categories.
The runtime object is a wire threaded through every generator that has not been declared to interchange, which is exactly how a sequential spine is made visible inside a diagram — and gandr's single-spine cell grammar is that same spine, currently structural rather than represented.

The adjacent trace-theory line makes the sequentialization question its subject: Mazurkiewicz trace languages are exactly symmetric monoidal languages over distributed alphabets, and premonoidal string diagrams are used to **derive serializations of traces** [@earnshaw-sobocinski-2023-string-diagrammatic-trace-theory].
"How many sequentializations does this diagram have, and when do they agree" is the question gandr's interchange decline is a special case of, and it has a literature.

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

## Matching, normalization, and the crate boundary

The three faces that make this "computing with" rather than "representing".

**Matching.** gandr's one-sided matcher and two-sided unifier are written against a pattern language whose consumer side is a linear spine.
A circuit pattern is not a spine and not a tree: it has several roots, may reconverge, and may have components with no wire between them.
Matching therefore stops being a structural recursion and becomes a **sub-diagram embedding problem**, which is where the DPO line's matching-plus-pushout-complement formulation is the published answer and where the mono-left-leg condition earns its keep.
The open half is what the corpus's own span-level seam data means when a match is an embedding rather than a position.

**Normalization.** Two normal-form questions must be kept apart, and conflating them is the hazard.

* _Diagram normal form_ — when do two circuit terms denote the same diagram?
  For the connected Frobenius case this is the spider collapse; in general it is a graph-isomorphism-flavoured question, and the corpus's own linear-time acyclicity test is a different and weaker check.
* _Rewriting normal form_ — the result of running the rewrite system to completion, which is what the certificate algebra already means by normalization.

The first is a property of the representation and is what content-addressing must intern on; the second is a property of the theory. gandr's `Rigid` device is where the first lands, and `Rigid.canon-sound` at the circuit rung is the standing obligation that owes it.
**Whether the first needs machinery of its own is the lane's largest unpriced question**, and it is what would justify a crate of its own.

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
   **Carried.**
6. **circuit-terms-question-06** — **does the hypergraph DPO instance become the applicable one at this rung**, retiring the scoping that says graph-shaped double-pushout instances do not apply?
   **Carried, and owed a decision before the alphabet changes** — the scoping is stated in the metatheory track and would otherwise become wrong silently.
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
    **Carried, and cheapest settled before the alphabet grows.**
13. **circuit-terms-question-13** — **does gandr want checked implicit coercions**, and is the circuit-term boundary one of their first customers?
    The observation is that a proof assistant's coercion mechanism is normally a bare insertion rule with no evidence attached, whereas gandr already plans a directed transformation family, a certificate layer, and named rewrite cells — so a coercion could be an **inhabitant of an existing evidence type** rather than new machinery, and mediating between primitive terms and circuit terms is the obvious motivating case.
    **Carried as a future direction, explicitly not scoped**, with its hazards named in the spike below.
14. **circuit-terms-question-14** — **is gandr's cell layer an effectful category?** Interchange holding only on a declared subclass of morphisms is the defining feature of premonoidal and effectful categories, and gandr's disjoint-positions reversal condition is that shape arrived at independently.
    **Carried**, with the specific sub-question being whether the **runtime object** device — a wire threaded through every generator not declared to interchange — is what gandr's single-spine cell grammar should become once the spine is represented rather than structural.

## Spikes

### circuit-terms-spike-01

**Does the hypergraph DPO-with-interfaces instance apply to gandr's circuit cells?** Take one circuit cell shape, write it as a hypergraph with interface, and check three claims: that the interface is the coproduct of the cell's input and output ports, that gandr's rules have mono left legs so pushout complements are unique, and that a rewrite respecting the interface is the same relation as a gandr cell application.
**Small.** Settles the DPO-applicability question and decides whether the corpus's scoping is retired or re-scoped rather than contradicted.

### circuit-terms-spike-02

**Measure the enumerator blowup.** Instantiate the overlap enumerator on a toy circuit alphabet and count the overlap families against the single-output baseline, so the enumerator-cost question is answered by measurement rather than prediction.
**Small**, and worth running before the alphabet grows, because the number is an input to whether ordered ports are enough.

### circuit-terms-spike-03

**Is the fan-in obligation expressible as a per-type supply over gandr's own formers?** Write the obligation for one concrete target, check that the coherence the supply theorem grants is the coherence gandr would otherwise check per cell, and confirm the construction stays per-type rather than ambient.
**Small**, and it is the construction whose existence narrows the fan-in decline.

### circuit-terms-spike-04

**Is gandr's cell-visible pattern fragment position-removable?** Compare the pure-constructor restriction class of the reversible-rewriting line against gandr's cell pattern discipline and decide whether recorded positions are necessary or merely convenient.
**½ day.**

### circuit-terms-spike-05

**Does the spider normal form give fan-in cells a decision procedure?** Where a supply exists, check whether the connected-diagram collapse is checkable on gandr's representation, and how it relates to the corpus's linear-time acyclicity test.
**Small.** Feeds the diagram-normal-form half rather than the rewriting half.

### circuit-terms-spike-06

**What would a checked implicit coercion cost, and what would it be evidence of?** Write one coercion between a primitive term and a circuit term and answer three questions: what evidence type inhabits it; whether coherence — two coercion paths between the same pair agreeing — is a theorem, a certificate, or an unmet obligation; and whether insertion is decidable without search.
**Unmeasured, and deliberately fenced.** The hazards are named rather than solved: an insertion rule that fires silently is a readability surface and a soundness surface at once, and the corpus's standing position that variance is derived and never declared has coercion as its neighbouring temptation.

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

## The corpus witness plan

| witness                                                         | what it pins                                                                          |
| --------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| a many-out `op` member that elaborates and runs                 | the Π-layer, promoted from the existing parse-only witness                            |
| a bridge arity whose maps do not compose                        | the `WfKind::ArityDoesNotCompose` decline, at the declaration table                   |
| two disjoint redexes in one body                                | the declined-horizontal-composition guard, writable once bodies exist                 |
| a fan-in cell whose target carries no commutative monoid        | the aggregation obligation, named at the declaration rather than implied by a picture |
| a body with an unbound internal wire                            | the internal-wire binder, and the disjointness check shaped after `rwf`               |
| a wheel with no delay                                           | the wheel guard, which is a **new** guard and owes a witness once it exists           |
| a multi-consumer command the typed-IL checker admits or refuses | the tag-declared arity, replacing the hard-coded one                                  |

The middle three are owed to the binding-guards inventory independently; this lane is where they stop being unwritable.

## Source and confidence

* **Every as-built row was verified against this tree at the time of writing**, with the crate and symbol named at the claim.
  The load-bearing negative claims — that no construction site emits more than one consumer, that the machine reads only the first, and that the description table's operations are never read — were checked at their sites.
* **The literature findings come from a triage sweep, not from close readings**, and are marked accordingly: abstracts and section maps for the whole set, with targeted section-level reads for the hypergraph-rewriting, supply, ancilla-scope, and reversible-term-rewriting results.
  Anything a rung depends on is filed as a spike rather than treated as established.
* **The Ricercar transcription is checked against its own figures**; the four-clause well-formedness fold and the inversion clause are quoted, not paraphrased.
* **One neighbouring result was already absorbed and carries a standing ruling**: the planar string-diagram normalization work is cited by the metatheory track with the ruling that its quotient must **not** be substituted for the symmetric one [@delpeuch-vicary-2022-normalization].
* **The hypergraph reading of the closest Agda encoding is this pass's own**, and is a fork-identification rather than a claim that thesis makes about gandr [@altenmuller-2026-string-diagrams].
* The deeper sweep these findings trigger is scheduled by the metatheory track's reading queue, not here.
