# Changelog

The format is hand-maintained and grows only with real changes; it is not auto-generated.

## 2026-08-08 — Un-stub the driver with the script-runner face

* `current`: `gandr <file>` runs one gandr source file through `gandr_surface_engine::run::run_source_file`, and `gandr --help` prints the accepted command line.
  Everything else is refused, so a deferred face fails loudly rather than being read as a filename.
* `current`: The outcome-to-status contract is the crate's substance — `0` for a value terminal, the script's own `proc.exit` code reduced to a byte, `1` for a blame, a stuck configuration, or a fatal host abort, and `2` for a source that never reached the machine.
  A successful run prints nothing of the driver's own; diagnostics go to standard error.
* `current`: The manifest's predecessor dependency surface was **rewritten, not uncommented** (the stale-manifest hazard the front-end port study records): it named crate paths that do not exist under the reboot's category scheme, and every entry had since split or been renamed.
  What remains names which reboot crate each deferred face waits on, so nothing uncommentable is left.
* `current`: Three dependencies, each directly used — `gandr-surface-engine`, `gandr-runtime-host`, `gandr-core-checker`.
* `current`: Landed the port study's driver-faces owner call — the script-runner face only; the REPL and the `tui`/`lsp`/`mcp`/`fmt`/`build` faces stay deferred with their reasons recorded in the manifest and in `docs/STATUS.md`.
* `current`: `scripts/agda-deps.gandr` is the first production consumer, and `mise run agda:deps` invokes it.
* `current`: Tests — `tests/cli.rs` drives the real binary across sixteen cases, including the exit-code reduction (`-1` leaves 255, `300` leaves 44), the bare-dash and deferred-subcommand argument shapes, and the pinned fact that a shell block's output is captured for the program rather than relayed to the caller.
