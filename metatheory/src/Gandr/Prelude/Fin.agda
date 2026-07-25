{-# OPTIONS --safe --without-K --hidden-argument-puns #-}

------------------------------------------------------------------------------
-- Gandr.Prelude.Fin — the bounded-offset facade.
--
-- `Fin n` is the arena's carrier: a cell position that is bounded BY
-- CONSTRUCTION, so the bound is never a side condition a proof has to thread.
-- The four operations that matter are the flat encoding itself —
--
--   combine / remQuot   product pairing and its inverse (row-major)
--   _↑ˡ_ / _↑ʳ_         sum injection into the low and high block
--   splitAt / join      the injection's inverse and its section
--
-- — together with `toℕ`, which reads a bounded position back as the raw
-- offset. `toℕ` is what the value-setoid grade is measured at: two values are
-- identified exactly when they name the same cell, which is why no law in this
-- tree ever needs a transport along a size equation.
------------------------------------------------------------------------------

module Gandr.Prelude.Fin where

open import Data.Fin.Base public
  using
    ( Fin
    ; toℕ
    ; combine
    ; remQuot
    ; splitAt
    ; join
    ; _↑ˡ_
    ; _↑ʳ_
    )
  renaming
    ( zero to fzero
    ; suc to fsuc
    )

open import Data.Fin.Properties public
  using
    ( toℕ-injective
    ; toℕ-combine
    ; toℕ-↑ˡ
    ; toℕ-↑ʳ
    ; combine-remQuot
    ; remQuot-combine
    ; combine-surjective
    ; splitAt-↑ˡ
    ; splitAt-↑ʳ
    ; splitAt-join
    ; join-splitAt
    )
