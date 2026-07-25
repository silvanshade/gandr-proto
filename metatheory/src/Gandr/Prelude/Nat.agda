{-# OPTIONS --safe --without-K --hidden-argument-puns #-}

------------------------------------------------------------------------------
-- Gandr.Prelude.Nat — the offset arithmetic kit.
--
-- The flat-arena presentation (Gandr.Spike.FlatCoherence) computes with
-- OFFSETS: a value of a code is a position in a flat run of cells, and every
-- structural transformation is arithmetic on that position. So the semiring
-- laws of `Nat` are not incidental plumbing here — they ARE the structural
-- coherence of the code universe, which is the whole point of the flat recast.
--
-- Self-contained over `Agda.Builtin` (no agda-stdlib yet: vendoring rides
-- gandr-5lf.9.1, and the spike must be runnable before it lands). Each law is
-- proved by the shortest structural induction that closes it, in the flat
-- proof-term style the house discipline asks for.
------------------------------------------------------------------------------

module Gandr.Prelude.Nat where

open import Agda.Builtin.Nat public using (Nat; zero; suc; _+_; _*_)
open import Agda.Builtin.Equality public using (_≡_; refl)

-- ════════════════════════════════════════════════════════════════════════════
-- Equality plumbing. J-derived; nothing here needs K.
-- ════════════════════════════════════════════════════════════════════════════

sym : {A : Set} {a b : A} → a ≡ b → b ≡ a
sym refl = refl

trans : {A : Set} {a b c : A} → a ≡ b → b ≡ c → a ≡ c
trans refl q = q

cong : {A B : Set} (f : A → B) {a b : A} → a ≡ b → f a ≡ f b
cong f refl = refl

cong₂ : {A B C : Set} (f : A → B → C) {a a′ : A} {b b′ : B}
      → a ≡ a′ → b ≡ b′ → f a b ≡ f a′ b′
cong₂ f refl refl = refl

-- ════════════════════════════════════════════════════════════════════════════
-- Additive laws. `_+_` recurses on its LEFT argument (`suc n + m = suc (n + m)`),
-- so left-nested statements close definitionally and right-nested ones need the
-- induction.
-- ════════════════════════════════════════════════════════════════════════════

+-assoc : (a b c : Nat) → (a + b) + c ≡ a + (b + c)
+-assoc zero    b c = refl
+-assoc (suc a) b c = cong suc (+-assoc a b c)

+-idr : (a : Nat) → a + zero ≡ a
+-idr zero    = refl
+-idr (suc a) = cong suc (+-idr a)

+-suc : (a b : Nat) → a + suc b ≡ suc (a + b)
+-suc zero    b = refl
+-suc (suc a) b = cong suc (+-suc a b)

+-comm : (a b : Nat) → a + b ≡ b + a
+-comm a zero    = +-idr a
+-comm a (suc b) = trans (+-suc a b) (cong suc (+-comm a b))

-- ════════════════════════════════════════════════════════════════════════════
-- Multiplicative laws. `zero * m = zero` and `suc n * m = m + n * m`, so the
-- multiplier distributes on the RIGHT by induction on the left factor.
-- ════════════════════════════════════════════════════════════════════════════

*-idr : (a : Nat) → a * suc zero ≡ a
*-idr zero    = refl
*-idr (suc a) = cong suc (*-idr a)

*-zeror : (a : Nat) → a * zero ≡ zero
*-zeror zero    = refl
*-zeror (suc a) = *-zeror a

-- (a + b) * c ≡ a * c + b * c
*-distribʳ : (a b c : Nat) → (a + b) * c ≡ a * c + b * c
*-distribʳ zero    b c = refl
*-distribʳ (suc a) b c =
  trans (cong (λ z → c + z) (*-distribʳ a b c))
        (sym (+-assoc c (a * c) (b * c)))

*-assoc : (a b c : Nat) → (a * b) * c ≡ a * (b * c)
*-assoc zero    b c = refl
*-assoc (suc a) b c =
  trans (*-distribʳ b (a * b) c)
        (cong (λ z → b * c + z) (*-assoc a b c))

-- a * (b + c) ≡ a * b + a * c — the DISTRIBUTOR's arithmetic shadow. Unlike the
-- laws above it is not needed to show a generator is the identity; it is the
-- law the `dist` generator's offset formula is measured against, which is
-- exactly why `dist` survives the flattening as genuine content.
*-distribˡ : (a b c : Nat) → a * (b + c) ≡ a * b + a * c
*-distribˡ zero    b c = refl
*-distribˡ (suc a) b c =
  trans (cong (λ z → (b + c) + z) (*-distribˡ a b c))
        (trans (+-assoc b c (a * b + a * c))
        (trans (cong (λ z → b + z) (sym (+-assoc c (a * b) (a * c))))
        (trans (cong (λ z → b + (z + a * c)) (+-comm c (a * b)))
        (trans (cong (λ z → b + z) (+-assoc (a * b) c (a * c)))
               (sym (+-assoc b (a * b) (c + a * c)))))))
