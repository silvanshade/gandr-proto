{-# OPTIONS --safe --without-K --hidden-argument-puns #-}

------------------------------------------------------------------------------
-- Gandr.Arena.Tree — the derived tree-shaped presentation, and the bridge.
--
-- Owner ruling for this lane: the flat arena is the PRIMARY value
-- presentation and the tree-shaped one is derived. This module derives it and
-- proves the two agree — the same surface-to-arena split gandr already runs in
-- Rust, where the surface form is a nested shape and the stored form is a run
-- of cells.
--
-- ── WHY THE BRIDGE MATTERS TO THE VERDICT ───────────────────────────────────
-- Gandr.Arena.Coherence shows the associativity/unit hierarchy costs nothing
-- on the flat presentation. A reader is entitled to suspect that of being an
-- artifact: perhaps the flat model simply cannot SEE the structure whose
-- coherence it trivializes. `tree-flat` and `flat-tree` refute that. The two
-- presentations are isomorphic, so the flat one loses no distinction the
-- tree-shaped one draws.
--
-- The generator bridges then say the isomorphism is structure-preserving:
-- each flat generator is the tree-shaped reshuffle, read across. So the
-- collapse is a fact about the same structure under a different presentation,
-- which is exactly the claim the spike set out to test.
--
-- Note where the asymmetry sits, since it is the whole point. On the tree
-- side, reassociation is a real function that rebuilds a real nested pair. On
-- the flat side it is the identity on offsets. Both are correct, they agree
-- across the bridge, and only one of them costs anything to normalize.
------------------------------------------------------------------------------

module Gandr.Arena.Tree where

open import Gandr.Arena.Code
open import Gandr.Arena.Structure
open import Gandr.Arena.Value

open import Data.Product
open import Data.Sum
open import Data.Unit
open import Relation.Binary.PropositionalEquality

-- ════════════════════════════════════════════════════════════════════════════
-- The tree-shaped presentation: a value is a nested pair-and-injection term,
-- with the bracketing REPRESENTED. That representation is the thing the flat
-- presentation deletes.
-- ════════════════════════════════════════════════════════════════════════════

Tree : Code → Set
Tree 𝟙       = ⊤
Tree (c ⊗ d) = Tree c × Tree d
Tree (c ⊕ d) = Tree c ⊎ Tree d

-- ════════════════════════════════════════════════════════════════════════════
-- The bridge. `flat` lays a tree out; `tree` reads a layout back.
-- ════════════════════════════════════════════════════════════════════════════

flat : (c : Code) → Tree c → Val c
flat 𝟙       tt        = unit
flat (c ⊗ d) (t , s)   = pair (flat c t) (flat d s)
flat (c ⊕ d) (inj₁ t)  = inl (flat c t)
flat (c ⊕ d) (inj₂ s)  = inr (flat d s)

tree : (c : Code) → Val c → Tree c
tree 𝟙       x = tt
tree (c ⊗ d) x = tree c (proj₁ (⊗-split x)) , tree d (proj₂ (⊗-split x))
tree (c ⊕ d) x = [ (λ i → inj₁ (tree c i)) , (λ j → inj₂ (tree d j)) ]′ (⊕-split x)

-- ── The two presentations are isomorphic ────────────────────────────────────

tree-flat : (c : Code) (t : Tree c) → tree c (flat c t) ≡ t
tree-flat 𝟙       tt       = refl
tree-flat (c ⊗ d) (t , s)  =
  trans (cong (λ p → tree c (proj₁ p) , tree d (proj₂ p))
              (⊗-split-pair (flat c t) (flat d s)))
        (cong₂ _,_ (tree-flat c t) (tree-flat d s))
tree-flat (c ⊕ d) (inj₁ t) =
  trans (cong [ (λ i → inj₁ (tree c i)) , (λ j → inj₂ (tree d j)) ]′
              (⊕-split-inl (flat c t)))
        (cong inj₁ (tree-flat c t))
tree-flat (c ⊕ d) (inj₂ s) =
  trans (cong [ (λ i → inj₁ (tree c i)) , (λ j → inj₂ (tree d j)) ]′
              (⊕-split-inr (flat d s)))
        (cong inj₂ (tree-flat d s))

flat-tree : (c : Code) (x : Val c) → flat c (tree c x) ≡ x
flat-tree 𝟙       x = sym (𝟙-η x)
flat-tree (c ⊗ d) x =
  trans (cong₂ pair (flat-tree c (proj₁ (⊗-split x))) (flat-tree d (proj₂ (⊗-split x))))
        (⊗-η x)
flat-tree (c ⊕ d) x with ⊕-split x in eq
... | inj₁ i =
  trans (cong inl (flat-tree c i))
        (trans (cong [ inl , inr ]′ (sym eq)) (⊕-η x))
... | inj₂ j =
  trans (cong inr (flat-tree d j))
        (trans (cong [ inl , inr ]′ (sym eq)) (⊕-η x))

-- ════════════════════════════════════════════════════════════════════════════
-- The generators, read across the bridge. On trees each is the obvious
-- reshuffle of a nested term; the bridge says the flat generator computes the
-- same value. This is what makes the collapse a statement about the structure
-- rather than about the model.
-- ════════════════════════════════════════════════════════════════════════════

⊗assoc-tree : {c d e : Code} → Tree ((c ⊗ d) ⊗ e) → Tree (c ⊗ (d ⊗ e))
⊗assoc-tree ((t , s) , r) = t , (s , r)

bridge-⊗assoc
  : {c d e : Code} (t : Tree ((c ⊗ d) ⊗ e))
  → ⊗assoc (flat ((c ⊗ d) ⊗ e) t) ≡ flat (c ⊗ (d ⊗ e)) (⊗assoc-tree t)
bridge-⊗assoc {c} {d} {e} ((t , s) , r) = ⊗assoc-pair (flat c t) (flat d s) (flat e r)

⊗comm-tree : {c d : Code} → Tree (c ⊗ d) → Tree (d ⊗ c)
⊗comm-tree (t , s) = s , t

bridge-⊗comm
  : {c d : Code} (t : Tree (c ⊗ d))
  → ⊗comm (flat (c ⊗ d) t) ≡ flat (d ⊗ c) (⊗comm-tree t)
bridge-⊗comm {c} {d} (t , s) = ⊗comm-pair (flat c t) (flat d s)

⊗unitr-tree : {c : Code} → Tree (c ⊗ 𝟙) → Tree c
⊗unitr-tree (t , tt) = t

bridge-⊗unitr
  : {c : Code} (t : Tree (c ⊗ 𝟙))
  → ⊗unitr (flat (c ⊗ 𝟙) t) ≡ flat c (⊗unitr-tree t)
bridge-⊗unitr {c} (t , tt) = ⊗unitr-pair (flat c t) unit

⊕assoc-tree : {c d e : Code} → Tree ((c ⊕ d) ⊕ e) → Tree (c ⊕ (d ⊕ e))
⊕assoc-tree (inj₁ (inj₁ t)) = inj₁ t
⊕assoc-tree (inj₁ (inj₂ s)) = inj₂ (inj₁ s)
⊕assoc-tree (inj₂ r)        = inj₂ (inj₂ r)

bridge-⊕assoc
  : {c d e : Code} (t : Tree ((c ⊕ d) ⊕ e))
  → ⊕assoc (flat ((c ⊕ d) ⊕ e) t) ≡ flat (c ⊕ (d ⊕ e)) (⊕assoc-tree t)
bridge-⊕assoc {c} {d} {e} (inj₁ (inj₁ t)) = ⊕assoc-inl-inl {c} {d} {e} (flat c t)
bridge-⊕assoc {c} {d} {e} (inj₁ (inj₂ s)) = ⊕assoc-inl-inr {c} {d} {e} (flat d s)
bridge-⊕assoc {c} {d} {e} (inj₂ r)        = ⊕assoc-inr {c} {d} {e} (flat e r)

⊕swap-tree : {c d : Code} → Tree (c ⊕ d) → Tree (d ⊕ c)
⊕swap-tree (inj₁ t) = inj₂ t
⊕swap-tree (inj₂ s) = inj₁ s

bridge-⊕swap
  : {c d : Code} (t : Tree (c ⊕ d))
  → ⊕swap (flat (c ⊕ d) t) ≡ flat (d ⊕ c) (⊕swap-tree t)
bridge-⊕swap {c} {d} (inj₁ t) = ⊕swap-inl {c} {d} (flat c t)
bridge-⊕swap {c} {d} (inj₂ s) = ⊕swap-inr {c} {d} (flat d s)

dist-tree : {c d e : Code} → Tree (c ⊗ (d ⊕ e)) → Tree ((c ⊗ d) ⊕ (c ⊗ e))
dist-tree (t , inj₁ s) = inj₁ (t , s)
dist-tree (t , inj₂ r) = inj₂ (t , r)

bridge-dist
  : {c d e : Code} (t : Tree (c ⊗ (d ⊕ e)))
  → dist (flat (c ⊗ (d ⊕ e)) t) ≡ flat ((c ⊗ d) ⊕ (c ⊗ e)) (dist-tree t)
bridge-dist {c} {d} {e} (t , inj₁ s) = dist-inl {c} {d} {e} (flat c t) (flat d s)
bridge-dist {c} {d} {e} (t , inj₂ r) = dist-inr {c} {d} {e} (flat c t) (flat e r)
