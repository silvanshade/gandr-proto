# Metrics

Line coverage (2026-06-21 baseline, `mise run cargo:llvm-cov`): the crate sits near 98% line coverage, with every module at or above ~94% (`control`, `ctx`, `grade`, `subtype`, `types` at 100%).
The workspace-level baseline and the regression-gate plan live in `docs/workflow/ci.md`.

No performance thresholds are set, and the crate has no benchmarks yet; the only tuning is the workspace release profile (`docs/gandr/`-level, not crate-specific).
