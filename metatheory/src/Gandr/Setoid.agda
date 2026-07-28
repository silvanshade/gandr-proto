{-# OPTIONS --safe --guardedness --without-K --hidden-argument-puns #-}

------------------------------------------------------------------------------
-- Gandr.Setoid — reflexivity, transitivity and symmetry on a carrier's cells.
--
-- A `Setoid Ξ` equips the cells of an ∞-graph with `idnˢ`, `seqˢ` and `invˢ`:
-- Bishop's setoid, and the one-dimensional base of the structure tower. The
-- whole tree is the generalization of setoids to ∞-graphs, so this is where
-- that generalization starts.
--
-- ── WHY THIS RECORD CARRIES NO LAWS ─────────────────────────────────────────
-- Nothing relates the three operations. That is weakness by default, and it is
-- the honest floor: a `Setoid` is the LAWLESS proof-relevant equivalence, so it
-- needs no strictness mark and makes no claim it cannot back. Layers that want
-- laws state them one dimension up, as cells, where they can be witnessed
-- rather than asserted.
--
-- ── WHY THIS MATTERS FOR EVERY RESULT IN THIS TREE ──────────────────────────
-- This tree does not have SET. It has SETOID. Equality of cells is a structure
-- a carrier supplies, not a proposition the ambient theory hands over, and a
-- result proved here is a result about setoids unless it says otherwise. That
-- is a real restriction on what may be claimed — quotients need not be
-- effective, and a map need not respect an equivalence unless shown to.
-- Keeping the equivalence a named, projected structure rather than ambient
-- justification is what makes the restriction checkable instead of assumed.
--
-- Standing role: the repackaging target of `Gandr.Category`'s `hom`. A
-- Category's `idn₁`/`seq₁`/`inv₁` — reflexivity, transitivity and symmetry of
-- the 2-cells over a fixed hom — ARE a `Setoid` on that hom, so the hom-setoid
-- is read back off the Category by projection rather than rebuilt.
------------------------------------------------------------------------------

module Gandr.Setoid where

open import Gandr.Graph
  using (∞Graph)
  using (ϵ°)
  using (δ°)
  using (module 𝔾)

import Relation.Binary.Bundles as Bundles
open import Relation.Binary.PropositionalEquality
  using (refl)
  using (trans)
  using (sym)

-- Reflexivity, transitivity and symmetry on the cells of `Ξ`, proof-relevantly
-- and without laws: the base the Category and Groupoid towers sit over.
record Setoid {ℓ} (Ξ : ∞Graph ℓ) : Set ℓ where
  field
    -- reflexivity: a chosen cell at every 0-cell
    idnˢ : ∀ a
      → Ξ .δ° a a .ϵ°
    -- transitivity: cells compose along a shared middle 0-cell
    seqˢ : ∀ {a b c}
      → Ξ .δ° a b .ϵ°
      → Ξ .δ° b c .ϵ°
      → Ξ .δ° a c .ϵ°
    -- symmetry: every cell reverses
    invˢ : ∀ {a b}
      → Ξ .δ° a b .ϵ°
      → Ξ .δ° b a .ϵ°
open Setoid public

-- ══════════════════════════════════════════════════════════════════════════════
-- The bridge to agda-stdlib, and why it is exact rather than approximate.
--
-- An ∞-graph's coboundary IS a relation on its cells: `Ξ .δ° x y .ϵ°` has the
-- shape `Rel (Ξ .ϵ°) ℓ` on the nose. And stdlib's `IsEquivalence` carries
-- `refl`/`sym`/`trans` and NO LAWS — so it is exactly as lawless as the record
-- above, and the bundle below loses nothing and adds nothing.
--
-- Two consequences worth naming, because they are why no bespoke reasoning
-- machinery is needed anywhere in this tree:
--
--   * Proof-relevance survives. `_≈_` is `Set`-valued and never truncated, so a
--     2-cell stays a cell rather than collapsing to a proposition.
--   * ONE bundle serves EVERY dimension. `Setoid` is parameterized by an
--     arbitrary ∞-graph, and any level's coboundary is itself an ∞-graph, so
--     the same function bundles 0-cells, homs, and every level above.
--
-- So stdlib's `begin`/`≈⟨_⟩`/`∎` applies directly. What stdlib does not supply
-- is the CATEGORICAL combinator suite — reassociation ladders, cancellers,
-- square extensions — which is `Gandr.Category.Reasoning`'s job.
-- ══════════════════════════════════════════════════════════════════════════════

-- THE DISCRETE SETOID ON THE IDENTITY TYPE: `refl`, `trans` and `sym`, over
-- `Gandr.Graph.≡°`. This is the `Setoid` every `Set`-level structure in this
-- tree presents, so `bundle (≡ˢ _)` is the bundle their reasoning chains run
-- in — the `Set`-level counterpart of reading a category's hom-setoid off
-- `homˢ`, and the reason a chain in a `Set`-level module is the same chain as
-- one in the profunctor layer rather than a second vocabulary.
≡ˢ : ∀ {ℓ} (A : Set ℓ) → Setoid (𝔾.≡° A)
≡ˢ A .idnˢ a = refl
≡ˢ A .seqˢ = trans
≡ˢ A .invˢ = sym

-- The stdlib setoid bundle this structure presents. Both levels are `ℓ`: the
-- cells and the relation on them live in the same universe, since the relation
-- is just the carrier one dimension up.
bundle : ∀ {ℓ} {Ξ : ∞Graph ℓ} → Setoid Ξ → Bundles.Setoid ℓ ℓ
bundle {Ξ} S = record
  { Carrier = Ξ .ϵ°
  ; _≈_ = λ a b → Ξ .δ° a b .ϵ°
  ; isEquivalence = record
      { refl = λ {a} → S .idnˢ a
      ; sym = S .invˢ
      ; trans = S .seqˢ
      }
  }
