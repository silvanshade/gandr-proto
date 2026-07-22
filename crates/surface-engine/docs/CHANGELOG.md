# Changelog

The format is hand-maintained and grows only with real changes; it is not auto-generated.

## 2026-07-21 — Port the complete surface engine from wyrd (F3)

* `current`: Landed `gandr-surface-engine`, the complete CST-to-core front-end engine (rung F3 of `docs/research/front-end-port-staging.md` §9), ported from the wyrd `gandr-pipeline` crate.
* `current`: Modules — 27,997 source lines across 25 files covering the SynNode CST adapter; total lowering; datatype/codata/recursive lowering; origins, goals, diagnostics, attributes, and rendering; prelude, host, FFI, linking, and sessions; the one-shot run driver; and edit/footprint/checkpoint incrementality.
* `current`: Re-pointed seven predecessor project dependencies one-to-one (`gandr-desc` → `gandr-theory-levitation`, `gandr-grammar` → `gandr-surface-grammar`, `gandr-nominal` → `gandr-theory-nominal-automata`, `gandr-order-maintenance` → `gandr-theory-orders`, `gandr-parser` → `gandr-surface-parser`, `gandr-recursion` → `gandr-theory-recursion`, `gandr-syntax` → `gandr-surface-syntax`).
  The eighth, `gandr-core`, splits into `gandr-core-checker` for typing and the canonical host signatures and `gandr-core-sequent` for L-machine evaluation; the host-capability adapter adds `gandr-runtime-host` as the tenth reboot project edge.
  Every normal edge is directly used; `checkpoint` consumes `gandr-theory-orders`, and `lower::recursive` consumes `gandr-theory-recursion`.
* `current`: Session and linker evaluation run on the L machine.
  Typing and pre-evaluation terms retain checked annotations; runtime values erase them, pinned by linker plus non-empty record/list/operator session tests.
* `current`: Adapted statement recognition and diagnostics to the reboot grammar's `val` / `run` spellings.
  No compatibility alias or legacy surface spelling remains.
* `current`: Retargeted stage-0 elaboration to `gandr-theory-levitation`.
  Its `Attr`, `CellFace`, `check_desc`, and declaration-table API is source-compatible with the predecessor use sites, so the package rename is the complete delta.
* `current`: Kept the canonical host signatures beside the representation-independent seam in `gandr-core-checker::host`.
  `gandr-surface-engine::host` explicitly re-exports that API and adds source-level module metadata; the signature authority couples neither the engine nor the runtime, and the engine's only runtime edge is the host-capability adapter `run::run_source` composes.
* `current`: Gained `run::run_source`, the one-shot source-program driver (lower → link → prelude-check → host-run) relocated from `gandr-runtime-host`.
  The engine owns the language-level source entry; the runtime stays the host-capability adapter it composes.
* `current`: Preserved and directly tested the predecessor `Prelude` contract — unvalidated ordered bindings with later-name shadowing — behind a local wrapper that lends the L machine its binding slice.
* `current`: Replaced the dev-only grammar-contract-fixtures dependency with 19 engine-local fixtures; the acceptance harness discovers every `current/*.gandr` file rather than maintaining a second filename list.
  The predecessor helper was data-only at the engine's three use sites (two root paths and one JSON path manifest).
  All thirteen `current/` sources are byte-identical to the predecessor set; both readers target the local copies, and directory discovery replaces manifest parsing.
  All 299 predecessor tests remain semantically: 297 names are unchanged, while the linker and tuple-statement tests were renamed for the L-machine and `val` adaptations; two review regressions bring the total to 301.
  The executable corpus harness has since landed (`crates/surface-corpus`); promoting the four `surface/` witnesses and the `corpus/` model/pathological pair into its coverage map remains open, while the thirteen `current/` parser-state fixtures remain local.
* `current`: O3 scrub removed 108 direct wyrd tracker-ID occurrences and 126 numeric predecessor ADR citations from the engine and the core/runtime host files changed by F3, replacing them with current domain language and, where a citation carried the design, the inlined substance.
* `current`: Tests — the surface-engine all-features aggregator runs 323 tests; 33 runtime-host tests make the affected validation 356 tests total, with none ignored.
* `current`: Feature posture — `default = []`, `codecs` enables serde/JSON reports, `regex` enables the matching primitive in both core engines, and `full = ["codecs", "regex"]`.
