# Changelog

The format is hand-maintained and grows only with real changes; it is not auto-generated.

## 2026-07-21 — Port the checked PBG grammar core + mold highlighter from wyrd (F1)

* `current`: Landed `gandr-surface-grammar`, the front-end's checked precedence-bounded grammar (PBG) core plus the mold-driven highlighter (rung F1 of `docs/research/front-end-port-staging.md` §9), ported from the wyrd `gandr-grammar` crate.
* `current`: The port renames the package to `gandr-surface-grammar` (staging call O1) and re-points the three workspace dependencies to their landed reboot homes — `gandr-graph` → `gandr-theory-graphs` (the `prec` precedence DAG and the finite walk-machine engine), `gandr-render-proto` → `gandr-surface-render-remote` (the highlighter's `HlRole`/`HlSpan` types, codecs off), and `gandr-syntax` → `gandr-surface-syntax` (the flat-arena CST).
* `current`: Modules — `check`, `model`, `mold`, `walk`, `surface` (with `surface/term` + `surface/type_shell`), `highlight`, and `parity`; the checked PBG construction, the regex-zipper mold table, the generative walk index, the `built_in` surface assembly, the normative mold highlighter, and the tree-sitter named-kind provenance inventory port with their types, tables, `built_in` grammar, and hashing intact.
* `current`: O3 / ADR scrub — 75 wyrd bead-ID occurrences (19 distinct ids) and 19 wyrd `ADR-NN` citations (ADR-54/57/66/70/76) were dropped or inlined in current terms; zero survive into rustdoc, and the crate is clean under `cargo:doc-check`.
  The wyrd design-wave labels (W4d/W4e/W5′) were preserved verbatim, including inside load-bearing `AdaptationReason` data strings where a rewrite would change behaviour.
* `current`: The `parity` feature and its optional `gandr-tree-sitter` + `tree-sitter` edges are omitted entirely — tree-sitter returns at rung F6 — so the default dependency graph is tree-sitter-free by construction.
  This also resolves the wyrd HZ-8 / port-map H1 wart by construction: with no `parity` feature, `full = []` no longer silently omits a differential-parity lane.
* `current`: The deliberate DEV-only parser dependency (the wyrd parser↔grammar cycle break) is not carried; `surface-parser` lands at F2.
  The parser-coupled `tests/surface.rs` `contracts` suite is parked to F2 (which re-adds the dev-dependency), and the tree-sitter drift-gate / differential-parity suites (`tests/node_types_gate.rs`, `tests/highlight_parity.rs`, `tests/token_stream_parity.rs`) are parked to F6 — all listed by name in `docs/STATUS.md`.
* `current`: Tests carried across — the parser-free grammar unit goldens `tests/pbg.rs` and `tests/walk.rs`, funnelled through the `autotests = false` aggregator `tests/surface.rs`, plus the `highlight` module unit tests; 28 pass under nextest, including the `built_in` fingerprint, mold-count, candidate-inventory, and reachable-mold goldens.
  Feature posture — `default = []`, `full = []`; a `walk_index` criterion bench is present.
* `current`: The wyrd `gandr-grammar-contract-fixtures` fold (staging call O4) is flagged and deferred to F6 rather than forced — its manifest is a cross-language Rust/tree-sitter differential-parity contract whose consumers are all deferred; see `docs/STATUS.md` for the rationale.
