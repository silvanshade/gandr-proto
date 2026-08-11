# Regularity compression — the Tarau program as a candidate second compression axis for massive terms (gandr-9a9)

> **Status: RESEARCH RECORD — owner-dispositioned, not a decision record.** This document absorbs a nine-document source-grounded read of Paul Tarau's compressed-representation research program (2008–2016) and the coordinator synthesis over it, executed for `gandr-9a9` as a follow-on to the massive-term ratification (`gandr-bvf` / `gandr-5t3`).
> **Owner disposition (2026-07-21, ratifying the synthesis):** the axis stays at **research status** under the massive-term program (`gandr-3ln`) and is **not prioritized now**; it is held as a candidate S2+ direction because it would differentiate gandr from existing systems on yet another axis — of plausible interest to mathematicians studying large constructions — with the concrete design template recorded here.
> **Consumers.** The future S2 conversion/former design pass (the anticipated description-code formers, `massive-term-design.md` §11 item 2); the corpus/property-test generator backlog (§7.1 below); `gandr-3ln` plane 2 (compression posture); the `gandr-9a9` bead, which carries the per-paper scout reports in its comment record.
>
> **Method and citation conventions.** All nine documents were read in full (the 104-page 0808.2953v4 selectively, by section relevance) in a single research pass; every load-bearing claim below carries its source anchor as `id §section/p.page` or `deck:slide`, carried verbatim from the read.
> Proven results are marked **[P]**, claimed/argued results **[C]**, empirical-only results **[E]** — the program itself is scrupulous about this distinction and this record preserves it.
> Two of the nine documents are conference slide decks (claims maps, nearly proof-free); their attributions to full papers are recorded where the decks state them.
> No machine-local paths and no session forensics appear in this file.

---

## 1. The question, and the organizing lens

The v1 export format (`massive-term-design.md` §4; landed at `gandr-5t3`) compresses by **maximal DAG/subterm sharing**: each structurally distinct subterm is stored once and referenced by index.
That axis captures _repeated identical_ subterms at arbitrary positions; it cannot capture _regular-but-structurally-distinct_ families — a chain of `K` nested binders is `K` distinct subterms and costs `K` table entries.
The question studied here: does Tarau's run-length/regularity compression line offer a **complementary, non-codec** compression axis for massive terms — structural compression with operations that work directly on the compressed form?

The organizing lens is the **free-structure reframe** (owner, 2026-07-21, recorded on `gandr-9a9`): rather than asking which operations _happen to be_ closed over a fixed compressed form, _choose_ the compressed form as the free structure over the signature Σ of operations one cares about, subject to the encoding schema.
Closure then holds by construction, and the difficulty relocates to three places:

1. **The encoding side-condition** — the free presentation must map _bijectively_ onto the semantic domain, or a quotient appears whose decision procedure is exactly the decompression being avoided.
2. **Operations outside Σ** — defined by structural recursion plus re-normalization, with representation-entropy-dependent cost and possible compression erosion under chaining.
3. **Conversion against the unfold equations** — compressed constructors carry defining equations relating them to their expansions; relating compressed to expanded forms without full unfolding is the controlled-decompression problem, for which the smart-unfolding/transparency discipline (`impl-models-deep-read.md` §2.4) is the worked precedent.

The Tarau program, reread through this lens, is a sixteen-year sequence of instances of exactly this design move — for **numbers**, never for term **semantics** (§6).

---

## 2. Corpus map

| Document           | Identity                                                                                                               | Role                                                                                                                                                                                                                    | Proof weight                                    |
| ------------------ | ---------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------- |
| arXiv 0808.0754v1  | _A Functional Hitchhiker's Guide to Hereditarily Finite Sets, Ackermann Encodings and Pairing Functions_ (2008, draft) | HFS/Ackermann made executable; size-proportionate bit-interleaving pairing; Mostowski decoration; **DAG-sharing of encoded integers proposed as future work** (§8)                                                      | bijection proofs; size claims structural        |
| arXiv 0808.0753v1  | _Ranking Catamorphisms and Unranking Anamorphisms on Hereditarily Finite Datatypes_ (2008, draft)                      | the generic `rank`/`unrank` hylomorphism (Prop 1 [P]: bijectivity + termination under _children strictly smaller than parent_); the `nat2rle` run-length bijection (Prop 7 [P]) — the program's seed; HFF universes     | core props proven                               |
| arXiv 0808.2953v4  | _Isomorphic Data Encodings in Haskell…_ (2008/09, 104 pp)                                                              | the Iso-groupoid over 59 datatypes; HFF succinctness (info-density > 0.53, **conjecture**, §18.2); giant sparse numbers; hereditary base-k trees; honest negative: general-purpose compression **not** achieved (§18.2) | mixed; key density claim conjectural            |
| arXiv 1006.5768v1  | _A Unified Formal Description of Arithmetic and Set Theoretical Data Types_ (2010)                                     | the compositionality pole: one generic operation set over 5 primitives runs on Peano / bit-stacks / HFS / GMP                                                                                                           | weakest: Props asserted; one op an open problem |
| arXiv 1306.1128v1  | _Arithmetic Algorithms for Hereditarily Binary Natural Numbers_ (2013)                                                 | **the proof core** (§3, §4 below)                                                                                                                                                                                       | strongest; partially Coq-mechanized             |
| arXiv 1406.1796v2  | _A Generic Numbering System based on Catalan Families of Combinatorial Objects_ (2014)                                 | the generic form: the `Cat` free-structure presentation; proven successor-family bounds; the erosion and worst-case results                                                                                             | successor family [P]; block ops [C]+[E]         |
| SYNASC'14 deck     | _New Arithmetic Algorithms for Hereditarily Binary Natural Numbers_ (25 slides)                                        | sequel algorithms; **the honest benchmark ledger** (§5.3)                                                                                                                                                               | mul block identities [P]; rest asserted         |
| PADL'16 deck A     | _A Size-proportionate Bijective Encoding of Lambda Terms as Catalan Objects…_ (25 slides)                              | the lambda-term extension (§6.2); propositions stated, proofs in the unseen full paper                                                                                                                                  | statements only                                 |
| PADL'16 deck B     | _Computing with Catalan Families, Generically_ (25 slides)                                                             | deck of the 1406.1796 line; successor complexity proofs present on-slide                                                                                                                                                | s/s′ proofs on-slide                            |
| arXiv 1507.06944v1 | _A Logic Programming Playground…_ (2015, 70 pp)                                                                        | the synthesis paper; compressed de Bruijn (binder-run-length only, **static**); §8.13.2 endorses stacking DAG-folding on the trees                                                                                      | props stated; defers arithmetic proofs          |

Direction note carried from the read: the **Ackermann encoding is direction-asymmetric** — _unranking_ (number → tree) is size-proportionate (hence "giant sparse numbers" as small trees, 0808.2953 §4.1), while _ranking_ (nested set → number) is non-elementary (height-`h` nesting → a tower of 2s).
The 2015 paper's use of Ackermann as a negative example of size-proportionate encodings (1507.06944 §7.1.2) concerns the ranking direction only.

---

## 3. The three generations of bijectivity mechanism

The load-bearing design element in every generation is **canonicality by construction**: the constructor set is chosen so that the map to the semantic domain is a bijection, hence no runtime normalization pass exists and equality is syntactic.

- **Ackermann / HFS (2008).** `f(x) = Σ_{a∈x} 2^{f(a)}` [P bijective, 0808.0753 Props 1–2]; canonical given set-distinctness; direction-asymmetric size behavior (§2 note).
- **Hereditarily binary (2013).** `data T = E | V T [T] | W T [T]` — a number is its bijective-base-2 digit string with _run-lengths encoded recursively as further `T` values_; the alternation of `V`/`W` blocks is the uniqueness discipline.
  **[P]** `n : T → ℕ` is a canonical bijection (1306.1128 Prop 5); **[P]** successor/predecessor are total inverses, _mechanized in Coq with the induction principle `hbNat_ind` printed_ (1306.1128 p.3) — the program's one existence proof that a compressed type supports total induction-shaped definitions.
- **Catalan-generic (2014).** The `Cat` class — `e` nullary, `c` binary, `c′` its inverse, with laws exactly presenting the **free structure over `{e, c}`** (1406.1796 §3.1; deck B slide 6 laws (1)–(2)).
  Canonical form: maximal runs, adjacent blocks differing, top digit 1 (1406.1796 eq. (3)).
  Because `c` is a bijection onto ℕ⁺, a non-canonical representation is _unrepresentable_ — canonicality needs no enforcement pass at all (1406.1796 §3.4 + Conclusion p.38).
  Genericity: the same algorithms run on binary trees, multiway trees, Dyck words, and ℕ itself (the cross-validation oracle), covering the 58 known Catalan instances.

The run-length uniqueness condition (adjacent blocks differ) is the regularity-axis analogue of v1's "no two structurally equal table entries" (E4 condition 2, `massive-term-design.md` §4.6): in both, the canonical form is the _maximally-compressed_ form and the discipline makes it unique.

---

## 4. The operation ledger — proven core, claimed periphery

| Operation family                                         | Status                                                                                                                                                                                                         | Anchor                                                                                   |
| -------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| successor/predecessor `s`/`s′`, `db`/`hf`, `exp2`/`log2` | **[P]** constant average, O(log\*) worst — _with the linchpin caveat below_                                                                                                                                    | 1406.1796 §4.2–4.3 Props 5–7; 1306.1128 Props 6–9; deck B slides 19–21 (proofs on-slide) |
| add / sub / shifts (block-at-a-time)                     | **[C]+[E]** — block identities proven (1306.1128 Props 11–12; SYNASC deck (6),(7) for mul), _complexity_ argued from average-block-size ≤ 2; efficiency **verbatim conditional on few blocks** (1306.1128 p.5) | 1406.1796 §5; 1306.1128 §6.3                                                             |
| multiplication / square / pow                            | block identities **[P]**, cost **[C]** ("∝ blocks in first arg, not digits")                                                                                                                                   | SYNASC deck 13–14; 1406.1796 §6.1–6.2                                                    |
| **comparison** `cmp`                                     | order-correctness **[P]** (bitsize-monotone, Props 8–9); cost **[C]** ∝ the _smaller_ compressed operand, skipping identical leading blocks; equality is derived structural `==`, sound by canonicality        | 1406.1796 §5.3 pp.16–17                                                                  |
| division / integer sqrt                                  | the resistant operations: "does not provide the same complexity gains"; block-at-a-time division "if possible at all, is subject of future work"                                                               | 1406.1796 §6.3; 1306.1128 Appendix                                                       |

**The linchpin caveat, stated exactly:** the "constant time" successor is _"practically constant"_ — recursive calls land on terms roughly logarithmic in operand bitsize, giving the recurrence `T(n) = T(log n) + O(1)` = O(log\*) worst; the O(1) reading is an iterated-log heuristic, not a proved constant bound (1306.1128 p.4; deck B slide 20 scope caveat: the bounds assume O(1) constructors, true for tree instances, false for the ℕ instance).

**The clique caveat:** the block operations form one mutual-recursion clique (`add ↔ leftshiftBy ↔ cmp ↔ bitsize ↔ sub ↔ s`, 1406.1796 §2 report), and block counts are first-class compressed numbers manipulated by the full arithmetic — a "block rule" cannot be verified in isolation; any judgment-level import drags the clique into the metatheory.

---

## 5. Size behavior

### 5.1 Worst case is proven harmless

**[P]** `catsize t ≤ bitsize t` for all `t` (1406.1796 Prop 11); the worst case is exactly characterized — alternating single digits, `worseCase k = 4(4^k − 1)/3` (1306.1128 Props 17–18) — where compression gains nothing but _costs_ nothing: totality, canonicality, and the order are unaffected.
The representation cannot lose more than a constant factor to flat binary.
This is the **cannot-lose criterion** the disposition adopts as a design bar (§7.2).

### 5.2 Best case and erosion

Best case is tetration: `2↑↑k`-neighborhood numbers have trees of size ~`k` against non-representable bitsizes (1406.1796 §10.3: `bestCase (t 5)` has bitsize 65536, catsize 5; record-holder primes at tree sizes 27–62 against bitsizes to 8.2 × 10⁷, Fig 4).
Erosion is real and quantified: combining two _differently-structured_ compressed operands multiplies representation size — adding towers of heights 101 and 103 yields catsize 10500 ≈ their product (1406.1796 §11.2 **[E]**).

### 5.3 The honest benchmark ledger (SYNASC'14 deck, slide 19, ms, vs GMP)

Tower-shaped inputs: `2^2^30` at 0 vs 10192; two GMP-intractable rows (1000 Collatz steps from a tower; product of five giant primes) complete in seconds.
Ordinary inputs: `factorial 200` at 8040 vs 2; prime generation at 4807 vs 6 — **losses of 100–4000×** on irregular data.
The compression axis is for machine-generated regular families, not general-purpose data — consistent with 0808.2953's own negative result (§18.2: the parenthesis-code compression of naturals beats the plain form only on sparse powers of two).

### 5.4 The axes stack

1306.1128 §8 **DAG-folds the run-length trees** (the 48th Mersenne prime: 7 shared nodes, structural complexity 22, Fig 2) — run-length regularity compression and subterm sharing composed in one artifact; 1507.06944 §8.13.2 endorses exactly this stacking, and 0808.0754 §8 had proposed it as future work in 2008.
Composition with v1's subterm table is therefore demonstrated, not hypothesized.

---

## 6. Terms — where the program stops

### 6.1 Compressed de Bruijn is static

The 2015 representation (`v(K,N)`/`a(K,X,Y)`, 1507.06944 §2.5) run-length-encodes **consecutive binders only**; it is a plain tree with no sharing edges.
Direct-on-compressed operations: closedness and ranking.
**Substitution, β, normalization, and type inference all decompress** — the paper's own evaluator is decompress → normalize → recompress (§5 p.22).

### 6.2 The lambda ranking exists; the semantics do not

PADL'16 deck A defines `data X a = Vx a a | Ax a (X a) (X a)` and the ranking `x2t` into any `Cat` instance — bijective on open terms, size-proportionate, with random generation demonstrated to 50,000-node open terms (slides 5, 9–10, 20–23; proposition statements only, proofs in the unseen full paper).
Evaluation is `evalT = x2t . evalX . t2x` — through decompression, and explicitly non-total (slide 16).

**The decisive gap:** across all nine documents the program computes arithmetic on compressed _numbers_ and never a semantic operation on compressed _terms_.
Tarau supplies representation theory — the free-structure presentation, by-construction canonicality, syntactic equality, an honest cost model — and no judgment theory.
Induction-shaped _checking_ over compressed structure has exactly one existence proof, and only for numbers: `hbNat_ind` with total `s`/`s′` in Coq (1306.1128 p.3).

---

## 7. Disposition for gandr (owner-ratified 2026-07-21)

The hard gate from the `gandr-9a9` intake analysis stands: format-level compressed nodes **without** checker operations on the compressed form re-open the amplification surface the 5t3 budgets closed (small bytes → huge expanded work), and buy nothing on checker time.
The sound home for the axis is **judgment-level, at S2+**: regularity formers (iteration / description codes, the anticipated S2 former set) whose typing rules check a block body once and quantify over the count — closure over _checking_ by construction, per the free-structure reframe.
The sweep upgrades that staging from a direction to a template:

- present the former set as a **free structure with by-construction canonicality** (an alternation-style uniqueness condition; §3);
- keep **equality syntactic on canonical forms** — the conversion-checking payoff (§4), with `cmp`'s skip-identical-leading-blocks shape as the model;
- expect Σ-adjacent operations cheap and everything else conditional; treat division-like operations as the resistance test;
- prove the compressed type's **induction principle in the metatheory mirror** (the `hbNat_ind` precedent);
- enforce canonical form by the **generalized E4 pattern** (canonical bytes = maximally-compressed normal form, compression-aware re-encode-compare), with the new proof obligation the sweep makes precise: **uniqueness/confluence of maximal compression** — trivial for DAG sharing, genuinely nontrivial for run-length regularities (overlapping runs compete; a canonical decomposition rule needs a uniqueness argument);
- conversion against the unfold equations goes through **controlled decompression** — the smart-unfolding/transparency recipe (`impl-models-deep-read.md` §2.4) reread for compressed constructors.

### 7.1 Near-term cheap wins (no judgment changes)

1. **Unranking-based term generation** for the corpus/property suites: size-proportionate bijective unranking yields well-scoped (open) terms to 50k nodes (deck A slides 20–23) — a principled generator axis for the hardening/amplification test families.
2. **The cannot-lose criterion** (§5.1) as a design bar for any future format node: admit only compression with a proven `compressed-size ≤ plain-size` bound.

### 7.2 Not prioritized — and why it stays recorded

Owner framing at disposition: pursuing this further would differentiate gandr from existing systems on another axis — of plausible interest to mathematicians studying large constructions — but it does not take priority now.
The record here (template + ledger + gap) is the S2-era input; nothing in the current buildout depends on it.

---

## 8. Open items

1. **Fetch targets if graduated:** the PADL'16 lambda full paper (the `b2x`/`x2b`, `t2x`/`x2t` bijection and size-proportionality proofs — only stated on the deck); the CICM/Calculemus Gödel-numbering companion (deck A slide 24; the ℕ → ℕᵏ side is noted there as binary-search-limited to small open terms).
2. **The uniqueness-of-maximal-compression obligation** (§7) has no analogue in the Tarau corpus (his canonicality is by bijective construction, not by normalization) — it is gandr's own proof debt if regularity formers ever meet a canonical-bytes format.
3. **Erosion under mixing** (§5.2) predicts that regular-family compression degrades when proofs combine heterogeneous structure; whether real proof corpora are regularity-homogeneous enough is an empirical question for the D3 telemetry era.

## 9. Cross-references

Beads: `gandr-9a9` (this record's tracker home; per-document scout reports and the framing/synthesis comments live there), `gandr-3ln` (massive-term program), `gandr-5t3`/`gandr-bvf` (the landed sharing axis and its ratification).
Sibling records: `massive-term-design.md` (the DAG-sharing axis, E4 canonicality, the reader budgets), `impl-models-deep-read.md` (§2.4 smart unfolding; §3.2/§5.1 sharing-as-boundary-pass), `storage-rkyv-design.md` (the storage-tier posture the format planes sit over).
