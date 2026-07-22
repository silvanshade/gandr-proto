# Workflow: documentation discipline

> Read when: adding or restructuring documentation, or authoring math-dense Markdown.
> Base statement: `.agents/core/core/WORKFLOW.md` §"Documentation economy".
> Corpus trust machinery (MANIFEST/BLAKE3, edge vocabulary, authority): `docs/KNOWLEDGE.md`.

## Documentation economy — the gandr posture

Documentation accumulation is a **named project killer**: doc bloat helped sink a predecessor, and stale accumulated context confuses agents as much as humans.
The standing posture (owner, 2026-07-12): **prefer forgetting over hoarding.** A question that was set aside can be re-asked later if it ever actually arises; most never do.
Economy governs **which documents exist and where** — it is never a license to thin load-bearing content (owner, 2026-07-21, `gandr-fid.0`; the 2026-07-21 fidelity audits measured the first absorption pass at ~50-70% retention because this scoping was implicit).
Every `docs/spec/` and `docs/research/` change carries a mandatory fidelity review against its declared source set (`review.md` §"Documentation fidelity review").

* **Relevant** — every added doc is graded by the role it actually plays; never waved on by inertia.
* **Deduped** — cross-link what another doc states; never restate non-load-bearing content.
* **Concise, scannable, chunked** — lead with a summary or table; split by concern instead of appending.
* **Placed** — deep material stays off the agent orientation main-path (`AGENTS.md` §"Start here", `docs/gandr/VISION.md` §6): reachable from it, never inlined into it.
* **Fidelity overrides economy** for load-bearing content: never truncate or lossily summarize it — reorganize (chunk, relocate intact, archive) instead.
  When uncertain whether content is load-bearing, treat it as load-bearing.
  For spec absorption the bar is **superset-transfer**: the source is the floor, never the ceiling, and the acceptance test is that an implementer could build the component without opening the source tree (`docs/spec/README.md` §"Absorption fidelity").
* **Research/analysis surveys, session plans, handoffs, and adversary reports are contributor-concern**: they live in the sibling `wyrd-notes` repository (a separate local git repo beside this one), never in the tracked tree.
  What a survey _decides_ gets distilled into an ADR; the survey itself does not move into `docs/`.

Per-crate `crates/*/docs/STATUS.xml` is the lean tier, off the design-corpus main-path and unregistered in the MANIFEST.
Legacy `STATUS.md` migrates opportunistically.
CHANGELOG is retired as a document class: fold dated changes into STATUS plus beads and git history; do not create a new CHANGELOG in either format.
Per-crate ADR/METRICS/OPTIMIZATION files are legacy material outside the three prose classes, and TODO files are retired because beads tracks work.

## Specification corpus and the doc tool

The design corpus is migrating to a validated XML component model (`gandr-fcw.8`, owner-confirmed).
What has landed:

* **The doc tool.** `crates/workflow-docs` (package `gandr-workflow-docs`, a **provisional** name — owner ratification is pending, `gandr-wvd.17`) holds the typed model, parser/validator, and canonical XML formatter.
  Parsing _is_ validation: it enforces define-once, `term`/`cite`/`ref` resolution, ID uniqueness, and status presence.
* **The corpus root.** `docs/spec/` holds the component files plus the shared `component-vocabulary.xml` and `index.xml`.
  Canonical XML formatting is wired into treefmt as the `docs-xml` formatter (`gandr-workflow-docs fmt` over `docs/spec/*.xml`).
* **The citation register.** `docs/spec/refs.yml` is the central Hayagriva bibliography — 379 entries, one per row of the register `docs/research/bibliography-v2.md` (`gandr-fcw.10`).
  It is a **derived artifact**: the generator is `scripts/refs-yml/` (typed Nushell); to change a citation, edit the register and re-derive, never hand-edit `refs.yml`.
* **The prose document classes** (`gandr-712`).
  The same tool grows three classes beyond the component corpus, sharing one minimal block/inline substrate (`section`, `prose`, `list`, `table`, `code`; `inline-code`, `label`/`ref` coined anchors, `cite` register keys) and the same parse-is-validate discipline (banner presence, status presence, label define-once, label/cite resolution, per-class schema):
  + **research records** — `docs/research/*.xml` (`<research-record>`): status banner, sections, tables, code, coined-label anchors (`R1`/`HZ-1`/`O1`), register-key citations.
  + **workflow docs** — `docs/workflow/*.xml` (`<workflow-doc>`): a required `read-when` banner and rule/convention lists.
  + **the per-crate lean tier** — `crates/*/docs/STATUS.xml` (`<crate-status>`): a `crate` scope, dated sections, current-state prose.
  `check-docs` (`mise run docs:check-classes`) is the class gate; the `docs-xml` formatter covers all class roots.
  The status lifecycle is the shared five-value vocabulary (`built | partial | adopted-unbuilt | design-pass | dormant`); a research proposal under review authors as `design-pass`, its human status phrase carried in the banner.

**Authoring policy — XML-first (`gandr-712`).** Once a class has an XML home, a new document in that class authors as **XML**, not Markdown.
Markdown is the **legacy tail only**: do not add a new `.md` file in a class that now has an XML home (research, workflow, per-crate STATUS).
CHANGELOG has no XML class and is retired: do not create one in either format; fold dated changes into STATUS plus beads and git history.
Existing research/workflow/STATUS Markdown migrates opportunistically when touched, never in a mass sweep; existing CHANGELOG Markdown folds into STATUS when touched.
`.md` and `.xml` coexist until those tails are gone.
The math- and symbol-dense Markdown conventions below stay in force for the un-migrated tail — they are the workarounds for Markdown's lack of first-class math, and they retire with the last `.md` in a migrated class.
Repository entrypoints whose consumers require Markdown names (`README.md`, `AGENTS.md`, `CLAUDE.md`) are routing adapters, not authored-document classes: keep them thin and point substantive material into the XML homes.
Do not add new top-level Markdown guidance.
Route process material to `docs/workflow/*.xml`, design material to `docs/spec/*.xml`, and staging/design-study material to `docs/research/*.xml`; the legacy `docs/WORKFLOW.md` umbrella migrates to `docs/workflow/index.xml`.

The static-HTML render pipeline the resolution specifies — Typst math compiled to MathML Core, `typst-fletcher` diagrams to SVG, progressive-enhancement WebComponent islands — is the design target, not yet built; describe it as design, not as a shipped capability.
The pre-`gandr-fcw.8` proposal-lifecycle model below (proposal files as document classes, manual absorption) is superseded by that status-attributed component model, and is retained only until the tool subsumes it.

### Cite with a resolvable locator

Every external-literature citation in a notes, analysis, or spec doc records a **resolvable locator** — a DOI, arXiv id, or stable URL — at first mention.
Name-only citations are the recorded hazard that forced `gandr-fcw.10`'s bulk re-verification of the register; a locator at first citation is what keeps a claim checkable without archaeology.
`refs.yml` is the canonical home for those locators: a doc cites the register key and the register carries the DOI/arXiv/URL.

### Wyrd ADRs are source material, never citable authorities

New gandr docs and specs do **not** cite wyrd ADR numbers (`ADR-NN`) or wyrd bead ids (owner rule, 2026-07-21).
Much of the wyrd decision record is superseded by reboot decisions; a numbered citation quietly codifies the retired idea into the new corpus and forces every reader through stale context to resolve it (the motivating case: a wyrd-era "evidence layer" reservation rode two reboot design studies verbatim before it was caught — the concept had been superseded by the B6/B7 backbone).
Where wyrd ADR content is load-bearing, **inline the applicable content restated in current terms**, and only where it actually applies to the current context.
If provenance matters, a distilled prose line ("adapted from the wyrd interpreter-architecture record") suffices; the wyrd tree stays readable as source-material history through the checkout, and `docs/research/crate-port-map.md` carries the wyrd→reboot reconciliation.
Landed research records that already carry `ADR-NN` citations are historical documents — sync them opportunistically when touched; do not mass-rewrite them.

A proposal file is always in exactly one state, named in its status banner, and it moves — stale ACTIVE banners were a principal drift source:

* **Active** — being designed or decided; cites its bead and intended ADR.
* **Adopted** — its decision face (ADR) landed; the banner names the ADR and the proposal becomes the design record behind it.
  **Amend the banner with an as-built note at each implementation landing** — a proposal whose content has shipped must say so.
  The manual presents adopted-but-unbuilt designs compactly (Part IV, decided directions); the exhaustive treatment waits for construction.
* **Implemented** — the surface is built: the manual **absorbs the enduring content exhaustively** — the owning chapter presents it and the proposal's banner gains a `> **Manual:** …` pointer line, while the file stays in place as the authoritative design record (absorption never deletes, moves, or truncates the spec; where a chapter and the corpus disagree, the corpus is correct).
  Crate-scoped proposals may instead move to the owning crate's `docs/` (precedent: the TUI proposal); purely historical ones are marked as such.
  Either way the proposal stops being a to-do: its endgame is implementation plus absorption into the manual, the durable user-facing home.
* **Dormant / retired** — no activity and no owner: mark dormant with a reader caution (precedent: skuld), or delete outright when the content is re-derivable — prefer forgetting.

Research surveys never enter `docs/` at all: distill what was _decided_ into an ADR and keep the survey in the notes repo.

## Formatters and linters are best-effort

A formatter/linter must never be satisfied at the cost of an artifact's **fidelity**; relax or scope the tool (raise the limit, disable the rule, exclude the path), never alter content to appease it (`.agents/core/core/WORKFLOW.md` §"Formatters…" + core/HAZARDS.md H8).
Hand-authored corpus docs (`docs/gandr/`) follow the formatter by default; the content-mutating `typos` and `sizelint` run tree-wide with targeted fidelity excludes (`*.typ`, `*.agda`, `*.agda-lib` in `treefmt.toml`, mirrored in `typos.toml` so standalone editor/CLI runs are safe).

## Authoring math- and symbol-dense Markdown

Markdown has no first-class math: `*`, `_`, `[`, `^` are structural tokens.
The one **corrupting** hazard is a bare `*` used as an operator (e.g. `d*G`): CommonMark pairs intraword `*`s into emphasis and `rumdl fmt` rewrites them **silently while reporting success** — never run the formatter on a doc that still holds bare-`*` math (if you did, `git checkout` the file, wrap the math, retry).
A capitalised name-like `[Label]` bracket is the only other trip (MD052); plain `_` subscripts (`U_r`), `^`, and standalone symbols (`Σ`, `↠`, `s²=0`) are inert.

The convention (authored docs are clean by construction):

* **Inline math / `*`-bearing operators** — default to `$…$` LaTeX (renders as real math, is typst-portable, rumdl-inert): `$d^{*}G$`, `$s^2 = 0$`.
  A backtick code span is the fallback for code-like identifiers.
  Wrap the _whole_ expression containing the `*`.
  Math holding a literal `|` inside a pipe-table cell needs `\|`.
* **Display math** — a fenced code block tagged `math`; **never** bare `$$…$$` (MD013 reflow joins it to one line).
* **Editorial bracket-notes** (`[corrected: …]`) — plain prose; MD052 `shortcut-syntax = false` keeps them inert.
