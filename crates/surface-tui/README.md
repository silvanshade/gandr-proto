# gandr-surface-tui

The terminal programming environment over the landed headless session engine.

The TUI is a renderer: it draws transcript frames, goal cards, and an input buffer.
Submit goes through the same read-evaluate loop as the line-editor face.
This crate does not parse, lower, type, or mark.

## Current provision

- A full-screen face on `gandr tui`.
- A smoke face on `gandr tui --smoke` that draws one frame on a test backend and prints a launch note.
- Input, transcript, and goals panes over the shared loop.

## Planned but absent

- Semantic-token highlighting.
  The highlighter consumes `HlSpan` produced by the language-server face.
- Goal-directed completion.
- PTY job panes.

## Using it

```console
gandr tui --smoke
gandr tui
```

The smoke command is the observable launch path.
The interactive command needs a terminal.

## Theoretical ideas relied on

The same call-by-push-value session and material-obligation completeness as the read-evaluate loop.

## Primary references

- Paul Blain Levy, _Call-By-Push-Value: A Functional/Imperative Synthesis_, Springer Netherlands, 2003, `doi:10.1007/978-94-007-0954-6`.
- David Moon, Andrew Blinn, Thomas J. Porter and Cyrus Omar, _Syntactic Completions with Material Obligations_, 2025, `doi:10.1145/3763182`.
