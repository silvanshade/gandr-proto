# Workflow: Rust coding conventions

> Read when: writing or reviewing Rust in `crates/`.
> The `Cargo.toml` `[workspace.lints]` wall and `mise run cargo:dylint` are the enforcement; this file is the rationale and the parts those linters cannot check.
> Test obligations for the blocks defined here: [mutation-adequacy.md](mutation-adequacy.md).

## The conventions

* **Keep crates boring and explicit.** One job per crate, a flat data path (parser → CST → lowering → core IR → checker/machine).
  Prefer a clean cutover over a compatibility shim; no aliases or dead paths unless an accepted ADR requires them.
  General/reusable machinery gets its own crate (precedents: `gandr-theory-orders`, `gandr-theory-nominal-automata`) — a design pass owes the crate/module-boundary judgement, not only the edits (modularity-first, `.agents/core/core/PRINCIPLES.md` §"Working posture").
* **Crate naming follows the category schema; consult the rename table before minting any crate (owner rule, 2026-07-21).** Directory = `<category>-<name>`, package = `gandr-<directory>`.
  Categories as built: `core-*` (the frozen CBPV core: checker + sequent IL), `kernel-*` (the certified TCB and its substrate), `theory-*` (reusable metatheory machinery), `surface-*` (language surface: syntax, grammar, parsing, lowering/engine, corpus, render, editor faces), `runtime-*` (host effects, codecs, FFI), `storage-*` (the CAS tier), `workflow-*` (repo tooling and gates).
  The wyrd→reboot rename table in `docs/research/crate-port-map.md` §"Naming authority" is the per-crate authority: ports and new crates consult it FIRST; a crate not listed derives its name from the schema and adds a table row in the same change.
  Naming principles the table encodes: prefer intent over mechanism (`surface-engine`, not `-pipeline`; `theory-levitation`, not `-desc`); avoid collision with overloaded terms (`runtime-codecs`, not `-data`); the remote/wire face of a local surface takes a `-remote` suffix beside its local sibling (`surface-render-remote` beside the future `surface-render`).
* **Choose architecture with performance in view.** Before concrete implementation, enumerate the plausible architectures and compare their runtime and memory profiles alongside correctness, extensibility, maintainability, and implementation complexity.
  Prefer the best-performing design that does not materially sacrifice those other qualities.
  Do not default to the most direct design when improving it later would require an expensive change to ownership, representation, interfaces, persistence, or other foundations; equally, do not micro-optimize local code without evidence.
* **Consult the implementation-model record before design decisions (owner directive, 2026-07-20).** [`docs/research/impl-models-deep-read.md`](../research/impl-models-deep-read.md) — the source-grounded Idris 2 / Lean 4 / Agda / smalltt internals map, the performance program's primary record — is a mandatory consult before any concrete decision about term/value representation, sharing or interning, evaluation/machine shape, conversion/def-eq, erasure, serialization, or incrementality.
  Cite the sections consulted in the decision record (design doc, bead comment, or ADR); a decision in these areas that does not engage the record is a review-blocking finding.
  Standing per-node reminders live in the tracker notes of the pending design beads.
* **Design memory behavior deliberately.** Prefer zero-copy data flow and borrowed views where they do not impose disproportionate lifetime or API complexity.
  When allocation is necessary, minimize allocation count and copying through appropriate capacity planning, arenas, interning, buffer or object reuse, and workload-justified caching.
  Account for invalidation, retained memory, and synchronization in the cost of a cache.
* **Prefer incremental, resumable, streaming-compatible execution.** Where the problem permits, structure work as bounded steps over explicit state rather than as one monolithic call.
  Make progress checkpointable so interrupted work can resume without replaying completed steps, and consume or produce streams without retaining the complete input or result in memory.
* **Prefer first-order representations.** Represent continuations, callbacks, control states, and work items as explicit data plus an interpreter or state machine; defunctionalize higher-order machinery where practical.
  First-order data keeps execution inspectable, serializable, persistable, cacheable, testable, and resumable.
  Use opaque closures or dynamic dispatch only when those properties do not matter and the higher-order form materially improves the design.
* **Single-field structs are transparent.** Every named or tuple struct with exactly one field must carry `#[repr(transparent)]`.
  An exception requires a concrete layout, ABI, or soundness reason documented in the item's `# Contract`; convenience or omission is not an exception.
* **Crate-defined signatures preserve semantic information.** A function or method defined by a workspace crate must not accept or return a bare primitive value (`bool`, `char`, numeric primitives, or `str`; see the [Rust primitive overview](https://doc.rust-lang.org/rust-by-example/primitives.html)), whether directly or beneath references, pointers, tuples, arrays, slices, or configured generic containers before reaching a nominal type boundary.
  This applies regardless of visibility to free, const, async, and extern functions; inherent methods; local-trait declarations and defaults; and local-trait implementations.
  The sole exception is a method implementing a trait defined in an external crate whose required signature contains primitive types.
  Introduce a nominal domain wrapper rather than a type alias, give each single-field wrapper `#[repr(transparent)]`, and implement the utility traits needed for effective use.
  Wrappers prevent semantically distinct values from becoming interchangeable and preserve meaning for humans and agents.
* **Lifetimes name semantic roles.** Use relationship-bearing names such as `'source`, `'arena`, or `'world`; never alphabetical or positional names such as `'a` or `'b`.
  Carry the same name through related struct, impl, trait, and associated-type signatures so the borrowing relationship remains traceable.
* **Partial functions are banned.** Never index or slice; use `.get(..)`, `split_first`, iteration.
  Never `unwrap`/`expect` on a fallible value in shipping code — return a typed error.
  `unwrap_used`, `expect_used`, `unwrap_in_result`, `get_unwrap`, `panic!`, `unreachable!`, `todo!`, `unimplemented!`, `exit`, integer `/`, and bare overflowing arithmetic are all lint-denied.
  The keystone: the checker and the machine are **total** — structured errors (`TypeError`, `Outcome::Error`, `LowerError`), never divergence.
* **The `?` operator is a statement, not a subexpression.** Never bury `?` inside a larger expression (`f(x?)`, `Some(v?.0)`, `break g()?`): bind the fallible step with `let` first, then use the bound name (`question_mark_in_expression` is denied).
  When the introduced binding collides with an existing name, prefer shadowing (`let node = node(parent)?;`) over inventing a near-duplicate name — the shadow keeps each binding's provenance in source order.
* **No unbounded recursion in the interpreter (ADR-47).** Rust has no guaranteed TCO, so recursion whose depth scales with unbounded input (list length, term/AST depth, environment size) is a latent stack overflow on real data.
  Across the interpreter use an explicit worklist / heap frame-stack / iterative loop; new input-scaled recursion is a review-blocking finding.
  Recursion bounded by a fixed small static structure is fine.
  The Agda metatheory is the specification oracle, not an implementation blueprint — the Rust is its _iterative shadow_; a divergence in shape is expected, only a divergence in result is a bug (the differentials compare answers, not call graphs).
* **Arithmetic is checked, never bare.** `saturating_*` for monotone counters/depths, `checked_*` where overflow must surface (the grade semiring clamps finite overflow to `ω`), `wrapping_*` only for hashing.
  `arithmetic_side_effects` is denied workspace-wide.
* **Test/bench code may relax the wall; production never does.** The standard test-allow set (`arithmetic_side_effects`, `expect_used`, `indexing_slicing`, `panic`, `unwrap_used`, plus what clippy requires) via a single crate-level `#![cfg_attr(test, allow(...), reason = "..."))]`.
  The relaxation is the FIRST thing added when a crate or a test target is created (owner directive, 2026-07-23): the lib root carries it from day one, and each `tests/*.rs` integration file carries it at its own root — crate-level attributes do not cross compilation roots, so the lib's block never reaches integration targets.
  A crate that gains its first `#[cfg(test)]` module or `tests/` file later adds the relaxation in the same change, before writing the tests.
* **Panic policy.** Production paths return typed errors.
  A panic is acceptable only as a `debug_assert!` of an internal invariant or in test/bench code.
  Every reachable panic is either routed through a structured error variant or documented in the item's `# Contract`.
* **Do not silence a diagnostic to pass a gate.** Narrow relaxations are `#[expect(lint, reason = "...")]`, scoped tightly (`allow_attributes` is denied).
  Blanket test relaxation uses the crate-level `cfg_attr` form above (`allow`, not `expect` — the lint does not fire through `cfg_attr`, and a multi-lint `expect` raises `unfulfilled_lint_expectations`).
  A `harness = false` bench takes a file-level `#![expect(...)]`.
  Fix the source or file a bead — never bury drift in a `// TODO` (`todo` is lint-denied).
* **Diagnostics through aifix** ([scripting.md](scripting.md)); `mise run cargo:clippy` and `mise run cargo:dylint` are pass/fail gates, not diagnostic-enumeration interfaces.
* **Project-local Dylint rules.** `mise run cargo:dylint` loads the immutable Trail of Bits v6.0.1 source pin plus `gandr-workflow-dylint`.
  The upstream inventory is exhaustive outside `Experimental` and `Testing`: eight `General` lints, all nine lints exported by `Supplementary`, and twelve `Restriction` lints.
  Five isolated driver invocations cover project-local rules, ordinary upstream rules, `non_local_effect_before_unhandled_error`, `crate_wide_allow`, and warning-level `register_lints_warn`; `DYLINT_RUSTFLAGS="-D warnings"` makes every late-registered warning fatal.
  `crate_wide_allow` deliberately omits test targets: the standard crate-level `cfg_attr(test, allow(...))` wall relaxation above is the one exception.
  `gandr-workflow-dylint` requires `#[repr(transparent)]` on every single-field struct and rejects project-defined function or method signatures that expose types from the [official primitive index](https://doc.rust-lang.org/std/primitive/index.html).
  Primitive detection follows aliases and non-nominal structural/generic containers; a semantically named transparent wrapper is the boundary, with explicit utility traits.
  The sole signature exception is a method implementing a trait defined in an external crate.
  Project-local rules also enforce source-grounded recursive `# Termination` contracts and reject false `input recursion: none` claims.
  `mise run cargo:dylint:recursion` puts that rule on the merge wall by running only the project-local driver over the Dylint-covered workspace targets; `gandr-workflow-gates` and the driver itself retain the same explicit exclusions as the strict lane.
  It denies every warning except the unrelated `primitive_signature` and `single_field_struct_needs_transparent_repr` debt tracked in `gandr-vp8`; the strict `cargo:dylint:local` and full `cargo:dylint` tasks retain `-D warnings` and therefore do not claim those rules are clean.
  The strict and recursion-relaxed passes use distinct Cargo target directories because `DYLINT_RUSTFLAGS` does not distinguish Cargo fingerprints; sharing artifacts could make one policy incorrectly reuse the other's result.
  **Known blind spot (2026-07-20):** the recursion rule sees only crate-local source-level call edges — derived-trait recursion (`Clone`/`PartialEq`/`Hash`/`Debug` on a recursive owned type routes through non-local std generics such as `Box<T>: Clone`, so no crate-local edge ever exists) and compiler-generated drop glue (no HIR function at all) are structurally invisible to it.
  Closure-mediated recursion is likewise invisible (2026-07-23, gandr-aaq): a self-call wrapped in a closure argument (`walk_list(xs, \&mut |x| f(x))`) produces no extracted edge, so a recursive walker shaped that way passes unmeasured while the identical direct recursion is held to the contract — write the explicit work stack regardless, treating the lint as a floor, not a proof.
  Destruction/duplication totality on recursive owned types is therefore **not gate-proven**; the mitigations are flat/arena representations or manual worklist impls, and a complementary type-plane lint is tracked as `gandr-cfo`.
  `non_local_effect_before_unhandled_error` remains a separate driver invocation after a historical upstream panic on a core lib-test target; it currently shares the standard Dylint package scope and `--all-targets` selection, and must be re-split per-core if that defect recurs.

### Dylint adoption and residual ledger

The 2026-07-17 restoration re-enabled the Rust workspace and removed phased package allowlists from the canonical lint tasks, while retaining two explicit Dylint exclusions:

* build, documentation, nextest, careful, coverage, Miri, live graph-gate discovery, and Clippy address the complete enabled workspace; Dylint addresses the complete Dylint-covered subset;
* `cargo:clippy` checks every enabled workspace target with `features=full`, including the in-workspace `gandr-workflow-dylint` driver;
* every workspace-wide Dylint pass uses the same package scope, excluding `gandr-workflow-gates` and `gandr-workflow-dylint`; the pinned non-local-effect lint remains a separate driver invocation but uses the same package and target selection;
* `gandr-workflow-gates` remains excluded while its crate-local lint-wall overrides are triaged by `gandr-0ze`; the driver remains excluded pending workspace lint-wall adoption under `gandr-3yh`;
* Clippy and Dylint run as the local `mise run cargo:clippy` and `mise run cargo:dylint` gates over those respective scopes; the hosted CI that ran them as separate dependency-free jobs — and the `ci_contracts` test locking their job independence and exact package scopes — is parked for the reboot, returning with the `.github/workflows/` surface (`gandr-kk7`; [ci.md](ci.md)).

New untracked package allowlists or exclusions are prohibited; the two tracked Dylint exclusions above remain visible until their owning beads close.

The rollout established these durable findings and actions:

1. **Semantic wrappers are API boundaries, not aliases.** Primitive and tuple APIs hid byte offsets, grammar slots, identities, generations, and proof positions.
   Use private `#[repr(transparent)]` newtypes, checked constructors, and exact external-trait conversions.
   Do not restore compatibility with generic `Into`, inherent primitive getters, or primitive `PartialEq`.
2. **Recursion claims require call-graph evidence.** Free functions and methods are checked by recursive SCC, including nested calls and generic arguments.
   Recursion over caller-owned input must become an explicit bounded worklist; documentation alone cannot relabel it as non-recursive.
3. **Mutation-before-error paths are architectural evidence.** Preserve the isolated upstream non-local-effect pass and investigate its `gandr-core-checker` lib-test panic; never disable the lint globally.
4. **Future rules should encode cross-module invariants.** Tracked follow-up candidates: checker/machine constructor parity, atomic force-state transitions, subprocess-boundary policy, nominal-id replay provenance, and semantic-wrapper escape prevention.

* **Dependencies.** Use the `find-best-rust-crates` skill before adding a nontrivial crate, but treat external implementations as design references rather than automatic dependencies.
  Machinery that is load-bearing for the core or certified-kernel boundary — including recursion/control runtimes, graph representations and algorithms, proof-state machinery, and semantic normalization — stays gandr-owned even when reimplementation costs more.
  Prefer `core`/`alloc`, a focused local crate, and existing workspace dependencies in that order; external convenience must not enlarge the trusted or bootstrapping-critical base.
  Existing external graph dependencies such as `petgraph` are migration debt rather than precedent: when their role becomes kernel-relevant, replace them with local representations and algorithms.
  For non-kernel dependencies that pass that boundary test, default `default-features = false` with an explicit feature list (exceptions where required runtime lives behind defaults: `proptest`, `libfuzzer-sys`).
* **Crates join the workspace; detached crates are prohibited.** Every new crate is a member of the root workspace from day one — no crate-local `[workspace]` tables, no out-of-workspace drivers with their own lockfiles, toolchain pins, or lint posture.
  Detached crates escape the `[workspace.lints]` wall and silently accumulate lint debt that returns as a remediation project (the out-of-workspace Dylint driver is the case history: it arrived with an entire lint surface to triage).
  A crate that cannot satisfy the wall yet still joins the workspace and instead carries a crate-local override block referencing a triage bead, so the debt stays visible and scoped.
* **Dependencies live once, at the workspace root, and are inherited everywhere.** Every external dependency — normal, build, or dev — is declared in the root `Cargo.toml` under `[workspace.dependencies]`, and member crates reference it with `{ workspace = true }`.
  Each crate version and feature set is stated exactly once and the workspace stays fully deduped; a crate-local `version =` re-declaration is a review finding.
  When one crate needs a feature no other consumer needs, enable it on the workspace declaration rather than forking the pin.
  Every `[workspace.dependencies]` entry carries, directly above it, one brief comment saying what the dependency does, followed by a `# consumers:` block: a dashed comment list, one consuming crate per line, with dev-only consumers marked `(dev)`:

  ```toml
  # BLAKE3 hashing for content and manifest digests.
  # consumers:
  # - gandr-workflow-gates
  [workspace.dependencies.blake3]
  version = "1.8.5"
  default-features = false
  features = ["std"]
  ```

  Keep the comment to what the crate _is_ — a phrase, not a paragraph: no architectural specifics, no design rationale, no usage narrative (those belong to the consuming crate's own docs).
  When a crate starts or stops using a dependency, its name joins or leaves the `# consumers:` block in the same change; when the last consumer leaves, the workspace declaration is removed in the same change.

## Documentation by contract

Every nontrivial item (public or private — `missing_docs_in_private_items` is denied) carries a one-line summary plus a `# Contract` rustdoc block; fallible functions also carry `# Errors`; nontrivial items in new or substantially-refactored code also carry `# Adequacy` (ADR-71).

`mise run cargo:doc-check` is the merge-wall gate for this section: it runs `cargo doc --workspace --features=full --no-deps --document-private-items` on the pinned nightly with `RUSTDOCFLAGS="-D warnings"`, so an intra-doc link that does not resolve — or a redundant explicit link target — fails the wall rather than accreting as silent rustdoc debt.
An in-scope item links by its bare name (`` [`Value`] ``); a cross-module item uses a reference-style link — the short label `` [`TermArena`] `` in the prose with its path collected once as a definition at the end of the doc block (`` [`TermArena`]: crate::TermArena ``) — rather than repeating the full crate-root re-export or module path inline at every occurrence.

```rust
/// Convert a zero-based protocol coordinate into a one-based coordinate.
///
/// # Contract
/// - requires: `value` is a zero-based coordinate from an external protocol.
/// - ensures: returns `value + 1` when representable.
/// - provides: a one-based coordinate for the display layer.
/// - fails: returns `None` on `u32::MAX` overflow rather than panicking.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 only — the `+ 1` and the overflow guard are separated
///   solely by the boundary pair `u32::MAX` / `u32::MAX - 1` plus one ordinary
///   value asserted exactly.
/// - witness: `position::tests::one_based_ordinary_value_is_exact`
/// - witness: `position::tests::one_based_boundary_at_u32_max`
```

* Clauses in fixed order, as `-` bullets: `requires` (caller preconditions), `ensures` (postconditions on success), `provides` (what the item yields), `fails` (failure modes and how they surface), `panics`, optionally `intension` (last); omit a clause only when it does not apply.
  Write `- panics: none.` explicitly — the absence of a panic is a contract.
  An `unsafe` item adds `- unsafe invariants:`, the rustdoc `# Safety` section, and `// SAFETY:` comments (`undocumented_unsafe_blocks` is denied).
* `- intension:` states properties of _how_ the computation proceeds (enumeration/tie-break order, traversal strategy, cost, determinism, trace shape) — only those the item **promises**, each observable through a **declared semantic projection** the API exposes.
  Intensional tests assert only declared projections; extensional clauses never reference intensional observations (ADR-71 D4, the calf noninterference discipline).
* `# Errors` (clippy pedantic) coexists with `# Contract`: `- fails:` is the design-level statement, `# Errors` the per-variant enumeration.
* `# Termination` is mandatory on every directly or mutually recursive function or method.
  Use the fixed grammar below; each field needs a concrete explanation, not an assertion that termination is obvious:

  ```rust
  /// # Termination
  /// - reason: why recursion is the appropriate control structure.
  /// - measure: the quantity that strictly decreases on every recursive edge.
  /// - boundedness: where the finite or well-founded bound comes from.
  /// - input recursion: none.
  ```

  `- input recursion: none.` is required everywhere except the model recursive checker in `gandr-core-checker::checker::Rec`, if that implementation is still recursive.
  That checker may instead name structural descent through the finite checked term, because serving as the direct recursive reference model is its purpose; the defunctionalized machine remains the adversarial-depth path.
  Tail-call position does not remove this obligation because Rust does not guarantee tail-call optimization; a genuinely iterative implementation is not recursive and needs no termination section.
* `# Adequacy`: `- hypothesis:` — a falsifiable claim naming which adequacy-ladder rung kills each decision surface's mutants, plus the distinguishing inputs and observations for the pointwise residue — then one `- witness:` bullet per witnessing test (crate-qualified when it lives in another crate).
  Fixed grammar — the adequacy gates machine-extract it.
* "Nontrivial" = has a precondition a caller can violate, can fail, or has a non-obvious postcondition.
  Thin builders, trivial accessors, and data constants get the one-line summary only — do not manufacture blocks for them.
