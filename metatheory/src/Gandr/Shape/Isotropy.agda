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
-- The other half is why the hypothesis is stated rather than assumed away: two
-- parallel through-wires admit a non-identity automorphism swapping them, and
-- the map collapsing both onto the single wire of `edge` is fixed by it. It is
-- a property of a subcategory, and gandr has not yet carved one out — which is
-- the finding, not the failure.
--
-- ── THE OTHER HALF IS TWO STATEMENTS, AND THE ASYMMETRY IS DELIBERATE ───────
-- This section carried ONE refutation until 2026-08-14, and its type quantified
-- `θ` over ENDOMORPHISMS while the header above states the axiom with `θ`
-- INVERTIBLE. So the header carried a premise no theorem carried, in the
-- direction that makes the result look stronger than it was: what was refuted
-- was unrestricted cancellation, not the axiom. There are now two theorems.
--
--   `collapse-witnesses-cancellation-fails` — endomorphisms, no invertibility.
--   `collapse-witnesses-axiom-fails`        — automorphisms, invertibility
--                                             witnessed by `swap-invertible`.
--
-- `iv′` and the second of those are NOT exact complements, and that is the
-- point rather than an oversight. `iv′` deliberately drops invertibility, which
-- is a genuine strengthening — it proves the same conclusion from a weaker
-- hypothesis, and the argument never reaches for an inverse. Pairing it with a
-- refutation that DOES assume invertibility says something stronger than a
-- matched pair would: `EdgeMono` is necessary even when `θ` is restricted to
-- automorphisms.
--
-- Stating the axiom's form at all required composition of `GMap`s, which this
-- map layer did not have — so an inverse was not merely unconstructed here, it
-- was inexpressible. `gcomp`, its two unit laws, and `Invertible` are that gap
-- closed. The unit laws cost no hypotheses, because `_≐_` compares actions
-- rather than tables.
--
-- ── WHAT THIS DOES NOT SETTLE, STATED SO IT IS NOT OVER-READ ────────────────
-- `GMap` is NOT the site's morphism class. The published Γ⁺ contains the
-- contracting cofaces, which join a leg to a leg and turn two edges into one
-- inner edge — and the incidence conditions above make that INEXPRESSIBLE as a
-- `GMap`. So the verdict here is about the carrier's homomorphisms, and
-- transporting it to the site needs the site's own presentation, which does not
-- exist. What would close it: a morphism class in which an end may change kind,
-- and its degree-raising subcategory.
--
-- ── WHAT THE DEGREE WITNESSES DO AND DO NOT SETTLE ──────────────────────────
-- An earlier reading of this section argued that the counterexample is void for
-- a generalized Reedy structure on an independent ground: the axiom quantifies
-- its map over the degree-RAISING subcategory, and `collapse-wires` was said to
-- LOWER the vertex-and-edge count, so a degree function would exclude it.
--
-- The count is not what that argument assumed. The degree is the vertex count
-- plus the INTERNAL-edge count, and an internal edge is one with a vertex at
-- BOTH ends; a wiring has no vertices at all, so no edge of one is internal.
-- `two-wires` and `edge` therefore both sit at degree ZERO, and `collapse-wires`
-- is degree-PRESERVING rather than degree-lowering. `deg-two-wires` and
-- `collapse-preserves-degree` below check that by `refl`.
--
-- THE CONCLUSION SURVIVES THE CORRECTED REASON, and an intermediate revision of
-- this header had that backwards. A non-invertible map in a degree-raising class
-- must raise the degree STRICTLY, so degree-preserving excludes it exactly as
-- degree-lowering would have. The original argument had the wrong reason and the
-- right conclusion; the correction to it had the right reason and the wrong
-- conclusion. **The invertibility witness above does not rest on any of this** —
-- it rests on the header having stated a premise no theorem carried, which is a
-- defect whatever the degree function does.
--
-- What the witnesses DO support is a constraint rather than a verdict, and that
-- is the useful half: a non-invertible map preserving the degree sits in NEITHER
-- the raising class nor the lowering one, while a Reedy structure requires every
-- map to factor through them. So this pair is a condition any future
-- factorization has to meet, not a refutation of any axiom. The general fact is
-- stronger than the instance — the ENTIRE zero-vertex stratum sits at degree
-- zero identically — so the constraint is not about one accidental pair.
--
-- AND THE CONSTRAINT BINDS THE SITE PRESENTATION RATHER THAN THIS CARRIER,
-- which is where the section above leaves things too. Nothing here refutes any
-- statement about the site and nothing here proves one, because the site has no
-- morphism class to quantify over. The published setting never meets this pair,
-- and the reason does not transfer: its objects are CONNECTED, so its only
-- vertex-free objects are the exceptional edge and the nodeless loop, while
-- `two-wires` is disconnected and is not an object of it at all. **A gandr site
-- admitting disconnected wirings inherits no protection from that definition**,
-- so connectivity — or a restriction of equal force — is a required axiom rather
-- than a free rider.
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
  using (actV)
  using (actE)
  using (actI)
  using (actO)
  using (onE-end₀)
  using (onE-end₁)
  using (Grounded)
  using (_≐_)
  using (atV)
  using (atE)
  using (edge-determined)
open import Gandr.Shape.Degree
  using (deg)

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
-- FUSING TWO ACTIONS ACROSS AN END. An end is a three-way sum, and every
-- incidence obligation states its equation through `smap actV (smap actI actO)`.
-- Composing two maps therefore has to push one such triple past another and
-- land on a THIRD, and the third is not the composite of the two functions but
-- the action read off the composite's own tables — equal to it pointwise and
-- not definitionally, since `mapTab` computes and `app` recurses.
--
-- So the two steps that would otherwise be separate — functoriality of `smap`,
-- and rewriting along the pointwise agreement — are one lemma, taking the three
-- agreements and doing the case split once.
------------------------------------------------------------------------------

module _
  {a b c a′ b′ c′ a″ b″ c″}
  {A : Set a} {B : Set b} {C : Set c}
  {A′ : Set a′} {B′ : Set b′} {C′ : Set c′}
  {A″ : Set a″} {B″ : Set b″} {C″ : Set c″}
  where

  smap-fuse
    : {u₁ : A′ → A″} {u₂ : B′ → B″} {u₃ : C′ → C″}
    → {v₁ : A → A′} {v₂ : B → B′} {v₃ : C → C′}
    → {w₁ : A → A″} {w₂ : B → B″} {w₃ : C → C″}
    → ((x : A) → u₁ (v₁ x) ≡ w₁ x)
    → ((y : B) → u₂ (v₂ y) ≡ w₂ y)
    → ((z : C) → u₃ (v₃ z) ≡ w₃ z)
    → (x : A ⊎ (B ⊎ C))
    → smap u₁ (smap u₂ u₃) (smap v₁ (smap v₂ v₃) x) ≡ smap w₁ (smap w₂ w₃) x
  smap-fuse p q r (inj₁ x) = cong inj₁ (p x)
  smap-fuse p q r (inj₂ (inj₁ y)) = cong (λ z → inj₂ (inj₁ z)) (q y)
  smap-fuse p q r (inj₂ (inj₂ z)) = cong (λ z′ → inj₂ (inj₂ z′)) (r z)

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
-- COMPOSITION, AND WHY IT HAD TO BE BUILT HERE.
--
-- The axiom quantifies its automorphism over INVERTIBLE maps, and invertibility
-- is not a property one can state of a `GMap` in this tree: there was no
-- composition of maps anywhere in the map layer, so an inverse had nothing to
-- be an inverse with respect to. It was not unconstructed, it was
-- INEXPRESSIBLE — which is why the refutation below could only ever have been
-- about endomorphisms until this section existed.
--
-- Composition is post-composition on all four tables, which is exactly what
-- `mapTab` is. The work is entirely in the two incidence obligations, and
-- `smap-fuse` is where it goes.
------------------------------------------------------------------------------

module _ {ℓ} {Ob : Set ℓ} where

  module _
    {Γ Δ Γ′ Δ′ Γ″ Δ″}
    {S : Shape Ob Γ Δ} {T : Shape Ob Γ′ Δ′} {U : Shape Ob Γ″ Δ″}
    (g : GMap T U) (f : GMap S T)
    where

    private
      -- The composite's own action on an end agrees with the two actions
      -- applied in turn — pointwise, by `app-mapTab` at each of the three
      -- kinds, and the case split is `smap-fuse`'s.
      fuse
        : (x : Vtx S ⊎ (Ix Γ ⊎ Ix Δ))
        → smap (actV g) (smap (actI g) (actO g))
            (smap (actV f) (smap (actI f) (actO f)) x)
          ≡ smap
              (app (mapTab (actV g) (onV f)))
              (smap
                (app (mapTab (actI g) (onI f)))
                (app (mapTab (actO g) (onO f))))
              x
      fuse =
        smap-fuse
          (λ v → sym (app-mapTab (actV g) (onV f) v))
          (λ i → sym (app-mapTab (actI g) (onI f) i))
          (λ o → sym (app-mapTab (actO g) (onO f) o))

    -- Composition of maps of shapes.
    gcomp : GMap S U
    gcomp .onV = mapTab (actV g) (onV f)
    gcomp .onE = mapTab (actE g) (onE f)
    gcomp .onI = mapTab (actI g) (onI f)
    gcomp .onO = mapTab (actO g) (onO f)
    -- Both obligations are the same four steps: factor the composite's edge
    -- action, spend the outer map's incidence, spend the inner map's under a
    -- congruence, and fuse the two action triples into the composite's own.
    gcomp .onE-end₀ e =
      trans
        (cong (end₀ U) (app-mapTab (actE g) (onE f) e))
        (trans
          (onE-end₀ g (actE f e))
          (trans
            (cong (smap (actV g) (smap (actI g) (actO g))) (onE-end₀ f e))
            (fuse (end₀ S e))))
    gcomp .onE-end₁ e =
      trans
        (cong (end₁ U) (app-mapTab (actE g) (onE f) e))
        (trans
          (onE-end₁ g (actE f e))
          (trans
            (cong (smap (actV g) (smap (actI g) (actO g))) (onE-end₁ f e))
            (fuse (end₁ S e))))

  -- THE IDENTITY IS A UNIT ON BOTH SIDES, and at no cost in hypotheses. The
  -- unit laws for the WIRING layer's composition need set-ness of the colours,
  -- because they compare witnesses; these compare ACTIONS, since `_≐_` is
  -- pointwise, so nothing has to be decided about the objects.
  module _
    {Γ Δ Γ′ Δ′} {S : Shape Ob Γ Δ} {T : Shape Ob Γ′ Δ′}
    (f : GMap S T)
    where

    gcomp-idnˡ : gcomp (gid T) f ≐ f
    gcomp-idnˡ .atV v =
      trans (app-mapTab (actV (gid T)) (onV f) v) (app-idTab (verts T) (actV f v))
    gcomp-idnˡ .atE e =
      trans (app-mapTab (actE (gid T)) (onE f) e) (app-idTab (edges T) (actE f e))

    gcomp-idnʳ : gcomp f (gid S) ≐ f
    gcomp-idnʳ .atV v =
      trans
        (app-mapTab (actV f) (idTab (verts S)) v)
        (cong (actV f) (app-idTab (verts S) v))
    gcomp-idnʳ .atE e =
      trans
        (app-mapTab (actE f) (idTab (edges S)) e)
        (cong (actE f) (app-idTab (edges S) e))

  -- INVERTIBILITY, the premise the axiom states and this module could not.
  -- Two-sided, up to `_≐_` rather than on the nose, which is the equality every
  -- other statement here uses.
  record Invertible
    {Γ Δ Γ′ Δ′} {S : Shape Ob Γ Δ} {T : Shape Ob Γ′ Δ′}
    (f : GMap S T)
    : Set ℓ
    where
    field
      inv : GMap T S
      invˡ : gcomp inv f ≐ gid S
      invʳ : gcomp f inv ≐ gid T

  open Invertible public

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

-- AND THE SWAP IS AN AUTOMORPHISM, which is the premise the axiom states and
-- which nothing here could express before `gcomp` did. A transposition is its
-- own inverse, so the same map serves as the inverse and both round trips are
-- one case split with nothing in the cases: the vertex clause is absurd because
-- a wiring has no vertices, and the two edge clauses compute.
swap-invertible : Invertible swap-wires
swap-invertible .inv = swap-wires
swap-invertible .invˡ .atV ()
swap-invertible .invˡ .atE here = refl
swap-invertible .invˡ .atE (there here) = refl
swap-invertible .invʳ .atV ()
swap-invertible .invʳ .atE here = refl
swap-invertible .invʳ .atE (there here) = refl

-- SO UNRESTRICTED CANCELLATION FAILS FOR `GMap` AT LARGE: a map, an
-- ENDOMORPHISM of its source fixing it, and the endomorphism is not the
-- identity. Drop `EdgeMono` from `iv′` and the conclusion is false.
--
-- The hypothesis is quantified the way `iv′` quantifies it — fixedness at EVERY
-- edge, inside the implication. An earlier form of this theorem put the edge
-- outside, which negates a strictly stronger statement and therefore says less
-- about `iv′` than it appeared to; `collapse-fixed` supplies the ∀-form
-- outright, so the honest statement was always available at the same cost.
collapse-witnesses-cancellation-fails
  : ¬ (((e : Edg two-wires)
        → actE collapse-wires (actE swap-wires e) ≡ actE collapse-wires e)
       → swap-wires ≐ gid two-wires)
collapse-witnesses-cancellation-fails h = swap-not-id (h collapse-fixed)

-- AND THE AXIOM ITSELF FAILS — the statement WITH the invertibility premise,
-- which is what the module header has always said the axiom is and what no
-- theorem here used to carry. This is the one that bears on a generalized Reedy
-- structure; the theorem above bears on unrestricted cancellation, which is a
-- different and stronger statement.
collapse-witnesses-axiom-fails
  : ¬ ((θ : GMap two-wires two-wires)
       → Invertible θ
       → ((e : Edg two-wires)
          → actE collapse-wires (actE θ e) ≡ actE collapse-wires e)
       → θ ≐ gid two-wires)
collapse-witnesses-axiom-fails h =
  swap-not-id (h swap-wires swap-invertible collapse-fixed)

-- THE DEGREE VALUES THE HEADER'S LAST SECTION TURNS ON, checked rather than
-- argued. `deg` counts vertices plus INTERNAL edges, and `inner?` calls a wire
-- internal only when both of its ends are at vertices; a wiring has no vertices,
-- so it has neither. Both shapes of the counterexample sit at degree zero and
-- the collapse preserves it.
--
-- Preserving is enough to keep the map out of a degree-raising class, since a
-- non-invertible member of one has to raise STRICTLY. So these witnesses do not
-- put the counterexample inside the axiom's scope, and they are not what makes
-- the invertibility witness above worth having — that rests on the header having
-- claimed a premise no theorem carried. What they do give is a constraint on any
-- future factorization: this map is in neither class, and a Reedy structure
-- needs every map to factor through them.
deg-two-wires : deg two-wires ≡ 0
deg-two-wires = refl

collapse-preserves-degree : deg two-wires ≡ deg edge
collapse-preserves-degree = refl
