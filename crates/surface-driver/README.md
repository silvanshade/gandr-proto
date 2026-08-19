# gandr

The toolchain's process boundary: the `gandr` binary, the script-runner face, and the language-server face.

The driver itself decides almost nothing.
It reads the command line, hands work to the crate that owns it, and turns the outcome into a process exit status.
Everything the program does between those two points belongs to the crates below it.

Naming note: the package is `gandr` and its directory is `crates/surface-driver`.
The package name is what a user types; the directory name places the crate in the surface tier.

## Current provision

- `gandr <file>` runs one gandr source file.
  The path goes to `gandr-runtime-ffi`'s source entry, which lowers, links, prelude-checks and runs the program under the combined native and shell host.
- `gandr lsp` serves the language-server protocol on standard input and output.
- `gandr lsp --capabilities` prints the advertised initialize result, including the semantic-token legend.
- A returned value is written once to standard output.
  Non-returning outcomes produce no result output.
- `gandr --help` and `gandr -h` print the usage text and leave successfully.
- The outcome-to-status contract, which is the crate's real content:

  | outcome                                                   | status                        |
  | --------------------------------------------------------- | ----------------------------- |
  | the program returned a value                              | 0                             |
  | the program called `proc.exit`                            | that code, reduced modulo 256 |
  | the machine or the host did not complete the run          | 1                             |
  | a usage error, or a source that never reached the machine | 2                             |

  Reducing the exit code modulo 256 is what keeps a language-level exit code from being silently truncated by the operating system into a different one.

## Planned but absent

- The REPL, which waits on a line-editor decision wired to the landed grammar, parser and syntax crates.
- The `tui`, `mcp`, `fmt` and `build` faces.
  None has an implementing crate in the tree.

These are deferred rather than parked and ready.
`Cargo.toml` records which crate each would need.
They arrive with their dependency edges, not by uncommenting a line.

## Using it

```console
gandr path/to/program.gandr
gandr lsp --capabilities
```

Build it from the workspace with `cargo run -p gandr -- <file>`, or run the crate's own behaviour suite with `cargo nextest run -p gandr`.
The exit status is the observable half of a run, so a script consuming this binary reads the table above rather than the standard output.

## Theoretical ideas relied on

None of its own.
The driver is a process-boundary adapter: the language-level ideas it exposes are stated by the crates it calls, chiefly the call-by-push-value outcome vocabulary of `gandr-core-term` and the host effect boundary of `gandr-runtime-ffi`.

## Primary references

- Paul Blain Levy, _Call-By-Push-Value: A Functional/Imperative Synthesis_, Springer Netherlands, 2003, `doi:10.1007/978-94-007-0954-6` — the value and computation split whose terminal cases this crate classifies into process exit statuses.
