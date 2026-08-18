# gandr-theory-cell-complexes

The cell-shape substrate for gandr's rewriting stack: what a rewriting cell is, over what pattern language, and under which alphabet.

Every crate above this one is stated over the vocabulary defined here and introduces none of its own at this level.
The crate names no other workspace crate except `gandr-core-sequent`, and that only for the polarity tag the sequent alphabet carries.

## What it provides

- The command-pattern language: the cut grammar with pattern metavariables, the operation and return-side constructor frames, positions, and the erased node view the orders are taken over.
- Substitutions, with one-sided matching for cell application and two-sided unification for overlaps, both iterative.
- The reduction order used to orient a divergent pair: a hole-occurrence-guarded size comparison, with a lexicographic path order deciding the ties.
- The cell-alphabet trait every engine above quantifies over, and the sequent-kernel command-pattern alphabet as its one inhabitant in this tree.
- The cell itself — a left-hand side rewriting to a right-hand side, with orientation, provenance and derived metadata — and the structurally deduplicated, insertion-ordered cell store, generic over the alphabet.
- The linearity admission boundary: cell patterns are linear, so a rule that copies a hole is refused where cells are admitted rather than where patterns are constructed.
- The nominal wrapper vocabulary for every count, index, budget, depth and verdict that crosses a public signature in this crate or above it.

## What is planned and absent

- A second production alphabet.
  The trait exists for one, the tree has one, and the compile-visible pattern grammar stays pointed at every match site until a second arrives.

## Using it

Build a store and insert cells; every engine above takes the store and the alphabet from here.

```rust
use gandr_theory_cell_complexes::CellStore;
use gandr_theory_cell_complexes::frame_defining_cell;

let mut store = CellStore::new();
let id = store.insert(frame_defining_cell(&constructor));
```

## Theoretical ideas it relies on

Polygraphs and computads; cells as generating rewrites; critical-pair matching and unification; reduction orders and lexicographic path orders; linear left-hand sides; polarized call-by-push-value and the sequent command language.

## Primary references

- Dimitri Ara, Albert Burroni, Yves Guiraud, Philippe Malbos, François Métayer and Samuel Mimram, "Polygraphs: From Rewriting to Higher Categories", 2023.
  `doi:10.1017/9781009498968`, arXiv:2312.00429
