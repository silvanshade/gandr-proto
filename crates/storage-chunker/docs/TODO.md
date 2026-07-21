# TODO

## current

* Added after prior benchmark docs updated `METRICS.md` and `CHANGELOG.md` but omitted mandatory crate-scoped `ADR.md`, `STATUS.md`, and `TODO.md`.
* Root cause: orchestration/assignment acceptance criteria did not explicitly require the full mandatory crate-doc set, even though coding guidance listed it generically.
* Correct, fail-closed record-safe chunking over canonical ordered records remains higher priority than micro-optimization.
* Benchmark source now includes prior-art candidate row families for Okra-style BLAKE3, Dolt-style, and hybrid hard-cap surfaces, plus Mach `chunk_spans` flattened-buffer rows.
  No default/runtime profile changed.

## designed direction

* Use the recorded apples-to-apples Mach `chunk_spans` release baseline to decide whether fixture layout explains part of the Mach/FastCDC throughput gap.
  This run suggests it is not dominant.
* Right-size boundary output capacity instead of reserving one `ChunkSpan` per record.
  Use bytes/target plus a small margin when safe.
* Profile before tuning:
  + Criterion plus Instruments or flamegraph
  + allocator behavior
  + inner-loop codegen
* Repeated chunk limit reads now use scan-local `ChunkLimits`; the benchmark subset was mixed, so further hot-loop work still needs profile evidence.
* `gear_for()` now uses an invariant-justified unchecked read from the fixed 256-entry Gear table; the benchmark subset did not show a clear throughput win, so deeper inner-loop work still needs profiler evidence.
* Keep correctness/fail-closed behavior first.
  Any fast path must preserve exact record-boundary semantics, max/min caps, and precise errors.
* Run release Criterion baselines for the candidate row families:
  + `mach-record-safe/prior-art-fixture/...`
  + `prior-art-candidate/not-proof-equivalent/okra-blake3-record-threshold/...`
  + `prior-art-candidate/not-proof-equivalent/dolt-key-only-salted-u32-cdf/...`
  + `prior-art-candidate/not-proof-equivalent/hybrid-mach-gear-hard-cap/...`
* Keep the recorded flattened-span release subset current if fixture generation or scanner internals change:
  + `mach-record-safe/flattened-spans/...`
* Add benchmark distribution summaries:
  + chunk count
  + p50/p95/max bytes
  + cap-hit rate
  + trigger reason counts
* After candidate chunking is integrated into tree construction, compare proof-size effects against the current record-safe default.
  Do not treat boundary-only chunker rows as proof-size evidence.
* Decide whether key-only chunking remains an open design option or should be closed after tree/proof comparison.

## open decision

* Whether a specialized infallible internal scanner is worth adding after profile-guided changes.
  It must not weaken validation or public error behavior.
* Whether key-only chunking remains open after release candidate baselines and tree/proof-size comparison.
* Whether a dev-only `rkyv` fixture-materialization benchmark is useful after flattened-span results are reviewed.
* Whether Arrow belongs in later analysis/export tooling; it is not currently a chunker dependency.
