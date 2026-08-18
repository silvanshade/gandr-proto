# surface-engine

`gandr-surface-engine` is the front end between a committed CST and the core: it lowers surface syntax to core IR, keeps source identity in a side table so the core stays span-free, and drives the session that carries checked state across submissions.

## What it currently provides

- **CST-to-core lowering** over the covered fragment, with every core node's provenance in an origin map rather than in the core syntax.
  Surface sugar is recorded as it is elaborated, so a display layer can recover what the user wrote.
- **Total lowering.** Every parseable input lowers: a syntax error or an out-of-fragment construct becomes a hole carrying a note, because a hole is a term with a typing rule rather than a parse failure with a placeholder.
  The goals report then lists each hole with its span, expected type, and local context.
- **The session.** `Session::submit` types each item, carries declarations and imports forward across submissions, maintains one typing checkpoint per item, and offers every typed definition to the certified kernel through the bridge — where the crossing observes and never decides.
- **Diagnostics.** One versioned report joins typing failures, hole goals, and entity attributes into the envelope the inspection surface projects from.
- **The item seam.** The melder-and-lowering front end implements the incremental typer's item source, so incremental checking runs against real source without depending on this crate or naming a parser.
- **Annotated surfaces.** The check-only eliminators take an optional answer type after the scrutinee (`if c -> B { … } else { … }`, `case v -> B { … }`), and the computation bind takes an optional annotation on its source (`run p : B <- t ;`).
  Both lower through the computation ascription, so they add spellings and no checker behaviour.

## Planned but not implemented

- Module sealing past the syntax front: the opaque ascription parses and the lowerer declines it by name, because reading it as transparent would expose every component while the source says sealed.
- User-declared operators reaching the grammar-extension seam, mutual-recursion blocks, and indexed constraints — each reserved in the grammar and declined here.
- Dependent motives on the eliminators, which will replace what the answer-type slot holds without moving the slot.

## Using it

`Session::submit` is the entry point for anything stateful; `lower::lower_source` and `lower::lower_source_total` are the one-shot lowering faces, strict and total respectively.

## Theoretical ideas it relies on

Call-by-push-value, bidirectional typing, elaboration to a core calculus with typed holes, and error localization and recovery as a total procedure.

## Primary resources

- Paul Blain Levy, _Call-By-Push-Value: A Functional/Imperative Synthesis_, Springer Netherlands, 2003, `doi:10.1007/978-94-007-0954-6` — the value/computation split the core IR and this crate's sort-directed lowering are organized by.
- Eric Zhao, Raef Maroof, Anand Dukkipati, Andrew Blinn, Zhiyi Pan and Cyrus Omar, _Total Type Error Localization and Recovery with Holes_, 2024, `doi:10.1145/3632910` — the marked-expression discipline behind total lowering and the no-meaningless-states posture.
- Jana Dunfield and Neel Krishnaswami, _Bidirectional Typing_, 2019, arXiv:1908.05839 — the check/synthesize discipline that makes `if` and `case` check-only here, which is what the answer-type slot exists to serve.
