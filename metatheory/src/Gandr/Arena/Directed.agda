{-# OPTIONS --safe --without-K --hidden-argument-puns #-}

------------------------------------------------------------------------------
-- Gandr.Arena.Directed — the four offset functions, and the `≤` verdict.
--
-- Verdict record for meta-spike-03 (docs/gandr/spec/metatheory/roadmap.md).
--
-- THE QUESTION. The rigid class (Gandr.Arena.Structure) closes the invertible
-- generators under composition and whiskering, with `ext : size c ≡ size d`
-- pinning the two runs of cells to the same extent. The directed alphabet —
-- the projection, the diagonal, the injection, and the codiagonal — is what
-- the next tier adds on top of the invertible fragment. This module realizes
-- all four against the offsets of Gandr.Arena.Offset and records the two
-- tests the spike called for, every verdict machine-checked: the positives
-- are proved, the negatives are concrete counterexample cells.
--
--   WHICH SATISFY `fixed`? Only the LEFT injection, everywhere
--   (`injˡ-fixed`): it adds no offset. The right injection adds the left
--   block's width and moves every cell (`injʳ-moves`). The diagonal fixes
--   exactly the cell at offset 0 (`dup-fixed₀`) and moves every other
--   (`dup-moves`): a copy at row-major offset `b * i + i` is `i` only when
--   `i = 0`. The codiagonal fixes exactly the left summand
--   (`codiag-fixed-inl`) and moves every right cell (`codiag-moves`): its
--   right leg IS the right injection. Each projection moves every cell whose
--   dropped mate or surviving index is not at offset 0 (`projˡ-moves`,
--   `projʳ-moves`); the cases where a projection is fixed are exactly the
--   unit laws, already rigid in the hierarchy (`rigid-⊗unitr`,
--   `rigid-⊗unitl`). At 𝟙 all four collapse to the rigid identity; the
--   counterexamples live at the first non-trivial code, `𝟙 ⊕ 𝟙`.
--
--   DOES THE CLASS SURVIVE `≤`? As a CATEGORY, yes: `Rigid≤` — the rigid
--   record with `ext` weakened to `size c ≤ size d` — has identity and
--   composition (`rigid≤-id`, `rigid≤-∘`; the fixed-cell certificate never
--   used the extent equation), and the injection inhabits it
--   (`rigid≤-injˡ`). As a MONOIDAL class, no. Whiskering survives only on
--   the side whose extent never enters a later offset: the widened factor
--   may sit on the LEFT of a product (`rigid≤-⊗ˡ`) or on the RIGHT of a sum
--   (`rigid≤-⊕ʳ`). The other two positions move cells: widening the right
--   factor widens every block, so any cell with a non-zero left index shifts
--   (`rigid≤-⊗ʳ-moves`); widening the left summand shifts every right cell
--   (`rigid≤-⊕ˡ-moves`). The shrinking direction adds nothing at all: every
--   one of the four in that direction fails `fixed`, so a `Rigid≥` would be
--   the rigid class and no more.
--
-- THE SETTLEMENT. The generalization is exactly as cheap as the arithmetic
-- says and no cheaper: of the directed alphabet only the injection survives
-- tier 2, and its closure is a category with one-sided whiskering on each
-- tensor — not a monoidal class. Anything that needs the full two-sided
-- closure needs the extent equation back; that is the rigid class.
------------------------------------------------------------------------------

module Gandr.Arena.Directed where

open import Gandr.Arena.Code
open import Gandr.Arena.Offset
open import Gandr.Arena.Value
open import Gandr.Arena.Structure

open import Data.Nat
open import Data.Nat.Properties using (≤-refl; ≤-trans; m≤m+n; +-mono-≤; *-mono-≤; *-zeroʳ)
open import Data.Product using (proj₁; proj₂)
open import Data.Sum using ([_,_]′)
open import Relation.Nullary using (¬_)
open import Relation.Binary.PropositionalEquality

-- ────────────────────────────────────────────────────────────────────────────
-- The four realizations, and the identity they are whiskered with.
-- ────────────────────────────────────────────────────────────────────────────

-- The identity, named so the maps below pin their target codes. Every code
-- implicit in this module is given EXPLICITLY: `size` is not injective, so a
-- code left to inference under it blocks the unifier.
id-Val : {c : Code} → Val c → Val c
id-Val x = x

-- The projection: drop the right factor. The unit case `d = 𝟙` is `⊗unitr`.
projˡ : {c d : Code} → Val (c ⊗ d) → Val c
projˡ x = proj₁ (⊗-split x)

-- The other projection: drop the left factor.
projʳ : {c d : Code} → Val (c ⊗ d) → Val d
projʳ x = proj₂ (⊗-split x)

-- The diagonal: copy a cell into both factors.
dup : {c : Code} → Val c → Val (c ⊗ c)
dup x = pair x x

-- The left injection. It is `inl` — the only one of the four that adds no
-- offset anywhere.
injˡ : {c d : Code} → Val c → Val (c ⊕ d)
injˡ = inl

-- The right injection: every cell shifts past the left block.
injʳ : {c d : Code} → Val d → Val (c ⊕ d)
injʳ = inr

-- The codiagonal: collapse both summands onto one run.
codiag : {c : Code} → Val (c ⊕ c) → Val c
codiag {c} x = [ id-Val {c} , id-Val {c} ]′ (⊕-split x)

-- ────────────────────────────────────────────────────────────────────────────
-- Which of the four satisfy `fixed`.
-- ────────────────────────────────────────────────────────────────────────────

-- The moving cell, at the smallest non-trivial code: offset 1 of `𝟙 ⊕ 𝟙`.
private
  cell₁ : Val (𝟙 ⊕ 𝟙)
  cell₁ = inr unit

opaque
  unfolding ⊗-ix
  unfolding ⊗-ixˡ
  unfolding ⊗-ixʳ

  -- ⊕ injections.

  -- The left injection moves nothing: `⊗-ixˡ` is the identity on offsets.
  injˡ-fixed : {c d : Code} (i : Val c) → injˡ {c} {d} i ≐ i
  injˡ-fixed {c} {d} i = ix-inl {c} {d} i

  -- The right injection shifts every cell by the left block's width.
  injʳ-moves : ¬ ((x : Val 𝟙) → injʳ {c = 𝟙} {d = 𝟙} x ≐ x)
  injʳ-moves h with trans (sym (ix-inr {c = 𝟙} {d = 𝟙} unit)) (h unit)
  ... | ()

  -- ⊗ diagonal.

  -- The diagonal fixes the cell at offset 0: `b * 0 + 0 = 0`.
  dup-fixed₀ : {c : Code} (x : Val c) → ix x ≡ 0 → dup x ≐ x
  dup-fixed₀ {c} x p =
    trans (ix-pair x x)
      (trans (cong₂ (λ u v → ⊗-ix (size c) u v) p p)
        (trans (cong (λ n → n + 0) (*-zeroʳ (size c))) (sym p)))

  -- …and only that cell: the copy of offset 1 sits at row-major offset
  -- `2 * 1 + 1 = 3`.
  dup-moves : ¬ ((x : Val (𝟙 ⊕ 𝟙)) → dup x ≐ x)
  dup-moves h with trans (sym (ix-pair cell₁ cell₁)) (h cell₁)
  ... | ()

  -- ⊗ projections.

  -- The left projection of `pair unit cell₁` reads offset 0 of a cell stored
  -- at offset `2 * 0 + 1 = 1`.
  projˡ-moves : ¬ ((x : Val (𝟙 ⊗ (𝟙 ⊕ 𝟙))) → projˡ x ≐ x)
  projˡ-moves h with
    trans (sym (≡-≐ (cong proj₁ (⊗-split-pair unit cell₁))))
      (trans (h (pair unit cell₁)) (ix-pair unit cell₁))
  ... | ()

  -- The right projection of `pair cell₁ unit` reads offset 0 of a cell stored
  -- at offset `1 * 1 + 0 = 1`.
  projʳ-moves : ¬ ((x : Val ((𝟙 ⊕ 𝟙) ⊗ 𝟙)) → projʳ x ≐ x)
  projʳ-moves h with
    trans (sym (≡-≐ (cong proj₂ (⊗-split-pair cell₁ unit))))
      (trans (h (pair cell₁ unit)) (ix-pair cell₁ unit))
  ... | ()

  -- ⊕ codiagonal.

  -- The codiagonal fixes the left summand: its left leg is the identity,
  -- whose offsets the left injection already preserves.
  codiag-fixed-inl : {c : Code} (i : Val c) → codiag (inl {c} {c} i) ≐ inl {c} {c} i
  codiag-fixed-inl {c} i =
    trans (≡-≐ (cong [ id-Val {c} , id-Val {c} ]′ (⊕-split-inl {c} {c} i)))
      (sym (ix-inl {c} {c} i))

  -- …and only the left summand: the right cell collapses from offset 1 to
  -- offset 0.
  codiag-moves : ¬ ((x : Val (𝟙 ⊕ 𝟙)) → codiag x ≐ x)
  codiag-moves h with
    trans (sym (≡-≐ (cong [ id-Val {𝟙} , id-Val {𝟙} ]′ (⊕-split-inr {c = 𝟙} {d = 𝟙} unit))))
      (h cell₁)
  ... | ()

-- ────────────────────────────────────────────────────────────────────────────
-- The weakened class: `≤` on extents in place of `≡`.
-- ────────────────────────────────────────────────────────────────────────────

-- A `Rigid≤` map still relabels nothing; the two runs of cells may now differ
-- in extent, the source no wider than the target.
record Rigid≤ (c d : Code) : Set where
  constructor rigid≤
  field
    app   : Val c → Val d
    ext   : size c ≤ size d
    fixed : (x : Val c) → app x ≐ x

open Rigid≤

-- Closure as a CATEGORY survives: the fixed-cell certificate never mentions
-- the extent, and `≤` is reflexive and transitive.
rigid≤-id : {c : Code} → Rigid≤ c c
rigid≤-id {c} = rigid≤ (id-Val {c}) (≤-refl {size c}) (λ x → ≐-refl {c} {x})

rigid≤-∘ : {c d e : Code} → Rigid≤ c d → Rigid≤ d e → Rigid≤ c e
rigid≤-∘ r s = rigid≤ (λ x → app s (app r x)) (≤-trans (ext r) (ext s))
  (λ x → trans (fixed s (app r x)) (fixed r x))

-- The injection is the new inhabitant the weakening buys.
rigid≤-injˡ : {c d : Code} → Rigid≤ c (c ⊕ d)
rigid≤-injˡ {c} {d} = rigid≤ (injˡ {c} {d}) (m≤m+n (size c) (size d)) (injˡ-fixed {c} {d})

-- One-sided whiskering survives, on the side whose extent no later offset
-- reads. Inside a product the block width is the RIGHT factor's extent, so
-- widening the LEFT factor moves nothing.
⊗-map-fixedˡ
  : {c c′ d : Code} (f : Val c → Val c′)
  → ((u : Val c) → f u ≐ u)
  → (x : Val (c ⊗ d)) → ⊗-map f (id-Val {d}) x ≐ x
⊗-map-fixedˡ {c} {c′} {d} f hf = ⊗-map-fixed f (id-Val {d}) refl hf (λ v → ≐-refl {d} {v})

rigid≤-⊗ˡ : {c c′ d : Code} → Rigid≤ c c′ → Rigid≤ (c ⊗ d) (c′ ⊗ d)
rigid≤-⊗ˡ {c} {c′} {d} r = rigid≤ (λ x → ⊗-map (app r) (id-Val {d}) x)
  (*-mono-≤ (ext r) (≤-refl {size d})) (⊗-map-fixedˡ {c} {c′} {d} (app r) (fixed r))

-- Inside a sum the shift past a right cell is the LEFT summand's extent, so
-- widening the RIGHT summand moves nothing.
⊕-map-fixedʳ
  : {c d d′ : Code} (g : Val d → Val d′)
  → ((v : Val d) → g v ≐ v)
  → (x : Val (c ⊕ d)) → ⊕-map (id-Val {c}) g x ≐ x
⊕-map-fixedʳ {c} {d} {d′} g hg = ⊕-map-fixed (id-Val {c}) g refl (λ u → ≐-refl {c} {u}) hg

rigid≤-⊕ʳ : {c d d′ : Code} → Rigid≤ d d′ → Rigid≤ (c ⊕ d) (c ⊕ d′)
rigid≤-⊕ʳ {c} {d} {d′} r = rigid≤ (λ x → ⊕-map (id-Val {c}) (app r) x)
  (+-mono-≤ (≤-refl {size c}) (ext r)) (⊕-map-fixedʳ {c} {d} {d′} (app r) (fixed r))

opaque
  unfolding ⊗-ix
  unfolding ⊗-ixˡ
  unfolding ⊗-ixʳ

  -- The other two whiskerings FAIL. Widening the right factor of a product
  -- widens every block: `pair cell₁ unit` sits at `1 * 1 + 0 = 1`, and its
  -- image at `2 * 1 + 0 = 2`.
  rigid≤-⊗ʳ-moves
    : ¬ ((x : Val ((𝟙 ⊕ 𝟙) ⊗ 𝟙)) → ⊗-map (id-Val {𝟙 ⊕ 𝟙}) (injˡ {c = 𝟙} {d = 𝟙}) x ≐ x)
  rigid≤-⊗ʳ-moves h with
    trans
      (trans
        (sym (trans
          (≡-≐ (⊗-map-pair (id-Val {𝟙 ⊕ 𝟙}) (injˡ {c = 𝟙} {d = 𝟙}) cell₁ unit))
          (ix-pair cell₁ (inl {c = 𝟙} {d = 𝟙} unit))))
        (h (pair cell₁ unit)))
      (ix-pair cell₁ unit)
  ... | ()

  -- Widening the left summand of a sum shifts every right cell: `inr unit`
  -- sits at `1 + 0 = 1`, and its image at `2 + 0 = 2`.
  rigid≤-⊕ˡ-moves
    : ¬ ((x : Val (𝟙 ⊕ 𝟙)) → ⊕-map (injˡ {c = 𝟙} {d = 𝟙}) (id-Val {𝟙}) x ≐ x)
  rigid≤-⊕ˡ-moves h with
    trans
      (trans
        (sym (trans
          (≡-≐ (⊕-map-inr (injˡ {c = 𝟙} {d = 𝟙}) (id-Val {𝟙}) unit))
          (ix-inr {c = 𝟙 ⊕ 𝟙} {d = 𝟙} unit)))
        (h (inr {c = 𝟙} {d = 𝟙} unit)))
      (ix-inr {c = 𝟙} {d = 𝟙} unit)
  ... | ()
