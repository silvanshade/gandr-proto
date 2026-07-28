{-# OPTIONS --safe --guardedness --without-K --hidden-argument-puns #-}

------------------------------------------------------------------------------
-- Gandr.Category — the 1-category structure on a carrier ∞-graph.
--
-- A `Category Ξ` is composition-and-identity structure ON the ambient carrier
-- `Ξ`: identity and composition of 1-cells, the 2-cell equivalence over each
-- hom, horizontal composition of 2-cells, and the monoidal coherences as
-- WITNESSED 2-cells.
--
-- ── WEAK BY DEFAULT, AND WHY THAT IS THE HONEST SETTING ─────────────────────
-- The laws are cells, not equations. `mon-α` does not say the two bracketings
-- ARE equal; it supplies a 2-cell between them, which a consumer must then
-- transport along explicitly. Nothing here is strict and nothing needs
-- decidable equality.
--
-- This is forced, not stylistic. The carrier's hom is an ∞-graph, so its cells
-- carry their own equivalence one dimension up — that is what `Gandr.Setoid`
-- names, and it is the only equality available. A strict law would have to be
-- an equation in the ambient theory, which would silently promote the setoid to
-- a set. Stating laws as cells keeps the tree inside SETOID, where its results
-- actually hold.
--
-- ── STRUCTURE ON A CARRIER, NOT A BUNDLE ────────────────────────────────────
-- `Category` is parameterized BY the carrier rather than bundling one. So a
-- hom-setoid is literally `Ξ .δ° a b`, read off by projection, and two
-- structures over the same carrier are comparable without a coercion. This is
-- what makes "we have SETOID, not SET" structural rather than a caveat in prose.
--
-- ── AT ONE DIMENSION, OR AT EVERY DIMENSION ─────────────────────────────────
-- `Category Ξ` is the structure at ONE address. `Everywhere Category Ξ` is the
-- certification at all of them, and `ℂ.Id°` below is the worked instance —
-- what shows the address machinery carries the DOCTRINES and not only the
-- setoid layer that first demanded it.
--
-- The two are not interchangeable, and `ℂ.≡°-not-everywhere` is why: the
-- discrete setoid on the identity type — how every `Set`-level structure in
-- this tree presents as a category — admits no such certification, because it
-- stops with `𝟘` above its 1-cells. So the certification is a strictly stronger
-- hypothesis, available where a carrier has genuine cells all the way up and
-- never assumable in general.
--
-- ── ON `opaque` ─────────────────────────────────────────────────────────────
-- Absent, for the same reason as `Gandr.Graph`: every consumer meets these
-- through copattern matching, and the instances below are definitional
-- unfoldings that later proofs depend on. Opacity resumes at the first module
-- that proves something.
--
-- The layer letter is ℂ, a wrapper mirroring 𝕊 and 𝔾 one dimension up, so
-- later layers requalify as ℂ.idn₀ / ℂ.seq₀ rather than minting fresh names.
-- The congruence field is spelled `seq↕`, not `seq*`, on purpose: `seq*` is the
-- propositional-equality congruence one dimension down, whereas a tower's
-- VERTICAL congruence is `seq↕`, and conflating them loses the dimension.
------------------------------------------------------------------------------

module Gandr.Category where

open import Gandr.Graph
  using (∞Graph)
  using (∞Map)
  using (ϵ°)
  using (δ°)
  using (ϵ↬)
  using (δ↬)
  using (Everywhere)
  using (here°)
  using (up°)
  using (module 𝔾)
open import Gandr.Setoid
  using (Setoid)
  using (idnˢ)
  using (seqˢ)
  using (invˢ)

open import Data.Empty.Polymorphic
  using (⊥)

-- The `Set`-level structure the category layer mirrors one dimension up: the
-- pair vocabulary the product category computes in, and the unique cell of the
-- terminal category.
private
  module 𝕊 where
    open import Data.Unit.Polymorphic public
      using (tt)
    open import Data.Product public
      using (_,_)
      renaming (proj₁ to fst)
      renaming (proj₂ to snd)
  open 𝕊
    using (tt)
    using (_,_)

  -- Propositional equality as a category's worth of operations, named to match
  -- the fields they instantiate. This is the ambient equality read as the
  -- STRICT category on a `Set`, and it is the only strict instance in the tree.
  module ≡ where
    open import Relation.Binary.PropositionalEquality public
      using (_≡_)
      using (cong₂)
      renaming (refl to idn)
      renaming (trans to seq)
      renaming (sym to inv)
    open import Relation.Binary.PropositionalEquality.Properties public
      using (trans-reflʳ)
      using (trans-assoc)

    -- Congruence for transitivity: the horizontal composition of 2-cells.
    seq* : ∀ {a} {A : Set a} {x y z : A} {p p′ : x ≡ y} {q q′ : y ≡ z}
      → p ≡ p′
      → q ≡ q′
      → seq p q ≡ seq p′ q′
    seq* = cong₂ seq

    -- The left unit law. Holds definitionally: `seq` matches on its first
    -- argument, so `seq idn p` already reduces to `p`.
    mon-λ : ∀ {a} {A : Set a} {x y : A} (p : x ≡ y)
      → seq idn p ≡ p
    mon-λ p = idn

    -- The right unit law.
    mon-ρ : ∀ {a} {A : Set a} {x y : A} (p : x ≡ y)
      → seq p idn ≡ p
    mon-ρ = trans-reflʳ

    -- Associativity. The argument order matches the `mon-α` field's, which
    -- takes the two outer cells before the middle one.
    mon-α : ∀ {a} {A : Set a} {w x y z : A}
      → (p : w ≡ x)
      → (r : y ≡ z)
      → (q : x ≡ y)
      → seq (seq p q) r ≡ seq p (seq q r)
    mon-α p r q = trans-assoc p {q} {r}

module ℂ where

  -- ═══════════════════════════════════════════════════════════════════════════
  -- The category structure over `Ξ`. Fields cluster by dimension: the 1-cell
  -- monoid, the 2-cell equivalence, the interchange, then the coherences.
  -- ═══════════════════════════════════════════════════════════════════════════

  record Category {ℓ} (Ξ : ∞Graph ℓ) : Set ℓ where
    field
      -- the identity 1-cell at each 0-cell
      idn₀ : ∀ a
        → Ξ .δ° a a .ϵ°
      -- composition of 1-cells, diagrammatic
      seq₀ : ∀ {a b c}
        → (f : Ξ .δ° a b .ϵ°)
        → (g : Ξ .δ° b c .ϵ°)
        → Ξ .δ° a c .ϵ°
    field
      -- reflexivity of the 2-cells over each hom
      idn₁ : ∀ {a b}
        → (f : Ξ .δ° a b .ϵ°)
        → Ξ .δ° a b .δ° f f .ϵ°
      -- transitivity of the 2-cells over each hom
      seq₁ : ∀ {a b} {f g h}
        → (p : Ξ .δ° a b .δ° f g .ϵ°)
        → (q : Ξ .δ° a b .δ° g h .ϵ°)
        → Ξ .δ° a b .δ° f h .ϵ°
      -- symmetry of the 2-cells over each hom
      inv₁ : ∀ {a b} {f g}
        → (p : Ξ .δ° a b .δ° f g .ϵ°)
        → Ξ .δ° a b .δ° g f .ϵ°
    field
      -- horizontal composition: 2-cells compose over composed 1-cells
      seq↕ : ∀ {a b c} {f f′ g g′}
        → (α : Ξ .δ° a b .δ° f f′ .ϵ°)
        → (β : Ξ .δ° b c .δ° g g′ .ϵ°)
        → Ξ .δ° a c .δ° (seq₀ f g) (seq₀ f′ g′) .ϵ°
    field
      -- the left unit law, as a 2-cell
      mon-λ : ∀ {x y}
        → ∀ f
        → Ξ .δ° x y .δ° (seq₀ (idn₀ x) f) f .ϵ°
      -- the right unit law, as a 2-cell
      mon-ρ : ∀ {x y}
        → ∀ f
        → Ξ .δ° x y .δ° (seq₀ f (idn₀ y)) f .ϵ°
      -- associativity, as a 2-cell
      mon-α : ∀ {w x y z}
        → ∀ f h
        → (g : Ξ .δ° x y .ϵ°)
        → Ξ .δ° w z .δ° (seq₀ (seq₀ f g) h) (seq₀ f (seq₀ g h)) .ϵ°

    -- The 2-cell equivalence over a fixed hom, repackaged as a `Setoid` by
    -- projection. Kept qualified so it never leaks to the bare scope: a
    -- consumer that wants the hom-setoid asks the Category for it.
    homˢ : (a b : Ξ .ϵ°) → Setoid (Ξ .δ° a b)
    homˢ a b .idnˢ = idn₁
    homˢ a b .seqˢ = seq₁
    homˢ a b .invˢ = inv₁
  open Category public
    hiding (homˢ)

  -- ═══════════════════════════════════════════════════════════════════════════
  -- Functors and natural transformations, both weak.
  -- ═══════════════════════════════════════════════════════════════════════════

  -- A functor: an ∞-map whose action preserves identities and composition up to
  -- WITNESSED 2-cells. `map` is the underlying ∞-map, opened locally so a `Map`
  -- reads its cell action as `F .ϵ↬` / `F .δ↬`.
  record Map {ℓ} {A B : ∞Graph ℓ} (𝒜 : Category A) (ℬ : Category B) : Set ℓ where
    field
      -- the underlying map of carriers
      map : ∞Map A B
    open ∞Map map public
    field
      -- the identity is preserved, up to a 2-cell
      idn₀⇒ : ∀ {a}
        → B
        .δ° _ _
        .δ° (map .δ↬ .ϵ↬ (𝒜 .idn₀ a)) (ℬ .idn₀ (map .ϵ↬ a))
        .ϵ°
      -- composition is preserved, up to a 2-cell
      seq₀⇒ : ∀ {a b c}
        → (f : A .δ° a b .ϵ°)
        → (g : A .δ° b c .ϵ°)
        → B
        .δ° _ _
        .δ° (map .δ↬ .ϵ↬ (𝒜 .seq₀ f g)) (ℬ .seq₀ (map .δ↬ .ϵ↬ f) (map .δ↬ .ϵ↬ g))
        .ϵ°
  open Map public

  -- A natural transformation: components as 1-cells `F a ⇴ G a`, and the
  -- naturality 2-cell filling the square its two whiskered composites bound.
  record Nat {ℓ}
    {A B : ∞Graph ℓ}
    {𝒜 : Category A}
    {ℬ : Category B}
    (F G : Map 𝒜 ℬ)
    : Set ℓ where
    field
      -- the component at each 0-cell
      cmp : ∀ a
        → B
        .δ° (F .ϵ↬ a) (G .ϵ↬ a)
        .ϵ°
      -- the naturality square, as a 2-cell
      nat : ∀ {a₀ a₁}
        → (f : A .δ° a₀ a₁ .ϵ°)
        → B
        .δ° (ϵ↬ (map F) a₀) (ϵ↬ (map G) a₁)
        .δ° (ℬ .seq₀ (cmp a₀) (G .δ↬ .ϵ↬ f)) (ℬ .seq₀ (F .δ↬ .ϵ↬ f) (cmp a₁))
        .ϵ°
  open Nat public

  -- ═══════════════════════════════════════════════════════════════════════════
  -- Instances. Each is a named object rather than an appeal to an ambient one,
  -- so a later result that quantifies over categories genuinely has these in
  -- range.
  --
  -- These are the CARRIER-LEVEL instances: each is read off one of `𝔾`'s
  -- objects by copattern matching, and nothing is constructed. Instances with
  -- content — SETOID above all — live in `Gandr.Category.Instances`. If one of
  -- these is ever wanted there, MIGRATE it rather than writing a second copy:
  -- a duplicate would be definitionally equal to the original and so invisible
  -- to the gate. The rule holds in both directions.
  -- ═══════════════════════════════════════════════════════════════════════════

  -- The category a `Set` presents through its identity type. This is the one
  -- STRICT instance in the tree — every law is an equation, because the ambient
  -- equality supplies them — and it is exactly the boundary where a setoid
  -- becomes a set.
  Id : ∀ {ℓ} (A : Set ℓ) → Category (𝔾.Id A)
  Id A .idn₀ a = ≡.idn
  Id A .seq₀ = ≡.seq
  Id A .idn₁ p = ≡.idn
  Id A .seq₁ = ≡.seq
  Id A .inv₁ = ≡.inv
  Id A .seq↕ = ≡.seq*
  Id A .mon-λ = ≡.mon-λ
  Id A .mon-ρ = ≡.mon-ρ
  Id A .mon-α = ≡.mon-α

  -- THE SAME TOWER, CERTIFIED AT EVERY DIMENSION — the worked `Everywhere
  -- Category`, and what shows the certification carries the DOCTRINES and not
  -- only the setoid `Gandr.Setoid.Idˢ` certifies one layer down. One dimension
  -- up from `𝔾.Id A` over a parallel pair is `𝔾.Id (x ≡ y)`, so the same
  -- instance serves there and the corecursion is the whole statement.
  --
  -- The dress: the bare name is the structure AT THE CARRIER, `°` marks its
  -- certification at every address, and `at° (Id° A) ⋆` is `Id A` on the nose
  -- (`Gandr.Graph.at°-⋆`), so the two never drift apart.
  Id° : ∀ {ℓ} (A : Set ℓ) → Everywhere Category (𝔾.Id A)
  Id° A .here° = Id A
  Id° A .up° x y = Id° (≡._≡_ x y)

  -- AND THE REFUTATION, WHICH BITES ONE DIMENSION LOWER THAN IT LOOKS. `𝔾.≡°`
  -- stops with `𝟘` above its 1-cells, so a `Category` over it must produce a
  -- 2-cell — `idn₁` at any 1-cell — out of nothing. So it is not merely the
  -- CERTIFICATION that fails on `≡°`: the bare structure already does.
  --
  -- This is stated first, and separately, because the weaker form is the one
  -- that misleads. `Everywhere Category (𝔾.≡° A)` is refuted too, but only as a
  -- corollary, and reading that corollary as a SEPARATION of the certification
  -- from the structure is an error this record carried until it was checked:
  -- both are absurd here, so `≡°` witnesses no gap between them.
  --
  -- What `≡°` does witness is a REGION. Its content stops at the 1-cells, and
  -- `Gandr.Graph.At` is where that is said — `At Setoid (≡° A) Only⋆` holds
  -- (`Gandr.Setoid.≡ˢ` is the structure), while `At Category (≡° A) Only⋆` does
  -- not, because `Category` asks for one dimension more than `≡°` carries.
  --
  -- Note the doctrines therefore differ, and the difference is the point of
  -- naming regions at all: `Setoid` has content at dimensions 0–1 and `Category`
  -- at 0–2, so the same carrier serves one and fails the other. A `Set`-level
  -- CATEGORY presents with `≡°` on each HOM — `δ° x y = ≡° (H x y)` — and that
  -- carrier's region is `Only⋆`.
  ≡°-not-category : ∀ {ℓ} {A : Set ℓ}
    → A
    → Category (𝔾.≡° A)
    → ⊥
  ≡°-not-category a 𝒞 = 𝒞 .idn₁ {a} {a} ≡.idn

  -- the corollary, kept because it is the statement the certification layer
  -- quotes, and because a predicate nothing refutes may be vacuous
  ≡°-not-everywhere : ∀ {ℓ} {A : Set ℓ}
    → A
    → Everywhere Category (𝔾.≡° A)
    → ⊥
  ≡°-not-everywhere a 𝒞° = ≡°-not-category a (𝒞° .here°)

  -- The empty category: the unique structure on `𝔾.𝟘`. Every clause is absurd,
  -- there being no cells to define at any dimension.
  𝟘 : ∀ {ℓ} → Category {ℓ} 𝔾.𝟘
  𝟘 .idn₀ ()
  𝟘 .seq₀ {()}
  𝟘 .idn₁ {()}
  𝟘 .seq₁ {()}
  𝟘 .inv₁ {()}
  𝟘 .seq↕ {()}
  𝟘 .mon-λ {()}
  𝟘 .mon-ρ {()}
  𝟘 .mon-α {()}

  -- The terminal category: the unique structure on `𝔾.𝟙`. Every operation and
  -- every law is the unique cell, at every dimension.
  𝟙 : ∀ {ℓ} → Category {ℓ} 𝔾.𝟙
  𝟙 .idn₀ _ = tt
  𝟙 .seq₀ _ _ = tt
  𝟙 .idn₁ _ = tt
  𝟙 .seq₁ _ _ = tt
  𝟙 .inv₁ _ = tt
  𝟙 .seq↕ _ _ = tt
  𝟙 .mon-λ _ = tt
  𝟙 .mon-ρ _ = tt
  𝟙 .mon-α _ _ _ = tt

  -- The terminal functor. `𝔾.!` underneath, and since every cell of `𝔾.𝟙` is
  -- the unique one at every dimension, both preservation cells are too.
  ! : ∀ {ℓ} {A : ∞Graph ℓ} {𝒜 : Category A} → Map 𝒜 𝟙
  ! .map = 𝔾.!
  ! .idn₀⇒ = tt
  ! .seq₀⇒ f g = tt

  -- The product category: structure on the graph product, computed level-wise
  -- in each factor. The fixity mirrors `𝔾._⊗_`, so `𝒜 × ℬ × 𝒞` parses as its
  -- carrier does.
  infixr 2 _×_
  _×_ : ∀ {ℓ} {A B : ∞Graph ℓ} → Category {ℓ} A → Category {ℓ} B → Category {ℓ} (A 𝔾.⊗ B)
  _×_ 𝒜 ℬ .idn₀ (a , b) = (𝒜 .idn₀ a , ℬ .idn₀ b)
  _×_ 𝒜 ℬ .seq₀ (f₀ , g₀) (f₁ , g₁) = (𝒜 .seq₀ f₀ f₁ , ℬ .seq₀ g₀ g₁)
  _×_ 𝒜 ℬ .idn₁ (p , q) = (𝒜 .idn₁ p , ℬ .idn₁ q)
  _×_ 𝒜 ℬ .seq₁ (α₀ , β₀) (α₁ , β₁) = (𝒜 .seq₁ α₀ α₁ , ℬ .seq₁ β₀ β₁)
  _×_ 𝒜 ℬ .inv₁ (α , β) = (𝒜 .inv₁ α , ℬ .inv₁ β)
  _×_ 𝒜 ℬ .seq↕ (α₀ , β₀) (α₁ , β₁) = (𝒜 .seq↕ α₀ α₁ , ℬ .seq↕ β₀ β₁)
  _×_ 𝒜 ℬ .mon-λ (α , β) = (𝒜 .mon-λ α , ℬ .mon-λ β)
  _×_ 𝒜 ℬ .mon-ρ (α , β) = (𝒜 .mon-ρ α , ℬ .mon-ρ β)
  _×_ 𝒜 ℬ .mon-α (α₀ , β₀) (α₁ , β₁) (α₂ , β₂) = (𝒜 .mon-α α₀ α₁ α₂ , ℬ .mon-α β₀ β₁ β₂)

  -- The left projection, as a STRICT functor: the product's operations compute
  -- componentwise, so the preservation cells are reflexive.
  fst : ∀ {ℓ} {A B : ∞Graph ℓ} {𝒜 : Category A} {ℬ : Category B}
    → Map (𝒜 × ℬ) 𝒜
  fst .map = 𝔾.fst
  fst {𝒜} .idn₀⇒ {a} = 𝒜 .idn₁ (𝒜 .idn₀ (𝕊.fst a))
  fst {𝒜} .seq₀⇒ f g = 𝒜 .idn₁ (𝒜 .seq₀ (𝕊.fst f) (𝕊.fst g))

  -- The right projection, likewise strict.
  snd : ∀ {ℓ} {A B : ∞Graph ℓ} {𝒜 : Category A} {ℬ : Category B}
    → Map (𝒜 × ℬ) ℬ
  snd .map = 𝔾.snd
  snd {ℬ} .idn₀⇒ {a} = ℬ .idn₁ (ℬ .idn₀ (𝕊.snd a))
  snd {ℬ} .seq₀⇒ f g = ℬ .idn₁ (ℬ .seq₀ (𝕊.snd f) (𝕊.snd g))

  -- The pairing: the product's universal property, with the factors' own
  -- preservation cells paired.
  ⟨_,_⟩ : ∀ {ℓ} {X A B : ∞Graph ℓ}
    {𝒳 : Category X} {𝒜 : Category A} {ℬ : Category B}
    → Map 𝒳 𝒜 → Map 𝒳 ℬ → Map 𝒳 (𝒜 × ℬ)
  ⟨_,_⟩ F G .map = 𝔾.⟨ F .map , G .map ⟩
  ⟨_,_⟩ F G .idn₀⇒ = (F .idn₀⇒ , G .idn₀⇒)
  ⟨_,_⟩ F G .seq₀⇒ f g = (F .seq₀⇒ f g , G .seq₀⇒ f g)

-- Only the structure vocabulary is unqualified at the top level; the instances
-- stay behind `ℂ.` so later layers can mirror them without collision.
open ℂ public
  using (Category)
  using (module Category)
  using (idn₀)
  using (seq₀)
  using (idn₁)
  using (seq₁)
  using (inv₁)
  using (seq↕)
  using (mon-λ)
  using (mon-ρ)
  using (mon-α)
  using (Map)
  using (module Map)
  using (map)
  using (idn₀⇒)
  using (seq₀⇒)
  using (Nat)
  using (module Nat)
  using (cmp)
  using (nat)
