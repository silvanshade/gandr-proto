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
-- ── THE FIRST CANDIDATE REASON IS NOT A REASON, AND THAT IS THE FINDING ─────
-- `SoundPrefilter` was drafted as the convergent-fragment REASON — a canonical
-- form whose comparison decides — against `CarriedWitness` as the general band
-- where a per-instance witness is the only currency. The asymmetry was supposed
-- to be that the general band is inhabited unconditionally while the
-- convergent-fragment reason is a claim about a fragment.
--
-- **It is not a claim.** Its fields ask only for a fragment inside validity, a
-- decidable equality, and one-way soundness. Take the normal form to BE the
-- boundary and the equality to be the system's own, and soundness is the
-- boundary-kernel result: `boundary-prefilter` builds one for EVERY fragment
-- inside validity, exactly as unconditionally as `carried` does. Both records
-- are floors. Neither is a reason.
--
-- **And the trivial one is complete, which is the sharper half.** On a fragment
-- inside validity the boundary prefilter answers yes on exactly the
-- replay-equivalent pairs (`boundary-prefilter-complete`), so it is the oracle
-- wearing this record's shape. Meanwhile `nf-determines-boundary` says every
-- sound prefilter's yes-set sits INSIDE the boundary prefilter's. Put together:
-- **no sound prefilter can answer yes more often than the trivial one, and a
-- finer normal form answers yes strictly less often.** A prefilter buys cost and
-- can only lose coverage.
--
-- So what the record actually captures is a **sound positive prefilter with an
-- oracle fall-through**, and it is renamed to that. What it does NOT capture is
-- anything that would distinguish a tractability reason from any other sound
-- observation: no cost bound, no coverage claim, no completeness statement over
-- a named fragment. Adding one of those three is what would make a reason, and
-- the tree cannot supply the first — cost is invisible here.
--
-- **This is a finding about the design space rather than a shortfall.** The
-- classification the design record asks for exists to say WHY a fragment is
-- cheap; discovering that the first candidate reason carries no such content is
-- precisely what a classification by reason is for. `gandr-5lf.9.12` carries
-- what a reason would have to contain.
--
-- ── THE MULTISET INSTANCE, AND WHAT IT NOW DEMONSTRATES ─────────────────────
-- `walk-nf` compares the multiset of steps a derivation records, on top of the
-- boundary. It is a strictly finer prefilter than the boundary one, and
-- `walk-nf-incomplete` exhibits the separation: two presentations of one
-- certificate whose recorded step multisets differ. Under the reading above
-- that is no longer merely "sound but not complete" — it is the concrete
-- instance of coverage being LOST by refining the normal form, against a
-- trivial prefilter that already decides.
--
-- Its multiset component carries NO soundness weight; the soundness proof
-- routes entirely through the boundary components. What it carries is
-- order-insensitivity (`walk-nf-permutation-invariant`), which is what makes it
-- a normal form rather than the recorded list, and which is where its cost
-- advantage would come from if this tree could see cost.
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
  -- A sound positive prefilter over a fragment.
  ----------------------------------------------------------------------------

  -- On `F`, a comparison that is decidable and SOUND for replay-equivalence —
  -- so a positive answer may be taken, and a negative answer decides nothing.
  --
  -- **This is not a claim about `F`.** It was drafted as the
  -- convergent-fragment tractability reason and is not one: `boundary-prefilter`
  -- inhabits it for every fragment inside validity. The record is kept because
  -- the shape is the right one for a guarded fast path — the guard plus the
  -- soundness certificate the design record asks for — and renamed to what it
  -- actually is.
  --
  -- `valid` is a field rather than a consequence, and it is the field that
  -- makes `sound` provable at all. A normal form is a function of a certificate
  -- and can never establish that the certificate replays, so a fragment outside
  -- validity has no sound comparison of this shape — the universal direction of
  -- `Properties` met from the other side.
  record SoundPrefilter (F : Fragment) : Set (suc ℓ) where
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
  open SoundPrefilter public

  ----------------------------------------------------------------------------
  -- The general band, where the certificate is the currency.
  ----------------------------------------------------------------------------

  -- No canonical form; the per-instance witness is the only currency, and
  -- deciding means replaying.
  record CarriedWitness (F : Fragment) : Set (suc ℓ) where
    field
      valid : ∀ {c} → F c → Replays c
      decide : ∀ {a b} → F a → F b → Dec (a ≈ b)

  -- Inhabited for every fragment inside validity, unconditionally.
  carried : ∀ {F}
    → (∀ {c} → F c → Replays c)
    → CarriedWitness F
  carried v .CarriedWitness.valid = v
  carried v .CarriedWitness.decide {a} {b} _ _ = a ≈? b

  ----------------------------------------------------------------------------
  -- The prefilter record is a floor too, and the trivial one already decides.
  ----------------------------------------------------------------------------

  -- **THE NARROWING, AS A CONSTRUCTION.** Take the normal form to be the
  -- boundary and the equality to be the system's own. Soundness is then the
  -- boundary-kernel result, and it needs nothing about `F` beyond validity — so
  -- `SoundPrefilter` is inhabited exactly as unconditionally as `CarriedWitness`
  -- and its existence is not a claim about a fragment.
  boundary-prefilter : ∀ {F}
    → (∀ {c} → F c → Replays c)
    → SoundPrefilter F
  boundary-prefilter v = record
    { Nf = Term × Term
    ; _≟ᶠ_ = _≟ᵇ_
    ; nf = boundary
    ; valid = v
    ; sound = λ fa fb p → replay-equivalent (cong proj₁ p) (cong proj₂ p) (v fa) (v fb)
    }
    where
      _≟ᵇ_ : DecidableEquality (Term × Term)
      _≟ᵇ_ (p , q) (p′ , q′) =
        map′
          (λ { (refl , refl) → refl })
          (λ { refl → refl , refl })
          (_≟ᵀ_ p p′ ×? _≟ᵀ_ q q′)

  -- And it is COMPLETE on any fragment inside validity: it answers yes on
  -- exactly the replay-equivalent pairs. So the trivial prefilter is the oracle
  -- wearing this record's shape, and the record cannot tell them apart.
  boundary-prefilter-complete : ∀ {a b}
    → a ≈ b
    → boundary a ≡ boundary b
  boundary-prefilter-complete = ≈-boundary

  -- **COVERAGE IS BOUNDED BY THE TRIVIAL PREFILTER.** Every sound prefilter's
  -- yes-set sits inside the boundary prefilter's, by `nf-determines-boundary`;
  -- the boundary prefilter's yes-set is already all of replay-equivalence on
  -- its fragment, by the line above. So no sound prefilter answers yes more
  -- often than the trivial one, and a finer normal form answers yes strictly
  -- less often. A prefilter can buy cost and can only lose coverage.
  prefilter-coverage-bounded : ∀ {F} (W : SoundPrefilter F) {a b}
    → F a
    → F b
    → W .nf a ≡ W .nf b
    → boundary a ≡ boundary b
  prefilter-coverage-bounded W = nf-determines-boundary W

  -- A prefilter is INCOMPLETE at a pair when the pair is one certificate that
  -- it separates. Named so that the coverage lost by refining a normal form is
  -- a stated property rather than an omission.
  Incomplete : ∀ {F} → SoundPrefilter F → Certificate → Certificate → Set ℓ
  Incomplete W a b = (a ≈ b) × ¬ (W .nf a ≡ W .nf b)

------------------------------------------------------------------------------
-- The multiset prefilter.
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
  walk-nf : SoundPrefilter Replays
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

  -- And so is the trivial prefilter, which is the narrowing made concrete at
  -- this system: it inhabits the same record `walk-nf` does, it DECIDES on this
  -- fragment, and `walk-nf-incomplete` is a pair it answers and `walk-nf` does
  -- not. Refining the normal form bought nothing this tree can see and lost a
  -- pair it can.
  walk-boundary : SoundPrefilter Replays
  walk-boundary = boundary-prefilter (λ v → v)
