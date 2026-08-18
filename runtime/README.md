# runtime

The C++ tree. gandr's compilation host lives here, distinct from the Rust workspace under [`crates/`](../crates/) — in particular from `crates/runtime-effects`, which is the Rust host-effect runtime and shares nothing with this tree but a word.

| Path                                      | Holds                                                                  |
| ----------------------------------------- | ---------------------------------------------------------------------- |
| [`compile-host/`](compile-host/README.md) | the C++26 MLIR compilation host: dialect, lowering, JIT, and its tests |

This tree is not part of the Rust workspace and is not built by `mise run gate:merge`.
Its toolchain is a **discovered** MLIR installation rather than a pinned dependency, so requiring it on the merge wall would make every Rust change depend on an installation the wall cannot provide.
The named `compile-host:*` tasks are its entry points; [`compile-host/README.md`](compile-host/README.md) lists them.
