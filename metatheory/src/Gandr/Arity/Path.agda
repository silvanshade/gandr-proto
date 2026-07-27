{-# OPTIONS --safe --without-K --hidden-argument-puns #-}

------------------------------------------------------------------------------
-- Gandr.Arity.Path — the linear arity kit: snoc paths and the witnessed
-- concatenation over them.
--
-- Over a set of positions and an edge family, `Path` is the snoc list of
-- composable edges — the free-category monad's carrier — `_++_` its derived
-- concatenation, and `Cat` the GRAPH of that concatenation as a first-order
-- witness relation.
--
-- ── WHERE THIS SITS IN THE PROPERAD PLAN ────────────────────────────────────
-- This is the FIRST of two intended arity kits. The design bet is that a cell
-- complex can be parameterized by its arity, so that:
--
--   * this kit — arities are PATHS, chained end to end — gives the ordinary and
--     virtual-double-category case; and
--   * a many-in/many-out kit gives the properadic case.
--
-- The bet is licensed rather than merely convenient. What makes generalized
-- multicategory theory apply at all is that the arity monad be CARTESIAN, and
-- what buys cartesianness for the properadic monad is that the symmetric group
-- act freely — the same freeness that the ordered representation and the
-- simple-connectivity restriction were already adopted for. The free-category
-- monad is cartesian too. So the two kits are two instances of one criterion,
-- not two unrelated constructions.
--
-- ── WHY THERE IS NO ARITY INTERFACE RECORD YET ──────────────────────────────
-- One instance does not determine an abstraction. Extracting a record from this
-- module alone would encode the linear case's accidents — that positions are
-- objects rather than lists, that the unit is a CONSTRUCTOR rather than derived
-- — as if they were the general shape. The interface is extracted once the
-- second kit exists and the genuinely shared part is visible.
--
-- What is expected to be shared, recorded so the extraction has a target: the
-- carrier, a unit, a multiplication spoken only through its graph, and the four
-- derived lemmas below (totality, functionality, associativity, whiskering).
-- What is expected NOT to be shared: the unit's status. Here `here` is a
-- constructor; in the many-out kit the identity must be DERIVED, because an
-- identity-shaped constructor repeating a frame variable across its result
-- indices is exactly the discipline violation this tree forbids.
--
-- ── THE TWO WITNESS DISCIPLINES, WHICH ARE WHY THIS MODULE LOOKS ODD ────────
-- Both exist to keep structures computing under `--without-K`.
--
--   * A defined function never appears in a matchable index. Concatenation
--     enters an index only through `Cat`, its inductive graph. The units
--     `here`/`then` may head an index; the multiplication `_++_` may not.
--   * No identity-shaped constructor repeats a frame variable across its result
--     indices. `Cat`'s `nil` copies a variable across its first and third index,
--     which is the μ-witness's unit clause and is the endorsed shape.
--
-- ── A LOCATED WALL, NOT A GAP ───────────────────────────────────────────────
-- Decidable equality on `Path` is NOT here, and the reason is specific. The
-- naive homogeneous comparison hits `--without-K` twice: matching a second
-- `here` needs to eliminate a reflexive equation on a position variable, and
-- `then`-injectivity needs the same on the existential middle position.
-- Comparing HETEROGENEOUSLY — `Same` below, endpoints kept as distinct
-- variables — makes the positive direction K-free, and `same-here` is that
-- direction. A third wall blocks the packaged decision procedure: every
-- refutation must eliminate a `Same` proof, and matching `then≈` unifies
-- compound indices, forcing the same deletion again. The K-free completion
-- turns `Same` into a recursive relation, so refutations project rather than
-- match, and pays decidable-equality-implies-UIP at each boundary. That is a
-- real development and it is deferred, not overlooked.
------------------------------------------------------------------------------

module Gandr.Arity.Path where

-- The ambient equality, repackaged under the names the witness lemmas use.
private
  module ≡ where
    open import Relation.Binary.PropositionalEquality public
      using (_≡_)
      renaming (refl to idn)
      renaming (cong to fun*)
open ≡
  using (_≡_)

------------------------------------------------------------------------------
-- The carrier and the graph-of-multiplication witness. Positions and edges are
-- explicit here so a consumer's abbreviation reads `Path Pos Edge`.
------------------------------------------------------------------------------
module _ {ℓ} (Pos : Set ℓ) (Edge : Pos → Pos → Set ℓ) where

  -- Snoc paths: the free-category monad's carrier. `here` and `then` are its
  -- units, and both may head a matchable index.
  data Path (a : Pos) : Pos → Set ℓ where
    -- the empty path
    here : Path a a
    -- extend a path by one edge on the right
    then : ∀ {m c} → Path a m → Edge m c → Path a c

  -- The graph of concatenation: `Cat p q r` witnesses `p ++ q = r` as a
  -- first-order constructor-headed relation, so concatenation can be spoken in
  -- a matchable index without the defined function appearing there.
  data Cat {a} : ∀ {b c} → Path a b → Path b c → Path a c → Set ℓ where
    -- concatenating the empty path on the right changes nothing
    nil
      : ∀ {b}
      → {p : Path a b}
      → Cat p here p
    -- extend the witness by the second path's last edge
    cons
      : ∀ {b m c}
      → {p : Path a b} {q : Path b m} {r : Path a m}
      → {g : Edge m c}
      → Cat p q r
      → Cat p (then q g) (then r g)

------------------------------------------------------------------------------
-- The derived operations. Positions and edges are implicit from here on, since
-- they are recovered from the paths' types.
------------------------------------------------------------------------------
module _ {ℓ} {Pos : Set ℓ} {Edge : Pos → Pos → Set ℓ} where

  -- Concatenation, derived rather than adjoined, by right recursion on the
  -- second argument — so the RIGHT unit law holds definitionally.
  infixl 5 _++_
  _++_
    : ∀ {a b c}
    → Path Pos Edge a b
    → Path Pos Edge b c
    → Path Pos Edge a c
  p ++ here = p
  p ++ (then q g) = then (p ++ q) g

  -- Totality: the graph holds at concatenation's own value. This is what lets a
  -- consumer produce a witness wherever it would otherwise have written `_++_`.
  cat-graph
    : ∀ {a b c}
    → (p : Path Pos Edge a b) (q : Path Pos Edge b c)
    → Cat Pos Edge p q (p ++ q)
  cat-graph p here = nil
  cat-graph p (then q g) = cons (cat-graph p q)

  -- Functionality: the witness really is a graph, determined by its endpoints.
  cat-fun
    : ∀ {a b c}
    → {p : Path Pos Edge a b} {q : Path Pos Edge b c} {r r′ : Path Pos Edge a c}
    → Cat Pos Edge p q r
    → Cat Pos Edge p q r′
    → r ≡ r′
  cat-fun nil nil = ≡.idn
  cat-fun (cons w) (cons w′) = ≡.fun* (λ r → then r _) (cat-fun w w′)

  -- Associativity, spoken entirely through witnesses: from `p++q=r`, `r++s=t`
  -- and `q++s=u`, derive `p++u=t`. Contractible given the endpoints, so this is
  -- lemma-layer rather than structure.
  cat-assoc
    : ∀ {a b c d}
    → {p : Path Pos Edge a b} {q : Path Pos Edge b c} {s : Path Pos Edge c d}
    → {r : Path Pos Edge a c} {t : Path Pos Edge a d} {u : Path Pos Edge b d}
    → Cat Pos Edge p q r
    → Cat Pos Edge r s t
    → Cat Pos Edge q s u
    → Cat Pos Edge p u t
  cat-assoc w₁ nil nil = w₁
  cat-assoc w₁ (cons w₂) (cons w₃) = cons (cat-assoc w₁ w₂ w₃)

  -- Left-whiskering a witness by a fixed prefix. Recursion on the witness, which
  -- avoids the general reassociation — this is what a consumer needs to move an
  -- empty-arity cell past an adjacent frame.
  cat-whisk
    : ∀ {i a b c}
    → (g : Path Pos Edge i a)
    → {h : Path Pos Edge a b} {f : Path Pos Edge b c} {r : Path Pos Edge a c}
    → Cat Pos Edge h f r
    → Cat Pos Edge (g ++ h) f (g ++ r)
  cat-whisk g nil = nil
  cat-whisk g (cons w) = cons (cat-whisk g w)

------------------------------------------------------------------------------
-- Heterogeneous structural equality: the K-free half of the comparison. See
-- the header for why the homogeneous form does not typecheck and what the full
-- decision procedure would cost.
------------------------------------------------------------------------------
module _ {ℓ} {Pos : Set ℓ} {Edge : Pos → Pos → Set ℓ} where

  -- Structural equality of paths whose END positions may differ. Keeping the
  -- two endpoints as DISTINCT variables is what lets the constructors match.
  data Same {a} : ∀ {b b′} → Path Pos Edge a b → Path Pos Edge a b′ → Set ℓ where
    here≈
      : Same here here
    then≈
      : ∀ {m c} {q q′ : Path Pos Edge a m} {g g′ : Edge m c}
      → Same q q′
      → g ≡ g′
      → Same (then q g) (then q′ g′)

  -- Reflexivity into it, which constructs with no index matching and so is
  -- K-free.
  same-here : ∀ {a b} (x : Path Pos Edge a b) → Same x x
  same-here here = here≈
  same-here (then q g) = then≈ (same-here q) ≡.idn
