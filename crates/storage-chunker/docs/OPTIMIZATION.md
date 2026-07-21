# Optimization notes

## current

* Latest benchmark discussion:
  + Mach record-safe chunker measured about `1.69–1.70 GiB/s`.
  + FastCDC v2020 level-1 raw-byte comparator measured about `2.07–2.58 GiB/s`.
* FastCDC is consistently faster, but it is not semantically equivalent.
  It scans flattened bytes and does not enforce Mach between-record cuts.
* The benchmark source now includes `mach-record-safe/flattened-spans/...` rows that use `chunk_spans` over the same flattened fixture-buffer shape while preserving Mach between-record cuts.
* The first implementation was performance-aware but not profile-tuned:
  + borrowed records
  + no payload copying
  + precomputed cut mask/initial state
  + dev-only FastCDC comparator
  + no flamegraph/Instruments pass, allocator tuning, or assembly review has been done
* Round 1 autonomous tuning result:
  + A pre-scan capacity estimate for `ChunkSpan` output was tested and reverted after the benchmark subset regressed by about `0.6–2.0%`.
  + `gear_for(u8)` now uses an invariant-justified unchecked read from the 256-entry Gear table instead of carrying an impossible fallback branch.
  + The unchecked-read subset comparison against `main` showed only noise-level movement, so it is a codegen cleanup rather than a claimed throughput win.
* Round 2 autonomous tuning result:
  + `ChunkScan` now stores immutable scan-local copies of `ChunkLimits`, the initial Gear state, and the cut mask.
  + The subset benchmark was mixed versus Round 1, with borrowed medium and adversarial rows slower and flattened rows faster.
    Treat this as state-shape cleanup, not a proven throughput win.

## designed direction

* Prior-art chunk-profile rows are benchmark candidates only, not consensus defaults:
  + `mach-record-safe/prior-art-fixture/...` keeps the current record-safe public API on the new deterministic fixture families.
  + `prior-art-candidate/not-proof-equivalent/okra-blake3-record-threshold/...` measures an unkeyed BLAKE3 complete-record threshold comparator using Okra's reviewed `2^32 / Q`, `Q = 32` boundary predicate.
  + `prior-art-candidate/not-proof-equivalent/dolt-key-only-salted-u32-cdf/...` hashes only record keys while accounting for complete key+value record bytes and size-pressure cuts.
  + `prior-art-candidate/not-proof-equivalent/hybrid-mach-gear-hard-cap/...` keeps Mach Gear scanning but reports hard byte/record cap counts distinctly.
  + All Okra/Dolt/hybrid candidate rows are boundary-only benchmark candidates; they are not proof-equivalent tree-construction evidence and do not change the default profile.
  + Benchmark registration validates representative prior-art fixtures for exact byte/record coverage and expected trigger categories before timed loops run.
* Mach flattened-span rows are current scanner-cost evidence, not proof or tree construction evidence.
* No default profile changed.
  `ChunkerParams::default_fastcdc()` remains the current runtime profile; no Okra BLAKE3, Dolt key-only, or hybrid hard-cap row became a default on this branch.
* Updated gap reading after the flattened-span subset:
  + scattered record slices versus flattened bytes no longer looks like the dominant cost;
  + remaining likely costs are non-identical work versus raw FastCDC, heavier `ChunkSpan` metadata/output, a correctness-heavy checked hot path, repeated limit access, and no serious tuning pass yet.
* A release Criterion subset is recorded for `mach-record-safe` borrowed-record and flattened-span rows.
  The Okra/Dolt/hybrid candidate families still need release baselines before profile decisions.
* Recommended tuning order:
  1. Add chunk distribution summaries for borrowed-record and flattened-span rows.
  2. Right-size output capacity before micro-optimizing hot loop code.
  3. Make limit-hoisting and `gear_for()` table-access changes only with profile evidence or equivalent tests.
  4. Consider `rkyv` only as a dev-only fixture-materialization or internal archived-view experiment with explicit format controls.
  5. Keep Arrow out of the chunker unless a later analytics/export task needs a columnar derived view.
  6. Add a specialized infallible internal scanner only if correctness, record-boundary semantics, caps, and precise public errors are preserved.

## open decision

* Whether to keep only the checked public scanner or split a validated public entry point from an infallible internal hot path after measurement.
* Whether `rkyv` archived fixtures provide useful measurement signal beyond the current flattened-byte rows.
* Whether Arrow belongs in separate analytics/export tooling rather than this chunker crate.
