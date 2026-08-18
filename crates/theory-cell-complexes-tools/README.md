# gandr-theory-cell-complexes-tools

The second inhabitant of the cell-alphabet trait, and the adversarial variants built over it.

The workspace ships exactly one production alphabet, so every measured property of the engines above the substrate would otherwise be a property of that one alphabet rather than of the trait.
This crate is what keeps the alphabet-generic claims honest.

It is **test-facing**: it is a dev-dependency of the crates whose engines quantify over the trait, and no production crate links it.
It depends on `gandr-theory-cell-complexes` and nothing else, so it introduces no dependency cycle in either direction.

## Module map

| module        | what it holds                                                                                                                                      |
| ------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| `toy`         | a single-sorted first-order term language (`Zero` / `Succ` / `Add` with metavariables) inhabiting the trait from outside the crates that define it |
| `adversarial` | five wrappers that delegate to `toy` in everything except the one law each is built to break                                                       |

## What it provides

- An alphabet inhabited from outside the substrate crate — exactly the path a future shape-layer or directed rule-layer alphabet takes.
- The only alphabet in the tree whose terms nest commands, so it is where anything about two applications in one term can be exercised at all.
- Five adversarial wrappers: one that calls every position pair incomparable, one that withholds the convexity discharge, one whose splice disturbs a sibling it was not asked about, one that gives two distinct cells the same address, and the delegating wrapper the other four are expressed through.

## What is planned and absent

- Nothing.
  The crate grows when a new law needs an adversary, not on a schedule.

## Using it

Take it as a dev-dependency and instantiate an engine at the toy alphabet, or at a wrapper that breaks the law under test.

```rust
use gandr_theory_cell_complexes::CellStore;
use gandr_theory_cell_complexes_tools::adversarial::{IncomparablePositions, Lying};
use gandr_theory_cell_complexes_tools::toy::ToyAlphabet;

let honest: CellStore<ToyAlphabet> = CellStore::new();
let lying: CellStore<Lying<IncomparablePositions>> = CellStore::new();
```

## Theoretical ideas it relies on

Cell alphabets as a trait with multiple inhabitants; first-order term rewriting with metavariables; position orders and convexity discharge; content-addressed cell identity.

## Primary references

- Dimitri Ara, Albert Burroni, Yves Guiraud, Philippe Malbos, François Métayer and Samuel Mimram, "Polygraphs: From Rewriting to Higher Categories", 2023.
  `doi:10.1017/9781009498968`, arXiv:2312.00429
