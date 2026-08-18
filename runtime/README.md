# runtime

The C++ tree. gandr's compilation host lives here, distinct from the Rust workspace under [`crates/`](../crates/) — in particular from `crates/runtime-effects`, which is the Rust host-effect runtime and shares nothing with this tree but a word.

| Path                                      | Holds                                                                  |
| ----------------------------------------- | ---------------------------------------------------------------------- |
| [`compile-host/`](compile-host/README.md) | the C++26 MLIR compilation host: dialect, lowering, JIT, and its tests |

This tree is not part of the Rust workspace, and nothing in that workspace links it: [`crates/runtime-compile-host`](../crates/runtime-compile-host/README.md) is the Rust side of the boundary and resolves the host's C ABI by name at run time, so a checkout with no MLIR still builds and tests everything there.

Its toolchain is a **discovered** MLIR installation rather than a pinned dependency, so the merge wall requires it conditionally rather than absolutely: `mise run gate:merge` runs `compile-host:wall`, which skips only when it can prove no toolchain is there and makes every step fatal once one is found.
The named `compile-host:*` tasks are its entry points; [`compile-host/README.md`](compile-host/README.md) lists them.
