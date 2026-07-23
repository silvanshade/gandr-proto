# Proposal: the GF-native documentation pipeline

> **Status:** proposal, pending owner acceptance (gandr-zuy; epic gandr-2vv).
> **Supersedes:** nothing — it re-homes the doc-pipeline work sequenced before it (visual design gandr-4l9, pacing gandr-aaq, term registry gandr-38l) as follow-ups on the architecture defined here.
> **Date:** 2026-07-22.

## 1. Purpose

Re-architect the gandr documentation pipeline around **Grammatical Framework (GF)** principles: every document is a well-typed tree in an **abstract syntax**; every rendering — HTML today, LaTeX, plain text, agent projections, and natural-language prose in other languages later — is a **linearization**; parsing, type-checking, and tree introspection ride the **GF toolchain** rather than bespoke Rust.

The mandate is structural, not cosmetic: the component vocabulary stops being an XML schema with a renderer attached and becomes a grammar with renderings attached.
The leverage this buys is enumerated where each piece appears: typed documents (§3.4), a runtime-read authoring substrate with a mandatory type-check lane (§3.2), multilingual documentation (§3.3), exact metrics over document structure (§3.5), and a grammar the self-documentation system (gandr-b1d) can inherit (§8).

## 2. What GF is, precisely

GF is **a special-purpose programming language for grammars**, built on Martin-Löf type theory: an abstract syntax is a judgment system — categories (`cat`) as types and constructors (`fun`) as typed constants, including dependent function types — and a concrete syntax assigns each category a linearization type (`lincat`) and each constructor a linearization rule (`lin`).
One abstract syntax, many concrete syntaxes: that is the entire architecture we are adopting. ([grammaticalframework.org](https://www.grammaticalframework.org/); Ranta, _Grammatical Framework: a type-theoretical grammar formalism_, JFP 2004; Ranta, _Type-Theoretical Grammar_, Oxford 1994.)

The pieces this pipeline consumes:

* **The GF compiler** (`gf --make`) type-checks grammars and compiles them to **PGF**, the portable runtime format.
  Available in nixpkgs (`gf`, unstable 2026-06-16).
* **The PGF C runtime** (`libpgf` + `libgu`) parses concrete text to abstract trees, linearizes trees to text, and introspects both.
  All language bindings — Haskell, **Python** (`pgf` 1.1 on PyPI, released 2025-08-08), Java, C# — wrap this one C runtime. ([runtime-api.html](https://www.grammaticalframework.org/doc/runtime-api.html).)
* **The runtime API surface we will use:** `readPGF`; per-language `parse` (lazy, probability-ranked, with start-category override, heuristics, and literal callbacks); `linearize` / `linearizeAll` / `tabularLinearize` / `bracketedLinearize` (brackets carry category _and_ abstract function — phrase-structure recovery); full tree construction and deconstruction (`Expr`, `unpack`/`unApp`/`unStr`/`unAbs`); grammar introspection (`functions`, `categories`, `functionsByCat`, `functionType`); morphological lookup (`lookupMorpho`, `fullFormLexicon`); tree type-checking (`inferExpr`/`checkExpr`).
* **One documented limitation that shapes the design:** the runtime's type checker covers simple types; **dependent types are "still not fully implemented in the current runtime"** (runtime-api.html §"Type Checking Abstract Trees").
  Referential integrity in this pipeline is therefore engineered with _generated lexica and simple types_ (§3.4), not dependent categories.
* **The RGL** (resource grammar library, ~45 languages) is the precedent and the eventual multilingual substrate (§3.3), not a v1 dependency.

Two ecosystem findings from the sweep (2026-07-22), stated because the search record is polluted: **no Rust GF crate exists** on crates.io (`pgf`, `libpgf`, `grammatical` all empty), and no `gf-rust` repository exists in the GrammaticalFramework org — search results claiming otherwise were confabulated; the real runtime bindings are Haskell/Python/Java/C#/TypeScript (`gf-typescript`, 2024).
We roll our own interop either way (§4).

## 3. The re-architecture

### 3.1 The document abstract syntax

The abstract grammar is one GF module — the normative statement of the corpus vocabulary, replacing `model.rs`'s enums as the source of truth.
Two kinds of categories:

**Discourse categories** (already semantic in the current vocabulary): `Component`, `Section`, `Prose`, `Judgements`, `Grammar`, `Rule`, `Definition`, `Example`, `Diagram`, `References`, and the inline family (`Term`, `Cite`, `Ref`, emphases, code spans, math).

**Payload categories** (the GF move): the current vocabulary's `table`, `list`, and code blocks are _presentational_ constructors.
The re-architecture names the semantic payloads the corpus doctrine requires — `DecisionTable` (recorded either/or choices with rationale), `StagingPlan` / `GateTable` (phase × obligation × gate), `Inventory` (maps: module→role, surface→home), `Register` (enumerated obligations, open questions, residual seams), `Signature` (normative API surface — today's `code[role=api]`).
Honest provenance: these categories derive from the README skeleton's payload doctrine more than from current usage — decision rationale today lives mostly in prose, and two of the corpus's largest maps (core-ir's realization and staged-extension tables) are authored as aligned-column `code[language=text]` blocks, i.e. `Inventory`/`StagingPlan` content the presentational schema had no slot for.
Migration's semantic annotation is therefore **re-classification, not ceremony** (§7).
Tables and lists become **linearization choices** of these categories (a `DecisionTable` linearizes to `<table>` in HTML, to a typed tuple structure in an agent projection), not abstract constructors.
This is the "huge leverage" of the GF discipline: semantics authored once, presentation derived.

Constructor discipline, stated now so the grammar stays honest: every constructor is total on its category; text payloads (code, math source, mermaid) are `Str`-valued leaves, never grammar; attributes of the current schema (status, ids, anchors) are constructor arguments of dedicated categories (`Status`, `Anchor`), so the grammar — not a validator — fixes their domains.

### 3.2 The authoring representation (open by owner directive)

**The authoring surface is a free choice.** Linking, highlighting, and styling are linearization-side concerns — they operate on typed trees, so the surface authors type can be swapped for whatever interops best with GF (owner directive 2026-07-22, explicitly overriding the earlier XML-first framing).

The options, honestly weighed:

| Option                                        | Shape                                                                                                          | Cost                                                                                                                                                               | Verdict                                      |
| --------------------------------------------- | -------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------- |
| **A — XML-faithful concrete syntax**          | The corpus's tags linearize the grammar; PGF parses the corpus unchanged                                       | The plan's largest technical risk: GF lexing/parsing of tag-balanced, attribute-laden, mixed inline text is a stress case whose failure mode arrives mid-migration | **Dropped** (superseded by B′)               |
| **B — designed s-expression concrete syntax** | A purpose-built authoring sugar                                                                                | One concrete module to design and maintain                                                                                                                         | Deferred as pure sugar (never touches trees) |
| **B′ — raw abstract syntax via `readExpr`**   | Documents _are_ GF expression trees (`Component ... (Section ...)`), read by a shipped, tested runtime feature | Zero authoring grammar; every document is type-checked at the mandatory `checkExpr` lane; generators (migration translator, agents) emit it trivially              | **Adopted for v1**                           |
| **C — Rust-side parser feeding trees**        | Any surface, bespoke parser                                                                                    | Costs the free parser                                                                                                                                              | Documented fallback (§9 R2)                  |
| **D — Markdown-flavored**                     | Informal grammar, significant whitespace                                                                       | Worst possible GF fit                                                                                                                                              | Declined                                     |

**Recommendation: B′.** The authoring risk collapses to `readExpr`; migration rides the existing `workflow-docs` XML parser (tree construction in Rust, then a deterministic B′ printer — the printer _is_ the formatter, so canonicalization is free); corpus files become `docs/spec/*.gfd`.
Verbosity is the honest cost, shown honestly: inline structure is explicit, `Cons`-nested constructors — `Prose (ConsInline (Txt "…") (ConsInline (TermRef term_cut) BaseInline))` (PGF expression syntax has no list literals) — verbose for humans, natural for agents and generators.
A future prose-sugar lane (option B) restores fluid authoring without touching the tree layer.
Payload text (code, math, mermaid) is `Str` literals — the exact case GF handles natively.
One precision, spike-verified (2026-07-22): `readExpr` reads **untyped** — validation is the pipeline's explicit `checkExpr` lane per document (§3.4).
`checkExpr` errors are message-only (no source offsets), a regression from today's file:line diagnostics: the B′ reader/printer is Rust-side, so the pipeline tracks source spans itself and reports tree-path + file:line — a named, buildable cost owned by us, not by GF.

### 3.3 Linearizations

The HTML rendering is **one concrete syntax among several**, which is the point:

* **HTML** (v1): emits the same structured, class-hooked HTML the design language (gandr-4l9) will style.
  Semantic payload categories linearize to their presentational devices here.
* **LaTeX/PDF** (later): a second concrete syntax; book-quality output becomes a grammar task, not a renderer rewrite.
* **Plain text / agent projection** (later): the corpus served to agents as structured text — the authoring substrate for agent-facing docs.
* **Markdown** (near-term, owner directive 2026-07-22): workflow docs (`docs/workflow/*.md`) are future `.gfd` citizens — authored as trees to get the same semantic analysis and consistency checking as specs, with the repo-facing Markdown **generated** by a Markdown concrete syntax, so readers and agents consume plain Markdown without learning GFD.
  Design constraints this sets now: the abstract doc model must cover workflow-doc shapes (a subset of the component vocabulary: headed prose, lists, tables, intra-repo links); the Markdown linearization must be deterministic and diff-minimal because generated files are committed (the derived-file gate pattern — CI checks `generated == committed`, exactly as `docs/spec/refs.yml` works today); specs and workflow docs share the lexica, so cross-links between them become typed refs.
* **Natural-language prose** (research direction, §8): doc content authored against the abstract syntax linearizes into multiple natural languages — **automatic translation of the documentation corpus**, the headline win the owner named, and the connection to the self-documentation epic (gandr-b1d).
  Ranta's _Type-Theoretical Grammar_ is the foundational text; the RGL supplies per-language surface structure when this lane opens.

### 3.4 Validation as grammar

The current validator's checks become grammatical facts:

* **Terms.** Defined terms are constants in a _generated_ lexicon module (`fun term_data_declaration : Term`); referencing a term is using the constant, so an unresolvable reference fails the `checkExpr` lane (`PGFError: Unknown function` — spike-verified) and define-once is module uniqueness.
  The term registry (gandr-38l) is exactly this lexicon plus its index linearization — **the dangling `#term-*` link class is eliminated by construction**, not patched.
* **Citations.** `refs.yml` generates a cite-key lexicon; unknown keys fail `checkExpr`.
* **Cross-references.** Section anchors generate per-component lexica; `Ref` constructors take anchor constants, so broken refs fail `checkExpr`.
* **Provenance edges.** The corpus index generates a component-id lexicon; `grounds`/`derives` edges take component constants, so dangling provenance fails `checkExpr` (the current validator's `unresolved-provenance` check is preserved, not dropped).
* **Id uniqueness.** The lexicon generator sees every id of every kind corpus-wide at generation time — it is the duplicate detector, failing generation on any collision (the current single-namespace `record_id` check has exactly one home: here).
* **Well-formedness.** The simple-type checker (`checkExpr`) suffices end-to-end — no dependent types needed by design (§2's runtime limitation is engineered around, not hit).

Two things worth saying plainly.
**Validation moved, it did not shrink:** for a simple-typed signature, abstract-syntax membership (`checkExpr`) and CFG grammaticality coincide, and `checkExpr` additionally performs name resolution — the lane is strictly stronger than the CFG parse it replaces.
And the lane is **mandatory**: the runtime linearizes trees containing unknown functions by printing the function name into the output, so any path that skips `checkExpr` leaks constructor names into rendered HTML — no document reaches `linearize` unchecked (§3.6).

The residual validator shrinks to what grammar cannot express: BLAKE3 manifest drift, conflict markers, reference-integrity across files — the existing gates, unchanged.

### 3.5 Metrics over trees

The pacing/linguistic program (gandr-aaq) computes over abstract trees — a strict upgrade over surface heuristics:

* **Exact by construction:** emphasis density (bold/italic are abstract constructors — counting them is a tree walk, not a regex); block-mix and weave compliance (payload blocks per section, prose-per-block ratios); section shape (paragraph and sentence counts from `Prose` structure).
* **On linearized prose:** sentence-length distribution (mean/median/p90/max/variance — the rhythm signal), paragraph word counts, Flesch/Fog-style formulas with the documented technical-corpus caveats.
* **Clause-level (the application-grammar lane):** prose parsed into `RGL` trees gives the doctrine's sentence-level quantities as tree walks — clauses per sentence, embedding depth, passives, nominalizations — _with coverage reporting_ (every metric from this lane is reported with its parse-coverage percentage).
  The grammar is a **domain application grammar**, not wide-coverage English: an `RGL` Lang subset plus a generated domain lexicon (the docs lexicon's display texts plus coined terms as declared lexemes) — the configuration `GF` is designed for (small search space, domain terms first-class) and the remedy for both spike failures (5.6% wide-coverage coverage; the debugger-verified libpgf C-stack crash on 65k-lemma ambiguity).
  Its purpose is **mechanically-assisted revision**: findings name the construction and locate the split point, a deterministic measure → locate → rewrite → re-measure loop for the authoring agent.
  Posture: coverage reported, never gated; structural findings fire only where prose already fails the pacing doctrine (docs/workflow/specs.md §"The sentence-level twin").
  The build is gandr-739 (owner decision, 2026-07-23).

### 3.6 What stays outside GF

GF grammars own document _structure_; they do not replace: typst math → SVG (a payload transform on `Str` leaves, kept); mermaid diagrams (same); the bibliography machinery (`refs.yml` → lexicon generation _reuses_ it); BLAKE3/gate harness.
The pipeline is: author → `readExpr` → `checkExpr` → payload transforms → linearize → page.

## 4. Interop decision

Three routes weighed against the verified ecosystem (§2):

| Route                         | Buildout                                                                              | Env complexity                                                                                                                                                                                                        | Maintenance        | Verdict                       |
| ----------------------------- | ------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------ | ----------------------------- |
| **PyO3 → `pgf` (Python API)** | Thin: PyO3 glue over a maintained upstream binding                                    | Moderate: embedded Python + `uv add pgf`; the 1.1 wheels (mac arm/x86, manylinux, musllinux) **bundle** `libpgf`/`libgu` — no C-runtime install on CI platforms; sdist compile only where wheels are absent (Windows) | Upstream (gf-core) | **Adopt** (owner's lean)      |
| **Direct C FFI → `libpgf`**   | Real: bindgen + a safe wrapper (~1–2k LOC) over `pgf.h`/`gu`, unsafe-correctness risk | Lowest at runtime: one native lib                                                                                                                                                                                     | **Ours**           | Documented fallback           |
| **Existing Rust crate**       | —                                                                                     | —                                                                                                                                                                                                                     | —                  | **Does not exist** (verified) |

**Decision: PyO3**, behind a `GfRuntime` trait whose surface is exactly §2's used API (`read_pgf`, `parse`, `linearize`/`bracketed`, expr construct/deconstruct, `function_type`/`functions_by_cat`, `check_expr`).
The trait isolates the backend: if Python-in-the-build proves intolerable in CI, the C-FFI backend slots in without touching the pipeline.
The C-runtime install is a **shared, unavoidable cost of both routes** — PyO3's marginal cost is only the Python env, C FFI's marginal cost is binding code we must then trust.
The trait is also the seam for the long-term **pure-Rust internalization** direction: [internalizing-gf.md](internalizing-gf.md) records what that would require and the observation log the PoC must feed.

**Environment provisioning** (the PoC's first gate): `gf` compiler from the official release `.pkg`/`.deb` (nixpkgs' gf derivation is broken on darwin-aarch64 — spike-verified); Python + `uv` (uv via mise; the interop crate carries its own `pyproject.toml`/uv config — the owner's stated preference); `uv add pgf` pinned (1.1) — on macOS/Linux the wheel bundles the runtime, so no `libpgf` build task is needed (building the C runtime from gf-core source is the documented contingency for wheel-less platforms).
No Haskell toolchain enters the build.

## 5. Greenfield crate plan

> **As landed (gandr-5n6, 2026-07-23):** the plan below is the as-built layout with two owner-directed changes at the migration's retirement step: the crate is **`crates/workflow-docs`** (the new pipeline inherited the canonical name; the legacy renderer it replaced was deleted), and the `rt.rs`/`sexp.rs` interop split into **`crates/workflow-grammatical-framework`** (the pyo3 quarantine and the physical internalization seam, with the uv project beside it).
> The legacy parser/renderer machinery retired with the XML corpus; the bibliography, typst leaf compiler, references renderer, and the prose document classes moved into the renamed crate.

One new crate, **`crates/gf-docs`**, beside (not inside) `workflow-docs`:

```text
crates/gf-docs/
  grammar/            # the .gf sources — the normative corpus grammar
    GandrDocs.gf          # abstract: categories + constructors (§3.1)
    GandrDocsHtml.gf      # concrete: the HTML linearization (§3.3)
    lexica/               # generated: Term/CiteKey/Anchor modules (§3.4)
  corpus/             # the corpus as .gfd trees (B′ authoring substrate, §3.2)
  src/
    rt.rs             # GfRuntime trait + PyO3 backend
    pipeline.rs       # read → validate → payload transforms → linearize
    lexicon.rs        # lexica generation (refs.yml, term scan, anchors)
    migrate.rs        # XML (workflow-docs parse) → trees → B′ printer
    metrics.rs        # tree-walk metrics (§3.5)
    main.rs           # CLI: check / build / lexicon / migrate / metrics
  pyproject.toml      # uv env: pinned pgf
```

Reused from `workflow-docs` (imported or lifted, per fit): `refs.yml`/bibliography handling, the typst/mermaid payload transforms, gate-harness conventions.
The old pipeline stays the corpus's production renderer until §7's migration gate flips.

## 6. Proof of concept

Scope (gandr-wrs, after this proposal's acceptance):

1. Env provisioning per §4 — **gate 0: `uv run python -c "import pgf; pgf.readPGF(...)"` works in CI shape.**
2. The §3.1 grammar at the **full block inventory of the chosen conversion target** — `component-vocabulary.xml` exercises `Definition`, `Grammar`, `Judgements` (math leaves), `Rule` (premise/conclusion math), `Table`, `List`, api-role `Code`, `Diagram` (typst source + cites), `Example` (expect-output), inline math, and cite/ref/term inlines, so the PoC grammar covers all of it (nothing the file authors may fail to convert — the fid fidelity bar applies to the PoC itself).
3. The HTML concrete syntax for that inventory; authoring rides `readExpr` on B′ text (§3.2) — no authoring grammar to build in the PoC.
4. **One real component rendered end-to-end:** the PoC translator (old XML parse → trees → B′ print) converts `component-vocabulary.xml` (small, self-describing, exercises the subset); the new pipeline reads the `.gfd` and renders an HTML page; content equivalence against the current page checked section-by-section.
5. Term-registry seed: a generated `Term` lexicon; a dangling term reference **fails `checkExpr`** (the negative test is the point).
6. Gates: `cargo nextest -p gandr-workflow-docs`, `mise run docs:build`-equivalent for the PoC page.

**PoC acceptance:** the owner inspects the rendered page; equivalence holds; the negative test fires; sign-off unblocks migration (gandr-5n6).

## 7. Migration story

After PoC sign-off: (1) mechanical translation of the ten components — the old pipeline's own parser produces their trees; the translator emits B′ `.gfd` files (payload categories get their semantic annotation at this step, human-checked per component — **re-classification, not ceremony** (§3.1): the plain-text map blocks in core-ir and the prose-carried decision rationale move into `Inventory`/`StagingPlan`/`DecisionTable`; the corpus is ten documents, this is bounded, and per-component acceptance is the dual-run gate); (2) dual-run: old renderer and new linearization both build every page; section-level content comparison is the migration gate; (3) `docs:check` re-points at the GF pipeline, and the `.gfd` tooling surface is wired — treefmt scope, spell-check, conflict-marker and reference-integrity gates all re-point at `docs/spec/*.gfd`; (4) `workflow-docs`'s renderer retires; its reusable pieces land in `gf-docs`.
No big-bang: the old pipeline stays until (2) is green for every component.

## 8. Follow-ups riding this architecture

* **gandr-38l term registry** — the generated lexicon + its index linearization (§3.4); lands with migration.
* **gandr-aaq pacing** — metrics over trees (§3.5); the recursion-surface revision is re-authored in the new pipeline post-migration, measured before/after.
* **gandr-4l9 visual design** — styles the HTML linearization (Tufte-grade typography/layout/color; the earlier Tufte/Butterick research carries over unchanged).
* **gandr-b1d self-documentation** — doc objects as gandr-language attributes (the wyrd entity-attributes layer) serialize to this same abstract syntax; multilingual linearization is the shared endgame (§3.3).
* **Workflow docs as `.gfd`** — the Markdown-linearization pattern of §3.3 applied to `docs/workflow/` after migration (tracked in its own bead).

## 9. Risks and mitigations

* **R1 — Env fragility** (Python + native libs in the doc build).
  Mitigation: gate 0 first; `GfRuntime` trait seam to the C-FFI fallback; uv/nix pins; pgf version pinned and vendored-build documented.
  **Spike-verified 2026-07-22:** gf 3.12 ships a macOS-arm `.pkg` whose payload extracts without system install and bundles `libpgf` + headers (the nixpkgs gf derivation is broken on darwin-aarch64 — the pkg is the macOS path, the `.deb` the CI path); the `pgf` 1.1 wheels **bundle the C runtime** (mac arm/x86, manylinux, musllinux — verified by wheel inspection), so `uv add pgf` needs no C-runtime install on CI platforms; `readPGF`/`readExpr`/`linearize`/`inferExpr` all work from the uv env.
  Residual: the sdist path (Windows, or a pinned source build) compiles `pypgf.c` against a preinstalled runtime (`EXTRA_INCLUDE_DIRS`/`EXTRA_LIB_DIRS`) — the contingency, not the plan.
* **R2 — B′ authoring ergonomics** (verbose constructor trees for hand-authored prose).
  Mitigation: the corpus is agent-authored and generator-emitted; option-B sugar is a planned pure-surface addition; if ergonomics fail before sugar lands, §3.2's option C (bespoke parser over any surface, trees unchanged) isolates the pain to the parser layer.
* **R3 — Category-design churn** (payload categories §3.1 wrong on first pass).
  Mitigation: one .gf module, cheap to revise pre-migration; corpus is ten documents; the audit grounding is already done.
* **R4 — RGL English coverage** on technical prose (§3.5's staged lane).
  Mitigation: coverage reporting built into the lane; lane is optional; v1 metrics need nothing from it.
  **Outcome (gandr-aaq spike, 2026-07-23): the risk materialized — 5.6% coverage and a debugger-verified libpgf C-stack segfault on a 24-token ambiguous sentence; the lane stays staged (§3.5, gandr-739).**
* **R5 — `readExpr`/`checkExpr` performance and lexicon-regeneration cost** in the `docs:check` inner loop (no XML is PGF-parsed under B′; the live costs are the runtime reader/checker on ~10–60 KB `.gfd` files plus `gf --make` on regenerated lexica).
  Mitigation: measure both in the PoC; lexica regenerate only on lexicon-affecting changes (content-hash gated).

## 10. References

* GF: [grammaticalframework.org](https://www.grammaticalframework.org/) (site, book, tutorial); [runtime-api.html](https://www.grammaticalframework.org/doc/runtime-api.html) (bindings + the dependent-types caveat); [gf-core](https://github.com/GrammaticalFramework/gf-core) (compiler + C runtime + `src/runtime/python`); [gf-rgl](https://github.com/GrammaticalFramework/gf-rgl).
* Ranta: _Grammatical Framework: a type-theoretical grammar formalism_ (JFP 2004); _Type-Theoretical Grammar_ (Oxford, 1994); Angelov & Ranta et al., _Abstract Syntax as Interlingua_ (Computational Linguistics, 2020).
* Lineage this proposal absorbs (wyrd decision records, named without record numbers per the corpus reference gate): the Smalltalk/Lisp-machine introspective environment; content-addressing as a third identity discipline; comments as the literate vehicle with doc-object semantics deferred; the typed attribute layer whose named consumers include doc objects; gandr-b1d (self-documentation, absorbing wyrd-31yx).
* Sequenced follow-ups carry their own references (Tufte/Bringhurst/Butterick for gandr-4l9; readability literature for gandr-aaq).
