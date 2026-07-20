# Optimization

No crate-specific optimization work has been done.

The only tuning is the workspace release profile (`Cargo.toml` `[profile.release]`: `opt-level = 3`, `lto = "fat"`, `codegen-units = 1`, `panic = "abort"`, `strip = true`), which also satisfies the preconditions a `#[no_panic]` attempt needs (`docs/ADR.md` "no_panic strategy").

The dual checker / machine implementation is correctness infrastructure, not a performance cost the shipping path pays — the machine is the operational artifact and the recursive checker is exercised only by the conformance suite.
