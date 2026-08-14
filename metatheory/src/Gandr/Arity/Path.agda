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
-- The target recorded in advance was: the carrier, a unit, a multiplication
-- spoken only through its graph, and the four derived lemmas below (totality,
-- functionality, associativity, whiskering); with the unit's STATUS named as
-- the one thing expected not to generalize.
--
-- ── WHAT THE SECOND INSTANCE ACTUALLY SHOWED ────────────────────────────────
-- `Gandr.Shape.Graph` and `Gandr.Shape.Graft` are now that instance: `Shape Γ Δ`
-- over `List Ob`, with `idn` derived and `graft` composing along the shared
-- interface. Reading the two side by side moves three things, and only the
-- first was anticipated.
--
--   * **The unit's status differs, and further than expected.** It is not only
--     that `here` is a constructor and `idn` a construction. It reaches into
--     which unit LAW is free: here `nil : Cat p here p` IS the right unit,
--     because concatenation recurses on its second argument, and `cat-idnˡ`
--     below has to be proved. In the graph kit neither side is a constructor
--     and both laws are lemmas. So an interface cannot present either unit law
--     as structure; both are fields.
--
--   * **The multiplication's ATOMICITY does not generalize, and this is the
--     expensive one.** `_++_` is one structural recursion, so one inductive
--     graph speaks it. `graft` is a composite of nine operations — the vertex
--     interleaving forces a whiskering, which forces a wire-threading, which
--     forces an insertion exchange — and the witness discipline does not stop
--     at the outermost: each auxiliary's result sits in the next one's graph
--     index. "A multiplication spoken only through its graph" is therefore one
--     relation in this kit and nine in that one. An interface may still demand
--     it, but it must demand it as a FIELD an instance supplies however it can,
--     never as a construction, and the asymmetry has to be stated where a
--     reader meets it rather than discovered by whoever writes the instance.
--
--   * **Functionality cannot be demanded at propositional equality.**
--     `cat-fun` is h-level free because `Cat`'s constructors carry no
--     existential witness: every path in an index is determined by the indices
--     around it. The graph kit's cannot be. Its carrier's `node` carries
--     `Append` witnesses, so the graph of grafting must existentially quantify
--     the whiskered operand's witness, and two witnesses of the same grafting
--     may differ there. Identifying two ARBITRARY such witnesses is
--     `append-uniq`, whose price is `UIP Ob`. So a `mul-fun` landing in `_≡_`
--     charges the law layer an h-level condition in one instance and nothing
--     in the other.
--
-- THE MEASUREMENT THIS SECTION ONCE READ OFF THAT POINT IS WITHDRAWN, and the
-- point itself stands. `Gandr.Shape.Graft`'s unit laws were stated on the
-- FUNCTION and charged `UIP Ob`, and this section read the charge as the
-- h-level condition MOVING rather than vanishing — function-side onto the unit
-- laws, graph-side onto functionality. Those laws are now proved hypothesis-
-- free through `Gandr.Shape.Graph.append-canon`, which compares a witness
-- against the canonical one instead of comparing two arbitrary ones, so the
-- function side of that contrast is empty: nothing is charged there to move.
--
-- What survives is the narrower fact, and it is the one the bullet above
-- states: identifying two arbitrary witnesses at one triple costs set-ness,
-- and a `mul-fun` at `_≡_` is a statement that genuinely has two of them.
--
-- What removes it from both is the heterogeneous comparison. `Same` below was
-- introduced as the K-free way to compare paths with different endpoints; the
-- extraction gives it a second and larger job, because a structural equality
-- can ignore the witness layer that propositional equality has to identify. So
-- the interface's functionality lemma should land in `Same`, and `Same` is not
-- an optional convenience for the heterogeneous case — it is the equality the
-- laws are stated at, and the graph kit owes one before the record can be
-- written.
--
-- A relation the laws are stated at owes all three of its own laws, and it now
-- has them: `same-here`, `same-inv` and `same-seq` below. All three are K-free
-- for the reason the heterogeneous presentation exists — the endpoints are
-- distinct variables, so a constructor match solves each once and no reflexive
-- equation is deleted — and the homogeneous forms do not typecheck, which is
-- the wall recorded below rather than a second one.
--
-- The third job arrives with the universe presentation of the same interface in
-- `Gandr.Arity.Universe`: `Same` is what the REPRESENTATION MAP lands in. That
-- module refutes the published map against the bare positions and proves it
-- over the positions with their order, so `Same` is exactly the identification
-- relation an enriched interpretation determines.
--
-- ── THE PROPOSED INTERFACE, AND WHAT IS STILL OWED ──────────────────────────
-- Recorded rather than written, because two of its fields have no inhabitant in
-- the graph kit yet and a record with one instance is the mistake this section
-- opens by naming.
--
--   Iface     : Set                                  -- positions / interfaces
--   Ar        : Iface → Iface → Set                  -- the arities
--   Same      : Ar a b → Ar a b′ → Set               -- structural equality
--   idn       : (a : Iface) → Ar a a                 -- constructor OR derived
--   mul       : Ar a b → Ar b c → Ar a c
--   Mul       : Ar a b → Ar b c → Ar a c → Set       -- its graph
--   mul-graph : (p : Ar a b) (q : Ar b c) → Mul p q (mul p q)
--   mul-fun   : Mul p q r → Mul p q r′ → Same r r′   -- NOT _≡_
--   mul-idnˡ  : (q : Ar a b) → Mul (idn a) q q
--   mul-idnʳ  : (p : Ar a b) → Mul p (idn b) p
--   mul-assoc : Mul p q r → Mul r s t → Mul q s u → Mul p u t
--   mul-whisk : Mul g h gh → Mul h f r → Mul g r gr → Mul gh f gr
--
-- This module supplies every field. The graph kit supplies `Iface`, `Ar`, `idn`
-- and `mul`, and owes `Same`, `Mul`, and the six lemmas over them.
--
-- Two defects in this module were found by attempting the extraction rather
-- than by reading it, which is the argument for having run the pass before
-- building the nine graphs: `cat-idnˡ` was absent, and `cat-whisk` was stated
-- with the DEFINED concatenation in two of `Cat`'s index positions — see its
-- comment for why that survived local review. Both are repaired below.
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
      renaming (sym to inv)
      renaming (trans to seq)
      renaming (cong to fun*)
      renaming (subst to coe*)
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

  -- The LEFT unit, which is a lemma rather than a constructor. `nil` gives the
  -- right unit definitionally, because concatenation recurses on its second
  -- argument; the other side has to be earned. Worth noting against the graph
  -- kit, where the unit is itself a construction and BOTH laws are lemmas: the
  -- asymmetry between the units is not only about the unit's status, it reaches
  -- into which of its two laws comes for free.
  cat-idnˡ
    : ∀ {a b}
    → (q : Path Pos Edge a b)
    → Cat Pos Edge here q q
  cat-idnˡ here = nil
  cat-idnˡ (then q g) = cons (cat-idnˡ q)

  -- Left-whiskering, spoken entirely through witnesses: from `g++h=gh`,
  -- `h++f=r` and `g++r=gr`, derive `gh++f=gr`. This is what a consumer needs to
  -- move an empty-arity cell past an adjacent frame.
  --
  -- An earlier revision stated it as `Cat (g ++ h) f (g ++ r)`, which put the
  -- DEFINED concatenation into two of `Cat`'s index positions — the exact thing
  -- this module's own header forbids, and it survived because `Cat`'s
  -- constructors leave the first index a variable, so nothing local ever had to
  -- match on it. That is the shape of the failure the discipline warns about:
  -- safe at the site that writes it, stuck at the first consumer that must
  -- match a specific index. Restated over witnesses it costs one extra premise
  -- and one transport in the unit case.
  cat-whisk
    : ∀ {i a b c}
    → {g : Path Pos Edge i a} {h : Path Pos Edge a b} {f : Path Pos Edge b c}
    → {gh : Path Pos Edge i b} {r : Path Pos Edge a c} {gr : Path Pos Edge i c}
    → Cat Pos Edge g h gh
    → Cat Pos Edge h f r
    → Cat Pos Edge g r gr
    → Cat Pos Edge gh f gr
  cat-whisk {gh} w₁ nil w₃ =
    ≡.coe* (λ z → Cat Pos Edge gh here z) (cat-fun w₁ w₃) nil
  cat-whisk w₁ (cons w₂) (cons w₃) = cons (cat-whisk w₁ w₂ w₃)

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

  -- SYMMETRY AND TRANSITIVITY, which make `Same` an equivalence rather than
  -- merely a reflexive comparison. The interface's functionality lemma lands
  -- here rather than in `_≡_`, so it is the relation the laws are stated at,
  -- and a relation the laws are stated at owes all three.
  --
  -- Both are K-free for the reason the heterogeneous presentation exists: the
  -- two endpoints are DISTINCT variables, so a constructor match solves each of
  -- them once and no reflexive equation is ever deleted. The homogeneous forms
  -- do not typecheck, which is the same wall the header records for decidable
  -- equality and is not a second one.
  same-inv
    : ∀ {a b b′}
    → {p : Path Pos Edge a b} {q : Path Pos Edge a b′}
    → Same p q
    → Same q p
  same-inv here≈ = here≈
  same-inv (then≈ s e) = then≈ (same-inv s) (≡.inv e)

  same-seq
    : ∀ {a b b′ b″}
    → {p : Path Pos Edge a b} {q : Path Pos Edge a b′} {r : Path Pos Edge a b″}
    → Same p q
    → Same q r
    → Same p r
  same-seq here≈ here≈ = here≈
  same-seq (then≈ s e) (then≈ t f) = then≈ (same-seq s t) (≡.seq e f)
