# gandr-theory-circuit-algebras

## Intent

`gandr-theory-circuit-algebras` provides the circuit-algebra machinery at the cell-alphabet seam.
It owns interface bookkeeping, embedding-based matching with convexity checks, and diagram normal forms.
It does not own the port-bijection representation, cell engines, overlap enumeration, completion, tracelets, or rewrite execution.

## Current provision

- `interface` validates monogamous acyclic wirings, records discrete boundaries, and provides span-level seam data plus the sequent-alphabet spine reading.
- `matching` searches every embedding within a budget and reports admitted embeddings or convexity refusals.
  A generator's label carries the role its name is worn in — a value constructor, an operation frame, a return-side frame, a terminal, or a fired rewrite — because two boxes of one spelling in different roles are two different boxes.
- `normal_form` canonicalizes diagrams with boundary-anchored linearization, checkable relabelling witnesses, and same-diagram decisions.

The crate is internal machinery, and its consumer supplies the matcher at an engine-instantiation seam rather than introducing a dependency cycle.
`gandr-surface-engine` is that seam: it reads a declared circuit rule body as a `Wiring` and matches one body into another, and the description route runs it over the circuit rules a source declares.
The placement is forced rather than chosen — this crate depends on `gandr-theory-computads`, so the reverse library edge is a cycle the resolver rejects, and the consumer has to sit above both.

## Planned but absent

The crate does not provide a user-facing circuit-term former, rewrite execution, boundary-complement construction, or storage interning.
Those require the circuit-algebra surface and consumer rungs that are outside this crate.
Completion does not consume the matcher yet: `gandr-theory-computads` still enumerates overlaps through its one-sided command-pattern matcher, which is right for command patterns and is not what a circuit pattern needs.

## Usage

The public modules can be used directly by an internal Rust consumer:

```rust
use gandr_theory_circuit_algebras::{interface, matching, normal_form};
```

Construct a validated `interface::Wiring`, call `matching::embeddings` or `matching::embeddings_by_sweep`, and use `normal_form::same_diagram` for representation-level equality.
No command-line or user-language syntax reaches this crate.

## Theoretical ideas

Circuit algebras, monogamous acyclic hypergraphs, convex double-pushout matching, boundary-anchored canonicalization, cospan-isomorphism equality, and checkable relabelling witnesses.

## Primary references

- Filippo Bonchi, Fabio Gadducci, Aleks Kissinger, Paweł Sobociński, and Fabio Zanasi, _String Diagram Rewrite Theory II: Rewriting with Symmetric Monoidal Structure_ (2022), arXiv:2104.14686, DOI:10.48550/arXiv.2104.14686.
- Philip Hackney, Marcy Robertson, and Donald Yau, _On Factorizations of Graphical Maps_ (2018), DOI:10.4310/HHA.2018.v20.n2.a11, arXiv:1705.08546v2.
