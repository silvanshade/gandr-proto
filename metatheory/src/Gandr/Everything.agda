{-# OPTIONS --safe --guardedness --without-K --hidden-argument-puns #-}

------------------------------------------------------------------------------
-- Gandr.Everything — the strict gate root (docs/workflow/agda.md).
--
-- Imports every hole-free module of the metatheory. Agda refuses to import a
-- module with open interaction holes, so a stray meta anywhere in the imported
-- graph fails this root; declared holey LEAVES are gated separately with
-- tolerated `UnsolvedInteractionMetas` and are never imported here.
------------------------------------------------------------------------------

module Gandr.Everything where

import Gandr.Arena.Code
import Gandr.Arena.Coherence
import Gandr.Arena.Directed
import Gandr.Arena.Offset
import Gandr.Arena.Structure
import Gandr.Arena.Tree
import Gandr.Arena.Value
import Gandr.Arity.Path
import Gandr.Arity.Universe
import Gandr.Category
import Gandr.Category.Functor
import Gandr.Category.Instances
import Gandr.Category.Reasoning
import Gandr.Graph
import Gandr.Profunctor
import Gandr.Profunctor.Yoneda
import Gandr.Rigid
import Gandr.Setoid
import Gandr.Shape.Decidable
import Gandr.Shape.Graft
import Gandr.Shape.Graph
import Gandr.Shape.Structure
