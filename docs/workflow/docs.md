# Workflow: documentation discipline

> Read when: adding or restructuring documentation, or authoring math-dense Markdown.
> Corpus authority and authoring discipline: [specs.md](specs.md).
> The MANIFEST/BLAKE3 registration machinery and its drift gate retired when the corpus left this repository.
> **Standing rule, whatever the task:** before recording that something does not apply, is not needed, or cannot be done, read [review.md](review.md) §"Declining is a claim too" and §"Refutations bind only with owner sign-off" — a refutation binds only with the owner's sign-off.

## Documentation economy — the gandr posture

Documentation accumulation is a **named project killer**: doc bloat helped sink a predecessor, and stale accumulated context confuses agents as much as humans.
The standing posture (owner, 2026-07-12): **prefer forgetting over hoarding.** A question that was set aside can be re-asked later if it ever actually arises; most never do.
Economy governs **which documents exist and where** — it is never a license to thin load-bearing content (owner, 2026-07-21, `gandr-fid.0`; the 2026-07-21 fidelity audits measured the first absorption pass at ~50-70% retention because this scoping was implicit).
Every `spec:` change carries a mandatory fidelity review against its declared source set (`review.md` §"Documentation fidelity review").

- **Relevant** — every added doc is graded by the role it actually plays; never waved on by inertia.
- **Deduped** — cross-link what another doc states; never restate non-load-bearing content.
- **Concise, scannable, chunked** — lead with a summary or table; split by concern instead of appending.
- **Placed** — deep material stays off the agent orientation main-path (`AGENTS.md` §"Start here", `docs/gandr/VISION.md` §6): reachable from it, never inlined into it.
- **Fidelity overrides economy** for load-bearing content: never truncate or lossily summarize it — reorganize (chunk, relocate intact, archive) instead.
  When uncertain whether content is load-bearing, treat it as load-bearing.
  For spec absorption the bar is **superset-transfer**: the source is the floor, never the ceiling, and the acceptance test is that an implementer could build the component without opening the source tree ([specs.md](specs.md)).
- **Research/analysis surveys, session plans, handoffs, and adversary reports are contributor-concern**: they live in the contributor's private workspace, never in the tracked tree (`AGENTS.md` §"Commits and publishable history").
  What a survey _decides_ gets distilled into the design record; the survey itself does not move into `docs/`.

Per-crate `crates/*/docs/STATUS.xml` is the lean tier, off the design-corpus main-path and unregistered in the MANIFEST.
Legacy `STATUS.md` migrates opportunistically.
CHANGELOG is retired as a document class: fold dated changes into STATUS plus beads and git history; do not create a new CHANGELOG in either format.
Per-crate ADR/METRICS/OPTIMIZATION files are legacy material outside the three prose classes, and TODO files are retired because beads tracks work.

## Specification corpus and the doc tool

The **live design corpus is Markdown** under `spec:` — four tracks with their sub-documents and roadmaps, cited against `spec:bibliography.yml` ([specs.md](specs.md) is the authoring discipline).
It is held outside this repository and cited by the alias.
It is the authority; nothing else describes the design normatively.

Beside it sits the **prose document-class tool**, `crates/workflow-docs` (package `gandr-workflow-docs`, a **provisional** name — owner ratification is pending, `gandr-wvd.17`), which validates three XML classes (`gandr-712`).
They share one minimal block/inline substrate (`section`, `prose`, `list`, `table`, `code`; `inline-code`, `label`/`ref` coined anchors, `cite` bibliography keys) and one parse-is-validate discipline (banner presence, status presence, label define-once, label/cite resolution, per-class schema):

- **research records** — `<research-record>`: status banner, sections, tables, code, coined-label anchors (`R1`/`HZ-1`/`O1`), bibliography citations.
  **This class has no home in this repository any more**: `docs/research/` left with the corpus, and the class definition survives only for the parked tool's revisit.
- **workflow docs** — `docs/workflow/*.xml` (`<workflow-doc>`): a required `read-when` banner and rule/convention lists.
- **the per-crate lean tier** — `crates/*/docs/STATUS.xml` (`<crate-status>`): a `crate` scope, dated sections, current-state prose.

The status lifecycle is the shared five-value vocabulary (`built | partial | adopted-unbuilt | design-pass | dormant`); a research proposal under review authors as `design-pass`, its human status phrase carried in the banner.

**The tool is parked** (owner directive, 2026-07-30): the crate is commented out of the workspace pending a complete revisit, so its class gate (`check-docs`) and its `docs-xml` treefmt formatter are both disabled and the wiring is kept verbatim for the revisit.
Until it returns, the three tracked `.xml` documents are unformatted and unvalidated by any gate — treat that as the reason to keep the Markdown tail rather than as licence to author more XML by hand.

**Authoring policy (`gandr-712`).** A class with an XML home takes new documents as **XML**, not Markdown; Markdown is the legacy tail there (research, workflow, per-crate STATUS), migrating opportunistically when touched, never in a mass sweep.
The design corpus is the exception and is not on that path: it authors as Markdown under `spec:`.
CHANGELOG has no XML class and is retired: do not create one in either format; fold dated changes into STATUS plus beads and git history.
`.md` and `.xml` coexist until those tails are gone.
The math- and symbol-dense Markdown conventions below stay in force for the un-migrated tail and for the design corpus — they are the workarounds for Markdown's lack of first-class math.
Repository entrypoints whose consumers require Markdown names (`README.md`, `AGENTS.md`, `CLAUDE.md`) are routing adapters, not authored-document classes: keep them thin and point substantive material into the class homes.
Do not add new top-level Markdown guidance.
Route process material to `docs/workflow/` and design material to `spec:`; the legacy `docs/WORKFLOW.md` umbrella migrates to `docs/workflow/index.xml`.
**Staging and design-study material has no destination here** — that class left with the research corpus and is authored in the maintainer's private research workspace, from which only what it decides comes back.

A static-HTML render target for the corpus — Typst math compiled to MathML Core, `typst-fletcher` diagrams to SVG, progressive-enhancement WebComponent islands — remains a design target with nothing built; describe it as design, not as a shipped capability.
The proposal-lifecycle model below (proposal files as document classes, manual absorption) is the pre-`gandr-fcw.8` model, superseded by the status-attributed class model and retained only until the tool subsumes it.

### Cite with a resolvable locator

Every external-work citation in this repository is **complete at the claim**: the full title, its authors, its year, and a stable identifier such as a DOI, arXiv id, ISBN, or stable URL.
This repository holds no reference register, so a citation here carries its own locator.
The `spec:bibliography.yml` register left with the specification corpus and governs citations in that external corpus; no citation in this tree depends on it to resolve.

### Rewrite documents in place

**A document is rewritten, never amended.** New knowledge rewrites the sentence it changes; git holds what the document said before.

### Wyrd ADRs are source material, never citable authorities

New gandr docs and specs do **not** cite wyrd ADR numbers (`ADR-NN`) or wyrd bead ids (owner rule, 2026-07-21).
Much of the wyrd decision record is superseded by reboot decisions; a numbered citation quietly codifies the retired idea into the new corpus and forces every reader through stale context to resolve it (the motivating case: a wyrd-era "evidence layer" reservation rode two reboot design studies verbatim before it was caught — the concept had been superseded by the B6/B7 backbone).
Where wyrd ADR content is load-bearing, **inline the applicable content restated in current terms**, and only where it actually applies to the current context.
If provenance matters, a distilled prose line ("adapted from the wyrd interpreter-architecture record") suffices, and the wyrd tree stays readable as source-material history through the checkout.
The wyrd→reboot reconciliation that used to sit beside it left with the research corpus; `crates/` is now the reconciliation, because a crate that exists is the record of what its predecessor became.
Landed research records that already carry `ADR-NN` citations are historical documents — sync them opportunistically when touched; do not mass-rewrite them.

Landed proposal files still carry the retired lifecycle banners (**Active** / **Adopted** / **Implemented** / **Dormant**); the model is superseded by the status-attributed class model above and is not authored to.
Three of its obligations survive it and still bind on any banner met in place: amend the banner with an as-built note at each implementation landing; absorption never deletes, moves, or truncates the spec, and where a chapter and the corpus disagree, the corpus is correct; and a dormant file is marked with a reader caution or deleted outright when re-derivable — prefer forgetting.

Research surveys never enter `docs/` at all: distill what was _decided_ into the design record and keep the survey in the contributor workspace.

## Formatters and linters are best-effort

A formatter/linter must never be satisfied at the cost of an artifact's **fidelity**; relax or scope the tool (raise the limit, disable the rule, exclude the path), never alter content to appease it .
Hand-authored corpus docs (`docs/gandr/`) follow the formatter by default; the content-mutating `typos` and `sizelint` run tree-wide with targeted fidelity excludes (`*.typ`, `*.agda`, `*.agda-lib` in `treefmt.toml`, mirrored in `typos.toml` so standalone editor/CLI runs are safe).

## The unordered-list marker is `-`

Every unordered list takes `-`, at every nesting depth (owner ruling, 2026-08-10); `rumdl.toml` `[ul-style] style = "dash"` is what enforces it, and MD004 under `treefmt` is where a stray marker is caught.
The two reasons are both about what the character does when it is _not_ being read as a marker.
`*` is also the emphasis token, so a leading `*` and an emphasis span opened later on the same line are one character playing two roles — the ambiguity the section below spends its length on.
And `*` is auto-paired by editors, which completes the typed marker into `**` and turns a new list item into a dangling strong-emphasis run.

The retired `sublist` style additionally made a marker a function of its item's **depth** (`*` at the top level, `+` beneath it), so re-indenting a block rewrote markers that had nothing else wrong with them.
One marker everywhere decouples the two: an indent change stays an indent change.

Markdown has no first-class math: `*`, `_`, `[`, `^` are structural tokens.
The one **corrupting** hazard is a bare `*` used as an operator (e.g. `d*G`): CommonMark pairs intraword `*`s into emphasis and `rumdl fmt` rewrites them **silently while reporting success** — never run the formatter on a doc that still holds bare-`*` math (if you did, `git checkout` the file, wrap the math, retry).
A capitalised name-like `[Label]` bracket is the only other trip (MD052); plain `_` subscripts (`U_r`), `^`, and standalone symbols (`Σ`, `↠`, `s²=0`) are inert.

The convention (authored docs are clean by construction):

- **Inline math / `*`-bearing operators** — default to `$…$` LaTeX (renders as real math, is typst-portable, rumdl-inert): `$d^{*}G$`, `$s^2 = 0$`.
  A backtick code span is the fallback for code-like identifiers.
  Wrap the _whole_ expression containing the `*`.
  Math holding a literal `|` inside a pipe-table cell needs `\|`.
- **Display math** — `$$…$$`, which is what the corpus's own conventions prescribe (`spec:README.md`).
  A balanced block is inert to the reflow wherever it sits: its own paragraph, tight against prose, inside a list item or a blockquote, all on one line, or carrying sentence-looking periods in its body.
  An **unbalanced** `$$` is the live hazard — with no closing delimiter the block stops being recognised, the reflow absorbs it into the surrounding prose line, and `rumdl check` reports success on the result.
- **Editorial bracket-notes** (`[corrected: …]`) — plain prose; MD052 `shortcut-syntax = false` keeps them inert.
- **Write every paragraph expecting the line breaks to fall at sentence boundaries** — that is what `reflow-mode = "semantic-line-breaks"` means, and inline formatting must not straddle one of those boundaries.
  A sentence's period therefore sits _outside_ the emphasis that ends it: write `**the rule**.` rather than `**the rule.**` (both are code spans here, so this bullet does not demonstrate the effect on itself).
  **Dangling formatting** — an emphasis span that swallows the full stop it ends on — hides that boundary from the splitter, which then leaves the sentences joined on one line instead of splitting them there.
  The outcome is stable rather than stuck: `rumdl check` accepts the joined line and the merge wall stays green, so the cost is a paragraph that stops reading one sentence per line, not a failing gate.
