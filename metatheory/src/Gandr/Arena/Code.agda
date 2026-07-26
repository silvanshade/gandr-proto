{-# OPTIONS --safe --without-K --hidden-argument-puns #-}

------------------------------------------------------------------------------
-- Gandr.Arena.Code — the code universe and its flat measure.
--
-- A code is a SHAPE. Its `size` is the length of the flat run of cells a value
-- of that code indexes into: a strict function of the shape, computed once and
-- never carried as structure. That is what makes the presentation flat, and it
-- is the modelled counterpart of gandr's arena layout — the surface form is
-- tree-shaped, the stored form is a run of cells with a computed extent.
--
-- Scope: the leaf-free fragment {𝟙, ⊗, ⊕}. Leaves add cell CONTENT, which the
-- structural generators never inspect, so admitting them would lengthen every
-- statement without changing a single coherence obligation. There is
-- deliberately no `𝟘`: an empty code would make `Val` uninhabited and the
-- distributor's inverse a partial function, and the arena has no use for it.
------------------------------------------------------------------------------

module Gandr.Arena.Code where

open import Data.Nat

infixr 3 _⊕_
infixr 4 _⊗_

-- The code universe: unit, product, sum.
data Code : Set where
  𝟙   : Code
  _⊗_ : Code → Code → Code
  _⊕_ : Code → Code → Code

-- The flat measure. A product multiplies extents (row-major layout), a sum
-- adds them (blocks laid end to end).
size : Code → ℕ
size 𝟙       = 1
size (c ⊗ d) = size c * size d
size (c ⊕ d) = size c + size d
