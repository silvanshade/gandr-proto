{-# OPTIONS --safe --without-K --hidden-argument-puns #-}

------------------------------------------------------------------------------
-- Gandr.Metatheory.Certificate.Examples — the witnesses the general statements
-- are vacuous without.
--
-- `Properties` proves two incomparability directions, each conditioned on a
-- witness: a replay-equivalent pair the observation separates, and a
-- certificate that does not replay. Neither is a theorem about certificates in
-- general; each is a claim that a certain certificate EXISTS. This module
-- supplies both, at a system small enough to compute.
--
-- ── THE SYSTEM ──────────────────────────────────────────────────────────────
-- Terms are naturals and steps are `up` and `down`. It is chosen for three
-- properties the abstract statements need exercised, and not for realism:
--
--   * the step action is genuinely PARTIAL — `down` does not apply at zero — so
--     a recorded derivation can fail to replay for a structural reason rather
--     than only by naming the wrong join;
--   * every step has an inverse, so `Composition`'s groupoid half has somewhere
--     to be instantiated; and
--   * one boundary carries derivations of different lengths and different step
--     multisets, which is what a witness for the first direction has to be.
--
-- ── THE WITNESS PAIR, AND WHAT IT MINIATURIZES ──────────────────────────────
-- `climb` records two `up` steps from 0 to 2. `detour` records four steps —
-- three `up` and one `down` — from 0 to 2. They are one certificate: same peak,
-- same join, both replay. They record different derivations, of different
-- lengths, over different multisets of steps.
--
-- That is deliberately the shape the engine side found when it took a fused
-- derivation's two legs as two presentations of one boundary — the two-step
-- form and the single-step form, replay-equivalent with different recorded
-- support. The pair here is the same phenomenon with the cell alphabet removed,
-- which is the point: nothing about it needed cells, cuts, or variance.
------------------------------------------------------------------------------

module Gandr.Metatheory.Certificate.Examples where

open import Gandr.Metatheory.Certificate.Base
  using (ReplaySystem)
  using (module Certificates)
open import Gandr.Metatheory.Certificate.Composition
  using (Invertible)
  using (module Compose)
  using (module Groupoid)
open import Gandr.Metatheory.Certificate.Properties
  using (module Facts)

open import Data.List.Base
  using (List)
  using ([])
  using (_∷_)
open import Data.Maybe.Base
  using (Maybe)
  using (just)
  using (nothing)
open import Data.Product.Base
  using (_×_)
  using (_,_)
open import Data.Nat.Base
  using (ℕ)
  using (zero)
  using (suc)
open import Data.Nat.Properties
  using (_≟_)
open import Level
  using (0ℓ)
open import Relation.Binary.PropositionalEquality
  using (_≡_)
  using (refl)
open import Relation.Nullary.Negation.Core
  using (¬_)

------------------------------------------------------------------------------
-- The system.
------------------------------------------------------------------------------

-- One step up, or one step down. `down` is partial at zero, which is what makes
-- replay a real obligation in this system rather than a bookkeeping identity.
data Dir : Set where
  up : Dir
  down : Dir

step : Dir → ℕ → Maybe ℕ
step up n = just (suc n)
step down zero = nothing
step down (suc n) = just n

walk : ReplaySystem 0ℓ
walk = record
  { Term = ℕ
  ; Step = Dir
  ; apply = step
  ; _≟ᵀ_ = _≟_
  }

open Certificates walk
open Facts walk

------------------------------------------------------------------------------
-- Two presentations of one certificate.
------------------------------------------------------------------------------

-- The direct derivation from 0 to 2.
climb : Certificate
climb = certificate 0 (up ∷ up ∷ []) (up ∷ up ∷ []) 2

-- Another derivation of the same boundary, which goes past the join and comes
-- back. Same peak, same join, longer path, different step multiset.
detour : Certificate
detour = certificate 0 (up ∷ up ∷ up ∷ down ∷ []) (up ∷ up ∷ up ∷ down ∷ []) 2

climb-replays : Replays climb
climb-replays = replays refl refl

detour-replays : Replays detour
detour-replays = replays refl refl

-- They are ONE certificate. This is the whole hypothesis of the first
-- incomparability direction, and it holds by `refl` twice plus the two replays
-- — which is what "the relation does not read the paths" amounts to in
-- practice.
presentations : climb ≈ detour
presentations = replay-equivalent refl refl climb-replays detour-replays

-- And they record different derivations.
paths-differ : ¬ (climb .pathᵃ ≡ detour .pathᵃ)
paths-differ ()

------------------------------------------------------------------------------
-- A certificate that does not replay.
------------------------------------------------------------------------------

-- `down` does not apply at zero, so this records a derivation that gets stuck
-- immediately. Its boundary is perfectly well formed; what fails is validity,
-- and that is the distinction the second incomparability direction turns on.
stuck : Certificate
stuck = certificate 0 (down ∷ []) (down ∷ []) 0

stuck-invalid : ¬ Replays stuck
stuck-invalid (replays () _)

------------------------------------------------------------------------------
-- The two directions, instantiated.
------------------------------------------------------------------------------

-- FIRST DIRECTION. Replay-equivalence is not contained in the kernel of the
-- recorded left leg: `climb` and `detour` are one certificate recording two
-- derivations.
≈-not-⊆-recorded-path : ¬ (∀ {x y} → x ≈ y → Kernel pathᵃ x y)
≈-not-⊆-recorded-path = ≈-not-⊆-kernel pathᵃ presentations paths-differ

-- SECOND DIRECTION. The kernel of the recorded left leg is not contained in
-- replay-equivalence either: `stuck` records the same derivation as itself and
-- is not replay-equivalent to itself.
--
-- Note what this proof does NOT use: anything about the observation. The same
-- argument refutes containment for the cell support, for a flow projection, for
-- a normal form, and for structural equality of the whole record.
recorded-path-not-⊆-≈ : ¬ (∀ {x y} → Kernel pathᵃ x y → x ≈ y)
recorded-path-not-⊆-≈ = kernel-not-⊆-≈ pathᵃ stuck-invalid

-- So the recorded-path relation and replay-equivalence are INCOMPARABLE, and
-- the two failures have two different causes. This is the statement the engine
-- side reached for cell-support equality; the observation is a parameter here,
-- so the same pair of refutations covers every relation of that shape.
recorded-path-incomparable :
  ¬ (∀ {x y} → x ≈ y → Kernel pathᵃ x y)
  × ¬ (∀ {x y} → Kernel pathᵃ x y → x ≈ y)
recorded-path-incomparable = ≈-not-⊆-recorded-path , recorded-path-not-⊆-≈

------------------------------------------------------------------------------
-- The invertible instance, and a groupoid pin.
------------------------------------------------------------------------------

-- `Composition`'s groupoid half is a parameterized module, and a parameterized
-- module type-checks whether or not its parameters can ever be supplied. This
-- discharges them: `up` and `down` invert one another, and the partiality of
-- `down` at zero is where the hypothesis has real work to do.
walk-inv : Dir → Dir
walk-inv up = down
walk-inv down = up

walk-invertible : Invertible walk
walk-invertible = record { inv = walk-inv ; apply-inv = undo }
  where
    undo : ∀ {s t u}
      → step s t ≡ just u
      → step (walk-inv s) u ≡ just t
    undo {s = up} refl = refl
    undo {s = down} {t = zero} ()
    undo {s = down} {t = suc n} refl = refl

open Compose walk
open Groupoid walk-invertible

-- A computational pin for the groupoid statement: the direct derivation from 0
-- to 2, grafted onto its own inverse, is the identity certificate at 0.
climb-inverse : graft climb (invert climb) ≈ idc (climb .peak)
climb-inverse = graft-inverseʳ climb-replays
