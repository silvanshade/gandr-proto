{-# OPTIONS --safe --without-K --hidden-argument-puns #-}

------------------------------------------------------------------------------
-- Gandr.Shape.Decidable — maps of cell shapes, and where decidable equality
-- actually comes from.
--
-- The surrounding design rests on comparing stored representations to decide a
-- semantic question, and the licence for that is meant to be edge-
-- extensionality: a map of shapes is determined by its action on the FINITE
-- edge set, so equality of maps is decidable. This module builds the map layer
-- `Gandr.Shape.Graph` deliberately stopped short of, and then proves that
-- statement rather than assuming it.
--
-- ── WHAT IS PROVED, AND WHAT IS ONLY CITED ──────────────────────────────────
-- HRY Cor 6.62 is a statement about GRAPHICAL maps — the morphisms of `Γ`,
-- which are built from subgraph inclusions and are a larger and more delicate
-- class than the homomorphisms below. That corollary is CITED by the plan as
-- the reason the property is expected to survive into that setting; it is NOT
-- what is proved here and this module does not pretend otherwise.
--
-- What IS proved here is the analogue for `GMap`, the structure-preserving maps
-- of the shape objects this tree actually carries. `edge-determined` is a
-- theorem about those, with a hypothesis, and `_≟map_` is its decision
-- procedure. When the graphical category lands, this is the statement it has to
-- reproduce, not a lemma it can import.
--
-- ── THE HYPOTHESIS IS REAL, AND IT IS EXHIBITED ─────────────────────────────
-- Edge-extensionality is FALSE without `Grounded` — every vertex attached to
-- some edge — and the counterexample is at the bottom of this module rather
-- than left to the reader's good faith. `corolla 0 0` is a single isolated
-- vertex with no edges at all, so any two maps out of it agree on every edge
-- vacuously while being free to send its vertex anywhere. `pt-differ` is that,
-- concretely. An isolated vertex is invisible to the edge set, which is exactly
-- what the hypothesis rules out.
--
-- Note this is not an artificial restriction smuggled in to make a theorem go
-- through: the graphs the shape class cares about are connected with at least
-- one edge, and `chain-grounded` and `corolla-grounded` discharge it for the
-- worked examples that have any edges at all.
--
-- ── WHY THE CONCLUSION IS `≐` AND NOT `≡`, WHICH IS A WALL AND NOT A CHOICE ─
-- The design sketch asks for `(f g : G ⟶ H) → … → f ≡ g` and then a `Dec`
-- of that. Propositional equality of the RECORDS is not available here, and
-- the reason is exact rather than incidental: `GMap`'s fields are FUNCTIONS,
-- and `--without-K` Agda proves no function extensionality. From agreement at
-- every vertex and every edge one cannot conclude the two vertex actions are
-- equal as functions, so one cannot conclude the records are equal — and this
-- is so even though the two incidence-proof fields ARE pointwise unique, since
-- `Maybe (Fin n)` has decidable equality and Hedberg applies.
--
-- So the equality decided here is `_≐_`, agreement at every vertex and every
-- edge. That is not a weakening dressed up: it is the setoid equality of maps,
-- and this tree has SETOID rather than SET precisely so that such a statement
-- can be made honestly instead of being forced through an axiom. `mapˢ` below
-- packages it as an agda-stdlib bundle, which is what makes `G ⟶ H` an object
-- of the ambient `SETOID` — with the import deliberately not taken, since
-- `Gandr.Category.Instances` would drag `--guardedness` into a module that is
-- otherwise finite combinatorics, and the bundle is the shared interface
-- anyway.
--
-- ── RELATION TO `Gandr.Rigid`, WHICH IS CLOSE BUT NOT AN INSTANCE ───────────
-- `Gandr.Rigid` decides a setoid's equivalence by canonicalizing and comparing
-- representatives. This module reaches the same conclusion — a decision
-- procedure for the setoid relation — by a different route, and there is no
-- `Rigid` instance here, for a reason worth stating. `Rigid` needs `canon` to
-- land in propositional equality, and propositional equality of `GMap` is
-- exactly what the funext wall above denies; no choice of canonical
-- representative can repair that, because the obstruction is in the ambient
-- theory and not in the representation.
--
-- What replaces canonicalization is finiteness. The edge set already IS the
-- canonical datum, so there is nothing to normalize — `all?` over `Fin` does
-- the whole job. That is the honest form of "decidability comes from the finite
-- edge set".
--
-- ── ON `opaque` ─────────────────────────────────────────────────────────────
-- Absent. Everything here is either a record a consumer pattern-matches on
-- (`Side`, `Attached`, `_≐_`) or a proof term nothing eliminates, so no
-- definition has an unfolding cost to control, and the worked examples below
-- rely on the incidence tables reducing.
--
-- ── WHAT IS NOT HERE, WITH ITS OBLIGATION STATED ────────────────────────────
-- Decidable equality of OBJECTS. The sketch also asks for `(G H : Graph) →
-- Dec (G ≅ H)`, and neither half of that is reachable yet:
--
--   * `Dec (G ≡ H)` is blocked by the same funext wall, one level up — `Graph`
--     carries `src`, `tgt`, `inp` and `out` as functions. The repair is the
--     same as here: define pointwise equality of graphs and decide THAT. It is
--     more than a transcription because the two graphs' vertices and edges
--     inhabit different types until their cardinalities are identified, so the
--     relation has to carry `∣V∣ G ≡ ∣V∣ H` and `∣E∣ G ≡ ∣E∣ H` and transport
--     along them. That is index bookkeeping of real size and it is deferred.
--   * `Dec (G ≅ H)` — ISOMORPHISM — is strictly harder and is not merely the
--     above. Deciding it means searching the space of candidate maps. That
--     space is finite, so the decision exists in principle, but realizing it
--     needs an enumeration of the finite function space `Fin p → Fin q` that
--     this tree does not have. Nothing below depends on either, and the map
--     layer is the prerequisite for both.
--
-- Also absent: the graphical category itself. Worth recording that the levels
-- work out, since that is not obvious — `Graph`, `GMap` and `_≐_` all live in
-- `Set`, so the carrier ∞-graph sits at level zero and needs none of the
-- universe step that `SETOID` pays for its objects. So `Θ` is a `Category` on
-- an ∞-graph in the ordinary way whenever someone wants it; what it waits on is
-- the graphical maps, not a level.
------------------------------------------------------------------------------

module Gandr.Shape.Decidable where

open import Gandr.Shape.Graph
  using (Graph)
  using (Vtx)
  using (Edg)
  using (src)
  using (tgt)
  using (chain)
  using (corolla)
  using (corolla-tgt-leg)
  using (0#)
  using (1#)
  using (2#)

-- Arithmetic and finite-index vocabulary, repackaged so `zero`/`suc` read
-- unambiguously and the decision procedures keep their stdlib names.
private
  module ℕ where
    open import Data.Nat.Base public
      using (ℕ)
      using (suc)
      using (_+_)
  module Fin where
    open import Data.Fin.Base public
      using (_↑ˡ_)
    open import Data.Fin.Properties public
      using (all?)
      renaming (_≟_ to _≟ᶠ_)
  module Maybe where
    open import Data.Maybe.Base public
      using (just)
      using (nothing)
      using (map)
    open import Data.Maybe.Properties public
      using (just-injective)
open ℕ
  using (ℕ)
open Maybe
  using (just)
  using (nothing)
  using (just-injective)

open import Level
  using (0ℓ)
open import Relation.Binary.Bundles
  using (Setoid)
open import Relation.Binary.PropositionalEquality
  using (_≡_)
  using (refl)
  using (cong)
  using (sym)
  using (trans)
open import Relation.Nullary.Decidable
  using (Dec)
  using (map′)
open import Relation.Nullary.Negation
  using (¬_)

-- ════════════════════════════════════════════════════════════════════════════
-- MAPS OF SHAPES. A map acts on vertices and on edges and preserves incidence
-- — and `Maybe.map` is what makes "preserves incidence" say the right thing at
-- a leg: a dangling end goes to a dangling end, an attached end to the image
-- vertex, in one equation rather than two cases.
--
-- The chosen orders are NOT required to be preserved. That is deliberate: the
-- listings are representation content, so a map that respected them would be
-- the planar notion the surrounding design declined, not the general one.
-- ════════════════════════════════════════════════════════════════════════════

record GMap (G H : Graph) : Set where
  field
    -- the action on vertices
    onV : Vtx G → Vtx H
    -- the action on edges
    onE : Edg G → Edg H
    -- an edge's source goes to its image's source, legs included
    onE-src : (e : Edg G)
      → src H (onE e) ≡ Maybe.map onV (src G e)
    -- and likewise its target
    onE-tgt : (e : Edg G)
      → tgt H (onE e) ≡ Maybe.map onV (tgt G e)
open GMap public

-- The identity map. Both incidence obligations are reflexivity, since
-- `Maybe.map` of the identity is the identity on the nose at each constructor.
idn-gmap : (G : Graph) → GMap G G
idn-gmap G .onV v = v
idn-gmap G .onE e = e
idn-gmap G .onE-src e with src G e
... | just _ = refl
... | nothing = refl
idn-gmap G .onE-tgt e with tgt G e
... | just _ = refl
... | nothing = refl

-- ════════════════════════════════════════════════════════════════════════════
-- ATTACHMENT. A vertex the edge set cannot see is a vertex no amount of edge
-- data determines, so this is exactly the hypothesis edge-extensionality needs.
-- ════════════════════════════════════════════════════════════════════════════

-- Which end of `e` the vertex `v` sits at. A datatype rather than a sum,
-- because the two cases are eliminated separately in every consumer.
data Side (G : Graph) (e : Edg G) (v : Vtx G) : Set where
  -- `e` leaves `v`
  leaving : src G e ≡ just v → Side G e v
  -- `e` enters `v`
  entering : tgt G e ≡ just v → Side G e v

-- Some edge is incident to `v`, at one end or the other.
record Attached (G : Graph) (v : Vtx G) : Set where
  constructor attached
  field
    -- the witnessing edge
    edge : Edg G
    -- and the end it meets
    side : Side G edge v
open Attached public

-- No isolated vertices: every vertex is visible to the edge set.
Grounded : Graph → Set
Grounded G = (v : Vtx G) → Attached G v

-- ════════════════════════════════════════════════════════════════════════════
-- EQUALITY OF MAPS. Pointwise, for the funext reason in the header.
-- ════════════════════════════════════════════════════════════════════════════

record _≐_ {G H : Graph} (f g : GMap G H) : Set where
  field
    -- agreement at every vertex
    atV : (v : Vtx G)
      → f .onV v ≡ g .onV v
    -- and at every edge
    atE : (e : Edg G)
      → f .onE e ≡ g .onE e
open _≐_ public

module _ {G H : Graph} where

  -- It is an equivalence, pointwise in each component.
  ≐-refl : {f : GMap G H} → f ≐ f
  ≐-refl .atV v = refl
  ≐-refl .atE e = refl

  ≐-sym : {f g : GMap G H} → f ≐ g → g ≐ f
  ≐-sym p .atV v = sym (p .atV v)
  ≐-sym p .atE e = sym (p .atE e)

  ≐-trans : {f g h : GMap G H} → f ≐ g → g ≐ h → f ≐ h
  ≐-trans p q .atV v = trans (p .atV v) (q .atV v)
  ≐-trans p q .atE e = trans (p .atE e) (q .atE e)

  -- The maps `G ⟶ H` as a setoid. This is the bundle the rest of the tree
  -- reasons through, and it is what makes `G ⟶ H` an object of SETOID.
  mapˢ : Setoid 0ℓ 0ℓ
  mapˢ = record
    { Carrier = GMap G H
    ; _≈_ = _≐_
    ; isEquivalence = record
        { refl = ≐-refl
        ; sym = ≐-sym
        ; trans = ≐-trans
        }
    }

-- ════════════════════════════════════════════════════════════════════════════
-- EDGE-EXTENSIONALITY, AND THE DECISION IT BUYS.
-- ════════════════════════════════════════════════════════════════════════════

module _ {G H : Graph} (gr : Grounded G) (f g : GMap G H) where

  -- The vertex half. A vertex is pinned by any edge incident to it: the two
  -- maps send that edge to the same place, so that edge's endpoint in `H` is
  -- the image of `v` under both, and `just` is injective.
  --
  -- Both cases are the same three-step chain read through a different
  -- projection, which is why neither is factored out — the shared skeleton is
  -- shorter than the abstraction that would capture it.
  same-onV
    : ((e : Edg G) → f .onE e ≡ g .onE e)
    → (v : Vtx G)
    → f .onV v ≡ g .onV v
  same-onV h v with gr v
  ... | attached e (leaving p) =
    just-injective
      (trans
        (sym (trans (f .onE-src e) (cong (Maybe.map (f .onV)) p)))
        (trans
          (cong (src H) (h e))
          (trans (g .onE-src e) (cong (Maybe.map (g .onV)) p))))
  ... | attached e (entering p) =
    just-injective
      (trans
        (sym (trans (f .onE-tgt e) (cong (Maybe.map (f .onV)) p)))
        (trans
          (cong (tgt H) (h e))
          (trans (g .onE-tgt e) (cong (Maybe.map (g .onV)) p))))

  -- A map is determined by its action on the edge set. The analogue of HRY
  -- Cor 6.62 for these maps — proved, not cited; see the header for what the
  -- citation does and does not cover.
  edge-determined
    : ((e : Edg G) → f .onE e ≡ g .onE e)
    → f ≐ g
  edge-determined h .atV = same-onV h
  edge-determined h .atE = h

  -- And therefore equality of maps is decidable: the edge set is finite, so the
  -- quantifier over it is, and the vertex half comes free. No canonical form is
  -- involved anywhere — the edge action already is one.
  _≟map_ : Dec (f ≐ g)
  _≟map_ =
    map′
      edge-determined
      (λ p e → p .atE e)
      (Fin.all? (λ e → f .onE e Fin.≟ᶠ g .onE e))

-- ════════════════════════════════════════════════════════════════════════════
-- THE HYPOTHESIS IS NECESSARY. Without `Grounded`, edge-extensionality fails,
-- and here is the failure rather than a promise of one.
-- ════════════════════════════════════════════════════════════════════════════

-- `corolla 0 0` is one vertex and no edges — the arity-zero corolla, which is a
-- legitimate cell shape and an isolated vertex at the same time.
corolla-0-ungrounded : ¬ Grounded (corolla 0 0)
corolla-0-ungrounded gr with gr 0#
... | attached () _

-- A map out of it is free to send its one vertex anywhere, and has no edges to
-- be constrained by.
pt : Vtx chain → GMap (corolla 0 0) chain
pt w .onV _ = w
pt w .onE ()
pt w .onE-src ()
pt w .onE-tgt ()

-- Two such maps agree on every edge, vacuously.
pt-agree : (e : Edg (corolla 0 0)) → pt 0# .onE e ≡ pt 1# .onE e
pt-agree ()

-- And are not equal. So `Grounded` is load-bearing in `edge-determined`, not
-- decorative: drop it and the conclusion is false.
pt-differ : ¬ (pt 0# ≐ pt 1#)
pt-differ p with p .atV 0#
... | ()

-- ════════════════════════════════════════════════════════════════════════════
-- AND IT IS SATISFIABLE. The shapes with edges discharge it, so the theorem is
-- not a statement about an empty class either.
-- ════════════════════════════════════════════════════════════════════════════

-- The two-vertex composite: each vertex is entered by an edge.
chain-grounded : Grounded chain
chain-grounded 0# = attached 0# (entering refl)
chain-grounded 1# = attached 2# (entering refl)

-- A corolla with at least one input leg: its vertex is entered by that leg.
-- The output-leg case is symmetric and is not restated.
corolla-grounded : (n m : ℕ) → Grounded (corolla (ℕ.suc n) m)
corolla-grounded n m 0# =
  attached (0# Fin.↑ˡ m) (entering (corolla-tgt-leg (ℕ.suc n) m 0#))

-- Reflexivity is inhabited, so `_≐_` is not the empty relation either, and the
-- decision procedure above has both answers in range.
idn-≐ : (G : Graph) → idn-gmap G ≐ idn-gmap G
idn-≐ G = ≐-refl
