# Changelog

The format is hand-maintained and grows only with real changes; it is not auto-generated.

## 2026-07-21 — Port the complete surface engine from wyrd (F3)

* `current`: Landed `gandr-surface-engine`, the complete CST-to-core front-end engine (rung F3 of `docs/research/front-end-port-staging.md` §9), ported from the wyrd `gandr-pipeline` crate.
* `current`: Modules — 27,081 source lines across 23 files covering the SynNode CST adapter; total lowering; datatype/codata/recursive lowering; origins, goals, diagnostics, attributes, and rendering; prelude, host, FFI, linking, and sessions; and edit/footprint/checkpoint incrementality.
* `current`: Re-pointed the predecessor's eight normal project dependencies to `gandr-core-checker`, `gandr-theory-levitation`, `gandr-surface-grammar`, `gandr-theory-nominal-automata`, `gandr-theory-orders`, `gandr-surface-parser`, `gandr-theory-recursion`, and `gandr-surface-syntax`.
  Every edge is directly used.
* `current`: Added `gandr-core-sequent` as the reboot evaluation authority.
  Typing stays on `gandr-core-checker`; session and linker evaluation run on the L machine, with checked annotations erased from returned value payloads.
* `current`: Adapted the predecessor engine to the reboot grammar's `val` / `run` statement spellings and to the current levitation description API.
  No compatibility alias or legacy surface spelling was retained.
* `current`: Preserved the engine's host-signature table as the source-facing authority; `gandr-runtime-host` re-exports it rather than duplicating signatures or forming a dependency cycle.
* `current`: Replaced the dev-only grammar-contract-fixtures dependency with 19 engine-local fixtures exercised by named integration tests.
  Rung F4 owns their user-visible corpus consolidation/promotion.
* `current`: O3 scrub removed 108 direct wyrd tracker-ID occurrences and 126 numeric predecessor ADR citations from the crate's Rust/test documentation, replacing them with current domain language.
* `current`: Tests — all ported suites funnel through the `autotests = false` `tests/engine.rs` aggregator; 299 pass under nextest with all features and none are ignored.
* `current`: Feature posture — `default = []`, `codecs` enables serde/JSON reports, `regex` enables the matching primitive in both core engines, and `full = ["codecs", "regex"]`.
