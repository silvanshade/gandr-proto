{-# OPTIONS --safe --without-K --hidden-argument-puns #-}

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
import Gandr.Arena.Offset
import Gandr.Arena.Structure
import Gandr.Arena.Tree
import Gandr.Arena.Value
import Gandr.Prelude.Data
import Gandr.Prelude.Equality
import Gandr.Prelude.Fin
import Gandr.Prelude.Nat
