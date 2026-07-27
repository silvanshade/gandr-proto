{-# OPTIONS --safe --guardedness --without-K --hidden-argument-puns #-}

------------------------------------------------------------------------------
-- Gandr.Category.Instances — SETOID as a named object, and the enrichment.
--
-- Every result in this tree is a result about setoids. Until now that was a
-- claim made in prose, in module headers, about an ambient that was never
-- constructed. This module constructs it: `SETOID` is a `Category` like any
-- other, so a statement that quantifies over categories genuinely has the
-- ambient in range, and a statement about "the full subcategory of SETOID
-- equivalent to decidable-equality SET" has something to be a subcategory OF.
--
-- ── OBJECTS ARE STDLIB BUNDLES, NOT THIS TREE'S `Setoid` RECORD ─────────────
-- `Gandr.Setoid`'s record is structure ON an ∞-graph; a stdlib `Setoid` is the
-- packaged pair. `Gandr.Setoid.bundle` converts, losing nothing, because
-- stdlib's `IsEquivalence` is exactly as lawless as ours and never truncates.
-- Taking the bundle as the object is what makes this category the ambient the
-- tree ACTUALLY reasons in: every `Reasoning.MultiSetoid` chain in this tree
-- runs over a bundle. Taking the Σ of a carrier ∞-graph with a `Setoid` on it
-- would instead give objects carrying an entire tower of higher cells that
-- SETOID then discards above dimension 1 — structure asserted and immediately
-- thrown away.
--
-- ── THE UNIVERSE STEP, AND WHY THE HOMS ARE PURPOSE-BUILT ───────────────────
-- `Setoid ℓ ℓ` lives in `Set (lsuc ℓ)`, so the carrier is an `∞Graph (lsuc ℓ)`
-- and `Category` — which is homogeneous in the level — needs EVERY cell above
-- it in `Set (lsuc ℓ)` too. stdlib's `Func` and its pointwise equality land in
-- `Set ℓ`, one level short. `Fnc` and `Htpy` below are those two notions on the
-- nose, declared at the carrier's level so that no `Lift`/`lower` pair appears
-- at any use site. The content is stdlib's; only the universe is ours.
--
-- ── WHAT THE COBOUNDARY IS ABOVE DIMENSION 2, AND WHY ───────────────────────
-- SETOID is a 1-CATEGORY. In this tree a `Category`'s 2-cells ARE the equality
-- on its 1-cells, so SETOID's 2-cells are pointwise equivalence — a `Htpy` —
-- and above that there is nothing left to track. The carrier is therefore
-- CODISCRETE from dimension 3 up: exactly one cell over every parallel pair of
-- homotopies, which is the statement that we draw no further distinctions.
--
-- `disc` would be the wrong choice, and not merely a weaker one: it puts NO
-- cell over a parallel pair, so a homotopy would fail to be related even to
-- itself, and the carrier would deny a reflexivity it has no reason to deny.
--
-- ── SETOID IS STRICT, AND THAT LOCATES WHERE WEAKNESS ENTERS ────────────────
-- All three coherence fields — `mon-λ`, `mon-ρ`, `mon-α` — are REFLEXIVITY
-- here. Composition of setoid maps is composition of functions on carriers and
-- composition of congruence proofs alongside it, and both are strictly unital
-- and associative, so the two sides of each coherence are definitionally the
-- same map. `Gandr.Category` is weak by default because a general carrier
-- forces it to be; SETOID shows that the weakness is not in the ambient. It
-- arrives with the structures built OVER the ambient, which is where this tree
-- wants it and is why the coherences are witnessed cells rather than equations.
--
-- ── ONE THING DELIBERATELY ABSENT: THE COVARIANT HOM FUNCTOR ────────────────
-- There is no `Map 𝒞 SETOID` here. `Map` is homogeneous in the universe level
-- and `Setoids` sits one level above any `Ξ : ∞Graph ℓ`, so a representable
-- `𝒞 → SETOID` is not expressible without a level-lifting operation on
-- ∞-graphs. That is not a gap being papered over. The representable this tree
-- uses is `Gandr.Profunctor.hom-pro`, and Yoneda is proved against it in the
-- VALUE setoids (`Gandr.Profunctor.Yoneda`); the `Set`-valued presheaf form is
-- precisely what a tree without SET does not have. `homᵒ` below supplies the
-- object part, which is all the enrichment statement needs.
--
-- ── WHERE THE OTHER INSTANCES LIVE, AND WHAT TO DO IF YOU WANT THEM HERE ────
-- `ℂ.Id`, `ℂ.𝟘`, `ℂ.𝟙`, `ℂ.!`, `ℂ._×_`, `ℂ.fst`, `ℂ.snd` and `ℂ.⟨_,_⟩` stay in
-- `Gandr.Category`. They are definitional companions of `𝔾`'s objects — each is
-- read off its carrier by copattern matching and nothing is constructed —
-- whereas the instances here are built. This module holds the instances that
-- have content, and that is the whole of the split.
--
-- **If this module ever needs them, MIGRATE them; do not re-derive them.** The
-- split above is a judgement call about where a definition reads best, not a
-- claim that the two sets are different in kind, so the moment a consumer wants
-- `ℂ.Id` alongside `SETOID` the right move is to move the definition and its
-- header note across and update `Gandr.Category`'s importers — not to write a
-- second `Id` here. A duplicate would be definitionally equal to the original
-- and therefore invisible to the gate, which is exactly the kind of drift that
-- is cheap to prevent now and expensive to find later. The same rule applies in
-- reverse: nothing here should be re-derived in `Gandr.Category`.
------------------------------------------------------------------------------

module Gandr.Category.Instances where

open import Gandr.Graph
  using (∞Graph)
  using (ϵ°)
  using (δ°)
  using (module 𝔾)
open import Gandr.Setoid
  using (bundle)
open import Gandr.Category
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

open import Level
  renaming (suc to lsuc)
open import Relation.Binary.Bundles
  using (Setoid)

-- The unique cell of the terminal ∞-graph, which is every cell of the
-- indiscrete category at the bottom of this file.
private
  module 𝕊 where
    open import Data.Unit.Polymorphic public
      using (tt)
open 𝕊
  using (tt)

------------------------------------------------------------------------------
-- The vocabulary a setoid object is spoken in. Both are projections, named so
-- the definitions below read as mathematics rather than as record access.
------------------------------------------------------------------------------

-- The elements of a setoid object.
∣_∣ : ∀ {ℓ} → Setoid ℓ ℓ → Set ℓ
∣ X ∣ = Setoid.Carrier X

-- Its equivalence, with the object named first so a chain says which setoid it
-- is running in.
infix 4 _⊢_≈_
_⊢_≈_ : ∀ {ℓ} (X : Setoid ℓ ℓ) → ∣ X ∣ → ∣ X ∣ → Set ℓ
X ⊢ x ≈ y = Setoid._≈_ X x y

------------------------------------------------------------------------------
-- The 1-cells and 2-cells. Both records are declared one universe above their
-- fields, for the reason in the header: `Category` is homogeneous in the level
-- and the objects already cost a universe.
------------------------------------------------------------------------------

-- A morphism of setoid objects: a map on elements together with the proof that
-- it respects the equivalences. This is stdlib's `Func`, at the level the
-- carrier needs.
record Fnc {ℓ} (X Y : Setoid ℓ ℓ) : Set (lsuc ℓ) where
  field
    -- the action on elements
    ap : ∣ X ∣ → ∣ Y ∣
    -- which is congruent: equivalent arguments have equivalent images
    ap* : ∀ {x x′}
      → X ⊢ x ≈ x′
      → Y ⊢ ap x ≈ ap x′
open Fnc public

-- A homotopy: two morphisms agreeing pointwise, in the TARGET's equivalence.
-- This is equality of setoid morphisms, and it is what SETOID's 2-cells are.
record Htpy {ℓ} {X Y : Setoid ℓ ℓ} (F G : Fnc X Y) : Set (lsuc ℓ) where
  field
    -- the witness at each element
    at : ∀ x
      → Y ⊢ F .ap x ≈ G .ap x
open Htpy public

------------------------------------------------------------------------------
-- The carrier, then the structure on it.
------------------------------------------------------------------------------

-- The carrier ∞-graph of SETOID: setoid objects, their morphisms, the
-- homotopies between those, and no distinctions above.
Setoids : ∀ {ℓ} → ∞Graph (lsuc ℓ)
Setoids {ℓ} .ϵ° = Setoid ℓ ℓ
Setoids .δ° X Y .ϵ° = Fnc X Y
Setoids .δ° X Y .δ° F G = 𝔾.codisc (Htpy F G)

-- SETOID itself: the ambient, as a named object.
SETOID : ∀ {ℓ} → Category (Setoids {ℓ})
-- The identity morphism acts as the identity on elements AND on proofs.
SETOID .idn₀ X .ap x = x
SETOID .idn₀ X .ap* p = p
-- Composition is diagrammatic on elements, and the congruences compose the
-- same way — which is why the coherences below are reflexive.
SETOID .seq₀ F G .ap x = G .ap (F .ap x)
SETOID .seq₀ F G .ap* p = G .ap* (F .ap* p)
-- The 2-cell equivalence is pointwise, so it is the TARGET setoid's own
-- equivalence, applied at each element.
SETOID .idn₁ {b = Y} F .at x = Setoid.refl Y
SETOID .seq₁ {b = Y} p q .at x = Setoid.trans Y (p .at x) (q .at x)
SETOID .inv₁ {b = Y} p .at x = Setoid.sym Y (p .at x)
-- Horizontal composition: carry the source homotopy through the second
-- morphism's congruence, then paste the second homotopy at the shifted point.
-- This is the only field with real content.
SETOID .seq↕ {c = Z} {f′ = F′} {g = G} α β .at x =
  Setoid.trans Z (G .ap* (α .at x)) (β .at (F′ .ap x))
-- The coherences, all three REFLEXIVE: each pair of composites is the same
-- morphism definitionally, on elements and on proofs alike.
SETOID .mon-λ {y = Y} F .at x = Setoid.refl Y
SETOID .mon-ρ {y = Y} F .at x = Setoid.refl Y
SETOID .mon-α {z = Z} F H G .at x = Setoid.refl Z

------------------------------------------------------------------------------
-- The enrichment, named.
------------------------------------------------------------------------------

-- Every hom of every category in this tree is an object of SETOID. That is the
-- whole content of "gandr's categories are SETOID-enriched": a `Category`'s
-- `idn₁`/`seq₁`/`inv₁` over a fixed hom are reflexivity, transitivity and
-- symmetry, `Gandr.Setoid` names that as a structure, and `bundle` presents it.
-- Nothing is constructed here — the enrichment was already there, unnamed.
homᵒ : ∀ {ℓ} {Ξ : ∞Graph ℓ} (𝒞 : Category Ξ) (a b : Ξ .ϵ°) → Setoid ℓ ℓ
homᵒ 𝒞 a b = bundle (Category.homˢ 𝒞 a b)

------------------------------------------------------------------------------
-- The indiscrete category, for contrast.
------------------------------------------------------------------------------

-- The indiscrete category on a `Set`: exactly one cell over every parallel
-- pair, at every dimension. It is the category-level image of `𝔾.codisc`, and
-- so the right adjoint to the 0-cells projection at this layer, exactly as
-- `𝔾.codisc` is one dimension down.
--
-- Every field is the unique cell, so every law holds for the reason a terminal
-- object makes every law hold — which is the point of naming it: it is the
-- category that proves nothing, and a result that holds only here holds
-- nowhere.
INDISC : ∀ {ℓ} (A : Set ℓ) → Category (𝔾.codisc A)
INDISC A .idn₀ _ = tt
INDISC A .seq₀ _ _ = tt
INDISC A .idn₁ _ = tt
INDISC A .seq₁ _ _ = tt
INDISC A .inv₁ _ = tt
INDISC A .seq↕ _ _ = tt
INDISC A .mon-λ _ = tt
INDISC A .mon-ρ _ = tt
INDISC A .mon-α _ _ _ = tt
