# Metatheory roadmap

What remains to realize the accepted directions of [[../metatheory|the metatheory track]], in executable detail.
Ordered within each section by what unblocks the most; costs are estimates against the current tree.

## Spikes — cheap experiments that decide design questions

Each spike is a heading so it can be linked into directly — `[[metatheory/roadmap#meta-spike-04]]` and the like.
The numbering is stable and its gaps are meaningful: a missing number is a spike that has been executed and retired, and its verdict is recorded where the decision it settled lives.

### meta-spike-01

**Make the `theory-computads` enumerator alphabet-polymorphic.** EXECUTED on the Rust side — the engines are generic over `CellAlphabet` with `SequentAlphabet` as the first inhabitant, and an external toy alphabet drives all three engines; the guard-plus-witness half of the warning below is tracked as `gandr-s9q`.
`enumerate_overlaps`, `completion::complete`, and `rewrite::normalize` were gate-tested but monomorphic over the sequent-kernel command-pattern alphabet.
**Days.**

Settles the highest-leverage engineering item on the board: the coherence grinds ahead — the four-layer exchange identity, the shape-layer interchange equation — are exactly the hand work the tool exists to prevent.
Carry the warning verbatim: off-TCB applies to the **enumerator only**; the `cells_equal` normal-form fast path is TCB-adjacent and needs a guard plus a soundness witness, never documentation.

### meta-spike-02

**Is the measured cellular-Conduché condition the exponentiability condition the convolution face waits on?** **≈ 1 day.**

Closes a gate on the convolution face using a measurement already taken; the one open axiom row (cylindrical decomposition) is what convolution needs _beyond_ exponentiability.

### meta-spike-03

**The four offset functions.** Check the monotone-rung placement the directed arena is built on — the computations [[layout-and-coherence]] cites as unchecked — against `Gandr.Arena.Offset`.
**½ day.** Settles the arithmetic substrate of the arena's directed generalization: which of the four one-way generator classes lands in the monotone rung, and where the codiagonal breaks order.

Partially executed in `Gandr.Arena.Directed`: the six realizations; the offset-fixed boundary with proofs and pinned counterexample cells (`inl` fixed verbatim, `inr` fixed modulo the right-injection shift, the diagonal fixing exactly offset 0, the projections fixed exactly at the unit laws, the codiagonal fixing exactly the left leg); and the shift-0 core of `RigidMono`, closed as a category with one-sided whiskering.
Remaining: the named offset transforms (the right injection's shift, the projections' floor-division, the diagonal's `b · i + i`), the four monotonicity proofs, the codiagonal's order-break witness at a size-2 code, and `RigidMono` carrying the shift as data.

### meta-spike-04

**Is gandr's pattern _analytic_?** Check the two conditions — a **strict Segal morphism** to pointed finite sets, and **conservativity on the inert subcategory** — with underlying-legs as the candidate functor.
**½ day.**

Settles whether arity approximation applies — whether circuit algebras at `Set` are determined by arity-≤k data, a truncation result more useful than raw finiteness.

**EXECUTED 2026-08-02, and its disposition is an open owner decision** (`gandr-hpck`): the verdict reposes the spike rather than answering it, so the question above is recorded as asked and is not the question to run.

**Two defects in this entry's statement of the conditions are corrected here rather than carried.** The published form is [@barkan-2022-arity, def 3.1], cited against the held arXiv v4 throughout.
_Conservative **interior**_ was a mis-expansion of the source's superscript, which abbreviates **inert** — a map of pointed finite sets with singleton fibres [ibid., def 1.7] — so the condition is that an inert morphism whose underlying map of pointed finite sets is an isomorphism is itself an isomorphism.
The word _interior_ occurs once in that paper, of a manifold's interior, and never in this sense.
And _strict Segal morphism_ is **undefined at source**: the phrase occurs exactly twice, both times in use, against three defined grades (Segal, iso-Segal, strong Segal); the paper's own example asserts it by pointing at an earlier example that says **iso-Segal**, and the corollary consuming it needs the iso-Segal identification to go through, so that is the reading [ibid., def 2.24].

### meta-spike-05

**Does the arity-approximation machine instantiate at gandr's rung, and at what price?** **2 days.**

**Three gates, and the spike may not be run before all three are open.**
[[#meta-spike-04|meta-spike-04]] decides the analyticity question this one inherits, and reaches the same obstruction from the other direction.
**The presentation of the graphical category** — the standing obligation below — supplies the object every check is run against.
And **the identification of gandr's substrate with an operad of downward wiring diagrams** must be settled before it is used, because two standing cautions aim at exactly that phrase: the _operad of wiring diagrams_ names a different object under the same word, hierarchically nested boxes with ports rather than a matching datum, with gandr's downward wiring sitting strictly below it ([[../metatheory#The naming hazard, kept where a reader meets it]]); and what is special about the substrate is that the wiring category is **free**, not that it is operadic, since every permutative category has an underlying operad and that construction distinguishes nothing ([[../metatheory#The rung, identified]]).
Deciding which caution applies to this use is the first part of the re-basing work tracked as `gandr-9ulr`, and until it is settled nothing downstream may be quoted.

**What is established already, which is why this is a price question rather than a yes-or-no.** The entry's earlier form — is the operadic partition complex built from the graphical category's slice — is answered **definitionally yes**: the complex is built from slices by definition, the graphical category's active slice at a corolla is exactly gandr's shape family with that boundary, and gandr's site is literally a slice of the graphical category over the orientation object.
**The machine nonetheless does not instantiate at the graphical presentation.** It consumes an analytic pattern [@barkan-2022-arity, def 3.1]; iso-Segality forces the arity to be the count of elementary objects, and the source's corollary then reads elementary if and only if arity one, while in the circuit-algebra pattern the elementary objects are the edge and the corollas and a corolla's element category has one object per leg plus one — so every legged corolla is elementary of arity greater than one.
That negative is high-value and rests on a reading, which is why the programme below opens by implementing rather than by reading: an implemented tracking is what would prove it or catch it wrong.

**The spike opens at the language level, not inside the approximation machinery.**

1. **Implement the operadic-presentation arity tracking first.** Carry **box-arity** — the number of inner boxes of a multi-morphism, how many operations are plugged in at once — as an additional tracked quantity beside the leg count and the coherence-debt arity, rather than swapping the representation for the operadic one.
2. **State the expected-holds and the expected-fails against that implementation before any literature contact.** The three-quantities caveat supplies the first expected-fail: a truncation at box-arity k bounds nothing about port counts, because box-arity is compositional depth and the many-out content has been pushed into the **colours** — a box with many outputs is one colour — so the filtration is blind to the many-out axis.
3. **Then map the implemented tracking onto the machinery's hypotheses**, in three checks, in order, each able to stop the spike:
   + fix the arity functor and expect the elementary-iff-arity-one corollary to fire;
   + check hereditariness, and if it fails note that the weaker quasi-partition theorem still survives and is still usable — hereditariness is decided nowhere in this corpus and stays an open lead ([[guards#Name collisions — read the definition, not the section title]]);
   + do **not** reach for the slice description at all [@barkan-2022-arity, thm 5.1], since its unary hypothesis fails: it needs the unary part to be a groupoid [ibid., obs 5.9], and gandr's unary part contains grafted unit-corolla chains.

**Two items leave this entry and outlive whatever it returns.** The corolla and diamond computations are [[#meta-datum-01|meta-datum-01]]; and the claim that the Morita-restriction failure, the elegance gate and the coherence-connectivity criterion are three faces of one condition is retired from this spike and re-filed as a question in its own right, [[#Open questions|meta-question-25]].

### meta-spike-06

**Does the site carry the Reedy structure the staging consumes, and is that structure attributed?** **REFRAMED 2026-08-02**, after the earlier form — _is the graphical category elegant?_ — was executed and found **ill-formed**: elegance is a property of a **strict** Reedy category, in which identity maps are the only isomorphisms, and this site's objects carry the symmetric groups among their automorphisms, so the condition is _undefined_ here rather than false.
The disposition of the reframing is an open owner decision (`gandr-hpck`); the questions below are the ones to run.

**The mechanism is not missing, and the earlier alarm about it was misplaced.** Generalized Reedy _is_ the technology for categories with nontrivial automorphisms — equivariant latching and matching, with factorization unique only up to isomorphism — and it is already what [[../metatheory#The site, the strata, and the fuel are one object|the staging]] runs on.
`Rigid.canon` is what turns that up-to-iso factorization into an actual function, which is the third of its four appearances.
So the residual risk was never _is the site Reedy_ but **does the canonicalization stay computable**, and that failure mode is already carried as a falsifier: the generalized-Reedy factorization not being computable even with canonicalization, at which point strata and fuel decouple ([[../metatheory#What would falsify this]]).

**What is genuinely owed is the instantiation, and the warrant for it lapsed rather than never existing.** The nerve route's original warrant was at the **properad** rung — a fully faithful nerve with strict Segal characterization, finite hom-sets from the finite-edge-set determination, and decidable morphism equality — and the generalized-Reedy structure came with it.
Those attributions did not survive the move to the circuit-algebra rung, and what stands in their place is a citation of the _definition_ [@berger-moerdijk-2011-reedy], which is why a reader checking the claim finds no proof for this site.
**This is the second recorded instance of a warrant lapsing silently at a rung change**; the first is [[#Open questions|meta-question-19]], and the pattern — a citation correct at the rung it was taken at, surviving verbatim into a document written at a higher rung — is worth a sweep of its own.

Three questions, in order:

1. **Follow the positive result in the factorization paper.** [@hackney-robertson-yau-2018-factorizations, prop 3.7] gives the _wheeled_ graphical category a generalized Reedy structure, factorization unique up to isomorphism, after the modification its own second section introduces — and it is the same paper whose theorem 4.11 supplies the Eilenberg–Zilber _negative_ result, so both faces come from one read.
   What is owed is a **bridge from that category to the nonunital circuit-algebra site**, not a theory from scratch; that bridge is the presentation obligation below, which now has a target.
2. **Decide whether the general case is needed.** If the bridge does not go through for this site specifically, the question becomes whether a generalized-Reedy structure can be established for the site directly, or whether the staging should be re-based on a weaker structure that supplies degree, the two subcategories, and equivariant staging without the full package.
   State which of the three the answer is; do not silently weaken.
3. **Only then, the Eilenberg–Zilber question**, which is the well-formed replacement for elegance and is what the univalence _transfer_ of [[../metatheory#Univalence beyond the code universe — transfer, structures, repair|the third item]] wanted.
   Note that the transfer is the **auxiliary** route: it makes the ambient diagram model behave, and is not the route by which gandr's own code universe is univalent.

Two pieces of evidence to carry into all three.
The published Eilenberg–Zilber negative at the wheeled rung runs through the **nodeless loop**, which neither the circuit-algebra formalism nor gandr's carrier can express, so it does not transfer on the nose; the sharp test is that paper's second, loop-free counterexample class.
And symmetric-group automorphisms are demonstrably **not** per se an obstruction to univalence — a univalent model exists over presheaves on the cartesian cube category, which is generalized Reedy with exactly those automorphisms, at the price of an **equivariance condition on fibrations** [@awodey-cavallo-coquand-riehl-sattler-2026-equivariant].
That price is the shape of price to expect here.

**The `Rigid` branch is restated rather than retired.** `Rigid` supplies skeletality; equivalence to a _strict_ Reedy category needs the absence of nontrivial automorphisms, which is the other half.
The device that supplies the missing half restricts **morphisms**: the total category of a crossed group over a strict base, for which the dendroidal category is a worked published instance, proved by the restriction functor's monadicity creating the absolute pushouts the third axiom needs.
So the live question is whether this site is such a total category over a strict Eilenberg–Zilber planar base, with the ordered `Shape` carrier as the candidate — and even a yes does not descend the transfer for free, because presheaves on the total category are _equivariant_ presheaves on the base.

### meta-spike-07

**The descent corolla-restriction lemma** — reflect an isomorphism of free algebras back to the generating species.
**Small.**

Per-stratum `ua` is a citation **plus this lemma**; easier at the nonunital rung; must not be discovered at implementation time.

### meta-spike-08

**Exhibit the protype whose tabulation is funext over cellular data**, and confirm both ends are objects of one equipment.
A named candidate answer exists from the synthetic-calculi analysis: the protype is the **loose unit**, and funext is **unit-pureness** — full faithfulness of the unit — so the spike may be a verification rather than a search.
The fallback template is the syntactic lax-cones-over-computads construction ([@mikhail-2025-thesis] ch. 1; globular contexts only, a limit not a tabulator, general case a conjecture).
**Unmeasured.**

The one genuine construction the equipment join rests on.

### meta-spike-09

**Is the removal/rebuild pair an instance of the context comonad for tree-like types**, and does the spanning-tree traversal give a shape for `canon`?
[@altenmuller-2026-string-diagrams] **½ day.**

Generalizes a bespoke device; the only concrete lead for the _form_ of a canonical linearization.

**Two concrete candidate shapes are now on the table, and they bracket the family** (2026-08-01): for one syntax over unrooted trees, orienting its cut equation one way makes normal forms **corolla decompositions** — pick a vertex and recurse into the components its removal leaves — and orienting it the other way makes them **edge decompositions** — pick an edge and recurse into the two components its removal leaves, which is the spanning-tree traversal this spike went looking for; the source exhibits both and observes that they are the two extremes of a mixed style [@obradovic-2017-thesis, sec. 2.4.2].
The same source is a caution as well as a lead: its rewriting system is **non-confluent** at exactly the cut symmetry — a redex and both its reducts denote one tree — and its worked example exhibits **five** distinct normal forms for a single tree, so a decomposition shape is a canonical-form _candidate_ and owes its own confluence argument.

**The spanning-tree half is answered at the circuit rung, and the caution is answered with it (2026-08-08).** The edge orientation is selected and built as a boundary-anchored traversal over the monogamous fragment's diagram view, with the reason, the premises it rests on, and its cost recorded at [[../implementation/circuit-terms#Matching, normalization, and the crate boundary]].
The confluence argument the caution demands is not owed, because the reading changed rather than the shape: producing a normal form by _rewriting toward_ it raises a confluence question, while producing it by a deterministic _traversal_ does not — uniqueness is by construction.
**Two halves of this spike stay open.** Whether the removal/rebuild pair is an instance of the context comonad is untouched, and the built canon is a canon on the **diagram view**, not on the carrier's construction terms — so `Rigid.canon-sound` below keeps its own recipe and its own residual.
Whether the two canons agree is no longer this spike's residue: it is scheduled as [[#meta-obligation-01|meta-obligation-01]], where what would discharge it is stated.

### meta-spike-12

**The finiteness/simple-connectivity measurement, re-specified.** The gate must measure the _semantic_ shape (count cells with a repeated metavariable), not the as-built shape — nominal sharing makes the as-built measurement circular.
**1 day.**

Settles what fraction of real cells leave the dioperad fragment once sharing is a wire.

### meta-spike-13

**Supply the tile relation** and instantiate the axiomatic-rewriting axiom interface non-vacuously, resolving the contested axiom count first (nine axioms in one presentation, a ten-item interface in another — both may be right for different presentations).
**2 days.**

Four standardization theorems by citation; turns a vacuous pass into real inheritance.

### meta-spike-16

**Re-decide the double-pushout scoping at the circuit rung.** **EXECUTED 2026-08-01.** The verdict is **re-scoped, not retired**, and its full statement with citations is [[../implementation/circuit-terms#The correspondence at gandr's own rung, at theorem grade]]; the track's own sentence is restated at [[../metatheory#The operational substrate — the polarized sequent kernel]].

In one paragraph: the applicable instance at gandr's rung is **convex** DPO with interfaces over **monogamous acyclic** hypergraphs, and the correspondence with syntactic rewriting is an **iff for arbitrary symmetric monoidal theories, coloured ones included** [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-ii, thm 25, thm 35, thm 39].
The fragment's two conditions are exactly gandr's two standing declines — monogamy is the fan-in (and fan-out) refusal, acyclicity is wheel-freeness — so many-out, reconvergence and disconnection are inside it and wheels are not.
The Frobenius hypothesis is not the obstacle it looked like: it is what the _first_ paper assumes, and the sequel discharges it.
What replaces the mono-left-leg condition is the **boundary complement**, unique whenever it exists, explicitly including rules that are not left-linear [ibid., prop 31] — which matters because gandr's patterns are not left-linear in general.

Two things the spike did not settle and that are now carried as named questions rather than as this spike's residue: whether gandr's cell application is **convex**, and what covers the **traced** rung the arity ruling has made gandr's destination ([[../implementation/circuit-terms#The design questions]], `circuit-terms-question-15` and `circuit-terms-question-19`).

### meta-spike-17

**Does decidable confluence transfer, and does the critical-pair procedure?** **EXECUTED 2026-08-01**, jointly with [[#meta-spike-16|meta-spike-16]].

The result transfers, and the mechanism is the part worth carrying.
Local confluence of DPO-with-interfaces follows from joinability of all pre-critical pairs, and for a **computable terminating** such system confluence is **decidable** [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-iii, thm 3.1, cor 3.1] — where _computable_ is a defined condition (computable pullbacks; a finite computable set of quotients of $L_i + L_j$ per rule pair; enumerable one-step rewrites) and the ambient hypotheses hold in any presheaf category and are stable under slice [ibid., asm 3.1].
**The interface is what saves Knuth–Bendix, and the empty-interface case is the undecidable one**: the authors' own framing is that hypergraphs with empty interface are the graphical analogue of ground terms, and that ground confluence is undecidable for terms and graphs alike while confluence is decidable for both [ibid., secs. 3 and 7].

**What it means for gandr's contract.** gandr's completion engine already enumerates critical pairs and already carries seam data, so it is on the decidable side of that line; its _completed means the worklist drained_ caveat is therefore about **budget**, not about undecidability, and saying so is a material sharpening of the contract.
What the engine does **not** have is the other half: the published decision requires termination, and gandr's completion has no termination argument — its reduction order is plain node count, not a substitution-stable order.
Without Frobenius there are additionally two routes and gandr must pick one: **left-connected** systems (left-linear, ma-rules, strongly connected left-hand sides), where the notion of critical pair is unchanged and confluence of a terminating system is decidable [ibid., def 5.6, thm 5.3, cor 5.1]; or **path joinability**, which checks a critical pair under every maximal path relation via three formal path generators and is necessary as well as sufficient over the extended signature [ibid., def 5.7–5.10, thm 5.4, thm 5.5].
The residual obligation is therefore **a termination argument and a route choice**, both of which belong to the engine and are recorded on the implementation lane rather than here.
The route choice has since been taken for the **as-built** cell alphabet and re-opened by its growth — every expressible cell left-hand side is strongly connected, so the left-connected route is the one to take, its two remaining conjuncts are build items rather than research questions, and the multi-output and disconnection axes each break the verdict ([[../implementation/circuit-terms#The correspondence at gandr's own rung, at theorem grade]]) — while the termination argument stands open.

### meta-spike-15

**Map the landed description constructors onto a graphical-species profile.** The six code-grammar variants against the tiny base of finite sets, bijections, and the input/output involution.
**1 day.**

The first thing to test on the pasting side — everything in the univalence section assumes it; the falsifier is a description needing dependency or indexing the base cannot express.

## Standing data — measurements that outlive the spikes that produced them

A spike can be reposed, retired, or answered negatively and still leave a measurement behind.
Those measurements live here rather than inside a spike entry, so that citing one does not depend on the entry's fate.
The numbering is stable in the same way the spikes' is.

### meta-datum-01

**The partition complexes of gandr's own shapes, computed — and the disconnection axis moves the homotopy type without moving the arity bound.** At a corolla the partition category is empty, so the complex is empty and its connectivity is the floor of the scale; the reading is that a generator is indecomposable and arity approximation buys nothing there.
At the diamond the partition category is a rank-two poset whose order complex is a graph, and the block convention changes what that graph is: **thirteen vertices and eighteen edges, a wedge of six circles, when disconnected blocks are admitted**, which is gandr's rung; **ten vertices and twelve edges, a wedge of three circles, when blocks must induce connected subgraphs**, which is the properadic rung.
**Both give connectivity zero.** The disconnected-block count reproduces the arity-four value the arity-approximation source works out, which is an independent check on the model [@barkan-2022-arity]; that value is carried without a locator against the held artifact, so the check is at report grade rather than at locator grade.

So **disconnection** — the axis the generality ruling insists on — changes the homotopy type of the complex and leaves the bound the arity-approximation theorem consumes exactly where it was.
That is what makes the measurement decision-relevant, and it is robust to the block convention the computation could not settle, which is why it is recorded independently of what [[#meta-spike-05|meta-spike-05]] returns.

**Two limits travel with the number, and neither is a footnote.** The model is an ad hoc vertex-partition model rather than a structural computation, so a graph-side description of the partition complex is what would make gandr's connectivity bounds computable rather than conjectural — locating one, or establishing its absence with the search recorded, is tracked as `gandr-8v8b`.
And **which arity the diamond has is not settled**: gandr's diamond is a closed shape with empty interfaces on both sides — `Gandr.Shape.Graph` declares it `diamond : Shape ⊤ [] []` — so under the underlying-legs candidate functor of [[#meta-spike-04|meta-spike-04]] it has arity zero and this computation would be vacuous.
Which functor is right is the first of the three checks at [[#meta-spike-05]], and neither entry is corrected before it is fixed.

## Standing obligations — what must be proved or built

* **Grafting associativity, and the shape layer's `Monoidal` instance with its interchange equation** — the duoid target.
  **The wiring half beneath them is closed** (2026-08-01): `match-comp` is associative unconditionally, so the wiring category's associator parameter is dropped and the degenerate instance that existed only to rule out that parameter's vacuity is retired.
  What closed it, since the estimates were revised twice on the way: the exchange is packaged as the **symmetric group's Coxeter presentation** on a tower of nested insertions — one generator and three relations, over a tower indexed by the list of colours it inserts, so one structure serves every arity — which replaced a ladder of fixed-arity records with a bespoke coherence per rung.
  Over that presentation the four entries of the threading commutation table are word equations; both lookups' apart-lemmas (what a lookup sees of a threading at a position _apart_ from the one threaded) are each the same three steps, rebuild the wiring from the lookup's own value, commute the threading past the rebuild, read the answer back through the round trip; and the two commutation laws the associativity reduction turns on are not inductions at all, each being both factors rebuilt from what the lookups returned and then whichever composition lemma the rebuilt pair asks for.
  **The prediction that no further coherence was owed was falsified once and then held**, and both readings are worth keeping.
  It failed at the cut apart-lemma, which lands on two threaded cuts commuting — five layers, and under the fixed-arity records a new rung _and_ a new permutation family; that measurement is what the presentation was taken for, and under it the same fact is one more word.
  It then held at the fused half of the wire case, whose deepest branch was the named falsifier — a fact relating two inverse lookups at different positions — and is supplied by the rebuild instead, spent a second time on the first factor's own tail.
  The halves still differ for the structural reason previously recorded: a cut leaves the second wiring's sources alone, so both sides consult it at the same position, while threading a wire _moves the lookup_, so the two sides consult the second wiring at two different positions.
  Two things this entry previously anticipated did not arise in the shape stated: the glue lemmas relating `removal-tail` and `removal-recap` to removal composition were not needed, because the block liftings were proved to _be_ the threading operations at the head rather than second devices to keep in step; and the inverse lookup's law, carried as not analysed to the removal's depth, is strictly the simpler of the two, since only a wire reaches a sink and its cut branch has no counterpart.
  The postmortem's guard — that naming a structure by generators and relations claims the listed relations are all of them, and that the claim is **arity-scoped** — is recorded in [`docs/workflow/agda.md`](../../../workflow/agda.md) and is the reusable part.
  Keep acyclicity, tractability, and termination as separate named obligations with separate suppliers, never one monolithic convergence proof.
  **The rungs still owed are re-scoped by the former ruling of the arity-interface item below.** Substitution is primitive, so the closure subsumes `match-comp`: grafting associativity is reached through the monad law rather than through the accumulator, and what stays owed here is that and the interchange equation.
* **Unit five's interchange as a `Tower` consumer**: the interchange equation's middle profile is an existential intermediate — the coend variable, appearing on one side only — which is exactly the shape the `Tower` device packages (carry the intermediates as fields, not indices); reach for it before stating the equation.
* **`Rigid.canon-sound` at the circuit rung**: the construction-term normal form (permutations outermost, ordered tree monomials, unique minimal representative), with the **monomial-to-monomial rewriting condition** checked before anything leans on it.
  This is the **theory-side** canon; the boundary-anchored traversal built at the circuit rung is the representation-side one, and that the two agree is [[#meta-obligation-01|meta-obligation-01]] rather than something discharging this entry would establish.
  **The setoid the instance is taken over is now built, and the condition is checked** (2026-08-08, `Gandr.Arity.Universe` §_Step six_).
  A `Rigid` instance needs its **relation** before it needs a canonicalization, and that relation had never been written down: it is now the congruence generated by transposing two adjacent vertices, over a block swap that leaves the interface and everything behind the two published blocks alone.
  Proved: the transposition, that it exchanges exactly those two profiles in the vertex listing, that the generated relation is an equivalence, that it is not vacuous at a closed pair and at an open one — both decidably apart as terms — and that supplying `canon` with its two laws yields the instance, so **`canon-sound` and the decision procedure are derived rather than owed separately**.
  Stated and not inhabited: the canonicalization and its two laws.
  Not attempted, and deliberately not stated as a type either: that a step relates isomorphic graphs, whose statement needs an edge bijection and a vertex relabelling that would have to be built first — one computed instance is pinned instead, and until it lands the relation's soundness is designed-in rather than proved.
  **What the build returned that the recipe did not predict**: two of the three steps are discharged by the representation rather than by a construction (the chain order **is** the outermost permutation; the monomial is that order forgotten), the condition is discharged by the ambient (a relation on an inductive family in `Set` has single terms on both sides, and the consumable form is that `canon` is an endomap), and the residual is the total order with the minimum over the orbit.
  **One reading is challenged and filed**: whether the equivalence `canon` sections fixes the interface pointwise, which decides whether the open merger-swap pair is an identification it owes at all, and therefore whether the generator above is the whole of the relation — filed as `gandr-hpck-question-09` on the metatheory decision queue, unanswered, with nothing edited on its strength.
  **A worked precedent and a bounded residual are recorded against it, and the precedent is the larger half** (2026-08-01, reframed the same day after the reading rule was stated).
  The precedent first, because an earlier draft of this entry led with the hazard and thereby made the same mistake the rule names.
  Categorified cyclic operads are given a coherence theorem **non-skeletally**, and the skeletal presentation enters _inside that proof_: the third of its three staged reductions builds a skeletal non-symmetric categorified operad whose objects carry the ordering **as data** — an operation, a distinguished entry, and a bijection totally ordering the remaining entries — and the theorem is discharged by reducing through it, the coherence of that skeletal target being a prior published result [@obradovic-2017-thesis, sec. 4.1.5–4.1.6] [@dosen-petric-2015-weak-cat-operads] (the same construction and proof are in the standalone joint paper [@curien-obradovic-2019-categorified-cyclic-operads, sec. 2.5–2.6]).
  That is [[../metatheory#The representation is not the theory|the section discipline]] carried out at an adjacent rung by someone else: the theorem is stated at the symmetric layer and _proved through_ the ordered one, the coherence flowing from the ordered side to the symmetric one, which is exactly what `Rigid` is for.
  It is therefore evidence that gandr's whole representation discipline is workable, not evidence against it.
  The three staged reductions it runs through — get rid of symmetries, get rid of cyclicity, establish skeletality — are the same ones [[#Open questions|meta-question-19]] reads for their shape, so the two entries are reading one proof and must not drift apart.
  **A second claim of that thesis is not this one, and an earlier revision of this entry put that one in the instance's place.** Skeletal _exchangeable-output_ coherence is stated to follow by lifting the non-skeletal/skeletal equivalence to the categorified setting — and the lifting is omitted in the source's own words; what the appendix works out in detail is the uncategorified equivalence, and that one is for symmetric operads [@obradovic-2017-thesis, sec. 4.2.2, app.
  A.1].
  The claim itself is not in doubt — the source states plainly that skeletal coherence in the presence of symmetries holds by reduction to the non-skeletal setting — but it is a stated claim with its lifting omitted, not a worked construction, and only the worked one carries the instance.
  **The residual is one named proof move, and it is a cost rather than an obstruction.** The thesis's appendix states that non-skeletality is crucial for the rewriting its proof uses: non-skeletally a symmetric-group action can always be pushed _inward_, from a composite onto its operands, by orienting the equivariance law that way, and the first reduction depends on it; skeletally that distribution "doesn't work in general" — an observation the work credits to Petrić — exhibited at a three-element composite, and the source adds that it does not know whether orienting the equivariance the **other** way would serve, a caveat the thesis carries and the standalone paper's counterpart remark does not [@obradovic-2017-thesis, app.
  A.2] [@curien-obradovic-2019-categorified-cyclic-operads, rem. 4.5].
  The inward move is the equivariance law read from the composite onto the operands; gandr's recipe pushes permutations **outward**, so it takes the opposite orientation, which is exactly the one the source declines to claim about — the residual is therefore precisely that **a gandr proof may not reach for the inward move, and must route through the non-skeletal setting instead, as the source itself does.** What is on the record is therefore a price, not a warning: an ordered representation is not coherence-neutral, and the move it forgoes is named here rather than discovered at proof time.
* **The arity interface at the circuit kit.** The interface is settled and landed universe-style in `Gandr.Arity.Universe`, with the linear instance complete and ten of thirteen fields inhabited at the circuit kit ([[../metatheory#The arity interface, universe-style]]).
  **Graph substitution is built**, over the two-sided closure it decomposes into, and so is its interpretation law; an earlier revision of this entry recorded eight of thirteen and the whole former as owed.
  **The former was ruled** primitive, with grafting derived from it by substituting into a two-corolla series shape, and the build confirmed the ruling's cost estimate on the reuse side exactly: the twelve auxiliaries named as reusable were reused, the four named as retired were not needed, and the kit's only well-founded recursion is retired.
  The estimate was low on the new side by two, because a vertex's two port blocks are crossed and the block iteration is therefore two operations rather than one.
  Its circles are **counted, not discarded** — a code is a shape with its number of closed components, which is the source's own definition of a Brauer diagram rather than a gandr device, and `Match` and `Shape` are untouched; that the count is populated rather than merely carried is pinned by computation.
  **The three that remain are the two unit laws and associativity**, each carried as a type so the next pass starts from a signature, and two obligations travel with them — the **agreement lemma** against the built `graft` (without which `verts-graft`, the two unit laws and the merger's incidence theorems do not transfer) and the **count law** for associativity.
  What is known about the unit laws' cost, and is not more than this: they need the witness the closure rebuilt a node with to be the witness the original node carried, and both natural formulations of that are stuck on a reflexive list equation — the condition the grafting unit laws already take as parameters.
  The interpretation is already derived — the profile-indexed vertex family is the generic listing-occurrence family at the vertex listing, so it is not a second induction over the shape and must not be re-declared as one.
  The obligation the presentation promotes rather than discharges is the representation map, which at this rung _is_ `Rigid.canon-sound`; it is a field of the interface, so no circuit instance exists before it does.
  The linear kit is the worked precedent for what that field asks: refuted against the bare interpretation, proved against the ordered one, with the two bracketing what the interpretation must remember.
  Two spikes are retired here rather than carried: the control experiment at the linear kit (the presentation is cheaper — the unit and associativity laws are the existing lemmas read off the graph by functionality, the whiskering lemma has no counterpart, and one new lemma is the whole price), and the coherence-law count (three, because the former is a monad multiplication, and homogeneous because the codes are indexed).
* **The presentation of the graphical category** $Θ_(T^times, "Gr")$ — degree, the two subcategories, computable factorization through canonicalization, decidable morphism equality — and **the oriented-slice arities restatement** (a paragraph: transfer the arities claim along the slice equivalence to the oriented monad).
* **The Segal certification ladder**: per-degree Segal checks; equivalence certificates and per-degree checking on a toy signature with one nontrivial automorphism; fuelled transport with cost accounting (do the two cost measures separate cleanly in code?).
* **The two-cell coherence of the layout univalence map** (the typoid-function obligation), and the _statement_ of its pasting-side analogue.
* **The directed convergence pass** for the directed rule layer — **re-quote its price against the arena presentation first** (the recorded transformation-monoid price was quoted against the superseded tree presentation), and evaluate a focusing-staged normal form before any raw completion pass.
* **The carrier-to-source translation lemma** (the shape carrier against the graphical-species presentation).
* **The decomposition-space edge**: verify the measured strict pullbacks are the stability condition, establish or refuse a set-level shadow, and record the certificate-layer/doctrine-layer identification with its citations.
  The unitality half is dissolved by citation — every 2-Segal space is unital [@feller-garner-kock-proulx-weber-2019-unital]; the pita-nerve result — the nerve of a strictly factorisable operadic category with invertible quasibijections is a decomposition space, with an explicit non-Segal counterexample when they are not invertible [@batanin-kock-weber-2018-regular-patterns] — is adjacent input; and the decomposition-space line's _locally discrete_ grading is the literature's closest analog of a witness h-level, logically independent of Segal versus 2-Segal, and bears on [[#Open questions|meta-question-08]].
  Two adjacent set-level facts from the held properads literature frame the same edge [@hackney-robertson-yau-2015-properads]: the strict-Segal nerve theorem (a fully faithful nerve for properads, with the strict properadic Segal condition and unique inner-horn fillers) is a set-level warrant for the nerve direction at the properadic rung, decoupled from the polynomial-interpretation obstruction; and the finiteness wall — the free many-to-many term set over a cell is finite exactly when the cell is simply connected — is the computable-layout-relevant boundary, more than automorphisms or the nerve theorem itself.
* **The core-coincidence theorem** of the directed statement (the groupoid statement as the invertible core), and the directed normal-form/faithfulness wall behind it.
* **Deleting the cell record's simple-connectivity field** (or re-carrying it as a consumer-side predicate) under the generality ruling, with the surface-language question — whether the _surface_ still hides wheels and disconnection — as its own design pass.
* **The polygraph presheaf-criterion check is discharged** (2026-08-08), and it moved the condition rather than confirming it.
  The obligation as recorded asked whether gandr's pattern-to-pattern rules satisfy **non-unitality**; against [@henry-2019-nonunital-polygraphs] they do not, and the escape hatch does not need them to.
  Theorem 2.4.8 there makes the class of _all_ **source-positive** polygraphs a good class of polygraphs, and that is where gandr sits: `elaborate_operation_cut` in `crates/theory-computads/src/elaborate.rs` admits only an operation-headed left-hand side, so every generator's source carries its operation symbol.
  Target-positivity fails on projections, so the stronger non-unital class of Definition 1.3.2 is unavailable and the hatch holds through Theorem 2.4.8 directly rather than through Corollary 2.4.9.
  The corrected claim, its locators, and the source/target asymmetry it exposes are [[../metatheory#Cellular data — descriptions, cells, and computads|the metatheory track's cellular-data section]].
  **What remains open is carried, not closed**: the target-side half rests on the reading that a bare-metavariable command is an identity 1-arrow on its sort, which is the standard polygraph correspondence but is not something the code states, and the source-side half does not depend on it.
* **Standing obligations inherited from the identity/reflection arcs**, each a named hole until discharged: J-as-tabulator-elimination is a **theorem obligation, not an identification**; higher-stratum transport owes explicit lifting/cumulativity coherence; protype-isomorphism certificates stay **separate** from equivalence/univalence certificates until the bridge theorem lands; the variance accounting of directed observations and corecursion owes a theorem rather than a description; unfolding and certificate replay must be defined as an **operational relation** before justifying temporal transport with them; and the base-stage identity law must be stated honestly (never implying full path induction where only the saturated-instance eliminator exists).

**Named obligations — anchored, because these are cited from outside this document.** The bullets above are carried inline and cited by their subject; an obligation another document must point at is minted below as a numbered heading instead of living as a sentence inside the block that discovered it.
The numbering is stable in the same way the spikes' is: retiring an entry leaves its number unused.

### meta-obligation-01

**Do gandr's two canonical forms agree?** The **diagram-view** canon is built (2026-08-08) — the boundary-anchored traversal over the monogamous fragment, with its premises and its cost recorded at [[../implementation/circuit-terms#Matching, normalization, and the crate boundary|the circuit lane's normalization block]].
The **construction-term** canon is the recipe `Rigid.canon-sound` above owes — permutations outermost, ordered tree monomials, unique minimal representative — and it is not built.
Nothing on the record says the two pick the same representative, and **agreement is owed rather than assumed**.

**The layer split is why this is an obligation and not a detail.** _Diagram normal form_ asks when two circuit terms denote the same diagram and is a property of the **representation**; _rewriting normal form_ is the result of running the rewrite system and is a property of the **theory**.
The traversal answers the first and the construction-term recipe answers the second, so the two are canons on different objects at different layers.
**Conflating them is the named hazard** — the circuit lane states it where a reader meets it, and this entry exists so that the statement is not carried only by a sentence inside an as-built block.
A reader who takes the built canon as discharging `Rigid.canon-sound`, or the ruling as covering what the traversal decides, has made exactly that mistake.

**Stating the claim is part of the obligation, because the two canons do not act on the same objects.** What relates them is the interpretation carrying a construction term to its diagram view, and agreement is the claim that the equality each canon decides is the same equality once pulled back along it.
A discharge is therefore a proof of that at the level the claim lives, or a **counterexample** — a pair of construction terms that one canon identifies and the other separates, in either direction.

**The suggested route is executable, and it is a suggestion rather than scheduled work.** Under the implementation-first posture [[#meta-spike-05|meta-spike-05]] now takes — implement first, state the expected-holds and expected-fails against the implementation, and only then map onto the theory — the natural shape here is a differential check: run the built traversal over the diagram views of construction terms whose theory-side normal forms are computed, and look for a disagreeing pair.
No spike is opened by this entry; the route is recorded so that a first pass at it does not have to be designed from nothing, and so that the choice of proof-first is a decision rather than a default.

**Provenance.** Scheduled by `gandr-hpck-answer-16` on the metatheory decision queue, which ruled that this question be carried as a named obligation rather than as a sentence inside an as-built block.
The layer split it rests on is that queue's `gandr-hpck-note-03` amending `gandr-hpck-answer-08`, whose construction-term ruling is untouched by this entry.

### meta-obligation-02

**On which gandr diagrams does a wiring factor as an interface permutation followed by a crossing-free core?** The corpus consumes a theorem stated for connected **planar** diagrams [@majid-rietsch-2021-planar-spider, cor 2.4], and the passage that licenses the consumption asserted the factorization as a fact about the representation: the shape being a canonical boundary permutation composed with the standard form, with the permutation canonicalized the way `Rigid`'s recipe already canonicalizes construction terms.
That assertion is **restated here as an obligation** rather than carried as an inference, because the word "planar" wears two conditions and the assertion silently spends both.

**The statement, so that a discharge has something to hit.** For a shape $S$, the factorization exists exactly when the underlying graph of $S$ admits a crossing-free embedding **after** reordering its interface — and reordering the interface is precisely what an outermost permutation does, so the condition reduces to planarity of the underlying graph itself.
Boundary reordering absorbs a boundary crossing and touches nothing inside a component; **no reordering repairs a non-planar interior**, which is why the two halves of the condition do not collapse into one.

**Two things are settled already, in opposite directions, and both were measured rather than argued.**

* **Refuted on the carrier.** `Gandr.Shape.Planarity` builds `k33 : Shape ⊤ [] []` — six vertices, nine flow-through wires, connected, and wheel-free by a rank certificate — whose derived incidence realizes the complete bipartite graph on three and three, so its underlying graph is non-planar [@kuratowski-1930-courbes-gauches].
  The witness is inside the fragment the directed layer admits and is not a pathology quarantined elsewhere, so the factorization does **not** exist for gandr diagrams in general.
* **Available on cells.** A `Cell` carries `SimplyConn` — connected and acyclic — which is a tree, and trees are planar; the machine-checked half of that here is only the negative, that the K3,3 witness is not a cell.
  So the failure enters with **grafting**, which is where the carrier already knows it leaves the cell class: `bigon` is two cells grafted into a composite that reconverges.

**What would discharge it.** A characterization of the carrier's crossing-free fragment together with a decision for membership on gandr's own representation — planarity is classically decidable in linear time, so the question is what the decision costs over `Shape` and not whether one exists — plus evidence that the consuming site's diagrams lie inside it.
A discharge that only names the fragment without deciding membership leaves the consumption unusable, and a discharge that decides membership without the consuming site's diagrams leaves it unspent.

**What this does not license, in either direction.** It does not license refusing the theorem: the base-objects sense of "planar" is had everywhere, and the two senses are distinguished at each of the four sites that consume the inference.
Nor does it license reading the cell-layer availability as the general case, which is the mistake the entry exists to stop.

**Provenance.** Scheduled by `gandr-hpck-answer-17` on the metatheory decision queue, which pre-authorized this disposition on a non-planar result from the carrier planarity test and ran the test first as the proving spike.
The reanalysis it rests on identified the candidate factorization as the outermost-permutation normal form already selected for canonicalization, and found it insufficient for the interior — the test is what turned "insufficient as argued" into "refuted as measured".

## The wager's falsifiers

The coherence-debt arity law (debt arity = threaded positions + the head met; blocks contribute nothing) is the architecture's central bet.
Its three falsifiers, all scheduled or spikeable:

1. the residue after the epi–mono and Coxeter decompositions cannot be decided cheaply — the codiagonal alone forces the transformation monoid, which has no register row and no formalized rewriting twin;
2. the next unit's interchange requires two cuts to commute — the ladder is not finite, and gandr enters the measured-blowup regime knowing the growth law rather than merely measuring it.

The third falsifier the law once carried — that the graph former's coherence laws might not stay finite, making the universe route a rename — is retired.
The former is the arity monad's multiplication, so its laws are the monad laws and there are three of them whatever it multiplies; the interface index makes all three homogeneous.
What that retirement does **not** buy is a cheap construction: the coherence count and the construction cost are different questions, and the second is the listing algebra, which is untouched by how the interface is presented.

## Parked deliberately, with reasons

The free-bifibration STOP (gated on the factorization-preorder check, not scheduled); the coproduct-as-cache-keys and antipode-as-rollback directions (need their own design passes); the acceleration band (until the certificate relation has non-empty extension); the session/protocol code stratum (identity at a universe of protocols needs sessions reflected as codes; the cross-stratum seam must be flagged before either stratum hardens — passing to components across an ordering/no-ordering boundary is known not to be conservative); the permission-monoid question against the grade design; higher-order cells and the second hole theory (conditional on wanting them); instance-level keying of the overlap-support relation (the obvious first improvement once the relation is non-empty).
Also parked with their design substance pinned: **the sized/termination direction** — sizes enter as _indices in their own sort_ reusing the grade-zero erasure machinery, never as a fresh semiring grade (they need a well-founded order the resource semiring cannot express); bounded size quantification is unsound without a consistency gate on reduction; the well-founded fixed-point former is the single recursion-plus-corecursion former that retires the productivity/termination split; the guardedness check is a two-state flag automaton over observation-record introductions; and four named deep-guardedness programs pin the syntactic-check/sizes cliff for the corpus.
And **the codata dependent slots** — self-dependent projection result types, indexed codata, the without-K unifier extension, forced copatterns, and the empty cosplit — with the elaboration hazard pinned: lowering codata to positive records of thunks smuggles a computation into the value zone unless mediated by the shift, so the value-side and computation-side readings must be resolved deliberately, and codata has **no η** (undecidable, and recursive-record η breaks the elaborator's scope invariant).
And **the exact-reals / synthetic-topology track** — a lateral, firewalled line whose design, staged plan (stages A–G with gates), decision register, and open obligations are dispositioned in [[exact-reals]]; the metatheory-side carries are the equipment reading (the modal-law checklist as cartesian-equipment conditions, stated once when stages E/G approach), the temporal reading (observational backend equivalence as a temporal certificate), and the `ua_topo` statement shape with its precedent stack, none of which schedules work on the minimal-kernel path.

## Open questions

1. **meta-question-01** — Is the free-rig monad cartesian?
   — the single technical question on which any nerve route for the _layout_ universe turns; no published answer.
2. **meta-question-02** — Does the description universe fit a graphical species?
   — gated by its named spike; falsifier: dependency or indexing.
3. **meta-question-03** — Where does shift equivalence sit in certificate identity — should the _store_ key on the normal form, or only the comparator?
4. **meta-question-04** — Does the reflection face's cartesian-fibrational target restrict to the double-theory cartesian notions where both apply?
5. **meta-question-05** — ~~With non-linear patterns admitted, is the overlap family still finite and multi-universal, or does an occurs-check fragment need fencing?~~ — **dissolved by ruling 2026-08-01**, not answered: cell patterns are linear, so the premise no longer holds ([[../implementation/circuit-terms#The design questions|circuit-terms-question-17]]).
   Two things the dissolution changes rather than removes.
   The **globularity-above-the-base trigger** was "a non-linear pattern producing a genuine, non-singleton multi-sum family", which the ruling now prevents; it must be restated against the per-type-comonoid generalization, because that is the construction under which a genuine family could reappear.
   And the question **returns unchanged** if that generalization lands, so it is retired-with-a-reversal-condition rather than tombstoned.
6. **meta-question-06** — Does gandr _state_ the trek-to-tracelet seam (a publishable convergence of two disconnected literatures) or merely use it?
7. **meta-question-07** — Which cell classes exercise the residual part of the target-opfibration axiom beyond redex-creating instantiations?
8. **meta-question-08** — Is there operational content in the correspondence between the determinism axis (Segal → 2-Segal) and the symmetry axis, or is it orientation only?
9. **meta-question-09** — ~~The directed eliminator and diagonal-intro spellings; whether directed composition shares the groupoid composition name — surface vocabulary, cheapest settled before the identity-layer rules land~~ — **settled 2026-07-31 (owner decision)**: the eliminator is the shared `walk` (under the motive-covariance side condition), the diagonal intro is `diag`, and directed composition shares `then`.
   Landed at [[../surface-language/directed-family]].
10. **meta-question-10** — Where do the constant-map and constant-literal witnesses land as permanent negative tests — the first phase carrying a directed certificate stock over codes.
11. **meta-question-11** — The variance/directedness annotation slot in the export format: reserved early (recommended) or deferred to the certificate phase at the price of a coordinated format bump on the TCB.
12. **meta-question-12** — The doctrine complex's carrier shape: does it want **two node sorts per dimension** — signatures and relations, with a higher graph per pair of signatures and only the tight cells carrying cross-dimensional meaning — or does the single-sorted telescope suffice?
    (An open owner question of record, with a runnable sketch; the sketch's source file is in the pending sweep, so it re-lands when that sweep runs.
    Adjacent to, not settled by, the three-role split.)
13. **meta-question-13** — A two-point relevant/irrelevant variance record in place of a four-point lattice — co/contravariance presupposes cumulative subtyping, which gandr rejects, so only the irrelevant fragment transfers; and the elaborator will still meet stuck max-plus level equations (the oracle gives entailment and benign loops, not most general unifiers) — an unsolved user-experience surface gandr must own.
14. **meta-question-14** — **Cauchy completion as the representability axis** — how Cauchyness and Cauchisation sit under both univalence statements, and the equipment-level Rezk completion; state before either statement's representability is claimed.
15. **meta-question-15** — **The contraction locus** — what adopting the internal-logic equipment costs (no endo-coends), with its honesty gate; state the cost where the equipment is adopted, not after.
16. **meta-question-16** — **The Σ-former at the multi-output face** — the Σ-η direction is where fan-out actually bites (the dual of the data-η discipline), and premise-form statement is what keeps associative–commutative completion out of the rule layer; design before the term face hardens.
17. **meta-question-17** — **The Tietze ancestry note** — the edit-polygraph fullness statement ("complete up to a located obstruction") has a classical ancestor in Tietze-transformation completeness, with the simple-homotopy line as the cautionary instance above dimension one; record the lineage when the layout statement is next touched.
18. **meta-question-18** — ~~Pending targeted reads before import~~ — **resolved by the phase-2 sweep** (each item read at import grade against its primary and folded): the statement-blocker lesson with its blanket-base instance, the frame-bound impossibility, and the observation-grade ledger are carried in [[../proof-engineering#Lessons with no other home|the proof-engineering lessons]], as is the compare-site four-class taxonomy with the shape/witness grading ledger; the per-level cost law of the alphabet discipline (the square-compatibility cylinder one level up is the shape of term the naturality meta-operation generates, and any filler works there) is stated in [[ambient-and-primitives#The technology cluster|the technology cluster]].
19. **meta-question-19** — **The strictness warrant at the circuit rung.** The old licence — the rectification theorem, which said strict semantics is provably adequate at the dioperad rung and not above it — lapsed with the rung change, and nothing has replaced it: either show the rectification dichotomy has no set-level shadow (gandr's cells are finite ordered data compared by content address, not models of an ∞-object), or re-warrant strictness by a coherence-by-decision-procedure story — the skew-monoidal focusing line, the duploid line, or the Schwarz-paper Koszul machine at gandr's own rung [@kaufmann-ward-2024-schwarz].
    This was the substrate arc's deepest open question and it is still open; the re-read of the rectification paper against the shadow question is the named next step, and no consumer may silently assume strictness is adequate.
    **A worked instance of the second route is now on the record** (2026-08-01): a coherence theorem for categorified cyclic operads is proved by **three staged reductions** — get rid of symmetries, get rid of cyclicity, establish skeletality — which is coherence discharged by a decision procedure rather than by a coherence-term generator, at a rung adjacent to gandr's [@obradovic-2017-thesis, sec. 4.1].
    It is an instance to read for its shape, not a candidate re-warrant: it is one-output and unoriented, and its first reduction is the one whose skeletal failure is recorded against `Rigid.canon-sound` above.
20. **meta-question-20** — **A derived `Path`→`Flow` coercion**, once `ua-dir` lands — wanted as a derived surface form, and at which stratum?
    (No kernel coercion: the comparison is the core-coincidence theorem, and a coercion before it would assume the theorem as an axiom.)
21. **meta-question-21** — **Does the certificate layer's one-directional interchange force laxity into the statement's rule layer?** The directed statement keeps the dimension-2 rules an equivalence at this stratum; if mixed-polarity boundaries eventually force one-directional interchange down into the statement, the η grade needs restating — a new design pass, not an amendment.
22. **meta-question-22** — **Variance as a shared kind** carried by the reflected universe, decided when directed univalence is scoped on the reflection face.
23. **meta-question-23** — **The canonical (co)end representation**: does the finite-carrier diagram become the canonical reflected end-object once the description layer can express dipresheaves, or does it remain a property-test vehicle?
24. **meta-question-24** — **What does an adequacy claim for the certificate layer look like?** The diagrammatic-calculus literature has worked examples of the shape — a calculus, a semantics, and a completeness proof that the rewriting is exactly right for it — and gandr has no fixed semantic target, so the theorems do not transfer but the **argument shape** is the closest available model.
    **Carried**, with the stabilizer and Clifford+T completeness results and the ZW calculus recorded as the worked examples to read when the certificate layer's own adequacy statement is drafted; specific examples are more useful here than stronger general statements.
25. **meta-question-25** — **Is one condition standing behind the Morita-restriction failure, the coherence complex's simple-connectivity criterion, and the elegance gate?** The only honest shared statement anyone has formulated is that _the category of non-trivial decompositions of an operation is sufficiently connected_, with the three as its homotopical, presentation-level and strict instances; it rests on structural analogy and on no theorem in any held source, which makes it a good conjecture and a bad spike consequence.
    Three things stand against an affirmative answer.
    **Two of the three faces are related by a published open conjecture that points the opposite way**: the coherence-complex authors write that their results "seem likely" to be a strict version and special case of the arity-approximation theorem, and that it "would be interesting to see" how the two are related [@curien-laplante-anfossi-2023-topological] [@barkan-2022-arity] — which makes the coherence result a special case of the other rather than a co-face of a shared condition.
    **The third face has no bridge to either**: the arity-approximation source contains no occurrence of _Reedy_ or _elegant_, and the source the corpus cites for the elegance definition contains none of _partition_, _Morita_ or _connectivity_ ([[../metatheory#Stratified univalence]]).
    **And the three are not the same kind of condition** — an absolute-pushout condition on the degeneracy subcategory, a high-connectivity condition on slices of the active subcategory, and a simple-connectivity condition on a complex built from a presentation — which is the distinction [[guards#Name collisions — read the definition, not the section title|the guards ledger]] keeps under one phrase, so an affirmative verdict would have to retire that guard.
    Retired from [[#meta-spike-05]], which claimed it as a consequence; the quoted phrases and the two string searches are recorded without locators against the held artifacts, so they are evidence at report grade.

## Reading queue, by leverage

**Next.** The parametricity-cluster entry point [@vanmuylder-2026-thesis]; the pretype-theory report and slides [@nuyts-2026-natpt]; transpension, sections 1–2 [@nuyts-devriese-2024-transpension]; the discrete-Conduché paper, sections 1–2 (for [[#meta-spike-02|meta-spike-02]]) [@guetta-2020-conduche]; the polygraph shape category, introduction and section 5 [@hadzihasanovic-2020-shape]; the (∞,∞)-thesis ch. 1 part 2 (lax cones, for [[#meta-spike-08|meta-spike-08]]) and ch. 2 (the decomposition-space equivalence) [@mikhail-2025-thesis].

**The univalence-transfer chain.** The Univalence Principle, graphs-and-nets chapter first [@ahrens-north-shulman-tsementzis-2021-univalence-principle]; inverse diagrams, sections 11–12 (for [[#meta-spike-06|meta-spike-06]]) [@shulman-2015-inverse-diagrams]; the synthetic line as vocabulary only [@riehl-shulman-2017-synthetic].

**The hypergraph-rewriting sweep**, opened by [[#meta-spike-16|meta-spike-16]], **executed 2026-08-01 for the part that decided the spike** and reduced here to what remains.
Consumed at theorem grade, with what was taken recorded at the spikes above and in [[../implementation/circuit-terms]]: parts III and II of the series [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-iii] [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-ii], and the two implementations read as artifacts, Cartographer and Chyp [@sobocinski-wilson-zanasi-2019-cartographer] [@chyp].
The ESOP statement of the decidability claim needs no separate read — part III is its journal successor and says so [@bonchi-gadducci-kissinger-sobocinski-zanasi-2017-confluence-interfaces].

What the sweep still owes, in order of leverage:

* **the traced/wheeled gap** — nothing consumed reaches gandr's destination rung, and no candidate source has been identified.
  This is the sweep's open end, not a pending read;
* equational reasoning with **context-free families** of string diagrams, the published mechanism for rule schemas over unbounded arity, which a many-out rule with a variable port count would need [@kissinger-zamdzhiev-2015-context-free-families], with its language-theoretic successor beside it [@earnshaw-roman-2024-context-free-languages];
* initial-algebra semantics for **cyclic sharing** structures, the wheel axis from the syntax side rather than the carrier side [@hamana-2009-cyclic-sharing];
* the **multi-device** direction, which is where gandr's disconnection axis lands: effectful categories over several devices, presented by resourceful traces, with a commuting tensor product [@earnshaw-nester-roman-2025-resourceful-traces], read after the thesis that collects the single-device case [@earnshaw-2025-thesis].

The interface-literature map itself is already assembled in the related-work section of the combinatorial string-diagram thesis and should be read there first rather than reconstructed [@altenmuller-2026-string-diagrams].

**The arc's own outstanding reads.** Schwarz modular operads revisited — the one paper running this machine at gandr's exact rung [@kaufmann-ward-2024-schwarz]; circuit algebras are wheeled props (check whether the equivalence survives at `Set` — stated for linear wheeled props) [@dancso-halacheva-robertson-2021-circuit-wheeled]; the graphs/hypergraphs translation source [@kock-2016-graphs-hypergraphs]; the naturality meta-operation [@benjamin-markakis-offord-sarti-vicary-2025-naturality]; monoidal context theory, re-read at the sections the hole identification selects [@roman-2023-monoidal-context].

**Calibration, not adoption.** The combinatorial string-diagram thesis (the closest existing Agda encoding; three independent arrivals at gandr's decisions, one instructive fork, and the context comonad) [@altenmuller-2026-string-diagrams].

**Consumed.** The HoTT-operads internalization [@hewer-2025-hott-operads] — sections 4 to 6 read at import grade, and its generalised-operad-universe record is the shape the arity interface now takes ([[../metatheory#The arity interface, universe-style]]).
Its _development_ remains calibration rather than import for the reasons it was filed under: the wrong rung (one output), truncation to h-sets to avoid higher coherences, and a higher inductive type for the free construction — all three declined here, and the setoid substitutes recorded at the interface.
What was taken is the record, not the mathematics around it; what does not transfer is the representation map, refuted at both of gandr's kits.

**Technique, on demand.** Operads, clones, and distributive laws [@curien-2012-operads-clones]; topological coherence proofs [@curien-laplante-anfossi-2023-topological]; coinserters versus coequifiers [@lucatelli-nunes-2026-freely]; the source-positivity presheaf criterion for polygraphs [@henry-2019-nonunital-polygraphs]; cubical internal parametricity (admissible under the trusted-surface criterion) [@cavallo-harper-2021-internal-parametricity]; the skew-coherence-by-focusing line [@veltri-2021-coherence-focusing]; real-cohesion as the cost sheet if a modality revisit condition ever fires.
