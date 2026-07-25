{-# OPTIONS --safe --without-K --hidden-argument-puns #-}

------------------------------------------------------------------------------
-- Gandr.Arena.Value — bounded values, and the grade laws are stated at.
--
-- A value of a code is a BOUNDED offset: a cell position within that code's
-- flat run. The bound rides in the carrier (`Fin (size c)`) rather than being
-- threaded as a side condition, so no law in this tree ever pays for it.
--
-- `Val` is a record rather than a bare `Fin (size c)` for a structural reason,
-- not a cosmetic one: `size` is a defined function, so `Fin (size c)` does not
-- determine `c`, and every constructor's code would have to be supplied by
-- hand at every use site. Wrapping restores inference, and matches the house
-- preference for purpose-built records.
--
-- ── THE GRADE (binding, from the lane's statement-shape contract) ───────────
-- Identity is `_≐_`: two values are the same when they name the same cell.
-- Deliberately HETEROGENEOUS in the code — `x ≐ y` is an equation in `Nat`,
-- so a law relating `Val c` to `Val d` needs no transport, no cast and no
-- record equality even though the two are different types. This is the flat
-- analogue of the value-setoid discipline the lane inherits: laws are
-- witnessed componentwise, at the grade the engine actually compares at.
--
-- `≐-≡` reads the grade back: at a FIXED code, naming the same cell IS
-- equality (`Fin` is discrete on `toℕ`, and the record has η). So nothing is
-- lost by working at the coarser grade — it is coarser only ACROSS codes,
-- which is exactly where the transports would otherwise have been.
--
-- ── THE INTERFACE ───────────────────────────────────────────────────────────
-- `pair`/`inl`/`inr` construct, `⊗split`/`⊕split` eliminate, β and η relate
-- them, `⊗-ind`/`⊕-ind` do induction at the grade, and `ix-pair`/`ix-inl`/
-- `ix-inr` connect the algebra to the offset arithmetic of Gandr.Arena.Offset.
-- Everything downstream speaks through this interface: no proof below reaches
-- for `combine`, `splitAt` or a raw `Fin`, so what is being reasoned about is
-- the arena's algebra rather than the encoding that happens to implement it.
------------------------------------------------------------------------------

module Gandr.Arena.Value where

open import Gandr.Arena.Code
open import Gandr.Arena.Offset
open import Gandr.Prelude.Data
open import Gandr.Prelude.Equality
open import Gandr.Prelude.Fin
open import Gandr.Prelude.Nat

-- ════════════════════════════════════════════════════════════════════════════
-- The carrier and the grade.
-- ════════════════════════════════════════════════════════════════════════════

-- A value is a cell position within the code's flat run.
record Val (c : Code) : Set where
  constructor cell
  field
    pos : Fin (size c)

open Val public

-- The raw offset a value names. This is what the grade is measured at.
ix : {c : Code} → Val c → Nat
ix x = toℕ (pos x)

infix 4 _≐_

-- Value-grade identity: naming the same cell. Heterogeneous in the code, so a
-- law that changes the code's shape is still a plain equation in `Nat`.
_≐_ : {c d : Code} → Val c → Val d → Set
x ≐ y = ix x ≡ ix y

-- The grade is an equivalence, inherited from equality of offsets.
≐-refl : {c : Code} {x : Val c} → x ≐ x
≐-refl = refl

≐-sym : {c d : Code} {x : Val c} {y : Val d} → x ≐ y → y ≐ x
≐-sym p = sym p

≐-trans : {c d e : Code} {x : Val c} {y : Val d} {z : Val e} → x ≐ y → y ≐ z → x ≐ z
≐-trans p q = trans p q

-- Structural equality refines the grade.
≡-≐ : {c : Code} {x y : Val c} → x ≡ y → x ≐ y
≡-≐ p = cong ix p

-- …and at a fixed code the two coincide, so the coarser grade loses nothing
-- except the transports it was adopted to avoid.
≐-≡ : {c : Code} {x y : Val c} → x ≐ y → x ≡ y
≐-≡ p = cong cell (toℕ-injective p)

-- Re-index a value-grade equation along a structural equation on its input.
-- The induction principles below are the only consumers; keeping it here
-- keeps every downstream proof a flat ladder.
≐-at
  : {c z z′ : Code}
  → (f : Val c → Val z)
  → (g : Val c → Val z′)
  → {x y : Val c}
  → x ≡ y
  → f x ≐ g x
  → f y ≐ g y
≐-at f g refl h = h

-- ════════════════════════════════════════════════════════════════════════════
-- Constructors, and their offsets.
-- ════════════════════════════════════════════════════════════════════════════

-- The unit code spans one cell, at offset 0.
unit : Val 𝟙
unit = cell fzero

-- Row-major pairing.
pair : {c d : Code} → Val c → Val d → Val (c ⊗ d)
pair u v = cell (combine (pos u) (pos v))

-- Injection into the low block.
inl : {c d : Code} → Val c → Val (c ⊕ d)
inl {d} i = cell (pos i ↑ˡ size d)

-- Injection into the high block.
inr : {c d : Code} → Val d → Val (c ⊕ d)
inr {c} j = cell (size c ↑ʳ pos j)

-- Each constructor computes the offset its Gandr.Arena.Offset formula names.
-- These are the whole bridge between the algebra and the arithmetic; every
-- collapse theorem downstream factors through them.
ix-unit : ix unit ≡ zero
ix-unit = refl

ix-pair
  : {c d : Code} (u : Val c) (v : Val d)
  → ix (pair u v) ≡ ⊗ix (size d) (ix u) (ix v)
ix-pair u v = toℕ-combine (pos u) (pos v)

ix-inl
  : {c d : Code} (i : Val c)
  → ix (inl {c} {d} i) ≡ ⊕ixˡ (ix i)
ix-inl {d} i = toℕ-↑ˡ (pos i) (size d)

ix-inr
  : {c d : Code} (j : Val d)
  → ix (inr {c} {d} j) ≡ ⊕ixʳ (size c) (ix j)
ix-inr {c} j = toℕ-↑ʳ (size c) (pos j)

-- ════════════════════════════════════════════════════════════════════════════
-- Elimination. The encoding's inverses (`remQuot`, `splitAt`, `join`) appear
-- ONLY in this section; downstream proofs speak through `⊗split`/`⊕split` and
-- the β/η rules, and never mention the encoding again.
-- ════════════════════════════════════════════════════════════════════════════

-- Read a decomposed pair of raw offsets back as a pair of values. Its only
-- job is to PIN the decomposition's type before `remQuot` is elaborated:
-- `size` is not injective, so the factor extents cannot be recovered from the
-- product's, and an unannotated `remQuot` leaves an unsolved constraint.
cells : {c d : Code} → Fin (size c) × Fin (size d) → Val c × Val d
cells p = cell (proj₁ p) , cell (proj₂ p)

-- The same service on the sum side.
injs : {c d : Code} → Fin (size c) ⊎ Fin (size d) → Val c ⊎ Val d
injs = [ (λ i → inj₁ (cell i)) , (λ j → inj₂ (cell j)) ]′

-- Decompose a product value into its two factors.
⊗split : {c d : Code} → Val (c ⊗ d) → Val c × Val d
⊗split {c} {d} x = cells {c} {d} (remQuot (size d) (pos x))

-- Decide which block of a sum a value lies in, and give its position there.
⊕split : {c d : Code} → Val (c ⊕ d) → Val c ⊎ Val d
⊕split {c} {d} x = injs {c} {d} (splitAt (size c) (pos x))

-- The β-rules: eliminating a constructor returns what was built.
⊗split-pair
  : {c d : Code} (u : Val c) (v : Val d)
  → ⊗split (pair u v) ≡ (u , v)
⊗split-pair {c} {d} u v = cong (cells {c} {d}) (remQuot-combine (pos u) (pos v))

⊕split-inl
  : {c d : Code} (i : Val c)
  → ⊕split (inl {c} {d} i) ≡ inj₁ i
⊕split-inl {c} {d} i = cong (injs {c} {d}) (splitAt-↑ˡ (size c) (pos i) (size d))

⊕split-inr
  : {c d : Code} (j : Val d)
  → ⊕split (inr {c} {d} j) ≡ inj₂ j
⊕split-inr {c} {d} j = cong (injs {c} {d}) (splitAt-↑ʳ (size c) (size d) (pos j))

-- The η-rules: every value is in constructor form. These are what make the
-- induction principles complete — the arena has no values the algebra cannot
-- name.
⊗-η
  : {c d : Code} (x : Val (c ⊗ d))
  → pair (proj₁ (⊗split x)) (proj₂ (⊗split x)) ≡ x
⊗-η {c} {d} x = cong cell (combine-remQuot {size c} (size d) (pos x))

⊕-η
  : {c d : Code} (x : Val (c ⊕ d))
  → [ inl , inr ]′ (⊕split x) ≡ x
⊕-η {c} {d} x with splitAt (size c) (pos x) in eq
... | inj₁ i = cong cell (trans (cong (join (size c) (size d)) (sym eq))
                                (join-splitAt (size c) (size d) (pos x)))
... | inj₂ j = cong cell (trans (cong (join (size c) (size d)) (sym eq))
                                (join-splitAt (size c) (size d) (pos x)))

-- To compare two maps out of a product at the grade, compare them on pairs.
⊗-ind
  : {c d z z′ : Code}
  → (f : Val (c ⊗ d) → Val z)
  → (g : Val (c ⊗ d) → Val z′)
  → ((u : Val c) (v : Val d) → f (pair u v) ≐ g (pair u v))
  → (x : Val (c ⊗ d))
  → f x ≐ g x
⊗-ind f g h x = ≐-at f g (⊗-η x) (h (proj₁ (⊗split x)) (proj₂ (⊗split x)))

-- To compare two maps out of a sum at the grade, compare them on injections.
⊕-ind
  : {c d z z′ : Code}
  → (f : Val (c ⊕ d) → Val z)
  → (g : Val (c ⊕ d) → Val z′)
  → ((i : Val c) → f (inl i) ≐ g (inl i))
  → ((j : Val d) → f (inr j) ≐ g (inr j))
  → (x : Val (c ⊕ d))
  → f x ≐ g x
⊕-ind f g hl hr x with ⊕split x in eq
... | inj₁ i = ≐-at f g (trans (cong [ inl , inr ]′ (sym eq)) (⊕-η x)) (hl i)
... | inj₂ j = ≐-at f g (trans (cong [ inl , inr ]′ (sym eq)) (⊕-η x)) (hr j)

-- ════════════════════════════════════════════════════════════════════════════
-- Whiskering. A structural generator is applied INSIDE a larger code by
-- mapping over a factor or a summand; this is the positional closure the
-- coherence diagrams are stated over. Without it a pentagon could only be
-- checked at the top level, which is precisely the case the presented
-- calculus finds easy.
-- ════════════════════════════════════════════════════════════════════════════

-- Map both factors of a product.
⊗map
  : {c c′ d d′ : Code}
  → (Val c → Val c′)
  → (Val d → Val d′)
  → Val (c ⊗ d) → Val (c′ ⊗ d′)
⊗map f g x = pair (f (proj₁ (⊗split x))) (g (proj₂ (⊗split x)))

-- Map both summands of a sum.
⊕map
  : {c c′ d d′ : Code}
  → (Val c → Val c′)
  → (Val d → Val d′)
  → Val (c ⊕ d) → Val (c′ ⊕ d′)
⊕map f g x = [ (λ i → inl (f i)) , (λ j → inr (g j)) ]′ (⊕split x)

-- The computation rules. Whiskering is the encoding's round trip, so each
-- rule is the corresponding β-rule pushed through the mapped functions.
⊗map-pair
  : {c c′ d d′ : Code}
  → (f : Val c → Val c′) (g : Val d → Val d′)
  → (u : Val c) (v : Val d)
  → ⊗map f g (pair u v) ≡ pair (f u) (g v)
⊗map-pair f g u v =
  cong (λ p → pair (f (proj₁ p)) (g (proj₂ p))) (⊗split-pair u v)

⊕map-inl
  : {c c′ d d′ : Code}
  → (f : Val c → Val c′) (g : Val d → Val d′)
  → (i : Val c)
  → ⊕map f g (inl {c} {d} i) ≡ inl (f i)
⊕map-inl f g i =
  cong [ (λ i′ → inl (f i′)) , (λ j → inr (g j)) ]′ (⊕split-inl i)

⊕map-inr
  : {c c′ d d′ : Code}
  → (f : Val c → Val c′) (g : Val d → Val d′)
  → (j : Val d)
  → ⊕map f g (inr {c} {d} j) ≡ inr (g j)
⊕map-inr f g j =
  cong [ (λ i → inl (f i)) , (λ j′ → inr (g j′)) ]′ (⊕split-inr j)
