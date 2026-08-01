# Circuit cells — the surface question for reconvergence, disconnection, and wheels

The design pass on a standing question: the substrate carries the full generality of circuit algebras, and the surface does not.
This document asks whether it should, sketches a syntax that would, and prices each feature.

* Status: **design sketch.** **Nothing here is committed, and no concrete syntax is chosen.** Every spelling below is a starting point recorded so the decision is cheap when it is taken; every example is **unworked** — none has a corpus witness, an elaboration, or a checked semantics.
* This is the surface half of a standing metatheory obligation: deleting the cell record's simple-connectivity field, or re-carrying it as a consumer-side predicate, "with the surface-language question — whether the _surface_ still hides wheels and disconnection — as its own design pass" ([[../metatheory/roadmap]]).
* The carrier-side facts it rests on are landed and machine-checked ([[../metatheory/carrier]]); the ruling it must honour is the generality ruling of the [[../metatheory#The substrate is the full circuit-algebra rung|metatheory track]].
  **Nothing here proposes a carrier change.**
* The cell members it extends are [[higher-cells]]; the grammar every form must clear is [[grammar]].

## The question, and why it is not about arity

> **The generality ruling (owner, binding).** The substrate carries the **full generality of circuit algebras** — many-in/many-out arity, wheels, disconnection, and the cut — and gandr's restrictions are enforced by **static analysis, not structure**, at the tightest boundary to what gandr currently handles.
> A carrier notion that cannot express what the shape layer provides is _wrong_, not merely scoped.
> Assume today's restrictions will be removed over time.

The surface is where "static analysis, not structure" is either honoured or quietly broken.
A restriction that exists only because the surface cannot **write** the thing is a structural restriction wearing an analysis costume.

**Arity is the wrong axis, and the corpus has already retired it.** The guards ledger tombstones "restrict to dioperads, therefore give up many-out" with the reason: dioperads have the same colour set as properads, and **what the higher rungs add is reconvergence, disconnection, and wheels — not arity**.
So a document organized around "multi-in, multi-out" would be answering a question the corpus has closed.
The three real axes:

| axis              | what it means on a diagram                       | carrier status                                                                                                     | why the surface currently cannot write it                                              |
| ----------------- | ------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------- |
| **reconvergence** | two paths out of one cell rejoin at another      | expressible; excluded from a `Cell` only by the simple-connectivity predicate                                      | a term has one root; a rejoin has no term spelling                                     |
| **disconnection** | two sub-diagrams with no wire between them       | expressible, **and proved**: a merge of two connected shapes has exactly two components, and they are the operands | the block grammar has one spine; there is nowhere to put a second, unrelated component |
| **wheels**        | an output feeds back, through cells, to an input | expressible; `Cell` derives wheel-freeness from simple connectivity                                                | no surface form closes a cycle                                                         |

Fan-in and fan-out are a **different** question — aggregation, not connectivity — and are treated separately under [[#Aggregation is not wiring]].

## Why the surface must be able to write the refusable thing

The strongest argument for surfacing these is not expressiveness.
It is that the carrier **deliberately** made them refutable, and a surface that cannot express them wastes that choice.

* The carrier's `node` constructor publishes a corolla's ports into the shared interface **rather than consuming from a pool** — and the recorded reason is exact: "a pool discipline would make wheel-freeness **structural** and cost the predicates their **refuters**."
* The governing principle is stated once and binds everywhere: **an invariant can be structural or refutable, never both in one type.**
* Grafting and merging are total and do **not** preserve the predicates.
  Connectivity is a predicate on objects, so any two shapes compose and cell-ness is checked of the result — with the counterexamples exhibited and refuted in the tree: `bigon`, a graft that reconverges and loses acyclicity; `two-points`, a merge that disconnects and loses connectivity; and `wheel`, a vertex whose out-leg is glued to its own in-leg.
  The tree's named self-gluing, `gluing`, is connected, acyclic, and a cell — a self-gluing is not per se a wheel.

So the carrier already pays for refutability, and the refuters already exist as Agda terms.
A surface that cannot write a wheel makes the wheel guard **unfalsifiable** — the checker would be refusing something no program can express, which is not a guard but a tautology.

This is the same discipline the identity layer already runs: the K-derivation witness is a program that **must fail elaboration**, and it is load-bearing precisely because it is writable.

The corpus's witness inventory ([[../implementation/roadmap]]) records seven binding guards with **no pathological witness yet**, each owed one on the K-derivation precedent.
Two of them are guards this document's axes would make writable:

* **declined horizontal composition** — writable exactly when a body can hold two disjoint redexes, which is [[#Reconvergence]];
* **the symmetry-derivation refusal** — exercised where the parallel direction is written down, which is [[#Disconnection]].

The wheel is **not** in that inventory, and the inventory's "acyclicity gate's decline" is a different object — the certificate algebra's composition gate declining on a variable-flow cycle, not a shape wheel.
So the honest claim is narrower than "these axes make the owed witnesses writable": two of the seven become writable, and **a wheel guard would be a new guard**, owing a witness once it exists rather than discharging one already owed.
That, and not expressive power, is the argument.

## The sketch

Nothing in this section is decided.
It is one coherent set of choices, recorded with its alternatives.

### Port-named cells

Cells are declared with the [[higher-cells#The keyword ladder|higher-cells keywords]] `oper`/`cons`; every port carries its direction as a prefix on its name — `-` for inputs, `+` for outputs — and a leading `*` marks the cell as being in explicit-port (circuit) form:

```text
oper *add(-x: Nat, -y: Nat, +z: Nat) -> *
cons *add(-x: Nat, -y: Nat, +z: Nat): *
```

* **The plain form is sugar.** The ordinary positional declaration parses to the same thing, with fresh port names minted in order:

```text
oper add(Nat, Nat) -> Nat     // parses to: oper *add(-x: Nat, -y: Nat, +z: Nat) -> *
```

The starred form is what the engine ever sees.
The `cons`/`oper` split keeps its generative meaning; the trailing `-> *` / `: *` marks that the starred ports are the whole interface.

* **The result clause is kept for readability, not necessity.** The `+` ports already _are_ the result, so a circuit cell has no distinguished return that the clause names.
  It is retained because a reader should see where the interface ends without counting polarities — recorded as a readability decision so a later pass does not "discover" the redundancy and delete it.
* **The marker is load-bearing at application sites.** `-` is the language's **only** unary operator, so in argument position `-x` already parses as _negate `x`_.
  The marker is what switches the argument sort from expressions to ports, licensing `-`/`+` to mean polarity rather than arithmetic.
  In a _declaration's_ parameter list there is no expression context, so there the marker is redundant for parsing and earns its keep on legibility, on zero-port cells, and on making a half-prefixed port list a hard error rather than a silent reinterpretation.
* **The sigil spelling is open.** `*` is doubly taken in the landed grammar — binary multiplication and the eager-product former `A * B` — so `-> *` is that former written with no operands.
  The alternatives are surveyed under [[#Open questions, dispositioned]]; the decision is the owner's and is parked rather than settled here.

### Attachment is by name, and that is a soundness property

Wiring is name-sharing: a diagram is a sequence of cell applications with shared port names, polarity-checked at every attachment (`+` feeds `-`, always).

Three reasons, in order of force — the first is the one that matters:

* **Names cannot accidentally order the parallel direction.** The corpus's symmetry ruling is blunt about the stakes: ordering the parallel-component direction "would be a silent catastrophe" — the certificate normal form is a disjoint union of primitives under a **symmetric** monoidal structure, symmetry gives cocommutativity, cocommutativity gives the enveloping-algebra theorem, and that licenses the bracket-vanishing oracle.
  A positional port list imposes an order at every site and would have to be quotiented back out; a name-keyed one never imposes it.
  The cyclic-operad literature makes the same choice for the same reason: its simultaneous-composition term `a{tₓ | x ∈ X}` is explicitly order-irrelevant [@curien-obradovic-2017-cyclic].
* **A wiring error becomes a named-port diagnostic** rather than an arity-index mismatch, and the polarity prefix makes the direction of every edge readable at the attachment site.
* **Names dovetail with dependency**: a port's type may mention an earlier port's value (`+r: Vec(x)`), which is the shape the dependent-core era needs anyway.

The published attachment discipline closest to this is the named approach to opetopes [@curien-hothanh-mimram-2019-opetopes], whose grafting rule attaches at a **named face** and carries exactly two side conditions worth copying:

* the target face must be **unused** — "ensures that `a` hasn't been used for grafting beforehand", i.e. port linearity;
* the boundaries must **agree** — the glued cell's iterated source must match the face's source.

Its authors are explicit that these are not bureaucracy: "ill-formed graftings may occur with n-pasting diagrams, for `n ≥ 3`, and the side condition is necessary to rule them out."
A second datum from the same system is worth carrying: substitution there can **add equations** to the ambient theory.
Their worked case, in their notation — grafting the degenerate cell `β_y : ȳ ⊷ y` into `α : g(y ← f) ⊷ x` at the face `g`:

```text
   g(y ← f)[ȳ / g]  =  ȳ(y ← f)  =  f          -- the composite collapses to f
```

and grafting the other degenerate cell, at `f`, collapses to `g` **and emits the equation `x = y`** into the ambient theory.
So degenerate attachment is an **identification, not a no-op** — a surface that treats a trivial wire as erasable would silently drop a port equality the checker needs.

### Frame and redex

A block's body is a list of circuit applications, and each node is one of two kinds:

* **frame** — an operation (a 1-cell): the context a rewrite happens inside, which is what the boundary language's whiskering construction `f(t̄, ρ, ū)` spells;
* **redex** — a rewrite (a 2-cell): a named rule instantiation, or a port whose sort is a rewrite face.

A body whose nodes are all frame is a **1-cell definition** and takes `oper`/`cons`; a body containing redexes is a **2-cell** and takes `rule`.
A circuit `rule` therefore needs no `lhs ~> rhs`: the **wiring determines both boundaries** — the source boundary is the diagram with every redex replaced by its source, the target boundary the same diagram with every redex replaced by its target.
The rewrite _is_ the diagram.

**Where that has to land, and does not yet.** The corpus's boundaries are not free-standing terms: they are **globular telescopes**, sphere-indexed, so parallelism is judgmental and a mis-glued boundary fails at the declaration table rather than downstream ([[higher-cells#Sphere-typed boundaries]]).
A derived-boundary rule owes an account of how the wiring's computed source and target become that sphere — plausibly by computing the pair and checking it against the sphere the surrounding block already fixes, but this document does **not** establish it.

**And the codata position is not addressed.** `codata` blocks already parse `rule` members, and [[higher-cells#Open questions, dispositioned]] carries as its ninth question that declining there is a legitimate answer but must be a **decision, not an omission**.
This document proposes starred `oper`/`cons`/`rule` forms and says nothing about their codata behaviour, so it inherits that question rather than answering it.

### Sorting a rewrite port

A port that ranges over rewrites is sorted by the **2-cell face at the level's own arrow**, not by an operation type and not by the `Model` field's type:

| candidate sort for a rewrite port      | what it would make the block                                                | verdict                                                                       |
| -------------------------------------- | --------------------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| `Nat -> Nat` — a 1-cell                | every node becomes frame, no redex exists, and the block is a plain circuit | **wrong**: it de-dimensions the block and deletes the rewriting content       |
| `Path(Nat, x, x′)` — the `Model` field | the _interpretation_ written into the syntax, and `Path` is groupoidal      | **wrong level, and unsound**: it silently makes a directed rewrite invertible |
| `Nat ~> Nat` — the 2-cell face         | two rewrites, whiskerable into one frame                                    | **adopted for the sketch**                                                    |

* The sort is the **boundary sort, not the boundary terms**.
  `*f(-x, +x′)` unifies `f`'s source with `x` and binds `x′` to its target, so the inhabitant is the triple `(a, b, ρ : a ~> b)` — the same interface-pair shape as a hole.
  A pinned form `-f: x ~> x′` stays available when the interface should name its endpoints.
* The spelling is **stable across the directed family**: it interprets as `Path` under today's invertible overlay and as the directed former when that lands, with no surface change.
* **Hazard recorded**: this puts `~>` in _type_ position, where the corpus already has `~~>` for the directed former at the type level.
  They are distinguishable — `~>` relates terms of one sort, `~~>` relates types — but confusable.
  `Step(Nat, x, x′)` is the alternative, and it is not free: `Step` is already taken by the abstract machine's successor-state outcome ([[higher-cells#As-built impact]]).

## Reconvergence

The dioperad fragment's actual boundary, and the one a term syntax hides most completely: a term has one root, so two paths that rejoin have no spelling.

```text
rule *cong2(-f: Nat ~> Nat, -g: Nat ~> Nat, -x: Nat, -y: Nat, +z: Nat) {
  *f(-x, +x′);             // redex: rewrite the first argument
  *g(-y, +y′);             // redex: the second — no port shared with the line above
  *add(-x′, -y′, +z);      // frame: an operation, not a redex
}
```

```text
   x ──[f]── x′ ──┐
                  ├──[ add ]── z        (one diagram, two disjoint redexes)
   y ──[g]── y′ ──┘
```

This is the shape of Agda's `cong₂` — two rewrites applied to two arguments of one operation — and it is exactly what the boundary language **declines** today.
The reason is worth writing out rather than naming, because the whole section turns on it.

One diagram, but **two ways to read it as a sequence** — fire the first argument's redex first, or the second's:

```text
add(f, y) then add(x′, g)        -- rewrite x first, then y
add(x, g) then add(f, y′)        -- rewrite y first, then x
```

Both start at `add(x, y)` and both end at `add(x′, y′)`, so they are parallel composites with the same boundary — but they are **not the same composite**, and nothing makes them equal by construction.
They agree only **up to interchange**, and adjudicating that silently is exactly the coherence smuggling the boundary language refuses.
That is why two simultaneous rewrite arguments are declined: the diagram is unambiguous, but its sequentializations are not, and a rule that fires "both at once" would be picking one without saying so.

**The circuit form is what turns that decline's reversal condition into a construction.** The guards ledger fences it precisely: acceptance is licensed "exactly on **disjoint positions**, where the two readings are shift-equal", and "do not accept it any earlier or any wider" ([[../metatheory/guards#Horizontal-composition surface sugar]]).
In a port-named body, **two redexes are disjoint iff they share no port name** — and in the block above, `f`'s ports are `x`/`x′` while `g`'s are `y`/`y′`, sharing nothing.
So disjointness becomes a check the parser performs on names rather than a property the reader asserts, and the two readings above become shift-equal rather than merely parallel.
That is the trigger the guard was waiting for, and it is why this document and the boundary language's open question have to move together.

The interchange witness is the certificate algebra's own: shift equivalence of adjacent applications at disjoint positions with trivial overlap, **earned per pair by a trivial-overlap witness, never imposed** ([[../metatheory#Interchange, by layer]]).

The general point is larger than the example: **congruence is the free coherence of parallel composition** — every whiskering in the boundary language is a degenerate two-sided congruence where one side is `here`.

## Disconnection

Two independent components in one diagram, with no wire between them:

```text
oper *both(-src: A, -aux: B, +mid: C, +dst: D) -> * {
  *pipeline1(-src, +mid);   // shares nothing with the next line
  *pipeline2(-aux, +dst);
}
```

This is the case with the **least** design risk, because the carrier operation already exists and is proved.
The merger's incidence theorem holds in both directions — no edge of a merge joins the two operands, and each operand's own adjacencies survive — so a merge of two connected shapes has exactly two components, and they are the operands.
**Disconnection is what the substrate says, not what the engine arranges.**

Two consequences for the surface:

* The surface form is the shell's `&` (parallel jobs) lifted into the cell layer, and it is what the cell record's single-spine pattern must grow to hold.
  It connects to the corpus's existing ruling that a multi-root (forest) interface stays symmetric.
* What disconnection **buys** in the derivation dimension is named already: **concurrency** — parallel independent rewriting.

The hazard to state rather than solve: the symmetry discipline above applies here most sharply, because a parallel body is precisely where an implementation is tempted to impose a traversal order.

## Wheels

The axis that needs the most care, and where the sketch's earlier framing was weakest.

### A wheel must close through a cell, because the carrier has no cup

This is not a surface-imposed restriction; it is what the carrier can represent.
The rung is the **nonunital (downward) circuit-algebra rung**: the wiring datum pairs every source with a partner — a sink, or another source (the cut) — and **no constructor pairs two sinks**.
Three consequences stand or fall together, and the middle one is this section's:

> the nodeless loop is **inexpressible** — a closed circle needs a cap composed with a cup, the cup does not exist, so no scalar ever has to be assigned to a free loop.

And the standing instruction: "if a cup is ever added, all three go at once; **do not add one to make an operation total**."

So a surface `wheel` may never desugar to a cup.
Every wheel passes through at least one cell.
This is worth stating as a positive result rather than a restriction: **the guarded-delay discipline and the no-cup carrier constraint agree.** A delay is a cell, so a delay-guarded wheel is representable; an unguarded free loop is not merely refused by analysis, it is not expressible at all.

By contrast, a production implementation at the _unital_ rung must carry the free loop as data: DisCoPy's combinatorial maps carry `loops`, "the types of closed wire components with no ports", which correspond to scalar spiders in its hypergraph form [@discopy]. gandr's rung is strictly below that, and the "problem of loops" it avoids is precisely the scalar those loops would demand.

### Two guard disciplines, and they are complementary

The handoff sketch proposed a `wheel` keyword marking the block, with a delay cell as the checker's obligation:

```text
wheel oper *accumulate(-stream: Stream(Nat), +out2: Stream(Nat)) -> * {
  *zip(-stream, -state, +next, +out2);
  *delay(-next, +state);      // the unit delay: `state` is `next` one step later
}
```

```text
        stream ──────────────────┐
                                 │
                                 v
        state ──> (delay) ──> [ zip ] ──> out2
          ^                    │
          │                    v
          └────── next <───────┘

   the wheel: zip's +next feeds back (through delay) as its own -state
   — an output port attached, one step later, to an input port of the same cell
```

There is a second discipline, and it is **stronger**: make the delay a **type-level** operation, so that a feedback former only typechecks when the fed-back port is delayed.
Written out, the same cell under that discipline — note the `.d` on the fed-back **type**, not on a cell:

```text
oper *zip(-stream: Stream(Nat), -state: Stream(Nat).d, +next: Stream(Nat), +out2: Stream(Nat)) -> *

wheel oper *accumulate(-stream: Stream(Nat), +out2: Stream(Nat)) -> * {
  *zip(-stream, -state, +next, +out2);
  feedback next as state;      // legal: `state`'s type is delayed, `next`'s is not
}
```

and the ill-formed one, which is now a **type** error rather than an analysis refusal:

```text
oper *zip'(-stream: Stream(Nat), -state: Stream(Nat), +next: Stream(Nat), +out2: Stream(Nat)) -> *

wheel oper *diverge(-stream: Stream(Nat), +out2: Stream(Nat)) -> * {
  *zip'(-stream, -state, +next, +out2);
  feedback next as state;      // declined: -state: Stream(Nat), expected Stream(Nat).d
}
```

This is the **feedback-category** structure [@katis-sabadini-walters-2002-feedback]: a symmetric monoidal category with a monoidal endofunctor `delay` and a `feedback` operator taking `f : x ⊗ delay(m) → y ⊗ m` to `f : x → y`.
It is **not** a new proposal here — feedback categories are the project's already-adopted entry tier for wheels, and the stateful-stream reading is recorded beside it [@dilavore-defelice-roman-2022-monoidal-streams].
What this pass adds is corroboration against a production implementation rather than a decision: DisCoPy implements exactly this structure, carrying the delay **on the object** as a time step and defining `wait` as the feedback of a swap [@discopy].

The fact that makes the adoption legible, quoted from that implementation:

> **Every traced symmetric category is a feedback category with a trivial delay.** Trace is the degenerate case where `delay` is the identity.

So the two structures are not alternatives to weigh — one is the degeneration of the other, and the choice of feedback over trace is the choice **not** to degenerate.
That is what makes the guard a structural matter rather than a stylistic one: a cartesian trace **is** a fixpoint [@hasegawa-1997-recursion-cyclic-sharing], the recorded cartesian-trace hazard, so admitting the traced structure at the surface would admit unguarded recursion through the wiring.

**What the corpus actually fences today, stated precisely, because it is not this.** Wheels are fenced by one mechanism only: the `Cell` record's simple-connectivity predicate, from which wheel-freeness is _derived_ ([[../metatheory/carrier]], `simply-conn⇒wheel-free`), under the "static analysis, not structure" ruling.
There is no guarded-form fence in the corpus, and "guardedness" there is exclusively the corecursion productivity discipline ([[recursion]]) — a different obligation about structural descent and copattern observation.
**The guarded-delay discipline below is this document's proposal**, offered as what should replace the connectivity predicate when it is deleted; it is not a standing decision, and nothing above should be read as reporting one.

The two disciplines compose rather than compete:

| discipline            | what it gives                                                                                                          | what it does not give                                          |
| --------------------- | ---------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| `wheel` on the block  | the cycle is declared **once, at the top**, and is visible without tracing every port name; the refuter stays writable | nothing about whether the guard is present                     |
| delay on the **type** | the guard becomes a **typing obligation** — an unguarded feedback does not typecheck                                   | nothing about visibility; a reader still has to find the cycle |

**Recommendation for the sketch**: carry both.
The keyword answers "is there a cycle here", which is a legibility and refutability question; the typed delay answers "is it well-founded", which is a soundness question.
Neither answers the other's.

Alternatives recorded: a glyph on the feeding attachment itself (more local, but splits the marking across sites); the reserved `↻u` marker from the deferred inventory, which this design may consume; a separate `wheel` declaration form beside `rule` (heavier — a second mold family).

### What the wheel is, and where it materializes

Not in any cell, but **in the wiring** — the cycle exists between applications.
This is why the keyword marks the block: the wheel is a property of the whole wiring, not of any node.

Two implementation precedents for the checking, both read:

* DisCoPy detects cycles with Kahn's algorithm over the box-dependency graph, and separates `is_acyclic` from `is_causal` (left-monogamous **and** acyclic **and** topologically ordered) and from `is_boundary_connected` — with the documentation careful that these are independent: "there are acyclic diagrams that are not boundary-connected … and there are boundary-connected diagrams that are cyclic, e.g. the trace of an endomorphism" [@discopy].
* Its `make_causal` is the **cycle-breaking constructor**: for each back-edge it splits a wire onto the boundary and traces it away.
  That is "cut the wheel, expose it as feedback" as an executable operation, and it is the shape gandr's normalization would take.

What a wheel **buys** in the derivation dimension is named in the corpus already: **cyclic derivation** — the completion loop's fixpoints.

## Holes and contexts

A diagram with a hole is a context.
Whether the optimizer's overlap shapes are themselves hole-contexts is a plausible identification this document does **not** establish — the corpus identifies holes with vertices and the vertex listing with the context, and says nothing about overlaps.
A fusion is the plugging of two of them:

```text
rule *fuse(-x: A, +r: C) {
  *producer(-x, +?mid: B);   // the hole: an open port the context owns
  *consumer(-?mid, +r);
}
```

The adopted hole theory is monoidal context theory [@roman-2023-monoidal-context], read for this pass.
Four things it supplies, and one correction to how the corpus currently states it.

* **A hole is its interface pair.** Contexts are objects of `Cᵒᵖ × C` — a pair `(X / Y)` for a hole admitting a process from `X` to `Y` — and in the one-dimensional case a context is literally a list of such pairs with the arrow-fragments between them, composed by substitution at a numbered hole.
* **Two tensors, with an operational reading that matches gandr's own.** `X ◁ Y` is "Y happens after X, and may depend on it"; `X ⊗ Y` is "X and Y happen independently, in parallel".
  The interchange between them is **lax**, one-way — which is precisely the strength the corpus's interchange stratification assigns to the ambient duoidal category of interfaces ([[../metatheory#Interchange, by layer]]).
* **A hole at the unit severs the diagram.** The `I` unit is a hole with no wires, which splits the diagram into two independent pieces; the `N` unit is no hole at all; and normalization is what "sews these two parts".
  Written in this sketch's notation, the two units look like this — and the first is the `both` block of [[#Disconnection]], read as one context rather than as two components:

```text
rule *sever(-src: A, +dst: D) {
  *pipeline1(-src, +mid);
  ?gap();                    // the I-hole: an interface pair with no ports
  *pipeline2(-aux, +dst);    // shares nothing above the gap
}

rule *whole(-src: A, +dst: D) {
  *pipeline1(-src, +mid);
  *pipeline2(-mid, +dst);    // no hole at all — the N unit
}
```

**This is the disconnection axis and the hole axis meeting**: disconnection is a hole with an empty interface.
That identification is the most useful thing this reading produced, and it is now carried by the corpus's Holes section ([[../metatheory#Holes]]).

* **Polarity plus acyclicity is a published, linear-time discipline.** The same work's polar lists are lists of types with a polarity function, and a **polar shuffle** is a bijection between the positive and negative halves whose induced directed graph is **acyclic** — with validity checkable in time linear in the interfaces, at most one shuffle between distinctly-typed lists, and composition preserving acyclicity.

  gandr's `-`/`+` ports **are** a polar list, and its attachment check **is** a polar shuffle.
  Side by side, the `fuse` block above and its polar reading:

```text
*producer(-x: A, +mid: B)     polar list:  [ A°, B• ]
*consumer(-mid: B, +r: C)     polar list:  [ B°, C• ]

the shuffle pairs producer's B• with consumer's B°, leaving [ A°, C• ]
— acyclic, so the attachment is legal and the composite is again a polar list
```

A wheel is exactly a pairing whose induced graph is **not** acyclic, which is why it needs a form of its own rather than falling out of ordinary attachment.
The corpus's own "the normal-form check is a linear-time acyclicity test" and this bound are the same shape.

**The precise statements, now carried by the corpus's Holes section.** The cofree produoidal category is over **a** (monoidal) category, not the free one, and the "derivations form the free …" statement is a **different** theorem about message theories.
The precise statements are that spliced monoidal arrows are the cofree produoidal category over a monoidal category, and that monoidal lenses are the **free normalization** of it.
The "contexts as lists of interface pairs" half is right for the **sequential** fragment and only there — parallel holes need the other tensor, and are not a list.
An earlier revision of the corpus's Holes section conflated the first two and overgeneralized the third; the correction landed in the phase-4 pass.

One honest note the source itself supplies: its author records that these produoidal types "are tedious — see, for instance, the long type of Stage — and it is not clear how to compose them."
A gandr surface should not copy the notation; it should copy the structure.

## Aggregation is not wiring

Fan-in and fan-out are a separate axis from connectivity, and the corpus already splits it:

* **Routing is free.** Non-combining multi-output is just the target map — pure routing, no structure required.
* **Combining is not.** Aggregating several contributions into one destination requires a **commutative monoid on the target**; unrestricted fan-in is not free wiring.

So a fan-in cell is lawful only when its target carries that structure, and the surface should make the obligation visible rather than letting a diagram imply it:

```text
// free: pure routing. one source, two destinations, nothing combined.
oper *split(-p: Pipe, +l: Pipe, +r: Pipe) -> *

// not free: two sources arriving at one destination.
oper *merge(-p: Pipe, -q: Pipe, +r: Pipe) -> *   // lawful only because stream append
                                                 // is a commutative monoid
```

The two look symmetric on the page and are not symmetric in what they cost.
`*split` needs nothing: the target map is the whole content.
`*merge` needs a commutative monoid on `Pipe`, and if `Pipe` has none the cell is **not** a wiring question that a diagram can settle — it is an unmet obligation the surface should name at the declaration rather than let the picture imply.

The related precedent, read: in DisCoPy a spider is **boxless** — `spiders(n_legs_in, n_legs_out, typ)` introduces no generator at all, it simply repeats one wire label on both sides, so many-in/many-out is the wiring map being allowed to repeat a label rather than a primitive to add [@discopy].
Its Frobenius and commutative-monoid laws are then not rewrite rules but consequences of the representation: equality is decided by translating to hypergraph form and comparing.
That is a second, independent reason arity is the wrong axis — at the wiring level, many-in/many-out is not a feature to add.

## What adapting gandr would cost

Per feature, so the sketch is priced rather than merely proposed.

| feature                      | cost                                                                                                                                                                                                                                                                                                                                                                                                        |
| ---------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **cell grammar**             | grows from single-continuation — one variant, a linear spine, one return continuation, verified as built — to named-port interfaces. A cell-pattern change with the compile-visible tripwire the pattern grammar's narrowness was designed for.                                                                                                                                                             |
| **overlap enumerator**       | multiplies. Non-linear interfaces fan out families, and the measured multi-sum degeneracy ends — which is the corpus's own named trigger for revisiting full multi-globularity, so this is a scheduled consequence rather than a surprise.                                                                                                                                                                  |
| **surface grammar**          | grows the marked explicit-port form on the existing `oper`/`cons`/`rule` members. Note `cell` is **not** available: [[higher-cells#The dimension policy]] reserves it for dimension ≥ 4. The rule-condition seam stays open — rule conditions are a binding forward constraint on that form ([[roadmap]]).                                                                                                  |
| **description universe**     | the cost the corpus already prices and this sketch inherits: multi-output arities are **an index change on the description universe** — generalizing the recursive-occurrence code to a multiset of output sorts is exactly a container, so the multi-output term face forces the **indexed** description universe, and the signature universe wants a container basis precisely because sorts are arities. |
| **`Cell` record**            | the simple-connectivity field is deleted or re-carried as a consumer-side predicate — the standing obligation this document is the surface half of.                                                                                                                                                                                                                                                         |
| **static-analysis firewall** | stays, and gets sharper: the delay guard for wheels, the symmetry discipline for parallel composition, the commutative-monoid obligation for combining fan-in.                                                                                                                                                                                                                                              |

## Precedents, read

Each was read for this pass; each entry says what was verified and what was **not**.

| source                                                              | what it supplies                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | what it does not                                                                                                                                                                                                                                                                                                     |
| ------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **DisCoPy** [@discopy]                                              | a production data model at this shape: the `(dom_wires, box_wires, cod_wires)` triple with spiders as shared labels; four port kinds with a polarity axis; a predicate hierarchy where three of the four **name the doctrine the diagram lives in** — `is_bijective` compact-closed, `is_monogamous` traced, `is_causal` symmetric monoidal with a supply of commutative comonoids; `is_acyclic` names none; four toggleable structural knobs; `make_causal` as the cycle-breaking constructor; and the feedback/delay structure | it sits at the **unital** rung — it carries cups, caps and free loops as data, which gandr's carrier cannot represent. Its invariants apply where the rungs agree, not above                                                                                                                                         |
| **Monoidal context theory** [@roman-2023-monoidal-context]          | the hole-as-interface-pair, the two tensors with a lax interchange, the unit-hole-as-severing identification, and polar shuffles with a linear-time acyclicity check                                                                                                                                                                                                                                                                                                                                                             | a usable notation — its author says so. And the corpus's paraphrase of its main theorem needs the correction recorded above                                                                                                                                                                                          |
| **Opetopes, named approach** [@curien-hothanh-mimram-2019-opetopes] | attachment at a **named face** with two side conditions worth copying: the face must be unused, and the boundaries must agree; plus substitution that adds equations when a degenerate cell is grafted                                                                                                                                                                                                                                                                                                                           | its acyclicity condition is **not a theorem**. It is an unnumbered remark about the shape of the rules — "if `x` occurs in the type of `y` then `dim x < dim y`" — with no use made of it anywhere in the paper. Cite it as an observation, never as a result                                                        |
| **Cyclic operads, μ-syntax** [@curien-obradovic-2017-cyclic]        | edges as **involution pairs over named half-edges**; order-irrelevant simultaneous grafting; and — most usefully — the structure-versus-predicate split in a published formalism: a _graph_ may have loops and cycles, and these are excluded from _unrooted trees_ only by a side condition the paper declines to formalize                                                                                                                                                                                                     | it is **not** a polarity precedent. Its entries-only setting abolishes the input/output distinction and has a **single** binder with symmetric composition. The `μ`/`μ̃` provider-consumer pairing belongs to the λμμ̃ calculus it cites for motivation, not to its own syntax. The word "wheel" does not appear in it |
| **The same syntax at thesis length** [@obradovic-2017-thesis]       | the rewriting theory behind the paper, and two things the paper does not carry: normal forms as **decompositions of unrooted trees**, in two families that bracket the design space — corolla-first and edge-first, according to which way the cut equation is oriented (sec. 2.4.2, sec. 2.4.4); and an equivalence between the entries-only presentation and an **exchangeable-output** one, in which an output _is_ distinguished (sec. 3.3.3)                                                                                | it does not weaken the hazard beside it, and read carefully it strengthens it — see below                                                                                                                                                                                                                            |

**The polarity hazard is bounded rather than dissolved, and the bound is narrow enough to state at the claim.** The thesis carries an exchangeable-output presentation and proves it equivalent to the entries-only one, so entries-only machinery is in principle transportable to a setting with a distinguished output.
Three hypotheses stop that from touching the hazard as recorded.

* The equivalence holds for **constant-free** cyclic operads only — the underlying species must be empty on the empty set and on every singleton — which gandr's signatures are not, since nullary constructors are ordinary and the arity-zero corolla is a legitimate cell shape.
* The distinguished output is **exchangeable with any input**, by an involutive action satisfying an exchange law; that is a _marking_, and gandr's polarity is an _orientation_ — an involution on colours with a fixed pole, under which a wire runs at most one way (`forth-not-back`, `cut-oriented`). gandr's own carrier already carries the theorem that separates them: over one self-dual colour there is **no** orientation at all (`mono-unoriented`), and the entries-only setting is that case.
* The observable consequence of having no polarity is in the syntax itself: the μ-syntax's rewriting system is **not confluent**, and the failure is exactly at the symmetry of the cut, with all three of its reducts denoting one tree (sec. 2.4.2).
  A per-cut polarity orientation is what gandr has and what that system lacks.

So the disposition is: **the hazard stands, with its reason upgraded from "no distinction exists" to "the distinction that exists is the wrong one, and its absence is observable as non-confluence".** Do not cite the equivalence as a licence to import the entries-only machinery into a polarised setting without checking constant-freeness first.

**One further precedent was read at source for the circuit-terms pass and answers this document's naming question by refusing it** [@chyp].
Chyp is the closest existing thing to the question this sketch asks — a declarative textual language for string diagrams with rewriting behind it — and its answer is that **a wire is never named**.
Generators are declared by arity alone (`gen f : 2 -> 1`); diagrams are built from sequential `;`, parallel `*`, `id`, the empty diagram `id0`, and an arbitrary-permutation swap `sw[…]`; internal wires exist only as the positional seam of a composition; and there is consequently no ancilla scope, no binder, and no unbound-wire error class.
Two data points travel with that choice and both bear on [[#Attachment is by name, and that is a soundness property]].

* **The cost of all-positional wiring is visible and admitted.** Permutation indices are local to each swap, so "splitting or combining swap maps will change some indices in general" — positional wiring is not compositional in its indices, which is the concrete price this sketch's name-keyed attachment is paying to avoid.
* **Its golden rule is the diagram-normal-form half made into a user-visible law**: "only connectivity matters", decided by cospan isomorphism rather than by rewriting.
  A surface that wires by name has to earn the same law rather than inherit it, because two name-different bodies must still denote one diagram.

This is a **counter-model for the notation and a confirmation for the semantics**, and saying which is which is the useful outcome of reading it.

The naming hazard binds this document in particular. gandr's carrier is a circuit algebra in the Bar-Natan–Dancso sense [@dancso-halacheva-robertson-2021-circuit-wheeled]; it is **not** a Feynman category, and **not** the operad of wiring diagrams — hierarchically nested boxes with ports, a different object under the same word — and it sits strictly below the double-operadic undirected wiring diagrams.
A document about "circuit cells" is exactly where the wrong neighbour is imported by accident.

## Open questions, dispositioned

| #   | question                                                                                                                            | disposition                                                                                                                                                                                                                                                                                                                                                                         |
| --- | ----------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | the circuit-form **sigil**: `*` is doubly taken (binary multiply, eager product), so `-> *` is that former with no operands         | **parked with the owner**, who has seen the collision and elected to keep `*` pending their own decision. Surveyed: `#` is free apart from `#{…}`/`#!{…}` and would extend the labeled-bundle reading; `%` risks modulo; `@` was already declined for the `@[…]` conflict; `^` is wanted by the deferred sized-type surface; `~` reads as the rewrite family; `!` risks logical-not |
| 2   | whether the **result clause** `-> *` / `: *` should exist at all, given the `+` ports are the result                                | **carried as a readability decision** (owner, explicit). Recorded so it is not deleted as redundant by a later pass                                                                                                                                                                                                                                                                 |
| 3   | `~>` in **type position** for a rewrite port, against `~~>` for the directed former                                                 | **carried** as a recorded hazard; `Step(…)` is the alternative and is not free                                                                                                                                                                                                                                                                                                      |
| 4   | whether the explicit two-body form `{ … } ~> { … }` stays available beside the derived-boundary form                                | **carried**; the derived form is what makes disjointness a name check, but a checkable explicit spelling may still be wanted                                                                                                                                                                                                                                                        |
| 5   | whether `wheel` marks the block, the attachment, or a separate declaration form                                                     | **carried**, with the block form preferred because the wheel is a property of the whole wiring                                                                                                                                                                                                                                                                                      |
| 6   | whether the delay guard is **static analysis** or a **typing obligation** on a delayed type                                         | **carried**, with the recommendation that it be both — they answer different questions                                                                                                                                                                                                                                                                                              |
| 7   | whether the reserved `↻u` marker is consumed by this design                                                                         | **carried**; the glyph is recorded but the marker is deferred-with-reasons and has no grammar rule, slot, or sort ([[roadmap]])                                                                                                                                                                                                                                                     |
| 8   | multi-hole contexts: whether the surface tracks hole **order** (the sequential fragment is a list) or the full two-tensor structure | **carried**; the source's own notation is acknowledged tedious, so this is a design question and not a transcription                                                                                                                                                                                                                                                                |
| 9   | whether the corpus's Holes section is corrected in place                                                                            | **carried, and owed** — the "cofree produoidal over the free monoidal category" phrasing conflates two theorems and should be fixed where it stands                                                                                                                                                                                                                                 |
| 10  | the higher-order hole direction — a circuit with a hole taking another circuit                                                      | **parked**, conditional on wanting higher-order cells; the second hole theory already has a home in the metatheory track                                                                                                                                                                                                                                                            |
| 11  | the feedback-category sources — read only through DisCoPy's implementation of them, not in the original                             | **carried, with the reading gap marked at the claim.** Both locators are in the project register: the origin [@katis-sabadini-walters-2002-feedback] and the stream reading [@dilavore-defelice-roman-2022-monoidal-streams]. Nothing here rests on either beyond the implementation actually read, and neither is quoted                                                           |
| 12  | how a **derived** rule boundary lands on the corpus's sphere-indexed (globular-telescope) boundaries                                | **carried**, and unresolved. The derived-boundary form is what makes disjointness a name check, so this interaction has to be settled before it lands, not after                                                                                                                                                                                                                    |
| 13  | the respelled starred forms in **codata** position                                                                                  | **carried**, inherited from [[higher-cells#Open questions, dispositioned]]. Declining there is legitimate; omitting it is not                                                                                                                                                                                                                                                       |
| 14  | traversal order in a parallel body — where an implementation is tempted to impose one, against the symmetry ruling                  | **carried as a stated hazard**, not solved. It is the sharpest place the symmetry refusal bites, and the sketch offers name-keyed attachment as the mitigation rather than a proof                                                                                                                                                                                                  |
| 15  | adding the disconnection-is-a-unit-hole identification to the corpus's Holes section                                                | **carried, and owed** — like the produoidal correction of question 9, it is a change to a document this pass did not edit                                                                                                                                                                                                                                                           |

## Source and confidence

* **This document is a sketch, and its status is the load-bearing part of it.** No example has been elaborated, checked, or given a corpus witness; no syntax is committed; the sigil is undecided.
  Treat every code block as a shape, not a spelling.
* The **carrier facts are high confidence**: they are landed, machine-checked, and named in [[../metatheory/carrier]].
  Nothing here proposes changing them, and the no-cup consequence is quoted rather than inferred.
* The **rulings it honours** — the generality ruling, the symmetry refusal, the horizontal-composition decline and its reversal condition, the aggregation split — are quoted from the corpus with their reversal conditions intact.
* The **precedents were read for this pass**, and three claims that an intermediate summary had attributed to them did not survive: the μ-syntax is not a polarity precedent and has no `μ̃`; the opetopic acyclicity condition is not a theorem; and the corpus's own paraphrase of monoidal context theory's main result conflates two statements.
  Those corrections are recorded above rather than quietly dropped.
* The **feedback-versus-trace distinction is not a finding of this pass.** Feedback categories are the project's already-adopted entry tier for wheels, and the cartesian-trace hazard is recorded beside them in the register.
  What this pass contributes is corroboration against a production implementation, plus the observation that the adopted structure supplies a **second, typed** guard discipline beside a block keyword.
  An earlier draft claimed the distinction as new and claimed a locator was owed for it.
  Both were wrong and both are withdrawn: the register carries the locators, and the position was already taken.
* This document was **adversarially reviewed** against its sources before landing, and that pass caught four real defects, all corrected here: a witness-inventory claim naming a guard the inventory does not contain; a "`fix`-versus-wheel firewall" that exists nowhere in the corpus; the novelty and locator errors above; and a guarded-form fence stated in the present indicative when it is this document's own proposal.
  The lesson recorded for the next pass: **check the project's bibliography register before declaring a locator owed**, and before calling a position new.
