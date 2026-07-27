{-# OPTIONS --safe --guardedness --without-K --hidden-argument-puns #-}

------------------------------------------------------------------------------
-- Gandr.Rigid — the section discipline: an effective quotient on a setoid.
--
-- gandr stores a canonical linearization of an object whose semantics is
-- symmetric, and decides semantic equality by comparing stored representations.
-- `Rigid` is the structure that makes that legitimate, and the point of naming
-- it is that BOTH directions are obligations:
--
--   * `canon-resp` — equivalent elements canonicalize to the SAME
--     representative, so the representation is COMPLETE for the algebra. This
--     is what makes ordering a section of the quotient rather than a
--     strengthening of it. Without it the stored form would decide something
--     FINER than the semantics, and two semantically equal objects would be
--     reported distinct.
--   * `canon-sound` — equal representatives come from equivalent elements, so
--     the representation is SOUND. Without it the stored form would decide
--     something coarser, and distinct objects would be conflated.
--
-- ── WHAT THIS IS, CATEGORICALLY ─────────────────────────────────────────────
-- `canon` is an idempotent on a setoid, split by the elements it fixes. Its
-- splitting is `Nf` below, and `Nf` carries PROPOSITIONAL equality — so the
-- splitting is not merely a retract, it is an EFFECTIVE quotient: the setoid
-- relation is recovered exactly as equality of normal forms.
--
-- That is the honest content of "Rigid", and it is not the old reading. The old
-- reading — that the objects have trivial automorphisms — is false for the
-- graphical category, whose automorphism groups contain the symmetric groups on
-- the input and output legs. Rigidity is a property of the REPRESENTATION, not
-- of the objects, and this record is that property stated as structure.
--
-- ── WHY THIS IS THE KEYSTONE, AND EXACTLY WHAT IS PROVED ────────────────────
-- This tree has SETOID, not SET, and the standing consequence is that quotients
-- NEED NOT BE EFFECTIVE. `Rigid` is precisely the extra structure that makes
-- one effective, and `nf-equiv`/`nf-equiv⁻` below state that in the ambient
-- itself rather than in prose: every `Rigid` setoid is isomorphic IN SETOID to
-- a setoid whose equivalence is propositional equality, whose equality is
-- decidable, and which is a genuine set. That is a type here, not a claim.
--
-- The converse is NOT proved, and it is worth saying why rather than leaving
-- the omission to be read as an oversight: it is false as stated. An
-- isomorphism in SETOID to a decidable-equality set supplies decidable `≈` on
-- the source, but `Rigid` asks for decidable `≡` on the source's CARRIER, which
-- no such isomorphism can hand back — the isomorphism only ever speaks up to
-- the setoid relation. So `Rigid` is STRICTLY stronger than "isomorphic in
-- SETOID to a decidable-equality set", and the extra strength is exactly
-- decidable equality of the representation, which is what a content-addressed
-- store actually has and what the Hedberg step below needs.
--
-- Read carefully, then: this module proves the INCLUSION of the `Rigid` objects
-- into the part of SETOID equivalent to decidable-equality SET, object by
-- object. It does not prove that inclusion is essentially surjective, and by
-- the paragraph above it is not. Nor is the assignment shown functorial; that
-- would need `Gandr.Category.Functor` and a category of `Rigid` objects, which
-- nothing yet consumes.
--
-- Where a SET is genuinely produced — `nf-uip` — it is produced from a DECISION
-- PROCEDURE and not from an axiom: decidable equality implies uniqueness of
-- identity proofs by Hedberg's argument, which is K-free. No `--with-K` island
-- is opened, and none is needed.
--
-- ── FOUR FIELDS, NOT FIVE, AND THE DIVERGENCE IS PROVED LOSSLESS ────────────
-- The design record states five obligations: `canon`, `canon-idem`,
-- `canon-resp`, `canon-sound`, and decidable equality. Two of them are
-- derivable once the section is stated. Adding `canon-≈` — the normal form is
-- EQUIVALENT to what it normalizes, which is what a normalization function
-- means — gives `canon-idem` as `canon-resp canon-≈` and `canon-sound` by
-- transitivity through it. `Split` at the bottom of this module is the
-- five-field presentation, and `fromSplit`/`toSplit` prove the two carry the
-- same information, so the smaller record is a repackaging rather than a
-- weakening.
--
-- ── WHAT IS NOT HERE ────────────────────────────────────────────────────────
-- The instance for gandr's cell shape. It needs decidable equality of graphical
-- maps, which comes from HRY Cor 6.62 — a graphical map is determined by its
-- action on the finite edge set, so hom-sets are finite and equality decidable —
-- and that is `Gandr.Shape.Decidable`, which does not exist yet. The instance
-- below is the OTHER one the design calls for: the multiset case, where the
-- canonical form is the content-address sort, and which is what the symmetric
-- monoidal structure of the algebra rests on. Its splitting is characterized
-- rather than merely inhabited — `nf-sorted`/`sorted-nf` say the normal forms
-- there are EXACTLY the sorted lists — and its payoff, `_↭?_`, is decidable
-- multiset equality of primitive factorizations.
--
-- One honest limitation of that instance, since a green gate does not show it:
-- stdlib seals its sorting algorithm behind `abstract`, so `sort` does not
-- reduce on closed terms here and the instance cannot be exercised by
-- evaluation inside this tree. Everything below is proved, nothing is computed.
------------------------------------------------------------------------------

module Gandr.Rigid where

open import Gandr.Category
  using (idn₀)
  using (seq₀)
open import Gandr.Category.Instances
  using (∣_∣)
  using (_⊢_≈_)
  using (Fnc)
  using (Htpy)
  using (ap)
  using (ap*)
  using (at)
  using (SETOID)

open import Level
  using (Level)
open import Relation.Binary.Bundles
  using (DecTotalOrder)
  using (Setoid)
open import Relation.Binary.Definitions
  using (DecidableEquality)
  using (Decidable)

-- The ambient equality and the two stdlib results this module turns into
-- structure: Hedberg's theorem, and decidability transport along a logical
-- equivalence.
private
  module ≡ where
    open import Relation.Binary.PropositionalEquality public
      using (_≡_)
      using (refl)
      using (cong)
      using (subst)
      using (setoid)
    open import Axiom.UniquenessOfIdentityProofs public
      using (UIP)
      using (module Decidable⇒UIP)
  module Dec where
    open import Relation.Nullary.Decidable public
      using (map′)
open ≡
  using (_≡_)

------------------------------------------------------------------------------
-- The record.
------------------------------------------------------------------------------

-- A canonicalization on a setoid that decides its equivalence.
record Rigid {ℓ} (X : Setoid ℓ ℓ) : Set ℓ where
  field
    -- the canonical representative of an element
    canon : ∣ X ∣ → ∣ X ∣
    -- which is EQUIVALENT to that element: this is what makes `canon` a
    -- normalization rather than an arbitrary endomap, and it is the splitting's
    -- section law
    canon-≈ : ∀ a
      → X ⊢ canon a ≈ a
    -- and COMPLETE for the equivalence: equivalent elements have the SAME
    -- representative, on the nose. This is the field that makes ordering a
    -- section of the quotient rather than a strengthening of it
    canon-resp : ∀ {a b}
      → X ⊢ a ≈ b
      → canon a ≡ canon b
    -- equality of representatives is decidable
    _≟_ : DecidableEquality ∣ X ∣

  -- `canon` is idempotent. Derived, not assumed: a representative is
  -- equivalent to its element, so re-canonicalizing lands in the same place.
  canon-idem : ∀ a
    → canon (canon a) ≡ canon a
  canon-idem a = canon-resp (canon-≈ a)

  -- The converse of `canon-resp`: equal representatives come from equivalent
  -- elements. Derived too — travel out to the representative, across, and back.
  canon-sound : ∀ {a b}
    → canon a ≡ canon b
    → X ⊢ a ≈ b
  canon-sound {a} {b} p =
    Setoid.trans X
      (Setoid.sym X (canon-≈ a))
      (Setoid.trans X (Setoid.reflexive X p) (canon-≈ b))

  -- The payoff: the setoid's own equivalence is decidable, by deciding the
  -- representatives. Soundness and completeness are exactly the two directions
  -- the transport needs, which is why neither may be dropped.
  decide : Decidable (X ⊢_≈_)
  decide a b = Dec.map′ canon-sound canon-resp (canon a ≟ canon b)
open Rigid public

------------------------------------------------------------------------------
-- The splitting: normal forms, and their propositional equality.
------------------------------------------------------------------------------

-- A normal form: an element that is its own canonical representative. These are
-- the fixed points of the idempotent, i.e. the object it splits through.
record Nf {ℓ} {X : Setoid ℓ ℓ} (R : Rigid X) : Set ℓ where
  constructor normal
  field
    -- the underlying element
    elt : ∣ X ∣
    -- pinned to be canonical
    fix : R .canon elt ≡ elt
open Nf public

module _ {ℓ} {X : Setoid ℓ ℓ} (R : Rigid X) where

  -- Normal forms are determined by their elements. The fixed-point witness is
  -- unique by Hedberg's theorem — decidable equality gives uniqueness of
  -- identity proofs, and the stdlib derivation of it is K-free — so nothing
  -- here needs an axiom the tree has refused.
  nf-ext : ∀ {m n : Nf R}
    → m .elt ≡ n .elt
    → m ≡ n
  nf-ext {normal e p} {normal _ q} ≡.refl =
    ≡.cong (normal e) (≡.Decidable⇒UIP.≡-irrelevant (R ._≟_) p q)

  -- So equality of normal forms is decidable.
  _≟ⁿ_ : DecidableEquality (Nf R)
  m ≟ⁿ n = Dec.map′ nf-ext (≡.cong elt) (R ._≟_ (m .elt) (n .elt))

  -- And normal forms are a genuine SET, not merely a setoid: their identity
  -- proofs are unique. This is where a setoid becomes a set, and the set is
  -- bought with a decision procedure rather than with an axiom.
  nf-uip : ≡.UIP (Nf R)
  nf-uip = ≡.Decidable⇒UIP.≡-irrelevant _≟ⁿ_

  -- Normalization, into the splitting.
  nf : ∣ X ∣ → Nf R
  nf a = normal (R .canon a) (canon-idem R a)

  -- And the inclusion back.
  emb : Nf R → ∣ X ∣
  emb n = n .elt

  -- The idempotent factors through the splitting DEFINITIONALLY: `canon` is
  -- normalize-then-include, with nothing inserted. That is what "split" means,
  -- and here it is not even a propositional equality.
  canon-split : ∀ a
    → emb (nf a) ≡ R .canon a
  canon-split a = ≡.refl

  -- Including a normal form and renormalizing returns it, ON THE NOSE — this is
  -- the retraction half, and it is where the propositional equality of normal
  -- forms is bought.
  nf-emb : ∀ n
    → nf (emb n) ≡ n
  nf-emb n = nf-ext (n .fix)

  -- Normalizing an element and including it returns something EQUIVALENT to it
  -- — the section half, and only up to the setoid, which is the honest
  -- direction of the asymmetry.
  emb-nf : ∀ a
    → X ⊢ emb (nf a) ≈ a
  emb-nf a = R .canon-≈ a

------------------------------------------------------------------------------
-- The statement in the ambient: a Rigid setoid IS a decidable-equality set.
------------------------------------------------------------------------------

module _ {ℓ} {X : Setoid ℓ ℓ} (R : Rigid X) where

  -- The normal forms as an OBJECT of SETOID, whose equivalence is propositional
  -- equality. Nothing here is a quotient presented as a setoid; the relation is
  -- the ambient identity type, and `_≟ⁿ_` decides it.
  NF : Setoid ℓ ℓ
  NF = ≡.setoid (Nf R)

  -- Normalization as a morphism of SETOID. Its congruence is exactly
  -- `canon-resp`: completeness for the equivalence is what makes normalization
  -- a map OUT of the quotient.
  nfᶠ : Fnc X NF
  nfᶠ .ap = nf R
  nfᶠ .ap* p = nf-ext R (R .canon-resp p)

  -- The inclusion as a morphism of SETOID. Its congruence is free, since the
  -- source relation is already the identity type.
  embᶠ : Fnc NF X
  embᶠ .ap = emb R
  embᶠ .ap* ≡.refl = Setoid.refl X

  -- The two composites are the identities, up to SETOID's own 2-cells — which
  -- is to say `X` and `NF` are isomorphic in SETOID. `Htpy` IS the 2-cell layer
  -- of `Setoids`, so these are cells of the ambient and not a bespoke notion.
  --
  -- Together with `_≟ⁿ_` and `nf-uip`, this is the object half of the keystone:
  -- a `Rigid` setoid is isomorphic in SETOID to a setoid carrying
  -- propositional, DECIDABLE equality — a genuine set. The converse is false
  -- and the header says why, so this is an inclusion into decidable-equality
  -- SET rather than an equivalence with it.
  nf-equiv : Htpy (SETOID .seq₀ nfᶠ embᶠ) (SETOID .idn₀ X)
  nf-equiv .at = emb-nf R

  nf-equiv⁻ : Htpy (SETOID .seq₀ embᶠ nfᶠ) (SETOID .idn₀ NF)
  nf-equiv⁻ .at = nf-emb R

------------------------------------------------------------------------------
-- The design record's five-field presentation, carried so the divergence from
-- it is auditable rather than asserted.
------------------------------------------------------------------------------

-- The five-field presentation, as a constructor. Supplying idempotence and
-- soundness in place of the section law loses nothing: the section law IS
-- soundness applied to idempotence, which is the only step the two shapes do
-- not share.
--
-- With `canon-idem` and `canon-sound` derived inside the record above, this is
-- the other direction, so the two presentations carry the same information and
-- the smaller record is a repackaging rather than a weakening. It is stated as
-- a constructor rather than a second record because a second record would put a
-- clashing copy of every field name in this scope for no content.
rigid : ∀ {ℓ} {X : Setoid ℓ ℓ}
  -- the canonical representative
  → (canon : ∣ X ∣ → ∣ X ∣)
  -- which is idempotent
  → (canon-idem : ∀ a
    → canon (canon a) ≡ canon a)
  -- complete for the equivalence
  → (canon-resp : ∀ {a b}
    → X ⊢ a ≈ b
    → canon a ≡ canon b)
  -- and sound for it
  → (canon-sound : ∀ {a b}
    → canon a ≡ canon b
    → X ⊢ a ≈ b)
  -- with decidable equality on representatives
  → DecidableEquality ∣ X ∣
  → Rigid X
rigid c ci cr cs d .canon = c
rigid c ci cr cs d .canon-≈ a = cs (ci a)
rigid c ci cr cs d .canon-resp = cr
rigid c ci cr cs d ._≟_ = d

------------------------------------------------------------------------------
-- Instances.
------------------------------------------------------------------------------

-- Any type with decidable equality is Rigid over its identity setoid, with the
-- identity map as canonicalization. So SET embeds in Rigid — which is the
-- degenerate case, recorded because it says what the structure costs when there
-- is nothing to quotient by.
discrete : ∀ {ℓ} {A : Set ℓ}
  → DecidableEquality A
  → Rigid (≡.setoid A)
discrete d .canon a = a
discrete d .canon-≈ a = ≡.refl
discrete d .canon-resp p = p
discrete d ._≟_ = d

-- The multiset case, which is the one the algebra rests on.
--
-- A tracelet's primitive factorization is a MULTISET of primitives; the stored
-- form is a list sorted by content address. The equivalence is therefore
-- permutation, the canonical form is the sort, and the two obligations are that
-- sorting is a permutation of its input and that permutations sort alike. The
-- second is the load-bearing one: it is `canon-resp`, and it is what makes the
-- sorted list a SECTION of the multiset quotient rather than a finer datum.
--
-- MANDATORY MARK. `≈⇒≡` is an assumption carried in the signature: the order's
-- own equivalence is propositional equality. It holds for a content-address
-- order, which is what this instance is for — addresses are compared as data —
-- but it is a genuine hypothesis and it is why this is a parameterized module
-- rather than an unconditional instance. The other direction is free from the
-- order and is not assumed.
module Multiset
  {ℓ ℓ₁ ℓ₂}
  (O : DecTotalOrder ℓ ℓ₁ ℓ₂)
  (≈⇒≡ : ∀ {x y}
    → DecTotalOrder._≈_ O x y
    → x ≡ y)
  where

  open DecTotalOrder O
    using (totalOrder)
    renaming (Carrier to K)

  open import Data.List.Properties
    using (≡-dec)
  open import Data.List.Relation.Binary.Permutation.Propositional
    using (_↭_)
    using (↭-setoid)
    using (↭-sym)
    using (↭-trans)
    using (↭⇒↭ₛ′)
  open import Data.List.Relation.Binary.Pointwise
    using (Pointwise-≡⇒≡)
    renaming (map to pointwise-map)
  open import Data.List.Relation.Unary.Sorted.TotalOrder totalOrder
    using (Sorted)
  open import Data.List.Relation.Unary.Sorted.TotalOrder.Properties
    using (↗↭↗⇒≋)
  open import Data.List.Sort O
    using (sort)
    using (sort-↭)
    using (sort-↗)

  -- Decidable equality on the keys, from the order's decision procedure read
  -- through the assumption.
  _≟ᵏ_ : DecidableEquality K
  _≟ᵏ_ x y = Dec.map′ ≈⇒≡ (DecTotalOrder.Eq.reflexive O) (DecTotalOrder._≟_ O x y)

  -- The uniqueness the whole instance turns on: two SORTED lists that permute
  -- into one another are equal. stdlib proves it pointwise in the order's own
  -- equivalence; the assumption carries that down to propositional equality.
  ↗-unique : ∀ {xs ys}
    → Sorted xs
    → Sorted ys
    → xs ↭ ys
    → xs ≡ ys
  ↗-unique xs↗ ys↗ p =
    Pointwise-≡⇒≡
      (pointwise-map ≈⇒≡
        (↗↭↗⇒≋ totalOrder xs↗ ys↗
          (↭⇒↭ₛ′ (DecTotalOrder.Eq.isEquivalence O) p)))

  -- Permutations sort alike — the completeness obligation. Route both inputs
  -- through their sorts, which are sorted and permute into one another.
  sort-resp : ∀ {xs ys}
    → xs ↭ ys
    → sort xs ≡ sort ys
  sort-resp {xs} {ys} p =
    ↗-unique
      (sort-↗ xs)
      (sort-↗ ys)
      (↭-trans (sort-↭ xs) (↭-trans p (↭-sym (sort-↭ ys))))

  -- Lists under permutation, canonicalized by the content-address sort. This is
  -- the multiset quotient made effective, and it is what the stored form of a
  -- primitive factorization is.
  sorted : Rigid (↭-setoid {A = K})
  sorted .canon = sort
  sorted .canon-≈ = sort-↭
  sorted .canon-resp = sort-resp
  sorted ._≟_ = ≡-dec _≟ᵏ_

  -- The payoff, in the form the algebra consumes it: multiset equality of
  -- primitive factorizations is decidable.
  _↭?_ : Decidable _↭_
  _↭?_ = decide sorted

  -- And the splitting is not abstract nonsense here: the normal forms of this
  -- instance are EXACTLY the sorted lists. Both directions, so `Nf sorted` is
  -- characterized rather than merely inhabited.
  nf-sorted : ∀ {xs}
    → Sorted xs
    → sort xs ≡ xs
  nf-sorted {xs} xs↗ = ↗-unique (sort-↗ xs) xs↗ (sort-↭ xs)

  sorted-nf : ∀ {xs}
    → sort xs ≡ xs
    → Sorted xs
  sorted-nf {xs} p = ≡.subst Sorted p (sort-↗ xs)

-- The assumption set above is SATISFIABLE, and this is here to prove it. A
-- parameterized module type-checks whether or not its parameters can ever be
-- supplied, so an instance whose hypotheses were jointly unsatisfiable would
-- pass the gate while proving nothing. The natural numbers under their
-- decidable total order discharge it — that order's equivalence IS
-- propositional equality, so the assumption is the identity — and every
-- definition in `Multiset` therefore exists at a concrete key type.
module ℕ-multiset where
  open import Data.Nat.Properties
    using (≤-decTotalOrder)
  open Multiset ≤-decTotalOrder (λ p → p) public
