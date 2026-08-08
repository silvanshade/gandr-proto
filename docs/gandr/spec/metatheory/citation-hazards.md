# Citation hazards

Locator defects, version drift, unverified reports, and per-source publication status.
Each entry would otherwise be re-derived at the cost of an hour or a wrong citation; check here before citing anything in its row.

## Publication status that must travel with claims

* [@raynor-2026-nerve] (the circuit-algebra nerve theorem) is a **preprint**; the disconnected nerve exists only there. [@raynor-2025-functorial] and [@raynor-2021-graphical] are published.
* The held arXiv v3 of [@raynor-2021-graphical] is the _corrected_ version and **renumbers against the published version** (Adv.
  Math. 392): the nerve preprint's citations `[41, Prop 5.16]`, `[41, Cor 5.19]`, `[41, Prop 7.15]` resolve in the held v3 as Prop 5.13, Cor 5.18 (connectivity half Cor 5.19), and Prop 7.17 respectively.
  **Its Section 6, the problem of loops, is stable across that renumbering** — checked in the held v3, so the circuit-algebra combinatorics' citation of it resolves as written and needs no translation.
* The unpublished Hackney–Robertson–Stoeckl ∞-props announcement (a fully faithful nerve for props, slide-deck only) is a **watch item**: cite nothing from it.
* The pretype-theory technical report [@nuyts-2026-natpt] is v0.3 and in motion.
* [@mangel-mellies-munch-maccagnoni-2026-hasegawa-thielecke] exists as **two artifacts that are easy to cite as one**, and the entry is the published article.
  The preprint at arXiv:2502.13033 is an _extended version_ "with more illustrations and proofs", its printed title ends "(extended version)", and it is the copy the contributor register holds — so **a locator read from the preprint does not resolve in the published article**, and a claim resting on one of its proofs must say which artifact it read.
  Both of the published article's identifiers name the same work: the publisher identifier and the repository deposit were checked against each other on 2026-08-02.

## Defects in the literature

* A combinatorics survey cites "every Segal space is a decomposition space" at the wrong proposition number in its companion; the statement is two propositions later.
* The computad-pathology result is cited defectively by both readily available routes (a personal letter; mixed metadata); cite the journal article by DOI [@makkai-zawadowski-2008-computads].
* A bipermutative-category source corrects a published claim that _both_ distributors are identities in the matrix model; only one is.
* The tracelet line uses "primitive" and "irreducible" interchangeably and never defines the irreducible object in its freeness theorem; neither local finiteness nor completeness is established there, though both are used [@behr-2019-tracelets] [@behr-kock-2021-tracelet-hopf].
* The tracelet paper's shift-equivalence definition carries **no unit-insertion relation**, and its shift equivalence is explicitly "strictly more general" than classical sequential independence; gandr implements the less permissive trivial-overlap restriction, so the definition and the free-symmetric-monoidal factorization theorem may be cited only at that restriction, never as though the relations coincided (the guards tombstone carries the ruling; this is the definitional evidence).
* **The general spider theorem is a _planar_ theorem, and planarity is what buys its generality.** The statement that a connected diagram equals a standard form with $m$ in, $n$ out and $j$ beads holds for **noncommutative** algebras and **asymmetric** Frobenius forms, and it is proved for planar connected diagrams [@majid-rietsch-2021-planar-spider, cor 2.4].
  The familiar bead-free "collapses to a single $n → m$ generator" is the **special** case, where speciality sets the bead to the unit; citing the special form as though it were the general one silently discards an invariant gandr's own carrier carries as its circle count.
  And citing the general form in a symmetric setting takes the planarity with it — the same substitution the row below forbids, in a second paper.
  **Do not over-correct into refusing the theorem outright.** gandr's own merger is a planar tensor with symmetry recovered at the quotient, so the planar form is consumable at the **representation** layer through canonicalization; what is forbidden is planarizing the **theory** ([[../implementation/circuit-terms#The design questions|circuit-terms-question-21]]).
  The distinction to hold is the corpus's own: ordering is a section, never a planarization.
* **Do not substitute the planar-normalization paper for the tracelet line as the quotient's citation**: the planar paper's quotient is the free _monoidal_ category (Joyal–Street), the tracelet line's is the free _symmetric monoidal_ category — the swap would read as a strengthening while narrowing what `cells_equal` accepts, the false-completeness failure mode in its most dangerous form.
  In the same paper, `[JS78]` is Jones–Singerman, not Joyal–Street; the coherence contribution is the recumbent-isotopy section, and its Joyal–Street coherence statement cites the 1988 unpublished manuscript, not the 1991 journal paper.
* The decomposition-space sentence that licenses planarity is one numbered paragraph **later** than the one a register row recorded, and the same passage states the space is "monoidal but NOT symmetric monoidal" — in direct tension with the free-symmetric-monoidal shift quotient; the corpus's scoping ruling (ordering within a cell only; the parallel-component interface stays symmetric) must not be cited against that passage in either direction.
  The Petri-net monograph's "which monad?" section argues the free-symmetric-monoidal-category monad already gives cartesianness without linear orders — at the price of groupoids rather than sets — so the ordered representation is a two-sided trade, not a free win.
* **The second-order-rewriting paper's own reference list prints its FoSSaCS'11 predecessor at the wrong volume** [@hamana-2022-second-order-rewriting]: the entry gives LNCS 3467, which is the volume of the RTA'05 item two lines above it; the correct locator is LNCS 6604, pp. 381–395, DOI 10.1007/978-3-642-19805-2_26, verified against the publisher record.
  Transcribing the citation from the paper reproduces the defect.
* **The sheaf-noninterference adequacy theorems currently have no standing proof, per the author's own erratum** [@sterling-harper-2022-sheaf]: the erratum [@sterling-2023-sheaf-erratum] — locator fetch-verified 2026-08-02 — states the defect (the admissibility proof for the logical relation on free algebras is not correct, found by Yue Niu in July 2023) is **as-yet unfixed**, with the author's belief that a differently structured adequacy proof exists.
  The earlier form of this row, transcribed from the cost-adequacy paper's related-work text [@niu-sterling-harper-2024-cost-adequacy], read as though the erratum supplied a correction to cite alongside the theorems; the live page says the theorems are currently unproven.
  Theorems 32, 34, and 35 carry the hazard; do not cite any of the three as established.
* The focalisation paper has the two polarity shifts swapped in one proposition and both surrounding bullets; do not copy those labels [@mangel-mellies-munch-maccagnoni-2026-focalisation].
* The planar string-diagram paper carries three initial-matching bibliography keys, and the one attached to its topological theorem is not the coherence source it resembles [@delpeuch-vicary-2022-normalization].
* The nerve preprint's Rmk 8.17 first display appears to index the wrong monad's colimit (connected graphs where all graphs are meant); the conclusion is independently stated in its Rmk 5.18 and does not depend on it — a tension with an obvious resolution, not an error claim.
* Uniqueness of primitive factorization must be cited to the uniqueness lemma, not the existence proposition of the same paper.
* The arities/nerve paper's propositions are cited by a properads paper at numbers that resolve only against the first preprint version — version drift [@berger-mellies-weber-2012-arities].
* The analytic-monads paper's Prop 3.2.10 says "filtered colimits" where its own Def 3.1.1 says "sifted" — filtered is strictly weaker than analytic; quote the definition, not the parenthetical [@gepner-haugseng-kock-2022-analytic].
* Every theorem of the data-types-with-symmetries paper is attributed there to the then-in-preparation analytic-monads paper: cite [@gepner-haugseng-kock-2022-analytic] for the mathematics, [@kock-2012-data-types] for the framing.

## Unverified items, marked

1. **The bipermutative locator and its rigidity package** — **cleared**, by re-checking against the bimonoidal monograph [@johnson-yau-2024-bimonoidal] rather than the PROPs monograph the original report cited.
   The arena is literally the category `Σ′` there, and the three structural locators verify verbatim: **Def I.2.4.18** defines `Σ′`; **Prop I.2.4.23** states it is "a small and tight symmetric bimonoidal category whose additive structure and multiplicative structure are both permutative categories"; **Ex I.2.5.8** states it is a right bipermutative category.
   The row-major index formula is **(I.2.4.19)**, and the split gandr relies on — right distributivity the identity, left distributivity an explicit permutation — is **verbatim in Def I.2.4.18**, with the left distributor given by **(I.2.4.21)**.
   **One part of the recorded repair path was wrong and is corrected rather than carried**: the "nine-versus-three reduced-Laplaza split" is substantively right but was attached to the wrong theorem.
   The Laplaza **reduction** is **Thm I.2.2.13**, and it is 24 axioms to 12, not nine to three.
   The nine-and-three belong to the proof of **Prop I.2.5.7** (every right bipermutative category is a tight symmetric bimonoidal category): three axioms — (2.1.5), (2.1.7), (2.1.13) — hold **by assumption** and are the definitional content, nine are discharged by the identity assumptions on the multiplicative zeros and the right distributor, one more by a lemma, and the rest by the reduction.
   Cite Prop I.2.5.7 for the split and Thm I.2.2.13 for the reduction; they are different facts.
   **Author-pair trap, still live**: the bimonoidal monograph is Niles Johnson with Donald Yau; the PROPs monograph [@yau-johnson-2015-props] is Mark W. Johnson with Donald Yau — different first authors, same second, one citation-slip apart.
2. **Two works by one author pair share an initial-matching key**: the source of the Σ-freeness lemmas cited in the rectification paper is _not_ the bipermutative monograph; resolve before citing either as the other.
3. The Kaufmann–Ward and Bar-Natan–Dancso attributions in the naming note are recall-grade; verify before a citation-bearing surface.
4. The Hasegawa–Thielecke attribution of the shifts-invertible-iff-thunkable-and-linear characterization is recall-grade.
   **Still recall-grade, but now cheaply checkable:** [@mangel-mellies-munch-maccagnoni-2026-hasegawa-thielecke] entered this bibliography on 2026-08-02 as a verified primary that states the theorem it names, and its own statement is that **central and thunkable maps coincide** in a dialogue duploid — read from the artifact's abstract, and _not_ the same sentence as the characterization above.
   Whoever clears this row compares the two rather than assuming the entry settles it.
5. **The polygraph-mechanization datum has no locator yet**: the "one published mechanization of polygraphs in a proof assistant" cited in the coherence-economy section (HITs that do not compute are "not well-suited to intricate uses"; functoriality of the free construction unproved in the cubical setting; zero rewriting or coherence content) reaches this corpus through a scout report, not a held artifact.
   Name the work and give it an entry before the claim is quoted further; the pending sweep that reported the datum is the place to resolve it.

## Further per-source traps

* The planar string-diagram paper's coherence contribution is its **recumbent-isotopy section, not the linear-time word-problem section** — a mid-arc report cited the wrong one and was corrected [@delpeuch-vicary-2022-normalization].
* The axiomatic-rewriting **axiom count is contested**: one presentation prints nine standardization axioms, another describes a ten-item interface; both may be right for different presentations — resolve before citing a count (gates the tile-relation spike).
* The mapping of the abstraction and unit relations onto Lévy counterparts is **open**, not settled: the claim that they have none was refuted by its own verifier against the source's own section; only the shift-equivalence/reversible-permutation relationship is precisely stated.
* The substitude/Feynman-category line contains **no computad or polygraph content** (exhaustively searched); do not cite it for computads.
* The decomposition-space papers contain **no polygraph or rewriting content**; the rewriting bridge is the tracelet line, not the combinatorics line.
* Three refuted over-claims from a factcheck that a fresh document could resurrect: the partition-Lie-algebra link (refuted), "normal ordering is monadic substitution" (refuted), species differential calculus (overstated).
* The rung/shape figure is the polynomial-monads paper's, not the graphs-hypergraphs paper's — the error propagated into session briefs once.
* Two different Weber papers travel under one shorthand: the nerve theorem is the familial-2-functors paper; the weakly-cartesian content is the earlier generic-morphisms paper — and the familial entry in this bibliography is one of the four synthesized rows, so the conflation is live.
* The tracelet paper's tracelet-abstraction definition is one printed page earlier than the page a register row recorded.
* **A register key ending `-reedy` names a paper whose title is about fully faithful functors and pushouts** [@haine-ramzi-steinebrunner-2025-reedy]: the key records what the corpus cites it _for_, not what the work is about, and only its Reedy-extension section bears on gandr.
  Two things must be read from that section rather than from the key: its Reedy-∞ definition is stated as a **proposal**, not shown to hold of any graphical category, so it cannot be cited as the attribution [[../metatheory#The site, the strata, and the fuel are one object|the staging]] owes; and its own remark directs the 1-categorical citation elsewhere, so the paper is the ∞-generalization and not the statement gandr consumes.
  Keys are stable, so this is a reading instruction and not a rename.

## Register gaps

* The **transformation-monoid / finite-set-category presentation** (the directed word problem's classical presentations) has no bibliography row and no formalized rewriting twin; the row must exist before the directed convergence pass can cite it.
  Candidate payers: the structural-focalization and skew-monoidal-sequent lines referenced by [@clarke-scherer-zeilberger-2026-bifibration].
* The **classical bigluing antecedents** of the staged latching/matching construction have no rows, and the corpus currently names them descriptively — which is exactly the unnamed-work defect this document exists to prevent.
  They are the 1-categorical statements the ∞-source's own remark directs its readers to, and they are what `Gandr.UA.Reedy` should cite rather than the ∞-generalization; the rows must exist, verified against their artifacts, before [[../metatheory#The site, the strata, and the fuel are one object|the staging's]] warrant is quoted further.
* The Johnstone source for the unique-diagonal-filler condition, and the Dawson–Paré–Pronk free-adjoint line, are needed only if the tractability predicate or the zigzag construction becomes citation-bearing.
* Four bibliography entries were synthesized rather than copied from the research library: the semirings/rig-groupoids paper, the pretype-theory report, the Cubical Agda paper, and the familial-2-functors paper.
  One has since been locator-verified (the familial entry, at TAC 18(22), Thm 4.10, p. 690); the other three still carry in-file verification notes.

## Verified-at-source registers carried forward

* The Cubical Agda evaluator claims (face-algebra-in-the-evaluator; transp/hcomp reduction; the Glue face-quantifier elimination; the regularity no-go and its documented costs; the `primIdJ`/`Id` retirement chronology) were adversarially verified against a pinned Agda checkout and the tracker; the register lives with the cubical contact analysis in [[ambient-and-primitives]].
* The carrier-side claims of the substrate section (downwardness, no-cup, incidence theorems, palette theorems) are machine-checked in `metatheory/src/Gandr/Shape/*` and need no external citation.
