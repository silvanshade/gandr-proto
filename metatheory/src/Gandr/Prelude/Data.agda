{-# OPTIONS --safe --without-K --hidden-argument-puns #-}

------------------------------------------------------------------------------
-- Gandr.Prelude.Data — the facade for the ambient type formers.
--
-- Products, sums and the unit type enter only as PLUMBING: they carry the
-- decomposition of a flat value while a proof is in flight, and they present
-- the tree-shaped value universe (Gandr.Arena.Tree). The house rule that
-- purpose-built records beat raw sigmas still holds for gandr's own
-- structures — this module is what the rule makes an exception of.
------------------------------------------------------------------------------

module Gandr.Prelude.Data where

open import Data.Product.Base public
  using
    ( _×_
    ; _,_
    ; proj₁
    ; proj₂
    ; uncurry
    ; ∃
    ; ∃₂
    )

open import Data.Sum.Base public
  using
    ( _⊎_
    ; inj₁
    ; inj₂
    ; [_,_]′
    )

open import Data.Unit.Base public
  using
    ( ⊤
    ; tt
    )

open import Function.Base public
  using
    ( _∘_
    )
