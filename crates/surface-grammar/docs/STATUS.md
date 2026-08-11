# Status

Crate scope: `crates/surface-grammar` (package `gandr-surface-grammar`).

Status vocabulary in this file is limited to `current`, `designed direction`, and `open decision`.

## current

- The crate is the front-end's checked precedence-bounded grammar (PBG) core plus the mold-driven syntax highlighter.
  Clients supply constant `Rule` values over a validated `PrecDag` (the precedence DAG re-exported by `gandr-theory-graphs` as `prec::*`), and `Pbg::build` performs the cross-rule checks; it deliberately carries no parser-specific declaration language.
- Ported at rung F1 of the surface front-end port (`docs/research/front-end-port-staging.md` §9) from the wyrd `gandr-grammar` crate.
  The recut renames the package to `gandr-surface-grammar` (staging call O1) and re-points the three workspace dependencies to their landed reboot homes: `gandr-graph` → `gandr-theory-graphs` (the `prec` DAG plus the finite walk-machine engine), `gandr-render-proto` → `gandr-surface-render-remote` (the highlighter's `HlRole`/`HlSpan` wire types, taken as plain in-process data with the wire codecs off), and `gandr-syntax` → `gandr-surface-syntax` (the flat-arena CST).
- Modules — `check` (the cross-rule PBG checks: unique tiles, operator form, assumption-3 adjacency), `model` (`Pbg`, `Rule`, `PrecTable`, `SurfaceForm`, `Tile`, the `Adaptation` surface, and the `MoldId`/fingerprint model), `mold` (the `MoldDef` regex-zipper table and reachable-context steps), `walk` (the generative walk-index front-end over the delivered walk engine — `walk_index`, `comparison_table`, `reachable_molds`, `seen_key_verdict`), `surface` (`built_in`/`built_in_prec_table`, `PBG_ONLY_KINDS`, `TREE_SITTER_NAMED_KINDS`, and the `term`/`type_shell`/`circuit` submodules that assemble the full surface grammar — `circuit` carries the ruled circuit block form: the `sign` block, its `sort`/`data`/`oper`/`rule` judgment members, the four-glyph arrow grid, the two-sided port lists, and the `node`/`feed` body statements), `highlight` (the normative mold highlighter: `highlight`, `role_of`), and `parity` (the standalone tree-sitter named-kind provenance inventory: `NamedKind`, `named_kind_parity`, `named_kind_realization`).
- The `parity` module is retained (it is always-compiled plain data, not gated by the omitted `parity` feature): it is the PBG's own inventory of named kinds and their realisation, a substrate table the deferred tree-sitter differential harness will consume — it has no tree-sitter or parser code dependency.
- O3 / ADR scrub: all retired-tracker provenance was dropped from the ported source — 75 wyrd bead-ID occurrences (19 distinct ids) and 19 wyrd `ADR-NN` citations (ADR-54, ADR-57, ADR-66, ADR-70, ADR-76) were inlined in current terms or dropped where the surrounding prose already carried the point.
  Zero survive into rustdoc; the crate documents clean under the merge-wall `cargo:doc-check` (`cargo doc --document-private-items -D warnings`).
  The wyrd design-wave labels (W4d / W4e / W5′) were preserved: they are not bead ids or ADR citations, and several appear inside load-bearing `AdaptationReason` data strings where a rewrite would be a behaviour change.
- Tests — the parser-free PBG, walk-index, comparison-table, reachability, and highlighter contracts plus the parser-driven surface-acceptance suite all funnel through the `autotests = false` `tests/lib.rs` aggregator (the `surface` test target).
  All 36 pass under nextest, including the dedicated instantiation-sort decoder and recursion-marker clean-parse contract, the pinned `built_in` grammar fingerprint (`0x483c_357b_641a_298e`), declared mold count (1724), per-label candidate inventory, and reachable multi-mold goldens.
  A `walk_index` criterion bench (`harness = false`) is present.
- Feature posture — `default = []`, `full = []`; the `parity` feature is omitted entirely (see below).

## designed direction

- The `parity` feature and its optional `gandr-tree-sitter` + `tree-sitter` edges do NOT land at F1; they return with the tree-sitter parity reference at rung F6 (`front-end-port-staging.md` §9, §5).
  The default dependency graph is therefore tree-sitter-free by construction, satisfying the F1 exit gate and the reboot `workflow-gates` forbidden-default-graph-package rule.
- Carrying no `parity` feature also resolves the wyrd HZ-8 / port-map H1 wart by construction: wyrd's `full = []` silently omitted `parity`, so `--features full` never ran the differential-parity lane; with `parity` gone entirely there is no feature for `full` to omit.
- The deliberate DEV-only dependency on the parser (the wyrd cycle break — the parser depends on the grammar, so the reverse edge can only ever be a dev edge) is carried at rung F2, when `surface-parser` lands: `gandr-surface-parser` sits in `[dev-dependencies]` only, and exists solely to drive the restored surface-acceptance `contracts` suite.
  - `tests/contracts.rs` `contracts` module (coupled to the parser through a module-level `use gandr_surface_parser::parse`), wired into the `tests/lib.rs` aggregator: `named_kind_coverage_is_semantic`, `built_in_precedence_bands_are_exact`, `right_associative_type_operator_chains_parse_cleanly`, `mixed_set_type_operators_require_parentheses`, `cyclic_named_precedence_spec_reports_closed_named_witness`, `built_in_adaptations_name_concrete_rules_without_relaxing_checks` — all six green at F2, unchanged from their parked form (the F1 scrub-adjusted golden data verified safe on restore).
- The tree-sitter differential-parity and drift-gate suites are parked and return at F6 with the tree-sitter reference and `packages/tree-sitter-gandr`:
  - `tests/node_types_gate.rs` (pins the generated `node-types.json` against `TREE_SITTER_NAMED_KINDS`; reads the committed tree-sitter package).
  - `tests/highlight_parity.rs` (E2 highlight-span differential against `gandr_tree_sitter::highlight`).
  - `tests/token_stream_parity.rs` (E1 token-stream differential against the tree-sitter leaf tokens).
- The mold highlighter's forward references to the future front-end crates are kept in doc comments as reboot-consistent names — `gandr_surface_parser` (F2), `surface-engine` (the F3 lowering hub, wyrd `gandr-pipeline`), `surface-corpus` (F4) — and to the not-yet-rehomed `packages/tree-sitter-gandr` reference (F6).

## open decision

- **O4 — the `gandr-grammar-contract-fixtures` fold is flagged, not forced, at F1.** The staging study recommends folding that crate into `surface-grammar`, treating the Rust/Node shared-manifest contract and the repo-relative path constants as the real work (staging call O4).
  Investigation of the wyrd source shows the fold is Node-coupled with no reboot home yet: the 567-line `fixtures/manifest.json` is a cross-language differential-parity contract whose `node_consumption` / `rust_consumption` / `parity` blocks (the `e1_declared_table`, the `PBG → grammar.json` projection, the E1/E2 relations) are defined against the tree-sitter grammar and the Rust parser — every consumer is deferred (`surface-parser` F2, the tree-sitter / `packages/tree-sitter-gandr` Node side F6, the pipeline dev-dep F3), and the manifest's own `root` / `manifest_path_from_repo_root` hardcode a repo-relative path whose re-decision is inseparable from where the tree-sitter reference re-homes.
  Per the charter's escape hatch (fold only if it does not drag Node-side machinery that has no reboot home), the fold is deferred to co-land with the tree-sitter return at F6, when its consumers and the shared-manifest path convention both exist.
  Landing the fixtures at F1 would add forward-looking parity contract data with zero F1 consumer.
  Whether to fold at F6 or keep a separate `gandr_test_*` fixtures crate remains the owner call.
- The wyrd crate carried a per-crate `docs/ADR.md` (the flat-CST / contract-fixtures rationale for the surrounding crates).
  The reboot homes ADRs at repo root (`docs/adr/`), so applicable rationale is distilled into this STATUS narrative rather than ported as a per-crate ADR file; whether any of it graduates into a reboot `docs/adr/` entry is deferred with the rest of the surface front-end's design-record reconciliation.
