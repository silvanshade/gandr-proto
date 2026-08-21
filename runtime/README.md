# runtime

The C++ tree. gandr's compilation host lives here, distinct from the Rust workspace under [`crates/`](../crates/) — in particular from `crates/runtime-effects`, which is the Rust host-effect runtime and shares nothing with this tree but a word.

| Path                                      | Holds                                                                  |
| ----------------------------------------- | ---------------------------------------------------------------------- |
| [`compile-host/`](compile-host/README.md) | the C++26 MLIR compilation host: dialect, lowering, JIT, and its tests |

This tree is not part of the Rust workspace, and nothing in that workspace links it: [`crates/runtime-compile-host`](../crates/runtime-compile-host/README.md) is the Rust side of the boundary and resolves the host's C ABI by name at run time, so a checkout with no MLIR still builds and tests everything there.

Its toolchain is a **pinned** MLIR revision — `runtime/compile-host/cmake/mlir-pin.cmake`, currently `llvmorg-22.1.8` — so the merge wall requires it absolutely: `mise run gate:merge` runs `compile-host:wall`, which fails when nothing satisfies the pin and makes every step after that fatal, the format and tidy lanes included.
The named `compile-host:*` tasks are its entry points; [`compile-host/README.md`](compile-host/README.md) lists them.
