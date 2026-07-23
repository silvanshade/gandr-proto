# Internalizing GF: the long-term pure-Rust direction

> **Status:** standing note (related: gandr-2vv epic; proposal-docs-gf-pipeline.md §4).
> **Date:** 2026-07-22.

## Why this document exists

The owner's stated direction: if the GF-native documentation pipeline (gandr-2vv) proves out, gandr will likely **internalize GF as a pure-Rust implementation maintained locally** — the same pattern as the graph-stack internalization (`gandr-graph` over petgraph and friends).
This document is the living record of what that would require.
It is **not** a commitment and not scheduled work; it exists so the PoC and migration record the facts an internalization decision will need.

## What "internalize GF" decomposes into

GF is two systems with very different internalization costs:

**1.** **The PGF runtime (small, well-bounded) — the realistic target.**

* A PGF **binary-format reader** (the compiled grammar image; format lives in gf-core's `src/runtime/c` and the Haskell `PGF2` serializers — undocumented on paper, so the reader is written against source + fixtures, with format-version drift as the standing risk).
* A **parser** for concrete syntax: the C runtime implements a continuation-passing chart parser over the grammar's PMCFG, probability-ranked, lazy — a known-algorithm reimplementation, but subtle (packed forests, literal callbacks, heuristic pruning).
* A **linearizer**: evaluation of linearization rules over `lincat` records — parameter tables, `pre`/`bind` token glue, variants, discontinuous constituents (bracketed linearization).
* A **tree checker** for simple types (the runtime's documented dependent-type limitation matches what our pipeline needs).
* Morpho-lexicon access (full-form tries) — needed only if the RGL lane (proposal §3.5's clause-level metrics) opens.

**2.** **The GF compiler (large) — not an initial target.** Grammar source parsing, the module system (multiple inheritance, parametrized modules, `open`/`interface`/`instance`), dependent type checking, PMCFG conversion and optimization.
Internalizing this is a research-scale project; the Haskell `gf` compiler stays the grammar-build tool for the foreseeable horizon, exactly as cargo-adjacent codegens stay external elsewhere.

## The seam we are already building

`workflow-grammatical-framework`'s `GfRuntime` trait (proposal §4) is deliberately the internalization surface: `read_pgf`, `parse`, `linearize`/`bracketed_linearize`, expr construct/deconstruct, `function_type`/`functions_by_cat`, `check_expr`.
A pure-Rust runtime becomes a third backend beside PyO3 and the documented C-FFI fallback — swappable without touching the pipeline.
**Design rule for all gandr-2vv work: nothing outside `rt.rs` (in `workflow-grammatical-framework`) may know which backend is live.**

## Observation log (maintained during PoC and migration)

Facts to record here as they are learned — each is something an internalization decision needs and a PoC will not teach us twice:

* The exact runtime API surface the docs pipeline actually exercises (vs. the full binding surface).
* PGF binary format version observed, and where the version check lives in the C runtime.
* Parse performance on real corpus documents (file sizes, timings) — bounds the reimplementation's performance bar.
* Any runtime behavior we depend on that is _not_ in the documented API (error shapes, probability semantics, bracket ids).
* Env pain actually encountered (the case for/against internalization is partly an operational one).

Entries so far (env spike, 2026-07-22):

* **Distribution:** gf 3.12 macOS-arm `.pkg` payload extracts without system install and bundles the full C runtime (`libpgf.a`/`libgu.a`, dylibs, headers, pkgconfig) — a static-link path exists (no `DYLD_*` needed). nixpkgs `gf` is broken on darwin-aarch64 (g++ missing in its build env); the pkg is the macOS path, `.deb` the Linux/CI path.
* **`readExpr` is untyped:** unknown function names and ill-typed applications parse fine; rejection lives in `inferExpr`/`checkExpr` (`PGFError: Unknown function "…"`; `TypeError: expected … but … is inferred` [sic] — upstream error-text typo worth noting for a reimplementation that aims for bug-for-bug compatibility tests).
* **Grammar-authoring gotchas:** `String` linearization arguments are `{s : Str}` records (use `x.s`); list categories need explicit `cat [C];` and their lincat is the derived record `{s : …}` (`Base`/`Cons` linearizations build records).
* **API surface exercised:** `readPGF`, `languages`, `readExpr`, `linearize`, `functionType`, `functionsByCat`, `inferExpr` — all present and working in the Python binding (pgf 1.1) against libpgf from the same 3.12 release.
* **Distribution (binding):** the pgf 1.1 PyPI wheels **bundle** `libpgf`/`libgu` (mac arm/x86, manylinux, musllinux — verified by wheel inspection); only the sdist path (Windows, pinned source builds) compiles `pypgf.c` against a preinstalled runtime.

Entries (bindings-surface verification, 2026-07-23, pgf 1.1 against the compiled corpus lexicon PGF):

* **`PGF.functionsByCat(cat)`** returns the category's function names as plain strings (exercised for `Term`/`Anchor` — the metrics lane's lexicon-view source).
* **`Concr.tabularLinearize(expr)`** returns the record-field map (`{key, text}` for `Term`, `{id, title}` for `Anchor`) — display texts come from the compiled grammar itself, so no consumer ever parses the generated `.gf` modules.
* **`Expr.unpack()`** returns a plain `str` for string literals and `(head, [arg…])` for applications (nullary constructors included), so one `pgf.Expr` extracts recursively into a `Rust` tree in a single boundary crossing (`FromPyObject` on the crate tree type, the vkpp wrapper pattern — `rt::GfRuntime::read_tree`).
  The extracted shapes match the retired house reader's exactly: the lexicon lane regenerates byte-identical modules through the runtime path.
* **`Concr.parse`** requires an explicit start category for grammars without one (the corpus grammar has none — expected; the RGL lane carries its own).
* **The C parser is unhardened against deep generated-category recursion** (2026-07-23, pgf 1.1 wheel): parsing a 24-token ambiguous sentence (`"this is completion over the convergent slice of a polygraph presentation , and it is the optimization-time twin of the runtime matching modulo stance"`) against a Lang+DictEng English `PGF` segfaults the process — the debugger shows `pgf_process_generated_cat` recursing past one million frames (`gu_map_find_default`/`gu_map_lookup` in the cycle) until the C stack exhausts.
  Short inputs and single unknown tokens fail cleanly (`ParseError`); the crash is scale-dependent, independent of the `literal=Symb` flag, and reproduces through any binding (it is the shared runtime, not the `Python` glue).
  Any future parse lane needs an input-length/complexity guard or chunked parsing.
  **Update (same day):** the crash signature is _unknown tokens deep inside structurally complex input_ (a relative-clause+coordination sentence with one unlexicalized token crashes the Lang+small-domain-lexicon grammar the same way).
  The parse lane's guard is therefore a pre-parse unknown-token check, not just a length cap.
* **`Concr.lookupMorpho` and `Concr.fullFormLexicon` under-cover structural words** (2026-07-23, pgf 1.1): function words the parser accepts (`is`, `an`, `at`, modals) are absent from both `API`s — the parser itself is the only complete oracle for "is this token known".
  The parse lane's unknown-token guard therefore combines the full-form lexicon with a documented structural-word whitelist.

## Decision triggers

Open an internalization bead when any of: (a) the pipeline is migrated and GF is load-bearing for the corpus; (b) the Python/libpgf env cost measurably hurts (CI time, contributor setup); (c) a needed runtime capability is unreachable through the C API.
Until then this document only accrues the observation log.
