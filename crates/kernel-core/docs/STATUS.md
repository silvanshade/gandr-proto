# Status

The crate is slice 3 of the minimal certified kernel (`docs/gandr/spec/kernel-boundary.md` §8), stage B2.1: the **S1 pure polarized core** — term/type language, declaration vocabulary, environment + choke point, checker, and the C5-quarantined conversion — complete and green.
The export writer/reader (B2.2), the core-checker bridge and corpus gate (B2.3), and the fcw.8 spec components (B2.4) are the remaining B2 stages.

Implemented (B2.1, slice 3):

* **Term/type language** (`term`, `types`, `base`) — the closed, de-novo S1 CBPV vocabulary (K1): values (variable, constant reference, unit, integer/string/numeric literals, pair, sum injection, thunk, explicit lift), computations (lambda, application, return, bind, force, sum case), value types (base atom, unit, product, sum, thunk, universe, lift), computation types (`F` pure, arrow).
  Terms are nameless (de Bruijn), so α-equivalence is syntactic; no hole/mark/metavariable/annotation/effect-row/datatype constructor exists to be represented.
  Literal payloads are canonical (normalized) so syntactic equality tracks value equality.
* **Levels** (`levels`) — the per-declaration `LevelContext`: prenex parameter scoping and the universe rule `U_l : U_m` iff `l < m`, decided by `check_universe_below`.
  With no landmark constraints declared this is exactly `gandr_kernel_strata::Level::lt` (the free oracle); with constraints declared it is landmark entailment under the admitted poset (`entails_lt`), which strata guarantees agrees with the free oracle on the empty poset.
  Admission of the landmark constraints is a consistency gate (a looping set is rejected).
* **Checker** (`check`) — the zero-inference (K2), bidirectional S1 checker: annotation-free introductions check against the declared type flowing down; eliminators and atoms synthesize; a mode switch converts.
  No term-into-type substitution is ever needed at S1 (no type former is indexed by a value term), so context types are closed and never shifted.
  Total on adversarial depth via a recursion-depth budget (`Depth::LIMIT`).
* **Conversion** (`conv`) — the C5-quarantined definitional equality.
  At S1 the checker's conversion is type-only and coincides with structural equality (canonical levels); it never descends into a term, so the "never evaluate a computation during conversion" quarantine holds **vacuously**.
  The value/computation term conversion (α-structural, no β) is present, quarantined, and unused by checking — the seed for term-indexed extensions.
  Every face is iterative over a heap worklist (total at any depth; the derived `PartialEq` would recurse and overflow).
* **Environment + choke point** (`env`) — the append-only `Environment` with the single checked entry `add_decl -> Result<CheckedId, KernelError>`, the single warned bypass `add_decl_unchecked`, and the `#print axioms` audit (`audit -> AxiomReport`) reporting the transitive axioms and unchecked admissions a declaration rests on.
  A `CheckedId` is unforgeable outside the crate.
  One checked/unchecked bit, no trust lattice (K3).
* **Errors** (`error`) — the closed, honest `KernelError` vocabulary; level and universe failures carry the `gandr_kernel_strata` refutation evidence they rest on.
  There is deliberately no non-canonical-level variant: an in-memory `Level` is canonical by construction, so the "reject non-canonical input" obligation is discharged by construction here and re-armed at the decode boundary in B2.2.

Tests are green under `cargo test -p gandr-kernel-core` (63 total): per-module unit goldens (each checker rule, each error arm, the audit closure, the universe rule and its irreflexivity, landmark admission both ways, the depth-budget totality guard), plus `tests/conversion.rs` (reflexivity/symmetry/separation property differentials over generated types and terms) and `tests/checker.rs` (the kernel-native golden corpus of well-typed and ill-typed declarations, plus a totality-and-determinism property over arbitrary bodies).

## Design decisions recorded for review

* **Recursive checker + depth budget vs defunctionalization.** The checker is a recursive bidirectional reference (transparently correct, matching the S1 rules 1:1).
  Totality on adversarial depth — a hard kernel requirement — is achieved by a recursion-depth budget that returns `KernelError::DepthLimitExceeded` rather than overflowing the stack, not by defunctionalization.
  This mirrors the untrusted `gandr-core-checker`'s recursive reference; the budget-free **defunctionalized adversarial-depth machine** is the tracked follow-up.
  Consequence: the `docs/workflow/rust.md` "input recursion: none" discipline (enforced by the project-local Dylint, not by the `cargo:clippy`/`cargo:nextest` merge gate) is not met by the recursive checker/type-formation methods; their `# Adequacy`/`# Contract` blocks describe the structural descent honestly rather than claiming `none`.
  Conversion and the audit/dependency walks are already iterative.
* **Lift is a value term, not implicit cumulativity.** `Value::Lift { target, body }` explicitly inhabits `ValueType::Lift { inner, target }` when the inner type's level is strictly below `target` — so a bare `body : A` never inhabits `Lift A target` on its own.
  This is the clean stratified realization of "explicit lift terms, no implicit cumulativity"; the spec's phrasing ("lift terms") is honoured as a value former plus its type former.
* **Declarations are value-polarity at the boundary.** A `Def` pairs a declared value type with a value body; a computation definition enters as a thunk (`U C`, body `thunk …`) and is used via `force`.
  This keeps `add_decl` single-polarity with no polarity-mismatch error.
* **A `Value::Constant` reference form** (a reference to a prior admitted declaration by admission position) was added beyond the coordinator's terse S1 value stock, because the append-only environment and its transitive `#print axioms` audit structurally require cross-declaration references.
  It is the only representable way an axiom can be "rested on".
* **Base-type atoms are `{ Integer, String, Numeric }`** — the three the S1 literal stock (`int/str/num`) names, taken as a closed, rigid set.
  Literal payloads are inert to checking at S1 (no type embeds a value term); the exact numeric-literal grammar is a v0 placeholder.

Deliberately **not** here (kernel-boundary.md §7 S1 exclusions): effects/handlers, the control fragment, general recursion/natives, datatype declarations and description codes, `Sigma`/`Split`, `Path`/identity, `List`/`Record`/`With`, holes, marks, annotations, `dup`/`drop`.
The export format's seven ratified reservations are format-plane only and land in B2.2 — nothing is reserved inside the S1 types.

The crate is `#![no_std]` over `core`/`alloc`, depending only on `gandr-kernel-strata` — the design record's TCB dependency wall (kernel-boundary.md §2) in its sharpest form.
