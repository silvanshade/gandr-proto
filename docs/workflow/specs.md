# Workflow: authoring the spec corpus (docs/spec)

> Read when: creating, editing, or re-absorbing any `docs/spec/` component; the posture also governs `docs/research/` records.
> Base practice: `docs/spec/README.md` (the authoring discipline), [review.md](review.md) §"Documentation fidelity review" (the mandatory review), [docs.md](docs.md) (the economy posture).

## The priority order

1. **Technical precision and exhaustive detail** — implementation details, normative signatures, proof obligations, theorem numbers, gate conditions — are the single most important content to retain across these documents.
2. Natural prose and conversational tone are an aspiration beyond that.
   When the two axes conflict, the first wins, always.

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

## Pointers

* `docs/spec/README.md` — the authoring discipline (skeleton, blocks, links, required attributes).
* `crates/workflow-docs/src/model.rs` — the normative schema; when in doubt about what validates, read it.
* [review.md](review.md) — the review doctrine and finding dispositions.
* [corpus.md](corpus.md) — corpus treatment for surfaced features (same-change rule).
* [docs.md](docs.md) — documentation economy, scoped to _which documents exist_, never fidelity.
