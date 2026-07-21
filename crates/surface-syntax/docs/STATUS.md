# Status

Crate scope: `crates/surface-syntax` (package `gandr-surface-syntax`).

Status vocabulary in this file is limited to `current`, `designed direction`, and `open decision`.

## current

* The crate is the front-end's flat concrete-syntax-tree (CST) leaf: the compact arena representation shared by the parser bridge, syntax-aware tests, and incremental diffing.
  It has zero workspace dependencies and zero external dependencies — a true substrate leaf.
* Ported verbatim at rung F0 of the surface front-end port (`docs/research/front-end-port-staging.md` §9) from the wyrd `gandr-syntax` crate.
  The recut renames the package to `gandr-surface-syntax` (staging call O1) and drops the retired tracker-bead comments (staging call O3), with no change to types, arena layout, hashing, or diff behavior.
* Model (`model`): `Cst` stores one source buffer, one dense node arena, and one flattened child arena; `NodeId` is a dense arena location, stable only inside one `Cst` and not structural identity.
  `NodeKind` distinguishes token/cell/meld/wall nodes; `Material` distinguishes space/grout/tile significance; `MoldPayload` is material-governed (tiles carry an opaque `MoldId` scoped by the CST's `GrammarFingerprint`, grout carries a `GroutShape` plus its sort tag, space carries neither); `TextRange` stores source byte offsets; `NodeView` exposes read-only node data plus zero-copy text.
* Builder (`builder`): `CstBuilder` owns the shared source buffer and the producing grammar revision, appends checked token and interior nodes, and `finish` validates parent closure into a `Cst`.
  `BuildError` reports malformed ranges, invalid token/interior or material/payload combinations, duplicate parents, unknown roots, orphan nodes, and arena bound failures — the builder never exposes unchecked arena state.
* Diff (`diff`): `diff(old, new)` returns a `Diff` summary over two builder-produced CSTs; `matches()` lists hash-pruned equal-subtree roots, `unmatched_old()`/`unmatched_new()` list conservative changed-or-unreadable roots, and `SubtreeMatch` records the pruned old/new pairs.
  Alignment ignores space children, aligns significant children by deterministic LCS over `(kind, payload, hash)`, and recurses only into aligned pairs.
* Structural identity is a stable framed 64-bit FNV-1a subtree hash (the `StableHash` candidate): whitespace is outside significant identity (space bytes never enter parent hashes), mold is inside it (same text under a different mold is not the same significant syntax), and hash equality is a fast pruning candidate rather than proof.
  Debug builds re-verify significant structure and tile text before pruning an equal-hash subtree; release builds accept the quantified 64-bit collision risk.
  Changing the hash width, frame vocabulary, byte order, or algorithm is a compatibility decision, because consumers observe hashes through `NodeView::hash`.
* Tests: the crate carries its module unit tests (`src/tests.rs`) and the public-API structural-diff integration suite (`tests/diff.rs`) verbatim; both pass under nextest.

## designed direction

* Grammar construction, molding, melding, and consumer integration live in their owning crates (the F1 `surface-grammar` and F2 `surface-parser` rungs); this crate is only their arena substrate.

## open decision

* The wyrd crate carried a per-crate `docs/ADR.md` recording the flat-CST structural-identity and collision policy.
  The reboot homes ADRs at repo root (`docs/adr/`), so that rationale is distilled into this STATUS narrative (and already present in the `lib.rs` module docs) rather than ported as a per-crate ADR file.
  Whether it graduates into a reboot `docs/adr/` entry is an owner call, deferred with the rest of the surface front-end's design-record reconciliation.
