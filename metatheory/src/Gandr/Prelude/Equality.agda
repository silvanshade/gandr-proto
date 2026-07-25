{-# OPTIONS --safe --without-K --hidden-argument-puns #-}

------------------------------------------------------------------------------
-- Gandr.Prelude.Equality — the propositional-equality facade.
--
-- House facade over agda-stdlib (docs/workflow/agda.md): a substantive module
-- imports `Gandr.Prelude.*` and never `Relation.*` or `Data.*` directly, so
-- the tree can re-choose its foundations without a sweep through every proof.
--
-- Only J-derived combinators appear here. `--without-K` is binding: nothing in
-- this facade may be strengthened to UIP or to definitional proof irrelevance,
-- and no downstream module may reach past it to obtain either.
------------------------------------------------------------------------------

module Gandr.Prelude.Equality where

open import Relation.Binary.PropositionalEquality public
  using
    ( _≡_
    ; refl
    ; sym
    ; trans
    ; cong
    ; cong₂
    ; subst
    )
