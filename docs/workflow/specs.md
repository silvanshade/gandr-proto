# Workflow: authoring the spec corpus (docs/spec)

> Read when: creating, editing, or re-absorbing any `docs/spec/` component; the posture also governs `docs/research/` records.
> **Pipeline note (2026-07-22):** the corpus is migrating to the GF-native pipeline (`gandr-2vv`); `.gfd` authoring is governed by [gfd.md](gfd.md), and this document's discipline (fidelity bar, two-register weave, reviews) carries over unchanged.
> Base practice: `docs/spec/README.md` (the authoring discipline), [review.md](review.md) §"Documentation fidelity review" (the mandatory review), [docs.md](docs.md) (the economy posture).

## The priority order

1. **Technical precision and exhaustive detail** — implementation details, normative signatures, proof obligations, theorem numbers, gate conditions — are the single most important content to retain across these documents.
2. Natural prose and conversational tone are an aspiration beyond that.
   When the two axes conflict, the first wins, always.

## The register is timeless

Component prose is written for the technical manual the corpus becomes: it says what the design **is and does**, forward-looking, never how the design was reached.
Absorption is not a chronicle — superseded verdicts, “what changed” narratives, and session archaeology (who ran which pass, when, on what steer) are decision-record and bead content, and they live there, not in the component.
Load-bearing rationale is unaffected: the reasons a decision binds stay inlined in current terms, as they always have.
What leaves is the retrospective _framing_ — “earlier passes filed this as X”, “two things changed since”, “this component absorbs the Y session” — replaced by the positive statement the framing was carrying.
Status carries the time dimension: the status attributes (`built`, `partial`, `adopted-unbuilt`, `design-pass`, `dormant`) are where a reader learns what is realized and what is pending, and the staging section sequences the future.
Provenance that is itself content-honesty — a claim resting on a talk transcript or a delegated close-read — stays marked at the claim, because that is about the source, not the session.
Decision and rung tags are semantic names, section anchors, or tracker ids — never letter serials (`R1`, `T0`) in titles or prose.
The test: a reader meeting the corpus as a manual learns the design without learning its biography.

## The per-document procedure

1. **Declare the source set before writing.** Absorption: the source files (and the ledger row, once `docs/research/absorption-ledger.xml` exists).
   Net-new: the commissioning bead plus the realized code the document must stay faithful to.
   A change with no declared source set cannot be fidelity-reviewed, and therefore cannot land.
2. **Read the whole source set; build the content-class inventory.** For each class — decision/summary tables; grammars; typing rules; type/code signatures; architecture (crate/module homes); algorithms; staging plans with gates; corpus-example plans; open questions; dependency tables; precise citations (theorem numbers, section anchors) — record what exists and where it will live in the component.
   Omission is invisible in the artifact; the inventory is how the author sees it before the reviewer has to.
3. **Write to the nine-section skeleton** (`docs/spec/README.md` §"Per-feature component skeleton") in the two-register weave: payload lives in blocks, every block gets an introducing prose paragraph, and density is answered by spreading out, explaining, and linking — never by dropping.
   Re-absorption is a **merge**: reboot truth wins on status and naming; the source wins on payload.
   Pre-reboot record numbers stay banned, but their load-bearing rationale is inlined restated in current terms; `gandr-*` bead ids are permitted.
4. **Code and signature discipline.** Quote code **byte-exact** — visibility, attributes, field names and types (a "quoted" block that silently drops `pub` is a factual error, not an excerpt convention; mark excerpts as excerpts).
   Normative signatures use `<code role="api">` and never carry expectation attributes.
   Prefer separate code blocks over inline code — easier to read, easier to style.
   Escape angle brackets in code text (`Vec&lt;Inline&gt;`): an unescaped `<` parses as an XML tag and fails `docs:check` with a confusing error.
   When naming fixtures, name where the test actually lives (unit file vs acceptance file — a wrong locator in a corpus-plan table is a factual error).
5. **Ref composition.** `<ref>` link text is the target's title, so compose sentences that read naturally with the full title inline, or restructure to a parenthetical ("see `<ref>`").
6. **Gates, in order:** `mise run docs:check`; `mise run treefmt`; `mise run docs:build` — all green.
   If `crates/workflow-docs` changed, `cargo nextest run -p gandr-workflow-docs` too.
   Registered docs: update the `docs/gandr/MANIFEST.yml` hash in the same commit (`docs:manifest-drift` watches it).
7. **Visual inspection.** Open the built page (`target/docs-spec/<component>.html`), scroll the changed sections, screenshot.
   Check: tables render header/body; code blocks are monospace; ref links read as sentences; captions and status chips are sane.
   Reasonable one-line styling fixes in passing are fine; anything larger — syntax highlighting, executable blocks, hover types, bead links, an inline-code element — is a bead, never a mid-document tangent (the standing set: `gandr-4wg`, `gandr-r8x`, `gandr-gq2`, `gandr-6lc`, `gandr-l5v`, `gandr-6yx`).
8. **Mandatory two-axis review** ([review.md](review.md) §"Documentation fidelity review"): an independent read-only reviewer, given the changed files and the declared source set (not the author's rationale), stanced adversarially ("prove load-bearing detail was lost"), recording the per-class retained/compressed/dropped inventory.
   Gate: zero dropped load-bearing classes.
   Binding findings are fixed before landing; the artifact goes to the notes `adversary/` register; the commit message cites the review.
9. **Commit per component** (`docs(spec): …`), classified publishable (no machine-local paths, no session forensics), with the canonical co-author trailer.
10. **Tracker and ledger.** Bead notes updated and pushed (`bd dolt commit` + `bd dolt push`); once the absorption ledger exists, its row rides the same change.
11. **Report.** A per-document completion report carries the deliverables _and_ the process findings — workflow-guidance gaps, tool problems, lessons — so the guidance improves with every document.

## The loss signature (what the audits measured)

The 2026-07-21 audits measured ~50–70% retention on the first-pass absorptions; the recurring drops: decision tables prosified; Rust type/trait/enum signatures dropped; crate/module layouts dropped; corpus-example plans dropped; open-questions sections dropped; decision-trail rationale erased along with the (correctly banned) record numbers.
The counter is structural: the skeleton's four closing sections (6–9) exist precisely to hold those classes, and "not applicable" is stated, never omitted.

## The design language (gandr-4l9)

Rendered pages from the `GF` pipeline carry the house design language (`docs/gandr/spec/proposal-docs-design-language.md`, owner-accepted 2026-07-22): Tufte-grade typography and layout (ET Book, 62ch text column, margin-zone captions), the closed color palette with semantic status colors, booktabs tables, the normative-block hairline, and the Munch-derived math doctrine (TeX Gyre Pagella Math at the splice lane; category/sort/system names bold italic with upright subscripts; italic metavariables; upright operators).
Author-facing consequences:

* **Presentation never enters the source.** The vocabulary's constructors are semantic; the renderer supplies every class, and authors never hand-style.
* **Production bodies carry no `::=`** — the stylesheet injects the separator; write `'<component>' status blocks* '</component>'`, never `component ::= …`.
* **Rule names render parenthesized below the conclusion** (`(T-App)`) — write the bare name, the rendering supplies the parentheses.
* **Normative API surfaces** (`api`-role code) render at full inner measure and must not scroll: keep lines ≤ 96 columns, exactly like the 72-column canonical `.gfd` layout discipline.
* **Math authoring** (typst source, splice lane pending): write category/sort/system names so they can be set bold italic with upright subscripts; keep operators and keywords upright-able (word-like, not letter-like).

## The prose-pacing doctrine (gandr-aaq)

Prose in the corpus is **aired out**: one load-bearing idea per paragraph, with paragraph breaks taken freely.
**Bold and italics mark the load-bearing ideas** — a reader skimming only the emphasized text recovers the document's spine; that is what makes the structure scannable.
Density belongs in payload blocks (tables, grammars, code, registers); prose carries one idea at a time.
A paragraph that needs two load-bearing ideas gets split; a paragraph whose idea needs emphasis gets it; emphasis is the concept marker, so roughly one emphasized idea per paragraph is the shape of the target, not a quota.

Explicitly _not_ the target: syllable- and complex-word readability scores (Flesch/Fog and friends).
This is a mathematics corpus — complex words are not the enemy (owner direction, 2026-07-23); density, pacing, concept placement at paragraph boundaries, and flow are.

The **measurement** is the metrics lane (`cargo run -p gandr-workflow-docs -- metrics [FILES…]`, proposal §3.5):

* **Exact by construction** (tree walks): section shape, block mix, weave compliance (zero violations is the gate — every payload block gets an introducing prose paragraph), emphasis spans per paragraph (the concept-marking signal), and term/cross-reference chains between adjacent paragraphs (flow: adjacent paragraphs should share references; a chronically zero chain reads as a pile of notes, not a document).
* **On linearized prose**: sentence- and paragraph-length distributions (mean/median/p90/max/stdev) — the rhythm numbers.
  Sentences trend shorter than the first-pass corpus's (mean ≈ 28, p90 ≈ 49); paragraphs trend thinner (median ≈ 108 words is over-dense).
  Calibrated on the recursion-surface revision (gandr-aaq, the doctrine's reference point, 3047 prose words over 14 sections): **sentence words mean ≈ 18, median ≈ 17, p90 ≈ 29, max ≈ 38; paragraph words mean ≈ 42, median ≈ 38, p90 ≈ 74; ≈ 2.3 sentences per paragraph; ≈ 1 emphasis span per 100 words marking ≈ 38% of paragraphs; zero weave violations; adjacent-paragraph reference chains visibly above the first pass's 0.00.** These are the shape of a passing revision, not gates — the revision itself moved mean sentence length 28.3 → 17.9 and mean paragraph length 95.5 → 41.7 under them.

## The sentence-level twin (the application-grammar lane)

"One load-bearing idea per paragraph" has a sentence-level twin: **one clause spine per sentence, emphasis on the spine.** The application-grammar lane (gandr-739) parses prose into `RGL` trees and makes that checkable: clauses per sentence, embedding depth (center-embedding fails; long right-branching sentences pass — length is the wrong instrument), nominalized main actions (verbalize them), emphasis-role alignment (the emphasized span belongs on the main-clause spine, not buried in an adjunct), and given/new subject-chain continuity across a paragraph.

Its purpose is **mechanically-assisted revision**: a quantitative, deterministic _measure → locate → rewrite → re-measure_ loop for the documentation agent — findings name the construction and locate the split point, they do not gesture at word counts.
The posture that keeps prose alive: **coverage is reported, never gated** — a coverage gate makes authors write to the grammar and kills the essay; structural findings and narrow structural gates fire only where the prose already fails this doctrine, and nobody's voice lives in four-deep center-embedding.
The reference instrument (the spike probe, `MetricsLex` = `Lang` + `DictEngAbs` minus the segfault-guard exclusions, with `CodeInline`/`MathInline`/`CiteRef` read as placeholders) measures the recursion-surface arc **5.6% → 9.4% raw, 5.6% → 10.5% code-stripped** (108 → 171 sentences — the splitting itself raises coverage, shorter spines parse).
The residual failures are lexicon, not spine: unglossed technical vocabulary (`corecursion`, `pre-lowering`) and punctuation-adjacent tokens dominate the reject list, which is exactly the application grammar's lexicon gap (gandr-739) rather than prose pathology.

## Pointers

* `docs/spec/README.md` — the authoring discipline (skeleton, blocks, links, required attributes).
* `crates/workflow-docs/src/model.rs` — the normative schema; when in doubt about what validates, read it.
* [review.md](review.md) — the review doctrine and finding dispositions.
* [corpus.md](corpus.md) — corpus treatment for surfaced features (same-change rule).
* [docs.md](docs.md) — documentation economy, scoped to _which documents exist_, never fidelity.
