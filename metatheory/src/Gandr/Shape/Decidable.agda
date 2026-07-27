{-# OPTIONS --safe --without-K --hidden-argument-puns #-}

------------------------------------------------------------------------------
-- Gandr.Shape.Decidable — maps of cell shapes, and where decidable equality
-- actually comes from.
--
-- The surrounding design rests on comparing stored representations to decide a
-- semantic question, and the licence for that is meant to be edge-
-- extensionality: a map of shapes is determined by its action on the FINITE
-- edge set. This module builds the map layer `Gandr.Shape.Graph` stops short
-- of, and then proves that statement rather than assuming it.
--
-- ── WHAT IS PROVED, AND WHAT IS ONLY CITED ──────────────────────────────────
-- HRY Cor 6.62 is a statement about GRAPHICAL maps — the morphisms of `Γ`,
-- built from subgraph inclusions, a larger and more delicate class than the
-- homomorphisms below. That corollary is CITED by the plan as the reason the
-- property is expected to survive into that setting; it is NOT what is proved
-- here and this module does not pretend otherwise.
--
-- What IS proved is the analogue for `GMap`, the structure-preserving maps of
-- the shape objects this tree carries. `edge-determined` is a theorem about
-- those, with a hypothesis, and `_≟≐_` is its decision procedure. When the
-- graphical category lands, this is the statement it has to reproduce, not a
-- lemma it can import.
--
-- ── THE ACTIONS ARE TABLES, AND THAT IS THE WHOLE POINT ─────────────────────
-- An earlier revision typed the actions as FUNCTIONS `Vtx G → Vtx H`, and
-- reported that propositional equality of maps was therefore out of reach —
-- correctly, since `--safe --without-K` has no function extensionality — and
-- then explained the gap as the SETOID-not-SET commitment paying rent.
--
-- That explanation was a rationalisation of a representation choice. A map's
-- action is a finite table, and a finite table carried as DATA has pointwise
-- agreement implying propositional equality by ordinary induction: `tab-ext`
-- below is two clauses and needs no axiom. So the actions here are `Tab`s, and
-- `≐⇒actions` is the statement the funext wall was standing in for.
--
-- Note what this does NOT reach, stated so the residue is not mistaken for a
-- discharge. Equality of the RECORDS additionally requires the two incidence
-- fields to agree, and those are equality proofs; identifying them needs UIP
-- at `Vtx T ⊎ Ix Γ′`, which is available (that type has decidable equality)
-- but is not taken here, because every consumer wants the setoid relation.
--
-- ── THE HYPOTHESIS IS REAL, AND IT IS EXHIBITED ─────────────────────────────
-- Edge-extensionality is FALSE without `Grounded` — every vertex attached to
-- some edge — and the counterexample is at the bottom of this module rather
-- than left to the reader's good faith. `point` is a single vertex with no
-- ports at all, so any two maps out of it agree on every edge vacuously while
-- being free to send its vertex anywhere. An isolated vertex is invisible to
-- the edge set, which is exactly what the hypothesis rules out.
--
-- This is not a restriction smuggled in to make a theorem go through: the
-- shapes the cell class cares about are connected with at least one edge, and
-- `chain-grounded` discharges it for the two-vertex composite.
--
-- ── LEGS ARE PART OF A MAP, WHICH THE TABULAR PRESENTATION COULD NOT SAY ────
-- A shape is indexed by its interfaces, so a map carries actions on the input
-- and output legs alongside the actions on vertices and edges, and incidence
-- preservation is ONE equation per direction covering legs and internal
-- endpoints together. Under the previous encoding legs were `nothing`s and the
-- leg action could not be stated at all.
------------------------------------------------------------------------------

module Gandr.Shape.Decidable where

open import Gandr.Shape.Graph
  using (Shape)
  using (Ix)
  using (here)
  using (there)
  using (Vtx)
  using (Edg)
  using (verts)
  using (pool)
  using (origin)
  using (dest)
  using (point)
  using (chain)

open import Data.List.Base
  using (List)
  using ([])
  using (_∷_)
open import Data.Product.Base
  using (_×_)
  using (_,_)
open import Data.Sum.Base
  using (_⊎_)
  using (inj₁)
  using (inj₂)
  renaming (map to smap)
open import Data.Sum.Properties
  using (inj₁-injective)
open import Level
  using (Level)
  using (_⊔_)
open import Relation.Binary.Bundles
  using (Setoid)
open import Relation.Binary.PropositionalEquality
  using (_≡_)
  using (refl)
  using (sym)
  using (trans)
  using (cong)
open import Relation.Nullary.Decidable
  using (Dec)
  using (yes)
  using (no)
  using (map′)
open import Relation.Nullary.Negation
  using (¬_)

-- ════════════════════════════════════════════════════════════════════════════
-- TABLES. A finite map out of the positions of a list, carried as data. This
-- is the `Vec`-not-`Fin n → A` move in the form the shape kit needs, and it is
-- what makes every equality below structural.
-- ════════════════════════════════════════════════════════════════════════════

data Tab {a b} {A : Set a} (B : Set b) : List A → Set (a ⊔ b) where
  -- nothing to map out of
  [] : Tab B []
  -- one target per position
  _∷_ : ∀ {x xs} → B → Tab B xs → Tab B (x ∷ xs)

module _ {a b} {A : Set a} {B : Set b} where

  -- Reading the table back as the function it stands for. A derived
  -- OPERATION over data, which is what the representation rule permits.
  app : ∀ {xs : List A} → Tab B xs → Ix xs → B
  app (t ∷ ts) here = t
  app (t ∷ ts) (there i) = app ts i

  -- the two projections, so a refutation never matches a `refl` whose
  -- components a previous `with` has already identified
  hd : ∀ {x} {xs : List A} → Tab B (x ∷ xs) → B
  hd (t ∷ ts) = t

  tl : ∀ {x} {xs : List A} → Tab B (x ∷ xs) → Tab B xs
  tl (t ∷ ts) = ts

  -- THE STATEMENT THE FUNEXT WALL WAS STANDING IN FOR: pointwise agreement of
  -- two tables IS their propositional equality, by ordinary induction.
  tab-ext
    : ∀ {xs : List A}
    → (s t : Tab B xs)
    → ((i : Ix xs) → app s i ≡ app t i)
    → s ≡ t
  tab-ext [] [] h = refl
  tab-ext (s ∷ ss) (t ∷ ts) h with h here
  ... | refl with tab-ext ss ts (λ i → h (there i))
  ...   | refl = refl

  -- and tables compare, whenever their targets do
  tab?
    : ((u v : B) → Dec (u ≡ v))
    → ∀ {xs : List A}
    → (s t : Tab B xs)
    → Dec (s ≡ t)
  tab? _≟_ [] [] = yes refl
  tab? _≟_ (s ∷ ss) (t ∷ ts) with s ≟ t
  ... | no ¬p = no λ eq → ¬p (cong hd eq)
  ... | yes refl with tab? _≟_ ss ts
  ...   | yes refl = yes refl
  ...   | no ¬p = no λ eq → ¬p (cong tl eq)

-- A finite quantifier over the positions of a list, which is what makes
-- edge-extensionality a DECISION rather than only a theorem.
ix-all?
  : ∀ {a p} {A : Set a}
  → (xs : List A) {P : Ix xs → Set p}
  → ((i : Ix xs) → Dec (P i))
  → Dec ((i : Ix xs) → P i)
ix-all? [] d = yes λ ()
ix-all? (x ∷ xs) d with d here | ix-all? xs (λ i → d (there i))
... | no ¬p | _ = no λ h → ¬p (h here)
... | _ | no ¬p = no λ h → ¬p (λ i → h (there i))
... | yes p | yes h = yes λ { here → p ; (there i) → h i }

-- ════════════════════════════════════════════════════════════════════════════
-- MAPS OF SHAPES. Four actions — vertices, edges, input legs, output legs —
-- all tabulated, with incidence preserved in one equation per direction.
--
-- The chosen port orders are NOT required to be preserved. That is deliberate:
-- the listings are representation content, so a map respecting them would be
-- the planar notion the surrounding design declined, not the general one.
-- ════════════════════════════════════════════════════════════════════════════

module _ {ℓ} {Ob : Set ℓ} where

  record GMap
    {Γ Δ Γ′ Δ′ : List Ob}
    (S : Shape Ob Γ Δ)
    (T : Shape Ob Γ′ Δ′)
    : Set ℓ
    where
    field
      -- the action on vertices
      onV : Tab (Vtx T) (verts S)
      -- the action on edges
      onE : Tab (Edg T) (pool S)
      -- and on each interface
      onI : Tab (Ix Γ′) Γ
      onO : Tab (Ix Δ′) Δ

    -- the actions, read back as operations
    actV : Vtx S → Vtx T
    actV = app onV

    actE : Edg S → Edg T
    actE = app onE

    actI : Ix Γ → Ix Γ′
    actI = app onI

    actO : Ix Δ → Ix Δ′
    actO = app onO

    field
      -- an edge's source goes to its image's source — legs included, in one
      -- equation rather than a case split
      onE-src : (e : Edg S) → origin T (actE e) ≡ smap actV actI (origin S e)
      -- and likewise its target
      onE-tgt : (e : Edg S) → dest T (actE e) ≡ smap actV actO (dest S e)

  open GMap public

  -- ══════════════════════════════════════════════════════════════════════════
  -- ATTACHMENT. A vertex the edge set cannot see is a vertex no amount of edge
  -- data determines, so this is exactly the hypothesis edge-extensionality
  -- needs.
  -- ══════════════════════════════════════════════════════════════════════════

  -- Which end of `e` the vertex `v` sits at. A datatype rather than a sum,
  -- because the two cases are eliminated separately in every consumer.
  data Side {Γ Δ} (S : Shape Ob Γ Δ) (e : Edg S) (v : Vtx S) : Set ℓ where
    -- `e` leaves `v`
    leaving : origin S e ≡ inj₁ v → Side S e v
    -- `e` enters `v`
    entering : dest S e ≡ inj₁ v → Side S e v

  -- Some edge is incident to `v`, at one end or the other.
  record Attached {Γ Δ} (S : Shape Ob Γ Δ) (v : Vtx S) : Set ℓ where
    constructor attached
    field
      -- the witnessing edge
      edge : Edg S
      -- and the end it meets
      side : Side S edge v

  open Attached public

  -- No isolated vertices: every vertex is visible to the edge set.
  Grounded : ∀ {Γ Δ} → Shape Ob Γ Δ → Set ℓ
  Grounded S = (v : Vtx S) → Attached S v

  -- ══════════════════════════════════════════════════════════════════════════
  -- EQUALITY OF MAPS. Pointwise on every action — the setoid equality, which
  -- is what consumers reason through.
  -- ══════════════════════════════════════════════════════════════════════════

  record _≐_
    {Γ Δ Γ′ Δ′} {S : Shape Ob Γ Δ} {T : Shape Ob Γ′ Δ′}
    (f g : GMap S T)
    : Set ℓ
    where
    field
      -- agreement at every vertex
      atV : (v : Vtx S) → actV f v ≡ actV g v
      -- and at every edge
      atE : (e : Edg S) → actE f e ≡ actE g e

  open _≐_ public

  module _ {Γ Δ Γ′ Δ′} {S : Shape Ob Γ Δ} {T : Shape Ob Γ′ Δ′} where

    -- It is an equivalence, pointwise in each component.
    ≐-refl : {f : GMap S T} → f ≐ f
    ≐-refl .atV v = refl
    ≐-refl .atE e = refl

    ≐-sym : {f g : GMap S T} → f ≐ g → g ≐ f
    ≐-sym p .atV v = sym (p .atV v)
    ≐-sym p .atE e = sym (p .atE e)

    ≐-trans : {f g h : GMap S T} → f ≐ g → g ≐ h → f ≐ h
    ≐-trans p q .atV v = trans (p .atV v) (q .atV v)
    ≐-trans p q .atE e = trans (p .atE e) (q .atE e)

    -- The maps `S ⟶ T` as a setoid: the bundle the rest of the tree reasons
    -- through, and what makes `S ⟶ T` an object of the ambient SETOID. The
    -- import is deliberately not taken, since `Gandr.Category.Instances` would
    -- drag `--guardedness` into a module that is otherwise finite
    -- combinatorics, and the bundle is the shared interface anyway.
    mapˢ : Setoid ℓ ℓ
    mapˢ = record
      { Carrier = GMap S T
      ; _≈_ = _≐_
      ; isEquivalence = record
          { refl = ≐-refl
          ; sym = ≐-sym
          ; trans = ≐-trans
          }
      }

    -- AND THE UPGRADE the tabular presentation could not state: agreement
    -- pointwise IS equality of the two actions, propositionally, with no
    -- function extensionality anywhere.
    ≐⇒actions
      : {f g : GMap S T}
      → f ≐ g
      → (onV f ≡ onV g) × (onE f ≡ onE g)
    ≐⇒actions {f} {g} p =
      tab-ext (onV f) (onV g) (p .atV) , tab-ext (onE f) (onE g) (p .atE)

  -- ══════════════════════════════════════════════════════════════════════════
  -- EDGE-EXTENSIONALITY, AND THE DECISION IT BUYS.
  -- ══════════════════════════════════════════════════════════════════════════

  module _
    {Γ Δ Γ′ Δ′}
    {S : Shape Ob Γ Δ} {T : Shape Ob Γ′ Δ′}
    (gr : Grounded S)
    (f g : GMap S T)
    where

    -- The vertex half. A vertex is pinned by any edge incident to it: the two
    -- maps send that edge to the same place, so that edge's endpoint in `T` is
    -- the image of `v` under both, and `inj₁` is injective.
    --
    -- Both cases are the same three-step chain read through a different
    -- projection, which is why neither is factored out — the shared skeleton
    -- is shorter than the abstraction that would capture it.
    same-onV
      : ((e : Edg S) → actE f e ≡ actE g e)
      → (v : Vtx S)
      → actV f v ≡ actV g v
    same-onV h v with gr v
    ... | attached e (leaving p) =
      inj₁-injective
        (trans
          (sym (trans (onE-src f e) (cong (smap (actV f) (actI f)) p)))
          (trans
            (cong (origin T) (h e))
            (trans (onE-src g e) (cong (smap (actV g) (actI g)) p))))
    ... | attached e (entering p) =
      inj₁-injective
        (trans
          (sym (trans (onE-tgt f e) (cong (smap (actV f) (actO f)) p)))
          (trans
            (cong (dest T) (h e))
            (trans (onE-tgt g e) (cong (smap (actV g) (actO g)) p))))

    -- A map is determined by its action on the edge set. The analogue of HRY
    -- Cor 6.62 for these maps — proved, not cited; see the header for what the
    -- citation does and does not cover.
    edge-determined
      : ((e : Edg S) → actE f e ≡ actE g e)
      → f ≐ g
    edge-determined h .atV = same-onV h
    edge-determined h .atE = h

    -- And therefore equality of maps is decidable, given a decision at the
    -- target's edges: the edge set is finite, so the quantifier over it is,
    -- and the vertex half comes free. No canonical form is involved anywhere —
    -- the edge action already is one.
    _≟≐_
      : ((u v : Edg T) → Dec (u ≡ v))
      → Dec (f ≐ g)
    _≟≐_ _≟ᵉ_ =
      map′
        edge-determined
        (λ p e → p .atE e)
        (ix-all? (pool S) (λ e → actE f e ≟ᵉ actE g e))

-- ════════════════════════════════════════════════════════════════════════════
-- THE HYPOTHESIS IS NECESSARY. Without `Grounded`, edge-extensionality fails,
-- and here is the failure rather than a promise of one.
-- ════════════════════════════════════════════════════════════════════════════

-- `point` is one vertex with no ports — the arity-zero corolla, which is a
-- legitimate cell shape and an isolated vertex at the same time.
point-ungrounded : ¬ Grounded point
point-ungrounded gr with gr here
... | attached () _

-- A map out of it is free to send its one vertex anywhere, and has no edges to
-- be constrained by.
pt : Vtx chain → GMap point chain
pt w .onV = w ∷ []
pt w .onE = []
pt w .onI = []
pt w .onO = []
pt w .onE-src ()
pt w .onE-tgt ()

-- Two such maps agree on every edge, vacuously.
pt-agree : (e : Edg point) → actE (pt here) e ≡ actE (pt (there here)) e
pt-agree ()

-- And are not equal. So `Grounded` is load-bearing in `edge-determined`, not
-- decorative: drop it and the conclusion is false.
pt-differ : ¬ (pt here ≐ pt (there here))
pt-differ p with p .atV here
... | ()

-- ════════════════════════════════════════════════════════════════════════════
-- AND IT IS SATISFIABLE. The shapes with edges discharge it, so the theorem is
-- not a statement about an empty class either.
-- ════════════════════════════════════════════════════════════════════════════

-- The two-vertex composite: each vertex is entered by an edge — vertex `0` by
-- an input leg, vertex `1` by the internal edge.
chain-grounded : Grounded chain
chain-grounded here = attached (there (there here)) (entering refl)
chain-grounded (there here) = attached (there here) (entering refl)
chain-grounded (there (there ()))
