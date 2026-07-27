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
