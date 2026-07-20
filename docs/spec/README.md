# Authoring the gandr specification corpus

This directory holds the gandr specification corpus: a central index (`index.xml`), one component file per feature area alongside it, and a `refs.yml` sibling (hayagriva).
The corpus is authored in a custom XML component vocabulary and validated by the doc tool, `gandr-workflow-docs`.

The normative schema is the Rust model in `crates/workflow-docs/src/model.rs`.
This README is the _authoring discipline_: the shape a well-formed feature component takes and the tool commands that keep it honest.

## The doc tool

Run these from the workspace root (they are also wired as mise tasks):

| Command | mise task                        | Effect                                                                                                                                                           |
| ------- | -------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `check` | `mise run docs:check`            | Parse **is** validate: ID uniqueness, define-once, term / cite / provenance resolution, status presence. Any violation fails.                                    |
| `build` | `mise run docs:build`            | Render no-JavaScript static HTML into `target/docs-spec/` (only when `check` is clean). Math and diagram leaves compile to inline SVG via the pinned typst tool. |
| `fmt`   | `treefmt` (formatter `docs-xml`) | Canonical XML formatting, idempotent.                                                                                                                            |

`parse = validate`: there is no separate lint pass.
A file that yields any diagnostic produces no page.

## Per-feature component skeleton

Author every feature component in this order — it mirrors the Lean reference manual's per-feature rhythm (term-definition intro, titled syntax box, semantics, examples, edge cases):

1. **Term-definition intro** — a `<section>` of `<prose>` that _defines_ the feature's terms inline with `<term-def key="...">`.
   Define once; reference with `<term key="..."/>` everywhere after.
   The validator enforces define-once across the whole corpus.
2. **Syntax box** — a `<grammar>` block of `<production>` rows (the surface / concrete syntax), optionally a titled `<judgements>` block for the judgement forms the feature introduces.
3. **Semantics** — `<rule>` blocks (premises over a conclusion, rendered as an inference tree) and `<definition>` blocks for the metatheoretic definitions.
   Leaf formulas are typst math in `<math>` / `<premise>` / `<conclusion>`; the house prelude (mu-tilde, cut brackets `⟦ ⟧`, inference `frac`) is prepended by the tool — do not restate it.
4. **Examples** — `<example>` blocks.
   Use `<code>` with `expect-output` / `expect-error` to anticipate a checked example.
   The tool validates the _shape_ of the expectation; execution gating is deferred until the interpreter runs.
5. **Edge cases** — a closing `<section>` of `<prose>` for corner cases and non-goals.

Add a `<diagram>` (fletcher typst source, carrying `id` / `caption` and optional `cites`) wherever a commutative or structural picture earns its place, and a `<references>` block declaring the component's cite keys.

### Required attributes and edges

* Every `<component>` carries a **required** `status`: `built | partial | adopted-unbuilt | design-pass | dormant`.
* Record typed provenance with `grounds="..."` / `derives="..."` on the component root (space-separated component ids); the validator resolves them.
* Cite keys (`<cite key="..."/>`) must resolve in `refs.yml`, whose keys must in turn be ids present in `docs/research/bibliography-v2.md`.

`component-vocabulary.xml` is a worked, self-describing example: it documents the vocabulary using the vocabulary, exercising every block type.

## Lean reference register

The vocabulary borrows Lean 4 reference-manual patterns — titled syntax boxes, term links, stable tags, status attributes, checked examples.
When extending the schema, consult that register (the fp-lean tutorial voice is reserved for the future gandr manual, not this normative corpus).
