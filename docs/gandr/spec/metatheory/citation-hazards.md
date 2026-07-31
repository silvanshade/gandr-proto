# Citation hazards

Locator defects, version drift, unverified reports, and per-source publication status.
Each entry would otherwise be re-derived at the cost of an hour or a wrong citation; check here before citing anything in its row.

## Publication status that must travel with claims

* [@raynor-2026-nerve] (the circuit-algebra nerve theorem) is a **preprint**; the disconnected nerve exists only there. [@raynor-2025-functorial] and [@raynor-2021-graphical] are published.
* The held arXiv v3 of [@raynor-2021-graphical] is the _corrected_ version and **renumbers against the published version** (Adv.
  Math. 392): the nerve preprint's citations `[41, Prop 5.16]`, `[41, Cor 5.19]`, `[41, Prop 7.15]` resolve in the held v3 as Prop 5.13, Cor 5.18 (connectivity half Cor 5.19), and Prop 7.17 respectively.
* The unpublished Hackney–Robertson–Stoeckl ∞-props announcement (a fully faithful nerve for props, slide-deck only) is a **watch item**: cite nothing from it.
* The pretype-theory technical report [@nuyts-2026-natpt] is v0.3 and in motion.

## Defects in the literature

* A combinatorics survey cites "every Segal space is a decomposition space" at the wrong proposition number in its companion; the statement is two propositions later.
* The computad-pathology result is cited defectively by both readily available routes (a personal letter; mixed metadata); cite the journal article by DOI [@makkai-zawadowski-2008-computads].
* A bipermutative-category source corrects a published claim that _both_ distributors are identities in the matrix model; only one is.
* The tracelet line uses "primitive" and "irreducible" interchangeably and never defines the irreducible object in its freeness theorem; neither local finiteness nor completeness is established there, though both are used [@behr-2019-tracelets] [@behr-kock-2021-tracelet-hopf].
* The focalisation paper has the two polarity shifts swapped in one proposition and both surrounding bullets; do not copy those labels [@mangel-mellies-munch-maccagnoni-2026-focalisation].
* The planar string-diagram paper carries three initial-matching bibliography keys, and the one attached to its topological theorem is not the coherence source it resembles [@delpeuch-vicary-2022-normalization].
* The nerve preprint's Rmk 8.17 first display appears to index the wrong monad's colimit (connected graphs where all graphs are meant); the conclusion is independently stated in its Rmk 5.18 and does not depend on it — a tension with an obvious resolution, not an error claim.
* Uniqueness of primitive factorization must be cited to the uniqueness lemma, not the existence proposition of the same paper.
* The arities/nerve paper's propositions are cited by a properads paper at numbers that resolve only against the first preprint version — version drift [@berger-mellies-weber-2012-arities].

## Unverified items, marked

1. **The bipermutative locator and its rigidity package** [@yau-johnson-2015-props]: reported by a scout whose adversarial verifier died mid-run; two of its three sibling reports came back overstated.
   Partial repair: the two Σ-freeness results it depended on are independently cited and relied on by [@chu-hackney-2021-rectification], so the package survives; only the proposition number wants confirmation.
   Do not cite outside the repository until re-checked.
2. **Two works by one author pair share an initial-matching key**: the source of the Σ-freeness lemmas cited in the rectification paper is _not_ the bipermutative monograph; resolve before citing either as the other.
3. The Kaufmann–Ward and Bar-Natan–Dancso attributions in the naming note are recall-grade; verify before a citation-bearing surface.
4. The Hasegawa–Thielecke attribution of the shifts-invertible-iff-thunkable-and-linear characterization is recall-grade.

## Register gaps

* The **transformation-monoid / finite-set-category presentation** (the directed word problem's classical presentations) has no bibliography row and no formalized rewriting twin; the row must exist before the directed convergence pass can cite it.
  Candidate payers: the structural-focalization and skew-monoidal-sequent lines referenced by [@clarke-scherer-zeilberger-2026-bifibration].
* The Johnstone source for the unique-diagonal-filler condition, and the Dawson–Paré–Pronk free-adjoint line, are needed only if the tractability predicate or the zigzag construction becomes citation-bearing.
* Four bibliography entries were synthesized rather than copied from the research library and carry in-file verification notes: the semirings/rig-groupoids paper, the pretype-theory report, the Cubical Agda paper, and the familial-2-functors paper.

## Verified-at-source registers carried forward

* The Cubical Agda evaluator claims (face-algebra-in-the-evaluator; transp/hcomp reduction; the Glue face-quantifier elimination; the regularity no-go and its documented costs; the `primIdJ`/`Id` retirement chronology) were adversarially verified against a pinned Agda checkout and the tracker; the register lives with the cubical contact analysis in [[ambient-and-primitives]].
* The carrier-side claims of the substrate section (downwardness, no-cup, incidence theorems, palette theorems) are machine-checked in `metatheory/src/Gandr/Shape/*` and need no external citation.
