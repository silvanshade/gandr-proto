# The layout calculus and the coherence modules

Detail behind [[../metatheory#Representation — decidability, the arena, and the layout calculus|the representation section]] and [[../metatheory#The coherence economy|the coherence economy]]: the per-former path-structure routes, the coherence modules as landed, and the arena generalization in full.

## The per-former compositional routes

The presented layout calculus is frozen at the base signature; each richer former obtains path structure compositionally:

| former                             | path structure via                                                                                                                                      | warrant                                        |
| ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------- |
| finite sums over a tag enumeration | the base stratum directly (tags are positions)                                                                                                          | the sum spine is the flat normal form          |
| finite products                    | the base stratum directly (positions multiply)                                                                                                          | the multiplicative layer, paid once            |
| nested first-order codes           | structural congruence over component isomorphisms                                                                                                       | free — congruence is already present           |
| dependent Σ with finite fibres     | closure theorem: base isomorphism plus fibrewise isomorphisms give an isomorphism on the total; finite fibres are tuples, so no function extensionality | the finite-container payoff                    |
| genuinely infinite positions       | leaves the base stratum — tabulated pointwise certificates                                                                                              | the tabulation mechanism of the equipment join |
| recursive codes                    | leaves the base stratum — bisimulation-shaped carried certificates                                                                                      | the colimit boundary                           |
| binder fields                      | nominal congruence; a path calculus under a binder is a STOP pending its own design pass                                                                | the firewall                                   |

The early-warning instrument: when a closure theorem for a new former resists, look first for a conserved quantity separating the two sides (a net crossing count found the base presentation's one genuine specification gap analytically, before any proof failed).
If one exists, the failure is specification-level, not proof-level.

## The coherence modules as landed

`Gandr.Arena.{Code,Value,Offset,Structure,Coherence,Tree}`:

- the code rig with `size` as the cardinality homomorphism, values indexed by `Fin (size c)`, and the offset algebra (`⊗ix b i j = b·i + j`, `⊕ixʳ a j = a + j`; the left injection is a no-op on offsets, which is why it never appears in a coherence obligation);
- the rigid class:

  ```agda
  record Rigid (c d : Code) : Set where
    field
      app   : Val c → Val d
      ext   : size c ≡ size d            -- same extent
      fixed : (x : Val c) → app x ≐ x    -- the identity on the run of cells
  ```

  with closure `rigid-id`, `rigid-∘`, `rigid-⊗`, `rigid-⊕`, and `rigid-inv` free;
- **`rigid-coherence`**: any two rigid words with a common source agree at value grade — so every diagram whose edges lie in the associativity/unit hierarchy commutes, at every code, with no cell imposed; the pentagon, triangle, and sum-pentagon are stated as instances at their full diagram shapes so the general theorem can be checked against the real diagrams;
- the content-carrying generators (`⊗comm`, `⊕swap`, the left distributor — the right distributor is offset-identical) with the sum hexagon and distributor naturality proved directly by induction through the β-rules;
- the exhibited witness `dist-moves` that the distributor genuinely permutes offsets, so the non-rigid classification has teeth;
- scope note: the empty code `𝟘` is excluded because it would make values uninhabited and the distributor's inverse partial — declined for partiality, with the (larger) rig-with-zero coherence family a second, independent reason to price it if ever wanted.

A type-theoretic reading of the rigid/non-rigid split worth keeping beside the computational one: in truncation-based treatments the row-major layout choice is invisible _only because of the propositional truncation_; dropping the truncation — as this development does — makes the choice observable, which is precisely why the associativity/unit generators are rigid while the symmetries and the distributor carry content.

## The residues, by class

The two ways to fall out of the rigid class are the two residues, and the classification _derives_ the word-problem growth rather than observing it:

| class                                                             | `ext` | `fixed`               | residue                            |
| ----------------------------------------------------------------- | ----- | --------------------- | ---------------------------------- |
| the hierarchy (associators, unitors)                              | ✓     | ✓                     | none — dissolved                   |
| the permutations (`⊗comm`, `⊕swap`, left distributor)             | ✓     | ✗ (genuinely permute) | symmetric-group word problem       |
| the one-way classes (projection, diagonal, injection, codiagonal) | ✗     | —                     | transformation-monoid word problem |

Neither residue is owed (both are the declined completeness half — [[guards#the declined completeness half]]).

## The arena generalization, in full

**The classical ladder** (keep the objects — linearly ordered finite cardinalities — and vary the morphisms):

| morphism class                                        | presentation              | word problem                                                         |
| ----------------------------------------------------- | ------------------------- | -------------------------------------------------------------------- |
| offset-fixed (`Rigid`)                                | none needed               | trivial — the dissolution theorem                                    |
| monotone (the augmented simplex category)             | the simplicial identities | classical, convergent; **epi–mono factorization is the normal form** |
| symmetric (the ordered-cardinality groupoid as built) | Coxeter                   | classical, convergent                                                |
| all functions (finite sets)                           | —                         | the transformation monoid                                            |

Computed from the offset formulas (unchecked in Agda; the half-day spike is on [[roadmap]]): injections are monotone (the left one offset-fixed verbatim, the right one fixed modulo one shift the arena already has); projections are monotone surjections (floor-division); diagonals are monotone injections; **the codiagonal alone is not monotone** — co-cartesian structure on ordered sets is not order-preserving — and the full transformation monoid is generated by the symmetric group plus any one non-injective map, so the codiagonal alone forces the top of the ladder.

**The construction of record**: split `Rigid` as a factorization system — `RigidMono` (offset-fixed into a longer run, modulo a base shift that is already the right-injection offset) and `RigidEpi` (offset-*determined* collapse), with `Rigid = RigidMono ∩ RigidEpi` and every map carrying its own decomposition as data.
Four reasons, the second load-bearing: it is an extension, not a replacement (everything that typechecks today is untouched, and the dissolution theorem keeps its statement verbatim); the existing record is _explained_ by the split (the isos of an orthogonal factorization system are the intersection of the classes); factorization systems are already the idiom at five layers (the generalized-Reedy factorization; the active/inert system on the graphical category; the bo-ff factorization defining the site; the factorization preorder behind the decidability fence; and this one); and building the split **is** building the simplex category's epi–mono normal form, so the decision procedure arrives with the construction.
`rigid-inv` correctly does not survive the split — an embedding has no rigid inverse — which is what a directed alphabet wants.

**The warrant is soundness**: one-way generators are outside the rigid class _definitionally_ (the extent field cannot be inhabited), so without the enlarged class every simplicial-style identity, every naturality square against the structural subsystem, and every bialgebra-style exchange across distribution becomes an individual soundness obligation at general codes — the per-generator grind the dissolution theorem exists to prevent.
The scope is the directed rule layer's actual cell list, not an open-ended redesign.

**The characterization** (recorded, not built): the enlarged arena is the clone — the algebraic theory with projections and duplication — and the published decomposition of the ladder is three monads on `Cat` (planar, symmetric, clone) related by distributive laws, landing on profunctors [@curien-2012-operads-clones].
Decompose the transformation monoid rather than grinding it monolithically: the simplex category's identities (convergent, epi–mono normal form) and Coxeter (convergent), related by a distributive law — the same technique as the term monad's own decomposition and the staged-normal-form recommendation, a fourth independent arrival.

**Limits, so this is not over-read**: the monotonicity computation is not typechecked; the simplex category is not free (a known finite cell list with a decision procedure is the cheapest non-free thing on offer, but it is a real cell list); the arena's published identity is itself flagged unverified ([[citation-hazards]]); and the dissolution of the _embedding fragment_ (weakening the extent field to `≤`, keeping `fixed`) is a prediction with a named spike, not a report.

**The tooling fallback** where dissolution is unavailable on the nose: a DSL that makes each per-former commutation instance a one-liner — the same architectural slot as the enumerator and the generate-and-check meta-operation (a tool producing candidate terms verified by already-trusted machinery, adding no trusted surface).

## The coherence-complex connection

Coherence of a _presented_ structure holds iff its coherence complex is simply connected (vertices the structural words, 2-cells the coherence diagrams) [@curien-laplante-anfossi-2023-topological]; the theorem's own Morse-theoretic variant is strictly less general, and the abstract-rewriting route to the symmetric-group residue is independently judged uninformative — corroborating the decision to stop at the dissolution-plus-decline line rather than grinding.
The quantitative sibling: coherence data above a connectivity-determined arity bound is forced rather than assumed (arity approximation [@barkan-2022-arity]), whose applicability to gandr's pattern is a named spike.
The geometric object for polygraph-presented structures exists (a CW complex with one cell per element) [@hadzihasanovic-2020-shape]; if a pasting formalism is ever needed, route via Steiner-style constructible/torsion-free complexes — **not** parity complexes or pasting schemes, which are refuted by counterexample [@forest-2022-unifying].
These are three different complexes under one phrase — see the name-collision entry in [[guards#Name collisions — read the definition, not the section title]].
The pasting structure currently has **no** coherence complex at all — its operations are defined functions with proved equations, not presented structure, and no coherence theorem addresses defined-mode equations; canonicalization is what would create one, which is why it is the item to watch.
