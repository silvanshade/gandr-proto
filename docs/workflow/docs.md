# Workflow: documentation discipline

> Read when: adding or restructuring documentation, or authoring math-dense Markdown.
> Base statement: `.agents/core/core/WORKFLOW.md` §"Documentation economy".
> Corpus trust machinery (MANIFEST/BLAKE3, edge vocabulary, authority): `docs/KNOWLEDGE.md`.

## Documentation economy — the gandr posture

Documentation accumulation is a **named project killer**: doc bloat helped sink a predecessor, and stale accumulated context confuses agents as much as humans.
The standing posture (owner, 2026-07-12): **prefer forgetting over hoarding.** A question that was set aside can be re-asked later if it ever actually arises; most never do.

* **Relevant** — every added doc is graded by the role it actually plays; never waved on by inertia.
* **Deduped** — cross-link what another doc states; never restate non-load-bearing content.
* **Concise, scannable, chunked** — lead with a summary or table; split by concern instead of appending.
* **Placed** — deep material stays off the agent orientation main-path (`AGENTS.md` §"Start here", `docs/gandr/VISION.md` §6): reachable from it, never inlined into it.
* **Fidelity overrides economy** for load-bearing content: never truncate or lossily summarize it — reorganize (chunk, relocate intact, archive) instead.
  When uncertain whether content is load-bearing, treat it as load-bearing.
* **Research/analysis surveys, session plans, handoffs, and adversary reports are contributor-concern**: they live in the sibling `wyrd-notes` repository (a separate local git repo beside this one), never in the tracked tree.
  What a survey _decides_ gets distilled into an ADR; the survey itself does not move into `docs/`.

Per-crate `crates/*/docs/` (STATUS, crate ADR, CHANGELOG — no TODO.md; beads tracks that) are a distinct lean tier, off the design-corpus main-path and unregistered in the MANIFEST.

## Specification corpus and the doc tool

The design corpus is migrating to a validated XML component model (`gandr-fcw.8`, owner-confirmed).
What has landed:

* **The doc tool.** `crates/workflow-docs` (package `gandr-workflow-docs`, a **provisional** name — owner ratification is pending, `gandr-wvd.17`) holds the typed model, parser/validator, and canonical XML formatter.
  Parsing _is_ validation: it enforces define-once, `term`/`cite`/`ref` resolution, ID uniqueness, and status presence.
* **The corpus root.** `docs/spec/` holds the component files plus the shared `component-vocabulary.xml` and `index.xml`.
  Canonical XML formatting is wired into treefmt as the `docs-xml` formatter (`gandr-workflow-docs fmt` over `docs/spec/*.xml`).
* **The citation register.** `docs/spec/refs.yml` is the central Hayagriva bibliography — 379 entries, one per row of the register `docs/research/bibliography-v2.md` (`gandr-fcw.10`).
  It is a **derived artifact**: the generator is `scripts/refs-yml/` (typed Nushell); to change a citation, edit the register and re-derive, never hand-edit `refs.yml`.

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
