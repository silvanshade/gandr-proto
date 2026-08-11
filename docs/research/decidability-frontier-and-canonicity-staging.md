# The decidability frontier and staged canonicity — a design study for the completion machinery (the J-42 relevance pass)

> **Status: PROPOSAL for owner review — not a decision record.** Owner-commissioned (2026-07-20) relevance exploration of register row **J-42** (Clarke, Scherer & Zeilberger, _The Free Bifibration on a Functor_), whose register row records "relevance not yet explored"; this study is that exploration, scoped to the two imports the owner selected: (1) the **decidability frontier** as the justification and sharpening of gandr's certificate-carried design, and (2) **staged canonicity with executable tooling** as the build template for the Squier/completion machinery.
> No ticket exists yet; minting one is §6 Q1.
> Its consumers are named: (a) the **T5 tree-ARS engine / `VSquier` spec register** (`iu:src/Internal/Doctrine/Complex.agda`, where Squier completion is "named per level as an engine dependency, not built" — `iu-absorption.md` §3.2); (b) the **B9 certificate component** (gandr-wvd.9; the composition-mode tag + acyclicity witness of `directed-univalence-design.md` §8.1); (c) the **Temporal Univalence re-entry** (gandr-fcw.12; the `⇒²⇝` convergence pass, `directed-univalence-design.md` §5.1/§8.2-8.3); (d) the **`cells_equal` decidable normal-form fast path** (`iu-absorption.md` §7.1, `iu-ij6`/W10).
>
> **Alias key**: `iu:` = the internal-univalence Agda library (the metatheory substrate of the merged project); `iu-notes:` = its intake-ledger notes repo; `wyrd@failed-refactor:` = the pre-reboot worktree carrying the gandr crates and spec corpus; `research:` = the research-paper corpus.
> Literature citations use register keys from `docs/research/bibliography-v2.md`.
>
> **Provenance note (J-42 internals).** Every J-42-internal anchor (§/Thm/Defn/page) below derives from one full-document read pass (2026-07-20) of `research:papers/2511.07314v3.pdf` (96 pp; clean extraction).
> The anchors have **not** received the independent verification pass the register's `+hold` discipline requires before citation-bearing use; §7 records the upgrade path.
> The PDF prints no journal/volume/DOI (LMCS house style, no imprint): cite as **arXiv:2511.07314v3 [math.CT], stamped 2026-01-15**, preprint until corroborated.

## 1. What J-42 is, and what this study takes from it

J-42 constructs the **free bifibration on an arbitrary functor** p : D → C proof-theoretically (Thm 1.17, p. 20): objects are formulas over the unary **pushforward/pullback** connectives f⁺/g⁻ (the adjoint ∃/∀ doctrine), arrows are cut-free sequent derivations modulo permutation equivalence, composition is admissible cut.
Beck–Chevalley is deliberately **not** imposed; the BC cell is a genuine non-invertible generator whose quotient is analyzed separately (p. 78; Prop 5.7, pp. 80–81).
At p = id the construction yields the zigzag double category as the **free fibrant double category** (Thm 2.8), recovering the Dawson–Paré–Pronk free-adjoint construction (Cor 2.9).

> **Register correction (§7).** The J-42 row's summary — "Bicartesian monoidal coherence" — is inaccurate: the paper contains **no monoidal structure, no ⊗/⊕, no distributivity, and no bicartesian doctrine**.
> That phrase describes the Došen–Petrić line (unregistered; §7).
> J-42's actual content is the free bifibration, focusing/completion canonicity, and the decidability frontier.

The doctrine is disjoint from both the groupoid ua-base fragment and the directed `ℰ⇝` vocabulary, so **nothing transfers at the level of generators, cells, or statements**.
What transfers is architecture, and precisely two pieces of it:

1. **The decidability frontier** — the word problem for these free structures is **undecidable in general** (Thm 3.27, inheriting DPP) and decidable exactly under tractability conditions (**FP**, Defn 3.18; local finiteness, Thm 3.28) — which upgrades gandr's certificate-carried posture from a pragmatic choice to a **necessity claim with an external theorem behind it** (§3).
2. **The staged-canonicity pipeline with an executable enumerator alongside** — progressive normal forms, Knuth–Bendix-completed orientation, Newman-style confluence, explicit termination weight, canonicity (Thm 3.23), with a Haskell proof-search/enumeration tool run beside the paper proofs that caught a real error in the side conditions (footnote 3, p. 73) — which is the closest executed template for building `VSquier`/T5 and the convergence passes (§4).

One boundary is stated up front and honestly: J-42's focusing draws its canonicity from the **polarity of non-invertible adjoint structure**.
On gandr's invertible (groupoid) fragments that discipline degenerates, and the polygraph school (J-1; J-21; J-25b/c; J-28) remains the technical source — J-42 is a methodological exemplar there, not a proof source.
But the **directed** program is not iso-only: the one-way classes of `ℰ⇝` (E-proj/E-dup/E-inj/E-fold, `directed-univalence-design.md` §5.1) carry genuine polarity, and there the applicability is direct (§4.4).

## 2. Inventory — the load-bearing content of J-42, with anchors

| item                                  | anchor (read pass)                                             | one-line content                                                                                                                                                |
| ------------------------------------- | -------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Free bifibration construction         | Thm 1.17, p. 20; Defn 1.8 (cut)                                | free Bif(p) → C via cut-free derivations mod permutation; composition = admissible cut                                                                          |
| Conservativity                        | Prop 1.23                                                      | η_p full and faithful — the separation tool for non-derivability                                                                                                |
| Non-invertibility separations         | pp. 37–38                                                      | R f⁺ / L g⁻ not invertible: refuted by a concrete counterexample functor, not an invariant                                                                      |
| Zigzag / free fibrant double category | Thm 2.8; Cor 2.9                                               | companions-and-conjoints free completion; DPP Π₂ recovered (out of this study's scope; see §7)                                                                  |
| Non-thinness                          | p. 67                                                          | 11 distinct morphisms between two plane trees — coherence here means _decide_, not _collapse_                                                                   |
| Focusing pipeline                     | §3.1, §3.3, §3.4, §3.6                                         | unrestricted → weakly focused (strictly alternating formulas) → strongly → **maximally multifocused**                                                           |
| Completion                            | §3.5 ("par ∪ gra")                                             | divergent critical pairs oriented by Knuth–Bendix completion (their [28])                                                                                       |
| Confluence + termination              | Thm 3.21 (Newman/critical pairs, App A.2); Lemma 3.22 (weight) | local confluence enumerated; termination by explicit measure                                                                                                    |
| **Canonicity**                        | Thm 3.23                                                       | under FP: unique normal forms; α ~ β ⟺ NF(α) = NF(β)                                                                                                            |
| **FP condition**                      | Defn 3.18 (their [26], Johnstone); Ex 3.19                     | every commuting square has ≤ 1 diagonal filler; free categories, epi-only, mono-only are FP                                                                     |
| **Undecidability / decidability**     | Thm 3.27 / Thm 3.28                                            | word problem undecidable in general (DPP pedigree); decidable under FP or local finiteness                                                                      |
| Free-base collapse                    | p. 57                                                          | over a free base the factorization order is linear; the FP test is word prefix/suffix comparison                                                                |
| Homset enumeration                    | Thm 3.30                                                       | duplicate-free enumeration of finite homsets from the canonical forms                                                                                           |
| Executable tooling                    | footnotes 3–4 (pp. 73, 81)                                     | Haskell enumerator (github.com/noamz/free-bifibrations) — "revealed a bug in a previous incorrect formulation of the lock conditions"; SageMath post-processing |
| Formalization status                  | (whole document)                                               | pen-and-paper; **no proof assistant anywhere** — the tooling is executable, not certified                                                                       |

## 3. Import 1 — the decidability frontier justifies, and sharpens, the certificate posture

### 3.1 What the pre-reboot corpus already decided

The certificate-carried design is settled in the pre-reboot spec and survives into the reboot as posture:

- "U3's computability claim inherits η's qualifier until the sibling closes it — **but the acyclicity floor is operationally sufficient on its own**" (`wyrd@failed-refactor:docs/gandr/spec/proposal-ua-base.md` Ln 110).
- The U3 gate line: " _η-unconditional in the sibling, **or** the acyclicity floor with certificate-carried per-instance discharge_ (the latter is **not a degraded mode**; it defers the universal theorem only)" (ibid.
  Ln 115; staged into gate row D at Ln 238).
- Decision (4): two-mode certificate composition — unconditional on the invertible band, gated (lax, side-conditioned interchange) on the directed band (ibid.
  Ln 245); executed pre-reboot as `directed_cut`'s routing (`wyrd@failed-refactor:crates/gandr-vdc/src/directed/boundary.rs:41-54`; surveyed in `directed-univalence-design.md` §7.1 Ł4).

All of this was argued _internally_: from the η staging, from LLV's composability boundary, from the temporal program's P4.
What it lacked was an **external ceiling** — a theorem that no design could have done better.

### 3.2 What J-42 adds: the ceiling

Thm 3.27 is that ceiling: for free structures of this genre the word problem is **undecidable in general**.
The consequence for gandr, stated as the design principle this study proposes to record:

> **Beyond a tractability fence, per-instance certificates are the only general currency.** A decision procedure may be _promised_ only on fragments satisfying a named tractability condition (FP-style, or local finiteness); everywhere else the certificate-carried discharge is not a fallback but the mathematically maximal offer.

This converts the U3 gate's "operationally sufficient" from a defensive posture into a positive one, and it prices any future request for "just decide equality globally" at its true cost: such a request asks for a theorem that is false in the general case.

### 3.3 FP as a named, checkable criterion — and the fast-path scope

The second half of the import is that the tractability fence should be **named data, not folklore**.
J-42's FP condition (≤ 1 diagonal filler per commuting square) is checkable per presentation, is satisfied by free bases, and is exactly what makes canonicity (Thm 3.23) and decidability (Thm 3.28) go through.
Two places in the merged project already gesture at this without naming it:

- **`cells_equal`'s decidable normal-form fast path** is recorded as available "on convergent fragments" (`iu-ij6` 2026-07-15 addition; W10) — but _which_ fragments is currently informal.
  An FP-style predicate is the natural scope annotation: the fast path is sound exactly where a convergence/canonicity theorem holds, and the certificate-carried mode is the residual everywhere else.
- **The base-stratum decidability** of the ua-base tower is, in J-42's terms, the free-base collapse (p. 57): iu's normalization-to-towers puts every canonical arrow into a fragment where the word problem reduces to permutation-word comparison over a convergent (Coxeter) presentation — the iso/groupoid analogue of "the factorization order becomes linear and the FP test is prefix/suffix comparison."
  This is the honest metatheory face of the U3 claim "this stratum is entirely computable" (`proposal-ua-base.md` Ln 106).

### 3.4 Priced recommendations

- **R1 — record the frontier in the B9 certificate component's rationale.** The B9 schema already carries the composition-mode tag + acyclicity witness (`directed-univalence-design.md` §8.1 item 2).
  Extend the _rationale element_ (not the schema) with the tractability classification: each mode names _why_ it is available — `convergent-fragment` (a canonicity theorem covers the boundary; decision by NF) vs `certificate-carried` (general band; per-instance witness).
  Price: prose in the rationale element, zero schema bytes, zero semantics.
  Expensive later only in the soft sense: undocumented, the fast-path/general split re-derives itself in every review.
- **R2 — name an FP-style predicate in the T5 spec register, statement-grade.** The tree-ARS kernel (T5) is already spec-first — `VSquier` is "named here, never built" (`iu:src/Internal/Doctrine/Complex.agda` Ln 272-312 register discipline).
  Add one named predicate to that register: `Tractable Φ` (working name), the per-presentation condition under which the completion machinery may promise unique normal forms — FP-shaped, instantiated by the convergent fragments (the Coxeter stratum today; any future `⇒²⇝` convergence pass).
  Price: one statement-grade record field beside `VAcyclicAt`; no proof obligation created.
  This is deliberately the same absorb-as-statement discipline the doctrinal carrier HANDOFF set ("enough to show there is some sort of analogous tower").

## 4. Import 2 — staged canonicity as the build template for the completion machinery

### 4.1 The stage correspondence

J-42's pipeline factors canonicity into independently meaningful stages.
The merged project's layers already line up with it more closely than either source planned:

| J-42 stage                                             | gandr/iu counterpart                                                                              | status                                        |
| ------------------------------------------------------ | ------------------------------------------------------------------------------------------------- | --------------------------------------------- |
| cut-free presentation (Thm 1.17)                       | the free complex / word substrate (`iu:src/Internal/Rewriting.agda`; `𝔉`, `𝔇ω`)                   | landed / statement-grade                      |
| weak focusing → strictly alternating formulas (§3.3)   | normalization to canonical shapes (iu: codes to towers; certificates to normal boundaries)        | landed (groupoid stratum)                     |
| strong/maximal multifocusing (§3.4, §3.6)              | canonical-representative choice inside a permutation class (iu: perm-het canonical words)         | landed (groupoid stratum)                     |
| par ∪ gra completion (§3.5)                            | Squier completion (`iu:src/Internal/Rewriting/Confluence.agda`; the `VSquier` per-level register) | engine landed; doctrinal face named-not-built |
| local confluence by critical pairs (Thm 3.21, App A.2) | critical-branchings layer (`Internal.Rewriting`)                                                  | landed (engine)                               |
| termination weight (Lemma 3.22)                        | accessibility component of the T5 tree-ARS kernel                                                 | unbuilt (T5)                                  |
| canonicity (Thm 3.23)                                  | `coherence-from-acyclicity` (Squier internalized; `Internal.Rewriting` Ln 454-496)                | landed (engine)                               |
| duplicate-free homset enumeration (Thm 3.30)           | — no counterpart —                                                                                | candidate property harness (§4.3)             |

Two architectural lessons ride on this table, and they are the actual import.

### 4.2 Lesson 1 — stage the canonicity results so each stage is independently certifiable

J-42 proves each stage as its own theorem with its own interface (every derivation _has_ a focused form; focused forms admit a rewrite system; the system is locally confluent; it terminates; hence canonical forms).
This is the same factoring the VDC-shifted coherator already adopted — coherence-supply free by construction (`freeCoherence = coherenceAt coh`), **acyclicity isolated as the one genuine crux**, completion named per level rather than interpreted inline (`iu-absorption.md` §3.2).
J-42 is evidence, at research grade and in a harder (non-invertible) doctrine, that this factoring carries all the way to a decision procedure without ever needing the stages to be re-entangled.
The recommendation is to keep that discipline under pressure: when T5 lands, `VAcyclicAt`/`Tractable`/termination should stay **separate named obligations** with separate suppliers, never one monolithic convergence proof.

### 4.3 Lesson 2 — run an executable enumerator beside the proofs (R3)

J-42's Haskell tool is not part of any proof; it enumerates derivations/normal forms, generated the paper's examples, and **found a bug in a previous formulation of the lock conditions** (footnote 3).
The merged project has no counterpart, and three standing needs it would serve:

1. **Critical-pair exploration ahead of completion passes.** The iu campaign's third erratum family (the double-distribution interchange) was localized by hand-grinding across multiple sessions; unresolvable critical pairs _are_ missing cells, and a pair-enumerator over a cell alphabet surfaces them mechanically.
   Any future alphabet work (the `⇒²⇝` one-way coherence classes are the known next case) should have this tool available _before_ the grind starts.
2. **Alphabet-completeness / underivability evidence.** The open interchange-independence question iu-side (is the ratified cell underivable from the rest?) is exactly a bounded-search question on a minimal instance; J-42's practice says: run the cheap executable search first, escalate to the certified engine only if the answer matters at theorem grade.
3. **A property harness for `cells_equal`.** Thm 3.30's duplicate-free homset enumeration is the exact shape of a differential test: enumerate a finite homset two ways (canonical forms vs raw generation + `cells_equal` dedup) and compare cardinalities.
   This tests the fast path _and_ its `Tractable` fence on real fragments.

**Pricing and placement.** Tooling-side, semantics-free, **no TCB contact** — the enumerator asserts nothing; it is a bug-finder and evidence-generator (J-42's own status for theirs: implementation, not formalization).
Two candidate homes: a Rust crate in the tooling workspace (beside the doc-gen extractor; strict-lint house style already exists) or a meta-layer Agda reflection tool (stdlib-backed, off the kernel gate).
Recommendation: **Rust tooling-side** — the consumers (pair exploration, differential tests) are engineering workflows, the kernel gains nothing from having it in-language, and the meta layer's charter is elaboration-time tactics, not search.
Owner call recorded as §6 Q2.

### 4.4 Where focusing applies _directly_: the directed band

The §1 boundary — focusing degenerates on iso-only fragments — inverts on the directed program.
The `ℰ⇝` one-way classes (E-proj/E-dup/E-inj/E-fold) introduce genuine polarity into the path calculus, and the directed convergence pass is flagged in the directed design as its honest price: the word problem grows to the transformation-monoid setting, "no formalized rewriting twin is known," and the register holds no presentation row (`directed-univalence-design.md` §5.1, §8.2, §11.2 / §10 Q8).
J-42 bears on exactly this gap, twice:

- **Methodologically**: a focusing-staged canonical form (alternating-polarity towers, then multifocusing to canonical representatives, then completion of the residue) is the natural architecture for `⇒²⇝` — J-42's strictly-alternating formula normalization is face/degeneracy-flavored in the same way the E-inj/E-fold simplicial-identity cells are.
  The register already carries the polarity line this would draw on (C-5, C-7 — the same Zeilberger).
- **Bibliographically**: J-42's own cross-references (Simmons's structural focalization; the Uustalu–Veltri–Wan line, their [52], [54–55]) are the candidate payers for the missing transformation-monoid/finite-set-category register row.

**R4 — when the `⇒²⇝` convergence pass is scoped at the Temporal Univalence re-entry, evaluate a focusing-staged normal form before committing to a raw completion pass.** Price now: one planning note (this section); the evaluation itself is re-entry work and blocks nothing.
This recommendation deliberately does _not_ touch the groupoid stratum, whose walls-based closure is nearly complete and should not be re-architected (`directed-univalence-design.md` §11.1's in-motion caveat; the closure has advanced further since — re-baseline at re-entry per its §10 Q9).

## 5. Landing plan — nothing blocks the backbone

Per the standing fcw.11 rule (buildout first; metatheory parked), every item here is schema, spec-register naming, tooling, or planning — no theorem obligations on any backbone phase.

| phase / track                    | carries from this study                                                                                        | price                               | blocks backbone?      |
| -------------------------------- | -------------------------------------------------------------------------------------------------------------- | ----------------------------------- | --------------------- |
| **B9 certificates**              | R1: tractability classification in the certificate component's rationale element (mode tag already ordered)    | prose; zero schema                  | no                    |
| **T5 / `VSquier` spec register** | R2: named `Tractable` predicate beside `VAcyclicAt`, statement-grade                                           | one record field                    | no                    |
| **tooling (any time)**           | R3: the enumerator crate (pair exploration; bounded underivability search; `cells_equal` differential harness) | one small crate; off-TCB            | no                    |
| **Temporal Univalence re-entry** | R4: focusing-staged NF evaluation for `⇒²⇝`; the FP-scoped statement of the fast-path theorem                  | planning note now; work at re-entry | no — parked by design |

## 6. Open questions for the owner

1. **Q1 — ticket.** This study is unticketed.
   Mint under the buildout epic (as a B9/T5 input) or the research epic?
   The consumers span both; recommendation: one research bead with the four R-items as children, closed individually as their phases arrive.
2. **Q2 — enumerator home.** Rust tooling crate (recommended, §4.3) vs meta-layer Agda reflection tool.
   Also genuinely open: whether the iu-side interchange-independence search (a natural first user) should drive the tool's first iteration, since it needs only the groupoid cell alphabet.
3. **Q3 — scope of the `Tractable` predicate.** Per-fragment instances only (cheapest, matches current need) vs a general predicate over presentations in the T5 register (more honest to the J-42 shape, slightly more design).
   R2 as written is compatible with either.
4. **Q4 — register upgrades.** J-42 sits at `+hold` with an inaccurate summary (§1 box).
   The upgrade to citation-bearing status requires the independent verification pass on the anchors used here (§7).
   Who pays, and when — before or at first citation-bearing use?

## 7. Register notes

- **J-42 row correction (recommended regardless of Q4)**: summary should read _Free bifibration on a functor; focusing/multifocusing canonical forms; KB-completion + Newman canonicity (Thm 3.23); FP decidability frontier (Thms 3.27/3.28); executable enumerator (footnote 3); no monoidal/distributive content; no formalization_ — the current "Bicartesian monoidal coherence" belongs to no content of this paper.
- **Adjacent unregistered line (optional row).** Došen & Petrić, _Coherent Bicartesian and Sesquicartesian Categories_ (arXiv math/0006091 **v5** — cite v5 only; the printed 2001 bicartesian claim is withdrawn in-paper).
  Relevant as the graph-faithfulness/decision precedent for the _groupoid_ statement and as the actual referent of the mis-filed summary.
  Low urgency; iu-side manual work is its first consumer.
- **Register gaps this study states without inventing locators** (the §11.2 discipline of the directed design): (i) Johnstone's FP source (J-42's [26]); (ii) the Dawson–Paré–Pronk free-adjoint/Π₂ line (their [15–16]) — both needed only if/when the FP predicate or the zigzag adjacency becomes citation-bearing; (iii) the transformation-monoid / finite-set-category presentations row — already demanded by `directed-univalence-design.md` §10 Q8; J-42's [52], [54–55] are candidate payers.
- **Out-of-scope flags, recorded so they are not lost.** (a) J-42's zigzag/free-fibrant-double-category construction (Thm 2.8) is the closest prior art for the B10 companions/conjoints gap (`directed-univalence-design.md` §7.2 item 2) — a separate study if B10 wants it. (b) J-42 is pen-and-paper with executable-but-uncertified tooling; together with the unpaid-multiplicative-coherence status of Q-14 this bears on the iu manual's machine-checked novelty claim — iu-side manual work, not gandr design. (c) The Uustalu–Veltri–Wan Agda formalizations named in J-42's references require a claim-scoping verification before any iu-side novelty sentence lands; flagged to the iu track.
