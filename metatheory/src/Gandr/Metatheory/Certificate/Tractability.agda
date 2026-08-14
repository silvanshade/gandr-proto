{-# OPTIONS --safe --guardedness --without-K --hidden-argument-puns #-}

-- `--guardedness` is carried for one reason and it is recorded here rather than
-- inferred: this module imports `Gandr.Rigid` for the multiset quotient, and
-- that module reaches the ∞-graph carrier through `Gandr.Category`. Nothing
-- here is coinductive, and `Base`, `Properties`, `Examples` and `Composition`
-- are all flag-free — the infection stops at the one module that consumes the
-- effective quotient.

------------------------------------------------------------------------------
-- Gandr.Metatheory.Certificate.Tractability — the tractability axis, born here.
--
-- The design record asks for one record per tractability REASON, kept separable
-- from the invertible/directed mode tag, on the ground that the two coincide in
-- today's band structure by accident rather than by construction. This module
-- is that axis, and nothing in it mentions a mode.
--
-- ── WHAT A TRACTABILITY WITNESS IS FOR, AND WHAT IT IS NOT FOR ──────────────
-- `Base` decides replay-equivalence outright: `_≈?_` compares two terms and
-- runs four paths, with no fragment condition and no budget. **So a
-- tractability witness never buys decidability.** It buys cost, and cost is
-- invisible in this tree.
--
-- What the metatheory can therefore hold a witness to is exactly one thing, and
-- it is the thing the engine cannot check for itself: **soundness against the
-- replay oracle**. The design record names this as the part of a normal-form
-- fast path that is easiest to skip, because the code compiles without it. Here
-- it is a FIELD, so a witness cannot be constructed without it.
--
-- ── THE THEOREM THE RECORD FORCES ───────────────────────────────────────────
-- `nf-determines-boundary` is derived, not assumed: **any sound normal form
-- determines the boundary on its fragment.** A normal form that forgot the peak
-- or the join could not be sound, whatever else it recorded and however
-- canonical it was.
--
-- That is `Properties`' characterization arriving one level up, and it is the
-- same fact the engine side reached by repair when a projected-derivation
-- comparison without the boundary turned out to answer yes across different
-- boundaries. Two independent routes to one statement: **a certificate relation
-- carries the boundary or is not sound**, and everything finer than the
-- boundary is about cost or availability.
--
-- ── TWO REASONS, AND THE ASYMMETRY BETWEEN THEM ─────────────────────────────
-- `NormalFormComparison` is the convergent-fragment reason: a canonical form
-- whose comparison decides. `CarriedWitness` is the general band, where a
-- per-instance witness is the only currency.
--
-- The asymmetry is worth stating because it is what makes the axis informative.
-- `CarriedWitness` is inhabited for EVERY fragment inside validity,
-- unconditionally — `carried` builds one from the oracle — so it is never a
-- claim about a fragment. `NormalFormComparison` is a claim, and `walk-nf`
-- below is the first inhabitant.
--
-- ── SOUND, AND NOT COMPLETE, BY CONSTRUCTION RATHER THAN BY OVERSIGHT ───────
-- The first inhabitant compares the multiset of steps a derivation records, on
-- top of the boundary. It is strictly finer than replay-equivalence, and
-- `walk-nf-incomplete` exhibits the separation: two presentations of one
-- certificate whose recorded step multisets differ. So the fast path answers
-- "yes" soundly and answers "not decided here" on pairs that are in fact equal,
-- and the fall-through in `fast-path` is not an optimization detail but the
-- only correct shape.
--
-- The multiset component of that normal form carries NO soundness weight — the
-- soundness proof routes entirely through the boundary components. What it
-- carries is order-insensitivity (`walk-nf-permutation-invariant`), which is
-- what makes it a normal form rather than the recorded list, and which is where
-- its cost advantage would come from if this tree could see cost.
------------------------------------------------------------------------------

module Gandr.Metatheory.Certificate.Tractability where

open import Gandr.Metatheory.Certificate.Base
  using (ReplaySystem)
  using (module Certificates)
open import Gandr.Metatheory.Certificate.Examples
  using (Dir)
  using (up)
  using (down)
  using (walk)
  using (climb)
  using (detour)
  using (climb-replays)
  using (detour-replays)
  using (presentations)
open import Gandr.Rigid
  using (module ℕ-multiset)

open import Data.Empty
  using (⊥)
open import Data.List.Base
  using (List)
  using ([])
  using (_∷_)
  using (length)
  using (map)
open import Data.List.Properties
  using (length-map)
open import Data.List.Relation.Binary.Permutation.Propositional
  using (_↭_)
  using (↭-sym)
open import Data.List.Relation.Binary.Permutation.Propositional.Properties
  using (↭-length)
open import Data.Nat.Base
  using (ℕ)
open import Data.Nat.Properties
  using (≤-decTotalOrder)
  renaming (_≟_ to _≟ⁿ_)
open import Data.Product.Base
  using (_×_)
  using (_,_)
  using (proj₁)
  using (proj₂)
open import Level
  using (Level)
  using (suc)
  using (0ℓ)
open import Relation.Binary.Definitions
  using (DecidableEquality)
open import Relation.Binary.PropositionalEquality
  using (_≡_)
  using (refl)
  using (cong)
  using (trans)
  using (sym)
open import Relation.Nullary.Decidable.Core
  using (Dec)
  using (yes)
  using (no)
  using (map′)
  using (_×?_)
open import Relation.Nullary.Negation.Core
  using (¬_)

private
  module Sort where
    open import Data.List.Sort ≤-decTotalOrder public
      using (sort)
      using (sort-↭)
  module List≡ where
    open import Data.List.Properties public
      using (≡-dec)

------------------------------------------------------------------------------
-- The reasons.
------------------------------------------------------------------------------

module Reasons {ℓ} (S : ReplaySystem ℓ) where

  open Certificates S

  -- A FRAGMENT is a property of certificates. Nothing here requires it to be
  -- decidable, closed under anything, or related to a composition mode.
  Fragment : Set (suc ℓ)
  Fragment = Certificate → Set ℓ

  ----------------------------------------------------------------------------
  -- Reason 01 — a convergent fragment, decided by normal-form comparison.
  ----------------------------------------------------------------------------

  -- The claim: on `F`, a canonical form exists whose comparison is decidable
  -- and SOUND for replay-equivalence.
  --
  -- `valid` is a field rather than a consequence, and it is the field that
  -- makes `sound` provable at all. A normal form is a function of a certificate
  -- and can never establish that the certificate replays, so a fragment outside
  -- validity has no sound normal-form comparison — which is the same wall the
  -- second incomparability direction in `Properties` describes, met from the
  -- other side.
  record NormalFormComparison (F : Fragment) : Set (suc ℓ) where
    field
      -- the canonical form
      Nf : Set ℓ
      -- compared as data
      _≟ᶠ_ : DecidableEquality Nf
      nf : Certificate → Nf
      -- the fragment is inside validity
      valid : ∀ {c} → F c → Replays c
      -- SOUNDNESS AGAINST THE REPLAY ORACLE — the obligation the engine cannot
      -- discharge for itself, and the reason this record exists
      sound : ∀ {a b} → F a → F b → nf a ≡ nf b → a ≈ b

    -- DERIVED, AND IT IS THE POINT. A sound normal form determines the
    -- boundary on its fragment. Nothing was assumed about `nf` beyond
    -- soundness; the boundary falls out of it.
    nf-determines-boundary : ∀ {a b}
      → F a
      → F b
      → nf a ≡ nf b
      → boundary a ≡ boundary b
    nf-determines-boundary fa fb p = ≈-boundary (sound fa fb p)

    -- The guarded fast path, assembled: take the cheap branch where the normal
    -- forms agree, and FALL THROUGH to the oracle otherwise. The guard is
    -- sound by `sound`; the fall-through is what keeps the procedure a decision
    -- procedure despite the normal form being incomplete.
    fast-path : ∀ {a b} → F a → F b → Dec (a ≈ b)
    fast-path {a} {b} fa fb with nf a ≟ᶠ nf b
    ... | yes p = yes (sound fa fb p)
    ... | no _ = a ≈? b
  open NormalFormComparison public

  ----------------------------------------------------------------------------
  -- Reason 02 — the general band, where the certificate is the currency.
  ----------------------------------------------------------------------------

  -- No canonical form; the per-instance witness is the only currency, and
  -- deciding means replaying.
  record CarriedWitness (F : Fragment) : Set (suc ℓ) where
    field
      valid : ∀ {c} → F c → Replays c
      decide : ∀ {a b} → F a → F b → Dec (a ≈ b)

  -- And it is inhabited for every fragment inside validity, unconditionally.
  -- So reason 02 is a floor rather than a claim: what a classification by
  -- reason records is whether a fragment has escaped it.
  carried : ∀ {F}
    → (∀ {c} → F c → Replays c)
    → CarriedWitness F
  carried v .CarriedWitness.valid = v
  carried v .CarriedWitness.decide {a} {b} _ _ = a ≈? b

  ----------------------------------------------------------------------------
  -- What separates the two reasons.
  ----------------------------------------------------------------------------

  -- A normal-form witness is INCOMPLETE at a pair when the pair is one
  -- certificate that the normal form separates. Named so the incompleteness of
  -- the first inhabitant is a stated property rather than an omission.
  Incomplete : ∀ {F} → NormalFormComparison F → Certificate → Certificate → Set ℓ
  Incomplete W a b = (a ≈ b) × ¬ (W .nf a ≡ W .nf b)

------------------------------------------------------------------------------
-- The first inhabitant.
------------------------------------------------------------------------------

-- Over `Examples.walk`, on the fragment of certificates that replay: the
-- boundary paired with the sorted multiset of the recorded left leg's step
-- keys.
--
-- The sort is the effective multiset quotient `Gandr.Rigid.ℕ-multiset.sorted`
-- makes legitimate — that instance is what says comparing sorted lists decides
-- permutation, and this normal form is that comparison with the boundary in
-- front of it.
module Walk where

  open Certificates walk
  open Reasons walk

  -- Steps as content addresses.
  key : Dir → ℕ
  key up = 0
  key down = 1

  keys : Certificate → List ℕ
  keys c = Sort.sort (map key (c .pathᵃ))

  Nf₀ : Set
  Nf₀ = ℕ × ℕ × List ℕ

  nf₀ : Certificate → Nf₀
  nf₀ c = c .peak , c .join , keys c

  _≟₀_ : DecidableEquality Nf₀
  _≟₀_ (p , q , xs) (p′ , q′ , ys) =
    map′
      (λ { (refl , refl , refl) → refl })
      (λ { refl → refl , refl , refl })
      (_≟ⁿ_ p p′ ×? (_≟ⁿ_ q q′ ×? List≡.≡-dec _≟ⁿ_ xs ys))

  -- THE WITNESS. Soundness routes through the boundary components and through
  -- nothing else — the multiset component is never consulted by this proof,
  -- which is the honest statement of what it does and does not buy.
  walk-nf : NormalFormComparison Replays
  walk-nf = record
    { Nf = Nf₀
    ; _≟ᶠ_ = _≟₀_
    ; nf = nf₀
    ; valid = λ v → v
    ; sound = λ va vb p → replay-equivalent (cong proj₁ p) (cong (λ n → proj₁ (proj₂ n)) p) va vb
    }

  ----------------------------------------------------------------------------
  -- Sound and not complete, with the separation exhibited.
  ----------------------------------------------------------------------------

  -- The normal form is invariant under permuting the recorded derivation, which
  -- is what makes it a NORMAL FORM rather than the recorded list. This is the
  -- `Rigid` multiset instance's completeness obligation, consumed.
  walk-nf-permutation-invariant : ∀ {a b}
    → a .peak ≡ b .peak
    → a .join ≡ b .join
    → map key (a .pathᵃ) ↭ map key (b .pathᵃ)
    → nf₀ a ≡ nf₀ b
  walk-nf-permutation-invariant refl refl p =
    cong (λ xs → _ , _ , xs) (ℕ-multiset.sort-resp p)

  -- And it separates two presentations of one certificate, so it is strictly
  -- finer than replay-equivalence: sound, never complete.
  --
  -- The separation is proved through LENGTH rather than by computing the two
  -- sorted lists, because the stdlib sort is sealed and does not reduce on
  -- closed terms. Sorting is a permutation, permutations preserve length, and
  -- the two recorded derivations have different lengths.
  keys-length : ∀ c
    → length (keys c) ≡ length (c .pathᵃ)
  keys-length c =
    trans (↭-length (Sort.sort-↭ (map key (c .pathᵃ)))) (length-map key (c .pathᵃ))

  walk-nf-separates : ¬ (nf₀ climb ≡ nf₀ detour)
  walk-nf-separates p =
    lengths-differ
      (trans
        (sym (keys-length climb))
        (trans (cong (λ n → length (proj₂ (proj₂ n))) p) (keys-length detour)))
    where
      lengths-differ : length (climb .pathᵃ) ≡ length (detour .pathᵃ) → ⊥
      lengths-differ ()

  -- Stated in the vocabulary the axis uses.
  walk-nf-incomplete : Incomplete walk-nf climb detour
  walk-nf-incomplete = presentations , walk-nf-separates

  -- The general band is available on the same fragment, unconditionally.
  walk-carried : CarriedWitness Replays
  walk-carried = carried (λ v → v)
