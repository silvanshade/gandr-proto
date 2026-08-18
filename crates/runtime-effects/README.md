# gandr-runtime-effects

The headless host-effect runtime: the top-level handler that intercepts the operations no source-level handler claims, performs the real syscall, and resumes the delimited continuation with the reply.

This is the host side of the effect seam.
A gandr program that performs a process, filesystem, environment or exit operation is not describing a primitive the language implements; it is performing an operation that falls through to whoever is holding the boundary.
This crate is one such holder, and it runs the program headlessly, with no line editor and no terminal interface.

The seam it binds is representation-independent: a signature name, an operation name, and a payload.
Every driver over that seam presents the same projection, so swapping one host runtime for another does not change a program's observable outcome.

## Current provision

- `ShellHandler`, the dispatcher.
  It carries each intercepted operation out to a real syscall over the standard library's process, filesystem and environment interfaces.
- `run_program`, the driver.
  It hands a lowered program and a host handler to the L machine and reads back an evaluation outcome.
  A program exit and a fatal syscall both truncate the run.
- `run_program_with_prelude`, the same drive with an ambient value prelude installed.

The operation set is `Exec`, `Fs`, `Proc` and `Env`.
It deliberately does not take the name `Shell`, which is reserved for the typed shell surface the effects and control design record specifies and which is not built.

## Planned but absent

- **Effect-row discipline at the host boundary.** The effect row is vacuous here, and resumption is multi-shot: a captured continuation prefix is reified as a plain stack value with the handler reinstalled, so it may be resumed any number of times.
  The host is therefore an always-resume ambient handler.
  A host that honoured a row, or a once-only resumption discipline, would be a different handler on the same seam.
- **The typed pipe session.** Piping between external commands is an eager operating-system pipe today.
  The session-typed pipe the design record specifies is not built, and the eager pipe is a stopgap standing in for it rather than an implementation of it.
- **The typed shell surface** that would own the reserved `Shell` name.

## Using it

The crate takes already-lowered programs.
Source text goes through `gandr-surface-engine`'s run entry, which composes lowering, linking and prelude checking with the driver here.

```rust
use gandr_runtime_effects::run_program;

let outcome = run_program(&program);
let returned = outcome.returned();
```

The module documentation carries a runnable example that performs one `Exec` operation and reads the exit code back out of the returned record.

## Theoretical ideas relied on

Algebraic effects and handlers; delimited continuations and the reification of a continuation prefix; call-by-push-value, whose computation terminals the outcome vocabulary is stated over; the ambient top-level handler as the boundary between a language's effects and a host's syscalls.

## Primary references

- Gordon D. Plotkin and Matija Pretnar, _Handling Algebraic Effects_, Logical Methods in Computer Science, 2013, `doi:10.2168/LMCS-9(4:23)2013` — the handler discipline the top-level host handler instantiates.
- Paul Blain Levy, _Call-By-Push-Value: A Functional/Imperative Synthesis_, Springer Netherlands, 2003, `doi:10.1007/978-94-007-0954-6` — the value and computation split the machine and the outcome vocabulary are built on.
- Daniel Hillerström and Sam Lindley, _Liberating Effects with Rows and Handlers_, Type-Driven Development (TyDe), 2016, `doi:10.1145/2976022.2976033` — the row-based account of effect signatures the vacuous row here is measured against.
