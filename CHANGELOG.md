# Changelog

Notable changes to the gandr workspace, newest first.
This is the single workspace changelog; the per-crate `docs/` directories it replaces are leaving the tree, and their salvageable history is preserved here.
Entries dated before 2026-07-21 record the relevant tier's lineage before its absorption into this tree.

## 2026-08-13

- **surface-engine / core-incremental**: the interactive session now retains its ordered item program and checkpoint set, resumes each appended submission through `gandr-core-incremental`, and derives item outcomes from the resumed typings instead of re-running the interim session-local typing path.
  `Resume` now carries the edited checkpoints needed by the next append; the script runner remains on its one-shot path.
  The real-front-end and parser-free resume differentials, the session append differential, and all 119 executable corpus examples cover the cutover.

## 2026-08-11

- The retirement continues: the `surface-syntax`, `surface-parser`, `surface-grammar`, and `surface-render-remote` sets leave the tree.
  `surface-corpus` shipped no docs directory, so nothing relocates for it.
  The design material the four sets carried is corrected at its new home rather than copied — including the grammar STATUS's stale grammar fingerprint and declared mold count (two updates behind the tree's pinned values) and the parser STATUS's stale test count, line count, and corpus-gate narrative.
- The retirement completes with the tier's last two sets: `surface-engine` and `surface-driver` leave the tree.
  The engine record's stale figures are repaired at its new home rather than copied — its STATUS recorded 27,997 source lines across 25 files and 379 tests where the tree measures 33,441 lines across 35 source files and 463 tests — and the driver's manifest head claim that each deferred face needs an unlanded crate (false for three of the four it named) is repaired in the same area.

## 2026-08-10

- The per-crate `docs/` tier (STATUS, ADR, CHANGELOG, METRICS, OPTIMIZATION) begins its retirement: the `storage-artifact`, `storage-chunker`, and `storage-prolly-trees` sets leave the tree, and this file becomes the workspace changelog of record.
  The design material they carried is corrected at its new home rather than copied.
- The retirement continues: the `kernel-core` and `kernel-strata` sets leave the tree.
- And the `theory-orders` set — its order-maintenance decisions, deferrals, and coverage record are corrected at their new home.
- And the `core-checker` set — the workspace's largest record, including its layout audit's reorganization findings, is corrected at its new home.
- **core-checker**: extracted the A2.3 incremental trio (`checkpoint`, `footprint`, `region`, with the four boundary wrappers only they used and the `theory-orders` dependency) into `gandr-core-incremental` — the engine existed twice in the workspace, and the extraction takes the better-written half of each differing pair into one crate; nothing here consumed the trio, so the move is a re-home.
  Same landing: `effect::host` now owns the canonical alloc-only `Exec` / `Fs` / `Proc` / `Env` signatures beside the representation-independent host seam, so surface lowering and the native runtime share one authority.

## 2026-08-09

- **surface-engine**: handed the item-granular checkpoint engine to `gandr-core-incremental` — removed this crate's second copy (the `footprint` and `checkpoint` modules, near-identical to core-checker's by common descent), retired `lower::LoweredItem` in favour of the seam's `region::Item` it was field-for-field identical to, dropped the `gandr-theory-orders` edge that left with the engine, and removed four `boundary` wrappers that named the extracted engine's vocabulary with no use site.
  The from-scratch-versus-resume differential stays here in `tests/incremental.rs`, resuming through the item seam against `prelude_ctx` — the extracted crate's own gate runs parser-free against a test double, so the two gates cover the engine and the front end separately.

## 2026-08-08

- **surface-engine**: gave the source driver a path-shaped face — `run::run_source_file` reads a source file and runs it exactly as `run::run_source` runs source text, reporting the path in `RunFileError::Read` so a diagnostic can name the file, with a source-level failure travelling unchanged.
  This is the seam the `gandr <file>` script runner stands on; a `#!…` shebang line needs no stripping anywhere (it is grammar trivia), so an executable script and its text run identically.
- **surface-driver**: un-stubbed the toolchain driver with the script-runner face (front-end rung F5) — `gandr <file>` runs one source file through `run_source_file`, `gandr --help` prints the accepted command line, and every other command-line shape is refused, so a deferred face fails loudly rather than being read as a filename.
  The outcome-to-status contract is the crate's substance: `0` for a value terminal, the script's own `proc.exit` code reduced to a byte, `1` for a blame, stuck configuration, or fatal host abort, and `2` for a source that never reached the machine; a successful run prints nothing of the driver's own.
  `scripts/agda-deps.gandr` is the first production consumer, run by `mise run agda:deps`.

## 2026-08-07

- **surface-grammar**: the nested generator block becomes the one `data` form — the declaration head binds the family's parameters once as typed binders and carries the index arity as the head annotation, every generator member is a judgment with its telescope kept local, and the retired Haskell-style shapes (the bare-parameter head, the field-tuple member, the comma member separator) stay admissible so the stage-0 elaborator declines them with the respelling; `codata` takes the same head discipline.
  Same landing: every `sign` member is terminated by `;` and the terminator is load-bearing — an unterminated member list was a clean parse of the wrong tree — dissolving the `sort` collision recorded when the block form landed, and a `sign` block may be named with a primitive-type spelling (the uppercase reservation is a preference with a generic-label fallback, not a ban).
  The declared mold count reaches 1783, pinned in the walk contracts.

## 2026-08-02

- **surface-engine**: landed the circuit surface check — the `circuit` module confirms every arrow of the ruled circuit block form against the kind of the thing it belongs to (a declaration's from its kind keyword, a body line's from the applied head's kind), a row disagreement is a named, located diagnostic, and the reserved `<->` declines naming the reversible-oper lane; the check deliberately stops at arrows and names.
  Same landing: `desc_elab`'s `rule` member reads the ruled `==>` face arrow, declining the retired `~>` with the respelling located at the arrow itself, and the `cst_read` module was extracted from `desc_elab` — the flat-tile-run `Reader` / `Cursor` both the stage-0 elaborator and the circuit check walk declarations with.
- **surface-grammar**: landed the ruled circuit block form in the checked surface — the `sign` block with `sort` / `data` / `oper` / `rule` judgment members, the four-glyph arrow grid (`-->` / `<->` circuit 1-cell formers, `==>` / `<=>` rewrite faces), arrow-separated two-sided port lists, and the top-level `oper` / `rule` declaration with its `node` / `feed` body statements.
  The grammar admits any grid glyph at every arrow position by design — arrow-kind confirmation is an environment fact the checker owns — and a top-level circuit declaration takes parenthesized sides so no Item form ends in a sort hole.
  Same landing: the `data` / `codata` `rule` member's face arrow migrates from `~>` to `==>`, with the retired `~>` kept admissible in the arrow slot so the decline names the respelling.
- **surface-parser**: lexed the ruled circuit arrow grid and the primed word — the four grid glyphs sit ahead of the shorter tiles each strictly extends in the longest-first table, a word may carry trailing primes (`′`, U+2032) while ASCII `'` stays the shell single-quote opener, and the molder reserves the item-position circuit leads `sign` and `oper` while `sort` / `node` / `feed` stay contextual.
  Same landing: the retired `~>` stays in the multi-punctuation table so a stale face munches as one tile and the migration decline can name it.

## 2026-07-21

- **core-checker**: added the elaborator-side kernel bridge — a total, iterative lowering from the checked core CBPV forms into the kernel's closed S1 vocabulary, rejecting every out-of-S1 node structurally with a precise `BridgeRejection`, erasing the operationally-transparent forms, resolving names through a `BridgeContext`, and applying the value-polarity declaration convention (a computation definition enters as a thunk `U C`); the kernel re-derives every obligation.
- **kernel-core**: retuned the reader budget constants on real corpus telemetry (the export exit gate over the 21 S1-eligible corpus items and 6 kernel-native goldens): `MAX_EXPANDED_TERM_WORK` `1 << 24 → 1 << 20`, `MAX_TABLE_ENTRIES` `1 << 20 → 1 << 18` — the binding floor is the deepest artifact the kernel itself round-trips (~200k entries, ~400k expanded work), and the export boundary goldens now derive from the constants so a future retune needs no hand-editing.
  Same landing: the kernel-native levelled-universe and explicit-lift goldens, and the artifact-total amplification bound `MAX_ARTIFACT_EXPANDED_WORK` with the deterministic `DecodeMetrics` the export exit gate records.
- **storage-artifact**: landed the outer-layer CAS wiring for kernel v1 export artifacts — the record model over `write_segmented`, declaration-granular chunking into a `BlockStore`-backed prolly tree, the canonical versioned `ArtifactManifest`, and `ArtifactIdentity = BLAKE3(manifest)` as the b3sum-provenance successor, with the two-wall discipline (outer integrity, inner validity) pinned in docs and history-independence pinned by a differential.
- **storage-chunker**: reconciled the absorbed documentation — removed benchmark claims that have no executable source in this tree; the crate ships no benchmark target and an empty dev-dependency table.
- **surface-engine**: ported at rung F3 — the complete CST-to-core front-end engine (total lowering, origin tracking, structured diagnostics and goals, prelude/host/attribute tables, edit-action reconstruction, linking, stateful sessions, and the one-shot source driver), re-pointing seven predecessor dependencies one-to-one, splitting the eighth (`gandr-core`) by authority into `gandr-core-checker` and `gandr-core-sequent`, and adding the host-capability adapter `gandr-runtime-effects` as the tenth edge.
  Session and linker evaluation moved to the L machine; `run::run_source` relocated from the runtime so the engine owns the language-level source entry; statement recognition and diagnostics adapted to the reboot grammar's `val` / `run` spellings with no compatibility alias left.
- **surface-syntax / surface-render-remote**: ported into the tree at front-end rung F0 — the flat-arena CST leaf (the `Cst` arena, the checked `CstBuilder`, the framed FNV-1a structural-identity hash, the whitespace-insensitive structural diff) and the leaf wire-protocol types of the typing-machine inspection render bus (the `present` seam and the versioned `wire` frame + delta schema), verbatim ports of the wyrd `gandr-syntax` and `gandr-render-proto` crates with retired-tracker provenance dropped.
- **surface-grammar**: ported at rung F1 — the checked precedence-bounded grammar core plus the mold-driven highlighter, re-pointing the three workspace dependencies to their reboot homes and omitting the `parity` feature entirely (tree-sitter returns at rung F6, so the default dependency graph is tree-sitter-free by construction); the parser-coupled contracts suite was parked to F2.
- **surface-parser**: ported at rung F2 — the resumable push-machine melder plus the obligation taxonomy, dropping the vestigial direct graph edge (the grammar's re-exports carry every precedence type the parser uses) and restoring the grammar's six parser-coupled contracts tests through the deliberate dev-only cycle-break edge.

## 2026-07-20

- **kernel-core**: minted the S1 core of the minimal certified kernel — the closed polarized CBPV term/type vocabulary, the `Def`/`Axiom` declaration vocabulary with prenex level contexts, the append-only environment with the single `add_decl` choke point (one warned bypass, the print-axioms audit, an unforgeable `CheckedId`), the zero-inference bidirectional checker, and the quarantined conversion — followed by the K5 export: the v1 maximal-sharing subterm-table writer with content-keyed dedup, the total validating reader with expanded-work budgets, the canonical re-encode-compare, and the seven format-plane reservations held empty.
- **kernel-core**: restructured S1 terms into the D1(C) per-environment append-only arena with typed node ids — shallow derived `Clone`/`Drop`/equality, flat teardown — retiring the hand-written iterative `Drop`/`Clone` worklists the owned-tree representation had required for adversarial-depth totality.
- **kernel-core**: defunctionalized the S1 checker and retired the depth budget — the mutually recursive checking, synthesis, and type-formation methods became one machine over goal and produced registers, a heap frame stack, and an explicit context stack, with fail-closed register projections; totality on adversarial depth is structural, and the retired rejection tests inverted into ~200k-deep admission witnesses.

## 2026-07-19

- **core-checker**: ported into the tree — the core CBPV bidirectional checker, the defunctionalized typing machine, the grade semiring carrier, subtyping, the checker ≡ machine conformance suite, and the staged extensions (A2.1 integer literals, A2.2 holes, and later effects, control, identity, pattern matching, and declared data), including the `mark` total semantic marking layer (a third, additive realization of the type system, recovering at each abort site instead of failing fast, oracle-tested against the recursive checker) with its type content hash and hash-consing interner, and `Hash` on the whole type graph enabling them.
- **kernel-strata**: ported into the tree as the first `gandr-kernel-*` crate (the lineage entries below date from the absorbed implementation).

## 2026-07-13

- **kernel-strata**: initial implementation — the free-fragment level oracle: the always-canonical `Level` over the `{0, +1, max}` algebra (derived equality is the level-equality oracle), the domination-based `leq`/`lt` oracle with checkable witness/refutation evidence and validators, and the checked `eval` semantic anchor.
- **kernel-strata**: landmark posets and entailment — Bezem–Coquand loop-checking (TCS 913, 2022) over declared variable-only constraints, admission as the Corollary 3.5 dichotomy with evidence on both sides (`ConsistencyWitness` homomorphism or replayable `LoopWitness`), Corollary 3.4 entailment with `EntailmentWitness`/`EntailmentCountermodel`, the pinned-bottom constant encoding, and the empty-poset property differential against the free oracle.

## 2026-06-21

- **theory-orders**: initial implementation — the self-contained order-maintenance structure for the incremental pipeline: `OrderMaintenance<T>` (single-level list-labeling, O(1) comparison, O(log² n) amortized insertion with the density-capped relabel keeping the structure total), generation- and structure-checked `Pos` handles, the `Interval` pre/post-order containment query, the reference-oracle property test, and the `order` criterion bench.

## 2026-06-05

- **storage-prolly-trees**: compact proof node material — non-membership proofs carry only the root, the selected leaf, and an optional required successor leaf; range proofs carry the root plus the contiguous selected leaf interval; witness transcript verification reuses the compact material and rejects omitted or extra leaves, swapped children, and non-root-first order.
  Added the fixed-width and source-file-like profile-curve benches.
- **storage-prolly-trees**: exported encoded-node layout inspection (`inspect_encoded_node`, `EncodedNodeKind`, `EncodedNodeLayout`) and the in-memory `PackedSegmentStore` prototype, both behind the unchanged encoded-node `BlockStore` boundary.
- **storage-chunker**: scanner hot-path cleanup — `ChunkScan` copies immutable scan-local limits, initial Gear state, and cut mask instead of retaining a parameter borrow; no public API, commitment, or dependency changed.

## 2026-06-02

- **storage-prolly-trees**: canonical snapshot byte-stream bridge — snapshot encode, materialize, and verify entry points with contract coverage that runs the real Bao verifier path over the emitted bytes as adapter evidence in dev tests (`bao` stays dev-only).
- **storage-prolly-trees**: native witness transcript contract — versioned transcripts for membership, non-membership, and range query responses under an agreed `TreeRoot` and `TreeParams`, terminating in a deterministic end summary binding version, kind, root, parameters, body, and proof nodes, verified through the existing proof verifiers rather than a second verification path.
- **storage-prolly-trees**: minimal Iroh interop rejection evidence — canonical snapshot and witness bytes are deterministic and exact-byte cacheable under flat BLAKE3 content hashes, with root-bound verification still required after readback; no Iroh dependency added, because the inspected API is peer-to-peer transport, not local blob storage.
- **storage-prolly-trees**: recorded the naming boundary — the crate name is evocative and Bao-inspired; native witnesses verify ordered-map facts under an agreed root, not byte offsets under a Bao blob hash.
- **storage-prolly-trees**: first proof API slice — store-independent membership, non-membership, and range proofs that recompute BLAKE3 node identity and fail closed on malformed or tampered material; tree construction rejects unsorted input and duplicate keys rather than choosing a merge policy; the thirteen-row local Criterion baseline recorded, with direct Dolt and Bao comparisons deferred for want of a semantically fair harness.
