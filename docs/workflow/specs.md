# Workflow: identifier and prose rules surviving the corpus migration

> Read when: choosing an identifier for anything referenced across documents, beads, or tracks; citing an external work; or authoring Markdown prose in this repository that `rumdl` reflows.
> The specification corpus this file once governed left this repository on 2026-08-12 — 48 documents, including `spec:README.md` and `spec:bibliography.yml` — and the corpus-authoring discipline moved with it to the maintainer's research workspace.
> What remains below is what was always repo-wide.
> **Standing rule, whatever the task:** before recording that something does not apply, is not needed, or cannot be done, read [review.md](review.md) §"Declining is a claim too" and §"Refutations bind only with owner sign-off" — a refutation binds only with the owner's sign-off.

## There is no `spec:` tree in this repository

Nothing in this tree is authored, edited, migrated, or re-absorbed under `spec:`: the corpus lives in the maintainer's research workspace and is cited from here by the `spec:` alias only.
The corpus's own conventions — format, math, citation register, anchors, status, dispositions — govern there, not here; the corpus registry and its `docs:manifest-drift` gate retired with it.
What this repository still receives from a design is a **thin decision record** stating the outcome and what it binds — never a corpus document.
Dispatched work follows the same rule by default: a dispatch into this repository builds.

## Citations carry their own locator here

This repository holds no reference register: the `spec:bibliography.yml` register left with the corpus, and no citation in this tree resolves by key alone.
Every external work cited here carries its own locator — full title, authors, and a stable identifier (DOI, ISBN, arXiv id, HAL id, or the like) — and a corpus key is used only where the surrounding text already cites the corpus.
A claim resting on an unverified locator says so at the claim.

## Identifiers are informative, prefixed, and linkable — never a bare letter and a number

**No uninformative single-letter naming schemes.** Anything referenced anywhere that matters — a spike, an open question, a stage, an obligation, a decision — gets an identifier whose prefix abbreviates what it _is_ or the topic it belongs to, followed by a zero-padded number: `meta-spike-04`, `meta-question-19`.
A bare `S1` or `P1` names nothing, so a reader who meets it outside its home document cannot tell what it refers to, or even which document to open.

**This is not hypothetical, and the corpus has already paid for it.** `S1` meant two unrelated things at once: a spike in the metatheory roadmap and "the trusted S1 core" in the implementation track — the second of which was never defined anywhere, in three uses.
Collisions like that are invisible until someone cites one and means the other.

Three rules follow, and the third is what makes the first two worth the edit:

- **Prefix by topic, number by position.** The prefix is the disambiguator; the number is only an index within it.
  Numbering is **stable** — retiring an item leaves its number unused rather than renumbering the rest, because renumbering silently invalidates every reference taken before it.
- **Give the identifier an anchor.** An identifier nobody can link to is a search string.
  Prefer a heading per item (`### meta-spike-04`), which Obsidian resolves as a heading link into its home document (`spec:metatheory/roadmap.md §"meta-spike-04"`); a table cannot be linked into row by row, so a list of items that get cited individually should not be a table.
  Where a heading per item is too heavy, lead the item with the bolded identifier and link to the section.
- **Cite by link, not by code.** `[[roadmap#Open questions|meta-question-19]]` survives a document being reorganised and tells the reader where to go; `open question 19` does neither.

Retired schemes are the exception and stay exactly as they were: the concordance in the guards ledger exists to decode old notes, so the codes in its left-hand column are data, not usage.

## A period is a sentence end, so do not spend one on an abbreviation

`rumdl` reflows prose here with `reflow-mode = "semantic-line-breaks"`, which puts one sentence per line by splitting at a period followed by a space.
**It cannot know that an abbreviating period is not a sentence end, and it should not have to** — the ambiguity is in the prose, not in the tool.
So the abbreviating period is what gives way.

The symptom is worth knowing because it does not point at its cause: the text is fine as written, `treefmt` reflows it, and what you notice afterwards is a line shattered mid-clause with the fragment stranded below.
The fix belongs in the sentence, never in a formatter exemption.

**The trigger is narrower than "any abbreviation", and knowing which one saves rewriting prose that was never at risk.** Measured against the pinned `rumdl`: a split needs a period, a space, and then a **capital letter**.

| written         | reflowed | why                 |
| --------------- | -------- | ------------------- |
| `Prop. I.2.5.7` | splits   | capital `I` follows |
| `Def. B.2`      | splits   | capital `B` follows |
| `Fig. 3`        | survives | a digit follows     |
| `i.e. the …`    | survives | lowercase follows   |

So the hazard is the abbreviation standing before a capitalised token, which in this project's prose is almost always a **locator with a letter in it** — and the Latin abbreviations are safe in practice, because what follows them is normally lowercase.

- **Where the punctuation introduces or separates, use a colon.** `Note:`, `Caveat:`, `Definition B.2:` — a colon is never a sentence end, so nothing has to be disambiguated and nothing depends on what follows.
- **Where the period only abbreviates a locator, drop it.** The corpus already reads `Prop 5.13`, `Thm 4.10`, `Def 3.1.1`; keep locators in that form and the capital after them stops mattering.

## Two more formatter rules that rewrite structure rather than reporting it

Both were hit authoring the 2026-08-02 absorptions, both cost a round trip, and both are invisible in the source you wrote.

**A paragraph that follows a table and contains a `|` is read as an orphaned table row.** The pipe does not have to be a table pipe — a wikilink alias is enough, so `[[#some-anchor|some-anchor]]` in the first paragraph after a table trips `MD075` and the formatter refuses the file.
The fix is in the link, not the table: drop a redundant alias (an anchor whose display text repeats the anchor needs none), or escape the pipe.

**Heading levels are increment-enforced, and the formatter silently corrects rather than failing.** `MD001` forbids skipping a level, so a block of identifier headings under a `##` section must be `###` — writing `####` there does not error, it gets rewritten to `###` on the next format, and a reviewer reading the pre-format text will report a nesting problem that no longer exists.
Check the post-format file before acting on any structural finding about headings.

**And one neighbouring convention, which no longer has a gate behind it.** `§` is reserved for _corpus_ anchors, and the reference-integrity gate used to read `§N.N` as a section reference and fail it as dangling when no corpus document defined it.
The gate retired with the corpus; the convention stands, unenforced.
An external source's own sections are written out — "appendix B.11", "Example 3.6".

## Pointers

- [review.md](review.md) — the review doctrine and finding dispositions.
- [docs.md](docs.md) — documentation economy, scoped to _which documents exist_, never fidelity.
