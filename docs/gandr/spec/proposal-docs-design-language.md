# Proposal: a visual design language for the gandr documentation corpus

> **Status:** proposal, pending owner acceptance (gandr-4l9; epic gandr-9na).
> **Date:** 2026-07-22.
> **Owner amendment 2026-07-22** (post-acceptance, owner-directed): the math-typography doctrine is restyled after Guillaume Munch-Maccagnoni's thesis (_Syntax and Models for a non-Associative Composition of Programs and Proofs_) — Palatino-lineage math, bold-italic serif category names, `::=` productions, parenthesized rule names — adapted under Tufte priority (§2, §7.2, §7.4–§7.6, and §12 decision 6).
> **Review record:** independent adversarial review 2026-07-22 (report: `adversary/2026-07-22-gandr-4l9-design-language.md` in the sibling notes repository) — 13 binding findings fixed in this revision, 5 challenged findings recorded with dispositions where they arise.
> **Scope:** the `gf-docs` HTML linearization and page shell only — typography, layout, color, tables, code, captions, links, print.
> Explicitly out of scope, as later tooling work: syntax highlighting (gandr-r8x), executable blocks (gandr-6lc), type-on-hover (gandr-l5v), bead-reference inlines (gandr-gq2), the inline-code element (gandr-4wg), sidenote grammar constructors, and the LaTeX and Markdown linearizations. gandr-6yx (legacy-renderer polish, including table caption placement) overlaps this bead's caption design: the design language governs placement doctrine; gandr-6yx inherits or defers for the legacy renderer.

## 1. Purpose

The `gf-docs` renderer ships a bare page shell with zero CSS: the corpus's first rendered page (`component-vocabulary.html`) presents in unstyled user-agent defaults.
This proposal defines the house design language — the typographic, layout, and color system every gandr documentation page renders in — and its implementation as one stylesheet plus a page-shell revision inside `gf-docs`.
The mandate (owner directive 2026-07-22): **Tufte-grade typesetting, a distinctive house style**, engineered at the CSS level so that nothing in the tree layer or the linearization's semantics depends on presentation.

Three properties make this a design _language_ rather than a stylesheet that happens to exist: every choice is **derived from a stated doctrine** (§2), every value is **specified** so it can be reviewed, measured, and tuned (§4–§9), and the system is **named and versioned in the repo** so the workflow guidance can cite it (§11).

## 2. Sources and what each contributes

The design is a synthesis of four references, each mined for a specific doctrine, plus one table manual.
Nothing is copied wholesale; where we deviate from a source, the deviation is named.

**Edward Tufte — the doctrine of restraint.** From _The Visual Display of Quantitative Information_ (1983): the **data-ink ratio** — ink that carries no information should be erased.
Applied to documents: no boxes, no shaded panels, no decorative rules, no color that is not doing semantic work.
From his book designs (_Envisioning Information_, _Beautiful Evidence_): **marginalia instead of footnotes** (annotations live in a wide margin beside the text they gloss), an **ivory paper** (`#fffff8`) rather than clinical white, and the **ET Book** typeface — a Bembo-revival book face designed by Dmitry Krasny, Bonnie Scranton, and Edward Tufte for his own books, released under the MIT license ([github.com/edwardtufte/et-book](https://github.com/edwardtufte/et-book)).

**Robert Bringhurst — the measure and the dose.** _The Elements of Typographic Style_: a text column of **45–75 characters is satisfactory, 66 is ideal**; leading is added in measured doses; headings are distinguished by **size, italic, and space — not by weight inflation**; old-style figures and real small caps belong in running text (we take the figures — the vendored roman cut supplies them natively, §4.1; synthetic small caps we refuse, §4.4).

**Matthew Butterick — the screen-pragmatic floor.** _Practical Typography_ ([practicaltypography.com](https://practicaltypography.com)): line length **45–90 characters** including spaces; line spacing **120–145%** of point size; body text on the web sits in the **15–25px** band — err larger, not smaller; paragraphs get first-line indents **or** space between, never both; small caps must be real cuts, never synthesized.
One deliberate refusal: Butterick argues the web should [move beyond underlined hyperlinks](https://practicaltypography.com/underlining.html); we decline, following tufte-css and WCAG instead (§8) — in a spec corpus dense with term and section cross-references, the persistent underline is the most widely recognized link indicator, and the refusal is conscious, not drift.

**tufte-css — the working web mechanics.** The stylesheet at [github.com/edwardtufte/tufte-css](https://github.com/edwardtufte/tufte-css) (demo: [edwardtufte.github.io/tufte-css](https://edwardtufte.github.io/tufte-css/)) proves the architecture we adopt: a **text column plus a margin column**, marginalia without JavaScript, the ivory/ink palette (`#fffff8` on `#111`), ET Book via `@font-face`, and a single-collapse responsive breakpoint.
Its link doctrine is ours: links are **underlined at rest**, “the most widely recognized indicator of clickable text” (the demo's own words), which also satisfies WCAG 1.4.1 (no meaning by color alone).
We retune every value (§5) and explicitly do **not** take: its `html { font-size: 15px }` root pin (we respect the reader's root size, §4.2), its 55%-of-body percentage widths (we use `ch` measures), its in-figure caption float (`figcaption { float: right; max-width: 40% }` inside a 55%-wide figure — cramped, and structurally unavailable to us, §5.3), and its 21px body size (one step too large for a dense technical corpus).

**booktabs — the table doctrine.** The LaTeX `booktabs` manual's two rules — _never, ever use vertical rules_; _never use double rules_ — plus its positive doctrine: rules of **varying weight** (heavy top and bottom, light midrule) and generous white space instead of ruling every row.
Row striping is refused on Tufte's data-ink grounds, not the manual's.

**Munch-Maccagnoni — the math-typography surface (owner amendment).** Guillaume Munch-Maccagnoni's thesis (_Syntax and Models for a non-Associative Composition of Programs and Proofs_, 2009) supplies the math face of the language, absorbed with Tufte priority where the two disagree:

* **Palatino-lineage math.** Category names (Dupl, Adj) are set in **bold italic Palatino-class serif** with sweeping calligraphic capitals and upright roman subscripts — the thesis's most distinctive mark, and the owner's named preference.
  Its web/typst heir is **TeX Gyre Pagella Math** (GUST, free license): the splice lane's configured math font (§7.4).
* **The italic/upright discipline.** Metavariables italic; operators and keywords (fst, snd, fv, stop) upright roman; category/sort/system names bold italic — three registers a reader learns once.
* **`::=` productions** with aligned alternatives (BNF canon, not an invented arrow).
* **Parenthesized rule names** — `(cut)`, `(⊢→)` — lowercase roman with math symbols.
* **Declined under Tufte priority:** the thesis's thin full-frame boxes around figure apparatus (boxes are non-data-ink; our hairline carries the grouping information at a fraction of the ink) and its bold-labelled centered captions (our margin captions are the Tufte signature).

## 3. Principles

1. **Data-ink first.** Every styled element answers "what does this mark tell the reader?"
   A hairline that marks the extent of a normative block carries information; a card shadow does not.
2. **The margin is the annotation column.** Captions, and later sidenotes, live beside the text they gloss — never below it on wide screens, never in a tooltip.
3. **Color is a controlled vocabulary.** Five status colors, one interaction accent, one hairline gray, paper, and ink.
   Nothing else may enter the palette without a semantic charter.
4. **Respect the reader's root.** No `px` pin on `html`; the reader's font-size preference scales the whole page proportionally.
5. **No JavaScript, ever, in the design layer.** Layout, marginalia, and responsiveness are pure CSS. (Tooling beads may add JS later; the design must never require it.)
6. **Real cuts or redesign.** Vendored ET Book roman, italic, and bold — no synthesized bold/italic/small-caps anywhere.
7. **Print is a first-class linearization target.** The same page prints as a competent book chapter with no author intervention.

## 4. Typography

### 4.1 Faces

| Role     | Stack                                                                                       | Notes                                                                                                                                                                                                                                                                                                   |
| -------- | ------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Text** | `et-book, Palatino, "Palatino Linotype", "Palatino LT STD", "Book Antiqua", Georgia, serif` | ET Book **vendored**: the upstream **ligatures-enabled** (`ETBookOT`) set — roman (old-style figures), italic (old-style figures), bold (lining figures — the only bold cut published) — as WOFF2, MIT license, `LICENSE.et-book` beside the fonts. The fallback chain is tufte-css's exact body stack. |
| **Mono** | `ui-monospace, "SF Mono", Menlo, Consolas, "Liberation Mono", monospace`                    | System stack — good monos ship with every OS; vendoring a mono adds license surface for no identity gain. The serif _is_ the identity.                                                                                                                                                                  |
| **Sans** | `"Gill Sans", "Gill Sans MT", Calibri, "Segoe UI", system-ui, sans-serif`                   | One use only: the status chip (§6.1). Extends tufte-css's `.sans` stack with two modern fallbacks.                                                                                                                                                                                                      |

Upstream publishes two sets under `edwardtufte/et-book`: the `et-book/` directory (no ligatures; WOFF/EOT/SVG/TTF) that tufte-css consumes, and `et-book-ligatures-enabled/` (the `ETBookOT` cuts; OTF masters plus WOFF **and WOFF2** webfonts).
We vendor the ligatures-enabled WOFF2 files — `etbookot-roman-webfont.woff2`, `etbookot-italic-webfont.woff2`, `etbookot-bold-webfont.woff2` (~135KB total) — because they are the book-authentic cuts: real ligatures, and a roman whose figures are **old-style by design**, so running text, cite keys, and register numerals get text figures natively (the §2 doctrine) rather than by a no-op `font-variant-numeric` plea.
One disclosed deviation: the bold cut exists only with lining figures; bold numerals are rare in corpus prose (strong leads are words), and the trade is accepted here rather than silently.
`@font-face` declares one `et-book` family mapping the three files to `400 normal` / `400 italic` / `700 normal`, `font-display: swap`.

### 4.2 Scale and rhythm

Root: **no `html` font-size rule** — `1rem` is the reader's preference (16px by default).
All sizes in `rem`; the vertical rhythm is a simple spacing scale, not a baseline grid (marginalia placement defeats strict baseline alignment; Tufte's own pages forgo it).

| Element                    | Size                                     | Leading | Weight/Style  |
| -------------------------- | ---------------------------------------- | ------- | ------------- |
| Body prose                 | `1.1875rem` (19px @16)                   | `1.45`  | roman         |
| `h1` (component title)     | `2.4rem`                                 | `1.1`   | roman, `400`  |
| `h2` (section)             | `1.7rem`                                 | `1.15`  | italic, `400` |
| `h3` (block title)         | `1.25rem`                                | `1.2`   | italic, `400` |
| Captions / marginalia      | `0.9375rem` (15px)                       | `1.4`   | roman         |
| Code blocks                | `0.875rem` (14px)                        | `1.5`   | mono          |
| References list, `.expect` | `0.875rem`                               | `1.45`  | roman         |
| Status chip                | `0.75rem`, letter-spacing `0.12em`, caps | —       | sans          |

19px body with 1.45 leading = 27.6px ≈ **145%** — Butterick's upper bound, inside his 15–25px web band, chosen because ET Book is a delicate, small-x-height face that reads small at nominal size (tufte-css sets it at 21px/143%; we are one step denser for a technical corpus).

The `h1` ratio is ratified, not drifted into: at 2.02× body it sits at the ratio Butterick mocks as a _default_, but a component title is a **once-per-page chapter head**, the single largest mark on an otherwise quiet page — Tufte's book design and tufte-css both run titles well above 2× (tufte-css: 3.2rem over 21px ≈ 2.4×).
The 2.4rem choice stands as a conscious chapter-head doctrine, not an inherited browser default.

**Verification target at inspection:** measured characters-per-line in the text column lands in **65–80** (Bringhurst's 45–75 band, allowing ET Book's narrow advance widths; the 80 top end exceeds Bringhurst 75 but sits inside Butterick 90 — disclosed).
The stylesheet ships `max-width: 62ch` and the inspection step tunes the `ch` value if measurement says otherwise.

### 4.3 Emphasis and inline semantics

* `<strong>` → ET Book **bold** (the real cut), used by the authored vocabulary only (`Definition`/`MkItem` leads).
* `<em>` → ET Book **italic** (the real cut).
* `<dfn>` → italic; term definition is already announced by the block's `Definition` label, so no second signal.
* Inline `<code>` → mono at `0.9em`, no background, no border.
* `.math` (typst source pending the SVG splice lane) → italic serif, visually continuous with prose; `.math-block` display rows centered (§7.4).
  Forward rule, harmless now, load-bearing at splice: `.math svg { height: 1em; vertical-align: -0.15em; }`.
* `<sup>` → `line-height: 0` (citations must not inflate the leading).

### 4.4 What we refuse

Synthetic small caps (Butterick: fake SC is worse than none — ET Book ships no SC cut), letterspaced lowercase, all-caps headings, underlined non-links (underlines are the link affordance; spending them on emphasis bankrupts it), gray "de-emphasized" body text, drop caps, `text-align: justify` (ragged right; justification without professional hyphenation is a spacing disaster on screen).

## 5. Layout

### 5.1 The two-zone page

```text
|<- 4ch ->|<----------- 62ch ----------->|<- 4ch ->|<--- 30ch --->|<- 4ch ->|
  padding          text column             gap       margin zone    padding
```

* `main.page` is the container: `max-width: 104ch; margin-inline: auto; padding-inline: 4ch` — inner width exactly `96ch = 62 + 4 + 30`.
  The zones close with no slack; the margin zone begins at exactly `66ch` from the text edge.
* The width cap is assigned precisely: **every direct child of `article` except `section`, and every direct child of `section` except `figure`, gets `max-width: 62ch`.** `section` and `figure` are the two spanning elements — capping `section` would make the full-width figure (and with it the margin zone) impossible; the cap belongs on the text-bearing children (`p`, `dl`, `ol`, `ul`, `pre`, `.judgements`, `.rule`, `.definition`, `.example`, headings).
* The head matter (`h1`, `p.status-chip`) and the trailing `h2`/`ul.refs` are direct `article` children and take the `62ch` cap like everything else text.

Below a `68em` breakpoint (viewport can no longer fund `96ch` + padding; tuned at inspection): single column — all caps release to `none`, `figure` reverts to `display: block` with captions static below content, container padding tightens to `5vw`.
One breakpoint, tufte-css's shape, retuned.

### 5.2 Section rhythm

* `section` separates by **space, not rules**: `h2` carries `margin-top: 2.5rem` as the only section divider (Tufte: rules between sections are non-data-ink).
* Paragraphs: `1.4rem` vertical margins, **space-between, no first-line indent** (Butterick: one or the other; the corpus's short paragraphs and interleaved payload blocks favor space).
* Lists: hanging structure, `0.25rem` between items; the `RegisterBlock` ordered list keeps its numerals (they are data: obligation counts).

### 5.3 Marginalia posture (v1)

The grammar emits **no sidenote constructs today** — the only margin citizens are `<figcaption>` (tables and diagrams), and it emits them _after_ their content (`<figure><table>…</table><figcaption>…</figcaption></figure>`).
A trailing element cannot float beside an earlier block (CSS 2.1, section 9.5.1, rules 5–6 place a float no higher than the line box where it is encountered), so the tufte-css float recipe is structurally unavailable here without reordering the linearization — a grammar change this proposal declines (§10).
The v1 mechanism is therefore **grid, not float**: `figure { display: grid; grid-template-columns: minmax(0, 62ch) minmax(0, 30ch); column-gap: 4ch; align-items: start }`, with the content child assigned `grid-column: 1` and `figcaption { grid-column: 2; grid-row: 1 }`.
Explicit grid placement is source-order independent, puts the caption at exactly `66ch` top-aligned with its content, and contains tall captions inside the figure's own row — no negative margins, no clearfix duties, no caption-overhang into the next figure (the review's C1 concern is removed by construction, not managed).
Below the breakpoint and in print, `figure` reverts to `display: block` and the caption goes static under its content at caption size.
Future sidenote constructors (a later grammar bead) will be inline spans in prose; _they_ will use the tufte-css float pattern, which works from an inline position — what this section reserves is the shared **zone geometry** (`66ch` start, `30ch` width), not one mechanism.

### 5.4 The normative-block hairline

The corpus's titled payload blocks — `definition`, `rule`, `example`, `judgements` — and `pre` code blocks carry a **1px left hairline** (`--rule`, §6) with `1.5ch` padding-left.
This is the house's one structural device: a quiet edge marking _normative content_ (typeset obligations, rules, API surfaces) off from discursive prose, in a corpus where the distinction is the document's whole point.
It replaces boxes: same information, a fraction of the ink.
One hairline per unit: a normative block nested inside another (a `pre` inside an `example`) defers to the outer edge — nested hairlines are double-ruling, which booktabs bans.
Mechanism, stated so nothing is left to interpretation: the hairline is a `border-left` on the block element itself — on `pre` it is a border on the scroll container, which does not scroll with the content (borders never do); the `1.5ch` padding is inside the scrollport, so long lines scroll under it honestly.

## 6. Color

Custom properties, so a later dark scheme is a drop-in (§6.2), and so the palette's closedness is inspectable in one place.

| Token                      | Value     | Contrast on `--paper` | Charter                                                                                                                                                 |
| -------------------------- | --------- | --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `--paper`                  | `#fffff8` | —                     | Tufte ivory — warm, low-glare, the identity                                                                                                             |
| `--ink`                    | `#111111` | 17.4:1                | near-black, warm                                                                                                                                        |
| `--ink-soft`               | `#3d3c38` | 11.0:1                | captions, cite keys, secondary metadata                                                                                                                 |
| `--rule`                   | `#9a937d` | 3.06:1                | hairlines only (normative-block edge, booktabs midrule) — meets the 3:1 non-text-contrast guidance deliberately; the mark is quiet but never subliminal |
| `--accent`                 | `#93301a` | 7.81:1                | iron-oxide red — link hover/focus **only**; warm on ivory, rust-adjacent without being crate-orange                                                     |
| `--status-built`           | `#3f6b3a` | 6.20:1                | moss                                                                                                                                                    |
| `--status-partial`         | `#946518` | 5.06:1                | ochre (darkened from the first draft's `#9a6a1f` at 4.69:1 — the "darken, don't re-hue" rule exercised immediately)                                     |
| `--status-adopted-unbuilt` | `#3d5a80` | 7.03:1                | slate                                                                                                                                                   |
| `--status-design-pass`     | `#6b4f8e` | 6.67:1                | muted violet                                                                                                                                            |
| `--status-dormant`         | `#6e6a61` | 5.37:1                | warm gray                                                                                                                                               |

Contrasts computed per WCAG 2.x relative luminance against `#fffff8` (all ≥ 4.5:1 for text, `--rule` ≥ 3:1 as a non-text mark) and machine-verified at authoring time; the implementation re-verifies before landing.
Hue carries the semantics, lightness carries the legibility: any future miss gets darkened, never re-hued.
Body text never takes color; links are ink-colored at rest (§8), so on a fresh page the _only_ color above the fold is the status chip — that is the restraint doing its work.

### 6.1 Status chip

`status-chip` renders as letterspaced sans caps in the status color, preceded by a `0.55em` solid square (`::before`) in the same color — a swatch and a word, never color alone (color-vision deficiency: the word carries the meaning, the swatch carries the scan).

### 6.2 Dark scheme (proposed, owner decision)

A `prefers-color-scheme: dark` block inverting the same tokens — paper `#151512` (warm near-black), ink `#e6e2d5` (14.1:1 on the dark paper, machine-verified), hairline and accent lightened proportionally, status hues brightened ~20% to hold ≥4.5:1 on the dark paper.
It costs ~15 lines because the palette is tokenized.
Tufte purists object that ivory _is_ the identity; engineers reading at midnight object to being flashbanged.
**Recommendation: include.** Flagged as an owner decision (§12).

## 7. Component treatments

### 7.1 Head matter

`h1` roman at `2.4rem`, the status chip one line below (`margin-top: 0.5rem`), then a `4rem` silence before the first section.
No rule under the title — the chip's color and the white space do the work.

### 7.2 Tables (`figure.table`) — booktabs

* No vertical rules, no row rules except the booktabs three: `border-top: 2px solid var(--ink)` on the table, `1px solid var(--ink)` under `thead`, `2px solid var(--ink)` at the bottom. (Top/bottom in `--ink`, not `--rule`: booktabs' heavy rules are full-strength.)
* `th` roman (non-bold, the thesis/booktabs convention — the midrule already marks the header), left-aligned; cells `padding: 0.35em 1.2em 0.35em 0`; text at `0.95` of body size.
  Numerals in the text face are old-style by design (the vendored roman cut, §4.1); no tabular-figure feature is asserted — the corpus's tables are prose-heavy, and numeric-column alignment is revisited when such a column exists.
* Caption in the margin zone via the figure grid (§5.3); the table child takes `grid-column: 1` (`minmax(0, 62ch)`), `overflow-x: auto` on the table's own display context as backstop — corpus tables are authored narrow (the inventory is the widest and fits); a table that still overflows scrolls, honestly, inside its column.
* No striping, no hover states, no borders between rows: adjacent rows group by the `0.35em` rhythm alone.

### 7.3 Code (`pre`) and API surfaces

* Mono at `0.875rem/1.5`, the normative-block hairline (§5.4), `overflow-x: auto`, capped at `62ch`.
* **API-role blocks relax the container, not the child:** the grammar puts the `api` class on the `code` element inside the `pre`, so the reprieve must target the scroll container — `pre:has(> code.api) { max-width: 96ch }` (`:has()` is universally supported since 2023; styling `code.api` itself would leave the `62ch` `pre` scrolling and the reprieve dead).
  The reprieve is the **full inner measure** (text + gap + margin): `pre` has no marginalia, so payload without marginalia may span all three zones — Tufte's full-width-figure precedent applied to normative code.
  Normative signatures must not scroll: horizontal scroll on a normative API is a reading failure, not a layout choice.
  Lines longer than `96ch` still scroll — the honest backstop.
* `.expect` ("expected output: …") attaches to its `pre`: caption size, `--ink-soft`, pulled up `0.6rem` so the pair reads as one unit.
* No background tint, no line numbers, no language badge (all of those arrive — if at all — with the highlighting bead gandr-r8x, which inherits this shell).

### 7.4 Judgements and math — the Munch doctrine

`JudgementsBlock` rows: each `.math-block` centered, `0.6em` vertical air — display-math posture, matching the thesis's centered unnumbered displays.
Until the typst-SVG splice lane (proposal-docs-gf-pipeline §3.6) lands, `.math` spans carry typst source set as italic serif placeholders, visually continuous with prose.
The splice lane's math typography is fixed **now**, as design-language contract:

* **Math font: TeX Gyre Pagella Math** (the Palatino lineage of the thesis's math, GUST free license) — configured in the splice lane's typst preamble, vendored with it.
* **Category/sort/system names: bold italic serif** (the thesis's Dupl/Adj convention — sweeping calligraphic capitals), with **upright roman subscripts** (`Adj_eq`).
  This is the highest-rank type distinction in the corpus's math: a category name is visible from across the page.
* **Metavariables italic, operators/keywords upright roman** — the thesis's three-register discipline (bold italic / italic / upright) a reader learns once.
* **Displays centered, unnumbered**, with generous air; no equation numbers (the corpus cross-references by anchor, not by number).
* Inline math sized to text, never scaled down.

Prose-level category names (outside math) have no grammar inline today; a `CatName`-style inline is a candidate grammar addition sequenced with the recursion-surface re-authoring (gandr-aaq), to be set in vendored TeX Gyre Pagella bold italic — recorded here so the convention has one home.

### 7.5 Rules (`div.rule`) — textbook inference rules

The one place the design spends ink on structure, because the content _is_ the structure:

```text
            Γ ⊢ e₁ : A → B     Γ ⊢ e₂ : A
          ───────────────────────────────────
                  Γ ⊢ e₁ e₂ : B
                       (T-App)
```

* `.rule` is a **single-column grid** (`display: grid; grid-template-columns: minmax(0, auto); justify-content: center; justify-items: center`): one track sized to the **widest row**, all children centered in it — no wrapper element needed, and the premise rows, stroke, and conclusion share one width by construction.
* The stroke is `.conclusion`'s `border-top` (`1.5px solid var(--ink)`): `.conclusion { justify-self: stretch; text-align: center }`, so the rule line spans the widest row — premises above it, conclusion below it, `0.5em` of air on either side.
* The `h3` rule name renders **below the conclusion, centered, in parentheses, upright roman** — the thesis's `(cut)`/`(⊢→)` convention adapted: names in the thesis sit beside the bar, but the linearization emits `h3` _first_ and CSS alone cannot reliably seat a leading element beside a later sibling's border; below-and-parenthesized is the robust analog (grid `order` moves it after the conclusion; `::before`/`::after` supply the parentheses).
  Beside-the-stroke placement remains available to a later grammar bead as a one-line linearization reorder — named here so the deferral is a decision, not an oversight.
* The whole block keeps the normative hairline (§5.4).

### 7.6 Grammar (`dl.grammar`)

`dt`/`dd` pairs as a two-column grid (`max-content 1fr`, `2ch` column gap, `0.3rem` row gap): `dt` the production symbol in italic serif, `dd` the production body in mono at `0.9rem`.
Corpus production bodies carry **no separator** (verified: `MkProduction "component" "'<component>' status blocks* '</component>'"`), so the stylesheet injects the BNF-canon notation as generated content: `dl.grammar dd::before { content: "::= " }` (the thesis's convention, §2).
The separator is notation, not styling, and generated content is its honest home at the CSS layer: the `dt`/`dd` relation already encodes the production semantically, the tree stays clean, and no grammar change is spent on it.

### 7.7 Examples and definitions

`example` keeps the normative hairline; its `h3` ("Example: …") at standard `h3` style.
`definition` keeps the hairline; the authored `<strong>Definition</strong> (<dfn>…</dfn>).` lead reads as a textbook defined-term entry — no further apparatus.

### 7.8 Diagrams (`figure.diagram`)

The `.diagram-slot` (typst source today, SVG at the splice lane) takes `grid-column: 1`, content horizontally centered; caption in the margin zone via the figure grid (§5.3).
No hairline: figure content (tables, diagram slots) is delimited by the figure structure itself — the hairline is reserved for standalone normative blocks (§5.4).
Forward rule for the splice lane: `.diagram-slot svg { max-width: 100%; height: auto; }`.

### 7.9 References (`ul.refs`)

`0.875rem`, `list-style: none`, hanging indent `3ch`, cite keys in `--ink-soft`.
The bibliography splice (refs.yml → full entries) inherits this treatment unchanged.

## 8. Links

A **semantic link vocabulary** — the underline style says what the link is:

| Class                         | Treatment                                                     | Meaning                                    |
| ----------------------------- | ------------------------------------------------------------- | ------------------------------------------ |
| `a.term`                      | underline, `dotted`                                           | jumps to the term's `<dfn>` in this corpus |
| `a.xref`                      | underline, solid                                              | jumps to a section/block anchor            |
| `a.cite`                      | no underline (the `<sup>[key]</sup>` shape is the affordance) | jumps to the references list               |
| any `:hover`/`:focus-visible` | `color: var(--accent)` + underline thickens to `0.09em`       | interaction                                |

All links `color: inherit` (ink) at rest — meaning never rides on color alone (WCAG 1.4.1), the persistent underline being the corpus's chosen affordance (§2, tufte-css's doctrine, consciously taken over Butterick's contrary advice); `text-underline-offset: 0.12em`; `text-decoration-thickness: 0.06em`.
`:focus-visible` additionally gets a `2px` `--accent` outline — keyboard navigation is a first-class reading mode.

## 9. Print

`@media print`: paper forced to white with `--ink` full black (printers, not screens, own ivory); root `11pt` (Bringhurst's book size); measure uncapped to the page's text block; `figure` reverts to `display: block` with captions static under figures; the status chip prints as its word with a black swatch; links keep their underlines (they never had color to lose); `figure, .rule, .definition, pre { break-inside: avoid }`; `@page { margin: 2.2cm 2.4cm }`.
The same HTML prints as a competent book chapter — the rehearsal for the LaTeX linearization, whose doctrine this fixes in advance.

## 10. Page shell and asset engineering

The build lane (`crates/gf-docs/src/main.rs::do_build`) replaces the bare shell:

* `<meta name="viewport" content="width=device-width, initial-scale=1">`.
* `<title>` lifted from the rendered `<h1>` — one `h1` per page by grammatical construction (`MkComponent` is the sole `Component` linearization and its title is a plain `String` leaf, so the rendered `h1` contains no markup), extracted by a scoped scan in the pipeline post-pass; fallback is the file stem.
  No grammar change.
* Landmarks: `<body><main class="page"><article>…</article></main></body>` — one `<main>`, heading order already correct by construction (h1 → h2 → h3).
* The stylesheet is a compile-time asset: `crates/gf-docs/assets/gandr-docs.css`, embedded with `include_str!` and inlined into a `<style>` element — every page is **self-contained** (movable, archivable, diffable; no relative-path fragility), at ~12KB of CSS per page, nothing to cache-bust.
* Fonts cannot inline sanely: the build lane copies `crates/gf-docs/assets/fonts/` → `<out-dir>/fonts/` (idempotent, three WOFF2 files + `LICENSE.et-book`); the CSS references them relatively (`fonts/etbookot-…woff2`).
  A page moved without its `fonts/` sibling degrades to the Palatino/Georgia stack — the graceful, declared failure mode.
* Grammar (`GandrDocsHtml.gf`) changes: **none.** The grid marginalia (§5.3), the container-targeted API reprieve (§7.3), the generated-content production arrow (§7.6), and the grid rule stroke (§7.5) are all achievable against the emitted markup as-is; the PGF needs no recompile for this bead.

## 11. Implementation and verification plan

1. Vendor the fonts + license; write `assets/gandr-docs.css` implementing §4–§9 exactly (every value above is the spec).
2. Rework the page shell (§10) in `main.rs` + a scoped title extraction in `pipeline.rs`.
3. Rebuild the PoC (`mise run docs:gfd:poc`); **visually inspect in a real browser** at multiple widths and in print preview; measure actual CPL against the §4.2 target and tune; re-verify the §6 contrasts.
4. Named test(s) in the crate: the emitted page contains the lifted `<title>`, the inlined `<style>`, the `<main>` landmark, and the fonts copy exists — the shell's observable contract.
5. `cargo nextest run -p gandr-gf-docs`, treefmt, then fold the author-facing guidance into `docs/workflow/specs.md` and `docs/workflow/gfd.md` (what the design language is, what authors may rely on — semantic classes, figure/caption posture — and what they must never hand-style).
6. `mise run gate:merge` before closeout, per the closeout doctrine.

## 12. Decisions flagged for the owner

1. **Vendor ET Book — the ligatures-enabled `ETBookOT` set, as WOFF2 (recommended).** Three WOFF2 binaries + MIT license enter the repo at `crates/gf-docs/assets/fonts/`.
   This set (not the no-ligatures directory tufte-css consumes) is the book-authentic choice: real ligatures, old-style roman figures natively, and upstream WOFF2 files (~135KB total) — it satisfies the §2 figures doctrine by design rather than by feature-flag hope.
   The alternative — a pure system stack (Palatino/Georgia) — costs zero assets but renders differently on every platform and abandons the Tufte identity that is this bead's mandate.
   The repo's no-vendoring rule covers _research artifacts_ (companion code, mechanizations); a design dependency with an MIT grant is a different class — but the posture is strict enough that this is surfaced, not assumed.
2. **Include the dark scheme (recommended, §6.2).** Cut it and the tokens stay, so it can return later for ~15 lines.
3. **Marginalia v1 = captions only, via the figure grid (recommended, §5.3).** Sidenotes are grammar constructors — a later bead; the zone geometry is reserved now so that bead is additive.
4. **The normative-block hairline (recommended, §5.4).** The Tufte-purist alternative is no structural marks at all; judged insufficient for a corpus whose readers must locate normative content fast.
5. **Rule names below the conclusion, parenthesized, upright roman (recommended, §7.5).** Beside-the-stroke placement needs a linearization wrapper — deferred to a grammar bead rather than faked with fragile CSS.
6. **Math in TeX Gyre Pagella Math with bold-italic category names (owner-directed, §7.4).** The Munch thesis's math surface: Palatino-lineage math, the bold-italic/upright-subscript convention for category/sort/system names, italic metavariables, upright operators.
   The font config itself lands with the typst-SVG splice lane; the doctrine is fixed here so the splice lane and the recursion-surface re-authoring (gandr-aaq) inherit one convention.

## 13. References

* Edward Tufte, _The Visual Display of Quantitative Information_ (Graphics Press, 1983) — data-ink ratio; _Envisioning Information_ (1990) — marginalia, layering.
* [tufte-css](https://github.com/edwardtufte/tufte-css) and its [demo](https://edwardtufte.github.io/tufte-css/) — the two-column architecture, ivory/ink palette, ET Book stacks, single-breakpoint collapse, underlined-link doctrine; [et-book](https://github.com/edwardtufte/et-book) — the typeface (MIT license; the `et-book-ligatures-enabled/` `ETBookOT` cuts this proposal vendors).
* Robert Bringhurst, _The Elements of Typographic Style_ (Hartley & Marks, 4th ed.) — measure (45–75, 66 ideal), measured leading, heading discipline, real-cuts doctrine.
* Matthew Butterick, _Practical Typography_ — [line length](https://practicaltypography.com/line-length.html) (45–90), [line spacing](https://practicaltypography.com/line-spacing.html) (120–145%), [point size](https://practicaltypography.com/point-size.html) (15–25px on the web), first-line-indent exclusivity, real small caps; [underlining](https://practicaltypography.com/underlining.html) — the anti-underline position this proposal consciously declines (§2, §8).
* Simon Fear, _booktabs: publication quality tables in LaTeX_ (CTAN) — no vertical rules, no double rules, weighted rules, space over ruling.
* Guillaume Munch-Maccagnoni, _Syntax and Models for a non-Associative Composition of Programs and Proofs_ (PhD thesis, 2009) — the math-typography surface (owner amendment): bold-italic Palatino-lineage category names with upright subscripts, the italic/upright register discipline, `::=` productions, parenthesized rule names; and [TeX Gyre Pagella Math](https://ctan.org/pkg/tex-gyre-pagella-math) (GUST) — the splice lane's math font.
* `docs/gandr/spec/proposal-docs-gf-pipeline.md` — the architecture this styles (§3.3 HTML linearization, §3.6 payload transforms).
* `docs/gandr/spec/internalizing-gf.md` — the mechanics log (linearization arguments, glue behavior) the stylesheet never depends on.
