# gandr-surface-repl

The read-evaluate loop over the landed headless session engine.

The line editor owns raw text, editing, and history.
On submit the validator asks the parser whether the buffer is parse-complete.
Holes are typeable, so a hole is not incompleteness.
A complete buffer is handed to `Session::submit`, which already carries typed context across lines through its checkpoint set.
This crate encodes the resulting report and outcomes into a transcript.
It does not parse, lower, type, or mark.

## Current provision

- Parse-completeness via the parser's public `expected` query.
- Session submit and transcript encoding for definitions, expressions, located `annotate-snippets` diagnostics with stable codes, and goals.
- A batch face over standard input for non-interactive transcripts.
- An interactive face on a line editor for a terminal.

## Planned but absent

- Goal-directed completion (a synthetic hole at the cursor).
- Highlight spans: this crate consumes `HlSpan` and does not produce one.
  Semantic tokens come from the language-server face.
- Persistent content-addressed history.
  The interactive face keeps history in memory.

## Using it

```console
printf '42\n' | gandr
```

Bare `gandr` on a pipe runs the batch loop.
Bare `gandr` on a terminal runs the line editor.
`:q` leaves the interactive face.

## Theoretical ideas relied on

Call-by-push-value evaluation of hole-free rigid terms, and material-obligation parse completeness (holes are terms, not parse failures).

## Primary references

- Paul Blain Levy, _Call-By-Push-Value: A Functional/Imperative Synthesis_, Springer Netherlands, 2003, `doi:10.1007/978-94-007-0954-6` — the value and computation split the session evaluates.
- David Moon, Andrew Blinn, Thomas J. Porter and Cyrus Omar, _Syntactic Completions with Material Obligations_, 2025, `doi:10.1145/3763182` — the obligation query the validator reads for parse completeness.
