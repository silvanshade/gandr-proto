# Directed univalence — a design study for the identity layer and the Temporal Univalence re-entry

> **Status: PROPOSAL for owner review — not a decision record.** Research deliverable for gandr-wvd.21 (buildout epic gandr-wvd, minted from the gandr-fcw.11 resolution).
> Its consumers are named: (1) the **B7 identity-layer build phase** (gandr-wvd.7 — Path/walk + groupoid combinators + directed, both from the start, per `bd comments gandr-fcw.11`), and (2) the parked **metatheory track's re-entry** (gandr-fcw.12; program name settled as **Temporal Univalence**, natural earliest re-entry after B9 kernel-replay).
> Nothing here is decided; §10 separates open questions from recommendations, and every recommendation is priced.
>
> **Alias key**: `iu:` = the internal-univalence Agda library (canonical checkout, branch `main`); `iu-notes:` = its intake-ledger notes repo; `wyrd@failed-refactor:` = the wyrd worktree carrying the pre-reboot gandr crates and spec corpus.
> Literature citations use register keys from `docs/research/bibliography-v2.md` with locators verbatim from the register.
> No machine-local paths appear in this file.
>
> **Provenance note (ua-base copy used).** The settled groupoid statement was read from `iu:docs/spec/DESIGN-ua-base-vocabulary.md` **on the canonical checkout's `main`**, whose last amendment is the R-coh/distributor row (2026-07-17).
> `git log --all` over the iu repo confirms no branch — including the agent worktree branches — carries a newer copy of that file; the canonical copy is authoritative.
>
> **Provenance note (pass 2 — survey completion and corrections).** This revision completes §§6-7 and §§9-10 against a source survey of the pre-reboot tree and re-verifies the iu-side state; two pass-1 claims are corrected. (1) §1 understated the settled directed scaffolding ("exactly one sentence of direction"): the surface vocabulary (ADR-79), the stage gate (ADR-76 D5), and the representation home were already fixed pre-reboot; what remains undesigned is the statement's _content_, so the study's premise stands as restated. (2) All faithfulness/η status claims now carry an **as-of 2026-07-19** qualifier: the η closure is actively in motion iu-side — the conditional development is `--safe`-green with seven residual `letterC` walls (down from eight on 2026-07-17), `perm-hom` unconditional, and one wall reduced to a single inductive hole in uncommitted work — verified against the canonical checkout's `main` (tranche modules last amended 2026-07-17) and every live worktree branch (none carries newer tranche content).
> Minor: §5.3's guard-filing target is restated from "wyrd-side" to buildout-side — the pre-reboot tree is a read-only source; the phases that can carry the witness live in the reboot.

## 1. The settled groupoid statement, and the question this study answers

The groupoid/iso case is fixed (iu-notes:WYRD-INTAKE.md W3; `iu:docs/spec/DESIGN-ua-base-vocabulary.md` §§1–6): **ua-base** is the protype isomorphism `CodeIso(x ⨟ y) ≅ (x ⤳ y)`, decomposed as

* **O1 — sound realization**: `⟦−⟧ : (x ⤳ y) → CodeIso(x ⨟ y)`, iso-valued by construction (every edit generator names a translator pair with round-trip evidence);
* **O2 — fullness**: every certificate in the fixed iso stock is replay-equal to a realized path;
* **O3 — round trips**: a section `edit` with **β at replay-equivalence `≈ʳ`** and **η at rule congruence `≈ᶜ`** — never code equality on either side;

with **three quantifier alphabets fixed in the statement** (the eq² discipline, `DESIGN-ua-base-vocabulary.md` §2): paths **certificate-generated**, instances **saturated** (modules over the path relation = the store being a profunctor, W2), isos **leaf-natural**.
The executed demonstrator is `iu:src/Internal/UaBase/` — nine gate-green leaf modules (`Code`, `Value`, `Iso`, `Edit`, `Realization`, `Normalize`, `Complete`, `Rules`, `Negation`) proving O1, O2 (absolute on the leaf-free toy fragment), and O3-β, plus four η-tranche modules (`Faithful`, `WordProblem`, `Canonical`, `Coxeter`) in which η is **staged**: `--safe`-green as a _conditional_ development, reduced — **as of 2026-07-19, with the closure actively in motion** — to the **seven `letterC` walls** of the `Discharge` module (`iu:src/Internal/UaBase/Canonical.agda:1647-1662`; W14; iu bead `iu-c2h.1`), with the Sₙ Coxeter coherence (`perm-hom`) discharged **unconditionally** (`iu:src/Internal/UaBase/Coxeter.agda:1446-1664`).

Nobody has designed the **directed** analog — though more of its scaffolding is settled than pass 1 of this study recorded (corrected here; see the provenance note).
The pre-reboot corpus fixes the directed **surface vocabulary** (ADR-79: the former is `Step`, deliberately with **no `back`** — "the asymmetry of the directed family is visible in the vocabulary itself"; transformation notation `A ~~> B`; and the statement's outer shape already spelled, `Step Type A B <~> (A ~~> B)`, outer connective groupoidal), the **stage gate** (ADR-76 D5: directed identity of codes gated on the levitation stage; ADR-79's honesty gate: `Step` enters the surface only when its rules land), and the **representation home** (the reflected directed ladder surveyed in §7, whose own module records directed univalence as `homSet(A,B) ≅ (A ⇒ B)`, _direction only, not scoped_, with two named prerequisites — §7.2 item 5).
What no source designs is the statement's **content** — the protypes, quantifier alphabets, obligations, and guards — and the owner resolution at gandr-fcw.11 orders B7 to build groupoid and directed identity **both from the start**, with no metatheory gate.
This study scopes the directed statement, fixes its quantifier alphabets, and lays out what B7 builds, B9 carries, B10 consumes, and the Temporal Univalence re-entry must eventually prove.

## 2. The directed landscape — what constrains the design

Four independent sources triangulate what "directed univalence" has to mean here.

1. **The literature statement shape (Q-15).** Gratzer–Weinberger–Buchholtz, _Directed Univalence in Simplicial Homotopy Type Theory_ (arXiv:2407.09146), Definition 1.2: a universe `S` is **directed univalent** if the directed-interval hom `𝕀 → S` is isomorphic to `Σ_{A,B:S} A → B` over `S × S` — _homomorphisms in the universe are ordinary functions_, and the universe must be used **covariantly**.
   Their _structure homomorphism principle_ is the directed SIP: terms over a type of structures are automatically functorial in structure homomorphisms, not merely invariant under isomorphism.
   This is the certificate-free, spatial-model rendering of the target; the design below is its certificate-generated, temporal rendering.
2. **The dinaturality steering (Q-3, W12).** Laretto–Loregian–Veltri (arXiv:2409.10237) give directed J and locate the **composability boundary**: entailments (dinaturals) compose unconditionally over groupoid domains (their Thm 4.5 — landed internally as `seq-dinat`, `iu:src/Internal/Profunctor/Dinatural.agda`) but only under an acyclicity/loop-freeness gate in general (their Thm 5.3; Danos–Regnier pedigree, Q-10).
   ADR-69's two composition modes are exactly this pair (W12).
   Consequence for the design: **directedness costs nothing at dimension 1** (paths are strings; strings concatenate) — the boundary bites at _cell and certificate composition_, i.e. at B9/B10, not at the path protype.
3. **The VDC representation home (Q-2 primary, Q-1 secondary, Q-8).** The FVDblTT line gives hom-formation as _unit + restriction_: in a virtual double category the unit protype restricted along tight maps yields the hom stock, and companions/conjoints are the one-sided graphs of tight maps.
   The directed statement should therefore read as **stratified fullness at the unit-plus-restriction fragment** — one constructor class beyond the unit fragment at which the groupoid O2 is stated (`DESIGN-ua-base-vocabulary.md` §1 cites fullness "at the unit fragment"; the directed statement widens the fragment, not the discipline).
4. **The temporal program's own prediction (P4).** `iu:docs/spec/DESIGN-temporal-univalence.md` §5 item 6 reads directedness as time-asymmetry — "directed univalence's resistance is the _doubly_ temporal problem — identity as irreversible development" — and §6 P4 predicts that full composition of entailments is available exactly on the reversible (groupoid) fragment.
   The design below is P4's first engineered instantiation: it is falsifiable in P4's terms, and the metatheory re-entry inherits P4 as its validation criterion (§8.3).

## 3. The substrate is already directed — the evidence, and the exact sense in which the directed statement is primitive

The question posed by the fcw.11/fcw.15 tension register (`iu:docs/spec/PLAN-temporal-realignment.md` §2 T1) is whether the rewriting substrate is _naturally directed_ — paths before symmetrization — so that the directed statement is the primitive one.
The answer from the executed demonstrator's own source is **yes, with a precise qualifier**: the groupoid statement is a _localization of the directed statement's evidence-invertible restriction_, not the other way round, and not a quotient.

The evidence, each item cited to the executed code:

* **E1 — generators are oriented.** The edit alphabet is `data EditGen : Code → Code → Set` with directed schemas (`⊗assoc : EditGen ((c ⊗ d) ⊗ e) (c ⊗ (d ⊗ e))`, `dist : EditGen (c ⊗ (d ⊕ e)) ((c ⊗ d) ⊕ (c ⊗ e))`, …), and the in-code comment says it outright: _"each a directed schema with a formal inverse under `bwd`"_ (`iu:src/Internal/UaBase/Edit.agda:46-58`).
* **E2 — symmetrization is free doubling, not substrate.** Paths are words over the **involutive character alphabet** `Char` (`fwd`/`bwd`, `iu:src/Internal/Rewriting.agda:98-106`) — the _free strict involutive word category_, the module header's own phrase.
  `w ++ inv w` is **not** `[]`: cancellation is imposed one dimension up, as the oriented rules `cancelˡ`/`cancelʳ : (inv w ++ w) ⇒² []` inside the rule congruence (`iu:src/Internal/UaBase/Rules.agda:370-371`).
  The groupoid content of ua-base is therefore _rules over a directed substrate_, not a symmetric substrate.
* **E3 — the design already mandated directed paths; the code deviated.** `DESIGN-ua-base-vocabulary.md` §7 requires _"Paths are `Step*`-shaped"_ — the purely directed snoc kit with **no** inverse constructor (`here`/`then`, `iu:src/Internal/Step.agda:64-66`) — while the executed `Edit x y = Word ℰ x y` uses the involutive `Word` instead (`iu:src/Internal/UaBase/Edit.agda:72-73`).
  The directed statement _redeems the design's own substrate mandate_ rather than departing from it.
* **E4 — even the certificate grade is one-sided.** Replay-equivalence, the grade at which β is stated, compares **only the forward translator**: `f ≐ g = (a : A) → f .to a ≡ g .to a` (`iu:src/Internal/UaBase/Iso.agda:39-41`).
  The groupoid demonstrator never inspects the inverse when comparing certificates.
* **E5 — the saturation alphabet is already directed.** Saturation = the store being a profunctor (W2), and the profunctor module structure is variance-typed with no inverses anywhere: `actˡ` contravariant, `actʳ` covariant, laws `act-idn*`/`act-seq*`/`act-xchg` (`iu:src/Internal/Profunctor.agda:52-123`); the unit-UP discharge is the Yoneda correspondence whose extension map is one covariant action, `yoneda-from w .cmp h = P .actʳ (w .cmp a) h` (`iu:src/Internal/Profunctor/Yoneda.agda:50-51`), and whose round trips are `act-idnʳ` plus dinaturality — no symmetry consumed.
* **E6 — the directed precedents are in-tree.** `iu:src/Internal/Profunctor/Tabulator/Path.agda:29-33` is _directed path induction_ whose groupoid instance _specializes_ to the symmetric J; `iu:src/Internal/Profunctor/Tabulator/Pi.agda:21` declares _"EVERYTHING IN THIS MODULE IS DIRECTED — no inverses appear anywhere"_ and builds Π as a right extension along the display map's **conjoint**.
  The ratified charter language is already "directed core, groupoidal overlay" (`PLAN-temporal-realignment.md` §2 T1, §9 ADR-12 D5) — and the pre-reboot corpus carries the same finding as a named, load-bearing item: _"gandr's type-level operators are directed already; invertibility is an **overlay** (ADR-69's invertible flag), not the default"_ (wyrd@failed-refactor:docs/gandr/spec/proposal-identity-univalence.md:198-200, the item titled exactly "Directed core, groupoidal overlay").

**The exact primitive-to-derived relation.** Write `ℰ⇝` for the directed edit alphabet (§5.1) and `ℰ ⊂ ℰ⇝` for its **evidence-invertible sub-alphabet** — the generators carrying a designated inverse schema _plus per-schema round-trip evidence_ (the O1 hard constraint of the groupoid vocabulary).
Then:

> The groupoid path protype `x ⤳ y` is the **free involutive doubling of the `ℰ`-restricted directed protype, with cancellation adjoined as dimension-2 rules** — a polygraphically-presented groupoid completion (localization) of `(x ⇝ y)|ℰ`.
> The localization is _licensed_ by the round-trip evidence: `bwd` letters realize because `real-char (bwd g) = real-gen g ⁻¹` (`iu:src/Internal/UaBase/Realization.agda:144`), i.e. formal inverses are sound only where the generator already carries an inverse-valued realization.
> For the genuinely one-way generators of `ℰ⇝ ∖ ℰ` no such evidence exists, the doubling is unavailable, and **no construction recovers the directed statement from the groupoid one**.

Two readings are thereby rejected honestly:

* _Groupoid-as-quotient fails._ `x ⤳ y` is neither a sub-protype nor a quotient of `x ⇝ y` — it has more letters (the `bwd` half) and more rules (cancellation); the correct arrow is localization of a restriction.
* _Directed-by-forgetting fails._ One cannot obtain the directed statement by discarding inverses from the groupoid one: the one-way generator classes (§5.1) and their rule layer are strictly new material, and the fullness quantifier ranges over a strictly larger certificate stock (§5.3).

## 4. The directed statement — candidates, comparison, recommendation

### 4.1 The two protypes, replaced

* **`CodeHom(x ⨟ y)` — the directed certificate stock.** Translator _singletons_: a forward map with replay evidence on the fragment, no inverse and no round-trip demand.
  Identity is **replay-equivalence `≈ʳ` unchanged** — E4 shows the groupoid grade already compares only the forward map, so `≈ʳ` transfers verbatim.
  Note `≈ʳ` remains an _equivalence relation on each hom stock_; directedness lives in the 1-cells' orientation, not in the comparison grade.
* **`x ⇝ y` — the directed path protype.** Certificate-generated **positive words** (`Step*`-shaped, E3 — no `bwd` half) over the directed vocabulary `ℰ⇝`, compared by the directed rule congruence `≈ᶜ⇝` generated by an oriented dimension-2 rule layer (§5.1).

### 4.2 Candidate statements

**D-A — the maximal (GWB-shaped) statement.** `CodeHom(x ⨟ y) ≅ (x ⇝ y)` with the O2 quantifier over _all_ replay-total translators.
This is Q-15's Definition 1.2 transplanted: hom-certificates ≅ functions.

* On the **leaf-free** fragment this is the right absolute target (no leaves for naturality to constrain; the stock is all functions between finite value sets, and generation is elementary).
* Over leaves it is **refuted at the first infinite leaf**, by the directed sibling of the leaf-shift witness (W1): a **constant-literal translator** `x ⇝ Integer-leaf` (send everything to `17`) is replay-total and replay-distinct from every structurally-generated map, but is not uniform in the _target_ leaf contents — no finitary vocabulary reaches it.
  Verdict: keep D-A as the leaf-free absolute form; as the general statement it is D-C minus the fence, i.e. false.

**D-B — the lax/adjoint statement.** Require every one-way generator to carry one-sided adjoint evidence (a designated section or retraction), and state the round trips one-sidedly — β in one direction at `≈ʳ`, the other direction a comparison 2-cell.

* This is the companion/conjoint-*pair* reading, and it is the right shape **at the VDC face** for restrictions of realized tight maps (§7) — but as the base statement it fails the grade discipline: η would live at a lax 2-cell grade, adjoint choices are not schema-uniform (a non-injective, non-surjective translator has no canonical section _or_ retraction), and the statement stops being an isomorphism of protypes at stated grades.
  Verdict: decline as the ua statement; retain the content as the equipment structure the reflection face hosts.

**D-C — the fenced directed statement (RECOMMENDED).**

> **ua-dir (base-stratum, μ-free).** Over the same Desc fragment as ua-base, with the certificate alphabet fixed to the **leaf-natural one-way stock** (§5.3): the realization `⟦−⟧ : (x ⇝ y) → CodeHom(x ⨟ y)` of the directed edit polygraph `ℰ⇝` is
>
> + **O1⇝ sound**: every generator schema names a translator with replay evidence; paths realize by (unconditional) composition of translators;
> + **O2⇝ full**: every leaf-natural one-way certificate is replay-equal to a realized positive word — fullness of the D1 interpretation at the **unit-plus-restriction fragment** over the Desc signature;
> + **O3⇝ sectioned**: the constructed `edit⇝` satisfies **β at `≈ʳ`** and **η at `≈ᶜ⇝`** — the grade discipline verbatim from the groupoid case, never code equality;
>
> and additionally
>
> + **O4 — core coincidence (new, directed-only)**: the canonical comparison from the groupoid statement into the invertible core of the directed one — `(x ⤳ y)/≈ᶜ → core((x ⇝ y)/≈ᶜ⇝)` on paths, `CodeIso ↪ CodeHom`-image on certificates — is a bijection at the stated grades.
>   Concretely: every positive word whose realization is invertible is `≈ᶜ⇝`-congruent to (the positive image of) a groupoid-vocabulary word.

**What replaces symmetry: nothing at dimension 1, deliberately.** `sym` is overlay data — available exactly on the evidence-invertible sub-alphabet, via the localization of §3 — and its absence from `ℰ⇝` is the entire directed content.
At dimension 2 the rule layer **remains an equivalence** (`≈ᶜ⇝` is symmetric, like `≈ᶜ` — `iu:src/Internal/UaBase/Rules.agda:461-479` closes the oriented `⇒²` generators under `sym²`): 2-cells between parallel directed 1-cells stay invertible at this stratum.
This is the (n, p) dial reading — the directed case moves the invertibility threshold up one dimension, it does not delete invertibility from the theory.
The alternative (genuinely lax 2-cells, pushed by the produoidal laxity results, W18) is declined for the _statement_ and re-surfaces where it belongs, at certificate composition (§8.1); it is listed as an owner question (§10 Q4).

**Why O4 is genuinely new work.** Invertible realizations arise from non-invertible letters: on the leaf-free fragment, `fold ∘ inj₁ : 𝟙 ⇝ 𝟙 ⊕ 𝟙 ⇝ 𝟙` realizes the identity while neither letter is invertible.
O4 therefore needs the directed rule layer to contain the simplicial-identity-style cells (`fold ∘ inj ⇒ refl`, …) _and_ a directed word-problem argument that they suffice on the invertible-realization sub-stock.
This is the directed sibling of the `NFC` faithfulness reduction (W14) and it is metatheory-track work, not B7 work (§9).

## 5. The directed quantifier alphabets

The eq²/alphabet-first discipline is unchanged: fix every quantifier's range in the statement, price each failure with a witness.
The three groupoid alphabets transfer as follows.

| alphabet                              | groupoid (settled)                                                                      | directed analog                                                                                                                                                                                                                                                                       | failure if unfixed — the witness                                                                                                                                                                                                                                                                                                                          |
| ------------------------------------- | --------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **paths**                             | free involutive words over `ℰ` mod `≈ᶜ`                                                 | **positive (`Step*`-shaped) words over `ℰ⇝`** mod `≈ᶜ⇝` — no `bwd` half; invertibility per-generator overlay data                                                                                                                                                                     | a structural/thin hom protype (subtyping-style code ordering) is the directed blanket base: it collapses the stock at `(Bool, Bool)` where **four** replay-distinct one-way certificates exist (`id`, negation, `const true`, `const false`) against two invertible ones — the **constant-map witness**, the directed sibling of the U3.0c negation guard |
| **instances**                         | saturated: modules over the path relation = profunctor structure (`actˡ`/`actʳ` + laws) | **unchanged in form** — the profunctor module structure is already variance-typed (E5): `actˡ` along paths into the source, `actʳ` along paths out of the target, and the directed-J/unit-UP discharge is the (co)Yoneda extension, which consumes no inverses                        | identical to the groupoid failure: unit induction declines on non-empty directed paths over raw generating instances (the F0 law-5 shadow); the repair is the same absorbed-path module element                                                                                                                                                           |
| **certificates** (the O2⇝ quantifier) | leaf-natural isos                                                                       | the **leaf-natural one-way stock**: translators uniform in leaf contents on _both_ sides; structurally these are container-morphism-shaped — summand-map forward, factor-map backward (N-2, doi:10.1016/j.tcs.2005.06.002; the directed-container adjacency is N-8, arXiv:1604.01187) | over unrestricted closures the **constant-literal witness** refutes completeness at the first infinite leaf (§4.2 D-A); over leaf-free codes the restriction is vacuous, exactly as in the groupoid case                                                                                                                                                  |

### 5.1 The directed vocabulary `ℰ⇝`

`ℰ⇝` extends the groupoid generator menu (`DESIGN-ua-base-vocabulary.md` §4) with the one-way classes; every generator remains a directed schema applied positionally under one-hole contexts, and the evidence-invertible classes carry their inverse-plus-round-trip overlay unchanged.

| class                                               | generators               | realization                        | evidence class                                             |
| --------------------------------------------------- | ------------------------ | ---------------------------------- | ---------------------------------------------------------- |
| E-perm, E-assoc, E-unit, E-comm, E-dist, E-retrofit | as settled               | as settled                         | invertible (overlay: inverse schema + round-trip evidence) |
| **E-proj**                                          | `A × B ⇝ A`, `A × B ⇝ B` | component projection               | one-way                                                    |
| **E-dup**                                           | `A ⇝ A × A`              | diagonal (read one position twice) | one-way                                                    |
| **E-inj**                                           | `A ⇝ A + B`, `B ⇝ A + B` | summand injection                  | one-way                                                    |
| **E-fold**                                          | `A + A ⇝ A`              | codiagonal (case-collapse)         | one-way                                                    |

On the leaf-free fragment these generate **all** functions between the value sets (any map factors through its image; injections + codiagonals + permutations suffice), so O2⇝ has an absolute leaf-free form exactly as O2 does.
On leaved codes the stock generated is the container-morphism stock of §5's table, and leaf-naturality is the exact fence: E-proj/E-dup move _positions_, never leaf values.

**The dimension-2 rule layer `⇒²⇝`** extends R-cancel/R-comm/R-coxeter/R-coh with the one-way coherence classes: the simplicial-style identities relating E-inj/E-fold and E-proj/E-dup to each other and to the permutation subsystem (face/degeneracy-style relations), the bialgebra-style exchanges between the product and sum one-way classes across E-dist, and the naturality squares of the new classes against the structural subsystem.
**The word-problem cost jump is the honest price of the directed case**: the residual group of the normalize–conjugate–decide route grows from the symmetric group (R-coxeter — classical, convergent, with a formalized additive twin, W15/Q-14) to the **full transformation-monoid/finite-set-category word problem** on sorted normal forms.
Presentations of this monoid are classical, but the register holds **no row** for them and no formalized rewriting twin is known to this study (§11.2); the convergence pass for `⇒²⇝` is genuinely open metatheory, and the η staging precedent (`iu-c2h.1`: land generators with per-generator soundness first, stage the convergence walls) is the planning template.

### 5.2 Grades

β at `≈ʳ` (one-sided replay, E4), η at `≈ᶜ⇝` — the grade table of the groupoid statement carries over unedited, and the same two collapse guards apply: `≡` of certificates is the code-equality collapse (U3.0c side), `≡` of paths would demand strictness the free directed fragment does not have.

### 5.3 Degeneracy guards (the W16 caveat, directed)

The univalent-typoids precedent (Q-20, arXiv:2205.06651) carries a load-bearing caveat in the groupoid case: its `Ua` targets the _ambient identity type_, which Hedberg degenerates over decidable codes (W16).
The directed case splits this into two sharper facts:

* **There is no ambient crutch to degenerate to.** MLTT has no native hom former, so a directed `Ua` _cannot_ target ambient identity — the directed statement is forced into the typoid-function/Rezk-completion direction (realization-as-functor) from the start.
  What was a caveat in the groupoid case is a structural feature here.
* **The new degeneracy is thinness.** The directed analog of "UIP-by-stealth" is _poset collapse_: if the hom protype is proof-irrelevant (thin), ua-dir degenerates to an order-rigidity statement — the directed analog of the T2 rigidity horn (`PLAN-temporal-realignment.md` §2 T2).
  The constant-map witness (§5 table) is the permanent guard: four replay-distinct parallel certificates at `(Bool, Bool)` refute any thin rendering.
  Recommendation: file it buildout-side as a permanent negative test beside U3.0c and the leaf-shift witness (W1), at whichever phase first carries a directed hom stock (§10 Q7).

## 6. B7 shape — judgment forms, term language, and the kernel annotation slot

### 6.1 The landed groupoid core — port-ready typing, unpaid dynamics

B7 does not start from nothing.
The pre-reboot tree carries a complete groupoid identity layer whose **typing side is port-ready**:

* **Former.** `Path A x y` is the first dependent value former (rule `Path-Form`, ADR-76), with endpoints **invariant under subtyping** — _"covariant widening is unsound without transport"_ is recorded at the former itself (wyrd@failed-refactor:crates/gandr-core/src/types.rs:180-202).
  That invariance note is the groupoid core already refusing to fake directedness: covariant endpoint-widening _is_ a `Step`-shaped transport, and the core declines it until the directed family provides it honestly.
* **Intro / elim.** `Value::Here` (wyrd@failed-refactor:crates/gandr-core/src/syntax.rs:803, builder :1036-1042) and `Comp::Walk` with the `WalkMotive`/`WalkBase` binder pair (:1702, :1216, :1238) — the **dinatural ML-J primitive** of ADR-76 D4, motive over both endpoints and the path (`walk` delivers `C[a/x][b/y][p/q]`, :1688-1689).
  The dinatural (unbased) form was chosen precisely to keep the groupoidal core aligned with a directed reflected layer (wyrd@failed-refactor:docs/gandr/spec/proposal-identity-univalence.md:95-96); it is E6's directed-path-induction shape one level down.
* **Rules.** Rule `Walk` in the typing machine (wyrd@failed-refactor:crates/gandr-core/src/machine.rs:1678-1681; frame pops :2437, :2473; suspended frames :558-582), lock-step with the recursive checker.
* **The K fence.** The `case`-on-`Path` decline carries the literal `without-k` substring, asserted by the pathological corpus witness `pathological/identity/k-derivation.gandr` (wyrd@failed-refactor:crates/gandr-core/src/error.rs:126-144) — ADR-76 D4's binding adequacy obligation, executed.
* **Vocabulary.** ADR-79 fixes both families' spellings before any directed rule exists: groupoid `Path`/`here`/`walk`/`then`/`back`; directed former `Step`, **no `back` by design**, transformation notation `~~>`.

**The unpaid half is the dynamics.** The pre-reboot sequent lane declines any program mentioning `Value::Here`/`Comp::Walk` **whole** (`FocusOrigin::Unsupported`; wyrd@failed-refactor:crates/gandr-sequent/src/focus.rs:1644-1703) — L-machine Walk-β was explicitly the _other_ lane's work and never landed.
Since the reboot's B1 retires the CEK in favor of the L machine (`PLAN.html` §4, B1), **B7 must build the identity layer's L-machine dynamics new** — Walk-β and the directed eliminator's β — not port them.
Pass 1 could not see this; it changes the B7 estimate, not the design.

### 6.2 Judgment forms — the two families side by side

| form            | groupoid (settled, ADR-76/79)              | directed (this design)                                                                                                                                                                               |
| --------------- | ------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| former          | `Path A x y`, endpoints invariant          | `Step A x y` (ADR-79 D2), endpoints invariant **at B7** — the contravariant-source/covariant-target reading (§7's `DirectedHom` slots) becomes load-bearing only at the reflected layer              |
| intro           | `here(v) : Path A v v`                     | the diagonal intro `Step A v v` (spelling open — §10 Q2); forced by the hom-with-J shape, it is the pre-reboot `DirectedHom::refl` (wyrd@failed-refactor:crates/gandr-vdc/src/directed/hom.rs:75-85) |
| elim            | `walk` — full dinatural ML-J               | the directed walk (spelling open — §10 Q2) with the **motive-covariance side condition**: a motive placing the moving endpoint in the contravariant source slot is refused                           |
| β               | on `here`                                  | on the diagonal intro                                                                                                                                                                                |
| composition     | `then`, derived by one `walk`              | derived by the same one-walk script (covariant-target motive) — composition **is** covariant transport; the dimension-1 free lunch of §2 item 2                                                      |
| inversion       | `back`, derived                            | **none — underivable by construction**; the refused motive shape _is_ the symmetry shape                                                                                                             |
| permanent guard | `k-derivation.gandr` MUST fail elaboration | a `back`-derivation witness MUST fail elaboration (new, recommended binding — the directed sibling of the K witness)                                                                                 |

Two facts make the directed column cheap to build:

* **The side condition is term-structural and total — no variance-sorted contexts needed at B7.** The pre-reboot tree proves this implementable: `MotiveShape` (`CovariantTarget`/`Constant` admissible, `ContravariantSource` refused), `check_directed_j` as a total checker returning `JError::MotiveNotCovariant`, and named property witnesses (`symmetry_is_never_derivable`) — wyrd@failed-refactor:crates/gandr-vdc/src/directed/hom.rs:105-117, :230-251, :227-228.
  The polarity check inspects only the motive's use of the moving endpoint; contexts stay variance-blind.
* **The design stance is already precedented twice**: the without-K unifier refuses the deletion step exactly as the polarity condition refuses the contravariant motive — the pre-reboot module records the analogy in so many words (wyrd@failed-refactor:crates/gandr-vdc/src/directed/hom.rs:10-24).

**Variance staging** (what lands where): B7 — none (motive-shape check only, per the above); B10 — variance-sorted contexts on the **reflected layer only**, porting the Ł1 design record that `−ᵒᵖ` lives on reflected signatures and the frozen core is untouched (§7.1); metatheory — the general dipresheaf variance judgment.

**Coexistence mechanics**: `Path` and `Step` land as **independent primitive formers**.
No kernel coercion `Path A x y → Step A x y` lands at B7: §3's localization relation is O4's territory — a _theorem about realizations at the code stratum_ — and a kernel-level bridge would smuggle the core-coincidence statement in as an axiom, exactly the posture ADR-76 forbids ("never an axiom").
Whether such a coercion is ever wanted as a _derived_ form is an owner question (§10 Q3).
The §5.3 degeneracy witnesses (constant-map, constant-literal) are **not statable at B7** — they quantify over a directed certificate stock, which needs the code universe; their landing phase is §10 Q7.
B7's permanent guards are the two derivation witnesses in the table.

### 6.3 The S1/export-format directedness slot — the priced call

**Recommendation: yes — reserve one variance/directedness annotation slot in the S1/export format at B2**, with the honest note that B7 itself never reads it.

* **The moment is singular.** The S1/export format _already_ reserves annotation slots as a B2 design act — erasure and modes/grades (`PLAN.html` §4, B2 row) — so the reserved-slot vocabulary is open on the table exactly once.
* **What the slot is NOT for.** `Step` and its eliminator are _term constructors_; they ride the standing kernel subset-growth obligation at B7 like every other former, and need no reservation.
* **What it IS for.** The polarity/variance **annotation plane**: B9's variance-marked certificate boundaries (§8.1 item 1) and B10's variance-sorted reflected contexts annotate binders and boundaries _orthogonally to term structure_ — the same annotation shape as erasure and modes/grades, which is why the same mechanism fits.
* **Cheap now.** One reserved tag plus one format-doc line; semantics-free; compatible with ADR-79's honesty gate (the gate fences surface _rules_, not schema reservations — a reserved slot front-runs nothing).
* **Expensive later.** From B9 the format has **two independent consumers** — the kernel's export writer and the kernel-replay second checker (`PLAN.html` §4, B9; §6: the certified TCB is "S1 term language, conversion, replay").
  Retrofitting an annotation plane after that is a coordinated format bump across both checkers, on the TCB.
* **The honest alternative, priced.** Nothing before B9 reads the slot, so deferring to B7 (or even B9 itself) is _workable_ — but the deferral saves one line of schema and bets that B3–B6 consumers ossify no format assumptions; the price asymmetry says reserve now.
  Owner call recorded as §10 Q1.

## 7. The VDC face — what the pre-reboot directed modules already provide, what D-C still needs

The pre-reboot `gandr-vdc` crate (FVDblTT-shaped reflection over the rewrite layer, ADR-68/69) carries a **four-rung directed fragment** — strictly additive over the undirected face, staged behind the levitation gate, with every theorem-grade claim deliberately deferred off the Rust face (wyrd@failed-refactor:crates/gandr-vdc/src/directed.rs:1-87).
It is the LLV fragment (Q-3) engineered; the survey below is the honest inventory of how much of D-C's representation home already exists.

### 7.1 The inventory — the Ł1–Ł4 ladder, executed

* **Ł1 — variance-sorted reflected contexts.** `Variance` is the closed two-way polarity vocabulary, with the engine's third case `Mixed` deliberately _not_ a directed variance (`Variance::of_cell` maps it to `None` — it is the dinaturality shape); `OpSig` pairs a frozen-core signature with a variance slot, and the `−ᵒᵖ` involution lives **on reflected signatures only** — the frozen core carries no `op` (wyrd@failed-refactor:crates/gandr-vdc/src/directed/context.rs:33-93, :107-161).
  `DirectedContext::check_cell_variance` turns the engine's derived variance metadata into a _checkable_ judgment, rejecting mismatched and mixed holes (:183-214, :335).
  **Transfer**: this design record — variance on the reflected layer, never on the kernel's objects — is the binding precedent behind §6.2's variance staging.
* **Ł2 — hom as directed equality.** `DirectedHom {sig, src, tgt}` with contravariant source and covariant target, diagonal `refl`, and the polarity-restricted directed J: `check_directed_j` is a **total checker** under which **symmetry is underivable by construction** — the contravariant motive is refused, with `DirectedJ::symmetry` provided precisely so tests can assert its refusal on every generated hom (wyrd@failed-refactor:crates/gandr-vdc/src/directed/hom.rs:47-94, :105-117, :177-183, :230-251).
* **Ł3 — (co)ends as quantifiers, finite carriers.** `Diagram`/`End`/`Coend` over finite discrete carriers, with **Fubini and co-Yoneda as derived transformations** (`fubini_swap` an involution; `coyoneda_collapse` collapsing the density coend to the diagonal summand) — wyrd@failed-refactor:crates/gandr-vdc/src/directed/coend.rs:47-171, :255, :291.
  Two honesty boundaries are stated in the module itself: carriers are **finite**, and the hom is **refl-generated** — `discrete_hom_inhabited` is inhabited exactly on the diagonal (:314; module header :14-24).
* **Ł4 — the boundary theorem, operational.** `directed_cut` routes certificate composition off the invertibility boundary: wholly-invertible certificates compose through the **ungated** `compose_invertible` (`CutOutcome::Coherent` — never declined), everything else consults the acyclicity gate (`CutOutcome::Directed`/`Declined` with the flow cycle as diagnostic) — wyrd@failed-refactor:crates/gandr-vdc/src/directed/boundary.rs:41-54, :128-143, :160-170.
  This is §2 item 2's composability boundary (LLV Thm 4.5 vs 5.3, W12) already running as code, and the port precedent for §8.1 item 2's composition-mode tag.
* **The core hooks.** The undirected face already has restriction along tight maps (`Vdc::restrict`, wyrd@failed-refactor:crates/gandr-vdc/src/vdc.rs:900-915), the symmetric path protype, and the groupoid certificate stock as `Iso` _pairs_ of derivations.

### 7.2 The gap analysis — five items between the inventory and D-C

1. **No generated one-way stock.** The Ł3 hom is refl-generated (diagonal-only); there is no analog of the `ℰ⇝` one-way classes (§5.1) and no realization of positive words into translator singletons — `CodeHom` has no representation.
   The crate's only certificate stock is the groupoid `Iso` pair.
   This is the single largest distance to O2⇝: the _quantifier's range_ does not exist yet as data.
2. **No companions or conjoints.** The crate contains no companion/conjoint machinery at all (verified by sweep over the source).
   `Vdc::restrict` gives restriction of a loose arrow along tight maps, but the **unit-plus-restriction hom formation** (Q-8, Q-2) — hom as the unit protype restricted along the two endpoint maps — and the one-sided graphs of tight maps are unbuilt.
   These are exactly the equipment structure §4.2 assigns D-B's adjoint content; E6's in-tree Π-as-right-extension-along-a-**conjoint** precedent lives iu-side, not here.
3. **Finite carriers only.** End/Coend are literal finite (co)products; the dinaturality wedge quotient in full generality was explicitly deferred to an Agda face that never landed pre-reboot.
4. **No invertible-core comparison.** O4's subject — the map from the groupoid statement into the invertible core of the directed one — has no machinery: cell invertibility exists as _metadata_ (`CellMeta::invertible`, consumed by Ł4), but nothing computes or compares an invertible core.
5. **The module's own two named prerequisites for directed univalence** (its out-of-scope note, wyrd@failed-refactor:crates/gandr-vdc/src/directed.rs:58-65): the directed-hom type as an **object of the reflected universe** (levitation's `Universe`), and a **transport law relating it to the reflected function space**.
   Both remain unbuilt; they are B10 work by the staging of §9.

### 7.3 Hosting the recommended statement

D-C's representation home at the VDC face reads directly off the inventory:

* **`x ⇝ y` (the directed path protype)** refines the symmetric path protype exactly as `DirectedHom` refines it in the pre-reboot tree — same signature, oriented endpoints — with its members realized as positive words and its rule congruence `≈ᶜ⇝` riding the cell store.
* **O2⇝'s fragment is the unit-plus-restriction fragment** (§2 item 3): hom formation as unit restricted along endpoint maps is the _missing_ item 2 above, which is why §9 stations it at B10 and why the fullness statement cannot be even _stated_ on the reflection face before then.
* **D-B's content lands here, not in the statement**: companions/conjoints for realized tight maps are the equipment the reflection face hosts; one-way certificates with one-sided adjoint evidence become restrictions of realized tight maps, without ever weakening the ua statement's grades (§4.2).
* **O4's kernel-side face** is cheap where it touches Ł4: `all_participating_invertible` already computes the _certificate-level_ invertibility predicate; what O4 adds is the protype-level comparison, which is metatheory (§8.3 item 4).
* **The two-mode composition** (§8.1 item 2) ports `directed_cut`'s routing verbatim: invertible lane ungated, directed lane through the acyclicity gate, string-shaped composites structurally loop-free.

## 8. Certificates and metatheory

### 8.1 What B9 tracelet certificates must carry for directed cells

The B9 certificate component (gandr-wvd.9) gains four schema obligations from this design; none is a theorem, all are representation decisions, which is why they are cheap at B9 and expensive later:

1. **Variance-marked boundaries.** Every boundary position of a directed cell certificate carries a polarity/variance mark; the W17 recommendation — _polarize certificate boundaries_ — graduates from advice to load-bearing schema: in the groupoid case polarity is invisible (everything invertible), in the directed case the polarized fragment is where composition is free.
2. **Composition-mode tag + acyclicity witness.** Each composition step records its mode: invertible (unconditional — the groupoid lift, W12) or directed, the latter carrying its loop-freeness witness (LLV Thm 5.3 shape; Q-10 pedigree; linear-time decidable in the polar-shuffle normal form, W18).
   String-shaped composites — the realizations of path-protype members — discharge the gate structurally: a chain is loop-free by construction, so O1⇝'s "paths realize by composition" never blocks on it.
3. **The two-mode produoidal normal form** (already routed by fcw.11: it lands as the certificate component's rationale element, not an ADR).
   The directed-specific content from W17/W18: interchange `ψ₂` is lax and _stays_ lax after normalization; bidirectional interchange may be assumed only on pure-polarity boundaries (the polarized collapse); and the coherence theorem carries the **distinct-typing side condition** — at-most-one derivation only between distinctly-typed boundaries.
   Directed certificates are exactly where mixed-polarity boundaries first occur, so the certificate composer must plan for one-directional `ψ₂` from the start.
4. **The η-discipline standing check (W11)** applies to `⇒²⇝`'s orientations when the elaboration bridge lands: the one-way classes add η-like contraction cells (e.g. E-dup/E-proj triangles) whose orientation must be checked against the data-η/codata-η cut discipline.

### 8.2 The unpaid-coherence warning, compounded (W15)

The only published formalization of the symmetric-rig coherence adjacent to the groupoid statement leaves the multiplicative layer unpaid (Q-14, doi:10.1145/3498667 — 118 admit sites, `--allow-unsolved-metas` on the multiplicative modules; W15).
The directed statement adds strictly more: the one-way classes interact with E-dist (bialgebra-style cells between diagonals/codiagonals and distribution), and **no formalized twin exists for any of it**.
Consequence for planning: the directed convergence pass exceeds the literature on _two_ axes (multiplicative coherence and one-way coherence), and estimates should not assume a consultable proof shape beyond the additive/Coxeter core.

### 8.3 Obligations this design creates for the Temporal Univalence re-entry

The re-entry (gandr-fcw.12; post-B9; kernel-first) inherits, in dependency order:

1. **The statement shapes**: O1⇝/O2⇝/O3⇝ + O4 as protype-level statements over the kernel's exported identity layer, with the three directed alphabets fixed in the types (the alphabet-in-the-type code-review criterion, `DESIGN-ua-base-vocabulary.md` §7).
2. **A directed demonstrator** mirroring `Internal.UaBase.*`: positive `Step*`-shaped paths (redeeming the E3 mandate), the leaf-free absolute O2⇝, the constant-map and constant-literal witnesses as in-tree negative tests, O3⇝-β unconditional, η staged by rule subsystem exactly as `iu-c2h.1` staged the groupoid η.
3. **The directed NFC**: the canonical-word map for the enlarged monoid as a `≈ᶜ⇝`-homomorphism — the faithfulness wall, now against a transformation-monoid presentation rather than the Coxeter presentation (W14's directed sibling; the register gap of §11.2 must be paid before this is even citable).
   The groupoid-side precedent is a **moving target** — its walls are closing as of 2026-07-19 (§11.1) — so this obligation re-baselines at re-entry (§10 Q9).
4. **O4 core coincidence** — the two-statement compatibility theorem; its failure would falsify the localization reading of §3, which is exactly the P4 falsifiability posture (`DESIGN-temporal-univalence.md` §6): reversibility governs composability, and the directed band exhibits the restricted-cut shape.
5. **The W16-directed degeneracy audit**: verify the kernel's directed hom former cannot be collapsed to a thin/ambient rendering by any export-format consumer (the §5.3 guards as permanent tests).

## 9. Landing plan — phased so nothing blocks the backbone

The governing rule is the fcw.11 resolution itself: buildout first; the metatheory is parked and blocks nothing.
Every _theorem_ obligation this design creates sits in the parked track; what the backbone phases carry is schema, rules, and representation decisions — each cheap at its named phase and expensive at any later one.

| phase                                                                    | builds from this design                                                                                                                                                                                                                                                                                                                  | exit obligation                                                                                                               | blocks the backbone?                           |
| ------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------- |
| **B2 kernel-core**                                                       | the reserved variance/directedness annotation slot in the S1/export format (§6.3)                                                                                                                                                                                                                                                        | slot documented in the format; **no semantics**                                                                               | no — one schema line                           |
| **B7 identity layer**                                                    | both families per §6.2: `Path`/`here`/`walk`/`then`/`back` and `Step` + directed walk with the motive-covariance side condition, composition derived, **no `back`**; **L-machine dynamics for both** (Walk-β and the directed β — paying the pre-reboot decline, §6.1); the two pathological witnesses (K-derivation, `back`-derivation) | witnesses fail elaboration permanently; ADR-79 honesty gate holds — **no ua statement at B7** (there is no code universe yet) | no — rule-level work already ordered by fcw.11 |
| **B8 levitation**                                                        | the `Desc` fragment D-C ranges over; the directed edit vocabulary `ℰ⇝` (§5.1) becomes _statable over codes_ — ADR-76 D5's stage gate for directed identity of codes opens here                                                                                                                                                           | vocabulary expressible; nothing proved                                                                                        | no                                             |
| **B9 certificates**                                                      | the four schema obligations of §8.1 (variance-marked boundaries — the slot's first consumer; composition-mode tag + acyclicity witness — porting `directed_cut`'s routing, §7.1 Ł4; the two-mode produoidal normal form; the η-discipline check); kernel-replay parses the annotation plane                                              | replay green including the annotation plane                                                                                   | no — representation decisions, not theorems    |
| **B10 VDC + reflection**                                                 | D-C's representation home (§7.3): the Ł1–Ł4 ladder ported, unit-plus-restriction hom formation, companions/conjoints (closing §7.2 items 1-2), the reflected-universe object + transport law (§7.2 item 5); variance-sorted contexts land here and only here (§6.2 staging)                                                              | the O2⇝ statement _statable_ on the reflection face at engineering grade                                                      | no                                             |
| **Temporal Univalence re-entry** (post-B9 at the earliest, gandr-fcw.12) | §8.3 items 1-5: the statements, the directed demonstrator, the directed NFC, **O4**, the degeneracy audit; the `⇒²⇝` convergence pass (§5.1)                                                                                                                                                                                             | P4 validation posture (§2 item 4)                                                                                             | no — parked by design                          |

**The one cross-phase coupling to watch** is the annotation slot: B2 performs work whose first consumer is three phases later (B9).
That inversion is exactly why §6.3 prices it explicitly instead of leaving it implicit in the format design.
Everything else in the table is consumed in-phase or later without back-edges.

## 10. Open questions for the owner

Recommendations made in the body are restated here only where the owner still holds a call; everything else is genuinely open.

1. **Q1 — the B2 annotation slot.** Reserve the variance/directedness slot in the S1/export format at B2, or defer to B7/B9?
   Recommendation on file (§6.3): reserve at B2 — the hard deadline is B9 (replay), but the reserved-slot vocabulary is open exactly once and the deferral saves one schema line.
2. **Q2 — directed spellings.** ADR-79 fixes only the former (`Step`, no `back`) and the notations (`<~>`, `~~>`); still unnamed: the directed eliminator and diagonal-intro spellings, the `~~>` former's prose name (ADR-79's own candidate: `Transform`), and whether directed composition shares `then`.
   Pure surface, but by ADR-79's own blast-radius logic it is cheapest settled before the B7 rules land.
   No recommendation — owner vocabulary call.
3. **Q3 — a kernel `Path → Step` bridge.** Recommendation on file (§6.2): none at B7 — the groupoid-to-directed comparison is O4's _theorem_, and a kernel coercion would assume it as an axiom.
   Genuinely open: whether a _derived_ surface coercion is wanted once ua-dir lands, and at which stratum.
4. **Q4 — genuinely lax 2-cells.** Declined for the statement (§4.2: `≈ᶜ⇝` remains an equivalence; the (n, p)-dial reading).
   Genuinely open: whether the certificate layer's one-directional interchange (§8.1 item 3, W17/W18) eventually forces laxity down into the statement's rule layer at mixed-polarity boundaries — if it does, O3⇝'s η grade needs restating, and the owner should decide whether that restatement is a new design pass or an amendment here.
5. **Q5 — variance as a shared kind.** The pre-reboot tree recorded (but did not mint) a draft-ADR candidate: promote the variance enum to a shared kind carried by the reflected universe once directed univalence is scoped (wyrd@failed-refactor:crates/gandr-vdc/src/directed.rs:78-87).
   A B10-time decision; §6.2's staging is compatible with either answer.
6. **Q6 — the canonical (co)end representation.** Its twin candidate from the same record: whether the finite-carrier diagram becomes the canonical reflected end-object once the typed `Desc` layer can express dipresheaves, or remains a property-test vehicle.
   B10-time.
7. **Q7 — guard placement.** Where do the constant-map and constant-literal witnesses (§5, §5.3) land as permanent negative tests?
   Recommendation on file: at whichever phase first carries a directed certificate stock over codes, filed beside U3.0c and the leaf-shift witness.
   Genuinely open: whether that first phase is B10's engineering face or the metatheory demonstrator.
8. **Q8 — the register gap.** The transformation-monoid / finite-set-category presentation row (§11.2) must exist before the metatheory re-entry can cite the directed word problem.
   Cheap (a bibliography addition) but gating; open: who pays it and when.
9. **Q9 — re-baselining against the in-motion η closure.** The groupoid η walls are closing while this study is written (as of 2026-07-19: seven letterC walls, `perm-hom` unconditional, one wall down to a single inductive hole in uncommitted work — §11.1).
   The directed η plan (§8.3 item 2) copies the staging _shape_; genuinely open at re-entry: which walls survived, and whether the discharge machinery (the collapse / positional-step lemma kit) transfers to the transformation-monoid setting — the answer moves the §8.2 estimate materially.

## 11. Source-material findings and register notes

### 11.1 Findings about the sources

* **Design-vs-code substrate deviation (load-bearing here).** `DESIGN-ua-base-vocabulary.md` §7 mandates `Step*`-shaped paths; the executed demonstrator uses the involutive `Word` (`iu:src/Internal/UaBase/Edit.agda:72-73`).
  Not an error — the involutive form is what the _groupoid_ statement needs — but the directed design should be read as restoring the mandated substrate, and the deviation is worth an erratum note iu-side.
* **The groupoid η is staged, not landed — and closing in real time (as of 2026-07-19, proof in motion).** The execution record's "landed gate-green" covers the nine O1/O2/β modules; `faithful`/`η-section` exist only inside honest parameterized modules (walls are module parameters — zero postulates, zero holes, zero unsolved-metas pragmas across the tranche, all inside the `--safe` gate).
  The chain: `faithful`/`η-section` follow from one wall `triv` (`iu:src/Internal/UaBase/Faithful.agda:166-195`), `triv` is proved from `perm-hom` + `letterC` (`iu:src/Internal/UaBase/WordProblem.agda:176-221`), `letterC` from the **seven** `Discharge` walls (`iu:src/Internal/UaBase/Canonical.agda:1647-1662`: `collapse-⊗assoc`, `w-⊗comm`, `w-⊕swap`, `w-dist`, `w-⊗ˡ`, `w-⊗ʳ`, `w-⊕ˡ`), and `perm-hom` is **unconditional** (`Coxeter.agda:1446-1664`).
  The trajectory since pass 1's source snapshot: the entire Coxeter layer and the `collapse-⊗unit` wall closed on 2026-07-17 (walls eight → seven), and uncommitted work observed 2026-07-19 has one further wall (`w-⊕swap`) down to a single inductive hole.
  Consumers of "ua-base is proved" should carry this asterisk **with its date**; the directed η plan (§8.3 item 2) deliberately copies the staging shape rather than assuming a closed precedent, and re-baselines at re-entry (§10 Q9).
* **The vocabulary design doc lags the η code by one refinement step.** `DESIGN-ua-base-vocabulary.md`'s η staging record (2026-07-17) still frames the residual as one typed `NFC` lemma; the code has since refined `NFC` into `triv` discharged from the seven `letterC` walls plus the now-unconditional `perm-hom`.
  A refinement, not a contradiction — but a reader of the doc alone underestimates both the progress (the Coxeter layer is closed) and the residual structure (seven named walls).
  Worth folding into the same iu-side erratum sweep as the whisker-closure note below.
* **A stale note in the execution record.** The record flags whisker-closure of `≈ᶜ` as an open design decision, but the executed `Rules.agda:476-479` has since added the positional-whisker congruences (`cong-⊗ˡ*` …).
  Doc lags code by one step; harmless, worth a one-line iu-side sweep.

### 11.2 Register notes

* **Gap: transformation-monoid / finite-set-category presentations.** The directed word problem (§5.1) needs the classical presentations of the full transformation monoid and/or the skeletal category of finite sets (face/degeneracy-style convergent systems).
  The register holds no row; one should be added before the metatheory re-entry cites it.
  This study deliberately states the need without inventing a locator.
* **Riehl–Shulman was declined** from the register (Appendix 2 of `docs/research/bibliography-v2.md`); Q-15 builds on that line, and Q-15's own locator is sufficient anchor for this design's purposes — no re-registration is requested.
* The task-level shorthand "the FVDblTT thesis (register Q-1)" resolves in the register to **Q-2** (the thesis, PRIMARY) with Q-1 the secondary paper; citations above follow the register.
