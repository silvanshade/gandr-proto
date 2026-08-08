# Status

Crate scope: `crates/surface-driver` (package `gandr`).

Status vocabulary in this file is limited to `current`, `designed direction`, and `open decision`.

## current

* The crate is the toolchain's **process boundary**: it parses a command line, runs what it was asked to run, and turns the result into an exit status.
  It owns no pipeline, no grammar, and no rendering — every one of those belongs to a crate it calls.
* **The script-runner face is the only face that exists.** `gandr <file>` runs one gandr source file through `gandr_surface_engine::run::run_source_file`, which lowers, links, prelude-checks, and runs the program under the host-effect handler.
  `gandr --help` (or `-h`) prints the accepted command line.
  Anything else — a second operand, or a leading-dash argument that is not one of those two — is refused rather than read as a path, so a deferred face fails loudly instead of being silently interpreted as a filename.
* **Outcome-to-status is the driver's whole contract**, and it is what a calling shell reads: a value terminal leaves `0`; `proc.exit code` leaves that code reduced to a byte the way a shell reduces one; a blame, a stuck configuration, or a fatal host abort prints one diagnostic and leaves `1`; a source that never reached the machine — an unreadable file, or one that fails to lower, link, or type-check — prints its typed error and leaves `2`.
* **A successful run prints nothing of the driver's own.** A script speaks through the host effects it performs; a value renderer here would be a second, rival printer beside the corpus harness's deliberately structural one, so the returned value reaches the caller as an exit status and nothing else.
  Diagnostics go to standard error, so a script's own output reaches its consumer unmixed.
* Dependencies are exactly three and each has a direct use: `gandr-surface-engine` (the source entry), `gandr-runtime-host` (`ShellOutcome`, the shape a run terminates in), and `gandr-core-checker` (`Eval`, the core outcome that `ShellOutcome::Completed` carries).
* Un-stubbed at rung F5 of the surface front-end port (`docs/research/front-end-port-staging.md` §9), landing staging call O8 — the script-runner face alone.
  The predecessor manifest's commented dependency surface named crate paths that do not exist under the reboot's category scheme; it was **rewritten**, not uncommented, and what remains records which reboot crate each deferred face waits on.
* The production consumer is `scripts/agda-deps.gandr`, run by `mise run agda:deps`.
* Tests: `tests/cli.rs` drives the real binary, because the contract under test is what a calling shell observes — the four statuses, the refusals, the usage text, and the standard-output discipline.
  Two further cases anchor `scripts/agda-deps.gandr` without running it (it clones over the network): that it lowers and reaches the host, and that it has not drifted from the literate corpus copy that explains it.

## designed direction

* The **REPL** is the intended default face when the driver is invoked with no operand.
  It needs a line editor plus `gandr-surface-grammar` (the checked grammar its highlighter molds against), `gandr-surface-parser` (whose obligation queries drive the validator, hinter, and completer), and `gandr-surface-syntax` (the checked borrowed source slice the parser's boundary takes).
  Until it lands, no operand is a usage error.
* `gandr tui` needs the terminal programming environment, which has no reboot crate at all; `gandr lsp`, `gandr mcp`, `gandr fmt`, and `gandr build` are subcommand slots with no implementing crate in the tree.
  Each arrives with its real dependency edge, in the same change.
* A script cannot yet write to its caller's terminal: `#!{ … }` blocks always lower to the captured spawn mode and no host-module member writes to standard output, so a long-running command under `gandr <file>` is silent until it finishes (`gandr-czio`).
  The driver's captured-output case pins that contract and fails when it changes.

## open decision

* Whether the driver grows a way to run a program from standard input, or from an argument, alongside the file face.
  Nothing needs it today, and the script-runner face was deliberately cut to what `agda:deps` and the corpus harness require.
