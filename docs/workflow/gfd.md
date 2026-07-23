# Workflow: authoring .gfd documents

> Read when: writing or editing any `.gfd` file — spec components, and (as the GF pipeline lands, per the owner) workflow docs and other durable project documents.
> Base practice: [specs.md](specs.md) (the spec-corpus discipline), `docs/gandr/spec/proposal-docs-gf-pipeline.md` (the architecture), `docs/gandr/spec/internalizing-gf.md` (mechanics log).

## What a `.gfd` file is

One `.gfd` file is **one `GF` abstract-syntax tree** written in PGF expression syntax: constructor applications and string literals, nothing else.
Documents are **validated, never parsed blind**: the pipeline reads the text with `readExpr` and then type-checks it at the mandatory `checkExpr` lane — unknown lexicon constants and ill-typed applications are rejected before any rendering happens.
Renderings are **linearizations** of the same tree (HTML today; Markdown, LaTeX, and more later), so nothing presentational — styling, escaping, link targets — belongs in the tree.

## Authoring rules

1. **The grammar is the vocabulary.** Constructors come from the abstract grammar (`crates/gf-docs/grammar/GandrDocs.gf`); never invent them.
   If the grammar lacks a construct you need, the grammar change is the work — not a workaround in text.
2. **Lexicon constants name everything linkable.** Terms are `term_<key>`, citations `cite_<key>`, anchors `anchor_<id>` (hyphens become underscores).
   A reference to an undefined constant fails `checkExpr` — that rejection is the term/citation/cross-reference validation working, not an error in your document's structure.
   Constant inventories live in the generated lexicon modules (`grammar/GandrDocsLex*.gf` in the `PoC`; generated at migration).
3. **Layout is canonical** (what the translator's layout engine emits and the `fmt` lane — `gandr-hz8` — will enforce):
   + A constructor application whose arguments are all atomic stays **on one line** when it fits in 72 columns: `TermDef term_status "status"`.
   + Otherwise the head opens the line and **each argument gets its own line**, indented two columns under the head, compound arguments parenthesized.
   + **`Cons` chains flatten Lisp-style**: the element follows the head on the same line, the tail continues at constant indent, closing parens trail: see any `[Inline]` list in `crates/gf-docs/corpus/component-vocabulary.gfd` for the shape. (PGF expression syntax has no list literals; `Cons`/`Base` is the only spelling.)
   + Strings are the only leaves.
     Escape exactly `\"`, `\\`, `\n`, `\t`, `\r`.
4. **Punctuation glues left.** A `Txt` whose first character is sentence punctuation (`. , ; : ! ? ) ] } " '`) takes `ConsInlineGlued` instead of `ConsInline`, so the rendered text binds the punctuation to the preceding inline instead of inserting a word space.
5. **Payloads are raw strings.** Code, math, and diagram sources are string literals in the tree, unescaped for `HTML` (escaping and compilation are the renderer's payload transforms).
   Quote source code **byte-exact** (visibility, attributes, field names — the two review findings on this are in `docs/workflow/specs.md`'s failure list).

## Editing workflow

* Check a file: `cargo run -p gandr-gf-docs --locked -- check --pgf target/gf-docs/GandrDocsLex.pgf --lang GandrDocsLexHtml --gfd <file.gfd>` (after `mise run docs:gfd:grammar`).
* The full `PoC` arc (provision toolchain, compile grammar, migrate the example, validate, render): `mise run docs:gfd:poc`.
* Grammar changes require recompiling the PGF (`mise run docs:gfd:grammar`) before checks see them.
* `XML` → `.gfd` migration of a legacy component: the crate's `migrate` lane (semantic annotation is human-checked per component during the corpus migration, `gandr-5n6`).

## The design language (gandr-4l9)

Rendered pages carry the house design language (`docs/gandr/spec/proposal-docs-design-language.md`): the tree stays semantic, the renderer supplies all presentation.
What authors may rely on, and what they must never do:

* **Never hand-style.** Constructors are semantic; classes, colors, and layout are the renderer's.
  A presentation wish is a grammar or stylesheet change, not a text workaround.
* **Payload containers are `HTML`-escaped; prose is not.** `<pre><code>`, grammar `dt`/`dd`, diagram slots, and math spans have their `String`-leaf content escaped in the post-pass — payload text may contain `<`, `>`, `&` freely.
  Prose `Txt` interleaves with constructor tags and **cannot** be escaped, so raw `<` or `&` in prose is an authoring error (the checker does not catch it; the rendered page silently eats it).
* **Production bodies carry no `::=`** — injected at rendering.
  Rule names render parenthesized below the conclusion.
* **`api`-role code lines stay ≤ 96 columns** (full inner measure; longer lines scroll, and scrolled normative signatures are a defect).
* **Captions sit in the margin** beside their figure — write captions that reward beside-reading (short, glossing, complete sentences).
* **Math** (typst source until the splice lane): the page shows source as an italic placeholder today; the splice lane will render it in TeX Gyre Pagella Math with the corpus's category-name convention (bold italic, upright subscripts) — author typst source with that target in mind.

## Status

2026-07-22: `PoC` landed and owner-accepted (`gandr-wrs` closed).
The live corpus still authors `XML` until the migration epic lands; this guidance governs every `.gfd` file from the first one onward, and will govern workflow docs when they convert (`gandr-2u0`).

## Pointers

* `docs/gandr/spec/proposal-docs-gf-pipeline.md` — the architecture and its acceptance record.
* `docs/gandr/spec/internalizing-gf.md` — the `GF` mechanics log (list categories, record linearization arguments, the `+`-gluing compiler crash, `Predef` absence, `readExpr` vs `checkExpr`).
* [specs.md](specs.md) — the spec-corpus fidelity and review discipline this inherits.
* `gandr-hz8` — the `fmt` lane bead (canonical-format enforcement).
