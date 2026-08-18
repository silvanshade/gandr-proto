# gandr-runtime-ffi

`gandr-runtime-ffi` executes declared C ABI foreign operations at the host-effect boundary.

## Current provision

- Loads only declared dynamic libraries and resolves only declared symbols.
- Marshals `u32`, `u64`, `i32`, `i64`, `f32`, `f64`, copied C strings, opaque pointer handles, and void results.
- Refuses malformed declarations before loading a library and reports typed boundary failures.
- Composes with the shell host through `CombinedDriver`.

## Planned but absent

- Additional ABIs and richer ownership-aware foreign types are not implemented.
- A runtime codec crate is not part of this landing because no live consumer requires one.

## Usage

Enable the native fixture for the hermetic tests:

```text
cargo test -p gandr-runtime-ffi --features native-fixture
```

Construct `FfiHost` from lowered `ForeignModule` declarations and pass host offers to `dispatch`.

## Theoretical ideas

The boundary follows least authority, typed effect handlers, and explicit ownership at an ABI boundary.

## Primary references

- _The libffi Interface_, libffi contributors, current project documentation, stable URL: <https://sourceware.org/libffi/>
- _The Rustonomicon_, The Rust Project, current edition, stable URL: <https://doc.rust-lang.org/nomicon/>
