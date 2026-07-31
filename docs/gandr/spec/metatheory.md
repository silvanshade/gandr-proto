# Metatheory

This track owns the mathematics specific to gandr's semantic model: the Agda development under `metatheory/`, the theory crates it models (`theory-computads`, `theory-graphs`, `theory-levitation`, `theory-virtual-doctrines`, `core-sequent`), and the design record for identity, univalence, certificates, and the doctrine layer.
How structures are _mechanized_ (the ∞-graph substrate, the familial representation principle, the coherence-cost policy) is the [[proof-engineering]] track; what the Rust engine _does_ is the [[implementation]] track.
Detailed remaining work is in [[metatheory/roadmap]]; things that must not be re-opened are in [[metatheory/guards]]; known citation traps are in [[metatheory/citation-hazards]]; the full carrier record is in [[metatheory/carrier]].

Honesty gate, binding on the whole track: the ∞-end of this development is classical ∞-category theory with no formalization and none claimed; every adoption below is at the 0- or 1-truncated rung.
Claims are marked **verified** (in code or against a held source), **cited** (a published theorem gandr consumes), **conjecture** (ours), or **owed** (an accepted direction with named obligations).
Where a claim rests on a source held but not adversarially re-checked, it says so.

## The load-bearing decisions, named

Every decision below has a section in this document; the table is the index.
These names replace the letter-number codes of the retired consolidated proposal (the concordance is in [[metatheory/guards#code concordance]]).

| decision                                        | one line                                                                                                                                                                                      | section                                             |
| ----------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------- |
| **the placement thesis**                        | the decidable algebra that presents identity lives _in the codes_, not _in the shape_: identity is data, `refl` is a constructor, transport is induction on the generator                     | [[#The placement thesis]]                           |
| **two structures, two univalence statements**   | the description universe carries a layout structure and a pasting structure; each has its own identity theory and its own univalence statement                                                | [[#One universe, two structures]]                   |
| **the full-rung substrate**                     | the cell substrate carries the full generality of circuit algebras (many-in/many-out, wheels, disconnection, cut); gandr's restrictions are enforced by static analysis, never by the carrier | [[#The substrate is the full circuit-algebra rung]] |
| **one construction, two arity bases**           | the doctrine complex and the term algebra are one generalized-multicategory construction over two bases; the arity kit is a base together with the multiplication presented as a relation     | [[#One construction, two arity bases]]              |
| **ordering is a section**                       | the stored representation is ordered as a canonical linearization of a symmetric object — a section of the quotient, never a planarization of the theory                                      | [[#Symmetry, ordering, and the price]]              |
| **the parallel direction stays symmetric**      | ordering the parallel-component direction would silently void the bracket-vanishing oracle                                                                                                    | [[#Symmetry, ordering, and the price]]              |
| **canonicalization soundness is the price**     | at the circuit rung the entire cost of the ordered representation is `Rigid.canon-sound`: the representation must present the automorphism quotient faithfully                                | [[#Symmetry, ordering, and the price]]              |
| **the four-tier coherence policy**              | don't generate, then dissolve, then decide, then (last) generate off the TCB — the ordering, not a menu                                                                                       | [[#The coherence economy]]                          |
| **the nerve warrant**                           | the fully faithful nerve with Segal-characterized image at gandr's rung follows from the monad having arities at `Set`                                                                        | [[#The nerve at the circuit rung]]                  |
| **univalence is stratified fullness**           | `ua` per stratum is the inverse of a bijection someone else proved, gated on the per-degree Segal check — a theorem, never an axiom                                                           | [[#Stratified univalence]]                          |
| **the directed statement is primitive**         | the substrate is natively directed; the groupoid statement is the invertible core of the directed one, not the other way round                                                                | [[#Directed univalence]]                            |
| **certificate identity is replay-equivalence**  | two certificates are the same transformation when they replay the same; the normal form is a cost fast path, never a decidability claim                                                       | [[#The certificate algebra]]                        |
| **interchange is a witness**                    | exchanging two independent things is never an equation unless you accept losing information; invertibility of the witness is the design dial, and the strength splits by layer                | [[#Interchange, by layer]]                          |
| **no primitive without priced trusted surface** | the admissibility criterion for any ambient primitive is its trusted-surface cost, not its geometry and not merely its computability                                                          | [[#The ambient-primitive policy]]                   |
| **the decidability ceiling**                    | the word problem for structures of this genre is undecidable in general; beyond a named tractability fence, per-instance certificates are the mathematically maximal offer                    | [[#The ambient-primitive policy]]                   |

## The placement thesis

Both gandr and the cubical theories put a finitely presented algebra with a decidable word problem at the bottom of the identity story; the difference — and the decision the whole metatheory turns on — is _where_ the algebra is installed.

Cubical type theory [@cohen-coquand-huber-mortberg-2018-cubical] installs it **in the shape**: the interval is an indexing object, a path is a function out of it, and identity structure is uniform over an open universe because a kernel primitive (composition, defined by induction on the type) forces every type to be Kan. gandr installs it **in the codes**: an edit polygraph presents the universe itself, a path is _data_ — a free directed word of generators modulo a completed rule layer — and elimination is induction on that data.

Identity-as-function versus identity-as-data, with one consequence each:

* in the cubical setting, constancy of a path is a _property_ of a function, judgmentally invisible; in gandr, `refl` is a _constructor_ — the empty word — and therefore pattern-matchable;
* cubical transport computes by induction on the _type_; gandr's transport is realization computed by induction on the _generator_.

Two prior commitments, neither adopted for the identity story, compose into exactly what this placement needs: **levitation** supplies a recursable universe (one cannot compute identity by recursion over an open universe, but a levitated universe is an inductive object one can recurse over), and **polygraphs** supply a generated identity (elimination is structural, `refl` is detectable, the eliminator never consults a stuck uniform primitive).
That the design was over-determined by independent engineering decisions is recorded as evidence of its soundness, and the same over-determination shape recurs at [[#The coherence economy]].

The saturation mechanism makes the placement operational.
Raw path induction computes on `refl` and declines on every non-empty path; the repair is that instance stocks of universal properties are **modules over the path relation** — an instance carries an absorbed path, with covariant and contravariant actions and their laws — and the eliminator's value at a path is the diagonal value with the path absorbed.
This is the profunctor Yoneda correspondence: the unit universal property _is_ Yoneda, and the cubical connection square is the same extension rendered in interval algebra.
The economics differ — _wholesale versus retail_: a free De Morgan algebra makes every degenerate square exist at once, an infinite coherence machine bought with the shape, while the polygraph pays coherence per stratum as finitely many completed cells.
The price ledger of the two placements, with the verified Cubical Agda evaluator evidence and the `primIdJ` retirement chronology, is kept in [[metatheory/ambient-and-primitives#the cubical contact]].

## One universe, two structures

gandr's description universe carries **two independent structures**; they answer different questions, have different identity theories, and — stated here because the retired records never said it — **each carries its own univalence statement**.

|                      | **layout structure**                                                                                                                                         | **pasting structure**                                                                                                              |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------- |
| what it is           | the symmetric rig ${𝟙, ⊗, ⊕, δ}$ on the closed, variable-free fragment of the code grammar                                                                   | the graphical-species profile — which $(m, n)$ generators exist — and the pasting of generators along graphs                       |
| what it answers      | _where does a value's cell live in the flat run?_                                                                                                            | _what composites of generators exist, and when are two the same?_                                                                  |
| the invariant        | $"size" : "Code" → ℕ$, the cardinality homomorphism; offsets $⊗"ix" b i j = b·i + j$, $⊕"ix"^r a j = a + j$                                                  | Reedy degree $"deg" : Θ."Obj" → ℕ$                                                                                                 |
| its identities       | permutations of positions; the structural coherence family                                                                                                   | graphical maps; the Segal condition; the nerve                                                                                     |
| univalence statement | **the layout statement** (`ua-base`, directed form `ua-dir`): realization of the edit polygraph is sound, full, and sectioned — see [[#Directed univalence]] | **the pasting statement** (`ua` per stratum): the inverse of the fully faithful nerve's bijection — see [[#Stratified univalence]] |
| machinery            | the presented edit calculus, the completed rule layer, decidable rule congruence                                                                             | the nerve, the Segal condition, generalized Reedy induction                                                                        |
| firewall status      | **frozen**: the presented layout calculus is fixed at the base signature; no nerve theorem for rigs exists — see [[#The coherence economy]]                  | **open to the nerve route**                                                                                                        |

The two statements are complementary, not competing: the layout statement is about positions of a value at a fixed shape; the pasting statement is about which shapes and composites exist.
Neither subsumes the other, and no result about one transfers automatically to the other.
The finite-container example (two descriptions of "a pair of `A`", one indexing positions by booleans and one by a two-element type, identified by a decidable two-point bijection whose transport replays per position) is a **layout** claim; it is why the layout base stratum is reachable early and computes before any extensionality machinery exists.

The coherence obligation on the layout side also splits along this line: the layout statement owes the two-cell coherence of its univalence map (see [[#Directed univalence]]), while the pasting analogue — that per-stratum `ua` respect the graphical maps' 2-cells — is **owed and not yet stated**.

## The ambient — SETOID, not SET

Everything in the metatheory is built on ∞-graphs with lawless proof-relevant equivalence.
There is no `SET` here; there is `SETOID`, and set-like behaviour is bought back per object, as structure.
The mechanization discipline (why laws are cells one dimension up, what the Yoneda statement does and does not say, the landed module inventory) is [[proof-engineering]] material; what belongs to the metatheory is the one structure the model itself spends everywhere:

```agda
record Rigid (A : Set) (_≈_ : A → A → Set) : Set where
  field
    canon       : A → A                                  -- canonical representative
    canon-≈     : ∀ a → canon a ≈ a                      -- a representative is equivalent to its element
    canon-resp  : ∀ {a b} → a ≈ b → canon a ≡ canon b    -- COMPLETE for ≈
    _≟_         : Decidable (_≡_ {A = A})
  -- canon-idem and canon-sound are DERIVED; the five-field form is a constructor
```

`canon` is an idempotent on a setoid split by the elements it fixes; the splitting carries propositional equality, so it is an **effective quotient** — the setoid relation is recovered exactly as equality of normal forms.
Completeness (`canon-resp`) is what makes ordered storage a _section_ rather than a strengthening; soundness is what stops distinct objects being conflated.
The honest strength relation: every `Rigid` setoid is isomorphic in `SETOID` to a decidable-equality set (K-free, by Hedberg), and `Rigid` is _strictly stronger_ than admitting such an isomorphism, because it demands decidable propositional equality on the carrier itself — exactly what a content-addressed store has.

Rigidity is a property of the **representation**, never of the objects: the graphical category's automorphism groups contain the symmetric groups on the legs, and it is only a _generalized_ Reedy category for exactly that reason.
`Rigid` is one design decision seen four times: it makes ordered storage sound, it makes the generalized-Reedy factorization an actual function rather than an existence statement, it is what the parallel-component multiset instance needs, and it is a candidate load-bearing ingredient for univalence transfer (the elegance question of [[#Stratified univalence]]).

## The operational substrate — the polarized sequent kernel

The kernel is a polarized System-L command IL: producers, consumers, and commands $angle.l p bar.v_ε c angle.r$ as first-class arena-resident data, with the frozen call-by-push-value core [@levy-cbpv] as the source calculus and a static focusing translation between them.
Four of its properties carry the rest of this document.

* **Redexes are at the cut, so overlaps are shallow.** A rewrite cell's left-hand side is a cut between a constructor pattern and an operation frame; no rule searches a term tree.
  Critical-pair enumeration is tractable where tree rewriting needs full traversal, which is why the compositional-rewriting suite of [[#The doctrine layer]] can be run at all.
* **Consumers are first-class, so the seam is visible.** Under continuation-passing the same overlap hides behind a lambda; visibility is what makes fusion a derived 2-cell with a certificate rather than a pass.
* **Strategy is a per-cut polarity orientation.** Positive cuts fire the producer-side binder first, negative cuts the consumer-side one; evaluation strategy is an orientation choice on cells, not a global language property.
* **Multi-conclusion contexts have a home.** The linear consumer zone is where a multi-conclusion reading lives, and it is the declared growth point for the multi-output term face.

**Fusion is Squier completion on cut seams.** Surface rewrite members elaborate to oriented command cells; overlaps at cuts are bona-fide critical pairs; a budgeted completion loop synthesizes derived cells whose certificates are the pair of joining paths, differential-tested against the two-step composite and **replayed rather than trusted**.
Two limits are permanent: natives are opaque, and non-linear overlaps fan out into families rather than a single fused rule — the second is a theorem of the virtual reading ([[#The doctrine layer]]), not a shortfall.
The Squier citation is good at dimension one, where the completion loop lives; **finite derivation type fails above dimension one** (an explicit finite convergent 3-polygraph with finite critical branchings lacks it [@ara-burroni-guiraud-malbos-metayer-mimram-2025-polygraphs]), so the higher-cells lane must not assume the completion story lifts.

**Closure conversion is an in-IL rewrite** at the shift boundary; its named proof debt is that confluence of environment capture is modulo environment reordering, so the Agda face needs a permutation quotient — a `Rigid` instance, and it should be built as one.

**The kernel's polarity is a datum of the substrate's colour algebra.** gandr's producer/consumer polarity is the orientation morphism $θ : (C, ω) → {↑, ↓}$ of the carrier's palette (see [[#The substrate is the full circuit-algebra rung]]): a cut $angle.l p bar.v_ε c angle.r$ is precisely a contraction of a $c$-leg against an $ω c$-leg, and the involutive-colour duality that makes the modular-operad literature look "unrooted" is the kernel's own classicality with polarity forgotten.
Three two-valued dials must never be conflated: CBPV polarity (the palette orientation), the doctrine's tight/loose stratification, and functorial variance — they are different axes, and each has its own machinery.

**As built, the cell grammar is single-continuation.** Verified against the crate: the **cell-visible** command pattern has exactly one variant — a polarized cut with a producer half and a consumer half (the IL itself has three command forms, but primitives are the opaque host seam and by-reference jumps are outside the cell-visible fragment); the consumer pattern is a linear spine, each frame carrying exactly one return continuation (the exactly-one half is checker-enforced on the IL's destructors; the constructors' empty consumer lists are construction-site discipline, and the n-ary grammar on cut-adjacent constructs is the reserved growth point); every face and argument list is a positionally-indexed ordered sequence; the double-pushout inheritance is nominal, with no pushout complement in code — deliberately, since the term-rewriting double-category instance with discrete opfibrations and multi-sums as minimal unifiers is the right shape for a term-shaped cell store, where the graph-shaped double-pushout instances do not apply.
Three consequences: multi-output interfaces are unrepresentable today, not merely unused; within-cell ordering costs nothing to adopt, because no symmetry is present to give up; and the shift-equivalence relation of [[#The certificate algebra]] currently has empty extension, so its theory is forward-looking and a vacuous pass must never be read as a discharged obligation.
The **multi-output (destination-passing) term face is a ratified design direction** with nothing constructed; the sequent layer already carries the type shape (a consumer list on cut-adjacent constructs) while every construction site emits zero or one element.

**The localization move.** One pattern recurs: where a global gluing property fails, localize the choice and restrict the global operation.
Evaluation strategy (confluence fails; orient per cut), certificate composition (dinaturals do not compose; unconditional on the invertible fragment, acyclicity-gated on the directed band), loose composites (need not exist; virtual, as multi-sum-indexed families), and interchange (not an equation; a witness whose invertibility is the dial) are four instances of it.

## Cellular data — descriptions, cells, and computads

A datatype's description is a first-class value; generic operations are ordinary programs over descriptions, and the same artifact serves the cell layer, the matching engine, and the reflected judgment layer [@chapman-dagand-mcbride-morris-2010-levitation].
The ladder is staged and additive: (0) descriptions as host values with generic operations, faces stored untyped but checked; (1) a closed typed code universe with a trusted decoder; (2) descriptions reflected as gandr data, generic induction and the free monad as library surface; (3) full levitation.
The canonical shape is the tagged description — an enumeration of constructors times a first-order code per constructor, code grammar ${1, "var", ×, σ}$ plus additive decorations (a grade slot, an atom-abstraction code for binders, erased attribute slots).
Decidable equality of codes is load-bearing, not a convenience: it is what content-addressing interns on and what matching compares; the first-order fragment is chosen _because_ it keeps this.
Codata descriptions are the same codes under the final-coalgebra decoder, polarity-sorted from birth; intensionally this yields only weak final coalgebras, consistent with the no-η codata stance.

**Multi-output arities are bridge diagrams.** An operation $(X_a)_(a∈A) ↦ (Y_b)_(b∈B)$ with $Y_b = Σ_(i∈I_b) Π_(j∈J_i) X_(s(j))$ is presented by $A ←^s J →^π I →^t B$ and computed as $Σ_t ∘ Π_π ∘ Δ_s$ [@spivak-garner-fairbanks-2021-aggregation].
The Π-layer (one operation's named result tuple) and the Σ-layer (aggregating contributions into one destination) are different things, and the Σ-layer requires a commutative monoid on the target — unrestricted fan-in is not free wiring.
Read one level up, the bridge diagram _is the graphical-species profile_ of a generator, which is the identification [[#Stratified univalence]] runs on.

**Cells at every dimension.** The surface names sorts (0-cells), constructors and operations (1-cells), named directed rewrites (2-cells), declared coherences between rewrite composites (3-cells), with dimension ≥ 4 reserved parse-and-decline.
Names are mandatory at dimensions 2 and 3; the cell stays content-addressed and names never influence deduplication or replay.
The boundary language is four constructions — rule instantiation, identity rewrite, sequential composition, congruence in one argument position — deliberately the largest fragment whose engine reading and path reading agree; two simultaneous rewrite arguments are declined because that denotes horizontal composition, adjudicated by [[#Interchange, by layer]].
Boundaries are globular telescopes, so mis-glued boundaries fail once, at the declaration table.
**The filler ban**: the machine never adjoins a coherence cell the user did not declare or completion did not certify — a blanket filler between arbitrary parallel 2-cells would entail uniqueness of identity proofs without K. User coherences and machine tracelets are one species separated by provenance, and a declared coherence whose boundary a certificate already fills is _discharged_ — coherence computes.

**Shape signatures.** A description block in signature position presents a theory and mints no carrier; the derived signature-former maps sorts to type members, generators to operation fields, rules to path fields, coherences to iterated-path fields.
The 2-dimensional fragment has a theorem-backed home as a **cartesian double theory** with product-preserving lax-functor models [@lambert-patterson-2024-cartesian-double] [@patterson-2025-products]; the 3-cell layer sits above that literature, which is where the design already routes it.
The ∞-graph reading is executed, not speculative: the landed Agda category/functor records _are_ the carrier-general form, and the surface model former is its specialization.

**The computads-as-data hazard, scoped.** The category of $n$-computads is not a presheaf category for $n ≥ 3$ [@makkai-zawadowski-2008-computads]; the counterexample's mechanism is Eckmann–Hilton and its hypotheses — strictness, globular shapes, degenerate boundaries — are jointly required and individually unmet by gandr's pattern-to-pattern rules.
The applicable escape hatch is **non-unitality** [@henry-2019-nonunital-polygraphs]: source and target of a generator are never identities, which pattern-to-pattern rules satisfy.
Non-unitality is now a _four-times-independent_ arrival: the computad escape hatch here, the carrier's downward wiring (no cup — see the substrate section), the skew preference on coherence-burden grounds, and the coherence theorem naming units as the obstruction [@demirdilek-reiher-schweigert-2026-linearly-distributive]; polygraphs are moreover a presheaf category _only_ when non-unital.
Confirming the exact non-unitality condition against its source is cheap and still owed ([[metatheory/roadmap]]).

## The substrate is the full circuit-algebra rung

> **The generality ruling (owner, binding).** The substrate carries the **full generality of circuit algebras** — many-in/many-out arity, wheels, disconnection, and the cut — and gandr's restrictions are enforced by **static analysis, not structure**, at the tightest boundary to what gandr currently handles.
> A carrier notion that cannot express what the shape layer provides is _wrong_, not merely scoped.
> Assume today's restrictions will be removed over time.

### The rung, identified

gandr's cell substrate is the **nonunital (downward) circuit-algebra rung**: the monad is $O T^times$ on oriented graphical species $"OGS" = "GS"\/"Di"$, described in the source as _directed graphs with labelled input and output ports and port-preserving morphisms_, and its algebras are the **nonunital wheeled props** [@raynor-2026-nerve] [@raynor-2025-functorial].
The identification is structural, not aspirational: the carrier's wiring datum `Match` pairs every source with a partner — a sink (a flow-through wire) or another source (the cap, which is gandr's cut) — and no constructor pairs two sinks.
That is verbatim the **downward** condition on Brauer diagrams, and three consequences stand or fall together, each proved in the carrier rather than cited:

1. **the wiring is downward** — $"Match" Γ Δ$ is inhabited only when $Γ$ is at least as long as $Δ$, the difference paid in caps, reproducing the downward category's hom-emptiness;
2. **the nodeless loop is inexpressible** — a closed circle needs a cap composed with a cup; the cup does not exist, so no scalar ever has to be assigned to a free loop (the "problem of loops" of the unital rung never arises);
3. **composition cannot manufacture a closed component** — a composite of downward wirings is downward.

If a cup is ever added, all three go at once; do not add one to make an operation total.
Downward hom-sets are finite for all profiles with **no graph-shape hypothesis** — so hom-finiteness, previously attributed to simple connectivity, is free at this rung, and the old finiteness-motivated restriction ladder is retired.

### The carrier, as landed

The shape layer is an inductive family indexed by its interfaces — a list of corollas terminated by one wiring — with listings primary and incidence derived:

```agda
data Shape : List Ob → List Ob → Set where
  wires : Match Γ Δ → Shape Γ Δ
  node  : (A B : List Ob) → Append B Γ Γ′ → Append A Δ Δ′ → Shape Γ′ Δ′ → Shape Γ Δ
```

Landed and green on this carrier (the full record, with each theorem named, is [[metatheory/carrier]]):

* **the cut** (`cap` on `Match`): source-to-source pairing, with the flow-through fragment named by the `CapFree` predicate;
* **the merger** $⊠$ (`merge`, derived, not a constructor): the parallel composition of shapes, with whiskering falling out as the merger at an identity operand — definitionally, which is the strongest available evidence the operation is right;
* **the edge listing**: one entry per pair the wiring makes, correcting a half-edge/edge mis-identification the cut exposed; undirected predicates (`Connected`, `Acyclic`, `SimplyConn`, `Walk`) hold on every shape and mention no polarity, directed predicates (`Arc`, `WheelFree`, `Ranked`) take a polarity and are uniform in it;
* **the palette**: the colour involution $ω$ with the orientation $"pole"$, `cut-oriented` reading a legitimate cut's direction off the poles, and the theorem that one self-dual colour admits _no_ orientation — the free compact closed category on one self-dual object appearing in the carrier as an empty type;
* **the merger's incidence theorem, both directions**: no edge of a merge joins the two operands, and each operand's own adjacencies survive — so a merge of two connected shapes has **exactly two components, and they are the operands**.
  Disconnection is what the substrate says, not what the engine arranges.

Grafting and merging are total and **do not preserve the predicates**: connectivity is a predicate on objects, so any two shapes compose and cell-ness is checked of the result, with the counterexamples (a graft that reconverges; a merge that disconnects; a self-gluing that closes a wheel) exhibited and refuted in the tree.
The predicates need their refuters: an invariant can be structural or refutable, never both in one type, which is why a "generated cell" variant is deferred to the pasting layer as an adequacy pair rather than adopted as the carrier.

As built, the `Cell` record still demands simple connectivity — the dioperad fragment — and this is the **one remaining carrier restriction**; deleting it (or replacing `Cell` with a family of carried predicates) is an accepted direction under the generality ruling, scheduled in [[metatheory/roadmap]].
What each dropped restriction buys in the _derivation_ dimension is named there too: many-out is multi-conclusion derivation (the ratified term face), disconnection is **concurrency** (parallel independent rewriting arriving in the doctrine dimension), wheels are cyclic derivation (the completion loop's fixpoints).

### Decoration — what belongs on the substrate

A decoration that is **structure on the colour set** — the colours themselves, and polarity via the palette — is a carrier _parameter_: the undirected layer never mentions it, the directed layer is uniform in it, so every substrate theorem holds at every such decoration by quantification, with no transport theorem needed (mechanized; and the same mechanism is the source's own — coloured circuit algebras are the same monad restricted to the coloured species [@raynor-2026-nerve]).
A decoration that is a **value on the shape's parts** — grading, genus, operation symbols at vertices — genuinely changes the objects, and those rows still owe a transport-shaped warrant.
The placement test for any cell datum: does it assign values compatibly with graph substitution, and should the labelled objects inherit the substrate's theorems?
Both yes: it is a decoration and belongs with the shape.
Either no: it belongs to another layer — hole/linearity metadata to the matching layer, rewrite orientation to the computad layer, certificates to the certificate layer, and nominal sharing deliberately _not_ entrenched as a label because the accepted direction is that sharing should become a wire.

### The naming hazard, kept where a reader meets it

**Wiring, not Feynman.** This carrier is a circuit algebra in the Bar-Natan–Dancso sense (defined by non-planar wiring diagrams) on the nonunital rung; it is **not** a Feynman category (Kaufmann–Ward's formalism, a different object), and Joyal–Kock's _Feynman graphs_ name the shape half only.
Two nearer neighbours are explicitly not this object either: the _operad of wiring diagrams_ uses hierarchically nested boxes with ports rather than a matching datum — a different object under the same word — and the double-operadic _undirected_ wiring diagrams are cospans of finite sets, admitting arbitrary merging, so gandr's downward wiring (every sink hit exactly once, no cup, the nodeless loop inexpressible) sits strictly **below** that operad.
Wiring's ambiguity is between two formalisms of one idea; Feynman's is between two different objects, so importing the wrong one is being told something false.
The translation lemma between the carrier's presentation and the graphical-species presentation of the source [@kock-2016-graphs-hypergraphs] is a known-owed obligation ([[metatheory/roadmap]]).

## One construction, two arity bases

The doctrine complex ([[#The doctrine layer]]) and the term algebra are one generalized-multicategory construction instantiated at two **arity kits**, and everything above the base sphere is shared globular code: multi-ary at the base, globular above.
Multi-arity is needed at exactly one dimension — the cell shape; rules and coherence fillers above it have parallel _pairs_ as boundaries, which is what a globular coboundary already is.

The corrected statement of what a kit is (the retired record's "two arity monads" diagram was imprecise exactly where the many-out content lives):

> **An arity kit is a base together with the monad's multiplication presented as data.** The kit's carrier is the base category's edge datum ($"Step"^* : "Ob" → "Ob" → "Set"$ for the linear kit; $"Shape" : "List Ob" → "List Ob" → "Set"$ for the circuit kit); its concatenation is the multiplication's underlying operation; its `Cat` is the multiplication's **graph**, carried as a first-order relation; its `Same` is the heterogeneous comparison that graph needs.
> A generalized multicategory is $C_1 → T C_0 × C_0$ in a base $E$, and the many-out content lives in $E$, never in an outer application of $T$ — so the two kits are not two endofunctors of one category, and "compose the monads" is not even well-typed as posed.

| kit         | base $E$                     | monad                                                                                                                          | its generalized multicategories                                               | its algebras                                                      |
| ----------- | ---------------------------- | ------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------- | ----------------------------------------------------------------- |
| **linear**  | graphs (objects and edges)   | the free-category monad                                                                                                        | **virtual double categories**                                                 | categories                                                        |
| **circuit** | (oriented) graphical species | $T^times$, itself the composite of the merger monad over the contraction monad by a free distributive law [@raynor-2026-nerve] | the circuit-shaped doctrine (the accepted direction for the doctrine's cells) | **nonunital circuit algebras**; oriented: nonunital wheeled props |

The old licence — "both arities are cartesian because the symmetric group acts freely" — is **dead at the circuit rung and replaced by two facts**:

* **for the nerve**: $T^times$ **has arities** at `Set` (the graphs inside graphical species), which is the hypothesis the abstract nerve theorem consumes [@berger-mellies-weber-2012-arities]; cartesianness is not on that chain;
* **for the carrier**: Burroni–Leinster-style generalized-multicategory theory wants a cartesian arity, and gandr buys it by working over the **ordered representation**, where the monad is cartesian; the symmetric-group quotient is not avoided but _relocated_ into canonicalization soundness.

So the whole price of "one construction, two arity bases" at the circuit rung is one obligation: `Rigid.canon-sound` for shapes.
The construction's own atomicity does not generalize — the linear kit's multiplication is one structural recursion with one inductive graph, while grafting is a composite of nine operations each of whose graphs threads the next — and this asymmetry is why the arity-interface _record_ is still unwritten: two of its fields have no inhabitant in the circuit kit.
A **universe-style presentation** of the arity layer (codes, an interpretation, an arity former with an interpretation equivalence, a representation map from equivalences to code identifications) is the evaluated candidate for that record: it relocates the nine-relation obligation into a code former plus coherence laws, dissolves the unit-law asymmetry, and merges `Rigid` into the arity structure as the representation map.
The published parameterization varies the _symmetry_ axis and fixes the arity-shape former at a dependent sum (one-output tree grafting), so extending it to circuit algebras means **replacing the former, not instantiating the parameter** — and the deciding question, whether the graph former's coherence laws stay finite, is a half-day hand computation scheduled in [[metatheory/roadmap]] together with its control experiment at the linear kit.

Two senses of "arity" are in play in the literature and must be kept apart at every citation: a **monad with arities** (a property relative to a dense subcategory, consumed by the nerve theorem) and an **arity monad** (the shape of a cell's source, consumed by the carrier, wanting cartesianness).
A strongly cartesian monad has canonical arities, so the second implies the first — but at gandr's rung and base the implication is unavailable in the needed direction, and the two senses come apart: sense one is available at `Set` and carries the nerve; sense two fails at `Set` and is bought back by ordering.
Cartesianness claims in this literature must always be read with their **base**: the circuit monad is "strongly cartesian" in the ∞/groupoid-based framework [@chu-haugseng-2021-segal] where symmetry quotients are homotopy orbits, and that does not transport to `Set`, where the monad takes honest coinvariants for graph automorphism groups and an explicit ported pullback counterexample bites.

## Symmetry, ordering, and the price

**Σ-freeness is a property of the representation, not of the rung.** At the circuit rung the symmetric group does _not_ act freely, constitutively: graphs with cycles or parallel edges have nontrivial boundary-fixing automorphisms, the monad quotients by them, and the contraction's semantics _is_ that quotient.
At gandr's ordered representation Σ-freeness is true _by construction_ — and the reconciliation is published: graphical structures defined over ordered graphs are Σ-free by construction and are thereby a **different notion** from the classical symmetric ones [@batanin-berger-2017-polynomial] [@chu-hackney-2021-rectification].
Nothing gandr needs rides on Σ-freeness of the rung: the nerve runs on arities, which tolerate automorphisms.

> **Ordering is a section.** Symmetric objects, symmetric algebra, ordered representation: the stored form is a canonical linearization chosen for storage — a section of the quotient, never a planarization of the theory.

**The identification-sorting test** (proved in the carrier, one operation at a time): an identification that comes from the _presentation_ — which of two ports is named first — is already quotiented by the canonical wiring and is free, by `refl`; an identification that comes from a genuine _graph automorphism_ — the swap of two parallel components — cannot be expressed by an ordered representation and lands on `canon-sound`.
The cut's port symmetry is the first kind (paid three times, in three currencies, none of them canonicalization's); the merger swap is the second kind — it is **false on the nose** for the ordered carrier, as it must be, because the vertex order is representation content, and the identification of isomorphism _classes_ in the source is exactly what `Rigid.canon` owes.
Apply this test to every identification the literature offers.

> **The parallel direction stays symmetric.** Within-cell order (ports, positions, arguments) is free — the as-built grammar has no symmetry to give up.
> Ordering the parallel-component direction would be a silent catastrophe: the certificate normal form is a disjoint union of primitives under a **symmetric** monoidal structure; symmetry gives cocommutativity, cocommutativity gives the enveloping-algebra theorem, and that licenses the bracket-vanishing oracle of [[#The certificate algebra]].
> The ordered-forest variant is monoidal but _not_ symmetric monoidal, and adopting it would break a theorem the engine depends on while looking like a strengthening.

With the merger now an _operation on the carrier_, the symmetry moves one layer: the merger is not commutative on the ordered representation and cannot be; cocommutativity is a statement about the quotient, so the bracket oracle now depends on `canon-sound` — the same place everything else relocated to.
The sharpest form of that relocation is objects-level, stated with the duoidal identification in [[#Interchange, by layer]]: canonicalization's job is the passage from lists to multisets at the objects of the interface category, which is what makes the parallel tensor symmetric at all.

A naming hazard lives exactly here and is easy to import silently: **two structures in the tree are called `Rigid`** — the canonicalization record on a setoid (this section's, in `Gandr.Rigid`) and the extent-preserving offset-fixed _arena map class_ (in `Gandr.Arena.Structure`, the subject of the dissolution theorem and the factorization-system split).
They are different structures with one diagnosis in common — in both, the representation carries structure the semantics cannot see, and rigidity certifies it does not leak (bracketing in one, ordering in the other) — and a rename decision is owed before a third `Rigid` appears ([[metatheory/guards#Name collisions — read the definition, not the section title]]).

**`canon-sound` has a published shape and an external warrant.** The recipe is a normal form on the _construction term_ over the carrier's own two operations (merger and contraction): push permutations to the outermost operation, totally order the tree monomials, take the unique minimal one [@stoeckl-2024-koszul].
It is not canonical graph labelling in the nauty sense.
One condition carries it to `Set` and is checked before leaning: the defining relations must rewrite monomial-to-monomial rather than to sums.
The rigidify-then-transfer move itself is a published theorem (Koszulity transfers along the forgetful functor from groupoid-coloured to discrete-coloured modules), which is the best available warrant for the shape of gandr's whole representation discipline; and canonicity-under-a-tractability-fence (see [[#The ambient-primitive policy]]) is the external theorem behind `canon`'s existence.

## Representation — decidability, the arena, and the layout calculus

**Decidable equality comes from edge-determination, not from discreteness.** A shape homomorphism is determined by its action on the finite edge set; with actions carried as tabulated data, pointwise agreement implies propositional equality by ordinary induction, hom-sets are finite, and morphism equality is decidable.
The determination lemma needs a no-isolated-vertices hypothesis whose necessity is exhibited (the arity-zero corolla is a legitimate cell shape and an isolated vertex at once); equality of _shapes_ is decided outright, with the residual h-level condition closed by uniqueness of identity proofs on the **colours alone**, supplied constructively by their decidable equality; decidable equality at _isomorphism_ remains genuinely open pending an enumeration.
This replaces the essential-discreteness argument entirely, needs no planarity, and is what makes per-degree naturality checking finite in [[#Stratified univalence]].

**The flat arena is a published object.** $"size" 𝟙 = 1$, $"size"(c ⊗ d) = "size" c · "size" d$, $"size"(c ⊕ d) = "size" c + "size" d$, values indexed by $"Fin"("size" c)$, offsets $⊗"ix" b i j = b·i + j$ and $⊕"ix"^r a j = a + j$: the cardinality homomorphism, implementing the bridge diagram's three-step evaluation ($Σ_t ∘ Π_π ∘ Δ_s$) with the indexing made arithmetic.
The arena _is_ the bipermutative category whose objects are the natural numbers and whose morphisms are the symmetric groups, with the row-major index formula verbatim [@yau-johnson-2015-props]; its strictification theory fixes exactly how far strictification reaches — both associators, all unitors, one unit-side symmetry, and one distributor become identities; the two symmetries and the **left** distributor survive (the right distributor is already the identity on offsets).
The arena fixes a canonical layout that truncation-based treatments deliberately forget: gandr gains computable offsets and pays the visibility of the choice — ordering-as-section one layer down.
The precise proposition locator for the bipermutative identity is **unverified** (its adversarial check died mid-run); do not cite it outside the repository — [[metatheory/citation-hazards]].

**The coherence verdict.** Is the structural coherence family a tree-shaped edit calculus must impose mathematical or presentational?
Presentational, in two halves, and the proof is landed:

* _the hierarchy dissolves as one theorem, not a family_: a rigid arena map — extent-preserving (`ext`) and offset-fixed (`fixed`) — composes and whiskers to rigid maps, and any two rigid words with a common source agree at value grade.
  The associativity and unit generators are rigid, so **every** diagram built from that hierarchy commutes, at every code, with no cell imposed; the pentagon and triangle are instances.
  No uniqueness of identity proofs, no transport: a coherence cell here is an equation between _functions_, so the recast stays clean without K. Dissolution's cost is one theorem plus four closure lemmas, **independent of dimension** — which is the cost claim that matters, against a generated family whose members grow exponentially;
* _what carries content is proved directly_: the two symmetries and the left distributor are not rigid; their obligations (the sum hexagon, distributor naturality) are discharged by induction through the arena's computation rules.

The **completeness half is declined, with a reversal condition** — see [[metatheory/guards#the declined completeness half]]. gandr consumes only soundness (congruent words realize equally, cheap _because of_ the dissolution theorem); the engine's normal-form test is a decidable under-approximation of replay-equivalence, and a normal-form-equal, replay-divergent pair is a kill signal, not a soundness hole to close by theorem.
The residue after dissolution is the symmetric-group word problem in the groupoid alphabet and the full transformation-monoid word problem in the directed one; neither is owed, because both are the completeness half.

**The arena's directed generalization is warranted, priced, and shaped.** The arena's morphism class is bijections _by construction_ (its published identity has morphisms only at equal extent), so admitting the directed alphabet's one-way generators is a request to enlarge the morphism class, and the design question is _which rung of the classical ladder the arena sits on_: offset-fixed (trivial word problem, the dissolution theorem) ⊂ monotone (the simplex category — simplicial identities, classical and convergent, epi–mono factorization as the normal form) ⊂ symmetric (Coxeter) ⊂ all functions (the transformation monoid).
Computed from the offset formulas: three of the four one-way generator classes (projections, diagonals, injections) land in the monotone rung; the codiagonal alone forces the transformation monoid — co-cartesian structure on ordered sets is not order-preserving.
The decision of record: **characterize as the clone, build as the factorization system** — `Rigid` splits as `RigidMono ∩ RigidEpi`, the split _explains_ the existing record rather than displacing it, factorization systems are already the development's idiom at five layers, and building the split _is_ building the simplex category's epi–mono normal form, so the decision procedure arrives with the construction.
The warrant is **soundness**, not completeness: one-way generators fall outside the rigid class definitionally (they change the extent), so without the enlarged class every one-way coherence obligation returns as a per-generator grind — which is precisely what the dissolution theorem exists to prevent.
The published decomposition of the target ladder (planar / symmetric / clone as three monads on `Cat` related by distributive laws) is [@curien-2012-operads-clones]; scope the build by the directed rule layer's actual cell list, not as an open-ended redesign, and decline the clone-as-morphism-class rebuild with that reason.
Prior prices for the directed coherence pass were quoted against the tree presentation the arena has since replaced, and are **pending re-quote** — [[metatheory/roadmap]].

**The layout firewall.**

> The presented layout calculus is frozen at the base signature.
> Every richer former obtains its path structure compositionally — per-former closure theorems plus description-structural congruence — never by new generator letters and new coherence cells.
> A proposal requiring a new cell class is a STOP with a written cost model.

The cost attribution: the expensive part of a presented calculus scales with the generator and cell alphabet, not with the number of types; growing the universe while keeping the presentation fixed costs nothing new, growing the presentation re-runs the campaign, and binding structure would escalate the word problem qualitatively.
The nerve route does not lift this firewall: the layout universe is a **rig** — two monoidal structures and a distributor — and no nerve or Segal characterization for rig categories exists; whether the free-rig monad is cartesian is an open research question with no published answer, filed rather than bet on.
The early-warning instrument transfers with the firewall: when a closure theorem for a new former resists, first look for a conserved quantity separating the two sides — if one exists, the failure is specification-level, not proof-level.
Per-former routes (sums and products in the base stratum; nested codes by structural congruence; finite-fibre Σ by a closure theorem with no function extensionality; infinite positions and recursive codes leaving the base stratum for tabulated and bisimulation-shaped certificates; binder fields a STOP pending their own design pass) are carried in [[metatheory/layout-and-coherence]].

## The coherence economy

The failure mode this architecture exists to avoid has been observed in the wild: a published, well-engineered coherence-term generator reports type-checker memory exceeding workstation capacity, with artifacts growing roughly seven- to twelve-fold per dimension, and ships pre-computed artifacts because regeneration is amortizable while **elaboration is the wall** [@benjamin-markakis-offord-sarti-vicary-2025-naturality].
The trigger is architectural and single: _materialize a coherence witness, then have a kernel check it_. gandr's exposure is therefore one named condition — a coherence witness becoming an Agda term in the gate root — and the composition law of the wiring calculus is the live instance of it: the four-layer exchange coherence's cut half is **closed** (proved, no hypothesis, no parameter), and the wire half is the ladder in progress ([[metatheory/roadmap]]).
A second measured instance from the opposite direction: the one published mechanization of polygraphs in a proof assistant reports that higher inductive types that do not compute "are not well-suited to intricate uses", could not prove functoriality of the free construction in the cubical setting, and implements zero rewriting or coherence content — the wall again, reached via HITs rather than term generation.

**The four-tier policy** (an ordering, not a menu):

1. **don't generate** — decide cheaply and skip the witness: the bracket oracle, the acyclicity gate, the normal-form fast path;
2. **dissolve** — one theorem over a closed semantic class, cost independent of dimension: the rigid-coherence theorem, and its predicted extension to the embedding fragment of the directed alphabet;
3. **decide** — a normal form for the residue: the epi–mono factorization, Coxeter, the convergent fragments; polynomial in the word, never exponential in the dimension;
4. **generate** — the irreducible remainder only, **off the trusted base, verified by replay rather than by typechecking**.

The four tiers were adopted for four unrelated reasons (parallel-replay cost, a presentational-versus-mathematical spike, content-addressed storage, certificate identity), and that they compose into exactly this defence is over-determination from independent decisions — a stronger signal than a defence designed for its threat, and the second recorded instance of that shape (the first being levitation-plus-polygraphs composing into the cubical complement).
Four answers to a soundness-side coherence family now exist in the literature and the tree — dissolve by semantic invariant; tame by a DSL that makes each instance a one-liner; truncate (declined: untruncated protypes are the point); generate-and-check — and the middle two share one architectural slot: _a tool producing candidate terms or counterexamples, verified by already-trusted machinery, adding no trusted surface_.

The optimism is falsifiable rather than atmospheric, because gandr has an **arity law for its own coherence debts** where the blowup's victims have only measurements: the arity of a coherence debt is the positions the operations in play _thread_, plus the head they meet; whiskering and merging thread nothing and contribute nothing however large their blocks.
The law retrodicts both known debts exactly, yields boundedness, and names its own discriminator: a fifth layer needs two cuts to commute, so the next unit's interchange is the scheduled test.
The strongest available finiteness argument is now group-theoretic rather than inductive: the object the coherences act on is a tower of nested insertions with the symmetric group acting on its layers (the braid relation plus involutivity — the _symmetric_ group's presentation, a proved statement, not a guess), so both routes of a higher coherence are reduced words for one permutation, and the ladder's members are **consequences of the braid relation and far-commutation, not new generators**.
Its three falsifiers (the graph former's coherence laws fail to stay finite; the residue after the epi–mono and Coxeter decompositions cannot be decided cheaply — the codiagonal's transformation monoid has no register row and no formalized rewriting twin; the interchange does need two cuts to commute) are carried with it in [[metatheory/roadmap]].
This is the architecture's central wager, presented as a bet with a named test, not as a background assumption.

## The nerve at the circuit rung

The pasting side's completeness warrant is a fully faithful nerve with a Segal-characterized essential image, **at gandr's own rung**, obtained by instantiating the abstract nerve theorem rather than by restriction:

> The nonunital circuit-algebra monad $T^times$ **has arities** $"Gr" ⊂ "GS"$ at `Set` [@raynor-2026-nerve], and a monad with arities has a dense graphical category $Θ_(T^times, "Gr")$ (the bo-ff factorization of $"Gr" ↪ "GS" → "GS"^(T^times)$) whose induced nerve is **fully faithful with essential image exactly the Segal presheaves** [@berger-mellies-weber-2012-arities].

Read the warrant precisely: arities deliver _both halves_ — full faithfulness and the Segal characterization — so gandr's citation is this pair, **not** the source's headline theorem, which is stated for the harder _unital_ rung (where the monad does not have arities and the proof passes through a monad decomposition).
Two cautions travel with the warrant.
A Segal characterization is **not** a completeness condition — the ∞-analytic-monads line needs a further localization at fully-faithful-and-essentially-surjective maps beyond its Segal equivalence, so "Segal-characterized image" must never be silently read as "complete"; and for wheeled properads and their neighbours the published pattern is that the graph category must be _enlarged_ before the nerve is fully faithful, with the sources' own warning that neither case is a straightforward application of existing theory.
Independent corroboration that cartesianness is not on the chain: the relative-monad nerve theorem's only hypothesis is **density** of the root [@arkor-mcdermott-2024-nerve], with an explicit non-density counterexample showing the hypothesis cannot simply be dropped.
The Segal condition is **strict** — a limit over the graph's elements, a bijection, not a weak equivalence.
Monad decomposition is load-bearing here but _inside the term dimension_: the circuit monad decomposes as the merger monad over the contraction monad with a free distributive law, and decomposition is the general mechanism that creates arity candidates where a monolithic monad lacks them.

What this retires: the old restriction chain (properadic nerve restricted along the dioperad inclusion), its residual admissibility risk, and its open questions — gandr's route needs no restriction along a subcategory inclusion at all.
What it leaves owed, both scheduled:

* **the oriented-slice transfer**: the arities statement is made for $T^times$ on $"GS"$ and not restated for gandr's oriented $O T^times$ on $"OGS" = "GS"\/"Di"$; the transfer along the slice is routine (the slice equivalence is used in the source's own proofs) but unwritten — a paragraph, not a programme;
* **the presentation of $Θ_(T^times, "Gr")$**: the theorem needs only the category's existence; the _mechanization_ (degree, the degree-raising and degree-lowering subcategories, factorization, decidable morphism equality, the per-degree Segal check) needs its morphisms concretely, which the source does only for the unital case.

Publication status travels with each claim: the nerve-theorem paper is a **preprint**; the functorial-combinatorics and distributive-law papers are published [@raynor-2025-functorial] [@raynor-2021-graphical]; and the held arXiv version of the distributive-law paper renumbers against the published version — the locator table is in [[metatheory/citation-hazards]].

```typst
#import "@preview/fletcher:0.5.8": diagram, node, edge
#diagram(
  spacing: (14mm, 10mm),
  node((0,0), [signature — a circuit-shaped computad of generators]),
  node((-1,1), [$frak(D)_ω$ — the doctrine complex \ (derivation dimension)]),
  node((1,1), [free circuit algebra \ (term dimension)]),
  node((-1,2), [split cartesian fibrational VDC \ (the equipment)]),
  node((1,2), [Segal presheaves on $Θ_(T^times,"Gr")$]),
  node((0,3), [the CwF — with $U_n$, El, Equiv, and per-stratum ua as a universe inside it]),
  edge((0,0), (-1,1), "->", [free at the linear base], label-side: left),
  edge((0,0), (1,1), "->", [free at the circuit base]),
  edge((-1,1), (-1,2), "->", [internal-language semantics (candidate)], label-side: left),
  edge((1,1), (1,2), "->", [nerve: fully faithful + Segal]),
  edge((-1,2), (0,3), "->", [project: restriction $arrow.r.double$ substitution, tabulator $arrow.r.double$ context extension, \ hom protype $arrow.r.double$ Id, extension along the conjoint $arrow.r.double$ $Π$], label-side: left),
  edge((1,2), (0,3), "->", [a type over the site]),
)
```

The two columns are not two theories to reconcile; they are one free-then-nerve pattern at two arity bases, meeting twice: at the bottom, where the term column's output is a _type_ in the derivation column's category-with-families, and at the loose arrow between the two context families whose tabulator is function extensionality ([[#The doctrine layer]]).

## Stratified univalence

The pasting-side statement in one line: **internal univalence is stratified fullness** — at each certified stratum, every semantic coherence cell is the image of a marked syntactic one — and a fully faithful nerve says exactly that.

### What a code is

A description code is **not** an algebra; the identification is one level down.
A description is a **graphical species** — a presheaf on the tiny category of finite sets, bijections, and the involution pairing input legs with output legs; that species is the _signature_, its terms are the _free algebra_ on it, and the Segal condition is what says a presheaf on the site is such an algebra:

```agda
Sig   : Set₁                                        -- descriptions, as graphical species
Terms : Sig → Presheaf Θ                            -- the free Segal object
El    : Sig → Θ.Obj → Set ; El S G = Terms S .₀ G   -- values at a shape
```

Equality of species is far smaller data than equality of algebras; the base category is tiny and decidable; and it matches what a description is — a description describes _data_, not structures with composition.
The bridge-diagram profile of a generator is exactly the species profile, so the description layer and the pasting layer meet without translation.
This identification is the first thing to test (its falsifier: a description needing dependency or indexing the base category cannot express); everything below assumes it.

### The site, the strata, and the fuel are one object

$Θ$ is a **generalized Reedy category** — generalized precisely because its objects have nontrivial automorphisms:

```agda
deg    : Θ.Obj → ℕ                                  -- Reedy degree
Θ⁺ Θ⁻  : SubCategory Θ                              -- degree-raising / degree-lowering
factor : (f : G ⟶ H) → Σ[ K ] (Θ⁻ G K × Θ⁺ K H)     -- unique UP TO ISO
```

Stratum $n$ is the shapes of degree at most $n$; the universe at stratum $n$ is the codes whose terms are supported there; **fuel is the degree** — a natural number decreasing along degree-lowering maps, so induction on it terminates structurally.
"Unique up to iso, not up to unique iso" is where the automorphism groups sit, and it is exactly what `Rigid` discharges: canonicalization turns the factorization into an actual function.
Reedy theory hands over the staged construction — a presheaf is built degree by degree through latching and matching objects with automorphism-equivariance at each stage; the per-degree new data is exactly the delooped automorphism groups [@haine-ramzi-steinebrunner-2025-reedy], with the classical bigluing results as the 1-categorical citation per that paper's own direction.
**Staged certification is Reedy induction.**

### Equivalence as finite, checkable data

```agda
record Equiv (a b : Sig) : Set where
  field
    at  : (G : Θ.Obj) → El a G ≃ El b G       -- a bijection per shape
    nat : naturality of `at` in G             -- finitely checkable per degree
```

Two properties make this a certificate rather than a proposition: naturality up to degree $n$ is a finite conjunction of decidable equalities (hom-sets finite, morphism equality decidable), and the certificate is generated, not enumerated — determined by its behaviour on the degree generators, so the stored object is small.
Proof relevance is operational (two distinct certificates are two distinct artifacts), and composition follows the two-mode discipline of the localization move.

### Per-stratum univalence is a theorem, with two named obligations

```agda
ua      : Equiv a b → Terms a ≅ Terms b     -- an Equiv IS a natural iso
ua-desc : Terms a ≅ Terms b → a ≅ b         -- descent: full faithfulness + corolla restriction
ua_n    : Equiv a b → Id (U n) a b          -- gated on stratum-n Segal certification
```

`ua-desc` is _not_ bare full faithfulness: the nerve gives an isomorphism of **free algebras**, and reflecting it to the generating **species** is a further step — restrict the natural isomorphism to the corollas, whose values the Segal condition says determine everything.
That corolla-restriction lemma is small, plausibly easier at the nonunital rung, and it is a distinct obligation that must not be discovered at implementation time.
The gate is real and stays: per-stratum `ua` exists exactly where that stratum's Segal condition has been checked; an uncertified stratum genuinely lacks it, which is the design working.

Two further obligations sharpen the statement:

* **the two-cell coherence**: a univalence map must be a _typoid function_ — equivalent equivalences go to equal identifications ($"Ua"^2$) [@petrakis-2022-typoids]; the pasting-side analogue (respect for the graphical maps' 2-cells) is unstated and owed;
* **the target discipline**: the statement must **not** target the ambient identity type — over decidable codes Hedberg collapses it, so the groupoidal form says nothing exactly where gandr lives; the realization-as-functor direction is forced from the start, which is one of the three forcings of [[#Directed univalence]].

### Transport, and the two cost measures

Transport at a _known_ shape is free — the component of a natural transformation; **fuel pays for finding the shape**, descending the Reedy degree and replaying the certificate at each shape visited.
Two ℕ-valued measures do two jobs and must not be conflated: `size` bounds value-replay cost at a fixed shape (the layout structure's measure); `deg` bounds shape-search cost (the pasting structure's measure).
Cost is first-class and computable: transport at degree $n$ costs a statically-known number of naturality checks.

### Where the base stratum ends

The base stratum is "finitely supported in Reedy degree"; recursive codes leave it because their terms are not supported in any finite degree.
At the colimit universe the check no longer terminates, the certificate becomes coinductive, transport becomes corecursive and needs _productivity_ — the job of continuous normalization, whose repetition constructor is fuel made syntactic [@aehlig-joachimski-2005-continuous].
Boundary, restated so nobody imports it downward: **productivity is not decidability**; the corecursive layer belongs to certificates and codata, never to the kernel's conversion check.

### Univalence beyond the code universe — transfer, structures, repair

Three separate things are wanted; only the first is the nerve's.

1. **The code universe is univalent** — the construction above.
2. **The ambient diagram model satisfies univalence and function extensionality**, so the other formers behave.
   The transfer theorem exists for _inverse_ diagram categories [@shulman-2015-inverse-diagrams]: univalence, funext, and the universe tower all transfer to Reedy-fibrant diagrams, with admissibility cheap for a finite-graph site.
   The gate is that the theorem's mechanism needs Reedy and injective structures to coincide, which holds when the index is **elegant** in the Bergner–Rezk sense — and $Θ$ is _generalized_ Reedy, so the theorem does not apply on the nose.
   _Is the graphical category elegant?_ is the single sharpest open item on this side; if elegance fails for $Θ$ but holds for its rigidified form, `Rigid` is load-bearing for univalence transfer as well — the fourth appearance of one decision.
3. **Every structure layered on top satisfies its own univalence principle** — the general machine is the Univalence Principle [@ahrens-north-shulman-tsementzis-2021-univalence-principle]: structures as Reedy-fibrant diagrams over inverse-category signatures, indiscernibility via a joker element, univalence as indiscernibility-coincides-with-identification, generalizing Rezk completeness.
   Two structural caveats: it is written in two-level type theory (a strict layer gandr does not currently carry), and its signatures are inverse where gandr's graphical signature is not — the chapters on higher and enhanced categories are where the automorphism question would recur.

If a structure fails its univalence condition, the repair is a **Rezk completion**, available in gandr's own enrichment setting [@vanderweide-2024-enriched-rezk]; note the exact relation to the existing device — `Rigid` rectifies by chosen normal forms (a section), Rezk completion rectifies by adjoining identifications (a quotient) — the two directions the ambient section already distinguishes.
The layered mechanization method for proving a structure as complicated as an equipment univalent (displayed layers, each proved univalent) is [@vanderweide-rasekh-ahrens-north-2023-univalent-double].
The synthetic mirror supplies vocabulary: per-stratum `ua` is a _local_ univalence (Rezk-type) condition, and the landed directed eliminator is the semantic shadow of the dependent Yoneda lemma [@riehl-shulman-2017-synthetic] — but the synthetic route axiomatizes a directed interval, which the ambient-primitive policy prices rather than adopts.

## Directed univalence

The directed statement is the general case and the groupoidal one is its invertible core — forced three times over, not chosen:

1. **the structures are natively non-invertible** — the path substrate's generators are oriented schemas with invertibility as per-generator _overlay data_ (a designated inverse plus round-trip evidence); the shifts are non-invertible by design; the directed band composes only under an acyclicity gate.
   The groupoid path protype is the free involutive doubling of the evidence-invertible restriction with cancellation adjoined one dimension up — a localization of a restriction, neither a sub-protype nor a quotient — and no construction recovers the directed statement from the groupoid one;
2. **function extensionality needs both dimensions in one equipment** — funext is tabulation of a pointwise certificate, a tabulator is binary-ended, so the position side and the derivation side must be objects of one equipment, and the derivation side is directed;
3. **the groupoidal statement is degenerate at gandr's own codes** — Hedberg over decidable codes collapses any univalence map into ambient identity, so the groupoidal form is vacuous exactly at the base stratum.

**The statement of record** (the fenced directed statement; the maximal and lax alternatives are priced and declined in [[metatheory/directed-univalence]]): over the recursion-free description fragment, with the certificate alphabet fixed to the leaf-natural one-way stock, the realization of the directed edit polygraph is

* **sound** — every generator schema names a translator with replay evidence; paths realize by composition, unconditionally, because string-shaped composites are structurally loop-free;
* **full** — every leaf-natural one-way certificate is replay-equal to a realized positive word, at the unit-plus-restriction fragment;
* **sectioned** — β at replay-equivalence, η at the directed rule congruence, never code equality;
* **core-coincident** — the comparison from the groupoid statement into the invertible core of the directed one is a bijection at the stated grades; this is genuinely new work, because invertible realizations arise from non-invertible letters, and it needs the directed rule layer's simplicial-identity cells plus a word-problem argument on the invertible-realization sub-stock.

The alphabets are fixed in the statement: paths are **positive words** (no backward half; symmetry is nothing at dimension 1, deliberately), instances are saturated (profunctor modules, already variance-typed), certificates are translator _singletons_ — a forward map with replay evidence, no inverse and no round-trip demand.
The forward-only certificate shape is a decision, not an omission; the four-transport-fields-plus-corecursive-field package a computational bisimulation-style univalence carries belongs to the invertible core, i.e. to the core-coincidence obligation.
Two permanent degeneracy guards: the **constant-map witness** (four replay-distinct one-way certificates at the two-element code against two invertible ones) kills any thin/order-collapsed rendering, and the **constant-literal witness** (a replay-total translator to an infinite leaf reachable by no finitary vocabulary) is why the fullness quantifier is leaf-natural.
The dimension-2 rule layer remains an equivalence at this stratum: the directed case moves the invertibility threshold up one dimension, it does not delete invertibility from the theory.

**Kernel formers.** `Path` (groupoid: `here`/`walk`/`then`/`back`) and `Flow` (directed: diagonal intro, directed walk with a motive-covariance side condition, composition derived by one walk, **no inversion — the refused motive shape is the symmetry shape**) land as independent primitive formers; no kernel coercion between them, because the comparison is the core-coincidence _theorem_ and a coercion would smuggle it in as an axiom.
Two permanent negative witnesses guard the pair: a K-derivation must fail elaboration, and a symmetry-derivation for `Flow` must fail elaboration.
The directed word problem's honest price: the rule layer's residue grows from the symmetric group to the full transformation monoid, for which no register row and no formalized rewriting twin exists — and that price was quoted against the tree presentation, so it is re-quoted against the arena before it is spent ([[metatheory/roadmap]]).

## The doctrine layer

### Three roles, kept apart

Three different objects were once fused under "the virtual-double-category machinery", and the fusion is what made the question "where does it live?"
unanswerable:

| role                     | what it is                                                                                                                                                                         | side                                                           |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| **the arity kit**        | the parameter of the doctrine complex: a base plus the multiplication as a relation ([[#One construction, two arity bases]])                                                       | below both                                                     |
| **the doctrine complex** | the free telescope over a signature at a chosen arity — spheres, positions, cells, coherences                                                                                      | syntax                                                         |
| **the equipment**        | a split cartesian _fibrational_ virtual double category with chosen constructors: restriction, tabulator/comprehension, hom protype, extension fillers [@nasu-2024-internal-logic] | semantics — the thing a category-with-families projects out of |

The category-with-families comes from the **equipment**, never from the complex: `Ty Γ` is one-sided (covariant) profunctors, context extension is the tabulator, `Id` is the hom protype, `J` is profunctor Yoneda, `Π` is extension along the display map's conjoint, and substitution is literal precomposition — split by construction, so the two axes a classical construction conflates (substitution coherence and fibre coherence) never meet.
**No change of arity base, in any order, produces a category-with-families**: a generalized-multicategory construction supplies the shape of cells, never a tabulator.
The constructors _can_ be freely adjoined — the free bifibration supplies exactly the missing pushforwards with a clean proof theory [@clarke-scherer-zeilberger-2026-bifibration] — but 2-cell equality in such free constructions is undecidable unless the base is factorization-preordered, so freely adjoining fibrational structure is an owner STOP whose entry condition is that check.
Companions and conjoints, by contrast, are free (the zigzag construction manufactures them), which closes the reflection face's standing gap by import.

The CwF is a CwF **of the positive fragment**: types depend on values, never on computations; the negative fragment is an adjoint module over it, with the shifts a tight adjunction.
Three checkable consequences: no per-type `Id` at negative types except through the shift; type dependency confined to the value zone (a type may be indexed by producers, never by consumers or commands, unless routed through a shift); transport is directed, with unrestricted composition only on the invertible fragment.
One standing kill signal rides with this: the shifts are invertible exactly when every value is thunkable and every computation linear — if any construction forces the shifts invertible, gandr's effects have been strictified by accident, and the construction is wrong rather than the theory.

### The join

Contexts are the equipment's objects, so the site and the derivation category are both contexts — there is nothing to choose, and the covariance detail is load-bearing: the nerve's target is presheaves, so the context is the site's _opposite_, and a silent op is how directedness gets lost.
What is genuinely owed is the **loose arrow** between the two context families — the semantics relation, which derivations realize which shapes — and its **tabulator is function extensionality over cellular data**: a carried family of cells in context is a proterm, and the comprehension is the device turning a family-of-cells-in-context into a path-in-context, eliminated by per-position replay, lazily.
Exhibiting the protype whose tabulation is funext is the highest-information single construction owed on this side; the nearest literature template is the syntactic lax-cones construction over finite computads (a cone over a context _is_ a context), read as a starting shape rather than a drop-in — it is stated for globular contexts, as a limit notion rather than a tabulator, and its general case is a conjecture.

### The compositional-rewriting axioms, measured

The compositional-rewriting double-category axioms [@behr-harmer-krivine-2023-fundamentals] are a testable checklist run over the **real** structures — the overlap enumerator, matching and unification, rewriting and normalization, the tracelet certificates — never a second engine; the scope of every verdict is the cell-visible convergent fragment, natives outside every claim.
Verdicts: multi-sums hold degenerate-singleton (first-order syntactic unification makes the family at most one per ordered pair per kind); pullbacks in the tight and cell layers hold strictly; horizontal decomposition holds strictly; the source is a strong multi-opfibration in discrete form; the target is a residual multi-opfibration in per-instance form, exercised exactly by redex-creating instantiations; positive globular decompositions and cellular Conduché hold strictly on the free path algebra; **the cylindrical decomposition property is open** — a distinct obligation, not a corollary, and it is what the convolution face needs beyond exponentiability.
The payoff: the universal concurrency and associativity theorems hold on that fragment by the universal proofs, with the differentials retained as adequacy witnesses.
The measured cellular-Conduché row is a **definitional match** with the discrete Conduché condition (lifting of factorizations, uniquely) [@guetta-2020-conduche], which is both an exponentiability condition and one of directed type theory's fibrancy notions — so gandr has already measured a directed-fibrancy condition on its own cell store; whether it is the exponentiability the convolution face waits on is a scheduled one-day check.
The trigger to revisit globularity-above-the-base is precise and cheap to watch: a non-linear pattern producing a genuine (non-singleton) multi-sum family — many-out one dimension up.

### The convolution face

Over the doctrine, vertical presheaves carry a convolution product; the representable at a rule interface categorifies the rule-algebra basis vector, and under the measured axioms the convolution of two representables decomposes as a sum indexed by the multi-sum — the concurrency theorem, categorified: the fan-out family is the coefficient set of a genuine associative product one level up [@behr-mellies-zeilberger-2023-convolution].
Adopted as **specification, not a second engine**: the categorified concurrency isomorphism is the completeness contract of the overlap enumerator, landing as differential rows in both directions.
The virtual-honest form is published — a colax convolution on presheaves over a virtual double category, defined by a coend on the multicell profunctor, with no horizontal composition assumed, strong under positive globular decompositions [@thompson-carlson-2026-exponentiable]; the unconditional floor is a theorem (every virtual double category embeds in a locally cocomplete completion with composites), and the bridge from a pseudo double category's power into `Set` to exactly this convolution is numbered in [@arkor-2025-exponentiable].
The rule algebra and its representation on states unify through the Yoneda embedding, so "compose rules" and "apply rules to states" become one operation — the kernel already embodies the operational half.

### Interchange, by layer

> Exchanging two independent things is never an equation unless you are willing to lose information; every well-behaved treatment replaces the equation by a witness whose invertibility is the design dial.

The strength is a _stratification_, and collapsing its levels is a recorded error:

| level                                                    | interchange is                                                           | why                                                                                                                                                                                                            |
| -------------------------------------------------------- | ------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| the ambient duoidal category of interfaces               | **lax** — the laxator, by definition of duoidal                          | the physical tensor is an inclusion of admissible orderings; no normalization upgrade can invert it                                                                                                            |
| the shape layer — `Shape` as a **duoid** in that ambient | **an equation**, and proving it is the scheduled next-unit target        | shapes are defined operations with proved equations, not presented structure                                                                                                                                   |
| certificate composition                                  | **structurally lax**; imposing it strictly is _wrong_, not merely coarse | the deadlock counterexample: a strict interchange manufactures a synchronized diagonal move letting two mutually blocked strategies proceed [@mellies-2021-template-games]; duoidal coherence fails in general |

**What the ambient _is_ — a proved identification off the carrier's own typing, not an analogy.** The shape carrier is an object of the presheaf category over interface pairs, $["List Ob" × "List Ob", "Set"]$, and its two operations are the two tensors: grafting is the substitution tensor $◁$ (its composite over a shared middle interface — the implicit middle profile _is_ the coend variable), and the merger is Day convolution $⊗$ along concatenation — the carried `Append` witness _is_ the convolution's indexing profunctor.
The currying is literal: nothing is re-presented to see it.
Three verdicts follow: the pair is genuinely **duoidal** (the interchanger is one-way because the right-hand side lets its two halves split the middle profile differently, and nothing recovers a common splitting); it is **not normal** (the substitution unit has a cell over every profile, the convolution unit only at the empty one); and it is **not physical** — the convolution is not symmetric, and **the failure is at the objects**: the base is the _free_ monoid on the colours (lists under concatenation), which is not commutative, so the merger is a planar tensor and the merger-swap refutation is only the shadow of the reason, visible at the one profile where the swap is even well-typed.
This relocates the canonicalization obligation precisely: it is **not** "repair a merger that failed to be symmetric" but the passage from lists to multisets _at the objects_, which is what makes the Day convolution symmetric — the same delta the literature exhibits, where the normal oplax duoidal structure on the analogous construction is obtained over finite sets and bijections, the free _commutative_ monoid.
The coherence theorem for exactly this one-way distributor shape is the intermutation result [@intermutation-coherence-2013], and the free physical-duoidal case has a decision procedure via zig-zag-free posets [@shapiro-spivak-2025-duoidal] — filed as the tools for the shape-layer equation, not as obligations.

Never add an "all structure diagrams commute" fast path to the certificate store.
On **polarized** boundaries (purely produced or purely consumed) sequential and parallel composition coincide, the bookkeeping is definitional, and the normal-form check is a linear-time acyclicity test; the polarization substrate already ships as per-metavariable variance metadata.
The shift-equivalence quotient of the certificate algebra is admissible exactly because it is earned per pair by a trivial-overlap witness, never imposed; the declined horizontal-composition surface sugar is declined for the same reason and acquires its principled semantics (disjoint positions only) from the same quotient.
The sharpest one-line statement of the whole section, kept because it is the mechanism and not a slogan: the lack of composability is caused by **the decomposition destroying information that would have been needed to define a composition**.

### Holes

The adopted hole theory is **monoidal context theory** [@roman-2023-monoidal-context]: diagrams with holes over a polygraph, contexts as lists of interface pairs — one per hole — with derivations forming the cofree produoidal category over the free monoidal category.
The identification with the substrate is exact: the holes are the vertices, the vertex listing is the context, and the algebra of such things is a normal produoidal category; the ambient duoidal structure above is its instance at gandr's interface category.
A second, independent hole theory exists for the _higher-order_ direction — diagrams-with-holes as strong profunctors, embedded lax-lax duoidally with a sequencer interpreting one-way signalling, and a Yoneda lemma pinning the notion down given channel–state duality [@hefford-wilson-2024-profunctorial] [@wilson-hefford-2026-strong-profunctors] [@wilson-hefford-hoffreumon-2026-supermaps]; it is conditional on wanting higher-order cells (a circuit with a hole taking another circuit), and the convergence of both lines on duoidal-structure-over-profunctors is the signal that the hole layer's home is the equipment's loose direction.

### Doctrine odds and ends that are load-bearing

* **Cartesian double theories.** The 2-dimensional fragment of a shape block is a presentation of a cartesian double theory with product-preserving lax-functor models; test the cartesian law against the _framed_ pairing–projection bijection, expect iso-strong and never strict; a bare virtual double category is not cartesian in that sense, and until the reflection face's cartesian-fibrational notion is reconciled with the double-theory notions, every verdict names which notion it tested.
* **The recurring étale condition.** Homotopy quotients that add arrows, discrete-opfibration conditions making dependent sums compute coproducts, and exponentiability of polynomials in the virtual setting [@fujii-lack-2025-familial] are one invariant seen three times; name it and enforce it wherever a decomposition, sum, or product is formed.
* **Variance.** The comonoid-style settings pay an opposite-category operator with polarity machinery through every judgment; gandr's internal language deliberately excludes it, and the variance layer is a priced future axis, not a current structure ([[metatheory/ambient-and-primitives]]).
* **Aggregation is not functorial** while data migration is; every quantity accumulated over a derivation (fuel, cost, counters) lives in the non-functorial regime, where symmetry re-enters through the cost model rather than the type theory.

## The certificate algebra

**Certificate identity is replay-equivalence** — the replayed-not-trusted discipline promoted to the definition of when two tracelets are the same transformation; composition ships as two operations (unconditional on the invertible fragment; acyclicity-gated on the directed band, declining with the cycle as diagnostic).
The measured finer alternative is declined knowingly, not unknowingly: the asynchronous-games treatment quotients reschedulings by "same induced bijection on step indices" (the same device as the residual line's ancestor function on redex indices — one construction in both programmes), and gandr's identity is strictly coarser on two axes, forgetting the induced permutation and never comparing the two paths; the same programme makes a deliberate at-most-one-2-cell ("locally posetal") choice for one of its own bicategories, so the coarse discipline has a named precedent as a design choice — not a theorem.

**The normal form.** The certificate normal form is closure under abstraction isomorphism (content addressing — already the store's identity), trivial-unit insertion and removal (empty-path elimination — the path calculus's unit laws; the empty path is not an edge, so unit insertion has no tile of its own), and **shift equivalence**: two adjacent cell applications at disjoint positions with trivial overlap commute.
The contract, at full strength and correctly scoped:

* normal-form equality **decides shift equivalence** — an iff, from the uniqueness of primitive factorization and the shift-equivalence characterization [@behr-2019-tracelets] [@behr-kock-2021-tracelet-hopf];
* shift equivalence **implies** replay-equivalence — sound;
* the converse is **constructibly false** in gandr's own codebase: replay-equivalence is pure proof-irrelevance beyond replayability, so two confluence certificates joining by different routes are replay-equal with different primitive multisets.

> The normal form is a performance fast path, never a decidability result; replay-equivalence is already decidable by boundary equality plus two replays, and the normal form answers its _cost_ question.

The primitive **multiset with multiplicity** is load-bearing (deduplicating by identity returns one where the answer is two), and the canonical schedule (earliest causal position, content-address tie-break) loses nothing by the enveloping-algebra basis theorem.
Two hypotheses the source uses without establishing — local finiteness and completeness — are carried as flags, not assumed.

**The bracket oracle.** In the commutator over all overlaps the trivial and disjoint terms cancel, so the bracket's support is exactly the nontrivial overlaps: bracket vanishing ⟺ no nontrivial overlap ⟺ freely reorderable.
One overlap-support lookup serves four consumers (normalizer, completion scheduling, parallel replay, the acyclicity gate's structural cousin); the replay plan's fuel is the causal critical path, not the schedule length.
This is exactly where the symmetric parallel direction is cashed, and it would break _silently_ if that direction were ever ordered.
The full Hopf structure (formal sums, the coproduct as all splittings, the antipode) stays specification currency; the recorded directions (coproduct as cache-key decompositions; antipode as inverse-certificate construction on the invertible lane) are unscoped.

**The residual-theory position, named precisely.** gandr's shift equivalence is the _reversible_ permutation equivalence of the axiomatic-rewriting line — strictly finer than Lévy's, which additionally tiles duplicating and erasing squares gandr has no generator for [@mellies-axiomatic-rewriting]; soundness is inherited free, and the primitive-multiset invariant supplies exactly the modulo-reversible-equivalence residue that standardization leaves undecided — a relationship no publication states in either direction.
The tile relation of a two-dimensional transition system is freely chosen, so gandr's overlapping rules and completion loop are admissible by construction, and no orthogonality hypothesis was ever in the way; the current tile relation is _empty_, so instantiating the axiom interface non-vacuously is what buys the standardization theorems by citation.
A gandr tracelet is structurally a permutation _tile_ whose two legs are trek-shaped multi-step paths, with replay-equivalence in the seat of "same induced residual relation"; interaction nets are decisively the wrong shape (three defining constraints, all violated).
**Do not substitute the planar string-diagram quotient for the symmetric one** [@delpeuch-vicary-2022-normalization]: the relations are incomparable in both directions and no carrier translation exists; what transfers is the adjacency-as-height-order match and the rule _content address at the component level, geometry within a component_.

**The certificate layer is a decomposition space.** The tracelet algebra _is_ a 2-Segal object, and the doctrine layer above it is a double-categorical object with pullback axioms; the published equivalence between 2-Segal spaces and augmented stable double Segal spaces is the missing edge between the two layers, with the source's own redundant-data model (a 2-simplex as a 2-tracelet together with its chosen composite) being that input in S-construction vocabulary [@behr-kock-2021-tracelet-hopf].
The identification is to be _named and cited, not proved_ — verify that the measured strict pullbacks are the stability condition, establish or refuse a set-level shadow, and record the edge; scheduled in [[metatheory/roadmap]].

## Representation and performance

> Prefer representations in which the address map has bounded sensitivity under local edits.

A local edit must perturb a logarithmic number of content addresses; fan-node sharing gives no such bound.
That Lipschitz-style condition on the _addressing scheme_ is the honest salvage of the retired "perturbation is local" principle; it is load-bearing for content-addressed chunking specifically, while ordering, the arena, and the fuel stance are justified independently. gandr declines the optimal-reduction objective while learning its layout lesson (the best-known implementation's fiftyfold win came from a memory-layout change): linear runs, computable offsets, chunked storage.

**The acceleration band** exploits three aligned order-independence properties — shift equivalence (logical), signature-tensor associativity (algebraic), history-independent chunking (representational) — for four workload classes: batched signature scans (advisory), overlap screening (advisory, sound-direction only), chunk-parallel replay and rehash (exact, differentialed), rule-algebra numerics (analysis band).
Signatures are computed on the canonical schedule, where they are well defined on equivalence classes; the antisymmetric block is the numerical shadow of the Lie bracket, so a nonzero value for a recorded-independent pair is an arithmetic alarm before any replay diverges.

> **The accelerator firewall (binding).** Accelerator results are either advisory or exact-and-differentialed; none is ever soundness-bearing; the kernel never links this band; numeric nondeterminism must be unobservable; adoption is measurement-first.

## The ambient-primitive policy

Identity here is a construction in time over an unfinished substrate — the temporal rendering, against the completed (K/truncation) and spatial (interval/cubes) renderings; the substrate is codata with weak final coalgebras, no η, label-intensional identity never observed.
The without-K discipline is binding independent of everything else: no K eliminator, no deletion in unification, no type-constructor injectivity, no definitional proof irrelevance for identity, no interval or gluing primitive, no collapsing identity proofs because their codes are content-addressed equal; per-type set-ness by Hedberg over decidable equality and grade-discipline runtime erasure stay available and are not exceptions.
The adequacy witness is negative and binding: a corpus program deriving K must fail elaboration.

**The admissibility criterion for a primitive is trusted surface.** Not spatiality (geometry is fine), and not merely computability: a primitive is admissible outside the kernel and expensive inside it, whether or not it computes.
Three cases, in increasing cheapness: axiomatized in the theory with no computational interpretation — blocked; in the kernel and computational — admissible, _priced_ as permanent trusted surface (the cubical interval and Kan operations are the type case: the shipped computing-J primitive took two major releases to shed, and only after general machinery provably subsumed it); **explaining a meta-operation that adds nothing to the theory** — admissible and unpriced, the cheapest case, where the naturality-generation construction sits. gandr's own ledger: definitional J, no new trusted surface, native directedness, per-stratum decidability — paying fenced completeness, the retail coherence bill, and stratified growth as a standing obligation.

**The decidability ceiling.** For free structures of this genre the word problem is **undecidable in general**, and decidable exactly under named tractability conditions (unique diagonal fillers, or local finiteness) [@clarke-scherer-zeilberger-2026-bifibration]; over a free base the factorization order collapses to prefix/suffix comparison, which is the honest metatheory face of "this stratum is entirely computable".

> Beyond a tractability fence, per-instance certificates are the only general currency: a decision procedure may be promised only on fragments satisfying a named tractability condition; everywhere else certificate-carried discharge is not a fallback but the mathematically maximal offer.

Corollaries, each converting an assumption into a named fence: the normal-form fast path is sound exactly where a convergence witness applies and is **TCB-adjacent** (a guard plus a soundness witness, never documentation); coherence walls are staged as separate named obligations with separate suppliers (acyclicity, tractability, termination — never one monolithic convergence proof); and any "just decide it globally" request is priced at a theorem false in general.
This ceiling and the declined completeness half of the layout calculus are one fence seen from two sides.

**Internalizing diagram structure: the four currencies.** Reedy-fibrant diagram structure can be internalized at the cost of an interval (the synthetic and cubical routes), a modality (displayed type theory's guarded display [@kolomatskaia-shulman-2023-displayed]), or a finite-level bound with all coherences explicit [@kraus-sattler-2017-space-valued]; gandr pays a **fourth currency — coinduction under guardedness — and it is the only one of the four that reaches ω without a primitive**, resting on the library's own coinductive discipline with no upstream warrant (now priced against three named alternatives rather than hanging in the air).
Span-based internal parametricity needs no fibrancy and no interval [@altenkirch-chamoun-kaposi-shulman-2024-internal]; the deeper reframe of the whole operator zoo is _choose a base category and a fibrancy notion; the primitives are downstream_, with the transpension right adjoint organizing the operators and their affine-variable costs [@nuyts-devriese-2024-transpension].

**The modality decision.** Do not adopt cohesion internally and do not expose a modality at the surface: the modality's jobs in this literature are guarding display (gandr: guardedness in the coinductive tower), expressing globality of a universe (gandr: exempt, because the code universe is a Tarski-style _data_ universe — an inductive object one can recurse over, which is the load-bearing precondition of the whole placement, with the globality diagnosis in [@licata-orton-pitts-spitters-2018-internal-universes] as independent corroboration), and tracking variance (a cost gandr knows it must eventually pay, with a worked polarmode system as the named way to pay it [@nuyts-2026-natpt]).
Three revisit conditions, and no more: the code universe stops being a Tarski code family; a construction needs display on _open_ terms; the variance layer is built.

**Non-HIT discharges, recorded beside the no-spatial-primitive prediction.** Quotient-by-symmetry is `Rigid` (an effective quotient by decidable normalization); truncation is a structure change on a fixed type [@petrakis-2022-typoids]; free colimits are fenced to the colimit layer.
Two of the three standard higher-inductive-type uses have non-HIT discharges in gandr's own vocabulary and the third is out of scope by construction; gandr's computads adjoin higher cells as ordinary constructors over a lawless setoid, so the higher structure is _carried_, not _generated_.

**The adopted technology cluster.** The parametricity line is adopted as a technology source (not a comparison class): its stated motivations — types stratified by finite dimensions, strict equality unfeared but coherence obligations feared, models that need not present spaces but must compute — are gandr's own commitments arrived at independently, and the implementation base (bridges alongside cubical, univalence and internal parametricity coexisting in a shipping typechecker) is where "priced and shipping" now lives.
One recorded fork: factorization systems are gandr's idiom and not that line's.
The **nominal aim is adopted, including the higher case**: nullary internal parametricity _is_ name abstraction (an identification, not an analogy), with a nominal type theory built on it, the Schanuel topos as semantic confirmation, and higher-order abstract syntax proved adequate from parametricity alone [@vanmuylder-nuyts-devriese-2025-nominal] [@vanmuylder-2026-thesis]; binders-times-circuit-cells is the combination that makes gandr a metatheory tool rather than a rewriting engine with a type system, and the alternative route ("the sharing should become a wire") keeps the two genuine alternatives, so the design question is which, not whether.
Parametricity-as-coherence-management now has five independent arrivals (cohesion, displayed type theory, cubical internal parametricity, the pretype-theory line, and the ω-categorical naturality meta-operation); it is a pattern to act on, not a coincidence to note.

## What would falsify this

1. **Canonicalization completeness fails** for the parallel-component multiset — ordering is not a section, and the arena needs rethinking.
2. **The graphical-species identification fails** — a description needs structure the species base cannot express; the univalence programme re-bases before anything is built.
3. **The Segal condition cannot be checked at finite cost at some stratum** — per-stratum `ua` is unavailable there _by construction_, which is the design working, not breaking.
4. **The generalized-Reedy factorization is not computable** even with canonicalization — strata and fuel decouple.
5. **A shift-equivalent pair replays divergently** — a soundness bug in position or overlap bookkeeping; stops the certificate lane immediately.
6. **A forced non-computational primitive at any rung** — falsifies the temporal route for gandr and reopens design; a uniqueness-of-identity-proofs collapse forced at any stratum is a STOP.
7. **The shifts are forced invertible** by any construction — the effects have been strictified by accident.
8. **The coherence-debt arity law fails its scheduled test** — the interchange needs two cuts to commute, the ladder is not finite, and the four-tier policy's tier-2 coverage shrinks (the two other falsifiers of the law are in [[metatheory/roadmap]]).
9. **The monomial-to-monomial condition fails** for the construction-term normal form — canonicalization soundness loses its published route and needs another.

The retired falsifiers (real cells not simply connected; the term face needs PROP-style composition; rectification-admissibility) are dissolved by the full-rung substrate and its nerve warrant; they are tombstoned in [[metatheory/guards]] so they are not re-derived.

## Roadmap

The detailed queue — spikes with costs and deciders, standing obligations, open questions, the reading list, and the falsifier ledger — is [[metatheory/roadmap]].
The five headline directions, for orientation: extend to the directed case (it is forced, three ways); generalize the arena by the factorization-system route; make the four placements of the circuit algebra against the doctrine explicit in built artifacts (the term face is the gate); evaluate the universe presentation of the arity layer (two half-day spikes decide it); adopt the parametricity cluster's technology and take the nominal case as an aim.

## Sub-documents

* [[metatheory/roadmap]] — everything that remains, with costs and falsifiers.
* [[metatheory/carrier]] — the circuit-algebra carrier record: constructors, landed theorems, the edge identity, the palette, the incidence theorems.
* [[metatheory/directed-univalence]] — the directed statement in full: candidate comparison, alphabets, grades, guards, the kernel formers, the equipment inventory.
* [[metatheory/layout-and-coherence]] — the layout calculus per former, the coherence modules, the arena generalization detail.
* [[metatheory/ambient-and-primitives]] — the cubical contact in full (the J ledger, the verified evaluator register, the price ledger), the internalization currency table, the technology-cluster survey.
* [[metatheory/guards]] — the do-not-reopen ledger: declined halves with reversal conditions, dissolved forks, withdrawn claims, name-collision warnings, and the code concordance to the retired records.
* [[metatheory/citation-hazards]] — locator defects, version drift, unverified reports, and per-source publication status.
