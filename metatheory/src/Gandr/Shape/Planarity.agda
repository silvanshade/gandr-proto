{-# OPTIONS --safe --without-K --hidden-argument-puns #-}

------------------------------------------------------------------------------
-- Gandr.Shape.Planarity — the carrier planarity test: does `Gandr.Shape.Graph`'s
-- constructor admit a diagram whose underlying graph is non-planar?
--
-- This module is a TEST and nothing else. It adds no carrier machinery, no
-- predicate and no reusable certificate; every definition below exists to state
-- or to check one witness. It is separate from `Gandr.Shape.Graph` for that
-- reason — the worked examples there populate the carrier's predicates, and this
-- one answers a question asked from outside about what the carrier admits.
--
-- ── WHAT THE QUESTION IS, AND WHY IT HAS A CONSEQUENCE ──────────────────────
-- The corpus consumes a theorem stated for CONNECTED PLANAR diagrams, and the
-- word "planar" carries two different conditions across the sites that consume
-- it: the corpus's own sense — the monoidal BASE is the free monoid on the
-- colours, so interface objects are lists rather than multisets and the merger
-- is a planar tensor — and the cited theorem's sense, a CROSSING-FREE EMBEDDING
-- of the diagram in the plane. Neither implies the other.
--
-- The bridge that would let the second be consumed at the first is a
-- factorization of a wiring as an interface permutation followed by a
-- crossing-free core: push the permutation outward, and what remains embeds.
-- Pushing permutations outward is a device the corpus already owns, and it
-- absorbs exactly the BOUNDARY. It does nothing to the INTERIOR, and no
-- reordering of the boundary repairs a non-planar interior. So the factorization
-- exists in general exactly if the carrier's interiors are all planar, and that
-- is the question this module settles.
--
-- ── THE WITNESS, AND WHY IT IS K3,3 RATHER THAN K5 ──────────────────────────
-- Kuratowski's theorem is the classical fact that a finite graph is planar
-- exactly when it contains no subdivision of `K₅` or of `K₃,₃`. That theorem is
-- NOT mechanized here and is not the mechanized content: what is mechanized is
-- that the carrier admits a shape whose underlying graph CONTAINS `K₃,₃`, and
-- non-planarity of `K₃,₃` is quoted from the classical result. The statement
-- separation is deliberate — the carrier question is a question about terms of
-- `Shape`, and graph planarity theory is not a development this tree wants.
--
-- `K₃,₃` is the cheaper of the two witnesses in THIS vocabulary, for three
-- reasons, none of them about the graphs and all of them about the carrier:
--
--   * `node A B` publishes a whole port block at once, so a bipartition whose
--     two classes have uniform profiles — `prof [] 𝟛` three times, `prof 𝟛 []`
--     three times — makes every `Append` witness the same term. `K₅` needs five
--     different profiles, one per out-degree.
--   * nine edges rather than ten, so nine `Attach` witnesses rather than ten.
--   * the bipartition is a DEPTH-ONE orientation: every vertex of one class is
--     a pure source and every vertex of the other a pure sink, so the wiring
--     has no `cap`, no palette is needed to orient anything, and the rank
--     certificate is `0`/`1`.
--
-- ── THE RESULT, AND THE ONE DISTINCTION IT TURNS ON ─────────────────────────
-- `k33` below type-checks, so the answer is YES: the carrier admits a non-planar
-- diagram, and the boundary factorization therefore does not exist in general.
--
-- The witness is not a degenerate one. It is CONNECTED, which is the hypothesis
-- class the spider theorem quantifies over, and it is WHEEL-FREE at the
-- monochrome polarity by a rank certificate, so it sits inside the directed
-- fragment the carrier's directed layer admits rather than outside it.
--
-- What it is NOT is a `Cell`: it carries a reduced undirected closed walk, so
-- `Acyclic` fails and `SimplyConn` with it. That is not a weakening of the
-- result, it is the distinction the result turns on, and it is worth stating in
-- both directions:
--
--   * A `Cell` is connected and acyclic, which is to say a TREE, and trees are
--     planar. So the question asked of ONE CELL has the opposite answer. (Trees
--     are planar is classical and is not mechanized here either; what is
--     mechanized is that this witness is not a cell.)
--   * Diagrams are not cells. `Gandr.Shape.Graft`'s header says so at the
--     operation — "nothing has to be said about whether the composite is a cell
--     — and it usually is not" — and `Gandr.Shape.Graph`'s `diamond` is already
--     a wheel-free, connected, non-simply-connected shape. The theorem the
--     corpus consumes is stated about a DIAGRAM, so the carrier's diagrams are
--     what the question is about.
--
-- ── PROVENANCE ─────────────────────────────────────────────────────────────
-- Run as the proving spike `gandr-hpck-answer-17` pre-authorized on the
-- metatheory decision queue: option (e), the carrier planarity test, sequenced
-- ahead of the disposition its result selects. The expected-holds and
-- expected-fails were stated before the run, per the implementation-first
-- posture recorded at `docs/gandr/spec/metatheory/roadmap.md` §*meta-spike-05*.
------------------------------------------------------------------------------

module Gandr.Shape.Planarity where

open import Data.List.Base
  using (List)
  using ([])
  using (_∷_)
open import Data.Nat.Base
  using (ℕ)
open import Data.Nat.Properties
  using (n<1+n)
open import Data.Maybe.Base
  using (just)
open import Data.Unit.Base
  using (⊤)
  using (tt)
open import Relation.Binary.PropositionalEquality
  using (_≡_)
  using (refl)
open import Relation.Nullary.Negation
  using (¬_)

open import Gandr.Shape.Graph

-- ════════════════════════════════════════════════════════════════════════════
-- THE WITNESS. Six vertices in one node chain — three publishing three
-- out-ports each and no in-ports, three publishing three in-ports each and no
-- out-ports — over one wiring that sends every out-port block across every
-- in-port block exactly once.
--
-- Reading the innermost pools, exactly as `diamond` is read: the sources are
-- `out₂ ++ out₁ ++ out₀` and the sinks are `in₅ ++ in₄ ++ in₃`, because a node
-- publishes to the FRONT of the interface and the chain is descended outermost
-- first. So sources `0-2` belong to vertex `2`, `3-5` to vertex `1` and `6-8`
-- to vertex `0`; sinks `0-2` belong to vertex `5`, `3-5` to vertex `4` and
-- `6-8` to vertex `3`.
-- ════════════════════════════════════════════════════════════════════════════

-- the port block each vertex carries. `Gandr.Shape.Graph` names one wire and
-- two; this test needs three, and needs nothing else the carrier does not have.
𝟛 : List ⊤
𝟛 = tt ∷ tt ∷ tt ∷ []

k33 : Shape ⊤ [] []
k33 =
  node [] 𝟛 (cons (cons (cons nil))) nil
    (node [] 𝟛 (cons (cons (cons nil))) nil
      (node [] 𝟛 (cons (cons (cons nil))) nil
        (node 𝟛 [] nil (cons (cons (cons nil)))
          (node 𝟛 [] nil (cons (cons (cons nil)))
            (node 𝟛 [] nil (cons (cons (cons nil)))
              (wires
                (head
                  ∷ tail (tail head)
                  ∷ tail (tail (tail (tail head)))
                  ∷ head
                  ∷ tail head
                  ∷ tail (tail head)
                  ∷ head
                  ∷ head
                  ∷ head
                  ∷ [])))))))

-- Exactly six vertices, in two classes of three. Checked by computation, so the
-- bipartition is read off the term rather than asserted about it.
k33-verts
  : verts k33
    ≡ prof [] 𝟛
      ∷ prof [] 𝟛
      ∷ prof [] 𝟛
      ∷ prof 𝟛 []
      ∷ prof 𝟛 []
      ∷ prof 𝟛 []
      ∷ []
k33-verts = refl

-- and exactly nine edges, every one of them a flow-through wire: the wiring has
-- no `cap`, which is why nothing below needs a palette.
k33-edges
  : edges k33
    ≡ flow tt
      ∷ flow tt
      ∷ flow tt
      ∷ flow tt
      ∷ flow tt
      ∷ flow tt
      ∷ flow tt
      ∷ flow tt
      ∷ flow tt
      ∷ []
k33-edges = refl

-- The six vertices, named. They are the six distinct positions of a six-entry
-- listing, so their pairwise distinctness is a fact about the terms rather than
-- a lemma — the same reading `diamond`'s `d₀ … d₃` are named under.
v₀ v₁ v₂ v₃ v₄ v₅ : Vtx k33
v₀ = here
v₁ = there here
v₂ = there (there here)
v₃ = there (there (there here))
v₄ = there (there (there (there here)))
v₅ = there (there (there (there (there here))))

-- and the nine edges, likewise the nine distinct positions of a nine-entry
-- listing. Nine distinct edges at nine distinct vertex pairs is what makes the
-- biclique below a SUBGRAPH rather than a nine-fold reuse of fewer wires.
e₀ e₁ e₂ e₃ e₄ e₅ e₆ e₇ e₈ : Edg k33
e₀ = here
e₁ = there here
e₂ = there (there here)
e₃ = there (there (there here))
e₄ = there (there (there (there here)))
e₅ = there (there (there (there (there here))))
e₆ = there (there (there (there (there (there here)))))
e₇ = there (there (there (there (there (there (there here))))))
e₈ = there (there (there (there (there (there (there (there here)))))))

-- ════════════════════════════════════════════════════════════════════════════
-- THE INCIDENCE, DERIVED. Every one of the nine is `attach refl refl`: the two
-- ends are COMPUTED by `end₀`/`end₁` tracing outward through the node chain,
-- and the equation holds by reduction. Nothing here asserts an incidence.
-- ════════════════════════════════════════════════════════════════════════════

a₂₅ : Attach k33 e₀ v₂ v₅
a₂₅ = attach refl refl

a₂₄ : Attach k33 e₁ v₂ v₄
a₂₄ = attach refl refl

a₂₃ : Attach k33 e₂ v₂ v₃
a₂₃ = attach refl refl

a₁₅ : Attach k33 e₃ v₁ v₅
a₁₅ = attach refl refl

a₁₄ : Attach k33 e₄ v₁ v₄
a₁₄ = attach refl refl

a₁₃ : Attach k33 e₅ v₁ v₃
a₁₃ = attach refl refl

a₀₅ : Attach k33 e₆ v₀ v₅
a₀₅ = attach refl refl

a₀₄ : Attach k33 e₇ v₀ v₄
a₀₄ = attach refl refl

a₀₃ : Attach k33 e₈ v₀ v₃
a₀₃ = attach refl refl

-- ════════════════════════════════════════════════════════════════════════════
-- THE BIPARTITION, AND THE RESULT. `left-class` and `right-class` name the two
-- classes over the carrier's own three-element index, and the biclique says
-- every vertex of the first is adjacent to every vertex of the second — which
-- is `K₃,₃` on the nose.
-- ════════════════════════════════════════════════════════════════════════════

left-class : Ix 𝟛 → Vtx k33
left-class here = v₀
left-class (there here) = v₁
left-class (there (there here)) = v₂
left-class (there (there (there ())))

right-class : Ix 𝟛 → Vtx k33
right-class here = v₃
right-class (there here) = v₄
right-class (there (there here)) = v₅
right-class (there (there (there ())))

-- THE RESULT. The underlying graph of `k33` contains the complete bipartite
-- graph on three and three, so by Kuratowski's theorem — classical, quoted, not
-- mechanized — it is non-planar. The carrier admits a non-planar diagram.
k33-biclique
  : (i j : Ix 𝟛)
  → Adj k33 (left-class i) (right-class j)
k33-biclique here here = adj e₈ (along a₀₃)
k33-biclique here (there here) = adj e₇ (along a₀₄)
k33-biclique here (there (there here)) = adj e₆ (along a₀₅)
k33-biclique here (there (there (there ())))
k33-biclique (there here) here = adj e₅ (along a₁₃)
k33-biclique (there here) (there here) = adj e₄ (along a₁₄)
k33-biclique (there here) (there (there here)) = adj e₃ (along a₁₅)
k33-biclique (there here) (there (there (there ())))
k33-biclique (there (there here)) here = adj e₂ (along a₂₃)
k33-biclique (there (there here)) (there here) = adj e₁ (along a₂₄)
k33-biclique (there (there here)) (there (there here)) = adj e₀ (along a₂₅)
k33-biclique (there (there here)) (there (there (there ())))
k33-biclique (there (there (there ()))) j

-- ════════════════════════════════════════════════════════════════════════════
-- THE WITNESS IS NOT DEGENERATE. Two facts, and they are what stop the result
-- from being about a shape the carrier only technically admits.
-- ════════════════════════════════════════════════════════════════════════════

-- Heights `0 < 1`: the bipartition IS the rank, since every vertex of the first
-- class is a pure source and every vertex of the second a pure sink.
k33-ranked : Ranked mono k33
Ranked.rank k33-ranked here = 0
Ranked.rank k33-ranked (there here) = 0
Ranked.rank k33-ranked (there (there here)) = 0
Ranked.rank k33-ranked (there (there (there here))) = 1
Ranked.rank k33-ranked (there (there (there (there here)))) = 1
Ranked.rank k33-ranked (there (there (there (there (there here))))) = 1
Ranked.rank k33-ranked (there (there (there (there (there (there ()))))))
Ranked.climbs k33-ranked here _ _ (forth flowing (attach refl refl)) = n<1+n 0
Ranked.climbs k33-ranked here _ _ (back () _)
Ranked.climbs
  k33-ranked (there here) _ _ (forth flowing (attach refl refl)) = n<1+n 0
Ranked.climbs k33-ranked (there here) _ _ (back () _)
Ranked.climbs
  k33-ranked (there (there here)) _ _ (forth flowing (attach refl refl)) =
  n<1+n 0
Ranked.climbs k33-ranked (there (there here)) _ _ (back () _)
Ranked.climbs
  k33-ranked
  (there (there (there here)))
  _
  _
  (forth flowing (attach refl refl)) =
  n<1+n 0
Ranked.climbs k33-ranked (there (there (there here))) _ _ (back () _)
Ranked.climbs
  k33-ranked
  (there (there (there (there here))))
  _
  _
  (forth flowing (attach refl refl)) =
  n<1+n 0
Ranked.climbs k33-ranked (there (there (there (there here)))) _ _ (back () _)
Ranked.climbs
  k33-ranked
  (there (there (there (there (there here)))))
  _
  _
  (forth flowing (attach refl refl)) =
  n<1+n 0
Ranked.climbs
  k33-ranked (there (there (there (there (there here))))) _ _ (back () _)
Ranked.climbs
  k33-ranked
  (there (there (there (there (there (there here))))))
  _
  _
  (forth flowing (attach refl refl)) =
  n<1+n 0
Ranked.climbs
  k33-ranked (there (there (there (there (there (there here)))))) _ _ (back () _)
Ranked.climbs
  k33-ranked
  (there (there (there (there (there (there (there here)))))))
  _
  _
  (forth flowing (attach refl refl)) =
  n<1+n 0
Ranked.climbs
  k33-ranked
  (there (there (there (there (there (there (there here)))))))
  _
  _
  (back () _)
Ranked.climbs
  k33-ranked
  (there (there (there (there (there (there (there (there here))))))))
  _
  _
  (forth flowing (attach refl refl)) =
  n<1+n 0
Ranked.climbs
  k33-ranked
  (there (there (there (there (there (there (there (there here))))))))
  _
  _
  (back () _)

-- So the witness carries no feedback: it is inside the fragment the carrier's
-- directed layer admits, not outside it.
k33-wheel-free : WheelFree mono k33
k33-wheel-free = ranked⇒wheel-free k33-ranked

-- and it is CONNECTED, which is the hypothesis class the theorem being consumed
-- quantifies over — a disconnected witness would answer a weaker question.
k33-connected : Connected k33
Connected.root k33-connected = v₀
Connected.span k33-connected here = stop
Connected.span k33-connected (there here) =
  onward (onward stop (adj e₆ (along a₀₅))) (adj e₃ (against a₁₅))
Connected.span k33-connected (there (there here)) =
  onward (onward stop (adj e₆ (along a₀₅))) (adj e₀ (against a₂₅))
Connected.span k33-connected (there (there (there here))) =
  onward stop (adj e₈ (along a₀₃))
Connected.span k33-connected (there (there (there (there here)))) =
  onward stop (adj e₇ (along a₀₄))
Connected.span k33-connected (there (there (there (there (there here))))) =
  onward stop (adj e₆ (along a₀₅))
Connected.span k33-connected (there (there (there (there (there (there ()))))))

-- ════════════════════════════════════════════════════════════════════════════
-- AND IT IS NOT A CELL, which is the distinction the whole result turns on. A
-- `Cell` is connected and acyclic — a tree — and trees are planar, so the same
-- question asked of ONE CELL has the opposite answer. Diagrams are not cells:
-- `Gandr.Shape.Graft` says so at the operation, and `diamond` already exhibits
-- a connected wheel-free shape that is not simply connected.
-- ════════════════════════════════════════════════════════════════════════════

-- `0` along edge `6` to `5`, against edge `3` to `1`, along edge `4` to `4`,
-- against edge `7` back to `0`. Every step uses a different edge from the one
-- before it, so the walk is reduced — which is what makes it a cycle rather
-- than a there-and-back.
k33-cycle : Walk k33 v₀ v₀ (just e₇)
k33-cycle =
  hop e₇
    (hop e₄
      (hop e₃
        (hop e₆ stay (along a₀₅) opening)
        (against a₁₅)
        (apart (λ ())))
      (along a₁₄)
      (apart (λ ())))
    (against a₀₄)
    (apart (λ ()))

k33-cyclic : ¬ Acyclic k33
k33-cyclic ac = ac k33-cycle

k33-not-simply : ¬ SimplyConn k33
k33-not-simply sc = SimplyConn.acyclic sc k33-cycle
