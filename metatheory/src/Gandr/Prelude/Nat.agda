{-# OPTIONS --safe --without-K --hidden-argument-puns #-}

------------------------------------------------------------------------------
-- Gandr.Prelude.Nat — the natural-number facade.
--
-- The flat-arena presentation computes with OFFSETS: a value of a code is a
-- position in a flat run of cells, and every structural transformation is
-- arithmetic on that position. So the semiring laws of `Nat` are not
-- incidental plumbing here — under the flat presentation they ARE the
-- structural coherence of the code universe, which is the whole point of the
-- recast (Gandr.Arena.Offset turns them into the offset laws).
--
-- Facade discipline (docs/workflow/agda.md): re-export under stdlib's own
-- names, since inventing a parallel vocabulary for `+-assoc` buys nothing and
-- costs a translation layer that can be got wrong. What the facade buys is the
-- single import point.
------------------------------------------------------------------------------

module Gandr.Prelude.Nat where

open import Data.Nat.Base public
  using
    ( zero
    ; suc
    ; _+_
    ; _*_
    )
  renaming
    ( ℕ to Nat
    )

open import Data.Nat.Properties public
  using
    ( +-assoc
    ; +-comm
    ; +-identityʳ
    ; *-assoc
    ; *-comm
    ; *-distribˡ-+
    ; *-distribʳ-+
    ; *-identityʳ
    ; *-identityˡ
    ; *-zeroʳ
    )
