{-# OPTIONS --safe --without-K --hidden-argument-puns #-}

------------------------------------------------------------------------------
-- Gandr.Metatheory.Certificate.Base — certificates, replay, and the identity
-- relation on them.
--
-- The epic's binding statement-shape contract opens "identity is
-- replay-equivalence, never syntactic equality of certificates". This module is
-- where that sentence becomes a type, and it makes one structural choice that
-- the rest of the arc turns on.
--
-- ── CONTAINMENT BY CONSTRUCTION, NOT BY ARGUMENT ────────────────────────────
-- `_≈_` is a RECORD with four fields: the two boundary equalities and the two
-- replay witnesses. It is not a conjunction checked after the fact, and it is
-- not "the boundaries agree" with validity established elsewhere. A positive
-- answer cannot be produced without producing all four, so every consumer of a
-- positive answer holds equal peaks, equal joins, and two successful replays,
-- and holds them by projection rather than by re-derivation.
--
-- That is the shape the engine side arrived at independently and by repair: a
-- relation on certificates that compared projected derivation data WITHOUT the
-- boundary could answer yes across different boundaries, and the fix was to
-- make the positive answer force the boundary rather than check for it
-- afterwards. Stating it this way here is not a stylistic preference — it is the
-- reason `Properties`' soundness results are projections instead of proofs.
--
-- ── WHAT REPLAY-EQUIVALENCE DOES NOT READ ───────────────────────────────────
-- The recorded paths appear in `_≈_` only through `Replays`, which says each
-- path RUNS to the join — never that two certificates record the same steps,
-- the same number of steps, or steps in the same order. That omission is the
-- definition's whole content: it is what makes the relation proof-irrelevant
-- beyond replayability, and it is what `Properties` shows cannot be reconciled
-- with any observation of the recorded derivation.
--
-- ── THE ORACLE ──────────────────────────────────────────────────────────────
-- `_≈?_` decides `_≈_` outright, from decidable equality on terms alone. This
-- is worth stating early because it fixes what a tractability witness can and
-- cannot be for: replay-equivalence is ALREADY decidable, so a cheaper
-- comparison is never buying decidability. It is buying cost, and its only
-- obligation to the metatheory is soundness against this oracle.
--
-- ── ON THE ABSTRACTION ──────────────────────────────────────────────────────
-- `ReplaySystem` is deliberately thin: terms, steps, a partial one-step action,
-- and decidable term equality. It is not a model of the kernel's cell store, and
-- it does not know what a cut, a hole, or a variance is. Everything this arc
-- states is a statement about the SHAPE of certificate identity, and stating it
-- over the thin system is what shows the shape does not depend on the cell
-- alphabet. Where a result does need more — invertibility, for composition's
-- groupoid half — the extra structure is a named parameter rather than a
-- silently richer system.
------------------------------------------------------------------------------

module Gandr.Metatheory.Certificate.Base where

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
open import Data.Product.Base
  using (_×_)
  using (_,_)
open import Level
  using (Level)
  using (suc)
open import Relation.Binary.Definitions
  using (Decidable)
  using (DecidableEquality)
open import Relation.Binary.PropositionalEquality
  using (_≡_)
  using (refl)
  using (cong₂)
  using (sym)
  using (trans)
open import Relation.Nullary.Decidable.Core
  using (Dec)
  using (map′)
  using (_×?_)

private
  module Maybe≡ where
    open import Data.Maybe.Properties public
      using (≡-dec)

------------------------------------------------------------------------------
-- The substrate.
------------------------------------------------------------------------------

-- Terms, steps, a partial one-step action, and decidable equality on terms.
--
-- The action is PARTIAL — a step need not apply at a term — which is what makes
-- replay a real obligation rather than a bookkeeping identity. Take the terms as
-- the kernel's commands and the steps as recorded cell applications; nothing
-- below depends on that reading.
record ReplaySystem (ℓ : Level) : Set (suc ℓ) where
  field
    Term : Set ℓ
    Step : Set ℓ
    -- one recorded step, at a term, when it applies there
    apply : Step → Term → Maybe Term
    -- terms are compared as data; this is the content-addressing assumption,
    -- and it is the only decidability the oracle below needs
    _≟ᵀ_ : DecidableEquality Term

------------------------------------------------------------------------------
-- Certificates over a system.
------------------------------------------------------------------------------

module Certificates {ℓ} (S : ReplaySystem ℓ) where

  open ReplaySystem S public

  -- A certificate: a peak, two recorded derivations, and the join they are
  -- claimed to meet at. The two legs are the confluence shape — a certificate
  -- witnesses that two ways out of one peak arrive at one join.
  record Certificate : Set ℓ where
    constructor certificate
    field
      peak : Term
      pathᵃ : List Step
      pathᵇ : List Step
      join : Term
  open Certificate public

  -- The boundary: the pair replay-equivalence compares, and the only part of a
  -- certificate that it reads directly.
  Boundary : Set ℓ
  Boundary = Term × Term

  boundary : Certificate → Boundary
  boundary c = c .peak , c .join

  ----------------------------------------------------------------------------
  -- Replay.
  ----------------------------------------------------------------------------

  -- Run a recorded derivation from a term, stopping at the first step that does
  -- not apply. Written with `maybe′` rather than `with` so that it reduces on
  -- open terms, which every proof below relies on.
  run : List Step → Term → Maybe Term
  run [] t = just t
  run (s ∷ ss) t = maybe′ (run ss) nothing (apply s t)

  -- Running a concatenation is running the prefix and then the suffix. This is
  -- the one lemma the whole composition story rests on, and it is why grafting
  -- needs no side condition beyond the seam.
  run-++ : ∀ xs ys t
    → run (xs ++ ys) t ≡ maybe′ (run ys) nothing (run xs t)
  run-++ [] ys t = refl
  run-++ (s ∷ xs) ys t with apply s t
  ... | nothing = refl
  ... | just t′ = run-++ xs ys t′

  -- A certificate REPLAYS when both recorded legs run from the peak to the
  -- join. This is validity, and it is a property of the recorded derivation as
  -- well as of the boundary — which is exactly why it does not follow from
  -- boundary agreement and has to be carried.
  record Replays (c : Certificate) : Set ℓ where
    constructor replays
    field
      ᵃ-runs : run (c .pathᵃ) (c .peak) ≡ just (c .join)
      ᵇ-runs : run (c .pathᵇ) (c .peak) ≡ just (c .join)
  open Replays public

  -- Validity is decidable, by running.
  replays? : ∀ c → Dec (Replays c)
  replays? c =
    map′
      (λ { (p , q) → replays p q })
      (λ v → v .ᵃ-runs , v .ᵇ-runs)
      (Maybe≡.≡-dec _≟ᵀ_ (run (c .pathᵃ) (c .peak)) (just (c .join))
        ×? Maybe≡.≡-dec _≟ᵀ_ (run (c .pathᵇ) (c .peak)) (just (c .join)))

  ----------------------------------------------------------------------------
  -- The identity relation.
  ----------------------------------------------------------------------------

  infix 4 _≈_

  -- REPLAY-EQUIVALENCE: two certificates are the same transformation when they
  -- share a boundary and each replays.
  --
  -- The four fields are the definition, and their being fields is the design.
  -- A positive answer FORCES equal peaks, equal joins and two successful
  -- replays; it does not check for them, and there is no way to hold one of
  -- these and not the others. Anything downstream that needs the boundary
  -- projects it (`peak≡`, `join≡`); anything that needs validity projects that
  -- (`lhs`, `rhs`).
  record _≈_ (a b : Certificate) : Set ℓ where
    constructor replay-equivalent
    field
      peak≡ : a .peak ≡ b .peak
      join≡ : a .join ≡ b .join
      lhs : Replays a
      rhs : Replays b
  open _≈_ public

  -- The boundary is an invariant of the relation, by projection. Stated
  -- separately because it is the property every later soundness argument uses,
  -- and because reading it off is the payoff of the record shape above.
  ≈-boundary : ∀ {a b}
    → a ≈ b
    → boundary a ≡ boundary b
  ≈-boundary e = cong₂ _,_ (e .peak≡) (e .join≡)

  -- THE REPLAY ORACLE. Replay-equivalence is decidable outright: compare two
  -- terms, run two paths, run two more. No fragment condition, no normal form,
  -- no budget.
  --
  -- This is the fact a tractability witness is measured against, and it is why
  -- such a witness can never be justified by decidability. The oracle already
  -- decides; a cheaper procedure is a claim about cost, and its only debt to
  -- this tree is soundness.
  infix 4 _≈?_

  _≈?_ : Decidable _≈_
  a ≈? b =
    map′
      (λ { (p , (q , (v , w))) → replay-equivalent p q v w })
      (λ e → e .peak≡ , (e .join≡ , (e .lhs , e .rhs)))
      ((a .peak) ≟ᵀ (b .peak)
        ×? ((a .join) ≟ᵀ (b .join)
        ×? (replays? a ×? replays? b)))

  ----------------------------------------------------------------------------
  -- The relation's own laws.
  ----------------------------------------------------------------------------

  -- Symmetry and transitivity are free, and they use both replay witnesses:
  -- transitivity keeps the outer two and DISCARDS the middle certificate
  -- entirely, which is the first visible consequence of the relation not
  -- reading paths.
  ≈-sym : ∀ {a b}
    → a ≈ b
    → b ≈ a
  ≈-sym e = replay-equivalent (sym (e .peak≡)) (sym (e .join≡)) (e .rhs) (e .lhs)

  ≈-trans : ∀ {a b c}
    → a ≈ b
    → b ≈ c
    → a ≈ c
  ≈-trans e f =
    replay-equivalent
      (trans (e .peak≡) (f .peak≡))
      (trans (e .join≡) (f .join≡))
      (e .lhs)
      (f .rhs)

  -- Reflexivity is NOT free, and the hypothesis it needs is not decoration.
  -- A certificate that does not replay is not related to itself, so `_≈_` is a
  -- PARTIAL equivalence relation on `Certificate` and an equivalence relation
  -- only on the valid ones. `Properties` refutes the unrestricted form rather
  -- than leaving this to be read as a proof that was not attempted.
  ≈-refl : ∀ {c}
    → Replays c
    → c ≈ c
  ≈-refl v = replay-equivalent refl refl v v

  -- The converse: self-relation IS validity, so nothing is lost by taking the
  -- valid certificates to be exactly the reflexive ones.
  ≈-refl⁻ : ∀ {c}
    → c ≈ c
    → Replays c
  ≈-refl⁻ e = e .lhs
