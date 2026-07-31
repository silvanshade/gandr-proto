# Proof engineering

This track owns how mathematics is mechanized in this project, independent of gandr-specific content: the substrate the Agda development is built over, the representation and organization disciplines, and the coherence-cost engineering that keeps a higher-dimensional formalization tractable.
What the mathematics _says_ about gandr's model is the [[metatheory]] track; the operational workflow (gates, flags, dependency policy, commit shape, layout mechanics) is `docs/workflow/agda.md`, which deliberately carries no doctrine — this document is the doctrine's cross-module home, and per-module doctrine stays in the module headers, which remain authoritative for the structures they own.
Detailed remaining work is in [[proof-engineering/roadmap]].

## The substrate tower

**Everything is built over ∞-graphs**, the coinductive presentation of a globular carrier: a type of cells at each dimension with a coboundary at every parallel pair, observable on demand, never completed.
`Gandr.Graph` is the ambient category — initial and terminal objects, coproducts, products, the discrete and codiscrete inclusions of `Set`, the globes, the exponential, and the disc telescopes.

**Structures are projected out of carriers, not stacked on each other.** A `Setoid` equips a carrier's cells with reflexivity, transitivity, and symmetry and **carries no laws** — the lawless proof-relevant equivalence, the honest floor.
A `Category` is composition-and-identity structure **on** a carrier, not a bundle containing one; a groupoid likewise.
Because a hom-setoid is literally the carrier's coboundary at two objects, read off by projection, two structures over one carrier are comparable without coercion — which is why setoids are _projected out of_ the richer structures rather than the richer structures being built on top of setoids.
One bundle serves every dimension, since any level's coboundary is itself an ∞-graph.

**Laws are cells one dimension up.** Associativity does not say two bracketings are equal; it supplies a 2-cell a consumer transports along explicitly.
A strict law would be an equation in the ambient theory, silently promoting the setoid to a set; stating laws as cells is what keeps the development inside `SETOID`, where its results actually hold.
Consequences that are structural, not stylistic: quotients need not be effective (a canonical representative is _structure_, supplied by `Rigid` — see the metatheory's ambient section); a map respects an equivalence only when shown to; and the Yoneda correspondence is proved as pointwise value cells, never as an equality of records, because the development has no way to compare records and deliberately never does.
`SETOID` itself is constructed as a named category, so statements quantifying over categories genuinely have the ambient in range.

**Above the content, the tower is empty — never trivial.**

* `𝟘` above the last dimension with content is correct: it asserts nothing, discharges nothing, and reopens if genuine higher cells appear.
* `𝟙` above is forbidden by default: a terminal hom silently discharges coherences nobody checked — the premature-truncation failure mode.
* Forcing uniqueness of identity proofs is out of scope entirely; where set-ness is genuinely needed it enters as a `UIP` **parameter** on the index type, never through a shortcut.

**Where a structure has content is declared, not discovered**: the region-indexed certification (`At`, with `Everywhere` and the singleton region as its ends) certifies a structure at exactly the addresses a predicate admits, and an out-of-region address is discharged by constructor disjointness — the suite cannot even be formed there.
The region is per doctrine, not per carrier: one carrier can inhabit `Setoid` and refuse `Category`, and a refutation of a _certification_ must never be read as a refutation of the bare structure.
`Set`-level structures present through the discrete setoid on the identity type (the identity type as 1-cells, `𝟘` above), with a named former for the hom family so the separation between the bare instance and the dimension-wise certification stays real.

**The dimension is bound as a first-order code, never inferred.** A cell's type is a projection spine, and coinductive records have no eta, so anything that tries to infer a carrier or boundary from a cell gets stuck — a theorem about the unifier, not a limitation to engineer around.
The disc telescope is an inductive code interpreted onto the carrier; generic statements bind one telescope and instantiate at concrete addresses definitionally; the address code lets a statement bind its dimension _and_ refuse one.
This is one instance of the sort discipline that the known failure modes of higher-dimensional formalization all violate:

| the piece needs to be…                | render it as…                                               |
| ------------------------------------- | ----------------------------------------------------------- |
| discriminated, matched, recursed over | a **code** — inductive, injective constructors, first-order |
| witnessed or transported along        | a **certificate** — a cell, proof-relevant, carried         |
| neither                               | **unobserved** (object equality; record identity)           |

Forcing a certificate into a code is what uniqueness-of-identity-proofs and premature truncation are; leaving no code column is what makes an instance unwritable.
The design vocabulary for the tower is the explicit-coherence one — diagrams over an index category with all higher coherence laws carried explicitly — and the tower owes an equivalence statement against the Reedy-fibrant form of the same data ([[proof-engineering/roadmap]]); coinduction under guardedness is the currency that takes this to ω without a primitive, priced against the interval, modality, and finite-level alternatives in the metatheory's ambient-primitive policy.

## The representation discipline

**Familial first**: before writing a structure, ask what it is a family _over_, and index it by that; functions into data are the last resort, and a functional or higher-order encoding of a structure's _data_ is a hard STOP requiring design input.
The compounding payoffs: impossible cases stop being expressible (a case never written can never drift); the cost of the naive choice scales with dimension; witness disciplines become achievable by construction; equality becomes structural (inductive data has decidable equality whenever payloads do, with no function extensionality anywhere); and the index is the interface later abstraction quantifies over.
The failure mode the STOP exists for: an encoding defect described as a property of the foundation reads as settled and stops being questioned — before attributing a wall to the setting, produce the counterexample under an inductive encoding, or stop.

**Decidable equality is spiked first, never deferred.** The answer determines the representation, and the representation is the expensive thing to retrofit; a deferred decidability question is a wrong-path generator.
The spike must produce a typechecked decision procedure or a located failure with the exact stuck unification.
Two look-alike obstructions must be told apart: function-typed fields (a representation defect; carry the data) and forced-index deletion (not a defect; concentrate the debt into one injectivity lemma through a recursively computed code, close the residual reflexive equation with `UIP` on the **index type alone**, and reserve decidable equality for where a decision is actually computed).

**The witness discipline**: indices may carry the arity operation's _units_; its _multiplication_ (append, flatten, graft, substitution) never appears in a matchable index and enters instead as the inductive **graph** of the operation, a witness relation carried in constructors.
No identity-shaped constructor repeats a frame variable across its result indices — identity and diagonal cases are derived, never adjoined.
The strongest form: **index a datatype syntactically, never by its own interpretation** — a complex indexed by the fold that interprets it breaks case splitting and the termination checker at once.
And **never `with` on a recursive call** in a definition that will be reasoned about, from either side: on the definition's side write the recursive clause as an application; on the proof's side pass the scrutinee with its defining equation to a helper.
The pair — an unfolding lemma per head form, arguments-with-equations in proofs — is what makes a well-founded definition reasonable about at all.

**A family can over-determine**, and when it does the redundancy is real: `Rigid` (the effective quotient by a decidable canonicalization) is what reconciles a multi-derivation term calculus with a canonical stored form; a canonical section is never pretended free.
This is a rule about the metatheory's _presentation_, not about storage, which stays flat and tabular; the section discipline is the bridge.

## Organizing structures

**Characterize before building, at the most precise structure available, and prefer the lightest coherence burden.** Say what a thing is categorically and _define_ the instances before building — an instance you cannot fill is a hole you did not know you had (running this once found two obligations on no list).
Among characterizations that fit, prefer the one whose coherence is most decidable and least dependent on a strictness theorem: skew-monoidal over monoidal where it fits; a lax promonoidal structure (a multicategory) where two tensors mix and associativity holds only up to an inclusion of permutations; finer-than-the-literature characterizations are results to be kept, not liberties to apologize for.
The machinery inventory is open and demand-driven — build a structure against the consumer that demanded it, never speculatively.

**Weak by default; the marks go on strictness and decidability.** Everything reads as weak unless marked; every strict definition and every consumer of decidable cell equality carries a definition-site mark, because that is where collapse and the K-floor live and what a trust audit must find.
Names follow the highest-ranked source that owns them; a name is a claim, adopted where the correspondence is proved and marked candidate where conjectured.

**Package layout is by what a thing is**: generic type theory (`Prelude`), the mathematics gandr stands on (`Foundations`), gandr's own theory (`Metatheory`); within a package, split by role (base / properties / structure / examples), with headers migrating with their content.
Migrate, never duplicate: two definitions of one thing are definitionally equal, so the gate cannot see them drift.

## Coherence-cost engineering

The four-tier policy of the metatheory track, read as mechanization practice:

1. **don't generate** — a cheap decision means the witness is never built;
2. **dissolve** — one theorem over a semantic class closed under the syntax settles a whole family at once, at a cost independent of dimension; the worked instance is the arena's rigid-coherence theorem, and the same trade appears in a neighbouring mechanization as intrinsic surface embedding — choose a representation in which the structural hierarchy has no content;
3. **decide** — a normal form for the residue, polynomial in the word;
4. **generate** — off the trusted base only, verified by replay rather than elaboration; the measured blowup elsewhere is roughly an order of magnitude per dimension _in the typechecker_, which is the wall storage-layer sharing does not touch.

Supporting disciplines:

* **staged obligations**: acyclicity, tractability, and termination are separate named obligations with separate suppliers, never one monolithic convergence proof;
* **assumptions live in signatures**: where a piece will not close, discharge what closes and make the rest parameters of the smallest module that needs them — zero silent postulates, and a parameterized module must be instantiated at a concrete witness in the same change, or it is green and vacuous;
* **build the residual now**: three consecutive residuals taken rather than deferred each exposed a structural defect; a deferred residual is a claim that nothing depends on it;
* **refutable predicates need refuters**: an invariant is structural or refutable, never both in one type, and a predicate nothing refutes may be vacuous — the counterexample suite is part of the content;
* **computational pins**: where distinct data share a type (every wiring at a one-colour interface), predict normal forms by hand and pin them, because typechecking alone cannot catch a wrong construction;
* **the three-routes positioning**: to avoid a quotient, one can make the invariant intrinsic (cannot express non-instances — fatal where refuters are the content), go to higher structure (a HIT and a topologically sensitive ambient), or carry a decidable section — this development takes the third, the only one keeping a computational interpretation without either cost.

## Lessons with no other home

Four hard-won rules that belong to no single module header:

* **The lemma-list diagnostic** (the cheapest early-warning signal, checkable by reading a file's lemma list): _how many of my lemmas exist only to refute a case the encoding permits?_ A nonzero answer is the familial-first STOP firing early.
* **Check the direction you did not prove.** When writing "is" or "are exactly", check the converse; if it fails, say so and say why — a false converse is usually more informative than the theorem.
* **The h-level charge moves, it does not vanish**: demanded function-side it lands on the unit laws, graph-side on functionality; what removes it from both is the heterogeneous structural comparison, which can ignore the witness layer propositional equality has to identify.
  The companion rule: before paying for a discipline across a whole tower, build the operation and **measure** what the discipline buys — the measurement is a theorem, not an estimate — and when an interface is about to be extracted, _attempt the extraction against every instance before building any instance's obligations_; the attempt is a review technique that has found defects reading did not.
* **The `Tower` device**: the moment a coherence's intermediate object is existential (determined by neither endpoint), carry the intermediates as **fields of a record, not indices** — stated with indices the coherence is heterogeneous and needs a transport before it can even be written; packaged, the two routes compare by an ordinary equation and the whole coherence is one congruence.

## The toolchain probe matrix

Thirteen typechecked probes pin what the flag regime admits; none should be re-run to rediscover:

| probe                                                                       | result                                                                                                                                                                                             |
| --------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| any `postulate`; `--rewriting`; `--sized-types`; `--irrelevant-projections` | **rejected** under `--safe` — ruling out artifacts that depend on them                                                                                                                             |
| `--safe --cubical`; `--safe --without-K --prop`                             | **accepted** — so higher inductive types are blocked by gandr's own no-HIT policy, _not_ by the safety checker: a negotiable architectural choice with a recorded reason, like every other decline |
| small induction-recursion                                                   | typechecks under plain `--safe --without-K`                                                                                                                                                        |
| the Segal condition over a **semi**-simplicial object                       | typechecks — no degeneracies, no truncation, no HIT (Segal conditions mention only inert maps, which are injective)                                                                                |
| the augmented and full simplex categories                                   | typecheck with category laws by structural induction — no funext, no UIP, no termination pragma; hom counts pinned by `refl`                                                                       |
| symmetric species as carrier-plus-symmetry-relation                         | definable, but the fixpoint exists only for the raw unquotiented functor, whose identity type is that of ordered trees                                                                             |

The species probe's ceiling is a theorem, not an impression: an E-category promotes to an H-category only if uniqueness of identity proofs already holds [@palmgren-2019-equality-objects] — which is the theorem behind the three-routes positioning above: the decidable-section route is the only one that keeps a computational interpretation without a HIT or a topologically sensitive ambient.

## Reasoning and proof style

Equational arguments are multi-setoid reasoning chains everywhere, including `Set`-level modules (whose hom relation is the identity type, so a chain there is a `trans` ladder in the shared vocabulary); the chain layout, the four step markers (forward/backward, plain/under-a-head), the no-`trans`-inside-a-chain and no-`subst`-blob rules, and the head-binder congruence device are specified in `docs/workflow/agda.md`, with worked examples named there.
Solvers are reached for before hand proofs, on demand, quoted by hand, never reflection macros — the trusted content is an object-level function with a soundness proof, and nothing in the tree quotes or unquotes syntax; the recorded direction for a future coherence solver is the tree's own normal-form machinery instantiated at the free structure it decides.
Opacity is the default for definitions whose unfolding is a cost, with the compute surface (the carrier layer, the telescope interpretation, normal-form functions) never opaque and every unfolding block naming its computation dependence.

## Flags, gates, and scope

`--safe --without-K` on every module, with `--guardedness` infective from the coinductive carrier and accepted rather than routed around; the strict root / declared-holey-leaf split; stdlib imported directly with per-module repackaging (the facade is withdrawn); reading that would change what gets built is scheduled before the building, ranked by what it gates.
These are workflow rules owned by `docs/workflow/agda.md`; they are listed here only so this track is a complete map.

## Sub-documents

* [[proof-engineering/roadmap]] — the discipline-side backlog and owed statements.
