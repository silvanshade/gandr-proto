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
- **The description route and the matcher seam.** A `data`, `codata` or `sign` block elaborates to a description, and the description elaborates into the cell engine's store: a frame cell per constructor, a cell per admitted `rule` member, the cell a circuit rule's wiring derives, and the η cell a declaration licenses.
  This crate is also the **supply point** at which the circuit-algebra crate's embedding matcher reaches that route: a circuit rule's body is read as a wiring diagram and matched into the other rules of its own description, so the route records where one rule occurs inside another.
  The seam lives here because it is the one place above both crates — the matcher crate depends on the engine crate, so the reverse edge is impossible rather than merely forbidden.
- **Annotated surfaces.** The check-only eliminators take an optional answer type after the scrutinee (`if c -> B { … } else { … }`, `case v -> B { … }`), and the computation bind takes an optional annotation on its source (`run p : B <- t ;`).
  Both lower through the computation ascription, so they add spellings and no checker behaviour.
- **Pattern compilation.** The ordered arms a programmer writes become the tag-indexed arms the core eliminator carries: constructor patterns nested to any depth, wildcard and binder sub-patterns, as-binders, and pattern holes.
  A hole in a pattern is an unfinished **test**, so it is neither satisfied nor refuted: the constructor tags it shadows become stuck rather than falling through to the arms written after it, and the arms settled before it are untouched.
  A tag no arm reaches stays a missing-arm hole, and the note is what tells the two apart.
  Two arms sharing one constructor head are declined by name, because reaching the second needs an arm body two branches can jump to.

## Planned but not implemented

- Module sealing past the syntax front: the opaque ascription parses and the lowerer declines it by name, because reading it as transparent would expose every component while the source says sealed.
- User-declared operators reaching the grammar-extension seam, mutual-recursion blocks, and indexed constraints — each reserved in the grammar and declined here.
- Dependent motives on the eliminators, which will replace what the answer-type slot holds without moving the slot.
- Join points, and with them the pattern forms that need one: a top-level catch-all, an or-pattern with distinguishable alternatives, two arms at one constructor head, and the literal, tuple, record and list columns.
  Each is declined by name rather than dropped, so the boundary is visible in the decline instead of in a silently missing branch.

## Using it

`Session::submit` is the entry point for anything stateful; `lower::lower_source` and `lower::lower_source_total` are the one-shot lowering faces, strict and total respectively.

## Theoretical ideas it relies on

Call-by-push-value, bidirectional typing, elaboration to a core calculus with typed holes, error localization and recovery as a total procedure, live pattern matching in which an unfinished pattern makes a match indeterminate rather than absent, pattern-matrix compilation to a decision structure, and wiring diagrams with embedding-based sub-diagram matching.

## Primary resources

- Paul Blain Levy, _Call-By-Push-Value: A Functional/Imperative Synthesis_, Springer Netherlands, 2003, `doi:10.1007/978-94-007-0954-6` — the value/computation split the core IR and this crate's sort-directed lowering are organized by.
- Eric Zhao, Raef Maroof, Anand Dukkipati, Andrew Blinn, Zhiyi Pan and Cyrus Omar, _Total Type Error Localization and Recovery with Holes_, 2024, `doi:10.1145/3632910` — the marked-expression discipline behind total lowering and the no-meaningless-states posture.
- Jana Dunfield and Neel Krishnaswami, _Bidirectional Typing_, 2019, arXiv:1908.05839 — the check/synthesize discipline that makes `if` and `case` check-only here, which is what the answer-type slot exists to serve.
- Yongwei Yuan, Scott Guest, Eric Griffis, Hannah Potter, David Moon and Cyrus Omar, _Live Pattern Matching with Typed Holes_, _Proceedings of the ACM on Programming Languages_ 7 (2023), issue OOPSLA1, pages 609–635, `doi:10.1145/3586048` — the three-valued reading of a branch against a scrutinee, and the indeterminacy an unfinished pattern carries, which is what makes a hole arm stuck rather than dropped.
- Luc Maranget, _Warnings for Pattern Matching_, _Journal of Functional Programming_ 17:3 (2007), pages 387–421, `doi:10.1017/s0956796807006223` — the pattern-matrix reading of ordered arms, with the usefulness recursion this crate's coverage analysis instantiates and the specialize/default step its arm compilation follows.
- Filippo Bonchi, Fabio Gadducci, Aleks Kissinger, Paweł Sobociński and Fabio Zanasi, _String Diagram Rewrite Theory I: Rewriting with Frobenius Structure_, _Journal of the ACM_ 69:2 (2022), article 14, `doi:10.1145/3502719` — the convex sub-diagram matching discipline the circuit matcher this crate supplies is built on.
