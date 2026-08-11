# Status

Crate scope: `crates/surface-engine` (package `gandr-surface-engine`).

Status vocabulary in this file is limited to `current`, `designed direction`, and `open decision`.

## current

- The crate is the reboot's complete CST-to-core surface engine: total lowering, origin tracking, structured diagnostics and goals, prelude/host/attribute tables, edit-action reconstruction, the item seam onto `gandr-core-incremental`, linking, stateful REPL sessions, and the one-shot source-program driver.
  The item-granular checkpoint engine this crate once carried a second copy of lives in `gandr-core-incremental`; what stays here is the front end that feeds it.
- Ported at rung F3 of the surface front-end port (`docs/research/front-end-port-staging.md` §9) from the wyrd `gandr-pipeline` crate.
  The recut renames the package to `gandr-surface-engine` while preserving its public module shape and source-facing behavior; required reboot deltas are recorded below.
- Modules — 27,997 source lines across 25 source files:
  - `synnode` adapts the form-name-free melder CST to the lowerer's named-node view.
  - `lower` and its `data`, `codata`, `recursive`, `recursion_surface`, `types`, and `node_kinds` submodules implement total source-to-core lowering.
  - `origin`, `goals`, `diag`, `attributes`, and `render` project core results back to structured, source-ranged front-end data.
  - `prelude`, `host`, `ffi`, `link`, and `session` connect the lowered core to typed native bindings, host effects, whole-file linking, and evaluation.
  - `run` drives a one-shot source program through lowering, linking, prelude checking, and the host seam.
  - `edit` reconstructs localized edit actions between two lowerings, and `item_source` crosses a lowering to `gandr-core-incremental`'s parser-agnostic item seam, which is where dependency-validated incremental re-typing runs.
  - `boundary` owns the crate's typed scalar and string boundary wrappers.
  - `cst_read` reads a committed CST as a flat tile run — the `Reader` / `Cursor` and the depth-aware member split that `desc_elab` and `circuit` both walk declarations with.
  - `circuit` is the ruled circuit block form's surface check: it confirms every arrow against the kind of the thing it belongs to (a declaration's from its kind keyword, a body line's from the applied head's) and declines the reserved reversible glyph `<->`.
    It reads arrows and names only; the port/name fold, the back-edge sweep, and lowering are elsewhere or later.
- Dependency re-point:
  - Seven predecessor project edges mapped one-to-one at the port: `gandr-desc` → `gandr-theory-levitation`, `gandr-grammar` → `gandr-surface-grammar`, `gandr-nominal` → `gandr-theory-nominal-automata`, `gandr-order-maintenance` → `gandr-theory-orders`, `gandr-parser` → `gandr-surface-parser`, `gandr-recursion` → `gandr-theory-recursion`, and `gandr-syntax` → `gandr-surface-syntax`.
    Six of the seven are still direct edges; the order-maintenance one is not, having left with the checkpoint engine.
  - The predecessor's eighth edge, `gandr-core`, splits by authority: `gandr-core-checker` owns syntax, typing, diagnostics, marks, native primitive definitions, and the canonical host signatures beside the host seam; `gandr-core-sequent` owns L-machine evaluation and consumes the checker-owned `Eval` outcome.
  - `gandr-surface-engine::host` explicitly re-exports the signature API and adds only source-level metadata, so the signature authority couples neither the engine nor the runtime.
    The host-capability adapter adds `gandr-runtime-host` as the tenth reboot project edge: `run::run_source` composes its seam.
  - Every normal dependency has a direct source-level use, including the `Machine` / `Step` / `run` trio in `lower::recursive`.
    The `gandr-theory-orders` edge left with the checkpoint engine: this crate now reaches order maintenance only through `gandr-core-incremental`, which owns it.
    The predecessor's dev-only `gandr-grammar-contract-fixtures` edge was dropped: the engine tests discover and exercise every `.gandr` file under `tests/fixtures/current`.
    The helper was data-only at the three engine use sites: two root-path constants and one JSON source-path manifest.
    All thirteen `current/` source files are byte-identical to the predecessor set; the two path reads now target the local copies, and directory discovery replaces the manifest parser.
    All 299 predecessor tests remain semantically: 297 names are unchanged, while the linker and tuple-statement tests were renamed for the L-machine and `val` adaptations; two review regressions bring the total to 301.
- Reboot API adaptations are localized and behavior-pinned:
  - block statements and diagnostic contexts use the reboot grammar's `val PAT = E;` and `run PAT <- E;` spellings while retaining the same core `Let` / `Bind` lowering;
  - the levitation API was source-compatible after the package move to `gandr-theory-levitation`: `Attr`, `CellFace`, `check_desc`, and the declaration-table calls required no symbol- or behavior-level rewrite;
  - the L machine intentionally erases checked `Value::Annot` wrappers from runtime payloads.
    The linker test still pins exactly one annotation before evaluation; non-empty record/list/operator session tests pin the erased runtime values;
  - the engine-owned `Prelude` wrapper preserves the predecessor type's unvalidated ordered bindings and later-name shadowing, pinned through the L-machine focus, then lends that exact slice to `run_comp_with_prelude`;
  - the one-shot source driver `run::run_source` lives here — relocated from `gandr-runtime-host` — composing the engine's lowering, linking, and prelude checking with the runtime's host seam, so the runtime remains a source-free capability adapter.
    Its path-shaped face `run::run_source_file` lands beside it: the same composition over a file's bytes, reporting the path in `run::RunFileError::Read` and passing a source-level failure through unchanged, and it is the seam the `gandr <file>` script runner stands on;
  - `lower_source_total` is exercised directly by the acceptance, totality, diagnostics, attributes, and property suites.
    Stateful sessions call its seeded form and use the same `prelude_ctx` for typing.
- O3 scrub: 108 direct wyrd tracker-ID occurrences and 126 numeric predecessor ADR citations were removed from the engine and the core/runtime host files changed by F3, replaced by current domain terms and, where a citation carried the design, the inlined substance; predecessor package names survive only where status/changelog records port provenance.
- Tests — the surface-engine aggregator runs 379 tests under nextest with all features (2026-08-08; 323 at the F3 landing, grown by the circuit-surface and script-face work since); the 33 runtime-host tests make the affected authority/evaluation validation 412 tests total.
  None are parked or ignored.
- Feature posture: `default = []`; `codecs` enables the serde/JSON report surface; `regex` enables the matching primitive in both reboot core engines; `full = ["codecs", "regex"]`.

## designed direction

- Rung B3.1 owns structural deepening after the faithful port.
  F3 deliberately preserves the predecessor's module/API topology so behavior and parity remain reviewable before extracting deeper module boundaries.
- Rung F4 landed `crates/surface-corpus` and its executable harness.
  It must still promote the four `tests/fixtures/surface/*.gandr` witnesses and the model/pathological pair under `tests/fixtures/corpus/` into the corpus coverage map as required by `docs/workflow/corpus.md`.
  The thirteen `tests/fixtures/current/*.gandr` files are parser/CST/incrementality fixtures and remain engine-local unless F4 deliberately turns one into a user program.

## open decision

- The F3 crate has no unresolved authority split: core-checker owns typing plus the representation-independent host seam and canonical signatures, core-sequent owns evaluation, runtime-host owns native dispatch, and surface-engine owns the source-level host-module metadata and the one-shot source driver.
  The enumerated fixture promotion belongs to the landed corpus harness's coverage map.
