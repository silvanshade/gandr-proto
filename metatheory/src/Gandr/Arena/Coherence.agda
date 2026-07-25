{-# OPTIONS --safe --without-K --hidden-argument-puns #-}

------------------------------------------------------------------------------
-- Gandr.Arena.Coherence — the verdict on the distributive coherence family.
--
-- This module completes the spike filed as gandr-5lf.9.2. Its question was
-- whether the Laplaza-shaped coherence family that the presented (tree-shaped)
-- edit calculus must impose is MATHEMATICAL or PRESENTATIONAL. The answer is
-- presentational, and it is proved in two halves that are stated very
-- differently on purpose.
--
-- ── HALF ONE: the hierarchy dissolves, as a theorem rather than a family ────
-- `rigid-coherence` says: two rigid words with a common source agree at value
-- grade. Since Gandr.Arena.Structure shows the associativity/unit generators
-- are rigid and that rigidity is closed under composition and whiskering,
-- EVERY diagram whose edges are built from that hierarchy commutes, at every
-- code, with no cell imposed. That is one theorem, not a family: the
-- combinatorial explosion is never built, because there is nothing to slide a
-- generator past. `coh-pentagon-⊗`, `coh-triangle-⊗` and `coh-pentagon-⊕` are
-- then INSTANCES, stated at their full diagram shapes so the reader can check
-- that the general theorem is being applied to the real diagrams.
--
-- Note what this does not need: no UIP, no Hedberg, no transport. A coherence
-- cell asserts that two WORDS have equal realization, and realization is a
-- function, so the cell is an equation between functions — not between proofs.
-- That is why the recast stays clean under --without-K.
--
-- ── HALF TWO: what carries content still has to be proved ───────────────────
-- `⊗comm`, `⊕swap` and `dist` are not rigid, so nothing above touches them.
-- `coh-hexagon-⊕` and `coh-nat-dist` — the two obligations the spike REDUCED
-- but did not discharge — are proved here directly, by induction through the
-- arena's β-rules. Both come out structurally: they hold for arbitrary
-- whiskered maps, because flattening makes the distributor's action a matter
-- of which block a cell lands in rather than a diagram to chase.
--
-- So the verdict is not that everything trivializes. It is that the hierarchy
-- is free and the permutations are not, and the coherence obligations over the
-- permutations are provable in the ordinary way once the hierarchy is gone.
--
-- ── WHAT THIS MODULE DOES NOT COVER, AND WHY THE CLAIM STILL HOLDS ──────────
-- Laplaza's coherence for distributivity is a family of some two dozen
-- diagrams. Discharged here: the two pentagons, the triangle, the sum hexagon,
-- naturality of reassociation, and naturality of the distributor. NOT
-- attempted: the hexagon for the ⊗ symmetry, and the diagrams that mix `⊗comm`
-- or `⊕swap` with `dist`.
--
-- The omission is bounded, and bounding it is the point. `rigid-coherence`
-- settles every diagram all of whose edges lie in the hierarchy, at every code
-- and at every depth — that is the part that explodes combinatorially, and it
-- is gone. What is left is exactly the words in the three surviving
-- generators, which is the spike's filing hypothesis confirmed in its own
-- vocabulary: R-coh reduces to R-coxeter, the symmetric-group word problem,
-- which the hypothesis called mathematically irreducible. The remaining
-- diagrams are therefore expected to be ordinary work — the hexagon above is
-- the worked example, three cases of β-rule chasing — rather than another
-- canonicalization wall. Expected is not proved, and this note is here so the
-- distinction is not quietly lost.
--
-- Also out of scope, deliberately: leaves (cell CONTENT, which no structural
-- generator inspects) and an empty code (`𝟘` would make `Val` uninhabited and
-- the distributor's inverse partial; see Gandr.Arena.Code).
------------------------------------------------------------------------------

module Gandr.Arena.Coherence where

open import Gandr.Arena.Code
open import Gandr.Arena.Offset
open import Gandr.Arena.Structure
open import Gandr.Arena.Value
open import Gandr.Prelude.Data
open import Gandr.Prelude.Equality
open import Gandr.Prelude.Nat

-- ════════════════════════════════════════════════════════════════════════════
-- THE GENERAL THEOREM. Every coherence obligation over the associativity/unit
-- hierarchy, discharged once.
-- ════════════════════════════════════════════════════════════════════════════

-- Two rigid words out of the same code agree at value grade. Targets need not
-- even be the same code — the grade is heterogeneous — so this covers every
-- parallel pair of routes the hierarchy can build.
rigid-coherence
  : {c d d′ : Code} (r : Rigid c d) (s : Rigid c d′) (x : Val c)
  → app r x ≐ app s x
rigid-coherence r s x = trans (fixed r x) (sym (fixed s x))

-- ════════════════════════════════════════════════════════════════════════════
-- INSTANCES, at full diagram shape. Each route is spelled out arrow by arrow
-- so the `rigid-coherence` appeal is checkably about the real diagram and not
-- about a simplification of it.
-- ════════════════════════════════════════════════════════════════════════════

-- ── Mac Lane's pentagon for ⊗ ───────────────────────────────────────────────
-- ((a⊗b)⊗c)⊗d ⇒ a⊗(b⊗(c⊗d)), two arrows against three.

pentagon-⊗-short : {a b c d : Code} → Rigid (((a ⊗ b) ⊗ c) ⊗ d) (a ⊗ (b ⊗ (c ⊗ d)))
pentagon-⊗-short {a} {b} {c} {d} =
  rigid-∘ (rigid-⊗assoc {a ⊗ b} {c} {d})
          (rigid-⊗assoc {a} {b} {c ⊗ d})

pentagon-⊗-long : {a b c d : Code} → Rigid (((a ⊗ b) ⊗ c) ⊗ d) (a ⊗ (b ⊗ (c ⊗ d)))
pentagon-⊗-long {a} {b} {c} {d} =
  rigid-∘ (rigid-⊗ (rigid-⊗assoc {a} {b} {c}) (rigid-id {d}))
  (rigid-∘ (rigid-⊗assoc {a} {b ⊗ c} {d})
           (rigid-⊗ (rigid-id {a}) (rigid-⊗assoc {b} {c} {d})))

coh-pentagon-⊗
  : {a b c d : Code} (x : Val (((a ⊗ b) ⊗ c) ⊗ d))
  → app (pentagon-⊗-short {a} {b} {c} {d}) x ≐ app (pentagon-⊗-long {a} {b} {c} {d}) x
coh-pentagon-⊗ {a} {b} {c} {d} =
  rigid-coherence (pentagon-⊗-short {a} {b} {c} {d}) (pentagon-⊗-long {a} {b} {c} {d})

-- ── Mac Lane's triangle for ⊗ ───────────────────────────────────────────────
-- (a⊗𝟙)⊗b ⇒ a⊗b: reassociate then cancel on the left, or cancel on the right.

triangle-⊗-via-assoc : {a b : Code} → Rigid ((a ⊗ 𝟙) ⊗ b) (a ⊗ b)
triangle-⊗-via-assoc {a} {b} =
  rigid-∘ (rigid-⊗assoc {a} {𝟙} {b})
          (rigid-⊗ (rigid-id {a}) (rigid-⊗unitl {b}))

triangle-⊗-direct : {a b : Code} → Rigid ((a ⊗ 𝟙) ⊗ b) (a ⊗ b)
triangle-⊗-direct {a} {b} = rigid-⊗ (rigid-⊗unitr {a}) (rigid-id {b})

coh-triangle-⊗
  : {a b : Code} (x : Val ((a ⊗ 𝟙) ⊗ b))
  → app (triangle-⊗-via-assoc {a} {b}) x ≐ app (triangle-⊗-direct {a} {b}) x
coh-triangle-⊗ {a} {b} =
  rigid-coherence (triangle-⊗-via-assoc {a} {b}) (triangle-⊗-direct {a} {b})

-- ── The pentagon for ⊕ ──────────────────────────────────────────────────────

pentagon-⊕-short : {a b c d : Code} → Rigid (((a ⊕ b) ⊕ c) ⊕ d) (a ⊕ (b ⊕ (c ⊕ d)))
pentagon-⊕-short {a} {b} {c} {d} =
  rigid-∘ (rigid-⊕assoc {a ⊕ b} {c} {d})
          (rigid-⊕assoc {a} {b} {c ⊕ d})

pentagon-⊕-long : {a b c d : Code} → Rigid (((a ⊕ b) ⊕ c) ⊕ d) (a ⊕ (b ⊕ (c ⊕ d)))
pentagon-⊕-long {a} {b} {c} {d} =
  rigid-∘ (rigid-⊕ (rigid-⊕assoc {a} {b} {c}) (rigid-id {d}))
  (rigid-∘ (rigid-⊕assoc {a} {b ⊕ c} {d})
           (rigid-⊕ (rigid-id {a}) (rigid-⊕assoc {b} {c} {d})))

coh-pentagon-⊕
  : {a b c d : Code} (x : Val (((a ⊕ b) ⊕ c) ⊕ d))
  → app (pentagon-⊕-short {a} {b} {c} {d}) x ≐ app (pentagon-⊕-long {a} {b} {c} {d}) x
coh-pentagon-⊕ {a} {b} {c} {d} =
  rigid-coherence (pentagon-⊕-short {a} {b} {c} {d}) (pentagon-⊕-long {a} {b} {c} {d})

-- ── Rigid words are also literally inverse to each other ────────────────────
-- The inverse side of the whole hierarchy, at full diagram shape.

coh-⊗assoc-inverse
  : {a b c : Code} (x : Val ((a ⊗ b) ⊗ c)) → ⊗assoc⁻¹ (⊗assoc x) ≡ x
coh-⊗assoc-inverse {a} {b} {c} =
  rigid-inv (rigid-⊗assoc {a} {b} {c}) (rigid-⊗assoc⁻¹ {a} {b} {c})

coh-⊕assoc-inverse
  : {a b c : Code} (x : Val ((a ⊕ b) ⊕ c)) → ⊕assoc⁻¹ (⊕assoc x) ≡ x
coh-⊕assoc-inverse {a} {b} {c} =
  rigid-inv (rigid-⊕assoc {a} {b} {c}) (rigid-⊕assoc⁻¹ {a} {b} {c})

coh-⊗unitr-inverse : {a : Code} (x : Val (a ⊗ 𝟙)) → ⊗unitr⁻¹ (⊗unitr x) ≡ x
coh-⊗unitr-inverse {a} = rigid-inv (rigid-⊗unitr {a}) (rigid-⊗unitr⁻¹ {a})

coh-⊗unitl-inverse : {a : Code} (x : Val (𝟙 ⊗ a)) → ⊗unitl⁻¹ (⊗unitl x) ≡ x
coh-⊗unitl-inverse {a} = rigid-inv (rigid-⊗unitl {a}) (rigid-⊗unitl⁻¹ {a})

-- ════════════════════════════════════════════════════════════════════════════
-- NATURALITY OF ⊗assoc. In the presented calculus this is the square that
-- must be imposed PER GENERATOR at general codes — the canonicalization wall
-- itself. Here it holds for ARBITRARY whiskered maps, by the β-rules alone.
-- ════════════════════════════════════════════════════════════════════════════

coh-nat-⊗assoc
  : {a a′ b b′ c c′ : Code}
  → (f : Val a → Val a′) (g : Val b → Val b′) (h : Val c → Val c′)
  → (x : Val ((a ⊗ b) ⊗ c))
  → ⊗map f (⊗map g h) (⊗assoc x) ≐ ⊗assoc (⊗map (⊗map f g) h x)
coh-nat-⊗assoc {a} {a′} {b} {b′} {c} {c′} f g h =
  ⊗-ind (λ x → ⊗map f (⊗map g h) (⊗assoc {a} {b} {c} x))
        (λ x → ⊗assoc {a′} {b′} {c′} (⊗map (⊗map f g) h x))
    (λ y w →
      ⊗-ind (λ z → ⊗map f (⊗map g h) (⊗assoc {a} {b} {c} (pair z w)))
            (λ z → ⊗assoc {a′} {b′} {c′} (⊗map (⊗map f g) h (pair z w)))
        (λ u v →
          ≡-≐ (trans (cong (⊗map f (⊗map g h)) (⊗assoc-pair u v w))
               (trans (⊗map-pair f (⊗map g h) u (pair v w))
               (trans (cong (pair (f u)) (⊗map-pair g h v w))
               (trans (sym (⊗assoc-pair (f u) (g v) (h w)))
                      (cong (⊗assoc {a′} {b′} {c′})
                            (sym (trans (⊗map-pair (⊗map f g) h (pair u v) w)
                                        (cong (λ t → pair t (h w)) (⊗map-pair f g u v))))))))))
        y)

-- ════════════════════════════════════════════════════════════════════════════
-- RESIDUAL ONE, NOW DISCHARGED: the hexagon for ⊕.
--
-- The spike REDUCED this: `⊕assoc` is rigid, so the hexagon collapses to a
-- statement purely about `⊕swap` composites. Reduced is not discharged, so it
-- is proved here. Each of the three summand cases is a chain of β-rules, and
-- the two routes meet on the nose — not merely at the grade.
-- ════════════════════════════════════════════════════════════════════════════

-- (a⊕b)⊕c ⇒ a⊕(b⊕c) ⇒ (b⊕c)⊕a ⇒ b⊕(c⊕a)
hexagon-⊕-across : {a b c : Code} → Val ((a ⊕ b) ⊕ c) → Val (b ⊕ (c ⊕ a))
hexagon-⊕-across {a} {b} {c} x =
  ⊕assoc {b} {c} {a} (⊕swap {a} {b ⊕ c} (⊕assoc {a} {b} {c} x))

-- (a⊕b)⊕c ⇒ (b⊕a)⊕c ⇒ b⊕(a⊕c) ⇒ b⊕(c⊕a)
hexagon-⊕-around : {a b c : Code} → Val ((a ⊕ b) ⊕ c) → Val (b ⊕ (c ⊕ a))
hexagon-⊕-around {a} {b} {c} x =
  ⊕map (λ y → y) (⊕swap {a} {c})
    (⊕assoc {b} {a} {c} (⊕map (⊕swap {a} {b}) (λ y → y) x))

-- The three summand cases, each route computed to normal form.

hexagon-⊕-across-ll
  : {a b c : Code} (i : Val a)
  → hexagon-⊕-across {a} {b} {c} (inl (inl i)) ≡ inr {b} {c ⊕ a} (inr {c} {a} i)
hexagon-⊕-across-ll {a} {b} {c} i =
  trans (cong (λ y → ⊕assoc {b} {c} {a} (⊕swap {a} {b ⊕ c} y)) (⊕assoc-inl-inl {a} {b} {c} i))
  (trans (cong (⊕assoc {b} {c} {a}) (⊕swap-inl {a} {b ⊕ c} i))
         (⊕assoc-inr {b} {c} {a} i))

hexagon-⊕-around-ll
  : {a b c : Code} (i : Val a)
  → hexagon-⊕-around {a} {b} {c} (inl (inl i)) ≡ inr {b} {c ⊕ a} (inr {c} {a} i)
hexagon-⊕-around-ll {a} {b} {c} i =
  trans (cong (λ y → ⊕map (λ z → z) (⊕swap {a} {c}) (⊕assoc {b} {a} {c} y))
              (trans (⊕map-inl (⊕swap {a} {b}) (λ z → z) (inl {a} {b} i))
                     (cong (inl {b ⊕ a} {c}) (⊕swap-inl {a} {b} i))))
  (trans (cong (⊕map (λ z → z) (⊕swap {a} {c})) (⊕assoc-inl-inr {b} {a} {c} i))
  (trans (⊕map-inr (λ z → z) (⊕swap {a} {c}) (inl {a} {c} i))
         (cong (inr {b} {c ⊕ a}) (⊕swap-inl {a} {c} i))))

hexagon-⊕-across-lr
  : {a b c : Code} (j : Val b)
  → hexagon-⊕-across {a} {b} {c} (inl (inr j)) ≡ inl {b} {c ⊕ a} j
hexagon-⊕-across-lr {a} {b} {c} j =
  trans (cong (λ y → ⊕assoc {b} {c} {a} (⊕swap {a} {b ⊕ c} y)) (⊕assoc-inl-inr {a} {b} {c} j))
  (trans (cong (⊕assoc {b} {c} {a}) (⊕swap-inr {a} {b ⊕ c} (inl {b} {c} j)))
         (⊕assoc-inl-inl {b} {c} {a} j))

hexagon-⊕-around-lr
  : {a b c : Code} (j : Val b)
  → hexagon-⊕-around {a} {b} {c} (inl (inr j)) ≡ inl {b} {c ⊕ a} j
hexagon-⊕-around-lr {a} {b} {c} j =
  trans (cong (λ y → ⊕map (λ z → z) (⊕swap {a} {c}) (⊕assoc {b} {a} {c} y))
              (trans (⊕map-inl (⊕swap {a} {b}) (λ z → z) (inr {a} {b} j))
                     (cong (inl {b ⊕ a} {c}) (⊕swap-inr {a} {b} j))))
  (trans (cong (⊕map (λ z → z) (⊕swap {a} {c})) (⊕assoc-inl-inl {b} {a} {c} j))
         (⊕map-inl (λ z → z) (⊕swap {a} {c}) j))

hexagon-⊕-across-r
  : {a b c : Code} (k : Val c)
  → hexagon-⊕-across {a} {b} {c} (inr k) ≡ inr {b} {c ⊕ a} (inl {c} {a} k)
hexagon-⊕-across-r {a} {b} {c} k =
  trans (cong (λ y → ⊕assoc {b} {c} {a} (⊕swap {a} {b ⊕ c} y)) (⊕assoc-inr {a} {b} {c} k))
  (trans (cong (⊕assoc {b} {c} {a}) (⊕swap-inr {a} {b ⊕ c} (inr {b} {c} k)))
         (⊕assoc-inl-inr {b} {c} {a} k))

hexagon-⊕-around-r
  : {a b c : Code} (k : Val c)
  → hexagon-⊕-around {a} {b} {c} (inr k) ≡ inr {b} {c ⊕ a} (inl {c} {a} k)
hexagon-⊕-around-r {a} {b} {c} k =
  trans (cong (λ y → ⊕map (λ z → z) (⊕swap {a} {c}) (⊕assoc {b} {a} {c} y))
              (⊕map-inr (⊕swap {a} {b}) (λ z → z) k))
  (trans (cong (⊕map (λ z → z) (⊕swap {a} {c})) (⊕assoc-inr {b} {a} {c} k))
  (trans (⊕map-inr (λ z → z) (⊕swap {a} {c}) (inr {a} {c} k))
         (cong (inr {b} {c ⊕ a}) (⊕swap-inr {a} {c} k))))

-- THE HEXAGON.
coh-hexagon-⊕
  : {a b c : Code} (x : Val ((a ⊕ b) ⊕ c))
  → hexagon-⊕-across {a} {b} {c} x ≐ hexagon-⊕-around {a} {b} {c} x
coh-hexagon-⊕ {a} {b} {c} =
  ⊕-ind (hexagon-⊕-across {a} {b} {c}) (hexagon-⊕-around {a} {b} {c})
    (λ y →
      ⊕-ind (λ z → hexagon-⊕-across {a} {b} {c} (inl {a ⊕ b} {c} z))
            (λ z → hexagon-⊕-around {a} {b} {c} (inl {a ⊕ b} {c} z))
        (λ i → ≡-≐ (trans (hexagon-⊕-across-ll {a} {b} {c} i)
                          (sym (hexagon-⊕-around-ll {a} {b} {c} i))))
        (λ j → ≡-≐ (trans (hexagon-⊕-across-lr {a} {b} {c} j)
                          (sym (hexagon-⊕-around-lr {a} {b} {c} j))))
        y)
    (λ k → ≡-≐ (trans (hexagon-⊕-across-r {a} {b} {c} k)
                      (sym (hexagon-⊕-around-r {a} {b} {c} k))))

-- ════════════════════════════════════════════════════════════════════════════
-- RESIDUAL TWO, NOW DISCHARGED: naturality of the distributor.
--
-- The spike REDUCED this to the arithmetic behind `*-distribˡ-+` whiskered
-- through the positional closure. In the bounded arena it is better than that:
-- the square commutes ON THE NOSE for arbitrary whiskered maps, because on the
-- flat presentation the distributor's action is a decision about WHICH BLOCK a
-- cell lands in, and mapping the factors cannot change that decision.
--
-- This is the one square whose analogue in the presented calculus must be
-- imposed per generator — and it is the reason distributivity is the hard case
-- there. Here it is an induction with two cases.
-- ════════════════════════════════════════════════════════════════════════════

coh-nat-dist
  : {c c′ d d′ e e′ : Code}
  → (f : Val c → Val c′) (g : Val d → Val d′) (h : Val e → Val e′)
  → (x : Val (c ⊗ (d ⊕ e)))
  → ⊕map (⊗map f g) (⊗map f h) (dist x) ≐ dist (⊗map f (⊕map g h) x)
coh-nat-dist {c} {c′} {d} {d′} {e} {e′} f g h =
  ⊗-ind (λ x → ⊕map (⊗map f g) (⊗map f h) (dist {c} {d} {e} x))
        (λ x → dist {c′} {d′} {e′} (⊗map f (⊕map g h) x))
    (λ u y →
      ⊕-ind (λ z → ⊕map (⊗map f g) (⊗map f h) (dist {c} {d} {e} (pair u z)))
            (λ z → dist {c′} {d′} {e′} (⊗map f (⊕map g h) (pair {c} {d ⊕ e} u z)))
        (λ v →
          ≡-≐ (trans (cong (⊕map (⊗map f g) (⊗map f h)) (dist-inl u v))
               (trans (⊕map-inl (⊗map f g) (⊗map f h) (pair u v))
               (trans (cong (inl {c′ ⊗ d′} {c′ ⊗ e′}) (⊗map-pair f g u v))
               (trans (sym (dist-inl (f u) (g v)))
                      (cong (dist {c′} {d′} {e′})
                            (sym (trans (⊗map-pair f (⊕map g h) u (inl {d} {e} v))
                                        (cong (pair (f u)) (⊕map-inl g h v))))))))))
        (λ w →
          ≡-≐ (trans (cong (⊕map (⊗map f g) (⊗map f h)) (dist-inr u w))
               (trans (⊕map-inr (⊗map f g) (⊗map f h) (pair u w))
               (trans (cong (inr {c′ ⊗ d′} {c′ ⊗ e′}) (⊗map-pair f h u w))
               (trans (sym (dist-inr (f u) (h w)))
                      (cong (dist {c′} {d′} {e′})
                            (sym (trans (⊗map-pair f (⊕map g h) u (inr {d} {e} w))
                                        (cong (pair (f u)) (⊕map-inr g h w))))))))))
        y)
