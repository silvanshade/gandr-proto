{-# OPTIONS --safe --without-K --hidden-argument-puns #-}

------------------------------------------------------------------------------
-- Gandr.Arena.Directed — the directed alphabet at the offsets.
--
-- Partial execution of meta-spike-03 (docs/gandr/spec/metatheory/roadmap.md),
-- toward the arena's directed generalization. The design of record is
-- "characterize as the clone, build as the factorization system"
-- (docs/gandr/spec/metatheory.md; guards.md; layout-and-coherence.md):
-- `Rigid` splits as `RigidMono ∩ RigidEpi` at the monotone rung of the
-- classical ladder, with the simplex category's epi–mono factorization as
-- the normal form. This module builds the arithmetic substrate of that
-- split and measures exactly how far the offset-fixed fragment reaches.
--
-- DONE HERE, all machine-checked:
--
--   THE REALIZATIONS. All six generators of the directed alphabet — the two
--   projections, the diagonal, the two injections, the codiagonal — written
--   against Gandr.Arena.Offset. These are the maps the directed arena
--   consumes; everything below is their measured offset behaviour.
--
--   THE OFFSET-FIXED BOUNDARY. Exactly which cells each generator fixes,
--   with proofs for the positives and pinned counterexample cells for the
--   negatives: `inl` is offset-fixed verbatim (`injˡ-fixed`); `inr` shifts
--   every cell by the left block's width (`injʳ-moves`) — the base shift
--   the design's `RigidMono` names; the diagonal fixes exactly offset 0
--   (`dup-fixed₀`, `dup-moves`); each projection moves every cell past the
--   unit laws (`projˡ-moves`, `projʳ-moves`) — the unit cases being the
--   rigid ⊗-unitors already in the hierarchy; the codiagonal fixes exactly
--   the left summand (`codiag-fixed-inl`, `codiag-moves`). These witnesses
--   are the arithmetic the offset transforms (remaining, below) are stated
--   from.
--
--   THE SHIFT-0 CORE OF `RigidMono`, CLOSED. The offset-fixed maps into
--   longer runs form a category (`rigidMono-id`, `rigidMono-∘` — the
--   fixed-cell certificate never uses the extent), contain the left
--   injection (`rigidMono-injˡ`), and whisker on the sides whose extent no
--   later offset reads: the LEFT factor of a product (`rigidMono-⊗ˡ`) and
--   the RIGHT summand of a sum (`rigidMono-⊕ʳ`). The other two whiskerings
--   provably escape (`rigidMono-⊗ʳ-moves`, `rigidMono-⊕ˡ-moves`): widening
--   the right factor or the left summand changes the block arithmetic every
--   later offset is computed from. In the design's reading that escape is
--   expected, not a failure: those positions leave the offset-fixed
--   fragment and land in the monotone class — which is exactly why the
--   generalization sits at the monotone rung and not at offset-fixed.
--
-- REMAINING (the spike stays open):
--
--   * The offset transforms as named lemmas: the right injection's shift
--     (`ix (injʳ j) ≡ size c + ix j` — the arithmetic is already `ix-inr`),
--     the projections' floor-division (`ix (projˡ x) ≡ ix x / size d`, off
--     the `remQuot` decomposition `⊗-split` already computes), and the
--     diagonal's `ix (dup x) ≡ size c * ix x + ix x`.
--   * The four monotonicity proofs the layout document cites as unchecked:
--     injections, projections, and the diagonal are monotone — each is one
--     stdlib `≤`-monotonicity lemma applied to its transform.
--   * The codiagonal's order-break, witnessed at the block boundary. Needs
--     a size-2 code: at `c = 𝟙 ⊕ 𝟙`, offset 1 maps to 1 and offset 2 maps
--     to 0. (The `codiag-moves` cell here sits at size 1, where the
--     collapse happens to preserve order.)
--   * `RigidMono` carrying the shift as data (see the record's comment),
--     `RigidEpi` (offset-determined collapse — the projections and the
--     codiagonal's left leg, realized above, are its generators), and the
--     epi–mono decomposition carried as data per map.
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
-- The offset-fixed boundary: exactly which cells each generator fixes.
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

  -- The right injection shifts every cell by the left block's width: this is
  -- the base shift the design's `RigidMono` carries, exhibited at offset 0.
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
-- `RigidMono`: embeddings into longer runs. The shift-0 core, closed.
-- ────────────────────────────────────────────────────────────────────────────

-- A `RigidMono` map is an embedding that relabels nothing up to a base
-- shift: the design of record has `ext : size c ≤ size d` and
-- `fixed : (x : Val c) → ix (app x) ≡ shift + ix x` for a shift the map
-- carries as data. INCOMPLETE: this is the SHIFT-0 CORE — `fixed` is still
-- verbatim — so the right injection is not yet an inhabitant. Carrying the
-- shift admits it, and re-runs the whiskering results below with the shift
-- scaled by the block width.
record RigidMono (c d : Code) : Set where
  constructor rigidMono
  field
    app   : Val c → Val d
    ext   : size c ≤ size d
    fixed : (x : Val c) → app x ≐ x

open RigidMono

-- Closure as a CATEGORY: the fixed-cell certificate never mentions the
-- extent, and `≤` is reflexive and transitive. These two survive the
-- shift-carrying completion unchanged, with shifts adding under composition.
rigidMono-id : {c : Code} → RigidMono c c
rigidMono-id {c} = rigidMono (id-Val {c}) (≤-refl {size c}) (λ x → ≐-refl {c} {x})

rigidMono-∘ : {c d e : Code} → RigidMono c d → RigidMono d e → RigidMono c e
rigidMono-∘ r s = rigidMono (λ x → app s (app r x)) (≤-trans (ext r) (ext s))
  (λ x → trans (fixed s (app r x)) (fixed r x))

-- The left injection is the first directed inhabitant.
rigidMono-injˡ : {c d : Code} → RigidMono c (c ⊕ d)
rigidMono-injˡ {c} {d} = rigidMono (injˡ {c} {d}) (m≤m+n (size c) (size d)) (injˡ-fixed {c} {d})

-- One-sided whiskering survives, on the side whose extent no later offset
-- reads. Inside a product the block width is the RIGHT factor's extent, so
-- widening the LEFT factor moves nothing.
⊗-map-fixedˡ
  : {c c′ d : Code} (f : Val c → Val c′)
  → ((u : Val c) → f u ≐ u)
  → (x : Val (c ⊗ d)) → ⊗-map f (id-Val {d}) x ≐ x
⊗-map-fixedˡ {c} {c′} {d} f hf = ⊗-map-fixed f (id-Val {d}) refl hf (λ v → ≐-refl {d} {v})

rigidMono-⊗ˡ : {c c′ d : Code} → RigidMono c c′ → RigidMono (c ⊗ d) (c′ ⊗ d)
rigidMono-⊗ˡ {c} {c′} {d} r = rigidMono (λ x → ⊗-map (app r) (id-Val {d}) x)
  (*-mono-≤ (ext r) (≤-refl {size d})) (⊗-map-fixedˡ {c} {c′} {d} (app r) (fixed r))

-- Inside a sum the shift past a right cell is the LEFT summand's extent, so
-- widening the RIGHT summand moves nothing.
⊕-map-fixedʳ
  : {c d d′ : Code} (g : Val d → Val d′)
  → ((v : Val d) → g v ≐ v)
  → (x : Val (c ⊕ d)) → ⊕-map (id-Val {c}) g x ≐ x
⊕-map-fixedʳ {c} {d} {d′} g hg = ⊕-map-fixed (id-Val {c}) g refl (λ u → ≐-refl {c} {u}) hg

rigidMono-⊕ʳ : {c d d′ : Code} → RigidMono d d′ → RigidMono (c ⊕ d) (c ⊕ d′)
rigidMono-⊕ʳ {c} {d} {d′} r = rigidMono (λ x → ⊕-map (id-Val {c}) (app r) x)
  (+-mono-≤ (≤-refl {size c}) (ext r)) (⊕-map-fixedʳ {c} {d} {d′} (app r) (fixed r))

opaque
  unfolding ⊗-ix
  unfolding ⊗-ixˡ
  unfolding ⊗-ixʳ

  -- The other two whiskerings escape the shift-0 core — the step the design
  -- expects to leave offset-fixed for monotone, since the block-width
  -- change is a monotone reindexing rather than a shift. Widening the right
  -- factor of a product widens every block: `pair cell₁ unit` sits at
  -- `1 * 1 + 0 = 1`, and its image at `2 * 1 + 0 = 2`.
  rigidMono-⊗ʳ-moves
    : ¬ ((x : Val ((𝟙 ⊕ 𝟙) ⊗ 𝟙)) → ⊗-map (id-Val {𝟙 ⊕ 𝟙}) (injˡ {c = 𝟙} {d = 𝟙}) x ≐ x)
  rigidMono-⊗ʳ-moves h with
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
  rigidMono-⊕ˡ-moves
    : ¬ ((x : Val (𝟙 ⊕ 𝟙)) → ⊕-map (injˡ {c = 𝟙} {d = 𝟙}) (id-Val {𝟙}) x ≐ x)
  rigidMono-⊕ˡ-moves h with
    trans
      (trans
        (sym (trans
          (≡-≐ (⊕-map-inr (injˡ {c = 𝟙} {d = 𝟙}) (id-Val {𝟙}) unit))
          (ix-inr {c = 𝟙 ⊕ 𝟙} {d = 𝟙} unit)))
        (h (inr {c = 𝟙} {d = 𝟙} unit)))
      (ix-inr {c = 𝟙} {d = 𝟙} unit)
  ... | ()
