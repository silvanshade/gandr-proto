# Architecture

The project map: domains, repository layout, package layering, and the load-bearing invariants.
Read this before any structural change.
It routes into the authoritative docs instead of restating them; where two docs disagree, the linked authority wins.

- How work moves (posture, gates, tracker, worktrees): [AGENTS.md](AGENTS.md) and the workflow routing layer [docs/WORKFLOW.md](docs/WORKFLOW.md).
- What the design is: the specification corpus, `spec:README.md` and the four track documents under it.
  **It is not in this repository** — it is held in the maintainer's private research workspace, and this tree cites it by the `spec:` alias rather than by path.
- Why it was decided: the project's pages in the maintainer's private research workspace and the beads tracker.

## The system in one paragraph

gandr is a dependently typed language and shell built around a minimal certified kernel.
A polarized CBPV core is checked by a bidirectional typing machine, lowered by static focusing onto a polarized System-L command IL, and executed by the L machine.
The compiled route for that IL is MLIR, reached through a C++26 host under `runtime/compile-host/`; its first slice compiles the positive core, and `crates/runtime-compile-host` is the Rust side of that boundary.
Higher-dimensional rewriting — budgeted Squier completion over an oriented cell store, with replayable tracelet certificates reflected through a virtual-double-category judgement layer — is the computational-univalence story.
Persistence is content-addressed and untrusted; the mechanized metatheory is Agda and lives in the separate `gandr-metatheory` repository.

## Repository layout

| Path                                     | Holds                                                                                                       |
| ---------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| `crates/`                                | the Rust workspace; the domains, the member count and its counting convention are below                     |
| `docs/WORKFLOW.md` + `docs/workflow/`    | the workflow routing layer and its task-scoped sub-files                                                    |
| `fuzz/`                                  | independent AFL++ fuzz workspace — own lockfile and lint posture, excluded from the main workspace          |
| `runtime/`                               | the C++ tree: the C++26 MLIR compilation host, built by CMake against a discovered MLIR, off the merge wall |
| `scripts/`                               | legacy Nushell helpers, retired for new work ([docs/workflow/scripting.md](docs/workflow/scripting.md))     |
| `mise.toml`                              | canonical task and gate bodies plus the toolchain pins (stable + dated nightly)                             |
| `treefmt.toml` and the formatter configs | the format wall: rumdl, typos, sizelint, rustfmt, oxfmt, tombi and friends                                  |
| `prek.toml`, `.config/wt.toml`           | commit-hook and worktree/merge-hook wiring (the local merge wall)                                           |
| [CHANGELOG.md](CHANGELOG.md)             | the single workspace changelog; the per-crate `docs/` tier it replaced is gone                              |

Referenced by guidance but not landed: hosted CI (parked; the whole gate wall is local — [docs/workflow/ci.md](docs/workflow/ci.md)).

## Domains

Crate names are domain-prefixed; the prefix is the domain.
Roles are one-line condensations of each crate's `Cargo.toml` description, which stays the per-crate authority.
Counting convention: a member is an active entry in the root `Cargo.toml` `workspace.members` list — 40 members over 41 `crates/` directories, the 41st being the parked doc-class tool `workflow-docs` (commented out of the workspace), which no domain row or tier counts.

| Domain       | Crates                                                                                                                                                                                                                                                                                               | Role                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| ------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `kernel-*`   | kernel-strata, kernel-core                                                                                                                                                                                                                                                                           | the certified trusted core: universe-level oracle; S1 term/type language and env                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| `core-*`     | core-term, core-nbe, core-unify, core-checker, core-machine, core-checker-tools, core-sequent, core-incremental                                                                                                                                                                                      | the checked language: the CBPV term substrate every core judgement is stated over; the conversion engine that decides definitional equality; the predictable-fragment solver over the same theory; the recursive bidirectional judgement with its disciplines and kernel bridge; the defunctionalized typing machine derived from it; the shared free generators and the conformance suite that drives them; System-L IL, focusing, the L machine; item-granular incremental re-typing (seam, footprints, validated resume) |
| `theory-*`   | theory-nominal-automata, theory-orders, theory-graphs, theory-recursion, theory-levitation, theory-cell-complexes, theory-cell-complexes-tools, theory-coherent-resolutions, theory-deep-inference, theory-decomposition-spaces, theory-computads, theory-circuit-algebras, theory-virtual-doctrines | semantic machinery: atoms, orders, graphs, recursion; descriptions; the cell-shape substrate with its test-facing second inhabitant; critical pairs, coherence certificates and completion; the identity relations on derivations; the certificate algebra and its transport identity; the elaborator seam; circuit-algebra interface bookkeeping, embedding matching, and diagram normal form; VDC reflection                                                                                                              |
| `storage-*`  | storage-chunker, storage-prolly-trees, storage-artifact                                                                                                                                                                                                                                              | untrusted content-addressed persistence: chunking, Merkle search tree, CAS export                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| `runtime-*`  | runtime-effects, runtime-ffi, runtime-compile-host                                                                                                                                                                                                                                                   | headless host-effect runtime (Exec/Fs/Proc/Env) driven by the L machine; least-authority native C ABI host over the effect seam; the Rust side of the compilation host boundary, lowering checked core computations into program images and running them over a C ABI resolved at run time                                                                                                                                                                                                                                  |
| `surface-*`  | surface-syntax, surface-render-remote, surface-grammar, surface-parser, surface-lsp, surface-engine, surface-diagnostics, surface-repl, surface-tui, surface-corpus, surface-driver                                                                                                                  | user-facing syntax and tools: CST + diffing, inspection wire protocol, grammar, parser, language-server face, lowering engine with kernel admission, terminal diagnostic facade, read-evaluate loop, terminal face, example corpus, toolchain driver                                                                                                                                                                                                                                                                        |
| `workflow-*` | workflow-gates, workflow-dylint                                                                                                                                                                                                                                                                      | project tooling: the gate battery, project-local Dylint lints (the doc-class tool `workflow-docs` is parked)                                                                                                                                                                                                                                                                                                                                                                                                                |

## Package layering

`A → B` means _A has a library dependency on B_ — a `[dependencies]` edge, the only kind that reaches a consumer of A. Tier N crates depend only on tiers below N.
`[dev-dependencies]` are listed separately below the tiers, because they bind only a crate's own test targets: they do not compose, do not propagate to consumers, and may point up the tiers without breaking the ordering.

```text
tier 0   kernel-strata · storage-chunker · surface-syntax · surface-render-remote
         theory-graphs · theory-nominal-automata · theory-orders · theory-recursion
tier 1   core-term → kernel-strata, theory-nominal-automata
         kernel-core → kernel-strata
         storage-prolly-trees → storage-chunker
         surface-grammar → surface-render-remote, surface-syntax, theory-graphs
tier 2   core-checker-tools → core-term
         core-nbe → core-term
         core-sequent → core-term
         storage-artifact → kernel-core, storage-chunker, storage-prolly-trees
         surface-parser → surface-grammar, surface-syntax
         theory-levitation → core-term
tier 3   core-checker → core-nbe, core-term, kernel-core, kernel-strata
         core-unify → core-nbe, core-term
         runtime-effects → core-sequent, core-term
         theory-cell-complexes → core-sequent
tier 4   core-machine → core-checker, core-term
         theory-cell-complexes-tools → theory-cell-complexes
         theory-circuit-algebras → theory-cell-complexes
         theory-coherent-resolutions → theory-cell-complexes
tier 5   core-incremental → core-checker, core-machine, core-term, theory-orders
         runtime-compile-host → core-checker, core-machine, core-term
         theory-deep-inference → theory-cell-complexes, theory-coherent-resolutions
tier 6   theory-computads → core-sequent, theory-cell-complexes, theory-coherent-resolutions,
         theory-deep-inference, theory-levitation
         theory-decomposition-spaces → core-sequent, storage-artifact, theory-cell-complexes,
         theory-coherent-resolutions, theory-deep-inference, theory-graphs
tier 7   theory-virtual-doctrines → theory-cell-complexes, theory-coherent-resolutions,
         theory-decomposition-spaces, theory-levitation
tier 8   surface-engine → core-checker, core-incremental, core-machine, core-sequent, core-term,
         kernel-core, runtime-effects, storage-artifact, storage-prolly-trees, surface-grammar,
         surface-parser, surface-render-remote, surface-syntax, theory-circuit-algebras,
         theory-computads, theory-levitation, theory-nominal-automata, theory-recursion
tier 9   runtime-ffi → core-sequent, core-term, runtime-effects, surface-engine
         surface-diagnostics → core-term, surface-engine, surface-syntax
         surface-lsp → surface-engine, surface-grammar, surface-parser, surface-render-remote,
         surface-syntax
tier 10  surface-corpus → core-sequent, core-term, runtime-effects, runtime-ffi,
         surface-engine, theory-levitation
         surface-repl → surface-diagnostics, surface-engine, surface-grammar, surface-parser,
         surface-render-remote, surface-syntax
         (the read-evaluate loop consumes the tier-9 diagnostic facade)
tier 11  surface-tui → surface-repl, surface-render-remote, surface-syntax
off-tier workflow-gates, workflow-dylint — tooling; depend on no workspace crate
         (the doc-class tool workflow-docs is parked: commented out of the workspace, no tier)
         surface-driver → core-term, runtime-ffi, surface-diagnostics, surface-engine,
         surface-lsp, surface-repl, surface-tui — process entry point: script runner,
         read-evaluate loop, terminal face, and language-server face;
         the mcp/fmt/build faces land with the crates they wrap

```

The cross-tier `[dev-dependencies]`, which are where several crates are exercised from and nowhere else:

| test target's crate           | dev-depends on                                                                                   | what it drives                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| ----------------------------- | ------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `core-sequent`                | core-checker, kernel-core, kernel-strata, storage-artifact, storage-prolly-trees, surface-engine | the kernel export gate — bridge → `add_decl` → export → decode → replay — plus the corpus differentials that need a real front end; the checker edge is test-only because focusing and the L machine name the substrate alone                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| `theory-circuit-algebras`     | core-sequent                                                                                     | fixtures whose cuts carry the command IL's polarity, which the crate never names in shipping code                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| `theory-virtual-doctrines`    | core-sequent                                                                                     | the same, for the reflected judgment layer's fixtures                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| `theory-coherent-resolutions` | core-sequent, theory-cell-complexes-tools, theory-deep-inference                                 | the second inhabitant, and — through the **admitted dev-only cycle** back to `theory-deep-inference` — the certificate normal form that two witnesses read: `tests/completion.rs`'s `every_generated_certificate_matches_its_replay_plan` and `tests/overlap.rs`'s `a_relabelled_twin_schedules_and_replays_identically`. Both assert completion's own guarantee — that a generated certificate replays to the plan its normal form schedules — so they belong to this crate rather than to the one that computes the form. Both are integration tests rather than inline because an inline build is a distinct crate instance, and a dev cycle cannot unify types across one |
| `theory-deep-inference`       | core-sequent, theory-cell-complexes-tools                                                        | the second inhabitant and its five adversarial wrappers, which is where the identity relations are held to the trait rather than to one alphabet                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| `theory-decomposition-spaces` | theory-cell-complexes-tools, theory-computads, theory-levitation                                 | the second inhabitant, plus one composition fixture that reaches the engines through a real elaborated description                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| `theory-computads`            | theory-cell-complexes-tools                                                                      | the second inhabitant, for the circuit-instantiation witnesses                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| `surface-grammar`             | surface-parser                                                                                   | the deliberate cycle-break: the parser depends on the grammar, so the reverse edge is dev-only and drives the acceptance contracts                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| `core-checker`                | core-checker-tools                                                                               | the free proptest generators; the pair carries a deliberate dev-only cycle, since the generator crate's own conformance target drives the checker back                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| `core-checker-tools`          | core-checker, core-machine                                                                       | the two typing faces the conformance suite compares; the generators themselves name only the substrate, so a consumer of them links `core-term` and nothing above it                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| `runtime-compile-host`        | core-sequent                                                                                     | the L machine, as the oracle the bridge's compiled answers are compared against; the crate itself lowers over the substrate and checks over the machine, so the L machine edge is test-only                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| `surface-engine`              | core-checker-tools, theory-cell-complexes, theory-coherent-resolutions                           | the same generators, plus the cell and certificate vocabulary three integration suites name directly rather than through the elaborator facade the shipping path uses                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |

The rules the graph enforces:

0. **The substrate decides nothing, and the engines name no judgement.** `core-term` carries the vocabulary every core judgement is stated over and **has no upward dependency at all**, which is the precise property: the checker, the conversion engine, the solver, and the L machine are each stated over one vocabulary, and none of them depends on another merely to name a term.
   They are not independent of each other, and the tiers above are where their edges are stated — `core-checker → core-nbe` and `core-unify → core-nbe` are both deliberate, because **a definitional equality is decided in exactly one place**.
   `core-nbe` sits one step above the substrate and names no typing judgement either: consumers call into the conversion relation, which is what pins one equational theory across all of them instead of letting each grow its own, and `core-unify` draws every rule it applies from that same relation.
   A crate that only needs a term, a type, or an outcome takes `core-term` and stops there; six crates do.
   `core-unify` has no consumer at all yet, which is a scheduling fact rather than a layering one: the elaborator seam that will drive it is unbuilt, and the crate is exercised by its own tests.
   `core-machine` is above `core-checker` rather than beside it, and the edge is the derivation direction: the machine is obtained from the recursive judgement by the functional correspondence, so it names the judgement layer and the judgement layer names nothing of it.
   The step-for-step agreement between the two realizations is not carried by either of them — `core-checker-tools` compares them from outside, which is where it already lived.
1. **The kernel trusts only itself.** `kernel-core` depends on `kernel-strata` and nothing else; no `kernel-*` crate may gain a dependency outside the domain.
2. **Dependencies point inward.** Leaves stay leaves; no library crate may depend on `surface-driver` or on any `workflow-*` tooling crate (they sit off-tier by construction).
3. **Theory substrate is self-contained.** The `theory-*` leaves (graphs, orders, nominal-automata, recursion) have zero workspace dependencies; the higher theory (levitation, the cell-rewriting stack, circuit-algebras, virtual-doctrines) stacks over `core-*` — directly, or through another `theory-*` crate, as `circuit-algebras` does — and takes no direct dependency on `runtime-*` or `surface-*`.
   The reading is the direct one on both halves, and the rule constrains the edges a `theory-*` manifest may declare.
   **The cell-rewriting stack is one linear tier chain and every edge in it points down**: `cell-complexes` defines what a cell is, `coherent-resolutions` fires and completes them, `deep-inference` decides when two derivations are one, `decomposition-spaces` composes and transports the certificates, and `computads` elaborates descriptions into the whole of it.
   Two of its five crates carry a strongly connected component the boundary encloses rather than cuts: a certificate names the branching it joins, and an event's canonical key digests the causal order that reads the key back.
   `theory-cell-complexes-tools` is test-facing, depends on `theory-cell-complexes` alone, and is a dev-dependency of four crates and a library dependency of none.
   The stack admits exactly one `[dev-dependencies]` cycle, `theory-coherent-resolutions` back to `theory-deep-inference`, and it is admitted rather than tolerated: two completion witnesses read the certificate normal form, they assert completion's guarantee rather than the normal form's, and an inline test build is a distinct crate instance that no dev cycle can unify types across — so they are integration tests, which link the library and do.
   No crate in the stack depends on `theory-circuit-algebras`: a matcher reaches a consumer by being supplied where the engine is instantiated — `theory-computads`'s `instantiate` module is that site — and the reverse **library** edge is a dependency cycle the resolver rejects (a `[dev-dependencies]` cycle Cargo does admit, so a test-only edge is refused by this rule rather than by the resolver) (`spec:implementation/circuit-terms.md`, `circuit-terms-question-12`).
   That supply point is `surface-engine`, which already sits above both for independent reasons; the matcher edge adds no tier movement of its own.
   The direction is the whole of the guarantee — both theory crates are named by one consumer above them, and the reverse edge is a cycle rather than a rule anyone keeps.
4. **Storage stays untrusted plumbing.** `storage-*` crates are content-addressed plumbing with proof machinery, and the kernel never links them.
   Their shipping consumers are `theory-decomposition-spaces` (portable step identities through the canonical BLAKE3 framing) and `surface-engine` (artifact minting and the block-store boundary); `core-sequent` links them from its tests only, for the kernel export gate.
   The direction is what the rule constrains: `storage-*` depends on `kernel-core` and on itself, never upward.
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
5. **This repository holds no reference register, so a citation carries its own locator.** Every external work is cited at the claim with its full title, its authors, its year, and a stable identifier; the `spec:bibliography.yml` register belongs to the corpus and left with it.
   Sources: [docs/workflow/docs.md](docs/workflow/docs.md), [docs/workflow/specs.md](docs/workflow/specs.md).
6. **The design corpus is no longer in this repository.** It left with its BLAKE3 registry, and the `docs:manifest-drift` and `docs:reference-integrity` gates retired with it — there is nothing in this tree for them to register or resolve.
   The corpus is cited by the `spec:` alias and remains the authority on the design; what this repository relies on is restated here.
7. **Fidelity beats formatters.** A formatter or linter is relaxed or scoped, never satisfied at the cost of artifact fidelity.
   Source: [docs/workflow/docs.md](docs/workflow/docs.md).
8. **History is publishable.** Tracked content and commit messages are project-concern only; contributor-concern material lives outside the tree.
   Source: [AGENTS.md](AGENTS.md).
9. **The compilation host is discovered, not pinned, so the wall requires it conditionally and checks it unconditionally.** `runtime/compile-host/` builds against an installed MLIR whose version must match the compiling clang, and nothing in the Rust workspace links it — `runtime-compile-host` resolves the host's C boundary at run time, so a checkout with no MLIR still builds and tests everything here.
   The wall carries three things.
   `compile-host:wall` runs the host's own gates whenever the toolchain is **discoverable** and skips only on proven absence, so a broken `CMakeLists.txt` or a broken host source fails the wall rather than reading as an optional toolchain (`GANDR_COMPILE_HOST_STRICT=1` turns the absence skip into a failure too).
   `crates/runtime-compile-host/tests/contract.rs` holds the Rust mirror of the boundary — heap layout, cell and node numbering, constructor arities, wire version, ABI version and statuses, the boundary struct's field order, the verifier-first pipeline, and the grade operations' effect declarations — to the host's own sources, unconditionally.
   The residual gap is behaviour: no checkout without MLIR proves the host _does_ what its declarations say, and closing that means pinning MLIR.
   The third is the fixture differential: `crates/core-sequent/tests/compile_host_agreement.rs` holds `runtime/compile-host/fixtures/positive-core-samples.txt` to what the L machine answers, and the host's `jit_agrees_with_the_fixture_on_every_sample` case holds the compiled slice to the same file.
   Sources: [runtime/compile-host/README.md](runtime/compile-host/README.md), [crates/runtime-compile-host/README.md](crates/runtime-compile-host/README.md), [.config/mise/tasks/mise-tasks-compile-host.toml](.config/mise/tasks/mise-tasks-compile-host.toml).

## Routing

| Question                      | Authoritative source                                                                                                         |
| ----------------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| What is the language design?  | `spec:README.md` — the four track documents, held outside this repository                                                    |
| Why was it decided?           | the project's pages in the maintainer's private research workspace + the beads tracker                                       |
| What is a crate's status?     | its `Cargo.toml` description and its crate-root rustdoc — the per-crate `docs/` tier is gone                                 |
| What compiles gandr natively? | [runtime/compile-host/README.md](runtime/compile-host/README.md) — the slice it covers and the block it does not             |
| How does Rust reach it?       | [crates/runtime-compile-host/README.md](crates/runtime-compile-host/README.md) — the lowering, the wire form, the C boundary |
| How do I work on X?           | [docs/WORKFLOW.md](docs/WORKFLOW.md) → the matching `docs/workflow/` sub-file                                                |
| What studies back a design?   | the design record itself — the studies behind it are held in the maintainer's private research workspace                     |
