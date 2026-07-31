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
--     interpretation to the ORDERED labelled positions, and then the
--     interpretation is the code and `Inj` is free.
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
--     under substitution or reappear as a partial trace is spike S11 and is NOT
--     decided here; what is decided is that the linear kit's whiskering does
--     dissolve, which is the control the prediction needed.
--
-- The residual cost is `sub-cat` — substitution distributes over concatenation
-- — which is new, and is the one place the presentation pays rather than
-- reuses. It is the price of stating associativity over positions instead of
-- over witnesses.
--
-- ── WHAT IS NOT HERE ────────────────────────────────────────────────────────
-- The splitting half of `⟦Σ̂⟧` — that `pair` is a bijection and not merely a
-- map — is not proved. The three laws consume only the forward half, so its
-- absence does not weaken them; it does mean this module inhabits the FORMER
-- half of Definition 9 and not the representation half. `Inj`, `InjComp` and
-- the circuit-kit instance are the rest, and the first two are stated as a
-- refuted naive form rather than left unmentioned.
------------------------------------------------------------------------------

module Gandr.Arity.Universe where

open import Agda.Primitive
  using (lsuc)
open import Gandr.Arity.Path
  using (Path)
  using (here)
  using (then)
  using (_++_)
  using (cat-graph)
  using (cat-fun)
  using (cat-assoc)
  using (cat-idnˡ)
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
open import Data.Unit
  using (⊤)
  using (tt)
open import Relation.Binary.PropositionalEquality
  using (_≡_)
  using (refl)
  using (cong)

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
-- is the vertex family, and it needs one new definition — the FAMILIAL form of
-- `Vtx`, indexed by the profile it spans, which is what typing the substituted
-- family requires and what `docs/workflow/agda.md` §*Representation: familial
-- first* would have wanted anyway. `Vtxᶠ` is a view of `Vtx` and not a rival:
-- `toVtx` and the two profile lemmas below say so.
--
--   field       status at the circuit kit
--   ─────────   ───────────────────────────────────────────────────────────────
--   Ifc         `List Ob`
--   Code        `Shape`
--   Pos         `Vtxᶠ` — new, and the only new definition here
--   Gen         `⊤`, one generator per profile: the corolla
--   lab         trivial, for the same reason
--   unit        `corolla`
--   one         `top`
--   one-elim    `corolla-elim` — proved
--   ─────────   ───────────────────────────────────────────────────────────────
--   sub         OWED — `Subst` below
--   pair        owed, downstream of `sub`
--   idl idr     owed, downstream of `sub`
--   assoc       owed, downstream of `sub`
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
--     one. That construction is the residual, it is named here so it is not
--     rediscovered, and it is NOT built.
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
  open import Gandr.Shape.Graph
    using (Shape)
    using (wires)
    using (node)
    using (Append)
    using (Match)
    using (Prof)
    using (prof)
    using (corolla)
    using (Vtx)
    using (verts)
    using (ins)
    using (outs)
    using (lookup)
    renaming (here to ixᶻ)
    renaming (there to ixˢ)

  -- THE INTERPRETATION, familially. `Gandr.Shape.Graph.Vtx S` is `Ix (verts S)`
  -- — a position in a listing, with the profile read back by `lookup`. The
  -- universe presentation needs the profile in the INDEX, because it is what
  -- types the family being substituted, so the same information is presented as
  -- an inductive family here. This is `Gandr.Arity.Universe.Linear.Place` one
  -- rung up, and it is the only new definition the circuit half needs.
  data Vtxᶠ : ∀ {Γ Δ} → Shape Ob Γ Δ → List Ob → List Ob → Set ℓ where
    -- the outermost vertex
    top
      : ∀ {Γ Δ Γ′ Δ′ A B}
      → {p : Append Ob B Γ Γ′} {q : Append Ob A Δ Δ′}
      → {S : Shape Ob Γ′ Δ′}
      → Vtxᶠ (node A B p q S) A B
    -- one further in
    down
      : ∀ {Γ Δ Γ′ Δ′ A B C D}
      → {p : Append Ob B Γ Γ′} {q : Append Ob A Δ Δ′}
      → {S : Shape Ob Γ′ Δ′}
      → Vtxᶠ S C D
      → Vtxᶠ (node A B p q S) C D

  -- It is a VIEW of the listing position, not a rival to it.
  toVtx : ∀ {Γ Δ} {S : Shape Ob Γ Δ} {c d} → Vtxᶠ S c d → Vtx S
  toVtx top = ixᶻ
  toVtx (down v) = ixˢ (toVtx v)

  -- and the index is the profile the listing records, on both sides
  ins-toVtx
    : ∀ {Γ Δ} {S : Shape Ob Γ Δ} {c d}
    → (v : Vtxᶠ S c d)
    → ins (toVtx v) ≡ c
  ins-toVtx top = refl
  ins-toVtx (down v) = ins-toVtx v

  outs-toVtx
    : ∀ {Γ Δ} {S : Shape Ob Γ Δ} {c d}
    → (v : Vtxᶠ S c d)
    → outs (toVtx v) ≡ d
  outs-toVtx top = refl
  outs-toVtx (down v) = outs-toVtx v

  -- A WIRING HAS NO POSITION. This is what makes substitution's outer recursion
  -- trivial where grafting's is a well-founded composition of matchings, and it
  -- is the circuit-rung counterpart of `cat-whisk` having no counterpart at the
  -- linear kit.
  vtx-wires : ∀ {Γ Δ} {m : Match Ob Γ Δ} {c d} → Vtxᶠ (wires m) c d → ⊥
  vtx-wires ()

  -- `⊤̂`. The corolla is the unique generator of its profile, so `Gen` is
  -- trivial and the unit code is the corolla family.
  unit : (A B : List Ob) → Shape Ob A B
  unit = corolla

  one : ∀ {A B} → Vtxᶠ (unit A B) A B
  one = top

  -- `⟦⊤̂⟧`. The corolla has exactly one position, and the second clause is the
  -- emptiness above read at the corolla's own wiring.
  corolla-elim
    : ∀ {A B}
    → (P : ∀ {c d} → Vtxᶠ (unit A B) c d → Set ℓ)
    → P (one {A} {B})
    → ∀ {c d} (v : Vtxᶠ (unit A B) c d) → P v
  corolla-elim P x top = x
  corolla-elim P x (down ())

  -- ══════════════════════════════════════════════════════════════════════════
  -- WHAT IS OWED, AS TYPES RATHER THAN AS PROSE. Nothing here is postulated;
  -- these are the statements a circuit instance has to inhabit, written so the
  -- next pass starts from a signature instead of from a paragraph.
  -- ══════════════════════════════════════════════════════════════════════════

  -- The former. Graph substitution: replace every vertex of a shape by a shape
  -- of the same profile. Its base case is the two-sided closure named in the
  -- header, and that is the residual this spike locates rather than discharges.
  Subst : Set ℓ
  Subst =
      ∀ {Γ Δ}
    → (X : Shape Ob Γ Δ)
    → (∀ {c d} → Vtxᶠ X c d → Shape Ob c d)
    → Shape Ob Γ Δ

  -- `⟦Σ̂⟧`, the half the laws consume. At the binary rung this is `verts-graft`
  -- and `verts-merge`, both proved.
  Pairing : Subst → Set ℓ
  Pairing sb =
      ∀ {Γ Δ} {X : Shape Ob Γ Δ}
    → (Y : ∀ {c d} → Vtxᶠ X c d → Shape Ob c d)
    → ∀ {c d e f}
    → (v : Vtxᶠ X c d)
    → Vtxᶠ (Y v) e f
    → Vtxᶠ (sb X Y) e f
