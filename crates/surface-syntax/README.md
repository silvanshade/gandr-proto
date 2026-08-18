# gandr-surface-syntax

The front-end's flat concrete-syntax-tree substrate: one compact arena representation shared by the parser bridge, the syntax-aware tests, and incremental diffing.

It is a true leaf.
The crate names no workspace dependency and no external dependency.
Grammar construction, molding and melding live in `gandr-surface-grammar` and `gandr-surface-parser`; this crate is only the arena they build into and the structural comparison they read back out.

## Current provision

- The `Cst` arena: one source buffer, one dense node arena, one flattened child arena.
  A `NodeId` is a dense arena location, stable only inside one tree, and never structural identity across trees.
- The node vocabulary.
  `NodeKind` separates token, cell, meld and wall nodes; `Material` separates space, grout and tile significance; `MoldPayload` is governed by material, so a tile carries an opaque mold identifier scoped by the tree's grammar fingerprint, grout carries a shape and its sort tag, and space carries neither.
  `NodeView` exposes read-only node data and zero-copy text.
- `CstBuilder`, the checked construction path.
  It owns the shared source buffer and the producing grammar revision, appends checked token and interior nodes, and validates parent closure when it finishes.
  `BuildError` names malformed ranges, invalid material and payload combinations, duplicate parents, unknown roots, orphan nodes, and arena bound failures.
  Unchecked arena state is never exposed.
- `diff`, the deterministic structural comparison.
  It returns matched equal-subtree-root pairs plus the conservative changed-or-unreadable roots on each side, rather than a prefix-changed-suffix triple.
  Alignment ignores space children, aligns significant children by a deterministic longest-common-subsequence pass over kind, payload and hash, recurses only into aligned pairs, and advances the new side first on ties.

Structural identity is a framed 64-bit FNV-1a subtree hash, written little-endian, so the same tree hashes the same on any machine.
Whitespace sits outside significant identity, and the mold sits inside it: the same text under a different mold is not the same significant syntax.
**Hash equality is a pruning hint rather than proof.** Debug builds re-verify significant structure and tile text before accepting an equal-hash subtree; release builds accept the quantified 64-bit collision risk.

The hash's width, frame vocabulary, byte order and algorithm are part of the crate's observed surface, not its internals, because consumers read hashes through `NodeView::hash`.
Changing any of them is a compatibility change.

## Planned but absent

- Nothing is scheduled.
  The crate is complete against its contract; it grows when the parser or the grammar needs a node class it cannot currently express.

## Using it

Build a tree through the checked builder, then compare two of them.

```rust
use gandr_surface_syntax::CstBuilder;
use gandr_surface_syntax::diff;

let old = build_tree(source_before)?;
let new = build_tree(source_after)?;
let changes = diff(&old, &new);
```

A `NodeId` from one tree means nothing in another.
Carry a hash, or carry the diff's matched pairs, when you need to talk about the same subtree across two trees.

## Theoretical ideas relied on

Tile-based syntax with molds and grout; framed non-cryptographic subtree fingerprints as a structural identity; longest-common-subsequence alignment as the basis of a deterministic tree diff; the distinction between an identity that prunes and an identity that proves.

## Primary references

- David Moon, Andrew Blinn, Thomas J. Porter and Cyrus Omar, _Syntactic Completions with Material Obligations_, 2025, `doi:10.1145/3763182` (arXiv:2508.16848) — the tile, mold and grout vocabulary this arena stores.
- Glenn Fowler, Landon Curt Noll, Kiem-Phong Vo, Donald Eastlake and Tony Hansen, _The FNV Non-Cryptographic Hash Algorithm_, IETF Internet-Draft `draft-eastlake-fnv` — the hash function the framed subtree fingerprint is built from, and the source of the collision properties the pruning-not-proof rule is set against.
