{-# OPTIONS --safe --guardedness --without-K --hidden-argument-puns #-}

------------------------------------------------------------------------------
-- Gandr.Profunctor — setoid-valued profunctors between categories.
--
-- A `Profunctor 𝒜 ℬ` assigns to each object pair a VALUE ∞-graph carrying a
-- `Setoid` on its cells, together with a contravariant left action of 𝒜's
-- 1-cells and a covariant right action of ℬ's. All laws are witnessed value
-- cells.
--
-- ── TWO-SIDED, SO NO OPPOSITE CATEGORY IS EVER FORMED ───────────────────────
-- Contravariance lives in the ACTIONS, not in an `op` construction on the
-- source category. This is a deliberate choice, not a stylistic one: forming
-- `𝒜 ᵒᵖ` for a carrier-parameterized `Category` means building a new carrier
-- ∞-graph with the coboundary flipped at every dimension, and then every
-- statement about the opposite has to be transported back. The two-sided
-- formulation pays nothing and proves the same things.
--
-- ── WHY THE VALUES ARE GRAPHS RATHER THAN SETS ──────────────────────────────
-- `val a b` is an ∞-graph and `std a b` its cell setoid, so a profunctor value
-- is a proof-relevant setoid rather than a set. Profunctors are consequently
-- never compared AS RECORDS — only their values, in the value setoids. That is
-- the discipline the whole layer runs on, and it is what "we have SETOID, not
-- SET" amounts to here: there is no equality of profunctors to appeal to, so
-- every statement has to be about components.
--
-- Eta is switched off on the record for that reason, and for a mechanical one:
-- with eta, an implicit profunctor argument gets eta-expanded into a stuck
-- metavariable on the left-hand side of any cell-layer constructor.
--
-- Congruence fields follow two axes. The `*` family rewrites the acted-on
-- VALUE along a value cell; the `↕` family rewrites the acting ARROW along a
-- 2-cell of its category.
------------------------------------------------------------------------------

module Gandr.Profunctor where

open import Gandr.Graph
  using (∞Graph)
  using (ϵ°)
  using (δ°)
open import Gandr.Setoid
  using (Setoid)
  using (idnˢ)
  using (seqˢ)
  using (invˢ)
open import Gandr.Category
  using (Category)
  using (idn₀)
  using (seq₀)
  using (idn₁)
  using (inv₁)
  using (seq↕)
  using (mon-λ)
  using (mon-ρ)
  using (mon-α)

open import Level
  renaming (suc to lsuc)

-- ══════════════════════════════════════════════════════════════════════════════
-- The profunctor record: setoid values, two-sided actions, witnessed laws.
-- ══════════════════════════════════════════════════════════════════════════════

record Profunctor {ℓ} {A B : ∞Graph ℓ} (𝒜 : Category A) (ℬ : Category B)
  : Set (lsuc ℓ) where
  -- See the header: profunctors are never compared as records, and eta here
  -- would strand implicit profunctor arguments as stuck metas.
  no-eta-equality
  field
    -- the value ∞-graph at each object pair
    val : A .ϵ° → B .ϵ° → ∞Graph ℓ
    -- and the proof-relevant equivalence on its cells
    std : ∀ a b → Setoid (val a b)
  field
    -- the contravariant left action, diagrammatic
    actˡ : ∀ {a′ a b}
      → (f : A .δ° a′ a .ϵ°)
      → (p : val a b .ϵ°)
      → val a′ b .ϵ°
    -- the covariant right action
    actʳ : ∀ {a b b′}
      → (p : val a b .ϵ°)
      → (g : B .δ° b b′ .ϵ°)
      → val a b′ .ϵ°
  field
    -- value congruence: the left action respects the value setoid
    actˡ* : ∀ {a′ a b}
      → (f : A .δ° a′ a .ϵ°)
      → {p q : val a b .ϵ°}
      → val a b .δ° p q .ϵ°
      → val a′ b
      .δ° (actˡ f p) (actˡ f q)
      .ϵ°
    -- value congruence: the right action respects the value setoid
    actʳ* : ∀ {a b b′}
      → (g : B .δ° b b′ .ϵ°)
      → {p q : val a b .ϵ°}
      → val a b .δ° p q .ϵ°
      → val a b′
      .δ° (actʳ p g) (actʳ q g)
      .ϵ°
  field
    -- arrow congruence: the left action respects 2-cells of its category
    actˡ↕ : ∀ {a′ a b} {f f′ : A .δ° a′ a .ϵ°}
      → (α : A .δ° a′ a .δ° f f′ .ϵ°)
      → (p : val a b .ϵ°)
      → val a′ b
      .δ° (actˡ f p) (actˡ f′ p)
      .ϵ°
    -- arrow congruence: the right action respects 2-cells of its category
    actʳ↕ : ∀ {a b b′} {g g′ : B .δ° b b′ .ϵ°}
      → (β : B .δ° b b′ .δ° g g′ .ϵ°)
      → (p : val a b .ϵ°)
      → val a b′
      .δ° (actʳ p g) (actʳ p g′)
      .ϵ°
  field
    -- the identity acts trivially on the left
    act-idnˡ : ∀ {a b}
      → (p : val a b .ϵ°)
      → val a b
      .δ° (actˡ (𝒜 .idn₀ a) p) p
      .ϵ°
    -- the identity acts trivially on the right
    act-idnʳ : ∀ {a b}
      → (p : val a b .ϵ°)
      → val a b
      .δ° (actʳ p (ℬ .idn₀ b)) p
      .ϵ°
    -- a composite acts in stages, on the left
    act-seqˡ : ∀ {a″ a′ a b}
      → (f : A .δ° a″ a′ .ϵ°)
      → (f′ : A .δ° a′ a .ϵ°)
      → (p : val a b .ϵ°)
      → val a″ b
      .δ° (actˡ (𝒜 .seq₀ f f′) p) (actˡ f (actˡ f′ p))
      .ϵ°
    -- a composite acts in stages, on the right
    act-seqʳ : ∀ {a b b′ b″}
      → (p : val a b .ϵ°)
      → (g : B .δ° b b′ .ϵ°)
      → (g′ : B .δ° b′ b″ .ϵ°)
      → val a b″
      .δ° (actʳ p (ℬ .seq₀ g g′)) (actʳ (actʳ p g) g′)
      .ϵ°
    -- the two actions commute
    act-xchg : ∀ {a′ a b b′}
      → (f : A .δ° a′ a .ϵ°)
      → (p : val a b .ϵ°)
      → (g : B .δ° b b′ .ϵ°)
      → val a′ b′
      .δ° (actʳ (actˡ f p) g) (actˡ f (actʳ p g))
      .ϵ°
open Profunctor public

-- ══════════════════════════════════════════════════════════════════════════════
-- The hom profunctor. Values are the hom graphs themselves, the setoids are
-- exactly the category's repackaged 2-cell equivalences, the actions are
-- composition, and EVERY law is a monoidal coherence already in the record —
-- nothing new is proved. That it costs nothing is the point: the representable
-- is free, which is what makes the Yoneda correspondence a statement about the
-- category rather than about a construction over it.
-- ══════════════════════════════════════════════════════════════════════════════

hom-pro : ∀ {ℓ} {Ξ : ∞Graph ℓ} (𝒞 : Category Ξ) → Profunctor 𝒞 𝒞
hom-pro {Ξ} 𝒞 .val a b = Ξ .δ° a b
hom-pro {Ξ} 𝒞 .std = Category.homˢ 𝒞
hom-pro {Ξ} 𝒞 .actˡ f p = 𝒞 .seq₀ f p
hom-pro {Ξ} 𝒞 .actʳ p g = 𝒞 .seq₀ p g
hom-pro {Ξ} 𝒞 .actˡ* f σ = 𝒞 .seq↕ (𝒞 .idn₁ f) σ
hom-pro {Ξ} 𝒞 .actʳ* g σ = 𝒞 .seq↕ σ (𝒞 .idn₁ g)
hom-pro {Ξ} 𝒞 .actˡ↕ α p = 𝒞 .seq↕ α (𝒞 .idn₁ p)
hom-pro {Ξ} 𝒞 .actʳ↕ β p = 𝒞 .seq↕ (𝒞 .idn₁ p) β
hom-pro {Ξ} 𝒞 .act-idnˡ p = 𝒞 .mon-λ p
hom-pro {Ξ} 𝒞 .act-idnʳ p = 𝒞 .mon-ρ p
hom-pro {Ξ} 𝒞 .act-seqˡ f f′ p = 𝒞 .mon-α f p f′
hom-pro {Ξ} 𝒞 .act-seqʳ p g g′ = 𝒞 .inv₁ (𝒞 .mon-α p g′ g)
hom-pro {Ξ} 𝒞 .act-xchg f p g = 𝒞 .mon-α f g p

-- ══════════════════════════════════════════════════════════════════════════════
-- Cells between parallel profunctors: two-sided natural families with
-- congruent components. Cell equality is pointwise value equivalence, so the
-- naturality witnesses are CARRIED rather than compared.
-- ══════════════════════════════════════════════════════════════════════════════

record Pronat {ℓ} {A B : ∞Graph ℓ} {𝒜 : Category A} {ℬ : Category B}
  (P Q : Profunctor 𝒜 ℬ) : Set ℓ where
  field
    -- the component family
    cmp : ∀ {a b}
      → P .val a b .ϵ°
      → Q .val a b .ϵ°
    -- which is congruent for the value setoids
    cmp* : ∀ {a b} {p q : P .val a b .ϵ°}
      → P .val a b .δ° p q .ϵ°
      → Q .val a b
      .δ° (cmp p) (cmp q)
      .ϵ°
  field
    -- naturality for the left action
    natˡ : ∀ {a′ a b}
      → (f : A .δ° a′ a .ϵ°)
      → (p : P .val a b .ϵ°)
      → Q .val a′ b
      .δ° (cmp (P .actˡ f p)) (Q .actˡ f (cmp p))
      .ϵ°
    -- naturality for the right action
    natʳ : ∀ {a b b′}
      → (p : P .val a b .ϵ°)
      → (g : B .δ° b b′ .ϵ°)
      → Q .val a b′
      .δ° (cmp (P .actʳ p g)) (Q .actʳ (cmp p) g)
      .ϵ°
open Pronat public

-- The identity cell: identity components, reflexive squares.
idn-pronat : ∀ {ℓ} {A B : ∞Graph ℓ} {𝒜 : Category A} {ℬ : Category B}
  → (P : Profunctor 𝒜 ℬ)
  → Pronat P P
idn-pronat P .cmp p = p
idn-pronat P .cmp* σ = σ
idn-pronat P .natˡ f p = P .std _ _ .idnˢ (P .actˡ f p)
idn-pronat P .natʳ p g = P .std _ _ .idnˢ (P .actʳ p g)

-- Vertical composition of cells: composed components, pasted squares. Carry
-- the first square through the second's congruence, then paste the second's.
seq-pronat : ∀ {ℓ} {A B : ∞Graph ℓ} {𝒜 : Category A} {ℬ : Category B}
  → {P Q R : Profunctor 𝒜 ℬ}
  → (ν : Pronat P Q)
  → (μ : Pronat Q R)
  → Pronat P R
seq-pronat ν μ .cmp p = μ .cmp (ν .cmp p)
seq-pronat ν μ .cmp* σ = μ .cmp* (ν .cmp* σ)
seq-pronat {R} ν μ .natˡ f p =
  R .std _ _ .seqˢ
    (μ .cmp* (ν .natˡ f p))
    (μ .natˡ f (ν .cmp p))
seq-pronat {R} ν μ .natʳ p g =
  R .std _ _ .seqˢ
    (μ .cmp* (ν .natʳ p g))
    (μ .natʳ (ν .cmp p) g)

-- ══════════════════════════════════════════════════════════════════════════════
-- The diagonal notions: wedges and dinatural transformations of
-- endo-profunctors — the contraction locus.
-- ══════════════════════════════════════════════════════════════════════════════

-- A wedge of an endo-profunctor: a diagonal family whose two action images
-- agree. These are the elements of the end, and the semantic shape of the
-- Yoneda correspondence below.
record Wedge {ℓ} {Ξ : ∞Graph ℓ} {𝒞 : Category Ξ} (P : Profunctor 𝒞 𝒞)
  : Set ℓ where
  field
    -- the diagonal component at each object
    cmp : ∀ a
      → P .val a a .ϵ°
    -- the wedge condition: acting on the right at `a` and on the left at `b`
    -- land in the same value
    dinat : ∀ {a b}
      → (f : Ξ .δ° a b .ϵ°)
      → P .val a b
      .δ° (P .actʳ (cmp a) f) (P .actˡ f (cmp b))
      .ϵ°
open Wedge public

-- A dinatural transformation between endo-profunctors.
--
-- Composition of these is NOT admissible in general — that is a real boundary,
-- not an omission. The identity below exists over any category, so the
-- diagonal's unit is free; it is composition where directedness bites.
record Dinat {ℓ} {Ξ : ∞Graph ℓ} {𝒞 : Category Ξ} (P Q : Profunctor 𝒞 𝒞)
  : Set ℓ where
  field
    -- the diagonal component family
    cmp : ∀ a
      → P .val a a .ϵ°
      → Q .val a a .ϵ°
    -- which is congruent for the value setoids
    cmp* : ∀ {a} {p q : P .val a a .ϵ°}
      → P .val a a .δ° p q .ϵ°
      → Q .val a a
      .δ° (cmp a p) (cmp a q)
      .ϵ°
  field
    -- the dinaturality hexagon
    dinat : ∀ {a b}
      → (f : Ξ .δ° a b .ϵ°)
      → (p : P .val b a .ϵ°)
      → Q .val a b
      .δ° (Q .actʳ (cmp a (P .actˡ f p)) f) (Q .actˡ f (cmp b (P .actʳ p f)))
      .ϵ°
open Dinat public

-- The identity dinatural. Its hexagon is exactly the action-exchange law, so
-- it costs nothing and needs no hypothesis on the category.
idn-dinat : ∀ {ℓ} {Ξ : ∞Graph ℓ} {𝒞 : Category Ξ}
  → (P : Profunctor 𝒞 𝒞)
  → Dinat P P
idn-dinat P .cmp a p = p
idn-dinat P .cmp* σ = σ
idn-dinat P .dinat f p = P .act-xchg f p f
