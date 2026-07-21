# Metrics

Documents current benchmark coverage and the previously recorded 2026-06-02 expanded FastCDC v2020 comparison benchmark baseline.

## current

* Benchmark footprint: 1 Criterion bench source file, `benches/chunker.rs`.
* Benchmark target: `chunker`.
* Benchmark scope: Mach borrowed-record rows call `chunk_record_slices` over borrowed canonical record fixtures; Mach flattened-span rows call `chunk_spans` over one canonical byte buffer plus record spans; FastCDC rows measure raw byte-slice CDC throughput.
* Fixture coverage: original small, medium, low-entropy repeated-window, and local-edit perturbation cases for insert, update, and delete streams; current benchmark source also covers prior-art candidate fixtures:
  + source-file-like records
  + task-record-like records
  + low-entropy keys
  + fixed-width value updates
  + large-value references
  + adversarial boundary-seeking records
  These fixture rows are benchmark coverage only; they are not proof-size, tree-construction, or consensus-default evidence.
* Output handling: Mach record-safe returned boundary spans are passed through `black_box` in timed loops.
  Flattened-span rows check `chunk_spans` output against `chunk_record_slices` before registering each timed row.
* Previously recorded benchmark baseline: local release Criterion estimates for the original row set only.
  + Environment: darwin 25.5.0, arm64, Apple M3 Max.
  + Run order: commands were run sequentially from repository root.
  + Branch: `tasks`.
  + Sequential command context:

    ```text
    cargo bench --package storage-chunker --bench chunker &&
    cargo bench --package storage-prolly-trees --bench prolly_bao
    ```

  + Full two-crate command wall time: `253.03 seconds`.
* Flattened-span continuation baseline: local release Criterion estimates for the `mach-record-safe` row subset are recorded below.
  The prior-art candidate Okra/Dolt/hybrid row families still need a release baseline on their renamed rows.
* Observed command: `cargo bench --package storage-chunker --bench chunker`.
* Criterion run:
  + `Finished bench profile [optimized] target(s) in 0.09s`
  + `Running benches/chunker.rs (target/release/deps/chunker-916ffbac925b3e91)`
* Interpretation: these are local Criterion estimates, not regression thresholds, hardware-normalized guarantees, or allocation measurements.
* FastCDC comparator limitation: `fastcdc-v2020-level1-raw-bytes` rows measure raw byte-slice CDC throughput only; they do not enforce Mach record-boundary semantics or prove Prolly-Bao tree/proof equivalence.
* Flattened-span comparator limitation: `mach-record-safe/flattened-spans/...` rows remove scattered record-vector fixture layout from the timed loop, but they still measure boundary-only `ChunkSpan` output rather than Prolly-Bao tree construction, proof/witness size, or storage effects.
* Current candidate row families:
  + `mach-record-safe/prior-art-fixture/...`: current record-safe public API over the deterministic prior-art fixtures.
  + `prior-art-candidate/not-proof-equivalent/okra-blake3-record-threshold/...`: unkeyed BLAKE3 complete-record threshold using Okra's `2^32 / Q`, `Q = 32` predicate.
  + `prior-art-candidate/not-proof-equivalent/dolt-key-only-salted-u32-cdf/...`: key-only salted hash with complete-record byte accounting.
  + `prior-art-candidate/not-proof-equivalent/hybrid-mach-gear-hard-cap/...`: Mach Gear scanner row with distinct hard-cap reporting.
  + `mach-record-safe/flattened-spans/...`: current record-safe public API over one canonical byte buffer plus precomputed record spans.
* Current non-timing guard: benchmark registration validates representative source-file-like and adversarial fixtures for exact coverage and expected Okra/Dolt/hybrid trigger categories before timed loops are registered.
* `rkyv` and Arrow note: neither was added to `storage-chunker`.
  `rkyv` remains a possible future dev-only fixture-materialization/internal archived-view experiment with explicit format controls; Arrow remains a possible future derived analytics/export format.
* Benchmark rows:
  + Row:
    - Name:

      ```text
      chunker/boundary-only/mach-record-safe/small
      ```

    - Time: `[17.911 µs 17.930 µs 17.952 µs]`.
    - Throughput: `[1.6999 GiB/s 1.7020 GiB/s 1.7038 GiB/s]`.
  + Row:
    - Name:

      ```text
      chunker/boundary-only/fastcdc-v2020-level1-raw-bytes/small
      ```

    - Time: `[11.820 µs 11.831 µs 11.844 µs]`.
    - Throughput: `[2.5766 GiB/s 2.5794 GiB/s 2.5818 GiB/s]`.
  + Row:
    - Name:

      ```text
      chunker/boundary-only/mach-record-safe/medium
      ```

    - Time: `[572.33 µs 572.83 µs 573.39 µs]`.
    - Throughput: `[1.7031 GiB/s 1.7048 GiB/s 1.7063 GiB/s]`.
  + Row:
    - Name:

      ```text
      chunker/boundary-only/fastcdc-v2020-level1-raw-bytes/medium
      ```

    - Time: `[401.60 µs 401.97 µs 402.38 µs]`.
    - Throughput: `[2.4270 GiB/s 2.4295 GiB/s 2.4317 GiB/s]`.
  + Row:
    - Name:

      ```text
      chunker/boundary-only/mach-record-safe/low-entropy
      ```

    - Time: `[572.67 µs 573.10 µs 573.55 µs]`.
    - Throughput: `[1.7027 GiB/s 1.7040 GiB/s 1.7053 GiB/s]`.
  + Row:
    - Name:

      ```text
      chunker/boundary-only/fastcdc-v2020-level1-raw-bytes/low-entropy
      ```

    - Time: `[471.36 µs 471.88 µs 472.49 µs]`.
    - Throughput: `[2.0669 GiB/s 2.0695 GiB/s 2.0718 GiB/s]`.
  + Row:
    - Name:

      ```text
      chunker/boundary-only/mach-record-safe/local-edit/insert-near-beginning
      ```

    - Time: `[287.91 µs 288.23 µs 288.55 µs]`.
    - Throughput: `[1.6938 GiB/s 1.6957 GiB/s 1.6976 GiB/s]`.
  + Row:
    - Name:

      ```text
      chunker/boundary-only/fastcdc-v2020-level1-raw-bytes/local-edit/insert-near-beginning
      ```

    - Time: `[200.92 µs 201.17 µs 201.45 µs]`.
    - Throughput: `[2.4262 GiB/s 2.4296 GiB/s 2.4327 GiB/s]`.
  + Row:
    - Name:

      ```text
      chunker/boundary-only/mach-record-safe/local-edit/update-near-middle
      ```

    - Time: `[286.84 µs 287.18 µs 287.54 µs]`.
    - Throughput: `[1.6981 GiB/s 1.7002 GiB/s 1.7023 GiB/s]`.
  + Row:
    - Name:

      ```text
      chunker/boundary-only/fastcdc-v2020-level1-raw-bytes/local-edit/update-near-middle
      ```

    - Time: `[200.03 µs 200.30 µs 200.60 µs]`.
    - Throughput: `[2.4341 GiB/s 2.4377 GiB/s 2.4410 GiB/s]`.
  + Row:
    - Name:

      ```text
      chunker/boundary-only/mach-record-safe/local-edit/delete-near-end
      ```

    - Time: `[286.32 µs 286.71 µs 287.13 µs]`.
    - Throughput: `[1.6989 GiB/s 1.7014 GiB/s 1.7037 GiB/s]`.
  + Row:
    - Name:

      ```text
      chunker/boundary-only/fastcdc-v2020-level1-raw-bytes/local-edit/delete-near-end
      ```

    - Time: `[200.01 µs 200.25 µs 200.52 µs]`.
    - Throughput: `[2.4327 GiB/s 2.4360 GiB/s 2.4389 GiB/s]`.
* Flattened-span release subset:
  + Command:

    ```text
    mise exec -- cargo bench -p storage-chunker --bench chunker -- mach-record-safe
    ```

  + Branch: `feat/prolly-bao-flattened-spans`.
  + Base branch: stacked on `feat/prolly-bao-realistic-benchmarks` at `97f56c4`.
  + Interpretation: these rows compare borrowed-record and flattened-span Mach scanner input layout only.
    They are not profile-selection, proof-size, or witness-size evidence.
  + Observed flattened-span rows:
    - `small`:
      * Time: `[18.161 µs 18.190 µs 18.224 µs]`.
      * Throughput: `[1.6746 GiB/s 1.6777 GiB/s 1.6804 GiB/s]`.
    - `medium`:
      * Time: `[580.19 µs 580.62 µs 581.08 µs]`.
      * Throughput: `[1.6806 GiB/s 1.6819 GiB/s 1.6832 GiB/s]`.
    - `low-entropy`:
      * Time: `[580.68 µs 581.20 µs 581.75 µs]`.
      * Throughput: `[1.6787 GiB/s 1.6802 GiB/s 1.6818 GiB/s]`.
    - `local-edit/insert-near-beginning`:
      * Time: `[290.52 µs 290.79 µs 291.08 µs]`.
      * Throughput: `[1.6791 GiB/s 1.6808 GiB/s 1.6823 GiB/s]`.
    - `local-edit/update-near-middle`:
      * Time: `[290.48 µs 290.78 µs 291.10 µs]`.
      * Throughput: `[1.6774 GiB/s 1.6792 GiB/s 1.6810 GiB/s]`.
    - `local-edit/delete-near-end`:
      * Time: `[290.02 µs 290.28 µs 290.56 µs]`.
      * Throughput: `[1.6788 GiB/s 1.6804 GiB/s 1.6820 GiB/s]`.
    - `prior-art-fixture/source-file-like-records`:
      * Time: `[108.73 µs 108.86 µs 108.99 µs]`.
      * Throughput: `[1.6800 GiB/s 1.6821 GiB/s 1.6840 GiB/s]`.
    - `prior-art-fixture/task-record-like-records`:
      * Time: `[54.451 µs 54.495 µs 54.538 µs]`.
      * Throughput: `[1.6787 GiB/s 1.6800 GiB/s 1.6814 GiB/s]`.
    - `prior-art-fixture/low-entropy-keys`:
      * Time: `[54.423 µs 54.467 µs 54.507 µs]`.
      * Throughput: `[1.6796 GiB/s 1.6809 GiB/s 1.6822 GiB/s]`.
    - `prior-art-fixture/fixed-width-value-updates`:
      * Time: `[46.453 µs 46.491 µs 46.531 µs]`.
      * Throughput: `[1.6806 GiB/s 1.6821 GiB/s 1.6834 GiB/s]`.
    - `prior-art-fixture/large-value-references`:
      * Time: `[53.902 µs 53.953 µs 54.006 µs]`.
      * Throughput: `[1.6952 GiB/s 1.6969 GiB/s 1.6985 GiB/s]`.
    - `prior-art-fixture/adversarial-boundary-seeking-records`:
      * Time: `[532.53 µs 533.18 µs 533.91 µs]`.
      * Throughput: `[1.6933 GiB/s 1.6956 GiB/s 1.6977 GiB/s]`.
  + Immediate reading: flattened-span rows stayed close to borrowed-record Mach rows on this run.
    The suspected scattered-fixture layout cost did not dominate the earlier Mach/FastCDC throughput gap.
* Dependency scope:
  + Criterion, `fastcdc`, and `blake3` are dev-only benchmark dependencies.
  + `fastcdc` is used only as the raw byte-slice comparator.
  + `blake3` is used only by the Okra-style complete-record threshold comparator.
  + `rkyv` and Arrow are not `storage-chunker` dependencies.
  + No runtime `chunk`, semantic text chunker, storage, SQL/DataFusion, IPLD/CAR, Iroh, Git, Automerge, async runtime, random-number, Arrow, `rkyv`, or database dependency is measured as part of this crate.

## designed direction

* Maintain release-build benchmark baselines separately from crate and workspace verification gates.
* Track boundary-only throughput separately from any future materialized payload chunk, storage, BLAKE3, Bao proof, or Prolly-Bao tree-construction metrics.
* Keep low-entropy and local-edit perturbation fixtures as first-class benchmark cases because they exercise scanner behavior that average random-looking input can hide.

## open decision

* Whether a dev-only `rkyv` fixture-materialization benchmark or separate Arrow analytics/export benchmark is useful after flattened-span results are reviewed.
* No regression threshold is selected yet.
* No allocation-count baseline exists yet for boundary output allocation versus payload-copy avoidance.
