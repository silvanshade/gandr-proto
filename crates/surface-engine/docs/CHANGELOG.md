# Changelog

The format is hand-maintained and grows only with real changes; it is not auto-generated.

## 2026-07-21 — Port the complete surface engine from wyrd (F3)

* `current`: Landed `gandr-surface-engine`, the complete CST-to-core front-end engine (rung F3 of `docs/research/front-end-port-staging.md` §9), ported from the wyrd `gandr-pipeline` crate.
* `current`: Modules — 27,920 source lines across 25 files covering the SynNode CST adapter; total lowering; datatype/codata/recursive lowering; origins, goals, diagnostics, attributes, and rendering; prelude, host, FFI, linking, and sessions; the one-shot run driver; and edit/footprint/checkpoint incrementality.
* `current`: Re-pointed seven predecessor project dependencies one-to-one (`gandr-desc` → `gandr-theory-levitation`, `gandr-grammar` → `gandr-surface-grammar`, `gandr-nominal` → `gandr-theory-nominal-automata`, `gandr-order-maintenance` → `gandr-theory-orders`, `gandr-parser` → `gandr-surface-parser`, `gandr-recursion` → `gandr-theory-recursion`, `gandr-syntax` → `gandr-surface-syntax`).
  The eighth, `gandr-core`, splits into `gandr-core-checker` for typing and `gandr-core-sequent` for L-machine evaluation; host authority adds `gandr-runtime-host` as the tenth reboot project edge.
  Every normal edge is directly used.
* `current`: Session and linker evaluation run on the L machine.
  Typing and pre-evaluation terms retain checked annotations; runtime values erase them, pinned by linker plus non-empty record/list/operator session tests.
* `current`: Adapted statement recognition and diagnostics to the reboot grammar's `val` / `run` spellings.
  No compatibility alias or legacy surface spelling remains.
* `current`: Retargeted stage-0 elaboration to `gandr-theory-levitation`.
  Its `Attr`, `CellFace`, `check_desc`, and declaration-table API is source-compatible with the predecessor use sites, so the package rename is the complete delta.
* `current`: Kept `gandr-runtime-host::sig` as the canonical host-signature authority.
  `gandr-surface-engine::host` re-exports those signatures and adds source-level module metadata without duplication or a runtime → engine dependency.
* `current`: Gained `run::run_source`, the one-shot source-program driver (lower → link → prelude-check → host-run) relocated from `gandr-runtime-host`.
  The engine owns the language-level source entry; the runtime stays the host-capability adapter it composes.
* `current`: Preserved the predecessor `Prelude` contract — unvalidated ordered bindings with later-name shadowing — behind a local wrapper that lends the L machine its binding slice.
* `current`: Replaced the dev-only grammar-contract-fixtures dependency with 19 engine-local fixtures exercised by named integration tests.
  The executable corpus harness has since landed (`crates/surface-corpus`); promoting the four `surface/` witnesses and the `corpus/` model/pathological pair into its coverage map remains open, while the thirteen `current/` parser-state fixtures remain local.
* `current`: O3 scrub removed 108 direct wyrd tracker-ID occurrences and 126 numeric predecessor ADR citations from the engine and the runtime-host files changed by F3, replacing them with current domain language and, where a citation carried the design, the inlined substance.
* `current`: Tests — all ported suites funnel through the `autotests = false` `tests/engine.rs` aggregator; 299 pass under nextest with all features and none are ignored.
* `current`: Feature posture — `default = []`, `codecs` enables serde/JSON reports, `regex` enables the matching primitive in both core engines, and `full = ["codecs", "regex"]`.
