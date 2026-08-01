# Multi-out — the destination-passing term face

This document owns the multi-out lane end to end: what a many-output operation _is_ in gandr, what the tree carries today, what must be built, and in what order.
It exists as its own component because the lane crosses every layer — the sequent IL, the L machine, the cell alphabet, the description universe, the checker, and the surface — and because the [[../metatheory#The substrate is the full circuit-algebra rung|generality ruling]] makes it a feature the rest of the language is expected to be designed _around_ rather than one that arrives late.

* Status: **the two ends are built and the middle is empty.** The surface writes multi-out and the description layer models it exactly; nothing between them can represent, check, or run it.
  Every as-built claim below names the crate and symbol it was verified against.
* The carrier-side ruling this lane serves is the metatheory track's generality ruling; the surface half of the same question is the design sketch [[../surface-language/circuit-cells]], whose concrete syntax is deliberately unsettled and lands last.
* The mathematics of the arity is the metatheory track's [[../metatheory#Cellular data — descriptions, cells, and computads|bridge-diagram account]]; nothing here proposes changing it.

## What multi-out is, and what it is not

**Arity is the retired axis.** The guards ledger tombstones "restrict to dioperads, therefore give up many-out" with the reason that dioperads have the same colour set as properads, and what the higher rungs add is reconvergence, disconnection, and wheels rather than arity.
A lane organized around "more than one return value" would be answering a closed question.

What the lane is actually about is **where the outputs go**.
An operation with two results whose outputs both land in one consumer is a tuple, and gandr can already write that.
Multi-out is the case where the outputs go to _different destinations_, no product is allocated, and the wiring is the datum — which is why the term face is called destination-passing and why it has consequences at the machine rather than only in the type.

**The face splits in two, and the halves cost differently.** The [[../metatheory#Cellular data — descriptions, cells, and computads|bridge diagram]] $A ←^s J →^π I →^t B$ separates them [@spivak-garner-fairbanks-2021-aggregation]:

| layer       | what it is                                           | what it costs                                                                                      |
| ----------- | ---------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| **Π-layer** | one operation's named result tuple                   | **free** — non-combining multi-output is just the target map, pure routing                         |
| **Σ-layer** | aggregating several contributions to one destination | **not free** — requires a commutative monoid on the target; unrestricted fan-in is not free wiring |

The Π-layer is what `op divmod(m, n) -> (q: Nat, r: Nat)` asks for, and it is the near-term deliverable.
The Σ-layer is a structural obligation on the target type, and the literature sweep below upgrades what kind of obligation it is.

**The aggregation colimit is a specification; the destination-passing writeback is its operational realization.** The source gives no execution, cost, or linearity story and must not be over-read as one.
Supplying that story is this lane's own work, and it is the part with no precedent: every machine and IR in the [[performance-architecture|performance-architecture]] read is single-result.

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

The summary a reader should take away: **the sequent layer's support for multi-out is exactly that a field is a `Vec`.** That is real and worth having, because it means no serialized format or node layout has to change.
It is also strictly weaker than support, and the cell alphabet — the layer where many-out interfaces would actually earn something — cannot express them at all.

## What the diagrammatic-rewriting literature supplies

The lane's neighbours are the string-diagram and circuit-algebra implementations, which have been computing with many-in/many-out structures for a decade.
The findings below are what a triage sweep of that literature returned; each names what it supplies and what it does not.

### The hypergraph correspondence is the applicable rewriting instance once cells go many-out

The metatheory track records that gandr's double-pushout inheritance is nominal, with no pushout complement in code, because the term-rewriting double-category instance is the right shape for a **term-shaped** cell store and the graph-shaped double-pushout instances do not apply.
That scoping is correct today and **stops being correct at the moment the cell grammar goes many-out**, because the cells stop being term-shaped.

The literature has already built the replacement, and it is not a sketch [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting]:

* string-diagram rewriting **modulo Frobenius structure** corresponds exactly to **double-pushout rewriting on hypergraphs**, proved sound and complete, and generalized to rewriting modulo _multiple_ Frobenius structures;
* the category of labelled directed hypergraphs is a **presheaf topos and therefore adhesive**, which is what makes DPO well-behaved there;
* the operative notion is **DPO with interfaces**, where a rewrite is taken relative to an interface that lets the diagram be glued into a larger, possibly unknown context — the presence of the interface decides which rewrites are applicable at all;
* pushout complements are **unique when the rule's left leg is mono**, and when they are not unique they can still be **effectively enumerated**.

That last point is the same phenomenon gandr already records from the virtual reading — non-linear overlaps fan out into families rather than a single fused rule — arriving independently from the DPO side, which is corroboration rather than a new constraint.
The free Frobenius PROP being isomorphic to cospans of finite sets is the combinatorial characterisation that makes the correspondence computable.

**What it does not supply.** It is a theory of rewriting modulo Frobenius, so it assumes the fan-in structure gandr's aggregation split treats as an obligation to be discharged per target.
Importing it wholesale would silently supply commutative Frobenius everywhere, which is exactly the free-fan-in gandr declines.

### Fan-in is a supply, not a per-cell side condition

gandr currently states the fan-in obligation as a proviso attached to a cell — a combining `merge` is lawful only because its target carries a commutative monoid.
The published treatment makes that a structure on the category rather than a note on the cell: a symmetric monoidal category **supplies** a prop when every object is equipped with that prop's structure compatibly with the monoidal product [@fong-spivak-2020-supply].

Three of its results bear directly:

* all the coherence isomorphisms of the ambient category — associators, unitors, braiding — are **automatically homomorphisms** for any supply, so the compatibility gandr would otherwise check per cell is free;
* supplies correspond one-to-one with strong monoidal functors of a stated shape, so "which types admit fan-in" becomes a functor rather than a predicate;
* a supply **extends to the strictification**, which matters because gandr's carrier is strict.

Hypergraph categories are the maximal case — every object supplied with a special commutative Frobenius monoid — and they have a coherence theorem and a cospan-algebra characterisation [@fong-spivak-2019-hypergraph-categories].
The decorated-cospan and decorated-corelation constructions are the standard way to _build_ such a category from a functor, with corelations giving the black-boxing direction that discards interior structure [@fong-2015-decorated-cospans] [@fong-2017-decorated-corelations] [@fong-2016-thesis], and the universal-construction account presents these semantic categories as colimits of simpler ones, which is what makes complete axiomatisation tractable [@fong-zanasi-2018-universal-corelations].

**The gandr-specific reading is the opposite of the usual one.** This literature is interested in categories where _every_ object supplies Frobenius; gandr's aggregation split says fan-in is available exactly where the supply exists and is an unmet obligation elsewhere.
So the useful import is the notion of supply and its coherence theorem, used as the _shape of the obligation_, with the ambient category deliberately not supplying it everywhere.

### The spider theorem is the normal form many-out cells want

Where a commutative Frobenius structure is present, any connected diagram built from its generators with $n$ inputs and $m$ outputs equals the single $n → m$ generator — the generalised spider theorem, whose companion result identifies complementarity with the Hopf law [@coecke-duncan-2011-interacting-observables].
A production implementation takes this literally: a spider is **boxless**, introducing no generator at all, and many-in/many-out is the wiring map being allowed to repeat a label [@discopy].

This is the second independent argument that arity is the wrong axis, and it is the normal-form statement a fan-in cell would be checked against.
Completeness of finite matrices for (dagger-)hypergraph categories is the semantic counterpart [@kissinger-2015-finite-matrices-hypergraph].

### Semi-strictification is a live candidate for the standing strictness warrant

The metatheory track's deepest open question is the strictness warrant at the circuit rung: the rectification licence lapsed with the rung change and nothing has replaced it, leaving either a no-set-level-shadow argument or a coherence-by-decision-procedure story.

A semi-strict algebraic model of $(∞, n)$-categories has since been proved equivalent to a weak non-algebraic one, with **algebraic units and composition of round pasting diagrams satisfying a strict form of associativity and interchange**, constructed entirely combinatorially from regular directed complexes [@chanavat-hadzihasanovic-2025-semistrictification].
Its companion extends the pasting theorem to directed complexes with **frame-acyclic** molecules and compares them with regular polygraphs, showing they coincide up to dimension three [@chanavat-2026-pasting-theorem].

Both are directly on gandr's axis — combinatorial, directed, acyclicity-conditioned, dimension-three-relevant — and the same author's earlier polygraph shape category is already carried by the corpus [@hadzihasanovic-2020-shape].
Whether either discharges or reshapes the strictness warrant is a question for the metatheory track, not this one; this lane records the pointer and the reason it is a candidate.

### The reversible line is mostly a neighbouring concern with two exceptions

The reversible-computing literature overlaps gandr on invertibility, not on multi-out.
Its centre of gravity — join inverse categories and their rig extension as the categorical model of reversible functional programming [@kaarsgaard-axelsen-gluck-2017-join-inverse-recursion] [@kaarsgaard-rennela-2021-join-inverse-rig], the pattern-matching structure that must be added to model reversible case analysis [@chardonnet-lemonnier-valiron-2021-reversible-pattern-matching], the rig-groupoid presentation of type isomorphisms [@choudhury-karwowski-sabry-2022-symmetries-reversible], and the survey of the whole arc [@carette-heunen-kaarsgaard-sabry-2024-compositional-reversible] — routes to the identity and univalence layers rather than here, and is collected under [[#Findings that route to other tracks]] below.

Two items bear on this lane specifically.

**Trace positions.** Reversible term rewriting extends rewriting conservatively so that each forward step can be undone, and then works to **remove positions from traces**, with the stated reason that positions are dynamic — they depend on the term being reduced — so carrying them requires complex and inefficient instrumentation [@nishida-palacios-vidal-2017-reversible-term-rewriting]. gandr's normalization deliberately records _which cell fired where_ and not the matched substitution, precisely so that replay must re-match.
That is the same cost, paid on purpose for a different reason, and the interesting fact is that the paper's escape route is a **restriction class** — pure-constructor systems, with basic left-hand sides and constructor right-hand sides — that is very close in shape to gandr's own cell-visible pattern discipline, where a rule's left side is a cut between a constructor pattern and an operation frame.
Whether gandr's fragment already sits in a class where positions are removable is a cheap and worthwhile check, filed as [[#multi-out-spike-04|multi-out-spike-04]].

**Compact closure by negative and fractional types.** A first-order reversible language of type isomorphisms extends to a compact closed category by adding a dual to sums and a dual to products, with an operational semantics in which the negative type reverses execution flow and the fractional type garbage-collects or throws [@chen-sabry-2021-negative-fractional]. gandr's carrier is at the **nonunital (downward) rung with no cup**, so this is precisely the structure gandr declines — and it is valuable exactly as a worked account of what admitting a cup buys and costs, which is the standing instruction's other half.

## The design questions

Each is anchored and cited by link, never by position.
Every one carries a disposition.

1. **multi-out-question-01** — **does the alphabet grow in place, or does a second alphabet stand beside the first?** Growing `ConsPat` fires the compile-visible tripwire at every match site, which is what the pattern grammar's narrowness was designed for; a second `CellAlphabet` inhabitant leaves the landed one untouched at the price of two to maintain.
   **Carried, with growing in place preferred**, keeping an arity-one smart constructor so existing call sites move minimally, and pinning the single-output fragment against a frozen snapshot on the machine-port precedent.
2. **multi-out-question-02** — **are a cell's ports ordered, named, or both?** The corpus rules that within-cell ordering costs nothing to adopt because no symmetry is present to give up, while ordering the **parallel-component** direction would be a silent catastrophe.
   **Carried, with ordered-plus-named preferred** — which is already what `BridgeArity` does, carrying named `SortRef` ports indexed positionally by its three maps.
   Name-keyed attachment is a question about components, and it belongs to [[../surface-language/circuit-cells]].
3. **multi-out-question-03** — **what is a destination, operationally?** A store cell written before a single control transfer, or a covariable bound to a consumer that the machine enters?
   The second sequentializes and therefore re-raises interchange; the first is the destination-passing reading the metatheory ratifies.
   **Carried, and load-bearing** — nothing above [[#multi-out-rung-04|multi-out-rung-04]] can be built without it.
4. **multi-out-question-04** — **the Σ-former at the multi-output face**, the metatheory track's own open question, restated here because this lane consumes it: the Σ-η direction is where fan-out bites, and premise-form statement is what keeps associative–commutative completion out of the rule layer.
   **Parked on the metatheory track, and a hard gate on the Σ-layer half.** The Π-layer half does not wait for it.
5. **multi-out-question-05** — **is the fan-in obligation carried as a supply?** The supply notion makes the obligation a structure with automatic coherence rather than a per-cell proviso, and it survives strictification.
   **Carried**, with the caveat that gandr must not supply it everywhere; the import is the shape of the obligation, not the ambient assumption.
6. **multi-out-question-06** — **does the hypergraph DPO instance become the applicable one at the many-out rung**, retiring the scoping that says graph-shaped double-pushout instances do not apply?
   **Carried, and owed a decision before the alphabet changes** — the scoping is stated in the metatheory track and would become wrong silently.
7. **multi-out-question-07** — **how does the multi-out arity index the description universe?** Generalizing the recursive-occurrence code to a multiset of output sorts is a container, so the multi-out term face forces the **indexed** description universe.
   **Carried**, and shared with the higher-cells lane, which wants `sort` members for the same reason.
8. **multi-out-question-08** — **what does the enumerator cost once interfaces are many-out?** Non-linear interfaces fan out families and the measured multi-sum degeneracy ends, which the corpus already names as the trigger for revisiting full multi-globularity.
   **Carried as a scheduled consequence**, to be measured rather than predicted.
9. **multi-out-question-09** — **is there a term syntax for multi-out that is not a tuple?** The obvious surface binder allocates a product, imposes an order, and is therefore not multi-out at all.
   **Declined for now, with its reversal condition**: a genuine term-level spelling is the circuit body of [[../surface-language/circuit-cells]], and committing a tuple-shaped binder before that lands would freeze the wrong thing.
10. **multi-out-question-10** — **do multi-out operations exist in `codata` position?** `codata` blocks already parse `rule` members, and the higher-cells lane carries the same question for its respelled ladder.
    **Carried, inherited**; declining there is a legitimate answer but must be a decision, not an omission.
11. **multi-out-question-11** — **can trace positions be dropped on gandr's cell fragment**, as the reversible-rewriting line drops them on pure-constructor systems?
    **Carried**, and cheap to settle — see [[#multi-out-spike-04|multi-out-spike-04]].

## The staged plan

Each rung is an addition over the previous, and each names its gate.
The plan deliberately front-loads the rungs that need neither the machine nor the dependent era, because those are where multi-out becomes a feature the rest of the language can be designed around.

### multi-out-rung-01 — pin the substrate claim

Convert "the sequent layer already carries the type shape" from prose into a machine-checked statement: construct multi-consumer commands directly in `CommandArena` and assert, per construct, which arities the typed-IL checker admits and which it refuses.
The expected finding is that `Dtor` refuses anything but one while `Ctor`, `Prim`, and `Jump` admit any count with no construction site and no machine behaviour behind them.

**Gate:** none.
**Unlocks:** an honest baseline, and the arity invariants become visible rather than incidental.

### multi-out-rung-02 — the wire

Land the `surface-engine` → `theory-computads` dependency and drive `elaborate_data_desc` from the pipeline, single-output only.
This is owed three times over already: the higher-cells lane calls it the load-bearing wire, the surface roadmap makes it the gate on the `rule` member's graduation, and the implementation track lists the engines having no external consumer among its honest limits.

**Gate:** none.
**Unlocks:** everything user-visible in this lane, and the `rule` member's own graduation independently of multi-out.

### multi-out-rung-03 — the alphabet grows to many-out

Grow `ConsPat::Op` and `ConsPat::Frame` from a single `ret` to an ordered port list with names, and follow the tripwire through `subst`, `overlap`, `elaborate`, `sequent`, and `bridge`.
The engines are generic over `CellAlphabet` and do not move.
Settle [[#The design questions|multi-out-question-01]], [[#The design questions|multi-out-question-02]], and [[#The design questions|multi-out-question-06]] before the first edit, and measure [[#The design questions|multi-out-question-08]] after.

**Gate:** [[#multi-out-rung-01|multi-out-rung-01]].
**Unlocks:** **many-out cells become representable, matchable, and rewritable** — the lane's largest single step, and the one that makes reconvergence and the declined-horizontal-composition guard writable at all.

### multi-out-rung-04 — the IL invariant becomes tag-declared

Give `DtorTag` a `consumer_arity()` beside its `producer_arity()`, replace the hard-coded one-consumer check in `core-sequent/src/check.rs` with the tag's declared count, and add the first real construction site for a many-consumer command.
The focusing translation is unchanged: it never emits multi-out, and the point of the rung is that the _checker_ stops asserting an arity the grammar does not impose.

**Gate:** [[#multi-out-rung-01|multi-out-rung-01]].
**Unlocks:** multi-out is checkable in the IL, and the reserved growth point stops being a comment.

### multi-out-rung-05 — destination-passing on the L machine

Give the machine a semantics for a command with several return continuations.
This is the term face proper and the part with no precedent in the read implementations.

**Gate:** [[#multi-out-rung-04|multi-out-rung-04]], and [[#The design questions|multi-out-question-03]] decided; the Σ-layer half additionally gates on [[#The design questions|multi-out-question-04]].
**Unlocks:** multi-out runs.

### multi-out-rung-06 — the description universe's index change

Generalize the recursive-occurrence code to carry its output sort, and add the sort table the shape blocks want.

**Gate:** [[#multi-out-rung-03|multi-out-rung-03]]; shared with the higher-cells lane's sort members.
**Unlocks:** multi-out descriptions that decode, and the indexed universe both lanes need.

### multi-out-rung-07 — surface graduation

Graduate the `op` member from parse-and-decline to accepted at the Π-layer, promoting the existing parse-only corpus witness to a runnable one.
The circuit-cell concrete syntax — starred port forms, disconnection, wheels, and the sigil decision — lands after, in [[../surface-language/circuit-cells]], and is deliberately last.

**Gate:** [[#multi-out-rung-02|multi-out-rung-02]], [[#multi-out-rung-03|multi-out-rung-03]], [[#multi-out-rung-06|multi-out-rung-06]].
**Unlocks:** the surface stops declining what the description layer has modelled since it was written.

## Spikes

### multi-out-spike-01

**Does the hypergraph DPO-with-interfaces instance apply to gandr's many-out cells?** Take one many-out cell shape, write it as a hypergraph with interface, and check the three claims that matter: that the interface is the coproduct of the cell's input and output ports, that gandr's rules have mono left legs (so pushout complements are unique), and that a rewrite respecting the interface is the same relation as a gandr cell application.
**Small.** Settles [[#The design questions|multi-out-question-06]] and decides whether the corpus's DPO scoping is retired or re-scoped rather than contradicted.

### multi-out-spike-02

**Measure the enumerator blowup.** Instantiate the overlap enumerator on a toy many-out alphabet and count the overlap families against the single-output baseline, so the enumerator-cost question is answered by measurement rather than by prediction.
**Small**, and worth running before the alphabet grows rather than after, because the number is an input to whether ordered ports are enough.

### multi-out-spike-03

**Is the fan-in obligation expressible as a supply over gandr's own type formers?** Write the obligation for one concrete target and check that the coherence the supply theorem grants is the coherence gandr would otherwise check per cell.
**Small.** Settles [[#The design questions|multi-out-question-05]].

### multi-out-spike-04

**Is gandr's cell-visible pattern fragment position-removable?** Compare the pure-constructor restriction class of the reversible-rewriting line against gandr's cell pattern discipline — a cut between a constructor pattern and an operation frame — and decide whether recorded positions are necessary or merely convenient.
**½ day.** Settles [[#The design questions|multi-out-question-11]]; a positive answer would shrink tracelets, and a negative one records why the position cost is structural.

### multi-out-spike-05

**Does the spider normal form give the fan-in cells a decision procedure?** Where a supply exists, check whether "any connected diagram of the supplied generators equals the single many-to-many generator" is checkable on gandr's representation, and how it relates to the corpus's linear-time acyclicity test.
**Small.** Feeds the normal-form half of the certificate story rather than the rewriting half.

## Findings that route to other tracks

Recorded here so nothing the sweep found vanishes, with the receiving track named.
None of these is scoped by this lane.

| finding                                                                                                                                                                                               | routes to                                                                                     |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------- |
| Semi-strictification with algebraic units and strict interchange, built combinatorially from regular directed complexes [@chanavat-hadzihasanovic-2025-semistrictification]                           | the metatheory track's strictness-warrant question — a candidate re-warrant, not a discharge  |
| The pasting theorem for frame-acyclic directed complexes, and the coincidence with regular polygraphs up to dimension three [@chanavat-2026-pasting-theorem]                                          | the metatheory track's polygraph and acyclicity obligations                                   |
| Reversible programs as paths in a univalent universe, with combinator optimizations as 2-paths [@carette-chen-choudhury-sabry-2017-reversible-univalent]                                              | the identity layer — a computational reading of paths and univalence at a computable universe |
| Π presented by the free symmetric rig groupoid, with 1-combinators as 1-paths and 2-combinators as 2-paths, sound and complete at both levels [@choudhury-karwowski-sabry-2022-symmetries-reversible] | the identity layer, and the certificate layer's 2-cell discipline                             |
| Join inverse categories giving reversible recursion, a †-trace, and algebraic ω-compactness [@kaarsgaard-axelsen-gluck-2017-join-inverse-recursion]                                                   | the recursion surface's productivity ladder — a trace gandr's feedback rung declines to use   |
| Join inverse rig categories as the common model of several reversible languages [@kaarsgaard-rennela-2021-join-inverse-rig]                                                                           | the same, as the ambient-model question                                                       |
| Pattern-matching needing structure beyond join inverse rig categories to be modelled [@chardonnet-lemonnier-valiron-2021-reversible-pattern-matching]                                                 | the codata and case-tree lane, where gandr's own eliminators live                             |
| Negative and fractional types giving compact closure operationally [@chen-sabry-2021-negative-fractional]                                                                                             | the metatheory track's no-cup ruling — the worked account of what adding a cup would cost     |
| The historical and categorical arc of compositional reversible computation [@carette-heunen-kaarsgaard-sabry-2024-compositional-reversible]                                                           | orientation only; no gandr obligation                                                         |

## The corpus witness plan

Features land with their corpus examples, and this lane's witnesses are the executable half of its guards.

| witness                                                         | what it pins                                                                          |
| --------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| a many-out `op` member that elaborates and runs                 | the Π-layer, promoted from the existing parse-only `NatDiv` witness                   |
| a bridge arity whose maps do not compose                        | the `WfKind::ArityDoesNotCompose` decline, at the declaration table                   |
| two disjoint redexes in one body                                | the declined-horizontal-composition guard, which becomes writable at many-out         |
| a fan-in cell whose target carries no commutative monoid        | the aggregation obligation, named at the declaration rather than implied by a picture |
| a multi-consumer command the typed-IL checker admits or refuses | the tag-declared arity, replacing the hard-coded one                                  |

The first three of these are owed to the binding-guards inventory independently; this lane is where two of them stop being unwritable.

## Source and confidence

* **Every as-built row was verified against this tree at the time of writing**, with the crate and symbol named at the claim rather than carried across from a design record.
  The load-bearing negative claims — that no construction site emits more than one consumer, that the machine reads only the first, and that `desc.ops` is never read — were checked at their sites.
* **The literature findings come from a triage sweep, not from close readings**, and are marked accordingly: abstracts and section maps for the whole set, with targeted section-level reads for the hypergraph-rewriting, supply, and reversible-term-rewriting results specifically.
  Anything above that a rung depends on is filed as a spike rather than treated as established, and no theorem is quoted that was not read at its statement.
* **One neighbouring result was already absorbed and carries a standing ruling**: the planar string-diagram normalization work is cited by the metatheory track with the ruling that its quotient must **not** be substituted for the symmetric one, the relations being incomparable in both directions [@delpeuch-vicary-2022-normalization].
  Its disconnected-diagram extension is adjacent to this lane's disconnection axis and inherits that ruling unchanged.
* **The reversible-computing group is largely a neighbouring concern**, and saying so is a disposition rather than a dismissal: its results are routed above with their receiving tracks named.
* The declined half of the sweep, with a reason per item, is a contributor-side triage artifact and is not carried here.
