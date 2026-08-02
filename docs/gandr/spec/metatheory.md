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

The kernel is a polarized System-L command IL [@binder-2024-grokking]: producers, consumers, and commands $angle.l p bar.v_ε c angle.r$ as first-class arena-resident data, with the frozen call-by-push-value core [@levy-cbpv] as the source calculus and a static focusing translation between them.
Four of its properties carry the rest of this document.

* **Redexes are at the cut, so overlaps are shallow.** A rewrite cell's left-hand side is a cut between a constructor pattern and an operation frame; no rule searches a term tree.
  Critical-pair enumeration is tractable where tree rewriting needs full traversal, which is why the compositional-rewriting suite of [[#The doctrine layer]] can be run at all.
* **Consumers are first-class, so the seam is visible.** Under continuation-passing the same overlap hides behind a lambda; visibility is what makes fusion a derived 2-cell with a certificate rather than a pass.
* **Strategy is a per-cut polarity orientation.** Positive cuts fire the producer-side binder first, negative cuts the consumer-side one; evaluation strategy is an orientation choice on cells, not a global language property, and the two-region store discipline the machine adopts is the heap/frame split of the modal-sequent line [@caspar-munch-maccagnoni-2026-s4] with its modality reinterpreted as residency, never imported as a type former.
* **Multi-conclusion contexts have a home.** The linear consumer zone is where a multi-conclusion reading lives, and it is the declared growth point for the multi-output term face.

**Fusion is Squier completion on cut seams** [@squier-1987-word-problems].
Surface rewrite members elaborate to oriented command cells; overlaps at cuts are bona-fide critical pairs; a budgeted completion loop synthesizes derived cells whose certificates are the pair of joining paths, differential-tested against the two-step composite and **replayed rather than trusted**.
Two limits are permanent: natives are opaque, and non-linear overlaps fan out into families rather than a single fused rule — the second is a theorem of the virtual reading ([[#The doctrine layer]]), not a shortfall.
The Squier citation is good at dimension one, where the completion loop lives [@squier-1987-word-problems] [@squier-otto-kobayashi-1994-finiteness]; **finite derivation type fails above dimension one** (an explicit finite convergent 3-polygraph with finite critical branchings lacks it [@ara-burroni-guiraud-malbos-metayer-mimram-2025-polygraphs]), so the higher-cells lane ([[surface-language/higher-cells]]) must not assume the completion story lifts.

**Closure conversion is an in-IL rewrite** at the shift boundary, from the abstract-closures account of Sullivan's thesis [@sullivan-2023-reflections] (its published précis is [@sullivan-downen-ariola-2023-little-pieces]).
The device: environments are first-class syntax — a delayed substitution $sigma ::= epsilon | sigma, V \/ x$ — and an abstract closure $\{sigma, "force" arrow.r M\} : U_r B$ attaches one to the only introduction site that needs it.
Because gandr's λ is a computation and never bound, **closures live at the `U`/`force` shift alone**; there is no λ-site closure to manage.
The load-bearing piece is that capture is **partial**: the introduction rule types the body under the ambient context _plus_ the captured part, which is what makes conversion decomposable into little pieces — one free variable at a time, by the oriented cell $\{sigma, "force" arrow.r M\} arrow.r_("CC") \{sigma, x \/ x, "force" arrow.r M\}$ for $x$ free in $M$ outside $sigma$'s domain.
Closure β is then an interaction rule on the cut, $angle.l \{sigma, "force" arrow.r M\} bar.v_ε "force" dot K angle.r arrow.r angle.l M[sigma] bar.v_ε K angle.r$ — closure conversion joins the same completion framework as fusion.
Environment sharing and lambda-lifting are _derived_ 2-cells, not new machinery; and grades refine capture in a way the source lacks — a grade-zero variable need not be captured at all.

The correctness story is three theorems of the source: **backward simulation** (every machine step decodes to a derivable IL equality — machine execution _is_ equational rewriting), **soundness** (typed equality of the IL by Kripke logical relations over heaps, so conversion is correct by construction), and **adequacy** — the payoff: for closure-conversion-normal programs, a machine that `Build`s (capturing) and a machine that `Build^cl`s (looks up only the variables the closure names, emits a fixed code sequence, never captures dynamically) _provably agree_.
Adequacy is what makes the compile-versus-evaluate differential a **theorem on the closure-conversion-normal fragment** — with closure _entry_ left unfactored, so lowering supplies the match-then-jump, and closure-conversion normal form is the gate under which the backend may emit a flat environment struct plus code pointer.

Three caveats travel with the account.
The source machine is a stack machine, not a sequent machine: the laws transfer unchanged at the polarity level (the value/computation split, closures-only-at-the-shift, environments-as-explicit-substitutions, the conversion orientation), but the machine-level form is re-derived against the L machine, where call-by-need memoization becomes consumer-side sharing (a μ̃-bound shared consumer).
Recursion and effects are absent from the source: its future-work note — fixpoints living at the thunk type as recursive closures — endorses gandr's graded-thunk `fix` without proving anything about it, and the soundness proof is not stable under effect handlers by inspection, so **closure-conversion correctness under effects is owed proof, not literature** (the intersection of the two literatures is empty).
And the named proof debt: confluence of environment capture is **modulo environment reordering**, so the Agda face needs a permutation quotient — a `Rigid` instance, and it should be built as one; the ordering corollary is that hash-consing happens **after** reaching closure-conversion normal form, so environments are canonical.
Consumer-side closures (co-closures) are undeveloped in the source line and are owed by the codata and first-class-consumer directions.

**Four kernel boundaries that are invariants, not preferences.**

* **Progress holds only for focused statements**: un-focused input can be stuck, so elaboration runs the focusing translation (administrative-redex-avoiding) or accepts partial progress — a real invariant on the checker-to-machine boundary.
* **Where the function type lives**: the frozen CBPV core keeps its formers as the source and typing calculus; the IL _represents_ abstraction and application as application-codata internally — a representation choice inside the IL, invisible to the contract.
* **Handlers are consumer-side case analysis**: a deep handler is a consumer pattern-matching operation constructors and binding the resumption as a _covalue_ argument; the delimiter is a prompt covalue and capture is a μ-binding up to it.
  One mechanism replaces operation-keyed handler frames, delimited-control primitives, and loop operations, and it is more inspectable (a handler is IL data); the reserved further unification is that copatterns and delimited control are the same mechanism.
* **η-hygiene in completion**: codata-η is valid only call-by-name and data-η only call-by-value, so the completion engine must consult the cut polarity before using any η-shaped step in a joining path — an easy-to-miss soundness constraint, to be pinned by a pathological corpus witness.

**Two fusion-engine obligations and one scope fence.** When handler reductions enter the cell store, completion runs _over the handler reduction theory_ (the fine-grained reduction theory as the rewrite system), making confluence-with-handler-reductions a discharge obligation of the fusion engine rather than an assumption; full handler residualization holds only for a typed subset, so graded thunks plausibly fall outside it and the compile-versus-evaluate differential is the safety net for the dynamic-handler fragment — an explicit boundary to record when that phase lands.
And the standing scope fence: **never claim fusion for the whole language while natives are opaque**.
One elaboration limit rides with the kernel's grade discipline: grades live in a resource semiring solved by semiring constraints, which pattern unification cannot express — implicit resolution reaches type-level indices only, never grades.

**The kernel's polarity is a datum of the substrate's colour algebra.** gandr's producer/consumer polarity is the orientation morphism $θ : (C, ω) → {↑, ↓}$ of the carrier's palette (see [[#The substrate is the full circuit-algebra rung]]): a cut $angle.l p bar.v_ε c angle.r$ is precisely a contraction of a $c$-leg against an $ω c$-leg, and the involutive-colour duality that makes the modular-operad literature look "unrooted" is the kernel's own classicality with polarity forgotten.
Three two-valued dials must never be conflated: CBPV polarity (the palette orientation), the doctrine's tight/loose stratification, and functorial variance — they are different axes, and each has its own machinery.

**As built, the cell grammar is single-continuation.** Verified against the crate: the **cell-visible** command pattern has exactly one variant — a polarized cut with a producer half and a consumer half (the IL itself has three command forms, but primitives are the opaque host seam and by-reference jumps are outside the cell-visible fragment); the consumer pattern is a linear spine, each frame carrying exactly one return continuation (the exactly-one half is checker-enforced on the IL's destructors; the constructors' empty consumer lists are construction-site discipline, and the n-ary grammar on cut-adjacent constructs is the reserved growth point); every face and argument list is a positionally-indexed ordered sequence; the double-pushout inheritance is nominal, with no pushout complement in code — deliberately, since the term-rewriting double-category instance with discrete opfibrations and multi-sums as minimal unifiers is the right shape for a term-shaped cell store.

> **The scoping that graph-shaped double-pushout instances do not apply is re-scoped rather than retired, and the re-scoping is now cited rather than predicted** — the executed record is [[metatheory/roadmap#meta-spike-16]].
> It stands for as long as the cell store is term-shaped, which is today.
> It lapses on three of gandr's four circuit axes the moment the cell grammar changes: **convex** double-pushout rewriting with interfaces over **monogamous acyclic** hypergraphs is sound and complete for arbitrary symmetric monoidal theories, coloured ones included, and that fragment admits many-out, reconvergence and disconnection while excluding exactly the fan-in and fan-out gandr already declines [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-ii].
> It **does not** lapse on the wheel axis: acyclicity is a hypothesis of that fragment, of convex matching, and of the published termination arguments, and gandr's traced destination rung is reached by no statement in that line.
> The engineering consequences, the convexity hazard it carries, and the residual questions are [[implementation/circuit-terms#The correspondence at gandr's own rung, at theorem grade]].
> Three consequences: multi-output interfaces are unrepresentable today, not merely unused; within-cell ordering costs nothing to adopt, because no symmetry is present to give up; and the shift-equivalence relation of [[#The certificate algebra]] currently has empty extension, so its theory is forward-looking and a vacuous pass must never be read as a discharged obligation.
> The **multi-output (destination-passing) term face is a ratified design direction** with nothing constructed; the sequent layer already carries the type shape (a consumer list on cut-adjacent constructs) while every construction site emits zero or one element.
> The lane that builds it, together with the reconvergence, disconnection, and wheel axes it is a special case of, is [[implementation/circuit-terms]].

**The localization move.** One pattern recurs: where a global gluing property fails, localize the choice and restrict the global operation.
Evaluation strategy (confluence fails; orient per cut), certificate composition (dinaturals do not compose; unconditional on the invertible fragment, acyclicity-gated on the directed band [@laretto-loregian-veltri-2026-directed]), loose composites (need not exist; virtual, as multi-sum-indexed families), and interchange (not an equation; a witness whose invertibility is the dial) are four instances of it.

## Cellular data — descriptions, cells, and computads

A datatype's description is a first-class value; generic operations are ordinary programs over descriptions, and the same artifact serves the cell layer, the matching engine, and the reflected judgment layer [@chapman-dagand-mcbride-morris-2010-levitation].
The ladder is staged and additive: (0) descriptions as host values with generic operations, faces stored untyped but checked; (1) a closed typed code universe with a trusted decoder; (2) descriptions reflected as gandr data, generic induction and the free monad as library surface; (3) full levitation.
The canonical shape is the tagged description — an enumeration of constructors times a first-order code per constructor, code grammar ${1, "var", ×, σ}$ plus additive decorations (a grade slot, an atom-abstraction code for binders, erased attribute slots).
Decidable equality of codes is load-bearing, not a convenience: it is what content-addressing interns on and what matching compares; the first-order fragment is chosen _because_ it keeps this.
Codata descriptions are the same codes under the final-coalgebra decoder, polarity-sorted from birth; intensionally this yields only weak final coalgebras, consistent with the no-η codata stance.

**Multi-output arities are bridge diagrams.** An operation $(X_a)_(a∈A) ↦ (Y_b)_(b∈B)$ with $Y_b = Σ_(i∈I_b) Π_(j∈J_i) X_(s(j))$ is presented by $A ←^s J →^π I →^t B$ and computed as $Σ_t ∘ Π_π ∘ Δ_s$ [@spivak-garner-fairbanks-2021-aggregation].
The Π-layer (one operation's named result tuple) and the Σ-layer (aggregating contributions into one destination) are different things, and the Σ-layer requires a commutative monoid on the target — unrestricted fan-in is not free wiring; non-combining multi-output (pure routing) is just the target map and is free, so the multi-output face splits into a free half and a structure-requiring half.
The aggregation colimit is a _specification_; the destination-passing writeback is its operational realization — the source gives no execution, cost, or linearity story, and must not be over-read as one.
Multi-output arities are an index change on the description universe: generalizing the recursive-occurrence code to a multiset of output sorts is exactly a container, so the multi-output term face forces the **indexed** description universe, and the signature universe should be based on containers precisely because sorts are arities.
Read one level up, the bridge diagram _is the graphical-species profile_ of a generator, which is the identification [[#Stratified univalence]] runs on.

**Cells at every dimension.** The surface names sorts (0-cells), constructors and operations (1-cells), named directed rewrites (2-cells), declared coherences between rewrite composites (3-cells), with dimension ≥ 4 reserved parse-and-decline: `sort <S>(indices)?`, `cons <C>(fields)? (: T)?`, `oper <f>(params) -> R`, `rule <name>: lhs ~> rhs`, `meta <name>: ρ ~>> ρ′`, and `cell …` reserved.
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

gandr's cell substrate is the **nonunital (downward) circuit-algebra rung**: the monad is $O T^times$ on oriented graphical species $"OGS" = "GS"\/"Di"$ — the slice of the graphical-species category over the orientation object — described in the source as _directed graphs with labelled input and output ports and port-preserving morphisms_, and its algebras are the **nonunital wheeled props** [@raynor-2026-nerve] [@raynor-2025-functorial].
The identification is structural, not aspirational: the carrier's wiring datum `Match` pairs every source with a partner — a sink (a flow-through wire) or another source (the cap, which is gandr's cut) — and no constructor pairs two sinks.
That is verbatim the **downward** condition on Brauer diagrams, and three consequences stand or fall together, each proved in the carrier rather than cited:

1. **the wiring is downward** — $"Match" Γ Δ$ is inhabited only when $Γ$ is at least as long as $Δ$, the difference paid in caps, reproducing the downward category's hom-emptiness;
2. **the nodeless loop is not _derivable_** — a closed circle needs a cap composed with a cup, and the cup does not exist;
3. **composition cannot manufacture a closed component** — a composite of downward wirings is downward.

If a cup is ever added, all three go at once; do not add one to make an operation total.

> **That ruling stood when an operation turned out not to be total, and the repair went the other way.** Graph substitution's base case is a two-sided closure — a trace — and its degenerate case is exactly a closed circle ([[#The arity interface, universe-style]]).
> The cup is far stronger than that needs: it is a binary pairing of two named sinks, so it inhabits hom-sets that must stay empty and takes 1 and 3 with it, while a **circle consumes no port and produces none** and touches neither.
> Not derivable and not adjoinable are different claims, and only the first is above.

All three are the source's, not gandr's restatements of it, and each has a locator [@raynor-2025-functorial, def 3.14 and rmk 3.9]: the downward hom-sets are empty whenever the target is longer, the cup is not one of their morphisms, and the open diagrams fail to be a subcategory of the _full_ Brauer category precisely because the unit trace $"tr"("id"_1) = ⃝$ is not open.
That last reading is the sharp one, and it is what the arity ruling below turns on — **the open fragment is closed under composition and not under contraction**, which is why the count could stay at zero for exactly as long as the former was grafting.

Consequence 2 once carried a third clause — that no scalar ever has to be assigned to a free loop, so the "problem of loops" of the unital rung never arises.
That name is the literature's and is worth carrying with its locator, since the phrase reads as informal and is not: it is the subject of Section 6 of [@raynor-2021-graphical], which the circuit-algebra combinatorics cites for it directly.
That clause is **retired**: it was bought by the trace not existing, and the trace is now the arity's primitive former.

> **What that clause recorded was "not yet", and it is worth saying so rather than reading it as either an error or an abstention.** No scalar was assigned because no operation produced a circle: grafting and merging cannot, the first by consequence 3 and the second by construction.
So the tree was not silently setting the loop to the unit — but neither had it decided the loop was unobservable, because the operation that observes it was simply unbuilt, and from inside an incomplete operation set those two look identical ([[proof-engineering#The substrate tower]]).
The clause was therefore true when written and had no expiry on it; completing the operation set is what expired it, and had substitution been built without anyone reading it, its degenerate clause would have assigned the unit by default.
What replaces it is the source's own answer rather than a gandr device — a Brauer diagram **is** a pairing together with $k$, its number of closed components, with $k$ additive under composition plus the components that composition closes [@raynor-2025-functorial, def 3.4]. gandr's `Match` is the $k = 0$ fragment, the _open_ diagrams; carrying $k$ at the code is the passage from the open fragment to the definition, and it leaves 1, 2 and 3 exactly as stated.
Downward hom-sets are finite for all profiles with **no graph-shape hypothesis** — so hom-finiteness, previously attributed to simple connectivity, is free at this rung, and the old finiteness-motivated restriction ladder is retired.
Two cautions keep the rung honest.
What is special about the substrate is that the wiring category is **free** — the free compact closed category on a palette — not that it is operadic (every permutative category has an underlying operad; that construction distinguishes nothing).
And the passage from gandr's duals-free directed interfaces to the compact-closed Brauer world is the Int construction [@joyal-street-verity-1996-traced]: gandr needs no duals, Int supplies them, and the inclusion is an equivalence.

**Those two sentences are about different categories, and reading them as one is the trap.** The source separates three, and naming all three is what stops the freeness line from looking like a claim that gandr's own wiring admits duals [@raynor-2025-functorial, ex 3.25]:

| category               | what it is                                                                                                                             | gandr                                        |
| ---------------------- | -------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------- |
| $W_D$                  | the full subcategory on all-upward objects — canonically a $D$-coloured wheeled prop, **not** compact closed, since it admits no duals | **this one**                                 |
| $"Int"(W_D) = W"BD"_D$ | walled Brauer diagrams, on objects $↑c ↓d$: duals arrive by reversing a strand across the wall, never by a cup                         | the passage                                  |
| $"OBD"_D$              | all oriented Brauer diagrams — the free compact closed prop on the palette and its formal duals                                        | the ambient the literature states results in |

So the licence Int buys is precise: **a result about the compact-closed ambient reaches gandr without a cup ever entering the carrier.** Its hypothesis is that $W_D$ be traced, and the trace on a wheeled prop is contraction — feeding an output back into an input of its own colour.
That hypothesis is discharged by the arity ruling below rather than assumed: before the closure was primitive, the trace existed in the structure gandr is _identified with_ and not in the carrier as built.
The same passage is what fixes the accounting question, since $W"BD"_D$'s diagrams carry $k$: dropping the circles would land Int in a quotient of it and not in it.

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

Grafting and merging are total and **do not preserve the predicates**: connectivity is a predicate on objects, so any two shapes compose and cell-ness is checked of the result, with the counterexamples exhibited and refuted in the tree — `bigon`, two corollas grafted along two legs that reconverge and so lose acyclicity; `two-points`, a merge of two cells that disconnects and so loses connectivity; and `wheel`, one vertex whose out-leg is glued to its own in-leg, closing a directed loop.
The tree's named _self-gluing_ witness, `gluing` — two vertices joined by one contracted wire — is connected, acyclic, and a cell: a self-gluing as such is not a wheel; the loop at one vertex is.
The predicates need their refuters: an invariant can be structural or refutable, never both in one type, which is why a "generated cell" variant is deferred to the pasting layer as an adequacy pair rather than adopted as the carrier.

As built, the `Cell` record still demands simple connectivity — the dioperad fragment — and this is the **one remaining carrier restriction**; deleting it (or replacing `Cell` with a family of carried predicates) is an accepted direction under the generality ruling, scheduled in [[metatheory/roadmap]].

> **Read that sentence precisely, because it is easy to over-read and has been.** The restriction is on the **record**, not on the algebra: `graft`, `merge` and substitution are all typed at `Shape`, they are total, and they do not preserve the predicate — so nothing in the operations is confined to the dioperad fragment, and reconvergence and disconnection are already ordinary results of composing.
> `Cell` is a bundle of a shape with a proof, consumed by nothing outside its own witnesses (`corolla-cell`, `point-cell`, `chain-cell`, `gluing-cell`).
> The restriction therefore stopped being structural when the operations were typed at `Shape`; what remains is a record definition and a bookkeeping item, not a constraint a consumer is fighting.
> What each dropped restriction buys in the _derivation_ dimension is named there too: many-out is multi-conclusion derivation (the ratified term face), disconnection is **concurrency** (parallel independent rewriting arriving in the doctrine dimension), wheels are cyclic derivation (the completion loop's fixpoints).

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
The obvious first move — reuse the existing virtual-double-category complex verbatim and encode circuit-shaped cells as data in it — fails for two independent reasons, worth keeping written down: its cell target is a _single_ loose generator where a circuit-shaped cell has an output _string_; and its source is a linear path chained end-to-end, where a circuit-shaped source is a _graph_ that fans out.

The corrected statement of what a kit is (the retired record's "two arity monads" diagram was imprecise exactly where the many-out content lives):

> **An arity kit is a base together with the monad's multiplication presented as data.** The kit's carrier is the base category's edge datum ($"Step"^* : "Ob" → "Ob" → "Set"$ for the linear kit; $"Shape" : "List Ob" → "List Ob" → "Set"$ for the circuit kit); its concatenation is the multiplication's underlying operation; its `Cat` is the multiplication's **graph**, carried as a first-order relation; its `Same` is the heterogeneous comparison that graph needs.
> A generalized multicategory is $C_1 → T C_0 × C_0$ in a base $E$, and the many-out content lives in $E$, never in an outer application of $T$ — so the two kits are not two endofunctors of one category, and "compose the monads" is not even well-typed as posed.

| kit         | base $E$                     | monad                                                                                                                          | its generalized multicategories                                               | its algebras                                                      |
| ----------- | ---------------------------- | ------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------- | ----------------------------------------------------------------- |
| **linear**  | graphs (objects and edges)   | the free-category monad                                                                                                        | **virtual double categories**                                                 | categories                                                        |
| **circuit** | (oriented) graphical species | $T^times$, itself the composite of the merger monad over the contraction monad by a free distributive law [@raynor-2026-nerve] | the circuit-shaped doctrine (the accepted direction for the doctrine's cells) | **nonunital circuit algebras**; oriented: nonunital wheeled props |

The old licence — "both arities are cartesian because the symmetric group acts freely" — is **dead at the circuit rung and replaced by two facts**:

* **for the nerve**: $T^times$ **has arities** at `Set` (the graphs inside graphical species), which is the hypothesis the abstract nerve theorem consumes [@berger-mellies-weber-2012-arities]; cartesianness is not on that chain;
* **for the carrier**: Leinster's _presentation_ of generalized-multicategory theory wants a cartesian arity [@leinster-2003-higher-operads], and gandr's **ordered representation** supplies one — but the construction's existence needs neither that nor pullback preservation (below), so what the ordering buys on this side is **decidability** and not existence; the symmetric-group quotient is not avoided but _relocated_ into canonicalization soundness.

**The carrier-side requirement is not what it was assumed to be, and the unified framework prices it in three tiers rather than one** [@cruttwell-shulman-2009-generalized-multicategories].
Cartesian means the monad preserves pullbacks _and_ every naturality square of the unit and multiplication is a pullback (Definition B.2).
A **pullback-preserving** monad on a category with pullbacks already extends to a strong monad on the spans, and generalized multicategories are its monoids; cartesianness is what makes that extension _horizontally_ strong, which is what recovers Leinster's bicategory on the nose (Example 3.6; appendix B.1; Proposition B.3).
And **Burroni's tier requires nothing at all of the monad** — not even pullback preservation, only that the base have pullbacks — which the framework reaches by observing that any endofunctor on such a base induces an **oplax** functor on the spans, hence any monad an oplax monad, with the horizontal Kleisli construction extended to those (appendix B.11; Definition B.12).

gandr's base qualifies at every tier: graphical species is a presheaf category, so it has all limits.
So the existence of the generalized-multicategory structure over $T^times$ at `Set` does **not** depend on a cartesian arity, and it does not depend on pullback preservation either.

> **The consequence is a re-basing, not a removal.** What the ordered representation buys on the carrier side is **decidability** — a decidable section, content-addressable storage, and the identification-sorting test — and not the existence of the construction, which Burroni's tier supplies without it.
> `canon-sound` is unaffected: it is owed for the decidability, for the merger's commutativity, and now as a field of the arity interface.
> The claim narrowed is the _warrant_ recorded for the ordered representation on the carrier side, and narrowing a load-bearing warrant is a refutation.
> **Ruled (owner): the narrowing stands, and it is propagated** — the carrier-side bullet above is restated to it, and the superseded form is tombstoned in [[metatheory/guards]].
> What is narrowed is the warrant and nothing else: the ordered representation is not in question, and every other reason for it is untouched.

So the price of "one construction, two arity bases" at the circuit rung is one obligation: `Rigid.canon-sound` for shapes.
The construction's own atomicity does not generalize — the linear kit's multiplication is one structural recursion with one inductive graph, while grafting is a composite of ten operations each of whose graphs threads the next.
That asymmetry is what an arity interface has to absorb, and the shape that absorbs it is the universe-style presentation below.

### The arity interface, universe-style

The arity layer is presented as a **universe**: a type of codes, an interpretation, a unit code, a substitution former, and the interpretation equivalences the laws are stated over.
The published form of that record is a generalised operad universe [@hewer-2025-hott-operads, def. 9] — codes; an interpretation family; a unit code with an equivalence to the singleton; a **dependent-sum code former** with an equivalence from the interpretation of a formed code to the dependent sum of the interpretations; a **representation map** sending an equivalence of interpretations to an identification of codes, with a composition coherence; and **three path-level closure laws** placing the left-unit, right-unit and associativity equivalences in the representation map's image.
Its four instances are totally-ordered finite sets (planar operads), the groupoid of finite sets and bijections (symmetric operads), the ambient type universe, and the `n`-types for `n ≥ −1`.

**Every published instance is a universe whose code _is_ its interpretation.** That is what the representation map asserts and what its derived section lemma makes precise: the codes' identifications reflect the interpretations' equivalences, and nothing more.
The consequences of gandr's codes carrying more than that are the whole content of this section.

**The parameterization varies the symmetry axis and fixes the arity-shape axis.** Planar and symmetric operads differ by the universe of codes — ordered versus unordered — while composition is always the dependent sum, which is one-output tree grafting. gandr settles ordering with `Rigid` and needs the other axis varied, so extending to circuit algebras means **replacing the former, not instantiating the parameter**.
The former to replace it with is **substitution** — the arity monad's multiplication in polynomial form, an outer code together with a code at each of its positions — and not the binary grafting the operations are written as; the graph shape is Raynor's colimit over graphs of a limit over the graph's elements [@raynor-2026-nerve], so the interpretation is a colimit of products where the operad's is a dependent sum.

The dictionary is read off the monad rather than by analogy: a code is an element of the arity monad applied to the one-point species, and the interpretation is the polynomial fibre — that element's positions.

| universe field         | planar operad             | linear kit                   | circuit kit                |
| ---------------------- | ------------------------- | ---------------------------- | -------------------------- |
| codes                  | the natural numbers       | `Path a b`                   | `Shape Γ Δ`                |
| interpretation         | standard finite sets      | the path's **edges**         | the shape's **vertices**   |
| unit code              | the singleton             | the one-edge path            | `corolla A B`              |
| the former             | finite summation          | path substitution            | graph substitution         |
| its interpretation law | the sum's fibre bijection | positions of a concatenation | `verts-graft`, generalized |
| representation map     | cardinality injectivity   | see below                    | `Rigid.canon-sound`        |

The interpretation is the **vertex** family at the circuit rung, not the leg family, and `Gandr.Shape.Graft.verts-graft` — grafting concatenates the vertex listings — is already the former's interpretation law at the binary rung.

**Three divergences from the published record are forced, and one of them deletes three of its fields.**

* **The codes are indexed by their interface**, not a bare type.
  This is the reason the cell shape was re-presented familially: an unindexed carrier gives an arity abstraction nothing to quantify over.
* **The unit is a family, not an element** — one unit code per generator, which at the circuit rung is the corolla family and at the linear kit is the one-edge path.
* **The positions are labelled**, so the interpretation is a family too: the positions spanning a given interface.
  Substitution needs the label to _type_ the family it substitutes, and the right-unit law needs it to name which unit goes where.
  The published positions are bare because the published codes are bare finite sets.

> **The three path-level closure laws disappear.** They exist so that the unit and associativity equivalences on positions are representable as identifications of codes, because in the unindexed setting a formed code is a _new_ code and the operad laws are heterogeneous over it.
> Here substitution preserves the index — a substituted code and its outer code span the same interface — so all three laws are homogeneous equations and nothing is transported.
> The index does the work the three laws were doing, and it is a decision the tree had already taken for an unrelated reason.

**The two fields the interface record was short are the two the universe names.** The recorded interface asks for a carrier, a unit, a multiplication spoken only through its **graph**, a heterogeneous structural equality, and six lemmas over them; the circuit kit supplies the carrier, the unit and the multiplication, and owes the graph and the structural equality.

| owed field                 | what it was                                                                                                 | what the universe says it is                                                                                                                               |
| -------------------------- | ----------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| the multiplication's graph | a first-order relation keeping a defined function out of a matchable index; nine of them in the circuit kit | the code former **together with its interpretation law** — the law's only moving part is a bijection of positions                                          |
| the structural equality    | introduced for the heterogeneous comparison, then found to be the equality the laws are stated at           | the representation map's **codomain** — the identification relation on codes, whose own coherence is the two-cell obligation of [[#Stratified univalence]] |

Both presentations answer one question: how to state the laws without the defined multiplication appearing where something must match on it.
The graph answers it by never computing; the universe answers it by making the laws quantify over positions instead of over constructions.

**The representation map does not survive the rung change, and that is the interface's real price.** In its published form it is refuted at gandr's rungs, and the two refutations differ.
At the linear kit the surplus a code carries over its position family is the **order**, and the refutation holds even against a hypothesis strengthened to label-preserving bijections: two paths using the same edges in the other sequence have isomorphic labelled position families and are distinct codes (`Gandr.Arity.Universe.Refute`).
There the repair is to enrich the interpretation to the ordered labelled positions, and it is carried out — `Refines` is the enriched map, `inj` the representation map over it, `inj-sound` the converse — so the code relation and the enriched interpretation determine each other.

> **The refutation and the repair bracket the enrichment exactly, and the bracket is tighter than "order is needed".** The refuted hypothesis supplies a full bijection — two maps _and_ both round trips — plus label preservation, and does not determine the code.
> The sufficient hypothesis supplies two maps, **no** round trips, plus labels and order.
> At this kit order is therefore worth more than invertibility, which is a measurement of what the interpretation has to remember rather than a restatement that it has to remember something.

At the circuit kit the surplus is the **vertex ordering**, and no interpretation of a graph sees it: `Gandr.Shape.Graft.merge-swap-apart` and `corollas-swap-apart` exhibit shapes isomorphic as graphs and decidably distinct as terms.

> **So at the circuit rung the representation map cannot land in propositional equality; it lands in the code setoid's relation, and what decides that relation is `canon-sound`.** The map _is_ `Rigid` at this rung — not "merges with" — and specifically it is the canonicalization-soundness obligation.
> The universe presentation therefore does not dissolve that obligation; it **promotes** it, from a debt recorded in a header to a field the typechecker asks for before the interface can be inhabited at all.
> This is "ordering is a section" meeting the arity interface, and it is forced by the representation discipline rather than by the choice of ambient.

**What the presentation costs, measured at both kits.** The linear instance is landed in `Gandr.Arity.Universe`, where the answer was already known and so serves as the control: if the presentation is not cheaper there, it will not be cheaper where the answer is unknown.

* **The unit and associativity laws are the existing lemmas, read off the graph by functionality.** The left-unit law _is_ the linear kit's left-unit lemma; associativity runs on its associativity lemma.
  The presentation reuses the kit rather than replacing it.
* **The whiskering lemma has no counterpart.** Whiskering is a compatibility the _binary_ multiplication needs — moving one operand past the other's interface — and substitution at all positions never moves an operand past anything.
* **One lemma is the whole new price**: substitution distributes over concatenation, which is what stating associativity over positions costs in place of stating it over witnesses.
* **The interpretation law is inhabited as an equivalence and not as a map.** The three laws consume only its pairing direction, so the splitting direction is what makes the instance satisfy the published field as stated rather than a weakening of it; it is written as a view throughout, so a consumer learns the decomposition by matching rather than by inverting a defined function in an index.

The code relation carries its own three laws, which the presentation is what forced: it is the relation the laws are stated at and the relation the representation map lands in, so reflexivity alone does not suffice.
All three are available heterogeneously and none of them is available homogeneously — the endpoints must stay distinct variables, or a constructor match has a reflexive equation to delete, which is the same wall that defers decidable equality on the linear carrier and not a second one.

At the circuit kit, eight of the interface's thirteen fields are inhabited against the tree as it stands, and the five that are not all descend from one construction.
The interpretation is the vertex family **indexed by the profile it spans**, which is what typing the substituted family requires — and it is derived rather than declared.
A position with its profile read back by lookup and a position with its profile in the index are the same positions addressed two ways, so the refinement belongs to the **listing** and not to the shape: `Gandr.Shape.Graph.Occ` is the position family with its element carried, and the arity interpretation is its instance at the vertex listing.
Neither addressing replaces the other, and the rule is stated where the generic family is defined: reasoning about **order**, or relating two positions of one listing, wants the bare position — reachability, adjacency, incidence, rank and connectivity are all of that kind, and profile indices there carry nothing — while **typing a datum at a position** is the one job that needs the element in the index.
The interpretation side of the owed half is already built at the binary rung — `verts-graft`, `verts-merge`, `verts-lwhisk`, `verts-preplug`, `verts-wire-in`, `verts-cap-in` and `verts-wires-in` — where the graph-of-multiplication route's nine relations, with totality and functionality for each, have none of their twenty-seven pieces built.

> **The relocation is a simplification on the side the cost was measured on, and neutral on the side the cost actually sits.** "A multiplication spoken only through its graph is one relation in one kit and nine in the other" is a statement about the **witness** discipline, and the witness discipline is exactly what the interpretation law replaces: one bijection of positions in place of nine relations threading one another's indices.
> It buys nothing on the **construction** side, whose cost is the listing algebra — matchings, insertions, exchanges — which no presentation of the interface touches.

Two movements in that construction cost are known and they run in opposite directions.
Substitution's **outer** recursion becomes trivial: grafting onto a pure wiring is a well-founded composition of matchings, where substituting at a wiring is the identity because a wiring has no vertex to substitute at.
Its **base case** becomes harder: attaching a graph where a vertex's ports were published closes a block of sources against a block of sinks — a two-sided closure, which is what creates a wheel, and which grafting never needs.

That closure is now scoped, and the scoping moved the question rather than answering it as posed.

> **The closure is not the substitution route's price; it is the interface's.** `sub` is a total field of the record, so a circuit instance owes the closure whichever former is primitive — substituting a graph at a vertex whose ports are wired back to each other is not reachable from grafting and merging, both of which leave the block's two sides alone.
> So the auxiliary count cannot decide "primitive or derived", because the operation it was counting is common to both routes.
> What the count does decide is what each route owes _beyond_ it.

**The closure decomposes into the merger and one new operation.** Substituting at the head vertex is merging the replacement beside the rest — `merge` places `Shape A B` beside `Shape (B ++ Γ) (A ++ Δ)`, disconnected, by `merge-apart` — and then closing the two blocks one wire at a time.
Each closure deletes one source and one sink and joins whatever they were attached to, so the residual is a single-wire operation and its block form is that operation iterated: the technique `lwhisk` and `wires-in` already use, and the reason no permutation enters.

```agda
-- what a closure returns: a wiring, AND the vertexless circles it closed
record Closed (Γ Δ : List Ob) : Set ℓ where
  field
    wiring : Match Ob Γ Δ
    loops  : ℕ

-- the residual proper: delete the source at `i` and the sink at `j`, and join
-- whatever each was attached to. `match-insert`'s inverse, and `wire-in`'s
match-close
  : Insert Ob x Γ Γˣ → Insert Ob x Δ Δˣ → Match Ob Γˣ Δˣ → Closed Γ Δ
```

**It is three cases and it does not recurse.** `match-remove i` reads the source's partner and `match-unhit j` reads the sink's hitter — both landed, both structural — and the rebuild is one existing operation: `match-insert` when the source ran through to some other sink, `match-cap` when the source was already cut, and neither when the source ran to _that_ sink, which is the case that closes a circle.
The cut case cannot close one, because a source has exactly one end: if the source at `i` is cut to `w`, then `w` is spent and does not hit `j`.

> **So the closure retires the kit's only well-founded recursion rather than adding to it.** `match-comp` is well-founded because its fused clause recurses on a matching `match-unhit` produced rather than on a subterm, and that accessibility bookkeeping is what makes its associativity hard.
> Composing two wirings is merging them and closing the shared interface, so the closure subsumes `match-comp` — as a value on the nose, since `Match` has one term per wiring and two operations computing one wiring compute one term.
> The Agda induction saying so is not written, and until it is, the tree would carry two compositions.

**The count, against grafting's nine — which is ten.** The recorded list omits `match-unhit`, which joined the chain when the cut did.

| chain                                | auxiliaries | already built | new |
| ------------------------------------ | ----------- | ------------- | --- |
| grafting, as it stands               | 10          | 10            | 0   |
| substitution, as merger plus closure | 16          | 12            | 4   |

Substitution reuses six of grafting's ten (`wire-in`, `match-insert`, `insert-shift`, `insert-swap`, `match-remove`, `match-unhit`) and six of the merger's, which the interface owes anyway (`merge`, `wires-in`, `append-regroup`, `insert-widen`, `cap-in`, `match-cap`).
Its four new ones are `match-close`, its shape-level lift past each published port block, the block iteration, and the `Closed` record.
The four it does not need are grafting's own: `preplug`, `lwhisk`, `match-lwhisk`, and `match-comp` with its accumulator.

**And the closure's degenerate case has no term in the carrier, which is what the scoping was for.** A vertex whose output is wired back to its own input is a legal shape; a bare wire of the same profile is a legal code; substituting the second at the first closes a circle with no vertex, no leg and one edge.
`Gandr.Shape.Graph`'s carrier excludes exactly that object, deliberately and by name.
Checked rather than argued: over closed interfaces, a shape with no vertex has no edge either (`Gandr.Arity.Universe.Circuit.no-circle`, with `selfloop` and `bare-wire` beside it).

> **The exclusion is of a _derivation_, and what the closure needs is an _adjunction_ of the object.** Deriving a circle would need a cup, which inhabits hom-sets that must stay empty; a circle consumes no port and produces none, so it disturbs neither the downward condition nor the finiteness that rests on it ([[#The rung, identified]]).
> That distinction is the whole width of the repair, and the standing ruling against adding a cup to make an operation total is untouched by it.

**Ruling (owner): substitution is the primitive former, and grafting is derived from it.** The reason is not the auxiliary count, which is common to both routes.
Route two inhabits `Arity.sub` only where no strand closes — a weakening of the interface rather than the interface — so the presentation the whole arc is for is never reached.

| route                      | the closure           | `Arity.sub`                           | grafting associativity         |
| -------------------------- | --------------------- | ------------------------------------- | ------------------------------ |
| **substitution primitive** | total, at `Shape × ℕ` | inhabited as stated                   | a corollary                    |
| grafting primitive         | partial, at `Shape`   | inhabited only where no strand closes | a goal, on the existing ladder |

Grafting is derived by substituting into a two-corolla series shape, which is written as an ordinary term and needs no grafting to build, so grafting associativity follows from the monad law.

**Ruling (owner): the closure's circles are counted, not discarded** — `Code a b = Shape a b × ℕ`, a shape together with its number of closed components.
This is [@raynor-2025-functorial, def 3.4] rather than a gandr device: a Brauer diagram is a pairing together with that count, and gandr's codes were its open fragment.
Discarding them is not the neutral option it looks like — it sets $"tr"("id"_X)$ to the unit at every colour, which is `𝟙` where `𝟘` was available, and leaves no term able to witness the other answer ([[proof-engineering#The substrate tower]]).

The count sits at the **code** and not in `Match`, and the two are isomorphic, since a shape has one wiring at the bottom.
Carrying it in `Match` would put a number inside every listing-algebra lemma — `match-insert`, `match-cap`, the braid, `insert-swap-coh⁴` — none of which can close a circle, while at the code exactly the operations that close one touch it.

> **And the placement is load-bearing rather than merely cheaper, which the source settles rather than suggests.** The downward category is _defined_ as the subcategory of **open** morphisms, and its hom-finiteness is stated with openness as the reason: since its morphisms are open, each hom-set is finite [@raynor-2025-functorial, def 3.14].
> $"Match"$ is that hom-set, so carrying the count there would delete the stated hypothesis of the finiteness that retired the old restriction ladder ([[#The rung, identified]]), while $"Shape" Γ Δ$ is infinite already and the count changes nothing.
> The code placement is also the source's own decomposition rather than a gandr convenience: every Brauer diagram is a horizontal sum of an open diagram and a **scalar**, the scalar being $(∅, k)$.

**What the ruling costs the downward condition is nothing, and saying why keeps the two questions apart.** The condition is a property of the _pairing_, which the count does not touch, so consequences 1 and 3 stand verbatim.
What is true, and narrower, is that the open fragment is closed under **composition** — which is consequence 3, and is why $k = 0$ sufficed while the former was grafting — and **not** under contraction.
The trace is not a downward-category operation but the wheeled prop's, and it is what generates the count.
`Shape` is untouched, and so are the incidence, `WheelFree`, `Acyclic`, the cell record and every refuter standing on them; `Pos`, `lab`, `one` and `one-elim` are unchanged because a circle carries no vertex; the unit code counts zero, the merger adds counts, and substitution adds what the closure returns.

Two things the ruling owes, and one prediction it makes:

* **an agreement lemma** between the derived grafting and the built `graft`, without which `verts-graft`, the two unit laws and the merger's incidence theorems do not transfer, and the tree carries two compositions;
* **the count law** — that the two sides of associativity close the same number of circles — which is the source's $k$ formula as an induction, and is the one place the count is paid for rather than carried;
* **the ladder-depth claim is a prediction**: the closure's own exchange law — closing two wires in either order — is predicted at arity three by the recorded debt-arity law, which is the braid and is proved, and its falsifier is a cap case that reaches the four-layer coherence instead.
  That coherence is held too, so the falsifier costs the prediction and not the route.

> **Two alternatives were assessed and are not taken; both are recorded so neither is re-proposed.** Mapping circles to zero inside the _full_ Brauer category is unavailable at this base before it is weighed at all: **it needs a zero morphism**, being an ideal construction in a linear setting, and gandr's carrier is combinatorial at `Set`.
> Reaching for it means enriching in pointed sets or abelian groups, which is a larger change than the cup it was to pay for.
> Weighed anyway, for the case the enrichment ever exists: it does restore hom-finiteness with cups present, since pairings on a finite interface are finite once the count is killed — but it spends consequences 1 and 3 and therefore the rung, it leaves whether the circuit-algebra nerve theorem still covers the result an open question rather than an assumed one, and $δ = 0$ is the same collapse as discarding, pointed the other way.
> The prior question is whether standalone cups are ever needed at all: what a cup buys is **duals**, and duals are what Int supplies without touching the carrier, so only something demanding a sink-sink pairing _in the carrier_ would force one.
> And the modules-over-the-Brauer-properad presentation of modular operads [@stoll-2022-modular-brauer], which reaches the Feynman transform as a cobar construction and carries no downward restriction, is not a substitute for the rung's nerve theorem: it works in differential graded vector spaces over a field of characteristic zero, over _unrooted_ modular operads, and its result is a bar–cobar resolution rather than a nerve theorem for a monad with arities.
> Its "no downward restriction" is vacuous for this question, because an unrooted setting has no direction to restrict; it stays adjacent input for a Feynman-transform or Koszul-duality angle, and for nothing here.

The setoid translation the presentation rests on is mechanical: h-sets become carried cells one dimension up (the lawless-setoid discipline), the ambient univalence becomes the layout univalence map with its two-cell coherence (which stops being a tidy correction and becomes a **prerequisite** — the representation map's coherence is exactly what that two-cell obligation states), truncated finiteness becomes decided finiteness, and the one genuinely different piece is the free construction, where the literature's higher inductive type is replaced by the inductive-family-over-a-lawless-setoid answer of [[#The ambient-primitive policy]].

Filed beside it as a direction, not a decision: **internalizing** the arity universe — operations as data the way descriptions are data, the rule algebra as an internal object, and univalence at the operation layer.
That is the one concrete payoff a composite doctrine-and-term monad was ever contemplated for (univalence for certificates, not only for codes), reached here without one; the applications are metaprogramming over rewrite systems, signature-to-optimizer, and transport of a program along an equivalence of operations.
Its three standing risks: an object-language universe is a trusted-surface question with a plausible but unchecked Tarski exemption; the representation map makes the two-cell univalence obligation load-bearing; and the free construction's non-HIT substitute is recorded but unexercised at this scale.
The fourth risk it once carried — that the graph former's coherence laws might not stay finite — is retired: the former is a monad multiplication, so its laws are three whatever it multiplies, and the index makes them homogeneous.

Two senses of "arity" are in play in the literature and must be kept apart at every citation: a **monad with arities** (a property relative to a dense subcategory, consumed by the nerve theorem) and an **arity monad** (the shape of a cell's source, consumed by the carrier, wanting cartesianness).
A strongly cartesian monad has canonical arities, so the second implies the first — but at gandr's rung and base the implication is unavailable in the needed direction, and the two senses come apart: sense one is available at `Set` and carries the nerve; sense two fails at `Set` and is bought back by ordering.
Cartesianness claims in this literature must always be read with their **base**: the circuit monad is "strongly cartesian" in the ∞/groupoid-based framework [@chu-haugseng-2021-segal] where symmetry quotients are homotopy orbits, and that does not transport to `Set`, where the monad takes honest coinvariants for graph automorphism groups and an explicit ported pullback counterexample bites.

## Symmetry, ordering, and the price

**Σ-freeness is a property of the representation, not of the rung.** At the circuit rung the symmetric group does _not_ act freely, constitutively: graphs with cycles or parallel edges have nontrivial boundary-fixing automorphisms, the monad quotients by them, and the contraction's semantics _is_ that quotient.
At gandr's ordered representation Σ-freeness is true _by construction_ — and the reconciliation is published: graphical structures defined over ordered graphs are Σ-free by construction and are thereby a **different notion** from the classical symmetric ones [@batanin-berger-2017-polynomial] [@chu-hackney-2021-rectification].
Nothing gandr needs rides on Σ-freeness of the rung: the nerve runs on arities, which tolerate automorphisms.

> **Ordering is a section.** Symmetric objects, symmetric algebra, ordered representation: the stored form is a canonical linearization chosen for storage — a section of the quotient, never a planarization of the theory.

**The identification-sorting test** (proved in the carrier, per operation): an identification that comes from the _presentation_ — which of two ports is named first — is already quotiented by the canonical wiring and is free, by `refl`; an identification that comes from a genuine _graph automorphism_ — the swap of two parallel components — cannot be expressed by an ordered representation and lands on `canon-sound`.
The cut's port symmetry is the first kind, absorbed three times — at the wiring, at the edge listing, at the incidence — in three different currencies, none of them canonicalization's; the merger swap is the second kind — it is **false on the nose** for the ordered carrier, as it must be, because the vertex order is representation content, and the identification of isomorphism _classes_ in the source is exactly what `Rigid.canon` owes.
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

**Decidable equality comes from edge-determination, not from discreteness.** A shape homomorphism is determined by its action on the finite edge set — the published corollary is about graphical maps [@hackney-robertson-yau-2015-properads], and this tree proves the analogue for its own homomorphisms rather than importing it; with actions carried as tabulated data, pointwise agreement implies propositional equality by ordinary induction, hom-sets are finite, and morphism equality is decidable.
The determination lemma needs a no-isolated-vertices hypothesis whose necessity is exhibited (the arity-zero corolla is a legitimate cell shape and an isolated vertex at once); equality of _shapes_ is decided outright, with the residual h-level condition closed by uniqueness of identity proofs on the **colours alone**, supplied constructively by their decidable equality; decidable equality at _isomorphism_ remains genuinely open pending an enumeration.
This replaces the essential-discreteness argument entirely, needs no planarity, and is what makes per-degree naturality checking finite in [[#Stratified univalence]].
A binding constraint on this layer from the reserved sized/irrelevant-index direction: **content-address on the erased skeleton** — if content-addressing does not erase irrelevant indices, two propositionally equal nodes get distinct identities, hash-consing loses sharing, and conversion diverges from node identity; erasure before addressing is mandatory, not optional, and it binds the canonicalization layer _before_ any size discipline lands.

**The flat arena is a published object.** $"size" 𝟙 = 1$, $"size"(c ⊗ d) = "size" c · "size" d$, $"size"(c ⊕ d) = "size" c + "size" d$, values indexed by $"Fin"("size" c)$, offsets $⊗"ix" b i j = b·i + j$ and $⊕"ix"^r a j = a + j$: the cardinality homomorphism, implementing the bridge diagram's three-step evaluation ($Σ_t ∘ Π_π ∘ Δ_s$) with the indexing made arithmetic.
The arena _is_ the right bipermutative category `Σ′` whose objects are the natural numbers and whose morphisms are the symmetric groups — Def I.2.4.18, tight with both structures permutative by Prop I.2.4.23, right bipermutative by Ex I.2.5.8 [@johnson-yau-2024-bimonoidal] — with the row-major index formula verbatim at (I.2.4.19).
Its strictification theory fixes exactly how far strictification reaches, and the reach is in the definition rather than inferred: both associators, all unitors, one unit-side symmetry, and one distributor become identities; the two symmetries and the **left** distributor survive, the latter as the explicit permutation (I.2.4.21), and the right distributor is already the identity on offsets.
The code universe's reversible-language reading is likewise prior art: the reversible language for the rig structure comes with a published minimization of its relation set and two corrections to its definitions [@carette-sabry-2016-semirings].
The arena fixes a canonical layout that truncation-based treatments deliberately forget: gandr gains computable offsets and pays the visibility of the choice — ordering-as-section one layer down.
The locators above are verified at source; what remains marked is the author-pair trap, since the bimonoidal monograph and the PROPs monograph share a second author and are one citation-slip apart — [[metatheory/citation-hazards]].

**The coherence verdict.** Must the structural coherence family of a tree-shaped edit calculus be imposed as mathematics, or is it presentational?
Presentational, in two halves, and the proof is landed:

* _the hierarchy dissolves as one theorem, not a family_: a rigid arena map — extent-preserving (`ext`) and offset-fixed (`fixed`) — composes and whiskers to rigid maps, and any two rigid words with a common source agree at value grade.
  The associativity and unit generators are rigid, so **every** diagram built from that hierarchy commutes, at every code, with no cell imposed; the pentagon and triangle are instances.
  No uniqueness of identity proofs, no transport: a coherence cell here is an equation between _functions_, so the recast stays clean without K. Dissolution's cost is one theorem plus four closure lemmas, **independent of dimension** — which is the cost claim that matters, against a generated family whose members grow exponentially;
* _what carries content is proved directly_: the two symmetries and the left distributor are not rigid; their obligations (the sum hexagon, distributor naturality) are discharged by induction through the arena's computation rules.

The **completeness half is declined, with a reversal condition** — see [[metatheory/guards#the declined completeness half]]. gandr consumes only soundness (congruent words realize equally, cheap _because of_ the dissolution theorem); the engine's normal-form test is a decidable under-approximation of replay-equivalence, and a normal-form-equal, replay-divergent pair is a kill signal, not a soundness hole to close by theorem.
The residue after dissolution is the symmetric-group word problem in the groupoid alphabet and the full transformation-monoid word problem in the directed one; neither is owed, because both are the completeness half.

**The arena's directed generalization is warranted, priced, and shaped.** The problem first: the arena's morphism class is bijections _by construction_ (its published identity has morphisms only at equal extent), so admitting the directed alphabet's one-way generators is a request to enlarge the morphism class, and the design question is _which rung of the classical ladder the arena sits on_: offset-fixed (trivial word problem, the dissolution theorem) ⊂ monotone (the simplex category — simplicial identities, classical and convergent, epi–mono factorization as the normal form) ⊂ symmetric (Coxeter) ⊂ all functions (the transformation monoid).
Computed from the offset formulas: three of the four one-way generator classes (projections, diagonals, injections) land in the monotone rung; the codiagonal alone forces the transformation monoid — co-cartesian structure on ordered sets is not order-preserving.
The decision of record: **characterize as the clone, build as the factorization system** — `Rigid` splits as `RigidMono ∩ RigidEpi`, the split _explains_ the existing record rather than displacing it, factorization systems are already the development's idiom at five layers, and building the split _is_ building the simplex category's epi–mono normal form, so the decision procedure arrives with the construction.
The warrant is **soundness**, not completeness: one-way generators fall outside the rigid class definitionally (they change the extent), so without the enlarged class every one-way coherence obligation returns as a per-generator grind — which is precisely what the dissolution theorem exists to prevent.
The published decomposition of the target ladder (planar / symmetric / clone as three monads on `Cat` related by distributive laws) is [@curien-2012-operads-clones]; scope the build by the directed rule layer's actual cell list, not as an open-ended redesign, and decline the clone-as-morphism-class rebuild with that reason.
A filed probe rides with the ordered-representation half: whether the weaker-than-cartesian hypotheses of the unified generalized-multicategory framework [@cruttwell-shulman-2009-generalized-multicategories] apply to $T^times$ at `Set` — if they do, the ordered-representation purchase shrinks, and canonicalization is owed only for decidable isomorphism of shapes.
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
A second measured instance from the opposite direction: the one published mechanization of polygraphs in a proof assistant reports that higher inductive types that do not compute "are not well-suited to intricate uses", could not prove functoriality of the free construction in the cubical setting, and implements zero rewriting or coherence content — the wall again, reached via HITs rather than term generation (the work is not yet named here; the locator is owed, and rides with the pending sweep that reported the datum — [[metatheory/citation-hazards]]).

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

> The nonunital circuit-algebra monad $T^times$ **has arities** $"Gr" ⊂ "GS"$ at `Set` [@raynor-2026-nerve], and a monad with arities has a dense graphical category $Θ_(T^times, "Gr")$ (the bo-ff factorization — bijective-on-objects, then fully faithful — of $"Gr" ↪ "GS" → "GS"^(T^times)$) whose induced nerve is **fully faithful with essential image exactly the Segal presheaves** [@berger-mellies-weber-2012-arities].

Read the warrant precisely: arities deliver _both halves_ — full faithfulness and the Segal characterization — so gandr's citation is this pair, **not** the source's headline theorem, which is stated for the harder _unital_ rung (where the monad does not have arities and the proof passes through a monad decomposition).
Two cautions travel with the warrant.
A Segal characterization is **not** a completeness condition — the ∞-analytic-monads line needs a further localization at fully-faithful-and-essentially-surjective maps beyond its Segal equivalence [@gepner-haugseng-kock-2022-analytic], so "Segal-characterized image" must never be silently read as "complete"; and for wheeled properads and their neighbours the published pattern is that the graph category must be _enlarged_ before the nerve is fully faithful, with the sources' own warning that neither case is a straightforward application of existing theory.
Independent corroboration that cartesianness is not on the chain: the relative-monad nerve theorem's only hypothesis is **density** of the root [@arkor-mcdermott-2024-nerve], with an explicit non-density counterexample showing the hypothesis cannot simply be dropped.
The Segal condition is **strict** — a limit over the graph's elements, a bijection, not a weak equivalence.
Monad decomposition is load-bearing here but _inside the term dimension_: the circuit monad decomposes as the merger monad over the contraction monad by a free distributive law (Lemma 7.4 of [@raynor-2026-nerve]), and decomposition is the general mechanism that creates arity candidates where a monolithic monad lacks them.

What this retires: the old restriction chain (properadic nerve restricted along the dioperad inclusion), its residual admissibility risk, and its open questions — gandr's route needs no restriction along a subcategory inclusion at all.
The chain's last unchecked step (whether the old framework's Segal-object category is literally the monograph's strict properadic one, with the pushout-of-corolla-representables computation unverified) is retired with it, not carried.
What it leaves owed, both scheduled:

* **the oriented-slice transfer**: the arities statement is made for $T^times$ on $"GS"$ and not restated for gandr's oriented $O T^times$ on $"OGS" = "GS"\/"Di"$; the transfer along the slice is routine (the slice equivalence is used in the source's own proofs) but unwritten — a paragraph, not a programme;
* **the presentation of $Θ_(T^times, "Gr")$**: the theorem needs only the category's existence; the _mechanization_ (degree, the degree-raising and degree-lowering subcategories, factorization, decidable morphism equality, the per-degree Segal check) needs its morphisms concretely, which the source does only for the unital case.

A caution that travels with the re-basing: the connected and disconnected presentations are genuinely different theories (the generating map between them is not quadratic), so properad-level results must not be expected to transfer by restriction — the rung change is a re-warrant, not a restriction.

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
The free object is named precisely: `Terms` is the **free Segal object**, monadic at `Set` — not an envelope, which is a different construction with a different source and target.
An explicit formula for it exists (a necklace-style formula, with a multi-simplicial generalization [@barkan-steinebrunner-2023-segalification]); it is stated at simplicial level throughout, so treat it as a template to strictify, relevant only if `Terms` must ever be _evaluated_ rather than merely characterized.
The bridge-diagram profile of a generator is exactly the species profile, so the description layer and the pasting layer meet without translation.
This identification is the first thing to test (its falsifier: a description needing dependency or indexing the base category cannot express); everything below assumes it.

### The site, the strata, and the fuel are one object

$Θ$ is a **generalized Reedy category** [@berger-moerdijk-2011-reedy] — generalized precisely because its objects have nontrivial automorphisms:

```agda
deg    : Θ.Obj → ℕ                                  -- Reedy degree
Θ⁺ Θ⁻  : SubCategory Θ                              -- degree-raising / degree-lowering
factor : (f : G ⟶ H) → Σ[ K ] (Θ⁻ G K × Θ⁺ K H)     -- unique UP TO ISO
```

Stratum $n$ is the shapes of degree at most $n$; the universe at stratum $n$ is the codes whose terms are supported there; **fuel is the degree** — a natural number decreasing along degree-lowering maps, so induction on it terminates structurally.
"Unique up to iso, not up to unique iso" is where the automorphism groups sit, and it is exactly what `Rigid` discharges: canonicalization turns the factorization into an actual function.
Reedy theory hands over the staged construction — a presheaf is built degree by degree through latching and matching objects with automorphism-equivariance at each stage; the per-degree new data is exactly the delooped automorphism groups [@haine-ramzi-steinebrunner-2025-reedy], with the classical bigluing results as the 1-categorical citation per that paper's own direction.
**Staged certification is Reedy induction.** One structural caution for the mechanization: the graphical category is **not** closed under finite colimits — graph substitution is not "take a pushout", and only graph-of-graphs diagrams are guaranteed colimits — so the presentation work of [[metatheory/roadmap]] must not assume general colimits of shapes.

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

The module layout, when this side is built: `Gandr.UA.Site` (the site, degree, the two subcategories, factorization — re-exporting the shape decidability), `Gandr.UA.Reedy` (latching/matching, equivariant staging, degree induction), `Gandr.UA.Sig` (descriptions as graphical species; `Terms`), `Gandr.UA.Segal` (the condition and its per-degree certification), `Gandr.UA.Equiv` (certificates and `check n`), `Gandr.UA.Descent` (the nerve citation and `ua_n`), `Gandr.UA.Transport` (transport-at, fuelled transport, cost accounting), `Gandr.UA.Colimit` (the colimit universe, coinductive certificates, continuous normalization, fenced).
Dependencies run strictly downward; `Descent` is the only module that cites an external theorem, and `Colimit` is the only one that is not decidable.

### Univalence beyond the code universe — transfer, structures, repair

Three separate things are wanted; only the first is the nerve's.

1. **The code universe is univalent** — the construction above.
2. **The ambient diagram model satisfies univalence and function extensionality**, so the other formers behave.
   The transfer theorem exists for _inverse_ diagram categories [@shulman-2015-inverse-diagrams]: univalence, funext, and the universe tower all transfer to Reedy-fibrant diagrams, with admissibility cheap for a finite-graph site.
   The gate is that the theorem's mechanism needs Reedy and injective structures to coincide, which holds when the index is **elegant** in the Bergner–Rezk sense [@bergner-rezk-2013-comparison] — and $Θ$ is _generalized_ Reedy, so the theorem does not apply on the nose.
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

**Kernel formers.** `Path` (groupoid: `here`/`walk`/`then`/`back`) and `Flow` (directed: the diagonal intro `diag`, the shared `walk` with a motive-covariance side condition, composition derived by one walk and spelled `then` as in the groupoid family, **no inversion — the refused motive shape is the symmetry shape**) land as independent primitive formers; no kernel coercion between them, because the comparison is the core-coincidence _theorem_ and a coercion would smuggle it in as an axiom.
The shared eliminator and composition spellings are the settled answer to the metatheory roadmap's open question 9 (owner decision, 2026-07-31); the surface account of the family is [[surface-language/directed-family]].
Two permanent negative witnesses guard the pair: a K-derivation must fail elaboration, and a symmetry-derivation for `Flow` must fail elaboration.
The directed word problem's honest price: the rule layer's residue grows from the symmetric group to the full transformation monoid, for which no register row and no formalized rewriting twin exists — and that price was quoted against the tree presentation, so it is re-quoted against the arena before it is spent ([[metatheory/roadmap]]).

## Exact reals and synthetic topology

A lateral track, firewalled from the minimal-kernel path: exact real computation as a **reified Abstract Stone Duality subsystem outside the frozen kernel** [@taylor-2010-lamcra] [@bauer-2008-dedekind-reals] — Sierpiński-valued semidecisions rather than booleans, lower and upper reals paired as certified Dedekind cuts, open formulae with bounded quantification over overt and compact domains, and a resumable refinement machine whose certificates an independent checker replays.
Its contact with this track is threefold: the **equipment reading** (the modal-law checklist of the exact-real fragment decomposes without remainder as the cartesian-equipment conditions of the doctrine layer, and identity of points is polarity-split exactly where spaces stop being discrete); the **temporal reading** (observation is semidecision, real equality is not semidecidable while apartness is, and backend equivalence is an observational — never geometric — certificate relation, with the cubical interval route declined-not-refuted); and **`ua_topo`**, the third instance of the univalence statement family — every certified locale-isomorphism the image of a space-code path — designed from the start as a fullness theorem over a polygraph presentation of space codes, with interaction structures [@hancock-hyvernat-2006-interfaces] at the object level, a univalent formal-topology SIP [@tosun-2020-formal-topology] as the completed rendering, and the Tietze/polygraphic tradition at the identification level.
The full design, the staged plan with its gates, the decision register, and every open obligation dispositioned are in [[metatheory/exact-reals]].

## The doctrine layer

### Three roles, kept apart

Three different objects were once fused under "the virtual-double-category machinery", and the fusion is what made the question "where does it live?" unanswerable:

| role                     | what it is                                                                                                                                                                         | side                                                           |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| **the arity kit**        | the parameter of the doctrine complex: a base plus the multiplication as a relation ([[#One construction, two arity bases]])                                                       | below both                                                     |
| **the doctrine complex** | the free telescope over a signature at a chosen arity — spheres, positions, cells, coherences                                                                                      | syntax                                                         |
| **the equipment**        | a split cartesian _fibrational_ virtual double category with chosen constructors: restriction, tabulator/comprehension, hom protype, extension fillers [@nasu-2024-internal-logic] | semantics — the thing a category-with-families projects out of |

Where the parameterized complex sits in the literature, stated so the novelty claim is made once: a polygraph/computad is the free-algebra-with-generators construction for a monad, built dimension by dimension, and the dimension-generic half of that is CaTT — finite computads are precisely its contexts [@finster-mimram-2017-catt]; the arity-generic half is `T`-multicategory theory, at dimension one in the sources.
The crossing — `T`-polygraphs, uniformly in `T` — has no counterpart in the consulted corpus; the nearest categorical vocabulary for the presented mode distinguishes **coinserters** (freely adjoining a cell at a boundary) from **coequifiers** (imposing an equation between cells) [@lucatelli-nunes-2026-freely], and in that vocabulary gandr's complex is coinserters all the way up, with no coequifiers — its structural operations are derived, never adjoined.

The category-with-families comes from the **equipment**, never from the complex: `Ty Γ` is one-sided (covariant) profunctors, context extension is the tabulator, `Id` is the hom protype, `J` is profunctor Yoneda, `Π` is extension along the display map's conjoint, and substitution is literal precomposition — split by construction, so the two axes a classical construction conflates (substitution coherence and fibre coherence) never meet.
**No change of arity base, in any order, produces a category-with-families**: a generalized-multicategory construction supplies the shape of cells, never a tabulator.
The constructors _can_ be freely adjoined — the free bifibration supplies exactly the missing pushforwards with a clean proof theory [@clarke-scherer-zeilberger-2026-bifibration] — but 2-cell equality in such free constructions is undecidable unless the base is factorization-preordered, so freely adjoining fibrational structure is an owner STOP whose entry condition is that check.
Companions and conjoints, by contrast, are free (the zigzag construction manufactures them), which closes the reflection face's standing gap by import.

The CwF is a CwF **of the positive fragment**: types depend on values, never on computations; the negative fragment is an adjoint module over it, with the shifts a tight adjunction.
Three checkable consequences: no per-type `Id` at negative types except through the shift; type dependency confined to the value zone (a type may be indexed by producers, never by consumers or commands, unless routed through a shift); transport is directed, with unrestricted composition only on the invertible fragment.
One standing kill signal rides with this: the shifts are invertible exactly when every value is thunkable and every computation linear [@munch-maccagnoni-duploids] — if any construction forces the shifts invertible, gandr's effects have been strictified by accident, and the construction is wrong rather than the theory.

### The join

Contexts are the equipment's objects, so the site and the derivation category are both contexts — there is nothing to choose, and the covariance detail is load-bearing: the nerve's target is presheaves, so the context is the site's _opposite_, and a silent op is how directedness gets lost.
What is genuinely owed is the **loose arrow** between the two context families — the semantics relation, which derivations realize which shapes — and its **tabulator is function extensionality over cellular data**: a carried family of cells in context is a proterm, and the comprehension is the device turning a family-of-cells-in-context into a path-in-context, eliminated by per-position replay, lazily.
Exhibiting the protype whose tabulation is funext is the highest-information single construction owed on this side; the nearest literature template is the syntactic lax-cones construction over finite computads (a cone over a context _is_ a context), read as a starting shape rather than a drop-in — it is stated for globular contexts, as a limit notion rather than a tabulator, and its general case is a conjecture.

### The compositional-rewriting axioms, measured

The compositional-rewriting double-category axioms [@behr-harmer-krivine-2023-fundamentals] are a testable checklist run over the **real** structures — the overlap enumerator, matching and unification, rewriting and normalization, the tracelet certificates — never a second engine; the scope of every verdict is the cell-visible convergent fragment, natives outside every claim.
Verdicts: multi-sums hold degenerate-singleton (first-order syntactic unification makes the family at most one per ordered pair per kind); pullbacks in the tight and cell layers hold strictly; horizontal decomposition holds strictly; the source is a strong multi-opfibration in discrete form; the target is a residual multi-opfibration in per-instance form, exercised exactly by redex-creating instantiations; positive globular decompositions and cellular Conduché hold strictly on the free path algebra; **the cylindrical decomposition property is open** — a distinct obligation, not a corollary, and it is what the convolution face needs beyond exponentiability.
The payoff: the universal concurrency and associativity theorems hold on that fragment by the universal proofs, with the differentials retained as adequacy witnesses.
The measured cellular-Conduché row is a **definitional match** with the discrete Conduché condition (lifting of factorizations, uniquely) [@guetta-2020-conduche], which is both an exponentiability condition and one of directed type theory's fibrancy notions — so gandr has already measured a directed-fibrancy condition on its own cell store; whether it is the exponentiability the convolution face waits on is a scheduled one-day check.
How to hold the framework: its fibrational reformulation was reverse-engineered from a naturality observation rather than posited as a doctrine, so testing it empirically per system is the intended use — and the indexed side (what the Grothendieck construction of these multi-opfibrations yields) is explicitly unexplored by its authors, a research door for the fibration-shaped reflection face.
The trigger to revisit globularity-above-the-base was precise and cheap to watch: a non-linear pattern producing a genuine (non-singleton) multi-sum family — many-out one dimension up.
**That trigger cannot fire under the linearity ruling** ([[implementation/circuit-terms#The design questions|circuit-terms-question-17]]), which refuses the pattern that would produce the family; so it is restated rather than watched, and its new form is the **per-type comonoid generalization** — a type supplying a copy is where a genuine family could reappear, and landing that supply is what re-arms the watch.

### The convolution face

Over the doctrine, vertical presheaves carry a convolution product; the representable at a rule interface categorifies the rule-algebra basis vector, and under the measured axioms the convolution of two representables decomposes as a sum indexed by the multi-sum — the concurrency theorem, categorified: the fan-out family is the coefficient set of a genuine associative product one level up [@behr-mellies-zeilberger-2023-convolution].
Adopted as **specification, not a second engine**: the categorified concurrency isomorphism is the completeness contract of the overlap enumerator, landing as differential rows in both directions.
The virtual-honest form is published — a colax convolution on presheaves over a virtual double category, defined by a coend on the multicell profunctor, with no horizontal composition assumed, strong under positive globular decompositions [@thompson-carlson-2026-exponentiable]; the unconditional floor is a theorem (every virtual double category embeds in a locally cocomplete completion with composites), and the bridge from a pseudo double category's power into `Set` to exactly this convolution is numbered in [@arkor-2025-exponentiable].
The rule algebra and its representation on states unify through the Yoneda embedding, so "compose rules" and "apply rules to states" become one operation — the kernel already embodies the operational half.

### Interchange, by layer

> Exchanging two independent things is never an equation unless you are willing to lose information; every well-behaved treatment replaces the equation by a witness whose invertibility is the design dial.

The strength is a _stratification_, and collapsing its levels is a recorded error.
The literature carries interchange at four different strengths, and the strength is the design decision: an **invertible** 2-cell (a Gray commutation — the two interleavings stay distinct, deadlock correctly unresolvable); a **strict equation** (the rejected alternative there — it manufactures a synchronized diagonal move letting two mutually blocked strategies proceed, "excess of synchronization … a defect (not as a feature!)") [@mellies-2021-template-games]; a **non-invertible lax coercion** (concurrent separation logic, where the Hoare inequality is _derived_, not postulated) [@mellies-stefanesco-2020-csl]; and **equations in a coherent congruence** (the layered games, where interleaving and true concurrency coincide) [@oliveira-vale-mellies-shao-koenig-stefanesco-2022-layered].

| level                                                    | interchange is                                                           | why                                                                                                               |
| -------------------------------------------------------- | ------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------- |
| the ambient duoidal category of interfaces               | **lax** — the laxator, by definition of duoidal                          | the physical tensor is an inclusion of admissible orderings; no normalization upgrade can invert it               |
| the shape layer — `Shape` as a **duoid** in that ambient | **an equation**, and proving it is the scheduled next-unit target        | shapes are defined operations with proved equations, not presented structure                                      |
| certificate composition                                  | **structurally lax**; imposing it strictly is _wrong_, not merely coarse | the deadlock counterexample above; duoidal coherence fails in general (the certificate arc's produoidal analysis) |

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

The adopted hole theory is **monoidal context theory** [@roman-2023-monoidal-context]: diagrams with holes over a polygraph, where a context in the **sequential** fragment is a list of interface pairs — one per hole; the full theory is built to allow incomplete morphisms of any two-dimensional shape, and the normalization monad is what permits them.
Each monoidal category induces the **cofree produoidal category** of its monoidal spliced arrows (the splice–contour adjunction, its Theorem 3.3.10), whose free normalization is the normal produoidal category of monoidal lenses (its Theorem 3.5.3); the same lenses carry a second, independent universal characterization as **the free message theory** (its Theorem 4.5.9), which belongs to the message-passing application and is not the context theory's construction.
The identification with the substrate is exact: the holes are the vertices, the vertex listing is the context, and the algebra of such things is a normal produoidal category; the ambient duoidal structure above is its instance at gandr's interface category.
**A hole at the monoidal unit severs a diagram.** The splice distinguishes a plain morphism from one with a hole in the monoidal unit, because the latter hole splits the morphism in two parts — disconnection _is_ a hole with an empty interface — and normalization is what sews the two parts back into one.
A hole layer that must see disconnection therefore lives at the splice, before the sewing.
A second, independent hole theory exists for the _higher-order_ direction — diagrams-with-holes as strong profunctors, embedded lax-lax duoidally with a sequencer interpreting one-way signalling, and a Yoneda lemma pinning the notion down given channel–state duality [@hefford-wilson-2024-profunctorial] [@wilson-hefford-2026-strong-profunctors] [@wilson-hefford-hoffreumon-2026-supermaps]; it is conditional on wanting higher-order cells (a circuit with a hole taking another circuit), and the convergence of both lines on duoidal-structure-over-profunctors is the signal that the hole layer's home is the equipment's loose direction.

### Doctrine odds and ends that are load-bearing

* **Cartesian double theories.** The 2-dimensional fragment of a shape block is a presentation of a cartesian double theory with product-preserving lax-functor models; test the cartesian law against the _framed_ pairing–projection bijection, expect iso-strong and never strict; a bare virtual double category is not cartesian in that sense, and until the reflection face's cartesian-fibrational notion is reconciled with the double-theory notions, every verdict names which notion it tested.
* **Variance.** The comonoid-style settings pay an opposite-category operator with polarity machinery through every judgment; gandr's internal language deliberately excludes it, and the variance layer is a priced future axis, not a current structure ([[metatheory/ambient-and-primitives]]).
* **Aggregation is not functorial; data migration is** — the source's own words, and its title is the programme, not a property: _Functorial Aggregation_ models aggregation _inside_ the functorial data-migration ecosystem (disjoint unions of conjunctive queries as parametric right adjoints), while observing that inserting a row gives no map between the aggregated results [@spivak-garner-fairbanks-2021-aggregation].
  Aggregation needs a commutative monoid on the target — multisets, the free one, are the canonical target — and every quantity accumulated over a derivation (fuel, cost, counters) lives in that non-functorial regime, where symmetry re-enters through the cost model rather than the type theory.
* **The recurring étale condition.** Homotopy quotients that add arrows keep symmetries carried and positions decidable [@kock-2012-data-types]; discrete-opfibration conditions make dependent sums compute coproducts; exponentiability of polynomials in the virtual setting [@fujii-lack-2025-familial] — one invariant seen three times; name it and enforce it wherever a decomposition, sum, or product is formed.
* **Virtual-honesty formulations worth keeping.** Bicategories are virtual bicategories with all composites (via cocartesian 2-cells), so the virtual reading and the relative-monad machinery share one formulation of "composite".
  And the game-semantics layer stack does **not** map onto gandr's universe stratification, nor onto module or abstract-type sealing — sealing is generative.

## The certificate algebra

**Certificate identity is replay-equivalence** — the replayed-not-trusted discipline promoted to the definition of when two tracelets are the same transformation; composition ships as two operations (unconditional on the invertible fragment; acyclicity-gated on the directed band, declining with the cycle as diagnostic).
The measured finer alternative is declined knowingly, not unknowingly: the asynchronous-games treatment quotients reschedulings by "same induced bijection on step indices" [@mellies-2021-template-games] (the same device as the residual line's ancestor function on redex indices — one construction in both programmes), and gandr's identity is strictly coarser on two axes, forgetting the induced permutation and never comparing the two paths; the same programme makes a deliberate at-most-one-2-cell ("locally posetal") choice for one of its own bicategories, so the coarse discipline has a named precedent as a design choice — not a theorem.

**The normal form.** The certificate normal form is closure under abstraction isomorphism (content addressing — already the store's identity), trivial-unit insertion and removal (empty-path elimination — the path calculus's unit laws; the empty path is not an edge, so unit insertion has no tile of its own), and **shift equivalence**: two adjacent cell applications at disjoint positions with trivial overlap commute.
The contract, at full strength and correctly scoped:

* normal-form equality **decides shift equivalence** — an iff, from the uniqueness of primitive factorization and the shift-equivalence characterization [@behr-2019-tracelets] [@behr-kock-2021-tracelet-hopf];
* shift equivalence **implies** replay-equivalence — sound;
* the converse is **constructibly false** in gandr's own codebase: replay-equivalence is pure proof-irrelevance beyond replayability, so two confluence certificates joining by different routes are replay-equal with different primitive multisets.

> The normal form is a performance fast path, never a decidability result; replay-equivalence is already decidable by boundary equality plus two replays, and the normal form answers its _cost_ question.

**Shift equivalence has a name and a literature, and a hazard at the circuit rung.** The quotient — adjacent applications at disjoint positions commute — is the **trace monoid** of the independence relation "disjoint support", and the canonical schedule is its normal form; the identification is exact rather than analogical, since Mazurkiewicz trace languages are precisely symmetric monoidal languages over monoidal distributed alphabets, where a generator carries a set of locations and independence is disjointness of those sets, and the serialization square exhibits the free monoid on generators (one runtime, every order distinct) quotienting onto the trace monoid by erasing that runtime [@earnshaw-sobocinski-2023-string-diagrammatic-trace-theory].
Nothing is adopted; what the identification buys is that gandr's quotient is a known object with known decision procedures rather than a bespoke device.

> **The hazard is that disjointness stops implying independence once matches must be convex.** In convex diagrammatic rewriting, two rule applications on **disjoint** sets of hyperedges can block one another, because applying one creates a directed path that destroys the other's convexity [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-ii, ex 45]. gandr's shift quotient is TCB-adjacent — it is what the `cells_equal` normal-form fast path decides — so at the circuit rung it is warranted only on a fragment where a match cannot be made non-convex, and establishing that fragment is a guard obligation rather than a scheduling note ([[implementation/circuit-terms#circuit-terms-spike-07]]).
> Today's cell store is term-shaped and the hazard is vacuous there; it becomes live at exactly the moment the cell alphabet changes, which is why it is recorded before the change rather than after.

What the fast path buys beyond comparison: compression to a primitive multiset plus a minimal schedule, and **coherence-cell elimination** — the 3-cell relating the two orders of two independent steps becomes _definitional_ under shift equivalence, discharging a whole class of interchange obligations by normalization instead of carrying them.
That is also the principled semantics for the declined horizontal-composition sugar of the boundary language ([[surface-language/higher-cells#The boundary language]]): accept exactly on disjoint positions, where the two sequential readings are shift-equal.

**Certificate verdicts are three-valued.** A declined round-trip or composition check leaves a certificate **stuck, not refuted**: the store must distinguish _holds_, _refuted_, and _declined-within-budget_, or it silently conflates "not yet composable within budget" with "not univalent" — the verdict discipline that budget-gated checking forces.

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
One comfort from that line, marked as an inference rather than a source statement: what fails there for disconnected diagrams is _termination of the rewriting strategy_, not finiteness of the class, and since every gandr cell has a non-empty left-hand side, its generically disconnected certificates would be **boundary-connected** — landing in the tractable regime rather than the divergent one.
Verify before relying on it.

**Two axes, bridged not merged.** Two independent axes run through this layer and the ones above it, and conflating them is a recorded error: the **symmetry** axis (coefficients: sets to groupoids, for data with automorphisms — codes and descriptions) and the **determinism** axis (exactness: Segal to 2-Segal, for composition that destroys information — tracelets and certificates).
Every Segal space is a decomposition space and the converse fails, with the sharpest counterexample being the rewriting one, where a 2-simplex cannot be reconstructed from its two short edges because composition is non-deterministic [@galvez-kock-tonks-2018-decomposition]; 2-Segal is not "multi-valued composition" — it is composition unique relative to a richer boundary.
Two cautions the source line uses without establishing — local finiteness and completeness — are carried as flags, not assumed: local finiteness is automatic for a _free_ operad's decomposition space but not in general [@galvez-kock-tonks-2018-restriction-species], and gandr's rules are not a free operad.

**The certificate layer is a decomposition space.** The tracelet algebra _is_ a 2-Segal object, and the doctrine layer above it is a double-categorical object with pullback axioms; the published equivalence between 2-Segal spaces and augmented stable double Segal spaces is the missing edge between the two layers, with the source's own redundant-data model (a 2-simplex as a 2-tracelet together with its chosen composite) being that input in S-construction vocabulary [@behr-kock-2021-tracelet-hopf].
The identification is to be _named and cited, not proved_ — verify that the measured strict pullbacks are the stability condition, establish or refuse a set-level shadow, and record the edge; scheduled in [[metatheory/roadmap]].

## Representation and performance

> Prefer representations in which the address map has bounded sensitivity under local edits.

A local edit must perturb a logarithmic number of content addresses; fan-node sharing gives no such bound.
That Lipschitz-style condition on the _addressing scheme_ is the honest salvage of the retired "perturbation is local" principle; it is load-bearing for content-addressed chunking specifically, while ordering, the arena, and the fuel stance are justified independently. gandr declines the optimal-reduction objective while learning its layout lesson: the best-known implementation's fiftyfold win came from a memory-layout change with no algorithmic change, and the frequently cited optimality result is called a **thesis** by its own authors — its theorem-level content is an invariance result, a semantic equivalence, and optimality in a cost model that counts parallel-beta steps as unit steps, with bookkeeping, garbage collection, and useless work all excluded by the authors' own statement.
Linear runs, computable offsets, chunked storage.

**The acceleration band** exploits three aligned order-independence properties — shift equivalence (logical), signature-tensor associativity (algebraic), history-independent chunking (representational) — for four workload classes: batched signature scans (advisory), overlap screening (advisory, sound-direction only), chunk-parallel replay and rehash (exact, differentialed), rule-algebra numerics (analysis band).
Signatures are computed on the canonical schedule, where they are well defined on equivalence classes; the corresponding invariance theorem — signatures are invariant under exactly the tree-like excursions, so backtracking that cancels is invisible — is the analytic twin of the unit quotient.
The antisymmetric block is the numerical shadow of the Lie bracket, so a nonzero value for a recorded-independent pair is an arithmetic alarm before any replay diverges.

> **The accelerator firewall (binding).** Accelerator results are either advisory or exact-and-differentialed; none is ever soundness-bearing; the kernel never links this band; numeric nondeterminism must be unobservable; adoption is measurement-first.

## The ambient-primitive policy

Identity here is a construction in time over an unfinished substrate — the temporal rendering, against the completed (K/truncation) and spatial (interval/cubes) renderings; the substrate is codata with weak final coalgebras, no η, label-intensional identity never observed.
The without-K discipline is binding independent of everything else [@cockx-devriese-piessens-2014-without-k]: no K eliminator, no deletion in unification, no type-constructor injectivity, no definitional proof irrelevance for identity, no interval or gluing primitive, no collapsing identity proofs because their codes are content-addressed equal; per-type set-ness by Hedberg over decidable equality and grade-discipline runtime erasure stay available and are not exceptions.
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
6. **A forced non-computational primitive at any rung** — falsifies the temporal route for gandr and reopens design; a uniqueness-of-identity-proofs collapse forced at any stratum is a STOP; and **function extensionality arriving as an axiom is a kill signal** — it must arrive as a certificate-layer theorem (tabulated pointwise certificates), never as a postulate.
7. **The shifts are forced invertible** by any construction — the effects have been strictified by accident.
8. **The coherence-debt arity law fails its scheduled test** — the interchange needs two cuts to commute, the ladder is not finite, and the four-tier policy's tier-2 coverage shrinks (the two other falsifiers of the law are in [[metatheory/roadmap]]).
9. **The monomial-to-monomial condition fails** for the construction-term normal form — canonicalization soundness loses its published route and needs another.

The retired falsifiers (real cells not simply connected; the term face needs PROP-style composition; rectification-admissibility; gandr's composition is not the Segal composition — subsumed by the presentation obligation of the graphical category) are dissolved by the full-rung substrate and its nerve warrant; they are tombstoned in [[metatheory/guards]] so they are not re-derived.

## Roadmap

The detailed queue — spikes with costs and deciders, standing obligations, open questions, the reading list, and the falsifier ledger — is [[metatheory/roadmap]].
The five headline directions, for orientation: extend to the directed case (it is forced, three ways); generalize the arena by the factorization-system route; make the four placements of the circuit algebra against the doctrine explicit in built artifacts (the term face is the gate); carry the universe-style arity interface to the circuit kit (graph substitution is the one construction the five open fields descend from); adopt the parametricity cluster's technology and take the nominal case as an aim.

## Sub-documents

* [[metatheory/roadmap]] — everything that remains, with costs and falsifiers.
* [[metatheory/carrier]] — the circuit-algebra carrier record: constructors, landed theorems, the edge identity, the palette, the incidence theorems.
* [[metatheory/directed-univalence]] — the directed statement in full: candidate comparison, alphabets, grades, guards, the kernel formers, the equipment inventory.
* [[metatheory/layout-and-coherence]] — the layout calculus per former, the coherence modules, the arena generalization detail.
* [[metatheory/ambient-and-primitives]] — the cubical contact in full (the J ledger, the verified evaluator register, the price ledger), the internalization currency table, the technology-cluster survey.
* [[metatheory/exact-reals]] — the exact-reals and synthetic-topology line: the semantic contract, the reified architecture, the staged plan, the equipment and temporal readings, and `ua_topo`.
* [[metatheory/guards]] — the do-not-reopen ledger: declined halves with reversal conditions, dissolved forks, withdrawn claims, name-collision warnings, and the code concordance to the retired records.
* [[metatheory/citation-hazards]] — locator defects, version drift, unverified reports, and per-source publication status.
