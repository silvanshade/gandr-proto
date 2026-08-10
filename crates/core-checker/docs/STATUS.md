# Status

Stage 1 (milestone A1) is complete: the core CBPV bidirectional type system is realized as both the recursive `checker` and the defunctionalized `machine`, with the two held in step-for-step conformance.

Implemented:

* CBPV bidirectional checker and the defunctionalized typing machine (`checker`, `machine`, `control`).
* The grade semiring carrier with overflow clamped to `ω` (`grade`) and the grade structural operations `dup` / `drop`.
* The effect layer: effect-graded returners `F^ε`, the sealed effect-row carrier, and the `perform` / `handle` terms (A3.2 `+effects`, `effect`).
* The host seam (`effect::host`) owns the canonical `Exec` / `Fs` / `Proc` / `Env` signature builders and operation/field constants beside `HostOp` / `HostHandler`.
  Surface lowering and the native runtime consume this alloc-only authority without depending on each other.
* The control layer: first-class reified stacks `Stk(B, C)`, `resume`, and delimited `reset` / `shift`, with the two-zone context `Γ; Σ` (A3.3 `+control`, `stack`, `ctx`).
* Subtyping — reflexive, but not transitive once `Unknown` participates (`subtype`).
* The two spec-grounded A2 extensions: A2.1 integer literals and A2.2 holes with the `Unknown` type.
* The total semantic marking layer (`mark`): per-node dual-type + mark + dirty-bit decoration, the Pantograph `{t}_{T1/T2}` boundary, the reconciled mark taxonomy, and the unchanged-type interner — oracle-tested against the checker.
* The conformance suite exercising checker ≡ machine agreement, totality, and the grade laws.
* Evaluation lives in `gandr-core-sequent`'s L machine, the sole evaluation driver; the frozen-core CEK evaluator is retired and removed, with the differential suite comparing against frozen outcome snapshots rather than a live second machine.
* The live value/data carriers used by lowering: numeric and string atoms, sums, products, lists, records, thunks/functions, native builtins, and the prelude environment.
* Identity rung 1 (ADR-76): `Path`, `here`, and full Martin-Löf `walk` with explicit motives and definitional walk-β, under the without-K discipline.
* The per-run type interner and reflexive-subtyping pointer fast path.
* The **kernel bridge** (`kernel_bridge`, stage B2.3): the elaborator-side, total, iterative lowering from the checked core CBPV forms into the minimal certified kernel's closed S1 vocabulary (`gandr-kernel-core`).
  Out-of-S1 nodes are rejected structurally with a precise `BridgeRejection`; `Annot`/`dup`/`drop` are erased (C4); names resolve to de Bruijn indices or cross-declaration `Value::Constant` admission indices; a computation definition enters the single-polarity kernel as a thunk (`U C`, B2.1 decision 3).
  This crate now depends on `gandr-kernel-core` — the permitted direction (the section-2 TCB wall forbids the reverse).
  The kernel re-derives every obligation (K2); the bridge is untrusted.
* The **A2.3 incremental checkpoint/validated-resume engine** is built, and it is not built here: it lives in `gandr-core-incrementality`, which depends on this crate and drives `machine` over this crate's `syntax`, `types`, and `ctx` vocabulary.
  This crate carried it — as `checkpoint`, `footprint`, and `region` — until the extraction of 2026-08-09, which merged it with the near-identical second copy that had grown in `gandr-surface-engine`.

Tests are green (the conformance and marking suites run under `mise run cargo:nextest`).

Not yet built: the linear context `Σ` is the committed shape but **vacuous in v0** (no obligation source; the discipline is unit-tested over `Sigma` directly); its obligation sources (sessions, sharing, worlds) are deferred `+feature`s.
Process-soup dynamics (`fork` / `acquire` / `migrate` and async signals), unions / intersections, polymorphism, and the row-polymorphic open tail `ρ` are not yet built.
L1 realization and promotion belong to `gandr-sequent`; they are not missing frozen-core forms.

## Module hierarchy, audited 2026-08-08

The crate declares twenty unconditional public modules at the root, one feature-gated public module (`strategies`), and one test-only private module (`conformance`).
The audit was taken on 2026-08-08, when the count was twenty-four; the three incremental modules of `core-checker-layout-01` have since left the crate.
Exactly one of them has a child: `effect::host`, moved there on 2026-07-31 because `EXEC` and `FS_READ` carry no meaning without an informative module path.

The audit below is layout only.
Each finding states its consumer count, because that number — not the shape argument — is what decides whether a reorganization is cheap.
Counts are use sites of the module path outside `gandr-core-checker`, cross-checked against entity-level dependents.
`gandr-kxv` is the audit's tracker item and hosts the workspace-wide spot-check; `gandr-kxv.1` is its owner-decision queue and holds the reorganizations that need a ruling.
None of the findings below are executed: every one of them changes a path or a name that other code and other documents already cite.

### core-checker-layout-01

**The A2.3 incremental trio is flat, and `region`'s item names are the evidence it should not be.** _Dissolved rather than executed, 2026-08-09._

The finding asked whether `checkpoint`, `footprint`, and `region` should sit under a shared parent module, and priced the move on their having zero consumers outside this crate.
The owner's ruling on the duplicate-engine question took a different cut: the three modules left the crate entirely for `gandr-core-incrementality`, so there is no in-crate move left to make and the crate-root flatness the finding objected to is no longer this crate's to answer for.
The naming half of the argument travelled with them — `region::Item` and `region::Program` now read against a crate whose name supplies the missing word.

The residual the finding left open — whether `intern` and `mark` belong with the incremental layer — did not travel, and is now a cross-crate question rather than a module-nesting one: `mark` has twelve use sites in two other crates, so it does not move at the price this finding assumed.

### core-checker-layout-02

**`machine::Outcome` collides with the crate's own `outcome` module.**

The root `outcome` module carries `Eval`, `Blame`, `StuckReason`, and `STEP_BUDGET`, with thirty-four use sites in four other crates.
`machine::Outcome` is an unrelated enum one path segment away, with three dependents — `run`, `run_report`, `run_to_failure` — all inside `machine.rs` and none outside the crate.

Two different things named "outcome" is the reference discipline's own failure at code scale: the shorter name reads as precise and resolves to the wrong thing.
Renaming `machine::Outcome` costs three in-crate edits and zero consumer edits.

### core-checker-layout-03

**`mark::Boundary` collides with the crate's own `boundary` module.**

`boundary` is the semantic-wrapper module the project-local Dylint primitive wall requires, and the same module name carries the same role in `core-sequent`, `surface-engine`, `theory-computads`, and `theory-levitation`; in this crate it has forty-eight use sites in four other crates.
`mark::Boundary` is the Pantograph `{t}_{T1/T2}` mismatch pair, which is unrelated to that role.
It has five references, all inside `mark.rs`, and none outside the crate, so renaming it costs five in-crate edits.

Recorded so a later reader does not mistake tooling silence for absence of use: entity-level impact analysis reports _no_ dependents for this struct, under-reporting the five same-file constructions grep finds.

### core-checker-layout-04

**`syntax::Side` is the crate's only forced import alias, and qualification alone cannot remove it.**

`kernel_bridge.rs` imports `gandr_kernel_core::Side as KernelSide` and `crate::syntax::Side as CoreSide` because both names are in scope in one file.
`syntax::Side` has twenty-four dependents across eleven files in four crates — `core-checker`, `core-sequent`, `surface-engine`, and surface-engine's integration tests.

The first remedy the Rust workflow prescribes — spell the longer path instead of importing the item — is unavailable here.
That file imports thirteen further `gandr_kernel_core` items and four further `crate::syntax` items, so spelling `Side` long is exactly the inconsistent qualification the lint wall denies.
What remains is a rename on one side or accepting the alias, and both are rulings rather than cleanups.

### core-checker-layout-05

**The remaining twenty-one root modules are correctly flat, and grouping them would be a regression.**

`syntax`, `types`, `ctx`, `grade`, `effect`, `subtype`, `subst`, `identity`, `stack`, `nominal`, `checker`, `machine`, `mark`, `control`, `error`, `outcome`, `prim`, `intern`, `kernel_bridge`, `boundary`, and `strategies` each name a domain that reads unambiguously at the crate root.
Together they carry 561 use sites in five other crates: `syntax` 159, `types` 99, `effect` 75, `boundary` 48, `grade` 37, `outcome` 34, `prim` 20, `checker` 14, `ctx` 13, `machine` 13, `kernel_bridge` 12, `mark` 12, `error` 11, `control` 10, `strategies` 3, and `nominal` 1.
A domain parent over any of them would move every one of those sites and buy no legibility the module name does not already carry.

`checker`, `machine`, and `mark` are the specific case worth stating, because they look like the most natural grouping and are the worst candidate: they are the crate's three independent realizations of one type system, and `lib.rs` leads with that distinction.
A shared parent would bury exactly what a reader needs first.
