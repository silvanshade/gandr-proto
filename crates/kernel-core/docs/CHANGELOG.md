# Changelog

The format is hand-maintained and grows only with real changes; it is not auto-generated.

## Unreleased

### Changed

* Defunctionalized the S1 checker and retired the depth budget (slice 3, gandr-98o): the six formerly mutually recursive checking/synthesis/type-formation methods become one **defunctionalized machine** (`check::run`) over a goal register, a produced register, a heap frame stack, and an explicit typing-context stack, with type formation and conversion as the two self-contained iterative walks it calls directly.
  The `Depth::LIMIT` recursion budget and `KernelError::DepthLimitExceeded` are **removed** — totality on adversarial depth is now structural (no recursion, no stack to overflow), meeting the `docs/workflow/rust.md` "input recursion: none" discipline for the kernel checker.
  The public API (`Environment::add_decl`, `Checker::check_value_type`, `Checker::check_definition`) is unchanged; the module docs carry an old-arm ↔ machine-step correspondence table for TCB audit.
  The retired depth-budget rejection test inverts into small-stack totality witnesses: the machine now admits a ~200k-deep well-typed pair definition and a ~200k-deep bind definition rather than rejecting them.

### Fixed

* Deallocation and duplication totality (slice 3, gandr-i3i): the four recursive owned S1 enums (`Value`, `Computation`, `ValueType`, `CompType`) now `Drop` and `Clone` **iteratively** over an explicit heap worklist rather than through the derived recursive glue, so an adversarial-depth term or type — which export `decode` can build from bytes — never overflows the stack when it is destroyed (directly, or indirectly through a `KernelError` payload or a decoded declaration) or duplicated.
  `Drop` extracts each node's children by placeholder-swap so the compiler's glue only sees leaves; `Clone` is a two-stack goal/produced walk (the `conv`/`export` idiom).
  Adding `Drop` forbids by-value moves out of these types (E0509), so the synthesizing checker arms extract through `mem::replace` take-helpers instead of destructuring.
  `PartialEq`/`Eq`/`Hash` stay derived (recursive), exercised only by tests on bounded fixtures.
  Covered by the `tests/hardening.rs` small-stack deep-chain suite.

### Added

* Initial implementation (slice 3, stage B2.1): the S1 pure polarized core of the minimal certified kernel — the closed de-novo CBPV term/type language, the `Def`/`Axiom` declaration vocabulary, the per-declaration prenex level context with the universe rule over `gandr-kernel-strata`, the append-only environment with its single `add_decl` choke point (the one warned `add_decl_unchecked` bypass and the `#print axioms` audit, on an unforgeable `CheckedId`), the zero-inference bidirectional S1 checker (total on adversarial depth), the C5-quarantined iterative conversion (type-only at S1, coinciding with structural equality; the value-fragment α-equality present and quarantined), the closed evidence-carrying `KernelError` vocabulary, kernel-native golden fixtures, and the conversion/checker property differentials.
* K5 export (slice 3, stage B2.2): the re-checkable export module (`export`) — `write` serializes an `Environment` to self-contained, deterministic canonical bytes (admission-ordered declarations, `BTreeMap`-sorted level atoms, per-declaration admission marks, and a version header; the transitive audit sets are recomputed on replay, not written); `decode` is the total validating reader over a closed error vocabulary (`DecodeError`: the truncation / bad-tag / malformed rejection triple, plus named refusals for reserved declaration kinds, non-empty reserved slots/sections, and an unknown version); `read` replays the decoded sequence through the choke point (`ReadError` holding the decode and re-admission planes distinct).
  Levels rebuild through the strata smart constructors and a whole-artifact canonical-bytes check rejects non-canonical encodings; term and type trees decode iteratively over an explicit worklist.
  The seven ratified format-plane reservations (the four reserved declaration kinds, structured names, the four per-`Def` annotation slots, and the reserved minted-atom table) are present and empty at v0.
  Round-trip, determinism, and rejection-totality property differentials accompany it.
