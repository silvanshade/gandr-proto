{-# OPTIONS --safe --guardedness --without-K --hidden-argument-puns #-}

-- `--guardedness` is here for the REASONING VOCABULARY and nothing else, on
-- `Gandr.Shape.Graft`'s terms: `Gandr.Setoid` sits over the coinductive ∞-graph
-- carrier and the flag is infective, so a module with a multi-step equational
-- argument takes it rather than reason in a second style.

------------------------------------------------------------------------------
-- Gandr.Arity.Universe — the arity interface, presented UNIVERSE-STYLE: codes,
-- an interpretation, a unit code, a substitution former, and the two
-- interpretation equivalences the laws are stated over.
--
-- This is spike S10 of the metatheory roadmap, run as a CONTROL EXPERIMENT.
-- `Gandr.Arity.Path` records an arity interface it declines to write, because
-- two of its twelve fields — `Same` and `Mul` — have no inhabitant in the graph
-- kit. The published generalised-operad-universe record [@hewer-2025-hott-operads,
-- Def. 9] is the same interface presented differently, and it names both of the
-- missing fields. This module writes that record in gandr's vocabulary,
-- instantiates it at the LINEAR kit where the answer is already known, and
-- measures. The measurement is in §*What it cost* below.
--
-- ── THE DICTIONARY, PINNED TO THE MONAD RATHER THAN GUESSED ─────────────────
-- A code is an element of the arity monad applied to the one-point species, and
-- the interpretation is the polynomial fibre — the positions of that element.
-- Reading it off that way rather than by analogy fixes every field at once:
--
--   universe field  planar operad   linear kit (here)      circuit kit
--   ──────────────  ─────────────   ────────────────────   ───────────────────
--   Code            ℕ               `Path a b`             `Shape Γ Δ`
--   ⟦ U ∋ _ ⟧       `Fin n`         the path's EDGES       the shape's VERTICES
--   ⊤̂ (unit code)   1               `then here g`          `corolla A B`
--   Σ̂ (the former)  `sum`           path substitution      graph substitution
--   ⟦Σ̂⟧             `sumΣ`          `pair`/positions       `verts-graft`, general
--   Inj             `Fin`-injective  see the refutation     `Rigid.canon-sound`
--
-- The interpretation is the VERTEX set at the circuit rung, not the leg set,
-- and `Gandr.Shape.Graft.verts-graft` — grafting concatenates the vertex
-- listings — is already `⟦Σ̂⟧` at the binary rung. That identification is what
-- makes the row above a reading rather than a hope.
--
-- ── THREE THINGS THE PUBLISHED RECORD DOES NOT HAVE, AND ALL THREE ARE FORCED
-- The published parameterization varies the SYMMETRY axis (ordered versus
-- unordered codes) and FIXES the arity-shape axis at a dependent sum. gandr
-- needs the second axis varied and settles the first with `Rigid`. Three
-- divergences follow, none of them a matter of taste:
--
--   * **The codes are INDEXED by their interface.** `Code : Ifc → Ifc → Set`,
--     not `Code : Set`. This is the whole reason `Gandr.Shape.Graph` was
--     re-presented familially — "there was no index for an arity abstraction to
--     quantify over" — so it is settled tree policy rather than a choice made
--     here.
--
--   * **The unit is a FAMILY, not an element.** `unit : Gen a b → Code a b`.
--     Unindexed codes have one unit; a coloured rung has one per generator, and
--     at the circuit rung that is the corolla family.
--
--   * **The positions are LABELLED, so the interpretation is a family too.**
--     `Pos : Code a b → Ifc → Ifc → Set` — the positions spanning a given
--     interface. Substitution needs the label to TYPE its argument, and the
--     right-unit law needs it to name which unit goes where. Hewer's positions
--     are bare because his codes are bare finite sets.
--
-- And one consequence of the first divergence is worth stating on its own,
-- because it deletes three of the eleven published fields. THE THREE PATH-LEVEL
-- CLOSURE LAWS DISAPPEAR. `⟦Σ̂Idl⟧`, `⟦Σ̂Idr⟧` and `⟦Σ̂Assoc⟧` exist so that the
-- unit and associativity equivalences on positions are REPRESENTABLE as paths
-- of codes, because there `Σ̂ A B` is a new code and the operad laws are
-- heterogeneous over it. Here substitution PRESERVES the index — `sub A B` and
-- `A` are both `Code a b` — so all three laws are homogeneous equations and
-- nothing has to be transported. The index does the job the three laws were
-- doing. That is the largest single saving the universe presentation makes at
-- gandr's rung, and it is bought by a decision the tree already took.
--
-- ── WHAT THE TWO MISSING FIELDS TURN OUT TO BE ──────────────────────────────
-- `Gandr.Arity.Path`'s interface owes `Same` and `Mul`. Under the dictionary:
--
--   * `Mul` — the multiplication's GRAPH, invented to keep a defined function
--     out of a matchable index, and costing nine relations in the graph kit —
--     is `Σ̂` together with `⟦Σ̂⟧`. Both presentations solve ONE problem: how to
--     state the laws without the defined multiplication appearing where
--     something has to match on it. The graph solves it by never computing;
--     the universe solves it by making the law's only moving part a bijection
--     of POSITIONS. `pair` below is that bijection's forward half, and it is
--     the only part the three laws consume.
--
--   * `Same` — the heterogeneous structural equality, introduced ad hoc and
--     then found to be "the equality the laws are stated at" — is `Inj`'s
--     CODOMAIN. `Inj` sends an isomorphism of interpretations to an
--     identification of codes; `Same` is the identification relation it lands
--     in, and `InjComp` is that relation's own coherence (the two-cell
--     obligation the univalence section carries).
--
-- So the record was short exactly two fields, and the published record has
-- names, laws and four worked instances for exactly those two.
--
-- ── AND `Inj` DOES NOT SURVIVE THE TRANSLATION, WHICH IS THE FINDING ────────
-- `Inj` is the one field this module does NOT provide, because in the published
-- form it is FALSE at both of gandr's rungs, and the two failures are different.
--
-- Every published instance — totally-ordered finite sets, the groupoid of
-- finite sets and bijections, `Type`, the `n`-types — is a universe whose code
-- IS its interpretation. The paper says so: the codes' path spaces "precisely
-- reflect the path spaces of the underlying types, and no more." At a rung
-- where a code carries structure its position family does not determine, the
-- premise of `Inj` is too weak to conclude anything.
--
--   * **At the linear kit the surplus is ORDER, and it is recoverable.**
--     `naive-rep-refuted` below refutes `Inj` outright, at a hypothesis made
--     deliberately STRONGER than the published one — a bijection of positions
--     that also preserves labels — by exhibiting two paths with the same
--     labelled positions in the other sequence. The repair is to enrich the
--     interpretation to the ORDERED labelled positions, and it is carried out:
--     `Refines` is the enriched map, `inj` is the representation map over it,
--     and `inj-sound` is the converse.
--
--     **The two together bracket the enrichment rather than gesturing at it,
--     and the measurement is sharper than expected.** The refuted hypothesis
--     supplies a full bijection — two maps AND both round trips — plus label
--     preservation, and does not determine the code. The hypothesis proved
--     sufficient supplies two maps, NO round trips, plus labels and depth. So
--     order does not merely close the gap: at this kit it is worth more than
--     invertibility.
--
--   * **At the circuit kit the surplus is ORDER AGAIN, and it is NOT
--     recoverable.** Enriching to the richest available interpretation — the
--     graph's category of elements, vertices and legs and incidence — gives
--     isomorphism of graphs, and `Shape` stores an ordering that graph
--     isomorphism does not see. The refuters are already in the tree and were
--     never read as a statement about this: `Gandr.Shape.Graft.merge-swap-apart`
--     and `corollas-swap-apart` exhibit two pairs of shapes isomorphic as graphs
--     and decidably distinct as terms.
--
-- **So at the circuit rung `Inj` cannot land in `_≡_`, and this is forced by C3
-- rather than by the choice of ambient.** It must land in the code setoid's
-- relation, which is `Same`; and what makes that relation decidable is `canon`
-- with `canon-resp`/`canon-sound`. `Inj` IS `Rigid` at this rung — not "merges
-- with", but is — and specifically it is `canon-sound`, which is the tree's
-- open obligation D4. The universe presentation therefore does not dissolve
-- that obligation. It PROMOTES it: from a debt owed somewhere to a field every
-- instance must supply before the interface can be inhabited at all.
--
-- That is a gain, not a loss, and it is worth saying which kind. An obligation
-- that is a field is one the typechecker asks for; an obligation recorded in a
-- header is one a reader has to remember.
--
-- ── WHAT IT COST, AT THE KIT WHERE THE ANSWER IS KNOWN ──────────────────────
-- The control experiment's whole point is that the linear kit's answer is
-- already written, so a presentation that is not cheaper here will not be
-- cheaper where the answer is unknown.
--
--   what `Gandr.Arity.Path` has     what the universe presentation needs
--   ─────────────────────────────   ─────────────────────────────────────────
--   `Cat` (the graph)               `Place` (the positions)
--   `cat-graph` (totality)          — not needed
--   `cat-fun` (functionality)       — not needed AS A FIELD; used to derive
--   `cat-assoc`                     reused, through `cat-fun`, for `++-assoc`
--   `cat-idnˡ`                      reused, through `cat-fun`, for `idl`
--   `cat-whisk`                     — GONE
--   `Same`                          `Inj`'s codomain (not supplied here)
--
-- Two results, and the second is the one that bears on the graph kit.
--
--   * **`idl` and `assoc` are the existing lemmas, run through `cat-fun`.**
--     Nothing new is proved for them: `++-idnˡ` is `cat-idnˡ` and `++-assoc` is
--     `cat-assoc`, each read off the graph by functionality. The universe
--     presentation reuses the kit rather than replacing it.
--
--   * **`cat-whisk` has no counterpart, and that is the transferable finding.**
--     Whiskering is a compatibility the BINARY multiplication needs — moving one
--     operand past the other's interface — and substitution-at-all-positions
--     never moves anything past anything. In the graph kit five of the nine
--     operations (`lwhisk`, `wire-in`, `cap-in`, `insert-shift`,
--     `match-lwhisk`) exist for exactly that crossing. Whether they dissolve
--     under substitution or reappear as a partial trace was deferred from here;
--     what is decided here is that the linear kit's whiskering does dissolve,
--     which is the control the prediction needed.
--
--     **They split, and the disjunction was too clean.** Scoping the closure
--     (§*And that closure is now scoped* below) answers it: `lwhisk` and
--     `match-lwhisk` dissolve, since substitution never moves an operand past
--     an interface; `wire-in`, `cap-in` and `insert-shift` SURVIVE, but as the
--     merger's threading rather than as a crossing, which the interface owes
--     whatever the former is. And the partial trace does appear — as a NEW
--     operation and not as one of these five reappearing, because it is not a
--     crossing at all.
--
-- The residual cost is `sub-cat` — substitution distributes over concatenation
-- — which is new, and is the one place the presentation pays rather than
-- reuses. It is the price of stating associativity over positions instead of
-- over witnesses.
--
-- ── WHAT IS NOT HERE ────────────────────────────────────────────────────────
-- The circuit instance's three LAWS, and the representation map, which at that
-- rung is `canon-sound`. The former itself is here: `sub` and `pair` are built
-- below, over the two-sided closure. An earlier revision of this paragraph
-- recorded the whole of `Subst` and everything downstream of it as the
-- residual.
--
-- Everything else of Definition 9's former half is inhabited at the linear kit,
-- including the splitting half of `⟦Σ̂⟧` (`place-split`) — the published field
-- asserts an equivalence and the three laws consume only the pairing, so the
-- splitting is what makes the instance inhabit the field as stated rather than
-- a weakening of it.
--
-- `InjComp` is not stated. Its content is that the representation map preserves
-- composition, and at this kit that is `same-seq` — `Same`'s transitivity, now
-- proved in `Gandr.Arity.Path` — so what is missing is the packaging and not
-- the fact. It is left until a consumer needs the two-cell form, because that
-- is where the layout univalence map's own coherence will fix its shape.
------------------------------------------------------------------------------

module Gandr.Arity.Universe where

open import Agda.Primitive
  using (lsuc)
open import Gandr.Arity.Path
  using (Path)
  using (here)
  using (then)
  using (cat-graph)
  using (cat-fun)
  using (cat-assoc)
  using (cat-idnˡ)
  using (Same)
  using (here≈)
  using (then≈)
  using (same-inv)
open import Gandr.Setoid
  using (≡ˢ)
  using (bundle)
  using (step-≈·)
open import Data.Bool
  using (Bool)
  using (true)
  using (false)
open import Data.Empty
  using (⊥)
  using (⊥-elim)
open import Data.Nat
  using (ℕ)
  using (zero)
  using (suc)
  using (_+_)
open import Data.Nat.Properties
  using (suc-injective)
open import Data.Product
  using (_×_)
  using (_,_)
  using (proj₁)
  using (proj₂)
open import Data.Unit
  using (⊤)
  using (tt)
open import Relation.Binary.PropositionalEquality
  using (_≡_)
  using (refl)
  using (sym)
  using (trans)
  using (cong)
  using (subst)

-- The reasoning vocabulary, on `Gandr.Shape.Graft`'s terms: `bundle (≡ˢ _)` is
-- the `Set`-level bundle, so a chain here reads as a chain over a hom-setoid.
open import Relation.Binary.Reasoning.MultiSetoid

------------------------------------------------------------------------------
-- THE INTERFACE. Definition 9's former half, indexed, labelled, and with the
-- three path-level closure laws deleted by the index. `Inj` and `InjComp` are
-- deliberately absent — see the header.
------------------------------------------------------------------------------

record Arity o : Set (lsuc o) where
  field
    -- the interfaces a code can span
    Ifc : Set o
    -- the codes: `T` applied to the one-point species, indexed by interface
    Code : Ifc → Ifc → Set o
    -- the interpretation: a code's positions, each spanning an interface of
    -- its own. Labelled and indexed, because substitution has to TYPE the
    -- family it substitutes
    Pos : ∀ {a b} → Code a b → Ifc → Ifc → Set o
    -- the generators a position can be labelled by
    Gen : Ifc → Ifc → Set o
    -- and what sits at a position. Hewer's positions carry nothing; a coloured
    -- rung's do, and the right unit law is what needs it
    lab : ∀ {a b} {A : Code a b} {c d} → Pos A c d → Gen c d
    -- `⊤̂`, as a family: one unit code per generator
    unit : ∀ {a b} → Gen a b → Code a b
    -- `Σ̂`, the arity former: substitute a code at every position of a code.
    -- This is the monad multiplication in polynomial form, and it is the
    -- primitive the published record fixes at a dependent sum
    sub
      : ∀ {a b}
      → (A : Code a b)
      → (∀ {c d} → Pos A c d → Code c d)
      → Code a b

    -- `⟦⊤̂⟧`, one half: the unit code's position
    one : ∀ {a b} {g : Gen a b} → Pos (unit g) a b
    -- and the other half, as an ELIMINATOR rather than as an equivalence,
    -- because the index is what would otherwise have to be deleted: every
    -- position of a unit code is that one
    one-elim
      : ∀ {a b} {g : Gen a b}
      → (P : ∀ {c d} → Pos (unit g) c d → Set o)
      → P (one {g = g})
      → ∀ {c d} (i : Pos (unit g) c d) → P i

    -- `⟦Σ̂⟧`, the forward half: a position of a substitution is an outer
    -- position paired with an inner one. This is the field that replaces the
    -- graph-of-multiplication, and it is all three laws consume
    pair
      : ∀ {a b} {A : Code a b}
      → (B : ∀ {c d} → Pos A c d → Code c d)
      → ∀ {c d e f}
      → (i : Pos A c d)
      → Pos (B i) e f
      → Pos (sub A B) e f

    -- LEFT UNIT: substituting into the unit code is the substituted code.
    -- Homogeneous, because the index is preserved
    idl
      : ∀ {a b} {g : Gen a b}
      → (B : ∀ {c d} → Pos (unit g) c d → Code c d)
      → sub (unit g) B ≡ B (one {g = g})
    -- RIGHT UNIT: substituting the unit at every position changes nothing. This
    -- is the law that needs `lab`
    idr
      : ∀ {a b}
      → (A : Code a b)
      → sub A (λ i → unit (lab i)) ≡ A
    -- ASSOCIATIVITY, stated over `pair` rather than over a witness. This is
    -- where the interpretation equivalence is load-bearing: without it the
    -- outer substitution's family cannot be RETYPED as a family over pairs, and
    -- the law cannot be written at all
    assoc
      : ∀ {a b}
      → (A : Code a b)
      → (B : ∀ {c d} → Pos A c d → Code c d)
      → (C : ∀ {c d} → Pos (sub A B) c d → Code c d)
      → sub (sub A B) C ≡ sub A (λ i → sub (B i) (λ j → C (pair B i j)))

------------------------------------------------------------------------------
-- THE LINEAR INSTANCE. Codes are paths, positions are edges, the unit code is a
-- one-edge path, and substitution replaces each edge by a path.
------------------------------------------------------------------------------

module Linear {o} (Ob : Set o) (Step : Ob → Ob → Set o) where

  -- PATH concatenation, imported HERE rather than at the top of the file
  -- because the circuit instance below needs the LIST concatenation under the
  -- same name. Per-module repackaging, on the tree's own rule for a
  -- vocabulary that is local to one consumer.
  open import Gandr.Arity.Path
    using (_++_)

  -- the codes, abbreviated the way a consumer reads them
  Pth : Ob → Ob → Set o
  Pth = Path Ob Step

  -- A POSITION of a path: one of its edges, indexed by the interface that edge
  -- spans. Indexed rather than bare, so that the substituted family types
  -- itself and no transport appears in any law.
  data Place : ∀ {a b} → Pth a b → Ob → Ob → Set o where
    -- the path's last edge
    last
      : ∀ {a m c}
      → {p : Pth a m} {g : Step m c}
      → Place (then p g) m c
    -- a position further back
    push
      : ∀ {a m c n d}
      → {p : Pth a m} {g : Step m c}
      → Place p n d
      → Place (then p g) n d

  -- What sits at a position. The generators of the linear kit are its edges.
  lab : ∀ {a b} {p : Pth a b} {c d} → Place p c d → Step c d
  lab (last {g}) = g
  lab (push i) = lab i

  -- The unit code: the one-edge path. `⊤̂` is a FAMILY here, one per edge.
  unit : ∀ {a b} → Step a b → Pth a b
  unit g = then here g

  -- SUBSTITUTION, by recursion on the outer path: replace each edge by the path
  -- chosen for it, and concatenate. This is the monad multiplication, and it is
  -- three lines because nothing has to be whiskered past anything.
  sub
    : ∀ {a b}
    → (p : Pth a b)
    → (∀ {c d} → Place p c d → Pth c d)
    → Pth a b
  sub here q = here
  sub (then p g) q = sub p (λ i → q (push i)) ++ q last

  -- ══════════════════════════════════════════════════════════════════════════
  -- `⟦⊤̂⟧`. The unit code has exactly one position, said as an eliminator.
  -- ══════════════════════════════════════════════════════════════════════════

  one : ∀ {a b} {g : Step a b} → Place (unit g) a b
  one = last

  -- Every position of a unit code is that one. The `push` case is absurd
  -- because the empty path has no position, and the absurdity is READ OFF the
  -- index rather than deleted from it.
  one-elim
    : ∀ {a b} {g : Step a b}
    → (P : ∀ {c d} → Place (unit g) c d → Set o)
    → P (one {g = g})
    → ∀ {c d} (i : Place (unit g) c d) → P i
  one-elim P x last = x
  one-elim P x (push ())

  -- ══════════════════════════════════════════════════════════════════════════
  -- `⟦Σ̂⟧`. A position of a substitution is an outer position paired with an
  -- inner one. Both injections into a concatenation are label-preserving on the
  -- nose, which is what keeps every law homogeneous.
  -- ══════════════════════════════════════════════════════════════════════════

  -- a position of the left factor, seen in the concatenation
  cat-inl
    : ∀ {a m b} {x : Pth a m}
    → (y : Pth m b)
    → ∀ {c d} → Place x c d → Place (x ++ y) c d
  cat-inl here i = i
  cat-inl (then y h) i = push (cat-inl y i)

  -- and of the right factor
  cat-inr
    : ∀ {a m b}
    → (x : Pth a m)
    → ∀ {y : Pth m b} {c d} → Place y c d → Place (x ++ y) c d
  cat-inr x last = last
  cat-inr x (push j) = push (cat-inr x j)

  -- The pairing. Recursion on the OUTER position, so each clause is one
  -- injection: the last edge's replacements land on the right of the
  -- concatenation, everything else on the left.
  pair
    : ∀ {a b} {p : Pth a b}
    → (q : ∀ {c d} → Place p c d → Pth c d)
    → ∀ {c d e f}
    → (i : Place p c d)
    → Place (q i) e f
    → Place (sub p q) e f
  pair q last j = cat-inr (sub _ (λ i → q (push i))) j
  pair q (push i) j = cat-inl (q last) (pair (λ k → q (push k)) i j)

  -- AND THE SPLITTING HALF, so the interpretation law is an equivalence rather
  -- than a map. The published field asserts an equivalence; the three laws
  -- consume only the pairing, so this is what makes the instance inhabit the
  -- field as stated rather than a weakening of it.
  --
  -- It is written as a VIEW throughout — an inductive family whose index is the
  -- position being viewed — so a consumer learns the decomposition by matching
  -- the view and never by inverting a defined function in an index.

  -- Which factor of a concatenation a position came from.
  data CatSide {a m b} (x : Pth a m) (y : Pth m b)
    : ∀ {c d} → Place (x ++ y) c d → Set o where
    left
      : ∀ {c d}
      → (i : Place x c d)
      → CatSide x y (cat-inl y i)
    right
      : ∀ {c d}
      → (j : Place y c d)
      → CatSide x y (cat-inr x j)

  -- Carrying the view past one more edge of the right factor. Written as an
  -- application rather than a `with` on the recursive call, so the call stays a
  -- visible subterm — the module-wide rule that `split`, `ends` and `route`
  -- follow in the shape kit.
  cat-view-push
    : ∀ {a m n b} {x : Pth a m} {y : Pth m n} {h : Step n b} {c d}
    → {k : Place (x ++ y) c d}
    → CatSide x y k
    → CatSide x (then y h) (push k)
  cat-view-push (left i) = left i
  cat-view-push (right j) = right (push j)

  cat-view
    : ∀ {a m b}
    → (x : Pth a m) (y : Pth m b)
    → ∀ {c d} (k : Place (x ++ y) c d)
    → CatSide x y k
  cat-view x here k = left k
  cat-view x (then y h) last = right last
  cat-view x (then y h) (push k) = cat-view-push (cat-view x y k)

  -- A position of a substitution, decomposed: which outer position it came
  -- from, which inner position of the code substituted there, and that pairing
  -- them recovers it.
  record Split {a b} (p : Pth a b)
               (q : ∀ {c d} → Place p c d → Pth c d)
               {e f} (k : Place (sub p q) e f) : Set o where
    constructor split
    field
      -- the interface the outer position spans
      {oc} : Ob
      {od} : Ob
      -- the outer position
      out : Place p oc od
      -- and the inner one, in the code substituted there
      inn : Place (q out) e f
      -- which is the position we started from
      spot : pair q out inn ≡ k

  -- The recursive step, taking the tail's decomposition rather than computing
  -- it, so the recursion below stays structural in the outer code.
  split-cat
    : ∀ {a m b} {p′ : Pth a m} {g : Step m b}
    → (q : ∀ {c d} → Place (then p′ g) c d → Pth c d)
    → ∀ {e f} {k : Place (sub p′ (λ z → q (push z)) ++ q last) e f}
    → CatSide (sub p′ (λ z → q (push z))) (q last) k
    → (∀ {e′ f′} (i : Place (sub p′ (λ z → q (push z))) e′ f′)
       → Split p′ (λ z → q (push z)) i)
    → Split (then p′ g) q k
  split-cat q (left i) rec =
    split
      (push (Split.out (rec i)))
      (Split.inn (rec i))
      (cong (λ z → cat-inl (q last) z) (Split.spot (rec i)))
  split-cat q (right j) rec = split last j refl

  place-split
    : ∀ {a b}
    → (p : Pth a b) (q : ∀ {c d} → Place p c d → Pth c d)
    → ∀ {e f} (k : Place (sub p q) e f)
    → Split p q k
  place-split here q ()
  place-split (then p′ g) q k =
    split-cat
      q
      (cat-view (sub p′ (λ z → q (push z))) (q last) k)
      (λ i → place-split p′ (λ z → q (push z)) i)

  -- ══════════════════════════════════════════════════════════════════════════
  -- THE TWO REUSES. `Gandr.Arity.Path` states the unit and associativity laws
  -- on the GRAPH; functionality reads them back off as equations on the
  -- function, and that is all the universe presentation needs of them.
  -- ══════════════════════════════════════════════════════════════════════════

  -- the left unit of concatenation, as an equation — `cat-idnˡ` through
  -- `cat-fun`
  ++-idnˡ
    : ∀ {a b}
    → (y : Pth a b)
    → here ++ y ≡ y
  ++-idnˡ y = cat-fun (cat-graph here y) (cat-idnˡ y)

  -- and associativity, likewise — `cat-assoc` through `cat-fun`
  ++-assoc
    : ∀ {a b c d}
    → (x : Pth a b) (y : Pth b c) (z : Pth c d)
    → (x ++ y) ++ z ≡ x ++ (y ++ z)
  ++-assoc x y z =
    cat-fun
      (cat-assoc (cat-graph x y) (cat-graph (x ++ y) z) (cat-graph y z))
      (cat-graph x (y ++ z))

  -- ══════════════════════════════════════════════════════════════════════════
  -- THE THREE LAWS.
  -- ══════════════════════════════════════════════════════════════════════════

  -- LEFT UNIT. Substituting into the one-edge path is `here ++ _`, so this law
  -- IS `cat-idnˡ` and nothing else. That is the sharpest single datum the
  -- control experiment produces.
  idl
    : ∀ {a b} {g : Step a b}
    → (q : ∀ {c d} → Place (unit g) c d → Pth c d)
    → sub (unit g) q ≡ q one
  idl q = ++-idnˡ (q last)

  -- RIGHT UNIT. One `cong`: the right unit of concatenation is definitional, so
  -- the recursive clause reduces to the induction hypothesis under `then`.
  idr
    : ∀ {a b}
    → (p : Pth a b)
    → sub p (λ i → unit (lab i)) ≡ p
  idr here = refl
  idr (then p g) = cong (λ z → then z g) (idr p)

  -- SUBSTITUTION DISTRIBUTES OVER CONCATENATION. The one genuinely new lemma,
  -- and the price of stating associativity over positions rather than over
  -- witnesses.
  sub-cat
    : ∀ {a m b}
    → (x : Pth a m) (y : Pth m b)
    → (r : ∀ {c d} → Place (x ++ y) c d → Pth c d)
    → sub (x ++ y) r
      ≡ sub x (λ i → r (cat-inl y i)) ++ sub y (λ j → r (cat-inr x j))
  sub-cat x here r = refl
  sub-cat x (then y h) r =
    begin⟨ bundle (≡ˢ _) ⟩
      [ z ↦ z ++ r last ]· sub (x ++ y) (λ k → r (push k))
    ≈·⟨ sub-cat x y (λ k → r (push k)) ⟩
      (sub x (λ i → r (push (cat-inl y i)))
        ++ sub y (λ j → r (push (cat-inr x j))))
        ++ r last
    ≈⟨ ++-assoc
         (sub x (λ i → r (push (cat-inl y i))))
         (sub y (λ j → r (push (cat-inr x j))))
         (r last)
     ⟩
      sub x (λ i → r (push (cat-inl y i)))
        ++ (sub y (λ j → r (push (cat-inr x j))) ++ r last)
    ∎

  -- ASSOCIATIVITY. Distribute, then apply the induction hypothesis to the left
  -- factor; the right factor matches definitionally, because `pair`'s two
  -- clauses ARE the two injections the distribution produced.
  assoc
    : ∀ {a b}
    → (p : Pth a b)
    → (q : ∀ {c d} → Place p c d → Pth c d)
    → (r : ∀ {c d} → Place (sub p q) c d → Pth c d)
    → sub (sub p q) r ≡ sub p (λ i → sub (q i) (λ j → r (pair q i j)))
  assoc here q r = refl
  assoc (then p g) q r =
    begin⟨ bundle (≡ˢ _) ⟩
      sub (sub p (λ i → q (push i)) ++ q last) r
    ≈⟨ sub-cat (sub p (λ i → q (push i))) (q last) r ⟩
      [ z ↦ z ++ sub (q last) (λ j → r (cat-inr (sub p (λ i → q (push i))) j)) ]·
        sub (sub p (λ i → q (push i))) (λ i → r (cat-inl (q last) i))
    ≈·⟨ assoc p (λ i → q (push i)) (λ i → r (cat-inl (q last) i)) ⟩
      sub p (λ i → sub (q (push i))
                     (λ j → r (cat-inl (q last) (pair (λ k → q (push k)) i j))))
        ++ sub (q last) (λ j → r (cat-inr (sub p (λ i → q (push i))) j))
    ∎

  -- ══════════════════════════════════════════════════════════════════════════
  -- THE ENRICHED INTERPRETATION, AND THE REPRESENTATION MAP IT RECOVERS.
  --
  -- The published representation map is refuted below against the BARE
  -- interpretation. This is its repair, and the two together bracket the
  -- enrichment exactly rather than gesturing at it: what the bare
  -- interpretation is missing is the ORDER, and nothing else.
  --
  -- The contrast is sharp enough to be worth stating as a measurement. The
  -- refuted hypothesis supplies a bijection — two maps AND both round trips —
  -- that preserves labels, and it does not determine the code. The hypothesis
  -- proved sufficient here supplies two maps, NO round trips, and preserves
  -- labels and depth. So order is not merely sufficient to close the gap; it is
  -- worth more than invertibility.
  -- ══════════════════════════════════════════════════════════════════════════

  -- A position's index into the sequence. This is the enrichment: the bare
  -- interpretation is the positions with their labels, and the enriched one
  -- remembers where in the order each position sits.
  depth : ∀ {a b} {p : Pth a b} {c d} → Place p c d → ℕ
  depth last = zero
  depth (push i) = suc (depth i)

  -- A map of interpretations respecting the labelling AND the order.
  record Refines {a b b′} (p : Pth a b) (q : Pth a b′) : Set o where
    field
      -- the map on positions
      map : ∀ {c d} → Place p c d → Place q c d
      -- which carries each position's label
      map-lab : ∀ {c d} (i : Place p c d) → lab (map i) ≡ lab i
      -- and its place in the order
      map-depth : ∀ {c d} (i : Place p c d) → depth (map i) ≡ depth i

  -- The empty path has no position, which is what rules out a code refining one
  -- of a different length.
  place-here : ∀ {a c d} → Place {a} {a} here c d → ⊥
  place-here ()

  -- A position at depth zero is the outermost one. This is an ELIMINATOR rather
  -- than an equation because what the outermost case needs from it is the INDEX
  -- identification — that the two codes' last steps span the same interface —
  -- and an equation at fixed indices cannot carry that.
  place-last
    : ∀ {a m b} {y : Pth a m} {h : Step m b}
    → (P : ∀ {c d} → Place (then y h) c d → Set o)
    → P (last {p = y} {g = h})
    → ∀ {c d} (k : Place (then y h) c d)
    → depth k ≡ zero
    → P k
  place-last P x last _ = x
  place-last P x (push i) ()

  -- A position at positive depth is a `push`, packaged with the equation that
  -- says so, since the caller has to rewrite both the label and the depth
  -- through it.
  record Tail {a m b} (y : Pth a m) (h : Step m b) {c d}
              (k : Place (then y h) c d) : Set o where
    constructor tail
    field
      -- the position one layer in
      pos : Place y c d
      -- and that it is the one
      spot : k ≡ push pos

  place-tail
    : ∀ {a m b} {y : Pth a m} {h : Step m b} {c d n}
    → (k : Place (then y h) c d)
    → depth k ≡ suc n
    → Tail y h k
  place-tail last ()
  place-tail (push i) _ = tail _ refl

  -- Restricting a refinement to the tails, which is what makes the recursion
  -- below structural. Every position other than the outermost has positive
  -- depth, so it lands on a `push` and its target is read off there.
  private
    tail-of
      : ∀ {a m b m′ b′}
      → {p′ : Pth a m} {g : Step m b} {q′ : Pth a m′} {h : Step m′ b′}
      → (r : Refines (then p′ g) (then q′ h))
      → ∀ {c d} (i : Place p′ c d)
      → Tail q′ h (Refines.map r (push i))
    tail-of r i =
      place-tail (Refines.map r (push i)) (Refines.map-depth r (push i))

  refines-tail
    : ∀ {a m b m′ b′}
    → {p′ : Pth a m} {g : Step m b} {q′ : Pth a m′} {h : Step m′ b′}
    → Refines (then p′ g) (then q′ h)
    → Refines p′ q′
  refines-tail r .Refines.map i = Tail.pos (tail-of r i)
  refines-tail r .Refines.map-lab i =
    trans
      (sym (cong lab (Tail.spot (tail-of r i))))
      (Refines.map-lab r (push i))
  refines-tail r .Refines.map-depth i =
    suc-injective
      (trans
        (sym (cong depth (Tail.spot (tail-of r i))))
        (Refines.map-depth r (push i)))

  -- THE REPRESENTATION MAP, at the linear kit. An enriched interpretation
  -- determines the code up to the code setoid's relation, which is all a setoid
  -- presentation ever asks of it — and `Same` is that relation, so this is the
  -- field the published record could not supply here.
  inj
    : ∀ {a b b′}
    → (p : Pth a b) (q : Pth a b′)
    → Refines p q
    → Refines q p
    → Same p q
  inj here here r r⁻ = here≈
  inj here (then q′ h) r r⁻ = ⊥-elim (place-here (Refines.map r⁻ last))
  inj (then p′ g) here r r⁻ = ⊥-elim (place-here (Refines.map r last))
  inj {a} (then p′ g) (then q′ h) r r⁻ =
    place-last
      (λ {c} {d} k →
        (p″ : Pth a c) (g″ : Step c d)
        → Same p″ q′
        → lab k ≡ g″
        → Same (then p″ g″) (then q′ h))
      (λ p″ g″ s e → then≈ s (sym e))
      (Refines.map r last)
      (Refines.map-depth r last)
      p′
      g
      (inj p′ q′ (refines-tail r) (refines-tail r⁻))
      (Refines.map-lab r last)

  -- And the converse, so the characterization is exact rather than one-sided:
  -- the code relation gives back the enriched interpretation's map.
  same-map
    : ∀ {a b b′} {p : Pth a b} {q : Pth a b′}
    → Same p q
    → ∀ {c d} → Place p c d → Place q c d
  same-map here≈ ()
  same-map (then≈ s e) last = last
  same-map (then≈ s e) (push i) = push (same-map s i)

  same-map-lab
    : ∀ {a b b′} {p : Pth a b} {q : Pth a b′}
    → (s : Same p q)
    → ∀ {c d} (i : Place p c d)
    → lab (same-map s i) ≡ lab i
  same-map-lab here≈ ()
  same-map-lab (then≈ s e) last = sym e
  same-map-lab (then≈ s e) (push i) = same-map-lab s i

  same-map-depth
    : ∀ {a b b′} {p : Pth a b} {q : Pth a b′}
    → (s : Same p q)
    → ∀ {c d} (i : Place p c d)
    → depth (same-map s i) ≡ depth i
  same-map-depth here≈ ()
  same-map-depth (then≈ s e) last = refl
  same-map-depth (then≈ s e) (push i) = cong suc (same-map-depth s i)

  same-refines
    : ∀ {a b b′} {p : Pth a b} {q : Pth a b′}
    → Same p q
    → Refines p q
  same-refines s .Refines.map = same-map s
  same-refines s .Refines.map-lab = same-map-lab s
  same-refines s .Refines.map-depth = same-map-depth s

  -- So at this kit the enriched interpretation and the code relation determine
  -- each other, which is the precise form of "the interpretation is the code".
  -- The circuit kit is where this fails and no enrichment repairs it, because
  -- there the surplus is the vertex ordering rather than a position order.
  inj-sound
    : ∀ {a b b′} {p : Pth a b} {q : Pth a b′}
    → Same p q
    → Refines p q × Refines q p
  inj-sound s = same-refines s , same-refines (same-inv s)

  -- The instance. Every field of the former half, at the kit whose answer was
  -- already written.
  linear : Arity o
  linear .Arity.Ifc = Ob
  linear .Arity.Code = Pth
  linear .Arity.Pos = Place
  linear .Arity.Gen = Step
  linear .Arity.lab = lab
  linear .Arity.unit = unit
  linear .Arity.sub = sub
  linear .Arity.one = one
  linear .Arity.one-elim = one-elim
  linear .Arity.pair = pair
  linear .Arity.idl = idl
  linear .Arity.idr = idr
  linear .Arity.assoc = assoc

------------------------------------------------------------------------------
-- THE REFUTATION. `Inj` is the field this module does not supply, and the
-- reason is not that it is hard: in its published form it is FALSE here.
--
-- The hypothesis refuted is deliberately STRONGER than the published one. `Inj`
-- asks only for an equivalence of interpretations; this asks for one that also
-- preserves LABELS, which is strictly more. It is still false, and the witness
-- says why in one word: the two paths have the same labelled positions in the
-- other ORDER, and a bijection cannot see an order.
--
-- The circuit-rung counterpart is already in the tree and needs nothing built:
-- `Gandr.Shape.Graft.merge-swap-apart` and `corollas-swap-apart` exhibit shapes
-- isomorphic as graphs and decidably distinct as terms. There the surplus is
-- not recoverable by enriching the interpretation — it is the vertex ordering,
-- which is representation content under C3 — so `Inj` must land in the code
-- SETOID's relation, and what decides that relation is `canon-sound`.
------------------------------------------------------------------------------

module Refute where

  open Linear ⊤ (λ _ _ → Bool)

  -- The published representation map, translated as favourably as possible:
  -- a bijection of position families that also preserves labels identifies the
  -- codes.
  NaiveRep : Set
  NaiveRep =
      ∀ {a b}
    → (x y : Pth a b)
    → (to : ∀ {c d} → Place x c d → Place y c d)
    → (from : ∀ {c d} → Place y c d → Place x c d)
    → (∀ {c d} (i : Place x c d) → from (to i) ≡ i)
    → (∀ {c d} (j : Place y c d) → to (from j) ≡ j)
    → (∀ {c d} (i : Place x c d) → lab (to i) ≡ lab i)
    → x ≡ y

  -- Two edges on one object, and the two paths that use both in either order.
  twiceᵃ : Pth tt tt
  twiceᵃ = then (then here true) false

  twiceᵇ : Pth tt tt
  twiceᵇ = then (then here false) true

  -- The bijection: each path's last position is the other's earlier one.
  swapᵃ : ∀ {c d} → Place twiceᵃ c d → Place twiceᵇ c d
  swapᵃ last = push last
  swapᵃ (push last) = last
  swapᵃ (push (push ()))

  swapᵇ : ∀ {c d} → Place twiceᵇ c d → Place twiceᵃ c d
  swapᵇ last = push last
  swapᵇ (push last) = last
  swapᵇ (push (push ()))

  swapᵃᵇ : ∀ {c d} (i : Place twiceᵃ c d) → swapᵇ (swapᵃ i) ≡ i
  swapᵃᵇ last = refl
  swapᵃᵇ (push last) = refl
  swapᵃᵇ (push (push ()))

  swapᵇᵃ : ∀ {c d} (j : Place twiceᵇ c d) → swapᵃ (swapᵇ j) ≡ j
  swapᵇᵃ last = refl
  swapᵇᵃ (push last) = refl
  swapᵇᵃ (push (push ()))

  -- and it preserves labels, which is the strengthening
  swap-lab : ∀ {c d} (i : Place twiceᵃ c d) → lab (swapᵃ i) ≡ lab i
  swap-lab last = refl
  swap-lab (push last) = refl
  swap-lab (push (push ()))

  -- The two paths are nevertheless distinct, and one projection sees it.
  outer : ∀ {a b} → Pth a b → Bool
  outer here = true
  outer (then _ g) = g

  apart : twiceᵃ ≡ twiceᵇ → ⊥
  apart e with cong outer e
  ... | ()

  -- So the published representation map does not translate. The interpretation
  -- has to be enriched before anything of this shape can be asked for.
  naive-rep-refuted : NaiveRep → ⊥
  naive-rep-refuted rep =
    apart (rep twiceᵃ twiceᵇ swapᵃ swapᵇ swapᵃᵇ swapᵇᵃ swap-lab)

------------------------------------------------------------------------------
-- THE CIRCUIT KIT, MEASURED RATHER THAN ESTIMATED. This is spike S11, whose
-- question is whether the universe presentation is a simplification or a
-- rename at the rung where the answer is not already known.
--
-- The roadmap prices S11 as a hand count of "the coherence laws the graph
-- former needs". S10's result makes the count structural instead, and the
-- answer has two halves that pull in opposite directions.
--
-- ── THE LAWS ARE THREE, AND THE DECIDING RISK DOES NOT FIRE ─────────────────
-- The former is SUBSTITUTION — the monad multiplication in polynomial form —
-- and not the binary grafting the record was written against. A multiplication
-- has three laws whatever it multiplies, because they are the monad laws; and
-- gandr's codes are indexed by their interface, so substitution preserves the
-- index and all three are homogeneous. Neither fact depends on the rung.
--
-- **So the falsifier the roadmap names — "the graph former's coherence laws do
-- not stay finite" — does not fire, and it could not have.** The published
-- record's three laws are three for a reason that survives replacing the
-- former, and the three PATH-LEVEL laws beside them are deleted by the index
-- rather than reproduced. What the count cannot settle is whether the former is
-- cheap to BUILD, and that is where the cost actually went; see below.
--
-- ── EIGHT OF THIRTEEN FIELDS ARE INHABITED TODAY, AND THE REST DESCEND FROM
-- ── ONE CONSTRUCTION
-- Everything below is supplied against the tree as it stands. The interpretation
-- is the vertex family, indexed by the profile it spans, which is what typing
-- the substituted family requires.
--
-- **It is DERIVED, not declared, and that is the answer to whether the vertex
-- family should be re-presented.** The two addressings — a position with the
-- profile read back by `lookup`, and a position with the profile in the index —
-- are the same positions, so the refinement belongs to the LISTING rather than
-- to the shape. `Gandr.Shape.Graph.Occ` is `Ix` with its element carried,
-- `Vtxᶠ` is its instance at the vertex listing, and there is no second
-- induction over `Shape` anywhere.
--
-- **Neither addressing replaces the other, and the rule is stated at `Occ`.**
-- Every consumer of `Vtx` in the shape kit — reachability, adjacency, the
-- incidence, the rank structure, connectivity — uses it as a bare position, and
-- several RELATE two vertices of one shape; profile indices there are four
-- extra arguments carrying nothing. Typing a datum at a position is the one job
-- that needs the element in the index, and it is why `Occ` exists.
--
--   field       status at the circuit kit
--   ─────────   ───────────────────────────────────────────────────────────────
--   Ifc         `List Ob`
--   Code        `Shape`
--   Pos         `Vtxᶠ` — the listing-occurrence family at the vertex listing
--   Gen         `⊤`, one generator per profile: the corolla
--   lab         trivial, for the same reason
--   unit        `corolla`
--   one         `top`
--   one-elim    `corolla-elim` — proved
--   sub         `sub` — built, over `plug` and the two block iterations
--   pair        `pair` — built, and it is `verts-plug` read at a position
--   ─────────   ───────────────────────────────────────────────────────────────
--   idl idr     OWED — stated below, downstream of `sub`
--   assoc       OWED — stated below, and it owes the count law with it
--
-- An earlier revision of this table recorded `sub` and `pair` as owed.
--
-- And the interpretation side of the OWED half is already built at the binary
-- rung: `verts-graft`, `verts-merge`, `verts-lwhisk`, `verts-preplug`,
-- `verts-wire-in`, `verts-cap-in` and `verts-wires-in` are the seven landed
-- lemmas that constitute `⟦Σ̂⟧` for the operations the tree has. The
-- graph-of-multiplication the other presentation asks for — nine inductive
-- relations, with totality and functionality for each — has NONE of its
-- twenty-seven pieces built.
--
-- ── SO: SIMPLIFICATION ON THE SIDE THE RECORD MEASURED, NEUTRAL ON THE OTHER
-- §5.4's cost — "a multiplication spoken only through its graph is one relation
-- in one kit and nine in the other" — is a statement about the WITNESS
-- discipline, and the witness discipline is exactly what `⟦Σ̂⟧` replaces: one
-- bijection of positions in place of nine relations that thread one another's
-- indices. That saving is real and it is large.
--
-- It buys nothing on the CONSTRUCTION side. `Subst` still has to be defined,
-- and its cost is the listing algebra — matchings, insertions, exchanges —
-- which no presentation of the interface touches. Two things are known about
-- that cost and they run in opposite directions:
--
--   * **The outer recursion becomes trivial.** `graft (wires m) T` is
--     `preplug m T`, which needs `match-comp` — a WELL-FOUNDED recursion — and
--     `match-lwhisk`. Substitution's corresponding clause is the identity,
--     because a wiring has no vertex to substitute at: `vtx-wires` below is
--     that fact, and it is the circuit-rung counterpart of `cat-whisk`
--     disappearing at the linear kit.
--
--   * **The base case becomes harder.** Peeling a vertex off the outer graph
--     leaves a graph to be attached where the vertex's ports were published,
--     and its wiring clause closes a block of sources against a block of sinks
--     — a two-sided closure, which is what creates a wheel. `graft` never needs
--     one. That construction is the residual; it is scoped below as
--     `MatchClose` and `CloseIn`, and both are now BUILT — `match-close`,
--     `close-in`, and the block iterations `close-block` and `close-cross`
--     that carry them to a whole port block. An earlier revision of this
--     paragraph recorded them as owed.
--
--     AND THE FINDING THE BUILD ADDED, which the scoping did not predict: the
--     single-wire closure compares two positions a caller always supplies at
--     ONE colour, which is the first comparison in the kit whose `same` case
--     would have to delete a reflexive equation. Generalizing the two colours
--     and carrying `x ≡ y` as a datum removes the deletion rather than paying
--     an h-level condition for it, so the closure is `--without-K` at no cost
--     — see `close-hit`.
--
-- ── AND THAT CLOSURE IS NOW SCOPED, WHICH MOVED THE QUESTION ────────────────
-- The scoping was run to decide whether substitution is the primitive former or
-- stays derived from grafting. Three results, and the first is that the count
-- cannot decide it.
--
--   * **The closure is owed on EITHER route.** `sub` is a total field, and
--     substituting a graph at a vertex whose ports are wired back to each other
--     is not reachable from `graft` and `merge`, both of which leave the
--     block's two sides alone. So the operation the count was counting is
--     common, and what the count decides is what each route owes BEYOND it.
--
--   * **It decomposes as the merger plus one non-recursive operation**, and it
--     retires the kit's only well-founded recursion rather than adding to it —
--     see `MatchClose` below. Sixteen auxiliaries against `graft`'s ten, of
--     which twelve are already built: six of `graft`'s own and six of the
--     merger's, which the interface owes anyway. Four are new. The four not
--     needed are `preplug`, `lwhisk`, `match-lwhisk`, and `match-comp` with its
--     accumulator.
--
--     `graft`'s count is recorded as nine in `Gandr.Shape.Graft`; it is ten,
--     `match-unhit` having joined the chain when the cut did.
--
--   * **And the closure's degenerate case has no term in the carrier.**
--     `no-circle` below is that, checked rather than argued. What the carrier
--     excludes is a DERIVATION — a circle would need a cap composed with a cup,
--     and there is no cup — and what the closure needs is the object ADJOINED.
--     Those are different claims: a cup inhabits hom-sets that must stay empty,
--     while a circle consumes no port and produces none, so the downward
--     condition and the finiteness resting on it are untouched.
--
-- ── THE RULING (OWNER) ──────────────────────────────────────────────────────
-- SUBSTITUTION IS THE PRIMITIVE FORMER, and grafting is derived from it by
-- substituting into a two-corolla series shape — an ordinary term, needing no
-- grafting to build — so grafting associativity follows from the monad law.
-- The reason is not the count, which is common to both routes: the other route
-- inhabits `sub` only where no strand closes, which is a weakening of the
-- interface rather than the interface.
--
-- AND THE CIRCLES ARE COUNTED, NOT DISCARDED. `Code a b` is `Shape a b × ℕ`, a
-- shape with its number of closed components. This is the source's own
-- definition rather than a gandr device — a Brauer diagram IS a pairing
-- together with that count, additive under composition plus the components the
-- composition closes [@raynor-2025-functorial, Def 3.4] — and gandr's codes
-- were its OPEN fragment. Discarding is not the neutral option it resembles:
-- it sets the trace of every identity to the unit, at every colour, which is
-- `𝟙` where `𝟘` was available and leaves no term able to witness the other
-- answer.
--
-- The count sits at the CODE and not in `Match`, and that is a cost decision
-- rather than a fidelity one — the two are isomorphic, since a shape has one
-- wiring at the bottom. In `Match` it would ride inside every listing-algebra
-- lemma (`match-insert`, `match-cap`, the braid, `insert-swap-coh⁴`), none of
-- which can close a circle; at the code, exactly the operations that close one
-- touch it.
--
-- ── AND ONE OBLIGATION IS PROMOTED RATHER THAN DISCHARGED ───────────────────
-- `Inj` is not in the table because it is not inhabitable here at all: at this
-- rung it is `Rigid.canon-sound`, which is the tree's open D4. The refutation
-- above shows the published premise is too weak in general; `merge-swap-apart`
-- and `corollas-swap-apart` show that at THIS rung enriching the premise does
-- not rescue it, because the surplus is the vertex ordering and no
-- interpretation of a graph sees it. That is C3 meeting the interface, and it
-- is the honest price of the route.
------------------------------------------------------------------------------

module Circuit {ℓ} {Ob : Set ℓ} where

  open import Data.List
    using (List)
    using ([])
    using (_∷_)
    using (_++_)
  open import Gandr.Shape.Graph
    using (Shape)
    using (wires)
    using (node)
    using (Match)
    using (Prof)
    using (prof)
    using (corolla)
    using (Occ)
    using (hit)
    using (miss)
    using (occ-ix)
    using (occ-lookup)
    using (occ-inl)
    using (occ-inr)
    using (swap-match)
    using (Vtx)
    using (verts)
    using (ins)
    using (outs)
    -- for the closure's signature and for the refuter below
    using (Insert)
    using (head)
    using (Append)
    using (nil)
    using (cons)
    using (Wire)
    using (flow)
    using (edges)
    -- and for the construction: the concatenation graph the published port
    -- blocks are threaded past
    using (append-graph)
    -- the wiring constructors, renamed: `[]` and `_∷_` are the list
    -- constructors in this module
    renaming ([] to no-wire; _∷_ to _wired∷_)
  -- THE AUXILIARIES THE CONSTRUCTION REUSES. Twelve are already built: six of
  -- grafting's (`wire-in`, `match-insert`, `insert-shift`, `insert-swap`,
  -- `match-remove`, `match-unhit`) and six of the merger's (`merge`,
  -- `wires-in`, `append-regroup`, `insert-widen`, `cap-in`, `match-cap`). Six
  -- of those are reached THROUGH the other six rather than named here —
  -- `insert-swap` inside the two threadings, and `wire-in`, `cap-in`,
  -- `wires-in`, `append-regroup` and `insert-widen` inside `merge` — so the
  -- list below is what this module writes, not what the construction spends.
  --
  -- `preplug`, `lwhisk`, `match-lwhisk` and `match-comp` with its accumulator
  -- are deliberately absent: composing two wirings is merging them and closing
  -- the shared interface, which retires the kit's only well-founded recursion.
  open import Gandr.Shape.Graft
    using (match-insert)
    using (match-cap)
    using (match-remove)
    using (match-unhit)
    using (insert-shift)
    using (merge)
    -- and for the agreement lemma's series shape and its statement
    using (append-regroup)
    using (Regroup)
    using (graft)
    -- the two lookups' result types, and the view the through case splits on
    using (Removal)
    using (through)
    using (capped)
    using (Unhit)
    using (unhit)
    using (InsertView)
    using (same)
    using (apart)
    using (insert-view)
    -- and the incidence lemma the pairing is read off
    using (verts-merge)

  -- THE INTERPRETATION, familially — and DERIVED rather than declared, which is
  -- the answer to whether the vertex family should be re-presented.
  --
  -- `Gandr.Shape.Graph.Vtx S` is `Ix (verts S)`: a position in a listing, with
  -- the profile read back by `lookup`. The universe presentation needs the
  -- profile in the INDEX, because that is what types the family being
  -- substituted. Those are the same positions addressed two ways, so the
  -- refinement belongs to the LISTING and not to the shape: `Occ` is `Ix` with
  -- its element carried, and the vertex family is its instance at the vertex
  -- listing. There is no second induction over `Shape` here.
  --
  -- Neither presentation replaces the other, and the rule is in `Occ`'s own
  -- comment: order and vertex-to-vertex relations want `Ix`; typing a datum at
  -- a position wants `Occ`.
  Vtxᶠ : ∀ {Γ Δ} → Shape Ob Γ Δ → List Ob → List Ob → Set ℓ
  Vtxᶠ S c d = Occ (verts S) (prof c d)

  -- The view maps come with the generic family, so the three shape-specific
  -- bridge lemmas the declared form needed are one generic lemma and two
  -- congruences.
  toVtx : ∀ {Γ Δ} {S : Shape Ob Γ Δ} {c d} → Vtxᶠ S c d → Vtx S
  toVtx v = occ-ix v

  ins-toVtx
    : ∀ {Γ Δ} {S : Shape Ob Γ Δ} {c d}
    → (v : Vtxᶠ S c d)
    → ins (toVtx v) ≡ c
  ins-toVtx {S} v = cong Prof.dom (occ-lookup (verts S) v)

  outs-toVtx
    : ∀ {Γ Δ} {S : Shape Ob Γ Δ} {c d}
    → (v : Vtxᶠ S c d)
    → outs (toVtx v) ≡ d
  outs-toVtx {S} v = cong Prof.cod (occ-lookup (verts S) v)

  -- A WIRING HAS NO POSITION, because its vertex listing is empty. This is what
  -- makes substitution's outer recursion trivial where grafting's is a
  -- well-founded composition of matchings, and it is the circuit-rung
  -- counterpart of `cat-whisk` having no counterpart at the linear kit.
  vtx-wires : ∀ {Γ Δ} {m : Match Ob Γ Δ} {c d} → Vtxᶠ (wires m) c d → ⊥
  vtx-wires ()

  -- `⊤̂`. The corolla is the unique generator of its profile, so `Gen` is
  -- trivial and the unit code is the corolla family.
  unit : (A B : List Ob) → Shape Ob A B
  unit = corolla

  one : ∀ {A B} → Vtxᶠ (unit A B) A B
  one = hit

  -- `⟦⊤̂⟧`. The corolla has exactly one position, and the second clause is the
  -- emptiness above read at the corolla's own wiring.
  corolla-elim
    : ∀ {A B}
    → (P : ∀ {c d} → Vtxᶠ (unit A B) c d → Set ℓ)
    → P (one {A} {B})
    → ∀ {c d} (v : Vtxᶠ (unit A B) c d) → P v
  corolla-elim P x hit = x
  corolla-elim P x (miss ())

  -- ══════════════════════════════════════════════════════════════════════════
  -- WHAT IS OWED, AS TYPES RATHER THAN AS PROSE. Nothing here is postulated;
  -- these are the statements a circuit instance has to inhabit, written so the
  -- next pass starts from a signature instead of from a paragraph.
  -- ══════════════════════════════════════════════════════════════════════════

  -- THE CODE, per the ruling: a shape with its number of closed components.
  -- `Match` and `Shape` are untouched; the count rides here, where the only
  -- operations that can close a circle are.
  Cod : List Ob → List Ob → Set ℓ
  Cod Γ Δ = Shape Ob Γ Δ × ℕ

  -- and the interpretation reads through it, because a circle carries no
  -- vertex — which is why `Pos`, `lab`, `one` and `one-elim` are unchanged
  Vtxᶜ : ∀ {Γ Δ} → Cod Γ Δ → List Ob → List Ob → Set ℓ
  Vtxᶜ (X , _) = Vtxᶠ X

  -- The former. Graph substitution: replace every vertex of a shape by a code
  -- of the same profile. Its base case is the two-sided closure below, whose
  -- circles are added to the counts the operands carried.
  Subst : Set ℓ
  Subst =
      ∀ {Γ Δ}
    → (X : Cod Γ Δ)
    → (∀ {c d} → Vtxᶜ X c d → Cod c d)
    → Cod Γ Δ

  -- `⟦Σ̂⟧`, the half the laws consume. At the binary rung this is `verts-graft`
  -- and `verts-merge`, both proved. The count is invisible to it, for the same
  -- reason it is invisible to `Pos`.
  Pairing : Subst → Set ℓ
  Pairing sb =
      ∀ {Γ Δ} {X : Cod Γ Δ}
    → (Y : ∀ {c d} → Vtxᶜ X c d → Cod c d)
    → ∀ {c d e f}
    → (v : Vtxᶜ X c d)
    → Vtxᶜ (Y v) e f
    → Vtxᶜ (sb X Y) e f

  -- ══════════════════════════════════════════════════════════════════════════
  -- AND WHAT THAT FORMER'S BASE CASE DECOMPOSES INTO. `Subst` is the statement
  -- the interface asks for; these are the statements it is assembled from, and
  -- below them is the one term the carrier does not have.
  --
  -- Substituting at the head vertex is MERGING the replacement beside the rest
  -- — `merge` places `Shape A B` beside `Shape (B ++ Γ) (A ++ Δ)` disconnected,
  -- which `merge-apart` proves — and then closing the two blocks ONE WIRE AT A
  -- TIME. So the residual is a single-wire operation and its block form is that
  -- operation iterated, which is `lwhisk`'s and `wires-in`'s technique and the
  -- reason no permutation enters.
  -- ══════════════════════════════════════════════════════════════════════════

  -- What a closure returns. The wiring is the point; the count is the residual
  -- the carrier cannot absorb. Carried HERE and not inside `Shape`, which is
  -- what leaves the incidence, the predicates and their refuters untouched.
  record Closed (Γ Δ : List Ob) : Set ℓ where
    constructor closed
    field
      wiring : Match Ob Γ Δ
      loops : ℕ

  -- THE RESIDUAL PROPER: delete the source at `i` and the sink at `j`, and join
  -- whatever each was attached to. `match-insert`'s inverse.
  --
  -- IT DOES NOT RECURSE, which is the finding that bears on the ladder.
  -- `match-remove i` reads the source's partner and `match-unhit j` reads the
  -- sink's hitter — both landed, both structural — and the rebuild is one
  -- existing operation: `match-insert` when the source ran through to some
  -- OTHER sink, `match-cap` when the source was already cut, and NEITHER when
  -- the source ran to that very sink, which is the case that closes a circle.
  -- The cut case cannot close one, because a source has exactly one end: a
  -- source cut to `w` leaves `w` spent, so `w` does not hit `j`.
  --
  -- So composing two wirings is merging them and closing the shared interface,
  -- and this SUBSUMES `match-comp` — whose fused clause recurses on a matching
  -- `match-unhit` produced rather than on a subterm, and whose accessibility
  -- bookkeeping is what makes its associativity hard. On the nose, since
  -- `Match` has one term per wiring; the induction saying so is not written.
  MatchClose : Set ℓ
  MatchClose =
      ∀ {x Γ Γˣ Δ Δˣ}
    → Insert Ob x Γ Γˣ
    → Insert Ob x Δ Δˣ
    → Match Ob Γˣ Δˣ
    → Closed Γ Δ

  -- and its shape-level lift, pushing past each published port block exactly as
  -- `wire-in` does. It lands in `Cod` rather than in `Shape`, which is where
  -- the count enters the interface: `Subst` adds what this returns to the
  -- counts its operands carried.
  CloseIn : Set ℓ
  CloseIn =
      ∀ {x Γ Γˣ Δ Δˣ}
    → Insert Ob x Γ Γˣ
    → Insert Ob x Δ Δˣ
    → Shape Ob Γˣ Δˣ
    → Cod Γ Δ

  -- ══════════════════════════════════════════════════════════════════════════
  -- WHY THE COUNT IS IN THOSE TYPES, CHECKED RATHER THAN ARGUED. The published
  -- `Subst` above is TOTAL, so this is not a corner a hypothesis can exclude.
  -- ══════════════════════════════════════════════════════════════════════════

  -- A vertex whose output is wired straight back into its own input. A legal
  -- shape, and closed: no legs, one vertex, one edge.
  selfloop : (x : Ob) → Shape Ob [] []
  selfloop x =
    node (x ∷ []) (x ∷ []) (cons nil) (cons nil) (wires (head wired∷ no-wire))

  -- A bare wire of that vertex's profile. A legal CODE, so `Subst` has to take
  -- it, and it has no vertex.
  bare-wire : (x : Ob) → Shape Ob (x ∷ []) (x ∷ [])
  bare-wire x = wires (head wired∷ no-wire)

  -- Substituting the second at the first closes a circle: no vertex, no leg,
  -- one edge. THE CARRIER HAS NO TERM FOR IT, for the reason
  -- `Gandr.Shape.Graph` records at the constructor — closing a circle with no
  -- vertex needs a pairing of two SINKS, and there is no such constructor.
  -- Said where a proof can use it: over closed interfaces, a shape with no
  -- vertex has no edge either.
  closed-vertexless-edgeless
    : (S : Shape Ob [] [])
    → verts S ≡ []
    → edges S ≡ []
  closed-vertexless-edgeless (wires no-wire) refl = refl

  no-circle
    : (x : Ob)
    → (S : Shape Ob [] [])
    → verts S ≡ []
    → edges S ≡ (flow x ∷ [])
    → ⊥
  no-circle x (wires no-wire) refl ()

  -- ══════════════════════════════════════════════════════════════════════════
  -- THE CONSTRUCTION. Each step inhabits one of the types declared above, so
  -- the contract can be read without the construction and the construction
  -- checked against the contract rather than against a paragraph.
  --
  -- The order is forced by what each step consumes: the wiring-level closure
  -- has no shape in it; the shape-level lift is that closure pushed past each
  -- published port block; the block iteration is the lift applied one wire at
  -- a time; and substitution is the merger followed by the block iteration at
  -- both of a vertex's port blocks.
  -- ══════════════════════════════════════════════════════════════════════════

  -- ── STEP ONE: THE SINGLE-WIRE CLOSURE AT THE WIRING ─────────────────────
  --
  -- `match-remove i` reads the closed source's partner and `match-unhit j` the
  -- closed sink's hitter; the rebuild is one existing operation. THERE IS NO
  -- RECURSION, which is the finding that retires `match-comp`.

  -- The rebuild when the closed source ran THROUGH to a sink other than the
  -- closed one: the source that hit the closed sink inherits that sink, so one
  -- wire replaces two and nothing closes.
  close-through
    : ∀ {x Γ Δ Δ˘}
    → Insert Ob x Δ˘ Δ
    → Unhit x Γ Δ˘
    → Closed Γ Δ
  close-through s (unhit p t) = closed (match-insert p s t) 0

  -- The rebuild when the closed source was already CUT to another source: the
  -- source that hit the closed sink is cut to that one instead.
  --
  -- THIS CASE CANNOT CLOSE A CIRCLE, and the reason is that a source has
  -- exactly one end: a source cut to `w` leaves `w` spent, so `w` does not hit
  -- the closed sink and the two ends being joined are distinct.
  close-cut
    : ∀ {x y Γ Γ˘ Δ}
    → Insert Ob y Γ˘ Γ
    → Unhit x Γ˘ Δ
    → Closed Γ Δ
  close-cut c (unhit p t) = closed (match-cap c p t) 0

  -- Which of the two the through case is, decided by comparing the sink the
  -- closed source ran to against the sink being closed. The verdict is taken
  -- as an ARGUMENT rather than as a `with`, on the module-wide rule: a `with`
  -- here would put both right-hand sides in an auxiliary that no lemma about
  -- the closure could name.
  --
  -- THE TWO COLOURS ARE GENERALIZED AND THEIR EQUATION IS CARRIED, and that is
  -- forced by `--without-K` rather than chosen. Every comparison the listing
  -- algebra already makes is between insertions of DIFFERENT colours — a
  -- removed source against a cut's inner port, a traced sink against a
  -- wiring's — so `same` unifies two colour variables and nothing is deleted.
  -- This one compares two positions that a caller always supplies at ONE
  -- colour, and matching `same` there would have to eliminate the reflexive
  -- equation `x = x`. Generalizing the colours and taking `x ≡ y` as a datum
  -- removes the deletion instead of paying an h-level condition for it: the
  -- `same` branch never looks at the equation, and the `apart` branch consumes
  -- it exactly where the two ends being joined have to agree.
  close-hit
    : ∀ {x y Γ Δ Δˣ Δ˘}
    → {spot : Insert Ob x Δ˘ Δˣ}
    → {j : Insert Ob y Δ Δˣ}
    → x ≡ y
    → InsertView spot j
    → Match Ob Γ Δ˘
    → Closed Γ Δ
  -- the source ran to that very sink, so the strand closes on itself. THIS IS
  -- THE CIRCLE, and what remains is the wiring underneath it untouched
  close-hit e same body = closed body 1
  close-hit refl (apart s j′) body = close-through s (match-unhit j′ body)

  -- and the outer split, on what the closed source's own lookup returned
  close-removal
    : ∀ {x Γ Δ Δˣ}
    → Insert Ob x Δ Δˣ
    → Removal x Γ Δˣ
    → Closed Γ Δ
  close-removal j (through spot body) = close-hit refl (insert-view spot j) body
  close-removal j (capped c body) = close-cut c (match-unhit j body)

  match-close : MatchClose
  match-close i j m = close-removal j (match-remove i m)

  -- ── STEP TWO: THE SHAPE-LEVEL LIFT ──────────────────────────────────────
  --
  -- Pushing past each published port block exactly as `wire-in` does — one
  -- `tail` per element via `insert-shift`, and no permutation. It lands in
  -- `Cod` rather than in `Shape`, which is where the count enters.

  close-in : CloseIn
  close-in i j (wires m) = wires (Closed.wiring c) , Closed.loops c
    where
      -- the wiring-level closure, at the shape's own matching
      c : Closed _ _
      c = match-close i j m
  close-in {Γ} {Δ} i j (node A B p q S) =
      node A B (append-graph B Γ) (append-graph A Δ) (proj₁ r)
    , proj₂ r
    where
      -- the same closure one vertex in, with both positions walked past the
      -- ports that vertex publishes
      r : Cod (B ++ Γ) (A ++ Δ)
      r =
        close-in
          (insert-shift (append-graph B Γ) p i)
          (insert-shift (append-graph A Δ) q j)
          S

  -- ── STEP THREE: THE BLOCK ITERATION, AND `plug` ─────────────────────────
  --
  -- One wire at a time, which is `lwhisk`'s and `wires-in`'s technique and the
  -- reason no permutation enters here either.

  -- Closing a whole block when both of its ends lead their interface.
  close-block
    : ∀ {Ξ Γ Γˣ Δ Δˣ}
    → Append Ob Ξ Γ Γˣ
    → Append Ob Ξ Δ Δˣ
    → Shape Ob Γˣ Δˣ
    → Cod Γ Δ
  close-block nil nil S = S , 0
  close-block (cons p) (cons q) S =
    proj₁ rest , proj₂ step + proj₂ rest
    where
      -- the block's leading wire, closed at both interfaces' heads
      step : Cod _ _
      step = close-in head head S
      -- and the rest of the block
      rest : Cod _ _
      rest = close-block p q (proj₁ step)

  -- What a CROSSED block closure leaves. The sink block being closed sits
  -- BEHIND another published block, so the interface the closure leaves is not
  -- determined by its inputs; it is carried rather than computed, which is the
  -- device `Widened` and `Regroup` already use for exactly this, and it is
  -- what keeps a computed list out of every downstream index.
  record Crossed (Γ B Δ : List Ob) : Set ℓ where
    constructor crossed
    field
      -- the sinks the closure leaves
      sinks : List Ob
      -- with the crossing block still in front of them
      over : Append Ob B Δ sinks
      -- the shape over that interface
      body : Shape Ob Γ sinks
      -- and the circles the closure shut
      loops : ℕ

  -- Closing a block whose sources lead their interface against sinks that sit
  -- behind one further published block. This is the vertex's INPUT block: the
  -- replacement's input legs are at the front of the merged sources, while the
  -- ports the vertex published to the sinks sit behind the replacement's own
  -- output legs.
  close-cross
    : ∀ {Ξ B Γ Γˣ Δ Δᵃ Δˣ}
    → Append Ob Ξ Γ Γˣ
    → Append Ob Ξ Δ Δᵃ
    → Append Ob B Δᵃ Δˣ
    → Shape Ob Γˣ Δˣ
    → Crossed Γ B Δ
  close-cross nil nil r S = crossed _ r S 0
  close-cross {B} (cons p) (cons q) r S =
    crossed
      (Crossed.sinks rest)
      (Crossed.over rest)
      (Crossed.body rest)
      (proj₂ step + Crossed.loops rest)
    where
      -- the block's leading wire: its source leads the interface and its sink
      -- is that same position walked past the crossing block
      step : Cod _ _
      step = close-in head (insert-shift (append-graph B _) r head) S
      -- and the rest of the block, over the interface that leaves
      rest : Crossed _ B _
      rest = close-cross p q (append-graph B _) (proj₁ step)

  -- SUBSTITUTING AT ONE VERTEX. Merge the replacement beside the rest —
  -- `merge` places them disconnected, which `merge-apart` proves — then close
  -- the vertex's two published port blocks against the replacement's legs.
  -- `|A| + |B|` closures, and every circle any of them shuts is returned.
  plug
    : ∀ {A B Γ Γ′ Δ Δ′}
    → Append Ob B Γ Γ′
    → Append Ob A Δ Δ′
    → Shape Ob A B
    → Shape Ob Γ′ Δ′
    → Cod Γ Δ
  plug {A} {B} {Γ} {Γ′} {Δ} {Δ′} p q V S =
    proj₁ outer , Crossed.loops inner + proj₂ outer
    where
      -- the vertex's IN-profile, closed against the replacement's input legs
      inner : Crossed Γ′ B Δ
      inner =
        close-cross
          (append-graph A Γ′)
          q
          (append-graph B Δ′)
          (merge (append-graph A Γ′) (append-graph B Δ′) V S)
      -- and its OUT-profile, which by then leads both interfaces
      outer : Cod Γ Δ
      outer = close-block p (Crossed.over inner) (Crossed.body inner)

  -- ── STEP FOUR: `Subst` AND `Pairing` ────────────────────────────────────
  --
  -- The outer recursion is trivial at a wiring — `vtx-wires` above is that
  -- fact — so substitution descends the node chain and plugs at each vertex.

  -- Substitution at the shape level, where the outer code's own count is not
  -- yet in play: the circles this returns are exactly the ones the plugging
  -- shut, plus the ones the substituted codes carried.
  subst-shape
    : ∀ {Γ Δ}
    → (X : Shape Ob Γ Δ)
    → (∀ {c d} → Vtxᶠ X c d → Cod c d)
    → Cod Γ Δ
  subst-shape (wires m) Y = wires m , 0
  subst-shape (node A B p q S) Y =
    proj₁ plugged , proj₂ (Y hit) + proj₂ below + proj₂ plugged
    where
      -- what substitution does to the rest of the node chain
      below : Cod _ _
      below = subst-shape S (λ v → Y (miss v))
      -- and the head vertex's replacement, plugged into it
      plugged : Cod _ _
      plugged = plug p q (proj₁ (Y hit)) (proj₁ below)

  sub : Subst
  sub (X , k) Y = proj₁ r , k + proj₂ r
    where
      -- the code's own circles ride through untouched: nothing the
      -- substitution does can open one
      r : Cod _ _
      r = subst-shape X Y

  -- ── WHAT THE CONSTRUCTION DOES TO THE VERTEX LISTING ────────────────────
  --
  -- `⟦Σ̂⟧`'s forward half is read off these: closing adds no vertex, so the
  -- substituted shape's listing is the merger's, which `verts-merge` already
  -- says is the concatenation.

  verts-close-in
    : ∀ {x Γ Γˣ Δ Δˣ}
    → (i : Insert Ob x Γ Γˣ)
    → (j : Insert Ob x Δ Δˣ)
    → (S : Shape Ob Γˣ Δˣ)
    → verts (proj₁ (close-in i j S)) ≡ verts S
  verts-close-in i j (wires m) = refl
  verts-close-in {Γ} {Δ} i j (node A B p q S) =
    cong
      (prof A B ∷_)
      (verts-close-in
        (insert-shift (append-graph B Γ) p i)
        (insert-shift (append-graph A Δ) q j)
        S)

  verts-close-block
    : ∀ {Ξ Γ Γˣ Δ Δˣ}
    → (p : Append Ob Ξ Γ Γˣ)
    → (q : Append Ob Ξ Δ Δˣ)
    → (S : Shape Ob Γˣ Δˣ)
    → verts (proj₁ (close-block p q S)) ≡ verts S
  verts-close-block nil nil S = refl
  verts-close-block (cons p) (cons q) S =
    trans
      (verts-close-block p q (proj₁ (close-in head head S)))
      (verts-close-in head head S)

  verts-close-cross
    : ∀ {Ξ B Γ Γˣ Δ Δᵃ Δˣ}
    → (p : Append Ob Ξ Γ Γˣ)
    → (q : Append Ob Ξ Δ Δᵃ)
    → (r : Append Ob B Δᵃ Δˣ)
    → (S : Shape Ob Γˣ Δˣ)
    → verts (Crossed.body (close-cross p q r S)) ≡ verts S
  verts-close-cross nil nil r S = refl
  verts-close-cross {B} (cons p) (cons q) r S =
    trans
      (verts-close-cross
        p
        q
        (append-graph B _)
        (proj₁ (close-in head (insert-shift (append-graph B _) r head) S)))
      (verts-close-in head (insert-shift (append-graph B _) r head) S)

  -- THE STATEMENT: plugging concatenates the listings, replacement first —
  -- `verts-merge` with the closures shown to contribute nothing.
  verts-plug
    : ∀ {A B Γ Γ′ Δ Δ′}
    → (p : Append Ob B Γ Γ′)
    → (q : Append Ob A Δ Δ′)
    → (V : Shape Ob A B)
    → (S : Shape Ob Γ′ Δ′)
    → verts (proj₁ (plug p q V S)) ≡ verts V ++ verts S
  verts-plug {A} {B} {Γ} {Γ′} {Δ} {Δ′} p q V S =
    begin⟨ bundle (≡ˢ _) ⟩
      verts (proj₁ (plug p q V S))
    ≈⟨ verts-close-block p (Crossed.over inner) (Crossed.body inner) ⟩
      verts (Crossed.body inner)
    ≈⟨ verts-close-cross
         (append-graph A Γ′)
         q
         (append-graph B Δ′)
         (merge (append-graph A Γ′) (append-graph B Δ′) V S)
     ⟩
      verts (merge (append-graph A Γ′) (append-graph B Δ′) V S)
    ≈⟨ verts-merge (append-graph A Γ′) (append-graph B Δ′) V S ⟩
      verts V ++ verts S
    ∎
    where
      inner : Crossed Γ′ B Δ
      inner =
        close-cross
          (append-graph A Γ′)
          q
          (append-graph B Δ′)
          (merge (append-graph A Γ′) (append-graph B Δ′) V S)

  -- `⟦Σ̂⟧`, at the shape level. A position of a substitution is an outer
  -- position paired with an inner one, and the two clauses ARE the two
  -- injections into the concatenation the listing became.
  pair-shape
    : ∀ {Γ Δ} {X : Shape Ob Γ Δ}
    → (Y : ∀ {c d} → Vtxᶠ X c d → Cod c d)
    → ∀ {c d e f}
    → (v : Vtxᶠ X c d)
    → Vtxᶠ (proj₁ (Y v)) e f
    → Vtxᶠ (proj₁ (subst-shape X Y)) e f
  pair-shape {X = node A B p q S} Y hit w =
    subst
      (λ l → Occ l (prof _ _))
      (sym
        (verts-plug
          p
          q
          (proj₁ (Y hit))
          (proj₁ (subst-shape S (λ v → Y (miss v))))))
      (occ-inl w)
  pair-shape {X = node A B p q S} Y (miss v) w =
    subst
      (λ l → Occ l (prof _ _))
      (sym
        (verts-plug
          p
          q
          (proj₁ (Y hit))
          (proj₁ (subst-shape S (λ u → Y (miss u))))))
      (occ-inr
        (verts (proj₁ (Y hit)))
        (pair-shape (λ u → Y (miss u)) v w))

  -- and at the code level, where the count is invisible to it for the same
  -- reason it is invisible to `Pos`
  pair : Pairing sub
  pair Y v w = pair-shape Y v w

  -- ── WHAT THE CONSTRUCTION COMPUTES, PINNED ──────────────────────────────
  --
  -- A green gate is not proof of meaning, and the count is exactly the kind of
  -- datum a construction can carry and never populate. These exercise it.

  -- the family substituted at the self-loop's one vertex: the bare wire of
  -- that vertex's profile, which is the code the count was introduced for
  loop-plug
    : (x : Ob)
    → ∀ {c d}
    → Vtxᶜ (selfloop x , 0) c d
    → Cod c d
  loop-plug x hit = bare-wire x , 0
  loop-plug x (miss ())

  -- THE COUNT IS NOT VACUOUS, and this is the whole of the reason `Cod` carries
  -- a number. Substituting the bare wire at the self-loop's vertex leaves no
  -- vertex, no leg and no edge — the configuration `no-circle` above refutes —
  -- together with the circle that closed, counted. Discarding it would return
  -- the empty wiring at zero, which is the same term the empty shape already
  -- has, and nothing downstream could tell the two apart.
  selfloop-counted
    : (x : Ob)
    → sub (selfloop x , 0) (loop-plug x) ≡ (wires no-wire , 1)
  selfloop-counted x = refl

  -- ── STEP FIVE: THE THREE LAWS, AND THE TWO OBLIGATIONS WITH THEM ────────
  --
  -- Stated as types, on the same terms as `Subst` and `Pairing` above: nothing
  -- here is postulated, and each statement is what the next pass starts from.
  -- NONE OF THE FIVE IS INHABITED IN THIS REVISION.
  --
  -- ONE THING ABOUT THEIR COST IS MEASURED RATHER THAN GUESSED, and it is
  -- worth having before the next pass starts. `close-in` rebuilds each node it
  -- pushes past over `append-graph`, so a unit law has to say that the witness
  -- it rebuilt with is the one the original node carried. Both natural
  -- formulations of that ingredient — `(p : Append Ob B Γ (B ++ Γ)) → p ≡
  -- append-graph B Γ`, and uniqueness of two witnesses at one triple — are
  -- STUCK under `--without-K`, on a reflexive `List Ob` equation the `nil`
  -- clause would have to delete. That is the forced-index deletion the design
  -- doctrine describes, and it is the same h-level condition `graft-idnˡ` and
  -- `graft-idnʳ` already take as `UIP (List Ob)` parameters.
  --
  -- What is NOT measured, and must not be read into the above: whether the
  -- unit laws themselves need that condition. A route that never compares two
  -- witnesses at one triple would not, and neither probe rules one out.

  -- the unit CODE, as the interface asks for it: the corolla with no circle
  -- beside it. `unit` above is the SHAPE, and predates the count
  unitᶜ : (A B : List Ob) → Cod A B
  unitᶜ A B = unit A B , 0

  oneᶜ : ∀ {A B} → Vtxᶜ (unitᶜ A B) A B
  oneᶜ = one

  -- LEFT UNIT. Substituting into the unit code is the substituted code.
  Idl : Set ℓ
  Idl =
      ∀ {A B}
    → (Y : ∀ {c d} → Vtxᶜ (unitᶜ A B) c d → Cod c d)
    → sub (unitᶜ A B) Y ≡ Y oneᶜ

  -- RIGHT UNIT. Substituting the unit at every position changes nothing.
  Idr : Set ℓ
  Idr =
      ∀ {Γ Δ}
    → (X : Cod Γ Δ)
    → sub X (λ {c} {d} _ → unitᶜ c d) ≡ X

  -- ASSOCIATIVITY, stated over `pair` exactly as the interface states it.
  --
  -- THE CODE IS NAMED AT THE USE SITE, and that is the count being invisible
  -- to `Pairing` showing up as an elaboration fact rather than as a remark.
  -- `Pairing`'s code argument is implicit and is meant to be recovered from
  -- the substituted family's type — but that type mentions the code only
  -- through `Vtxᶜ`, which reads the shape and drops the number, so the number
  -- is a metavariable nothing constrains. Naming it is the whole of the cost.
  Assoc : Set ℓ
  Assoc =
      ∀ {Γ Δ}
    → (X : Cod Γ Δ)
    → (Y : ∀ {c d} → Vtxᶜ X c d → Cod c d)
    → (Z : ∀ {c d} → Vtxᶜ (sub X Y) c d → Cod c d)
    → sub (sub X Y) Z
      ≡ sub X (λ v → sub (Y v) (λ w → Z (pair {X = X} Y v w)))

  -- AND THE COUNT LAW, which is associativity's SECOND projection read on its
  -- own: both bracketings shut the same number of circles.
  --
  -- It is stated separately because it is the half that is not about shapes at
  -- all, and because it is the half the source states directly: a Brauer
  -- diagram is a pairing together with a count, and composition adds the two
  -- counts plus the components the composition closes
  -- [@raynor-2025-functorial, Def 3.4]. Here the third summand is what `plug`
  -- returns, and the law says the two ways of bracketing a double substitution
  -- agree on the sum.
  CountLaw : Set ℓ
  CountLaw =
      ∀ {Γ Δ}
    → (X : Cod Γ Δ)
    → (Y : ∀ {c d} → Vtxᶜ X c d → Cod c d)
    → (Z : ∀ {c d} → Vtxᶜ (sub X Y) c d → Cod c d)
    → proj₂ (sub (sub X Y) Z)
      ≡ proj₂ (sub X (λ v → sub (Y v) (λ w → Z (pair {X = X} Y v w))))

  -- THE TWO-COROLLA SERIES SHAPE. Two vertices, the first's out-ports wired to
  -- the second's in-ports, the input legs to the first and the second's
  -- out-ports to the output legs. An ORDINARY term: two `node`s over one block
  -- swap, needing no grafting to build, which is what makes the agreement
  -- lemma below a statement about substitution rather than a restatement of
  -- grafting.
  --
  -- The re-association is `append-regroup`'s, for the reason the merger needs
  -- it: the second vertex publishes to the front of the FIRST vertex's
  -- extended interface, and the wiring is stated over the other bracketing.
  series : (Γ Δ Θ : List Ob) → Shape Ob Γ Θ
  series Γ Δ Θ =
    node Γ Δ (append-graph Δ Γ) (append-graph Γ Θ)
      (node Δ Θ
        (append-graph Θ (Δ ++ Γ))
        (Regroup.front regrouped)
        (wires
          (swap-match
            (append-graph Θ (Δ ++ Γ))
            (Regroup.back regrouped))))
    where
      -- the second vertex's ports, published to the front of the whole sink
      -- interface rather than of the first vertex's
      regrouped : Regroup Δ (Δ ++ Γ) Θ (Γ ++ Θ)
      regrouped = append-regroup (append-graph Δ Γ) (append-graph Γ Θ)

  -- the family the series shape's two positions ask for
  series-at
    : ∀ {Γ Δ Θ}
    → Shape Ob Γ Δ
    → Shape Ob Δ Θ
    → ∀ {c d}
    → Vtxᶜ (series Γ Δ Θ , 0) c d
    → Cod c d
  series-at S T hit = S , 0
  series-at S T (miss hit) = T , 0
  series-at S T (miss (miss ()))

  -- THE AGREEMENT LEMMA. Grafting IS substitution into the series shape.
  --
  -- It is what carries `verts-graft`, `graft-idnˡ`/`graft-idnʳ` and the
  -- merger's incidence theorems across to the primitive former; without it the
  -- tree carries two compositions and proves each fact twice. Expected on the
  -- nose, because `Match` has one term per wiring and the closure therefore
  -- subsumes `match-comp` exactly rather than up to anything — but that is an
  -- induction, and it is not written.
  Agreement : Set ℓ
  Agreement =
      ∀ {Γ Δ Θ}
    → (S : Shape Ob Γ Δ)
    → (T : Shape Ob Δ Θ)
    → sub (series Γ Δ Θ , 0) (series-at S T) ≡ (graft S T , 0)
