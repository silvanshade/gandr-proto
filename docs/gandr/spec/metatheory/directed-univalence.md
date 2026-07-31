# Directed univalence — the statement in full

The complete design of the layout-side directed univalence statement summarized in [[../metatheory#Directed univalence|the metatheory track]]: the candidate comparison, the alphabets, the grades, the guards, the kernel formers, and the equipment inventory.

## The substrate is directed — the evidence

* Generators are oriented schemas; the backward half is a _formal_ inverse, sound only where the generator carries inverse-plus-round-trip evidence.
* Symmetrization is free doubling, not substrate: groupoid paths are words over an involutive alphabet with cancellation imposed one dimension up as oriented rules — rules over a directed substrate, not a symmetric substrate.
* Even the groupoid certificate grade is one-sided: replay-equivalence compares only the forward translator.
* The saturation alphabet is variance-typed with no inverses anywhere; the eliminator's discharge is a covariant Yoneda extension consuming no symmetry.
* The directed eliminators are precedented in the landed profunctor layer (directed path induction whose groupoid instance specializes to symmetric J; Π as right extension along a conjoint, with no inverses anywhere in that module).
  One qualification survives adversarial reading: Π-*substitution* does consume inverses at named groupoid sites — one new site per fibre depth — so the blanket "the directed development uses no inverses" must be scoped to the eliminators, not the whole Π story.

The exact primitive-to-derived relation: the groupoid path protype is the **free involutive doubling of the evidence-invertible restriction of the directed protype, with cancellation adjoined as dimension-2 rules** — a localization of a restriction.
Groupoid-as-quotient fails (the groupoid protype has more letters and more rules); directed-by-forgetting fails (the one-way classes and their rule layer are strictly new material).

## Candidates, and the statement of record

| candidate                | shape                                                                                                                                                        | verdict                                                                                                                                                                                                                                     |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **maximal**              | hom-certificates ≅ all replay-total translators (the simplicial-model statement shape [@gratzer-weinberger-buchholtz-2024-directed-univalence] transplanted) | right as the absolute form on the leaf-free fragment; **refuted over leaves** at the first infinite leaf by the constant-literal witness (a replay-total translator sending everything to one literal, reachable by no finitary vocabulary) |
| **lax/adjoint**          | every one-way generator carries one-sided adjoint evidence; round trips stated one-sidedly                                                                   | declined as the base statement (η would live at a lax grade; adjoint choices are not schema-uniform); the content is retained as equipment structure — companions/conjoints for realized tight maps                                         |
| **fenced** (RECOMMENDED) | the four obligations below, over the recursion-free description fragment with the certificate alphabet fixed to the leaf-natural one-way stock               | the statement of record                                                                                                                                                                                                                     |

The four obligations:

* **sound**: every generator schema names a translator with replay evidence; positive words realize by unconditional composition (string-shaped composites discharge the acyclicity gate structurally);
* **full**: every leaf-natural one-way certificate is replay-equal to a realized positive word — stratified fullness at the unit-plus-restriction fragment (hom formation as the unit protype restricted along the endpoint maps, one constructor class beyond the groupoid statement's unit fragment);
* **sectioned**: the constructed inverse satisfies β at replay-equivalence and η at the directed rule congruence — the grade discipline verbatim, never code equality on either side;
* **core-coincident**: the canonical comparison from the groupoid statement into the invertible core of the directed one is a bijection at the stated grades.
  Genuinely new work: invertible realizations arise from non-invertible letters (a fold after an injection realizes the identity while neither letter is invertible), so the directed rule layer needs the simplicial-identity-style cells _and_ a word-problem argument on the invertible-realization sub-stock.

## The alphabets

| alphabet         | directed form                                                                                                                                                                                                                                                      | the witness if unfixed                                                                                                                                                                                                                   |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **paths**        | positive words (no backward half) over the extended generator menu, modulo the directed rule congruence; invertibility is per-generator overlay data                                                                                                               | a thin/subtyping-style hom protype collapses the stock at the two-element code, where **four** replay-distinct one-way certificates exist (identity, negation, two constant maps) against two invertible ones — the constant-map witness |
| **instances**    | unchanged: saturated profunctor modules, already variance-typed                                                                                                                                                                                                    | unit induction declines on non-empty paths over raw instances; the repair is the absorbed-path module element                                                                                                                            |
| **certificates** | the leaf-natural one-way stock; translators uniform in leaf contents on both sides; structurally container-morphism-shaped (summand-map forward, factor-map backward [@abbott-altenkirch-ghani-2005-containers] [@ahman-chapman-uustalu-2016-directed-containers]) | the constant-literal witness refutes completeness over unrestricted stocks at the first infinite leaf                                                                                                                                    |

The one-way generator classes extending the invertible menu: projections ($A × B ⇝ A$), diagonals ($A ⇝ A × A$), injections ($A ⇝ A + B$), codiagonals ($A + A ⇝ A$).
On the leaf-free fragment these generate **all** functions between the value sets, so the fenced statement has an absolute leaf-free form; on leaved codes leaf-naturality is the exact fence — the one-way classes move positions, never leaf values.
The dimension-2 rule layer adds the simplicial-style identities among the one-way classes, the bialgebra-style exchanges across distribution, and the naturality squares against the structural subsystem; its residue after the rigid hierarchy dissolves is the transformation-monoid word problem, whose price is quoted in the retired tree presentation and **pending re-quote against the arena** ([[roadmap]]).

Grades: β at replay-equivalence (one-sided), η at the directed rule congruence.
The two-cell coherence obligation (the typoid-function condition) lands on this statement; there is no ambient identity type to degenerate to — the directed statement is forced into the realization-as-functor direction from the start, and the new degeneracy to guard is **thinness** (poset collapse), permanently guarded by the constant-map witness.

## The kernel formers

| form            | groupoid                                     | directed                                                                                                                                                                                                                             |
| --------------- | -------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| former          | `Path A x y`, endpoints invariant            | `Flow A x y`, endpoints invariant at the identity-layer phase                                                                                                                                                                        |
| intro           | `here(v)`                                    | the diagonal intro                                                                                                                                                                                                                   |
| elim            | `walk` — full dinatural J                    | the directed walk with the **motive-covariance side condition**: a motive placing the moving endpoint in the contravariant slot is refused; the check is term-structural and total, no variance-sorted contexts needed at this phase |
| composition     | derived by one walk                          | derived by the same script — composition _is_ covariant transport; directedness costs nothing at dimension 1                                                                                                                         |
| inversion       | derived                                      | **underivable by construction** — the refused motive shape is the symmetry shape                                                                                                                                                     |
| permanent guard | a K-derivation witness must fail elaboration | a symmetry-derivation witness must fail elaboration                                                                                                                                                                                  |

The two formers are independent primitives; no kernel coercion between them (the comparison is the core-coincidence theorem, and a coercion would assume it as an axiom).
Variance staging: none at the identity layer (motive-shape check only); variance-sorted contexts on the reflected layer only; the general dipresheaf variance judgment is metatheory work.
An annotation slot for the polarity/variance plane in the export format is the one cross-phase coupling to watch: cheap reserved early, a coordinated format bump across two checkers on the trusted base if retrofitted after the replay checker exists.

## The equipment inventory

The reflected directed fragment already carries, as live code in `theory-virtual-doctrines`:

1. **variance-sorted reflected contexts** — the closed two-way polarity vocabulary with the engine's `Mixed` deliberately not a directed variance (it is the dinaturality shape); the opposite-category involution lives on reflected signatures only, never on the kernel's objects;
2. **hom as directed equality** — contravariant source, covariant target, diagonal `refl`, and the polarity-restricted directed J as a total checker under which symmetry is underivable, with a named witness asserting its refusal [@laretto-loregian-veltri-2026-directed];
3. **(co)ends as quantifiers over finite discrete carriers** — Fubini and co-Yoneda as derived transformations; carriers finite, hom refl-generated — both honesty boundaries stated in the module;
4. **the boundary theorem, operational** — certificate composition routed by invertibility: wholly-invertible certificates compose ungated; everything else consults the acyclicity gate, declining with the flow cycle as diagnostic.

The gap between the inventory and the fenced statement, in order of size: no generated one-way stock (the fullness quantifier's range does not exist as data); no companions/conjoints (the unit-plus-restriction hom formation is unbuilt — the fullness statement cannot even be _stated_ on the reflection face before it lands); finite carriers only; no invertible-core comparison machinery; and the reflected-universe object with its transport law.

## Certificate-layer obligations

Directed cells impose four schema obligations on the certificate component, all representation decisions (cheap at their phase, expensive later): variance-marked boundaries; a composition-mode tag with an acyclicity witness on the directed mode; the two-mode produoidal normal form with interchange lax and staying lax after normalization (bidirectional interchange only on pure-polarity boundaries; the coherence theorem carries a distinct-typing side condition); and the η-orientation check on the one-way contraction cells against the data-η/codata-η cut discipline.
The tractability classification (convergent-fragment versus certificate-carried) is a separate axis from the invertible/directed mode axis — they coincide in today's two-band design and will not once a convergent directed fragment exists; classify by tractability _reason_ so the accident is not baked in.

The honest price, stated with its datum: the only published formalization of the symmetric-rig coherence adjacent to the groupoid statement leaves its multiplicative layer unpaid — 118 admit sites under an unsolved-metas allowance [@choudhury-karwowski-sabry-2022-symmetries] — and the directed statement adds strictly more (the one-way classes interact with distribution), so the directed convergence pass exceeds the literature on two axes at once, and no formalized rewriting twin exists for either.
This is why the convergence pass is a named obligation with a re-quoted price rather than a consultable proof shape.

## Adjacent statements of the family

The fenced statement above is one instance of the family's directed form.
Two further instances are designed under the same obligations discipline: the session-stratum reserve (an identity at a universe of protocols, flagged before either stratum hardens) and **`ua_topo`** — identity of presented spaces as per-stratum fullness of a space-code edit vocabulary, with locale-isomorphism and homeomorphism as distinct certified notions and the simple-homotopy warning (complete, or complete up to a located obstruction) as its honesty bound; its precedent stack and design consequences are in [[exact-reals#ua_topo — the statement shape and its precedent stack|the exact-reals document]].
