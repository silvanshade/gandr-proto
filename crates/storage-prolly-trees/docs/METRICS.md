# Metrics

Recorded for the Prolly-Bao proof implementation and adaptation branch closeout.

## current

* Benchmark footprint: 3 Criterion bench source files, `benches/gandr_storage_prolly_trees.rs`, `benches/prolly_bao_profile_curve_fixed_width.rs`, and `benches/prolly_bao_profile_curve_source.rs`.
* Benchmark targets: `gandr_storage_prolly_trees`, `prolly_bao_profile_curve_fixed_width`, and `prolly_bao_profile_curve_source`.
* Comparator scope: direct Dolt and Bao comparison benchmarks are intentionally not included in the current `gandr_storage_prolly_trees` Criterion target.
* Dolt boundary: no direct Dolt benchmark is included.
  + Dolt's Prolly Tree implementation is a Go storage-engine component, not a Rust Criterion comparison target in this workspace.
  + A fair comparison would require a pinned external Go benchmark harness or reviewed Rust port/FFI plus shared ordered-record fixtures, schema/key encoding, chunking rules, operation definitions, toolchain/allocation reporting, and side-by-side caveats.
  + Metrics must not imply Dolt parity until that exists.
* Bao boundary: no direct Bao benchmark is included because the Rust `bao` crate verifies byte ranges in a flat BLAKE3 byte stream, while Prolly-Bao verifies ordered key/value membership, non-membership, and range claims against a Merkle search tree with committed parameters.
  Bao results must not be reported as Prolly-Bao tree/proof performance.
* Fixture scope: small deterministic borrowed key/value records plus medium deterministic throughput fixtures where listed.
* Operation scope:
  + Small latency operations: build, lookup, range scan, membership proof generation and verification, non-membership proof generation and verification, and range proof generation and verification.
  + Medium throughput operations: build, full-range scan, full-range proof generation, and full-range proof verification.
* Witness transcript behavior coverage is represented by contract tests in `crates/storage-prolly-trees/tests/prolly_bao_contract.rs`, not by a measured benchmark row.
  The added tests target membership, non-membership, and range encode/decode/verify round trips, proof `root_node_hash` getters, public end-summary getter coverage, and fail-closed malformed transcript/end-summary cases.
* Snapshot byte-stream behavior coverage is represented by contract tests in `crates/storage-prolly-trees/tests/prolly_bao_contract.rs`, not by a measured benchmark row.
  The tests target exact snapshot bytes verified with `bao::encode::encode` plus `bao::decode::decode`, empty and one-record trees, deterministic equivalent rebuilds, and fail-closed malformed, tampered, wrong-root, wrong-version, truncated, unordered, duplicate, and non-snapshot encodings.
* Minimal Iroh interop rejection coverage is represented by contract tests in `crates/storage-prolly-trees/tests/prolly_bao_iroh_interop.rs`, not by a measured benchmark row.
  The tests cover deterministic exact-byte cacheability of canonical snapshot bytes and native witness bytes under flat BLAKE3 hashes, Prolly-Bao root/parameter verification after readback, and fail-closed wrong hash, missing entry, mutation, truncation, unsupported version, and wrong-root cases.
* Coordinator verification for witness transcript and snapshot byte-stream integration ran `cargo fmt --all`, `cargo check -p storage-prolly-trees`, `cargo build -p storage-prolly-trees`, `cargo test -p storage-prolly-trees`, `cargo clippy -p storage-prolly-trees -- -D warnings`, and `cargo nextest run -p storage-prolly-trees`.
* Coordinator verification after commit `19d6f3e` covered `storage-chunker` and `storage-prolly-trees` with cargo fmt, cargo check/build/test, the clippy helper with zero diagnostics, rumdl check, typos, and treefmt.
  This document does not claim additional verification.
* Compact proof behavior coverage is represented by contract tests in `crates/storage-prolly-trees/tests/prolly_bao_contract.rs`, not by a measured benchmark row in this merge.
  The tests cover compact membership, non-membership, range, and witness paths plus fail-closed rejection for omitted selected leaves, omitted required successor leaves, extra successor leaves, swapped compact children, non-root-first node order, wrong range bounds, incomplete returned records, and unsorted returned records.
  `benches/prolly_bao_profile_curve_fixed_width.rs` adds a local Criterion profile curve for `fixed-width-value-updates`, and `benches/prolly_bao_profile_curve_source.rs` adds a local Criterion profile curve for source-file-like records.
  The compact planner recommendation for fully compact evidence is membership-heavy `target-x1`, mixed `target-x2`, and update-heavy `target-x2`.
  Source-file-like `target-x3` evidence remains policy input only and is tracked under `open decision`; neither benchmark changes the default tree profile.
* Encoded node layout inspection is `current` and covered by tests, but no allocation-count baseline, node-layout performance benchmark, or measured layout optimization number has been recorded.
* `PackedSegmentStore` is `current` as an in-memory packed segment `BlockStore` prototype and is covered by tests, but no LMDB or other persistent backend is selected and no storage benchmark has been recorded.
* Diff witness and adapter manifest documents are `designed direction` analysis only.
  No stable Rust API for them is `current`.
* The workspace lint configuration allows Clippy `missing_const_for_fn` by user preference.
  That is tooling policy, not Prolly-Bao runtime behavior or a performance metric.
* Benchmark setup: deterministic fixtures and reusable built trees are prepared outside timed loops where practical; timed loops pass outputs through `black_box`.
* Benchmark baseline: local release Criterion estimates observed on the listed workstation.
  Observed environment: darwin 25.5.0, arm64, Apple M3 Max.
* Run context: benchmark run was part of a sequential run from repository root on branch `tasks`.
* Sequential command:

  ```text
  cargo bench --package storage-chunker --bench chunker &&
  cargo bench --package storage-prolly-trees --bench gandr_storage_prolly_trees
  ```

* Observed command: `cargo bench --package storage-prolly-trees --bench gandr_storage_prolly_trees`.
* Criterion bench profile output:
  + `Finished bench profile [optimized] target(s) in 0.09s`
* Criterion target line:
  + `Running benches/gandr_storage_prolly_trees.rs (target/release/deps/gandr_storage_prolly_trees-f064354df5600d4f)`
* Full two-crate command wall time: `253.03 seconds`.
* Baseline rows:
  + `gandr_storage_prolly_trees/build/small`
    - Time: `[2.2658 µs 2.2712 µs 2.2768 µs]`
    - Throughput: not reported
  + `gandr_storage_prolly_trees/build/medium-throughput`
    - Time: `[2.6228 ms 2.6266 ms 2.6307 ms]`
    - Throughput: `[392.00 MiB/s 392.61 MiB/s 393.19 MiB/s]`
  + `gandr_storage_prolly_trees/lookup/small-hit`
    - Time: `[13.407 ns 13.444 ns 13.481 ns]`
    - Throughput: not reported
  + `gandr_storage_prolly_trees/range/small`
    - Time: `[160.20 ns 160.59 ns 160.98 ns]`
    - Throughput: not reported
  + `gandr_storage_prolly_trees/range/medium-all-throughput`
    - Time: `[157.82 µs 158.35 µs 158.93 µs]`
    - Throughput: `[25.772 Melem/s 25.866 Melem/s 25.953 Melem/s]`
  + `gandr_storage_prolly_trees/proof/generate-membership`
    - Time: `[1.2192 µs 1.2213 µs 1.2235 µs]`
    - Throughput: not reported
  + `gandr_storage_prolly_trees/proof/generate-non-membership`
    - Time: `[265.48 ns 266.24 ns 267.01 ns]`
    - Throughput: not reported
  + `gandr_storage_prolly_trees/proof/generate-range`
    - Time: `[317.88 ns 318.69 ns 319.46 ns]`
    - Throughput: not reported
  + `gandr_storage_prolly_trees/proof/generate-range/medium-all-throughput`
    - Time: `[232.50 µs 233.00 µs 233.51 µs]`
    - Throughput: `[17.541 Melem/s 17.580 Melem/s 17.617 Melem/s]`
  + `gandr_storage_prolly_trees/proof/verify-membership`
    - Time: `[961.53 ns 963.20 ns 964.84 ns]`
    - Throughput: not reported
  + `gandr_storage_prolly_trees/proof/verify-non-membership`
    - Time: `[1.9640 µs 1.9678 µs 1.9718 µs]`
    - Throughput: not reported
  + `gandr_storage_prolly_trees/proof/verify-range`
    - Time: `[2.0141 µs 2.0177 µs 2.0211 µs]`
    - Throughput: not reported
  + `gandr_storage_prolly_trees/proof/verify-range/medium-all-throughput`
    - Time: `[4.9690 ms 4.9808 ms 4.9938 ms]`
    - Throughput: `[252.28 MiB/s 252.93 MiB/s 253.53 MiB/s]`
* Baseline interpretation: these are local Criterion estimates, not generalized performance guarantees or regression thresholds.
  Throughput is reported only where Criterion emitted it:
  + Build throughput is key+value bytes.
  + Range scan and range proof generation throughput are records/elements.
  + Range proof verification throughput is proof-node bytes.
  Rows marked `not reported` have no throughput estimate, and the benchmark run did not measure coverage, security/vulnerability numbers, allocation counts, encoded node layout performance, or packed segment store performance.
* Dependency scope: the core identity dependency is BLAKE3.
  Boundary detection is delegated to local `storage-chunker` parameter commitments.
  Criterion is benchmark-only.
  `bao` 0.13.1 is dev/test-only for snapshot byte verifier evidence and uses `default-features = false`.
  No `iroh` or `iroh-blobs` dependency was added to `crates/storage-prolly-trees` for the minimal interop rejection; the workspace `iroh` entry remains unused by this crate.
* For the Iroh interop rejection slice, no benchmark, formatter, cargo gate, syft, or grype command was run by this agent because the assignment explicitly reserves verification for the coordinator and adds no dependency.
* Supply-chain scan: the coordinator generated `target/sbom/mach.cdx.json` with `syft dir:. -o cyclonedx-json=...` and scanned it with `grype sbom:target/sbom/mach.cdx.json`; `grype` reported no vulnerabilities.
  No `cargo bloat` result is recorded here.
* Non-goal scope: no performance claims are made for Bao verification, Bao transport, Iroh transport, IPLD/CAR import/export, SQL/DataFusion, persistent stores, network behavior, async runtimes, filesystem paths, Python, Git, Automerge, or text/vector search.

## designed direction

* Refresh release-build baselines after orchestrator-owned verification runs.
* Track build, lookup, range, proof generation, and proof verification separately.
* Track proof byte size separately from tree node byte size and store byte count.
* Track witness transcript encode/decode throughput, transcript byte size, and allocation counts only after an orchestrator-owned benchmark or profiling task defines fixtures and measurement commands.
* Track encoded node layout inspection cost and allocation counts only after a benchmark/profiling task defines fixtures, commands, and reporting units.
* Track packed segment store adapter throughput, byte overhead, and allocation counts only after a backend experiment defines the in-memory and persistent comparison surfaces.
* Track local-change structural sharing with changed leaf count, affected ancestor count, and hash-equal unchanged subtree count.
* Keep borrowed fixture preparation outside measured loops unless deliberately measuring end-to-end fixture construction.

## open decision

* No regression threshold is selected yet.
* No allocation-count baseline exists yet.
* No hardware-normalized threshold exists yet.
* No compact-proof-size target is committed yet.
* Source-file-like compact profile data is now represented by a local Criterion benchmark, but its `target-x3` balanced-compromise counterweight is not a default profile decision and does not override the current fully compact planner recommendation.
* No measured benchmark numbers were added for encoded node layout inspection or store adapters.
* No LMDB or other persistent store backend is selected yet.
* Future limited Bao benchmark scope:
  + Measure only canonical snapshot byte-stream or adapter encode/decode/slice overhead.
  + Do not report Bao transport measurements as native Prolly-Bao witness or tree/proof performance.
  + Keep `bao` dev/test-only unless a reviewed adapter task changes dependency scope.
  + Include `byte-stream`, `snapshot`, or `bao-raw` in benchmark names.
