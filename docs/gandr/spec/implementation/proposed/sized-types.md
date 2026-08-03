# Sized types — the design-pass evidence record

**Proposed.
This document is the evidence record the sized-types design pass consumes, not the design itself.** The normative surface remains the productivity ladder ([[../../surface-language/recursion#The productivity ladder]]): the sized rung is the **ratified destination** (owner ruling, 2026-08-02) and stays gated on the deliberate design pass, whose work item is the tracker's sized-types design-pass task.
Nothing below is normative; every row is an input the pass decides against, recorded here so the pass argues from citations rather than re-deriving or — worse — recalling.

**Provenance and grade discipline.** The rows distill two research passes run 2026-08-02: a literature pass over the sized-types line (acquisitions page-1-verified; locators Crossref-verified as noted per register entry) and a source pass over the Agda implementation, read at `github.com/agda/agda` commit `6e4d6e9543` (a 2026 nightly).
Implementation rows are graded **verified-in-source at that commit**: true of those sources as read, to be re-verified at the pass if load-bearing, and never a claim about other Agda versions.
The full deliberation records, with statement-level detail beyond what is distilled here, are the two research comments on the design-pass tracker item.

## The composition question — sizes with well-founded measures

### sized-evidence-01

**Lexicographic termination over tuples of size expressions is a principled, proved construct.** The reference system makes measures first-class typing data — a measure is a tuple of size expressions attached to a mutual block, uniform in arity across the block, erased after checking — and proves strong normalization by lexicographic ordinal induction, with the size-context **consistency check** (a decidable minimal-valuation judgement) as the soundness gate on reduction under bounded size abstraction [@abel-pientka-2016-wellfounded-sized].
Grade: theorem in the source; the acquired author version is page-1-verified and held in the research inbox.

### sized-evidence-02

**Sizes composed with a user-supplied well-founded measure over _values_ have no uniform published account.** The reference system is non-dependent, so an accessibility predicate over values is not expressible in it at all; the one machine-checked artifact in the record that fuses sizes with value-measure descent does the fusion **by delegation to Agda's syntactic termination checker** and says so [@limperg-2017-cofixpoint].
The literature pass attempted a rescue (a sized accessibility component inside the measure tuple) and refuted its own conjecture for that system.
Grade: theorem on cited plus a verified absence; the refutation is the research pass's own and marked so in its record.

## What the shipping implementation does — Agda at commit 6e4d6e9543

### sized-evidence-03

**Sized and structural descent fuse in ONE mechanism, judged by size-change rather than a literal lexicographic search.** The termination checker reads `Size<` bounds out of the **typing context** and emits them as order entries into the same call matrices structural descent populates (`src/full/Agda/Termination/TermCheck.hs`, `compareVarVar`); a per-argument mask makes arguments ineligible for structural descent while staying eligible for size descent (`src/full/Agda/Termination/Monad.hs`); the criterion is Lee–Jones–Ben-Amram size-change, self-described in-source as strictly more liberal than a lexicographic order.
Consequence for the pass: the shipped fusion answer is "one call graph, mixed evidence, size-change" — not the reference system's block-local measure tuples — and the two answers are not the same thing.

### sized-evidence-04

**Conversion treats sizes by subtyping with constraint postponement — not a quotient, and not irrelevance.** Size equality is defined as inequality both ways; the metavariable short-cut is explicitly disabled at size types; the `Size<` bound's shape-irrelevance is a run-time modality documented as treated relevantly during equality checking; and a search over the implementation and its manual finds **no size-irrelevance discipline at all** in mainline Agda.
Consequence: all three candidate irrelevance disciplines before the pass (below) differ from what the one practical shipping implementation actually does.

### sized-evidence-05

**The implementation exceeds the reference system in five places.** A size maximum operator, closing the reference system's own noted upper-bound gap — but at the **conversion** layer via a backtracking disjunctive check, not in the solver's constraint algebra, which has no join; a hypothesis-graph solver over `Size<` context assumptions with graph-relative least upper and greatest lower bounds; a `Size<`-**inhabitation gate** computing a least valuation of the size context — the implementation's analogue of the reference system's consistency check; a size-metavariable inference layer with staged defaulting to the infinite size; and a separated size universe.

### sized-evidence-06

**Two implementation facts that would silently shape a port.** The solver is invoked with an **empty polarity map**, so every size metavariable with both bounds resolves to its **least** solution — the greatest-solution branch is live only in a standalone harness; and a substantial part of the solver machinery is unreachable from the type checker, including the rule set one would assume is load-bearing.
Consequence: "do what Agda does" underdetermines the pass in exactly the places the pass exists to decide.

### sized-evidence-07

**The infinite-size defect is a disagreement between two layers, and the loci are known.** The solver **rejects** the reflexive bound on the infinite size, while two conversion-layer paths **accept** it (a coercion short-circuit when the bound is the infinite size, and the never-zero check).
This sharpens the sourced hedge's implementation-not-theory determination ([[../../surface-language/recursion#The productivity ladder]]) from an issue-tracker citation to code loci: a consistency-gated design closes exactly the accepting paths.
Recorded as loci only; the demonstrations remain the three cited open issues.

## The irrelevance trilemma

### sized-evidence-08

**Three distinct disciplines are on the table for how sizes vanish from equality, and the pass must pick one knowingly.** Subtyping with postponement (shipped — sized-evidence-04); a symmetric conversion quotient (the productivity ladder's original clause, now reopened as design-pass-owned); and the **two-quantifier** discipline — an irrelevant size quantifier alongside a shape-relevant one, with irrelevant application ignored by judgemental equality, decided by normalization-by-evaluation — which closes the erasure gap the reference system's own text leaves open [@abel-vezzosi-winterhalter-2017-nbe-sized].
The two-quantifier discipline's Agda demonstration runs only under an experimental flag that the safe mode rejects.
The erasure requirement is independently binding on gandr through content-addressing: node identity must quotient sizes ([[../../metatheory#Representation — decidability, the arena, and the layout calculus]] carries the erased-skeleton constraint: content-address on the erased skeleton, erasure before addressing mandatory, binding before any size discipline lands).

## The inference posture

### sized-evidence-09

**Inference-first sized typing has a measured case against it.** The sized-Coq practicality study found severe, algorithmically inherent compile-time cost to inference-first sizes (at least 5.5x on parts of the standard library measured) and recommends **explicit** size quantification with elaborated implicits instead [@chan-li-bowman-2023-sized-coq].
The predecessor position carried into the design-pass scope — inference-only surface sizes with an explicit escape hatch — must be argued against this evidence rather than adopted silently.

## The frontier

### sized-evidence-10

**The infinite-size defect has a live attack, and the syntactic rival has a soundness result of its own.** The large-sizes line removes the largest size entirely — a large type of sizes with parametric quantifiers, consistency by an impredicative realisability model — constructing both inductive and coinductive types, while explicitly not addressing lexicographic measures [@laarakker-otten-vandenberg-2026-large-sizes].
On the rival syntactic side, naive size-change is **unsound** when inductive and coinductive types nest, and the sound adaptation is published with a game-semantic totality account [@hyvernat-2025-size-change-mixed] [@hyvernat-2025-totality-mixed] — binding on any design that keeps a syntactic checker beside the sized one, including the guardedness rung the ladder ships first.

## What the pass decides, and the review this document owes

The design pass's decision list is its tracker item's contract: the two reopened ladder clauses (strict-descent; the irrelevance discipline of sized-evidence-08), the consistency-gate formulation (sized-evidence-01, -05, -07), the fusion mechanism (sized-evidence-03 against sized-evidence-01's measures), the erasure/content-addressing invariant, the solver interface, and the inference posture (sized-evidence-09).
This document was authored at session close from the two research records without an independent fidelity review; **the pass's own review discipline covers it** — each row it leans on is re-verified at its cited source before the decision record cites it as settled, per the standard two-axis pattern.
