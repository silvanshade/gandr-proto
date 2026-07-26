{-# OPTIONS --safe --without-K --hidden-argument-puns #-}

------------------------------------------------------------------------------
-- Gandr.Arena.Offset — the flat encoding, at the level of raw offsets.
--
-- Three formulas are the ENTIRE semantics of the code universe: a product
-- pairs row-major, and a sum injects by block. Everything the arena proves is
-- a consequence of these plus the semiring laws of `Nat`.
--
-- ── WHY THIS MODULE IS THE COST CENTRE ──────────────────────────────────────
-- The presented (tree-shaped) edit calculus must impose a Laplaza-shaped
-- coherence family: pentagon, triangle, the sum hexagon, and a distributor
-- naturality square PER GENERATOR at general codes. That family is generated
-- by the associativity/unit hierarchy, and the reason it costs anything is
-- that a tree REPRESENTS its bracketing, so reassociating is a real relabelling
-- that later generators must be slid past.
--
-- Flattening deletes the representation of bracketing. `⊗-ix-assoc`,
-- `⊗-ix-unitˡ` and `⊗-ix-unitʳ` below say so precisely: the offset computed
-- through the source bracketing is ALREADY the offset computed through the
-- target bracketing. This is the same reason concatenation is associative on
-- the flattened list — the bracketing was never there to move.
--
-- These are the load-bearing statements of the flat recast. Gandr.Arena.Value
-- lifts them to bounded values; Gandr.Arena.Structure turns them into the
-- rigidity of the generators; Gandr.Arena.Coherence discharges the family.
------------------------------------------------------------------------------

module Gandr.Arena.Offset where

open import Data.List
open import Data.Nat
open import Data.Nat.Properties
open import Data.Nat.Solver using (module +-*-Solver)
-- open import Data.Nat.Tactic.RingSolver
open import Relation.Binary.PropositionalEquality

open +-*-Solver
open ≤-Reasoning

-- ════════════════════════════════════════════════════════════════════════════
-- The encoding.
-- ════════════════════════════════════════════════════════════════════════════

-- `⊗-ix b i j` — the offset of the pair (i , j) in a product whose RIGHT factor
-- spans `b` cells. Row-major: the left index selects a block of width `b`, the
-- right index selects within it. The argument order matches `combine`'s.
opaque
  ⊗-ix : (b i j : ℕ) → ℕ
  ⊗-ix b i j = b * i + j

-- `⊗-ixˡ i` — the offset of a left injection. The left block starts at 0, so a
-- left injection is a no-op on offsets; that is why it never appears in a
-- coherence obligation.
opaque
  ⊗-ixˡ : (i : ℕ) → ℕ
  ⊗-ixˡ i = i

-- `⊗-ixʳ a j` — the offset of a right injection past a left block of `a` cells.
opaque
  ⊗-ixʳ : (a j : ℕ) → ℕ
  ⊗-ixʳ a j = a + j

opaque
  unfolding ⊗-ix
  unfolding ⊗-ixˡ
  unfolding ⊗-ixʳ

  -- ════════════════════════════════════════════════════════════════════════════
  -- THE COLLAPSE. Each law says the source bracketing and the target bracketing
  -- name the same cell — so the generator that mediates them relabels nothing.
  -- ════════════════════════════════════════════════════════════════════════════

  -- Reassociating a product does not move the cell:
  --   c * (b * i + j) + k  ≡  (b * c) * i + (c * j + k)
  -- Read left to right this is right-distribution, then multiplicative
  -- associativity and commutativity to rebuild the block width `b * c`, then
  -- additive associativity to re-split the remainder.

  ⊗-ix-assoc
    : (b c i j k : ℕ)
    → ⊗-ix c (⊗-ix b i j) k ≡ ⊗-ix (b * c) i (⊗-ix c j k)
  ⊗-ix-assoc = solve 5
    (λ b c i j k → c :* (b :* i :+ j) :+ k := b :* c :* i :+ (c :* j :+ k))
    refl

  -- Cancelling a unit on the RIGHT does not move the cell: the right factor
  -- spans one cell, whose only offset is 0.
  ⊗-ix-unitʳ
    : (i : ℕ)
    → ⊗-ix 1 i 0 ≡ i
  ⊗-ix-unitʳ = solve 1
    (λ i → con 1 :* i :+ con 0 := i)
    refl

  -- Cancelling a unit on the LEFT does not move the cell: the left factor spans
  -- one cell, whose only offset is 0, so it selects block 0.
  ⊗-ix-unitˡ
    : (b j : ℕ)
    → ⊗-ix b 0 j ≡ j
  ⊗-ix-unitˡ = solve 2
    (λ b j → b :* con 0 :+ j := j)
    refl

  -- Reassociating a sum does not move the cell: nesting two right injections
  -- adds the two block widths, and addition is associative.
  ⊗-ix-assocʳ
    : (a b k : ℕ)
    → ⊗-ixʳ (⊗-ixʳ a b) k ≡ ⊗-ixʳ a (⊗-ixʳ b k)
  ⊗-ix-assocʳ = solve 3
    (λ a b k → (a :+ b) :+ k := a :+ (b :+ k))
    refl
