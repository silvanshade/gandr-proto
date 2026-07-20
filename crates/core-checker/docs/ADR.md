# Architecture Decisions

Crate-local decisions for `gandr-core` (the CBPV core checker, typing machine, and grade carrier).
These extend, and never override, the design corpus in `docs/gandr/` and its ADR log (`docs/adr/`); the global log records the major decisions, and this file holds the crate-implementation detail.

## current

### Dual implementation kept in lockstep for conformance

Decision: the core type system is realized twice — a direct-style recursive bidirectional checker (`checker`) and a defunctionalized typing machine (`machine`) — and the two are property-tested for step-for-step agreement.

Rationale: the machine is the operational artifact (ADR-9, the functional correspondence), and keeping the recursive checker alongside it gives an independent oracle, so a divergence in either is caught by the conformance suite rather than shipping.

Consequences:

* `conformance` generates terms and asserts the recursive checker's control-event log equals the machine's control-register sequence; a failing call logs no `Return`, mirroring the machine taking no step past a failed frame.
* Both implementations move together for any rule change — a new constructor lands in `checker`, `machine`, and the conformance generators in one change.

### Totality is the keystone invariant

Decision: the checker and the machine are total — they return a structured result (`error::TypeError`, an `Outcome`) for every input, never a panic or a divergence.

Rationale: the pipeline lowers every editor state (holes included) and feeds it here, so a partial checker would break the "no parse wall" guarantee A2 builds on.

Consequences:

* The lint wall (`panic`, `unwrap_used`, `expect_used`, `indexing_slicing`, … all denied) enforces the no-partial-function discipline mechanically.
* `machine::step` carries `- panics: none` in its `# Contract`; `Ctx::unbind` documents its single `debug_assert!` precondition.

### Semantic marking is a third, additive, total realization — never a modification of the lockstep twin

Decision: the Hazel-line semantic marking discipline (ADR-17 "marks not aborts"; Zhao et al. POPL 2024) lands as a _third_ realization of the type system — the `mark` module's total marking traversal — **additive to** the `checker`/`machine` pair, never a rewrite of either.
The recursive checker and the typing machine stay byte-for-byte as they are (the `conformance` lockstep is untouched); the marking traversal mirrors the checker's rules but is _total_ — it never aborts, decorating every node and converting each of the five `error::TypeError` abort sites into a localized mark plus a matched-`Unknown` recovery (generalizing the existing `Value::Hole`/`Unknown` recovery discipline from holes to all failures).

Rationale: localized marks require typing to _continue past_ the first failure (Zhao totality), which is the structural opposite of the checker's `?`-short-circuit and the machine's terminal `Outcome::Error`.
Rewriting the checker to mark-and-recover would force the same rewrite of the machine to preserve ADR-9 agreement, and would entangle the most-tested code in the crate.
A separate total traversal keeps that risk out: it is held honest not by the lockstep but by an **oracle against the checker** — for every `(term, dir)`, the checker accepting (`Ok(ty)`) is equivalent to the marking carrying no _error_ mark and synthesizing the same root type, and the checker rejecting (`Err`) forces at least one error mark; the marking is total on every input.
Because recovery is type-stable (check-mode recovers with the _expected_ type, exactly what the inlined Sub rule returns on success), a well-typed program takes only success paths, so the marker's per-node types coincide with the checker's there — the oracle is tight, not approximate.

Consequences:

* The marking reuses the in-crate typing helpers (`subtype::{value_subtype,comp_subtype}`, the `stack` destructures, `effect` row arithmetic) so its _success_ path cannot drift from the checker; only the recovery branches are new.
* Decoration is a side-table (`Marking : BTreeMap<NodePath, NodeFacts>`) keyed by a structural `NodePath` that mirrors the pipeline's `origin::resolve` child-index convention — never an inline field on the `Rc`-shared `Value`/`Comp` nodes, which would corrupt their derived `PartialEq` (load-bearing for conformance and the trace equality) and leak typing artifacts into the machine trace.
* The mark taxonomy reconciles the syntactic empty-hole mark and the five semantic failure kinds into **one** discipline, multiplied with the spec's grade-budget / effect-row / thunkability kinds; the Pantograph typed error-boundary `{t}_{T1/T2}` (POPL 2025, harvest-only) is realized as the node's dual-type `NodeFacts` plus a `Boundary { expected, actual }` on the mismatch marks — it never truncates the (reusable) decoration.
* Deriving the marking from the checker (as the machine is derived), or unifying the three realizations, is future work; the oracle is the soundness bridge meanwhile.

### Unchanged-type optimization: `Hash` on the type graph plus a canonical interner

Decision: the type hierarchy (`ValueType`/`CompType`/`Ty`, and transitively the sealed `Grade` and `EffectRow` carriers) derives `Hash`, and the `mark` module provides a `TypeInterner` giving O(1) canonical type equality.

Rationale: the per-node dual-type decoration wants O(1) "did this node's type change" comparisons (Porter's unchanged-type optimization) instead of the O(tree-size) structural `==`.
The graph is already canonical for this: `Grade` is `Fin(u64)`/`Omega` with an overflow clamp, and `EffectRow` is a name-ordered `BTreeMap`, so "equal up to row reordering" is exactly `==` today — only `Hash` plus an intern table were missing.

Consequences:

* `Hash` is added _inside_ the sealed `grade`/`effect` modules (the seal is about the representation, not its trait set), consistent with the derived `Eq`; it changes no behaviour and does not widen the carriers' cross-module surface.
* The interner is the optimization _substrate_; threading interned ids through an incremental dual-type cache across edits is the pipeline-side consumer's job (deferred), so the decoration store holds owned `Ty` for now.

### Scope boundary: the marking layer lands in core; source-identity wiring is deferred

Decision: `gandr-core` gains the marking layer, the decoration representation, the carrier, the mark taxonomy, the interner, and the checker oracle.
The pipeline-side wiring — replacing the reserved `diag::Report.marks` placeholder with a typed `Vec<Mark>`, mapping a `NodePath` to a `TermPath`/byte span through the `OriginMap`, and setting the `dirty` bit from the edit / order-maintenance layer — is a deferred follow-on (A2.3 / edit-layer, gated).

Rationale: the marking traversal is pure typing — span-free and dependency-free — so it belongs beside the checker it is oracle-bound to (decision D3/D4 keep _source identity_ out of core, which the structural `NodePath` respects: it carries no byte spans).
Wiring marks to source ranges and to the incremental dirty-frontier is exactly the source-identity / order-maintenance work that lives pipeline-side and is gated on the A2.3 base.

Consequence: `NodeFacts` carries `dirty: bool` (the representation is complete) but the marking traversal leaves it at its `false` default; the incremental layer is its producer.

## designed direction

### Representations stay extensible

Decision: the syntax and type enums are non-exhaustive, so later A-track stages (effects `F^ε`, control, grade ops, the linear context) can add constructors without breaking downstream matches.

Rationale: the live spine is pure CBPV plus the two A2 extensions (integer literals, holes / `Unknown`), and the frozen-but-unbuilt constructs are recorded in `docs/gandr/spec/core-ir-contract.md` §0.

Consequence: downstream matches keep a catch-all arm and stay forward-compatible.

## open decision

### no_panic strategy

Decision: `Grade::{leq,plus,times}` carry dtolnay `#[no_panic]` always-on (no opt-in feature), gated at the use site on `all(not(debug_assertions), panic = "unwind")`.
It is active in any dev release build (`cargo build --release`), where the example smoke (`mise run cargo:no-panic` / the `cargo-no-panic-smoke` CI job — a plain `cargo build --release --example no_panic_smoke`) links the ops so the link check fires and fails the build unless the optimizer proves them panic-free.
The lint wall plus the proptest and fuzz behavioural legs remain the primary panic-freedom evidence.

Rationale: `#[no_panic]` is a link-time check that needs an optimized build AND the unwind panic path.
The repo's release profile `panic = "abort"` defeats it — verified empirically (a planted panic linked clean under abort but was caught under unwind: `ERROR[no-panic]: detected panic in function leq`).
So `[profile.release]` deliberately drops `panic = "abort"` (defaults to `unwind`), making no_panic active in a plain release build; the real release (`cargo build-dist`, `.cargo/config.dist.toml` `panic = "immediate-abort"`, plus `[profile.dist] panic = "abort"`) keeps the abort posture, and the `panic = "unwind"` half of the cfg keeps no_panic inert there so it never interferes with the real release path.
The `not(debug_assertions)` half keeps it inert in debug, where the unoptimized build cannot prove panic-freedom and would spuriously fail.
The grade leaf ops are pure `u64` / enum (no alloc, `format!`, or generics), so the optimizer proves them panic-free; a lib-only build never links, so the example is the link vehicle.

Consequence: a panic regression in a grade op fails any dev release build that links it — the "always active" property.
Widening `#[no_panic]` beyond the grade leaf ops (to `machine::step`, `subtype`, `ctx`) is future work; alloc / `format!` / generic-heavy code (e.g. `thiserror` `Display`) will not prove clean and stays un-annotated.
