{-# OPTIONS --safe --without-K --hidden-argument-puns #-}

------------------------------------------------------------------------------
-- Gandr.Metatheory.Certificate.Properties — what replay-equivalence is, and
-- what no relation reading the recorded derivation can be.
--
-- ── ONE DIRECTION IS UNIVERSAL AND THE OTHER IS A HYPOTHESIS ────────────────
-- The engine side established, for one relation, that cell-support equality and
-- replay-equivalence are INCOMPARABLE rather than ordered. Something general is
-- true here and it is NOT that incomparability. The two directions are not
-- symmetric, and the whole content is in which one carries a hypothesis.
--
--   * **No observation's kernel is contained in replay-equivalence**
--     (`kernel-not-⊆-≈`), as soon as one certificate fails to replay. **This
--     direction does not mention the observation.** A kernel is reflexive on
--     the nose and replay-equivalence is not reflexive at all, so what no
--     agreement between two certificates can supply is validity.
--   * **Replay-equivalence is contained in an observation's kernel exactly when
--     the observation is `Invariant`** — unchanged by replacing a certificate
--     with a replay-equivalent one. `≈-not-⊆-kernel` refutes that FROM A
--     SEPARATING PAIR, so it is a hypothesis about `f` and never a theorem
--     about every `f`.
--
-- **`boundary-invariant` is the standing counterexample to reading the second
-- direction as universal**: the boundary is an observation, and
-- replay-equivalence sits inside its kernel outright.
--
-- ── EXACTLY WHAT IS PROVED, AND THE EXHAUSTIVE FORM IS NOT ──────────────────
-- Three statements, and the shape of each is the point:
--
--   1. **Universal** — `kernel-not-⊆-≈`: no observation's kernel is contained
--      in replay-equivalence, given one certificate that fails to replay.
--   2. **Conditional on invariance** — an `Invariant` observation has
--      replay-equivalence inside its kernel, and STRICTLY, by (1).
--      `boundary-invariant` is one.
--   3. **Conditional on a separating pair** — `≈-not-⊆-kernel`: an explicit
--      replay-equivalent pair the observation separates gives `¬ Invariant f`,
--      hence incomparability with (1). `Examples` supplies one.
--
-- **What is NOT proved is that (2) and (3) are exhaustive.** Nothing here
-- decides `Invariant f` for an arbitrary `f`, and nothing extracts a separating
-- pair from a proof that `f` is not invariant. The first needs excluded middle
-- at that proposition; the second needs `¬∀ → ∃¬`. A `--safe` module with no
-- postulates carries neither, so **"every observation either … or …" is a
-- classical gloss on this file and not a theorem of it.**
--
-- `Classified` below is what closes that gap constructively, and it closes it
-- by ASKING rather than proving: a classified observation is one where the two
-- cases have been separated by hand. `classified-trichotomy` is then a genuine
-- theorem, and both observations this tree cares about are classified. The cost
-- is exactly that the classification is a per-observation datum — `Invariant f`
-- quantifies over all pairs of certificates, so there is no decision procedure
-- to appeal to in general, and each observation pays for its own case.
--
-- Two earlier revisions of this header overstated this, in the same direction
-- both times: first claiming incomparability for every observation, which
-- `boundary-invariant` refutes; then claiming the exhaustive trichotomy, which
-- is classically true and is not what the types below say. Recorded rather than
-- quietly rewritten, because a file whose header outruns its own lemmas is the
-- failure this tree keeps its claims in types to prevent — and it took two
-- rounds of that to notice the second one.
--
-- ── WHAT IS INSTANTIATED, AND WHAT ONLY FOLLOWS ─────────────────────────────
-- `Examples` instantiates both directions for ONE observation: the recorded
-- left leg. Structural equality, cell-support equality, flow equality and shift
-- equivalence are named in the design corpus and are neither defined nor
-- instantiated in this tree, so nothing here is a mechanized claim about them.
--
-- What does follow for them, without a witness, is the universal direction —
-- because it does not mention the observation. The corpus's clause that shift
-- equivalence IMPLIES replay-equivalence is exactly `Kernel f ⊆ ≈` for `f` the
-- normal form, which the corpus itself presents as an agreement of normal forms
-- (an iff), and `kernel-not-⊆-≈` refutes it from one invalid certificate alone.
-- **The containment that would make a chain is the one that fails universally.**
-- Incomparability proper, which needs the second direction too, is established
-- here for the recorded leg and for nothing else.
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
  using (_×_)
  using (_,_)
  using (proj₁)
  using (proj₂)
open import Data.Sum.Base
  using (_⊎_)
  using (inj₁)
  using (inj₂)
open import Level
  using (Level)
  using (_⊔_)
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

  -- An observation is INVARIANT when replay-equivalent certificates are observed
  -- alike — equivalently, when it factors through the quotient. Naming it is
  -- what keeps the direction below from being read as universal: it is the
  -- hypothesis, and observations both satisfying and failing it exist.
  Invariant : ∀ {ℓ′} {X : Set ℓ′}
    → (Certificate → X)
    → Set (ℓ ⊔ ℓ′)
  Invariant f = ∀ {x y} → x ≈ y → Kernel f x y

  -- And SOUND when observing alike is enough to be the same transformation.
  -- This is the direction that fails for every observation.
  Sound : ∀ {ℓ′} {X : Set ℓ′}
    → (Certificate → X)
    → Set (ℓ ⊔ ℓ′)
  Sound f = ∀ {x y} → Kernel f x y → x ≈ y

  -- A kernel is always reflexive, whatever the observation is. This innocuous
  -- fact is the whole of the universal direction below.
  kernel-refl : ∀ {ℓ′} {X : Set ℓ′} (f : Certificate → X) {c}
    → Kernel f c c
  kernel-refl f = refl

  -- THE STANDING COUNTEREXAMPLE to reading `Invariant` as always-false. The
  -- boundary is an observation, and replay-equivalence sits inside its kernel
  -- outright — so "replay-equivalence is contained in no observation's kernel"
  -- is refuted here, by projection, and incomparability is not universal.
  boundary-invariant : Invariant boundary
  boundary-invariant = ≈-boundary

  -- THE CONDITIONAL DIRECTION — path-blindness, and it needs its witness.
  --
  -- The hypothesis is the whole content: a replay-equivalent pair the
  -- observation separates. Without one there is nothing to say, and
  -- `boundary-invariant` is an observation for which no such pair exists. So
  -- this is a fact about `f` and never a theorem about all of them.
  ≈-not-⊆-kernel : ∀ {ℓ′} {X : Set ℓ′} (f : Certificate → X) {a b}
    → a ≈ b
    → ¬ (f a ≡ f b)
    → ¬ Invariant f
  ≈-not-⊆-kernel f a≈b separated contained = separated (contained a≈b)

  -- THE UNIVERSAL DIRECTION — validity. No observation's kernel is contained in
  -- replay-equivalence, as soon as one certificate fails to replay: the kernel
  -- relates that certificate to itself and replay-equivalence does not.
  --
  -- **This direction does not depend on the observation at all**, which is the
  -- point and is what makes it the one that travels. It is not that
  -- cell-support equality happened to be too coarse; it is that NO
  -- agreement-of-observations relation can carry validity, because validity is
  -- not an agreement between two certificates. It is also the direction that
  -- refutes a containment chain, since a chain needs exactly this containment.
  kernel-not-⊆-≈ : ∀ {ℓ′} {X : Set ℓ′} (f : Certificate → X) {c}
    → ¬ Replays c
    → ¬ Sound f
    -- and the counterexample is `c` against itself
  kernel-not-⊆-≈ f invalid contained = ≈-not-refl invalid (contained (kernel-refl f))

  ----------------------------------------------------------------------------
  -- The exhaustive form, and the datum it has to be paid for with.
  ----------------------------------------------------------------------------

  -- A CLASSIFIED observation is one whose case has been settled by hand: either
  -- it is invariant, or a replay-equivalent pair it separates is exhibited.
  --
  -- This is a datum, not a theorem, and that is the honest content. Deciding it
  -- in general would need `Invariant f` decidable, and `Invariant f` quantifies
  -- over all pairs of certificates — so there is no procedure to appeal to, and
  -- each observation pays for its own case. What the type buys is that the
  -- payment is visible: a consumer of `classified-trichotomy` can see it was
  -- made rather than assumed.
  Classified : ∀ {ℓ′} {X : Set ℓ′}
    → (Certificate → X)
    → Set (ℓ ⊔ ℓ′)
  Classified f =
    Invariant f
      ⊎ Σ Certificate (λ a → Σ Certificate (λ b → (a ≈ b) × ¬ (f a ≡ f b)))

  -- THE TRICHOTOMY, for classified observations and for no others: the kernel
  -- either strictly contains replay-equivalence or is incomparable with it, and
  -- the right-hand conjunct of both cases is the universal direction.
  --
  -- Read the hypothesis as the statement's scope rather than as bookkeeping.
  -- Dropping it gives the sentence this file twice claimed and never proved.
  classified-trichotomy : ∀ {ℓ′} {X : Set ℓ′} (f : Certificate → X) {c}
    → ¬ Replays c
    → Classified f
    → (Invariant f × ¬ Sound f) ⊎ (¬ Invariant f × ¬ Sound f)
  classified-trichotomy f invalid (inj₁ inv) =
    inj₁ (inv , kernel-not-⊆-≈ f invalid)
  classified-trichotomy f invalid (inj₂ (a , b , a≈b , separated)) =
    inj₂ (≈-not-⊆-kernel f a≈b separated , kernel-not-⊆-≈ f invalid)

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
