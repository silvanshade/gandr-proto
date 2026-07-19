# Workflow: Agda house style (metatheory)

> Read when: touching `metatheory/` (Agda is the sole proof vehicle, ADR-30).

**House-style rules: IU ADR-2, adopted by reference.** The `metatheory/` tree adopts the sister **internal-univalence** (IU) library's numbered `HS-n` discipline wholesale (purpose-built records over raw Σ; explicit record instances; record types imported at file top, projections opened at use site; `hiding`/`using` one name per line; no `private variable` blocks; copattern style for record values; eager arrow-leading line breaks; the flat proof-term ladder; every definition carries a comment).
The rules live in IU's `docs/spec/ADR.md` §ADR-2 and are cited bare (`HS-n`) from wyrd comments.

**wyrd deltas.**

* Vocabulary is the dictionary's, not IU's: construct names come from the calf/decalf line via `docs/gandr/dictionary.yml` (the `agda_module:` column); IU's ∞-graph layer letters are not transliterated onto gandr constructs.
* The engine layer is consumed from IU as a pinned, read-only git submodule (`metatheory/upstream/internal-univalence`; the `iu:check` gate guards initialized-clean-at-pin).
  Improvements land upstream, then the pin bumps — integration record in `metatheory/README.md` §"Upstream integration".

**Agda-DbC stance.** The TYPE is the contract; do not port the Rust `# Contract` block.
Every definition carries a comment (HS-15); load-bearing insight lives in the manual/dictionary and the code cites its sections.
Mandatory marks are reserved for trust-story exceptions only: signature parameters standing for assumptions, any future with-K or unsafe island, gradual/`Blame` boundaries.

**Flags and the gate.**

* Per-file `OPTIONS`: `--safe --without-K --hidden-argument-puns` mandated on every module under `metatheory/src`, enforced by the Rust `source_policy::run_options_policy` sweep (`mise run test:options-policy`; CLI subcommand `options-policy`; exemptions enumerated per flag with justification).
  The without-K mandate is binding project-wide (ADR-76): neither UIP nor definitional proof-irrelevance may enter through any shortcut.
* `--guardedness` is need-based and **infective**: any module importing the upstream coinductive `Internal.Graph` (directly or transitively) must carry it.
* Strict root / holey leaf: `Gandr.Everything` is the strict root — everything it imports is `--safe` and green.
  Mid-proof work lives in a **declared holey leaf** (not imported by the root, checked with `--expected-code UnsolvedInteractionMetas`, enumerated in the `--safe` exemption list).
  Zero silent postulates, ever.
* `mise run agda:check` = aifix over the strict root + the policy sweep.

**Dependencies.** Adding any Agda library or tool requires maintainer sign-off **first** — deliberately stricter than the Rust/TS trees.
External research artifacts (mechanizations, companion code) are reference-only: read and cite, never vendor, port, or depend on, regardless of license.

**The done-rule.** A metatheory milestone is DONE only when `agda:check` is green AND its doc face lands in the same motion — the dictionary/manual lock-step entry for new vocabulary, or the port-delta note for ported layers.
Gate-green alone is half a milestone.

**Commits.** Keep `metatheory/**` in a separate commit from the Rust it mirrors (distinct artifact whose history may be reorganized); the `docs/gandr/**` dictionary face may ride with either.
