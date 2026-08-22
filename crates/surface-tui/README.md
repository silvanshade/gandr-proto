# gandr-surface-tui

The terminal programming environment over the landed headless session engine.

The TUI is a renderer: it draws transcript frames, goal cards, and an input buffer.
Submit goes through the same read-evaluate loop as the line-editor face.
This crate does not parse, lower, type, or mark.

## Current provision

- A full-screen face on `gandr tui`.
- A smoke face on `gandr tui --smoke` that draws one frame on a test backend and prints a launch note.
- Syntax highlighting: the transcript pane paints each classified span in its role colour.
- A `HlRole` style map whose grouping matches the language-server face's `token_of_role`.
  The map is read directly; this crate does not depend on the language-server crate and does not speak its integer token legend.
  Text carrying no classified span draws in the terminal default.

## Planned but absent

- Goal-directed completion.
- Command history in the terminal face.
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
