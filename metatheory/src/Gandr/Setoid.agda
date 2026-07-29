{-# OPTIONS --safe --guardedness --without-K --hidden-argument-puns #-}

------------------------------------------------------------------------------
-- Gandr.Setoid — reflexivity, transitivity and symmetry on a carrier's cells.
--
-- A `Setoid Ξ` equips the cells of an ∞-graph with `idnˢ`, `seqˢ` and `invˢ`:
-- Bishop's setoid, and the one-dimensional base of the structure tower. The
-- whole tree is the generalization of setoids to ∞-graphs, so this is where
-- that generalization starts.
--
-- ── WHY THIS RECORD CARRIES NO LAWS ─────────────────────────────────────────
-- Nothing relates the three operations. That is weakness by default, and it is
-- the honest floor: a `Setoid` is the LAWLESS proof-relevant equivalence, so it
-- needs no strictness mark and makes no claim it cannot back. Layers that want
-- laws state them one dimension up, as cells, where they can be witnessed
-- rather than asserted.
--
-- ── WHY THIS MATTERS FOR EVERY RESULT IN THIS TREE ──────────────────────────
-- This tree does not have SET. It has SETOID. Equality of cells is a structure
-- a carrier supplies, not a proposition the ambient theory hands over, and a
-- result proved here is a result about setoids unless it says otherwise. That
-- is a real restriction on what may be claimed — quotients need not be
-- effective, and a map need not respect an equivalence unless shown to.
-- Keeping the equivalence a named, projected structure rather than ambient
-- justification is what makes the restriction checkable instead of assumed.
--
-- Standing role: the repackaging target of `Gandr.Category`'s `hom`. A
-- Category's `idn₁`/`seq₁`/`inv₁` — reflexivity, transitivity and symmetry of
-- the 2-cells over a fixed hom — ARE a `Setoid` on that hom, so the hom-setoid
-- is read back off the Category by projection rather than rebuilt.
------------------------------------------------------------------------------

module Gandr.Setoid where

open import Gandr.Graph
  using (∞Graph)
  using (ϵ°)
  using (δ°)
  using (Everywhere)
  using (here°)
  using (up°)
  using (module 𝔾)

import Relation.Binary.Bundles as Bundles
open import Relation.Binary.Reasoning.MultiSetoid
  using (IsRelatedTo)
  using (step-≈-⟩)
  using (step-≈-⟨)
open import Data.Empty.Polymorphic
  using (⊥)
open import Relation.Binary.PropositionalEquality
  using (_≡_)
  using (refl)
  using (trans)
  using (sym)
  using (cong)

-- Reflexivity, transitivity and symmetry on the cells of `Ξ`, proof-relevantly
-- and without laws: the base the Category and Groupoid towers sit over.
record Setoid {ℓ} (Ξ : ∞Graph ℓ) : Set ℓ where
  field
    -- reflexivity: a chosen cell at every 0-cell
    idnˢ : ∀ a
      → Ξ .δ° a a .ϵ°
    -- transitivity: cells compose along a shared middle 0-cell
    seqˢ : ∀ {a b c}
      → Ξ .δ° a b .ϵ°
      → Ξ .δ° b c .ϵ°
      → Ξ .δ° a c .ϵ°
    -- symmetry: every cell reverses
    invˢ : ∀ {a b}
      → Ξ .δ° a b .ϵ°
      → Ξ .δ° b a .ϵ°
open Setoid public

-- ══════════════════════════════════════════════════════════════════════════════
-- The bridge to agda-stdlib, and why it is exact rather than approximate.
--
-- An ∞-graph's coboundary IS a relation on its cells: `Ξ .δ° x y .ϵ°` has the
-- shape `Rel (Ξ .ϵ°) ℓ` on the nose. And stdlib's `IsEquivalence` carries
-- `refl`/`sym`/`trans` and NO LAWS — so it is exactly as lawless as the record
-- above, and the bundle below loses nothing and adds nothing.
--
-- Two consequences worth naming, because they are why no bespoke reasoning
-- machinery is needed anywhere in this tree:
--
--   * Proof-relevance survives. `_≈_` is `Set`-valued and never truncated, so a
--     2-cell stays a cell rather than collapsing to a proposition.
--   * ONE bundle serves EVERY dimension. `Setoid` is parameterized by an
--     arbitrary ∞-graph, and any level's coboundary is itself an ∞-graph, so
--     the same function bundles 0-cells, homs, and every level above.
--
-- So stdlib's `begin`/`≈⟨_⟩`/`∎` applies directly. What stdlib does not supply
-- is the CATEGORICAL combinator suite — reassociation ladders, cancellers,
-- square extensions — which is `Gandr.Category.Reasoning`'s job.
-- ══════════════════════════════════════════════════════════════════════════════

-- THE DISCRETE SETOID ON THE IDENTITY TYPE: `refl`, `trans` and `sym`, over
-- `Gandr.Graph.≡°`. This is the `Setoid` every `Set`-level structure in this
-- tree presents, so `bundle (≡ˢ _)` is the bundle their reasoning chains run
-- in — the `Set`-level counterpart of reading a category's hom-setoid off
-- `homˢ`, and the reason a chain in a `Set`-level module is the same chain as
-- one in the profunctor layer rather than a second vocabulary.
≡ˢ : ∀ {ℓ} (A : Set ℓ) → Setoid (𝔾.≡° A)
≡ˢ A .idnˢ a = refl
≡ˢ A .seqˢ = trans
≡ˢ A .invˢ = sym

-- THE IDENTITY-TYPE TOWER IS A SETOID AT EVERY DIMENSION, which is the honest
-- form of "propositional equality is an equivalence relation": not once, but at
-- each level of the tower of equalities-between-equalities, by the same three
-- constructors each time. The corecursion is the statement — one dimension up
-- from `Id A` is `Id (x ≡ y)`, and the same instance serves it.
--
-- This is `Everywhere`'s reason to exist, stated at the smallest carrier that
-- has one. Its counterpart `≡ˢ` above is NOT an instance of it and cannot be —
-- `≡°` stops with `𝟘` above its 1-cells, so there is no 2-cell for `idnˢ` to
-- produce — and that is refuted below rather than asserted.
Idˢ : ∀ {ℓ} (A : Set ℓ) → Everywhere Setoid (𝔾.Id A)
Idˢ A .here° .idnˢ a = refl
Idˢ A .here° .seqˢ = trans
Idˢ A .here° .invˢ = sym
Idˢ A .up° x y = Idˢ (x ≡ y)

-- AND THE REFUTATION, so the distinction above is a theorem and `Everywhere` is
-- not a predicate everything satisfies. One inhabitant of `A` is enough: ask the
-- certification for its structure one dimension up from a point to itself, and
-- `idnˢ` there must produce a cell of `𝟘` out of `refl`.
--
-- Read the other way round, this is what `𝟘`-above buys. A carrier that filled
-- its trivial dimensions with `𝟙` would satisfy `Everywhere Setoid` for free and
-- the certification would say nothing at all — which is exactly the silent
-- discharge the truncation rule exists to prevent.
--
-- ── AND HERE IT GENUINELY SEPARATES, WHICH ONE DOCTRINE UP IT DOES NOT ───────
-- Worth stating side by side, because the two look alike and are not. `Setoid`
-- has content at dimensions 0 and 1, and `≡°` carries exactly those — so `≡ˢ`
-- above INHABITS the structure while the certification below is refuted, and
-- the gap between them is real here.
--
-- `Category` asks for one dimension more than `≡°` carries, so over `≡°` the
-- bare structure is already uninhabited (`Gandr.Category.ℂ.≡°-not-category`)
-- and the corresponding refutation separates nothing. Stated in the vocabulary
-- that makes the difference visible: `≡°`'s REGION is `Only⋆` for `Setoid` and
-- empty for `Category`, and `Gandr.Graph.At` is where a structure says which.
≡°-not-everywhere
  : ∀ {ℓ} {A : Set ℓ}
  → A
  → Everywhere Setoid (𝔾.≡° A)
  → ⊥
≡°-not-everywhere a S° = S° .up° a a .here° .idnˢ refl

-- The stdlib setoid bundle this structure presents. Both levels are `ℓ`: the
-- cells and the relation on them live in the same universe, since the relation
-- is just the carrier one dimension up.
bundle : ∀ {ℓ} {Ξ : ∞Graph ℓ} → Setoid Ξ → Bundles.Setoid ℓ ℓ
bundle {Ξ} S = record
  { Carrier = Ξ .ϵ°
  ; _≈_ = λ a b → Ξ .δ° a b .ϵ°
  ; isEquivalence = record
      { refl = λ {a} → S .idnˢ a
      ; sym = S .invˢ
      ; trans = S .seqˢ
      }
  }

-- ══════════════════════════════════════════════════════════════════════════════
-- THE REVERSE STEP, IN THE HOUSE NOTATION.
--
-- A chain step that runs the other way is stdlib's `x ≈⟨ p ⟨ y`, which flips
-- the CLOSING bracket to carry the direction. That is the one place the
-- vocabulary is hard to read: the mark is at the far end of the proof, which is
-- exactly where a multi-line step puts it out of sight, and an opening and a
-- closing angle that differ by orientation alone do not survive skimming.
--
-- So the direction is written where a reader looks for it — on the relation,
-- as an inverse. `≈⁻¹⟨ p ⟩` IS stdlib's backward step, re-syntaxed and nothing
-- more: no combinator is reimplemented, so the reasoning machinery, its
-- transitivity and its `IsRelatedTo` are the library's own. stdlib's own form
-- stays in scope and is not an error; this is the one the tree writes.
-- ══════════════════════════════════════════════════════════════════════════════

infixr 2 step-≈⁻¹
step-≈⁻¹
  : ∀ {a ℓ} {S : Bundles.Setoid a ℓ}
  → (x : Bundles.Setoid.Carrier S)
  → ∀ {y z}
  → IsRelatedTo S y z
  → Bundles.Setoid._≈_ S y x
  → IsRelatedTo S x z
step-≈⁻¹ = step-≈-⟨
syntax step-≈⁻¹ x yRz y≈x = x ≈⁻¹⟨ y≈x ⟩ yRz

-- ══════════════════════════════════════════════════════════════════════════════
-- A CONGRUENCE STEP, WITH THE REWRITTEN POSITION MARKED IN THE TERM.
--
-- Most steps of a `Set`-level chain rewrite UNDER something, so most of them
-- read `≈⟨ cong f p ⟩` and the `cong` is noise repeated down the whole chain.
-- Moving the head into the term line — `[ x ↦ f x ]· u` for what would
-- otherwise be written `f u` — takes it out of every step at once.
--
-- ── WHY THE HEAD IS A BINDER AND NOT A FUNCTION SLOT ────────────────────────
-- The head is written with its own bound variable rather than as a bare
-- function, because a rewrite is almost never at the argument of a one-place
-- head: `cap q _`, `spot₂ ∷ _` and `λ r → removal-comp r o` all need the hole
-- named, and a section or a lambda in that slot reads as apparatus rather than
-- as the term. Agda's `syntax` binds it directly — `syntax step-≈· (λ x → f) …`
-- — so `[ x ↦ f ]· u` IS `f[x := u]` with the rewritten position marked, and
-- nothing in the term is spelled twice.
--
-- ── WHY THE TERM SPLITS RATHER THAN THE STEP SHRINKING ──────────────────────
-- A step of the shape `x ≈[ f ]⟨ p ⟩` cannot be written at all: it must build
-- `x ≡ f v` out of `cong f p : f u ≡ f v`, so it needs `x ≡ f u`, and for an
-- abstract `x` that is unavailable — Agda rejects the definition with
-- `f _x != x`. Taking `f` and `u` SEPARATELY is what makes it typeable, since
-- the step's own index is then `f u` and there is nothing left to reconcile.
-- The chain still shows the whole term, just with its head marked.
--
-- ── WHY THE STEP IS `≈·⟨` AND NOT `≈⟨` ──────────────────────────────────────
-- Plain `≈⟨` there is genuinely AMBIGUOUS and Agda says so: the argument slot
-- `u` can itself swallow `u ≈⟨ p ⟩ rest`, so the same text parses two ways. A
-- marker of its own settles it, and it pays for itself — the dot now appears in
-- both halves of the device, so the marked head and its step read as one thing.
--
-- This is `Set`-level vocabulary and says so: `cong` is what makes it, so it is
-- available exactly where the bundle is `bundle (≡ˢ _)`. A general setoid needs
-- a congruence witness rather than a function, which is the structure's own
-- business — `Category`'s `seq↕` and its relatives.
-- ══════════════════════════════════════════════════════════════════════════════

infixr 2 step-≈·
step-≈·
  : ∀ {a b} {A : Set a} {B : Set b} {v : B} {z : A}
  → (f : B → A)
  → (u : B)
  → IsRelatedTo (bundle (≡ˢ A)) (f v) z
  → u ≡ v
  → IsRelatedTo (bundle (≡ˢ A)) (f u) z
step-≈· f u rest p = step-≈-⟩ (f u) rest (cong f p)
syntax step-≈· (λ x → f) u rest p = [ x ↦ f ]· u ≈·⟨ p ⟩ rest
