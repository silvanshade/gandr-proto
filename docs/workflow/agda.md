# Workflow: the Agda metatheory

> Read when: touching `metatheory/`.
> Agda is the sole proof vehicle.

## What this document is, and is not

This file owns the **workflow**: layout, flags, gates, dependency policy, commit shape, and the done-rule.
It deliberately does **not** carry doctrine — the mathematical plan lives in the Agda module headers themselves, and the lane's scope and rationale live on its tracker epic.
That split is a decision, not an omission: the reboot has no separate design document, so a header and the code beneath it can never drift apart, and there is exactly one place to read for "why is this module shaped this way".

Consequently: do not restate a theorem, a substrate decision, or a scope fence here.
Record it in the module header that owns it, and reference the epic.

## Substrate: port-as-source

The metatheory is built **port-as-source** under the `Gandr.*` namespace.
There is no internal-univalence submodule, no `metatheory/upstream/`, and no `iu:check` pin gate.

The sister internal-univalence library remains a _reference_ — its structures are read, understood, and re-derived here under gandr's own naming and its own scope.
A submodule would import that library's research frontier and its release cadence into this gate, which is the wrong coupling for a tree whose purpose is to justify gandr's design.
Every ported module records its divergences in a port-delta note so the debt stays auditable.

House policy on external research artifacts applies unchanged: read and cite, never vendor, port, or depend on a companion mechanization, regardless of license.

## House style

Purpose-built records over raw sigma types; explicit record instances; record types imported at file top with projections opened at the use site; `hiding`/`using` listing one name per line; no `private variable` blocks; copattern style for record values; eager arrow-leading line breaks; the flat proof-term ladder rather than deep `where` nesting; and **every definition carries a comment**.

Two disciplines are load-bearing rather than cosmetic, and both exist to keep structures computing under `--without-K`:

* **Witness syntax stays first-order and constructor-headed.** A defined function must never appear in a matchable index.
  Where an operation would otherwise enter an index, speak it through the inductive _graph_ of that operation as a witness relation.
* **No identity-shaped constructor repeats a frame variable across its result indices.** Identity and diagonal cases are derived, never adjoined as constructor shapes.

**Agda-DbC stance.** The type is the contract; do not port the Rust `# Contract` comment block.
Load-bearing insight lives in the module header and the code cites it.
Mandatory marks are reserved for genuine trust-story exceptions: signature parameters standing for assumptions, and any future with-K or unsafe island.

## Flags and the gate

* Per-file `OPTIONS`: `--safe --without-K --hidden-argument-puns` on every module under `metatheory/src`, enforced by the Rust `source_policy` sweep (`options-policy` subcommand; exemptions are enumerated per flag with a justification).
  The without-K mandate is binding: neither UIP nor definitional proof-irrelevance may enter through any shortcut.
* `--guardedness` is need-based and **infective**: any module that transitively imports a coinductive carrier must carry it.
* **Strict root / holey leaf.** `Gandr.Everything` is the strict root — everything it imports is `--safe` and green.
  Mid-proof work lives in a _declared holey leaf_: a module the root does not import, checked on its own gate line with `--expected-code UnsolvedInteractionMetas`.
  Zero silent postulates, ever.
  Add a leaf's gate line in the same change as the leaf; a line ahead of its module is a gate that cannot fail.
* `mise run agda:check` = the strict root through aifix plus the OPTIONS-policy sweep.

## Dependencies

Adding any Agda library or tool requires maintainer sign-off **first** — deliberately stricter than the Rust and TypeScript trees.

`agda-stdlib` is **admitted** (pinned v2.4, verified under Agda 2.8.0) and is imported **directly**.

An earlier revision of this file required a house facade under `Gandr.Prelude.*` and forbade direct imports.
That facade is **withdrawn**: maintaining a parallel vocabulary over a library this tree wants to lean on heavily cost more than the foundation-swap freedom it bought, and the swap it insured against is not on the arc.
What replaces it is per-module repackaging — a `private module` that re-exports the stdlib names a module actually uses, under the names that module wants (`Gandr.Arena.Structure`'s `module Fin` / `module ℕ`, `Gandr.Graph`'s `module 𝕊`).
That keeps the vocabulary local and legible at each use site without a tree-wide surface to maintain.

`agda:deps` vendors stdlib into the gitignored `metatheory/vendor/`, so `agda:check` passes `-i metatheory/vendor/agda-stdlib/src` and **a fresh checkout must run `agda:deps` before its first `agda:check`**.
It stays a separate task rather than a gate dependency so a warm tree does not re-enter the fetch path on every run.

## Solvers

Proofs reach for a solver before they are written by hand, and the reach is **on demand** — a solver is a prerequisite of the first proof that wants it, never a speculative port.

1. **Use the stdlib solver if one fits.** `Gandr.Arena.Offset` against `Data.Nat.Solver` is the exemplar.
2. **If none fits, build it first**, packaged exactly as stdlib packages its own (`Algebra.Solver.Monoid`'s `Expression` / `Normal` / `Solver` / facade split), so local and provided solvers present one interface.
3. **Until then, leave the obligation by hand with a code note naming the solver that should discharge it.** An unmarked hand proof of solver-shaped work is the drift this rule exists to prevent.

Goals are **quoted by hand** into the solver's expression syntax, as `Gandr.Arena.Offset` does.
Reflection-based tactic macros (`Tactic.RingSolver`, `Tactic.MonoidSolver`) are declined as too brittle; proof-by-reflection solvers built on `Relation.Binary.Reflection` are not macros and are the intended target.

## Opacity

`opaque` is the default for a definition whose unfolding is a cost to be controlled, with `unfolding` naming each consumer that needs it — `Gandr.Arena.Offset`'s `⊗-ix` family is the exemplar.

The deliberate exception is the carrier layer.
`Gandr.Graph`'s definitions exist to be unfolded: every consumer meets them through copattern matching on `ϵ°`/`δ°`, and sealing them would sever the definitional equalities the whole tower is built from.
A module that opts out states why in its header.

## The done-rule

A metatheory milestone is done only when `agda:check` is green **and** its documentation face lands in the same motion — the module header for new structure, or the port-delta note for a ported layer.
Gate-green alone is half a milestone.

A green gate is also not proof of _meaning_.
State residuals honestly in the module header: a theorem that is reduced but not discharged says so, and a scope cut says what it cut and why it does not weaken the result.

## Commits

Keep `metatheory/**` in a **separate commit** from the Rust it mirrors — it is a distinct artifact whose history may be reorganized independently.
Repository plumbing (mise tasks, gates, this document) rides with whichever side it serves.

Commit messages follow the repository convention; `.commitlintrc.mts` is authoritative, including the canonical agent-trailer registry.
