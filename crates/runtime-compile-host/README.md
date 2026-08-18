# gandr-runtime-compile-host

## What it is for

The Rust side of the compilation host boundary.
A core computation goes in, a plain-old-data program image comes out, and the C++26 compilation host under [`runtime/compile-host/`](../../runtime/compile-host/README.md) compiles and runs it — returning the same value the L machine produces, with the run's accounted work beside it.

The host is **found at run time, never linked**.
It builds against a discovered MLIR installation, so linking it would make every Rust build in the workspace depend on a toolchain the workspace does not pin.
A checkout with no MLIR builds and tests this crate in full; the host's absence is an ordinary reported outcome.

## What it currently provides

- A **lowering** from the core's positive fragment to the image: names become de Bruijn distances counted inwards, the tree becomes a flat arena in dependency order with the terminal cut last, and every form outside the fragment is refused by name.
  The walk is an explicit stack, so term depth costs heap rather than the host's call stack.
- The **wire form**: the little-endian byte encoding the host's decoder accepts.
- A **typed gate**: the driving entry checks the computation on the core's typing machine before lowering it, so what crosses the boundary is a computation the checker accepted.
- The **C boundary**, resolved by name at run time: three run entries, a version check, and an owned-string release.
  A version mismatch is refused rather than called, because a struct whose layout changed has no other symptom across a dynamic boundary.
- The **canonical rendering** of an L machine terminal value in the grammar the host prints, deliberately partial: a value outside the compiled slice has no spelling here.
- A **source-level contract gate** over the host's own sources, in `tests/contract.rs`.
  It rides the merge wall unconditionally and holds every number this crate mirrors — heap layout, cell and node numbering, constructor arities, wire version and arena bound, ABI version and statuses, the boundary struct's field order — plus the verifier-first pipeline and the grade operations' effect declarations, to what the host declares.

### The typed core and the machine core are not the same set

The compiled slice lowers the L **machine's** positive core, and the machine's core is wider than the **typed** core in exactly one place.
The machine's duplication and discard are structural operations over any runtime value; the core's `dup` and `drop` are the grade rules, and want a graded thunk.
So `dup 4` runs on the machine and is not a typed computation at all, and the checker is right to refuse it.

Five of the host's eight named programs are typed and go through the checked entry; three are machine-level and go through the entry named for that.
The gap closes when the image can represent a thunk, which is the codata rung.

## What is planned and absent

- **No surface path.** Nothing in gandr's language surface produces a core computation and hands it here; the entry is the Rust API.
- **No effects, codata, reified continuations, or calls** — the compiled slice excludes them, so the lowering refuses them by name.
- **No streaming or incremental lowering.** One computation, one image, one call.
- **No measurement of the compiled program's throughput**, and no optimization of the emitted program beyond what the host's pipeline does.

## Using it

The crate builds and tests with no MLIR.
To exercise the half that needs a host, build one first:

```sh
mise run compile-host:build
cargo nextest run --package gandr-runtime-compile-host --all-targets --features=full
```

The bridge's own cases report an absent host and stop.
`GANDR_COMPILE_HOST_REQUIRED=1` turns that report into a failure, which is what `mise run compile-host:wall` sets once it has built a host — that task is the conditional present-toolchain lane on the merge wall.
`GANDR_COMPILE_HOST_LIBRARY` names the host library explicitly; otherwise the conventional build output under the workspace root is used.

## The ideas it rests on

- **Call-by-push-value** and the **polarized sequent calculus**: the fragment lowered here is the value-producing part of gandr's System-L command IL, and a consumer that binds is a `mu-tilde` frame.
- **Graded computation**: duplication and discard are ordinary computations rather than free wiring, which is why the boundary reports them as accounted work rather than eliding them.
- **de Bruijn representation**: the image carries binder distances rather than names, so an image is comparable and hashable without a name supply.

## Primary references

- Paul Blain Levy, _Call-By-Push-Value: A Functional/Imperative Synthesis_, Springer, 2003.
  ISBN 978-1-4020-1730-8.
- Pierre-Louis Curien and Hugo Herbelin, "The duality of computation", _Proceedings of the Fifth ACM SIGPLAN International Conference on Functional Programming (ICFP '00)_, 2000, pages 233–243.
  DOI 10.1145/351240.351262.
- N. G. de Bruijn, "Lambda calculus notation with nameless dummies, a tool for automatic formula manipulation, with application to the Church-Rosser theorem", _Indagationes Mathematicae (Proceedings)_ 75 (5), 1972, pages 381–392.
  DOI 10.1016/1385-7258(72)90034-0.
