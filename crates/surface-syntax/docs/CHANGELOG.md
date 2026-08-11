# Changelog

The format is hand-maintained and grows only with real changes; it is not auto-generated.

## 2026-07-21 — Port the flat-arena CST leaf from wyrd (F0)

- `current`: Landed `gandr-surface-syntax`, the front-end's flat concrete-syntax-tree leaf (rung F0 of `docs/research/front-end-port-staging.md` §9), ported verbatim from the wyrd `gandr-syntax` crate.
- `current`: The port renames the package to `gandr-surface-syntax` (staging call O1) and drops the retired tracker-bead comments (staging call O3); the types, arena layout, framed FNV-1a structural-identity hash, and whitespace-insensitive structural diff are byte-for-byte the wyrd surface.
- `current`: Modules — `model` (the `Cst` arena, the `NodeKind`/`Material`/`MoldPayload` vocabulary, `TextRange`, `NodeView`), `builder` (`CstBuilder` plus the checked `BuildError` surface), and `diff` (`diff` → `Diff`, hash-pruned `SubtreeMatch`es, LCS alignment over significant children).
- `current`: Feature posture carried faithfully — `default = []`, `full = []`; zero workspace and external dependencies.
- `current`: Tests carried across — the module unit tests (`src/tests.rs`) and the public-API structural-diff integration suite (`tests/diff.rs`), green under nextest.
