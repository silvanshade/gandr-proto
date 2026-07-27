{-# OPTIONS --safe --guardedness --without-K --hidden-argument-puns #-}

------------------------------------------------------------------------------
-- Gandr.Profunctor.Yoneda — cells out of the hom profunctor are exactly wedges
-- of the target.
--
-- Concretely: `Pronat (hom-pro 𝒞) P ≃ Wedge P`. Restriction along the
-- identities sends a cell to its diagonal at `idn₀`; the inverse extends a
-- wedge along the right action, and the wedge condition is exactly what makes
-- that extension natural on the left.
--
-- ── WHAT IS AND IS NOT CLAIMED ──────────────────────────────────────────────
-- This is stated in the VALUE SETOIDS and nowhere else. The two round trips
-- below are pointwise value cells — `yoneda-from-to` at each object,
-- `yoneda-to-from` at each 1-cell — NOT equalities of the records `Pronat` or
-- `Wedge`, which this tree has no way to compare and deliberately never does.
--
-- So the correspondence is an equivalence of setoids, not an isomorphism of
-- sets, and calling it "Yoneda" should not smuggle in more than that. No
-- univalence, no transport, no comparison of records. In a setting that had
-- SET one would go on to conclude the two types are equal; here that step does
-- not exist, and the pointwise statement is the whole result.
--
-- That is not a weakness of the proof — it is what the setting supports, and
-- writing it this way is what keeps the limitation visible instead of assumed.
------------------------------------------------------------------------------

module Gandr.Profunctor.Yoneda where

open import Gandr.Graph
  using (∞Graph)
  using (ϵ°)
  using (δ°)
open import Gandr.Setoid
  using (bundle)
  using (invˢ)
open import Gandr.Category
  using (Category)
  using (idn₀)
  using (seq₀)
  using (inv₁)
  using (mon-λ)
  using (mon-ρ)
open import Gandr.Profunctor
  using (Profunctor)
  using (Pronat)
  using (Wedge)
  using (hom-pro)
  using (val)
  using (std)
  using (actˡ)
  using (actʳ)
  using (actʳ*)
  using (actʳ↕)
  using (act-idnʳ)
  using (act-seqʳ)
  using (act-xchg)
  using (cmp)
  using (cmp*)
  using (natˡ)
  using (natʳ)
  using (dinat)

-- The chains below run in the value setoids, so stdlib's reasoning applies
-- directly through `Gandr.Setoid.bundle`.
open import Relation.Binary.Reasoning.MultiSetoid

module _ {ℓ} {Ξ : ∞Graph ℓ} {𝒞 : Category Ξ} (P : Profunctor 𝒞 𝒞) where

  -- Restrict a cell out of the hom profunctor along the identities: the
  -- diagonal family at `idn₀`. The wedge condition is the cell's two
  -- naturalities meeting at the unit laws.
  yoneda-to : Pronat (hom-pro 𝒞) P → Wedge P
  yoneda-to ν .cmp a = ν .cmp (𝒞 .idn₀ a)
  yoneda-to ν .dinat {a} {b} f =
    begin⟨ bundle (P .std a b) ⟩
      P .actʳ (ν .cmp (𝒞 .idn₀ a)) f  ≈⟨ P .std a b .invˢ (ν .natʳ (𝒞 .idn₀ a) f) ⟩
      ν .cmp (𝒞 .seq₀ (𝒞 .idn₀ a) f)  ≈⟨ ν .cmp* (𝒞 .mon-λ f) ⟩
      ν .cmp f                        ≈⟨ ν .cmp* (𝒞 .inv₁ (𝒞 .mon-ρ f)) ⟩
      ν .cmp (𝒞 .seq₀ f (𝒞 .idn₀ b))  ≈⟨ ν .natˡ f (𝒞 .idn₀ b) ⟩
      P .actˡ f (ν .cmp (𝒞 .idn₀ b))  ∎

  -- Extend a wedge along the right action. Left naturality routes the staged
  -- action through the wedge condition and the exchange law; right naturality
  -- is the staging law itself.
  yoneda-from : Wedge P → Pronat (hom-pro 𝒞) P
  yoneda-from w .cmp {a} h = P .actʳ (w .cmp a) h
  yoneda-from w .cmp* {a} σ = P .actʳ↕ σ (w .cmp a)
  yoneda-from w .natˡ {a′} {a} {b} f h =
    begin⟨ bundle (P .std a′ b) ⟩
      P .actʳ (w .cmp a′) (𝒞 .seq₀ f h)   ≈⟨ P .act-seqʳ (w .cmp a′) f h ⟩
      P .actʳ (P .actʳ (w .cmp a′) f) h   ≈⟨ P .actʳ* h (w .dinat f) ⟩
      P .actʳ (P .actˡ f (w .cmp a)) h    ≈⟨ P .act-xchg f (w .cmp a) h ⟩
      P .actˡ f (P .actʳ (w .cmp a) h)    ∎
  yoneda-from w .natʳ {a} h g = P .act-seqʳ (w .cmp a) h g

  -- Round trip at the wedge: extending then restricting returns the diagonal,
  -- because the identity action erases.
  yoneda-from-to : ∀ (w : Wedge P) a
    → P .val a a
    .δ° (yoneda-to (yoneda-from w) .cmp a) (w .cmp a)
    .ϵ°
  yoneda-from-to w a = P .act-idnʳ (w .cmp a)

  -- Round trip at the cell: restricting then extending returns the cell, by
  -- right naturality against the left unit.
  yoneda-to-from : ∀ (ν : Pronat (hom-pro 𝒞) P) {a b} (h : Ξ .δ° a b .ϵ°)
    → P .val a b
    .δ° (yoneda-from (yoneda-to ν) .cmp h) (ν .cmp h)
    .ϵ°
  yoneda-to-from ν {a} {b} h =
    begin⟨ bundle (P .std a b) ⟩
      P .actʳ (ν .cmp (𝒞 .idn₀ a)) h  ≈⟨ P .std a b .invˢ (ν .natʳ (𝒞 .idn₀ a) h) ⟩
      ν .cmp (𝒞 .seq₀ (𝒞 .idn₀ a) h)  ≈⟨ ν .cmp* (𝒞 .mon-λ h) ⟩
      ν .cmp h                        ∎
