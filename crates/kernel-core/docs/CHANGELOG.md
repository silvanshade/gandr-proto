# Changelog

The format is hand-maintained and grows only with real changes; it is not auto-generated.

## Unreleased

### Changed

* D3-retuned the reader budget constants (stage B2.3, RQ-5) from their blind launch values, now that real corpus telemetry exists (the export exit gate over the 21 S1-eligible corpus items + 6 kernel-native C5 goldens): `MAX_EXPANDED_TERM_WORK` `1 << 24 → 1 << 20`, `MAX_TABLE_ENTRIES` `1 << 20 → 1 << 18` (`MAX_DECODED_LEVEL_OFFSET = 4096` unchanged).
  The binding floor is **not** the tiny S1 corpus (max per-declaration expanded work `5`, max `6` table entries) but the deepest artifact the kernel itself round-trips — the `tests/hardening.rs` decode witness at ~200k depth (~400k expanded, ~200k entries) — since the reader must accept every artifact the kernel legitimately admits and round-trips; each budget sits above that floor with headroom yet rejects an obvious billion-laughs (`2^30`) by three-plus orders of magnitude.
  The `tests/export.rs` boundary goldens now derive their diamond depth / entry count from the constants, so a future retune to another power of two needs no hand-editing.
* Defunctionalized the S1 checker and retired the depth budget (slice 3, gandr-98o): the six formerly mutually recursive checking/synthesis/type-formation methods become one **defunctionalized machine** (`check::run`) over a goal register, a produced register, a heap frame stack, and an explicit typing-context stack, with type formation and conversion as the two self-contained iterative walks it calls directly.
  The `Depth::LIMIT` recursion budget and `KernelError::DepthLimitExceeded` are **removed** — totality on adversarial depth is now structural (no recursion, no stack to overflow), meeting the `docs/workflow/rust.md` "input recursion: none" discipline for the kernel checker.
  The public API (`Environment::add_decl`, `Checker::check_value_type`, `Checker::check_definition`) is unchanged; the module docs carry an old-arm ↔ machine-step correspondence table for TCB audit.
  The retired depth-budget rejection test inverts into small-stack totality witnesses: the machine now admits a ~200k-deep well-typed pair definition and a ~200k-deep bind definition rather than rejecting them.
  The machine's produced-register projections are fail-closed: a goal↔frame polarity mismatch (unreachable under correct wiring) surfaces as the new `KernelError::CheckerRegisterFault` rejection rather than a fabricated type, so a wiring defect can never accept an ill-typed declaration.

### Fixed

* Deallocation and duplication totality (slice 3, gandr-i3i): the four recursive owned S1 enums (`Value`, `Computation`, `ValueType`, `CompType`) now `Drop` and `Clone` **iteratively** over an explicit heap worklist rather than through the derived recursive glue, so an adversarial-depth term or type — which export `decode` can build from bytes — never overflows the stack when it is destroyed (directly, or indirectly through a `KernelError` payload or a decoded declaration) or duplicated.
  `Drop` extracts each node's children by placeholder-swap so the compiler's glue only sees leaves; `Clone` is a two-stack goal/produced walk (the `conv`/`export` idiom).
  Adding `Drop` forbids by-value moves out of these types (E0509), so the synthesizing checker arms extract through `mem::replace` take-helpers instead of destructuring.
  `PartialEq`/`Eq`/`Hash` stay derived (recursive), exercised only by tests on bounded fixtures.
  Covered by the `tests/hardening.rs` small-stack deep-chain suite.

### Added

* Artifact-total amplification bound and decode telemetry (stage B2.3, gandr-4p3 — closed here): a third reader budget `MAX_ARTIFACT_EXPANDED_WORK` bounds the **saturating sum** of every declaration root's expanded size, closing the residual amplification the per-declaration `MAX_EXPANDED_TERM_WORK` leaves — `N` cheap declaration segments sharing one near-cap root (cross-declaration sharing) force `N × MAX_EXPANDED_TERM_WORK` checker work.
  It rides the existing one forward `expanded_size` scan as a single extra saturating accumulator and compare, rejecting `DecodeError::Malformed { ArtifactExpandedWork }` before replay; reader acceptance policy only, no wire-format or E4 change.
  The same scan now yields a public `DecodeMetrics` (table-entry count, max per-declaration expanded work, artifact-total expanded work), carried on `DecodedArtifact`, that the B2.3 export exit gate records as D3 telemetry.
  Boundary goldens for the new budget (accept at the cap, reject just over) and a many-cheap-segments amplification (rejected on the decode plane before the checker) join the `tests/export.rs` suite.
* C5 goldens (stage B2.3): the kernel-native levelled-universe and explicit-lift fixtures (`tests/goldens.rs`), authored **directly in kernel-core terms** — they have no core-checker counterpart, so they are never lowered through the B2.3 bridge — admitting universe/lift declarations through the choke point and round-tripping byte-identically through the K5 export (`write ∘ read ∘ write` is a fixed point), exercising the level-signature, universe-constant, universe-variable, and lift serialization the bridge-fed corpus never reaches.
  The B2.3 deep-drop obligation (massive-term design §7 item 7: a decoded-then-rejected deep DAG drops without stack overflow) is discharged **structurally** by the D1(C) arena and already witnessed by `tests/hardening.rs::deep_decoded_declaration_drop_is_total` (landed with the arena restructure), which B2.3 cites rather than re-adds.
* Initial implementation (slice 3, stage B2.1): the S1 pure polarized core of the minimal certified kernel — the closed de-novo CBPV term/type language, the `Def`/`Axiom` declaration vocabulary, the per-declaration prenex level context with the universe rule over `gandr-kernel-strata`, the append-only environment with its single `add_decl` choke point (the one warned `add_decl_unchecked` bypass and the `#print axioms` audit, on an unforgeable `CheckedId`), the zero-inference bidirectional S1 checker (total on adversarial depth), the C5-quarantined iterative conversion (type-only at S1, coinciding with structural equality; the value-fragment α-equality present and quarantined), the closed evidence-carrying `KernelError` vocabulary, kernel-native golden fixtures, and the conversion/checker property differentials.
* K5 export (slice 3, stage B2.2): the re-checkable export module (`export`) — `write` serializes an `Environment` to self-contained, deterministic canonical bytes (admission-ordered declarations, `BTreeMap`-sorted level atoms, per-declaration admission marks, and a version header; the transitive audit sets are recomputed on replay, not written); `decode` is the total validating reader over a closed error vocabulary (`DecodeError`: the truncation / bad-tag / malformed rejection triple, plus named refusals for reserved declaration kinds, non-empty reserved slots/sections, and an unknown version); `read` replays the decoded sequence through the choke point (`ReadError` holding the decode and re-admission planes distinct).
  Levels rebuild through the strata smart constructors and a whole-artifact canonical-bytes check rejects non-canonical encodings; term and type trees decode iteratively over an explicit worklist.
  The seven ratified format-plane reservations (the four reserved declaration kinds, structured names, the four per-`Def` annotation slots, and the reserved minted-atom table) are present and empty at v0.
  Round-trip, determinism, and rejection-totality property differentials accompany it.
