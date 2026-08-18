# gandr-workflow-dylint

The project's own lint pass: three Dylint rules that hold gandr's Rust type boundaries where Clippy has no opinion.

Each rule enforces a convention `docs/workflow/rust.md` states in prose.
The value of moving them here is that a convention a tool checks is a convention that survives review fatigue, and the failure mode these three catch is a quiet one: a bare primitive at a crate boundary compiles perfectly and erases the meaning of the value crossing it.

## Current provision

- `single_field_struct_needs_transparent_repr`.
  Every single-field named or tuple struct declares `#[repr(transparent)]`. gandr uses single-field structs as semantic domain wrappers, and stating the transparent representation keeps the layout contract explicit while the wrapper stays nominal at the type boundary.
- `primitive_signature`.
  A function or method signature in a gandr-owned API does not expose a Rust primitive, including primitives under structural type layers, selected transparent containers, and type aliases.
  The rule establishes only that the signature reaches a local nominal transparent boundary; the wrapper's field visibility, conversions and documentation remain Clippy's and the workflow document's.
- `recursive_function_needs_termination`.
  Every recursive free function or method documents its termination argument in a `# Termination` rustdoc section, so a recursive control flow states its decreasing measure where a reviewer will read it.

All three warn rather than deny in the lint declaration; the merge wall promotes them by running the driver with warnings denied.

The crate builds as a `cdylib` under the pinned nightly, because a lint pass links rustc's internal libraries.
That is why the workspace build excludes it and why it has a crate-local cargo configuration.

## Planned but absent

- Nothing is scheduled.
  The rule set grows when a boundary convention proves worth mechanizing, which is a judgement made against the workflow document rather than here.

## Using it

```console
mise run cargo:dylint:local
mise run cargo:dylint:local gandr-theory-computads
```

A bare run checks the whole workspace, which is the merge wall's scope.
Trailing arguments are package names, for fast iteration on one crate.
Run it through the task rather than the bare binary: the task pins the toolchain, sets the driver's flags, and disables incremental compilation, which the driver reproducibly crashes on when reused.

## Theoretical ideas relied on

None of its own.
It mechanizes two design conventions: nominal typing at a module boundary, where a wrapper's distinct name is the point rather than its representation, and the termination argument as an explicit obligation attached to a recursive definition.

## Primary references

None.
The rules encode this project's own conventions, stated in `docs/workflow/rust.md`, and rest on no published result.
