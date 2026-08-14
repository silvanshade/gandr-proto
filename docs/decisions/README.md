# Decision records

> Read when: recording the outcome of a design decision that constrains code in this repository, or looking for why a constraint on the kernel, the engine, or the format exists.

**A decision record states an outcome and what it binds — never the design exploration that produced it.** The specification corpus is closed to new documents and its deep material lives in the maintainer's research workspace; what this repository receives from a design is the thin record, so that the code can be explained without reaching outside the repository.

## What belongs here

- The outcome, in one statement a reader can act on.
- What it constrains: which crates, which invariants, which formats, which future work is now forbidden or now required.
- The staged path, where the outcome lands over more than one change.
- The obligations the outcome creates, each one owned by a tracker item.

## What does not

- The survey, the comparison, the alternatives weighed, the reading behind it.
  Those live in the research workspace; a record that reproduces them is a corpus document under another name.
- References to anything outside this repository's own artifacts, other than published works cited in full.

## Conventions

- **One file per decision, named for its outcome** — `sharing-in-the-engine.md`, not `adr-0007.md`.
  A reader looking for a constraint guesses the outcome, not a serial number.
- **Identifiers carry a topic prefix and a zero-padded number** — `share-adopt-rung-01`, never `R1` or `S3`.
  The rule is the repository's standing one: an identifier that travels must still resolve when it arrives somewhere with no context.
- **References are complete at the claim** — title, authors, year, and a stable identifier.
  This repository holds no reference register, so a citation carries its own locator.
- **A record is rewritten, never amended.** New knowledge rewrites the sentence it changes; git holds what the record said before.
