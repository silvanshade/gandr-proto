{-# OPTIONS --safe --without-K --hidden-argument-puns #-}

------------------------------------------------------------------------------
-- Gandr.Metatheory.Certificate.Composition — grafting, its descent to the
-- replay quotient, and the groupoid fragment.
--
-- ── WHAT THE TWO LANES ACTUALLY DIFFER IN ───────────────────────────────────
-- The design record gives the coherence lane unconditional composition and the
-- directed lane an acyclicity gate, and reads the difference off the categorical
-- line: dinaturals over a groupoid always compose, dinaturals in general compose
-- only under loop-freeness. This module states the coherence half.
--
-- What it shows is that the unconditionality is a fact about GRAFTING, not
-- about invertibility: `graft-replays` needs the seam and the two validities and
-- nothing else, and the proof is `run-++`. Invertibility is not what makes
-- composition total — it is what supplies INVERSES, which is a different
-- theorem (`graft-inverseʳ`, `graft-inverseˡ`). Keeping the two apart matters
-- because the directed lane's gate is then visibly not a weaker version of this
-- argument: it is a condition on something this construction never consults.
--
-- ── DESCENT, WHICH IS THE LOAD-BEARING RESULT ───────────────────────────────
-- `graft-≈` is composition being WELL DEFINED on the replay quotient: replace
-- either argument by a replay-equivalent certificate and the composite is
-- replay-equivalent. The proof reads the boundary off both hypotheses and
-- rebuilds validity; it never compares the recorded derivations, and it could
-- not, since replay-equivalence does not hold them.
--
-- `seam-≈` is the small result that makes this work at all and is easy to miss:
-- **the seam condition is itself replay-invariant.** If `a₁` and `b₁` meet at a
-- seam and `a₂`, `b₂` are the same certificates, then `a₂` and `b₂` meet at a
-- seam too — so the hypothesis does not have to be re-supplied for the second
-- pair, and composition on the quotient is not conditioned on a choice of
-- representative.
--
-- This is exactly the property the directed lane's verdict does NOT have. Both
-- are conditions on a pair of certificates; the seam is a condition on their
-- boundaries and descends, while the acyclicity verdict reads the recorded cell
-- support and does not. The contrast is the sharpest available statement of why
-- one lane is gated and the other is not, and it is a statement about which data
-- the condition consults rather than about the lanes.
--
-- ── LAWS AT THE CONTRACT'S GRADE ────────────────────────────────────────────
-- Associativity and unitality are stated up to replay-equivalence, per the
-- epic's binding contract, and every one of them takes the validities as
-- hypotheses. That is not bookkeeping: an equation between certificates that
-- did not carry validity would be an equation in a relation that is not
-- reflexive, and `Properties` shows what that costs.
--
-- Underneath, the associativity is stronger than stated — the recorded paths
-- are equal by `++`-associativity, not merely replay-equivalent. It is stated at
-- the coarse grade anyway, because the coarse grade is the one every consumer
-- may rely on and the strict one holds only for this particular composite.
------------------------------------------------------------------------------

module Gandr.Metatheory.Certificate.Composition where

open import Gandr.Metatheory.Certificate.Base
  using (ReplaySystem)
  using (module Certificates)

open import Data.List.Base
  using (List)
  using ([])
  using (_∷_)
  using (_++_)
open import Data.Maybe.Base
  using (Maybe)
  using (just)
  using (nothing)
  using (maybe′)
open import Data.Maybe.Properties
  using (just-injective)
open import Level
  using (Level)
open import Relation.Binary.PropositionalEquality
  using (_≡_)
  using (refl)
  using (cong)
  using (sym)
  using (trans)

------------------------------------------------------------------------------
-- Grafting.
------------------------------------------------------------------------------

module Compose {ℓ} (S : ReplaySystem ℓ) where

  open Certificates S

  -- The sequential seam: the left certificate's join is the right one's peak.
  -- A condition on the two BOUNDARIES, which is what makes it descend.
  Seam : Certificate → Certificate → Set ℓ
  Seam a b = a .join ≡ b .peak

  -- The identity certificate at a term: the empty derivation, twice.
  idc : Term → Certificate
  idc t = certificate t [] [] t

  idc-replays : ∀ t
    → Replays (idc t)
  idc-replays t = replays refl refl

  -- The composite: the boundary of the outer pair, with each leg the
  -- concatenation of the corresponding legs.
  graft : Certificate → Certificate → Certificate
  graft a b =
    certificate
      (a .peak)
      (a .pathᵃ ++ b .pathᵃ)
      (a .pathᵇ ++ b .pathᵇ)
      (b .join)

  -- UNCONDITIONAL. Given the seam, the composite of two valid certificates is
  -- valid — no gate, no fragment condition, no budget. The whole content is
  -- `run-++`.
  graft-replays : ∀ {a b}
    → Seam a b
    → Replays a
    → Replays b
    → Replays (graft a b)
  graft-replays {a} {b} s va vb =
    replays
      (leg (a .pathᵃ) (b .pathᵃ) (va .ᵃ-runs) (vb .ᵃ-runs))
      (leg (a .pathᵇ) (b .pathᵇ) (va .ᵇ-runs) (vb .ᵇ-runs))
    where
      leg : ∀ xs ys
        → run xs (a .peak) ≡ just (a .join)
        → run ys (b .peak) ≡ just (b .join)
        → run (xs ++ ys) (a .peak) ≡ just (b .join)
      leg xs ys p q =
        trans
          (run-++ xs ys (a .peak))
          (trans
            (cong (maybe′ (run ys) nothing) p)
            (trans (cong (run ys) s) q))

  ----------------------------------------------------------------------------
  -- Descent to the replay quotient.
  ----------------------------------------------------------------------------

  -- The seam condition is replay-invariant: it is a condition on boundaries,
  -- and replay-equivalence preserves boundaries.
  seam-≈ : ∀ {a₁ a₂ b₁ b₂}
    → a₁ ≈ a₂
    → b₁ ≈ b₂
    → Seam a₁ b₁
    → Seam a₂ b₂
  seam-≈ ea eb s = trans (sym (ea .join≡)) (trans s (eb .peak≡))

  -- COMPOSITION IS WELL DEFINED ON THE REPLAY QUOTIENT. Replace either side by
  -- a replay-equivalent certificate — a different recorded derivation of the
  -- same transformation — and the composite is the same transformation.
  --
  -- One seam hypothesis suffices, by `seam-≈`.
  graft-≈ : ∀ {a₁ a₂ b₁ b₂}
    → Seam a₁ b₁
    → a₁ ≈ a₂
    → b₁ ≈ b₂
    → graft a₁ b₁ ≈ graft a₂ b₂
  graft-≈ s ea eb =
    replay-equivalent
      (ea .peak≡)
      (eb .join≡)
      (graft-replays s (ea .lhs) (eb .lhs))
      (graft-replays (seam-≈ ea eb s) (ea .rhs) (eb .rhs))

  ----------------------------------------------------------------------------
  -- The laws, at the contract's grade.
  ----------------------------------------------------------------------------

  graft-assoc : ∀ {a b c}
    → Seam a b
    → Seam b c
    → Replays a
    → Replays b
    → Replays c
    → graft (graft a b) c ≈ graft a (graft b c)
  graft-assoc sab sbc va vb vc =
    replay-equivalent
      refl
      refl
      (graft-replays sbc (graft-replays sab va vb) vc)
      (graft-replays sab va (graft-replays sbc vb vc))

  graft-unitˡ : ∀ {a}
    → Replays a
    → graft (idc (a .peak)) a ≈ a
  graft-unitˡ {a} va =
    replay-equivalent refl refl (graft-replays refl (idc-replays (a .peak)) va) va

  graft-unitʳ : ∀ {a}
    → Replays a
    → graft a (idc (a .join)) ≈ a
  graft-unitʳ {a} va =
    replay-equivalent refl refl (graft-replays refl va (idc-replays (a .join))) va

------------------------------------------------------------------------------
-- The invertible fragment.
------------------------------------------------------------------------------

-- Every step has an inverse step that undoes it wherever it applied.
--
-- This is the groupoid hypothesis, isolated. Nothing in `Compose` above needed
-- it, which is the point of stating it separately: composition is total without
-- it, and what it buys is inverses.
record Invertible {ℓ} (Sys : ReplaySystem ℓ) : Set ℓ where
  open ReplaySystem Sys
  field
    inv : Step → Step
    apply-inv : ∀ {s t u}
      → apply s t ≡ just u
      → apply (inv s) u ≡ just t

module Groupoid {ℓ} {S : ReplaySystem ℓ} (I : Invertible S) where

  open Certificates S
  open Compose S
  open Invertible I

  -- The reversed, inverted derivation. Written by direct recursion rather than
  -- as `reverse ∘ map inv` so that `run-invPath` can be an induction over the
  -- same recursion.
  invPath : List Step → List Step
  invPath [] = []
  invPath (s ∷ ss) = invPath ss ++ (inv s ∷ [])

  -- Running the inverted derivation from the far end returns to the near one.
  run-invPath : ∀ xs t t′
    → run xs t ≡ just t′
    → run (invPath xs) t′ ≡ just t
  run-invPath [] t t′ h = cong just (sym (just-injective h))
  run-invPath (s ∷ ss) t t′ h with apply s t in eq
  run-invPath (s ∷ ss) t t′ () | nothing
  run-invPath (s ∷ ss) t t′ h | just u =
    trans
      (run-++ (invPath ss) (inv s ∷ []) t′)
      (trans
        (cong (maybe′ (run (inv s ∷ [])) nothing) (run-invPath ss u t′ h))
        (cong (maybe′ (run []) nothing) (apply-inv eq)))

  -- The inverse certificate: the boundary turned around, each leg inverted.
  invert : Certificate → Certificate
  invert c =
    certificate
      (c .join)
      (invPath (c .pathᵃ))
      (invPath (c .pathᵇ))
      (c .peak)

  invert-replays : ∀ {c}
    → Replays c
    → Replays (invert c)
  invert-replays {c} v =
    replays
      (run-invPath (c .pathᵃ) (c .peak) (c .join) (v .ᵃ-runs))
      (run-invPath (c .pathᵇ) (c .peak) (c .join) (v .ᵇ-runs))

  -- THE GROUPOID FRAGMENT. Every valid certificate over an invertible system
  -- has a two-sided inverse for grafting, up to replay-equivalence.
  --
  -- Together with `graft-replays`, `graft-assoc`, `graft-unitˡ` and
  -- `graft-unitʳ` from `Compose`, this is the groupoid structure on the replay
  -- quotient of the valid certificates: composition total, associative and
  -- unital at the coarse grade, with inverses.
  graft-inverseʳ : ∀ {c}
    → Replays c
    → graft c (invert c) ≈ idc (c .peak)
  graft-inverseʳ {c} v =
    replay-equivalent
      refl
      refl
      (graft-replays refl v (invert-replays v))
      (idc-replays (c .peak))

  graft-inverseˡ : ∀ {c}
    → Replays c
    → graft (invert c) c ≈ idc (c .join)
  graft-inverseˡ {c} v =
    replay-equivalent
      refl
      refl
      (graft-replays refl (invert-replays v) v)
      (idc-replays (c .join))
