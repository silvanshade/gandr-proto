# gandr-theory-computads

The elaborator seam for gandr's cell-rewriting stack: where a levitation description becomes cells, where a circuit rule is instantiated against the cell store, and where the cell grammar is reified back into the frozen command arena.

This is the production-facing face of the stack.
The machinery it drives lives below it — `gandr-theory-cell-complexes` defines what a cell is, `gandr-theory-coherent-resolutions` fires and completes them, `gandr-theory-deep-inference` decides when two derivations are one, and `gandr-theory-decomposition-spaces` composes and transports the certificates.
A consumer that only wants descriptions turned into cells consumes this crate and nothing else: the vocabulary its signatures mention is re-exported here.

It reads the core sequent command language without modifying it.

## What it provides

- Elaboration of a whole levitation description into cells: surface rule faces become cells, and the declared operations' bridge arities decide which of them the single-continuation grammar admits.
- The circuit rule application site, where the shift-equivalence question about a two-redex rule body becomes well-posed.
  Each port is instantiated by a stored cell, the two positions are read from the block's own occurrence record rather than fabricated, and the guard decides the pair — with the composite replayed under both sequentializations before the identification is handed back.
- Reification of the frozen fragment into the L0 command arena.

The matcher is supplied at the instantiation site rather than depended upon, which is why no crate in this stack depends on `gandr-theory-circuit-algebras`.

## What is planned and absent

- Surface rule lowering and the end-to-end rewriting integration that would give the engines below a shipping consumer.
  In production this crate is consumed as an elaborator; the fusion engines beneath it are consumed by tests.

## Using it

Elaborate a description and read what it declined.

```rust
use gandr_theory_computads::elaborate_data_desc;

let elaborated = elaborate_data_desc(&desc);
let store = elaborated.store;
```

A declined face or operation is reported by index rather than dropped, so a caller can say which member of the description the grammar refused.

## Theoretical ideas it relies on

Computads and polygraphs; levitation-style datatype descriptions; circuit rules as schemas over rewrite-sorted ports; shift equivalence at the instantiation site; polarized call-by-push-value and the sequent command language.

## Primary references

- Dimitri Ara, Albert Burroni, Yves Guiraud, Philippe Malbos, François Métayer and Samuel Mimram, "Polygraphs: From Rewriting to Higher Categories", 2023.
  `doi:10.1017/9781009498968`, arXiv:2312.00429
