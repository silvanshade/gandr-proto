{-# OPTIONS --safe --guardedness --without-K --hidden-argument-puns #-}

------------------------------------------------------------------------------
-- Gandr.Shape.Structure — the circuit carrier's categorical instances.
--
-- The layers below this one prove content; this one says what that content IS,
-- by filling the records. It is the `X/Structure` role module of the layout
-- convention, and it exists because naming a characterization is not doing it:
-- an instance that cannot be filled is a hole nobody had listed.
--
-- ── THE WIRING CATEGORY ─────────────────────────────────────────────────────
-- Objects are the profiles `List Ob` — the free monoid on the colours, which is
-- what makes the ambient's parallel tensor planar and is a fact about the
-- OBJECTS rather than about any operation on them. Morphisms are the wirings
-- `Match Ob Γ Δ`: colour-preserving matchings, one term per bijection, with the
-- cap included so the morphisms are the downward ones rather than only the
-- bijective ones. The identity is `idn-match` and composition is `match-comp`.
--
-- ── WHAT THE CARRIER IS, AND WHY IT IS NAMED ONCE ───────────────────────────
-- `𝔾.homs° (List Ob) (Match Ob)`: 0-cells the profiles, 1-cells the wirings,
-- 2-cells their propositional equalities, `𝟘` above. Every `Set`-level category
-- in this tree goes through that former, which is why it sits beside `≡°` in
-- `Gandr.Graph` rather than being inlined here — this is its first consumer and
-- it will not be the last.
--
-- The carrier's REGION is `Only⋆` (`Gandr.Graph.At`): content at dimensions
-- 0–2, none above, so a consumer that wants the certification supplies `at⋆`
-- and `Everywhere Category` is refuted outright
-- (`Gandr.Category.ℂ.homs°-not-everywhere`). `WIRING` is the inhabitant that
-- makes that refutation a genuine SEPARATION rather than two absurdities: the
-- bare structure is here, and the certification over it is not.
--
-- ── WHAT THE INSTANCE COST, WHICH IS THE POINT OF WRITING IT ────────────────
-- Six of the nine fields were already proved and sat unassembled; `mon-λ` and
-- `mon-ρ` are the unit laws of the listing algebra. The ninth, `mon-α`, is
-- ASSOCIATIVITY OF `match-comp`, and it is what this instance cost: it had been
-- named as residue once and then appeared on no list until the record refused
-- to typecheck without it.
--
-- **It is now proved and the module is unconditional.** While it was open it was
-- a PARAMETER here, per the build-the-residual rule — the assumption in the
-- signature, not in a comment and not as a postulate — with a degenerate
-- instance at the empty colour set beside it to rule out the vacuity a
-- parameterized module risks. Both are retired: `Gandr.Shape.Graft` reduced
-- associativity to two commutation laws on the listing algebra and then proved
-- them, so `match-comp-assoc` is imported here like any other lemma.
--
-- ── ONE CORRESPONDENCE, RECORDED AS A CANDIDATE ─────────────────────────────
-- The arc's analysis reads this category as the DOWNWARD BRAUER CATEGORY of the
-- circuit-algebra literature (Raynor 2025, Def. 3.14 — the epic's references
-- carry the entry). That is a claim about a published object and it is NOT
-- proved here: no functor is built and no comparison is stated, so the name is
-- not adopted and the module says `WIRING`. Recorded as a candidate so the
-- correspondence is not silently promoted by repetition, and so that whoever
-- proves it knows what is owed.
--
-- ── ON THE FLAG ─────────────────────────────────────────────────────────────
-- `--guardedness` is here for the ordinary infective reason: `Category` is
-- structure on the coinductive ∞-graph carrier. Nothing about this module's
-- shape is arranged around it.
------------------------------------------------------------------------------

module Gandr.Shape.Structure where

open import Gandr.Graph
  using (At)
  using (Only⋆)
  using (at⋆)
  using (Everywhere)
  using (module 𝔾)
open import Gandr.Category
  using (Category)
  using (module ℂ)
  using (idn₀)
  using (seq₀)
  using (idn₁)
  using (seq₁)
  using (inv₁)
  using (seq↕)
  using (mon-λ)
  using (mon-ρ)
  using (mon-α)
open import Gandr.Shape.Graph
  using (Match)
  using ([])
  using (idn-match)
open import Gandr.Shape.Graft
  using (match-comp)
  using (match-comp-idnˡ)
  using (match-comp-idnʳ)
  using (match-comp-assoc)

open import Data.Empty.Polymorphic
  using (⊥)
open import Data.List.Base
  using (List)
  using ([])
open import Relation.Binary.PropositionalEquality
  using (_≡_)
  using (refl)
  using (trans)
  using (sym)
  using (cong₂)

------------------------------------------------------------------------------
-- The wiring category, over the listing algebra's own laws.
------------------------------------------------------------------------------

module _ {ℓ} (Ob : Set ℓ) where

  -- THE WIRING CATEGORY. Profiles, wirings, and the listing algebra's own unit
  -- and composition — nothing is constructed here that was not already proved,
  -- which is what assembling a characterization is supposed to look like.
  --
  -- STRICT: the laws are propositional equalities of matchings rather than
  -- nontrivial 2-cells. That is not this instance's choice but the `≡°`-hom
  -- presentation's — a `Set`-level structure has its equality at dimension 2 —
  -- so every `Set`-level instance in this tree is strict in the same sense and
  -- carries the same mark. No UIP is claimed or used: nothing above those
  -- equalities is asserted.
  WIRING : Category (𝔾.homs° (List Ob) (Match Ob))
  -- the identity wiring: each source onto the sink beside it
  WIRING .idn₀ Γ = idn-match Γ
  -- composition: each source takes its partner's partner
  WIRING .seq₀ = match-comp
  -- the 2-cells are equalities of wirings, so their equivalence is the ambient
  -- one at each hom
  WIRING .idn₁ m = refl
  WIRING .seq₁ = trans
  WIRING .inv₁ = sym
  -- horizontal composition is congruence of the composition in both arguments
  WIRING .seq↕ = cong₂ match-comp
  -- the two unit laws, h-level free, proved on the listing algebra
  WIRING .mon-λ m = match-comp-idnˡ m
  WIRING .mon-ρ m = match-comp-idnʳ m
  -- and associativity, proved there too. The field takes its two outer wirings
  -- before the middle one.
  WIRING .mon-α m o n = match-comp-assoc m n o

  -- THE REGION, STATED IN A SIGNATURE RATHER THAN LEFT TO BE DISCOVERED: this
  -- carrier's content stops at the 2-cells, so the certification it satisfies
  -- is the one over `Only⋆`, and an address above the carrier is refused by
  -- constructor disjointness inside `at⋆` rather than by convention. The
  -- implicits are bound because `𝒮₀ : 𝒮 Ξ` alone leaves both flex.
  WIRING⋆ : At Category (𝔾.homs° (List Ob) (Match Ob)) Only⋆
  WIRING⋆ = at⋆ {𝒮 = Category} {Ξ = 𝔾.homs° (List Ob) (Match Ob)} WIRING

  -- AND THE OTHER END IS REFUTED, WHICH IS WHAT MAKES THE REGION A CLAIM. The
  -- bare structure above inhabits this carrier and the dimension-wise
  -- certification over the same carrier does not, so the two hypotheses are
  -- genuinely apart here — which is exactly what `≡°` fails to witness, both
  -- being absurd there. One wiring is enough, and the empty profile has one.
  WIRING-not-everywhere
    : Everywhere Category (𝔾.homs° (List Ob) (Match Ob))
    → ⊥
  WIRING-not-everywhere = ℂ.homs°-not-everywhere (idn-match [])
