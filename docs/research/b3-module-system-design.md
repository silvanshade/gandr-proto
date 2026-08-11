# B3 module-system design study — functors + first-class modules over the CBPV core (gandr-wvd.3)

> **Status: PROPOSAL for owner review — not a decision record.** Design study for backbone phase B3 (`gandr-wvd.3`; design spike `gandr-wvd.3.1`), produced against the fcw.11 backbone resolution (`bd comments gandr-fcw.11`) and the fcw.9 crate-schema resolution (`bd comments gandr-fcw.9`).
> Nothing here is adopted until the owner says so; recommendations are marked as such, and §13 separates genuinely open questions from them.
> **Consumers:** the B3 backbone phase itself (`gandr-wvd.3`), B2's S1/export-format reservation decisions (`gandr-wvd.2`), and B4's normalizer design (`gandr-wvd.4`).
>
> **Citation conventions.** Repo paths use the corpus alias style of the sibling research docs: `wyrd@failed-refactor:` = the canonical wyrd source tree; a bare `spec/…` path means `wyrd@failed-refactor:docs/gandr/spec/…`; `ADR-NN` = `wyrd@failed-refactor:docs/adr/00NN-*.md`.
> Literature citations use register keys `[X-N]` from `docs/research/bibliography-v2.md`, with locators quoted verbatim from the register (§14).
> No machine-local paths and no retired-tracker bead IDs appear in this file.
>
> **Ground-truth method and caveat.** §2's implementation inventory was produced by a read-only sweep of `wyrd@failed-refactor` (indexed exploration cross-checked with grep; every claim carries a file:line).
> The tree was **not** compiled or run: "tested" below means dedicated, non-ignored test functions exist in source, not that they were executed this pass.
> The tree is a mid-restoration worktree (recent history is a run of per-crate lint-gate restoration commits), and two in-tree doc notes are stale against the code that exists (§12 C4) — read file:line claims as verified, global build status as unverified.

---

## 1. Executive summary — the recommended design

1. **Ground truth first: the module layer is genuinely partially built, and the spec-survey's "adopted-unbuilt" verdict is wrong** (§2, §12 C3).
   What exists today: the ADR-42 compile-time namespace (`M.l` ⇒ flat `Var("M.l")` + native-builtin prelude, with the operator and combinator libraries landed on that seam), the **M1-lite user module-declaration layer** (`module M : #{…} { def … }` lowering to a source-ordered `Bind`-chain returning an ADR-45 record, with transparent value-only signatures and a file linker), host modules `fs`/`env`/`proc` (ADR-63), attribute-hosted manifest schemas (ADR-56), and a real production consumer (`scripts/agda-deps.gandr`).
   What is absent with zero code: functors, sealing/opacity, signature matching beyond transparent record ascription, first-class packing, implicits, imports (parse-only stub), recursive modules.
   B3 = exactly that absent list minus implicits/imports/recursion, per the owner's scope note ("B3 = functors + first-class modules", `bd show gandr-wvd`).
2. **The primitive layer mirrors CBPV one level up** (§4): structures are module **values** (ordered, telescope-shaped, type-bearing); functors are module **computations** under thunks — ADR-11's `U_r (σ₁ → F σ₂)` shape is kept as the **shape of the primitive functor former**, while its reading as an _encoding into the core_ is rejected (it would drag `+poly`'s ∀/∃ and Mω higher kinds into B3; the fcw.11 primitive-layer decision forbids the sugar route anyway).
3. **Generative-only sealing at B3, over nominal atoms** (§4.4–4.5): sealing mints fresh abstract-type atoms on the ADR-41 `gandr-nominal` substrate (the ADR-80 `DataId` precedent), instead of adding ∃-binders to the core.
   Applicative functors are deferred, with a CBPV-native re-entry criterion recorded: applicative eligibility = body thunkability/purity, decidable from the B1 effect substrate — harmonized at B6, not decided here.
4. **Paths, not expressions, project** (§4.2): module projection and functor application are restricted to paths (named modules); type components resolve statically from paths; the avoidance problem is fenced out by construction (escape ⇒ annotation-demanding error, never inferred avoidance) — the ADR-12 principality posture applied to modules.
5. **Signature matching is coercive, not subsumption** (§4.3): matching elaborates an explicit repacking coercion; kernel conversion never sees signature subtyping.
   This single choice keeps B4's conversion engine signature-free and holds the B6 modules-as-telescopes door open.
6. **First-class modules land as one new frozen-core value former** `Package σ` with always-annotated `pack`/`unpack` (§4.6); static matching only at B3 — the dynamic-match slot stays with the packages pass.
7. **B2 reservation ask: YES, four items, all cheap-now** (§6.3): reserved declaration-kind tags (AbstractType / ModuleSig / ModuleDef / FunctorDef), path-segment structured names in the export format, a third annotation slot (sealing provenance) beside the reserved erasure and modes/grades slots, and a reserved minted-atom table section.
   No S1 term-language change and no kernel typing rule is needed at B2.
8. **B4 gets five named redex/discipline classes** (§7), the sharpest being: generative functor application is atom-minting and must never be memoized or hash-consed across instantiations, and sealing is the first genuine language-level unfolding barrier the glued-NbE normalizer meets (ADR-50 D/E; [L-6]).
9. **Effectful module initialization is allowed and row-recorded** (§10.4) — the M1-lite as-built already sequences effectful members, so a pure-only posture would regress shipped behavior; effectful bodies force generativity; kernel conversion never runs them (kernel-boundary §6 C5).
10. **Landing is five rungs** (§11): baseline port → structures/paths/ascription as primitives → sealing + kernel handshake → functors → packages, each with the ADR-84 corpus treatment, sized against the existing layer.

---

## 2. Ground truth — the module layer that exists today

### 2.1 Layer map

| Layer                                                                 | Status                                                                              | Decision face                        | Primary carriers                                                              |
| --------------------------------------------------------------------- | ----------------------------------------------------------------------------------- | ------------------------------------ | ----------------------------------------------------------------------------- |
| Namespace MVP: `M.l` ⇒ flat `Var`, native builtins, prelude env       | **implemented + tested**                                                            | ADR-42                               | `gandr-pipeline` `lower`/`prelude`, `gandr-core` `Comp::Native`/`prim`        |
| Operator + combinator libraries on the ADR-42 seam                    | **implemented + tested** (40+ `NativePrim` variants — far beyond the demonstrators) | ADR-42 D7 growth                     | `gandr-core` `prim.rs`, `gandr-pipeline` `prelude.rs`                         |
| User `module M (: #{…})? { def … }` declarations ("M1-lite")          | **implemented + tested**                                                            | `spec/modules.md` §11 M1-lite banner | `gandr-pipeline` `lower.rs`, `link.rs`, `synnode.rs`; grammar `term.rs`       |
| Record rung (`#{…}` literal, `Record` types, `RecordProj`, update)    | **implemented + tested**                                                            | ADR-45 (+ ADR-53/54 activations)     | `gandr-core` `types.rs`/`syntax.rs`/`subtype.rs`, `gandr-pipeline` `lower.rs` |
| Host modules `fs`/`env`/`proc` ⇒ performs                             | **implemented + tested**                                                            | ADR-63 (rule shape ADR-58)           | `gandr-pipeline` `host.rs`, `gandr-shell`                                     |
| Manifest attribute schemas (`package`/`dependency`/`toolchain`/…)     | **implemented**; module-root attachment is a documented stand-in (§12 C4)           | ADR-56                               | `gandr-pipeline` `attributes.rs`                                              |
| `import "uri" as x;`                                                  | **parse-only stub** (no node kind, no lowering)                                     | reserved                             | grammar `term.rs`; corpus `surface/module-import.gandr`                       |
| Functors, sealing, signature matching, first-class packing, implicits | **absent** — no code, no core node, no TODO markers; grammar comment defers them    | deferred by ADR-42 D7 / ADR-11       | —                                                                             |

### 2.2 File:line inventory

**Surface syntax** (`wyrd@failed-refactor:crates/gandr-grammar/src/surface/term.rs`):

- `module_declaration` rule at `term.rs:222-235` — `module TypeIdent (: record_type_ascription)? { module_member* }`; the transparent record ascription `#{ field: Type, … }` at `term.rs:322-335`; members at `term.rs:1555`.
- `import_declaration` at `term.rs:243-255` (`import <string> as identifier ;`); `operator_declaration` (reserved) at `term.rs:266-284`; `rec_block` (reserved) at `term.rs:290-307`.
- Record literal/update `record_expression` at `term.rs:1858` (comment `term.rs:1842-1858`); record patterns at `term.rs:1271`.
- Qualified paths `M.l` reuse the ordinary `projection_expression` node — there is **no** dedicated qualified-name grammar, and no `open`/`use` form exists.
- Registration: `wyrd@failed-refactor:crates/gandr-grammar/src/surface.rs:140`; pipeline node kind `wyrd@failed-refactor:crates/gandr-pipeline/src/lower/node_kinds.rs:32`; CST readers `wyrd@failed-refactor:crates/gandr-pipeline/src/synnode.rs:1959-2103` (recognition test at `synnode.rs:4612`).

**The ADR-42 namespace (module-select)**: `wyrd@failed-refactor:crates/gandr-pipeline/src/lower.rs:3125-3161` — `projection()` emits `Value::Var("M.l")` tagged `ElabKind::ModuleSelect` (`lower.rs:3141-3151`) when `is_module_member(M, l)` holds; a known module with an unknown member is a declined `Unsupported` (`lower.rs:3170-3175`).
The registry is the const table `MODULE_BUILTINS` at `wyrd@failed-refactor:crates/gandr-pipeline/src/prelude.rs:53-90` — `(module, member, NativePrim)` rows for the modules `prim`, `list`, `record`, `string`, `regex`, `path` — driving all three faces from one table: recognition (`prelude.rs:124`, `:157`), typing `prelude_ctx()` (`prelude.rs:192-208`), evaluation `prelude_env()` (`prelude.rs:238-254`), plus the unqualified `OPERATOR_BUILTINS` table (`prelude.rs:96-110`).
A module is a string prefix, fully erased at compile time; no module object exists at runtime.

**The M1-lite user module-declaration layer**: `wyrd@failed-refactor:crates/gandr-pipeline/src/lower.rs:4411-4549` (+ helpers to `:5010`, dispatched from `item()` at `lower.rs:5062`).
A `module M : #{…} { def a = …; def b = … }` lowers to **one** `LoweredItem` whose term is a source-ordered `Bind`-chain returning a `Value::Record` — `module_term()` at `lower.rs:4955-5010` builds `Bind(a, Bind(b, Ret(Record{a,b})))`.
The contract (`lower.rs:4402-4409`): members evaluate exactly once, left-to-right; earlier binders scope over later members and the final record.
Transparent signature checking at `lower.rs:4560-4575`; dangling per-member signatures at `lower.rs:4527-4537`; `DuplicateModuleMember` at `lower.rs:4487-4500`; total-mode member recovery at `lower.rs:4793`; provenance tag `ElabKind::ModuleDeclaration` (`wyrd@failed-refactor:crates/gandr-pipeline/src/origin.rs:258`).
Qualified use of a **user** module (`M.member`, `Facts.inner.answer`) is ordinary `Comp::RecordProj` once `M` is linked — there is no separate user-module resolver (test `user_module_field_selection_is_record_projection`, `wyrd@failed-refactor:crates/gandr-pipeline/tests/acceptance.rs:771`).

**Prelude/link**: the unit of linking is a lowered source file — named `def`/`module` items followed by one final unnamed runnable item (`wyrd@failed-refactor:crates/gandr-pipeline/src/link.rs:4-5`, `:101-111`); module ascriptions survive linking as value-sorted metadata (inline tests `link.rs:545-680`).
There is **no cross-file linking** — `import` is never lowered (grep of `gandr-pipeline/src` for import handling: nothing).
REPL sessions accumulate definitions and `extern` foreign modules across submissions (`wyrd@failed-refactor:crates/gandr-pipeline/src/session.rs:312-315`); the CEK machine consults the prelude env on a `Force(Var …)` miss per ADR-42 D4 (installed at `wyrd@failed-refactor:crates/gandr-shell/src/driver.rs:141`).

**Core IR**: the `Comp` enum (`wyrd@failed-refactor:crates/gandr-core/src/syntax.rs:1345`) contains **no module construct**.
The only module-relevant nodes are `Native { prim, args }` (`syntax.rs:1668-1681`) and `RecordProj { record, label }` (`syntax.rs:1505-1511`); `Value::Record` at `syntax.rs:743`, `ValueType::Record` at `wyrd@failed-refactor:crates/gandr-core/src/types.rs:157`, width/depth subtyping at `wyrd@failed-refactor:crates/gandr-core/src/subtype.rs:333`.
No `Module`/`Functor`/`Pack`/`Seal`/`Signature` variant exists anywhere; modules are fully resolved into `Bind`/`Ret`/`Record`/`Var` before the core sees them.
The `NativePrim` registry (`wyrd@failed-refactor:crates/gandr-core/src/prim.rs:87`, `#[non_exhaustive]`) holds 40+ variants — arithmetic/comparison/boolean, list combinators and functional update, record `Get`/`Insert`/`RecordUpdate`, string ops, `RegexExtract`, path ops — i.e. the operator and combinator libraries both landed on the ADR-42 seam as designed.

**Host modules (ADR-63)**: `wyrd@failed-refactor:crates/gandr-pipeline/src/host.rs` — `HostModule` (`host.rs:249-257`), `HOST_MODULES` reserving `fs`/`env`/`proc` (`host.rs:349-365`) with member tables at `host.rs:292-343`; member calls elaborate to `Comp::Perform` against the module's host `EffectSig` (`lower.rs:1981-2035`, tag `ElabKind::HostPerform`); bare member selection declines (`host.rs:367-378`).
The same elaboration shape serves FFI extern blocks (ADR-58 D1-D2: `extern "c" from "lib" { … }` binds members as module members; calls become `perform lib.op`).

**Metadata attributes (ADR-56)**: `wyrd@failed-refactor:crates/gandr-pipeline/src/attributes.rs` — typed schemas `package`/`dependency`/`toolchain`/`name`/`license`/`authors`/`doc`/`deprecated`, all `AttrTier::Inert` (`attributes.rs:180-230`); `run()` types payloads on the iterative machine with the full finding taxonomy (`attributes.rs:259-319`).
A stale note at `attributes.rs:175-179` claims the module-root host "is not landed" and manifests validate on a top-level `def` stand-in — contradicted by the implemented `module_declaration` lowering (§12 C4).

### 2.3 What the corpus exercises, and the agda-deps consumer

- `model/29-modules.gandr` (under `wyrd@failed-refactor:crates/gandr-corpus/examples/`) is the flagship: a transparent-signature module with an **effectful member** (`def reply = #!{ printf … };`), cross-member ordering, a nested record member, and `M.field` projection, with a pinned shell-mode expectation.
- Pathological goldens: `module-duplicate-member.gandr`, `module-forward-member-reference.gandr` (left-to-right scope; forward reference rejected), `module-malformed-recovery.gandr`.
- Namespace/host coverage: `model/25-host-modules.gandr` (fs/env/path members through `let x <- …`), `model/26-env-guard-exit.gandr`, `model/28-regex-and-path-builtins.gandr`, plus the list/string/record model files; pathological `host-module-uncalled-selection.gandr`.
- `surface/module-import.gandr` is the parse-only import witness ("Parse-only; never lowered").
- **The real consumer**: `wyrd@failed-refactor:scripts/agda-deps.gandr` is an executable production script (run via `mise run agda:deps`, `wyrd@failed-refactor:mise.toml:878`) that declares `module AgdaDeps : #{ repository: String, … } { def … }`, projects `AgdaDeps.branch`/`.repository`, and drives `fs.*`/`path.*`/`string.*` host and prelude members with shell blocks — the module layer is load-bearing for the project's own tooling, not a demo. (The corpus walkthrough `model/14-agda-deps-walkthrough.gandr` is an older, module-free version of the same script — a mid-restoration drift tell, §12 C4.)
- Test inventory (none `#[ignore]`d): nine module acceptance tests in `wyrd@failed-refactor:crates/gandr-pipeline/tests/acceptance.rs` (`:470`, `:499`, `:771`, `:853`, `:888`, `:966`, `:1327`, `:1405`, `:1977`), session tests (`tests/session.rs:353`, `:931`), surface (`tests/surface.rs:213`), link inline tests, and the `Native` directed-conformance suite (`wyrd@failed-refactor:crates/gandr-core/src/conformance.rs:6693` ff.).

### 2.4 Correcting the spec-survey verdict

`docs/research/spec-survey.md` §4.3 records: "`modules` — 1ML-style | adopted-unbuilt | `spec/modules.md`", and §8 lists modules under "**Adopted-unbuilt (decided design, no code)**".
Both restate `wyrd@failed-refactor:docs/gandr/status.yml:509-516`, whose `modules` row says `stance: adopted-unbuilt` with `as_built: "…nothing built."`.
That row is stale in exactly the pattern the survey itself documented for the ADR-80/81/82 rows (spec-survey §7.1): `spec/modules.md` carries its own §11 as-built banner recording the landed MVP slice (ADR-42) **and** the landed M1-lite slice, and the survey's own authority model (spec-survey §2, rank 1: "each spec's own status / as-built banner") outranks `status.yml`. §2.2 above confirms the banner against code.

**Corrected verdict:** the module area's stance is **partial** — the ADR-42 namespace layer, the M1-lite record-module declaration layer, and their consumers (operators, combinators, host modules, FFI externs, manifest schemas) are implemented and tested; the M1+ system of `spec/modules.md` (functors, sealing, first-class packages, implicits, distribution, Mω) is adopted-unbuilt.
B3's charter is the functors + first-class-modules slice of that unbuilt remainder.

### 2.5 What ADR-11 deferred, restated

ADR-11 split the module program three ways: **modules** (the namespace/structure layer — now partially built), **thunked functors** (`U_r (σ₁ → F σ₂)`, forced by ADR-1's polarity restoration — unbuilt), and **implicits** (visibility/search/usage split three ways — unbuilt, and **out of B3 scope**; `spec/modules.md` stages them at M3).
The staged ladder in `spec/modules.md` §11 (M1 structures/functors → M2 sealing/packages → M3 implicits → M4 distribution → M5 futures → M6 Mω) remains the reference decomposition; B3 covers M1+M2 content in a different order (§11, §12 C7) and deliberately none of M3-M6.

---

## 3. Binding constraints and design lineage

### 3.1 Owner-decided constraints (not relitigated here)

From the fcw.11 resolution (`bd comments gandr-fcw.11`) and the PLAN-review amendment (`bd show gandr-wvd`, comment of 2026-07-20):

1. B3 builds its **own primitive layer**: 1ML/F-omega-style functors + first-class modules over the CBPV core — explicitly **not** sugar over dependent records (which do not exist yet).
2. Record-like structuring becomes available from B3 onward.
3. Harmonization with future dependent records (modules-as-telescopes vs permanent primitive) is **owned by B6's design pass**; B3 keeps that door open without deciding it.
4. B3 **precedes B4**, so the normalizer is designed against a term language that already has functor application + module projections; B4's charter names "module unfolding forms" in its whnf/definitional-unfolding discipline.
5. The module system is **partially built already**; B3 = functors + first-class modules.

### 3.2 Lineage the design builds on

- **ADR-1** (CBPV adjunction restored): "module functors had to become thunked computations (ADR-11)" — polarity, not taste, fixes the functor's shape.
- **ADR-11 / `spec/modules.md`**: the 1ML-on-CBPV design record — modules as values, functors as thunked computations, `Package σ ≜ ∃β̄. U_r (F σ)`, Mω transparent existentials ([H-2], [H-3], [H-6]).
- **ADR-12** (predicativity as an inference fence): the small/large distinction protects **principality**, not consistency; explicit large instantiation always allowed, guessing fuel-bounded.
  This study applies the same posture to signature inference and avoidance (§4.2, §4.6).
- **ADR-42 / ADR-45 / ADR-53 / ADR-54 / ADR-58 / ADR-63 / ADR-56**: the built substrate of §2.
- **`spec/core-ir-contract.md`** §9: `+modules` is a staged extension row ("signatures, functors as `U_r (σ₁ → F σ₂)`, implicits — `modules.md` — signature contexts"); §0 discipline governs any frozen-core former B3 adds.
- **`spec/kernel-boundary.md`** (ADR-77/78): K1-K5, the `add_decl` choke point, the S-stage subset surface, export obligations E1-E6 — the B2 handshake target (§6).
- **ADR-50** D/E: glued-NbE value domain and the two-layer unfolding control — the B4 substrate B3's forms must respect (§7); the theory layer cites [L-6].
- Literature: 1ML [H-2] for the small/large fence and core/module unification thesis; F-ing modules [H-1] for signature matching, avoidance, and the generative/applicative analysis; Mω [H-3] for transparent existentials (deferred); Alice ML [H-6], [H-7] for packages and dynamic matching (deferred to the packages pass); the Definition of SML [T-12] as the stratified-baseline reference; Levy [A-1a], [A-2] for the polarity substrate.
  No MixML/recursive-modules row exists in the citation register — recursive modules stay deferred partly for that reason (§13 Q7).

---

## 4. The primitive layer

### 4.1 Sorts and polarity — the module stratum mirrors CBPV one level up

The design introduces a **module stratum**: a second, small copy of the CBPV discipline whose "values" are structures and whose "computations" are elaborations (functor bodies, applications, effectful initialization).
It is primitive — its judgments are not desugared into core terms — but every polarity decision is inherited from the core, so nothing about evaluation order needs re-deciding.

| Module-stratum form                        | Reading                                                                    | Core analogue            |
| ------------------------------------------ | -------------------------------------------------------------------------- | ------------------------ |
| structure value `struct { d̄ }`, path `P`   | a finished module: an ordered, label-addressed telescope of components     | value `v`                |
| structure signature `Sig { D̄ }`            | classifies structure values                                                | value type `A`           |
| module returner `Mod^ε σ`                  | an elaboration that produces a `σ`-module, may perform `ε`, may mint atoms | computation type `F^ε A` |
| functor signature `(X : σ₁) →g Mod^ε σ₂`   | a generative module function                                               | computation type `A → B` |
| functor thunk `U_r ((X : σ₁) →g Mod^ε σ₂)` | a named, storable, `r`-times-applicable functor                            | value type `U_r B`       |
| `Package σ`                                | a first-class module as a **core** value (§4.6)                            | new core value former    |

**Where force/thunk sits.** A surface functor declaration `module F (X : σ₁) = body` elaborates to a module-level **thunk binding**; a surface application `F(M)` elaborates to **force-then-apply**, a module computation.
A surface structure `module M { … }` whose members are pure is a value binding; one with effectful members elaborates to a `Mod^ε` computation the file linker sequences in source order — which is **exactly the landed M1-lite semantics** (§2.2), preserved unchanged.
This is ADR-11's "thunked functors" vindicated at the right level: `U_r (σ₁ → F σ₂)` survives as the **shape of the primitive functor former**, with the grade `r` counting applications, while its other possible reading — an _encoding_ of functors as ordinary core thunks over record values — is rejected (§12 C1): the encoding needs `+poly`'s `∀X. B`/`∃X. σ` and Mω's higher kinds (`spec/modules.md` §2, §7), none of which exist in the frozen core, and the fcw.11 constraint forbids the sugar route regardless.

**What is deliberately not unified at B3:** 1ML's full collapse of core functions and functors into one construct [H-2].
B3 keeps the stratification; the collapse (if ever) is the modules-as-telescopes endgame owned by B6 (§8).

### 4.2 Paths and projection

```text
P ::= X | P.ℓ                      paths — module variables and component projections
```

- **Paths are module values.** Projection from a path is pure and static; a path is a neutral head for conversion.
  Type components are projected **only from paths** (`P.t` in a type position), which is what makes type-level projection an elaboration-time lookup rather than a computation — the classical ML path discipline ([T-12]; [H-1] §4).
- **General module expressions do not project.** `(functor-application).ℓ` and `(struct { … }).ℓ` are ill-formed; the expression must be bound to a module name first (module bindings are declarations, so this is A-normalization at the declaration layer, not a term-level restriction).
- **Value-component projection from a path** yields the component at its declared type; in core-term position a path projection `P.ℓ` of a value component elaborates to that component's item reference (today: the linked binder; post-B3.2: possibly atom-qualified — §6).
- **Relation to the built layer.** ADR-45's record projection `r.ℓ` stays a computation delivering `F A` (data records are first-class values whose projection can meet an arbitrary scrutinee).
  Module paths are the static counterpart; the two do not collide because the ADR-42/ADR-45 dispatch already distinguishes known-module heads from record values (`lower.rs:3125-3161`, ADR-45 D4).
  The M1-lite implementation detail "user-module projection = `RecordProj` on a linked record value" is **superseded** at B3.1 (§9, §11).

Candidate considered and declined: making module projection uniformly computation-shaped (mirroring ADR-45).
Declined because type components cannot be computations, so a second, static discipline is needed anyway; paths give the kernel neutral heads and give B4 a spine-local whnf story (§7 N1), and every ML-family module system that supports type components lands here.

### 4.3 Signatures — ordered telescopes, coercive matching

```text
σ ::= Sig { D̄ }                          structure signature (ORDERED telescope)
    | (X : σ₁) →g Mod^ε σ₂               generative functor signature
D ::= val ℓ : A                          value component
    | type ℓ : κ                         abstract type component (κ = arity-only kinds at B3)
    | type ℓ = A                         manifest (transparent) type component
    | module ℓ : σ                       submodule component
```

- **Signatures are ordered and dependency-respecting**: later declarations may refer to earlier ones (`type t : *; val x : t`).
  This is the M1-lite scoping rule ("later members see earlier members, never forward ones") promoted to the signature level — and it is **deliberately not** the ADR-45 record representation (canonical sorted `BTreeMap`, order-free).
  Structure signatures are telescope-shaped because their components depend on each other; that ordering is the single most load-bearing B6-door invariant (§8 I1).
- **Matching is coercive.** `σ' matches σ` is checked by a matching algorithm that permits dropping components and reordering (the standard ML-family enrichment relation, [H-1] §5), but the result of a successful match is an **explicit elaborated coercion** — a repacking structure expression — not a subsumption step.
  Consequences: (i) kernel conversion and the B4 normalizer never compare signatures (§7 N6); (ii) width/permutation flexibility lives entirely in elaboration, so a future re-reading of structures as Σ-telescopes (where reordering is not a subtyping) forecloses nothing (§8 I3); (iii) matching failures are ordinary localized diagnostics.
- Candidate considered and declined: subsumption-style signature subtyping (the ADR-45 width/depth shape lifted to modules, or an MLsub-style polar treatment [B-5], [B-9]).
  Declined because it bakes permutation/width equations into conversion — precisely what the B6 telescope door cannot absorb — and because coercive matching is what F-ing modules proved sufficient for full ML modularity [H-1].
- Transparent ascription `m : σ` checks `m`'s signature against `σ` and **strengthens**: manifest equalities from `m` survive (the `Mod-Transparent` rule of `spec/modules.md` §3).
  The landed transparent record ascription of M1-lite is the degenerate value-only case and carries over unchanged.

### 4.4 Sealing and abstract types — generative, nominal, atom-minted

- `m :> σ` (opaque ascription) checks `m ⇓ σ`, then **mints one fresh nominal atom per abstract `type ℓ : κ` in `σ`** and returns the module at `σ` with those components rebound to the atoms.
  Minting rides the ADR-41 `gandr-nominal` `Atom`/`Gensym` substrate — the same inline-identity discipline as `perform`'s op tag, `NativePrim`, and ADR-80's generative-nominal `DataId` (whose "compare the minted id before the structure" subtyping is exactly abstract-type behavior; `spec/core-ir-contract.md` §2).
- **No ∃-binders enter the core.** `spec/modules.md` §3 presents sealing as `∃β̄. σ[β̄/ᾱ]`; B3 realizes the same abstraction judgment nominally: an abstract type is an atom with no definitional unfolding, recorded in the environment with its kind and provenance.
  This avoids adding quantifiers to the frozen value-type grammar (they are `+poly`-staged) and gives the kernel a checkable story: opacity is "this atom has no δ-rule", a fact the kernel re-derives rather than trusts (K2).
- **The avoidance problem is fenced, not solved.** Avoidance arises when inference must find a signature for a module expression that does not mention an out-of-scope abstract type; principal solutions do not exist in general ([H-1] §5.4 discussion; the 1ML fence [H-2]).
  B3's posture, per ADR-12: sealing and module bindings are **declaration-granular** (no term-local module bindings except `unpack`, §4.6), and any type that would escape its atom's scope is an **error demanding an annotation** — the checker never invents an avoiding supertype.
  This keeps inferred signatures principal and errors predictable; the cost (occasional explicit ascription) is the cost ADR-12 already accepted for the core.
- Sealing at B3 attaches to module declarations (`module M :> σ { … }` and functor result signatures); sealing arbitrary inner expressions adds nothing the declaration form cannot express.

### 4.5 Generative vs applicative functors

**Recommendation: generative-only at B3.** Every functor application re-elaborates the body and re-mints the atoms of its (sealed) result — `Mod-Seal`'s "fresh identities" per application (`spec/modules.md` §3).

Rationale, and the recorded re-entry path for applicative:

1. **Effects force it.** In this language a functor body is a computation that may perform effects and mint nominal data identities (ADR-80 declares data _generatively_ even today).
   Applicative semantics — two applications of `F` to equal arguments yield **equal** abstract types — is sound only for pure bodies; OCaml's applicative functors carry exactly this caveat and Mω's reconstruction makes purity the hinge ([H-3]; [H-1] §7).
   CBPV makes the criterion _typed_ rather than folkloric: **applicative eligibility = the body is thunkable/pure**, directly readable from the B1 effect substrate's row on the body's `Mod^ε`.
   That criterion is recorded here as the reversal trigger; deciding to exploit it is B6's harmonization pass (with dependent records in view), not B3's.
2. **Mω machinery is not available.** Applicative functors need transparent existentials lifted through arrows by skolemization — `∃(b : κ_α → κ_β). ∀α. σ(α, b α)` — the one place the design genuinely needs higher kinds (`spec/modules.md` §7).
   B3 has no `κ → κ'` (kinds at B3 are arities); importing them for applicativity alone is the tail wagging the dog.
3. **Generative is the conservative end.** A generative-only system is forward-compatible with adding applicativity (sealing one construct + a lifting law, Mω's point); the reverse migration is breaking.

Consequence worth stating for the owner: at B3, `F(M).t` and `F(M).t` from two separate applications are **distinct types**.
Idioms that want shared abstract types apply the functor once and reuse the named result — the path discipline (§4.2) makes this the natural spelling anyway.

### 4.6 First-class modules — `Package σ`, `pack`/`unpack`

```text
Package σ                    new frozen-core VALUE former (positive; the one §0-discipline addition)
pack P : σ                   value intro — a path, packed at an EXPLICIT signature
unpack v : σ as X in t       computation elim — binds module var X over body t, EXPLICIT signature
```

- `Package σ` internalizes `U_r (Mod σ)` under the abstraction barrier — `spec/modules.md` §3's `Package σ ≜ ∃β̄. U_r (F σ)` with the existential replaced by the nominal-atom discipline of §4.4: unpacking mints fresh atoms for `σ`'s abstract components (generative unpack; abstraction safety).
- **Decidability fence (the 1ML lesson, applied):** the module/core boundary is annotated in **both** directions.
  `pack` always carries `σ`; `unpack` always carries `σ`; a `Package σ` is opaque to core-type inference (no rule ever guesses a module type from core-term structure).
  This is the module-layer instance of ADR-12: checking a given large thing is easy, guessing one is fenced [H-2].
- **Static matching only at B3.** `spec/modules.md` §3's `Mod-Unpack` includes a _dynamic_ signature match (the Alice ML pickling boundary, [H-6], [H-7]); its consumer is the package/build manager's fetch boundary (`spec/proposal-packages.md` §4), which is the packages pass's territory (§10.1).
  B3's `unpack` is checked statically at elaboration and re-checked by the kernel; the dynamic-match slot is reserved, unbuilt.
- Grades: `Package` carries the thunk grade of its payload (`ω` default, `1` for single-use packages — the module-level linearity `spec/modules.md` §3 motivates); see §13 Q4 for how much of the grade story lands at B3 vs the modes/grades tail slot.
- Frozen-core impact: `Package` + `pack`/`unpack` are the **only** core grammar additions B3 makes, and they enter through the `core-ir-contract.md` §0 discipline (ADR + contract + dictionary in lock-step), exactly as ADR-45 entered the record former.

### 4.7 Elaboration summary table

| Surface                                 | Elaborates to                                                                       |
| --------------------------------------- | ----------------------------------------------------------------------------------- |
| `module M { d̄ }` (pure members)         | module value binding; members a telescope                                           |
| `module M { d̄ }` (effectful members)    | `Mod^ε` computation; linker sequences in source order (M1-lite semantics preserved) |
| `module M : σ { … }`                    | transparent ascription + strengthening                                              |
| `module M :> σ { … }`                   | sealing; atoms minted at declaration                                                |
| `module F (X : σ₁) (:> σ₂)? = body`     | thunk of a generative functor; result sealing per application                       |
| `F(M)` (paths only)                     | force + apply: a `Mod^ε` computation                                                |
| `P.ℓ` (type position)                   | static component lookup                                                             |
| `P.ℓ` (term position)                   | component reference (pure)                                                          |
| `pack P : σ` / `unpack v : σ as X in t` | core value intro / core computation elim of `Package σ`                             |

---

## 5. Typing architecture — what "1ML-style" buys when not elaborating into F-omega

The owner constraint says the rules are **primitive in the core**, so 1ML/F-ing function as _design references_, not as elaboration targets.
Precisely what transfers:

| 1ML / F-ing idea                                                     | Transfers to primitive CBPV rules?                                                                                                                                                 |
| -------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Small/large distinction as the decidability fence [H-2]              | **YES** — already adopted as ADR-12; realized here as the always-annotated `pack`/`unpack` boundary and no-guessed-signatures posture (§4.4, §4.6)                                 |
| "Checking given impredicativity is easy; guessing is hard" [H-2]     | **YES** — verbatim ADR-12; module types are checked, never inferred across the stratum boundary                                                                                    |
| Signature matching as enrichment with explicit coercions [H-1]       | **YES** — §4.3's coercive matching is F-ing's matching with the Fω-term output replaced by a module-stratum repacking                                                              |
| Bidirectional discipline extended to modules (`m ⇕ σ`)               | **YES** — `spec/modules.md` §2 already states the module judgment in the core's bidirectional form; the primitive rules keep that shape                                            |
| Generative/applicative as sealing inside/outside the λ [H-1], [H-3]  | **PARTIALLY** — the analysis transfers (it is how §4.5 derives "effectful ⇒ generative"); the mechanism (skolemized transparent existentials) does not, deferred with higher kinds |
| Elaboration into Fω types (semantic signatures as `∃/∀` types) [H-1] | **NO** — no `+poly` in the frozen core; replaced by nominal atoms + environment-recorded abstraction (§4.4)                                                                        |
| Mω transparent existentials / lifting laws [H-3]                     | **NO** at B3 — the applicative re-entry path, owned by B6+                                                                                                                         |
| 1ML's single-language collapse (functors = functions) [H-2]          | **NO** at B3 — stratification kept; collapse is telescope-era territory (§8)                                                                                                       |
| Avoidance via existential quantification over escaping types         | **NO** — replaced by the annotation-demanding fence (§4.4), the principality posture of ADR-12                                                                                     |

What the CBPV substrate adds that the ML-family references lack: polarity **types** the module layer's operational story (structure = value, functor application = computation, storable functor = thunk — ADR-1/ADR-11; [A-1a], [A-2]), grades count functor applicability and package use (`spec/modules.md` §10; [F-7]), and effect rows make the generative/applicative purity hinge checkable rather than conventional (§4.5).

---

## 6. Kernel impact — the B2 handshake

### 6.1 Where B3 meets the kernel

B2 (`gandr-wvd.2`) lands `kernel-core` at S1: term language for the pure polarized fragment, declaration vocabulary (`Def`, `Axiom`), environment, conversion, the K3 `add_decl` choke point, and the K5 export writer (`spec/kernel-boundary.md` §3, §7; fcw.9 rename table).
The fcw.11 exit-gate floor requires from B2 onward that each phase's corpus "exports cleanly and kernel `add_decl` accepts it" — so B3's module corpus must cross the choke point.

### 6.2 The posture: modules mostly flatten; atoms and packages do not

Two candidate postures for module-aware kernel checking:

- **(a) Full module vocabulary in the kernel** — `ModuleDef`/`FunctorDef` declarations, module terms, signature matching in the kernel.
  Maximal trust coverage, maximal TCB growth; K4 would have to police signature matching and functor instantiation inside the wall.
- **(b) Elaborator flattens; kernel checks the residue** — structures export as member `Def`s with **structured (path-segment) names** plus signature metadata; functor **applications are instantiated by the elaborator before export** (each generative instantiation exports its minted atoms + member `Def`s); the kernel's new obligations are only: an `AbstractType` declaration form (a minted atom with its arity, no δ-rule — what makes sealing _kernel-checkable_ rather than elaborator-promised), and (at B3.4) the `Package` former with `pack`/`unpack` typing.

**Recommendation: (b) at B3**, with (a)'s declaration tags reserved so graduating functor definitions into the kernel later (kernel subset-surface growth is a standing per-phase obligation, fcw.11) is additive.
Rationale: K-disciplines favor small closed growth (K1/K3); the replay checker (B9) prices every kernel form twice; and posture (b) already closes the soundness gap that matters — **opacity is re-derived by the kernel** (an atom admitted with no unfolding cannot be peeled by a malicious export), rather than trusted from the elaborator.
The cost — functor _bodies_ are not independently kernel-certified at B3, only their instantiations — is the honest trade; §13 Q1 puts it to the owner.

### 6.3 The reservation ask (what B2 must do NOW) — explicit

**Yes, B2 should reserve; four items, all cheap at B2 and expensive to retrofit:**

- **R1 — declaration-kind tag space.** Reserve export-format tags for `AbstractType`, `ModuleSig`, `ModuleDef`, `FunctorDef` in the K5 declaration vocabulary.
  E4/E5 (validating reader + versioned refusal) already make unknown tags safe; reserving the numbers now means B3 (and a later functor-graduation rung) never renumbers a shipped format.
- **R2 — structured declaration names.** The S1 export's name field should be a **path-segment list**, not a flat string, from v0.
  This is the cheapest/most-expensive item of the four: the ADR-42 layer's flat `"M.l"` strings (§2.2) must not become the export identity, or every future module-aware consumer (replay, packages ascription pointers per ADR-56, B6 telescopes) parses names out of strings forever.
- **R3 — a third annotation slot: sealing provenance.** Beside the two slots B2 already reserves (erasure; modes/grades — fcw.11), reserve one per-`Def` slot recording sealed-component provenance (owning module, owning atom set).
  Same mechanism, same cheap-now/expensive-later call the owner already made twice.
- **R4 — a minted-atom table section.** Reserve an export section for abstract-type atoms in admission order, so replay **re-mints deterministically** and freshness is a checkable property (K4 applied to generativity), not an imported claim.
  This also keeps E1 (self-contained) honest once sealed declarations exist.

**What B2 need NOT do:** no S1 term-language change; no `Package` former (B3 adds it through the §0 discipline at B3.4); no kernel signature matching ever, if §4.3's coercive posture holds (matching output is ordinary terms the kernel re-checks); no commitment now on functor-body kernel checking (R1 keeps both §6.2 postures open).

---

## 7. B4 anticipation — normalization-relevant forms B3 introduces

B4's charter (fcw.11) adapts the ADR-50 D/E glued-NbE, hash-consing normalizer to the L machine, with a whnf/definitional-unfolding discipline "covering module unfolding forms".
B3 hands B4 exactly these forms:

- **N1 — structure projection.** `struct { …, ℓ = v, … }.ℓ ▷ v`; a projection whose head is a neutral path stays neutral.
  Discipline: paths demand **spine-local** whnf only — normalize the head to structure form, never the sibling components (the glued representation's laziness is load-bearing here; [A-37] for NbE over the CBPV/polarized core).
- **N2 — functor β under generativity.** `(force F)(M) ▷ body[M/X]` **plus atom minting** for the sealed result.
  Generative application is therefore _not_ a confluent pure rewrite: two applications of the same functor to the same argument are **not** convertible, and the normalizer must (i) treat instantiation as a stateful step keyed by the minted atoms, and (ii) **never memoize or hash-cons across instantiations** — the content-addressed sharing of ADR-50 B must include minted atoms in identity or two distinct instantiations would silently alias.
  This is the single sharpest constraint B3 exports to B4; it should appear in B4's charter checklist verbatim.
- **N3 — sealing as an unfolding barrier.** A sealed component has **no δ-rule**; the glued value's top-level-unfolded face stops at the seal.
  This is the first genuine language-level unfolding boundary the normalizer meets, and it lands in ADR-50 E's _engine_ layer (transparency/reducibility control); the _theory_ layer — declared, scoped unfolding with extension-type semantics [L-6] — remains reserved for the B6+ dependent-core era (the wyrd-era "evidence layer", a concept superseded by the reboot backbone), and B3's sealing is designed not to preempt it (opacity here is total, not scoped).
- **N4 — transparent ascription strengthens.** `type ℓ = A` components contribute definitional equations (δ-rules) to the environment; strengthening (§4.3) is the mechanism by which sealed-then-transparently-viewed modules regain equations.
  The normalizer's definitional environment must be per-scope, since the same atom may be manifest in one scope (inside the sealed module) and opaque outside.
- **N5 — `unpack`∘`pack`.** Reduces, but generatively (fresh atoms per unpack, §4.6) — the N2 discipline applies; `Package` values are otherwise inert for conversion, and kernel/normalizer conversion never runs initialization effects (kernel-boundary §6 C5 stands unmodified).
- **N6 — coercions are terms, not conversion.** Because matching is coercive (§4.3), the normalizer never compares signatures and never needs permutation/width equations in conversion; a matching coercion normalizes like any other structure expression.

---

## 8. The B6 door — invariants kept, forecloser list

B6 owns the modules-as-telescopes vs permanent-primitive decision.
B3 maintains these invariants so **both** outcomes stay reachable:

- **I1 — signatures are ordered telescopes** (§4.3), never canonical-sorted maps.
  A telescope re-reading `Sig { type t; val x : t } ≈ Σ(t : Type). …` is representation-compatible; a sorted-map representation would not be.
- **I2 — abstract types are nominal atoms** (§4.4).
  Under telescopes, an atom becomes a Σ-bound type variable (the ADR-41 substrate already brokers atom↔de-Bruijn bridging by design); under permanent-primitive it simply stays an atom.
- **I3 — matching is coercive** (§4.3): no width/permutation subtyping enters conversion, which a Σ-telescope reading could not absorb.
- **I4 — the path discipline** (§4.2): paths map to telescope projections one-for-one.
- **I5 — structured names in exports** (§6.3 R2): no flat-string identity anywhere new.

What **would** foreclose the telescope future, recorded as anti-commitments: baking signature width/permutation equations into kernel or normalizer conversion; making flat mangled names the export identity; erasing structure boundaries in exports (flattening without the R3 provenance slot); making `Package` eliminable by anything but `unpack` (an implicit coercion Package→structure would commit modules to being second-class-convertible values).
Conversely, nothing here _decides for_ telescopes: the stratum stays primitive, and ADR-81's `Σ(x:A). B` (already Live, small-head-only) is untouched by B3.

---

## 9. Record-like structuring at B3

What the owner's "record-like structuring available from B3 onward" cashes out to:

- **ADR-45 records stay the data former, unchanged**: canonical sorted representation, width/depth subtyping by subsumption, direction-polymorphic `#{…}` literals, `RecordProj`, functional update (ADR-53), record patterns (ADR-54).
  Records never contain types; their order-free canonical form is correct _for data_.
- **Structures are the organization former** B3 adds: ordered, type-bearing, sealable, functor-composable, coercively matched.
  The two formers share surface affinity (`module M : #{…}` ascription survives as the value-only degenerate signature) but deliberately different metatheory (§4.3), and the ADR-45 D4 dispatch (module-member vs record-field projection) already keeps their surfaces from colliding.
- **The M1-lite lowering is superseded, its semantics preserved.** At B3.1, `module` declarations stop lowering to "one item whose value is a record" and become module-stratum items — necessary because structures now carry type components and atoms, which `Value::Record` cannot.
  The _observable_ M1-lite contract — members elaborate exactly once, in source order, left-to-right scoping, no forward references, transparent ascription checks the result — is preserved verbatim, so the existing corpus goldens (`model/29-modules.gandr`, the duplicate/forward/malformed pathologicals, `scripts/agda-deps.gandr`) carry over with unchanged expectations.
- **The boundary against future dependent records**: dependent records (B6) will be a _data_ former (records whose field types depend on earlier fields); B3's structures are _not_ that — they are second-class-by-default program structure with a first-class escape hatch (`Package`).
  If B6 unifies them, §8's invariants are what make the unification a refinement.

---

## 10. Interactions

### 10.1 The package boundary (what B3 leaves alone)

`spec/proposal-packages.md` owns the build/distribution sense of "package" (its §0 two-senses table); B3 owns only the language sense (`Package σ`).
B3 **provides** what the packages pass consumes: signatures with stable export identities (§6.3 R2), transparent/opaque ascription, and the `Package` former its §1 import-lowering targets.
B3 **deliberately does not build**: the import surface (`import "uri" as x` stays the parse-only stub of §2.2 — its lowering is the packages pass's §1), content-addressed resolution/lockfiles (§2 there), the manifest wiring beyond §10.3 below, **dynamic** signature matching at the fetch boundary (§4 there; Alice ML's contribution [H-6], [H-7]), and any distribution machinery (`offer`/`take` shared-session tickets, located modules — `spec/modules.md` §5, M4; [D-6], [G-3]).
One correction the packages pass should absorb: its MVP is stated as "gated on the M1 module layer"; B3 is the actual provider of that gate (§12 C6).

### 10.2 Host modules and the reserved-namespace gate (ADR-63, ADR-58)

Today `fs`/`env`/`proc` (and the prelude modules `prim`/`list`/`record`/`string`/`regex`/`path`) are reserved via a **syntactic** gate — a name-table check in projection position, deliberately not scope-aware (ADR-42 D2, ADR-63 D1), with ADR-63's own reversal trigger pointing at "scope-aware resolution when the module/import layer lands".
**Recommendation: B3.1 exercises that trigger.** The prelude, host, and extern module tables become genuine bindings in an outermost prelude scope; resolution becomes ordinary scoped name lookup; a user declaration shadows a builtin (preserving ADR-63's extern-shadow rule and closing its "real scripts collide with `env`" hazard).
Elaboration targets are untouched: prelude members still elaborate to `native`, host/extern members to `perform` (ADR-58's shape) — only _recognition_ graduates from string tables to scope.

### 10.3 Metadata attributes (ADR-56)

B3's module declaration becomes the real manifest attachment root ADR-56 designed against, retiring the top-level-`def` stand-in documented at `attributes.rs:175-179` (§2.2).
The manifest's growth-tier `exposes(σ)` field gains a real referent: a signature with a stable export identity (§6.3 R1/R2).
Whether the exposed signature is transparent-advertisement or identity-bearing seal remains the packages pass's open question (proposal-packages §7.11 Q2); B3 supplies both ascription forms and takes no position.

### 10.4 Effects — can module initialization perform effects?

**Recommendation: yes, row-recorded.**

- The as-built layer already does it: M1-lite sequences member computations in source order, its ascription deliberately does "not impose an empty effect row on effectful members" (`spec/modules.md` §11), and `model/29-modules.gandr` ships an effectful member with a pinned expectation.
  A pure-only B3 would _regress_ shipped, corpus-pinned behavior.
- The typed form: a structure with effectful members elaborates at `Mod^ε σ`; the file linker (today's source-order semantics) is the sequencing point; a functor whose body's row is non-empty is generative **by construction** (§4.5) — the effect substrate arriving from B1 makes this a checked property, not a convention.
- The kernel is untouched: initialization effects are runtime/elaborator territory; kernel conversion never evaluates effectful computations (kernel-boundary §6 C5), and exported declarations are the _results_ of elaboration.
- The one genuine hazard — initialization order becoming load-bearing across modules — is contained at B3 because linking stays single-file/source-ordered; the packages pass, which introduces cross-unit graphs, inherits the question with its DAG (proposal-packages §5) and should decide init-order policy there.

---

## 11. Landing plan

Five rungs, each behind the fcw.11 exit-gate floor (L differential green for the delta; corpus exports through `add_decl` from B2 on; docs + hygiene + tracker rungs).
Corpus treatment per `docs/workflow/corpus.md` (ADR-84): every surfaced rung lands literate model examples + runnable pathological coverage + harness assertions + coverage-map registration in the same change; new syntax lands a parse-gated `surface/` witness first, promoted at its semantics-graduation change.

| Rung     | Contents                                                                                                                                                                                                                    | Proves it (corpus plan)                                                                                                                                                                           | Size |
| -------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---- |
| **B3.0** | Baseline: confirm the §2 layer on the ported B1 core (namespace, M1-lite, host modules, prelude); port the existing module corpus + `agda-deps.gandr`; fix the two stale in-tree notes (§12 C4)                             | existing goldens green unchanged (`29-modules`, module pathologicals, `25-host-modules`)                                                                                                          | S    |
| **B3.1** | Structures/paths/signatures as primitives: module-stratum items replace record lowering (§9); path discipline; transparent ascription + strengthening; coercive matching; namespace graduation to scoped resolution (§10.2) | model: nested modules, type components, ascription; pathological: matching failures (missing/ill-typed/reordered components), forward-ref and duplicate goldens carried, shadowed-builtin witness | M    |
| **B3.2** | Sealing + abstract atoms; kernel handshake: `AbstractType` through `add_decl`, R3/R4 slots filled, export/replay of sealed decls                                                                                            | model: sealed counter/set modules ("this abstract type was born here"); pathological: abstraction-leak attempts, escape-demands-annotation (§4.4), export round-trip assertions                   | M    |
| **B3.3** | Generative functors: thunked functor bindings, application, result sealing per application; instantiation-at-export posture (§6.2 b)                                                                                        | model: classic `MkSet(IntOrd)` pair; pathological: generative-distinctness golden (two applications ⇒ distinct types), effectful-body functor, grade-exhausted application if Q4 lands            | L    |
| **B3.4** | `Package σ` + `pack`/`unpack` (frozen-core §0-discipline change: ADR + contract + dictionary lock-step, ADR-45 precedent)                                                                                                   | model: module-as-value dispatch (choose an implementation at runtime); pathological: unpack-mismatch static error, double-use of a `U_1` package (if Q4), atom-freshness-per-unpack golden        | M    |

Sizing against the existing layer: the built module machinery is elaboration-plus-tables (~600 lines of module lowering, the prelude/host const tables, grammar rules — §2.2) with **zero** typing-judgment or core-IR footprint.
B3 is categorically bigger: the first new judgment stratum since the A3 effect layer, one frozen-core former, and the first kernel-vocabulary growth past S1.
The shape most comparable in the record is the A3 effects landing (new judgment family + new declaration forms + machine arms), not the record rung.

---

## 12. Source-conflict register

Flagged, not smoothed, per the survey discipline.

- **C1 — ADR-11/`spec/modules.md` "functors are `U_r (σ₁ → F σ₂)`" vs fcw.11 "own primitive layer, not sugar".** The spec's table reads naturally as _elaboration into the core_ (structures as labeled products, signatures as value types with existential packaging) — which presupposes `+poly`'s `∀/∃` and, for applicative functors, Mω higher kinds, none of which exist.
  Resolution adopted here (§4.1): keep the formula as the **shape** of the primitive functor former, reject the encoding reading; the conflict is real and the owner should know the spec's §2 signature grammar (`∀X. σ | ∃X. σ`) is _not_ what B3 builds.
- **C2 — `spec/core-ir-contract.md` §9 vs `spec/modules.md` §11.** The contract says the MVP slice "takes no … record-valued modules" and `+modules` "remains future work", while `modules.md` §11's banner records the M1-lite _record-module_ slice as landed; the two were committed minutes apart.
  Technically reconcilable (M1-lite adds no core node, so the frozen contract had nothing to record), but a reader of the contract alone will under-count the built layer — the gandr corpus port should add an M1-lite note at the contract's §9 `+modules` row.
- **C3 — the spec-survey/`status.yml` stale verdict.** §2.4 above; corrected to **partial**.
  Same drift class as spec-survey §7.1's own findings; the corrected stance should reach the ported `status.yml` (or its gandr successor) with B3.0.
- **C4 — in-tree code-vs-doc drift (mid-restoration tells).** The grammar comment `term.rs:214` still says module-declaration "lowering deferred"; `attributes.rs:175-179` says the module root "is not landed"; the corpus walkthrough `model/14-agda-deps-walkthrough.gandr` is an older module-free version of the real `scripts/agda-deps.gandr`.
  All three contradict implemented code in the same tree; B3.0 fixes the notes and re-syncs the walkthrough.
- **C5 — ADR-63's syntactic gate vs scope-aware resolution.** Not a contradiction — ADR-63 names this exact reversal trigger — but B3.1 exercises it, and ADR-63's "reserved names a program cannot rebind" consequence is thereby _narrowed_ (shadowing becomes possible by design, per its own D1 extern-shadow precedent).
- **C6 — proposal-packages gating.** Its MVP is "gated on the M1 module layer (`modules.md` §11)"; M1 as staged there never landed and B3 re-cuts the staging.
  The packages pass should re-point its gate at B3's rungs (signatures/ascription at B3.1-3.2; `Package` at B3.4).
- **C7 — staging order vs `spec/modules.md` §11.** The spec stages sealing at M2 _after_ full M1 structures/functors; B3 lands sealing (B3.2) _before_ functors (B3.3) because the kernel handshake needs `AbstractType` early and functor semantics depend on sealing (§4.5).
  A deliberate reorder, not an oversight.

---

## 13. Open questions for the owner

Separated from the recommendations (§4-§11 state those); each is a genuine fork the owner should call.

- **Q1 — kernel posture for functors** (§6.2): accept instance-only export at B3 (recommended), or require `FunctorDef` bodies kernel-checked from the start?
  R1's reserved tags keep both open; the trade is TCB size + replay cost vs functor-body certification.
- **Q2 — effectful module initialization** (§10.4): confirm allow-with-row (recommended; pure-only would regress `model/29-modules.gandr` and the agda-deps script), or restrict at B3 and re-admit later?
- **Q3 — applicative functors** (§4.5): confirm deferral to B6's harmonization pass with the recorded purity-gated re-entry criterion, or pull a purity-restricted applicative form into B3?
- **Q4 — grades on the module layer** (§4.6): carry `U_r` on functor thunks and packages at B3 (the `spec/modules.md` posture; the `Grade` carrier exists), or defer all module-layer grading to the modes/grades tail slot B2 already reserves?
- **Q5 — namespace graduation timing** (§10.2): scope-aware resolution at B3.1 (recommended), or keep the syntactic gate until the packages pass to avoid touching three consumers (prelude/host/extern) mid-backbone?
- **Q6 — implicits**: confirm they are fully out of B3 (this study assumes so per the fcw.11 phrasing; `spec/modules.md` stages them at M3, and [H-4]/[H-5] remain the references when they return).
- **Q7 — recursive modules**: stay deferred (recommended — no register row exists for the MixML/recursive-modules literature, so pursuing them first requires a bibliography hydration pass), or name a B-phase for them now?
- **Q8 — named signature bindings**: B3 surfaces signatures structurally (inline `σ` only) with `signature S = σ` as later sugar (recommended), or land named signature declarations at B3.1?

---

## 14. References (register keys, locators verbatim from `docs/research/bibliography-v2.md`)

| Key  | Citation                                                                                                         | Locator                                                                         |
| ---- | ---------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| A-1a | Levy — _Call-By-Push-Value: A Subsuming Paradigm_ (TLCA 1999)                                                    | doi:10.1007/3-540-48959-2_17                                                    |
| A-2  | Levy — _Call-By-Push-Value_ (PhD thesis, Queen Mary, University of London, 2001)                                 | <https://pblevy.github.io/papers/thesisqmwphd.pdf> (QMRO handle 123456789/4742) |
| A-37 | Abel & Sattler (2019, PPDP) — _Normalization by Evaluation for Call-By-Push-Value and Polarized Lambda Calculus_ | doi:10.1145/3354166.3354168                                                     |
| B-5  | Dolan & Mycroft (2017, POPL) — _Polymorphism, Subtyping, and Type Inference in MLsub_                            | doi:10.1145/3009837.3009882                                                     |
| B-9  | Dolan — _Algebraic Subtyping_ (PhD thesis, University of Cambridge, 2016)                                        | <https://www.cs.tufts.edu/~nr/cs257/archive/stephen-dolan/thesis.pdf>           |
| D-6  | Balzer & Pfenning (2017, ICFP) — _Manifest Sharing with Session Types_                                           | doi:10.1145/3110281                                                             |
| F-7  | Torczon et al. (2024, OOPSLA) — _Effects and Coeffects in Call-by-Push-Value_                                    | doi:10.1145/3689750                                                             |
| G-3  | Murphy, Crary & Harper (2007, TGC) — _Type-Safe Distributed Programming with ML5_                                | doi:10.1007/978-3-540-78663-4_9                                                 |
| H-1  | Rossberg, Russo & Dreyer (2014, JFP) — _F-ing Modules_                                                           | doi:10.1017/s0956796814000264                                                   |
| H-2  | Rossberg (2015, ICFP; JFP 2018) — _1ML — Core and Modules United_                                                | doi:10.1017/s0956796818000205                                                   |
| H-3  | Blaudeau, Radanne & Rémy (2024) — _Fulfilling OCaml Modules with Transparency_ (Mω)                              | doi:10.1145/3649818                                                             |
| H-4  | White, Bour & Yallop (2014, ML Workshop) — _Modular Implicits_                                                   | doi:10.4204/eptcs.198.2                                                         |
| H-5  | Dreyer, Harper, Chakravarty & Keller (2007, POPL) — _Modular Type Classes_                                       | doi:10.1145/1190216.1190229                                                     |
| H-6  | Rossberg (PhD, Saarland, 2007) — _Typed Open Programming_                                                        | <https://publikationen.sulb.uni-saarland.de/handle/20.500.11880/25934>          |
| H-7  | Rossberg, Le Botlan, Tack, Brunklaus & Smolka (2004, TFP) — _Alice Through the Looking Glass_                    | doi:10.2307/j.ctv36xw0k5.9                                                      |
| L-6  | Gratzer, Sterling, Angiuli, Coquand & Birkedal — _Controlling Unfolding in Type Theory_                          | arXiv:2210.05420                                                                |
| T-12 | Milner, Tofte, Harper & MacQueen — _The Definition of Standard ML (Revised)_ (MIT Press, 1997)                   | no DOI (MIT Press book; ISBN 0-262-63181-4)                                     |
