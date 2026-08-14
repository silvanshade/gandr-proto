# Workflow: the Agda metatheory

> Read when: touching `metatheory/`.
> Agda is the sole proof vehicle.
> Design doctrine — representation, characterization, reasoning style, namespacing, the telescope — is `spec:proof-engineering.md`; this file is the lane's workflow.
> **Standing rule, whatever the task:** before recording that something does not apply, is not needed, or cannot be done, read [review.md](review.md) §"Declining is a claim too" and §"Refutations bind only with owner sign-off" — a refutation binds only with the owner's sign-off.

## What this document is, and is not

This file owns the lane's **workflow**: substrate, residual discipline, layout, flags, gates, dependency policy, solvers, opacity, commit shape, and the done-rule.
The **design doctrine** — how a structure is represented, characterized, reasoned about, and named — is the proof-engineering track, `spec:proof-engineering.md`: tree-wide rules no single module header can own.
Neither file carries the mathematical plan: that lives in the Agda module headers themselves, and the lane's scope and rationale live on its tracker epic, so a header and the code beneath it can never drift apart.

Consequently: do not restate a theorem, a substrate decision, or a scope fence here.
Record it in the module header that owns it, and reference the epic.

## Substrate: port-as-source

The metatheory is built **port-as-source** under the `Gandr.*` namespace.
There is no internal-univalence submodule, no `metatheory/upstream/`, and no `iu:check` pin gate.

The sister internal-univalence library remains a _reference_ — its structures are read, understood, and re-derived here under gandr's own naming and its own scope.
A submodule would import that library's research frontier and its release cadence into this gate, which is the wrong coupling for a tree whose purpose is to justify gandr's design.
Every ported module records its divergences in a port-delta note so the debt stays auditable.

House policy on external research artifacts applies unchanged: read and cite, never vendor, port, or depend on a companion mechanization, regardless of license.

## Build the residual now; parameterize what will not close

**Implement the remaining pieces and the suggested lemmas as they occur.** Recording an owed lemma at its site and moving on is no longer the default.

**Where a piece will not fully close, discharge what closes and make the rest parameters of a module** — so what is being assumed, and what is left to prove, appears in a signature rather than in a comment.
The form already exists in this tree: `Gandr.Shape.Graph`'s first equality layer sits in `module _ (uipᵒ : UIP Ob) (uipˡ : UIP (List Ob))`, and its second layer discharges both parameters from decidable colour equality through Hedberg.
This section extends that from h-level conditions to **any** undischarged obligation.

**And a parameter carries no presumption that it is owed.** `Gandr.Shape.Graft`'s unit laws stood in that same shape until the ingredient they were parameterized for turned out to have a formulation that closes — so re-examine a standing hypothesis when you next touch its module, and retire it in place rather than propagating the price into what the module feeds.
The surrounding discipline is unchanged: zero silent postulates, an assumption appears in the signature and never as a postulate, and a mid-proof module is a declared holey leaf gated on its own line.

**Why this is a rule and not a preference.** A deferred residual is a claim that nothing depends on it.
Three residuals taken rather than deferred, in consecutive sessions on the cell shape, each found something structural: that the edge set named half-edges and every predicate above it was wrong on exactly the shapes the cut had been added to express; that the colour involution was load-bearing for the incidence rather than a legitimacy predicate; and that a cut's ports being unordered constrains which theorem about merging is _statable_.
Each was a wrong assumption that would otherwise have been built over.

**What this is not.** It is not a licence to assume a hard result and continue over it.
An assumed hypothesis must be named, stated as a parameter of the smallest module that needs it, recorded in that module's header with what it would take to discharge, and filed on the tracker.
A hypothesis nobody can say how to discharge is a design smell, not a parameter.

## Package layout

Three packages, split by what a thing **is**, not by when it was built.

- **`Gandr.Prelude.*`** — generic type theory: list positions, insertion and concatenation relations and their views, sum and product plumbing, decidability and h-level helpers.
  Nothing that knows about an algebraic structure.
  _Test:_ would this make sense in a library that had never heard of gandr?
- **`Gandr.Foundations.*`** — the mathematics gandr is built **on**, including everything that exists in the literature prior to gandr: ∞-graphs, setoids, the category-theory tower, monoidal and monadic machinery, the circuit-algebra carrier and its operations, arenas, nerves, Reedy structure.
  _Test:_ is this gandr's own contribution, or the ground it stands on?
- **`Gandr.Metatheory.*`** — gandr's own theory: the machine, the term representation and its interpretation, the CwF instance, the judgement encodings, decidability and normalization results, and the account of how the circuit-algebra machinery combines with the rest of the language.
  _Test:_ would it be wrong to attribute this to anyone but gandr?

**Within a package, split by role rather than by topic**, on the stdlib pattern:

```text
X/Base.agda         definitions, constructors, derived operations. No theorems.
X/Properties.agda   the lemmas and theorems about Base.
X/Structure.agda    the categorical instances (below). Fold into Properties when
                    small; keep apart when it carries the interface.
X/Examples.agda     worked instances, computational pins, refutations.
```

Each split module carries the part of the old header that belongs to it.
The headers are the design record; a split that leaves a header behind has lost it.
`Migrate, never duplicate` applies throughout.

## Flags and the gate

- Per-file `OPTIONS`: `--safe --without-K --hidden-argument-puns` on every module under `metatheory/src`, enforced by the Rust `source_policy` sweep (`options-policy` subcommand; exemptions are enumerated per flag with a justification).
  The without-K mandate is binding: neither UIP nor definitional proof-irrelevance may enter through any shortcut.
  **`--hidden-argument-puns` changes what a bare `{x}` pattern means, and the tree relies on it.** In a left-hand side `f {Γ} = …` binds the implicit **named** `Γ`, not the first implicit positionally — so `match-comp {Γ}` reaches `Γ` past the colours in front of it, while `match-comp {x = Γ}` would name the colour and fail.
  Expressions are unaffected and stay positional, which is why a hidden argument supplied on the right is written out: `tail {y = w}`.
  **It is a guard rather than a quirk, and the failure it guards against is silent.** Without the flag the same pattern binds the first implicit and merely renames it, so where two implicits share a type — `∀ {m n : Nat}`, and this tree's telescopes are full of them — `g {n} = n` means `m`, typechecks, and complains nowhere; measured on both settings, not assumed.
  That is `AGENTS.md` §"Unambiguous reference — identifiers and citations" arriving inside a clause: the pattern reads as precise and is wrong.
  **And it makes the naming uniformity load-bearing instead of conventional.** A pattern that selects by name survives reordering an implicit telescope, and neither a reader nor a generator has to model that order to emit or check one — which is worth more, not less, as more of the tree is machine-written.
  A name that is not an implicit of the type is a `WrongHidingInLHS` error, so a wrong pun fails loudly where a wrong position does not.
  Write the pun and let the name do the selecting; read an existing one as named, never as positional.
- `--guardedness` is need-based and **infective**: any module that transitively imports a coinductive carrier must carry it.
  Reasoning is such a need — `Gandr.Setoid` is over the ∞-graph carrier — and a `Set`-level module takes the flag rather than reason in a second vocabulary.
  A module carrying the flag for that reason alone says so at the top of the file.
  Being flag-free is a property of a module that only _defines_; it is never a reason to reshape one.
- **Strict root / holey leaf.** `Gandr.Everything` is the strict root — everything it imports is `--safe` and green.
  Mid-proof work lives in a _declared holey leaf_: a module the root does not import, checked on its own gate line with `--expected-code UnsolvedInteractionMetas`.
  Zero silent postulates, ever.
  Add a leaf's gate line in the same change as the leaf; a line ahead of its module is a gate that cannot fail.
- `mise run agda:check` = the strict root through aifix plus the OPTIONS-policy sweep.

## Dependencies

Adding any Agda library or tool requires maintainer sign-off **first** — deliberately stricter than the Rust and TypeScript trees.

`agda-stdlib` is **admitted** (pinned v2.4, verified under Agda 2.8.0) and is imported **directly**.

An earlier revision of this file required a house facade under `Gandr.Prelude.*` and forbade direct imports.
That facade is **withdrawn**: maintaining a parallel vocabulary over a library this tree wants to lean on heavily cost more than the foundation-swap freedom it bought, and the swap it insured against is not on the arc.
What replaces it is per-module repackaging — a `private module` that re-exports the stdlib names a module actually uses, under the names that module wants (`Gandr.Arena.Structure`'s `module Fin` / `module ℕ`, `Gandr.Graph`'s `module 𝕊`).
That keeps the vocabulary local and legible at each use site without a tree-wide surface to maintain.

**The `Gandr.Prelude.*` namespace is reinstated (2026-07-28), with a different meaning, and the facade stays withdrawn.** What is withdrawn is a **facade over `agda-stdlib`**: a parallel vocabulary that re-exports stdlib names and a ban on importing stdlib directly.
Both stay withdrawn — direct stdlib imports remain mandatory, and per-module repackaging remains the way to localize vocabulary.
What `Gandr.Prelude.*` now means is a home for **gandr's own generic definitions**: list positions and their views, insertion and concatenation relations, sum and product plumbing, decidability and h-level helpers — anything that would make sense in a library that had never heard of gandr.
Today these sit wherever they were first needed, which is why the namespace is wanted; see the package layout above.
The test is ownership, not provenance: if we defined it and it knows nothing about an algebraic structure, it is prelude; if stdlib defines it, import it directly.

`agda:deps` vendors stdlib into the gitignored `metatheory/vendor/`, so `agda:check` passes `-i metatheory/vendor/agda-stdlib/src` and **a fresh checkout must run `agda:deps` before its first `agda:check`**.
It stays a separate task rather than a gate dependency so a warm tree does not re-enter the fetch path on every run.
The provisioning itself is `scripts/agda-deps.gandr`, run through the `gandr` script runner: the toolchain provisions its proof vehicle in the language the toolchain is for, and the script's `proc.exit` carries the shell's status out as the task's status.
It is idempotent by its own guard, and it is silent while it works, because a shell block captures its command's output instead of relaying it (`gandr-czio`) — the task's announcement line is what tells you a clone is under way.
The merge wall runs `agda:merge-check`, which delegates to `agda:check` when the merge range changes `metatheory/**` and otherwise exits without compiling the proof tree.
It derives the ordinary range from the merge base of `HEAD` and `main`; `GANDR_MERGE_BASE` pins an explicit base for deterministic replay.

## Solvers

Proofs reach for a solver before they are written by hand, and the reach is **on demand** — a solver is a prerequisite of the first proof that wants it, never a speculative port.

1. **Use the stdlib solver if one fits.** `Gandr.Arena.Offset` against `Data.Nat.Solver` is the exemplar.
2. **If none fits, build it first**, packaged exactly as stdlib packages its own (`Algebra.Solver.Monoid`'s `Expression` / `Normal` / `Solver` / facade split), so local and provided solvers present one interface.
3. **Until then, leave the obligation by hand with a code note naming the solver that should discharge it.** An unmarked hand proof of solver-shaped work is the drift this rule exists to prevent.

Goals are **quoted by hand** into the solver's expression syntax, as `Gandr.Arena.Offset` does.
Reflection-based tactic macros (`Tactic.RingSolver`, `Tactic.MonoidSolver`) are declined as too brittle; proof-by-reflection solvers built on `Relation.Binary.Reflection` are not macros and are the intended target.
This is the same line `spec:proof-engineering.md` §"The boundary telescope" draws: **the trusted content is an object-level function with a soundness proof, never a metaprogram**, and nothing in this tree quotes or unquotes syntax.

The direction of record for a future coherence solver, so it is not re-derived: its kernel should be **this tree's own machinery instantiated at the free structure it decides** — the normal-form function as the normalizer, the rewrite path as the emitted coherence cell — so the solver is the machinery's first consumer and a demonstrator that it computes, rather than a bespoke normalizer bolted on beside it.
No solver lands before a proof demands it.

## Opacity

`opaque` is the default for a definition whose unfolding is a cost to be controlled, with `unfolding` naming each consumer that needs it — `Gandr.Arena.Offset`'s `⊗-ix` family is the exemplar.

The placement policy, stated as three classes:

- **Never opaque — the compute surface.** Definitions whose definitional computation _is_ the design: the carrier layer, `⟦Disc⟧` and its disappearing-boundary behaviour, and any future normal-form function.
  `Gandr.Graph`'s definitions exist to be unfolded — every consumer meets them through copattern matching on `ϵ°`/`δ°`, and sealing them would sever the definitional equalities the whole tower is built from.
- **Opaque by default — derived reasoning and law surfaces off the compute path.** Combinator kits and law witnesses assembled over a primitive eliminator or over other combinators, where a use site should consume the type rather than the reduction behaviour.
- **Opaque as unfolding control**, where a deep coinductive tower makes normalization a performance concern.

**Every `opaque unfolding` block names its computation dependence** — a one-line comment saying _why_ reduction is needed at that site.
Blanket unfolding, whether whole-module or an unfocused name list, is a defect.
A module that opts out of the default states why in its header.

One dividend worth knowing, because it looks like a coincidence otherwise: an opaque definition is a rigid head the elaborator unifies spine-wise, the same way a record field is, so sealing a derived combinator can make previously-pinned implicits inferable.

## The done-rule

A metatheory milestone is done only when `agda:check` is green **and** its documentation face lands in the same motion — the module header for new structure, or the port-delta note for a ported layer.
Gate-green alone is half a milestone.

**At every session close, sweep the durable faces in this order and update each one that the session's work moved.** Not only the artifact you happened to touch: a claim left standing in a document nobody re-read is how a retracted diagnosis gets cited downstream, which is the failure this tree has already paid for twice.

1. **Module headers** — the design record for the structure they own; new content, retracted claims, and located walls all live here first.
2. **This file** — a gate change, a dependency or commit-shape change; a design rule that recurred belongs to `proof-engineering.md` at item 3 instead.
3. **The spec tracks under `spec:`** — the prose design record: `metatheory.md` and its sub-documents for substrate or design movement, `proof-engineering.md` for discipline movement, and the owning track's `roadmap.md` for build-order status.
   Say explicitly when no decision or commitment moved.
4. **The arc's own work list and decision log**, wherever the project's contributor notes keep them — next steps, status, and the lessons whose authoritative home is nonetheless item 2 or 3.
5. **The tracker** — progress as a comment on the owning item, then push.

Check the _whole_ of each artifact, not the section you edited last: status tables, build-order rows, and "as built" amendments go stale silently, and a superseded sketch must be marked superseded rather than deleted, so the amendment above it stays legible.

A green gate is also not proof of _meaning_.
State residuals honestly in the module header: a theorem that is reduced but not discharged says so, and a scope cut says what it cut and why it does not weaken the result.

Two ways a module passes the gate while proving nothing, both of which the author must close rather than the gate:

- **A parameterized module carrying assumptions must be instantiated somewhere.** Agda type-checks a module body whether or not its parameters can ever be supplied, so a module whose hypotheses are jointly unsatisfiable is green and vacuous.
  Discharge the parameters at a concrete witness in the same change, and say in the header that the witness is what makes the assumptions satisfiable.
  `Gandr.Rigid`'s `Multiset` against the natural numbers is the exemplar.
- **A predicate that nothing refutes may be vacuous.** A structure defined over a predicate proves nothing if no object fails it.
  Exhibit a counterexample alongside the examples — `Gandr.Shape.Graph`'s diamond and wheel are what stop its connectivity and wheel-freeness lemmas from being statements about an empty type.

## Commits

Keep `metatheory/**` in a **separate commit** from the Rust it mirrors — it is a distinct artifact whose history may be reorganized independently.
Repository plumbing (mise tasks, gates, this document) rides with whichever side it serves.

Commit messages follow the repository convention; `.commitlintrc.mts` is authoritative, including the canonical agent-trailer registry.
