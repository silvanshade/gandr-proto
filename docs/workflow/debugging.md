# Workflow: debugging workspace binaries with LLDB

> Read when: attaching LLDB to a gandr workspace binary or resolving a breakpoint in a core crate.
> Build and toolchain routing: [`WORKFLOW.md`](../WORKFLOW.md) and [`ci.md`](ci.md).

## Recipe

From a clean checkout, install the pinned dependencies and build the debuggable workspace binary:

```sh
mise run setup
mise run cargo:build
```

The `gandr` binary is the workspace's command-line surface.
Launch it through `rust-lldb`, which loads Rust's type summaries and synthetics:

```sh
rust-lldb -- target/debug/gandr
```

In LLDB, confirm the Rust summaries, locate the lowering seam, set the breakpoint, and run a committed source fixture:

```text
type summary list -w Rust
image lookup -rn lower_source_total
breakpoint set -n gandr_surface_engine::lower::lower_source_total_seeded
settings set target.run-args fuzz/corpus/check/seed-001.gandr
run
bt
```

`image lookup` is the symbol-discovery fallback when a function has been renamed.
Use the exact fully qualified name it prints with `breakpoint set -n`.

## Observable success

- `type summary list -w Rust` lists providers for `String`, `str`, slices, `Vec`, and other Rust standard-library types.
- `breakpoint set` reports at least one resolved location in the workspace binary, not only a pending breakpoint.
- `run` stops at the requested core-crate function, and `bt` shows the surface-engine lowering frame beneath the `gandr` binary entrypoint.

The recipe debugs the existing default-off binary.
It does not require tracing instrumentation or a special feature build.
