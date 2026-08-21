# compile-host

## What it is for

gandr's compilation host: a C++26 program that takes gandr's own machine structures, builds an MLIR module from them in process, lowers it, and runs it through the JIT.
MLIR's native interface is C++, which is what puts the host on this side of the boundary rather than in the Rust workspace.

## What it currently provides

The first production lowering slice — the L machine's **positive core** — end to end.

- A **program image**: a flat, index-addressed, plain-old-data structure holding the positive core's eight node forms, with a byte encoding and a total decoder.
- A **dialect**, `gandr`, declared in TableGen over one opaque value type: the six transitions this slice lowers (`gandr.cut`, `gandr.bind`, `gandr.case`, `gandr.ctor`, `gandr.dup`, `gandr.drop`) plus the producer leaf `gandr.lit` and the consumer-region terminator `gandr.yield`.
  Consumers that bind carry their continuations as **regions**, which is the shape the sequent IL has.
- A **mandatory verifier wall**.
  Nothing reaches canonicalization, lowering, or execution before `mlir::verify` accepts the module.
  A constructor's arity is checked there, against the arity its tag declares — the builder accepts a two-field tag given one field, and only the verifier objects.
- **Duplication and discard are effectful.** Both operations declare memory effects, and their lowering increments a heap ledger.
  A canonicalization cannot delete accounted work, and the regression witness reads the ledger back rather than trusting the declaration.
- A **bounds check the compiled code carries itself.** Every allocation compares against the heap's own extent, read from the descriptor the caller passed, and a run that would not fit sets an exhaustion flag and returns rather than writing past it.
  The reference walk refuses at the same word, so the two agree on short, exact and oversized heaps.
- A **structural lowering** from the dialect to `func`/`cf`/`arith`/`memref`, then the standard conversion to the LLVM dialect, then the JIT.
- A **reference interpreter** over the same image, sharing the heap layout and the value rendering with the compiled path and nothing else.
- An **agreement fixture** stating what gandr's Rust L machine answers for each named program.
  The Rust side pins the fixture to the machine; this side pins the compiled slice to the fixture.
- A **C boundary**, `include/gandr/compile_host/abi.h`, built as the shared library `gandr-compile-host-abi`.
  An encoded image goes in; a rendered value, both ledger counters, the arena words consumed, and a typed status come out.
  It is C rather than C++ because a caller sharing a C++ ABI with this host would have to share its MLIR installation too, and it is a separate library because nothing should have to link the host in order to build.
  Every entry is total: no exception escapes, and the only allocation on the message path is a `nothrow` buffer, so a failing one loses the message rather than the status.
  A second library, `gandr-compile-host-abi-partial`, exports the version and the run entry and **no release**, so a caller's symbol-resolution order is witnessed rather than argued.
  `crates/runtime-compile-host` is the Rust side of it: it lowers a checked core computation into an image, encodes it, and resolves this boundary by name at run time.

## What is planned and absent

The honest boundary of this slice, restated so nothing here is read as more than it is:

- **codata and call-by-need** — no comatch, thunk, force, or memo cell;
- **effects** — no perform, no handler, no prompt;
- **the host seam** — no native primitive, no foreign call;
- **reified continuations** — no boxed consumer, no resume, no shift;
- **the unconditional jump** — no top-level definition table and no call.

What the excluded block costs, stated precisely because it is the re-test trigger for the one prediction the slice still cannot measure: every route to a **runtime frame stack** in the L machine passes through a value this image cannot represent.
A handled `perform` binds its resumption as a boxed continuation at every hit, `shift` captures one by definition, an application builds a closure, and a `force` builds a thunk.
The lowering materializes a frame only where it cannot inline a region, which is exactly at a capture — so the frame-stack prediction waits on the codata and reified-continuation rungs rather than on a `perform` operation by itself.

There is no optimization of the emitted program beyond what the standard MLIR pipeline does, and no measurement of the compiled program's throughput.

## Using it

Every entry point is a named task, run from the repository root.

**MLIR is pinned.** `runtime/compile-host/cmake/mlir-pin.cmake` names one revision, one version, one archive and one sha256, and every consumer reads them from there: the CMake configure includes it, the tasks parse it.
The current pin is `llvmorg-22.1.8`, upstream unmodified — `mlir-patches/series` is empty and is the place to check that.

The source build is not landed yet, so the local bootstrap satisfies the pin from an installed toolchain: `GANDR_MLIR_PREFIX` wins, then a Homebrew `llvm` keg, then `llvm-config` on `PATH`.
What makes that a pin rather than a preference is the **equality check** — a candidate whose `LLVM_PACKAGE_VERSION` is not the pinned version is refused, not used, and the configure refuses it a second time.
The compiling clang must be the one that ships beside it, because mixing two builds across that boundary fails far from its cause.

Exceptions and RTTI follow the keg for the same reason: `LLVM_ENABLE_EH` is `OFF`, so the host builds `-fno-exceptions`, and `LLVM_ENABLE_RTTI` is `ON`, so RTTI stays — the lawful departure `docs/workflow/cpp.md` records, taken at the flag site and propagating nowhere.
Both are read from the keg at configure time; an EH-enabled keg is a fatal configure error.

The pin moves through one command, and only through it:

```sh
mise run compile-host:pin-update --revision llvmorg-22.1.9
```

It fetches that release's archive, hashes what it actually received, and rewrites all four facts together — a revision without a measured digest is not a pin.
It does not install the toolchain; the bootstrap has to supply the new version before anything configures again, which is the friction a pin is supposed to have.

| Task                                 | Does                                                                            |
| ------------------------------------ | ------------------------------------------------------------------------------- |
| `mise run compile-host:pin`          | print the pinned revision, version, archive and sha256                          |
| `mise run compile-host:prefix`       | resolve the install prefix that satisfies the pin, or fail naming why not       |
| `mise run compile-host:pin-update`   | move the pin to a named release, measuring the archive's digest                 |
| `mise run compile-host:configure`    | configure the build against the pinned MLIR                                     |
| `mise run compile-host:format`       | check every source against `runtime/.clang-format`                              |
| `mise run compile-host:tidy`         | check every source against `runtime/.clang-tidy`                                |
| `mise run compile-host:build`        | build the host, its test binary, and its fuzz entry                             |
| `mise run compile-host:test`         | run the regression suite                                                        |
| `mise run compile-host:differential` | diff the compiled slice's answers against the L machine's fixture               |
| `mise run compile-host:timings`      | report per-program compile and execute microseconds                             |
| `mise run compile-host:fuzz-smoke`   | replay every committed fuzz seed through the entry surface                      |
| `mise run compile-host:fuzz`         | run an AFL++ campaign over the entry surface                                    |
| `mise run compile-host:mutants`      | apply the curated mutant catalogue and require the suite to catch each          |
| `mise run compile-host:wall`         | run every gate above against the pinned toolchain                               |

The binary itself has modes for looking at intermediate stages:

```sh
runtime/compile-host/build/gandr-compile-host --dump-dialect=compound
runtime/compile-host/build/gandr-compile-host --dump-lowered=case
runtime/compile-host/build/gandr-compile-host --interpret-samples
```

`compile-host:wall` **is** on the merge wall, and the rest are the pieces it composes.

**It has no condition.** A pin is exactly the claim that the wall may assume the toolchain, so an installation that is absent, that carries no `lib/cmake/mlir`, that ships no `clang++`, or that is a version other than the pinned one is a **failure** of this task.
The absence skip and the `GANDR_COMPILE_HOST_STRICT` switch that turned it off are retired by the pin's execution, along with the discovered-toolchain reading they implemented.

**Everything after resolution is fatal**, which is the rule the old skip's own defect taught and the part that has not changed: a configure failure, a format finding, a tidy finding, a build failure, a failing case, a differential mismatch, a bridge failure.
None of them is an absence, and none may leave this task green with a reassuring message.

The two conventions lanes ride here, so conformance to `runtime/.clang-format` and `runtime/.clang-tidy` is gated rather than aspirational.
Both use the `clang-format` and `clang-tidy` that ship beside the pinned MLIR: a formatter of a different vintage reflows differently, and a tidy of a different vintage knows different checks.

Two things ride the wall beside it and need no toolchain at all: the Rust half of the agreement differential, and the source-level contract gate in `crates/runtime-compile-host/tests/contract.rs`, which holds this host's declared numbers and disciplines to what the Rust mirror assumes.
Neither reaches behaviour — that is what the lane above is for.
The contract gate reads source text and compares it with whitespace collapsed, deliberately: it is about what the host declares, and the format policy has a lane of its own.

## The ideas it rests on

- **Call-by-push-value** and the **polarized sequent calculus**: the positive core is the value-producing fragment of gandr's System-L command IL, and a consumer that binds is a `mu-tilde` frame.
- **Graded computation**: duplication and discard are ordinary computations rather than free wiring, which is why they are priced here rather than folded.
- **Convex double-pushout rewriting over monogamous acyclic hypergraphs**, the substrate the wider design lowers from; nothing of the rewriting layer is in this slice, but the no-implicit-sharing invariant on the image is the same commitment one level down.

## Primary references

- Paul Blain Levy, _Call-By-Push-Value: A Functional/Imperative Synthesis_, Springer, 2003.
  ISBN 978-1-4020-1730-8.
- Pierre-Louis Curien and Hugo Herbelin, "The duality of computation", _Proceedings of the Fifth ACM SIGPLAN International Conference on Functional Programming (ICFP '00)_, 2000, pages 233–243.
  DOI 10.1145/351240.351262.
- Chris Lattner, Mehdi Amini, Uday Bondhugula, Albert Cohen, Andy Davis, Jacques Pienaar, River Riddle, Tatiana Shpeisman, Nicolas Vasilache and Oleksandr Zinenko, "MLIR: Scaling Compiler Infrastructure for Domain Specific Computation", _2021 IEEE/ACM International Symposium on Code Generation and Optimization (CGO)_, 2021, pages 2–14.
  DOI 10.1109/CGO51591.2021.9370308.
- William Brandon, Benjamin Driscoll, Frank Dai, Wilson Berkow and Mae Milano, "Better Defunctionalization through Lambda Set Specialization", _Proceedings of the ACM on Programming Languages_ 7 (PLDI), 2023, article 132.
  DOI 10.1145/3591260. — the ruled closure-lowering strategy for the stage this slice does not yet reach.
