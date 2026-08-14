{-# OPTIONS --safe --without-K --hidden-argument-puns #-}

------------------------------------------------------------------------------
-- Gandr.Shape.Degree — the Reedy degree of a shape, and the two exceptions
-- this carrier does not need.
--
-- The template is Hackney-Robertson-Yau, "On factorizations of graphical maps",
-- Homology, Homotopy and Applications 20(2):217-238, 2018,
-- doi:10.4310/HHA.2018.v20.n2.a11, read at arXiv:1705.08546v2 — Definition 3.2,
-- which assigns
--
--     deg(G) = 0                             for the exceptional edge,
--              1                             for the isolated vertex,
--              |Vt(G)| + |Edge_i(G)| + 1     otherwise.
--
-- ── WHY THAT DEFINITION HAS EXCEPTIONS, WHICH IS WHAT DECIDES OURS ──────────
-- Each exception is forced by ONE morphism, and neither morphism exists here.
--
-- The ISOLATED VERTEX is exceptional because of the exceptional inner coface
-- into the NODELESS LOOP: the general formula gives both of them the same
-- value, so a non-invertible map would preserve degree and the first Reedy
-- axiom would fail. The source's Remark 3.6 makes exactly that point about the
-- offset. **This carrier cannot express the nodeless loop** — `Match` has no
-- constructor pairing two sinks, and composition of downward wirings cannot
-- manufacture a closed component — so the map that forces the exception has no
-- codomain, and the exception is void.
--
--   REVERSAL CONDITION, because this leg expires rather than holding forever.
--   "Cannot express" is a claim about `Shape`. The substrate ruling puts closed
--   components ONE LEVEL UP — a code is a shape paired with its count of closed
--   components — and says in terms that not-DERIVABLE and not-ADJOINABLE are
--   different claims, with only the first among the downward consequences. **If
--   the site is ever presented over CODES rather than over shapes, the nodeless
--   loop returns and this exception un-voids.** Nothing else records that.
--
-- The EXCEPTIONAL EDGE is exceptional as the bottom of the degeneracy tower:
-- the codegeneracy that substitutes it at a bivalent vertex is what it is the
-- target of. **That codegeneracy is excluded from gandr's site by
-- nonunitality** (`univalence-spike-01`), so again the map is gone and so is
-- the exception.
--
-- **Both exceptions are void, each for its own reason, and they are the same
-- two reasons that make the site direct.** That is worth stating rather than
-- discovering twice.
--
-- ── AND THEN THE OFFSET HAS NOTHING TO DO ───────────────────────────────────
-- The `+ 1` in the published formula exists to lift the general range clear of
-- the two exceptional values. With no exceptional values it is a constant, and
-- a constant offset cannot change which morphisms strictly raise the degree —
-- so it changes nothing about the axioms and would only leave an unexplained
-- number in the definition. `deg` here is `|Vt| + |Edge_i|`.
--
-- **That holds, and it holds more strongly than the paragraph above argues.**
-- This definition PINS NOTHING, so every offset it could carry is uniform, and
-- a uniform offset is inert by construction rather than by the exceptions being
-- void. Sharper still, on the pair that turns out to matter: the exceptional
-- edge and the two-wire identity have the same vertex count and the same
-- internal-edge count, both zero, so NO uniform offset whatsoever separates
-- them. Restoring the published `+ 1` would change nothing here.
--
-- ── BUT THE OFFSET AND THE PIN ARE A PAIR, AND ONLY ONE OF THEM IS INERT ────
-- The published definition does two things at once, and dropping them together
-- reads as dropping one. The PIN puts the exceptional edge at 0 while every
-- general graph gets `|Vt| + |Edge_i| + 1` and therefore at least 1 — so the
-- edge sits strictly BELOW the whole general range. The offset is only what
-- makes room for the pin. **The separating work is the pin's.**
--
-- Here the edge sits AT the general range instead of below it, and it has
-- company: `deg-empty` and `deg-edge` below are both 0, and so is the two-wire
-- identity (`deg-two-wires`, in `Gandr.Shape.Isotropy`), which carries a
-- non-invertible map onto `edge`. Extend the published function to that shape
-- and it lands at 1, so the same map lowers there and no such pair arises.
--
-- ── AND BEHIND THE PIN IS CONNECTEDNESS, WHICH IS WHAT ACTUALLY TRANSFERS ───
-- The published objects are CONNECTED, so a vertex-free one is the exceptional
-- edge or the nodeless loop and nothing else: two pins EXHAUST that stratum.
-- This carrier admits disconnection, so its vertex-free stratum is infinite and
-- no finite set of pins could exhaust it. **A site presentation over this
-- carrier admitting disconnected wirings inherits no protection from the
-- published definition** — connectivity, or a restriction of equal force, is a
-- required axiom rather than a free rider.
--
-- ── WHAT THAT IS, STATED SO IT IS NOT READ AS A REFUTATION ──────────────────
-- It is a CONSTRAINT ON A FUTURE SITE PRESENTATION. It is not a defect in this
-- degree function, which counts the right things, and it refutes no axiom: a
-- non-invertible degree-PRESERVING map is excluded from a degree-raising class
-- rather than falsifying its axiom, because a non-invertible member of that
-- class has to raise STRICTLY. The published pathology differs in exactly this
-- respect — its failing map is a COFACE, so it is in the raising class by
-- construction and has nowhere to be excluded to. Nothing of that kind exists
-- here, because there is no site and no coface. What the pair does establish is
-- that a map in neither class exists, while a Reedy structure requires every map
-- to factor through them.
--
-- ── WHAT IS PROVED ABOUT IT ─────────────────────────────────────────────────
-- `deg-node` is the degree-raising direction that this carrier CAN state:
-- extending a shape by a vertex strictly raises the degree. The proof is not
-- only the vertex count — publishing a vertex's ports can turn a leg end into a
-- vertex end, so the internal-edge count is monotone as well, and the two
-- effects run the same way. That monotonicity is `inner-node`, and it is where
-- the content is.
--
-- ── WHAT THIS IS NOT ────────────────────────────────────────────────────────
-- A degree function is only half of a Reedy structure; the other half is the
-- morphism class it grades, and this tree has no site. So the axiom "every
-- non-invertible morphism strictly raises the degree" is NOT proved here and
-- cannot be — `deg-node` proves it for the one generator the carrier exhibits.
-- Fuel re-bases on this in one line, once the site exists: it decreases along
-- the RESTRICTION maps of degree-raising morphisms, never along the
-- degree-lowering ones.
------------------------------------------------------------------------------

module Gandr.Shape.Degree where

open import Gandr.Shape.Graph
  using (Shape)
  using (node)
  using (Ix)
  using (here)
  using (there)
  using (Vtx)
  using (Edg)
  using (verts)
  using (edges)
  using (end₀)
  using (end₁)
  using (Append)
  using (empty)
  using (edge)
  using (point)
  using (chain)
  using (corolla-2-1)

open import Data.Bool.Base
  using (Bool)
  using (true)
  using (false)
  using (if_then_else_)
open import Data.List.Base
  using (List)
  using ([])
  using (_∷_)
  using (length)
open import Data.Nat.Base
  using (ℕ)
  using (zero)
  using (suc)
  using (_+_)
  using (_≤_)
  using (_<_)
  using (z≤n)
  using (s≤s)
open import Data.Nat.Properties
  using (+-monoʳ-≤)
  using (m≤n⇒m≤1+n)
open import Data.Sum.Base
  using (_⊎_)
  using (inj₁)
  using (inj₂)
open import Level
  using (Level)
open import Relation.Binary.PropositionalEquality
  using (_≡_)
  using (refl)

------------------------------------------------------------------------------
-- COUNTING OVER POSITIONS. The predicate needs the position rather than the
-- element — whether a wire is internal is a fact about its ends, not about its
-- colour — so the fold is over `Ix` rather than over the list.
------------------------------------------------------------------------------

module _ {a} {A : Set a} where

  count-ix : (xs : List A) → (Ix xs → Bool) → ℕ
  count-ix [] f = 0
  count-ix (x ∷ xs) f =
    (if f here then 1 else 0) + count-ix xs (λ i → f (there i))

  -- The count is monotone in the predicate: a position that counted before
  -- still counts, so nothing can be lost.
  count-ix-mono
    : (xs : List A) (f g : Ix xs → Bool)
    → ((i : Ix xs) → f i ≡ true → g i ≡ true)
    → count-ix xs f ≤ count-ix xs g
  count-ix-mono [] f g h = z≤n
  count-ix-mono (x ∷ xs) f g h with f here | g here | h here
  ... | false | false | _ =
    count-ix-mono xs (λ i → f (there i)) (λ i → g (there i)) (λ i → h (there i))
  ... | false | true | _ =
    m≤n⇒m≤1+n (count-ix-mono xs (λ i → f (there i)) (λ i → g (there i)) (λ i → h (there i)))
  ... | true | true | _ =
    s≤s (count-ix-mono xs (λ i → f (there i)) (λ i → g (there i)) (λ i → h (there i)))
  ... | true | false | hh with hh refl
  ...   | ()

------------------------------------------------------------------------------
-- THE INTERNAL EDGES, AND THE DEGREE.
------------------------------------------------------------------------------

module _ {ℓ} {Ob : Set ℓ} where

  -- A wire is INTERNAL when both of its ends are at vertices. Legs are the only
  -- alternative, and the cut is not special here: a cut between two vertex
  -- out-ports is internal, and a cut with a leg end is not.
  inner?
    : ∀ {Γ Δ}
    → (S : Shape Ob Γ Δ)
    → Edg S
    → Bool
  inner? S e with end₀ S e | end₁ S e
  ... | inj₁ _ | inj₁ _ = true
  ... | inj₁ _ | inj₂ _ = false
  ... | inj₂ _ | _ = false

  -- THE DEGREE. Vertices plus internal edges — the published formula with both
  -- of its exceptions and its offset removed, for the reasons in the header.
  deg
    : ∀ {Γ Δ}
    → Shape Ob Γ Δ
    → ℕ
  deg S = length (verts S) + count-ix (edges S) (inner? S)

  -- PUBLISHING A VERTEX'S PORTS CANNOT UN-INTERNALIZE A WIRE. An end already at
  -- a vertex is routed one step further out and stays at a vertex; only a leg
  -- end can change kind. This is the half of `deg-node` that is not arithmetic.
  inner-node
    : ∀ {Γ Δ Γ′ Δ′} {A B : List Ob} {S : Shape Ob Γ′ Δ′}
    → (p : Append Ob B Γ Γ′) (q : Append Ob A Δ Δ′)
    → (e : Edg S)
    → inner? S e ≡ true
    → inner? (node A B p q S) e ≡ true
  inner-node {S} p q e with end₀ S e | end₁ S e
  ... | inj₁ u | inj₁ v = λ _ → refl
  ... | inj₁ u | inj₂ _ = λ ()
  ... | inj₂ _ | _ = λ ()

  -- EXTENDING BY A VERTEX STRICTLY RAISES THE DEGREE. One vertex arrives and no
  -- internal edge departs, so the two counts move the same way.
  deg-node
    : ∀ {Γ Δ Γ′ Δ′} {A B : List Ob} {S : Shape Ob Γ′ Δ′}
    → (p : Append Ob B Γ Γ′) (q : Append Ob A Δ Δ′)
    → deg S < deg (node A B p q S)
  deg-node {S} p q =
    s≤s
      (+-monoʳ-≤
        (length (verts S))
        (count-ix-mono
          (edges S)
          (inner? S)
          (inner? (node _ _ p q S))
          (λ e → inner-node p q e)))

------------------------------------------------------------------------------
-- THE EXCEPTIONAL OBJECTS, AT THEIR GENERAL VALUES. Each of these is a value
-- the published definition would have overridden, and each is here because
-- nothing in this carrier forces the override.
------------------------------------------------------------------------------

-- The empty shape: nothing to count.
deg-empty : deg empty ≡ 0
deg-empty = refl

-- THE EXCEPTIONAL EDGE, at its general value rather than at an assigned 0. Its
-- one wire runs leg to leg, so it is not internal.
deg-edge : deg edge ≡ 0
deg-edge = refl

-- THE ISOLATED VERTEX, at its general value rather than at an assigned 1. One
-- vertex, no wires at all — which is also why it is not `Grounded`, and why the
-- codegeneracy deleting it is the site's one surviving degeneracy family.
deg-point : deg point ≡ 1
deg-point = refl

-- A corolla is one vertex and no internal wire: all four of its wires meet a
-- leg. So every corolla sits at degree one, exceptional or not — which is the
-- published definition's own behaviour once the offset is dropped.
deg-corolla : deg corolla-2-1 ≡ 1
deg-corolla = refl

-- And a two-vertex composite is one higher, the extra coming from the wire
-- between them rather than from the vertex count alone.
deg-chain : deg chain ≡ 3
deg-chain = refl
