# Toolchain & workflow retention/adaptation (gandr-fcw.7)

> Research report for the gandr language reboot. Source of record:
> `wyrd@failed-refactor` (the canonical wyrd tree), cross-checked against
> `iu` (`internal-univalence`). Every claim cites `alias:relative/path`.
> All paths are alias-relative; no machine-local paths appear here by design.

## Executive summary

1. **The toolchain nightly pin is internally out of sync.** The target is
   `nightly-2026-07-07`, and only `wyrd@failed-refactor:rust-toolchain.toml`
   (line 4) carries it. `wyrd@failed-refactor:mise.toml` (line 37) and
   `wyrd@failed-refactor:.github/workflows/ci.yml` (line 18) both still pin
   `RUSTUP_TOOLCHAIN_NIGHTLY = nightly-2026-05-28`. Worse, mise's `[env]` sets
   `RUSTUP_TOOLCHAIN` on every task, and the rustup proxies dispatch on it
   (mise.toml lines 28-46), so **inside mise the `rust-toolchain.toml` channel
   is shadowed** — the 2026-07-07 pin is effectively dead except for bare
   `cargo`/`rustup` invocations outside mise. Sync = bump mise.toml line 37 and
   ci.yml line 18 to `nightly-2026-07-07`. (Stable is pinned `1.96.0` at
   mise.toml line 36 and ci.yml line 17; `rust-toolchain.toml` has **no** stable
   channel.)

2. **The linter/formatter stack is retained wholesale, mechanism intact; only
   crate/label/tracker names carry a `wyrd-` prefix that must become `gandr-`.**
   The two load-bearing renames are the gate crate `wyrd-rust-gates` →
   `gandr-*-gates` (~24 subcommand callsites in mise.toml plus treefmt.toml's
   rumdl formatters) and the dylint driver crate `wyrd-dylint` → `gandr-dylint`.

3. **`treefmt.toml` orchestrates 10 active formatters** (oxfmt, oxlint,
   rumdl-format, rumdl-verify, rustfmt, sizelint, tombi-format, tombi-lint,
   typos, zizmor; yamllint is commented out). Every referenced config file
   exists and is retained: `.oxfmtrc.json`, `.oxlintrc.json`, `rumdl.toml`,
   `sizelint.toml`, `tombi.toml`, `typos.toml`, `rustfmt.toml`. rumdl runs
   *through* `wyrd-rust-gates`, not directly.

4. **docs/workflow (11 files): retain 10 near-verbatim, adapt names only;
   `scripting.md` is the one with real content shift** (its residual project-
   local Nushell — the `docs/manual/tools/*.nu` highlight/build tooling — is the
   "becomes Rust then gandr" target; shared-core `.agents/core/scripts/*.nu`
   stays Nushell and is out of scope to rewrite).

5. **CI: 11 active `ci.yml` jobs carry to gandr; 2 are already disabled**
   (`agda-check`, `cargo-miri-test-crates`, both `if: false`).
   `scheduled-campaigns.yml` **presumes a self-hosted `wyrd-maintenance`
   macOS/ARM64 runner that will not exist** in the reboot — both jobs will queue
   forever. Decision needed: rename label + provision runner, or defer the whole
   scheduled workflow until a runner exists.

6. **typos merge**: union `wyrd@failed-refactor:typos.toml` with
   `iu:typos.toml` (present), drop the wyrd-machine excludes, and rewrite the
   tracker-ID regex `\bwyrd-[0-9a-z]+\b` → `\bgandr-[0-9a-z]+\b`.

7. Small fixes: `.commitlintrc.mts` scope list drops `wyrd`, keeps `gandr-*`;
   `package.json` `name: "wyrd"` → gandr; `tsconfig.json` carries verbatim (its
   only job is silencing a TS2307 on the commitlint config import).

A cross-cutting **rename ledger** is in the last section — apply it uniformly.

---

## 1. Toolchain pin sync → `nightly-2026-07-07`

Enumeration of every pin location and its **current** value:

| Location | Key | Current value | Action |
| --- | --- | --- | --- |
| `wyrd@failed-refactor:rust-toolchain.toml` L4 | `[toolchain] channel` | `nightly-2026-07-07` | already at target; keep |
| `wyrd@failed-refactor:mise.toml` L36 | `[env] RUSTUP_TOOLCHAIN` default | `1.96.0` (stable) | keep (stable pin) |
| `wyrd@failed-refactor:mise.toml` L37 | `[env] RUSTUP_TOOLCHAIN_NIGHTLY` default | `nightly-2026-05-28` | **bump → 2026-07-07** |
| `wyrd@failed-refactor:.github/workflows/ci.yml` L17 | `env: RUSTUP_TOOLCHAIN` | `1.96.0` | keep (stable pin) |
| `wyrd@failed-refactor:.github/workflows/ci.yml` L18 | `env: RUSTUP_TOOLCHAIN_NIGHTLY` | `nightly-2026-05-28` | **bump → 2026-07-07** |

Supporting facts:

- `rust-toolchain.toml` also fixes `components = ["clippy", "llvm-tools-preview",
  "rustfmt", "rust-src"]`, `profile = "minimal"` (L5-6). CI instead sets
  components via `RUSTUP_COMPONENTS`/`RUSTUP_COMPONENTS_NIGHTLY`
  (ci.yml L19-20: stable `clippy`; nightly `rustfmt,rustc-dev,llvm-tools-preview`).
  These two component sets are **maintained independently** and do not match
  `rust-toolchain.toml`'s set — a second sync surface to reconcile.
- mise.toml's own comment (L29-31) asserts "Toolchain pins mirror
  `.github/workflows/ci.yml env:`" — true for mise↔ci, but neither mirrors
  `rust-toolchain.toml`, which is the actual point of divergence.
- Why nightly matters: `rustfmt.toml` uses nightly-only options
  (`imports_granularity`, `wrap_comments`, `group_imports`, …), so `cargo fmt`
  is always run under the nightly pin (mise.toml L616-629; treefmt's `rustfmt`
  formatter runs under `RUSTUP_TOOLCHAIN_NIGHTLY`, mise.toml L793-803).
- **Recommendation**: make `rust-toolchain.toml` the single source of truth and
  have mise `[env]`/ci `env` read from it, OR at minimum add a gate that
  asserts the three nightly values are equal. There is a `ci-contracts` gate
  (`wyrd-rust-gates -- ci-contracts`, mise.toml L830-834) that already validates
  ci.yml routing; extending it to assert pin equality is the natural home.

Adjacent version pins (not "toolchain" but co-located, for completeness):
`min_version = "2026.7.5"` (mise.toml L7); tool versions in `[tools]` (mise.toml
L894-950) including `github:agda/agda = 2.8.0`, `node = 26.5.0`, `aube = 1.26.0`,
`typst = 0.15.0`, `cargo:cargo-dylint = 6.0.1` / `dylint-link = 6.0.1` (the
Trail-of-Bits v6.0.1 pin), `cargo:nu = 0.113.1` (pinned `=`, comment L914);
CI mirrors of these in `ci.yml env:` L21-23 (`CI_VERSION_AUBE=1.26.0`,
`CI_VERSION_MISE=2026.7.5`, `CI_VERSION_NODE=26.5.0`). `mise.lock` pins exact
tool builds and carries over as-is (regenerate after any bump).

---

## 2. Root Rust configs: clippy.toml, dylint.toml, rustfmt.toml

| File | Disposition | Notes |
| --- | --- | --- |
| `clippy.toml` | **ADAPT** (mechanism verbatim, data regenerated) | Sole setting is `allowed-duplicate-crates` (L34-54) — a dependency-duplication allowlist for `multiple_crate_versions` (denied via the `cargo` group). The list (bit-set, ratatui/termwiz chain, reedline chain via `strum`/`itertools`/`vte`, `portable-pty`→`nix 0.28`, proptest's `getrandom`/`rand` split, `hashbrown` 0.16/0.17, `petgraph`'s `foldhash`) is **specific to gandr's actual lockfile** and must be re-derived from the reboot's `Cargo.lock`. The header comment already uses gandr crate names (`gandr-tui`, `gandr-data`, `gandr`), so the file is gandr-aware today; only the crate list needs regeneration. Keep the `#:tombi toml-version = "v1.0.0"` header. |
| `dylint.toml` | **RETAIN verbatim** | Two lines: `[non_local_effect_before_unhandled_error] work_limit = 5_000_000` (a Trail-of-Bits lint tuning). No wyrd/gandr-specific content. |
| `rustfmt.toml` | **RETAIN verbatim** | Full nightly style: `edition = "2024"`, `brace_style = AlwaysNextLine`, `group_imports = StdExternalCrate`, `imports_granularity = Item`, `fn_params_layout = Vertical`, `wrap_comments`, etc. (30 lines). No name coupling. Requires nightly (§1). |

---

## 3. treefmt.toml + every referenced linter config

`wyrd@failed-refactor:treefmt.toml` — retained. Global `excludes` (L17-25):
`.beads/**`, `.agents/core/**`, `.claude/worktrees/**`, `node_modules/**`,
`target/**`, `coverage/**`, `.rumdl_cache/**`. Formatter table (each carries a
`includes`/`excludes` scope):

| treefmt formatter | command | config file it references | disposition |
| --- | --- | --- | --- |
| `oxfmt` (L27) | `oxfmt --config .oxfmtrc.json` | `.oxfmtrc.json` | retain; excludes list is gandr-path-specific (see below) |
| `oxlint` (L68) | `oxlint --config .oxlintrc.json` | `.oxlintrc.json` | retain verbatim |
| `rumdl-format` (L95) | `cargo run -p wyrd-rust-gates -- rumdl fmt` | `rumdl.toml` | retain; **rename gate crate** |
| `rumdl-verify` (L101) | `cargo run -p wyrd-rust-gates -- rumdl check` | `rumdl.toml` | retain; **rename gate crate** |
| `rustfmt` (L107) | `rustfmt --edition 2024 --config skip_children=true` | `rustfmt.toml` | retain verbatim |
| `sizelint` (L112) | `sizelint check` | `sizelint.toml` | retain verbatim |
| `tombi-format` (L126) | `tombi format` | `tombi.toml` | retain verbatim |
| `tombi-lint` (L131) | `tombi lint --error-on-warnings` | `tombi.toml` | retain verbatim |
| `typos` (L136) | `typos --write-changes --force-exclude` | `typos.toml` | retain; **see §9 merge plan** |
| `zizmor` (L155) | `zizmor --no-progress` | (none; scans `.github/workflows`) | retain verbatim |
| `yamllint` (L151-153) | *commented out* | — | drop (already inert) |

Key details worth the coordinator's attention:

- **rumdl runs through the gate crate**, not the pinned `cargo:rumdl` binary
  directly (treefmt L97-98, L103-104). The mutating `rumdl-format` is
  `priority = 0` and the verifier `priority = 1`, so fmt precedes check.
- **oxfmt `excludes`** (treefmt L61-66) name generated/layout-sensitive JSON:
  `packages/tree-sitter-gandr/src/grammar.json`, `.../node-types.json`,
  `.../package-lock.json`, `crates/gandr-grammar-contract-fixtures/fixtures/manifest.json`.
  All already gandr-named — carry, verify the paths still exist in the reboot.
- **Fidelity excludes** on `sizelint`/`typos` (treefmt L116-124, L140-149):
  `*.typ`, `*.agda`, `*.agda-lib` — protecting Track-C typst sources and
  metatheory Agda's Unicode math identifiers (core hazard H8). Mirrored in
  `typos.toml extend-exclude`. Carry.
- Referenced configs, each retained:
  - `.oxfmtrc.json`: `printWidth 120`, `proseWrap preserve` (md override 80/always),
    `typeAware`/`typeCheck` true, import-sort groups, `sortPackageJson`. Verbatim.
  - `.oxlintrc.json`: `correctness`/`perf = error`, plugins
    `[typescript, oxc, import, jsdoc, node, promise]`, rule set (`no-console`,
    `import/no-cycle` maxDepth 3, `prefer-const`, `curly`, …). Verbatim.
  - `rumdl.toml`: `[global] flavor=standard`, `line-length=0`, exclude list
    (`.beads`, `.agents/core`, `.claude/worktrees`, `node_modules`, `vendor`,
    `dist`, `build`), and ~60 per-rule MD00x blocks incl. `MD052 shortcut-syntax=false`
    (keeps editorial `[label]` brackets inert — a documented math-prose hazard).
    Verbatim.
  - `sizelint.toml`: `max_file_size 2MB` / `warn 1MB`, staged+working-tree,
    `respect_gitignore`, `fail_on_warn`. Verbatim.
  - `tombi.toml`: `toml-version v1.0.0`, empty `[files] exclude`,
    `respect-ignore-files`. Verbatim.

---

## 4. docs/workflow/* — per-file retain / adapt / drop (all 11)

| # | File | Disposition | Adaptation |
| --- | --- | --- | --- |
| 1 | `agda.md` | **RETAIN** | Content is IU-house-style adoption + gandr Agda deltas; already gandr-named (`Gandr.Everything`, dictionary.yml, `iu:check`). No change beyond any doc-corpus path renames. |
| 2 | `ci.md` | **ADAPT (names)** | Gate/tier prose is structurally correct; rename `wyrd-*` bead IDs, the `wyrd-maintenance` runner reference (L70), and the temporary-remediation notes tied to the failed-refactor (`coverage:check` disabled note L43, dylint-landing ledger refs). |
| 3 | `corpus.md` | **RETAIN** | All gandr-named already (`gandr-corpus`, `gandr-pro`, ADR-52/84). Bead-ID renames only. |
| 4 | `docs.md` | **RETAIN** | Documentation-economy posture + math-Markdown authoring rules; references `treefmt.toml`/`typos.toml` fidelity excludes (still true). Bead-ID/notes-repo naming only. |
| 5 | `mutation-adequacy.md` | **RETAIN** | Adequacy-ladder discipline (ADR-71/72); tooling-neutral. Rename bead IDs and `mise run mutants:*` if task names change. |
| 6 | `review.md` | **RETAIN** | Adversarial-review discipline; references the sibling notes repo `wyrd-notes` (→ `gandr-notes` or equivalent) and `worktrees.md`. Naming only. |
| 7 | **`rust.md`** | **RETAIN (the 4 rules are the deliverable, §5)** | Enforcement references `wyrd-dylint`/`wyrd-rust-gates` (L52-61) and `Cargo.toml [workspace.lints]` — rename the crates; the rules themselves are language-neutral. |
| 8 | **`scripting.md`** | **ADAPT (real content shift)** | The one file whose content genuinely changes. See note below. |
| 9 | `soundness.md` | **RETAIN** | checker≡machine differential discipline (ADR-9/48); references `gandr-core`, `precise-analyst`. Naming only. |
| 10 | `tracker.md` | **ADAPT (names)** | Beads discipline; DoltHub repo `silvanshade/wyrd-beads` → gandr beads repo, prefix `wyrd-` → `gandr-`, notes repo rename. Structure retained. |
| 11 | `worktrees.md` | **RETAIN** | Worktrunk/isolation discipline; references `.config/wt.toml`, `.agents/core`, IU submodule, `wyrd-notes`. Naming only. |

**`scripting.md` content shift (per the task's framing).** As written, the file
already places project-local gates in the Rust crate `crates/wyrd-rust-gates`
(L8-10) and states the ownership boundary: shared-core Nushell under
`.agents/core/scripts/*.nu` (conflict-markers, core:check, the Worktrunk ADR
guard) **remains live Nushell** and is *not* a rewrite candidate (L12-15);
"Do not … copy shared-core Nushell into the Rust crate" (L15). The residual
**project-local** Nushell that the reboot sheds is the manual/highlight tooling
under `docs/manual/tools/*.nu` (`build-manual.nu`, `highlight-gandr.nu`),
invoked by the `docs:manual*`, `docs:highlight*`, and `docs:chapter:*` tasks
(mise.toml L274-419). The trajectory endpoint already exists in-tree: `agda:deps`
runs a **`.gandr` script** through the gandr driver
(`cargo run -q -p gandr -- scripts/agda-deps.gandr`, mise.toml L878). So
scripting.md's adaptation = drop the shared-core-Nushell-vs-project-Rust framing
as the whole story, and state the three-tier reality: (a) shared-core `.nu`
stays foreign Nushell, (b) project gate logic is Rust (`gandr-*-gates`),
(c) project *scripts* (manual build, agda-deps) become Rust then gandr `.gandr`
scripts driven by the `gandr` binary.

---

## 5. The 4 updated Rust rules — verbatim from `wyrd@failed-refactor:docs/workflow/rust.md`

**Rule 1 — `#[repr(transparent)]` single-field structs** (rust.md L23-24):

> * **Single-field structs are transparent.** Every named or tuple struct with
>   exactly one field must carry `#[repr(transparent)]`.
>   An exception requires a concrete layout, ABI, or soundness reason documented
>   in the item's `# Contract`; convenience or omission is not an exception.

**Rule 2 — no bare primitives in crate-defined signatures** + **Rule 3 —
nominal wrapper types** (rust.md L25-29, one bullet):

> * **Crate-defined signatures preserve semantic information.** A function or
>   method defined by a workspace crate must not accept or return a bare
>   primitive value (`bool`, `char`, numeric primitives, or `str`; see the
>   [Rust primitive overview](https://doc.rust-lang.org/rust-by-example/primitives.html)),
>   whether directly or beneath references, pointers, tuples, arrays, slices, or
>   configured generic containers before reaching a nominal type boundary.
>   This applies regardless of visibility to free, const, async, and extern
>   functions; inherent methods; local-trait declarations and defaults; and
>   local-trait implementations.
>   The sole exception is a method implementing a trait defined in an external
>   crate whose required signature contains primitive types.
>   Introduce a nominal domain wrapper rather than a type alias, give each
>   single-field wrapper `#[repr(transparent)]`, and implement the utility traits
>   needed for effective use.
>   Wrappers prevent semantically distinct values from becoming interchangeable
>   and preserve meaning for humans and agents.

**Rule 4 — mandatory `# Termination` blocks on recursion** (rust.md L119-132):

> * `# Termination` is mandatory on every directly or mutually recursive
>   function or method.
>   Use the fixed grammar below; each field needs a concrete explanation, not an
>   assertion that termination is obvious:
>
>   ```rust
>   /// # Termination
>   /// - reason: why recursion is the appropriate control structure.
>   /// - measure: the quantity that strictly decreases on every recursive edge.
>   /// - boundedness: where the finite or well-founded bound comes from.
>   /// - input recursion: none.
>   ```
>
>   `- input recursion: none.` is required everywhere except the model recursive
>   checker in `gandr-core::checker::Rec`, if that implementation is still
>   recursive.
>   That checker may instead name structural descent through the finite checked
>   term, because serving as the direct recursive reference model is its purpose;
>   the defunctionalized machine remains the adversarial-depth path.
>   Tail-call position does not remove this obligation because Rust does not
>   guarantee tail-call optimization; a genuinely iterative implementation is not
>   recursive and needs no termination section.

**Supporting/enforcement context** (rust.md L38-41, L53-61): unbounded recursion
in the interpreter is banned (ADR-47); `wyrd-dylint` "requires
`#[repr(transparent)]` on every single-field struct and rejects project-defined
function or method signatures that expose types from the official primitive
index" and "enforce[s] source-grounded recursive `# Termination` contracts and
reject[s] false `input recursion: none` claims" (L57, L60). These four rules are
the *rationale* face; `wyrd-dylint` (rename → `gandr-dylint`) is the mechanical
enforcer. All four are language/name-neutral and carry verbatim.

---

## 6. mise.toml task inventory

`wyrd@failed-refactor:mise.toml` defines **96 `[tasks.*]`** entries. The string
`wyrd-rust-gates` appears on **41 lines**; of those, **24 are
`cargo run … -p wyrd-rust-gates -- <subcommand>` invocations** (one commented
out, L528), the rest are `cargo nextest run -p wyrd-rust-gates` and
`cargo clippy/dylint --exclude wyrd-rust-gates`. **Every one requires the
crate rename** `wyrd-rust-gates → gandr-*-gates`.

### 6a. The `wyrd-rust-gates` subcommand callsites (mise.toml line → subcommand)

| line | subcommand | owning task | keep/rename/drop |
| --- | --- | --- | --- |
| 236 | `docs-manifest` | `docs:manifest-drift` | keep (rename crate) |
| 248 | `docs-reference` | `docs:reference-integrity` | keep |
| 271 | `iu-pin --workspace-root .` | `iu:check` | keep (IU submodule pin) |
| 295 | `page-balance` | `docs:balance` | keep |
| 509,516 | `contracts --scope crates/gandr-graph` | `test:graph-gates`, `test:adequacy-witnesses` | keep |
| 510,522 | `graph-boundary --workspace-root .` | `test:graph-gates`, `test:graph-boundary` | keep |
| 528 | `default-graph` (COMMENTED OUT) | `test:dep-graph` (commented, L525-529) | resolve: re-enable or drop (wyrd-66jm) |
| 540 | `soundness-oracles --workspace-root .` | `test:soundness-oracles` | keep |
| 547,887 | `options-policy --workspace-root .` | `test:options-policy`, `agda:check` | keep |
| 564 | `mutants snapshot` | `mutants:snapshot` | keep (needs runner, §10) |
| 570,576 | `mutants push` | `mutants:changed-vs-remote`, `mutants:push` | keep |
| 582,588 | `mutants merge` | `mutants:changed-vs-main`, `mutants:merge` | keep |
| 598 | `mutants scheduled --from --to` | `mutants:scheduled` | keep |
| 607 | `mutants clean` | `mutants:clean` | keep |
| 613 | `mutants sweep` | `mutants:sweep` | keep |
| 677,684 | `coverage {check,ratchet}` | `coverage:check:enforce`, `coverage:ratchet` | keep (check currently disabled, §CI) |
| 727 | `fuzz-smoke` | `fuzz:rust-smoke` | keep |
| 767 | `maintenance-range advance` | `maintenance:advance` | keep |
| 833 | `ci-contracts --workflow .github/workflows/ci.yml` | `ci:contracts:workflow` | keep |

### 6b. Task families (keep/rename/drop rollup, 96 tasks)

- **dev:enable / dev:disable** (L48-60) — keep; toggle `.miserc.toml`.
- **setup** (L62-67) — keep; `aube ci` (root + tree-sitter package).
- **grammar:test** (L69-75) — keep; `aube test/run` on `packages/tree-sitter-gandr`.
- **cargo:build / clippy / dylint / doc / doc-check** (L77-231) — keep, rename
  crate refs; `cargo:clippy` and `cargo:dylint` re-export
  `RUSTUP_TOOLCHAIN_NIGHTLY` (§1). `cargo:dylint` (L111-210) drives **6
  `cargo dylint` invocations** (project `wyrd_dylint` lib → rename `gandr_dylint`;
  the upstream example-lint battery of 18 `--lib` rules; the isolated
  `non_local_effect_before_unhandled_error` split for `gandr-core` lib-test per
  wyrd-2a2f; `crate_wide_allow`; `register_lints_warn`). `wyrd-rust-gates` is
  `--exclude`d everywhere (temporary remediation exemption wyrd-i8hq).
- **docs:manifest-drift / reference-integrity / balance** (L233-296) — keep.
- **docs:conflict-markers** (L239-243) — keep; **Nushell** (`.agents/core/scripts`,
  foreign — stays Nushell).
- **core:check / core:update / smoke:wt-core-init** (L251-266) — keep;
  `check-core-pin.nu` + `smoke-wt-core-init.nu` are **Nushell shared-core**.
- **iu:check** (L268-272) — keep.
- **docs:manual, docs:manual:watch, docs:highlight(:spike), docs:chapter:\***
  (~30 tasks, L274-497) — keep; these invoke **project-local Nushell**
  (`docs/manual/tools/build-manual.nu`, `highlight-gandr.nu`) — the §5
  "becomes Rust then gandr" candidates — plus `typst compile`. Also carry
  `wyrd-*` bead IDs in descriptions.
- **test:doc-gates / graph-gates / adequacy-witnesses / graph-boundary /
  soundness-oracles / options-policy / page-balance / tree-sitter-ref /
  coverage-ratchet / gate-parity / scheduled-campaigns** (L499-554, 805-821) —
  keep. `test:dep-graph` (L525-529) is **commented out** — resolve.
- **mutants:\*** (10 tasks, L561-614) — keep; all route through the microVM
  containment driver; require the self-hosted runner (§10).
- **cargo:fmt / fmt-check / nextest / careful-nextest / llvm-cov / no-panic /
  miri** (L619-791) — keep; nightly-gated ones re-export the nightly pin.
- **coverage:check / check:enforce / ratchet** (L666-685) — keep; note
  `coverage:check:enforce` is **commented out inside `coverage:check`** (L671)
  during the failed-refactor remediation.
- **fuzz:\*** (7 tasks, L689-743) — keep; AFL++ campaigns (5 targets:
  lower/parse/check/parity/gates) + `fuzz:rust-smoke` + `fuzz:weekly`.
- **maintenance:weekly / advance / monthly** (L745-771) — keep; scheduled-campaign
  entrypoints; watermark path `$HOME/.cache/wyrd/weekly-success.json` →
  rename `wyrd` segment.
- **treefmt / treefmt:check** (L793-803) — keep; nightly-gated.
- **ci:contracts / ci:contracts:workflow / wrkflw** (L823-841) — keep.
- **gate:merge** (L843-853) — keep; the ordered merge-tier composition.
- **agda:deps / agda:check** (L875-888) — keep; `agda:deps` runs a `.gandr`
  script (the Rust→gandr endpoint); `agda:check` runs `aifix batch agda` twice
  (strict root + declared holey leaf) + `options-policy` sweep.

**Nushell-dependent tasks (summary).** Two classes:
(i) **shared-core, stays Nushell** — `docs:conflict-markers`, `core:check`,
`smoke:wt-core-init` (`.agents/core/scripts/*.nu`);
(ii) **project-local, rewrite target** — every `docs:manual*` / `docs:highlight*`
/ `docs:chapter:{core,glance,surface,examples,appendix}` task calling
`docs/manual/tools/{build-manual,highlight-gandr}.nu`.

---

## 7. prek.toml, .commitlintrc.mts, tsconfig.json, package.json

**`wyrd@failed-refactor:prek.toml`** — RETAIN, rename crate refs.
`default_install_hook_types = [pre-commit, commit-msg, pre-push]` (L7).
Hooks: `prek-validate-config`, `no-machine-local-paths` (core Nushell),
`core-pin`/`iu-pin` (mise run), `docs-conflict-markers`, `treefmt-check`,
`cargo-fmt-check` (globs incl. `.cargo/cargo.toml`, `**/Cargo.toml`, `**/*.rs`,
`rust-toolchain.toml`, `rustfmt.toml`), `docs-manifest-drift`,
`docs-reference-integrity`, `wrkflw-validate`, `commitlint` (commit-msg);
pre-push: **`workflow-push`** = `cargo run -p wyrd-rust-gates -- workflow push`
(L134-141, **rename crate**), `commitlint-push-range`, `signed-commits`
(both core Nushell). No other change.

**`wyrd@failed-refactor:.commitlintrc.mts`** — ADAPT the scope list.
Imports the vendored-core base `./.agents/core/fragments/commitlintrc.base.mts`
(carries conventional-commits doctrine + agent-trailer registry). The
wyrd-specific `makeConfig([...])` scope vocabulary (L9-48) needs curation for
gandr: **drop** `wyrd`; **keep** the `gandr-*` crate scopes
(`gandr-core, gandr-corpus, gandr-data, gandr-ffi, gandr-lsp, gandr-metatheory,
gandr-pipeline, gandr-polygraph, gandr-render-proto, gandr-shell,
gandr-tree-sitter, gandr-tui, gandr-vdc`) and the topical scopes
(`analysis, coverage, crates, drift, edit, fuzz, gandr, incremental-pipeline,
kernel, knowledge, metatheory, mise, nominal, order-maintenance, pipeline,
polygraphs, profiles, proptest, rustdoc, shell, spec, surface, tooling, wt`);
**resolve** whether the reboot keeps a `wt` scope. Add gandr-reboot-only areas
deliberately.

**`wyrd@failed-refactor:tsconfig.json`** — RETAIN verbatim. Its only purpose is
to let the editor LSP resolve `.commitlintrc.mts`'s `.mts` import of the core
fragment under `nodenext` (needs `allowImportingTsExtensions` + `noEmit`),
silencing a spurious TS2307 (wyrd-g0dq). `include: [".commitlintrc.mts"]`. No
name coupling; the "fix" is already applied — carry it so the reboot doesn't
re-hit TS2307.

**`wyrd@failed-refactor:package.json`** — ADAPT. `name: "wyrd"` → gandr;
`private: true`; devDependencies = `@commitlint/cli ^21`,
`@commitlint/config-conventional ^21.0.2`, `@commitlint/types ^21.0.1`,
`@typescript/native-preview 7.0.0-dev.20260521.1` (retain). Note `author` field
is `darinmorrison@gmail.com` (a provenance detail — update to the reboot's
declared author as appropriate).

---

## 8. CI shape

### 8a. `.github/workflows/ci.yml` — job carry list

Triggers: `push` to `main`/`tasks`, `pull_request` (L3-8). The reboot may not
use a `tasks` branch — resolve. YAML anchors live in a `template` job with
`if: false` (L26-168) — a real pattern, keep. Active jobs and whether they carry:

| job | runs (mise task) | carry? |
| --- | --- | --- |
| `aube-build-grammar` | `grammar:test` | yes |
| `cargo-build-crates-stable` | `cargo:build` | yes (stable) |
| `cargo-doc-crates` | `cargo:doc-check` | yes |
| `docs-manifest-drift` | conflict-markers + manifest-drift + reference-integrity + soundness-oracles + doc-gates + page-balance + graph-gates | yes (bundles 7 gates) |
| `project-lint` | `treefmt:check` + `wrkflw` | yes (nightly) |
| `cargo-clippy-crates` | `cargo:clippy` | yes |
| `cargo-dylint-crates` | `cargo:dylint` | yes (nightly) |
| `cargo-nextest-crates` | `cargo:nextest` | yes |
| `cargo-llvm-cov-crates` | `coverage:check` | yes; **note** `coverage:check:enforce` currently no-op (§6b) |
| `cargo-careful-nextest-crates` | `cargo:careful-nextest` | yes (nightly) |
| `cargo-no-panic-smoke` | `cargo:no-panic` | yes |
| `agda-check` | `agda:check` | **`if: false`** (disabled; CI can't pass yet) — carry structure, keep disabled |
| `cargo-miri-test-crates` | `cargo:miri` | **`if: false`** (disabled, wyrd-1bqb: ~2.5h/run minutes-burn) — carry disabled |

AFL++ is **deliberately not a CI job** (comment L429-441) — it is a scheduled
campaign. Cache prefix-keys carry a `wyrd-v2-rust-deps` label (rename), plus
generic `aube-v2`/`rustup-v2` keys (keep). Toolchain env pins → §1.

### 8b. `.github/workflows/scheduled-campaigns.yml` — the runner hazard

- Two crons: **weekly** `17 3 * * 1` (bounded fuzz + changed-code mutation,
  `timeout-minutes: 90`) and **monthly** `29 4 1 * *` (full mutation sweep,
  `timeout-minutes: 540`), plus `repository_dispatch` types
  `weekly-maintenance` / `monthly-maintenance` (L3-11). No branch-selectable
  dispatch/push/PR path (by design, ci.md L61).
- **Both jobs pin `runs-on: [self-hosted, macOS, ARM64, wyrd-maintenance]`**
  (L29-33, L84-88). The `wyrd-maintenance` self-hosted Apple-Silicon runner
  (needed because microsandbox/libkrun requires nested virtualization) **will
  not exist in the gandr reboot** — an unavailable runner "stays visibly queued"
  (ci.md L70). Weekly resolves its commit range via
  `wyrd-rust-gates -- maintenance-range` against a runner-local watermark
  `$HOME/.cache/wyrd/weekly-success.json` (rename).
- **Decision for the coordinator**: (a) rename label → `gandr-maintenance` AND
  provision the runner, (b) keep the file but accept the jobs queue until a
  runner is stood up, or (c) defer/remove the whole workflow until the reboot
  needs scheduled campaigns. The task explicitly flags this as a
  will-not-exist dependency.

---

## 9. typos.toml merge plan

Goal: one gandr `typos.toml` = `wyrd@failed-refactor:typos.toml` ∪ `iu:typos.toml`,
with wyrd machine/tracker specifics rewritten.

**Base = `wyrd@failed-refactor:typos.toml`.**

- **`[files] extend-exclude`** (L15-23): `secrets/`, `treefmt.toml`,
  `zsh/.zcompdump`, `.agents/core/`, `.claude/worktrees/`, `*.agda`, `*.agda-lib`.
  - Keep: `secrets/`, `.agents/core/`, `.claude/worktrees/`, `*.agda`, `*.agda-lib`.
  - **Drop `zsh/.zcompdump`** (machine/dotfiles artifact, not a gandr repo path).
  - Keep `treefmt.toml` exclude (it holds the typos config's own excluded-word
    examples that would self-trip) — verify still needed.
  - **Merge in from `iu`**: `*.typ`, `docs/manual/refs.yml`, `vendor/**`
    (`iu:typos.toml` L16-22). gandr has `*.typ` Track-C sources and a manual
    refs file, so add both; add `vendor/**` if the reboot vendors Agda stdlib.
- **`[default] extend-ignore-re`** (L33): `\bwyrd-[0-9a-z]+\b` **→
  `\bgandr-[0-9a-z]+\b`** (the tracker-ID regex — this is the explicit
  rename); **keep** `\b[0-9a-f]{7,40}\b` (git short-hash guard).
- **`[default.extend-words]`** (L36-63): keep all — they are domain vocabulary,
  not wyrd-specific: `Ket/ket`, `Solum/solum` (gandr language constructs),
  `mis`, `Thm`, `unparseable`, `Yau`, `ratatui`, `edtui` (gandr-tui deps),
  `hom`, `equipments`. **Merge in from `iu`** the additional words: `missable`,
  `nd`, `thm` (lowercase — iu form, add alongside wyrd's `Thm`), `transfor`,
  `transfors` (`iu:typos.toml` L30-42). `equipments` and `hom` are shared
  (identical) — dedupe.
- **`[default.extend-identifiers]`** (L65-69): keep `CertiCoq`, `FoSSaCS`,
  `serie`, `shortcat`. `iu` also has `FoSSaCS` (identical) — dedupe.

**Cross-check / contradiction note**: `wyrd:typos.toml` comment (L61-63) says the
`equipments` entry "same entry exists in the sibling internal-univalence repo" —
confirmed at `iu:typos.toml` L31-33. The two configs agree on `hom`/`equipments`;
`iu` additionally documents that typos has *already* mis-corrected `aks-2015 →
ask-2015`, `hom → home`, `transfors → transforms`, `Nd → And` (H8 failures) —
those `iu` protections should carry into gandr's merged file.

---

## 10. Enumeration: .cargo/*, .config/*, .omp/*, .claude/*, .agents/*

**`.cargo/`**

- `config.toml` — **carry**. `build-dist`/`check-dist` cargo aliases (need
  `+nightly`), `[unstable]`, references `.cargo/config.dist.toml`.
- `config.dist.toml` — **carry**. `[profile.dist]` (LTO fat, `panic =
  immediate-abort`, `build-std` size-optimized, `-Zfmt-debug=none`, …) for
  size-optimized dist builds. No name coupling.
- `mutants.toml` — **adapt**. cargo-mutants correctness/scope config
  (`copy_vcs`, `gitignore`, `copy_target=false`, `test_workspace=true`,
  `test_tool=nextest`, timeout multipliers). The `exclude_re` list (L57-80) is
  **gandr-crate-specific** (gandr-graph survivors from bead wyrd-4gf0) and must
  be re-derived; comments reference `gandr-shell`/`gandr-graph`. Keep mechanism.

**`.config/`**

- `wt.toml` — **carry/adapt**. Worktrunk hooks: `pre-start` (copy-ignored,
  beads-chmod, mise-setup, core-init, iu-build-warmup, beads-pull), `pre-merge`
  (adr-guard, core-pin, `gate-merge = mise run gate:merge`, beads),
  `post-merge` (beads-pull), `step.copy-ignored` excludes. References
  `.agents/core/scripts/*.nu`, `metatheory/upstream/internal-univalence`,
  `.beads`. No literal `wyrd` outside comments; carry, verify submodule paths.

**`.omp/`** (all symlinks — skill aggregators for the OMP harness; carry all)

- `skills/find-best-rust-crates` → `../../.agents/core/skills/…` — carry.
- `skills/find-best-typescript-packages` → core skill — carry.
- `skills/gandr-pro` → `../../crates/gandr-corpus/skills/gandr-pro` — carry
  (already gandr).
- `skills/git-cliff-treefmt-changelog` → core skill — carry.
- `skills/project-coherence-sweep` → `../../.agents/skills/…` — carry.
- `skills/reference-project-convention-drift` → core skill — carry.

**`.claude/`**

- `settings.json` — **carry/adapt**. Hooks `PreCompact`→`bd prime`,
  `SessionStart`→`bd dolt pull; bd prime; check-core-freshness.nu`; worktrunk
  marketplace/plugin; `wt list statusline`. Verified as SUBSET of core base by
  `.agents/conventions.toml`. No `wyrd` literal. Carry.
- `settings.local.json` — **adapt/local**. Permission allowlist (tirith url,
  WebFetch claude.com, nushell evaluate, pctx list_functions). Local-scoped;
  keep or regenerate per environment.
- `skills/*` — six symlinks, identical set to `.omp/skills` — carry.
- `worktrees/` — **transient** (empty; in-repo agent worktrees, foreign
  checkouts per core H13; excluded by treefmt/rumdl/typos). Carry the
  *exclusion*, not the dir.

**`.agents/`**

- `conventions.toml` — **adapt**. `[project] name = "wyrd"` → gandr,
  `languages = ["rust","agda","nushell"]` (reconsider `nushell` as scripts move
  to gandr), `[core] path = .agents/core`, surfaces (docs/configs/settings
  lists — carry with renames), `[tracker] prefix = "wyrd"` → `gandr`. Note the
  `[reference]` section is intentionally omitted ("wyrd is the hub the core was
  extracted from").
- `core/` — **carry**. Vendored agentic-dev core submodule (gitlink, read-only,
  pinned; ADR-2). The whole shared-core system (WORKFLOW/HAZARDS/PRINCIPLES,
  scripts, fragments, base configs). Do not format/modify (H13).
- `skills/project-coherence-sweep/` — **carry**. Project-local skill (the one
  non-symlinked, non-core skill).

---

## 11. Cross-cutting rename ledger (`wyrd` → `gandr`)

Apply uniformly; these are the load-bearing identifiers:

| current | rename to | where |
| --- | --- | --- |
| crate `wyrd-rust-gates` | `gandr-*-gates` (e.g. `gandr-gates`) | mise.toml (~41 lines / 24 subcommand callsites), treefmt.toml rumdl formatters, prek.toml workflow-push, ci.md prose |
| crate `wyrd-dylint` / lib `wyrd_dylint` | `gandr-dylint` / `gandr_dylint` | mise.toml `cargo:clippy`/`cargo:dylint`, ci.yml, rust.md L52-61 |
| runner label `wyrd-maintenance` | `gandr-maintenance` (+ provision runner) | scheduled-campaigns.yml, ci.md L70 |
| tracker prefix `wyrd-` | `gandr-` | typos.toml regex, .agents/conventions.toml, tracker.md, all bead IDs in docs |
| npm `name: "wyrd"` | gandr | package.json |
| commitlint scope `wyrd` | drop / gandr | .commitlintrc.mts |
| CI cache prefix `wyrd-v2-rust-deps` | gandr | ci.yml |
| watermark `~/.cache/wyrd/…` | gandr | scheduled-campaigns.yml, maintenance:advance |
| DoltHub `silvanshade/wyrd-beads` | gandr beads repo | tracker.md |
| notes repo `wyrd-notes` | gandr notes repo | docs.md, review.md, worktrees.md |

---

## 12. Hazards & surprises for the coordinator

1. **Nightly-pin drift (§1)** — the single highest-value finding. `rust-toolchain.toml`
   (2026-07-07) is shadowed by mise `[env]` inside every mise task, and mise+ci
   both still say 2026-05-28. Fix all three; consider a pin-equality gate.
2. **Component-set drift (§1)** — `rust-toolchain.toml` components ≠ CI
   `RUSTUP_COMPONENTS*`; two independently-maintained surfaces.
3. **`scheduled-campaigns.yml` self-hosted runner will not exist (§8b)** — jobs
   queue forever silently; needs an explicit keep/defer/provision decision.
4. **Several gates are temporarily disabled in the failed-refactor and must be
   consciously re-enabled, not silently inherited-as-off**: `coverage:check:enforce`
   (mise.toml L671, commented inside `coverage:check`; and the push tier omits
   `coverage:check` per ci.md L43); `test:dep-graph`/`default-graph` (mise.toml
   L525-529, commented; ci.yml L253-254, commented); `agda-check` and
   `cargo-miri-test-crates` CI jobs (`if: false`). `wyrd-rust-gates` is itself
   `--exclude`d from clippy/dylint (temporary exemption wyrd-i8hq). Inheriting
   these as-is silently ships a weaker gate wall than the docs describe.
5. **`package.json author = darinmorrison@gmail.com`** differs from the repo's
   committing identity — a provenance detail to reconcile at rename time
   (contributor-concern; not editorializing here).
6. **rumdl indirection** — rumdl is a pinned tool (`cargo:rumdl 0.2.29`) but is
   invoked *only* through `wyrd-rust-gates -- rumdl` in treefmt; the direct
   binary is a transitive dependency of the gate crate's rumdl subcommand.
   The crate rename touches the formatter definition too.
7. **`.gitmodules` (adjacent, not in scope but load-bearing)**: two submodules —
   `.agents/core` (agentic-dev core) and `metatheory/upstream/internal-univalence`
   (the IU engine, pinned read-only). Both are structural to the gate wall
   (core-pin, iu-pin) and carry to gandr; flag for the plan even though the
   ticket scoped configs, not `.gitmodules`.
8. **typos already has a track record of H8 corruption** (`aks-2015→ask-2015`,
   `hom→home`, `transfors→transforms`, `Nd→And`, `ratatui→ratatouille`) — the
   merged allowlist (§9) is fidelity-critical, not cosmetic; under-merging it
   risks silent corpus corruption.
