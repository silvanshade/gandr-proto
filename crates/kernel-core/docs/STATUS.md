# Status

The crate is slice 3 of the minimal certified kernel (`docs/gandr/spec/kernel-boundary.md` §8), stages B2.1 and B2.2: the **S1 pure polarized core** — term/type language, declaration vocabulary, environment + choke point, checker, and the C5-quarantined conversion (B2.1) — plus the **K5 export writer and validating reader** (B2.2) — complete and green.
The core-checker bridge and corpus gate (B2.3) and the fcw.8 spec components (B2.4) are the remaining B2 stages.

Implemented (B2.1, slice 3):

* **Term/type language** (`term`, `types`, `base`) — the closed, de-novo S1 CBPV vocabulary (K1): values (variable, constant reference, unit, integer/string/numeric literals, pair, sum injection, thunk, explicit lift), computations (lambda, application, return, bind, force, sum case), value types (base atom, unit, product, sum, thunk, universe, lift), computation types (`F` pure, arrow).
  Terms are nameless (de Bruijn), so α-equivalence is syntactic; no hole/mark/metavariable/annotation/effect-row/datatype constructor exists to be represented.
  Literal payloads are canonical (normalized) so syntactic equality tracks value equality.
  The four recursive owned enums deallocate and duplicate **iteratively** — a worklist `Drop` and a hand-written `Clone` over an explicit heap stack — so an adversarial-depth term or type (which export decode can build from bytes) never overflows the stack when destroyed or duplicated (gandr-i3i); `PartialEq`/`Eq`/`Hash` stay derived and recursive (exercised only by tests, an iterative-rewrite residual).
* **Levels** (`levels`) — the per-declaration `LevelContext`: prenex parameter scoping and the universe rule `U_l : U_m` iff `l < m`, decided by `check_universe_below`.
  With no landmark constraints declared this is exactly `gandr_kernel_strata::Level::lt` (the free oracle); with constraints declared it is landmark entailment under the admitted poset (`entails_lt`), which strata guarantees agrees with the free oracle on the empty poset.
  Admission of the landmark constraints is a consistency gate (a looping set is rejected).
* **Checker** (`check`) — the zero-inference (K2), bidirectional S1 checker: annotation-free introductions check against the declared type flowing down; eliminators and atoms synthesize; a mode switch converts.
  No term-into-type substitution is ever needed at S1 (no type former is indexed by a value term), so context types are closed and never shifted.
  Total on adversarial depth via a **defunctionalized machine** (an explicit goal register, produced register, heap frame stack, and explicit typing-context stack) — not a recursion-depth budget — so it meets the `docs/workflow/rust.md` "input recursion: none" discipline (gandr-98o); type formation and conversion are the two self-contained iterative walks the machine calls directly.
* **Conversion** (`conv`) — the C5-quarantined definitional equality.
  At S1 the checker's conversion is type-only and coincides with structural equality (canonical levels); it never descends into a term, so the "never evaluate a computation during conversion" quarantine holds **vacuously**.
  The value/computation term conversion (α-structural, no β) is present, quarantined, and unused by checking — the seed for term-indexed extensions.
  Every face is iterative over a heap worklist (total at any depth; the derived `PartialEq` would recurse and overflow).
* **Environment + choke point** (`env`) — the append-only `Environment` with the single checked entry `add_decl -> Result<CheckedId, KernelError>`, the single warned bypass `add_decl_unchecked`, and the `#print axioms` audit (`audit -> AxiomReport`) reporting the transitive axioms and unchecked admissions a declaration rests on.
  A `CheckedId` is unforgeable outside the crate.
  One checked/unchecked bit, no trust lattice (K3).
* **Errors** (`error`) — the closed, honest `KernelError` vocabulary; level and universe failures carry the `gandr_kernel_strata` refutation evidence they rest on.
  There is deliberately no non-canonical-level variant: an in-memory `Level` is canonical by construction, so the "reject non-canonical input" obligation is discharged by construction here and re-armed at the decode boundary in B2.2.

Implemented (B2.2, slice 3):

* **Export writer/reader** (`export`) — the K5 re-checkable export (kernel-boundary.md §5, E1–E6).
  `write` serializes an `Environment` to self-contained, deterministic canonical bytes: a version header, the reserved sections, then the admission-ordered declaration sequence, iterating declarations in admission order and level atoms in `BTreeMap`-sorted order (never hash-order) so an identical environment yields byte-identical output (E1/E2/E4/E5).
  Each declaration carries its admission mark (checked vs unchecked-bypass) and `Axiom`s serialize as `Axiom`s, so the §3 audit survives (E6); the precomputed transitive `rested_on` sets are **not** written — the reader recomputes them by re-admitting through the environment (E3).
  `decode` is the validating reader: a total, closed-vocabulary parser with the rejection triple as a closed error vocabulary (`DecodeError`: truncation / an unknown tag / a structural violation), extended by named refusals for the reserved declaration kinds (R1), non-empty reserved slots/sections (R2/R3/R4), and an unknown version (E5).
  It decodes through constructors: levels rebuild through the `gandr-kernel-strata` smart constructors (canonical by construction), and a whole-artifact canonical-bytes check rejects any non-canonical _encoding_ — re-arming the B2.1 non-canonical-level obligation at the decode boundary.
  Term and type trees decode **iteratively** over an explicit frame worklist (the `conv` precedent), never input-scaled recursion, and the writer is likewise iterative, so an adversarially deep artifact (admissible through the bypass) never overflows the stack.
  `read` replays the decoded sequence through the choke point — `add_decl` for a checked mark, `add_decl_unchecked` for a bypass mark — reproducing the environment with its audits recomputed (E2/E3/E6); `ReadError` holds the decode plane (`DecodeError`) and the re-admission plane (`KernelError`) distinct.
  All seven ratified reservations are present in the v0 format and format-plane only (zero S1 typing consequence): the R1 reserved declaration-kind tags (`AbstractType`=2/`ModuleSig`=3/`ModuleDef`=4/`FunctorDef`=5, rejected distinctly), the R2 structured (segment-sequence) name pinned empty, the four R3 per-`Def` annotation slots pinned empty, and the R4 reserved minted-atom table pinned empty.

Tests are green under `cargo test -p gandr-kernel-core` (93 total): per-module unit goldens (each checker rule, each error arm, the audit closure, the universe rule and its irreflexivity, landmark admission both ways), plus `tests/conversion.rs` (reflexivity/symmetry/separation property differentials over generated types and terms), `tests/checker.rs` (the kernel-native golden corpus of well-typed and ill-typed declarations, plus a totality-and-determinism property over arbitrary bodies), `tests/export.rs` (the round-trip, determinism, and rejection-totality differentials: `read(write(env))` reproduces declarations, marks, and recomputed audits; truncation at every prefix and arbitrary random bytes always return; the reserved kinds/slots, an unknown version, and a non-canonical level/literal encoding each reject), and `tests/hardening.rs` (the adversarial-depth totality suite: ~1M-deep term and type chains drop and clone, and a decoded deep declaration drops, inside small-stack threads without overflow — plus indirect drop through a `KernelError` payload, and the defunctionalized checker admitting a ~200k-deep well-typed pair definition and a ~200k-deep bind definition where the old recursive descent would have overflowed).

## Design decisions recorded for review

* **Defunctionalized checker, no depth budget (gandr-98o).** The checker is a **defunctionalized adversarial-depth machine** ([`check::run`]): one goal register / produced register / heap frame stack / explicit typing-context stack replaces the six formerly mutually recursive methods, and the module docs carry a correspondence table (each old recursive arm ↔ one goal push plus at most one continuation frame) so the reviewer can walk the translation.
  Totality on adversarial depth is now structural — no recursion, so no stack to overflow, and no `Depth::LIMIT`/`DepthLimitExceeded` budget (both removed).
  This meets the `docs/workflow/rust.md` "input recursion: none" discipline for the kernel checker.
  Type formation (`type_level`) is a second self-contained iterative walk the machine calls directly, exactly as it calls the already-iterative conversion; the context is an explicit `Vec` of `Held` slots (`Borrowed` for a declaration-derived type, `Owned` for a `Bind`/`Case`-synthesized binder) with scope-exit frames, and slot lookup clones (total via the iterative `Clone` of gandr-i3i).
  The produced-register projections are **fail-closed** (`KernelError::CheckerRegisterFault`): a goal↔frame polarity mismatch — unreachable when the correspondence table is wired correctly — surfaces as a rejection rather than a fabricated type, so the machine's wiring is defense-in-depth over soundness, never a soundness assumption (coordinator review hardening; the `LevelOracleFault` surfaced-not-trusted posture applied to the machine's own registers).
  The kernel-vs-`gandr-core-checker` corpus differential stays green on its existing (bounded) domain; adversarial-depth inputs are excluded from that differential by design — the untrusted recursive reference keeps its own budget, so divergence there is deliberate, and `gandr-core-checker` is left unchanged.
  Deviation from the dispatch proposal (recorded for review): type formation is a **direct** iterative call rather than a goal in the term machine, because its `value_type_level`/`comp_type_level` recursion is a self-contained cluster (like conversion) — folding it into the term machine would force the `Lift` synthesis to compute the level of an owned, just-synthesized type through the goal stack, which the direct call avoids cleanly.
* **Lift is a value term, not implicit cumulativity.** `Value::Lift { target, body }` explicitly inhabits `ValueType::Lift { inner, target }` when the inner type's level is strictly below `target` — so a bare `body : A` never inhabits `Lift A target` on its own.
  This is the clean stratified realization of "explicit lift terms, no implicit cumulativity"; the spec's phrasing ("lift terms") is honoured as a value former plus its type former.
* **Declarations are value-polarity at the boundary.** A `Def` pairs a declared value type with a value body; a computation definition enters as a thunk (`U C`, body `thunk …`) and is used via `force`.
  This keeps `add_decl` single-polarity with no polarity-mismatch error.
* **A `Value::Constant` reference form** (a reference to a prior admitted declaration by admission position) was added beyond the coordinator's terse S1 value stock, because the append-only environment and its transitive `#print axioms` audit structurally require cross-declaration references.
  It is the only representable way an axiom can be "rested on".
* **Base-type atoms are `{ Integer, String, Numeric }`** — the three the S1 literal stock (`int/str/num`) names, taken as a closed, rigid set.
  Literal payloads are inert to checking at S1 (no type embeds a value term); the exact numeric-literal grammar is a v0 placeholder.
* **Worklist `Drop` and iterative `Clone` for the recursive owned types (gandr-i3i).** The derived `Drop` glue and a derived `Clone` on `Value`/`Computation`/`ValueType`/`CompType` recurse on term/type depth and would overflow the stack on an adversarial-depth tree (export decode builds one from bytes, and every checker rejection path drops the tree — including a `KernelError` whose payload boxes deep types).
  Both are hand-written to be iterative over an explicit heap worklist: `Drop` extracts each node's children by placeholder-swap (unit for the value slots — allocation-free; a trivial returner for the negative-polarity slots — one transient allocation) so the compiler's own drop glue only ever sees leaves, and `Clone` is a two-stack goal/produced walk mirroring the `conv`/`export` idiom.
  Alternatives rejected: derived recursion (the live overflow hazard), and `unsafe`/`ManuallyDrop` to avoid the placeholder allocation (unjustified — the transient allocation is negligible and the safe version is auditable).
  Consequence and residual: adding `Drop` forbids by-value moves out of these types (E0509), so the checker's synthesizing arms extract through `mem::replace` take-helpers rather than destructuring; and `PartialEq`/`Eq`/`Hash` stay derived (recursive) because the production comparator is the iterative `conv` — an iterative rewrite of those derives is a tracked residuals candidate, not a live hazard (they run only on bounded test fixtures).

Export (B2.2) design decisions recorded for review:

* **The transitive audit sets are recomputed, never exported (E3).** The precomputed `rested_on` sets are omitted from the bytes; the reader recomputes them by re-admitting each declaration through the choke point.
  Alternative: serialize the audit as a claim and re-validate it — rejected, because re-admission already recomputes it and shipping derived data invites trusting it (K4/E3).
* **Decode failure is a distinct type from typing failure.** `DecodeError` (format plane) is separate from `KernelError` (typing plane); `ReadError` unions the two for `read`.
  Alternative: widen `KernelError` with decode variants — rejected, because a malformed byte is not a typing judgment and the two planes must not blur.
* **A reserved declaration kind rejects distinctly (R1).** A reserved kind byte is `DecodeError::ReservedDeclarationKind`, not a generic unknown-tag rejection.
  Alternative: fold reserved kinds into the bad-tag arm — rejected as less honest and less diagnostic.
* **Structured names are a segment sequence pinned empty at v0 (R2).** The per-declaration name record is a segment count (zero at v0), never a flat dotted `M.l` string; a non-empty name is rejected, since S1 declarations are nameless and `add_decl` takes no name (a name could not round-trip).
  Alternative: omit the name field until names arrive — rejected, because R2 requires the structured shape present from birth.
* **Level decode is bounded by an offset cap.** Strata exposes no `O(1)` `var + offset` constructor, so a variable atom `x + o` is rebuilt through `o` applications of `succ`; a decode-time cap (`MAX_DECODED_LEVEL_OFFSET = 4096`) bounds that work — bounded work on adversarial input, the reader's totality posture.
  Consequence: a level whose _variable-atom_ offset meets the cap does not round-trip (implausible — real offsets are `0`/`1`; universe _constants_ are uncapped and `O(1)`).
  Alternative: an unbounded `succ` loop (rejected — not bounded work on adversarial input) or a fallible writer that refuses such levels (rejected — the task fixes the writer's return as `Vec<u8>`).
  The cap lifts when strata exposes an `O(1)` offset constructor (carry-note for a future strata slice; strata is out of B2.2 scope).
* **Canonical bytes are enforced by a whole-artifact re-encode-compare.** The decoder is tolerant (it normalizes through the constructors) and a single `encode(decode(bytes)) == bytes` check rejects every non-canonical encoding — a padded literal, an unsorted or dominated level atom, an overlong varint — as `DecodeError::Malformed { NonCanonical }`.
  Alternative: per-field canonical checks scattered through the decoder — rejected as more code with more room to leave a gap.
* **The writer reads the environment's admission log through a `pub(crate)` surface.** `Environment::admitted` (and the widened `Admission` visibility) expose the admission-ordered declarations and their marks to the same-crate writer without a public leak; the `to_digits`/`to_content` literal-payload accessors were added to `base` for the same reason.
  The byte-plumbing helpers (`ByteReader`, varints) traffic in `u8`/`usize` rather than nominal wrappers, a small deviation from the crate's primitive-wall discipline scoped to the codec interior; the domain surface (public API, tags-as-sites, error payloads) stays nominal.

Deliberately **not** here (kernel-boundary.md §7 S1 exclusions): effects/handlers, the control fragment, general recursion/natives, datatype declarations and description codes, `Sigma`/`Split`, `Path`/identity, `List`/`Record`/`With`, holes, marks, annotations, `dup`/`drop`.
The export format's seven ratified reservations are format-plane only and landed in B2.2 (`export`) — nothing is reserved inside the S1 types.

The crate is `#![no_std]` over `core`/`alloc`, depending only on `gandr-kernel-strata` — the design record's TCB dependency wall (kernel-boundary.md §2) in its sharpest form.
