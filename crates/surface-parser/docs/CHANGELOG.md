# Changelog

The format is hand-maintained and grows only with real changes; it is not auto-generated.

## 2026-08-02 — Lex the ruled circuit arrow grid and the primed word

* `current`: `label`'s `MULTI_PUNCT` table gains the four circuit arrow-grid glyphs (`-->`, `<->`, `==>`, `<=>`) ahead of the shorter tiles each strictly extends (`->`, `<-`, `==`, `<=`), so the grid glyph wins the maximal munch and every shorter tile keeps its own reading.
  `=>` (the case arm) is untouched — neither glyph prefixes the other — and `--` is not a comment lead in this language, so `-->` disturbs no comment convention.
* `current`: A word may now carry trailing primes (`′`, U+2032): `x′` is one word, a lone `′` stays an unknown byte, and ASCII `'` is deliberately still not a word byte (it opens a shell single-quoted run).
  The primed variable is the ruled circuit form's own spelling for a rewrite's target endpoint.
* `current`: `mold`'s keyword table reserves the two item-position circuit leads `sign` and `oper`.
  The member lead `sort` and the body leads `node` / `feed` stay contextual — inadmissible at every lowercase-word slot outside an open circuit block — so a user program may still bind them as ordinary names.

## 2026-07-21 — Port the resumable melder push-machine + obligation taxonomy from wyrd (F2)

* `current`: Landed `gandr-surface-parser`, the front-end's resumable push-machine melder plus obligation taxonomy (rung F2 of `docs/research/front-end-port-staging.md` §9), ported from the wyrd `gandr-parser` crate.
* `current`: Modules — `oblig` (the `Oblig` severity ladder, `ObligationInstance`, and the `Delta` count array), `label` (the hand-rolled total labeler DFA), `mold` (the deterministic molder / candidate resolution), `meld` (the resumable first-order push machine `MeldState` with `push`/`commit`/`checkpoint`/`resume`/`finalize`), and `parse` (the batch `parse` entry) — 9436 source lines across six modules, ported with the push machine, the obligation deltas, and the losslessness contract intact.
* `current`: Dependency re-point — `gandr-grammar` → `gandr-surface-grammar` and `gandr-syntax` → `gandr-surface-syntax`.
  The wyrd manifest's `gandr-graph` edge was dropped rather than re-pointed: the parser reaches every graph type through the grammar's re-exports and had no direct use, so the direct `gandr-theory-graphs` edge was vestigial.
* `current`: O3 / ADR scrub — 74 wyrd bead-ID occurrences and 7 wyrd `ADR-NN` citations (ADR-43/48/73/76/80) were dropped or inlined in current terms; zero survive into rustdoc, and the crate is clean under `cargo:doc-check`.
  The wyrd design-wave labels (W3′/W4′/W4b/W4d/W4e), academic `paper Fig. N` references, and `grammar.js` cross-references were preserved as behaviour-bearing / provenance content.
* `current`: Tests — the parser goldens funnel through the `autotests = false` aggregator `tests/parser.rs` (the self-contained `contracts` module plus the `acceptance` submodule); 66 pass under nextest.
* `current`: The zero-obligation corpus gate is ported faithfully with its count lock pinned in-body, but the corpus it reads lands at F4 with `surface-corpus`; the five corpus tests plus the inline `parse::tests::corpus_parses_totally` are parked with `#[ignore]` and return at F4, and `incomplete_input_flags_statement_local_obligations` (grammar-contract-fixtures) is parked to F6.
  Locked counts: model + pathological 87 / 87 clean, `surface/` tree non-empty and clean, zero total obligations.
* `current`: The wyrd `push` / `parse` criterion benches are corpus-driven and deferred to F4 to co-land with `surface-corpus`; the `criterion` dev-dependency was dropped with them.
* `current`: Feature posture — `default = []`, `full = []`; the parser carries no feature-gated code.
* `current`: F2 also re-adds `gandr-surface-parser` as a dev-only dependency of `surface-grammar` and restores the six parser-coupled `contracts` tests parked at F1 (`crates/surface-grammar/docs/CHANGELOG.md`).
