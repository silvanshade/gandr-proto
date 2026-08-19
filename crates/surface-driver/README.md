# gandr

The toolchain's process boundary: the `gandr` binary.

The driver itself decides almost nothing.
It reads the command line and hands work to the face that owns it.
Everything the program does between those two points belongs to the crates below it.

Naming note: the package is `gandr` and its directory is `crates/surface-driver`.
The package name is what a user types; the directory name places the crate in the surface tier.

## Current provision

- Bare `gandr` is the read-evaluate loop.
  On a terminal it uses a line editor.
  On a pipe it runs a batch transcript.
- `gandr tui` is the terminal programming environment.
  `gandr tui --smoke` draws one test-backend frame and prints a launch note.
- `gandr <file>` runs one gandr source file.
  The path goes to `gandr-runtime-ffi`'s source entry, which lowers, links, prelude-checks and runs the program under the combined native and shell host.
- `gandr lsp` serves the language-server protocol on standard input and output.
- `gandr lsp --capabilities` prints the advertised initialize result, including the semantic-token legend.
- A returned script value is written once to standard output.
  Non-returning outcomes produce no result output.
- `gandr --help` and `gandr -h` print the usage text and leave successfully.
- The script-runner outcome-to-status contract:

  | outcome                                                   | status                        |
  | --------------------------------------------------------- | ----------------------------- |
  | the program returned a value                              | 0                             |
  | the program called `proc.exit`                            | that code, reduced modulo 256 |
  | the machine or the host did not complete the run          | 1                             |
  | a usage error, or a source that never reached the machine | 2                             |

  Reducing the exit code modulo 256 is what keeps a language-level exit code from being silently truncated by the operating system into a different one.

## Planned but absent

- The `mcp`, `fmt` and `build` faces.
  None has an implementing crate in the tree.

These are deferred rather than parked and ready.
They arrive with their dependency edges, not by uncommenting a line.

## Using it

```console
printf '42\n' | gandr
gandr tui --smoke
gandr path/to/program.gandr
gandr lsp --capabilities
```

Build it from the workspace with `cargo run -p gandr -- <file>`, or run the crate's own behaviour suite with `cargo nextest run -p gandr`.

## Theoretical ideas relied on

None of its own.
The driver is a process-boundary adapter: the language-level ideas it exposes are stated by the crates it calls.

## Primary references

- Paul Blain Levy, _Call-By-Push-Value: A Functional/Imperative Synthesis_, Springer Netherlands, 2003, `doi:10.1007/978-94-007-0954-6` — the value and computation split whose terminal cases the script-runner face classifies into process exit statuses.
