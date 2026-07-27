{-# OPTIONS --safe --guardedness --without-K --hidden-argument-puns #-}

------------------------------------------------------------------------------
-- Gandr.Category.Functor — the 1-cells and 2-cells of the 2-category of
-- categories, assembled as functors and natural transformations.
--
-- This is the substrate the category-of-categories assemblies are built over:
-- identity and composition of functors, identity and vertical composition of
-- natural transformations, horizontal composition, and the unit and
-- associativity coherences.
--
-- ── A STRICTNESS FACT THAT IS EASY TO GET BACKWARDS ─────────────────────────
-- Coinductive records have NO eta. So `𝔾.seq 𝔾.idn F` is NOT definitionally
-- `F` as an ∞-map, and the functor unit and associativity coherences are
-- genuinely not identities at the level of the underlying maps.
--
-- But FUNCTION eta does hold, and the cell actions are functions. So the two
-- sides of each coherence agree definitionally on objects and on 1-cells, and
-- each coherence is witnessed by the IDENTITY-COMPONENT natural isomorphism.
-- The coherences are therefore strict where it counts and weak only in the
-- packaging — which is why `unit-λ-nat`, `unit-ρ-nat` and `assoc-nat` are all
-- the same two lines, and why each has a reverse that is the same two lines
-- again.
--
-- ── WHERE THE SOLVER WILL EVENTUALLY TAKE OVER ──────────────────────────────
-- The naturality squares in `seq-nat` and `hcomp-nat` are pure reassociation
-- and whiskering, which is exactly what a free-category solver decides. They
-- are written by hand here because that solver does not exist yet. When it
-- lands, these two proofs are its first customers.
------------------------------------------------------------------------------

module Gandr.Category.Functor where

open import Gandr.Graph
  using (∞Graph)
  using (ϵ°)
  using (δ°)
  using (ϵ↬)
  using (δ↬)
  using (module 𝔾)
open import Gandr.Category
  using (Category)
  using (Map)
  using (module Map)
  using (Nat)
  using (map)
  using (cmp)
  using (nat)
  using (idn₀)
  using (seq₀)
  using (idn₁)
  using (seq₁)
  using (inv₁)
  using (seq↕)
  using (mon-λ)
  using (mon-ρ)
  using (mon-α)
  using (idn₀⇒)
  using (seq₀⇒)
open import Gandr.Category.Reasoning

-- A functor inherits its underlying map's cell actions, so `F .ϵ↬` / `F .δ↬`
-- read directly off a `Map`. Brought in explicitly to OVERLOAD the ∞-map
-- projections of the same name: Agda picks by the principal argument's type.
open Map
  using (ϵ↬)
  using (δ↬)

-- ══════════════════════════════════════════════════════════════════════════════
-- Functors: identity and composition. Both are STRICT — the underlying ∞-map is
-- literally `𝔾.idn` / `𝔾.seq`, with no coherence inserted.
-- ══════════════════════════════════════════════════════════════════════════════

-- The identity functor. Its preservation cells are reflexive, since the
-- identity map leaves the identity and composition images definitionally
-- unchanged.
idn-map : ∀ {ℓ} {A : ∞Graph ℓ} (𝒜 : Category A) → Map 𝒜 𝒜
idn-map 𝒜 .map = 𝔾.idn
idn-map 𝒜 .idn₀⇒ {a} = 𝒜 .idn₁ (𝒜 .idn₀ a)
idn-map 𝒜 .seq₀⇒ f g = 𝒜 .idn₁ (𝒜 .seq₀ f g)

-- Composition of functors. The preservation cells transport the first factor's
-- cell through the second factor's action, then paste on the second's own.
seq-map : ∀ {ℓ} {A B C : ∞Graph ℓ} {𝒜 : Category A} {ℬ : Category B} {𝒞 : Category C}
  → Map 𝒜 ℬ
  → Map ℬ 𝒞
  → Map 𝒜 𝒞
seq-map F G .map = 𝔾.seq (F .map) (G .map)
seq-map {𝒞} F G .idn₀⇒ =
  𝒞 .seq₁ (G .map .δ↬ .δ↬ .ϵ↬ (F .idn₀⇒)) (G .idn₀⇒)
seq-map {𝒞} F G .seq₀⇒ f g =
  𝒞 .seq₁ (G .map .δ↬ .δ↬ .ϵ↬ (F .seq₀⇒ f g)) (G .seq₀⇒ (F .δ↬ .ϵ↬ f) (F .δ↬ .ϵ↬ g))

-- ══════════════════════════════════════════════════════════════════════════════
-- Natural transformations. The 2-cell layer of a `Category` is a bare setoid,
-- so each construction need only INHABIT a well-typed `Nat`; all the work is
-- filling the naturality square.
-- ══════════════════════════════════════════════════════════════════════════════

-- The square every identity-component natural transformation reduces to: both
-- units cancel, so `idn ⋆ φ` and `φ ⋆ idn` agree.
idn-□ : ∀ {ℓ} {B : ∞Graph ℓ} (ℬ : Category B) {x y} (φ : B .δ° x y .ϵ°)
  → B .δ° x y .δ° (ℬ .seq₀ (ℬ .idn₀ x) φ) (ℬ .seq₀ φ (ℬ .idn₀ y)) .ϵ°
idn-□ ℬ φ = ℬ .seq₁ (ℬ .mon-λ φ) (ℬ .inv₁ (ℬ .mon-ρ φ))

-- The identity natural transformation on a functor.
idn-nat : ∀ {ℓ} {A B : ∞Graph ℓ} {𝒜 : Category A} {ℬ : Category B}
  → (F : Map 𝒜 ℬ)
  → Nat F F
idn-nat {ℬ} F .cmp a = ℬ .idn₀ (F .ϵ↬ a)
idn-nat {ℬ} F .nat f = idn-□ ℬ (F .δ↬ .ϵ↬ f)

-- Vertical composition of natural transformations. The square is the standard
-- 2-by-2 pasting: slide the trailing square across, then the leading one, then
-- reassociate.
seq-nat : ∀ {ℓ} {A B : ∞Graph ℓ} {𝒜 : Category A} {ℬ : Category B}
  → {F G H : Map 𝒜 ℬ}
  → (α : Nat F G)
  → (β : Nat G H)
  → Nat F H
seq-nat {ℬ} α β .cmp a = ℬ .seq₀ (α .cmp a) (β .cmp a)
seq-nat {ℬ} {F} {G} {H} α β .nat {a₀} {a₁} f =
  begin⟨ homᵇ (F .ϵ↬ a₀) (H .ϵ↬ a₁) ⟩
    ℬ .seq₀ (ℬ .seq₀ α₀ β₀) Hf  ≈⟨ pullʳ (β .nat f) ⟩
    ℬ .seq₀ α₀ (ℬ .seq₀ Gf β₁)  ≈⟨ pullˡ (α .nat f) ⟩
    ℬ .seq₀ (ℬ .seq₀ Ff α₁) β₁  ≈⟨ ℬ .mon-α Ff β₁ α₁ ⟩
    ℬ .seq₀ Ff (ℬ .seq₀ α₁ β₁)  ∎
  where
    open Reasoning ℬ
    α₀ = α .cmp a₀
    α₁ = α .cmp a₁
    β₀ = β .cmp a₀
    β₁ = β .cmp a₁
    Ff = F .δ↬ .ϵ↬ f
    Gf = G .δ↬ .ϵ↬ f
    Hf = H .δ↬ .ϵ↬ f

-- Horizontal composition: `α : F ⟹ F′` in the middle category and
-- `β : G ⟹ G′` in the target compose to `F⋆G ⟹ F′⋆G′`. The square threads the
-- target functor's action on the source square through its own composition
-- preservation cells, which is the one place `seq₀⇒` does real work.
hcomp-nat : ∀ {ℓ} {A B C : ∞Graph ℓ} {𝒜 : Category A} {ℬ : Category B} {𝒞 : Category C}
  → {F F′ : Map 𝒜 ℬ}
  → {G G′ : Map ℬ 𝒞}
  → (α : Nat F F′)
  → (β : Nat G G′)
  → Nat (seq-map F G) (seq-map F′ G′)
hcomp-nat {𝒞} {F} {F′} {G} {G′} α β .cmp a =
  𝒞 .seq₀ (G .δ↬ .ϵ↬ (α .cmp a)) (β .cmp (F′ .ϵ↬ a))
hcomp-nat {𝒞} {F} {F′} {G} {G′} α β .nat {a₀} {a₁} f =
  begin⟨ homᵇ (G .ϵ↬ (F .ϵ↬ a₀)) (G′ .ϵ↬ (F′ .ϵ↬ a₁)) ⟩
    𝒞 .seq₀ (𝒞 .seq₀ Gα₀ βF′a₀) G′F′f  ≈⟨ pullʳ (β .nat F′f) ⟩
    𝒞 .seq₀ Gα₀ (𝒞 .seq₀ GF′f βF′a₁)   ≈⟨ pullˡ Gnat ⟩
    𝒞 .seq₀ (𝒞 .seq₀ GFf Gα₁) βF′a₁    ≈⟨ 𝒞 .mon-α GFf βF′a₁ Gα₁ ⟩
    𝒞 .seq₀ GFf (𝒞 .seq₀ Gα₁ βF′a₁)    ∎
  where
    open Reasoning 𝒞
    α₀ = α .cmp a₀
    α₁ = α .cmp a₁
    Ff = F .δ↬ .ϵ↬ f
    F′f = F′ .δ↬ .ϵ↬ f
    Gα₀ = G .δ↬ .ϵ↬ α₀
    Gα₁ = G .δ↬ .ϵ↬ α₁
    GFf = G .δ↬ .ϵ↬ Ff
    GF′f = G .δ↬ .ϵ↬ F′f
    G′F′f = G′ .δ↬ .ϵ↬ F′f
    βF′a₀ = β .cmp (F′ .ϵ↬ a₀)
    βF′a₁ = β .cmp (F′ .ϵ↬ a₁)
    -- The target functor's image of the source square, translated through the
    -- composition-preservation cells so it lands at the right boundary.
    Gnat =
      𝒞 .seq₁
        (𝒞 .inv₁ (G .seq₀⇒ α₀ F′f))
        (𝒞 .seq₁
          (G .map .δ↬ .δ↬ .ϵ↬ (α .nat f))
          (G .seq₀⇒ Ff α₁))

-- ══════════════════════════════════════════════════════════════════════════════
-- The functor coherences. Each is the identity-component natural isomorphism,
-- for the reason in the header: function eta makes both sides agree
-- definitionally on objects and 1-cells, so there is nothing to transport.
-- ══════════════════════════════════════════════════════════════════════════════

-- The left unit coherence.
unit-λ-nat : ∀ {ℓ} {A B : ∞Graph ℓ} {𝒜 : Category A} {ℬ : Category B}
  → (F : Map 𝒜 ℬ)
  → Nat (seq-map (idn-map 𝒜) F) F
unit-λ-nat {ℬ} F .cmp a = ℬ .idn₀ (F .ϵ↬ a)
unit-λ-nat {ℬ} F .nat f = idn-□ ℬ (F .δ↬ .ϵ↬ f)

-- The right unit coherence.
unit-ρ-nat : ∀ {ℓ} {A B : ∞Graph ℓ} {𝒜 : Category A} {ℬ : Category B}
  → (F : Map 𝒜 ℬ)
  → Nat (seq-map F (idn-map ℬ)) F
unit-ρ-nat {ℬ} F .cmp a = ℬ .idn₀ (F .ϵ↬ a)
unit-ρ-nat {ℬ} F .nat f = idn-□ ℬ (F .δ↬ .ϵ↬ f)

-- The associativity coherence.
assoc-nat : ∀ {ℓ} {A B C D : ∞Graph ℓ}
  → {𝒜 : Category A} {ℬ : Category B} {𝒞 : Category C} {𝒟 : Category D}
  → (F : Map 𝒜 ℬ) (G : Map ℬ 𝒞) (H : Map 𝒞 𝒟)
  → Nat (seq-map (seq-map F G) H) (seq-map F (seq-map G H))
assoc-nat {𝒟} F G H .cmp a = 𝒟 .idn₀ (seq-map F (seq-map G H) .ϵ↬ a)
assoc-nat {𝒟} F G H .nat f = idn-□ 𝒟 (seq-map F (seq-map G H) .δ↬ .ϵ↬ f)

-- The reverses. Same identity components: an assembly pairs each coherence
-- with its reverse into a natural isomorphism.
unit-λ-nat⁻ : ∀ {ℓ} {A B : ∞Graph ℓ} {𝒜 : Category A} {ℬ : Category B}
  → (F : Map 𝒜 ℬ)
  → Nat F (seq-map (idn-map 𝒜) F)
unit-λ-nat⁻ {ℬ} F .cmp a = ℬ .idn₀ (F .ϵ↬ a)
unit-λ-nat⁻ {ℬ} F .nat f = idn-□ ℬ (F .δ↬ .ϵ↬ f)

unit-ρ-nat⁻ : ∀ {ℓ} {A B : ∞Graph ℓ} {𝒜 : Category A} {ℬ : Category B}
  → (F : Map 𝒜 ℬ)
  → Nat F (seq-map F (idn-map ℬ))
unit-ρ-nat⁻ {ℬ} F .cmp a = ℬ .idn₀ (F .ϵ↬ a)
unit-ρ-nat⁻ {ℬ} F .nat f = idn-□ ℬ (F .δ↬ .ϵ↬ f)

assoc-nat⁻ : ∀ {ℓ} {A B C D : ∞Graph ℓ}
  → {𝒜 : Category A} {ℬ : Category B} {𝒞 : Category C} {𝒟 : Category D}
  → (F : Map 𝒜 ℬ) (G : Map ℬ 𝒞) (H : Map 𝒞 𝒟)
  → Nat (seq-map F (seq-map G H)) (seq-map (seq-map F G) H)
assoc-nat⁻ {𝒟} F G H .cmp a = 𝒟 .idn₀ (seq-map F (seq-map G H) .ϵ↬ a)
assoc-nat⁻ {𝒟} F G H .nat f = idn-□ 𝒟 (seq-map F (seq-map G H) .δ↬ .ϵ↬ f)
