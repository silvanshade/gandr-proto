{-# OPTIONS --safe --without-K --hidden-argument-puns #-}

------------------------------------------------------------------------------
-- Gandr.Shape.Isotropy — do automorphisms act freely on maps of shapes?
--
-- This is the shape-layer half of the dualizability axiom a generalized Reedy
-- structure needs on the presheaf side: `f θ = f` with `θ` invertible forces
-- `θ = id`. The template is Hackney-Robertson-Yau, "On factorizations of
-- graphical maps", Homology, Homotopy and Applications 20(2):217-238, 2018,
-- doi:10.4310/HHA.2018.v20.n2.a11, read at arXiv:1705.08546v2 — Proposition
-- 3.12, whose proof runs through their Lemma 3.11 (a degree-raising map is
-- injective on each of the two half-edge sets) and their Lemma 3.9 (an
-- isomorphism is determined by its action on edges).
--
-- ── WHAT THIS TREE ALREADY HAS, AND WHY IT IS STRONGER ──────────────────────
-- The Lemma 3.9 analogue is `Gandr.Shape.Decidable.edge-determined`, and it is
-- STRICTLY STRONGER than the statement it is modelled on: the published lemma
-- is about isomorphisms, while `edge-determined` holds of ARBITRARY maps out of
-- a `Grounded` shape. The strengthening is not cleverness, it is the carrier:
-- a graphical map sends a vertex to a SUBGRAPH, which the edge action need not
-- determine, whereas a `GMap` sends a vertex to a VERTEX, which `Grounded`
-- pins through any incident edge.
--
-- ── AND WHY THE SOURCE'S TWO-SET SPLIT IS UNNECESSARY HERE ──────────────────
-- The published Lemma 3.9 states its third clause over `Edge_i ⨿ in` and
-- `Edge_i ⨿ out` SEPARATELY, because a contracting coface identifies an input
-- leg with an output leg, so a degree-raising map is injective on each half and
-- not on their union. No `GMap` can do that: `onE-end₀` and `onE-end₁` compare
-- ends through `smap actV (smap actI actO)`, which preserves the sum structure,
-- so a leg end goes to a leg end and a vertex end to a vertex end, ALWAYS. The
-- split has nothing to separate. That is a fact about this map layer and it
-- cuts both ways — see the closing section.
--
-- ── THE RESULT, AND THE HYPOTHESIS IT CANNOT DROP ───────────────────────────
-- `iv′` proves the axiom for maps whose edge action is injective, in three
-- lines: injectivity turns `f (θ e) = f e` into `θ e = e`, and edge
-- determination turns that into `θ ≐ id`.
--
-- `collapse-witnesses-iv′-fails` is the other half, and it is why the
-- hypothesis is stated rather than assumed away: two parallel through-wires
-- admit a non-identity automorphism swapping them, and the map collapsing both
-- onto the single wire of `edge` is fixed by it. So AXIOM (iv′) IS FALSE FOR
-- `GMap` AT LARGE. It is a property of a subcategory, and gandr has not yet
-- carved one out — which is the finding, not the failure.
--
-- ── WHAT THIS DOES NOT SETTLE, STATED SO IT IS NOT OVER-READ ────────────────
-- `GMap` is NOT the site's morphism class. The published Γ⁺ contains the
-- contracting cofaces, which join a leg to a leg and turn two edges into one
-- inner edge — and the incidence conditions above make that INEXPRESSIBLE as a
-- `GMap`. So the verdict here is about the carrier's homomorphisms, and
-- transporting it to the site needs the site's own presentation, which does not
-- exist. What would close it: a morphism class in which an end may change kind,
-- and its degree-raising subcategory.
------------------------------------------------------------------------------

module Gandr.Shape.Isotropy where

open import Gandr.Shape.Graph
  using (Shape)
  using (Ix)
  using (here)
  using (there)
  using (Vtx)
  using (Edg)
  using (verts)
  using (edges)
  using (end₀)
  using (end₁)
  using (idn)
  using (𝟚)
  using (edge)
open import Gandr.Shape.Decidable
  using (Tab)
  using ([])
  using (_∷_)
  using (app)
  using (GMap)
  using (onV)
  using (onE)
  using (onI)
  using (onO)
  using (actE)
  using (onE-end₀)
  using (onE-end₁)
  using (Grounded)
  using (_≐_)
  using (atE)
  using (edge-determined)

open import Data.List.Base
  using (List)
  using ([])
  using (_∷_)
open import Data.Sum.Base
  using (_⊎_)
  using (inj₁)
  using (inj₂)
  renaming (map to smap)
open import Level
  using (Level)
open import Relation.Binary.PropositionalEquality
  using (_≡_)
  using (refl)
  using (sym)
  using (trans)
  using (cong)
open import Relation.Nullary.Negation
  using (¬_)

------------------------------------------------------------------------------
-- TABLES, TWO OPERATIONS THE MAP LAYER NEEDED AND DID NOT HAVE. Reindexing a
-- table and the identity table, with the two computation rules that make them
-- usable — the table layer's own `map` and `id`, proved rather than postulated
-- because both are ordinary induction over data.
------------------------------------------------------------------------------

module _ {a b c} {A : Set a} {B : Set b} {C : Set c} where

  -- Post-composing a table with a function on its targets.
  mapTab : ∀ {xs : List A} → (B → C) → Tab B xs → Tab C xs
  mapTab f [] = []
  mapTab f (t ∷ ts) = f t ∷ mapTab f ts

  -- and reading it back agrees with post-composition, position by position
  app-mapTab
    : ∀ {xs : List A}
    → (f : B → C) (t : Tab B xs) (i : Ix xs)
    → app (mapTab f t) i ≡ f (app t i)
  app-mapTab f (t ∷ ts) here = refl
  app-mapTab f (t ∷ ts) (there i) = app-mapTab f ts i

module _ {a} {A : Set a} where

  -- THE IDENTITY TABLE: each position at itself. Written by shifting the tail
  -- rather than by tabulating a function, so that it is data all the way down.
  idTab : (xs : List A) → Tab (Ix xs) xs
  idTab [] = []
  idTab (x ∷ xs) = here ∷ mapTab there (idTab xs)

  -- and it really is the identity when read back
  app-idTab : (xs : List A) (i : Ix xs) → app (idTab xs) i ≡ i
  app-idTab (x ∷ xs) here = refl
  app-idTab (x ∷ xs) (there i) =
    trans (app-mapTab there (idTab xs) i) (cong there (app-idTab xs i))

------------------------------------------------------------------------------
-- THE IDENTITY MAP. Needed because the axiom's conclusion is an equation with
-- the identity on one side, and `_≐_` compares maps rather than actions.
------------------------------------------------------------------------------

module _ {ℓ} {Ob : Set ℓ} where

  private
    -- Every end is fixed by the identity actions, at each of the three kinds an
    -- end can be. This is the whole content of the identity map's incidence
    -- obligation, and it is one case split.
    smap-id
      : ∀ {Γ Δ} {S : Shape Ob Γ Δ}
      → (x : Vtx S ⊎ (Ix Γ ⊎ Ix Δ))
      → smap (app (idTab (verts S))) (smap (app (idTab Γ)) (app (idTab Δ))) x ≡ x
    smap-id (inj₁ v) = cong inj₁ (app-idTab _ v)
    smap-id (inj₂ (inj₁ i)) = cong (λ z → inj₂ (inj₁ z)) (app-idTab _ i)
    smap-id (inj₂ (inj₂ j)) = cong (λ z → inj₂ (inj₂ z)) (app-idTab _ j)

  -- The identity map of shapes: the identity table on each of the four
  -- actions, with incidence preserved because nothing moved.
  gid : ∀ {Γ Δ} (S : Shape Ob Γ Δ) → GMap S S
  gid S .onV = idTab (verts S)
  gid S .onE = idTab (edges S)
  gid S .onI = idTab _
  gid S .onO = idTab _
  -- Both obligations are the same two steps: the identity edge action moves the
  -- edge nowhere, and then the identity actions move its end nowhere.
  gid S .onE-end₀ e =
    trans (cong (end₀ S) (app-idTab (edges S) e)) (sym (smap-id (end₀ S e)))
  gid S .onE-end₁ e =
    trans (cong (end₁ S) (app-idTab (edges S) e)) (sym (smap-id (end₁ S e)))

------------------------------------------------------------------------------
-- THE AXIOM, AND THE HYPOTHESIS IT NEEDS.
--
-- `EdgeMono` is the analogue of the published Lemma 3.11's conclusion. It is
-- stated as a hypothesis rather than derived, because — unlike there — it does
-- NOT follow from any structural condition this map layer imposes. The
-- refutation below is what makes that a finding rather than an omission.
------------------------------------------------------------------------------

module _ {ℓ} {Ob : Set ℓ} where

  -- The edge action is injective.
  EdgeMono
    : ∀ {Γ Δ Γ′ Δ′} {S : Shape Ob Γ Δ} {T : Shape Ob Γ′ Δ′}
    → GMap S T
    → Set ℓ
  EdgeMono {S} f = (e e′ : Edg S) → actE f e ≡ actE f e′ → e ≡ e′

  module _
    {Γ Δ Γ′ Δ′}
    {S : Shape Ob Γ Δ} {T : Shape Ob Γ′ Δ′}
    (gr : Grounded S)
    (f : GMap S T)
    (mono : EdgeMono f)
    (θ : GMap S S)
    (fix : (e : Edg S) → actE f (actE θ e) ≡ actE f e)
    where

    -- The edge half: injectivity cancels `f` from `f (θ e) = f e`.
    iv′-edges : (e : Edg S) → actE θ e ≡ e
    iv′-edges e = mono (actE θ e) e (fix e)

    -- AXIOM (iv′), for a map with injective edge action. Note that `θ` is not
    -- assumed invertible: the argument never needs it, which is a genuine
    -- strengthening over the published statement and comes from the same place
    -- `edge-determined`'s strengthening does.
    iv′ : θ ≐ gid S
    iv′ =
      edge-determined
        gr
        θ
        (gid S)
        (λ e → trans (iv′-edges e) (sym (app-idTab (edges S) e)))

------------------------------------------------------------------------------
-- AND THE HYPOTHESIS IS NECESSARY. Two parallel through-wires, the automorphism
-- that swaps them, and the map that collapses both onto one — exhibited, not
-- promised, in the style the surrounding modules use for their own refutations.
------------------------------------------------------------------------------

-- Two wires side by side: no vertices, two legs in, two legs out, each leg
-- wired straight across.
two-wires : Shape _ 𝟚 𝟚
two-wires = idn 𝟚

-- It is vacuously grounded — it has no vertices to be isolated.
two-wires-grounded : Grounded two-wires
two-wires-grounded ()

-- THE AUTOMORPHISM: exchange the two wires, and with them the two legs at each
-- end. Incidence holds because the exchange is uniform across all three.
swap-wires : GMap two-wires two-wires
swap-wires .onV = []
swap-wires .onE = there here ∷ here ∷ []
swap-wires .onI = there here ∷ here ∷ []
swap-wires .onO = there here ∷ here ∷ []
swap-wires .onE-end₀ here = refl
swap-wires .onE-end₀ (there here) = refl
swap-wires .onE-end₁ here = refl
swap-wires .onE-end₁ (there here) = refl

-- THE MAP THAT FORGETS THE DIFFERENCE: both wires onto the single wire of
-- `edge`, both legs at each end onto its single leg.
collapse-wires : GMap two-wires edge
collapse-wires .onV = []
collapse-wires .onE = here ∷ here ∷ []
collapse-wires .onI = here ∷ here ∷ []
collapse-wires .onO = here ∷ here ∷ []
collapse-wires .onE-end₀ here = refl
collapse-wires .onE-end₀ (there here) = refl
collapse-wires .onE-end₁ here = refl
collapse-wires .onE-end₁ (there here) = refl

-- It is not injective on edges, which is the hypothesis `iv′` spends.
collapse-not-mono : ¬ (EdgeMono collapse-wires)
collapse-not-mono mono with mono here (there here) refl
... | ()

-- The collapse is fixed by the swap, at every edge.
collapse-fixed : (e : Edg two-wires) → actE collapse-wires (actE swap-wires e) ≡ actE collapse-wires e
collapse-fixed here = refl
collapse-fixed (there here) = refl

-- And the swap is not the identity.
swap-not-id : ¬ (swap-wires ≐ gid two-wires)
swap-not-id p with p .atE here
... | ()

-- SO AXIOM (iv′) FAILS FOR `GMap` AT LARGE: a map, an automorphism of its
-- source fixing it, and the automorphism is not the identity. Drop `EdgeMono`
-- from `iv′` and the conclusion is false.
--
-- What the witness also shows, and what makes it more than a counterexample:
-- the offending map LOWERS the vertex-and-edge count, so a degree function
-- would place it outside any degree-raising subcategory. The axiom is therefore
-- not about this map layer at all — it is about a subcategory the degree
-- function has to carve out first, which inverts the order the two obligations
-- are usually named in.
collapse-witnesses-iv′-fails
  : ¬ ((e : Edg two-wires) → actE collapse-wires (actE swap-wires e) ≡ actE collapse-wires e
       → swap-wires ≐ gid two-wires)
collapse-witnesses-iv′-fails h = swap-not-id (h here (collapse-fixed here))
