# The gandr specification corpus

The specification corpus authors as **GF abstract-syntax trees**: one `.gfd` file per component (plus the corpus index component `spec-index.gfd`) under `crates/workflow-docs/corpus/`, and the hayagriva `refs.yml` sibling here.
The **normative schema is the GF grammar** (`crates/workflow-docs/grammar/GandrDocs.gf`); the HTML presentation is a concrete syntax (`GandrDocsHtml.gf`) plus the render post-pass.
Validation is the mandatory `checkExpr` lane: every term reference, cite key, cross-reference, and provenance edge must resolve against the generated corpus-wide lexicon — that is the type-check, and there is no separate lint pass.

This README is the corpus map and the content doctrine.
The **authoring discipline** — the `.gfd` surface, the constructor inventory with guidance, the design-language contract pages carry — lives in `docs/workflow/gfd.md` (the routing layer); the fidelity-review protocol lives in `docs/workflow/review.md`.

## Absorption fidelity — the bar

Absorbing a source document (a pre-reboot spec, a research sweep, a session decision record) into a component is **superset-transfer, never summarization** (owner decision `gandr-fid.0`): the source is the floor, not the ceiling.
The acceptance test: **an implementing agent must be able to build the component from the `.gfd` alone, without opening the source tree.**

The register is a **two-register weave**: prose explains, connects, and motivates; structured blocks (tables, typed code, rules, grammars) carry the dense payload.
Every load-bearing artifact lives in a block, and every block gets an introducing prose paragraph.
When density hurts readability, the sanctioned response is to spread out, explain, and link (`TermRef`, `XRef`, `CiteRef`) — never to drop or flatten.
Decision-trail rationale is inlined restated in current terms (pre-reboot record numbers stay banned; their load-bearing content does not).

Every component change **declares its source set** — for absorptions, the ledger row (`docs/research/`); for net-new components, the commissioning bead — and gets the mandatory two-axis fidelity review of `docs/workflow/review.md` §"Documentation fidelity review": zero dropped load-bearing content classes.

## The lanes

Run these from the workspace root (wired as mise tasks):

| Lane        | mise task                  | Effect                                                                                                                                                                                                |
| ----------- | -------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `check-all` | `mise run docs:check`      | Lexicon freshness (the duplicate detector), then the mandatory `checkExpr` lane over every corpus `.gfd`. Any violation fails.                                                                        |
| `build-all` | `mise run docs:build`      | Render every page plus the corpus index as no-JavaScript static HTML into `target/docs-spec/`, carrying the design language. Math and diagram leaves compile to inline SVG via the pinned typst tool. |
| `lexicon`   | (part of the arc)          | Regenerate the corpus-wide `GF` lexicon modules from the `.gfd` files and `refs.yml` — committed derived files; never hand-edit (`--check` is the freshness gate).                                    |
| corpus arc  | `mise run docs:gfd:corpus` | The full arc: uv env, lexicon, grammar compile, `check-all`, `build-all` into `target/docs-spec`, and the crate tests.                                                                                |

## Per-feature component skeleton

Author every feature component in this order — steps 1–5 mirror the Lean reference manual's per-feature rhythm (term-definition intro, titled syntax box, semantics, examples, edge cases); steps 6–9 carry the engineering payload a normative spec needs beyond a manual (`gandr-fid.0`):

1. **Term-definition intro** — a `MkSection` of `ProseBlock`s that _defines_ the feature's terms inline with `TermDef`.
   Define once; reference with `TermRef` everywhere after.
   The lexicon generator enforces define-once across the whole corpus.
2. **Syntax box** — a `GrammarBlock` of `MkProduction` rows (the surface / concrete syntax), optionally a titled `JudgementsBlock` for the judgement forms the feature introduces.
3. **Semantics** — `RuleBlock`s (premises over a conclusion) and `DefinitionBlock`s for the metatheoretic definitions.
   Leaf formulas are typst math in `MathInline` / the `MathRow` leaves; the house prelude (mu-tilde, cut brackets `⟦ ⟧`, inference `frac`) is prepended by the tool — do not restate it.
4. **Examples** — `ExampleBlock`s.
   Use `ExpectCodeBlock` to anticipate a checked example.
   The grammar validates the _shape_ of the expectation; execution gating is deferred until the interpreter runs.
5. **Edge cases** — a `MkSection` of prose for corner cases and non-goals.
6. **Architecture and API surface** — the crate/module homes and the normative typed signatures: `ApiCodeBlock`s (never executed), each introduced by a prose paragraph saying what the shape is _for_.
   Dictionary/decision/mapping content authors as the semantic table payloads, never flattened into prose: **`DecisionTableBlock`** (recorded either/or choices with rationale), **`StagingPlanBlock`** (phase × obligation × gate), **`InventoryBlock`** (maps: module→role, surface→home).
7. **Staging and gates** — the phase ladder with its go/no-go gates, as a `StagingPlanBlock` or a `RegisterBlock`.
8. **Corpus-examples plan** — the named model and pathological example files with their expected outcomes (a table or register; graduates into `ExampleBlock`s with expectations as the examples land).
9. **Open questions / residuals** — a closing `MkSection` recording what is deliberately unresolved; absorption may not silently drop a source's open questions.

Add a `DiagramBlock` (fletcher typst source, with anchor and caption) wherever a commutative or structural picture earns its place, and declare the component's cite keys in the `MkComponent` references list.

### Constructor notes

* Registers — `RegisterBlock` (items with bold leads, `MkItem`) and `PlainRegisterBlock` (`MkPlainItem`), each carrying an `OrderedList`/`UnorderedList` marker.
* `XRef anchor_*` — an inline cross-reference to any declared corpus anchor (component, section, rule, definition, diagram); the `checkExpr` lane rejects unresolved targets and the linearization links it with the target's title as text.
  Use section-granular `XRef`s where the pre-reboot corpus used `§`-anchors — links are the readability answer to density.
* Section status — every `MkSection` carries `InheritSectionStatus` or `WithSectionStatus Status` (the override renders as a status chip; the component's own status is required).
* Provenance — typed `grounds`/`derives` edges on `MkComponent` (anchor constants of other components); they render as the mono provenance line under the title.

`component-vocabulary.gfd` is a worked, self-describing example: it documents the vocabulary using the vocabulary, exercising every block type.

## Lean reference register

The vocabulary borrows Lean 4 reference-manual patterns — titled syntax boxes, term links, stable tags, status attributes, checked examples.
When extending the grammar, consult that register (the fp-lean tutorial voice is reserved for the future gandr manual, not this normative corpus).
