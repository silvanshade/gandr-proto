# Workflow: Rust coding conventions

> Read when: writing or reviewing Rust in `crates/`.
> The `Cargo.toml` `[workspace.lints]` wall and `mise run cargo:dylint` are the enforcement; this file is the rationale and the parts those linters cannot check.
> Test obligations for the blocks defined here: [mutation-adequacy.md](mutation-adequacy.md).
> **Standing rule, whatever the task:** before recording that something does not apply, is not needed, or cannot be done, read [review.md](review.md) §"Declining is a claim too" and §"Refutations bind only with owner sign-off" — a refutation binds only with the owner's sign-off.

## The conventions

### Design and representation

- **Keep crates boring and explicit.** One job per crate, a flat data path (parser → CST → lowering → core IR → checker/machine).
  Prefer a clean cutover over a compatibility shim; no aliases or dead paths unless an accepted ADR requires them.
  General/reusable machinery gets its own crate (precedents: `gandr-theory-orders`, `gandr-theory-nominal-automata`) — a design pass owes the crate/module-boundary judgement, not only the edits (modularity-first, `AGENTS.md` §"Working posture").
- **Crate naming follows the category schema; consult the rename table before minting any crate (owner rule, 2026-07-21).** Directory = `<category>-<name>`, package = `gandr-<directory>`.
  Categories as built: `core-*` (the frozen CBPV core — checker + sequent IL — and the incremental typing that rides directly on it), `kernel-*` (the certified TCB and its substrate), `theory-*` (reusable metatheory machinery), `surface-*` (language surface: syntax, grammar, parsing, lowering/engine, corpus, render, editor faces), `runtime-*` (host effects, codecs, FFI), `storage-*` (the CAS tier), `workflow-*` (repo tooling and gates).
  **The workspace is the naming authority**: the crates that exist are the record of what the schema has already decided, so a port or a new crate reads `crates/` first and derives its name from the schema and the principles below.
  The wyrd→reboot rename table that used to hold that authority left with the research corpus, and it was four rows behind the tree it governed when it went — a lookup table describing the workspace is a second source of truth that decays, where the workspace cannot.
  Naming principles that table encoded: prefer intent over mechanism (`surface-engine`, not `-pipeline`; `theory-levitation`, not `-desc`); avoid collision with overloaded terms (`runtime-codecs`, not `-data`); the remote/wire face of a local surface takes a `-remote` suffix beside its local sibling (`surface-render-remote` beside the future `surface-render`).
- **Choose architecture with performance in view.** Before concrete implementation, enumerate the plausible architectures and compare their runtime and memory profiles alongside correctness, extensibility, maintainability, and implementation complexity.
  Prefer the best-performing design that does not materially sacrifice those other qualities.
  Do not default to the most direct design when improving it later would require an expensive change to ownership, representation, interfaces, persistence, or other foundations; equally, do not micro-optimize local code without evidence.
- **Engage the implementation-model evidence before design decisions (owner directive, 2026-07-20).** Any concrete decision about term/value representation, sharing or interning, evaluation/machine shape, conversion/def-eq, erasure, serialization, or incrementality is made against how the working systems in this space actually do it — Idris 2, Lean 4, Agda, and smalltt — rather than from first principles alone.
  The source-grounded internals map that used to serve that consult left with the research corpus and is held in the maintainer's private research workspace; the directive survives it, because the obligation is to the evidence rather than to the document.
  So a decision in these areas cites what it consulted — the systems' own sources at an identified revision, or a dispatched brief carrying the map's findings — in the decision record (design doc, bead comment, or ADR).
  A decision in these areas that engages nothing is a review-blocking finding, and that has not changed.
  Standing per-node reminders live in the tracker notes of the pending design beads.
- **Design memory behavior deliberately.** Prefer zero-copy data flow and borrowed views where they do not impose disproportionate lifetime or API complexity.
  When allocation is necessary, minimize allocation count and copying through appropriate capacity planning, arenas, interning, buffer or object reuse, and workload-justified caching.
  Account for invalidation, retained memory, and synchronization in the cost of a cache.
- **Prefer incremental, resumable, streaming-compatible execution.** Where the problem permits, structure work as bounded steps over explicit state rather than as one monolithic call.
  Make progress checkpointable so interrupted work can resume without replaying completed steps, and consume or produce streams without retaining the complete input or result in memory.
- **Prefer first-order representations.** Represent continuations, callbacks, control states, and work items as explicit data plus an interpreter or state machine; defunctionalize higher-order machinery where practical.
  First-order data keeps execution inspectable, serializable, persistable, cacheable, testable, and resumable.
  Use opaque closures or dynamic dispatch only when those properties do not matter and the higher-order form materially improves the design.

### Types and signatures

- **Single-field structs are transparent.** Every named or tuple struct with exactly one field must carry `#[repr(transparent)]`.
  An exception requires a concrete layout, ABI, or soundness reason documented in the item's `# Contract`; convenience or omission is not an exception.
- **Crate-defined signatures preserve semantic information.** A function or method defined by a workspace crate must not accept or return a bare primitive value (`bool`, `char`, numeric primitives, or `str`; see the [Rust primitive overview](https://doc.rust-lang.org/rust-by-example/primitives.html)), whether directly or beneath references, pointers, tuples, arrays, slices, or configured generic containers before reaching a nominal type boundary.
  This applies regardless of visibility to free, const, async, and extern functions; inherent methods; local-trait declarations and defaults; and local-trait implementations.
  The sole exception is a method implementing a trait defined in an external crate whose required signature contains primitive types.
  Introduce a nominal domain wrapper rather than a type alias, give each single-field wrapper `#[repr(transparent)]`, and implement the utility traits needed for effective use.
  Wrappers prevent semantically distinct values from becoming interchangeable and preserve meaning for humans and agents.
- **Lifetimes name semantic roles.** Use relationship-bearing names such as `'source`, `'arena`, or `'world`; never alphabetical or positional names such as `'a` or `'b`.
  Carry the same name through related struct, impl, trait, and associated-type signatures so the borrowing relationship remains traceable.

### Totality and style

- **Partial functions are banned.** Never index or slice; use `.get(..)`, `split_first`, iteration.
  Never `unwrap`/`expect` on a fallible value in shipping code — return a typed error.
  `unwrap_used`, `expect_used`, `unwrap_in_result`, `get_unwrap`, `panic!`, `unreachable!`, `todo!`, `unimplemented!`, `exit`, integer `/`, and bare overflowing arithmetic are all lint-denied.
  The keystone: the checker and the machine are **total** — structured errors (`TypeError`, `Outcome::Error`, `LowerError`), never divergence.
- **The `?` operator is a statement, not a subexpression.** Never bury `?` inside a larger expression (`f(x?)`, `Some(v?.0)`, `break g()?`): bind the fallible step with `let` first, then use the bound name (`question_mark_in_expression` is denied).
  When the introduced binding collides with an existing name, prefer shadowing (`let node = node(parent)?;`) over inventing a near-duplicate name — the shadow keeps each binding's provenance in source order.
- **Shadow rather than suffix.** When a binding is never used again after a re-binding, reuse the same name instead of minting a suffixed variant (`jobs_value`, `entry_text`, `stripped`).
  Trivial projections — unwrapping a wrapper field (`.0`, `.token`), dereferencing, `.trim()` — never justify a new name: they produce the same value under the same concept (`let section = section.trim();`).
  Reserve a distinct name for a genuine transformation (parsing, decoding, conversion into a different domain type) or for a value that must stay live beside its successor.
  Fewer names means fewer things a reader must track.
- **Qualification is consistent, and the qualifier must earn its place (`inconsistent_qualification` is denied).** A file refers to a given module's items with one qualification throughout: importing some of a module's items while spelling others with a longer path is the denied inconsistency.
  External crates take full paths by default (`std::fs::read_to_string`, `syn::Item`, `serde_json::Value`); do not import the module itself (`use std::fs;`), and do not import external types under light-to-moderate use.
  A short prefix outweighs the heavy-use convenience: `syn::` and `serde_json::` stay fully qualified even where a type is used heavily.
  Repetition in the path justifies importing the module as the qualifier: `yaml_rust2::yaml::Hash` says "yaml" twice, so `use yaml_rust2::yaml;` and write `yaml::Hash`.
  Local project crates may import types more liberally where the type name is informative (`use gandr_kernel_core::TermArena;`).
  Where an item's own name is uninformative, qualify through an informative module path instead (`effect::host::EXEC`, not `sig::EXEC`); if no informative path exists, reorganize the module hierarchy rather than minting an alias.
- **No `as` conversions.** Never cast with `as` (`as_conversions` is denied): it truncates, wraps, and changes signedness silently.
  Convert through the value's total API — `uN::from`/`uN::try_from` with the error consumed, or serialize the source width and take the bytes needed (`value.to_be_bytes()`, then pick the lanes).
- **No unbounded recursion in the interpreter.** Rust has no guaranteed TCO, so recursion whose depth scales with unbounded input (list length, term/AST depth, environment size) is a latent stack overflow on real data.
  Across the interpreter use an explicit worklist / heap frame-stack / iterative loop; new input-scaled recursion is a review-blocking finding.
  Recursion bounded by a fixed small static structure is fine.
  The Agda metatheory is the specification oracle, not an implementation blueprint — the Rust is its _iterative shadow_; a divergence in shape is expected, only a divergence in result is a bug (the differentials compare answers, not call graphs).
- **Arithmetic is checked, never bare.** `saturating_*` for monotone counters/depths, `checked_*` where overflow must surface (the grade semiring clamps finite overflow to `ω`), `wrapping_*` only for hashing.
  `arithmetic_side_effects` is denied workspace-wide.
- **Embedded syntax is written raw.** A string literal carrying actual surface syntax — or any multi-line embedded content: fixtures, expected renderings, corpus snippets — is a raw string (`r#"…"#` style) with real newlines, never an escaped-`\n` literal split across backslash continuations.
  Escaped snippets are unreadable and undiffable, and rustfmt's continuation reflow moves the backslash breaks so the literal's visual shape drifts from its content.
  `mise run cargo:embedded-syntax` scans changed Rust sources under `crates/` and fails on recognizable embedded syntax; existing baseline debt is handled incrementally, while `workflow-gates: allow-escaped-newline` is reserved for tests whose escape decoding or non-syntax payload is the subject.
  `crates/surface-engine/tests/desc_cells.rs` carries the worked form: the snippet reads exactly as the language writes it.

### Lints and enforcement

- **Test code lives under the same wall; the only sanctioned test relaxation is `clippy.toml` configuration.** The repo `clippy.toml` enables clippy's native `allow-*-in-tests` options (`dbg`, `expect`, `indexing-slicing`, `panic`, `print`, `unwrap`), which relax exactly those lints inside test code (`#[cfg(test)]` modules and `tests/` targets) without any source attribute.
  Attribute-based test relaxations — a crate-level `#![cfg_attr(test, allow(...))]` or a per-file `#![allow(...)]` test wall — are prohibited (owner directive, 2026-07-30, superseding the 2026-07-23 crate-level relaxation rule): they leak across items, creep in scope, and bury drift.
  Every lint without a clippy.toml in-tests option — notably `arithmetic_side_effects` — binds tests exactly as production: restructure the test or use checked/saturating arithmetic rather than suppressing.
- **Panic policy.** Production paths return typed errors.
  A panic is acceptable only as a `debug_assert!` of an internal invariant or in test/bench code.
  Every reachable panic is either routed through a structured error variant or documented in the item's `# Contract`.
- **Do not silence a diagnostic to pass a gate.** Narrow relaxations are `#[expect(lint, reason = "...")]`, scoped tightly (`allow_attributes` is denied).
  Test-code relaxation is `clippy.toml` configuration, never attributes; benches are not test code for these options, so a `harness = false` bench takes a file-level `#![expect(...)]`.
  Fix the source or file a bead — never bury drift in a `// TODO` (`todo` is lint-denied).
- **Diagnostics through aifix** ([scripting.md](scripting.md)); `mise run cargo:clippy` and `mise run cargo:dylint` are pass/fail gates, not diagnostic-enumeration interfaces.
- **Iterate lints per touched crate; the wall runs the workspace.** `mise run cargo:clippy <package>…` and `mise run cargo:dylint:local <package>…` accept trailing package names and lint only the named crates on the pinned nightly; bare invocations keep the workspace scope the merge wall composes.
  While writing or modifying a module, run the scoped form over exactly the affected crate or crates — agent briefs should say so — and rely on the wall's workspace sweep for the composed proof rather than re-running it per iteration.
- **Project-local Dylint rules.** `mise run cargo:dylint` loads the immutable Trail of Bits v6.0.1 source pin plus `gandr-workflow-dylint`.
  The upstream inventory is exhaustive outside `Experimental` and `Testing` apart from three purposely-ignored lints: seven `General` lints (`non_local_effect_before_unhandled_error` ignored — its "handled" analysis false-positives against `Result`-consuming forms the wall accepts), all nine lints exported by `Supplementary`, and ten `Restriction` lints (`misleading_variable_name` ignored — its name/usage matching is too coarse to be useful; `non_topologically_sorted_functions` ignored — its single caller-before-callee order conflicts with deliberate top-down layouts and is unsatisfiable for shared test fixture helpers).
  Four isolated driver invocations cover project-local rules, ordinary upstream rules, `crate_wide_allow`, and warning-level `register_lints_warn`; `DYLINT_RUSTFLAGS="-D warnings"` makes every late-registered warning fatal.
  `crate_wide_allow` covers every target kind, tests included: no crate-level `allow` is approved anywhere — the sanctioned test relaxation lives in `clippy.toml` configuration, which source attributes cannot express.
  `gandr-workflow-dylint` requires `#[repr(transparent)]` on every single-field struct and rejects project-defined function or method signatures that expose types from the [official primitive index](https://doc.rust-lang.org/std/primitive/index.html).
  Primitive detection follows aliases and non-nominal structural/generic containers; a semantically named transparent wrapper is the boundary, with explicit utility traits.
  The sole signature exception is a method implementing a trait defined in an external crate.
  This rule proves only that a nominal transparent boundary exists; it does not validate field visibility, conversion traits, documentation, or the rest of the workspace Clippy contract.
  After any `primitive_signature` remediation, run the package-scoped `cargo:clippy` (`mise run cargo:clippy <package>`) before removing that package from a strict-Dylint exclusion.
  This scoped check is a diagnostic prerequisite, never unit-completion evidence: after the source and strict-lane wiring land together, do not complete or leave the remediation unit until the full `mise run gate:merge` wall passes.
  Project-local rules also enforce source-grounded recursive `# Termination` contracts and reject false `input recursion: none` claims.
  `mise run cargo:dylint:local` puts those contracts on the merge wall by running the project-local driver at `-D warnings` over every covered workspace target.
  Every workspace member is covered, `gandr-workflow-gates` and the driver included: the 2026-07-30 lint-wall pass remediated the last crate-local override blocks and removed the lane exclusions in the same change (provenance: `gandr-0ze`, `gandr-3yh`).
  The temporary recursion-relaxed lane and its isolated target directory were removed after `gandr-vp8` remediated the primitive-boundary debt.
  **Known blind spot (2026-07-20):** the recursion rule sees only crate-local source-level call edges — derived-trait recursion (`Clone`/`PartialEq`/`Hash`/`Debug` on a recursive owned type routes through non-local std generics such as `Box<T>: Clone`, so no crate-local edge ever exists) and compiler-generated drop glue (no HIR function at all) are structurally invisible to it.
  Closure-mediated recursion is likewise invisible (2026-07-23, gandr-aaq): a self-call wrapped in a closure argument (`walk_list(xs, \&mut |x| f(x))`) produces no extracted edge, so a recursive walker shaped that way passes unmeasured while the identical direct recursion is held to the contract — write the explicit work stack regardless, treating the lint as a floor, not a proof.
  Destruction/duplication totality on recursive owned types is therefore **not gate-proven**; the mitigations are flat/arena representations or manual worklist impls, and a complementary type-plane lint is tracked as `gandr-cfo`.
  `non_local_effect_before_unhandled_error` is purposely ignored (see above); its inert `cfg_attr(dylint_lib = ...)` allow sites remain from when the lint was active, as do those of `non_topologically_sorted_functions`.

### Toolchain upgrades

The nightly pin (`RUSTUP_TOOLCHAIN_NIGHTLY` in `mise.toml`, materialized into `crates/workflow-dylint/rust-toolchain` by `toolchain:materialize`) moves only deliberately: the upstream Dylint example lints track a specific rustc_private API and lag current nightlies by roughly a release cycle, so the pin targets the newest nightly the upstream Dylint rev supports, not the newest nightly available.
A bump is its own reviewed change: bump the pin, materialize, run the full merge wall, and clear the residuals list below (closing or refreshing each entry against the new toolchain).

#### Residuals

- **`std_instead_of_core` lane allowance (2026-07-30, pin `nightly-2026-05-28`).** The 2026-05-28 clippy flags seven `std::` import sites in `gandr-workflow-gates` that the 2026-07-07 clippy accepted (the `core::io` stabilization window shifted between the two; the runtime-effects cross-toolchain pair pattern covers the same class for `core::io::Error`).
  Rather than seven cross-toolchain allow pairs, every nightly clippy invocation in the workflow carries `-A clippy::std_instead_of_core` on the command line: `cargo:clippy` and the `cargo:dylint*` lanes (via `DYLINT_RUSTFLAGS`).
  **Revisit on the next major toolchain bump:** if the newer clippy agrees with the manifest wall on those paths — or the sites are rewritten to a form both toolchains accept — remove the lane flag.
  The residual exists because the pin is held at `nightly-2026-05-28` by the upstream Dylint examples (their `clippy_utils` pin tracks that nightly); it unblocks when Dylint upstream moves past the window.

### Dylint adoption and residual ledger

The 2026-07-17 restoration re-enabled the Rust workspace and removed phased package allowlists from the canonical lint tasks, while retaining two explicit Dylint exclusions:

- build, documentation, nextest, careful, coverage, Miri, live graph-gate discovery, and Clippy address the complete enabled workspace; Dylint addresses the complete Dylint-covered subset;
- `cargo:clippy` checks every enabled workspace target with `features=full`, including the in-workspace `gandr-workflow-dylint` driver;
- every workspace-wide Dylint pass uses the same package scope — the complete enabled workspace with no exclusions (the `gandr-workflow-gates` and `gandr-workflow-dylint` exclusions were removed by the 2026-07-30 lint-wall pass, under `gandr-0ze` and `gandr-3yh`); the pinned non-local-effect lint remains a separate driver invocation but uses the same package and target selection;
- Clippy and Dylint run as the local `mise run cargo:clippy` and `mise run cargo:dylint` gates over those respective scopes; the hosted CI that ran them as separate dependency-free jobs — and the `ci_contracts` test locking their job independence and exact package scopes — is parked for the reboot, returning with the `.github/workflows/` surface (`gandr-kk7`; [ci.md](ci.md)).

New untracked package allowlists or exclusions are prohibited; the Dylint lanes have carried zero exclusions since the 2026-07-30 lint-wall pass.

The rollout established these durable findings and actions:

1. **Semantic wrappers are API boundaries, not aliases.** Primitive and tuple APIs hid byte offsets, grammar slots, identities, generations, and proof positions.
   Use private `#[repr(transparent)]` newtypes, checked constructors, and exact external-trait conversions.
   Do not restore compatibility with generic `Into`, inherent primitive getters, or primitive `PartialEq`.
2. **Recursion claims require call-graph evidence.** Free functions and methods are checked by recursive SCC, including nested calls and generic arguments.
   Recursion over caller-owned input must become an explicit bounded worklist; documentation alone cannot relabel it as non-recursive.
3. **Mutation-before-error paths are architectural evidence.** Preserve the isolated upstream non-local-effect pass and investigate its `gandr-core-checker` lib-test panic; never disable the lint globally.
4. **Future rules should encode cross-module invariants.** Tracked follow-up candidates: checker/machine constructor parity, atomic force-state transitions, subprocess-boundary policy, and nominal-id replay provenance; semantic-wrapper escape prevention is tracked by `gandr-3da`.

### Dependencies and the workspace

- **Dependencies.** Use the `find-best-rust-crates` skill before adding a nontrivial crate, but treat external implementations as design references rather than automatic dependencies.
  Machinery that is load-bearing for the core or certified-kernel boundary — including recursion/control runtimes, graph representations and algorithms, proof-state machinery, and semantic normalization — stays gandr-owned even when reimplementation costs more.
  Prefer `core`/`alloc`, a focused local crate, and existing workspace dependencies in that order; external convenience must not enlarge the trusted or bootstrapping-critical base.
  Existing external graph dependencies such as `petgraph` are migration debt rather than precedent: when their role becomes kernel-relevant, replace them with local representations and algorithms.
  For non-kernel dependencies that pass that boundary test, default `default-features = false` with an explicit feature list (exceptions where required runtime lives behind defaults: `proptest`, `libfuzzer-sys`).
- **Crates join the workspace; detached crates are prohibited.** Every new crate is a member of the root workspace from day one — no crate-local `[workspace]` tables, no out-of-workspace drivers with their own lockfiles, toolchain pins, or lint posture.
  Detached crates escape the `[workspace.lints]` wall and silently accumulate lint debt that returns as a remediation project (the out-of-workspace Dylint driver is the case history: it arrived with an entire lint surface to triage, and clearing it took its own remediation pass — `crates/workflow-dylint` has carried `[lints] workspace = true` since 2026-07-30, and satisfies the wall with no crate-level relaxation and six item-scoped `#[expect]`s whose reasons name the rustc API shape that makes each lint inapplicable).
  A crate that cannot satisfy the wall yet still joins the workspace and instead carries a crate-local override block referencing a triage bead, so the debt stays visible and scoped.
- **Dependencies live once, at the workspace root, and are inherited everywhere.** Every external dependency — normal, build, or dev — is declared in the root `Cargo.toml` under `[workspace.dependencies]`, and member crates reference it with `{ workspace = true }`.
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

Every nontrivial item (public or private — `missing_docs_in_private_items` is denied) carries a one-line summary plus a `# Contract` rustdoc block; fallible functions also carry `# Errors`; nontrivial items in new or substantially-refactored code also carry `# Adequacy` ([mutation-adequacy.md](mutation-adequacy.md)).

`mise run cargo:doc-check` is the merge-wall gate for this section: it runs `cargo doc --workspace --features=full --no-deps --document-private-items` on the pinned nightly with `RUSTDOCFLAGS="-D warnings"`, so an intra-doc link that does not resolve — or a redundant explicit link target — fails the wall rather than accreting as silent rustdoc debt.

**That gate reads library targets and nothing else, and the boundary is worth knowing before you write a doc comment in `tests/`.** `cargo doc` does not document test targets, so until `test:doc-links` runs beside it, every intra-doc link written in an integration test is unchecked — and unchecked here means a link that could never have resolved passes indefinitely, which is what happened to a `theory-levitation` test doc comment linking into a crate that appears in none of its dependency sections.
**Widening the gate is the standing choice rather than exempting the class**, decided on measurement: running rustdoc over all thirty-two test targets surfaces exactly one broken link and seven test roots missing a crate-level `//!` header, and a fully cached sweep of all thirty-two costs about five seconds.
The fear that made the question hard — an unknown backlog surfacing at once — priced out at eight findings.
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

- Clauses in fixed order, as `-` bullets: `requires` (caller preconditions), `ensures` (postconditions on success), `provides` (what the item yields), `fails` (failure modes and how they surface), `panics`, optionally `intension` (last); omit a clause only when it does not apply.
  Write `- panics: none.` explicitly — the absence of a panic is a contract.
  An `unsafe` item adds `- unsafe invariants:`, the rustdoc `# Safety` section, and `// SAFETY:` comments (`undocumented_unsafe_blocks` is denied).
- `- intension:` states properties of _how_ the computation proceeds (enumeration/tie-break order, traversal strategy, cost, determinism, trace shape) — only those the item **promises**, each observable through a **declared semantic projection** the API exposes.
  Intensional tests assert only declared projections; extensional clauses never reference intensional observations (the calf noninterference discipline, [mutation-adequacy.md](mutation-adequacy.md)).
- `# Errors` (clippy pedantic) coexists with `# Contract`: `- fails:` is the design-level statement, `# Errors` the per-variant enumeration.
- `# Termination` is mandatory on every directly or mutually recursive function or method.
  Use the fixed grammar below; each field needs a concrete explanation, not an assertion that termination is obvious:

  ```rust
  /// # Termination
  /// - reason: why recursion is the appropriate control structure.
  /// - measure: the quantity that strictly decreases on every recursive edge.
  /// - boundedness: where the finite or well-founded bound comes from.
  /// - input recursion: none.
  ```

  `- input recursion: none.` is required everywhere except the model recursive checker in `gandr-core-checker::judgements::checker::Rec`, if that implementation is still recursive.
  That checker may instead name structural descent through the finite checked term, because serving as the direct recursive reference model is its purpose; the defunctionalized machine remains the adversarial-depth path.
  Tail-call position does not remove this obligation because Rust does not guarantee tail-call optimization; a genuinely iterative implementation is not recursive and needs no termination section.
- `# Adequacy`: `- hypothesis:` — a falsifiable claim naming which adequacy-ladder rung kills each decision surface's mutants, plus the distinguishing inputs and observations for the pointwise residue — then one `- witness:` bullet per witnessing test (crate-qualified when it lives in another crate).
  Fixed grammar — the adequacy gates machine-extract it.
- "Nontrivial" = has a precondition a caller can violate, can fail, or has a non-obvious postcondition.
  Thin builders, trivial accessors, and data constants get the one-line summary only — do not manufacture blocks for them.
