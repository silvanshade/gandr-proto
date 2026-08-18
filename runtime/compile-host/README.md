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
- A **structural lowering** from the dialect to `func`/`cf`/`arith`/`memref`, then the standard conversion to the LLVM dialect, then the JIT.
- A **reference interpreter** over the same image, sharing the heap layout and the value rendering with the compiled path and nothing else.
- An **agreement fixture** stating what gandr's Rust L machine answers for each named program.
  The Rust side pins the fixture to the machine; this side pins the compiled slice to the fixture.

## What is planned and absent

The honest boundary of this slice, restated so nothing here is read as more than it is:

- **codata and call-by-need** — no comatch, thunk, force, or memo cell;
- **effects** — no perform, no handler, no prompt;
- **the host seam** — no native primitive, no foreign call;
- **reified continuations** — no boxed consumer, no resume, no shift;
- **the unconditional jump** — no top-level definition table and no call.

Also absent: a bridge from the Rust side.
The image is the plain-old-data boundary a foreign caller would hand over, and the interop layer that would hand it over — `cxx`, per the ruled lowering boundary — is not built.
The compiled code carries **no bounds check** on its heap: the host sizes the heap from the image's own allocation bound, so the guarantee is the caller's rather than the compiled code's.
There is no optimization of the emitted program beyond what the standard MLIR pipeline does, and no measurement of the compiled program's throughput.

## Using it

Every entry point is a named task, run from the repository root.
MLIR is discovered rather than pinned: `GANDR_MLIR_PREFIX` wins, then a Homebrew `llvm` keg, then whatever `llvm-config` is on `PATH`.
The compiling clang must be the one that ships beside the discovered MLIR; the configure step refuses a version mismatch, because mixing two builds across that boundary fails far from its cause.

| Task                                 | Does                                                                   |
| ------------------------------------ | ---------------------------------------------------------------------- |
| `mise run compile-host:configure`    | configure the build against the discovered MLIR                        |
| `mise run compile-host:build`        | build the host, its test binary, and its fuzz entry                    |
| `mise run compile-host:test`         | run the regression suite                                               |
| `mise run compile-host:differential` | diff the compiled slice's answers against the L machine's fixture      |
| `mise run compile-host:timings`      | report per-program compile and execute microseconds                    |
| `mise run compile-host:fuzz-smoke`   | replay every committed fuzz seed through the entry surface             |
| `mise run compile-host:fuzz`         | run an AFL++ campaign over the entry surface                           |
| `mise run compile-host:mutants`      | apply the curated mutant catalogue and require the suite to catch each |

The binary itself has modes for looking at intermediate stages:

```sh
runtime/compile-host/build/gandr-compile-host --dump-dialect=compound
runtime/compile-host/build/gandr-compile-host --dump-lowered=case
runtime/compile-host/build/gandr-compile-host --interpret-samples
```

None of these tasks is on the merge wall.
The wall is Rust-only and every contributor runs it; requiring an MLIR installation to land a Rust change would be a much larger claim than this slice makes.
The Rust half of the agreement differential is on the wall, inside `mise run cargo:nextest`.

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
