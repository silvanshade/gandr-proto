# gandr-core-nbe

Normalization by evaluation for the gandr core: the engine that decides definitional equality.

A term is evaluated into a glued semantic domain, compared there, and read back into syntax when a term is wanted.
The crate names no typing judgement and no realization of one.
Callers come to it — the checker's subsumption relation decides its identity endpoints here, and a solver's certificate is re-checked by substituting and asking the same relation — which is what pins one equational theory across every consumer instead of letting each grow its own.

## Current provision

- `sem` — the glued value domain, its per-run arena, and the intrusive guard word that answers _distinct_ in constant time for a rigid, hole-free pair.
- `defs` — the per-scope definitional environment, with definition heights and transparency.
- `eval` — evaluation and the three force modes.
- `quote` — readback, its three options, and the generated-binder naming convention with its inverse.
- `conv` — the six-step definitional-equality pipeline: identity equality, cached-word guards, iterative structural comparison, lazy unfolding by height, smart unfolding gated on case progress, and three-state speculation.
- `intern` — the per-face syntax interner, a deduplicator that takes no table into the trusted base.
- The `Normalizer` itself: one arena, one definitional environment, one interner, and the fresh-variable counter readback draws from, with a fuel bound that stops unfolding rather than diverging.

Four anti-commitments are honoured as prohibitions rather than gaps: the engine never compares signatures by width or permutation, never memoizes across functor instantiations, makes no package eliminable by anything but its own elimination form, and takes no interning table into the trusted base.

## Planned but absent

- Conversion never runs an effect, a handler, or a control operator: those formers evaluate to neutrals, so the equality offered on them is congruence and nothing stronger.
- Five of the module layer's six holes remain neutrals; only structure projection reduces.
- The trace a shared duplication strategy would have to be certified by is designed and unbuilt, so nothing here is shared across a duplication decision yet.

## Using it

```toml
[dependencies]
gandr-core-nbe.workspace = true
```

```rust
use gandr_core_nbe::Normalizer;
```

A `Normalizer` owns its arena, so ids handed back to it must be its own; a watermark truncates the arena back to a recorded point.

## Theoretical ideas relied on

- **Normalization by evaluation** — deciding equality by evaluating into a semantic domain and quoting back, rather than by rewriting syntax.
  Call-by-push-value is the calculus being normalized, and its polarity is what makes the value and computation domains two domains here.
- **Glued evaluation** — a semantic value carrying both its unfolded face and the syntax it came from, so readback can return the source form and conversion can compare cheaply before unfolding.
- **Smart unfolding** — unfolding a definition only when doing so makes case-tree progress, so a comparison does not pay for definitions it will not need.
- **de Bruijn levels** — fresh variables drawn from a counter, which is what makes readback deterministic and independent of allocation order.
- **Hash consing** — content-addressed identity, used here per face as a deduplicator over syntax.

## Primary references

- Andreas Abel and Christian Sattler, "Normalization by Evaluation for Call-by-Push-Value and Polarized Lambda Calculus", in _Proceedings of the 21st International Symposium on Principles and Practice of Declarative Programming_ (PPDP '19), ACM, 2019, 1–12.
  DOI [10.1145/3354166.3354168](https://doi.org/10.1145/3354166.3354168).
- Paul Blain Levy, "Call-by-Push-Value: A Subsuming Paradigm", in _Typed Lambda Calculi and Applications_, Springer, 1999.
  DOI [10.1007/3-540-48959-2_17](https://doi.org/10.1007/3-540-48959-2_17), ISBN 978-3-540-48959-7.
- Andreas Abel, Andrea Vezzosi, and Theo Winterhalter, "Normalization by Evaluation for Sized Dependent Types", _Proceedings of the ACM on Programming Languages_ 1 (ICFP), 2017, 1–30.
  DOI [10.1145/3110277](https://doi.org/10.1145/3110277).
- Matthew Sirman, Meven Lennon-Bertrand, and Neel Krishnaswami, "Implementing a Type Theory with Observational Equality, Using Normalisation by Evaluation", in _30th International Conference on Types for Proofs and Programs_ (TYPES 2024), LIPIcs 336, Schloss Dagstuhl, 2025, 5:1–5:22.
  DOI [10.4230/LIPIcs.TYPES.2024.5](https://doi.org/10.4230/LIPIcs.TYPES.2024.5), ISBN 978-3-95977-376-8.
