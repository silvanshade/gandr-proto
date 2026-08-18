# gandr-surface-parser

`gandr-surface-parser` is the front-end's parser lane: a resumable push-machine melder over the checked grammar, plus the obligation taxonomy it emits beside every tree.
It is **total** — any source slice yields a well-formed parse result carrying its obligations, never a panic and never a failure value — which is what lets the layers above treat a malformed program as an ordinary input.

## Current provision

- **The labeler.** A hand-rolled total scanner from source bytes to lexical classes, with no scanner-generator dependency.
  Multi-byte operators munch longest-first, so the circuit arrow grid stays disjoint from the shorter tiles each one extends, and a byte the grammar has no tile for flows through the unmolded path rather than raising a lexer error.
- **The melder.** A first-order push machine whose single primitive is `push`, total over its three rules.
  Incomparable precedences complete and reduce with grout; grout is comparable to everything and sits at the bottom, which is what guarantees the machine concludes.
  The stack is one slope of terraces, and emission is an append-only log replayed into the CST builder at commit — so rollback is log truncation.
- **Checkpoints.** `checkpoint` / `resume` are serializable and cheap because of that log, and `finalize` is a non-destructive query.
  Together they are the streaming surface the incremental pipeline drives.
- **The obligation taxonomy.** A closed severity ladder, an instance carrying class and span, and a per-class count array compared lexicographically from the highest severity down — the ordering that ranks one completion against another.

## Planned but absent

- The tree-sitter differential harnesses (token-stream parity, node-types drift), parked with the tree-sitter reference.
- Completion ranking beyond the obligation ordering itself.

## Using it

`parse(pbg, source)` is the batch entry point and is the fold of `push` followed by `commit`; reach for `MeldState` directly only when you want the streaming or checkpointing surface.

## Theoretical ideas relied on

Tile-based parsing, operator-precedence parsing generalized to error handling, syntactic obligations as a generalization of holes, and grout as the bottom of the precedence order.

## Primary references

- David Moon, Andrew Blinn, Thomas J. Porter and Cyrus Omar, _Syntactic Completions with Material Obligations_, 2025, `doi:10.1145/3763182` (arXiv:2508.16848) — the molder/melder split, the meld calculus, and the obligation-minimizing completion principle this crate implements.
- David Moon, _Syntactic Completions with Material Obligations (Thesis)_, PhD thesis, University of Michigan, 2025 — the extended development of the same theory.
