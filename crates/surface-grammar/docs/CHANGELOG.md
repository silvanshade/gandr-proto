# Changelog

The format is hand-maintained and grows only with real changes; it is not auto-generated.

## 2026-08-07 — The `sign` block's members are `;`-terminated; the terminator is load-bearing

* `current`: Every `sign` member is now **terminated** by `;` (the surface's declaration terminator), and the terminator is load-bearing at the member level rather than merely admitted (owner directive, gandr-ng9.14).
  The unseparated member list was the one remaining carrier of the wrong-tree mechanism recorded below for the data blocks: a member ends in a sort hole, the walk's `≐`-relation crosses it, and the next member's lead can collapse the whole member into one repaired region — the graduation rung's `add(x, x)` collapse (gandr-ng9.14, hazard 1), which left the cell layer's linearity refusal unreachable from source.
  After the hole only `;` is admissible, and it never competes with hole content, so the discrimination is structural again; an unterminated member flags a repair naming the position.
  The reopening condition for a terminator-free spelling is a molder key change: hole-fill must outrank `≐`-continuation.
* `current`: The slot boundary is pinned: the retired `,` is NOT admissible at the `sign` block's member level — it stays admissible only inside the nested `data` / `codata` generator and observation lists (`member_list` in `surface/term.rs`), where it exists so a stale declaration parses whole and reaches the elaborator's migration decline.
  Top-level `oper` / `rule` declarations are unchanged: they end in `)` or `}` by the forced-parenthesization discipline and carry no trailing hole.
* `current`: The 2026-08-02 "recorded, not fixed" `sort` collision is **dissolved**: a member lead is admissible only after the terminator now, so `sign S { oper f : sort --> Nat; sort Bit : Type; }` molds the first `sort` as a `type_variable` (the bare-sort side it is) and the second as the member lead — the wrong regroup has no admissible reading left.
* `current`: A `sign` block may be named with a primitive-type spelling (`sign Unknown`).
  The labeler's uppercase-word reservation (`UPPER_KEYWORDS`) is a disambiguation preference at slots where the reserved tile molds; the molder falls back to the word's generic `constructor` / `type_identifier` labels when no reserved candidate is admissible at the live frontier (`Molder::gather_reserved_fallback` in `gandr-surface-parser`), so `def x : Unknown;` still molds the primitive-type atom while `sign Unknown { … }` molds the name slot.
* `current`: Mold-count effect, pinned in `tests/walk.rs` — declared molds 1782 → 1783 (the member terminator `;`, one mold beside the inlined member family), reachable multi-mold labels unchanged at 72, fingerprint `0xfa35_0169_cdda_acb1` → `0x11ed_981d_95a5_1344`.

## 2026-08-07 — The nested generator block is the one `data` form; the Haskell-style form stays admissible for its decline

* `current`: The `data` declaration's head now binds the family's parameters **once** as typed binders `(a : Type, …)` and carries the index arity as the head annotation `: Idx -> Type` (`: Type` when unindexed), and every generator member is a judgment `Ctor : (binders) --> Result ;` (bare-side rungs: `Nil : Vec(a, 0) ;`, `Some : a --> Maybe(a) ;`).
  The generator's telescope is kept LOCAL to the member (the `op_result` precedent), and its `:` lead discriminates against the retired field-tuple tail one tile after the constructor name.
  Three folded adaptation surfaces join `PBG_ONLY_KINDS`: `data_generator`, `constructor_block_member`, `bare_type_params`.
* `current`: The retired shapes stay **admissible** rather than deleted: the bare-parameter head `data Maybe(a)`, the head without an annotation, the field-tuple member `Ctor(x: A)`, and the comma member separator all still parse whole so `gandr-surface-engine`'s stage-0 elaborator declines them and names the respelling (the retired-`~>` precedent).
  The decline, not the grammar, is what refuses the old form — it is never a silent synonym.
* `current`: The `codata` block takes the same head discipline (`codata Stream(a : Type) : Type`); its observation members are unchanged (observations are not generators).
* `current`: Members are terminated by `;` (the surface's declaration terminator), with `,` admissible between members for the retired spelling.
  An **unseparated** member list was tried and is deliberately not admitted: a member ends in a sort hole (the generator's signature, the observation's payload), the walk's `≐`-relation crosses the hole to whatever may follow the member, and at the fill position the next member's lead mold (`constructor`, `identifier`) outranks the hole's own content in the molder's local key — `Nil : Vec` read as a member `Nil` plus a nullary member `Vec`, a clean parse of the wrong tree the zero-obligation gate cannot see.
  The `sign` block's unseparated list is exempt because its leads are reserved keywords, never hole-content candidates.
* `current`: Mold-count effect, pinned in `tests/walk.rs` — declared molds 1726 → 1782 (the typed head binder, the head annotation, the generator's local telescope + `-->` signature, and the `;` member terminator beside the admissible `,`), reachable multi-mold labels unchanged at 72, fingerprint `0x17f0_7f8d_0489_a2e2` → `0xfa35_0169_cdda_acb1`.

## 2026-08-02 — Migrate the description-rule face arrow from `~>` to `==>`

* `current`: The `data` / `codata` `rule` member's face arrow is now the ruled `==>` (`docs/gandr/spec/surface-language/circuit-cells.md` §"The block form, ruled" retires `~>` and makes `==>` the rewrite-face former at every position).
  Both member families share one `rule_face_arrow()` alternation.
* `current`: The retired `~>` stays **admissible in the arrow slot** rather than being deleted.
  Deleting it would turn a stale face into a parse repair naming a token; admitting it keeps the member whole so `gandr-surface-engine`'s stage-0 elaborator declines it and names the respelling.
  The decline, not the grammar, is what refuses the old spelling — it is never a silent synonym.
* `current`: `~~>`, the ratified directed former on types, is untouched: it never entered `MULTI_PUNCT`, and the ruling's whole point was that retiring `~>` dissolves the near-collision instead of managing it.
  Pinned by `gandr-surface-parser` `label::tests::the_face_migration_leaves_the_directed_type_former_run_alone`.
* `current`: Mold-count effect, pinned in `tests/walk.rs` — declared molds 1720 → 1724 (the arrow alternation, twice per member family through `comma1`), reachable multi-mold labels unchanged at 72, fingerprint `0x0ad7_e73c_f55a_db6c` → `0x483c_357b_641a_298e`.

## 2026-08-02 — Land the ruled circuit block form in the checked surface

* `current`: New `surface/circuit` submodule realising the ruled circuit block form (`docs/gandr/spec/surface-language/circuit-cells.md` §"The block form, ruled") — the `sign` block with `sort` / `data` / `oper` / `rule` judgment members, the four-glyph arrow grid (`-->` / `<->` circuit 1-cell formers, `==>` / `<=>` rewrite faces), arrow-separated two-sided port lists with parameter-side rewrite and data binders, and the top-level `oper` / `rule` declaration with its `node` / `feed` body statements.
* `current`: The grammar admits **any** grid glyph at **every** arrow position by design: the ruling requires a disagreement to be a localized, nameable error, and a body line's arrow is confirmed against the applied head's kind — an environment fact no grammar sees.
  Confirmation is `gandr-surface-engine`'s `circuit` pass.
* `current`: Three new PBG-only provenances (`sign_declaration`, `circuit_declaration`) and five folded adaptation surfaces (`circuit_member`, `circuit_signature`, `circuit_body`, `node_statement`, `feed_statement`) join `PBG_ONLY_KINDS`; the highlighter's keyword and operator tables gain the five circuit leads and the four grid glyphs.
* `current`: A top-level circuit declaration takes **parenthesized sides**; the sugar ladder's bare-sort rungs are `sign`-member-only.
  An Item-sort form that can end in a sort hole does not close — a bare-sort side detaches and the declaration silently keeps its prefix, a clean parse of the wrong tree the zero-obligation gate cannot see — and no other Item form in this grammar ends in one.
  Pinned by `gandr-surface-parser` `acceptance::a_top_level_circuit_declaration_keeps_its_whole_signature`, which asserts the arrow is a _descendant_ of the declaration rather than merely that the parse is clean.
* `current`: Recorded, not fixed: `sort` is a contextual keyword whose mold is admissible exactly where a member's bare-sort side sits, so `sign S { oper f : sort --> Nat … }` regroups cleanly and wrongly.
  Reserving `sort` is not available — `list.sort` is a live corpus projection — and the collision is bounded to that one slot and to that one keyword.
* `current`: Mold-count effect, pinned in `tests/walk.rs` — declared molds 1482 → 1720, reachable multi-mold labels 64 → 72, fingerprint `0x7b0c_4e6c_c16b_8608` → `0x0ad7_e73c_f55a_db6c`.
  The `oper` / `rule` judgment is declared once and shared by the `sign` member and the top-level declaration, and the telescope binders are kept off the result side; both are cost decisions against the `identifier` / `(` / `:` menus, which a duplicated tail would have widened twice as far.

## 2026-07-21 — Restore the parser-coupled surface-acceptance contracts suite (F2)

* `current`: Re-added `gandr-surface-parser` as a **dev-only** dependency (the deliberate parser↔grammar cycle-break — normal dep parser→grammar, dev dep grammar→parser) and restored the six parser-coupled surface-acceptance `contracts` tests parked at F1, now in `tests/contracts.rs` and wired into the `tests/surface.rs` aggregator.
* `current`: The suite is green unchanged — the F1 scrub-adjusted golden data verified safe on restore — bringing the grammar aggregator to 34 tests under nextest.

## 2026-07-21 — Port the checked PBG grammar core + mold highlighter from wyrd (F1)

* `current`: Landed `gandr-surface-grammar`, the front-end's checked precedence-bounded grammar (PBG) core plus the mold-driven highlighter (rung F1 of `docs/research/front-end-port-staging.md` §9), ported from the wyrd `gandr-grammar` crate.
* `current`: The port renames the package to `gandr-surface-grammar` (staging call O1) and re-points the three workspace dependencies to their landed reboot homes — `gandr-graph` → `gandr-theory-graphs` (the `prec` precedence DAG and the finite walk-machine engine), `gandr-render-proto` → `gandr-surface-render-remote` (the highlighter's `HlRole`/`HlSpan` types, codecs off), and `gandr-syntax` → `gandr-surface-syntax` (the flat-arena CST).
* `current`: Modules — `check`, `model`, `mold`, `walk`, `surface` (with `surface/term` + `surface/type_shell`), `highlight`, and `parity`; the checked PBG construction, the regex-zipper mold table, the generative walk index, the `built_in` surface assembly, the normative mold highlighter, and the tree-sitter named-kind provenance inventory port with their types, tables, `built_in` grammar, and hashing intact.
* `current`: O3 / ADR scrub — 75 wyrd bead-ID occurrences (19 distinct ids) and 19 wyrd `ADR-NN` citations (ADR-54/57/66/70/76) were dropped or inlined in current terms; zero survive into rustdoc, and the crate is clean under `cargo:doc-check`.
  The wyrd design-wave labels (W4d/W4e/W5′) were preserved verbatim, including inside load-bearing `AdaptationReason` data strings where a rewrite would change behaviour.
* `current`: The `parity` feature and its optional `gandr-tree-sitter` + `tree-sitter` edges are omitted entirely — tree-sitter returns at rung F6 — so the default dependency graph is tree-sitter-free by construction.
  This also resolves the wyrd HZ-8 / port-map H1 wart by construction: with no `parity` feature, `full = []` no longer silently omits a differential-parity lane.
* `current`: The deliberate DEV-only parser dependency (the wyrd parser↔grammar cycle break) is not carried; `surface-parser` lands at F2.
  The parser-coupled `tests/surface.rs` `contracts` suite is parked to F2 (which re-adds the dev-dependency), and the tree-sitter drift-gate / differential-parity suites (`tests/node_types_gate.rs`, `tests/highlight_parity.rs`, `tests/token_stream_parity.rs`) are parked to F6 — all listed by name in `docs/STATUS.md`.
* `current`: Tests carried across — the parser-free grammar unit goldens `tests/pbg.rs` and `tests/walk.rs`, funnelled through the `autotests = false` aggregator `tests/surface.rs`, plus the `highlight` module unit tests; 28 pass under nextest, including the `built_in` fingerprint, mold-count, candidate-inventory, and reachable-mold goldens.
  Feature posture — `default = []`, `full = []`; a `walk_index` criterion bench is present.
* `current`: The wyrd `gandr-grammar-contract-fixtures` fold (staging call O4) is flagged and deferred to F6 rather than forced — its manifest is a cross-language Rust/tree-sitter differential-parity contract whose consumers are all deferred; see `docs/STATUS.md` for the rationale.
