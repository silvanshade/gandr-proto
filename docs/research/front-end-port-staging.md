# Surface front-end port — staging study (gandr-wvd.24)

> Staging study for the firewalled surface front-end port: the B2-staging precedent (recon before rungs), owner-decided 2026-07-21 via `gandr-lsv` option (a) — the front-end ports into the reboot BEFORE B3.1.
> Deliverable: a design/staging record the coordinator mints rungs from.
> NO code, NO crate ports land in this lane.
> Port-plan authority is `docs/research/crate-port-map.md` (per-crate inventory §3, topology §2, tiers §8, the §7d tree-sitter demotion, the §7e formatting-crates finding).
> Reboot ground truth is the B3.0 stage record (`gandr-wvd.3.2`) plus the in-tree crate schema; the wyrd source tree is read ONLY via `git -C …/wyrd show failed-refactor:<path>` (shared, on `main`, never mutated).
> Prose is sentence-per-line (treefmt discipline); citations are `path §sec` in-repo and `wyrd@failed-refactor:crates/<crate>/<path>[:line]` source-side.
>
> **Naming amendment (owner, 2026-07-21):** the `PROMPT.md` rename table is the crate-naming authority (codified: `docs/workflow/rust.md` §conventions + `crate-port-map.md` §12 "Naming authority") — the render-proto leaf ports as **`surface-render-remote`** (the remote/wire face beside the future `surface-render`, the planned presentation printer) and the pipeline hub ports as **`surface-engine`**.
> Read this study's `surface-render-proto` / `surface-pipeline` occurrences accordingly; staging calls O1/O2 stand with those names substituted.
>
> **Status amendment (2026-08-08), and it reaches backwards as well as forwards:** §0 is the recommended cut as it stood on 2026-07-21, and two of its verdicts have since been refuted by what was built.
> `gandr-shell`/`runtime-host` is **not** "partially ported" and needed **no** adapt — the host-signature ruling at F3 put both source-runner faces in the engine and left the runtime untouched, so §0's row A prescribes something that is now a normal-dependency cycle (§4, §7, §10 HZ-6).
> The cut's shape — six new crates plus one un-stub — is otherwise as landed.
> Every §0 status claim reads against §4's table, which is the as-built record; §0 is retained as the recommendation of record, not as a description of the tree.

---

## 0. Executive summary — the recommended minimal cut

The single load-bearing finding: **most of the front-end's dependency substrate is already in the reboot.** The reboot did not merely rename `gandr-core`/`gandr-sequent`; it ported the whole substrate spine into the `core-*`/`kernel-*`/`theory-*` namespaces and re-homed the tooling crates as reboot-native (§4).
Of the wyrd 14-crate minimal buildable set for the six candidates (`docs/research/crate-port-map.md` §2; confirmed against `wyrd@failed-refactor:crates/*/Cargo.toml`), **eight are already ported**, **one (`gandr-shell`) is partially ported** (`runtime-host`, minus its parked source-runner faces), and only **five surface crates plus one leaf are genuinely absent.**

**The minimal cut that unblocks B3.1 — six new crates, one adapt, one un-stub:**

| #   | Port target (wyrd)   | Recommended reboot home                             | Size | Role in the cut                                                                                                   |
| --- | -------------------- | --------------------------------------------------- | ---- | ----------------------------------------------------------------------------------------------------------------- |
| 1   | `gandr-render-proto` | `crates/surface-render-proto` (or feature-gate out) | S    | leaf wire types `gandr-grammar` needs (owner call O2)                                                             |
| 2   | `gandr-syntax`       | `crates/surface-syntax`                             | S    | flat-arena CST + incremental diff (verbatim-clean leaf)                                                           |
| 3   | `gandr-grammar`      | `crates/surface-grammar`                            | M    | checked PBG over the precedence DAG + mold highlighter (parity deferred)                                          |
| 4   | `gandr-parser`       | `crates/surface-parser`                             | M    | resumable melder push-machine + obligation taxonomy                                                               |
| 5   | `gandr-pipeline`     | `crates/surface-pipeline`                           | L    | **the lowering hub** — `lower_source_total`, prelude/host/attributes tables, session; **this rung unblocks B3.1** |
| 6   | `gandr-corpus`       | `crates/surface-corpus`                             | M    | `.gandr` source trees + end-to-end harness; drives corpus regeneration                                            |
| A   | `gandr-shell`        | `crates/runtime-host` (exists)                      | S    | **adapt** — re-wire `run_source`/`run_source_file` onto the landed pipeline                                       |
| B   | `gandr` (driver)     | `crates/surface-driver` (stub)                      | M    | **un-stub** the script-runner face; rewrite the stale wyrd dep names/paths                                        |

**Deferred, with one-line reasons** (§5): `gandr-tui` (8-crate UI stack, REPL/editor face — not needed to parse/lower), `gandr-lsp` (server face), `gandr-ffi` (native/unsafe; corpus `ffi` feature only), `gandr-tree-sitter` + `packages/tree-sitter-gandr` (differential-parity reference; `parity` feature only — `mise grammar:test` stays parked), `gandr-data` (value codecs; shell/self-hosting lane), `gandr-doc`/`gandr-fmt`/`gandr-pretty` (spec-planned, no code yet — a graduation, not a port; §7e/H6), `gandr-grammar-contract-fixtures` (dev-dep only; fold-vs-keep is owner call O4).
`wyrd-rust-gates`/`wyrd-dylint` are **not ports at all** — the reboot already ships reboot-native `workflow-gates`/`workflow-dylint` (§4), dissolving port-map H8 for the reboot.

**The corpus is the exit gate, not a payload.** The 29 model + 27 pathological pre-lowered `.sexp`/`.outcome` fixtures under `crates/core-sequent/tests/fixtures/corpus/` are a frozen set hashed three ways (`corpus_fixtures_b3sum`, the `kernel_partition.manifest`, the per-item `kernel_export_gate` records).
Regeneration must reproduce them **byte-identically through `lower_source_total`** — the `corpus_fixtures.rs` reader retires exactly here per its own forward pointer (`crates/core-sequent/tests/corpus_fixtures.rs:20-23`) — and must **not** change the item set (no partition/export re-bless), which is the charter's hard constraint (§6, O5, O6).

---

## 1. Method and scope

Read-only throughout.
Reboot side: the worktree `crates/` schema, `mise.toml`, `.commitlintrc.mts`, the pre-lowered fixture trees, the `core-sequent` test harnesses, and the B3.0 stage record (`gandr-wvd.3.2`, read via `bd show` against the fresh worktree clone).
Source side: the six candidate manifests and their transitive workspace closure, the prelude/host/attributes modules, and the lowering entry, via `git -C …/wyrd show failed-refactor:<path>` only.
The port-plan authority (`docs/research/crate-port-map.md`) supplies the per-crate inventory; this study adds the reboot-side reconciliation the port-map (written against the empty greenfield repo, its §1) could not yet carry.

---

## 2. Ground truth — reboot side (what the port plugs into)

What the reboot already carries, verified file-level (and consistent with the B3.0 record `gandr-wvd.3.2`):

- **`crates/surface-driver`** is a stub: `src/main.rs` is an empty `fn main()`, and `Cargo.toml`'s real dependency surface is commented out — `gandr-core`, `gandr-grammar`, `gandr-parser`, `gandr-pipeline`, `gandr-shell`, `gandr-syntax`, `gandr-tui`, plus `reedline`/`nu-ansi-term`/`color-eyre` — "parked until those crates land."
  **These commented deps still spell the wyrd names and paths** (`gandr-core = { path = "../gandr-core" }`), which do not exist in the reboot's category-directory scheme — a stale-from-seed manifest, not a ready-to-uncomment one (§7's `surface-driver` row; hazard HZ-3).
  **Landed (F5, 2026-08-08):** the script-runner face carries its three real edges, and the commented dependency entries were deleted in favour of prose naming what each deferred face waits on — reboot crates for the REPL face, and an honest "no reboot crate at all" for `tui` and the four subcommand slots.
  Nothing uncommentable remains.
- **`mise.toml` `grammar:test` is commented out** (then at `mise.toml:83-89`; the task surface has since moved to `.config/mise/tasks/`, and the parked note now lives in `mise-tasks-gates.toml`), with the note that the tree-sitter grammar package "is not in the repo yet (it returns with the surface-grammar port); restore this task — and its `gate:merge` entry — with it."
  The `agda:deps` task ran `cargo run -q -p gandr -- scripts/agda-deps.gandr` — inert at the time of this study because the driver was a stub and the script did not exist (§7's `surface-driver` row ties the driver un-stub to re-enabling Agda vendoring).
  **The intervening state matters for anyone reading a later tree:** a 2026-07-24 repair replaced that invocation with an inline `git clone` body precisely because it named a script that was never written, so the task was live but no longer ran gandr.
  F5 restores the intended shape — the task announces, then runs `scripts/agda-deps.gandr` through the driver, and the script's `proc.exit` carries the shell's status out as the task's status.
- **`fuzz/` is workspace-excluded** (`Cargo.toml:30`, `exclude = ["fuzz"]`) and its manifest references unported crates by stale wyrd path — `gandr-core = { path = "../crates/gandr-core" }`, plus `gandr-pipeline`/`gandr-grammar`/`gandr-parser`/`gandr-syntax`/`gandr-render-proto`/`gandr-tree-sitter`/`wyrd-rust-gates`.
  Its `lower`/`parse`/`check` targets consume `gandr_pipeline::lower::lower_source_total` and `gandr_pipeline::prelude_ctx`; its `parity`/`gates` targets consume the tree-sitter reference and the wyrd gate crate.
  Restoration is downstream of the port and needs path/name rewrites (§8).
- **`crates/core-sequent/tests/corpus_fixtures.rs`** is the one-shot pre-lowered fixture reader (1626 lines) that documents its own retirement: "when the surface corpus itself ports (the firewalled front-end), the two sweeps re-point again at live lowering (or regenerated fixtures) and this reader retires" (`:20-23`).
  It also exposes `corpus_fixtures_b3sum(trees)` (`:110`) — the fold-many-fixtures-into-one-manifest guard shared by the partition sweep and the export exit gate.
- **The frozen fixture set:** 29 model + 27 pathological `.sexp`/`.outcome`, each carrying a provenance header (`; source: model/…/x.gandr`, `; b3sum: …`, `; lowering: gandr_pipeline::lower::lower_source_total`, `; items: N`).
  **The header's `b3sum` line is ruled a historical record, not a live integrity surface** (owner, 2026-08-02): the anchors have no generator, staleness against the current source is expected and does not gate anything, and refreshing them by hand is declined — if they are ever wanted live, the fix is a bless generator, filed as a task, never a hand edit.
  Layout: numbered wyrd-corpus files at the `model/` root (`05-pairs`, `06-sums-and-case`, `08-list-combinators`, `14-agda-deps-walkthrough`, `15-record-update`, `22-ffi-effect-and-capability`, `25-host-modules`, `26-env-guard-exit`, `28-regex-and-path-builtins`, `29-modules`, …) plus subdir-organized fixtures under `model/{attributes,codata,data,desc,identity,sequent}/` and `pathological/{attributes,codata,data,identity}/` and the `pathological/` root (`module-forward-member-reference`, `host-module-uncalled-selection`, `shell-host-escape-non-string`, `unfocused-stuck`, …).
  These currently **stand in for** the surface corpus.
- **The two kernel gates the fixture set feeds** (both re-bless on set change):
  - `kernel_corpus_partition.rs` — classifies the corpus per item (S1-eligible vs excluded), pins the result in `tests/fixtures/kernel_partition.manifest`, re-blessed by `GANDR_BLESS_KERNEL_PARTITION=1`.
  - `kernel_export_gate.rs` — the `GANDR_BLESS_KERNEL_EXPORT` exit-gate harness driving bridge → `add_decl` → export `write` → `decode` → `read` over the 21 S1-eligible items + the kernel-native C5 goldens, pinning per-item deterministic records (`size-bytes`, `table-entries`, `expanded-work`, `artifact-work`, `artifact-identity`).

**What is absent** (B3.0 confirmed, re-verified): no `gandr-syntax`/`grammar`/`parser`/`pipeline`/`corpus`/`shell` crate under the surface layer; `docs/gandr/` is a stub (`MANIFEST.yml` + `README.md` only — no `core-ir-contract.md`, no `modules.md`); the reboot spec is `docs/spec/*.xml` with no modules node.
Every §12 C2/C3/C4 fix-target of the B3.0 charter is therefore absent in-tree — the reason B3.0 was a confirm-and-record with zero commits.

---

## 3. Ground truth — source side (the six candidates)

From `wyrd@failed-refactor:crates/*/Cargo.toml`, workspace (path) edges only, dev/optional noted.
The transitive closure of the six candidates is 14 crates on the default normal-dep graph (`crate-port-map.md` §2 corroborated):

| Crate                     | Normal workspace deps                                                                                                                      | Dev / optional                                                    | External                                               | Features                                                           |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------- | ------------------------------------------------------ | ------------------------------------------------------------------ |
| `gandr-syntax`            | — (none)                                                                                                                                   | —                                                                 | —                                                      | `default`, `full`                                                  |
| `gandr-render-proto`      | —                                                                                                                                          | —                                                                 | `serde`(opt)                                           | `default`, `codecs`, `full=[codecs]`                               |
| `gandr-graph`             | —                                                                                                                                          | —                                                                 | `fixedbitset`, `petgraph`; `proptest`(dev)             | `default`, `full`                                                  |
| `gandr-nominal`           | —                                                                                                                                          | —                                                                 | `proptest`(dev)                                        | `default`, `full`                                                  |
| `gandr-order-maintenance` | —                                                                                                                                          | —                                                                 | `thiserror`; `criterion`/`proptest`(dev)               | `default`, `full`                                                  |
| `gandr-recursion`         | —                                                                                                                                          | —                                                                 | —                                                      | none                                                               |
| `gandr-core`              | `gandr-nominal`                                                                                                                            | —                                                                 | `no-panic`, `regex`(opt), `proptest`(opt), `thiserror` | `default`, `full=[regex]`, `proptest-strategies`, `regex`          |
| `gandr-desc`              | `gandr-core`                                                                                                                               | —                                                                 | `proptest`(dev)                                        | `default`, `full`                                                  |
| `gandr-grammar`           | `gandr-graph`, `gandr-render-proto`, `gandr-syntax`; **`gandr-tree-sitter`(opt,`parity`)**                                                 | `gandr-parser`(dev — deliberate, breaks the parser↔grammar cycle) | `tree-sitter`(opt), `criterion`/`serde_json`(dev)      | `default`, `full`, `parity` ⚠ (`full` omits `parity`, port-map H1) |
| `gandr-parser`            | `gandr-grammar`, `gandr-graph`, `gandr-syntax`                                                                                             | —                                                                 | `criterion`/`proptest`(dev)                            | `default`, `full`                                                  |
| `gandr-sequent`           | `gandr-core`                                                                                                                               | `gandr-pipeline`(dev)                                             | `proptest`(dev)                                        | `default`, **`features=[]`** ⚠ typo (port-map H2)                  |
| `gandr-pipeline`          | `gandr-core`, `gandr-desc`, `gandr-grammar`, `gandr-nominal`, `gandr-order-maintenance`, `gandr-parser`, `gandr-recursion`, `gandr-syntax` | `gandr-core`(strategies), `gandr-grammar-contract-fixtures`(dev)  | `serde`/`serde_json`(opt), `thiserror`                 | `default`, `codecs`, `full=[codecs,regex]`, `regex`                |
| `gandr-shell`             | `gandr-core`, `gandr-pipeline`                                                                                                             | —                                                                 | `thiserror`                                            | `default`, `full`                                                  |
| `gandr-corpus`            | `gandr-core`, `gandr-desc`, `gandr-pipeline`, `gandr-sequent`, `gandr-shell`; **`gandr-ffi`(opt,`ffi`)**                                   | —                                                                 | —                                                      | `default`, `ffi`, `full=[ffi,regex]`, `regex`                      |

Two dev-only cycle edges must **not** become normal deps in the port: `gandr-grammar` dev→`gandr-parser` and `gandr-sequent` dev→`gandr-pipeline`.

**The prelude / host / attributes tables all live in `gandr-pipeline`** (not the driver, not the shell — the lowerer needs them and cannot depend on the shell without a cycle):

- **Prelude** — `wyrd@failed-refactor:crates/gandr-pipeline/src/prelude.rs`.
  The table is `MODULE_BUILTINS` (`:53`): a `const &[(&str, &str, NativePrim)]` of `(module, member, primitive)` triples (ADR-42).
  It drives three faces from one source of truth: recognition (`is_module_member`/`is_module`), the **typing** prelude `prelude_ctx() -> Ctx` (`:192`), and the **eval** prelude `prelude_env() -> Prelude` (`:238`). (`prelude_ctx` is the exact accessor the fuzz `check`/`lower` targets call — its name is a reboot-visible API commitment.)
- **Host** — `wyrd@failed-refactor:crates/gandr-pipeline/src/host.rs`.
  Signature-name consts (`EXEC`/`FS`/`PROC`/`ENV`, operation names), the `Exec` signature builder `pub fn exec() -> EffectSig` (`:133`) with reply `{stdout,stderr,exit_code}`, companions `fs()`/`env()`/`proc()`, the ADR-74 D4 mode constants (`MODE_CAPTURED`/`MODE_INHERIT`), and the source-surface `HOST_MODULES` table.
  The shell re-exports the effect side as `gandr_shell::sig`.
- **Attributes** — `wyrd@failed-refactor:crates/gandr-pipeline/src/attributes.rs`.
  The table is `REGISTRY` (`:180`): a `pub const &[AttrSchema]` binding attribute names to `ValueType`s (ADR-56); all MVP schemas are `AttrTier::Inert` (side-table only, content-address-neutral).

**The lowering entry** is `pub fn lower_source_total(source: PipelineSource<'_>) -> LowerResult<Lowered>` at `wyrd@failed-refactor:crates/gandr-pipeline/src/lower.rs:572` — total mode (every parseable input lowers; error regions become `Value::Hole`/`Comp::Hole`), the exact spelling in the reboot fixture headers.
Siblings: `lower_source` (strict, `:547`), `lower_source_with` (`:592`), `lower_source_total_with_foreign` (FFI-seeded, `:627`).

**The corpus tree** is `wyrd@failed-refactor:crates/gandr-corpus/examples/{model,pathological,surface}/` (dir consts `MODEL_DIR`/`PATHOLOGICAL_DIR`/`SURFACE_DIR` in `src/lib.rs`), a library-only crate whose end-to-end harness lives in `tests/corpus.rs`; `split_items` (`:601`) slices a model example into top-level items.

---

## 4. Reconciliation table — wyrd crate → reboot home

The reboot is **not greenfield-empty** (the state `crate-port-map.md` §1 recorded); the substrate spine is landed under recut category names.

| wyrd crate                        | role                          | reboot package                   | reboot dir                        | status                                                                                                                                                                   |
| --------------------------------- | ----------------------------- | -------------------------------- | --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `gandr-core`                      | CBPV checker + typing machine | `gandr-core-checker`             | `crates/core-checker`             | **PORTED** (B1)                                                                                                                                                          |
| `gandr-sequent`                   | System-L IL + L machine       | `gandr-core-sequent`             | `crates/core-sequent`             | **PORTED** (B1)                                                                                                                                                          |
| `gandr-kernel-levels`             | level oracle (ADR-78)         | `gandr-kernel-strata`            | `crates/kernel-strata`            | **PORTED** (B2)                                                                                                                                                          |
| `gandr-nominal`                   | atom substrate (ADR-41)       | `gandr-theory-nominal-automata`  | `crates/theory-nominal-automata`  | **PORTED**                                                                                                                                                               |
| `gandr-order-maintenance`         | O(1) total order (A2)         | `gandr-theory-orders`            | `crates/theory-orders`            | **PORTED**                                                                                                                                                               |
| `gandr-graph`                     | graph algos + **`prec` DAG**  | `gandr-theory-graphs`            | `crates/theory-graphs`            | **PORTED**                                                                                                                                                               |
| `gandr-desc`                      | levitation stage-0 desc       | `gandr-theory-levitation`        | `crates/theory-levitation`        | **PORTED**                                                                                                                                                               |
| `gandr-recursion`                 | first-order recursion         | `gandr-theory-recursion`         | `crates/theory-recursion`         | **PORTED**                                                                                                                                                               |
| `gandr-polygraph`                 | Squier completion (L2)        | `gandr-theory-computads`         | `crates/theory-computads`         | **PORTED**                                                                                                                                                               |
| `gandr-vdc`                       | VDC reflection                | `gandr-theory-virtual-doctrines` | `crates/theory-virtual-doctrines` | **PORTED**                                                                                                                                                               |
| `gandr-shell`                     | host-effect runtime           | `gandr-runtime-host`             | `crates/runtime-host`             | **PORTED** (B1 base) — `run_program` + host; the source-runner faces landed in the engine instead, at F3 (2026-07-22) and F5 (2026-08-08); see the evidence anchor below |
| `gandr` (driver)                  | toolchain driver              | `gandr`                          | `crates/surface-driver`           | **PORTED** (F5, 2026-08-08) — script-runner face only; REPL/tui/lsp/mcp/fmt/build deferred                                                                               |
| `wyrd-rust-gates`                 | AST/graph gates               | `gandr-workflow-gates`           | `crates/workflow-gates`           | **RE-HOMED** (reboot-native, not a port)                                                                                                                                 |
| `wyrd-dylint`                     | dylint rules                  | `gandr-workflow-dylint`          | `crates/workflow-dylint`          | **RE-HOMED**                                                                                                                                                             |
| `gandr-syntax`                    | flat-arena CST                | `gandr-surface-syntax`           | `crates/surface-syntax`           | **PORTED** (F0, 2026-07-21)                                                                                                                                              |
| `gandr-render-proto`              | render wire types             | `gandr-surface-render-remote`    | `crates/surface-render-remote`    | **PORTED** (F0, 2026-07-21; owner-renamed from the `surface-render-proto` recommendation)                                                                                |
| `gandr-grammar`                   | PBG + mold                    | `gandr-surface-grammar`          | `crates/surface-grammar`          | **PORTED** (F1, 2026-07-21)                                                                                                                                              |
| `gandr-parser`                    | melder push-machine           | `gandr-surface-parser`           | `crates/surface-parser`           | **PORTED** (F2, 2026-07-21)                                                                                                                                              |
| `gandr-pipeline`                  | lowering hub                  | `gandr-surface-engine`           | `crates/surface-engine`           | **PORTED** (F3, 2026-07-22; owner-renamed from the `surface-pipeline` recommendation)                                                                                    |
| `gandr-corpus`                    | executable corpus             | `gandr-surface-corpus`           | `crates/surface-corpus`           | **PORTED** (F4, landed 2026-07-22)                                                                                                                                       |
| `gandr-tree-sitter`               | TS parity ref                 | —                                | —                                 | **ABSENT** — deferred (parity only)                                                                                                                                      |
| `gandr-ffi`                       | native FFI                    | —                                | —                                 | **ABSENT** — deferred                                                                                                                                                    |
| `gandr-data`                      | value codecs                  | —                                | —                                 | **ABSENT** — deferred                                                                                                                                                    |
| `gandr-lsp`                       | LSP server                    | —                                | —                                 | **ABSENT** — deferred                                                                                                                                                    |
| `gandr-tui`                       | ratatui surface               | —                                | —                                 | **ABSENT** — deferred                                                                                                                                                    |
| `gandr-grammar-contract-fixtures` | fixture locator               | —                                | —                                 | **ABSENT** — dev-dep (fold vs keep, O4)                                                                                                                                  |

Two evidence anchors for the non-obvious rows:

- **`runtime-host` IS `gandr-shell` minus its lowering-coupled faces.** `crates/runtime-host/src/driver.rs` exposes `run_program(comp: &Comp) -> ShellOutcome`, and at the time of this study its module doc stated plainly that `run_source`/`run_source_file` — the CST → core lowering convenience that ran the pipeline lowerer before handing the program to the host loop — were parked, with continuation-key installation "parked with the surface engine".
  It already depends on `gandr-core-checker` + `gandr-core-sequent` (the L machine is the evaluator, not the retired CEK).

  **Landed differently, and the difference is a decision, not a slip (F3, 2026-07-22).** This study prescribed re-wiring the two source-runner faces _into_ `runtime-host` by adding a dependency on the ported lowering hub.
  The F3 rung instead settled HZ-6 by naming `gandr_core_checker::effect::host` the single host-signature authority and pointing the crate edge **engine → runtime**, so the runtime stays a source-free capability adapter that owns native dispatch and nothing else.
  Under that direction the prescription is not merely undesirable but impossible: a normal-dependency `gandr-runtime-host` → `gandr-surface-engine` edge is a Cargo cycle (a dev-only edge would be permitted — the distinction this study draws in §3).
  Both faces therefore live in the engine — `run::run_source` (F3) and `run::run_source_file` (F5) — composing the engine's lowering, linking, and prelude checking with `run_program_with_prelude`.
  Every later reference in this document to "re-wire `run_source`/`run_source_file` onto `runtime-host`" reads against this paragraph.
- **The theory-* crates satisfy the front-end's substrate deps.* * `core-checker` depends on `gandr-theory-nominal-automata` (the ADR-41 `Gensym`/`Atom`) and `gandr-theory-orders`; `theory-graphs` re-exports `prec::*` (the precedence DAG `gandr-grammar`/`gandr-parser` need); `theory-levitation` carries the full desc surface (`desc`/`code`/`decode`/`generic`/`arity`/`cell`/`typed_cell`/`wellformed`/`builtin`/`intern`) and is already integrated against `gandr_core_checker::boundary`.

**Consequence for the cut:** the front-end's `gandr-core`/`gandr-nominal`/`gandr-graph`/`gandr-desc`/`gandr-order-maintenance`/`gandr-recursion` edges all resolve to already-landed reboot crates under new names.
The port re-points those `[dependencies]` (a mechanical rename, §7) rather than porting them.
The only genuinely-new crates are the surface layer (syntax, grammar, parser, pipeline, corpus) plus the `render-proto` leaf.

---

## 5. The minimal cut — in and out, argued

The goal is unblocking **B3.1** ("structures/paths/signatures as primitives + namespace graduation to scoped resolution", `b3-module-system-design.md` §11), not porting every surface crate.
B3.1 needs four capabilities, and each maps to a specific piece of the cut:

| Capability B3.1 needs                                              | Provided by                                                                               | In the cut?    |
| ------------------------------------------------------------------ | ----------------------------------------------------------------------------------------- | -------------- |
| Parse `.gandr` sources                                             | `surface-syntax` (CST) + `surface-grammar` (PBG) + `surface-parser` (melder)              | **yes** (#2–4) |
| Lower to the core IR the checker consumes                          | `surface-pipeline` `lower_source_total` → `Vec<gandr_core_checker::syntax::Term>`         | **yes** (#5)   |
| Namespace / prelude / host resolution                              | `surface-pipeline` `prelude`/`host`/`attributes` tables + the ADR-63 scope-aware resolver | **yes** (#5)   |
| Corpus regeneration (replace one-shot fixtures with live lowering) | `surface-corpus` `.gandr` sources + harness, driving `lower_source_total`                 | **yes** (#6)   |

**In (required):**

1. **`surface-render-proto`** (`gandr-render-proto`) — S.
   `gandr-grammar` depends on it normally (`default-features = false`) for the mold highlighter's wire types.
   A 2.3k-LOC leaf; port verbatim, or feature-gate the highlighter out of the minimal grammar cut and defer it with the TUI/LSP (O2).
2. **`surface-syntax`** (`gandr-syntax`) — S. Zero workspace deps, verbatim-clean (zero `wyrd-*` bead IDs, `crate-port-map.md` §4(2)); the flat-arena CST + `Diff` the parser and pipeline both build on.
3. **`surface-grammar`** (`gandr-grammar`) — M. Deps: `theory-graphs` (`prec`), `surface-render-proto`, `surface-syntax`.
   Port the checked PBG core + mold highlighter; **hold the `parity` feature and its `gandr-tree-sitter` edge deferred** (H1 wart carried — `full` omits `parity`).
4. **`surface-parser`** (`gandr-parser`) — M. Deps: `surface-grammar`, `theory-graphs`, `surface-syntax`.
   The resumable push-machine melder + obligation taxonomy (ADR-73); the parser's zero-obligation corpus gate is what parse-gates the `surface/` witness tree (`docs/workflow/corpus.md`).
5. **`surface-pipeline`** (`gandr-pipeline`) — L.
   **The hub; this rung unblocks B3.1.** Deps: `core-checker`, `theory-levitation`, `surface-grammar`, `theory-nominal-automata`, `theory-orders`, `surface-parser`, `theory-recursion`, `surface-syntax`.
   Carries `lower`, `prelude`, `host`, `attributes`, `session`, `desc_elab`, `origin`, `link`, `goals`.
   The ~600 lines of module lowering (`b3-module-system-design.md` §11 "elaboration-plus-tables") the B3 rungs elaborate live here.
6. **`surface-corpus`** (`gandr-corpus`) — M. Deps: `core-checker`, `theory-levitation`, `surface-pipeline`, `core-sequent`, `runtime-host` (the shell); `gandr-ffi`(opt) held deferred.
   Brings the `.gandr` source trees the reboot lacks and the harness that regenerates the fixtures (§6).

**Adapt / un-stub (not fresh crates):**

- **`runtime-host`** — restore the source-runner faces (superseded at F3: they landed in `surface-engine` instead, because the host-signature ruling made the reverse edge a dependency cycle — §4).
- **`surface-driver`** — un-stub the script-runner face (`gandr <file>`, the `run_source_file` seam), rewriting the stale wyrd dep names/paths; re-homes `mise agda:deps` onto `scripts/agda-deps.gandr`.

**Out (deferred), with reasons:**

- **`gandr-tui`** — the full ratatui surface (8 pinned UI crates, `crate-port-map.md` §6); the REPL editor face, not needed to parse/lower/check.
- **`gandr-lsp`** — the sans-io LSP server; a surface consumer, not on the B3.1 path (and carries the `full` no-op wart, H3).
- **`gandr-ffi`** — libffi/libloading/cc, unsafe, native; corpus `ffi` feature only.
  **Caveat (O6):** two frozen fixtures (`22-ffi-effect-and-capability`, `28-regex-and-path-builtins`) were captured with `ffi`/`regex` on; byte-exact regeneration of just those two needs those features — else keep them frozen (validated against `.sexp`, not regenerated) until FFI/regex land.
- **`gandr-tree-sitter`** + **`packages/tree-sitter-gandr/`** — the E1/E2 differential-parity reference (`crate-port-map.md` §7d); `parity` feature only.
  `mise grammar:test` and the `fuzz` `parity` target stay parked until this returns; the members∩exclude tree-sitter pattern (H5) and the `FORBIDDEN_DEFAULT_GRAPH_PACKAGES` gate (now in reboot `workflow-gates`) must keep tree-sitter/regex off the default graph.
- **`gandr-data`** — JSON/TOML/YAML ↔ value codecs; its consumer is the shell/self-hosting lane (`crate-port-map.md` §7c), not B3.1.
- **`gandr-doc`/`gandr-fmt`/`gandr-pretty`** — spec-planned, **no code exists** (`crate-port-map.md` §7e, H6).
  Consolidating the inline pretty/render modules into these is a **graduation of an inline facet into a component**, an owner scope decision (O7), not a port.
- **`gandr-grammar-contract-fixtures`** — dev-dep of the pipeline only; fold into `surface-grammar` vs keep as a `gandr_test_*` fixtures crate is O4 (`crate-port-map.md` §7b), with the Rust/Node shared-manifest + repo-relative paths as the real work.

---

## 6. Corpus regeneration plan

The pre-lowered `.sexp`/`.outcome` fixtures were captured **once** from wyrd; the port must regenerate them from `.gandr` sources through the ported pipeline and prove agreement, or explain each deviation.
This is where `corpus_fixtures.rs` retires, per its forward pointer (`:20-23`).

**The regeneration contract:**

1. **Port the `.gandr` sources** (`surface-corpus` `examples/{model,pathological}/`) corresponding to exactly the existing 29 model + 27 pathological fixtures — the numbered wyrd files (`05-…`, `29-modules`, `25-host-modules`, `14-agda-deps-walkthrough`, …) plus the subdir-organized set.
2. **Regenerate each `.sexp`** by running `lower_source_total` over its `.gandr` source and re-encoding the `Vec<Term>` in the fixture's s-expression format; **assert byte-identity** against the checked-in fixture (the `corpus_differential` per-file `sexp-b3sum` guard).
3. **Regenerate each `.outcome`** by running the lowered items through the **L machine** (`core-sequent`, `machine.rs`) — the reboot's sole evaluator, superseding the retired CEK oracle (`crate-port-map.md` §7a currency note) — and assert against the checked-in outcome (`corpus_totality` + the outcome-snapshot sweeps).
   The `.sexp` are oracle-independent (lowered terms); only the `.outcome` depends on the evaluator, and those are already pinned by the L machine.
4. **Retire `corpus_fixtures.rs`** and re-point the two sweeps (`corpus_differential`, `corpus_totality`) at live lowering (or at the regenerated fixtures kept as a fast path).

**The re-bless firewall (the charter's hard constraint):** The item set is frozen by three hashes that MUST NOT change at the port:

- `corpus_fixtures_b3sum` (`corpus_fixtures.rs:110`) — folds each fixture's source path + `.sexp` b3sum over the whole consumed set.
- `kernel_partition.manifest` — one line per item with its S1-eligibility class (`kernel_corpus_partition.rs`).
- the per-item `kernel_export_gate` records (`size-bytes`/`table-entries`/`expanded-work`/`artifact-work`/`artifact-identity`).

Because regeneration reproduces the **same lowered terms** for the **same source set**, all three are invariant — **no `GANDR_BLESS_KERNEL_PARTITION` / `GANDR_BLESS_KERNEL_EXPORT` re-bless at the port.** If the port's plan implies a re-bless, that is the tell that the item set changed, and it must be split out:

- **New corpus items** (a `surface/` parse-gated witness tree, or examples beyond the frozen 29+27) defer to their **graduation rungs** per ADR-84 (`docs/workflow/corpus.md`) — a syntax-first change lands a parse-gated `surface/` witness first, promoted to runnable `model/` at its semantics-graduation change.
  They enter the partition/export sets only when they graduate, with an intentional re-bless recorded then — **not** at the port (O5).
- **`22-ffi` / `28-regex`** byte-exactness needs the `ffi`/`regex` features (O6): regenerate the rest, keep these two frozen-and-verified until their features land, or pull `gandr-ffi`+`regex` into the corpus-regeneration cut for full regeneration.

**Companion re-syncs the port owns** (from the B3.0 record and the charter):

- **`14-agda-deps-walkthrough`** re-sync: the walkthrough is an older module-free version of the real `scripts/agda-deps.gandr` (§12 C4).
  The driver un-stub re-enables `mise agda:deps` (`cargo run -p gandr -- scripts/agda-deps.gandr`), so the walkthrough and the live script reconverge at the port.
- **C2/C3/C4 graduation** (`b3-module-system-design.md` §12): with the pipeline + a reboot module-spec home in place, the C2 M1-lite note, the C3 corrected "partial" verdict, and the C4 in-tree note fixes land against **reboot-native** homes (a node in the `docs/gandr/spec/` corpus) — the targets that were absent at B3.0 and forced the confirm-and-record.

---

## 7. Reconciliation with reboot reality — per-crate pricing

The reboot core has moved since wyrd: the L machine supersedes the CEK, kernel-core carries the D1(C) arena (ADR-50), and the B2 kernel layer sits below the checker.
Pricing the adaptation each ported crate needs — **verbatim** (rename deps only), **adapt** (retarget an API or a small rewrite), **rewrite** (structural):

| Crate                      | Price                  | Why                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| -------------------------- | ---------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `surface-syntax`           | **verbatim**           | zero workspace deps, zero wyrd bead IDs; a name-only landing                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| `surface-render-proto`     | **verbatim**           | leaf wire types; `serde`(opt) only                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| `surface-grammar`          | **adapt**              | re-point `gandr-graph`→`theory-graphs`, `gandr-render-proto`/`gandr-syntax`→surface names; hold `parity`/tree-sitter; the mold highlighter's render bus retargets the ported render-proto                                                                                                                                                                                                                                                                                                     |
| `surface-parser`           | **adapt**              | re-point grammar/graph/syntax deps; verify the push-machine survives `dylint:recursion` (it is a worklist by design — likely clean)                                                                                                                                                                                                                                                                                                                                                           |
| `surface-pipeline`         | **adapt→rewrite risk** | re-point 8 deps to reboot names; the `desc_elab` module targets `theory-levitation`'s reboot-adapted (not verbatim-wyrd) desc API; the lowerer's recursive descent may trip `dylint:recursion` and need the same defunctionalization the kernel checker got (`kernel-core` STATUS "input recursion: none") — the single biggest adaptation risk                                                                                                                                               |
| `surface-corpus`           | **adapt**              | harness eval retargets CEK→L machine; the `.gandr` sources port verbatim; re-point 5 deps                                                                                                                                                                                                                                                                                                                                                                                                     |
| `runtime-host` (adapt)     | **adapt**              | reconcile the host-signature split (pipeline `host.rs` `HOST_MODULES` table vs `runtime-host` `sig`) so there is one authority, not two drifting copies. **As landed (F3):** `gandr_core_checker::effect::host` is the authority, the engine re-exports it and adds source-level metadata, and the source-runner faces landed in the engine rather than here — the lowering-hub dep this row prescribed — the crate that landed as `surface-engine` — would be a normal-dependency cycle (§4) |
| `surface-driver` (un-stub) | **adapt**              | rewrite the stale wyrd commented deps to reboot names/paths; enable only the script-runner face (O8)                                                                                                                                                                                                                                                                                                                                                                                          |

Cross-cutting reconciliation the port must price once:

- **Package renames cascade.** If the reboot convention holds (dir `core-checker` ⇒ package `gandr-core-checker`), the surface crates become `gandr-surface-syntax`/`-grammar`/`-parser`/`-pipeline`/`-corpus`, and every internal `use gandr_syntax::` → `use gandr_surface_syntax::` across the ported crates is a mechanical pass (O1).
- **Retired wyrd bead-ID comments** (`crate-port-map.md` §0, H7): 126 distinct `wyrd-*` IDs and `wyrd@failed-refactor` file:line locators are woven through the doc comments of exactly these crates (heaviest in `gandr-pipeline` 150, `gandr-grammar` 90, `gandr-parser` 75).
  Under `doc-check` (`cargo doc --workspace -D warnings`) any that render as broken intra-doc links break the wall; the H7 policy (rewrite/drop/keep-as-provenance) must be decided **before** each crate's first green doc-check (O3), and the contributor-concern angle (bead forensics out of tracked history) argues for drop-with-distilled-provenance.
- **`wyrd-dylint`/`wyrd-rust-gates` coupling dissolves.** The reboot ships reboot-native `workflow-gates`/`workflow-dylint`; the ported crates answer to those, and the `dylint_lib = "wyrd_dylint"` test attributes (`crate-port-map.md` §4(3)) rewrite to the reboot lint name — a small rename, not a re-home decision.

---

## 8. Wall and config integration (per crate, per rung)

The six-task Rust-and-format portion of `gate:merge` is `cargo:build`, `cargo:clippy`, `cargo:dylint:local`, `cargo:doc-check`, `cargo:nextest`, `treefmt:check`.
Per-crate obligations the coordinator mints into each rung:

- **Commitlint scopes — RETIRED OBLIGATION (owner consolidation, 2026-07-21).** The per-crate scope registration this bullet originally prescribed (the `wvd.23` trap: each new crate's dir-scope added before its first commit) is superseded: the vocabulary is now CLOSED at eleven broad scopes (`GANDR_SCOPES` in `.commitlintrc.mts` — the seven crate-category prefixes + `analysis`/`docs`/`repo`/`spec`), and per-crate scopes no longer exist.
  Port rungs commit under their category prefix (`feat(surface): …`); no scope-registration commits from F2 on.
  Do not add scopes without owner authorization.
- **`doc-check`** (`cargo doc --workspace -D warnings`): every new crate must be rustdoc-clean.
  The wyrd sources carry rich `# Contract` rustdoc, but the 126 wyrd-bead-ID / `wyrd@failed-refactor` locator comments (O3) are the likely `-D warnings` failure surface — resolve the H7 policy per crate before its doc-check turns green.
- **`dylint:recursion`**: gates iterative style (no input recursion) on new code.
  `surface-parser` (push-machine) and most of `surface-pipeline` are worklist-shaped; the lowerer's recursive-descent arms are the risk (§7) — budget a defunctionalization pass if the gate fires.
- **`treefmt` / `rumdl`**: run over every new source and doc; `treefmt` reflows markdown to sentence-per-line.
- **`crate-port-map.md` row updates**: flip each ported crate's status as it lands (and correct §1's "no `crates/`" premise, now stale — the reboot substrate is landed).
- **Per-crate `docs/STATUS.md` + `docs/CHANGELOG.md`**: the workspace convention (`kernel-core`/`storage-artifact` are the templates — dated `## YYYY-MM-DD — <title> (stage)` sections with `current:` bullets in CHANGELOG; a prose STATUS narrating implemented/deferred).
  Present today for `core-checker`/`kernel-core`/`kernel-strata`/`storage-*`/`theory-orders`; the new surface crates each get the pair, and `runtime-host`/`surface-driver` get CHANGELOG entries for the re-wire/un-stub.
  **Disposition (2026-08-08):** the `runtime-host` half is **retired** — there was no re-wire to record, since the source-runner faces landed in the engine (§4), and that crate has no `docs/` tree at all; the engine's own CHANGELOG carries the entry instead.
  `surface-driver` gained the full STATUS/CHANGELOG pair at F5, not merely a CHANGELOG entry.
- **`mise grammar:test` + `fuzz/` restoration points**: both return with the tree-sitter reference (deferred), and `grammar:test` re-enters the `gate:merge` line then (`mise.toml:81`); `fuzz/`'s manifest gets its stale wyrd paths rewritten to reboot names when it un-excludes.
- **`workflow-gates` roster is dynamic** — no per-crate action; the gate crate discovers workspace members rather than hardcoding a roster.
- **Default-graph purity**: preserve the members∩exclude tree-sitter pattern (H5) and keep `regex`/`tree-sitter` behind non-default features so the reboot `workflow-gates` forbidden-package gate stays green.

---

## 9. Staging table — waves/rungs for coordinator minting

Shape follows `b3-module-system-design.md` §11 (contents / proves-it / size), with the dependency order and the six-task wall as each rung's exit gate.
Every rung is behind the fcw.11 exit-gate floor (build + goldens + differential + wall) and honors ADR-84 corpus discipline.

| Rung              | Contents                                                                                                                                                                                                                                                                                                                                                                                              | Dependency order                                                                                     | Exit gate (beyond the six-task wall)                                                                                                                                            | Size | Unblocks B3.1?                      |
| ----------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---- | ----------------------------------- |
| **F0**            | `surface-render-proto` + `surface-syntax` (leaves)                                                                                                                                                                                                                                                                                                                                                    | none (theory-* already present)                                                                      | crates build + rustdoc-clean; STATUS/CHANGELOG pair each; scopes registered                                                                                                     | S    | no                                  |
| **F1**            | `surface-grammar` (PBG + mold; `parity`/tree-sitter deferred)                                                                                                                                                                                                                                                                                                                                         | F0 + `theory-graphs`                                                                                 | grammar unit goldens green; default graph tree-sitter-free                                                                                                                      | M    | no                                  |
| **F2**            | `surface-parser` (melder push-machine + obligations)                                                                                                                                                                                                                                                                                                                                                  | F1                                                                                                   | parser goldens; zero-obligation corpus-gate count lock; `dylint:recursion` clean                                                                                                | M    | no                                  |
| **F3**            | **`surface-pipeline`** (`lower_source_total`, prelude/host/attributes, session, desc_elab)                                                                                                                                                                                                                                                                                                            | F2 + `core-checker`/`theory-levitation`/`theory-orders`/`theory-nominal-automata`/`theory-recursion` | lowering goldens; **L differential green** (lower→check→L machine) for the delta; `dylint:recursion` clean                                                                      | L    | **YES**                             |
| **F4**            | **`surface-corpus` regeneration** — port `.gandr` sources; regenerate `.sexp` (byte-identity) + `.outcome` (L machine); retire `corpus_fixtures.rs`; re-point the two sweeps                                                                                                                                                                                                                          | F3 + `core-sequent`/`runtime-host`                                                                   | `corpus_differential` + `corpus_totality` green **unchanged**; `corpus_fixtures_b3sum` + `kernel_partition.manifest` + `kernel_export_gate` records **unchanged (no re-bless)** | M    | consolidates B3.1's corpus proof    |
| **F5**            | _chartered:_ `runtime-host` re-wire (`run_source`/`run_source_file`) + `surface-driver` un-stub (script-runner face; rewrite stale deps). _As landed (2026-08-08):_ the re-wire half was refuted — `run_source_file` joined `run_source` in `surface-engine`, because the F3 host-signature ruling makes the chartered placement a normal-dependency cycle (§4); the un-stub half landed as chartered | F3 (F4 for the corpus harness)                                                                       | driver smoke; `mise agda:deps` runs `scripts/agda-deps.gandr`; `14-agda-deps-walkthrough` reconverges                                                                           | M    | enables Agda vendoring + C4 re-sync |
| **F6** (deferred) | tree-sitter parity reference + `mise grammar:test` + `fuzz/` restoration; C2/C3/C4 spec-home landings                                                                                                                                                                                                                                                                                                 | after F3–F5                                                                                          | grammar parity suite; fuzz targets build under reboot names                                                                                                                     | M    | out of the B3.1-critical path       |

**The critical path to B3.1 is F0 → F1 → F2 → F3.** F4 is the exit-gate proof (corpus regeneration with no re-bless); F5 restores the driver/Agda faces; F6 is the deferred parity/spec tail.
Sizing corroboration: the built module machinery B3 elaborates is "~600 lines of module lowering, the prelude/host const tables, grammar rules … with zero typing-judgment or core-IR footprint" (`b3-module-system-design.md` §11) — all of which lives in the F3 pipeline rung, which is why F3 is the B3.1 unblock.

---

## 10. Hazards

Flagged, not smoothed (the B2/B3 study convention).

- **HZ-1 — the corpus b3sum firewall.** The frozen 29+27 fixture set is hashed three ways (§6); regeneration must be byte-identical with **no** partition/export re-bless.
  Any plan that adds corpus items at the port (surface witness tree, new examples) silently changes a hash — those items must defer to their ADR-84 graduation rungs, not ride the port.
- **HZ-2 — `22-ffi` / `28-regex` feature coupling.** Two frozen fixtures need `ffi`/`regex` to byte-regenerate; the minimal cut defers FFI.
  Either keep them frozen-and-verified (not regenerated) until the features land, or pull `gandr-ffi`+`regex` into F4 — an owner call (O6).
- **HZ-3 — stale wyrd manifests, not ready-to-uncomment.** Both `surface-driver`'s commented deps and `fuzz/`'s manifest spell wyrd names/paths (`../gandr-core`, `../crates/gandr-core`) that do not exist in the reboot's category scheme; uncommenting them fails.
  They need a rewrite to reboot names/paths, and the package renames cascade into every `use` (O1).
  **Discharged for `surface-driver` (F5, 2026-08-08)**: the script-runner face took three real edges (`gandr-surface-engine` and `gandr-runtime-host` functionally, plus `gandr-core-checker` for the outcome vocabulary), and the commented entries were deleted in favour of prose naming what each deferred face waits on rather than a predecessor path — the hazard is retired for that manifest by removing the uncommentable text, not by uncommenting it.
  `fuzz/` still carries the stale manifest and stays on the F6 tail.
- **HZ-4 — `dylint:recursion` on the ported lowerer.** The reboot rewrote its core checker defunctionalized ("input recursion: none", `kernel-core` STATUS); the wyrd pipeline lowerer carries recursive descent that the reboot `dylint:recursion` gate may reject, forcing the same treatment — the largest single adaptation risk (§7), and it lands on the critical-path F3 rung.
- **HZ-5 — wyrd bead-ID debt vs `doc-check`.** 126 `wyrd-*` bead IDs + `wyrd@failed-refactor` locators live in the doc comments of exactly the ported crates; under `-D warnings` the broken-link subset fails the wall.
  The H7 policy must be resolved before each crate's first green doc-check (O3); contributor-concern discipline argues for drop-with-distilled-provenance.
- **HZ-6 — the host-signature split.** `gandr-pipeline` `host.rs` owns the source-surface `HOST_MODULES` table (lowering-time); `runtime-host` `sig` owns the runtime effect signatures (dispatch-time).
  The port must name one authority and derive the other, or the two `Exec`/`Fs`/`Proc`/`Env` signature sets drift.
  **Resolved (F3, 2026-07-22)**: `gandr_core_checker::effect::host` is the canonical signature authority beside the representation-independent host seam; `gandr-surface-engine::host` re-exports it and adds only source-level module metadata; `gandr-runtime-host` imports the same signatures for native dispatch.
  No `surface-engine` ↔ `runtime-host` signature edge remains, and two-way parity is tested.
  The consequence for F5 is recorded in §4: the source-runner faces land in the engine, because the direction this ruling fixes makes the study's `runtime-host` placement a dependency cycle.
- **HZ-7 — `render-proto` is genuinely missing.** `gandr-grammar` needs it as a normal dep; it is the one substrate leaf the reboot has not already ported.
  Port-as-leaf vs feature-gate-the-highlighter-out is O2 — do not silently drop it and leave grammar unbuildable.
- **HZ-8 — carried feature warts.** `gandr-grammar` `full` omits `parity` (port-map H1) and `gandr-sequent` has the `features=[]` typo (H2); if the port copies feature tables verbatim, the grammar parity lane is inert and any `--features full -p surface-grammar` misbehaves.
  Fix at the port, not after.
- **HZ-9 — evaluation oracle changed.** The wyrd corpus harness evaluated via the CEK (`gandr_core::eval::run`); the reboot evaluator is the L machine (`core-sequent`).
  The ported harness's eval step retargets CEK→L machine (§6/§7); the `.outcome` fixtures are already L-machine-pinned, so this is an adapt, but a silent CEK dependency in the harness would fail to build.

---

## 11. Open owner calls

Separated from the recommendations above; each a genuine fork.

- **O1 — surface crate naming.** `surface-{syntax,grammar,parser,pipeline,corpus}` dirs + `gandr-surface-*` packages (recommended — matches the `core-*`/`kernel-*`/`theory-*` precedent), or keep the wyrd `gandr-*` names.
  The recommended scheme prices in the `use`-rename cascade (§7).
- **O2 — `render-proto` disposition.** Port as `surface-render-proto` leaf (recommended — cheapest, keeps grammar's feature graph intact), or feature-gate the mold highlighter out of the minimal grammar cut and defer render-proto with the TUI/LSP.
- **O3 — wyrd bead-ID / locator policy (H7).** Drop as contributor-concern with a distilled provenance line where load-bearing (recommended), or rewrite to gandr beads, or keep as provenance.
  Blocks `doc-check`; must be decided before each crate's first green wall.
- **O4 — `gandr-grammar-contract-fixtures`.** Fold into `surface-grammar` (recommended, `crate-port-map.md` §7b) treating the Rust/Node shared-manifest + repo-relative paths as the real work, or keep as a separate `gandr_test_*` fixtures crate.
- **O5 — corpus item-set at the port.** Regenerate exactly the frozen 29+27 set (no re-bless), deferring the `surface/` witness tree and any new examples to their graduation rungs (recommended — honors the charter's hard constraint), or expand the set at the port (forces an intentional re-bless).
- **O6 — `22-ffi` / `28-regex` byte-exactness.** Keep the two feature-coupled fixtures frozen-and-verified until FFI/regex land (recommended), or pull `gandr-ffi`+`regex` into F4 for full regeneration.
- **O7 — formatting crates (§7e, H6).** Confirm `gandr-doc`/`gandr-fmt`/`gandr-pretty` stay out of the cut (recommended — they are a graduation of inline facets into components, no code exists to port), and are not silently dropped from the design record.
- **O8 — driver faces at the port.** Land only the script-runner face (`gandr <file>`, needed for `agda:deps` + the corpus harness; recommended), deferring the reedline REPL + `tui`/`lsp`/`mcp`/`fmt`/`build` faces.
  **Adopted and executed (F5, 2026-08-08.)** `gandr <file>` and `gandr --help` (or `-h`) are the whole accepted command line; an unrecognized flag is refused rather than read as a path, so a deferred face fails loudly instead of silently.
- **O9 — C2/C3/C4 spec home.** With no `core-ir-contract.md`/`modules.md` in the reboot, choose where the C2 M1-lite note, the C3 "partial" verdict, and the C4 corrections land in the `docs/gandr/spec/` corpus, so the B3.0 escalation (`gandr-wvd.3.2`) finally discharges at the pipeline rung.

---

## 12. Provenance

Source-side facts cite `wyrd@failed-refactor:crates/<crate>/<path>[:line]`, read via `git show` against the shared checkout (never mutated).
Reboot-side facts are the worktree `crates/` schema, `mise.toml`, `.commitlintrc.mts`, the `core-sequent` fixtures/harnesses, and the B3.0 stage record (`gandr-wvd.3.2`).
Port-plan authority is `docs/research/crate-port-map.md`; corpus discipline is ADR-84 (`docs/workflow/corpus.md`); the rung shape follows `docs/research/b3-module-system-design.md` §11.
This is a staging study — no code, no crate ports; the coordinator mints rungs from §9.
