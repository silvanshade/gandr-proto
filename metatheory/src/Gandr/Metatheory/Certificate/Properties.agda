{-# OPTIONS --safe --without-K --hidden-argument-puns #-}

------------------------------------------------------------------------------
-- Gandr.Metatheory.Certificate.Properties — what replay-equivalence is, and
-- what no relation reading the recorded derivation can be.
--
-- ── THE HEADLINE, AND IT IS NOT THE ONE THE ARC SET OUT TO PROVE ────────────
-- The engine side established, for one relation, that cell-support equality and
-- replay-equivalence are INCOMPARABLE rather than ordered. This module shows
-- that is not a fact about cell support. **For every observation `f` of
-- certificates, the kernel of `f` and replay-equivalence are incomparable**, as
-- soon as two conditions hold — and both are conditions this tree can exhibit
-- rather than assume:
--
--   * `f` separates some replay-equivalent pair, which gives
--     `≈ ⊄ Kernel f` (`≈-not-⊆-kernel`); and
--   * some certificate fails to replay, which gives
--     `Kernel f ⊄ ≈` (`kernel-not-⊆-≈`).
--
-- The two directions fail for DIFFERENT reasons, and keeping them apart is the
-- content. The first is path-blindness: replay-equivalence does not read the
-- recorded derivation, so anything that does will split a class. The second is
-- validity: a kernel relation is reflexive on the nose, and replay-equivalence
-- is not reflexive at all, so no observation can imply it.
--
-- Structural equality, cell-support equality, flow equality and shift
-- equivalence are all kernels of observations of the recorded derivation. So
-- the containment picture is not "a chain with replay-equivalence at the coarse
-- end". There is no chain. `Examples` supplies both witnesses at a concrete
-- system, which is what stops every statement here from being a claim about an
-- empty type.
--
-- ── WHERE CONTAINMENT DOES HOLD, AND EXACTLY WHAT BUYS IT ───────────────────
-- Restrict to the certificates that replay and the picture changes completely:
-- an observation's kernel is contained in replay-equivalence **iff** the
-- observation determines the boundary (`kernel-⊆-≈-on-valid`). Nothing else
-- about the observation matters — not how fine it is, not what it records, not
-- whether it is a normal form.
--
-- That is the metatheory's form of the engine-side lesson that a relation on
-- certificates carries the boundary or is not one. It is also the reason
-- `Tractability`'s soundness field routes through the boundary and through
-- nothing else: the finer content of a normal form buys cost, and can buy
-- availability, and cannot buy soundness.
--
-- ── ON THE TRIVIALITY OF THE PROOFS ─────────────────────────────────────────
-- `≈-not-⊆-kernel` is one application. That is the honest shape of the result:
-- once the witness pair exists, the theorem is immediate, and the work is
-- entirely in exhibiting the witness. Stating it as a lemma anyway is what makes
-- the witness's role visible at every instantiation, instead of each consumer
-- re-deriving a one-liner and losing track of what its hypothesis was.
------------------------------------------------------------------------------

module Gandr.Metatheory.Certificate.Properties where

open import Gandr.Metatheory.Certificate.Base
  using (ReplaySystem)
  using (module Certificates)

open import Data.Product.Base
  using (Σ)
  using (_,_)
  using (proj₁)
  using (proj₂)
open import Level
  using (Level)
open import Relation.Binary.Bundles
  using (PartialSetoid)
  using (Setoid)
open import Relation.Binary.Core
  using (Rel)
open import Relation.Binary.PropositionalEquality
  using (_≡_)
  using (refl)
  using (cong)
open import Relation.Binary.Structures
  using (IsEquivalence)
  using (IsPartialEquivalence)
open import Relation.Nullary.Negation.Core
  using (¬_)

module Facts {ℓ} (S : ReplaySystem ℓ) where

  open Certificates S

  ----------------------------------------------------------------------------
  -- The bundles.
  ----------------------------------------------------------------------------

  -- Replay-equivalence is a PARTIAL equivalence relation on all certificates.
  -- The bundle is the honest one: `PartialSetoid`, not `Setoid`, because
  -- reflexivity is false and `≈-not-refl` below says so with a witness.
  certificates : PartialSetoid ℓ ℓ
  certificates .PartialSetoid.Carrier = Certificate
  certificates .PartialSetoid._≈_ = _≈_
  certificates .PartialSetoid.isPartialEquivalence .IsPartialEquivalence.sym = ≈-sym
  certificates .PartialSetoid.isPartialEquivalence .IsPartialEquivalence.trans = ≈-trans

  -- A certificate together with its validity: the subtype on which
  -- replay-equivalence IS an equivalence relation.
  Valid : Set ℓ
  Valid = Σ Certificate Replays

  infix 4 _≈ᵛ_

  _≈ᵛ_ : Rel Valid ℓ
  a ≈ᵛ b = proj₁ a ≈ proj₁ b

  -- And there it is a genuine setoid. This is the object the epic's contract
  -- means by "value-setoid grade": laws about certificates are witnessed here,
  -- never as equalities of the `Certificate` record.
  valid : Setoid ℓ ℓ
  valid .Setoid.Carrier = Valid
  valid .Setoid._≈_ = _≈ᵛ_
  valid .Setoid.isEquivalence .IsEquivalence.refl {x} = ≈-refl (proj₂ x)
  valid .Setoid.isEquivalence .IsEquivalence.sym = ≈-sym
  valid .Setoid.isEquivalence .IsEquivalence.trans = ≈-trans

  -- Reflexivity fails, and it fails exactly where validity does. Stated as a
  -- refutation rather than left as a proof that was not attempted: a reader who
  -- sees only `≈-refl`'s hypothesis cannot tell whether it is needed.
  ≈-not-refl : ∀ {c}
    → ¬ Replays c
    → ¬ (c ≈ c)
  ≈-not-refl invalid e = invalid (≈-refl⁻ e)

  ----------------------------------------------------------------------------
  -- Observations of the recorded derivation, and their kernels.
  ----------------------------------------------------------------------------

  -- An OBSERVATION is any function out of certificates: the recorded legs, the
  -- set of steps they fire, a flow projection, a normal form. Its KERNEL is the
  -- relation "these two certificates are observed alike", and every relation
  -- this tree compares against replay-equivalence is of that form.
  Kernel : ∀ {ℓ′} {X : Set ℓ′}
    → (Certificate → X)
    → Rel Certificate ℓ′
  Kernel f a b = f a ≡ f b

  -- A kernel is always reflexive, whatever the observation is. This innocuous
  -- fact is the whole of the second incomparability direction.
  kernel-refl : ∀ {ℓ′} {X : Set ℓ′} (f : Certificate → X) {c}
    → Kernel f c c
  kernel-refl f = refl

  -- FIRST DIRECTION — path-blindness. An observation that separates a
  -- replay-equivalent pair is not implied by replay-equivalence.
  --
  -- The hypothesis is where the content is: it says the observation reads
  -- something the identity forgets. Every observation of the RECORDED
  -- DERIVATION does, because replay-equivalence reads the derivation only
  -- through "it runs".
  ≈-not-⊆-kernel : ∀ {ℓ′} {X : Set ℓ′} (f : Certificate → X) {a b}
    → a ≈ b
    → ¬ (f a ≡ f b)
    → ¬ (∀ {x y} → x ≈ y → Kernel f x y)
  ≈-not-⊆-kernel f a≈b separated contained = separated (contained a≈b)

  -- SECOND DIRECTION — validity. No observation's kernel implies
  -- replay-equivalence, as soon as one certificate fails to replay: the kernel
  -- relates that certificate to itself and replay-equivalence does not.
  --
  -- This direction does not depend on the observation at all, which is the
  -- point. It is not that cell-support equality happened to be too coarse; it
  -- is that NO agreement-of-observations relation can carry validity, because
  -- validity is not an agreement between two certificates.
  kernel-not-⊆-≈ : ∀ {ℓ′} {X : Set ℓ′} (f : Certificate → X) {c}
    → ¬ Replays c
    → ¬ (∀ {x y} → Kernel f x y → x ≈ y)
    -- and the counterexample is `c` against itself
  kernel-not-⊆-≈ f invalid contained = ≈-not-refl invalid (contained (kernel-refl f))

  ----------------------------------------------------------------------------
  -- Where containment does hold: the valid subtype, and only through the
  -- boundary.
  ----------------------------------------------------------------------------

  -- On certificates that replay, an observation's kernel is contained in
  -- replay-equivalence exactly when the observation DETERMINES THE BOUNDARY.
  --
  -- Sufficiency, here. The converse is `boundary-determines⁻` below, and the
  -- pair is what makes "carries the boundary or is not one" a characterization
  -- rather than a slogan.
  kernel-⊆-≈-on-valid : ∀ {ℓ′} {X : Set ℓ′} (f : Certificate → X)
    → (∀ {x y} → f x ≡ f y → boundary x ≡ boundary y)
    → ∀ {a b}
    → Replays a
    → Replays b
    → Kernel f a b
    → a ≈ b
  kernel-⊆-≈-on-valid f determines va vb k =
    replay-equivalent (cong proj₁ (determines k)) (cong proj₂ (determines k)) va vb

  -- The converse: if an observation's kernel lands inside replay-equivalence on
  -- the valid certificates, then it determines the boundary there. So carrying
  -- the boundary is not one way to be sound — it is the only way.
  boundary-determines⁻ : ∀ {ℓ′} {X : Set ℓ′} (f : Certificate → X)
    → (∀ {x y} → Replays x → Replays y → Kernel f x y → x ≈ y)
    → ∀ {a b}
    → Replays a
    → Replays b
    → Kernel f a b
    → boundary a ≡ boundary b
  boundary-determines⁻ f sound va vb k = ≈-boundary (sound va vb k)

  -- The identity observation is the extreme case, and it is worth having by
  -- name: replay-equivalence restricted to the valid certificates is the kernel
  -- of the boundary. Everything finer than the boundary is finer than the
  -- identity, and buys something other than soundness.
  boundary-kernel-is-≈ : ∀ {a b}
    → Replays a
    → Replays b
    → boundary a ≡ boundary b
    → a ≈ b
  boundary-kernel-is-≈ = kernel-⊆-≈-on-valid boundary (λ k → k)

  boundary-kernel-is-≈⁻ : ∀ {a b}
    → a ≈ b
    → boundary a ≡ boundary b
  boundary-kernel-is-≈⁻ = ≈-boundary
