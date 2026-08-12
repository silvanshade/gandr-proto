# Architecture

The project map: domains, repository layout, package layering, and the load-bearing invariants.
Read this before any structural change.
It routes into the authoritative docs instead of restating them; where two docs disagree, the linked authority wins.

- How work moves (posture, gates, tracker, worktrees): [AGENTS.md](AGENTS.md) and the workflow routing layer [docs/WORKFLOW.md](docs/WORKFLOW.md).
- What the design is: the specification corpus [docs/gandr/spec/](docs/gandr/spec/README.md), the authority within the `docs/gandr/` design corpus.
- Why it was decided: the approved [PLAN.html](PLAN.html) and the beads tracker (the per-file `docs/adr/` log is deferred, [docs/workflow/docs.md](docs/workflow/docs.md)).

## The system in one paragraph

gandr is a dependently typed language and shell built around a minimal certified kernel.
A polarized CBPV core is checked by a bidirectional typing machine, lowered by static focusing onto a polarized System-L command IL, and executed by the L machine.
Higher-dimensional rewriting — budgeted Squier completion over an oriented cell store, with replayable tracelet certificates reflected through a virtual-double-category judgement layer — is the computational-univalence story.
Persistence is content-addressed and untrusted; the mechanized metatheory is Agda.

## Repository layout

| Path                                     | Holds                                                                                                            |
| ---------------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| `crates/`                                | the Rust workspace (25 members; domains and layering below)                                                      |
| `crates/*/docs/`                         | per-crate lean doc tier (STATUS, ADR, CHANGELOG, METRICS, OPTIMIZATION) where present — off the corpus main-path |
| `docs/WORKFLOW.md` + `docs/workflow/`    | the workflow routing layer and its task-scoped sub-files                                                         |
| `docs/gandr/`                            | the design corpus root: index, BLAKE3 `MANIFEST.yml`, and the `spec/` tracks with their Hayagriva bibliography   |
| `metatheory/`                            | the Agda metatheory, built port-as-source under `Gandr.*` over a vendored agda-stdlib — no submodule, no facade  |
| `fuzz/`                                  | independent AFL++ fuzz workspace — own lockfile and lint posture, excluded from the main workspace               |
| `scripts/`                               | legacy Nushell helpers, retired for new work ([docs/workflow/scripting.md](docs/workflow/scripting.md))          |
| `mise.toml`                              | canonical task and gate bodies plus the toolchain pins (stable + dated nightly)                                  |
| `treefmt.toml` and the formatter configs | the format wall: rumdl, typos, sizelint, rustfmt, oxfmt, tombi and friends                                       |
| `prek.toml`, `.config/wt.toml`           | commit-hook and worktree/merge-hook wiring (the local merge wall)                                                |
| [PLAN.html](PLAN.html)                   | the approved build-out plan                                                                                      |
| [CHANGELOG.md](CHANGELOG.md)             | the single workspace changelog (the per-crate `docs/` tier is retiring)                                          |

Referenced by guidance but not yet landed: `docs/adr/`, `docs/KNOWLEDGE.md` and `docs/HAZARDS.md` (the corpus-trust and hazard catalogues `AGENTS.md` cites), and hosted CI (parked; the whole gate wall is local — [docs/workflow/ci.md](docs/workflow/ci.md)).

## Domains

Crate names are domain-prefixed; the prefix is the domain.
Roles are one-line condensations of each crate's `Cargo.toml` description, which stays the per-crate authority.
Counting convention: a member is an active entry in the root `Cargo.toml` `workspace.members` list — 26 members over 27 `crates/` directories, the 27th being the parked doc-class tool `workflow-docs` (commented out of the workspace), which no domain row or tier counts.

| Domain       | Crates                                                                                                                                                          | Role                                                                                                                                                                               |
| ------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `kernel-*`   | kernel-strata, kernel-core                                                                                                                                      | the certified trusted core: universe-level oracle; S1 term/type language and env                                                                                                   |
| `core-*`     | core-checker, core-sequent, core-incremental                                                                                                                    | the checked language: CBPV typing machine; System-L IL, focusing, the L machine; item-granular incremental re-typing (seam, footprints, validated resume)                          |
| `theory-*`   | theory-nominal-automata, theory-orders, theory-graphs, theory-recursion, theory-levitation, theory-computads, theory-circuit-algebras, theory-virtual-doctrines | semantic machinery: atoms, orders, graphs, recursion; descriptions; completion; circuit-algebra interface bookkeeping, embedding matching, and diagram normal form; VDC reflection |
| `storage-*`  | storage-chunker, storage-prolly-trees, storage-artifact                                                                                                         | untrusted content-addressed persistence: chunking, Merkle search tree, CAS export                                                                                                  |
| `runtime-*`  | runtime-host                                                                                                                                                    | headless host-effect runtime (Exec/Fs/Proc/Env) driven by the L machine                                                                                                            |
| `surface-*`  | surface-syntax, surface-render-remote, surface-grammar, surface-parser, surface-engine, surface-corpus, surface-driver                                          | user-facing syntax and tools: CST + diffing, inspection wire protocol, grammar, parser, lowering engine, example corpus, driver (stub)                                             |
| `workflow-*` | workflow-gates, workflow-dylint                                                                                                                                 | project tooling: the gate battery, project-local Dylint lints (the doc-class tool `workflow-docs` is parked)                                                                       |

## Package layering

`A → B` means _A depends on B_.
Tier N crates depend only on tiers below N.

```text
tier 0   kernel-strata · storage-chunker · surface-syntax · surface-render-remote
         theory-graphs · theory-nominal-automata · theory-orders · theory-recursion
tier 1   kernel-core → kernel-strata
         storage-prolly-trees → storage-chunker
         surface-grammar → surface-render-remote, surface-syntax, theory-graphs
tier 2   core-checker → kernel-core, theory-nominal-automata
         storage-artifact → kernel-core, storage-chunker, storage-prolly-trees
         surface-parser → surface-grammar, surface-syntax
tier 3   core-incremental → core-checker, theory-orders
         core-sequent → core-checker, kernel-core, kernel-strata, storage-artifact, storage-prolly-trees
         theory-levitation → core-checker
tier 4   theory-computads → core-sequent, theory-graphs, theory-levitation
         runtime-host → core-checker, core-sequent
tier 5   theory-circuit-algebras → theory-computads
         theory-virtual-doctrines → core-sequent, theory-computads, theory-levitation
         surface-engine → core-checker, core-incremental, core-sequent, runtime-host,
         surface-grammar, surface-parser, surface-syntax, theory-levitation,
         theory-nominal-automata, theory-recursion
tier 6   surface-corpus → core-checker, core-sequent, runtime-host, surface-engine, theory-levitation
off-tier workflow-gates, workflow-dylint — tooling; depend on no workspace crate
         (the doc-class tool workflow-docs is parked: commented out of the workspace, no tier)
         surface-driver — stub entry point; the REPL/script/tui/lsp/mcp/fmt/build faces land with the crates they wrap
```

The rules the graph enforces:

1. **The kernel trusts only itself.** `kernel-core` depends on `kernel-strata` and nothing else; no `kernel-*` crate may gain a dependency outside the domain.
2. **Dependencies point inward.** Leaves stay leaves; no library crate may depend on `surface-driver` or on any `workflow-*` tooling crate (they sit off-tier by construction).
3. **Theory substrate is self-contained.** The `theory-*` leaves (graphs, orders, nominal-automata, recursion) have zero workspace dependencies; the higher theory (levitation, computads, circuit-algebras, virtual-doctrines) stacks over `core-*` — directly, or through another `theory-*` crate, as `circuit-algebras` does — and takes no direct dependency on `storage-*`, `runtime-*`, or `surface-*`.
   The reading is the direct one on both halves: `core-sequent` links `storage-*` for persistence (rule 4), so every crate above it reaches storage transitively and the rule constrains the edges a `theory-*` manifest may declare.
   `theory-computads` owns the engines and never depends on `theory-circuit-algebras`: a matcher reaches completion by being supplied at the engine's instantiation site, and the reverse **library** edge is a dependency cycle the resolver rejects (a `[dev-dependencies]` cycle Cargo does admit, so a test-only edge is refused by this rule rather than by the resolver) ([docs/gandr/spec/implementation/circuit-terms.md](docs/gandr/spec/implementation/circuit-terms.md), `circuit-terms-question-12`).
4. **Storage stays untrusted plumbing.** `storage-*` crates are content-addressed plumbing with proof machinery; the kernel never links them (`core-sequent` does, for persistence).
5. **`fuzz/` is a separate workspace.** It path-deps ports-in-flight and keeps its own lint posture; the main workspace excludes it.

## Load-bearing invariants

Each invariant names its enforcement surface; the gates live in [docs/workflow/ci.md](docs/workflow/ci.md).

1. **Kernel minimality.** The kernel environment is append-only behind the single `add_decl` choke point (one warned bypass plus a print-axioms audit), the S1 checker is zero-inference, and definitional-equality conversion is quarantined.
   Sources: the `kernel-core` manifest and the crate's rustdoc.
2. **Content-addressed identity.** Artifact identity is a canonical BLAKE3 manifest hash; prolly-tree nodes carry BLAKE3 identity with membership, non-membership, and range proofs.
   Sources: the `storage-artifact` / `storage-prolly-trees` manifests.
3. **The Rust lint wall is absolute.** The workspace `[workspace.lints]` table denies clippy all/pedantic/nursery/restriction; partial functions, `unwrap`/`expect` in shipping code, and undocumented nontrivial items do not compile.
   Sources: [Cargo.toml](Cargo.toml), [docs/workflow/rust.md](docs/workflow/rust.md).
4. **Project-local Dylint contracts gate merges.** The recursion/termination contract (and its documented relaxations) runs on the merge wall between Clippy and the test suite.
   Sources: [docs/workflow/ci.md](docs/workflow/ci.md), [crates/workflow-dylint/](crates/workflow-dylint/).
5. **The specification corpus is cited, not free-form.** Every external work is cited by a key that resolves in `docs/gandr/spec/bibliography.yml`, and the bibliography holds no entry the corpus never cites.
   Source: [docs/gandr/spec/README.md](docs/gandr/spec/README.md), [docs/workflow/specs.md](docs/workflow/specs.md).
6. **The design corpus is hash-registered.** Editing a doc registered in `docs/gandr/MANIFEST.yml` updates its BLAKE3 sum in the same commit (`docs:manifest-drift` gate).
   Source: [docs/gandr/README.md](docs/gandr/README.md).
7. **Fidelity beats formatters.** A formatter or linter is relaxed or scoped, never satisfied at the cost of artifact fidelity.
   Source: [docs/workflow/docs.md](docs/workflow/docs.md).
8. **History is publishable.** Tracked content and commit messages are project-concern only; contributor-concern material lives outside the tree.
   Source: [AGENTS.md](AGENTS.md).

## Routing

| Question                     | Authoritative source                                                                                          |
| ---------------------------- | ------------------------------------------------------------------------------------------------------------- |
| What is the language design? | [docs/gandr/spec/](docs/gandr/spec/README.md) — the four track documents                                      |
| Why was it decided?          | [PLAN.html](PLAN.html) + the beads tracker, until `docs/adr/` is re-introduced                                |
| What is a crate's status?    | `crates/<crate>/docs/` (STATUS, ADR, CHANGELOG, METRICS, OPTIMIZATION) where present, else its `Cargo.toml`   |
| How do I work on X?          | [docs/WORKFLOW.md](docs/WORKFLOW.md) → the matching `docs/workflow/` sub-file                                 |
| What studies back a design?  | the design record itself — the studies behind it are held in the maintainer's private research workspace      |
