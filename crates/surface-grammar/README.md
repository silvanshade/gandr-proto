# surface-grammar

`gandr-surface-grammar` owns the checked grammar the surface front-end parses over: a precedence-bounded grammar (PBG) whose rules are constant data, validated once at build time, and whose every tile occurrence gets a stable identity the parser and the editor faces both address.
It carries no parser: clients supply `Rule` values over a validated precedence DAG, and `Pbg::build` performs the cross-rule checks.

## What it currently provides

- **The checked model.** `Pbg::build` runs three build-time gates — no concatenation exposes two adjacent sort holes, every tile occurrence interns to a distinct identity, and the precedence relation is conflict-free — so a colliding rule fails the build instead of parsing ambiguously.
- **The mold table.** One entry per tile occurrence, keyed by an interned regex-zipper context, with precomputed precedence bounds and zipper steps.
  Identities are assigned deterministically in canonical table order and fold into a grammar fingerprint that every produced CST records.
- **The generative walk index.** The precedence relation is carried on one representative tile per form group and read off the DAG, which is what makes the index tractable at this grammar's scale.
- **The built-in gandr surface.** Term, type/shell, and circuit rule assemblies over the five sorts, the named precedence bands, and the named-kind registries — including the set of forms the committed tree-sitter grammar does not produce, which is the parity exemption.
- **The mold-driven highlighter.** A pure classification from mold to highlight role over a committed CST, needing no query engine and strictly better informed than a flat capture query, because a mold carries its grammar zipper.
- **The `extend` seam.** Folds declared operators into a fresh grammar with base identities preserved.
  Only its own tests call it.

## Planned but not implemented

- Wiring `extend` to a real module or session path: declaration collection, the numeric fixity level, the non-associative class, and propagation of the extended grammar through parsing, lowering, and display.
- The tree-sitter differential and highlight-parity harnesses, deferred with the tree-sitter reference; the parity inventory is the data they will consume.

## Using it

`built_in()` returns the checked gandr grammar; `Pbg::build` takes your own rules and precedence table.
Both are pure and allocation-bounded, and both fail with a typed error rather than panicking.

## Theoretical ideas it relies on

Precedence-bounded grammars and precedence graphs, mixfix operator parsing, and the regex-zipper presentation of a rule's parse context.

## Primary resources

- Nils Anders Danielsson and Ulf Norell, _Parsing Mixfix Operators_, 2011, `doi:10.1007/978-3-642-24452-0_5` — the precedence-graph presentation the precedence DAG follows.
- David Moon, Andrew Blinn, Thomas J. Porter and Cyrus Omar, _Syntactic Completions with Material Obligations_, 2025, `doi:10.1145/3763182` (arXiv:2508.16848) — the tile-based parsing theory this grammar's molds, precedence comparisons, and obligation surface are built against.
- Matthew Flatt, _Binding as Sets of Scopes_, 2016, `doi:10.1145/2837614.2837620` — the hygiene reference the reserved user-operator surface is designed against.
