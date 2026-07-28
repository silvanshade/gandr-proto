{-# OPTIONS --safe --guardedness --without-K --hidden-argument-puns #-}

------------------------------------------------------------------------------
-- Gandr.Category.Reasoning — equational reasoning in a category's homs.
--
-- Two things, kept apart on purpose because only one of them is ours.
--
-- ── THE SYNTAX IS AGDA-STDLIB'S, UNMODIFIED ─────────────────────────────────
-- A category's 2-cells over a fixed hom form a genuine stdlib setoid bundle:
-- the carrier is the 1-cells, the relation is the 2-cells, and `idn₁`/`inv₁`/
-- `seq₁` are exactly `refl`/`sym`/`trans`. `Gandr.Setoid.bundle` performs that
-- repackaging, losing nothing, since stdlib's `IsEquivalence` is as lawless as
-- ours and never truncates the relation.
--
-- So `begin`/`≈⟨_⟩`/`∎` come straight from stdlib. The one wrinkle is that a
-- category's 2-cells are INDEXED by the hom, whereas `Reasoning.Setoid` is
-- parameterized by a single bundle. `Reasoning.MultiSetoid` is the exact fit:
-- it takes the setoid as a datatype PARAMETER, so it is recovered by
-- unification, and a chain names its hom once at `begin⟨_⟩`. One `open` then
-- serves every hom of the category.
--
-- ── THE COMBINATORS ARE OURS ────────────────────────────────────────────────
-- What stdlib does not supply, because it is categorical rather than
-- setoid-theoretic: whiskering, the reassociation ladders, identity
-- introduction and elimination, the cancellers, and the square extensions.
-- These are the agda-categories suite, re-derived here against this tree's
-- carrier-parameterized `Category` and its witnessed coherences.
--
-- Every one of them is a 2-cell built from `seq₁`/`inv₁`/`seq↕` and the three
-- coherences. Nothing here is strict, and nothing needs decidable equality.
--
-- ── AND THEY HOLD AT EVERY DIMENSION, THROUGH THE ADDRESS ───────────────────
-- `Reasoning` takes one `Category`, which reaches 1-cells up to 2-cells and no
-- further. `Reasoning°` below takes an `Everywhere Category` and a bound
-- telescope instead, so the same suite is available at every dimension — by
-- module application, with nothing restated. It is an ADDITIONAL entry point
-- and never a re-parameterization of the first: the certification is strictly
-- stronger than the structure, and `Set`-level structures provably fail it.
-- The section over `Reasoning°` carries the argument.
--
-- ── WHEN TO REACH FOR A SOLVER INSTEAD ──────────────────────────────────────
-- These combinators are for proofs a solver cannot yet take. The standing rule
-- is solver-first: a chain that is pure reassociation and unit-cancellation is
-- exactly what a free-category solver decides, and once that solver exists such
-- chains should move to it rather than accumulate here.
------------------------------------------------------------------------------

module Gandr.Category.Reasoning where

open import Gandr.Graph
  using (∞Graph)
  using (ϵ°)
  using (δ°)
  using (Disc)
  using (⋆)
  using (_▸ᵈ_⇴_)
  using (Everywhere)
  using (at°)
open import Gandr.Setoid
  using (bundle)
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

import Relation.Binary.Bundles as Bundles
open import Relation.Binary.PropositionalEquality
  using (_≡_)
  using (refl)
  using (trans)

-- The stdlib reasoning vocabulary, re-exported so a consumer opens ONE module.
-- `begin⟨ S ⟩` names the hom-setoid a chain runs in; the step and terminator
-- syntax recover it by unification.
open import Relation.Binary.Reasoning.MultiSetoid public

module Reasoning {ℓ} {B : ∞Graph ℓ} (ℬ : Category B) where

  -- The hom-setoid at a pair of 0-cells, as a stdlib bundle: 1-cells `a ⇴ b`
  -- under the 2-cell equivalence the category supplies over them. This is what
  -- `begin⟨_⟩` takes.
  homᵇ : (a b : B .ϵ°) → Bundles.Setoid ℓ ℓ
  homᵇ a b = bundle (Category.homˢ ℬ a b)

  -- ══════════════════════════════════════════════════════════════════════════
  -- Whiskering: the one-sided congruences of composition. Whiskering is
  -- horizontal composition against an identity 2-cell, which is the only
  -- non-mechanical step in reading this suite off its propositional-equality
  -- template.
  -- ══════════════════════════════════════════════════════════════════════════

  -- Rewrite the LEFT factor, holding the right factor fixed.
  seqˡ* : ∀ {a b c} {f f′ : B .δ° a b .ϵ°}
    → (g : B .δ° b c .ϵ°)
    → B .δ° a b .δ° f f′ .ϵ°
    → B .δ° a c .δ° (ℬ .seq₀ f g) (ℬ .seq₀ f′ g) .ϵ°
  seqˡ* g p = ℬ .seq↕ p (ℬ .idn₁ g)

  -- Rewrite the RIGHT factor, holding the left factor fixed.
  seqʳ* : ∀ {a b c} {g g′ : B .δ° b c .ϵ°}
    → (f : B .δ° a b .ϵ°)
    → B .δ° b c .δ° g g′ .ϵ°
    → B .δ° a c .δ° (ℬ .seq₀ f g) (ℬ .seq₀ f g′) .ϵ°
  seqʳ* f q = ℬ .seq↕ (ℬ .idn₁ f) q

  -- ══════════════════════════════════════════════════════════════════════════
  -- Reassociation ladders: pulls and pushes. A pull reassociates and THEN
  -- rewrites the exposed pair; a push rewrites first and then reassociates.
  -- ══════════════════════════════════════════════════════════════════════════

  -- Reassociate `a ⋆ (b ⋆ f)` to `(a ⋆ b) ⋆ f`, then rewrite `a ⋆ b` to `c`.
  pullˡ : ∀ {w x y z}
    → {a : B .δ° w x .ϵ°} {b : B .δ° x y .ϵ°} {c : B .δ° w y .ϵ°} {f : B .δ° y z .ϵ°}
    → B .δ° w y .δ° (ℬ .seq₀ a b) c .ϵ°
    → B .δ° w z .δ° (ℬ .seq₀ a (ℬ .seq₀ b f)) (ℬ .seq₀ c f) .ϵ°
  pullˡ {a} {b} {f} η =
    ℬ .seq₁
      (ℬ .inv₁ (ℬ .mon-α a f b))
      (seqˡ* f η)

  -- Reassociate `(f ⋆ a) ⋆ b` to `f ⋆ (a ⋆ b)`, then rewrite `a ⋆ b` to `c`.
  pullʳ : ∀ {v w x y}
    → {f : B .δ° v w .ϵ°} {a : B .δ° w x .ϵ°} {b : B .δ° x y .ϵ°} {c : B .δ° w y .ϵ°}
    → B .δ° w y .δ° (ℬ .seq₀ a b) c .ϵ°
    → B .δ° v y .δ° (ℬ .seq₀ (ℬ .seq₀ f a) b) (ℬ .seq₀ f c) .ϵ°
  pullʳ {f} {a} {b} η =
    ℬ .seq₁
      (ℬ .mon-α f b a)
      (seqʳ* f η)

  -- Rewrite `c` to `a ⋆ b`, then reassociate — the leading handedness.
  pushˡ : ∀ {w x y z}
    → {a : B .δ° w x .ϵ°} {b : B .δ° x y .ϵ°} {c : B .δ° w y .ϵ°} {f : B .δ° y z .ϵ°}
    → B .δ° w y .δ° c (ℬ .seq₀ a b) .ϵ°
    → B .δ° w z .δ° (ℬ .seq₀ c f) (ℬ .seq₀ a (ℬ .seq₀ b f)) .ϵ°
  pushˡ {a} {b} {f} η =
    ℬ .seq₁
      (seqˡ* f η)
      (ℬ .mon-α a f b)

  -- Rewrite `c` to `a ⋆ b`, then reassociate — the trailing handedness.
  pushʳ : ∀ {v w x y}
    → {f : B .δ° v w .ϵ°} {a : B .δ° w x .ϵ°} {b : B .δ° x y .ϵ°} {c : B .δ° w y .ϵ°}
    → B .δ° w y .δ° c (ℬ .seq₀ a b) .ϵ°
    → B .δ° v y .δ° (ℬ .seq₀ f c) (ℬ .seq₀ (ℬ .seq₀ f a) b) .ϵ°
  pushʳ {f} {a} {b} η =
    ℬ .seq₁
      (seqʳ* f η)
      (ℬ .inv₁ (ℬ .mon-α f b a))

  -- ══════════════════════════════════════════════════════════════════════════
  -- Identity introduction and elimination. Stated over a general `a ⇒ idn₀`
  -- witness rather than a literal identity, so the family applies wherever a
  -- cell is merely PROVABLY an identity.
  -- ══════════════════════════════════════════════════════════════════════════

  -- Cancel an identity on the left.
  elimˡ : ∀ {x y}
    → {a : B .δ° x x .ϵ°} {f : B .δ° x y .ϵ°}
    → B .δ° x x .δ° a (ℬ .idn₀ x) .ϵ°
    → B .δ° x y .δ° (ℬ .seq₀ a f) f .ϵ°
  elimˡ {f} η =
    ℬ .seq₁
      (seqˡ* f η)
      (ℬ .mon-λ f)

  -- Cancel an identity on the right.
  elimʳ : ∀ {x y}
    → {a : B .δ° y y .ϵ°} {f : B .δ° x y .ϵ°}
    → B .δ° y y .δ° a (ℬ .idn₀ y) .ϵ°
    → B .δ° x y .δ° (ℬ .seq₀ f a) f .ϵ°
  elimʳ {f} η =
    ℬ .seq₁
      (seqʳ* f η)
      (ℬ .mon-ρ f)

  -- Insert an identity on the left.
  introˡ : ∀ {x y}
    → {a : B .δ° x x .ϵ°} {f : B .δ° x y .ϵ°}
    → B .δ° x x .δ° a (ℬ .idn₀ x) .ϵ°
    → B .δ° x y .δ° f (ℬ .seq₀ a f) .ϵ°
  introˡ η = ℬ .inv₁ (elimˡ η)

  -- Insert an identity on the right.
  introʳ : ∀ {x y}
    → {a : B .δ° y y .ϵ°} {f : B .δ° x y .ϵ°}
    → B .δ° y y .δ° a (ℬ .idn₀ y) .ϵ°
    → B .δ° x y .δ° f (ℬ .seq₀ f a) .ϵ°
  introʳ η = ℬ .inv₁ (elimʳ η)

  -- Insert an identity one factor deep, on the right leg.
  intro-center : ∀ {w x y}
    → {a : B .δ° x x .ϵ°} {f : B .δ° w x .ϵ°} {g : B .δ° x y .ϵ°}
    → B .δ° x x .δ° a (ℬ .idn₀ x) .ϵ°
    → B .δ° w y .δ° (ℬ .seq₀ f g) (ℬ .seq₀ f (ℬ .seq₀ a g)) .ϵ°
  intro-center {f} η = seqʳ* f (introˡ η)

  -- Cancel an identity one factor deep, on the right leg.
  elim-center : ∀ {w x y}
    → {a : B .δ° x x .ϵ°} {f : B .δ° w x .ϵ°} {g : B .δ° x y .ϵ°}
    → B .δ° x x .δ° a (ℬ .idn₀ x) .ϵ°
    → B .δ° w y .δ° (ℬ .seq₀ f (ℬ .seq₀ a g)) (ℬ .seq₀ f g) .ϵ°
  elim-center {f} η = seqʳ* f (elimˡ η)

  -- ══════════════════════════════════════════════════════════════════════════
  -- Cancellers: an adjacent pair that composes to an identity. Stated over a
  -- general `h ⋆ i ⇒ idn₀` witness so the family is structure-generic — a
  -- groupoid feeds its own cancellation cells straight in, and nothing here
  -- presumes invertibility.
  -- ══════════════════════════════════════════════════════════════════════════

  -- Cancel a trailing pair.
  cancelʳ : ∀ {w x y}
    → {f : B .δ° w x .ϵ°} {h : B .δ° x y .ϵ°} {i : B .δ° y x .ϵ°}
    → B .δ° x x .δ° (ℬ .seq₀ h i) (ℬ .idn₀ x) .ϵ°
    → B .δ° w x .δ° (ℬ .seq₀ (ℬ .seq₀ f h) i) f .ϵ°
  cancelʳ {f} η =
    ℬ .seq₁
      (pullʳ η)
      (ℬ .mon-ρ f)

  -- Cancel a leading pair.
  cancelˡ : ∀ {w x y}
    → {h : B .δ° w x .ϵ°} {i : B .δ° x w .ϵ°} {f : B .δ° w y .ϵ°}
    → B .δ° w w .δ° (ℬ .seq₀ h i) (ℬ .idn₀ w) .ϵ°
    → B .δ° w y .δ° (ℬ .seq₀ h (ℬ .seq₀ i f)) f .ϵ°
  cancelˡ {h} {i} {f} η =
    ℬ .seq₁
      (pullˡ η)
      (ℬ .mon-λ f)

  -- Insert a trailing pair.
  insertʳ : ∀ {w x y}
    → {f : B .δ° w x .ϵ°} {h : B .δ° x y .ϵ°} {i : B .δ° y x .ϵ°}
    → B .δ° x x .δ° (ℬ .seq₀ h i) (ℬ .idn₀ x) .ϵ°
    → B .δ° w x .δ° f (ℬ .seq₀ (ℬ .seq₀ f h) i) .ϵ°
  insertʳ η = ℬ .inv₁ (cancelʳ η)

  -- Insert a leading pair.
  insertˡ : ∀ {w x y}
    → {h : B .δ° w x .ϵ°} {i : B .δ° x w .ϵ°} {f : B .δ° w y .ϵ°}
    → B .δ° w w .δ° (ℬ .seq₀ h i) (ℬ .idn₀ w) .ϵ°
    → B .δ° w y .δ° f (ℬ .seq₀ h (ℬ .seq₀ i f)) .ϵ°
  insertˡ η = ℬ .inv₁ (cancelˡ η)

  -- Cancel a pair sitting between two factors.
  cancelInner : ∀ {v w x y}
    → {f : B .δ° v w .ϵ°} {h : B .δ° w x .ϵ°} {i : B .δ° x w .ϵ°} {g : B .δ° w y .ϵ°}
    → B .δ° w w .δ° (ℬ .seq₀ h i) (ℬ .idn₀ w) .ϵ°
    → B .δ° v y .δ° (ℬ .seq₀ (ℬ .seq₀ f h) (ℬ .seq₀ i g)) (ℬ .seq₀ f g) .ϵ°
  cancelInner {f} {h} {i} {g} η =
    ℬ .seq₁
      (ℬ .inv₁ (ℬ .mon-α (ℬ .seq₀ f h) g i))
      (seqˡ* g (cancelʳ η))

  -- Insert a pair between two factors.
  insertInner : ∀ {v w x y}
    → {f : B .δ° v w .ϵ°} {h : B .δ° w x .ϵ°} {i : B .δ° x w .ϵ°} {g : B .δ° w y .ϵ°}
    → B .δ° w w .δ° (ℬ .seq₀ h i) (ℬ .idn₀ w) .ϵ°
    → B .δ° v y .δ° (ℬ .seq₀ f g) (ℬ .seq₀ (ℬ .seq₀ f h) (ℬ .seq₀ i g)) .ϵ°
  insertInner η = ℬ .inv₁ (cancelInner η)

  -- ══════════════════════════════════════════════════════════════════════════
  -- Extensions: carry a commuting square through a composition context. These
  -- are what make a naturality square usable where it sits rather than only at
  -- the boundary it was stated at.
  -- ══════════════════════════════════════════════════════════════════════════

  -- Extend a square by a common factor to the right of both legs.
  extendˡ : ∀ {w x y z}
    → {f : B .δ° w x .ϵ°} {h : B .δ° x y .ϵ°} {g : B .δ° w x .ϵ°} {i : B .δ° x y .ϵ°}
    → {a : B .δ° y z .ϵ°}
    → B .δ° w y .δ° (ℬ .seq₀ f h) (ℬ .seq₀ g i) .ϵ°
    → B .δ° w z .δ° (ℬ .seq₀ f (ℬ .seq₀ h a)) (ℬ .seq₀ g (ℬ .seq₀ i a)) .ϵ°
  extendˡ {f} {h} {g} {i} {a} η =
    ℬ .seq₁
      (ℬ .inv₁ (ℬ .mon-α f a h))
      (ℬ .seq₁
        (seqˡ* a η)
        (ℬ .mon-α g a i))

  -- Extend a square by a common factor to the left of both legs.
  extendʳ : ∀ {v w x y}
    → {a : B .δ° v w .ϵ°}
    → {f : B .δ° w x .ϵ°} {h : B .δ° x y .ϵ°} {g : B .δ° w x .ϵ°} {i : B .δ° x y .ϵ°}
    → B .δ° w y .δ° (ℬ .seq₀ f h) (ℬ .seq₀ g i) .ϵ°
    → B .δ° v y .δ° (ℬ .seq₀ (ℬ .seq₀ a f) h) (ℬ .seq₀ (ℬ .seq₀ a g) i) .ϵ°
  extendʳ {a} {f} {h} {g} {i} η =
    ℬ .seq₁
      (ℬ .mon-α a h f)
      (ℬ .seq₁
        (seqʳ* a η)
        (ℬ .inv₁ (ℬ .mon-α a i g)))

  -- Paste two squares along their shared edge — the vertical composition of
  -- commuting squares, and the one combinator the naturality proofs in
  -- `Gandr.Category.Functor` were each open-coding as a four-step chain.
  --
  --        p₀        q₀
  --   a₀ ─────→ b₀ ─────→ c₀
  --   │    □₀   │    □₁   │
  -- u │       v │       w │
  --   ↓         ↓         ↓
  --   a₁ ─────→ b₁ ─────→ c₁
  --        p₁        q₁
  --
  -- Both squares are stated in the DIAGRAMMATIC direction the tree composes in,
  -- so each hypothesis reads "across then down ≈ down then across".
  glue : ∀ {a₀ a₁ b₀ b₁ c₀ c₁}
    → {p₀ : B .δ° a₀ b₀ .ϵ°} {q₀ : B .δ° b₀ c₀ .ϵ°}
    → {u : B .δ° a₀ a₁ .ϵ°} {v : B .δ° b₀ b₁ .ϵ°} {w : B .δ° c₀ c₁ .ϵ°}
    → {p₁ : B .δ° a₁ b₁ .ϵ°} {q₁ : B .δ° b₁ c₁ .ϵ°}
    → B .δ° a₀ b₁ .δ° (ℬ .seq₀ p₀ v) (ℬ .seq₀ u p₁) .ϵ°
    → B .δ° b₀ c₁ .δ° (ℬ .seq₀ q₀ w) (ℬ .seq₀ v q₁) .ϵ°
    → B .δ° a₀ c₁ .δ° (ℬ .seq₀ (ℬ .seq₀ p₀ q₀) w) (ℬ .seq₀ u (ℬ .seq₀ p₁ q₁)) .ϵ°
  glue {a₀} {c₁} {p₀} {q₀} {u} {v} {w} {p₁} {q₁} □₀ □₁ =
    begin⟨ homᵇ a₀ c₁ ⟩
      ℬ .seq₀ (ℬ .seq₀ p₀ q₀) w  ≈⟨ pullʳ □₁ ⟩
      ℬ .seq₀ p₀ (ℬ .seq₀ v q₁)  ≈⟨ pullˡ □₀ ⟩
      ℬ .seq₀ (ℬ .seq₀ u p₁) q₁  ≈⟨ ℬ .mon-α u q₁ p₁ ⟩
      ℬ .seq₀ u (ℬ .seq₀ p₁ q₁)  ∎

  -- Extend a square on both sides at once.
  extend² : ∀ {u v w x y}
    → {a : B .δ° u v .ϵ°}
    → {f : B .δ° v w .ϵ°} {h : B .δ° w x .ϵ°} {g : B .δ° v w .ϵ°} {i : B .δ° w x .ϵ°}
    → {b : B .δ° x y .ϵ°}
    → B .δ° v x .δ° (ℬ .seq₀ f h) (ℬ .seq₀ g i) .ϵ°
    → B .δ° u y .δ° (ℬ .seq₀ (ℬ .seq₀ a f) (ℬ .seq₀ h b)) (ℬ .seq₀ (ℬ .seq₀ a g) (ℬ .seq₀ i b)) .ϵ°
  extend² η = extendʳ (extendˡ η)

  -- ══════════════════════════════════════════════════════════════════════════
  -- Centre operations and higher reassociation.
  -- ══════════════════════════════════════════════════════════════════════════

  -- Rewrite the middle pair of a 4-fold `(f ⋆ g) ⋆ (h ⋆ i)`, landing as
  -- `f ⋆ (a ⋆ i)`.
  center : ∀ {v w x y z}
    → {f : B .δ° v w .ϵ°} {g : B .δ° w x .ϵ°} {h : B .δ° x y .ϵ°} {i : B .δ° y z .ϵ°}
    → {a : B .δ° w y .ϵ°}
    → B .δ° w y .δ° (ℬ .seq₀ g h) a .ϵ°
    → B .δ° v z .δ° (ℬ .seq₀ (ℬ .seq₀ f g) (ℬ .seq₀ h i)) (ℬ .seq₀ f (ℬ .seq₀ a i)) .ϵ°
  center {f} {g} {h} {i} η =
    ℬ .seq₁
      (ℬ .mon-α f (ℬ .seq₀ h i) g)
      (seqʳ* f (pullˡ η))

  -- The dual: rewrite the two ends rather than the middle.
  center⁻¹ : ∀ {v w x y z}
    → {f : B .δ° v w .ϵ°} {g : B .δ° w x .ϵ°} {h : B .δ° x y .ϵ°} {i : B .δ° y z .ϵ°}
    → {a : B .δ° v x .ϵ°} {b : B .δ° x z .ϵ°}
    → B .δ° v x .δ° (ℬ .seq₀ f g) a .ϵ°
    → B .δ° x z .δ° (ℬ .seq₀ h i) b .ϵ°
    → B .δ° v z .δ° (ℬ .seq₀ f (ℬ .seq₀ (ℬ .seq₀ g h) i)) (ℬ .seq₀ a b) .ϵ°
  center⁻¹ {f} {g} {h} {i} η₁ η₂ =
    ℬ .seq₁
      (seqʳ* f (ℬ .mon-α g i h))
      (ℬ .seq₁
        (ℬ .inv₁ (ℬ .mon-α f (ℬ .seq₀ h i) g))
        (ℬ .seq↕ η₁ η₂))

  -- Reassociate a fully-left-nested 4-fold to fully-right-nested.
  assoc² : ∀ {v w x y z}
    → (f : B .δ° v w .ϵ°) (g : B .δ° w x .ϵ°) (h : B .δ° x y .ϵ°) (k : B .δ° y z .ϵ°)
    → B .δ° v z .δ° (ℬ .seq₀ (ℬ .seq₀ (ℬ .seq₀ f g) h) k) (ℬ .seq₀ f (ℬ .seq₀ g (ℬ .seq₀ h k))) .ϵ°
  assoc² f g h k =
    ℬ .seq₁
      (ℬ .mon-α (ℬ .seq₀ f g) k h)
      (ℬ .mon-α f (ℬ .seq₀ h k) g)

  -- The reverse reassociation.
  assoc²⁻¹ : ∀ {v w x y z}
    → (f : B .δ° v w .ϵ°) (g : B .δ° w x .ϵ°) (h : B .δ° x y .ϵ°) (k : B .δ° y z .ϵ°)
    → B .δ° v z .δ° (ℬ .seq₀ f (ℬ .seq₀ g (ℬ .seq₀ h k))) (ℬ .seq₀ (ℬ .seq₀ (ℬ .seq₀ f g) h) k) .ϵ°
  assoc²⁻¹ f g h k = ℬ .inv₁ (assoc² f g h k)

-- ══════════════════════════════════════════════════════════════════════════════
-- THE SAME SUITE, READ AT AN ADDRESS.
--
-- `Reasoning` above takes ONE `Category`, so it reasons about 1-cells up to
-- 2-cells and stops there: a step one dimension further needs a `Category` on
-- `B .δ° a b`, and a bare `Category B` does not supply one. `Everywhere
-- Category` supplies them all, `at°` reads off the one at a bound telescope, and
-- that module application is the whole of "the combinators hold at every
-- dimension". Nothing is restated, and nothing is proved twice.
--
-- ── AN ADDITIONAL DOOR, NEVER A RE-PARAMETERIZATION ──────────────────────────
-- The certification is a STRICTLY STRONGER hypothesis than the structure, and
-- the layer's own main consumer provably fails it: every `Set`-level structure
-- in this tree presents through `𝔾.≡°`, which stops with `𝟘` above its 1-cells,
-- and `ℂ.≡°-not-everywhere` refutes `Everywhere Category` there. So the
-- carrier-level `Reasoning` stays the primitive. A suite re-parameterized by the
-- certification would be a suite the `Set`-level structures could not use.
--
-- At `⋆` the two doors open on the same room, definitionally — `Gandr.Graph`'s
-- `at°-⋆` — so a consumer that already reasons at the carrier is unchanged when
-- it is read as reasoning at the empty telescope.
-- ══════════════════════════════════════════════════════════════════════════════

module Reasoning° {ℓ} {Ξ : ∞Graph ℓ} (𝒞 : Everywhere Category Ξ) (Θ : Disc Ξ)
  = Reasoning (at° 𝒞 Θ)

-- ══════════════════════════════════════════════════════════════════════════════
-- WORKED: ONE COMBINATOR, TWO ADDRESSES OF ONE TOWER.
--
-- `ℂ.Id°` certifies the identity-type tower as a category at every dimension, so
-- `glue` is available at every address of it. Neither reading below is proved:
-- each IS the generic combinator, instantiated. That is the claim the telescope
-- was adopted for, exercised against a combinator the tree actually uses rather
-- than against a spike.
--
-- These are computational pins; they move to the role split's `Examples` module
-- when it lands.
-- ══════════════════════════════════════════════════════════════════════════════

-- At `⋆`: the four-fold associativity of `trans`, in ordinary propositional
-- vocabulary. The statement mentions no dimension, no telescope and no address —
-- it is what a consumer holding only propositional equality would have written
-- by hand, and stdlib does not carry it — and the proof is the generic
-- combinator, applied. Nothing is transported and nothing is restated.
--
-- ONE ERGONOMIC CAVEAT, WHICH IS NOT THE ADDRESS'S DOING. A combinator whose
-- CELLS are implicit cannot be restated this way without passing them: at `⋆`
-- the cell type is `trans p q ≡ …`, and `trans` is a defined function, so
-- unification cannot recover `p` and `q` from it. That is the ordinary
-- non-injectivity of a defined symbol and it bites a hand-written restatement
-- exactly as hard; `assoc²` takes its cells explicitly and so is free of it.
trans-assoc² : ∀ {ℓ} {A : Set ℓ} {v w x y z : A}
  → (p : v ≡ w) (q : w ≡ x) (r : x ≡ y) (s : y ≡ z)
  → trans (trans (trans p q) r) s ≡ trans p (trans q (trans r s))
trans-assoc² {A} = Reasoning°.assoc² (ℂ.Id° A) ⋆

-- And one dimension up. The structure at `⋆ ▸ᵈ x ⇴ y` is the structure of the
-- tower one dimension up read at `⋆`, on the nose — so the same pasting for
-- 2-PATHS is `glue-path` at the path type, and the dimension is a parameter
-- rather than a restatement. Before the address existed this was not statable at
-- all: the type of a cell there is a projection spine, and no implicit can
-- reconstruct it.
Id°-↑ : ∀ {ℓ} {A : Set ℓ} {x y : A}
  → at° (ℂ.Id° A) (⋆ ▸ᵈ x ⇴ y) ≡ at° (ℂ.Id° (x ≡ y)) ⋆
Id°-↑ = refl
