# Authoring the gandr specification corpus

This directory holds the gandr specification corpus: a central index (`index.xml`), one component file per feature area alongside it, and a `refs.yml` sibling (hayagriva).
The corpus is authored in a custom XML component vocabulary and validated by the doc tool, `gandr-workflow-docs`.

The normative schema is the Rust model in `crates/workflow-docs/src/model.rs`.
This README is the _authoring discipline_: the shape a well-formed feature component takes and the tool commands that keep it honest.

## Absorption fidelity — the bar

Absorbing a source document (a pre-reboot spec, a research sweep, a session decision record) into a component is **superset-transfer, never summarization** (owner decision `gandr-fid.0`): the source is the floor, not the ceiling.
The acceptance test: **an implementing agent must be able to build the component from the XML alone, without opening the source tree.**

The register is a **two-register weave**: prose explains, connects, and motivates; structured blocks (tables, typed code, rules, grammars) carry the dense payload.
Every load-bearing artifact lives in a block, and every block gets an introducing prose paragraph.
When density hurts readability, the sanctioned response is to spread out, explain, and link (`<term>`, `<ref>`, `<cite>`) — never to drop or flatten.
Decision-trail rationale is inlined restated in current terms (pre-reboot record numbers stay banned; their load-bearing content does not).

Every component change **declares its source set** — for absorptions, the ledger row (`docs/research/`); for net-new components, the commissioning bead — and gets the mandatory two-axis fidelity review of `docs/workflow/review.md` §"Documentation fidelity review": zero dropped load-bearing content classes.

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

Author every feature component in this order — steps 1–5 mirror the Lean reference manual's per-feature rhythm (term-definition intro, titled syntax box, semantics, examples, edge cases); steps 6–9 carry the engineering payload a normative spec needs beyond a manual (`gandr-fid.0`):

1. **Term-definition intro** — a `<section>` of `<prose>` that _defines_ the feature's terms inline with `<term-def key="...">`.
   Define once; reference with `<term key="..."/>` everywhere after.
   The validator enforces define-once across the whole corpus.
2. **Syntax box** — a `<grammar>` block of `<production>` rows (the surface / concrete syntax), optionally a titled `<judgements>` block for the judgement forms the feature introduces.
3. **Semantics** — `<rule>` blocks (premises over a conclusion, rendered as an inference tree) and `<definition>` blocks for the metatheoretic definitions.
   Leaf formulas are typst math in `<math>` / `<premise>` / `<conclusion>`; the house prelude (mu-tilde, cut brackets `⟦ ⟧`, inference `frac`) is prepended by the tool — do not restate it.
4. **Examples** — `<example>` blocks.
   Use `<code>` with `expect-output` / `expect-error` to anticipate a checked example.
   The tool validates the _shape_ of the expectation; execution gating is deferred until the interpreter runs.
5. **Edge cases** — a `<section>` of `<prose>` for corner cases and non-goals.
6. **Architecture and API surface** — the crate/module homes and the normative typed signatures: `<code language="rust" role="api">` blocks (never executed; the `role` keeps them out of example gating), each introduced by a prose paragraph saying what the shape is _for_.
   Dictionary/decision/mapping content authors as `<table>` blocks, never flattened into prose.
7. **Staging and gates** — the phase ladder with its go/no-go gates, as a `<table>` or `<list>`.
8. **Corpus-examples plan** — the named model and pathological example files with their expected outcomes (a `<table>` or `<list>`; graduates into `<example>` blocks with `expect-output` / `expect-error` as the examples land).
9. **Open questions / residuals** — a closing `<section>` recording what is deliberately unresolved; absorption may not silently drop a source's open questions.

Add a `<diagram>` (fletcher typst source, carrying `id` / `caption` and optional `cites`) wherever a commutative or structural picture earns its place, and a `<references>` block declaring the component's cite keys.

### Payload blocks and links

* `<table caption="...">` — one `<header>` row plus `<row>` children, cells as `<cell>` of inline content (terms, cites, refs, math all work inside cells); the shape is identical to the prose-class table.
* `<list ordered="true|false">` — flat `<item>` children, each with an optional bold `lead` attribute.
* `<code role="api">` — a normative API-surface listing; mutually exclusive with `expect-output` / `expect-error`.
* `<ref target="..."/>` — an inline cross-reference to any declared corpus id (component, section, rule, definition, diagram); the validator rejects unresolved targets and the renderer links it with the target's title as text.
  Use section-granular `<ref>`s where the pre-reboot corpus used `§`-anchors — links are the readability answer to density.

### Required attributes and edges

* Every `<component>` carries a **required** `status`: `built | partial | adopted-unbuilt | design-pass | dormant`.
* Record typed provenance with `grounds="..."` / `derives="..."` on the component root (space-separated component ids); the validator resolves them.
* Cite keys (`<cite key="..."/>`) must resolve in `refs.yml`, whose keys must in turn be ids present in `docs/research/bibliography-v2.md`.

`component-vocabulary.xml` is a worked, self-describing example: it documents the vocabulary using the vocabulary, exercising every block type.

## Lean reference register

The vocabulary borrows Lean 4 reference-manual patterns — titled syntax boxes, term links, stable tags, status attributes, checked examples.
When extending the schema, consult that register (the fp-lean tutorial voice is reserved for the future gandr manual, not this normative corpus).
