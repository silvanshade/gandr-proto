# Proposal: the metatheory, consolidated — from the sequent kernel to stratified univalence

* **Status**: Proposal (consolidation pass, 2026-07-26).
  No decision face yet; §15 carries the draft decision candidates.
  Nothing here spends frozen-core budget.
  This document is written to be **self-contained**.
  It is intended as the single design record for the metatheory arc, and it _subsumes_ the earlier proposals listed in §0.2 rather than pointing at them: where an earlier record is still right, its content is restated here; where it is wrong, the correction is stated here with the reason.
* **Provenance**: the arc that produced it ran as a sequence of design passes — the sequent kernel, levitation, identity and univalence, `ua_base`, higher cells, the tracelet algebra, the analytic ladder — followed by an adversarially-verified literature sweep (2026-07-25/26) in which every load-bearing claim was paired with a refuter that reopened the cited page.
  That sweep changed conclusions, not merely citations: two central claims came back retracted, one fork dissolved, and a criterion that had been assumed turned out to be the thing the whole design rests on.
  The consolidation is the owner-directed response to the resulting scatter.
* **Scope**: the metatheory — the Agda development under `metatheory/`, the theory crates it models (`theory-computads`, `theory-graphs`, `theory-levitation`, `theory-virtual-doctrines`, `core-sequent`), and the design record for identity, univalence, certificates, and the doctrine layer.
  It adds no crate, no former, and no kernel obligation.
* **Honesty gate**: the ∞-end of this development is classical ∞-category theory with no formalization and none claimed; every adoption below is at the 0- or 1-truncated rung.
  Claims are marked **verified** (with locator), **conjecture** (ours), or **refuted**.
  Where a claim rests on a source the arc holds but has not adversarially re-checked, it says so.
  Two claims are carried explicitly as _unverified_: the `Σ′` locator of §7.3 and the rigidity package it belongs to (§18).
* **Citation discipline**: every external work is cited with a resolvable locator at first mention (§19).
  Locators the arc has not been able to confirm byte-for-byte are marked `[locator unconfirmed]` rather than guessed. §19 also records which works still need rows in the central citation register.

_Deep reference artifact._ _§0 and §1 suffice for orientation; §14–§17 are the checkable residue._

## 0. Summary

### 0.1 The decisions

| #       | Decision                                                                                                                                                                                                                                                                                                                                                                                           | §            |
| ------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------ |
| **M1**  | **One construction, two arities.** A virtual double category and a properad are the same generalized-multicategory construction at two different arity monads — linear (free-category) and graph-shaped. The carrier stays globular; multi-arity is confined to the arity kit. This is the structural claim the whole development turns on.                                                        | §1.1, §5     |
| **M2**  | **The symmetric group must act freely.** Σ-freeness is the single load-bearing criterion, established four times independently. Rectifiability, cartesianness and finiteness are its three consequences; ordering the representation and restricting to simply-connected graphs are the two ways to buy it.                                                                                        | §6           |
| **M3**  | **Symmetric objects, symmetric algebra, ordered representation.** Ordering is a _section_ of the quotient — a canonical linearization for storage — never a planarization of the theory. The planar-versus-groupoid _fork_ is dissolved at this altitude, not chosen.                                                                                                                              | §6.3         |
| **M4**  | **Axis A stays symmetric; only Axis B is ordered.** Within-cell order is free; ordering the multi-root `⊎` (parallel-component) direction would void the bracket-vanishing oracle. This scoping is the single most important thing to keep written down.                                                                                                                                           | §6.4         |
| **M5**  | **The cell shape is the dioperad rung**: directed, multi-in/multi-out, connected, wheel-free, **simply connected**. It has three independent justifications — finiteness, Σ-freeness, and rectifiability — and is no longer a concession made for one of them.                                                                                                                                     | §4.4, §6.1   |
| **M6**  | **Rigidity is a property of the representation, not of the objects.** `Rigid` is a decidable split idempotent on a setoid — an effective quotient — with `canon-resp` _and_ `canon-sound` both obligations. The old reading ("the objects have trivial automorphisms") is false for the graphical category.                                                                                        | §2.3         |
| **M7**  | **Decidable equality comes from edge-determination, not from discreteness.** A graphical map is determined by its action on the finite edge set; hom-sets are finite; equality is decidable. This replaces the essential-discreteness argument entirely and needs no planarity.                                                                                                                    | §7.1         |
| **M8**  | **Finiteness, not symmetry, is the arena-relevant wall.** The free term set over a cell is finite at the dioperad rung and infinite from properads onward.                                                                                                                                                                                                                                         | §7.2         |
| **M9**  | **There is a set-level nerve theorem for gandr's shape, and it does not need a cartesian monad.** The polynomial-functor interpretation and the nerve theorem come apart: the first is dead for symmetric many-to-many, the second holds, is 1-categorical, and is proved by hand from Segal cores.                                                                                                | §8.1, §8.4   |
| **M10** | **Internal univalence is stratified fullness, and a fully faithful nerve says exactly that.** `ua_n` is the inverse of a bijection someone else proved — not an axiom, not a higher inductive type.                                                                                                                                                                                                | §9.1, §9.5   |
| **M11** | **The site, the strata and the fuel are one object.** `Θ` is a generalized Reedy category; stratum _n_ is degree ≤ _n_; the degree is the fuel; latching/matching is staged certification; `Rigid.canon` is what turns "unique up to iso" into an actual function.                                                                                                                                 | §9.3         |
| **M12** | **Two cost measures, not one.** `size` is value-replay cost at a fixed shape; Reedy degree is shape-search cost. The earlier records used one word — "fuel" — for both, and the conflation hid that transport at a known shape is free.                                                                                                                                                            | §9.6         |
| **M13** | **Layout identities and pasting identities are different structures on one universe.** The rig structure (`⊗`/`⊕`/distributor) governs layout and is frozen behind the scaling firewall; the species/properad structure governs pasting and is where the nerve route applies. The apparent contradiction between "keep the firewall" and "the nerve retires the presentation" was this conflation. | §1.2, §7.3   |
| **M14** | **The scaling firewall stands for the rig presentation and does not reach the pasting layer.** No nerve or Segal characterization for rig categories exists; the gating research question (is the free-rig monad cartesian?) has no published answer.                                                                                                                                              | §7.4         |
| **M15** | **Certificate identity stays replay-equivalence; `≡_N` is a cost path.** The normal-form contract is strengthened to the _iff_ its own source states, and the converse direction (`replay-equal ⟹ ≡_S`) is **constructibly false** in gandr's own codebase.                                                                                                                                        | §12.2        |
| **M16** | **gandr's `≡_S` is not Behr's, and it is not Lévy's.** It is the sequential-independence restriction on one side and Melliès's _reversible_ permutation equivalence on the other. Neither theorem may be cited as though the relations coincided.                                                                                                                                                  | §12.1, §12.4 |
| **M17** | **Interchange is never an equation unless you are willing to lose information.** Every well-behaved treatment replaces the equation with a witness whose invertibility is the design dial; imposing it strictly is _wrong_, not merely coarse.                                                                                                                                                     | §12.5        |
| **M18** | **Tight versus loose is a stratification, not a fork.** The loose object is the ambient hom; the tight object is the modality living over it. The three-way architectural fork recorded earlier is deflated to a layering question.                                                                                                                                                                | §11.4        |
| **M19** | **The crDC axioms hold on the cell-visible convergent fragment**, measured rather than assumed, with the cylindrical decomposition property open. The concurrency and associativity theorems are therefore available by the universal proofs on that fragment.                                                                                                                                     | §11.2        |
| **M20** | **Computads-as-data is not compromised.** The pathology that would compromise it is an Eckmann–Hilton/strictness artifact whose three hypotheses gandr never meets; the applicable escape hatch is non-unitality, not many-to-one.                                                                                                                                                                 | §4.3         |
| **M21** | **The multi-output term face is decided but unbuilt**, and the as-built cell grammar is single-continuation. Everything in §6 governs where the design is going; nothing in it is a statement about the current crate.                                                                                                                                                                             | §3.2, §3.3   |
| **M22** | **Prefer representations in which the address map has bounded sensitivity.** The earlier "perturbation is local" formulation is refuted as stated and salvaged as a Lipschitz-style condition on the _addressing scheme_, not on the rewrite rules. Optimality is declined; layout-first engineering is not.                                                                                       | §13          |

### 0.2 What this document replaces

The arc's design record is currently spread over eight proposals written at five different stages of understanding.
This document is the consolidated successor.
Each row states what survives, what is corrected, and what is retired.

| Superseded record                       | Survives here                                                                                                                                           | Corrected or retired                                                                                                                                                     |
| --------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| the sequent-kernel proposal             | the polarized command IL, cuts as redex sites, fusion by completion, closures as rewrites (§3)                                                          | nothing; the kernel record is stable. Its consumer-side single-continuation shape is now stated as a _fact about the metatheory's scope_ (§3.2)                          |
| the levitation proposal (and addendum)  | the staged description ladder, the first-order fragment, faces as free-monad term pairs, bridge-diagram arities, polarity-sorted decoders (§4.1)        | the bridge diagram is now read as the _graphical-species profile_ rather than only as a container arity (§9.2)                                                           |
| the identity/univalence proposal        | without-K, `Path`/`here`/`walk`, `Equiv` as carried coherent certificates, the temporal reading, the composability boundary (§2, §10)                   | "univalence is a per-stratum theorem" is unchanged in substance and now has a mechanism and a citation (§9); "fuel" is split into two measures (§9.6)                    |
| the `ua_base` proposal                  | the division of labour, the semantic-currency rule, the grade smoke alarm, the K-free witness discipline (§9.9)                                         | the scaling firewall is **re-scoped** to the rig presentation only (§7.4); the completeness warrant changes from an η campaign to the nerve theorem (§9.9)               |
| the higher-cells proposal               | the keyword ladder, named 2-cells, declared 3-cells, sphere-typed boundaries, the filler ban, shape signatures and `Model` (§4.2)                       | `Model∞` over globular carriers is no longer speculative — it is what the landed Agda `Category`/`Functor` records _are_ (§4.2)                                          |
| the VDC-reflection record (via addenda) | virtual honesty, cell variance metadata, replay-equivalence as certificate identity, two-mode composition with the acyclicity gate (§11.1)              | nothing retired; the crDC ladder above it is now measured (§11.2)                                                                                                        |
| the tracelet-algebra proposal           | the crDC ladder, the convolution face, the normal form, the bracket oracle, the signature alarms, the growth firewall, the acceleration band (§11, §12) | the `≡_N` contract is strengthened and re-scoped (§12.2); the residual-theory lineage is new (§12.4)                                                                     |
| the analytic-ladder proposal            | the two axes, the cardinality reading of `size`, the arena's published identity, the distributor correction, the dialogue audit, the citation defects   | the root ladder is superseded as the organizing device (§11.4); the planar/groupoid fork is dissolved (§6.3); the "groupoid level is forced" reading is corrected (§8.4) |

**Three earlier claims are withdrawn outright**, and are recorded as withdrawn rather than quietly removed:

1. **The naive root ladder.** Varying the root along the symmetry axis fails at the first step: the bijections-only inclusion into `Set` is not dense, so the nerve theorem for relative monads does not apply to it.
   The corrected form (rungs as pairs `(A, Φ)` with the root a cocompletion) is coherent but is no longer what organizes this development — `Θ` is (§9.3).
2. **The HIT-free symmetry escape.** "Carry the choice in the witness and the symmetry rung becomes reachable without a higher inductive type" is not established, and three independent sources close the set and setoid routes to the groupoid of finite sets and bijections.
   What replaced it is not an escape but a _different question_: gandr does not need that groupoid, because decidability arrives from edge-determination (§7.1) and equality of representations from `Rigid` (§2.3).
3. **"Planarity does not cost cocommutativity."** The argument for it was a non sequitur, and the sources point the other way.
   The design consequence is M4: within-cell order is free, the parallel direction stays symmetric.

## 1. The shape of the whole thing

### 1.1 One construction, two arities

The claim that organizes everything else is small enough to state in a sentence.

> A **virtual double category** and a **properad** are the same construction — a generalized (`T`-)multicategory — at two different arity monads.
> The linear (free-category) monad gives the first; a graph-shaped monad gives the second.
> Both monads are cartesian, and they are cartesian for the _same reason_: the symmetric group acts freely.

This is why the doctrine machinery and the univalence machinery are not two programmes that must be reconciled.
They are one telescope, parameterized by an arity kit:

```text
                        ┌─────────────────────────────────────────────┐
                        │  Gandr.Complex — the telescope              │
                        │  spheres · positions · cells · coherences   │
                        │  multi-ary at the base, globular above      │
                        └───────────────────┬─────────────────────────┘
                                            │ parameterized by
                        ┌───────────────────┴─────────────────────────┐
                        │            the arity interface              │
                        │  carrier · units · _++_ · Cat · Same · kit  │
                        └────────┬──────────────────────┬─────────────┘
                                 │                      │
                  ┌──────────────┴───────────┐   ┌──────┴────────────────────┐
                  │  Gandr.Arity.Path        │   │  Gandr.Shape.Graph        │
                  │  LINEAR — snoc paths     │   │  GRAPH-SHAPED — graphs    │
                  │  free-category monad     │   │  free-dioperad monad      │
                  │  ⇒ virtual double cats   │   │  ⇒ properads / dioperads  │
                  └──────────────────────────┘   └───────────────────────────┘
                                 └──────────┬───────────┘
                                            │  both cartesian, because
                                            ▼
                                 ┌──────────────────────┐
                                 │  Σ acts FREELY (M2)  │
                                 └──────────────────────┘
```

**Why the carrier stays globular.** It is tempting to make the coboundary many-to-many at every dimension.
That is the wrong place for it.
Multi-arity is needed at exactly one dimension — the cell shape.
Above it sit rules and coherence fillers, whose boundaries are _parallel pairs_ of the level below, which is what a globular coboundary already is.
Pushing lists through every dimension pays for a generality only the base consumes and commits the carrier to a shape where every direction composes — which is not the _virtual_ line this development is on.
The Agda carrier module records this decision in its own header, and it is the reason `Gandr.Graph` looks like an ordinary ∞-graph despite M5.

**Why the licence is not merely convenient.** What makes generalized-multicategory theory apply at all is that the arity monad be cartesian.
The free-category monad is cartesian.
The free-dioperad monad on **ordered** graphs is cartesian, and it is cartesian precisely because ordering the representation makes the symmetric-group action free.
So "virtual double categories and properads are two instances of one abstraction" is Σ-freeness read a second time — the same criterion §6 is about — rather than an engineering convenience.

### 1.2 One universe, two structures — layout versus pasting

A conflation runs through the earlier records and is worth dissolving before anything else. gandr's description universe carries **two independent structures**, they answer different questions, and they have different identity theories.

|                    | **Layout structure**                                                                                             | **Pasting structure**                                                                                            |
| ------------------ | ---------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| What it is         | the symmetric **rig** `{𝟙, ⊗, ⊕, δ}` on the closed, variable-free fragment of the code grammar                   | the **graphical species** profile — which `(m, n)` generators exist — and the pasting of generators along graphs |
| What it answers    | _where does a value's cell live in the flat run?_                                                                | _what composites of generators exist, and when are two of them the same?_                                        |
| The invariant      | `size : Code → ℕ`, the cardinality homomorphism; offsets `⊗ix b i j = b·i + j`, `⊕ixʳ a j = a + j`               | Reedy degree `deg : Θ.Obj → ℕ`, the number of vertices/edges                                                     |
| Its identities     | permutations of positions; the Laplaza-shaped coherence family                                                   | graphical maps; the Segal condition; the nerve                                                                   |
| Published identity | Yau–Johnson's bipermutative `Σ′`; the reversible language Π                                                      | `Θ = Γ(Gr↑di)`; the properadic Segal presheaves                                                                  |
| Identity machinery | presented calculus in the metatheory as a completeness warrant; canonical permutation certificates in the kernel | the fully faithful nerve; descent (§9.5)                                                                         |
| Firewall status    | **frozen** (M14) — no nerve theorem for rigs exists, and the gating question is unpublished                      | **open to the nerve route** — this is where §9 lives                                                             |

**Consequence, stated once.** The earlier records contained what read as a contradiction: one said "keep the scaling firewall, because the nerve route for gandr's universe is unbuilt research", another said "the nerve retires the presentation".
Both are correct about different structures.
The firewall governs the _layout_ identities; the nerve governs the _pasting_ identities.
Neither subsumes the other, and no future result about one automatically transfers to the other.

**Consequence for `size` and `deg`.** They are two ℕ-valued gradings doing two jobs, and M12 follows: `size` measures a value at a fixed shape, `deg` measures the shape.
Transport pays `deg` to find the shape and `size` to replay at it.

### 1.3 The stack

Read bottom-up; each layer is stated in the section named.

| Layer                          | What it is                                                                                                                                                          | Where it lives                                                             | §   |
| ------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------- | --- |
| **L0 — ambient**               | ∞-graphs with proof-relevant lawless equivalence. There is no `SET` here, there is `SETOID`; `Rigid` is what buys set-like behaviour back, per object, as structure | `Gandr.Graph`, `Gandr.Setoid`, `Gandr.Category(.Instances)`, `Gandr.Rigid` | §2  |
| **L1 — operational substrate** | the polarized sequent kernel: CBPV in sequent form, cuts as redex sites, consumers first-class, rewrite cells at the seam                                           | `core-sequent`, `theory-computads`                                         | §3  |
| **L2 — cellular data**         | descriptions as data; generators, named 2-cells, declared 3-cells; the free-monad face encoding; sphere-typed boundaries                                            | `theory-levitation`, `theory-computads`                                    | §4  |
| **L3 — arity kits**            | the linear kit and the graph kit; one interface, two instances (M1)                                                                                                 | `Gandr.Arity.Path`, `Gandr.Shape.Graph`                                    | §5  |
| **L4 — the criterion**         | Σ-freeness, and the scoping it forces                                                                                                                               | design-level; enforced by L3 and by the representation                     | §6  |
| **L5 — representation**        | decidable equality by edge-determination; finiteness by simple connectivity; the flat arena                                                                         | `Gandr.Shape.Decidable`, `Gandr.Rigid`, `Gandr.Arena.*`                    | §7  |
| **L6 — the site**              | `Θ = Γ(Gr↑di)`, the Segal condition, the nerve                                                                                                                      | `Gandr.Nerve`, `Gandr.UA.Site`                                             | §8  |
| **L7 — univalence**            | signatures as graphical species; terms as the free Segal object; Reedy strata; `Equiv`; `ua_n`; fuelled transport                                                   | `Gandr.UA.*`                                                               | §9  |
| **L8 — doctrine**              | virtual honesty; the crDC ladder; the convolution face; cartesian double theories; the growth firewall                                                              | `theory-virtual-doctrines`                                                 | §11 |
| **L9 — certificates**          | tracelets, the normal form, the bracket oracle, replay, the residual-theory lineage                                                                                 | `theory-computads`                                                         | §12 |

**The temporal reading (§10) is not a layer.** It is the _stance_ the stack takes toward its own incompleteness: identity is a construction in time over an unfinished substrate, and every ingredient of that stance now has a landed artifact rather than a metaphor.

## 2. The ambient — SETOID, not SET

### 2.1 The carrier and its weakness

Every structure in the metatheory is built on **∞-graphs**: the coinductive presentation of a globular type, read internally, with an ∞-map as its morphism.
`Gandr.Graph` is the ambient category — initial and terminal objects, coproducts, products, the discrete and codiscrete inclusions of `Set`, the globes, the exponential, and the disc telescopes.

`Gandr.Setoid` equips a carrier's cells with reflexivity, transitivity and symmetry, and **carries no laws**.
That is weakness by default and it is the honest floor: a setoid is the _lawless_ proof-relevant equivalence, so it needs no strictness mark and makes no claim it cannot back.
Layers that want laws state them one dimension up, as cells, where they can be witnessed rather than asserted.

`Gandr.Category Ξ` is composition-and-identity structure **on** a carrier, not a bundle containing one.
Its laws are cells: associativity does not say the two bracketings _are_ equal, it supplies a 2-cell between them which a consumer must transport along explicitly.
Two consequences follow, and both are structural rather than stylistic:

* A hom-setoid is literally the carrier's coboundary at two objects, read off by projection, so two structures over the same carrier are comparable without a coercion.
* A strict law would have to be an equation in the ambient theory, which would silently promote the setoid to a set.
  Stating laws as cells is what keeps the development inside SETOID, where its results actually hold.

### 2.2 What that costs, stated once

The development does not have SET.
Equality of cells is a structure a carrier supplies, not a proposition the ambient theory hands over.
A result proved here is a result about setoids unless it says otherwise, and that is a real restriction on what may be claimed:

* **Quotients need not be effective.** A relation does not automatically come with a canonical representative, and comparing representatives does not automatically decide the relation.
* **A map need not respect an equivalence** unless shown to.
* **Yoneda is an equivalence of setoids, not an isomorphism of types.** The landed `Gandr.Profunctor.Yoneda` proves `Pronat (hom-pro 𝒞) P ≃ Wedge P` as _pointwise value cells_ — round trips at each object and at each 1-cell — never as an equality of the records, which this development has no way to compare and deliberately never does.
  In a setting with SET one would go on to conclude the two types are equal; here that step does not exist, and the pointwise statement is the whole result.

Keeping the equivalence a named, projected structure rather than ambient justification is what makes the restriction _checkable_ instead of assumed.
`Gandr.Category.Instances` completes the move by constructing `SETOID` as a category like any other, so that a statement quantifying over categories genuinely has the ambient in range, and "the full subcategory of SETOID equivalent to decidable-equality SET" has something to be a subcategory of.

### 2.3 Rigid — the structure that buys set-like behaviour back (M6)

gandr stores a canonical linearization of an object whose semantics is symmetric, and decides semantic equality by comparing stored representations.
`Gandr.Rigid` is the structure that makes that legitimate.
Both directions are obligations:

```agda
record Rigid (A : Set) (_≈_ : A → A → Set) : Set where
  field
    canon       : A → A                                  -- canonical representative
    canon-idem  : ∀ a → canon (canon a) ≡ canon a
    canon-resp  : ∀ {a b} → a ≈ b → canon a ≡ canon b    -- COMPLETE for ≈
    canon-sound : ∀ {a b} → canon a ≡ canon b → a ≈ b    -- SOUND for ≈
    _≟_         : Decidable (_≡_ {A = A})
  decide : Decidable _≈_
  decide a b = map′ canon-sound canon-resp (canon a ≟ canon b)
```

* `canon-resp` is what makes ordering a **section** rather than a strengthening.
  Without it the stored form would decide something _finer_ than the algebra, and two semantically equal objects would be reported distinct.
  This is the failure mode §12.6 names concretely.
* `canon-sound` is what makes the representation **sound**.
  Without it the stored form decides something coarser, and distinct objects are conflated.

**What it is, categorically.** `canon` is an idempotent on a setoid, split by the elements it fixes; the splitting carries propositional equality, so the splitting is not merely a retract but an **effective quotient** — the setoid relation is recovered exactly as equality of normal forms.
The `Rigid` objects are therefore the full subcategory of SETOID equivalent to decidable-equality SET.
That is the honest content of "rigidity", and it is **not** the old reading.

**The old reading is false.** "Rigid means the objects have trivial automorphisms" does not hold for the graphical category, whose automorphism groups contain the symmetric groups on the input and output legs, and which is only a _generalized_ Reedy category precisely because those automorphisms are nontrivial.
Rigidity is a property of the representation, stated as structure.

**Where it is load-bearing.** Three places, and they are the same design decision seen three times: it is what makes ordered storage sound (§6.3), it is what makes the generalized-Reedy factorization an actual function rather than an existence statement (§9.3), and it is what the `⊎` multiset instance needs so that the symmetric algebra of §12.3 survives contact with a sorted representation.

### 2.4 The landed Agda substrate

The following are typechecked under `--safe --without-K` (with `--guardedness` where coinduction is needed), imported by the strict gate root:

| Module                        | Content                                                                                                        |
| ----------------------------- | -------------------------------------------------------------------------------------------------------------- |
| `Gandr.Graph`                 | ∞-graphs and ∞-maps; `𝟘`/`𝟙`/`⊕`/`⊗`; discrete and codiscrete; the globes; the exponential; disc telescopes    |
| `Gandr.Setoid`                | the lawless proof-relevant equivalence, plus conversion to the standard-library bundle                         |
| `Gandr.Category`, `.Functor`  | categories, functors, natural transformations; coherences as witnessed 2-cells                                 |
| `Gandr.Category.Reasoning`    | the combinator suite the standard library does not supply                                                      |
| `Gandr.Category.Instances`    | `SETOID` as a named object, with purpose-built homs at the carrier's universe level                            |
| `Gandr.Profunctor`, `.Yoneda` | two-sided profunctors, dinaturality, restriction; the hom-profunctor correspondence in the value setoids       |
| `Gandr.Arity.Path`            | the linear arity kit: snoc paths, witnessed concatenation, the graph-of-multiplication relation and its kit    |
| `Gandr.Rigid`                 | §2.3, with the effective-quotient reading                                                                      |
| `Gandr.Arena.*`               | the flat layout algebra: codes, offsets, bounded values, generator rigidity, the coherence verdict (§7.3–§7.4) |

Three conventions are settled and worth recording because they are easy to re-litigate:

1. **No prelude facade.** The standard library is imported directly and repackaged per-module through qualified private re-exports.
2. **No custom reasoning syntax.** A category's hom is a genuine standard-library setoid bundle — the standard library's equivalence structure is exactly as lawless as this development's and never truncates — so the bundle plus multi-setoid reasoning covers every chain.
   The multi-setoid form is the right one because it takes the setoid as a _datatype parameter_, hence recovers it by unification; one bundle serves every dimension, since any level's coboundary is itself an ∞-graph.
3. **Solvers on demand.** Reach for a standard-library solver first; if none fits, build it as a prerequisite of the first proof that wants it, and until then leave the obligation by hand _with a code note naming the solver that should discharge it_.

## 3. The operational substrate — the polarized sequent kernel

### 3.1 Why the kernel shape is a metatheory concern

The kernel is a polarized System-L command IL: producers, consumers, and commands `⟨p ∣ε c⟩` as first-class arena-resident data, with the frozen call-by-push-value core as the source and typing calculus and a static focusing translation between them.
Four of its properties are what make the rest of this document possible.

* **Redexes are at the cut, so overlaps are shallow.** A rewrite cell's left-hand side is a cut between a constructor pattern and an operation frame; no rule ever searches a term tree.
  Critical-pair enumeration is therefore tractable where tree rewriting needs full subterm traversal, and it is why the crDC suite of §11.2 can be run at all.
* **Consumers are first-class, so the seam is visible.** Under continuation-passing the same overlap hides behind a lambda.
  This is what makes fusion a derived 2-cell with a certificate rather than a pass.
* **Strategy is a per-cut polarity orientation.** Positive cuts fire the producer-side binder first, negative cuts the consumer-side one; evaluation strategy becomes an orientation choice on cells rather than a global language property.
* **Multi-conclusion contexts have a home.** The linear consumer zone is where a multi-conclusion reading lives, and it is the declared growth point for multi-output (§3.3).

**Fusion is Squier completion on cut seams.** Surface rewrite members elaborate to oriented command cells; overlaps at cuts are bona-fide critical pairs; a budgeted completion loop synthesizes derived cells whose certificates are the pair of joining paths, differential-tested against the two-step composite and **replayed rather than trusted**.
Two honest limits are permanent and scope every claim downstream: natives are opaque (a primitive command has no seam a cell can see through), and non-linear overlaps fan out into families rather than a single canonical fused rule. §11.1 explains why the second is a theorem rather than a shortfall.

**Closure conversion is an in-IL rewrite** at the shift boundary, normalizing to a form in which no free variable escapes the environment; the resulting adequacy statement upgrades the compile-versus-evaluate differential to a theorem on the closure fragment.
The named proof debt is unchanged: confluence of the environment-capture rule is modulo environment reordering, so the Agda face needs a permutation quotient to sit in the convergent slice — which is a `Rigid` instance in the sense of §2.3, and should be built as one.

### 3.2 As-built, the cell grammar is single-continuation (M21)

This must be stated plainly, because several conclusions in §6 read as claims about the current engine and are not.

Verified against the crate:

* `CmdPat` has exactly **one** variant, a cut carrying its polarity, with a producer half and a consumer half.
* `ConsPat` is a **linear spine**: an operation frame carries its remaining producer arguments and a _single_ return continuation; a return-side constructor frame carries a _single_ return continuation; the spine terminates at the top-level consumer.
  There is no multi-consumer variant.
* Every face, argument list, position and variable list in the rewriting crate is a positionally-indexed **ordered** sequence.
  There is no permutation, orbit, or canonical-labelling machinery anywhere in it.
* The double-pushout inheritance is **nominal**: no interface object, no span, no pushout complement exists in code.

Three consequences:

1. **Multi-output interfaces are unrepresentable today, not merely unused.**
2. **Within-cell ordering costs nothing to adopt**, because there is no symmetry present to give up.
3. **The shift-equivalence relation currently has empty extension** on the as-built grammar, so the normal-form work of §12.2 is forward-looking rather than discharged.
   A future session must not mistake a vacuous pass for a discharged obligation.

### 3.3 The multi-output term face is decided but unbuilt

The narrower statement matters as much as the fact.
The _cell-pattern layer_ is single-continuation as built; the **multi-output (destination-passing) term face is a ratified design direction** with a design pass scheduled and nothing constructed.
The sequent layer already carries the type shape — a consumer list on cut-adjacent constructs — while every construction site emits zero or one element and every consumption site reads the first.

So the questions §6 answers are live for a ratified, scheduled feature rather than a hypothetical, and the scoping minute (M4) becomes _more_ urgent, not less: it must be written before the term face is designed rather than after.

### 3.4 The localization move, seen four times

One pattern recurs across otherwise unrelated parts of the design, and naming it saves re-deriving it.

> Where a global gluing property fails, localize the choice and restrict the global operation.

| Setting                 | The global property that fails                           | The localization                                                                                          |
| ----------------------- | -------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| evaluation strategy     | confluence of the producer/consumer binder critical pair | polarity orientation **per cut**                                                                          |
| certificate composition | dinatural entailments do not compose in general          | unconditional on the invertible fragment; the directed band composes only through an acyclicity gate      |
| loose composites        | composites of loose arrows need not exist                | _virtual_ — composites appear as multi-cells and as opcartesian coend quotients with universal properties |
| interchange             | the exchange law is not an equation                      | a witness whose invertibility is the design dial (§12.5)                                                  |

The polygraph story is the rewriting face of a phenomenon that also appears categorically and proof-theoretically; the composition gate is its correctness side.

## 4. Cellular data — descriptions, cells, and computads

### 4.1 The description ladder

A datatype's description is a first-class value, so generic operations are ordinary programs over descriptions, and the same artifact serves the cell layer, the matching engine, and the reflected judgment layer.
The ladder is staged and each stage is additive:

| Stage | What exists                                                                                                            | Prerequisite                                      |
| ----- | ---------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------- |
| **0** | descriptions as host values; generic map/equality/serialization/derivation over them; faces stored untyped but checked | none                                              |
| **1** | a closed, typed code universe with a trusted decoder; typed faces                                                      | one universe bump, dependent Σ, large elimination |
| **2** | descriptions reflected as gandr data; generic induction and the free monad exposed as library surface                  | stage 1                                           |
| **3** | full levitation: the code universe described by its own code                                                           | universe stratification                           |

The canonical shape is the **tagged description**: an enumeration of constructors × a first-order code per constructor.
The code grammar's fragment is `{1, var, ×, σ}` plus additive decorations — a grade slot on field codes, an atom-abstraction code for binder fields, and attribute slots erased by decoding.

**Decidable equality of codes is load-bearing**, not a convenience: it is what content-addressing interns on and what the matching engine compares.
The first-order fragment is chosen _because_ it keeps this; higher-order codes are excluded from the fragment rather than deferred.
Per-type set-ness by Hedberg over decidable equality is the supplier here, and never the ambient rule (§2.2).

**Faces are pairs of open terms in the free structure over the signature** — elements of the free monad on the description functor.
The typed statement is a dependent pair over the signature, which is exactly why typed faces need large elimination.
Nothing about the encoding changes between stages; only its checking moves.

**Codata descriptions are the same codes under the other decoder.** The first-order codes are containers, and containers have both fixpoints: the initial-algebra decoder lands in the positive value universe, the final-coalgebra decoder in the negative computation universe.
Two decoders, one code grammar, polarity-sorted from birth.
Intensionally this yields only _weak_ final coalgebras — bisimulation is not definitional equality — which is consistent with the no-η codata stance and is one of the places the temporal reading (§10) has real content.

**Multi-output arities are bridge diagrams.** An operation `(X_a)_{a∈A} ↦ (Y_b)_{b∈B}` with `Y_b = Σ_{i∈I_b} Π_{j∈J_i} X_{s(j)}` is presented by `A ←s— J —π→ I —t→ B` and computed as `Σ_t ∘ Π_π ∘ Δ_s`.
Two things follow, and the second is new here:

* The Π-layer (one operation's named result tuple) and the Σ-layer (aggregating several contributions into one destination) are **different things**, and the Σ-layer requires a commutative monoid on the target — unrestricted fan-in is not free wiring.
  This is the content of the linear multi-conclusion zone's discipline.
* Read one level up, the bridge diagram _is the graphical-species profile_ of a generator: `A` and `B` are the input and output legs, and their two cardinalities are the arity at which the generator sits. §9.2 uses exactly this to identify the description universe with a graphical species.

### 4.2 Cells at every dimension

The description layer carries faces at every dimension, and the surface names them.

```text
data <Name> (params)? {
  sort <S>(indices)?          -- 0-cell: a sort
  cons <C>(fields)? (: T)?    -- 1-cell: constructor (value generator)
  oper <f>(params) -> R       -- 1-cell: operation (defined symbol)
  rule <name>: lhs ~> rhs     -- 2-cell: named directed rewrite
  meta <name>: ρ ~>> ρ′       -- 3-cell: named coherence between rewrite composites
  cell …                      -- reserved: dimension ≥ 4 (parse-and-decline)
}
```

* **Names are mandatory at dimensions 2 and 3.** The cell stays content-addressed; the name is a declaration-table binding to that content.
  Names never influence deduplication or replay-equivalence.
* **The boundary language is four constructions** — rule instantiation, the identity rewrite at a term, sequential composition, and congruence in one argument position.
  It is deliberately the largest fragment whose two readings agree: engine rewrite paths on one side, `Path` composites on the other.
  Two simultaneous rewrite arguments are declined in the first cut, because that denotes horizontal composition, whose two sequential readings agree only up to interchange — and adjudicating that silently is exactly the smuggling this design refuses (§12.5 is why; §12.2 supplies the principled future semantics, on disjoint positions only).
* **Boundaries are globular telescopes.** A rule lives at a sphere over its sort; a coherence lives at a sphere over that.
  Globularity becomes judgmental — mis-glued boundaries fail at the declaration table, once — and model typing becomes one dimension-generic recursion rather than a clause per dimension.
* **The filler ban** — no blanket fillers.
  Adjoining a filler between arbitrary parallel 2-cells entails uniqueness of identity proofs without K. The law adopted is stronger and simpler: **the machine never adjoins a coherence cell the user did not declare or completion did not certify.** Declared coherences are hypotheses of the shape; machine coherences are replay-validated certificates; nothing is ambient.
* **User coherences and machine tracelets are the same species**, separated by provenance, and they interact in one valuable direction: for the generative reading of a convergent block, completion _derives_ confluence 3-cells, so a declared coherence whose boundary a certificate already fills is **discharged** — coherence computes, into a user-visible field.

**Shape signatures.** A description block in signature position _presents_ a theory and mints no carrier; the derived signature-former maps sorts to type members, generators to operation fields, rules to `Path` fields, and coherences to iterated-`Path` fields.
The record is the algebra; instances are modules.
The 2-dimensional fragment of this has a theorem-backed home as a **cartesian double theory** with product-preserving lax-functor models (§11.5); the 3-cell layer sits above that literature, which is exactly where the design already routes it.

**The ∞-graph reading is no longer speculative.** The earlier record proposed a carrier-general form interpreting sorts as positions in a globular carrier and each higher cell at the carrier's own next dimension, reconciled with the finite reading by the equation "the finite reading is the ∞ reading at the identity carrier".
That form is **what the landed `Gandr.Category`/`Gandr.Category.Functor` records are**: structure over an ∞-graph, weak by default, with laws as cells.
The Agda development is therefore the executed ∞ reading of the shape-signature design, and the surface `Model` former is its specialization — not a parallel construction.

### 4.3 The computads-as-data hazard, correctly scoped (M20)

The standing worry is that the category of `n`-computads is **not** a presheaf category for `n ≥ 3`.
That result is real and correctly cited by digital object identifier rather than through either of the two defective routes (§18).
It does not reach gandr, for reasons that must be stated with their hypotheses, because an earlier draft got this wrong twice:

* **The mechanism is Eckmann–Hilton.** The counterexample's key step is that two 2-cells commute under one composition, and the failure is scoped to _the monad of strict ω-categories on globular sets_.
* **The hypotheses are narrow and jointly required**: strictness, globular shapes, and **degenerate boundaries** — the counterexample needs no 1-cells and 3-cells whose targets are identity 2-cells. gandr's cells are pattern-to-pattern rewrite rules and meet none of the three.
* **The applicable escape hatch is non-unitality, not many-to-one.** Two presheaf subcategories are known: many-to-one polygraphs, where the target of each generator is a generator; and **non-unital** polygraphs, where the source and target of each generator cannot be identities. gandr is **many-to-many**, so the first does not apply; the second is exactly what defeats the counterexample and exactly what pattern-to-pattern rules satisfy.
  An earlier draft imported "many-to-one at every dimension" from a carrier design and misattributed it to gandr; that is corrected here.

**Residual, honestly.** Confirming the non-unitality condition against its source is cheap and still outstanding (§16 Q6); the source is not held.
The strength of the position does not depend on it — the three hypotheses are independently unmet — but the condition is the clean statement and should be checked before the higher-cells lane hardens.

**A design claim recorded as a claim.** The standing position that a polynomial encoding sidesteps the pathology entirely — a bridge diagram gives unique decomposition by construction, and the discrete-opfibration condition excludes the degenerate collapse the counterexample exploits — is structurally sound and coheres with the recurring étale discipline, but no theorem stating it was found.
It is a design claim to be discharged, not a citation.

### 4.4 The cell-shape commitment (M5)

> **C1 — the cell shape is directed, multi-in/multi-out, connected, wheel-free, and simply connected: the dioperad rung.**

The commitments the rest of the document uses, given short names because the Agda headers already refer to them and need a tracked referent:

| #      | Commitment                                                                              | Why                                                                                                                        |
| ------ | --------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| **C1** | cell shape = the dioperad rung (above)                                                  | finiteness (§7.2), Σ-freeness (§6.1), rectifiability (§6.1); satisfies the ratified multi-output direction (§3.3)          |
| **C2** | the theory is the properadic nerve theorem restricted to `Θ = Γ(Gr↑di)`                 | set-level, 1-categorical, no cartesian monad required (§8)                                                                 |
| **C3** | the representation is ordered and listed, canonicalized by content address              | edge-determined decidability plus flat-arena offsets; a **section**, not a quotient (§6.3)                                 |
| **C4** | the algebra stays symmetric — the parallel-component monoidal structure is symmetric    | preserves cocommutativity, hence the bracket-vanishing oracle (§6.4, §12.3)                                                |
| **C5** | wheels stay out of cells; feedback is confined to the term face and the completion loop | a wheeled nerve theorem exists, but wheels reintroduce symmetric-group actions on leg-free components and break finiteness |
| **C6** | never claim that the certificate normal form decides replay-equivalence                 | it decides shift-equivalence and is a _cost_ optimization; the converse is constructibly false (§12.2)                     |

**Simple connectivity is strictly stronger than wheel-freeness**, and the difference matters: what finiteness forbids is _undirected_ cycles — diamonds — not merely directed feedback.
And note the easy mistake in the other direction: simple connectivity forbids **reconvergence**, not **branching**.
A vertex may fan out to two independent downstream vertices; what it may not do is have them meet again.

## 5. The carrier decision — one arity kit, two instances

### 5.1 The three options

The question was how to carry a multi-in/multi-out cell shape on a globular development.
Three options were on the table; the third is adopted.

|       | Option                                                                                                        | Verdict                                                               |
| ----- | ------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------- |
| **A** | reuse the existing virtual-double-category complex verbatim and encode properads as data in it                | **fails** — two independent blockers, §5.2                            |
| **B** | generalize the carrier to fully multi-globular: many-to-many coboundaries at _every_ dimension                | **overshoots** — costs a great deal, buys nothing the design asks for |
| **C** | **one generic arity kit with two instances; the carrier stays globular; multi-arity is confined to the base** | **adopted**                                                           |

**Why B overshoots.** C1 puts multi-arity at exactly one place — the cell shape.
Dimensions ≥ 3 carry rules and coherence fillers whose boundaries are parallel _pairs_, which is what a globular coboundary already is.
Making every dimension many-to-many also adopts a shape that was examined and declined on its own terms: the multiple-∞-category line is _symmetric_ — every direction composes — which is not the **virtual** line this development is on.

**Why C is small.** The existing complex already factors into three layers, and only the first is arity-specific:

| Layer     | Content                                                                                                | Arity-specific?                          |
| --------- | ------------------------------------------------------------------------------------------------------ | ---------------------------------------- |
| arity kit | the path carrier, its concatenation, the graph-of-multiplication witness, the heterogeneous comparison | **yes — the only such layer**            |
| pasting   | the pasting/forest discipline and grafting                                                             | no — the discipline is generic           |
| telescope | spheres, positions, cells, coherences                                                                  | no — **already globular above the base** |

The kit was already written generically in its two parameters, so abstracting its interface is the whole job; the pasting and telescope layers then take the arity as a parameter.
The existing shape — _multi-ary at the base, globular above_ — is exactly the shape properads need.
Only the base boundary changes with the arity.

### 5.2 Why the virtual-double-category complex cannot be reused by instantiation

Two blockers, each independently fatal, and both worth recording because "just instantiate it" is the obvious first move.

1. **One output.** The complex's cell target is a _single_ loose generator.
   Properadic cells have an output _string_.
2. **Linear versus branching source.** Its source is a linear path chained end-to-end through intermediate objects.
   A dioperad's source is a _graph_: a vertex may fan out to two independent downstream vertices, and no linear path expresses that.

Note the second is **not** repaired by C1's simple connectivity.
C1 forbids reconvergence, not branching — the confusion is easy and it is the reason A looks viable longer than it is.

### 5.3 The `Web` correction

An earlier sketch proposed a `Web` construction as the many-to-many arity.
**That is wrong, and the correction unblocks rather than deepens the problem.**

Reading `Web` against what a properadic arity actually is: a web is a **forest** — each element of the output interface is produced by one code consuming a chunk of the input, assembled in parallel.
So it is many-in, **one-out per component**, composed in parallel: the operad/PROP-of-trees arity.
It has no genuinely multi-output vertex and no shared edges, which is exactly the content C1 needs.

The right second kit is the graph record of §5.4: the graphical category has **graphs as objects**, and the Segal condition says a presheaf's value at a graph is determined by its values at the vertices.
So the arity of a properadic composition _is_ a graph.

**What this dissolves.** A connectivity worry had been filed against the graph kit — that composing along a full interface is PROP-shaped and admits disconnected composition, whereas properads are connected.
That was an artifact of the wrong kit.
On graphs, connectivity is a **predicate on objects**, not a property the composition operation must be restricted to.
The corresponding falsifier stays where it belongs, at the term face (§14, falsifier 4), rather than arriving early at the arity kit.

**What survives of `Web` is its lineage**: the witness discipline it introduced — carry the graph of a partial operation as a first-order relation rather than asserting the operation's equations — is what the landed linear kit already carries, and the same discipline governs the graph kit.

### 5.4 The interface, and why it is not yet a record

The landed linear kit is `Gandr.Arity.Path`: over a set of positions and an edge family, the snoc list of composable edges is the free-category monad's carrier, concatenation is derived, and the **graph of that concatenation** is carried as a first-order witness relation with a derived kit of lemmas.

The second kit is the graph record, with the listing carried as **data in the object** rather than as a property proved about it:

```agda
record Graph : Set where
  field
    V E     : FinSet
    src tgt : E → Maybe V                 -- Nothing = a dangling leg (port)
    -- the ORDERED representation: a chosen linear order on each vertex's ports
    inp out : (v : V) → List E
    inp-iso : (v : V) → Bijective (inp v) (fibre tgt v)
    out-iso : (v : V) → Bijective (out v) (fibre src v)

WheelFree  : Graph → Set     -- no directed cycle
Connected  : Graph → Set     -- one component
SimplyConn : Graph → Set     -- Connected, and no undirected cycle either

record Cell : Set where      -- gandr's cell shape, C1
  field graph : Graph
        wf    : WheelFree graph × SimplyConn graph
```

That the ordering is _carried data_ rather than a checked condition is not an implementation detail.
The relevant literature is explicit that the cartesian natural transformation encoding a planar structure is **a structure, not a property**: ordering is data the system stores, not a condition it verifies.

**No arity interface record has been extracted yet, on purpose.** One instance does not determine an abstraction.
Extracting a record from the linear kit alone would encode that case's accidents as though they were the general shape — that positions are objects rather than lists, and that the unit is a _constructor_ rather than something the many-out kit must _derive_.
The linear kit's header records what the interface is expected to be and names that one expected non-generalization; the record is extracted when the second instance exists.

### 5.5 The telescope

The last module generalizes the existing telescope over the arity interface, so the globular case and the properadic case are one module: multi-ary at the base sphere, globular above.
The shape ports nearly verbatim; only the base boundary changes with the arity.

This is where M1 becomes executable rather than a slogan: a virtual double category is the telescope at the linear kit, a properad-shaped complex is the telescope at the graph kit, and everything above the base is shared code.

## 6. Σ-freeness — the load-bearing criterion

### 6.1 Four independent statements of one condition (M2)

> **The symmetric group must act freely.**

Four sources arrive at this from four directions, for four different purposes.
Three consequences follow, and the arc had been treating each consequence as an independent requirement.

| Source                                              | Statement                                                                                                                                                                                                                                                  | What it buys                                            |
| --------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------- |
| polynomial functors over groupoids                  | finitary polynomial functors are precisely the analytic functors preserving pullbacks strictly; in species terms, those whose **symmetric group actions are free**                                                                                         | **cartesianness**                                       |
| homotopy theory for algebras over polynomial monads | polynomial monads correspond exactly to coloured operads with **freely acting** symmetry groups, and that freeness is bought by working with **ordered graphs**                                                                                            | cartesianness, and the reason ordering is the mechanism |
| decomposition spaces in combinatorics               | for a polynomial endofunctor cartesian over the free-monoid monad, the groupoid of its trees is essentially discrete, "because the planar structure encoded in the cartesian natural transformation **fixes the automorphisms**"                           | **discreteness** at the tree rung                       |
| rectification and enrichment of ∞-properads         | dioperads and output properads are **Σ-free**, hence the symmetric flatness condition holds, hence **each up-to-homotopy object is equivalent to a strict one**; for general properads rectification **fails**, the counterexample being a `Σ₂` stabilizer | **rectifiability**                                      |

**Rectifiability is the one the arc never asked for and most needed.** gandr is a set-level, strict system, and it assumed strictness was legitimate rather than establishing it.
The rectification dichotomy answers the unasked question: at the dioperad rung, working strictly is _provably adequate_; at the properad rung it is not, and no amount of care recovers what strict models lose.
The counterexample is explicitly intrinsic — _any_ operand governing properads has such a fixed point — so there is no cleverer presentation.

**Consequence for C1.** The dioperad commitment now has **three independent justifications**: finiteness, Σ-freeness/cartesianness, and rectifiability.
It is no longer a concession made for finiteness; it is the rung at which strict semantics is sound.

### 6.2 Two orthogonal axes, and the rung table

The single most consequential correction of the earlier analytic pass was that two axes had been fused.
They are independent, and any design that conflates them is wrong.

|                 | **Axis I — symmetry**                             | **Axis II — determinism**                                                                             |
| --------------- | ------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| what moves      | coefficients: sets → groupoids                    | exactness: Segal → 2-Segal                                                                            |
| trigger         | data with automorphisms (unordered structure)     | composition that destroys information (rule overlaps)                                                 |
| named condition | _local discreteness_ of the decomposition map     | Segal: a single-vertex overlap suffices to glue. 2-Segal: the base 2-simplex is needed **as context** |
| what it costs   | _pseudo_-simpliciality — a **strictness** failure | genuine loss of composition                                                                           |
| gandr side      | codes and descriptions                            | tracelets and certificates                                                                            |

They are **bridged, not merged**: every Segal space is a decomposition space, and the converse fails — with the sharpest counterexample being the rewriting one, where a 2-simplex cannot be reconstructed from its two short edges _because composition is non-deterministic_.

**A slogan to correct.** 2-Segal is _not_ "multi-valued composition".
It is composition that is unique **relative to a richer boundary** — a pullback, not a span.

Now the rung table on Axis I. Two axes restrict independently within it, and conflating _them_ caused most of the remaining confusion:

| Rung                 | Colours (arities)                                   | Graph shape                         | Σ-free?                      |
| -------------------- | --------------------------------------------------- | ----------------------------------- | ---------------------------- |
| operads              | one output                                          | rooted trees                        | yes                          |
| **dioperads**        | **unrestricted** — the same colour set as properads | **simply connected**                | **yes**                      |
| **output properads** | every operation has at least one output             | **unrestricted** — diamonds allowed | **yes**                      |
| input properads      | dual                                                | unrestricted                        | yes                          |
| properads            | unrestricted                                        | unrestricted                        | **no, and intrinsically so** |

**Read the first two rows carefully: many-out is already available at the dioperad rung.** Dioperads have the _same colour set_ as properads; only the graph shape is restricted.
So every `(m, n)` arity including multi-output is inside C1 already.
**What properads add is reconvergence — diamonds — not arity.** This retires a long-running misreading in which "restrict to dioperads" was heard as "give up many-out".

**The mechanism, which closes a loop.** Σ-freeness holds at these rungs because _the only strict automorphisms are identities_: for dioperads because the graphs are simply connected; for output properads and operads because every component has a **leg**.
Output-positivity in a wheel-free graph forces every component to have an outgoing leg — so the "every component has a leg" precondition, previously flagged as unverified for gandr, _is_ the output-positivity condition, and it now has a name, a purpose, and a checkable form.

**Output properads are the designated fallback.** If real gandr cells turn out not to be simply connected (§14, falsifier 2), test the output-properad rung **before** abandoning strictness: it trades "no zero-output operations" for "diamonds allowed", imposes no graph-shape condition at all, and keeps Σ-freeness and hence rectification.
Cells with no outputs would have to be re-expressed — a plausible cost, since a rewriting cell with no output is degenerate anyway.
But note it does **not** move gandr toward PROPs: a PROP's extra content is _disconnectedness_, and output properads are still properads, hence connected.
Disconnectedness is Axis A, handled by keeping it symmetric (C4), not by a nerve theorem.

**One reconciliation worth citing when the words collide.** Graphical properads defined as algebras over a finitary polynomial monad in `Set` are Σ-free **by construction** (ordered graphs) and are therefore a _different notion_ from classical properads.
The authors of the rectification paper say so themselves.
Cite that alongside the polynomial-monad characterization, or two papers will appear to contradict each other.

### 6.3 What ordering is, and is not (M3)

Three things had been fused, and prising them apart is what dissolved the fork the arc had been treating as its principal decision.

|                                    | What it needs                                                | What supplies it                                                                                 |
| ---------------------------------- | ------------------------------------------------------------ | ------------------------------------------------------------------------------------------------ |
| **the nerve theorem**              | nothing about planarity or cartesianness; symmetry is _fine_ | the properadic nerve theorem, strict and set-level                                               |
| **decidable equality and offsets** | a canonical representative per orbit                         | edge-determination (§7.1) — hom-sets finite, equality decidable                                  |
| **the Hopf/Lie algebra**           | the parallel-component structure **symmetric** monoidal      | symmetric ⇒ cocommutative ⇒ the enveloping-algebra theorem, the antipode, and the bracket oracle |

> **Symmetric objects, symmetric algebra, ordered representation.** Ordering is a **section** of the quotient — pick a canonical linearization for storage — not a planarization of the theory.

This is what the sorted-multiset certificate representation already does.
**The design was right and the justification was wrong**, and the wrong justification was doing real damage: it made "planar versus groupoid" look like a fork with a performance side and a warrant side, and it made rigidity look like a claim about objects (§2.3, M6).

**Two consequences for how the fork is now recorded.**

* At the as-built layer the fork **does not exist**: there is no symmetry present to give up (§3.2).
* At the design layer it is **deferred, not cancelled**: it becomes live the moment the cell grammar grows a second consumer slot, which is a ratified and scheduled feature (§3.3).
  The inference from "unrepresentable today" to "not committed" is invalid and was made once; it is corrected here.

### 6.4 The scoping minute — Axis A stays symmetric (M4)

This is the single most important scoping decision in the document, and it must survive into whatever designs the term face.

| Axis                                         | What it orders                                     | Ruling                                                                            |
| -------------------------------------------- | -------------------------------------------------- | --------------------------------------------------------------------------------- |
| **Axis B** — order _within_ a cell           | the ports of a vertex; positions, arguments, faces | **ordered.** Free, per §3.2; this is what the flat arena needs                    |
| **Axis A** — order the _parallel components_ | the multi-root disjoint-union direction            | **stays symmetric.** Ordering it would silently void the bracket-vanishing oracle |

**Why Axis A is not negotiable.** The certificate normal form is a **disjoint union** of primitives under a **symmetric** monoidal structure, and that symmetry is what makes the induced coalgebra cocommutative, which is what the enveloping-algebra theorem runs on, which is what licenses the bracket oracle of §12.3.
The ordered-forest variant of the corresponding tree construction is explicitly _monoidal but not symmetric monoidal_, giving a **non-commutative** bialgebra.
Adopting it would look like a strengthening and would in fact break a theorem the engine depends on — the worst shape of error available here.

**The retraction that produced this ruling.** An earlier argument held that imposing an order was harmless because the relevant coproduct formula "sums over subsets without mentioning an order".
That is a non sequitur: the index set has no order _to_ mention because the basis is isomorphism classes and the union is unordered.
The formula's silence reflects the objects having no order, not robustness under giving them one.
Evidence points the other way from several directions at once — the bialgebra proof invokes cocommutativity by name, and the symmetric-monoidal ⇒ cocommutative implication is the headline of the construction gandr relies on.

**A related trap.** Some sources obtain cocommutativity _by fiat_ ("can be equipped with"), and at least one central theorem in this area is stated with no proof at all.
Re-warranting onto such a statement swaps one unproved assertion for another; check before leaning.

### 6.5 What Σ-freeness retires

Recorded so it is not re-argued.

* **The planar-versus-groupoid fork as a fork.** Dissolved at the as-built layer, deferred at the design layer (§6.3).
* **"Planarity does not cost cocommutativity."** Retracted (§0.2).
* **The polynomial-functor interpretation for symmetric many-to-many.** Dead, and ports do not rescue it (§8.4).
* **The claim that essential discreteness is what gandr needs.** Replaced by edge-determination (§7.1).
* **The reading in which many-to-manyness is what costs strictness.** It is _reconvergence_ that costs strictness (§6.2).

## 7. Representation — decidability, finiteness, and the flat arena

### 7.1 Decidable equality by edge-determination (M7)

> A graphical map is uniquely determined by its action on the **finite edge set**.

```agda
-- cite, do not reprove
edge-determined : (f g : G ⟶ H) → (∀ e → act f e ≡ act g e) → f ≡ g

_≟map_ : (f g : G ⟶ H) → Dec (f ≡ g)     -- finite edge set ⇒ decidable
_≟obj_ : (G H : Graph)  → Dec (G ≅ H)     -- via the listings, then edge-extensionality
```

Hom-sets in the graphical category are therefore **finite**, and morphism equality is **decidable**.
This is where gandr's decidable equality actually comes from, and three things follow:

1. **It replaces the essential-discreteness argument entirely.** That argument does not survive at this rung anyway: isomorphisms in the graphical category include input and output relabellings, so automorphism groups contain the symmetric groups on the legs, and the category is only _generalized_ Reedy for exactly that reason.
2. **It does not need planarity.** Nothing in the statement mentions an order.
3. **It is what makes per-degree naturality checking finite** (§9.4), which is what makes equivalence certificates finite objects rather than propositions.

**Exactly one rung above the base is essentially discrete**, and it is below gandr: the category rung, one-in/one-out.
Every rung from the operand rung upward has nontrivial automorphisms, and **the obstruction at the operand rung is not many-to-manyness — it is unordered profiles.** Many-to-manyness adds two further independent obstructions on top.
This sharpens M2: ordering is doing the work at _every_ rung, not only at the many-to-many ones.

### 7.2 Finiteness by simple connectivity (M8)

> The graphical category over a graph is a **finite** set if and only if the graph is **simply connected**.

That is the arena-relevant wall, and it is set-level and computational rather than homotopical.
At the properad rung an undirected cycle makes the freely generated properad on a _finite_ graph **infinite**, because one generator can be reused unboundedly often in a single operation.
So the free many-to-many term set over a cell is **finite at the dioperad rung and infinite from properads onward**.

**A hazard the earlier records did not carry.** The free properad on a finite graph may be an infinite hypergraph; only a "finite type" notion of finiteness is available in general.
The dioperad restriction is the counterweight, and it is the reason C1 names simple connectivity rather than merely wheel-freeness.

**What this means operationally.** If a real gandr cell is not simply connected, free term sets are infinite and the arena needs a different finiteness story — a laziness discipline, or the output-properad fallback of §6.2.
Measuring how many real cells satisfy the condition is cheap and is scheduled early precisely because it could invalidate C1 (§15).

### 7.3 The flat arena, relocated

The arena is the layout structure of §1.2, and it is a **published object** rather than something gandr derived.

* `size 𝟙 = 1`, `size (c ⊗ d) = size c · size d`, `size (c ⊕ d) = size c + size d`, with values indexed by `Fin (size c)` and offsets `⊗ix b i j = b·i + j`, `⊕ixʳ a j = a + j`.
* This is the **cardinality homomorphism** from finite sets with disjoint union and product to the natural numbers with addition and multiplication.
  It is **not** a generating function — there is no formal variable — and an earlier reading claiming otherwise is withdrawn.
* More usefully, the arena implements the polynomial's **evaluation**, not just its cardinality.
  Reading the bridge diagram as `Σ_t ∘ Π_π ∘ Δ_s`: the product offset _is_ the `Π` step, the sum offset _is_ the `Σ` step, and flattening-as-reindexing _is_ the `Δ` step.
  That is why the offsets compute — they are the three-step evaluation with the indexing made arithmetic.
* The arena is the groupoid of **linearly ordered** finite sets, not the groupoid of finite sets and bijections.
  The independent thesis treatment picks the _same two_ isomorphisms and then observes that "since everything is wrapped in a propositional truncation it does not ultimately matter".
  **That sentence is the exact price of gandr's design**: the arena _fixes_ a canonical layout that the truncated theory deliberately forgets. gandr gains computable offsets and loses the invisibility of the choice — which is M3 again, one layer down.

**The published identity.** The bipermutative category with objects the natural numbers, morphisms the symmetric groups, multiplication `m ⊗ n = mn`, and an index formula that is the row-major offset verbatim, is gandr's arena.
Its strictification theory fixes exactly how far strictification reaches: both associators, all unitors, one unit-side symmetry, **and one distributor** become identities; the two symmetries and **the other distributor** survive.
The code universe's reversible-language reading is likewise prior art, with a published minimization of its relation set and two corrections to its definitions.

**The distributor correction.** gandr's prose treats "the distributor" as one content-carrying generator.
**Only one of the two distributors is irreducible**, and under gandr's convention it is the left one: the right one is already the identity on offsets, while the left one fails at a small concrete instance.
The general fact is stated in the source, along with a defensive note correcting a published claim that _both_ distributors are identities in the matrix model.

**Unverified, and marked.** The precise proposition number for the bipermutative identity, and the rigidity package around it, were reported by a scout whose adversarial verifier died mid-run.
Do not cite either outside the repository until re-checked (§18).

### 7.4 The coherence verdict, and the firewall it justifies (M14)

The landed `Gandr.Arena.Coherence` answers a sharp question: is the Laplaza-shaped coherence family that a tree-shaped edit calculus must impose **mathematical** or **presentational**?
The answer is _presentational_, and it is proved in two deliberately different halves.

**Half one — the hierarchy dissolves as a theorem rather than a family.** Two rigid words with a common source agree at value grade.
Since the associativity and unit generators are rigid, and rigidity is closed under composition and whiskering, **every diagram built from that hierarchy commutes, at every code, with no cell imposed.** The pentagon, triangle and sum-pentagon are then _instances_, stated at their full diagram shapes so a reader can check the general theorem against the real diagrams.
Note what this does not need: no uniqueness of identity proofs, no Hedberg, no transport.
A coherence cell asserts two _words_ have equal realization, and realization is a function, so the cell is an equation between functions — not between proofs.
That is why the recast stays clean without K.

**Half two — what carries content is still proved.** The two symmetries and the distributor are not rigid, so nothing above touches them; the sum hexagon and the distributor-naturality obligation are proved directly by induction through the arena's computation rules.
Both come out structurally, because flattening makes the distributor's action a matter of which block a cell lands in rather than a diagram to chase.

**So the verdict is not that everything trivializes.** It is that _the hierarchy is free and the permutations are not_, and the obligations over the permutations are provable in the ordinary way once the hierarchy is gone.

**The firewall this justifies.**

> **The presented layout calculus is frozen at the base signature.** Every richer former obtains its path structure **compositionally** — per-former closure theorems plus description-structural congruence — never by new generator letters and new coherence cells.
> A proposal requiring a new cell class is a STOP with a written cost model.

The cost attribution is the reason: the expensive part of the earlier campaign was the _presented_ calculus, whose completeness cost scales with the **generator and cell alphabet**, not with the number of types.
Growing the universe while keeping the presentation fixed costs nothing new — more codes normalize to the same flat spine and the same permutation certificates serve.
Growing the presentation re-runs the campaign, and adding **binding** structure would escalate the word problem qualitatively.

**Why the nerve route does not lift the firewall.** gandr's layout universe is a **rig** — two monoidal structures plus a distributor — whereas every nerve theorem in the literature is stated for a _single_ monad.
A full-text sweep of the definitive rig-category reference returns zero occurrences of arities, polynomial monad, analytic, nerve theorem, Segal condition, or cartesian monad; its "Segal map" is a false friend for a different notion entirely.
The one monad-theoretic route is 2-monadic, and the source declines it explicitly as itself a coherence theorem requiring long and involved proofs plus a universes axiom.
The nearest precedent reaches a nerve theorem for a related many-to-many shape by fusing two monads through iterated distributive laws, and is explicitly not a routine application.

> **Open research question, filed rather than bet on: is the free-rig monad cartesian?** No published answer exists.
> This is the single technical question on which a nerve route for the _layout_ universe turns.

**The compositional route, per former.**

| Former                             | Path structure via                                                                                                                       | Warrant                                 |
| ---------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------- |
| finite sums over a tag enumeration | the base stratum directly (tags are positions)                                                                                           | the sum spine _is_ the flat normal form |
| finite products                    | the base stratum directly (positions multiply)                                                                                           | the multiplicative layer, paid once     |
| nested first-order codes           | structural congruence over component isomorphisms                                                                                        | free — congruence is already present    |
| dependent Σ with finite fibres     | closure theorem: base isomorphism plus fibrewise isomorphisms give an isomorphism on the total; finite fibres are tuples — **no funext** | the finite-container payoff (§10.5)     |
| genuinely infinite positions       | **leaves the base stratum** — tabulated pointwise certificates                                                                           | §10.5, act two                          |
| recursive codes                    | **leaves the base stratum** — bisimulation-shaped carried certificates                                                                   | §9.7                                    |
| binder fields                      | nominal congruence; a path calculus _under a binder_ is a STOP pending its own design pass                                               | firewall                                |

**The early-warning instrument.** A one-integer conserved quantity — a net crossing count that every coherence cell preserves except the designated crossing cells — found the base presentation's one genuine specification gap _analytically_, before any proof failed.
The firewall inherits it as its test: **when a closure theorem for a new former resists, first look for a conserved quantity separating the two sides.** If one exists, the failure is specification-level, not proof-level.
That converts "the proofs got mysteriously hard" into a diagnosable signal.

## 8. The nerve

### 8.1 The theorem gandr needs already exists (M9)

For a presheaf on the graphical category of connected wheel-free directed graphs, the following are equivalent: it is the nerve of a properad; it satisfies the **strict** properadic Segal condition; it is a strict ∞-properad in the unique-inner-horn-filler sense.
The nerve functor from properads to that presheaf category is **fully faithful**.
The same holds for **wheeled** properads at the wheeled graphical category.

Properads parametrize _operations with multiple inputs, multiple outputs, symmetric group actions, units, and associativity axioms along connected wheel-free graphs_.
**That is literally gandr's directed multi-in/multi-out cell shape, at set level, in a monograph the arc already holds.**

The Agda obligation is correspondingly small, and it is worth stating exactly what is cited versus proved:

```agda
Θ         : Category                       -- Γ(Gr↑di), on simply-connected graphs
Presheaf  : Set₁ ; Presheaf = Functor (Θ ᵒᵖ) Set

Segal     : Presheaf → Set                 -- STRICT: the Segal map is a BIJECTION
Nerve     : Properad → Presheaf
nerve-ff  : FullyFaithful Nerve                              -- cited
nerve-img : ∀ X → Segal X ↔ (Σ[ P ∈ Properad ] Nerve P ≅ X)  -- cited
```

**Cited, not proved.** gandr's obligation is only that its `Cell` embeds in `Θ` and that its composition is the Segal one.
That is the whole formalization bill for this layer, and it is deliberate: reproving a 350-page monograph is not the project.

### 8.2 The restriction argument, discharged by citation

C2 restricts the theorem to the dioperad rung, and the theorem is stated for properads.
The Segal half of that restriction is dischargeable by a chain of published results:

```text
Θ ⊂ Γ is FULL                                    (the graphical-category source)
  + the active/inert pair is an ORTHOGONAL FACTORIZATION SYSTEM
  ⇒ the inclusion is an ALGEBRAIC SUBPATTERN     (arity-approximation toolkit)
  ⇒ the inclusion is iso-Segal, so SEGAL OBJECTS RESTRICT
  ⇒ at ANY target with the relevant limits, including Set
```

Two details make this usable rather than merely quotable.
The arity-approximation toolkit is level-graded and its restriction result works for **every** target with the relevant limits — so this is not an ∞-only argument.
And the relevant Morita notion is explicitly a **set-level** notion, with `Set` named as a complete (1,1)-category, so the machinery is available at gandr's level.

The pattern structure to use is also published: on the opposite graphical category, **inert = convex open inclusions, active = refinements, elementary = graphs with at most one vertex**.

### 8.3 What restriction does not give

**Restriction gives the _Segal condition_, not the _nerve theorem_.** The corresponding Morita-equivalence results **fail** for this inclusion — correctly, because _a refinement of a simply-connected graph need not be simply connected_.
So the inclusion is not a Morita equivalence, and the equivalence-with-algebras half does not come along for free.

Two adjacent over-readings to avoid:

* A fully faithful left Kan extension does **not** imply that the extension preserves Segal objects.
* The free-Segal-object construction gandr calls `Terms` is **not** an envelope.
  It is the free Segal object of the Segal-conditions framework, **monadic at `Set`**; the envelope construction is a different construction with a different source and target.

**What actually resolved the residual risk** was a different theorem: a graph-category-invariance result showing the relevant restriction _is_ an equivalence on Segal objects, together with the rectification theorem of §6.1 which says the dioperad rung is exactly where strict semantics is adequate.
The residual hypothesis is **admissibility**, which is conditional in the rectification theorem's statement and has to be established for gandr's target or shown trivial in the discrete case (§16 Q4).

**Cautions on the framework of choice.** Everything from the polynomial-monad sections onward in the Segal-conditions paper is gated by the authors' own footnote that "this issue disappears if we replace sets by groupoids" — gandr's declined move.
The match between that framework's Segal condition on the graphical category and the monograph's strict properadic one is stated only as "presumably"; the graphical category is never listed as extendable; and it is explicitly **not saturated**, with its saturation expected to be a (2,1)-category.
Use the framework for the restriction argument, not as a blanket import.

### 8.4 What was retired (M9, second half)

**The polynomial route is dead, and ports do not rescue it.** The free-properad monad is _only weakly cartesian, due to the presence of the connected-components construction_ — stated as a claim, hedged, never proved, and with the escape hatch forward-referencing a paper that never appeared for properads.
Two independent confirmations that the defect is real and general:

* The monad formula for the closest published many-to-many construction takes **coinvariants for graph automorphism groups** — exactly the offending quotient.
* An explicit pullback-non-preservation counterexample was constructed **on a graph with ports** (two vertices, one port each, two parallel inner edges, an automorphism group containing an order-two element fixing both ports).
  So the hedge "this can only happen when there are no ports" does **not** transfer, and gandr cannot buy cartesianness back by requiring ports.
  A mid-session claim that the defect was confined to portless graphs is **retracted**.

**But the decoupling is the good news.** The nerve theorem requires **no polynomial and no cartesian monad**.
"Cartesian" never occurs in a monad-theoretic sense in the monograph; both nerve theorems are proved _by hand_, from Segal cores plus graph-substitution combinatorics, and the authors say explicitly that their theorem does not fit the abstract nerve framework.
**So the obstruction blocks the polynomial-functor interpretation and does not block the nerve theorem.** "No polynomial interpretation" and "a nerve theorem exists" are compatible and both true; gandr must keep them strictly separate.

Three further separations worth keeping:

* **"Has arities" and "a nerve theorem holds" are independent.** A published monad is exhibited that does **not** have arities — stated three times and _proved_ by counterexample — and yet its nerve theorem holds.
  So the chain cartesian ⇒ arities ⇒ nerve is not a chain; each link can fail independently.
* **A set-level nerve theorem for gandr's shape exists; a set-level _cartesian monad_ for it does not**, in this literature.
  Reporting otherwise would be flatly wrong.
* **PROPs remain uncovered.** No graphical category, no Segal condition and no nerve theorem for PROPs exists in the held corpus; the authors defer combinatorial models to later papers.
  **Properads, with connected composition only, are the proved ceiling** — which is precisely the boundary the Axis-A scoping of §6.4 sits on.

**The equifibered route is retired.** Its foundational example **fails for free commutative monoids in the 1-category `Set`**, by an explicit two-element-orbit counterexample.
So it has no useful set-level shadow and does not avoid the automorphism quotient — it is defined where that quotient is already harmless.
One partial shadow is worth noting: a monomorphism is equifibered exactly when the component set is closed under factoring, which is decidable and conservative-map-shaped, but it is a condition on sub-objects rather than on free or quotient maps.

**Modular operads are not the answer either.** There is a genuinely set-level, 1-categorical nerve theorem for modular operads — but modular operads are **unrooted, not multi-output**: their operations are indexed by a single finite coloured set with no input/output split, and the graphs are undirected.
They achieve many-to-manyness by making legs interchangeable plus self-contraction.
Direction can only be smuggled in through an involutive colour set, and the claim that this makes wheeled properads a special case is asserted in an introduction, unproved there.

**Where the many-to-many nerve theorem _does_ reach gandr's level** is the circuit-algebra line: a genuinely 1-categorical, `Set`-valued nerve theorem, fully faithful with its essential image characterized by a strict, finite, pointwise Segal limit, whose objects are all Feynman graphs **including disconnected ones**, and which transports to **wheeled props**.
That is the closest published match to gandr's cell shape.
Read it with the arities razor in mind: the unital circuit-algebra monad does **not** have arities and is "nervous" in the weaker sense; what has arities is a different, lifted monad.

### 8.5 Residual obligations at this layer

| #   | Obligation                                                                                                                                                                         | Status                                                                                                                                                                     |
| --- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| N1  | write the Segal-restriction minute as a citation chain (§8.2)                                                                                                                      | hours, paper only                                                                                                                                                          |
| N2  | the graph-category-invariance and rectification results discharge the substantive risk                                                                                             | **done by citation**                                                                                                                                                       |
| N3  | **verify admissibility** — the rectification theorem is conditional on the target being admissible; establish it for gandr's target or show the discrete case trivially admissible | outstanding, ~half a day                                                                                                                                                   |
| N4  | is the opposite graphical category **sound**? **extendable**? Is its Segal-object category _literally_ the monograph's strict properadic Segal condition?                          | open; partially checked, with the unverified step being that the monograph's union of corolla images equals the pushout of corolla representables over edge representables |

An explicit formula for the free Segal object exists — an analogue of the necklace formula, plus a formula for the multi-simplicial case — and becomes relevant only if `Terms` must ever be _evaluated_ rather than merely characterized.
It is at simplicial level throughout, so treat it as a template to strictify rather than a citation.

## 9. Stratified univalence

### 9.1 The identification (M10)

The earlier record already stated the target in one line:

> **internal univalence = stratified fullness** — at each certified stratum, every semantic coherence cell is the image of a marked syntactic one.

**A fully faithful nerve says exactly that.**

```text
Nerve : Properad → Set^{Θ^op}          nerve-ff : Properad(P, Q) ≅ Presheaf(N P, N Q)
```

Every _semantic_ map of presheaves is the image of a unique _syntactic_ map of properads.
That is fullness, and gandr now has it as a **published set-level theorem** rather than as an aspiration.
The whole of §9 is the consequence.

The three ingredients of the earlier formulation land in three published places:

| Earlier ingredient                                                           | Now                                                                             |
| ---------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| "every semantic coherence cell is the image of a marked syntactic one"       | the fully faithful nerve                                                        |
| "stratum _n_"                                                                | **generalized-Reedy degree** on `Θ`                                             |
| "transport computes by certificate replay (fuel), not geometric composition" | evaluate the natural transformation shape by shape; degree bounds the recursion |

### 9.2 What a code is

**A correction to the sketch this replaces.** A description code is **not** a properad — a properad is an _algebra_.
The right identification is one level down:

* **A description is a graphical species** — a presheaf on the tiny category of finite sets, bijections, and the involution that pairs input legs with output legs.
  This is the **signature**, and §4.1's bridge diagram is exactly the `(m, n)` profile that a graphical species assigns to a generator.
* **The code's terms** are the **free properad** on it.
* **The Segal condition** is what says a presheaf on `Θ` _is_ such an algebra.

```agda
Sig   : Set₁ ; Sig = Functor (B§ ᵒᵖ) Set            -- descriptions, as graphical species
Terms : Sig → Presheaf Θ                            -- free Segal object; Segal by construction
El    : Sig → Θ.Obj → Set ; El S G = Terms S .₀ G   -- values at a shape
```

This matters for three reasons.
Equality of graphical species is far smaller data than equality of properads; the tiny base category is decidable; and it matches what a description actually is — a description describes _data descriptions_, not structures with composition.

**One site, two readings, stated so they do not drift apart.** `Θ` indexes _pastings of generators_.
A signature says which generators exist at each profile; the free properad's value at a graph is the set of terms of that shape; the Segal condition says a presheaf is such an algebra.
A description block's generator members are the generators, and its rule members are relations between terms of the free properad — that is, a **presentation**. gandr's "cell shape" (C1) is the graph shape a single rule's faces may take; gandr's "term" is a point of `Terms S` at some shape.
The _layout_ of those points is the other structure entirely (§1.2).

**This identification is the first thing to test** (§15, S-A).
Its falsifier is named: a description needing structure the tiny base category cannot express — dependency, indexing.
Everything else in §9 assumes it.

### 9.3 The site, the strata, and the fuel are one object (M11)

`Θ` is a **generalized Reedy category** — _generalized_ precisely because its objects have nontrivial automorphisms (§7.1).
That structure supplies, for free, the three things the earlier formulation needed separately:

```agda
deg    : Θ.Obj → ℕ                                  -- Reedy degree = number of vertices/edges
Θ⁺ Θ⁻  : SubCategory Θ                              -- degree-raising / degree-lowering
factor : (f : G ⟶ H) → Σ[ K ] (Θ⁻ G K × Θ⁺ K H)     -- unique UP TO ISO
```

* **Stratum _n**_ is the full subcategory of shapes of degree ≤ _n_; the universe at stratum _n_ is the codes whose terms are supported there.
* **Fuel is the degree.** It is a natural number, it decreases along degree-lowering maps, and each shape has finitely many edges — so induction on it terminates.
  The productivity ladder's budget rung becomes _structural_ rather than measured.
* **"Unique up to iso", not up to _unique_ iso**, is where the automorphism groups sit — and it is exactly what `Rigid` exists to discharge.
  `canon` picks the representative; `canon-resp` makes the choice sound; so **the generalized-Reedy factorization becomes an actual function in gandr, not an existence statement.** §2.3 and §9.3 are the same design decision seen twice.

Reedy theory then hands over the staged construction directly.
A presheaf is built degree by degree through **latching** and **matching** objects:

```agda
L : (n : ℕ) → Presheaf Θ → Set        -- what degree-n data is forced by degree < n
M : (n : ℕ) → Presheaf Θ → Set        -- what degree-n data must be consistent with
stage : ∀ n X → L n X → X.₀ⁿ → M n X   -- with Aut(G)-EQUIVARIANCE at each stage
```

The equivariance condition is **where symmetry lives**, and `Rigid.canon` is what makes it computable.
**Staged certification is Reedy induction.**

**This has a theorem, and the right citations are not the obvious ones.** The relevant modern treatment gives Reedy extensions and latching/matching as a bigluing pullback, and — after the classical operadic Reedy work — states that the inclusion of one degree stage into the next is a Reedy extension whose complement is the **coproduct of the delooped automorphism groups** over the components of the new objects, with explicit Kan-extension formulas, and that the whole functor category is the limit of its stages.
**The automorphism groups are exactly the per-degree new data**, which is this section's claim, now with a theorem behind it.
For the **1-categorical** case, that paper's own remark directs the reader to the classical bigluing results rather than to itself; cite those.

### 9.4 Equivalence as finite, checkable data

```agda
record Equiv (a b : Sig) : Set where
  field
    at  : (G : Θ.Obj) → El a G ≃ El b G                       -- a bijection per shape
    nat : ∀ {G H} (f : G ⟶ H) → at H ∘ Terms a .₁ f
                               ≡ Terms b .₁ f ∘ at G          -- naturality
```

Two properties make this a **certificate** rather than a proposition:

1. **Naturality is finitely checkable at each degree.** Hom-sets in `Θ` are finite and morphism equality is decidable (§7.1), so naturality up to degree _n_ is a finite conjunction of decidable equalities.
2. **It is generated, not enumerated.** By §9.3 an equivalence is determined by its behaviour on the degree-lowering and degree-raising generators, so the stored certificate is small and the rest is derived.

```agda
check : (n : ℕ) → Equiv a b → Bool         -- decidable at each stratum
```

This is the same shape as the earlier `Equiv` design — a codata record of carried cells, never a truncated proposition, with a coherence carried rather than discharged — and it inherits that design's two disciplines unchanged: proof relevance is operational (two distinct certificates are two distinct artifacts), and composition follows the boundary of §3.4.

### 9.5 `ua_n` is a theorem, and this is where it comes from

```agda
ua      : Equiv a b → Terms a ≅ Terms b        -- trivially: an Equiv IS a natural iso
ua-desc : Terms a ≅ Terms b → a ≅ b            -- THE CONTENT: descent along nerve-ff
ua_n    : Equiv a b → Id (U n) a b
ua_n e  = ⌜ ua-desc (ua e) ⌝                   -- gated on stratum-n certification
```

`ua-desc` is the full faithfulness of the nerve.
**So `ua` is not an axiom and not a higher inductive type — it is the inverse of a bijection that someone else proved.** That is the sharp form of "a theorem at stratum _n_, never an axiom".

**The gate is real and stays.** `ua_n` is available exactly when stratum _n_'s Segal condition has been checked, because the nerve's essential image _is_ the Segal presheaves.
An uncertified stratum is one whose Segal condition has not been discharged, and there `ua_n` genuinely does not exist.
**The self-referential knot becomes a Segal-check obligation per degree**, and "the frontier advances as a rate, not a bound" acquires an exact meaning: each degree is a finite check, and there is always a next one.

**What this buys against the alternatives**, stated for the comparison ledger: the design keeps **definitional computation on the reflexivity case** and computing base-stratum transport _simultaneously_ — the combination that path-based cubical practice gives up, where the derived eliminator computes on reflexivity only propositionally.

### 9.6 Transport, and what each cost measure pays for (M12)

```agda
transport-at : Equiv a b → (G : Θ.Obj) → El a G → El b G
transport-at e G = e .at G .to               -- O(1) in the shape: just a component
```

**Transport at a known shape is free** — it is the component of the natural transformation.
Fuel is not paying for transport; **fuel pays for finding the shape**:

```agda
transport : (fuel : ℕ) → Equiv a b → El a G → Maybe (El b G)
-- descends the Reedy degree of G, replaying e's components stage by stage;
-- returns nothing exactly when deg G exceeds the fuel
```

This is the precise sense of "transport computes by certificate replay, not geometric composition": there is no composite to compute — the certificate is _evaluated at each shape actually visited_, and the visits are bounded by degree.

**The two measures, disentangled.**

| Measure | What it counts                                           | What it bounds                                                                       |
| ------- | -------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| `size`  | positions in the flat layout of a value at a fixed shape | **value-replay** cost — applying a finite bijection per position                     |
| `deg`   | vertices and edges of a shape                            | **shape-search** cost — the Reedy descent, plus the naturality checks at each degree |

The earlier records used the single word "fuel" for both, which hid M12's content: at a _known_ shape there is nothing to pay beyond the per-position replay, and the interesting cost is entirely in locating the shape.
**Cost is first-class and computable**: the cost of transport at degree _n_ is the number of naturality checks over shapes of degree ≤ _n_, a finite, statically-known number per shape.
Cost-as-effect stops being an annotation and becomes a count of Reedy cells.

### 9.7 Where the base stratum ends

The earlier record fenced off recursive codes without saying precisely where the fence was.
In this picture it is exact:

> **The base stratum is "finitely supported in Reedy degree".** Recursive codes leave it because a recursive code's terms are not supported in any finite degree.

So the top universe is the colimit of the strata, and at the colimit:

* `check` no longer terminates, so `Equiv` becomes a **coinductive** certificate rather than a finite one;
* transport becomes **corecursive**, and needs _productivity_ rather than decidability;
* which is precisely the job of **continuous normalization** — a coinductive reading of the syntax with repetition constructors marking "a step of work happened here, no output yet", so every finite prefix of the input determines a finite prefix of the output.

**Why continuous normalization is not the bookkeeping hack it can look like, here specifically.** The repetition constructor is _fuel made syntactic_: the temporal programme already centres fuel but carries it as an externally measured number, and this makes it structural.
The coinductive reading is already the charter's stance — all dimensions present only as observable-on-demand, never completed.
And it lands on a lane that was explicitly fenced off and left unequipped.
The legitimate worry is that the device is a genuine structural move when the coinductive reading is the _intended_ semantics and padding when it is only dodging partiality; gandr is in the first case by charter, and should be judged on that criterion rather than on aesthetics.

> **Boundary, restated so nobody imports it downward: productivity is not decidability.** The corecursive layer belongs to certificates and codata, **never** to the kernel's conversion check.

This is the same category error, one layer down, as claiming the certificate normal form decides replay-equivalence (C6, §12.2) — a sound device at the right layer, a mistake one layer below it.

### 9.8 Module layout

```text
Gandr.UA.Site          -- Θ, deg, Θ⁺/Θ⁻, factor; re-exports Shape.Decidable
Gandr.UA.Reedy         -- latching/matching, Aut-equivariant staging, degree induction
Gandr.UA.Sig           -- descriptions as graphical species; Terms (the free Segal object)
Gandr.UA.Segal         -- the Segal condition; the per-degree certification obligation
Gandr.UA.Equiv         -- Equiv, its generated presentation, `check n`
Gandr.UA.Descent       -- nerve-ff, ua-desc, ua_n            ← the theorem
Gandr.UA.Transport     -- transport-at, fuelled transport, cost accounting
Gandr.UA.Colimit       -- the colimit universe, coinductive Equiv, continuous normalization (fenced)
```

Dependencies run strictly downward.
`Descent` is the only module that cites an external theorem, and `Colimit` is the only one that is not decidable.

### 9.9 What this re-rules in the earlier `ua_base` record

| Earlier decision                                                                                                                                                      | Ruling now                                                                                                                                                                                                                                                                                                                                       |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **division of labour** — the presented calculus lives in the metatheory as a completeness warrant; the kernel implements the semantic side only                       | **stands, and generalizes.** The semantic currency is now `Rigid.canon` plus Reedy degree as well as canonical permutations; the _completeness warrant_ changes from an η campaign to the nerve theorem, which is stronger and is someone else's theorem                                                                                         |
| **base-stratum code isomorphism as a normalized finite bijection**, carried as certificate data, transport by per-position replay                                     | **stands**, and is now correctly located: it is the **layout** half (§1.2), with `size` as its cost measure (M12)                                                                                                                                                                                                                                |
| **two-mode certificate composition** with lax, side-conditioned interchange on the directed band and unconditional composition on the invertible band                 | **stands.** Base-stratum code isomorphisms are invertible by construction, so they never touch the lax band; the lax discipline binds the directed band only (§12.5)                                                                                                                                                                             |
| **the scaling firewall**                                                                                                                                              | **re-scoped (M13/M14).** It governs the _layout_ presentation, where it binds exactly as written. It does **not** govern the pasting layer, where the nerve route applies. The earlier "the firewall stands because the nerve alternative is unbuilt" verdict was right about the rig and was being read as a statement about the whole universe |
| **grades as conservativity smoke alarms; the K-free witness discipline verbatim**                                                                                     | **stands unchanged**, and §12.4's signature invariants are the same pattern one dimension up                                                                                                                                                                                                                                                     |
| the conditional status of the completeness theorem, with an acyclicity floor plus certificate-carried per-instance discharge as the operationally sufficient fallback | **stands as the layout-layer story.** At the pasting layer the corresponding gate is the per-degree Segal check (§9.5), which is a different obligation with a different discharge                                                                                                                                                               |

## 10. The temporal reading, re-indexed

_The honesty gate binds on this section: nothing here may be quoted as a result, and the contrast slogan stays out of user-facing material until a §10.4 prediction lands as a gate-green artifact._

### 10.1 Three renderings, two declined

For identity of structured or infinite objects there are three known renderings.

1. **Completed** — identity is a finished fact: uniqueness of identity proofs, truncation, definitional proof irrelevance.
   **Rejected**: it caps the tower at sets and forecloses univalence.
   It is what the without-K discipline bans.
2. **Spatial** — identity is a completed geometric datum: an interval, cubes, gluing.
   **Declined, not refuted**: cubical theories compute — canonicity and normalization hold there — but their primitives are _externally provided_, their meaning lives in the model rather than the theory, and a cubical conversion engine is a frozen-core rewrite gandr will not buy.
3. **Temporal** — identity is a construction _in time_: a developing agreement over an unfinished substrate, with each stage's identifications manufactured by the stages below and consumed as fuel by the stage above.
   **Adopted.**

**The without-K discipline that protects this is binding now**, independent of everything above: no K eliminator, no uniqueness-of-identity-proofs rule, no deletion in unification, no type-constructor injectivity, no definitional proof irrelevance for identity, no interval or gluing primitive, and no collapsing of identity proofs because their codes are content-addressed equal.
Two things stay explicitly available and are not exceptions to it: per-type set-ness by Hedberg wherever equality is decidable — a _theorem_, and the supplier for the whole first-order layer — and runtime erasure of identity evidence through the grade discipline, which is a parametricity fact about the runtime image rather than a claim that proofs are unique.
The adequacy witness is negative and binding: a corpus program that derives K must fail elaboration, naming the unification failure.

### 10.2 The ingredient table, landed

Every ingredient of the temporal stance now has an artifact rather than a metaphor, and after §9 most of them are the _same_ artifact.

| Temporal ingredient                           | Landed artifact                                                                                  |
| --------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| an unfinished, observable-on-demand substrate | a presheaf **is** observation-on-demand: only the values at shapes actually probed are ever held |
| developing agreement as certificate           | `Equiv`, built up by Reedy degree; `check n` is the agreement so far                             |
| inductive fuel against coinductive output     | **Reedy degree** (inductive) against the presheaf (coinductive) — now one structure, not two     |
| staged certification                          | per-degree Segal check plus automorphism-equivariant latching and matching                       |
| the cost of unfolding as first-class          | the naturality-check count at degree _n_; a computed number, not an annotation                   |
| certificate composition                       | the two-mode discipline of §3.4, with the invertible band unconditional                          |

The codata substrate underneath is unchanged and is what makes the stance honest: weak final coalgebras, no η, label-intensional identity never observed.

### 10.3 The rate, not a bound

The literature states gandr's own claim in its own terms, and it is also the honest warning.
The slogans of the analytic-functor theory hold already over 1-groupoids, but to get a good description of analytic functors in 1-groupoids one is forced to pass to 2-groupoids, and so on — _the usual infinite ladder_ — with a self-contained theory only at the ∞ level.

That is the temporal programme's shape, independently attested.
It is also the warning: **there is no comfortable finite-level theory of analytic functors.** The design must either stay Σ-free or commit to carrying symmetry as witness data at a fixed level; it may not expect the tower to close. gandr takes the first branch, and §6 is why it can.

### 10.4 Predictions and kill signals

Falsifiable, each with its artifact.

* **P-a — no spatial primitive.** The ladder completes with no interval, no gluing, no postulate.
  A forced external primitive at any rung falsifies the temporal route _for gandr_ and reopens design.
* **P-b — extensionality from certificates.** Extensionality principles for function values arrive as certificate-layer theorems (pointwise-agreement certificates tabulated into a path), never as axioms.
* **P-c — no collapse.** A uniqueness-of-identity-proofs collapse forced at any stratum is a STOP, returned to the owner with the located obligation.
* **P-d — reversibility governs composability.** Unrestricted certificate composition is available exactly on the invertible fragment; the directed band composes only under acyclicity.
  A counterexample in either direction is design-relevant news.

Kill signals specific to §9: if the graphical-species identification fails (§9.2), §9 needs re-basing before anything is built; if `canon-resp` fails for the parallel-component multiset, C3 and C4 are inconsistent and the arena needs rethinking; if a stratum's Segal condition cannot be checked at finite cost, `ua_n` is unavailable there by construction rather than by failure — which is the design working, not breaking.

### 10.5 Two applied readings, kept

**Containers, and the half that needs no extensionality.** Two descriptions for "a pair of `A`" — one indexing its two positions by booleans, the other by a two-element finite type — are the same shape as functors and distinct as codes.
At the base stratum the interchange is finite data: a two-point bijection, decidable, inspectable, replayable.
The base-stratum univalence theorem turns it into a path between the codes, and transporting a value replays the bijection at each position with `size` as the bill.
**This is the observational-equality container benefit without extensionality**, because finite positions are data rather than functions — and it is why the base stratum is reachable early.
In an axiomatic rendering the same move is a stuck term; here it computes before any extensionality machinery exists.

Extensionality proper becomes unavoidable exactly where positions stop being finite data.
The mechanism is **tabulation**: a carried family of cells in context is a proterm, and the comprehension/tabulator is the device that turns a family-of-cells-in-context into a path-in-context.
**Extensionality is tabulation of a pointwise certificate**, and eliminating the resulting path never consults a completed identification — it replays the family _at each demanded position_, so the cost is paid per observation, lazily.

**Protocols, and the seam warning.** Session types are interactive coinductive objects whose natural equivalence is bisimulation — the shape the temporal reading abstracts.
An equivalence between protocol types is a carried certificate; transport along it **is an adapter process**, with per-message replay as its cost.
Three ways the programming reading is stronger than the mathematical one: proof relevance becomes operational (two certificates are two deployable adapters with different buffering and latency); the directed family has an obvious customer (protocol evolution is one-way, and the directed family's deliberate lack of an inverse matches deprecation reality); and the cost hook becomes a compatibility budget an adapter chain can be refused against.

**The standing obligation, and it is real.** A protocol-code stratum does not exist: identity at a universe of protocols needs session types reflected as codes, and descriptions cover _data_ descriptions, not sessions.
Finite global types are first-order data, so the finite case is plausible; recursive and streaming protocols are colimit-layer material.

> **The seam is the risk.** If the data stratum and the session stratum interoperate, that is a boundary between a rung where ordering is a section and a rung where it may not be, and passing to components across such a boundary is known **not** to be a conservative map.
> Flag it before either stratum hardens.

There is one piece of good news for the session lane, recorded so the term-level scoping of §6.4 is not mistaken for a global commitment: progressive graphs form the operations of the **free PROP** with one generator in each input/output degree, the construction works for any PROP rather than only free ones, and the resulting simplicial groupoid is a decomposition space and in fact a Segal space.
So the session lane keeps many-to-many, keeps Segality, keeps the incidence bialgebra, and pays only the polynomial interpretation — which it was not using.
The same construction is important for the operational semantics of Petri nets, which is the obvious entry point when that stratum is designed.

## 11. The virtual doctrine machinery

### 11.1 Virtual honesty, and what it buys

The reflection face's internal language is a first-order virtual-double-category type theory, chosen with a written comparison and not reopened here.
The virtual reading's content is one sentence: **loose composites need not exist**, and where duplication is present they exist only as a multi-sum-indexed family.

That converts a standing honesty note into a theorem.
The completion engine's "non-linear overlaps fan out; the engine returns families" limit is **the mathematically forced behaviour**, not a shortfall; single canonical fused rules exist exactly on linear seams.
The linearity fence and the ordered-linear context discipline of the modern formal-category-theory calculi are the same corner of the same trade-off, reached independently.

Two further stances are settled and stay:

* **Certificate identity is replay-equivalence** — the replayed-not-trusted discipline promoted to the _definition_ of when two tracelets are the same transformation.
  This is what makes grafting associative and unital in the formal reading.
* **Composition ships as two operations**: unconditional on the invertible fragment, and gated by an acyclicity check on the variable-flow graph for the directed one, declining with the cycle as the diagnostic.
  Mixed-variance metavariables — exactly what the binder constructs and copattern objects create — make certificates dinaturality-shaped, and those compose only under loop-freeness in general and unconditionally when invertible.

A citable formulation worth adopting: bicategories are virtual bicategories with all composites, with composites defined via left- and right-cocartesian 2-cells.
The virtual-honesty stance and the relative-monad machinery therefore share a formulation of "composite", which is a small but real economy.

**One clean negative, recorded so it is not re-asked.** The layer stack of the game-semantics line does **not** map onto gandr's universe-stratification crate, and it does not coincide with module or abstract-type sealing, because sealing is generative.

### 11.2 The crDC ladder, measured (M19)

The compositional-rewriting double-category axioms are a **testable checklist** for the cell store rather than a doctrine to adopt, and the term-rewriting instance — not the graph instance — is the right template, because gandr's cell store is term-shaped.
The suite runs over the **real** structures — the overlap enumerator, matching and unification, rewriting and normalization, the tracelet certificates — never a second engine.

The verdicts are landed, and the scope of every claim is the **cell-visible convergent fragment**; natives remain opaque and outside every claim.

| Axiom                                     | Verdict                                                                                                                                                                                                                                                                         |
| ----------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| multi-sums in the tight layer             | **holds, degenerate-singleton** — the enumerated family is multi-universal; first-order syntactic unification makes the family at most one per ordered pair per kind, which is the discrete term-rewriting case                                                                 |
| pullbacks in the tight and cell layers    | **holds strictly** — unification computes pattern pullbacks; cell intersection is componentwise                                                                                                                                                                                 |
| horizontal decomposition                  | **holds strictly** — a cell over a composed seam factors as the two-step derivation, with the globular-iso residue the identity (stronger than required)                                                                                                                        |
| source a strong multi-opfibration         | **holds, discrete** — the singleton lift is op-cartesian: rewriting is substitution-stable and matches factor uniquely                                                                                                                                                          |
| target a residual multi-opfibration       | **holds in per-instance form** — the lift is the instantiated cell, the residue is the owed post-normalization derivation, exercised exactly when instantiation creates redexes; lifts compose functorially; pushed derivations are confluence-unique on the certified fragment |
| virtual: positive globular decompositions | **holds strictly on the free path algebra** — every derivation path decomposes uniquely into one-step cells and recomposes; two-step composites are pro-representable                                                                                                           |
| virtual: cellular Conduché                | **holds strictly** — path splittings refine, because concatenation is free                                                                                                                                                                                                      |
| the cylindrical decomposition property    | **open, not discharged** — a distinct obligation rather than a corollary; unconditional oplaxity is the honest default                                                                                                                                                          |

**The payoff.** The green rows make the universal concurrency and associativity theorems available for that fragment **by the universal proofs**: the fused-equals-two-step and grafting-associativity contracts upgrade from adopted-test to theorem-backed, with the differentials retained as adequacy witnesses.

**Two findings worth carrying.** The suite's first catch was an engine bug: the command unifier returned _triangular_ unifiers — a binding whose image mentions a metavariable bound after it — violating its own single-pass application contract and under-resolving the enumerator's peaks; fixed at source by fully resolving before returning.
And the residual part of the target axiom is exercised **exactly by redex-creating instantiations**, which answers an open question empirically: a three-step residue for a redex-bearing instantiation, none for a normal one.
The one named missing invariant is a symbolic normal-form constructor — the as-built residue is per-instance because normalization needs ground terms — which is informative, not fatal.

**A provenance note that sharpens how to hold the framework.** Its fibrational reformulation was reverse-engineered from a naturality observation rather than posited as an a-priori doctrine, which is precisely why testing it empirically per system is the intended use.
The indexed side — what the Grothendieck construction of these multi-opfibrations yields — is explicitly named unexplored by its authors, and gandr's reflection face is already fibration-shaped, so that is a research door rather than a gap.

### 11.3 The convolution face

Over a double category, vertical presheaves carry a convolution product; the representable at a rule interface — "all the ways this rule instantiates" — categorifies the rule-algebra basis vector; and under the crDC axioms the convolution of two representables decomposes as a sum of representables indexed by the multi-sum.
That is the concurrency theorem, categorified: **the fan-out family is the coefficient set of a genuine associative product one level up.**

Three facts govern how gandr holds it.

* **Adopted as specification, not as a second engine.** The categorified concurrency isomorphism is the **completeness contract of the overlap enumerator**: every element of the convolution factors through exactly one enumerated composite.
  It lands as differential rows in both directions, not as new machinery.
* **The virtual-honest form has a citation.** gandr has no horizontal composition to extend along, and the virtual statement exists: a **colax convolution on presheaves over a virtual double category, defined by a coend on the multicell profunctor**, with **no horizontal composition assumed**, colax by default and strong or representable under _positive globular decompositions_.
  Two facts calibrate the odds — cospan-style virtual double categories are always exponentiable with globular decompositions, and gandr's overlap data is span- or cospan-shaped — and the exponentiability chain gives the _named_ conditions to test.
  **The unconditional floor is a theorem, not a slogan**: every virtual double category embeds in a locally cocomplete completion with composites and restrictions, so "composites exist one level up" holds before any suite passes.
* **Associativity is structure to be earned.** Convolution is oplax in general, strong under the cylindrical decomposition property — which is _related to but not implied by_ the crDC axioms, and therefore carries its own line item (§11.2, open).

**A numbered bridge.** For a small pseudo double category, the power into `Set` is a weakly representable multicategory whose colax monoidal structure on presheaves over the arrow set **is** the convolution product of the rabbit-calculus line, with the arrow-family condition and the _n_-cylindrical decomposition property named as what makes exponentials admit composites.
That is a numbered bridge between the virtual-double-category line and the rewriting line, and it lands directly on the open convolution face.

One direction is recorded because it unifies two operations gandr already runs separately: the rule algebra and its representation on states unify through the Yoneda embedding, so "compose rules" and "apply rules to states" become one operation.
The kernel already embodies the operational half — states are cut-shaped and matches are shallow; the presheaf face names the mathematical half.

### 11.4 Tight versus loose is a stratification (M18)

The earlier record posed a three-way architectural fork: route the code layer through a presheaf-virtual-double-category and relative monads; through the polynomial comonoid setting; or through polynomial 2-functors native to virtual double categories.
The decision criterion offered was "is a description a profunctor between sorts composed by substitution (loose), or a map from arities to carriers (tight)?"

**That fork is deflated.** The decisive architectural datum comes from a focalisation paper in the same line: its relative monads and comonads are **tight** in the most elementary sense — an object map plus a Kleisli extension — taken relative to the polarity shifts themselves, and the loose object in the same development is the _ambient hom_ (an oblique-morphism distributor), with the tight object being the **modality living over it**.

> **Loose versus tight is a stratification, not a choice about one object.**

So the question is not which of three homes to pick but _which layer each object sits at_, and the answer for gandr is legible: the ambient hom of the reflection face is loose; descriptions-as-signatures sit over it; the arity/nerve story of §9 is the tight side and does not compete with the loose convolution face of §11.3.

**What survives from the fork's ledger, and matters.**

* **The recurring étale condition is one invariant seen three times**, arrived at independently for three reasons: homotopy quotients that **add arrows** rather than identify keep symmetries carried and positions decidable; a discrete-opfibration condition makes the dependent-sum step compute coproducts rather than general colimits; and a discrete opfibration between virtual double categories is what makes a polynomial's right adjoint exist at all. gandr should name it and enforce it explicitly wherever a decomposition, a sum, or a product is formed.
* **Exponentiability, correctly attributed.** The right adjoint of a polynomial in the virtual-double-category setting exists because the middle map is _powerful/exponentiable_, which follows from its being a discrete opfibration.
  An earlier draft attributed this to the wrong paper.
* **Polynomials already live in virtual double categories.** Enrichment over virtual double categories is a parametric right 2-adjoint and in fact familial, factoring through a **polynomial 2-functor** induced by an explicit polynomial in that setting, and the setting carries a universal discrete opfibration yielding a families construction and a category-of-elements construction.
  So the polynomial-to-virtual-double-category bridge is **not ours to build**; an earlier conclusion that it was is withdrawn.
* **The relative-monad line's own contribution is the root/arity framing**, and its nerve theorem's hypothesis is _density_ of the root plus smallness — with density sufficient but not necessary, and a counterexample showing it cannot simply be dropped.
  The word "Segal" never appears in that paper; the characterization is a density-and-pullback statement, and conflating the two vocabularies in gandr's prose is a category error.
  The presentation-to-nerve bridge is numbered there: for a theory-presentation, the category of concrete algebras is _defined to be_ the same pullback, so theory-algebras and monad-algebras coincide **through** the nerve wherever a dense root exists.
* **Aggregation is not functorial**, while data migration is, with multisets as the canonical target.
  Every gandr quantity accumulated over a derivation — fuel, cost, counters, observability aggregates — lives in that non-functorial regime, and it is also where symmetry re-enters through the _cost model_ rather than through the type theory.

**The variance cost is real and belongs in the ledger.** The polynomial-comonoid setting's tight maps are **retrofunctors**, so its vertical direction is cofunctorial; the chosen internal language deliberately excludes an opposite-category operator because it breaks the context discipline; and the one system that includes it pays with polarity machinery through every judgment.
Any move toward the comonoid setting puts the variance layer against the missing operator immediately.
That is a cost the relative-monad route does not obviously carry, and it should be settled before a reflection face is built on either.

### 11.5 Cartesian double theories and the shape-signature doctrine

The 2-dimensional fragment of a shape block — sorts, generators, and rules — is precisely a presentation of a **cartesian double theory**, and the derived model signature is its category of **product-preserving lax double functor** models, with functorial semantics, morphisms of models, free and initial models, and a **virtual double category of models** all supplied by cited theorems.
The "modules as instances" reading is therefore an instance, not an analogy.

**The gap is equally valuable.** That literature imposes coherence as _equations between cells_, where gandr declares them as proof-relevant 3-cells, and it is strictly 2-dimensional — the 3-cell layer is exactly the frontier its authors name. gandr's routing through the dimension-generic coherence-former line is therefore _ahead of_, not in conflict with, the doctrine literature.

**The dictionary's cartesian law is refined, not repaired.** Test it against the right universal property: the **framed** pairing–projection bijection rather than the weaker globular one; expect **iso-strong**, never strict, since products of relation-shaped loose arrows are genuinely lax in general and iso under bijective indexing; and decompose it as parallel products of signatures _plus_ restriction along structure arrows.
"Products preserved by restriction" is verified sound at the source.

**A bare virtual double category is not cartesian in that sense** — the product apparatus needs a genuine double category. gandr's legitimate cartesian claims live at the tight/object layer with _local_ products-and-restriction, and at the virtual double category **of models**.
And an honesty flag: neither source defines the _cartesian fibrational_ virtual double category that the reflection face actually targets, so reconciling the two notions needs a direct pass, and until then every verdict must name which cartesian notion it tested (§16 Q7).

### 11.6 The growth firewall

If a fibrational axiom fails, the **universal repair exists**: the free bifibration on the source and target functors adjoins exactly the missing pushforwards, with a clean proof theory — objects are alternating push/pull formulas, arrows are cut-free sequent derivations modulo permutation, and the zigzag double category is the free double category with companions and conjoints.

Three facts govern the posture.

1. **The decidability cliff.** Derivation equality is canonical — maximally multifocused normal forms — exactly when the base is **factorization preordered**; without that, 2-cell equality in such free constructions is undecidable.
   So freely adjoining fibrational structure is an owner **STOP** whose entry condition is a factorization-preorder check on the seam category.
   Content-addressed first-order syntax plausibly qualifies via canonical context and substitution factorization — testable, not assumed.
2. **Companions and conjoints are free.** The graph and cograph loose arrows of tight substitutions, which the restriction calculus implicitly uses, cost no axioms; the zigzag construction manufactures them universally.
   This closes, by import rather than construction, a standing gap in the reflection face's structure.
3. **The focusing convergence.** The canonical forms of the free bifibration are obtained by proof-theoretic focusing — the same technology as the kernel's own focusing translation (§3.1).
   If the STOP is ever taken, no foreign engine is needed.

### 11.7 Where the doctrine layer meets §9

The two halves of this document are not two programmes, and the join is M1.

* The doctrine layer governs the **derivation** dimension: cells, certificates, composition, and the fibrational axioms over them.
  Its arity is **linear** — a virtual double category is a generalized multicategory for the free-category monad.
* The nerve layer governs the **term** dimension: signatures, pastings, the Segal condition, fullness.
  Its arity is **graph-shaped**.
* They are the same telescope at two arities (§5.5), and both arities are cartesian for the same reason (§6.1).

That is why the crDC verdicts of §11.2 and the univalence construction of §9 can be pursued in parallel without an integration risk: they are different instantiations of one parameterized construction, not two theories that must later be reconciled.

## 12. The certificate algebra

### 12.1 What the source theorems actually say

The tracelet line supplies four results gandr leans on, and each has a correction attached.

| Result                                | Statement                                                                                                                                                           | Correction                                                                                                                                                  |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| uniqueness of primitive factorization | every maximal splitting has, up to isomorphism and permutation, the same primitive pieces                                                                           | **cite this for uniqueness.** The existence statement elsewhere in the same paper gives existence _only_, and the arc had been leaning on it for uniqueness |
| shift equivalence characterized       | tracelets are shift equivalent in the restricted trivial-overlap sense **if and only if** they have the same factorization into primitives                          | it is an **iff**, and gandr's own contract states only one direction (§12.2)                                                                                |
| freeness                              | the shift quotient is the free **symmetric** monoidal category on primitives                                                                                        | the symmetry is load-bearing (C4, §6.4)                                                                                                                     |
| the Hopf structure                    | filtration by number of primitive components; connected, hence an antipode; the whole is the universal enveloping algebra of the Lie algebra of primitive tracelets | **load the antipode from the connected-filtration theorem, never from Möbius inversion**                                                                    |

**Two unstated hypotheses.** Neither local finiteness nor completeness is established in the source, and both are used: local finiteness appears parenthetically with no definition, no proof, and no check; completeness is never established yet is invoked twice.
This is not pedantry — the corresponding literature is explicit that while the decomposition space of a _free_ operad is automatically locally finite, the general case is **not**, and the condition must be imposed separately if numerical examples are to be extracted.
The rules here are not a free operad, so the parenthetical does not discharge itself.
Additionally, the "irreducible" object appearing in the freeness theorem is never defined, and the paper uses "primitive" and "irreducible" interchangeably.

**gandr's shift equivalence is not the source's** (M16, first half).
The source states its notion is _strictly more general_ than shift equivalence in the standard rewriting literature; gandr implements the **less permissive sequential-independence version**.
The freeness theorem therefore **cannot** be cited as though the two relations were the same.

### 12.2 The normal form — contract strengthened, scope corrected (M15)

The certificate normal form is the reflexive-symmetric-transitive closure of three relations: abstraction isomorphism, trivial-unit insertion and removal, and shift equivalence.

* Abstraction isomorphism **is** content-addressing — already the store's identity.
* Unit insertion and removal **is** empty-path elimination — already the path calculus's unit laws.
  There is a structural reason it sits at a different level from shift equivalence: **the empty path is not an edge**, so unit insertion is absorbed into path equality and has no tile of its own.
* Shift equivalence is the new content: two adjacent cell applications whose positions are disjoint and whose overlap is trivial commute.

```rust
pub struct PrimCert { pub apps: Vec<CellApp> }        // a causally connected component
pub struct PrimId(ContentHash);                       // the canonical order key

pub struct TraceletNf {
    /// The unique primitive multiset, keyed and ordered by content address.
    /// The multiplicity is LOAD-BEARING, not an optimization.
    pub primitives: BTreeMap<PrimId, (PrimCert, u32)>,
    /// The canonical schedule: each application at its earliest causal position;
    /// ties broken by the content-address order.
    pub schedule: Vec<CellApp>,
}

pub fn normalize(t: &Tracelet, supp: &OverlapSupport) -> TraceletNf;
pub fn nf_equal(a: &TraceletNf, b: &TraceletNf) -> bool;
```

**The contract, corrected.** The source's characterization is an _iff_, and gandr's shift equivalence _is_ the trivial-overlap restriction, so:

```text
nf_equal(NF(a), NF(b))  ⟺  a ≡_S b        -- available now, from the cited theorems
a ≡_S b                 ⟹  replay-equal   -- sound
replay-equal            ⟹  a ≡_S b        -- FALSE, and constructibly so
```

**The contract was more conservative than its own source** in the first line, and silently ambitious in the third.
Both are fixed here.

**Why the converse is constructibly false, in gandr's own codebase.** Replay-equivalence is verified as: the two tracelets share a peak, share a join point, and both replay.
It is **pure proof-irrelevance** — it ignores the recorded paths beyond the fact that each replays.
So two confluence tracelets for one critical pair that join at the same normal form by different routes are replay-equivalent with **different primitive multisets**.
A refutation is constructible from the shipped code, not merely unproven.

> **C6, restated: the normal form is a performance fast path, never a decidability result.** Replay-equivalence is _already_ decidable — boundary equality plus two replays.
> The right framing is that the normal form answers the replay-equivalence **cost** question, and it must not drift into a decidability claim.

**Scope.** The relation currently has **empty extension** on the as-built grammar (§3.2), so this work is forward-looking.
Recording that is what stops a future session from reading a vacuous pass as a discharged obligation.

**Multiplicity is load-bearing.** If the metatheory ever computes the coproduct, it must sum over subsets of the index **multiset with multiplicity**; deduplicating by primitive identity returns one where the answer is two.
That is the classic "five cuts, four terms" failure one level up, and the multiplicity field is what prevents it.

**Without-K check.** Shift equivalence permutes genuinely independent components and identifies strictly _less_ than replay-equivalence, which is already the certificate identity.
No new collapse enters, and the no-blanket-fillers law is untouched: the quotient is **earned per pair** by a trivial-overlap witness, never adjoined.

**What it buys.** A fast path for certificate equality (the fast path decides equality, the slow path decides equivalence); compression to a primitive multiset plus a minimal schedule; and **coherence-cell elimination** — the 3-cell relating the two orders of two independent steps becomes _definitional_ under shift equivalence, discharging a whole class of interchange obligations by normalization instead of carrying them.
That also supplies the principled semantics for the horizontal-composition sugar declined in §4.2: accept it exactly on disjoint positions, where the two sequential readings are shift-equal.

### 12.3 The bracket oracle and parallel replay

The enveloping-algebra theorem's engineering shadow is **a relation, not a vector space**.
In the commutator over all overlaps, the trivial and disjoint terms cancel, so the bracket's support is exactly the set of nontrivial overlaps:

> `[T, T′] = 0` ⟺ no nontrivial overlap ⟺ the two are freely reorderable.

```rust
pub struct OverlapSupport { /* symmetric, memoized; rows keyed by content */ }
impl OverlapSupport {
    pub fn interferes(&self, a: &PrimId, b: &PrimId) -> bool;
    pub fn independent(&self, a: &PrimId, b: &PrimId) -> bool;
}

pub struct ReplayPlan {
    /// Antichains of the causal partial order: each level replays in parallel.
    pub levels: Vec<Vec<PrimId>>,
    /// Fuel = critical path (longest chain), NOT schedule length.
    pub critical_path: u64,
}
```

**One lookup, four consumers**: the normalizer's union-find edge test, completion-loop scheduling, the replay-parallelism test, and a structural cousin of the directed-composition acyclicity gate — both are reachability questions over shared data.
That is the third member of the "one algorithmic family serves several gates" observation the arc has now made three times.

**The schedule theorem.** The enveloping-algebra structure warrants that ordered products of primitives — for any fixed total order refining the causal order — form a basis, so the canonical schedule loses nothing.
Ordering by content address makes it strict and stable.

**This is exactly where C4 is cashed.** The theorem runs on cocommutativity, cocommutativity comes from the symmetric monoidal structure on the parallel direction, and ordering that direction would remove it (§6.4).
The bracket oracle is the concrete thing that breaks, and it breaks _silently_ — which is why the scoping minute matters more than its length suggests.

**Honest scope.** The full Hopf structure — formal sums, the coproduct as _all_ splittings, the antipode — stays specification currency.
Two readings are recorded as direction only: the coproduct as the space of cache-key decompositions for incremental recomputation, and the antipode as systematic inverse-certificate construction on the invertible lane.
Neither is scoped, and either would need its own design pass with a STOP if it wanted new presented structure.

### 12.4 The residual-theory lineage (M16, second half)

The arc entered this territory through _graph_ rewriting and inherited its shape.
There is a parallel **term-side** tradition — optimality and residual theory — that gandr never evaluated despite being a term rewriter, and evaluating it changed three things.

**gandr's shift equivalence is Melliès's _reversible_ permutation equivalence, not Lévy's.** The reversible notion is introduced in the axiomatic-rewriting line as _a stronger version of usual Lévy permutation equivalence_ — strictly finer.
Lévy equivalence additionally tiles **duplicating** and **erasing** squares — residual multisets of size greater than one and of size zero — for which shift equivalence has **no generator at all**.
So gandr inherits soundness for free, and the normal-form work is **neither re-derivation nor entirely new**: the primitive-multiset invariant supplies exactly the _modulo-reversible-equivalence_ residue that the standardization theorem leaves undecided — **and no publication states that relationship in either direction.** The mapping of the abstraction and unit relations onto Lévy counterparts is **open**: one scout claimed there are none, and its verifier refuted that against the source's own section on the topic.

**The tile is a freely chosen datum, and that is why overlapping rules were never a problem.** A two-dimensional transition system is a transition system plus a binary relation on paths, required only to relate coinitial and cofinal paths.
That is the whole condition.
**Nothing requires the relation to be total on coinitial pairs of redexes**: an overlapping or critical pair for which no tile is supplied is simply a _two-dimensional hole_, the source's own term.
So gandr's overlapping rules and its Knuth–Bendix completion loop are **admissible by construction**, and no orthogonality hypothesis was ever in the way.

It also explains the vacuity cleanly. gandr's tile relation is currently _empty_, so every axiom holds and nothing is bought.
**The inheritance route is about supplying the relation, not about verifying axioms** — and instantiating the axiom interface non-vacuously buys four major theorems by citation.
One caution before citing a count: one presentation states nine standardization axioms and another describes a ten-item interface; both may be right for different presentations, and the count should be resolved rather than guessed.

**Treks are not tracelets, and gandr's certificate is neither.** A trek is a multi-step _path_ promoted to a residuation atom inside a fixed ambient term — introduced precisely because the classical theory breaks when the residual of a redex is a path rather than a redex.
A tracelet is a minimal derivation trace carrying an aggregate rule interface, whose whole content is associative composition and universality, with no ambient object, no order on paths, and no family, ancestor, or optimality theory.

> **gandr's tracelet is structurally a permutation _tile**_ — a pair of coinitial, cofinal rewriting paths — **whose two legs are trek-shaped multi-step paths, with replay-equivalence occupying the seat the axiomatic theory gives to "same induced residual relation".**

**Nobody has carried out the convergence.** The categorification paper and the long compositional-rewriting monograph cite **zero** residual-theory work between them; the phrase "term rewriting" does not occur in the latter.
That is a genuine, publishable seam rather than merely a gap, and the arc must decide whether gandr _states_ it or merely uses it (§16 Q9).

Two corrections to how this was first reported, recorded so the corrected state is inherited:

* "The residual theory deliberately refuses to compose treks" is **refuted**.
  The cited passage is about _residuation_, not composition, and says only that residuals of a trek after a trek are deduced from coherence rather than defined directly.
  Treks _are_ paths, and path composition is defined explicitly.
  The honest asymmetry is "derived rather than primitive"; the trek-to-tracelet analysis survives, the "deliberate refusal" framing does not.
* A proposed decisive test would have discarded treks wrongly: the phenomenon it tested for is _absent_ in the lambda calculus, where redexes have only redexes as residuals, yet treks do not degenerate there — they remain a strictly larger set carrying the canonical-ancestor theorem.

**Interaction nets are the wrong shape, decisively.** The match is real but shallow: gandr's cut _is_ the interaction-net cut, with the same linear-logic provenance and the same polarized principal-port meeting, and gandr's rule shape is literally the interaction-system format.
But gandr violates **all three** defining constraints — at most one rule per symbol pair, left-linearity, and a left-hand side that is an active pair of two cells — and the Knuth–Bendix completion loop over genuine critical pairs settles it.

**A second disconnected seam, in the same author's hands.** The asynchronous game-semantics line's treatment of independence and the residual-permutation treatment are substantively the same device in **bibliographically separate programmes** — the game-semantics bibliographies contain no axiomatic-rewriting work at all, citing instead the concurrency and directed-algebraic-topology lineage.
And the device is literally shared: "the same induced bijection on step indices", by which the game line quotients its reschedulings, is the same construction as the tiling-graph ancestor function on redex indices.

### 12.5 The interchange unification (M17)

Interchange appears at three different strengths across the adjacent literature, **and the strength is the design decision**.

| Setting                        | Interchange is…                                                                                           | Consequence                                                                                             |
| ------------------------------ | --------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| asynchronous template games    | an **invertible 2-cell** (a Gray commutation)                                                             | the two interleavings stay distinct 1-cells; deadlock is correctly unresolvable                         |
| the rejected alternative there | a **strict equation** (a cartesian diagonal)                                                              | _"excess of synchronization… a defect (not as a feature!)"_ — **the model wrongly resolves a deadlock** |
| concurrent separation logic    | a **non-invertible lax coercion** — a map from a colimit of limits to the corresponding limit of colimits | the Hoare inequality is **derived**, not postulated                                                     |
| the layered game semantics     | **equations in a coherent congruence**                                                                    | interleaving and true concurrency coincide                                                              |

> **The unification: the horizontal/vertical exchange of two independent things is never an equation unless you are willing to lose information, and every well-behaved treatment replaces the equation by a witness whose invertibility is the design dial.**

The rewriting side says imposing interchange is _coarser_; the deadlock example says it is **wrong** — it manufactures a synchronized diagonal move letting two mutually blocked strategies proceed.
**That is a failure mode, not merely a loss, and it is a concrete counterexample gandr can point at.** It also locates the phenomenon precisely: the relevant structure is a sesquicategory first and becomes a 2-category only after the isomorphism quotient.

**This is a positive validation of gandr's certificate shape** — a pair of coinitial and cofinal paths _with_ a witness, rather than an equation — together with a precise warning about the one place gandr could get it wrong.
The shift-equivalence quotient of §12.2 is admissible exactly because it is earned per pair by a trivial-overlap witness rather than imposed; the declined horizontal sugar of §4.2 is declined for the same reason.

The produoidal reading of the two-mode certificate algebra says the same thing from the algebraic side: the interchange laxator is **structurally lax** — the physical tensor is an inclusion of admissible orderings, so no normalization upgrade can invert it — coherence at that level carries a distinct-typing side condition, and duoidal coherence fails in general.
Two operational consequences: **never add an "all structure diagrams commute" fast path to the certificate store**, and on **polarized** boundaries — interfaces that are purely produced or purely consumed — sequential and parallel composition coincide, so the bookkeeping there is definitional and its normal-form check is a linear-time acyclicity test.
The polarization substrate already ships as per-metavariable variance metadata; pure-producer and pure-consumer boundaries are the polarized fragment, mixed boundaries are the lax band, and no new field is needed.

### 12.6 What must not be substituted

Two substitutions look like strengthenings and are narrowings.
Both are recorded because each would fail silently.

**Do not swap the symmetric quotient for the planar one.** The certificate line's quotient is the free **symmetric** monoidal category; the planar string-diagram line's is the free **monoidal** category.
Importing the second would **silently replace a symmetric quotient with a planar one** — reading as a strengthening while _narrowing what certificate equality accepts_.
This is the false-completeness failure mode in its most dangerous form, and it is the same error as ordering Axis A (§6.4).

The two relations are moreover **incomparable in both directions**, so no translation is a refinement either way:

| Direction        | Why                                                                                                                                                                                                                                                                                                               |
| ---------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| gandr coarser    | the planar complete invariant retains planar data the primitive multiset provably discards — an ordered list of faces per component plus a per-component planar map                                                                                                                                               |
| gandr finer      | gandr's legality test is keyed by **cell identity, not instance**, and overlap enumeration ranges over _cell pairs with a unifier_, never term positions. Two cells that _could_ overlap somewhere but fired at disjoint positions here are exchangeable in the planar theory and **blocked** by gandr's relation |
| carrier mismatch | gandr's position is a child-index path in a first-order term tree; the planar theory's is a row-major offset in a flat wire word, and every algorithm there is built on that. **No translation exists in the code, the spec, or the tracker — without one, zero theorems transfer**                               |

Two clean matches are worth keeping: gandr's notion of _adjacency_ is exactly that theory's height order, and typing rides free there ("our results are applicable to any signature").
And one refinement of an earlier over-read: its normalization sorts face-node children by a recursively computed injective integer code — genuine content-address tie-breaking, with hashing plus full-comparison fallback recommended — but sorts component-node children by **order of introduction**, a _geometric_ order that must **not** be replaced by a content address.
The rule that falls out is worth adopting directly: **content address at the component level, geometry within a component.**

Better news than feared on disconnectedness: what fails there is _termination of the rewriting strategy_, not finiteness of the class, and the repair is a complete invariant.
Since every gandr cell has a non-empty left-hand side, every vertex has an input wire and traces to the boundary, so gandr's generically disconnected certificates would be **boundary-connected** — landing in the tractable regime rather than the divergent one.
That last step is an inference from a plausible encoding, not a statement in the source; verify before relying on it.

**A cost-model caution for the improvement lane.** The overlap-support relation is keyed by cell identity rather than instance, which is _strictly stronger than needed_.
Instance-level keying would give a coarser and more useful relation, and is the obvious first improvement once the relation has a non-empty extension.

## 13. Representation and performance

### 13.1 The design principle, refuted as stated and salvaged (M22)

The owner-formulated principle was **"prefer representations in which perturbation is local"**, offered as explaining four decisions at once: the flat arena, content-addressed sharing over fan-node sharing, ordering, and the observational/fuel stance.

**As stated it is refuted.** Sharing graphs _are_ interaction nets, every rule of which is a strictly local rewrite; read as rule-locality, the principle _favours_ maximal sharing rather than opposing it.
No held source states that optimal reduction violates a stability or continuity condition, and one source states the opposite — that sequentiality is a _precondition_ for optimal reduction.

**A sub-argument against it also failed**, and the correction matters: the refutation turned on rules being _constant-size_, which appears in no cited source and is contradicted by the primary text, where fan-node arity grows with bus width.
So the principle is not refuted _that_ way either.

**The salvage, and it is a good one** — and the reformulation is what matters.
The real content is not rule-locality but **bounded sensitivity of an address or identity map under an edit metric — a Lipschitz-style condition.** A local edit must perturb a logarithmic number of content addresses; fan-node sharing gives no such bound.
That is statable and defensible, it distinguishes gandr's strategy exactly where intended, and it is a claim about the **addressing scheme** rather than about the rewrite rules.

> **Prefer representations in which the address map has bounded sensitivity under local edits.**

**Consequence: the four decisions must be re-checked against the reformulation rather than assumed to survive it.** Two plainly do — the flat arena and content-addressed chunking are exactly bounded-sensitivity designs.
Ordering is justified independently (§6.3) and does not need this principle.
The observational/fuel stance is justified by §9.6 and does not need it either.
So the reformulated principle is load-bearing for one decision rather than four, which is a smaller but honest claim.

**One further correction to how the surrounding argument was held.** The optimality result frequently described as "the algorithm _is_ the geometry of interaction" is called a **thesis** by its own authors, not a theorem; the theorem-level content is an invariance result, a semantic equivalence, and optimality in a cost model **counting parallel-beta steps as unit steps** — charging nothing for bookkeeping, garbage collection, or useless work, with all three exclusions stated by the authors themselves.
So the cost-model critique is right in substance and _partly already conceded in the primary source_.

### 13.2 Layout-first, optimality declined

gandr and the optimal-reduction line give two different answers to the same problem — sharing — and gandr has already chosen one.

|                   | optimal-reduction line                          | gandr                                            |
| ----------------- | ----------------------------------------------- | ------------------------------------------------ |
| sharing mechanism | fan nodes rewritten in a graph                  | content addressing over a flat arena             |
| guarantee         | a redex is never duplicated if it can be shared | not optimal in that sense                        |
| memory behaviour  | pointer-chasing over a chaotic graph            | linear runs, computable offsets, chunked storage |
| hardware fit      | hostile to cache locality and vector pipelines  | designed for exactly those                       |

So the real question is not "should gandr adopt optimal reduction" but **"does gandr want optimality at all, given that it costs the layout strategy the rest of the design rests on?"** The answer taken is no.

**But the implementation record cuts the other way from the pessimistic framing**, and both framings should be resisted.
The best-known implementation of the optimal line reached roughly a fiftyfold improvement from a **memory-layout** change with no algorithmic change.
**That inverts the usual lesson: it is evidence _for_ layout-first engineering — i.e. for gandr's own bet — not evidence that graph rewriting is doomed.** The honest position is that neither the pessimistic nor the triumphant framing is established, and that gandr should **learn the layout discipline while declining the objective**.
That is the intended posture, and it is why §13.3 exists at all.

### 13.3 The acceleration band and its firewall

Three order-independence properties, one per layer, align — and their alignment is what would make batched execution _semantics-preserving_ rather than merely fast.

| Layer            | Property                                                                                                                               |
| ---------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| logical          | shift equivalence: schedules of independent work are interchangeable, with unique primitive factorization (§12.2)                      |
| algebraic        | the signature of a concatenation is the tensor of signatures, so the combine is associative and segment computation is a parallel scan |
| representational | history-independent chunking: chunked state converges regardless of update order                                                       |

**Four workloads, by honesty class.**

1. **Batched signature scans** (advisory): truncated causal signatures over cell-application streams as a parallel scan, batched across a certificate store; consumers are prefilters, alarms, dedup and cache keys.
2. **Overlap screening** (advisory, sound-direction only): content-fingerprint summaries per chunk, pairwise intersection as batched population counts.
   The screen may only say "definitely no overlap"; a surviving pair always reaches the exact enumerator.
3. **Chunk-parallel replay and rehash** (exact, differentialed): one replay level touches pairwise-disjoint regions, so it applies as one batched tree surgery plus one batched rehash of touched spines.
   History-independence makes the batch result independent of intra-level order — the representational twin of the shift-equivalence theorem that licensed the level.
4. **Rule-algebra numerics** (analysis band, direction only): pattern-count observables and their commutator evolution are genuinely linear-algebraic, and are the one place the linear currency becomes operationally real.
   Strictly outside the kernel.

**A precision that matters.** A raw depth-two signature of a _serialized_ schedule is **not** shift-invariant — two orders differ in the antisymmetric block — so signatures are computed on the **canonical schedule**, where they are well defined on equivalence classes.
The antisymmetric block is then not noise but signal: it is the **numerical shadow of the Lie bracket**, so for a pair recorded independent it must vanish on every observed schedule, and a nonzero value is an alarm about a mis-recorded independence, caught by arithmetic before any replay diverges.
The corresponding invariance theorem — signatures are invariant under exactly the tree-like excursions — is the analytic twin of the unit quotient: backtracking that cancels is invisible, which is correct for an invariant of _causal content_.

> **The firewall (binding).** Accelerator results are **either advisory** (screens, signatures, alarms, plans) **or exact-and-differentialed** (hashing, batched surgery, with the sequential path as the standing oracle).
> No accelerator result is ever soundness-bearing; the kernel never links this band; numeric nondeterminism must be unobservable.
> Adoption is measurement-first: vector before accelerator, with a dispatch-amortization benchmark gating any accelerator dependency.

## 14. What would falsify this

Stated so each is checkable rather than assumed.

1. **The canonicalization completeness law fails** for the parallel-component multiset.
   Then ordering is not a section, C3 and C4 are inconsistent, and the arena needs rethinking (§2.3, §6.3).
2. **Real gandr cells are not simply connected.** Then free term sets are infinite and the arena needs a different finiteness story; test the output-properad rung before abandoning strictness (§6.2, §7.2).
3. **gandr's composition is not the Segal composition.** Then C2 fails and the nerve theorem does not apply to gandr's cells (§8.1, §9.2).
4. **The term face needs horizontal/monoidal composition of cells.** That is PROPs, which the held literature does not cover — properads, with connected composition only, are the proved ceiling.
   **This is the most likely failure**, and it lands exactly on the ratified multi-output direction's parallel-composition zone (§3.3, §8.4).
5. **The graphical-species identification fails** — a description needs dependency or indexing the tiny base category cannot express.
   Then §9 needs re-basing before anything is built (§9.2).
6. **The generalized-Reedy factorization is not computable** even with canonicalization.
   Then strata and fuel are not one object and §9.3 collapses to two separate mechanisms (§9.3).
7. **A shift-equivalent pair replays divergently.** That is a soundness bug in position or overlap bookkeeping and stops the certificate lane immediately (§12.2).
8. **Admissibility fails** for gandr's target in the rectification theorem's sense.
   Then strictness at the dioperad rung is not licensed by that route and needs another (§8.5).

## 15. Build order and spike queue

The Agda substrate is landed through the categorical layers and the section discipline; the queue below is what remains, ordered by what unblocks the most.

| #        | Item                                                                                                                                                                                        | Decides                                                                                            | Cost                 |
| -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- | -------------------- |
| **S-1**  | **Write the scoping minute** — ordering is a representation section (C3); the parallel direction stays symmetric (C4). Amend every place that currently says "pending the planarity ruling" | unblocks everything downstream; urgent because the term face is next (§3.3)                        | hours                |
| **S-2**  | **Strengthen the normal-form contract** to the _iff_ its source states; record the false converse and the cost-not-decidability framing                                                     | a correctness statement gandr already owns and does not claim                                      | hours, no new source |
| **S-3**  | **The graph arity kit** (§5.4) — the second instance, with the listings as carried data                                                                                                     | whether the two-kit design is real                                                                 | 1–2 days             |
| **S-4**  | **Edge-determined decidability** (§7.1) — encode the determination lemma, derive object equality                                                                                            | whether decidable equality survives _without_ essential discreteness. Keystone                     | 1–2 days             |
| **S-5**  | **The finiteness gate** — implement the simple-connectivity check and measure how many real cells satisfy it                                                                                | whether C1 is free or a real restriction. Cheap and could invalidate C1, so run early              | 1 day                |
| **S-6**  | **The description-as-graphical-species map** (§9.2) — map the actual description constructors onto a species profile                                                                        | whether §9's identification holds at all. **Do this before S-8**                                   | 1 day                |
| **S-7**  | **Extract the arity interface** once S-3 exists, and generalize the telescope over it (§5.5)                                                                                                | whether M1 is executable rather than a slogan                                                      | 2–3 days             |
| **S-8**  | **The Reedy structure on the site** — degree, the two subcategories, factorization, with "unique up to iso" discharged through canonicalization                                             | whether strata and fuel really are one object. Keystone                                            | 2–3 days             |
| **S-9**  | **Equivalence certificates and per-degree checking** on a toy signature with one nontrivial automorphism                                                                                    | whether equivalence certificates are genuinely finite and small                                    | 1–2 days             |
| **S-10** | **Descent** — the univalence theorem restricted to the dioperad rung, plus the admissibility check (§8.5)                                                                                   | the payoff. Highest ceiling in the list                                                            | 2–3 days             |
| **S-11** | **Fuelled transport and cost accounting** on the toy signature                                                                                                                              | whether "replay, not composition" is real in code, and whether M12's two measures separate cleanly | 1–2 days             |
| **S-12** | **Supply the tile relation** and instantiate the axiom interface non-vacuously (§12.4)                                                                                                      | four theorems by citation; turns a vacuous pass into real inheritance                              | 2 days               |
| **S-13** | **Re-check the unverified bipermutative locator and its rigidity package** (§18)                                                                                                            | citation hygiene on the one report never adversarially checked                                     | minutes              |

```text
S-1 ─┬─> S-2                                            (paper only, do first)
     └─> S-5 ──> C1 confirmed ─┐
S-3 ──> S-4 ──────────────────┴─> S-7 ──> S-8 ──> S-9 ──> S-10 ──> S-11
S-6 ─────────────────────────────────────^
S-12, S-13 — independent, run whenever
```

**Parked deliberately**, with reasons: the free-bifibration STOP (gated, not scheduled); the coproduct-as-cache-keys and antipode-as-rollback directions; the acceleration band until the certificate relation has a non-empty extension; the session/protocol code stratum; and the permission-monoid question against the grade design.

## 16. Open questions

1. **Q1 — is the free-rig monad cartesian?** The single technical question on which any nerve route for the _layout_ universe turns.
   No published answer; the nearest precedent fuses two monads through iterated distributive laws and is explicitly not routine (§7.4).
2. **Q2 — does the description universe fit a graphical species?** Gated by S-6; its falsifier is dependency or indexing (§9.2).
3. **Q3 — is the opposite graphical category sound, and extendable, and is its Segal-object category literally the monograph's strict properadic condition?** Partially checked; the unverified step is that a union of corolla images equals a pushout of corolla representables over edge representables (§8.5).
4. **Q4 — admissibility.** Establish it for gandr's target, or show the discrete case trivially admissible (§8.5, falsifier 8).
5. **Q5 — do gandr's rules satisfy the non-unitality condition?** Cheap, high value, and the source is not held.
   On the face of it yes — a rule's faces are patterns, never identities — which would put gandr outside the pathology on hypotheses it never meets anyway (§4.3).
6. **Q6 — where does shift equivalence sit in certificate identity?** Should the _store_ key on the normal form (deduplicating shift-equivalent certificates at insertion) or only the comparator use it?
   Store-keying is more aggressive and interacts with provenance (§12.2).
7. **Q7 — cartesian reconciliation.** Does the reflection face's cartesian _fibrational_ target restrict to the double-theory cartesian notions on the fragments where both apply?
   Until answered, every dictionary verdict must name which notion it tested (§11.5).
8. **Q8 — non-linear patterns versus multi-sums.** With non-linear patterns admitted, is the overlap family still finite and multi-universal, or does an occurs-check fragment need fencing (§11.2)?
9. **Q9 — does gandr _state_ the trek-to-tracelet seam or merely use it?** It is a publishable seam rather than a gap, and the same question applies to the second disconnected seam between the asynchronous and residual treatments of independence (§12.4).
10. **Q10 — the two-dimensional translation.** Is there anything operational in the correspondence between the determinism axis over the simplex category and the symmetry axis over the tree category with an invertibility condition, or is it orientation only (§6.2)?
11. **Q11 — a protocol-code stratum.** Identity at a universe of protocols needs sessions reflected as codes; descriptions cover data, not sessions.
    Scope only when a customer exists, and flag the cross-stratum seam first (§10.5).
12. **Q12 — the residue's practical extent.** Which cell classes exercise the _residual_ part of the target axiom beyond the redex-creating instantiations already measured, and how much of the general definition must be implemented (§11.2)?

## 17. The corrections ledger

Every substantive reversal, in one place, so no reader of an earlier record is left holding a superseded claim.

| #   | Earlier claim                                                                        | Status now                                                                                                                                                                          |
| --- | ------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | the planar-versus-groupoid decision is the arc's principal fork                      | **dissolved at the as-built layer, deferred at the design layer.** Ordering is a section, not a planarization (§6.3)                                                                |
| 2   | planarity does not cost cocommutativity                                              | **retracted.** The supporting argument was a non sequitur and the sources point the other way (§6.4)                                                                                |
| 3   | many-to-many forces the groupoid level, so Segality is what survives                 | **half right.** The polynomial interpretation is dead; but a _set-level nerve theorem_ exists and needs no cartesian monad, so the groupoid level is not forced by the nerve (§8.4) |
| 4   | the defect in the free many-to-many monad is confined to portless graphs             | **refuted.** An explicit counterexample exists on a graph with ports (§8.4)                                                                                                         |
| 5   | rigidity means the objects have trivial automorphisms                                | **false at this rung.** Rigidity is a property of the _representation_, stated as an effective quotient (§2.3)                                                                      |
| 6   | decidable equality comes from essential discreteness                                 | **replaced** by edge-determination, which needs no planarity (§7.1)                                                                                                                 |
| 7   | symmetry is the layout-relevant wall                                                 | **finiteness is**, and it is exactly simple connectivity (§7.2)                                                                                                                     |
| 8   | the naive root ladder organizes the symmetry axis                                    | **refuted at the first rung** (the bijections-only inclusion is not dense) and superseded as an organizing device by the site (§0.2, §9.3)                                          |
| 9   | a HIT-free escape reaches the symmetry rung by carrying the choice in a witness      | **withdrawn.** What replaced it is that gandr does not need that rung (§0.2)                                                                                                        |
| 10  | the properadic arity kit is the forest construction                                  | **corrected.** A forest is many-in, one-out per component; the properadic arity is a _graph_ (§5.3)                                                                                 |
| 11  | restrict to dioperads, therefore give up many-out                                    | **false.** Dioperads have the same colour set as properads; what properads add is _reconvergence_ (§6.2)                                                                            |
| 12  | simple connectivity forbids branching                                                | **false.** It forbids reconvergence; branching is fine (§4.4, §5.2)                                                                                                                 |
| 13  | the certificate normal form implies replay-equivalence, one direction                | **strengthened to an iff** against shift equivalence, with the converse against replay-equivalence **constructibly false** (§12.2)                                                  |
| 14  | gandr's shift equivalence is the source's                                            | **no** — gandr's is the less permissive sequential-independence version (§12.1)                                                                                                     |
| 15  | gandr's shift equivalence is Lévy permutation equivalence                            | **no** — it is the _reversible_ refinement; Lévy additionally tiles duplicating and erasing squares (§12.4)                                                                         |
| 16  | the planar string-diagram line can substitute for the certificate line               | **no** — it would silently narrow a symmetric quotient to a planar one, and the carriers do not translate (§12.6)                                                                   |
| 17  | the residual theory refuses to compose treks                                         | **refuted** — the passage concerns residuation, not composition (§12.4)                                                                                                             |
| 18  | "prefer representations in which perturbation is local" explains four decisions      | **refuted as stated; salvaged** as bounded address-sensitivity, explaining one (§13.1)                                                                                              |
| 19  | the optimal-reduction result is a theorem identifying the algorithm with a semantics | **it is called a thesis by its own authors**; the theorem-level content is narrower, and the cost-model exclusions are conceded in the source (§13.1)                               |
| 20  | uniqueness of primitive factorization comes from the existence proposition           | **mis-loaded** — cite the uniqueness lemma; the proposition gives existence only (§12.1)                                                                                            |
| 21  | the scaling firewall stands, therefore the nerve route is unavailable to gandr       | **conflation.** The firewall governs layout identities; the nerve governs pasting identities (§1.2, M13)                                                                            |
| 22  | "fuel" is one quantity                                                               | **two.** Value-replay cost and shape-search cost are different measures with different bounds (§9.6)                                                                                |
| 23  | the computad pathology threatens computads-as-data                                   | **it does not** — three hypotheses, none met; and the escape hatch is non-unitality, not many-to-one (§4.3)                                                                         |
| 24  | the tight/loose question is a three-way architectural fork                           | **a stratification**, not a fork (§11.4)                                                                                                                                            |
| 25  | the polynomial-to-virtual-double-category bridge is ours to build                    | **it is built** (§11.4)                                                                                                                                                             |
| 26  | exponentiability is the presheaf paper's result                                      | **misattributed** — it is the familial-enrichment paper's, and it follows from a discrete opfibration (§11.4)                                                                       |
| 27  | the abstract nerve theory paper is a blocking acquisition                            | **retired** — everything used from it is stated and proved in a monograph already held (§19)                                                                                        |
| 28  | one particular literature table is the shape table for these rungs                   | **it is a different paper's figure**; the other source's table is a different analogy entirely (§18)                                                                                |

## 18. Citation defects and hazards

Recorded because each would otherwise be re-derived, and several are in works the development depends on.

**Defects in the literature.**

* A combinatorics survey cites "every Segal space is a decomposition space" at the wrong proposition number in its companion paper; the statement is two propositions later, and the cited one is only invoked in its proof.
* A properads paper cites propositions from an arities paper that **do not resolve** in the version of record; they resolve only against the first preprint version, where they carry different numbers.
  Version drift, not error — but it costs an hour to discover.
* The computad-pathology result is cited defectively by **both** readily available routes: one monograph cites a 2001 personal letter and never the published article, and a second source gives the letter's title with the article's metadata.
  **Cite the journal article by digital object identifier.**
* A bipermutative-category source corrects a published claim that both distributors are identities in the matrix model.
* The certificate line uses "primitive" and "irreducible" interchangeably, and the irreducible object in its freeness theorem is never defined.
* A focalisation paper has the two polarity shifts **swapped** in one proposition and both surrounding bullets.
  Do not copy those labels.
* The planar string-diagram paper carries **three** initial-matching bibliography keys; the one attached to its topological-graph-theory theorem is _not_ the coherence source it is easily mistaken for, and the coherence attribution is to an unpublished manuscript.
  Its coherence contribution is also in a later section than an intermediate report of ours claimed.

**Hazards in gandr's own records.**

* The two certificate-line register rows carry **wrong digital object identifiers and years** in the consolidated bibliography; one paper is a year later than recorded, and one printed definition is on the page before the one cited.
* An earlier note asserted that a particular shape table came from one author's paper.
  It is a different paper's figure; the first author's table is a different tree-to-graph analogy, and the actual shape-to-graph-class mapping there is one sentence of prose.
  The error propagated into a session's briefs.
* Two distinct papers by the same author were conflated under one reference in the acquisition list: the nerve-theorem content and the weak-cartesianness content are in **different papers, different journals, different years**.
  Only the second is still missing, and it is the one that matters if the cartesianness mechanism is ever pursued directly.
* Two contemporaneous works on exponentiable virtual double categories by **different authors** were treated in a research map as two versions of one work.
  They are independent and mutually acknowledging.

**The two unverified items, marked.**

1. **The bipermutative locator and the rigidity package around it** (§7.3).
   The report was never adversarially checked — its verifier died mid-run — and two of its three sibling reports came back overstated.
   Partial repair exists: the two Σ-freeness results it depended on are cited and relied upon by an independent source (§6.2), so the rigidity package survives; only the bipermutative proposition number still wants confirmation.
2. **Two different works share an initial-matching key across sources.** The source of the Σ-freeness lemmas cited in the rectification paper is _not_ the same work as the bipermutative-category monograph, despite both being by the same author pair.
   **Resolve the key before citing either as the other.**

**A dangling reference in tracked source.** The carrier module's header cites a commitment by a name that, until this document, had no tracked referent. §4.4 supplies it.
The one-line fix in that header is left to the module's owner rather than made here, since this pass is scoped to the specification tree.

## 19. Sources

Locators are given at first mention throughout; this section consolidates them and records what the citation register still needs.
Works marked **[held]** are in the local research corpus; **[unheld]** are wanted; **[locator unconfirmed]** means the arc has the work and the content but has not confirmed the bibliographic identifier byte-for-byte, and the register row must be built from the artifact rather than from this document.

**The nerve, the graphical categories, and rectification.**

* Hackney, Robertson & Yau — _Infinity Properads and Infinity Wheeled Properads_, Lecture Notes in Mathematics 2147, Springer (2015) **[held; locator unconfirmed]**.
  The nerve theorem (properads and wheeled properads), edge-determination, the finiteness criterion, the graphical category and its rungs, the named restriction to the dioperad rung.
* Hackney, Robertson & Yau — _Modular operads and the nerve theorem_, arXiv:1906.01144 **[held]**; companion arXiv:1906.01143.
  Set-level and 1-categorical throughout; the unrootedness caveat of §8.4.
* Chu & Hackney — _On rectification and enrichment of ∞-properads_, arXiv:2007.00634v3 **[held]**.
  The rectification dichotomy, the Σ-freeness mechanism, the graph-category-invariance theorem, the orthogonal factorization system, the rung table.
* Raynor — _Modular operads, iterated distributive laws and a nerve theorem for circuit algebras_, arXiv:2412.20262v4 **[held]**; _Graphical combinatorics and a distributive law for modular operads_, arXiv:1911.05914v3 **[held]**; _Functorial, operadic and modular operadic combinatorics of circuit algebras_, arXiv:2412.20260v2 **[held]**.
  The set-level many-to-many nerve theorem and its transport to wheeled props; the arities counterexample.
* Hackney — _Categories of graphs for operadic structures_, arXiv:2109.06231 **[held]**; Yeung — _Ribbon dioperads and modular ribbon properads_, arXiv:2202.13269 **[held]**; Gan — _Koszul duality for dioperads_, arXiv:math/0201074 **[held]**.
* Kock — _Graphs, hypergraphs and properads_ **[held; locator unconfirmed — draft consulted, page numbers are draft pages]**.
  The weak-cartesianness claim and its hedge; the free-properad finiteness hazard.

**Segal conditions, arities, and Reedy structure.**

* Chu & Haugseng — _Homotopy-Coherent Algebra via Segal Conditions_, arXiv:1907.03977v3 **[held]**.
  Restriction of Segal objects at any target; the free Segal object and its monadicity at `Set`; the inert/active/elementary structure; the groupoid-replacement footnote that gates the later sections.
* Barkan — _Arity Approximation_, arXiv:2207.07200v4 **[held]**.
  Algebraic subpatterns, the level-graded toolkit, the set-level Morita notion, and the failure of the Morita half for this inclusion.
* Barkan & Steinebrunner — arXiv:2211.02576v3 **[held]** (the equifibered route and its set-level failure); _Segalification and the Boardman–Vogt tensor product_, arXiv:2301.08650v3 **[held]**.
* Haine, Ramzi & Steinebrunner — arXiv:2503.03916v1 **[held]**.
  Reedy extensions, latching and matching as a bigluing pullback, and the identification of the per-degree new data with delooped automorphism groups.
  For the 1-categorical case cite the classical bigluing results that paper's own remark directs to, not that paper.
* Berger, Melliès & Weber — _Monads with arities and their associated theories_, arXiv:1101.3064 / journal version **[held]**.
  The nerve theorem, monads with arities, and the connectedness criterion; note the version drift of §18.
* Melliès — _Segal Condition Meets Computational Effects_, LICS 2010, 150–159 **[held]**.
* Weber — _Familial 2-functors and parametric right adjoints_, TAC 18 (2007) no. 22, 665–732 **[held]**; _Operads as polynomial 2-monads_, TAC 30 (2015) 1659–1712 **[held]**; _Generic morphisms, parametric representations and weakly Cartesian monads_, TAC 13 (2004) 191–234 **[unheld]** — the one that matters if the cartesianness mechanism is pursued.

**Polynomial functors, species, and symmetry.**

* Batanin & Berger — _Homotopy theory for algebras over polynomial monads_, TAC 32 (2017) no. 6 **[held]**.
  Polynomial monads are cartesian; the correspondence with freely acting symmetry groups; ordered graphs as the mechanism; the shape figure.
* Kock — _Data types with symmetries and polynomial functors over groupoids_, arXiv:1210.0828 **[held]**.
  The freeness characterization; the bad-quotient argument; positions stay discrete while shapes become groupoids; homotopy quotients add arrows rather than identify.
* Gálvez-Carrillo, Kock & Tonks — _Decomposition spaces, incidence algebras and Möbius inversion I_, Adv.
  Math. 331 (2018) **[held]**; _Decomposition spaces in Combinatorics_, arXiv:1612.09225 **[held]**; _Decomposition spaces and restriction species_, IMRN 2020, 7558–7616, arXiv:1708.02570v1 **[held]**.
  Local discreteness; the Segal-implies-decomposition bridge; the planarity-as-cartesian-transformation result; the symmetry warning; progressive graphs and free PROPs; local finiteness is not automatic.
* Gepner, Haugseng & Kock — _∞-Operads as Analytic Monads_, IMRN 2022, arXiv:1712.06469v3 **[held]**.
  The polynomial classification; analytic functors; the Segal condition as right Kan extension from elementary trees; the "usual infinite ladder" passage; "a structure, not a property".
* Hackney & Kock — _Free Decomposition Spaces_, arXiv:2210.11192v2 **[held]**; Dyckerhoff & Kapranov — _Higher Segal Spaces I_, arXiv:1212.3563v1 **[held]**.
* Batanin, Kock & Weber — _Regular Patterns, Substitudes, Feynman Categories and Operads_, TAC 33(7) (2018) 148–192, arXiv:1510.08934v3 **[held]**; Batanin, De Leger & White — arXiv:2311.07322v1 **[held]**.
* Yau & Johnson — _A Foundation for PROPs, Algebras, and Modules_, AMS Mathematical Surveys and Monographs 203 (2015) **[held]**.
  The bipermutative category, the index formula, the strictification reach, the distributor facts, and the reversible-language treatment.
  **Two locators here are unverified — see §18.**
* Carette & Sabry — _Computing with semirings and weak rig groupoids_, ESOP 2016, LNCS 9632, 123–148 **[held]**.
* Elgueta — JPAA 225 (2021) 106738, arXiv:2004.08684 **[held]**; Mimram et al. — _A Cartesian (2,1)-Category of Homotopy Polynomial Functors in Groupoids_ **[held; author list unconfirmed]**.
* Yorgey — _Combinatorial Species and Labelled Structures_, PhD thesis, University of Pennsylvania (2014) **[held]**; Joram & Veltri — _Constructive Final Semantics of Finite Bags_, ITP 2023, 20:1–20:19 **[held]**; Palmgren — on equality of objects in categories in constructive type theory **[held; locator unconfirmed]**.

**Virtual double categories and the doctrine layer.**

* Nasu — _Logical Aspects of Virtual Double Categories_, arXiv:2501.17869v2 **[held]**; and the companion internal-logic paper **[held]**.
* Arkor & McDermott — _The Formal Theory of Relative Monads_, arXiv:2302.14014v5 **[held]**; _The Nerve Theorem for Relative Monads_, arXiv:2404.01281v2 **[held]**.
* Arkor — _Exponentiable Virtual Double Categories and Presheaves for Double Categories_, arXiv:2508.11611v3 **[held]**.
  The numbered bridge to the convolution product.
* Thompson & Carlson — _Exponentiable Virtual Double Categories and Representability of Exponentials_, arXiv:2605.20586 **[held]**.
  The colax convolution on presheaves over a virtual double category; positive globular decompositions; the unconditional completion.
  A **different work** from the previous entry (§18).
* Fujii & Lack — _The Familial Nature of Enrichment over Virtual Double Categories_, arXiv:2507.05529v1 **[held]**.
  Polynomial 2-functors in the virtual setting; exponentiability from discrete opfibrations; the families and elements constructions.
* Spivak, Garner & Fairbanks — _Functorial Aggregation_, arXiv:2111.10968v7 **[held]**.
  The bridge diagram and its three-step evaluation; prafunctors and retrofunctors; the étale condition; aggregation is not functorial.
* Lambert & Patterson — _Cartesian double theories_, arXiv:2310.05384 **[held]**; Patterson — _Products in double categories, revisited_, TAC 45:16 **[held]**.
* Clarke, Scherer & Zeilberger — _The Free Bifibration on a Functor_, arXiv:2511.07314v3 **[held]**.
* Laretto, Loregian & Veltri — _Di- is for Directed_, POPL 2026, arXiv:2409.10237v2 **[held]**.
* Blom, Loubaton & Ruit — _Day Convolution for Algebraic Patterns_, arXiv:2603.29815 **[held, unread]**.

**Rewriting, certificates, and the residual line.**

* Behr, Harmer & Krivine — _Fundamentals of Compositional Rewriting Theory_, arXiv:2204.07175v3 **[held]**.
  The crDC axioms, multi-sums, multi-opfibrations, the concurrency and associativity theorems.
* Behr, Melliès & Zeilberger — _Convolution Products on Double Categories and Categorification of Rule Algebras_, FSCD 2023, LIPIcs 17:1–17:20 **[held]**.
* Behr — _Tracelets and Tracelet Analysis of Compositional Rewriting Systems_, ACT 2019, EPTCS 323 **[held]**; Behr & Kock — _Tracelet Hopf Algebras and Decomposition Spaces_, ACT 2021, EPTCS 372, 323–337 **[held]**.
  **Both register rows carry wrong identifiers and years — see §18.**
* Behr, Danos, Garnier & Heindel — arXiv:1612.06240 **[held]**; Behr & Sobociński — arXiv:1807.00785 **[held]**; Behr — arXiv:2102.02364 **[held]**; Behr, Heckel & Ghaffari Saadat — GCM 2020, EPTCS 330, 126–144 **[held]**.
* Melliès — _Axiomatic Rewriting Theory_ I, II, III and VI, and _Five Basic Concepts of Axiomatic Rewriting Theory_ **[held; locators unconfirmed]**.
  The two-dimensional transition system and the freely-chosen tile relation; standardization; the reversible permutation equivalence; treks; the ancestor function.
* Delpeuch & Vicary — _Normalization for planar string diagrams and a quadratic equivalence algorithm_, LMCS 18(1):10 (2022) **[held]**.
  **Two live mis-citation traps — see §18.**
* Ara, Burroni, Guiraud, Malbos, Métayer & Mimram — _Polygraphs: From Rewriting to Higher Categories_, Cambridge University Press (2025), DOI 10.1017/9781009498968 **[held]**.
  The pathology's mechanism and scope; the two presheaf subcategories; the categorical restatement of the residual line with a decision procedure; the finite-derivation-type counterexample above dimension one.
* Makkai & Zawadowski — _The category of 3-computads is not cartesian closed_, JPAA 212(11) (2008) 2543–2546, DOI 10.1016/j.jpaa.2008.04.010 **[held]**; independent proof by Cheng, Cahiers LIV(1) (2013) 3–12.
* Henry — _Non-unital polygraphs form a presheaf category_, Higher Structures 3(1) (2019) 248–291 **[unheld]** — gates Q5.
* Bezem, Klop & de Vrijer (eds.) — _Term Rewriting Systems_, Cambridge Tracts in Theoretical Computer Science 55 (2003) **[unheld]** — **the top acquisition**; the proof-term presentation of permutation equivalence has no held substitute.
* Lafont — _Interaction Combinators_ **[held]**; Lamping — POPL 1990 **[held]**; Gonthier, Abadi & Lévy — POPL 1992 **[held]**; Asperti & Laneve — _Interaction systems II_ **[held]**; Lawall & Mairson — _Optimality and inefficiency: what isn't a cost model of the lambda calculus?_ **[unheld]**.
* Aehlig & Joachimski — _Continuous normalization for the lambda calculus and Gödel's T_ **[held; locator unconfirmed]**; the earlier root is possibly a 1978 paper on finite investigations of transfinite derivations **[unheld, recall-grade]**.
* Koslowski — _A Monadic Approach to Polycategories_, TAC 14 no. 7 (2005) 125–156 **[held]**.

**The kernel, effects, and the game-semantics line.**

* Levy — call-by-push-value, in its conference, journal and thesis forms **[held]**.
* Munch-Maccagnoni — thesis, for the polarized/duploid material **[held]**; Mangel, Melliès & Munch-Maccagnoni — _Syntax and semantics of focalisation with relative monads and comonads_, arXiv:2606.14652 **[held]**.
  The tight/loose stratification of §11.4.
  **Its own polarity-shift erratum is recorded in §18.**
* Melliès — _Asynchronous Template Games and the Gray Tensor Product of 2-Categories_, arXiv:2105.04929 **[held]**; Melliès & Stefanesco — _Concurrent Separation Logic Meets Template Games_, arXiv:2005.04453 **[held]**; Oliveira Vale, Melliès, Shao, Koenig & Stefanesco — _Layered and Object-Based Game Semantics_ **[held; locator unconfirmed]**.
  The interchange strengths of §12.5; the certified-implementation criterion for handlers; the fibration properties of the machine comparison.
* Sullivan, Downen & Ariola — _Closure Conversion in Little Pieces_, PPDP 2023 **[held]**; Schuster, Brachthäuser & Ostermann — ICFP 2020 **[held]**; Xie & Leijen — ICFP 2021 **[held]**; Biernacki, Piróg, Polesiuk & Sieczkowski — POPL 2018/2019 **[held]**; Sieczkowski, Pyzik & Biernacki — ICFP 2023 **[held]**.
* Earnshaw, Hefford & Román — CSL 2024, and Román's _Monoidal Context Theory_ thesis **[held]**; Braithwaite & Román — functor boxes **[held]**.

**Descriptions, coherence syntax, and the type-theoretic frame.**

* Chapman, Dagand, McBride & Morris — _The Gentle Art of Levitation_, ICFP 2010 **[held]**; Dagand — _A Cosmology of Datatypes_, PhD thesis (2013) **[held]**; Dagand & McBride — LICS 2013 **[unheld]**.
* Gratzer — _An Inductive-Recursive Universe Generic for Small Families_, arXiv:2202.05529v1 **[held]**.
  Carried only for the "small induction-recursion is innocuous" point and the weak/strong universe distinction; its main construction is in a different setting with different assumptions and is **not** transferable.
* Cockx, Devriese & Piessens — pattern matching without K **[held]**.
* Finster & Mimram — _A type-theoretical definition of weak ω-categories_, LICS 2017 **[held]**; Squier; Guiraud & Malbos; Mac Lane's coherence theorem; Lumsdaine; van den Berg & Garner **[held]**.
* Abbott, Altenkirch, Ghani & McBride — quotient containers **[held]**; a monadic-container treatment distinguishing a map from an isomorphism in the decomposition condition **[held; locator unconfirmed]**.
* Bonchi et al. — the tape-diagram line, arXiv:2210.09950, arXiv:2410.03561, arXiv:2606.19017 **[held]**.
  Completeness theorems for rig categories, relevant to the faithfulness theorem the arena development declines.

### 19.1 Register gap

The central citation register covers the kernel, effects, doctrine, polygraph, tracelet and levitation lines.
It does **not** yet cover the properad, nerve, Segal-condition, arity-approximation, Reedy, rectification, circuit-algebra, or residual-theory works above — which is most of §§5–9 and §12.4.

Adding them is a register edit followed by re-derivation, never a hand edit of the derived file, and it is out of scope for this pass.
Until it lands, this document's inline locators are the authority, and the two `[locator unconfirmed]` classes above must be resolved **from the artifacts** rather than from this document when the rows are built.
