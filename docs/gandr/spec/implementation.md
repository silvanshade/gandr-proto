# Implementation

This track owns the Rust system: the crate map, the kernel IL and typing machine, the rewriting, completion, and tracelet engines, the content-addressed storage stack, the surface pipeline, the runtime, and the gates.
`PLAN.html` is the phase-roadmap authority; this document is the standing description of what is built and how it is shaped, with the phase table summarized for orientation.
Detailed remaining work is in [[implementation/roadmap]].
This markdown corpus is the only specification surface: the older machine-validated component corpus and its pipeline were removed, and `crates/workflow-docs` remains only as the parked prose document-class tool for the tracked `.xml` documents.

## The build-out at a glance

Eleven backbone phases plus an ordered tail, with parallel lanes; statuses are tracker truth, not plan prose.

| phase                              | content                                                                                                                                                                        | status                                                                                                                                                                  |
| ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| machine port                       | the L machine replaces the CEK; machine effect substrate                                                                                                                       | **closed** — the CEK is physically removed; differentials compare against frozen snapshots                                                                              |
| kernel-core skeleton               | the innermost trusted stage: term language, declaration vocabulary, environment with one admission choke point, small conversion, export writer with reserved annotation slots | **closed**                                                                                                                                                              |
| modules                            | 1ML-style functors and first-class modules over CBPV as their own primitive layer [@rossberg-2018-1ml]                                                                         | in progress (primitive modules, sealing with export replay, generative functors, package Σ remain)                                                                      |
| performance architecture           | glued-NbE hash-consing normalizer adapted to the L machine                                                                                                                     | open                                                                                                                                                                    |
| builtins                           | simple builtins over machine + modules                                                                                                                                         | open                                                                                                                                                                    |
| dependent core                     | Π/Σ, motives, patterns, elaborator; modules↔dependent-records harmonization                                                                                                    | open                                                                                                                                                                    |
| identity layer                     | `Path`/`walk` and `Flow`, groupoid and directed both from the start, without-K binding                                                                                         | open                                                                                                                                                                    |
| levitation                         | the descriptions layer over `theory-levitation`                                                                                                                                | open                                                                                                                                                                    |
| certificates                       | tracelet certificates; produoidal two-mode composition; **the independent second checker (kernel-replay) lands here**                                                          | open                                                                                                                                                                    |
| doctrines + reflection             | virtual doctrines and judgemental reflection                                                                                                                                   | open                                                                                                                                                                    |
| higher-dimensional-rewriting story | end-to-end demo/integration across the assembled stack (the machinery mostly exists; nothing in the checker/pipeline path consumes the two engine crates yet)                  | open                                                                                                                                                                    |
| tail                               | arbitrary-precision numerics, stone duality ([[metatheory/exact-reals                                                                                                          | the exact-reals line]]), sessions/worlds, intersections/unions, codata, codecs, user-facing effects, cost-as-effect, value semantics/modes, the shell language, erasure |

Lanes: the substrate burst (closed), incrementality (in progress; standing gate: incremental ≡ from-scratch), the surface ports (in progress), runtime, docs, performance discipline, and the **tracelet-algebra thread, declared priority over all other build-out**.

## Architectural commitments

* **The kernel is the TCB wall.** A `kernel-*` crate depends only on other `kernel-*` crates plus `core`/`alloc`; everything else — `core-*` included — is _permanently untrusted_.
  The wall is stated in the kernel manifests and enforced by the dependency graph; dev-dependencies are exempt because test code is not shipped trust.
* **The kernel never hashes.** Content addressing is untrusted plumbing in the storage stack; the kernel's export writer emits offsets and lengths only.
* **The export format reserves growth from birth.** The v1 format carries reserved declaration kinds (abstract types, module signatures and definitions, functors), structured names, four per-definition annotation slots, and a reserved minted-atom table — all present, all rejected as occupied at v1, so tail phases are additive rather than kernel-format rewrites.
* **An independent second checker is a scheduled artifact, not shared code.** Kernel-replay re-derives records from opaque foreign bytes by a reader-side framing walk against the format specification, never by linking the writer.
* **Accelerators are never soundness-bearing** — advisory or exact-and-differentialed, and the kernel never links the band.
* **Hopf/Lie/enveloping-algebra structure is metatheory currency only**: the kernel ships canonical forms, caches, counters, and replay; no vector space, formal sum, antipode, or word problem ever ships.
* **No hosted CI until go-public**: the local worktree-merge + hook + task wall is the whole wall.

## The crate map

Twenty-four workspace members (twenty-five directories; the doc-class tool `workflow-docs` is parked), edition 2024, uniform feature scheme, in dependency tiers:

| tier     | crates                                                                                                                                                         |
| -------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 0        | `kernel-strata`, `storage-chunker`, `surface-syntax`, `surface-render-remote`, `theory-graphs`, `theory-nominal-automata`, `theory-orders`, `theory-recursion` |
| 1        | `kernel-core` (→ strata), `storage-prolly-trees` (→ chunker), `surface-grammar` (→ render-remote, syntax, graphs)                                              |
| 2        | `core-checker` (→ kernel-core, nominal-automata, orders), `storage-artifact` (→ kernel-core, chunker, prolly-trees), `surface-parser` (→ grammar, syntax)      |
| 3        | `core-sequent` (→ checker; kernel-core, strata, artifact, prolly-trees dev-only), `theory-levitation` (→ checker)                                              |
| 4        | `theory-computads` (→ sequent, graphs, levitation), `runtime-host` (→ checker, sequent)                                                                        |
| 5        | `theory-virtual-doctrines` (→ computads, levitation; sequent dev-only), `surface-engine` (the pipeline hub)                                                    |
| 6        | `surface-corpus` (the executable corpus harness)                                                                                                               |
| off-tier | `workflow-gates`, `workflow-dylint`, `surface-driver` (a stub until the wrapped crates land)                                                                   |

## The trusted base

**`kernel-strata` is the certified level oracle** — a stratum is a _universe level_, nothing else.
A level is the canonical finite join over the algebra of zero, successor, and join; canonical form is a constructor invariant, so derived structural equality _is_ level equality.
Ordering is by domination, not a numeric ladder; the kernel spends levels on exactly one rule (a universe inhabits a universe iff strictly below, decided by the oracle), lifts are written and never inferred, and each declaration generalizes over its own prenex level variables and its own declared **landmark poset** of constraints, admitted by loop-checking.
Every decision returns checkable evidence — a consistency witness (an explicit ℕ-homomorphism) or a replayable loop witness — so **trust concentrates in the validators**, and the decision procedure is self-incriminating under mutation.
Refused by design: level inference, unification, cumulativity, `imax`, `Prop`, constants in declared constraints.
No dependencies, no `std`.

**`kernel-core` is the certified innermost trusted stage**: pure polarized CBPV terms and types, definition/axiom vocabulary, an append-only environment whose single admission choke point re-derives everything (the elaborator gets no credence), and quarantined conversion.
The named invariants: closed input (no hole, metavariable, mark, or effect row _exists to be represented_); zero inference; one choke point with an unforgeable checked-id and exactly one checked/unchecked bit (no trust lattice); derived schemes never trusted; re-checkable export.
The admission watermark: after admission returns — success or rejection — the arena holds exactly the prior-admitted nodes plus this declaration's content; checker intermediates never persist, and arena ids strictly increase so the graph is acyclic by construction.
Internal-defect paths **fail closed** through dedicated fault variants; there are no panic paths (zero `# Panics` sections across both kernel crates is the stated posture).
No trusted memo table enters the TCB: the conversion early-out decides only reflexive pairs, and pointer-keyed kernel caches are a priced trust expansion deliberately not taken at that stage.
The unsoundness threat model is answered structurally: type-in-type by the level oracle's stratification; positivity by levitated description codes being strictly positive by construction; non-termination by the effect quarantine with fixed-point and step budgets on the computation side against the evidence fragment.

**Export and replay.** Reading an artifact decodes and then **replays every declaration through admission**, so the audit is recomputed rather than believed: the rested-on audit sets are not in the bytes (a forged audit cannot ride along), canonical bytes are enforced by a sharing-aware whole-artifact re-encode-compare (the writer feeds no judgment and is not a trusted fast path), and reader budgets on expanded term work and table entries defend against adversarial sharing, which moves amplification attacks from memory to checker time.
The axiom audit (`print-axioms`) reports the transitive axioms and unchecked admissions a declaration rests on; the unchecked-admission bypass carries an explicit soundness warning.

## The checked language

**`core-checker` is the bidirectional typing machine, realized twice and gated on agreement.** Judgements infer or check values and computations, with introduction forms checking, elimination forms inferring their principal premise, and one inlined subsumption rule mediating; the mode split is carried by a direction type whose check mode carries the expected type.
Stack typing is internalized as a value type; the context has a linear zone (frozen shape, vacuous at v0); grades form a preordered semiring; effects are sealed rows on the returner type.
Subtyping is consistent subtyping in the gradual-typing sense once the unknown type participates: reflexive by rule, deliberately **not** transitive by rule — transitivity is admissible (provable from the others), not a rule of the calculus.
The recursive checker and the defunctionalized machine are property-tested for step-for-step agreement on a control log; a third, total _marking_ realization is oracle-bound to the recursive one.
The differential proves the two agree, not that either is correct — a shared soundness bug leaves them agreeing on the wrong answer — which is why the suite is supplemented by directed coherence oracles relating the two _modes_, a declared-companion gate on every biased oracle, and a standing adversarial pass before substantial checker changes.

**The frozen core's grammar, as built.** Value types: unit, eager products, sums, graded thunks `U_r B`, and the gradual `Unknown`, with the value-model ladder's literals, lists, and records; computation types: the returner `F A`, arrows, lazy products (`With`), and `Unknown`; unions, intersections, and the world modality are designed formers outside the current build.
Terms follow the bidirectional discipline: introductions check, eliminations infer, one subsumption mediates.
The dependent formers that have landed, each with its guard:

* **Declared data** — generative-nominal declarations with constructors and case elimination; a constructor in inference mode (or against a mismatched data type) is stuck, exactly as an injection.
* **`Σ` dependent pairs with the motive-carrying split eliminator** — inference-capable exactly when the motive is present (the motive supplies the result type); a motive-less split never infers, firing at rule entry before the scrutinee is touched — the binder-escape hazard resolved by dependent motive, never by a scope check.
* **The identity fragment, rung 1** — `Path A x y` (gandr's first dependent former: values occur inside the type, but types carry no binders), introduced by `here(v)` and eliminated by the full dinatural `walk` with an explicit motive (β only; no η, no K; Paulin–Mohring forms derived, never primitive).
  Endpoint comparison is structural value equality — α-respecting, pointer-fast-pathed, hole/annotation-consistent — with **no reduction inside types** at this rung (the NbE-era definitional equality that adds walk-β, congruence, and substitution laws is the identity layer's own phase).
  The **K-rejection witness is a live diagnostic**: a `case` on an identity type is rejected — the reserved here-pattern fragment requires the without-K unification fragment, whose solver declines the deletion step itself.
* **The un-levelled code universe** — the levitation stage-1 code former in the checker, with the description decoder; the kernel bridge rejects it — the kernel's levelled universe and explicit lifts are authored kernel-native, and bridged declarations stay level-monomorphic.

The phase table's identity-layer and dependent-core rows are open at the phase level (the directed former `Flow`, kernel inclusion, Π, patterns, and the elaborator), not at the rung level: everything above is landed and differentially gated.

**`core-sequent` is the polarized System-L command IL and the L machine — the sole evaluator.**

```text
Producer p ::= x | lit | μα.s | K(p̄; c̄) | cocase { D(x̄; ᾱ) ⇒ s, … }
Consumer c ::= α | μ̃x.s | D(p̄; c̄) | case { K(x̄; ᾱ) ⇒ s, … } | ★
Command  s ::= ⟨p |ε c⟩ | prim(p̄; c̄) | f(p̄; c̄)
```

The **static focusing translation is the only entry into the IL** and is total on well-formed core terms (proved on the corpus): binding _is_ the μ̃ seam, a non-value argument is μ̃-lifted at its bind seam, and the producer-side μ exists for grammar completeness — the translation never emits it.
Five checked structural invariants: reference integrity; scope (a focused command has no free covariables); focus (a producer in argument position is a substitutable value); arity (tag-declared counts, and **a destructor frame carries exactly one trailing return continuation — checker-enforced**, while constructors' empty consumer lists and the n-ary grammar on primitives are construction-site discipline and the reserved multi-output growth point); polarity (per-form constraints, with strategy a per-cut orientation: positive fires the producer-side binder first, negative the consumer-side).
The L machine runs persistent environments over a two-region store (call-by-need value cells plus a frame region), memoizes uniformly across grades, and never panics — a runtime halt surfaces as a value outcome.
By-reference jumps are represented but never constructed; the machine yields a stuck outcome on them.

## The engines

**`theory-computads` — the cell store, overlap enumeration, completion, and tracelets.** A 2-cell is a pair of command _patterns_ with orientation, provenance, and derived per-variable metadata (variance and linearity, identified by hole name across the faces); a 3-cell is a tracelet.
The **cell-visible pattern grammar is deliberately narrower than the IL**: one command pattern variant (the polarized cut), primitives being the opaque host seam and jumps outside the fragment; the consumer pattern is a linear spine (two frame kinds, each with exactly one return continuation, two terminators), so matches are total by policy and any extension is a compile-visible tripwire at every match site.
The engines are **generic over the cell alphabet**: the `CellAlphabet` trait carries the pattern grammar, substitution, and seam vocabulary the engines quantify over, and the sequent-kernel command-pattern alphabet is its first inhabitant (`SequentAlphabet`) — the executed [[metatheory/roadmap#meta-spike-01]], a deliberate design change (the earlier monomorphism was an intentional review tripwire), with an external toy alphabet witnessing that the interface is implementable from its contract alone.
The lift preserves the standing constraints: the enumerator still emits span-level seam data, content-addressed identity stays structural, and certificate equivalence stays replay-equivalence — never type-level identity.
Substitutions are ordered maps (determinism; iteration over hash maps is denied workspace-wide); one-sided matching serves cell application and two-sided unification serves overlap superposition, with full resolution before returning (the property suite caught a triangular-unifier bug at source).
Overlap enumeration produces both critical-pair kinds — confluence (Knuth–Bendix) and composition (sequential fusion) — as a complete family, never collapsed; since patterns have no nested commands, both are root-only single-seam in practice, and the measured multi-sum family is a degenerate singleton.
**Completion is budgeted with decline-and-report**, and its honest limits are part of the contract: completed means _the worklist drained_, not that the slice is confluent; three obstruction classes are silently dropped (normalization-budget exhaustion, unorientable equal-size divergences, duplicate derived cells); the reduction order is plain node count, not a substitution-stable reduction order.
Normalization is deterministic (outermost position, then store insertion order) and records which cell fired where — **not the matched substitution** — precisely so replay must re-match; replay skolemizes the peak deterministically and re-runs both recorded paths by ground rewriting, a second check in the replayed-not-trusted sense.
A tracelet is **not self-contained**: it is meaningful only against the append-only store it was minted against.
Derived-by-completion cells are stipulated invertible by provenance, not checked.

**`theory-virtual-doctrines` — the reflection dictionary, not a second engine.** Objects are signature references, tight arrows canonical generator renamings (tight laws hold on the nose), loose arrows named relation sets, multi-ary cells derivations, restriction frame precomposition; _virtual_ is load-bearing — loose composites are not in the interface and exist only as the overlap-indexed seam family a query enumerates.
Cell equality delegates to replay-equivalence, with the identity-versus-certificate case decided by the peak-equals-join-and-replays check — a TCB-adjacent fast path whose soundness witness is the tractability record (engine side tracked as `gandr-s9q`); **it is a partial equivalence relation** — a derivation whose elaboration is stuck is not equal to itself — and derivation elaboration is deliberately narrow (identity and unary-sequential grafts only), so the multicategorical structure is representable but not yet elaborable beyond that.
The bidirectional protype checker's soundness anchor is certificate checking: an expected path or relation protype whose derivation replays.
The **directed family** is staged and strictly additive: variance-checked reflected contexts (the opposite-category involution lives on reflected signatures only; mixed variance is the dinaturality shape, not a directed variance); directed homs with the polarity-restricted eliminator under which **symmetry is underivable by construction** (the symmetry motive exists so tests can assert its refusal); finite discrete (co)ends with Fubini and co-Yoneda (and the honest boundary that end and coend coincide over discrete carriers); and the two-mode composition boundary, produced at exactly one site, routing on participating-cell invertibility: the coherence lane (never declined), the directed lane (acyclicity gate passed), or a decline carrying the variable-flow cycle as diagnostic.
The crate's stated posture: checkers and property tests — engineering evidence; theorem-grade claims are not made in Rust.

**`theory-levitation` — descriptions as data, stage 0 with stage-1 decoders.** The code universe is six first-order variants (unit, recursive occurrence, product, inline sum, graded and attributed leaf field, atom-abstraction binder); higher-order codes **cannot be represented**, so decidable code equality — what content-addressing interns on and matching compares — holds by construction.
The decoder is the single dependent capability the simply-typed checker lacked (a function from data into the universe of types), folding constructors into a right-nested finite-tag coproduct, deferring recursive unrolling to a caller-supplied self type, erasing grades and attributes on the value side, and failing only at honest boundaries (atom abstraction, applied types, codata, uninhabited).
The generic layer is three total programs — generic equality, value serialization, description serialization — deterministic as content-addressing needs, plus interning; its consumers are the surface elaborator (the producer), both engine crates, and the corpus's description mode, which is the executable proof the layer is consumed.
The bridge arity for multi-output operations is the bridge-diagram composite with the Π-layer/Σ-layer firewall from the metatheory track.

**Supporting theory crates.** `theory-graphs`: deterministic dense-index graph algorithms behind an edge-source trait whose third-party adapter is crate-private and mechanically fenced by a gate; fixed seed-independent fingerprints.
`theory-orders`: an order-maintenance total order with O(1) compare for the incremental pipeline, capacity exhaustion a typed error.
`theory-recursion`: a tiny trampoline trait keeping deep syntax trees off the native stack.
`theory-nominal-automata`: sort-tagged atoms and a monotone gensym with an atom-versus-variable sort boundary (what preserves unitary most-general unifiers); unforgeable atoms, deliberate non-`Copy`; **no automaton exists yet despite the name** — the adopted design is recorded in [[nominal-automata]]: an NDA/RNTA model inventory, a decision-procedure catalogue, orbit-finite representation choices, and the unifying name-dropping + bounded-alphabet + reduce-to-classical-NFA engineering template, with a deterministic forward runtime monitor and interleaved (non-nested) scopes as the driving applications.

## Storage — content addressing, canonicalize-before-address

The pipeline is strictly ordered — canonicalize first, hash last — with the duty split deliberately: the artifact layer performs the sort; the tree layer **refuses** anything unsorted (fail-closed validation of sortedness and uniqueness before any hash).
The kernel writes canonical bytes (admission-ordered, maximal sharing) with no hash; artifact records are keyed by admission index — a content-addressing grain, not an independently replayable unit, since segments share subterm-table entries and replay is whole-artifact; the prolly tree hashes encoded nodes (BLAKE3 throughout) and roots commit their consensus parameters, including the chunker's 85-byte parameter commitment so records cannot be replayed under incompatible parameters; the artifact identity is the hash of a manifest binding the manifest version, inner format version, chunker commitment, record count, and root hash.
The chunker is a deterministic record-safe boundary detector (cuts land only between complete records; chunk-local state, so a cumulative hash per chunk rather than a rolling window) claiming **determinism, not history-independence**; history-independence is claimed exactly once, at the artifact layer, as a **tested property** (permuted build order yields the identical identity).
The governing slogan: **integrity never substitutes validity** — a matching identity proves provenance, never validity.
Honest deficits, self-declared: the tree is two-level; canonicality is by construction, not a named theorem; no persistent backend; no anti-boundary-grinding hardening beyond hard caps.
Store-layer policies of record from the grade/store design: cells are never deduplicated or merged (cell identity is nominal); the grade is part of the content key, with no grade-erasure quotient at the store layer (a grade-irrelevant identification, if ever wanted, belongs to conversion checking); runtime grade instrumentation counts per binding, never per cell (sharing conflates per-cell counts); binder grades are derived metadata with no per-binder fields; and heap-versus-frame residency is a per-cell property derived from grade and escape, never a type former.

## The surface pipeline

```text
source text → lexer (hand-rolled byte DFA) → molder (obligation-minimizing dry run over the grammar)
  → melder (resumable first-order push machine) → commit → flat-arena CST + obligations
  → named-AST read adapter → lowering (total, with origin map) → linking → one computation
  → the typing machine + the L machine → outcome
```

The grammar is a checked **precedence-bounded grammar** over named precedence DAGs — hand-written constructor code, no codegen, in the tylr lineage [@moon-blinn-porter-omar-2025-tylr] (not BNF, PEG, or tree-sitter); a mold is a zipper into the grammar interned to a compact id.
The melder's push is total; every molded tile drives shift, reduce, or degrout (incomparable precedences complete-and-reduce with grout at bottom, guaranteeing termination); the stack is one slope of terraces, the emission log is append-only so rollback is truncation and checkpoints are cheap; **error recovery is the obligation taxonomy, not panic-and-resync**, with obligations declared in severity order so the derived ordering is the truth, and lexical ambiguity resolved by obligation minimum rather than in the lexer.
The CST's node kinds are deliberately form-name-free; declaration forms live in grammar rules at the item sort and in node-kind constants the lowerer dispatches on — there is no typed top-level declaration enum.
Vocabulary decisions of record: the universe keyword is `Type`, never `Set`; a small closed set of globally reserved keywords with fixity classes contextual; type operators are right-associative (an unparenthesized flat chain is a user-visible error); value binds are `val p = v;` and computation-result binds `run p <- c;` (`let` retired; the answer-type annotation lane `run p : F T <- c` is pending); imports are `import "URI" as name` (plain string, file scheme now, others zero-grammar-change later); the shell fragment's braced parameters are deliberately distinct from string interpolation, with subshell brackets and file-descriptor redirections in the shell context and host-escape reserved; the bare binary on a terminal is the minimal shell-REPL and the programming environment is explicit.
A **pre-lowering recursion-surface scope pass** validates each parsed item before ordinary CBPV lowering: it resolves which bare names are fix-bound (a `def rec` scope or a `rec` block group), classifies instantiation-slot residents (type arguments accepted; the direction sigils scope-checked; the named-measure, explicit-instantiation, size, cost, and tail residents **declined by name**), and enforces that an unmarked self-reference is a hard error whose finding carries the marked spelling as structured data — with a lexical-shadow arena so qualified outer bindings survive exactly and total-mode recovery so a scope error never consumes its sibling item.
The (co)recursion surface's design and its full ladder are in [[surface-language/recursion]].
Thirteen reserved-slot forms parse and decline until their semantics land — the operation member (with its multi-output result), the `rule` rewrite member in `data` and `codata`, grade-prefixed fields (numeral or ω only), grade-prefixed and parameterized observations, generalized constructor result types, per-symbol attribute slots, with-view matches, `rec` blocks, operator-fixity declarations, the copattern default arm, and the instantiation slot's five reserved residents — inventoried with their decline semantics in [[surface-language/grammar#The parse-and-decline semantics]].
The surface has `data`/`codata` declarations whose members include constructors, operations (`op`), and directed rewrite members (`rule lhs ~> rhs`); coherence (`meta`) members do not exist yet.
Lowering and elaboration to core happen entirely in the engine crate, which _drives_ the machine and implements none; its own defunctionalized worklist exists for the lowerer.
The corpus is 101 executable files in three firewalled trees under `crates/surface-corpus/examples/` (model, pathological, and parse-only surface witnesses) with directive-comment modes; the driver binary is a stub until the crates it wraps land; the render-remote crate is the renderer firewall — leaf wire types shaped as the projection of a binary session type, parsing and typing nothing.

## The runtime host

The host-effect seam lives in the checker crate as a preserved boundary over public values and the operation _name_ — never a machine continuation frame; any closure of the right shape is a handler.
Four signatures with a closed operation set (exec; filesystem read/write/glob/stat/mkdir/tempdir/cwd/ls; env get/path, read-only; proc exit); dispatch is two-level — signature first, then operation — so a same-named operation in another signature is declined, not misrouted.
Representable outcomes (a non-zero exit, a stat of a missing path) are ordinary reply values, not errors; only a fatal syscall aborts.
Early outcomes (exit, host failure) are captured out of band and replace the machine's no-handler blame after termination.
**There is no capability or sandbox model** — ambient always-resume posture, no path allowlist, no per-signature grant — and the linear-zone claim is vacuous with multi-shot resumption; stating this here is the first step of the roadmap item that prices it, and the certified-implementation criterion from the layered-game-semantics line [@oliveira-vale-mellies-shao-koenig-stefanesco-2022-layered] is the named candidate for the soundness note the crate currently lacks.
The design owed by that roadmap item is [[implementation/capability-model]]: grants as explicit capabilities threaded through handler install and resume, a grant-check point at the driver/handler boundary with denial as a third runtime outcome, the linear zone made non-vacuous by held-capability obligations, and the shell language's staging obligations priced.

## The performance discipline

The normalizer and evaluator performance program has its design basis settled ahead of the phase: a role partition over four measured implementations — Lean 4 for storage/equality/memoization/unfolding recipes, Agda's first-order machine as the machine precedent, smalltt as the elaboration-performance blueprint, Idris 2 as the erasure-arc existence proof and the source of the negative precedents — recorded in [[implementation/performance-architecture]] from the primary read [[impl-models-deep-read]].
The load-bearing commitments: **glued is two mechanisms** (term-face gluing degenerates to origin-`NodeId` caching; unfolding-face gluing is the conversion-facing half, with the quote and unfold policies as one table); **no global hash-consing in the evaluator** (sharing is per-run machine state or a boundary pass, three independent precedents); **thunk memoization and first-order machine state are non-negotiable**; **values carry blocker identity** once a solver consumes them; and the unfolding recipe set (heights, failure-cached args-first, transparency lattice, smart unfolding on case progress) is adopted with its defaults as a named policy debt.
The composition — effects plus first-class continuations plus conversion over one value domain — has no precedent and remains gandr's own bet.

## Gates and quality machinery

The workspace lint wall denies every clippy group plus ~90 restriction lints (panics, unwraps, indexing, arithmetic side effects, missing docs on private items among them), warnings-as-errors across rust and rustdoc, with test-only relaxations.
The merge wall runs, in order: toolchain pin, docs conflict/manifest/reference checks, build, clippy, the local dylint lints, doc build with warnings denied, the test suite, and whole-tree formatting; commit hooks add machine-local-path and commit-message enforcement.
Three project lints shape the codebase architecturally: single-field structs must be transparent; **gandr-owned APIs may not expose bare primitives** (the reason every crate is saturated with newtype wrappers); recursive functions need a documented termination measure (the reason the trampoline crate exists and deep recursions are explicit frame stacks).
The gates crate is a standalone analyzer library (no workspace dependencies) with eighteen commands, including the contract-grammar gate (the `# Contract`/`# Adequacy` doc discipline), the graph-boundary fence, and static task plans for the merge and push tiers.
Off the wall but defined: coverage (judged per production file, never by crate aggregate), mutation campaigns in ephemeral microVMs, five fuzz targets, the Agda gate, and the soundness-oracle and adequacy-witness suites.

## Honest limits, collected

Stated in code and easy to drop in a naive read of the design docs: completion's completed-means-drained caveat and silent obstruction drops; cell equality's partiality (stuck is not reflexive); elaboration's unary-sequential ceiling; tracelets' store-relativity; the two-level tree and by-construction canonicality; the absent capability model; the nominal crate's absent automaton; the finite-discrete coincidence of ends and coends; and the two engine crates having **no consumer outside their own tests** — which is precisely the end-to-end integration phase's job.

## Sub-documents

* [[implementation/roadmap]] — the remaining build-out in detail, the anticipation register, the engine↔metatheory statement contract, and the stale-documentation repairs owed.
1: @both
