//! The **crDC suite** — the compositional-rewriting double-category axioms as
//! a staged property suite over the as-built cell store (gandr-5lf.3; the F0
//! extension of the §3 dictionary suite in `laws.rs`, following the TRS[Σ]
//! template of the tracelet-algebra component's crDC-ladder section).
//!
//! The stage is the seam category of the tracelet-algebra component: objects
//! are command patterns, tight morphisms are substitutions, loose arrows are
//! cells, and 2-cells are derivations. Every row runs over the **real**
//! `gandr-theory-computads` structures — the overlap enumerator, matching and
//! unification, rewriting and normalization, and the tracelet certificates —
//! never a second engine. Axioms are Behr–Harmer–Krivine Def 3.2
//! (arXiv:2204.07175); the virtual-side line items are Thompson–Carlson
//! (arXiv:2605.20586, Thm 5.11 / Thm 4.14 / Def 7.2).
//!
//! # Verdict note (the suite's deliverable; the associativity ledger gandr-5lf.6 consumes)
//!
//! Per-axiom verdicts; a failure names its missing invariant and is
//! informative, not fatal. Scope of every claim: the **cell-visible convergent
//! fragment** (positive polarity); natives remain opaque and outside every
//! claim.
//!
//! | axiom row (Def 3.2)                            | verdict | witness |
//! | ---------------------------------------------- | ------- | ------- |
//! | (i) multi-sums in the tight layer              | **holds, degenerate-singleton** — the enumerated family is multi-universal; first-order syntactic unification makes the family at most one per ordered pair per kind (the discrete TRS[Σ] case) | [`multi_sum_families_are_degenerate_singletons`], [`confluence_cospans_factor_uniquely`], [`composition_cospans_factor_uniquely`], [`confluence_cospans_found_by_matching_factor`], [`composition_cospans_found_by_matching_factor`] |
//! | (ii) pullbacks in the tight and cell layers    | **holds strictly** — unification computes pattern pullbacks; cell intersection is componentwise | [`unification_computes_pattern_pullbacks`], [`matched_pattern_instances_pull_back`], [`cell_intersection_is_componentwise`] |
//! | (iii) horizontal decomposition                 | **holds strictly** — a cell over a composed seam factors as the two-step derivation; the globular-iso residue is the identity (stronger than isoglobular) | [`fused_cells_decompose_as_two_step_derivations`] |
//! | (iv) source a strong multi-opfibration         | **holds, discrete** — the singleton lift (substitute through the whole cell) is op-Cartesian: rewriting is substitution-stable and matches factor uniquely | [`source_pushforward_is_substitution_stable`], [`source_pushforward_factors_matches_uniquely`] |
//! | (v) target a residual multi-opfibration        | **holds in per-instance form** — the lift is the instantiated cell; the residue is the owed post-normalization derivation, exercised exactly when instantiation creates redexes; lifts compose functorially; pushed derivations are confluence-unique on the certified fragment | [`target_pushforward_exists_with_derivation_residue`], [`target_pushforward_lifts_compose`], [`target_pushforward_is_confluence_unique`], [`residue_census_on_the_peano_family`] |
//! | virtual: positive globular decompositions      | **holds strictly on the free path algebra** — every derivation path decomposes uniquely into one-step cells and recomposes; two-step composites are pro-representable (the fused cell) | [`paths_decompose_uniquely_into_steps`], [`two_step_composites_are_pro_representable`] |
//! | virtual: cellular Conduché (2-layer criterion) | **holds strictly** — path splittings refine (concatenation is free) | [`path_splittings_refine`] |
//! | cylindrical decomposition property (CDP)       | **open line item, not discharged** — a distinct obligation (not a crDC corollary); unconditional oplaxity is the honest default (gandr-5lf.6 consumes) | recorded, not asserted |
//!
//! Green rows (i)–(v) make the universal concurrency theorem (Thm 3.4) and
//! the associativity theorem (Thm 3.5) available for the cell-visible
//! convergent fragment **by the universal proofs** — the fused ≡ two-step and
//! grafting-associativity contracts upgrade from adopted-test to
//! theorem-backed, with the differentials of `differential.rs` retained as
//! adequacy witnesses. Row (v)'s static form names one missing invariant: a
//! symbolic normal-form constructor (the as-built residue is per-instance,
//! because normalization needs ground terms) — informative, not fatal.
//!
//! # Findings landed with the suite
//!
//! - **Triangular-unifier resolution (engine fix, the suite's first catch).**
//!   Rows (i) and (ii) exposed that `unify_cmd` returned *triangular* unifiers
//!   — a binding whose image mentions a metavariable bound *after* it (the goal
//!   order is not topological) — violating its own `apply_cmd(a) ==
//!   apply_cmd(b)` contract under single-pass application, and under-resolving
//!   the enumerator's peaks. Fixed at source: `unify_cmd` now fully resolves
//!   its unifier (`Subst::resolve`) before returning, keeping application
//!   single-pass on the hot path (the implementation-model deep-read's
//!   substitution lessons: skip-traversal guards, no recompute-per-use).
//!   Regression: `subst::tests::unification_resolves_triangular_bindings`; the
//!   failing inputs are pinned in `crdc.proptest-regressions`.
//! - **The residual part of row (v) is exercised exactly by redex-creating
//!   instantiations** (the proposal's open question 2, answered for the add
//!   family): [`residue_census_on_the_peano_family`] shows a three-step residue
//!   for a redex-bearing instantiation and none for a normal one.
//! - **Generated cells follow the rewriting discipline** (right-face variables
//!   drawn from the left face) and the unifiable-pair rows build faces as two
//!   generalizations of one seam (unifiability by construction, no reject
//!   storms); the wild rows keep the independent direction (found by matching).
//!   The generators are deliberately narrow seams — a size-proportionate
//!   bijective unranking over the command-pattern signature (the Tarau axis,
//!   `docs/research/tarau-regularity-compression.md` §7.1) drops in as an
//!   alternative value source without touching the row logic (gandr-9a9).
//! - **Suite cost is wall-gated**: rows run at modest case counts with
//!   `PROPTEST_CASES` overriding for shakeout runs (the `conformance.rs`
//!   posture), and the residue row runs at a shallow normalization budget — a
//!   divergent generated system is outside the convergent fragment, never an
//!   axiom failure.
//!
//! [`multi_sum_families_are_degenerate_singletons`]: tests::multi_sum_families_are_degenerate_singletons
//! [`confluence_cospans_factor_uniquely`]: tests::confluence_cospans_factor_uniquely
//! [`composition_cospans_factor_uniquely`]: tests::composition_cospans_factor_uniquely
//! [`confluence_cospans_found_by_matching_factor`]: tests::confluence_cospans_found_by_matching_factor
//! [`composition_cospans_found_by_matching_factor`]: tests::composition_cospans_found_by_matching_factor
//! [`unification_computes_pattern_pullbacks`]: tests::unification_computes_pattern_pullbacks
//! [`matched_pattern_instances_pull_back`]: tests::matched_pattern_instances_pull_back
//! [`cell_intersection_is_componentwise`]: tests::cell_intersection_is_componentwise
//! [`fused_cells_decompose_as_two_step_derivations`]: tests::fused_cells_decompose_as_two_step_derivations
//! [`source_pushforward_is_substitution_stable`]: tests::source_pushforward_is_substitution_stable
//! [`source_pushforward_factors_matches_uniquely`]: tests::source_pushforward_factors_matches_uniquely
//! [`target_pushforward_exists_with_derivation_residue`]: tests::target_pushforward_exists_with_derivation_residue
//! [`target_pushforward_lifts_compose`]: tests::target_pushforward_lifts_compose
//! [`target_pushforward_is_confluence_unique`]: tests::target_pushforward_is_confluence_unique
//! [`residue_census_on_the_peano_family`]: tests::residue_census_on_the_peano_family
//! [`paths_decompose_uniquely_into_steps`]: tests::paths_decompose_uniquely_into_steps
//! [`two_step_composites_are_pro_representable`]: tests::two_step_composites_are_pro_representable
//! [`path_splittings_refine`]: tests::path_splittings_refine

#[cfg(test)]
mod tests
{
    use alloc::collections::BTreeSet;

    use gandr_core_sequent::il::Polarity;
    use gandr_theory_computads::Cat;
    use gandr_theory_computads::Cell;
    use gandr_theory_computads::CellApp;
    use gandr_theory_computads::CellId;
    use gandr_theory_computads::CellProvenance;
    use gandr_theory_computads::CellStore;
    use gandr_theory_computads::CmdPat;
    use gandr_theory_computads::CompletionBudget;
    use gandr_theory_computads::ConsPat;
    use gandr_theory_computads::MetaVar;
    use gandr_theory_computads::NormalizationBudget;
    use gandr_theory_computads::Orientation;
    use gandr_theory_computads::Overlap;
    use gandr_theory_computads::OverlapKind;
    use gandr_theory_computads::Pos;
    use gandr_theory_computads::PositionStep;
    use gandr_theory_computads::ProdPat;
    use gandr_theory_computads::Subst;
    use gandr_theory_computads::Sym;
    use gandr_theory_computads::Tracelet;
    use gandr_theory_computads::complete;
    use gandr_theory_computads::derive_fused;
    use gandr_theory_computads::enumerate_overlaps;
    use gandr_theory_computads::frame_defining_cell;
    use gandr_theory_computads::pattern::Node;
    use gandr_theory_computads::pattern::collect_cmd_metavars;
    use gandr_theory_computads::pattern::splice_at;
    use gandr_theory_computads::pattern::subterm_at;
    use gandr_theory_computads::rewrite::normalize;
    use gandr_theory_computads::rewrite::rewrite_at;
    use gandr_theory_computads::subst::match_cmd;
    use gandr_theory_computads::subst::unify_cmd;
    use proptest::prelude::*;

    /// The suite's proptest configuration: a modest native default so the
    /// merge wall stays fast, with `PROPTEST_CASES` overriding for longer
    /// shakeout runs (the `conformance.rs` posture).
    fn crdc_config(cases: Cases) -> ProptestConfig
    {
        let mut config = ProptestConfig::default();
        if std::env::var_os("PROPTEST_CASES").is_none() {
            config.cases = cases.0;
        }
        config
    }

    /// A proptest case count (a semantic wrapper per the lint wall).
    #[derive(Clone, Copy, Debug)]
    #[repr(transparent)]
    struct Cases(u32);

    // ---- The suite surface (the component's T0 sketch, suite-local) ---------

    /// The outcome of factoring a match cospan through the enumerated family
    /// (axiom (i)'s `factor_cospan`; the sketch's `MultiSumFailure` is the two
    /// non-`Factored` variants).
    #[derive(Clone, Debug)]
    enum Factorization
    {
        /// Exactly one enumerated overlap admits a mediator: the family is
        /// multi-universal at this cospan.
        Factored
        {
            /// The overlap factored through.
            overlap: Box<Overlap>,
            /// The mediator: the unique substitution from the overlap's seam
            /// instance to the cospan's common instance.
            mediator: Subst,
        },
        /// No enumerated overlap admits a mediator: enumeration is incomplete
        /// (a completeness bug).
        Incomplete,
        /// Several enumerated overlaps admit mediators: the family is not
        /// minimal (a universality bug).
        NonMinimal(Vec<Overlap>),
    }

    /// A cospan of matches: two pattern faces matched into one ground command.
    /// The left face is recoverable from the overlap and store; the right
    /// face is carried because the apartness-renamed face (what the unifier
    /// binds) needs the original to re-key the right leg.
    #[derive(Clone, Debug)]
    struct Cospan
    {
        /// The left leg: a match of the left face into `instance`.
        left_match: Subst,
        /// The right face (the right cell's `lhs`, before apartness renaming).
        right_face: CmdPat,
        /// The right leg: a match of `right_face` into `instance`.
        right_match: Subst,
        /// The common (ground) command both legs match into.
        instance: CmdPat,
    }

    /// A lifted cell — a member of a pushforward family (axioms (iv)/(v)).
    #[derive(Clone, Debug, Eq, PartialEq)]
    #[repr(transparent)]
    struct CellLift(Cell);

    /// The residue of a target pushforward on one ground instance: the
    /// post-normalization the composite still owes (Def 2.8's `post`; the
    /// as-built residue is a derivation — the record replay consumes — not a
    /// substitution, because the engine normalizes ground terms only).
    #[derive(Clone, Debug, Eq, PartialEq)]
    #[repr(transparent)]
    struct Residue
    {
        /// The owed normalization steps from the fired output to its normal
        /// form (empty when instantiation created no redexes).
        post: Vec<CellApp>,
    }

    /// Axiom (iii)'s factorization: a cell over a composed seam as a
    /// horizontal composition of per-frame cells plus the globular-iso
    /// witness (Remark 3.3 (11)).
    #[derive(Clone, Debug)]
    struct HorizontalFactorization
    {
        /// The per-frame cells, in firing order.
        frames: Vec<CellId>,
        /// The globular-iso witness: the fused ≡ two-step certificate.
        witness: Tracelet,
    }

    /// Whether two substitutions agree on a variable set (a suite-local
    /// verdict newtype; the lint wall forbids bare primitives in signatures).
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    #[repr(transparent)]
    struct Agreement(bool);

    impl From<bool> for Agreement
    {
        #[inline]
        fn from(value: bool) -> Self
        {
            Self(value)
        }
    }

    impl From<Agreement> for bool
    {
        #[inline]
        fn from(value: Agreement) -> Self
        {
            value.0
        }
    }

    /// A recursion-depth parameter for the generators (a semantic wrapper per
    /// the lint wall).
    #[derive(Clone, Copy, Debug)]
    #[repr(transparent)]
    struct Depth(u32);

    /// Number of Peano successors in generated Nat fixtures.
    #[repr(transparent)]
    #[derive(Clone, Copy)]
    struct NatSuccCount(u8);

    /// Factor `cospan` through the enumerated `family` (axiom (i)): find the
    /// overlaps of the ordered pair `(left, right)` of the given `kind` whose
    /// seam instance mediates the cospan, checking the leg factorizations.
    ///
    /// The mediator is computed by **matching** (one-sided) while the
    /// enumerated unifier comes from **unification** (two-sided) — the leg
    /// checks are a genuine cross-engine validation, not a restatement.
    ///
    /// Fragment scope: the composition reading assumes the cell-visible
    /// fragment's flat command grammar (the seam is always the root, so the
    /// unified pair is `(left.rhs, right.lhs)` whole). A future fragment with
    /// nested commands must generalize the seam-instance computation to the
    /// subterm at `overlap.seam` — the row shape is unchanged.
    fn factor_cospan(
        left: CellId,
        right: CellId,
        kind: OverlapKind,
        cospan: &Cospan,
        family: &[Overlap],
        store: &CellStore,
    ) -> Factorization
    {
        let mut mediators = Vec::new();
        for overlap in family {
            if overlap.kind != kind || overlap.left != left || overlap.right != right {
                continue;
            }
            let Some(seam) = seam_instance(overlap, store)
            else {
                continue;
            };
            let Some((left_pattern, right_pattern)) = overlap_faces(overlap, store)
            else {
                continue;
            };
            // The unifier must actually unify the two faces at the seam
            // instance (an engine self-check, not a suite assumption).
            if overlap.unifier.apply_cmd(left_pattern) != seam
                || overlap.unifier.apply_cmd(right_pattern) != seam
            {
                continue;
            }
            let mut mediator = Subst::new();
            if !bool::from(match_cmd(&seam, &cospan.instance, &mut mediator)) {
                continue;
            }
            let left_vars = cmd_vars(left_pattern);
            let left_composite = compose(&mediator, &overlap.unifier, &left_vars);
            if !bool::from(substs_agree_on(
                &cospan.left_match,
                &left_composite,
                &left_vars,
            )) {
                continue;
            }
            // The right leg reads against the original right cell; the
            // unifier binds the renamed cell — map the leg through the
            // apartness renaming (occurrence-parallel).
            let renamed_right_match =
                remap_match(&cospan.right_match, &cospan.right_face, right_pattern);
            let right_vars = cmd_vars(right_pattern);
            let right_composite = compose(&mediator, &overlap.unifier, &right_vars);
            if !bool::from(substs_agree_on(
                &renamed_right_match,
                &right_composite,
                &right_vars,
            )) {
                continue;
            }
            mediators.push((overlap.clone(), mediator));
        }
        match mediators.len() {
            | 0 => Factorization::Incomplete,
            | 1 => {
                let Some((overlap, mediator)) = mediators.pop()
                else {
                    return Factorization::Incomplete;
                };
                Factorization::Factored {
                    overlap: Box::new(overlap),
                    mediator,
                }
            },
            | _ => Factorization::NonMinimal(
                mediators.into_iter().map(|(overlap, _)| overlap).collect(),
            ),
        }
    }

    /// Axiom (iv): pushforward of a cell along a substitution on its input
    /// face — the discrete TRS[Σ] lift (apply the substitution to the whole
    /// cell), a singleton family.
    fn pushforward_src(
        cell: &Cell,
        sub: &Subst,
    ) -> Vec<CellLift>
    {
        vec![CellLift(lift_cell(cell, sub))]
    }

    /// Axiom (v): pushforward of a cell along a substitution on its output
    /// face — the same singleton lift; the residual part is per-instance (see
    /// [`residue_of`]) because the engine normalizes ground terms only.
    fn pushforward_tgt(
        cell: &Cell,
        sub: &Subst,
    ) -> Vec<CellLift>
    {
        vec![CellLift(lift_cell(cell, sub))]
    }

    /// The residue of a target pushforward on one ground `instance`: fire the
    /// lifted cell, then record the owed post-normalization (Def 2.8: lifts
    /// of `f` become lifts of `post ∘ f`).
    fn residue_of(
        store: &CellStore,
        lifted: &CellLift,
        instance: &CmdPat,
        budget: NormalizationBudget,
    ) -> Option<(CmdPat, Residue)>
    {
        let fired = rewrite_at(&lifted.0, instance, &Pos::root())?;
        let norm = normalize(store, &fired, budget);
        if norm.exhausted {
            return None;
        }
        Some((norm.normal, Residue { post: norm.path }))
    }

    /// Axiom (iii): factor the fused cell of a composition overlap as the
    /// horizontal composition of its per-frame cells, witnessed by the
    /// fused ≡ two-step tracelet (the globular iso, strict on the boundary).
    fn decompose_horizontal(
        overlap: &Overlap,
        store: &mut CellStore,
    ) -> Option<HorizontalFactorization>
    {
        let (_fused_id, witness) = derive_fused(overlap, store)?;
        Some(HorizontalFactorization {
            frames: vec![overlap.left, overlap.right],
            witness,
        })
    }

    /// Instantiate a cell through a substitution (the TRS[Σ] lift).
    fn lift_cell(
        cell: &Cell,
        sub: &Subst,
    ) -> Cell
    {
        Cell::new(
            sub.apply_cmd(&cell.lhs),
            sub.apply_cmd(&cell.rhs),
            cell.orient,
            cell.provenance,
        )
    }

    // ---- Substitution algebra helpers ---------------------------------------

    /// The distinct metavariables of a command pattern.
    fn cmd_vars(cmd: &CmdPat) -> Vec<MetaVar>
    {
        let mut occurrences = Vec::new();
        collect_cmd_metavars(cmd, &mut occurrences);
        let mut seen = BTreeSet::new();
        occurrences
            .into_iter()
            .filter(|mv| seen.insert(mv.clone()))
            .collect()
    }

    /// Apply a substitution to a producer metavariable read as a pattern.
    fn apply_prod_var(
        subst: &Subst,
        mv: &MetaVar,
    ) -> ProdPat
    {
        subst.apply_prod(&ProdPat::Meta(mv.clone()))
    }

    /// Apply a substitution to a consumer metavariable read as a pattern.
    fn apply_cons_var(
        subst: &Subst,
        mv: &MetaVar,
    ) -> ConsPat
    {
        subst.apply_cons(&ConsPat::Meta(mv.clone()))
    }

    /// The composite substitution `after ∘ before`, restricted to `vars`.
    fn compose(
        after: &Subst,
        before: &Subst,
        vars: &[MetaVar],
    ) -> Subst
    {
        let mut out = Subst::new();
        for mv in vars {
            match mv.cat {
                | Cat::Producer => {
                    let image = after.apply_prod(&before.apply_prod(&ProdPat::Meta(mv.clone())));
                    if image != ProdPat::Meta(mv.clone()) {
                        let _ = out.bind_prod(mv.clone(), image);
                    }
                },
                | Cat::Consumer => {
                    let image = after.apply_cons(&before.apply_cons(&ConsPat::Meta(mv.clone())));
                    if image != ConsPat::Meta(mv.clone()) {
                        let _ = out.bind_cons(mv.clone(), image);
                    }
                },
            }
        }
        out
    }

    /// Whether two substitutions agree on every variable of `vars` (unbound
    /// reads as the identity).
    fn substs_agree_on(
        a: &Subst,
        b: &Subst,
        vars: &[MetaVar],
    ) -> Agreement
    {
        Agreement::from(vars.iter().all(|mv| match mv.cat {
            | Cat::Producer => apply_prod_var(a, mv) == apply_prod_var(b, mv),
            | Cat::Consumer => apply_cons_var(a, mv) == apply_cons_var(b, mv),
        }))
    }

    /// Re-key a match from `from_face`'s metavariables to `to_face`'s, where
    /// the faces are related by a structure-preserving renaming (occurrence-
    /// parallel metavariable lists).
    fn remap_match(
        m: &Subst,
        from_face: &CmdPat,
        to_face: &CmdPat,
    ) -> Subst
    {
        let mut from_occurrences = Vec::new();
        collect_cmd_metavars(from_face, &mut from_occurrences);
        let mut to_occurrences = Vec::new();
        collect_cmd_metavars(to_face, &mut to_occurrences);
        debug_assert_eq!(
            from_occurrences.len(),
            to_occurrences.len(),
            "a structure-preserving renaming keeps occurrence lists parallel"
        );
        let mut out = Subst::new();
        for (from, to) in from_occurrences.iter().zip(to_occurrences.iter()) {
            match from.cat {
                | Cat::Producer => {
                    if let Some(image) = m.get_prod(from) {
                        let _ = out.bind_prod(to.clone(), image.clone());
                    }
                },
                | Cat::Consumer => {
                    if let Some(image) = m.get_cons(from) {
                        let _ = out.bind_cons(to.clone(), image.clone());
                    }
                },
            }
        }
        out
    }

    /// The overlap's seam instance: the peak for confluence, the instantiated
    /// left right-hand side for composition.
    fn seam_instance(
        overlap: &Overlap,
        store: &CellStore,
    ) -> Option<CmdPat>
    {
        match overlap.kind {
            | OverlapKind::Confluence => Some(overlap.peak.clone()),
            | OverlapKind::Composition => overlap.left_reduct(store),
        }
    }

    /// The two faces the overlap's unifier unifies: the left cell's `lhs`
    /// (confluence) or `rhs` (composition), and the renamed right cell's
    /// `lhs`.
    fn overlap_faces<'store>(
        overlap: &'store Overlap,
        store: &'store CellStore,
    ) -> Option<(&'store CmdPat, &'store CmdPat)>
    {
        let left = store.get(overlap.left)?;
        let left_pattern = match overlap.kind {
            | OverlapKind::Confluence => &left.lhs,
            | OverlapKind::Composition => &left.rhs,
        };
        Some((left_pattern, &overlap.right_renamed().lhs))
    }

    /// Run a recorded path from `start`, firing each step by ground rewriting
    /// (the replay fold; `tracelet`'s own is private).
    fn run_path(
        store: &CellStore,
        start: &CmdPat,
        path: &[CellApp],
    ) -> Option<CmdPat>
    {
        let mut current = start.clone();
        for step in path {
            let cell = store.get(step.cell)?;
            current = rewrite_at(cell, &current, &step.at)?;
        }
        Some(current)
    }

    // ---- Generators ---------------------------------------------------------

    /// The metavariable pools generators draw from — the default pool and
    /// the apart pools (disjoint names, so no apartness renaming is needed
    /// across generated sides).
    #[derive(Clone, Copy, Debug)]
    enum Pool
    {
        /// The default pool: `x`,`y` producers; `a`,`b` consumers.
        Default,
        /// The second pool (`x2`, `y2`, `a2`, `b2`).
        Two,
        /// The third pool (`x3`, `y3`, `a3`, `b3`).
        Three,
    }

    impl Pool
    {
        /// The pool's metavariables (producers, consumers).
        fn vars(self) -> (Vec<MetaVar>, Vec<MetaVar>)
        {
            let suffix = match self {
                | Self::Default => "",
                | Self::Two => "2",
                | Self::Three => "3",
            };
            let prod = ["x", "y"]
                .into_iter()
                .map(|base| MetaVar::producer(format!("{base}{suffix}")))
                .collect();
            let cons = ["a", "b"]
                .into_iter()
                .map(|base| MetaVar::consumer(format!("{base}{suffix}")))
                .collect();
            (prod, cons)
        }
    }

    /// A producer-pattern leaf over the pool (metavariable-biased, so
    /// non-linear left-hand sides arise).
    fn prod_leaf(vars: &[MetaVar]) -> BoxedStrategy<ProdPat>
    {
        if vars.is_empty() {
            prop_oneof![
                Just(ProdPat::ctor("Zero", [])),
                Just(ProdPat::ctor("Nil", [])),
            ]
            .boxed()
        }
        else {
            prop_oneof![
                Just(ProdPat::ctor("Zero", [])),
                Just(ProdPat::ctor("Nil", [])),
                proptest::sample::select(vars.to_vec()).prop_map(ProdPat::Meta),
                proptest::sample::select(vars.to_vec()).prop_map(ProdPat::Meta),
            ]
            .boxed()
        }
    }

    /// Producer patterns over the pool, depth-capped (`Zero`/`Nil` nullary,
    /// `Succ` unary, `Cons` binary).
    fn arb_prodpat(
        vars: &[MetaVar],
        depth: Depth,
    ) -> BoxedStrategy<ProdPat>
    {
        prod_leaf(vars)
            .prop_recursive(depth.0, 48, 3, |inner| {
                prop_oneof![
                    inner.clone().prop_map(|p| ProdPat::ctor("Succ", [p])),
                    (inner.clone(), inner)
                        .prop_map(|(l, r)| ProdPat::ctor("Cons", <[ProdPat; 2]>::from((l, r)))),
                ]
            })
            .boxed()
    }

    /// Consumer patterns over the pool, depth-capped (`★`, metavariables, the
    /// `add`/`f` operation frames, and the `Succ`/`Cons` return-side frames).
    fn arb_conspat(
        pvars: &[MetaVar],
        cvars: &[MetaVar],
        depth: Depth,
    ) -> BoxedStrategy<ConsPat>
    {
        let leaf = if cvars.is_empty() {
            Just(ConsPat::Top).boxed()
        }
        else {
            prop_oneof![
                Just(ConsPat::Top),
                proptest::sample::select(cvars.to_vec()).prop_map(ConsPat::Meta),
                proptest::sample::select(cvars.to_vec()).prop_map(ConsPat::Meta),
            ]
            .boxed()
        };
        let arg_vars = pvars.to_vec();
        leaf.prop_recursive(depth.0, 48, 3, move |inner| {
            let arg = arb_prodpat(&arg_vars, Depth(2));
            prop_oneof![
                inner.clone().prop_map(|ret| ConsPat::frame("Succ", ret)),
                inner.clone().prop_map(|ret| ConsPat::frame("Cons", ret)),
                (inner, arg.clone()).prop_map(|(ret, a)| ConsPat::op("add", [a], ret)),
                arg.prop_map(|a| ConsPat::op("add", [a], ConsPat::Top)),
                Just(ConsPat::op("f", [], ConsPat::Top)),
            ]
        })
        .boxed()
    }

    /// A pool with each metavariable of the pool independently present.
    fn arb_pool_over(pool: Pool) -> BoxedStrategy<(Vec<MetaVar>, Vec<MetaVar>)>
    {
        proptest::collection::vec(any::<bool>(), 4)
            .prop_map(move |bits| {
                let (prods, conss) = pool.vars();
                let mut all = prods;
                all.extend(conss);
                let kept: Vec<MetaVar> = all
                    .into_iter()
                    .zip(bits)
                    .filter_map(|(mv, keep)| keep.then_some(mv))
                    .collect();
                split_vars(&kept)
            })
            .boxed()
    }

    /// Split a variable set by category.
    fn split_vars(vars: &[MetaVar]) -> (Vec<MetaVar>, Vec<MetaVar>)
    {
        let prod = vars
            .iter()
            .filter(|mv| mv.cat == Cat::Producer)
            .cloned()
            .collect();
        let cons = vars
            .iter()
            .filter(|mv| mv.cat == Cat::Consumer)
            .cloned()
            .collect();
        (prod, cons)
    }

    /// A positive-polarity command pattern over the pool.
    fn arb_cmdpat(
        pvars: &[MetaVar],
        cvars: &[MetaVar],
        depth: Depth,
    ) -> BoxedStrategy<CmdPat>
    {
        (arb_prodpat(pvars, depth), arb_conspat(pvars, cvars, depth))
            .prop_map(|(prod, cons)| CmdPat::cut(Polarity::Positive, prod, cons))
            .boxed()
    }

    /// A generated surface-rule cell over the pool: positive polarity,
    /// right-hand-side variables drawn from the left-hand side's (the
    /// rewriting discipline, so generated cells can fire), non-linear
    /// left-hand sides admitted.
    fn arb_cell_over(pool: Pool) -> BoxedStrategy<Cell>
    {
        arb_pool_over(pool)
            .prop_flat_map(|(pvars, cvars)| {
                (
                    arb_prodpat(&pvars, Depth(3)),
                    arb_conspat(&pvars, &cvars, Depth(3)),
                )
            })
            .prop_flat_map(|(prod, cons)| {
                let lhs = CmdPat::cut(Polarity::Positive, prod, cons);
                let (pvars, cvars) = split_vars(&cmd_vars(&lhs));
                (
                    arb_prodpat(&pvars, Depth(2)),
                    arb_conspat(&pvars, &cvars, Depth(2)),
                )
                    .prop_map(move |(p, c)| (lhs.clone(), p, c))
            })
            .prop_map(|(lhs, prod, cons)| {
                let rhs = CmdPat::cut(Polarity::Positive, prod, cons);
                Cell::new(
                    lhs,
                    rhs,
                    Orientation::PolarityDerived,
                    CellProvenance::SurfaceRule,
                )
            })
            .boxed()
    }

    /// A generated surface-rule cell over the default pool.
    fn arb_cell() -> BoxedStrategy<Cell>
    {
        arb_cell_over(Pool::Default)
    }

    /// A substitution binding each variable of `vars` to a pattern over the
    /// target pools (empty target pools give a grounding substitution).
    fn arb_subst(
        vars: Vec<MetaVar>,
        ppool: &[MetaVar],
        cpool: &[MetaVar],
    ) -> BoxedStrategy<Subst>
    {
        let mut strategy = Just(Subst::new()).boxed();
        for mv in vars {
            strategy = match mv.cat {
                | Cat::Producer => (strategy, arb_prodpat(ppool, Depth(2)))
                    .prop_map(move |(mut subst, image)| {
                        let _ = subst.bind_prod(mv.clone(), image);
                        subst
                    })
                    .boxed(),
                | Cat::Consumer => (strategy, arb_conspat(ppool, cpool, Depth(2)))
                    .prop_map(move |(mut subst, image)| {
                        let _ = subst.bind_cons(mv.clone(), image);
                        subst
                    })
                    .boxed(),
            };
        }
        strategy
    }

    /// A grounding substitution for `vars` (binds every variable to a
    /// metavariable-free pattern).
    fn arb_ground_subst(vars: Vec<MetaVar>) -> BoxedStrategy<Subst>
    {
        arb_subst(vars, &[], &[])
    }

    /// A generated cell pair plus a grounding of the pair's seam variables —
    /// the directed cospan cases (every enumerated overlap of the ordered
    /// pair gets its cospan factored).
    fn cospan_case(kind: OverlapKind) -> BoxedStrategy<(Cell, Cell, Subst)>
    {
        unifiable_cells(kind)
            .prop_flat_map(move |(a, b)| {
                let mut store = CellStore::new();
                let a_id = store.insert(a.clone());
                let b_id = store.insert(b.clone());
                let family = enumerate_overlaps(&store);
                let mut seam_vars = Vec::new();
                for overlap in &family {
                    if overlap.kind != kind || overlap.left != a_id || overlap.right != b_id {
                        continue;
                    }
                    if let Some(seam) = seam_instance(overlap, &store) {
                        seam_vars.extend(cmd_vars(&seam));
                    }
                    if kind == OverlapKind::Composition {
                        seam_vars.extend(cmd_vars(&overlap.peak));
                    }
                }
                let mut dedup = BTreeSet::new();
                let seam_vars: Vec<MetaVar> = seam_vars
                    .into_iter()
                    .filter(|mv| dedup.insert(mv.clone()))
                    .collect();
                arb_ground_subst(seam_vars).prop_map(move |tau| (a.clone(), b.clone(), tau))
            })
            .boxed()
    }

    /// A cell pair whose designated faces are unifiable by construction (two
    /// independent generalizations of one seam): for confluence the two
    /// left-hand sides, for composition the left cell's right-hand side and
    /// the right cell's left-hand side.
    fn unifiable_cells(kind: OverlapKind) -> BoxedStrategy<(Cell, Cell)>
    {
        unifiable_faces()
            .prop_flat_map(move |(fa, fb)| match kind {
                | OverlapKind::Confluence => (cell_with_lhs(fa), cell_with_lhs(fb))
                    .prop_map(|(a, b)| (a, b))
                    .boxed(),
                | OverlapKind::Composition => (cell_with_rhs(fa), cell_with_lhs(fb))
                    .prop_map(|(a, b)| (a, b))
                    .boxed(),
            })
            .boxed()
    }

    /// A unifiable pair of command faces: independent generalizations of one
    /// generated seam, so the seam is a common instance and unification must
    /// succeed.
    fn unifiable_faces() -> BoxedStrategy<(CmdPat, CmdPat)>
    {
        let decisions = proptest::collection::vec(prop::bool::weighted(0.25), 96);
        arb_pool_over(Pool::Default)
            .prop_flat_map(move |(pvars, cvars)| {
                (
                    arb_prodpat(&pvars, Depth(3)),
                    arb_conspat(&pvars, &cvars, Depth(3)),
                    decisions.clone(),
                    decisions.clone(),
                )
            })
            .prop_map(|(prod, cons, d1, d2)| {
                let seam = CmdPat::cut(Polarity::Positive, prod, cons);
                (
                    generalize_cmd(&seam, GenPrefix::Left, &Decisions(d1)),
                    generalize_cmd(&seam, GenPrefix::Right, &Decisions(d2)),
                )
            })
            .boxed()
    }

    /// A cell with the given left-hand side; the right-hand side draws its
    /// variables from the left's (the rewriting discipline).
    fn cell_with_lhs(face: CmdPat) -> BoxedStrategy<Cell>
    {
        let (pvars, cvars) = split_vars(&cmd_vars(&face));
        (
            arb_prodpat(&pvars, Depth(2)),
            arb_conspat(&pvars, &cvars, Depth(2)),
        )
            .prop_map(move |(prod, cons)| {
                let rhs = CmdPat::cut(Polarity::Positive, prod, cons);
                Cell::new(
                    face.clone(),
                    rhs,
                    Orientation::PolarityDerived,
                    CellProvenance::SurfaceRule,
                )
            })
            .boxed()
    }

    /// A cell with the given right-hand side; the left-hand side mentions
    /// every variable of the right (the rewriting discipline, guaranteed by
    /// construction: producer variables folded into a `Cons` chain, the
    /// consumer variable forced as the return tail — a consumer spine carries
    /// at most one metavariable by grammar).
    fn cell_with_rhs(face: CmdPat) -> BoxedStrategy<Cell>
    {
        let (pvars, cvars) = split_vars(&cmd_vars(&face));
        let cons_tail = cvars.first().cloned();
        (arb_prodpat(&pvars, Depth(1)), arb_prodpat(&pvars, Depth(1)))
            .prop_map(move |(chain_base, arg)| {
                let mut prod = chain_base;
                for var in &pvars {
                    prod = ProdPat::ctor("Cons", [ProdPat::Meta(var.clone()), prod]);
                }
                let cons = match cons_tail.as_ref() {
                    | Some(tail) => ConsPat::op("add", [arg], ConsPat::Meta(tail.clone())),
                    | None => ConsPat::op("add", [arg], ConsPat::Top),
                };
                Cell::new(
                    CmdPat::cut(Polarity::Positive, prod, cons),
                    face.clone(),
                    Orientation::PolarityDerived,
                    CellProvenance::SurfaceRule,
                )
            })
            .boxed()
    }

    /// Every non-root position of a command pattern, shallowest first (an
    /// iterative worklist; [`generalize_cmd`] consumes it).
    fn all_positions(cmd: &CmdPat) -> Vec<Pos>
    {
        let mut out = Vec::new();
        let mut work = vec![(Pos::root(), Node::Cmd(cmd.clone()))];
        while let Some((pos, node)) = work.pop() {
            if !pos.as_ref().is_empty() {
                out.push(pos.clone());
            }
            for (index, child) in node.children().into_iter().enumerate() {
                work.push((pos.child(PositionStep::from(index)), child));
            }
        }
        out.sort_unstable_by_key(|pos| pos.as_ref().len());
        out
    }

    /// A generalization prefix for the fresh metavariable names
    /// [`generalize_cmd`] mints (disjoint across faces and sides).
    #[derive(Clone, Copy, Debug)]
    enum GenPrefix
    {
        /// The left generalization of a seam pair.
        Left,
        /// The right generalization of a seam pair.
        Right,
        /// The left-face generalization of a cell intersection partner.
        LhsFace,
        /// The right-face generalization of a cell intersection partner.
        RhsFace,
    }

    /// A decision vector for [`generalize_cmd`] (one bit per position).
    #[derive(Clone, Debug)]
    #[repr(transparent)]
    struct Decisions(Vec<bool>);

    /// Generalize a command pattern: replace a decision-driven subset of its
    /// subtrees by fresh metavariables (iterative, via the position
    /// machinery; the seam stays a common instance of the result).
    fn generalize_cmd(
        cmd: &CmdPat,
        prefix: GenPrefix,
        decisions: &Decisions,
    ) -> CmdPat
    {
        let stem = match prefix {
            | GenPrefix::Left => "g2",
            | GenPrefix::Right => "g3",
            | GenPrefix::LhsFace => "g2l",
            | GenPrefix::RhsFace => "g2r",
        };
        let mut out = cmd.clone();
        let mut replaced: Vec<Pos> = Vec::new();
        let mut fresh = 0_usize;
        let mut cursor = 0_usize;
        for pos in all_positions(cmd) {
            if replaced
                .iter()
                .any(|r| pos.as_ref().starts_with(r.as_ref()))
            {
                continue;
            }
            let replace = decisions.0[cursor.checked_rem(decisions.0.len()).expect("decisions are nonempty")];
            cursor = cursor.saturating_add(1);
            if !replace {
                continue;
            }
            let Some(node) = subterm_at(&Node::Cmd(out.clone()), &pos)
            else {
                continue;
            };
            let replacement = match node {
                | Node::Prod(_) => Node::Prod(ProdPat::meta(format!("{stem}p{fresh}"))),
                | Node::Cons(_) => Node::Cons(ConsPat::meta(format!("{stem}c{fresh}"))),
                | _ => continue,
            };
            fresh = fresh.saturating_add(1);
            let Some(Node::Cmd(rebuilt)) = splice_at(&Node::Cmd(out.clone()), &pos, replacement)
            else {
                continue;
            };
            out = rebuilt;
            replaced.push(pos);
        }
        out
    }

    /// The Peano `n` as `Succ^count(Zero)`.
    fn nat_of(count: NatSuccCount) -> ProdPat
    {
        let mut acc = ProdPat::ctor("Zero", []);
        for _ in 0 .. count.0 {
            acc = ProdPat::ctor("Succ", [acc]);
        }
        acc
    }

    /// (add-Z): `⟨Zero | add(n; α)⟩ ~> ⟨n | α⟩`.
    fn add_z() -> Cell
    {
        Cell::new(
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::ctor("Zero", []),
                ConsPat::op("add", [ProdPat::meta("n")], ConsPat::meta("alpha")),
            ),
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::meta("n"),
                ConsPat::meta("alpha"),
            ),
            Orientation::PolarityDerived,
            CellProvenance::SurfaceRule,
        )
    }

    /// (add-S): `⟨Succ(m) | add(n; α)⟩ ~> ⟨m | add(n; Succ⁻(α))⟩`.
    fn add_s() -> Cell
    {
        Cell::new(
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::ctor("Succ", [ProdPat::meta("m")]),
                ConsPat::op("add", [ProdPat::meta("n")], ConsPat::meta("alpha")),
            ),
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::meta("m"),
                ConsPat::op(
                    "add",
                    [ProdPat::meta("n")],
                    ConsPat::frame("Succ", ConsPat::meta("alpha")),
                ),
            ),
            Orientation::PolarityDerived,
            CellProvenance::SurfaceRule,
        )
    }

    /// The Peano-add cell store: the `Succ⁻` frame cell, (add-Z), (add-S) —
    /// an orthogonal (no critical pairs), hence convergent, certified
    /// fragment.
    fn peano_store() -> CellStore
    {
        let mut store = CellStore::new();
        store.insert(frame_defining_cell(&Sym::new("Succ")));
        store.insert(add_z());
        store.insert(add_s());
        store
    }

    /// A joinable overlap system: two rules erasing `f`, whose reducts
    /// coincide; the completed store is convergent by the completion
    /// certificates.
    fn joinable_store() -> CellStore
    {
        let mut store = CellStore::new();
        // r1: ⟨Zero | f(α)⟩ ~> ⟨Zero | α⟩
        store.insert(Cell::new(
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::ctor("Zero", []),
                ConsPat::op("f", [], ConsPat::meta("alpha")),
            ),
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::ctor("Zero", []),
                ConsPat::meta("alpha"),
            ),
            Orientation::PolarityDerived,
            CellProvenance::SurfaceRule,
        ));
        // r2: ⟨x | f(α)⟩ ~> ⟨x | α⟩
        store.insert(Cell::new(
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::meta("x"),
                ConsPat::op("f", [], ConsPat::meta("alpha")),
            ),
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::meta("x"),
                ConsPat::meta("alpha"),
            ),
            Orientation::PolarityDerived,
            CellProvenance::SurfaceRule,
        ));
        store
    }

    /// The completed joinable store (convergent by construction).
    fn completed_joinable_store() -> CellStore
    {
        let outcome = complete(
            joinable_store(),
            CompletionBudget::new(64_usize.into(), 32_usize.into(), 128_usize.into()),
        );
        assert!(
            bool::from(outcome.is_completed()),
            "the joinable system completes"
        );
        outcome.store().clone()
    }

    /// A certified convergent store kind for the confluence-scoped rows.
    #[derive(Clone, Copy, Debug)]
    enum CertifiedStore
    {
        /// The Peano-add store (orthogonal, hence convergent).
        Peano,
        /// The completed joinable store (convergent by completion).
        Joinable,
    }

    /// A certified convergent store for the confluence-scoped rows.
    fn certified_store(kind: CertifiedStore) -> CellStore
    {
        match kind {
            | CertifiedStore::Peano => peano_store(),
            | CertifiedStore::Joinable => completed_joinable_store(),
        }
    }

    // ---- Axiom (i): multi-sums in the tight layer ---------------------------

    proptest! {
        #![proptest_config(crdc_config(Cases(2048)))]

        /// The family census: first-order syntactic unification makes the
        /// multi-sum family at most one per ordered pair per kind — the
        /// discrete TRS[Σ] case (the "multi" shape is degenerate here).
        #[test]
        fn multi_sum_families_are_degenerate_singletons(
            (a, b) in (arb_cell(), arb_cell()),
        ) {
            let mut store = CellStore::new();
            let a_id = store.insert(a);
            let b_id = store.insert(b);
            let family = enumerate_overlaps(&store);
            for kind in [OverlapKind::Confluence, OverlapKind::Composition] {
                for (left, right) in [(a_id, b_id), (b_id, a_id)] {
                    let count = family
                        .iter()
                        .filter(|o| o.kind == kind && o.left == left && o.right == right)
                        .count();
                    prop_assert!(
                        count <= 1,
                        "at most one {kind:?} overlap per ordered pair, found {count}"
                    );
                }
            }
        }

        /// Axiom (i), confluence: every cospan of left-face matches factors
        /// through exactly one enumerated overlap, uniquely (Def 2.1).
        #[test]
        fn confluence_cospans_factor_uniquely(
            (a, b, tau) in cospan_case(OverlapKind::Confluence),
        ) {
            let mut store = CellStore::new();
            let a_id = store.insert(a);
            let b_id = store.insert(b);
            let family = enumerate_overlaps(&store);
            let cospans = family
                .iter()
                .filter(|o| o.kind == OverlapKind::Confluence && o.left == a_id && o.right == b_id)
                .count();
            prop_assume!(cospans > 0);
            for overlap in &family {
                if overlap.kind != OverlapKind::Confluence
                    || overlap.left != a_id
                    || overlap.right != b_id
                {
                    continue;
                }
                let seam = overlap.peak.clone();
                let instance = tau.apply_cmd(&seam);
                let left_face = store.get(a_id).expect("fresh id").lhs.clone();
                let left_vars = cmd_vars(&left_face);
                let left_match = compose(&tau, &overlap.unifier, &left_vars);
                let right_face = store.get(b_id).expect("fresh id").lhs.clone();
                let renamed_right_vars = cmd_vars(&overlap.right_renamed().lhs);
                let renamed_right_match = compose(&tau, &overlap.unifier, &renamed_right_vars);
                let right_match = remap_match(
                    &renamed_right_match,
                    &overlap.right_renamed().lhs,
                    &right_face,
                );
                let cospan = Cospan {
                    left_match,
                    right_face,
                    right_match,
                    instance,
                };
                let Factorization::Factored { overlap: factored, mediator } =
                    factor_cospan(a_id, b_id, OverlapKind::Confluence, &cospan, &family, &store)
                else {
                    panic!("the enumerated family must factor its own cospans");
                };
                prop_assert_eq!(
                    factored.as_ref(),
                    overlap,
                    "the unique factor is the overlap the cospan was built from"
                );
                let seam_vars = cmd_vars(&seam);
                prop_assert!(bool::from(substs_agree_on(&mediator, &tau, &seam_vars)));
            }
        }

        /// Axiom (i), composition: every cospan of (left `rhs`, right `lhs`)
        /// matches factors through exactly one enumerated overlap, uniquely.
        #[test]
        fn composition_cospans_factor_uniquely(
            (a, b, tau) in cospan_case(OverlapKind::Composition),
        ) {
            let mut store = CellStore::new();
            let a_id = store.insert(a);
            let b_id = store.insert(b);
            let family = enumerate_overlaps(&store);
            let cospans = family
                .iter()
                .filter(|o| o.kind == OverlapKind::Composition && o.left == a_id && o.right == b_id)
                .count();
            prop_assume!(cospans > 0);
            for overlap in &family {
                if overlap.kind != OverlapKind::Composition
                    || overlap.left != a_id
                    || overlap.right != b_id
                {
                    continue;
                }
                let Some(seam) = seam_instance(overlap, &store) else {
                    continue;
                };
                let instance = tau.apply_cmd(&seam);
                let left_face = store.get(a_id).expect("fresh id").rhs.clone();
                let left_vars = cmd_vars(&left_face);
                let left_match = compose(&tau, &overlap.unifier, &left_vars);
                let right_face = store.get(b_id).expect("fresh id").lhs.clone();
                let renamed_right_vars = cmd_vars(&overlap.right_renamed().lhs);
                let renamed_right_match = compose(&tau, &overlap.unifier, &renamed_right_vars);
                let right_match = remap_match(
                    &renamed_right_match,
                    &overlap.right_renamed().lhs,
                    &right_face,
                );
                let cospan = Cospan {
                    left_match,
                    right_face,
                    right_match,
                    instance,
                };
                let Factorization::Factored { overlap: factored, mediator } =
                    factor_cospan(a_id, b_id, OverlapKind::Composition, &cospan, &family, &store)
                else {
                    panic!("the enumerated family must factor its own cospans");
                };
                prop_assert_eq!(
                    factored.as_ref(),
                    overlap,
                    "the unique factor is the overlap the cospan was built from"
                );
                let seam_vars = cmd_vars(&seam);
                prop_assert!(bool::from(substs_agree_on(&mediator, &tau, &seam_vars)));
            }
        }

        /// Axiom (i), completeness in the wild (confluence): ground the left
        /// cell's left-hand side arbitrarily; whenever the right cell's
        /// left-hand side matches the same ground command, the pair is a
        /// cospan the enumerated family must explain (the independent
        /// direction — matching, never the enumerator, finds the cospan).
        #[test]
        fn confluence_cospans_found_by_matching_factor(
            (a, b, tau) in (arb_cell(), arb_cell()).prop_flat_map(|(a, b)| {
                arb_ground_subst(cmd_vars(&a.lhs)).prop_map(move |tau| (a.clone(), b.clone(), tau))
            }),
        ) {
            let mut store = CellStore::new();
            let a_id = store.insert(a.clone());
            let b_id = store.insert(b.clone());
            let instance = tau.apply_cmd(&a.lhs);
            let mut right_match = Subst::new();
            if !bool::from(match_cmd(&b.lhs, &instance, &mut right_match)) {
                return Ok(());
            }
            let family = enumerate_overlaps(&store);
            let cospan = Cospan {
                left_match: tau,
                right_face: b.lhs.clone(),
                right_match,
                instance,
            };
            match factor_cospan(a_id, b_id, OverlapKind::Confluence, &cospan, &family, &store)
            {
                | Factorization::Factored { .. } => {},
                | Factorization::Incomplete => {
                    prop_assert!(false, "enumeration incomplete: no overlap mediates a matched cospan");
                },
                | Factorization::NonMinimal(members) => {
                    prop_assert!(
                        false,
                        "family not minimal: {} overlaps mediate one cospan",
                        members.len()
                    );
                },
            }
        }

        /// Axiom (i), completeness in the wild (composition): ground the left
        /// cell's right-hand side arbitrarily; whenever the right cell's
        /// left-hand side matches, the enumerated family must factor it.
        #[test]
        fn composition_cospans_found_by_matching_factor(
            (a, b, tau) in (arb_cell(), arb_cell()).prop_flat_map(|(a, b)| {
                arb_ground_subst(cmd_vars(&a.rhs)).prop_map(move |tau| (a.clone(), b.clone(), tau))
            }),
        ) {
            let mut store = CellStore::new();
            let a_id = store.insert(a.clone());
            let b_id = store.insert(b.clone());
            let instance = tau.apply_cmd(&a.rhs);
            let mut right_match = Subst::new();
            if !bool::from(match_cmd(&b.lhs, &instance, &mut right_match)) {
                return Ok(());
            }
            let family = enumerate_overlaps(&store);
            let cospan = Cospan {
                left_match: tau,
                right_face: b.lhs.clone(),
                right_match,
                instance,
            };
            match factor_cospan(a_id, b_id, OverlapKind::Composition, &cospan, &family, &store)
            {
                | Factorization::Factored { .. } => {},
                | Factorization::Incomplete => {
                    prop_assert!(false, "enumeration incomplete: no overlap mediates a matched cospan");
                },
                | Factorization::NonMinimal(members) => {
                    prop_assert!(
                        false,
                        "family not minimal: {} overlaps mediate one cospan",
                        members.len()
                    );
                },
            }
        }
    }

    // ---- Axiom (ii): pullbacks in the tight and cell layers -----------------

    /// A unifiable pattern pair (two generalizations of one seam) with a
    /// grounding of their joint seam variables.
    fn pattern_pullback_case() -> BoxedStrategy<(CmdPat, CmdPat, Subst)>
    {
        unifiable_faces()
            .prop_flat_map(|(p, q)| {
                let mut unifier = Subst::new();
                let seam = if bool::from(unify_cmd(&p, &q, &mut unifier)) {
                    // Guaranteed by construction (generalizations of one seam);
                    // the row asserts it independently.
                    unifier.apply_cmd(&p)
                }
                else {
                    p.clone()
                };
                arb_ground_subst(cmd_vars(&seam)).prop_map(move |tau| (p.clone(), q.clone(), tau))
            })
            .boxed()
    }

    proptest! {
        #![proptest_config(crdc_config(Cases(2048)))]

        /// Axiom (ii), tight layer: unification computes the pullback of two
        /// patterns over a common instance — the square commutes (σ(P) =
        /// σ(Q)) and every common instance factors through the most general
        /// unifier uniquely.
        #[test]
        fn unification_computes_pattern_pullbacks(
            (p, q, tau) in pattern_pullback_case(),
        ) {
            let mut unifier = Subst::new();
            prop_assert!(
                bool::from(unify_cmd(&p, &q, &mut unifier)),
                "generalizations of one seam must unify"
            );
            // The square commutes: the unifier unifies.
            let seam = unifier.apply_cmd(&p);
            prop_assert_eq!(
                &seam,
                &unifier.apply_cmd(&q),
                "the unifier equalizes the two patterns"
            );
            // Universality: the grounded common instance factors through the
            // seam, and the legs factor through the unifier.
            let instance = tau.apply_cmd(&seam);
            let mut mediator = Subst::new();
            prop_assert!(bool::from(match_cmd(&seam, &instance, &mut mediator)));
            let p_vars = cmd_vars(&p);
            let p_leg = compose(&tau, &unifier, &p_vars);
            let p_factored = compose(&mediator, &unifier, &p_vars);
            prop_assert!(bool::from(substs_agree_on(&p_leg, &p_factored, &p_vars)));
            let q_vars = cmd_vars(&q);
            let q_leg = compose(&tau, &unifier, &q_vars);
            let q_factored = compose(&mediator, &unifier, &q_vars);
            prop_assert!(bool::from(substs_agree_on(&q_leg, &q_factored, &q_vars)));
            // The mediator is the grounding itself on the seam variables.
            let seam_vars = cmd_vars(&seam);
            prop_assert!(bool::from(substs_agree_on(&mediator, &tau, &seam_vars)));
        }

        /// Axiom (ii), tight layer in the wild: a common instance found by
        /// matching (never by unification) must still factor through the most
        /// general unifier — completeness of the pullback construction.
        #[test]
        fn matched_pattern_instances_pull_back(
            (p, q, tau) in {
                let (p1, c1) = Pool::Default.vars();
                let (p2, c2) = Pool::Two.vars();
                (arb_cmdpat(&p1, &c1, Depth(3)), arb_cmdpat(&p2, &c2, Depth(3)))
                    .prop_flat_map(|(p, q)| {
                        arb_ground_subst(cmd_vars(&p)).prop_map(move |tau| {
                            (p.clone(), q.clone(), tau)
                        })
                    })
            },
        ) {
            let instance = tau.apply_cmd(&p);
            let mut q_match = Subst::new();
            if !bool::from(match_cmd(&q, &instance, &mut q_match)) {
                return Ok(());
            }
            // (τ, q_match) is a cospan: the pullback must exist and factor it.
            let mut unifier = Subst::new();
            prop_assert!(
                bool::from(unify_cmd(&p, &q, &mut unifier)),
                "a common instance exists, so the patterns must unify"
            );
            let seam = unifier.apply_cmd(&p);
            let mut mediator = Subst::new();
            prop_assert!(
                bool::from(match_cmd(&seam, &instance, &mut mediator)),
                "the common instance factors through the most general unifier"
            );
            let p_vars = cmd_vars(&p);
            let p_factored = compose(&mediator, &unifier, &p_vars);
            prop_assert!(bool::from(substs_agree_on(&tau, &p_factored, &p_vars)));
            let q_vars = cmd_vars(&q);
            let q_factored = compose(&mediator, &unifier, &q_vars);
            prop_assert!(bool::from(substs_agree_on(&q_match, &q_factored, &q_vars)));
        }

        /// Axiom (ii), cell layer: cells with a common instance cell pull
        /// back componentwise — the consistent pair-MGU exists, the
        /// intersection cell is well-formed, and the factorization is unique
        /// on both faces.
        #[test]
        fn cell_intersection_is_componentwise(
            (a, b, tau_a) in arb_cell().prop_flat_map(|a| {
                let mut vars = cmd_vars(&a.lhs);
                vars.extend(cmd_vars(&a.rhs));
                let mut dedup = BTreeSet::new();
                let vars: Vec<MetaVar> = vars
                    .into_iter()
                    .filter(|mv| dedup.insert(mv.clone()))
                    .collect();
                let decisions = proptest::collection::vec(prop::bool::weighted(0.25), 96);
                (arb_ground_subst(vars), decisions.clone(), decisions).prop_map(
                    move |(tau, d1, d2)| {
                        // The common instance cell d = τ_a(a), and b a
                        // generalization of d (so d is an instance of b by
                        // construction; the two faces generalize with
                        // disjoint fresh names, or no shared hole could
                        // bind consistently).
                        let b = Cell::new(
                            generalize_cmd(
                                &tau.apply_cmd(&a.lhs),
                                GenPrefix::LhsFace,
                                &Decisions(d1),
                            ),
                            generalize_cmd(
                                &tau.apply_cmd(&a.rhs),
                                GenPrefix::RhsFace,
                                &Decisions(d2),
                            ),
                            Orientation::PolarityDerived,
                            CellProvenance::SurfaceRule,
                        );
                        (a.clone(), b, tau)
                    },
                )
            }),
        ) {
            // The common instance cell: d = τ_a(a) on both faces.
            let d_lhs = tau_a.apply_cmd(&a.lhs);
            let d_rhs = tau_a.apply_cmd(&a.rhs);
            // b admits d as an instance consistently (one substitution) —
            // guaranteed by the generalization construction.
            let mut b_match = Subst::new();
            prop_assert!(bool::from(match_cmd(&b.lhs, &d_lhs, &mut b_match)));
            prop_assert!(bool::from(match_cmd(&b.rhs, &d_rhs, &mut b_match)));
            // The pair-MGU: unify the left-hand sides, then the instantiated
            // right-hand sides, through one substitution.
            let mut unifier = Subst::new();
            prop_assert!(
                bool::from(unify_cmd(&a.lhs, &b.lhs, &mut unifier)),
                "a common instance exists, so the left-hand sides unify"
            );
            prop_assert!(
                bool::from(unify_cmd(
                    &unifier.apply_cmd(&a.rhs),
                    &unifier.apply_cmd(&b.rhs),
                    &mut unifier,
                )),
                "the right-hand sides unify through the same substitution"
            );
            // The intersection cell is well-formed: both faces agree.
            prop_assert_eq!(
                unifier.apply_cmd(&a.lhs),
                unifier.apply_cmd(&b.lhs),
                "the intersection's left face"
            );
            prop_assert_eq!(
                unifier.apply_cmd(&a.rhs),
                unifier.apply_cmd(&b.rhs),
                "the intersection's right face"
            );
            // The common instance factors through the intersection,
            // componentwise, and the legs factor uniquely.
            let i_lhs = unifier.apply_cmd(&a.lhs);
            let i_rhs = unifier.apply_cmd(&a.rhs);
            let mut mediator = Subst::new();
            prop_assert!(bool::from(match_cmd(&i_lhs, &d_lhs, &mut mediator)));
            prop_assert!(bool::from(match_cmd(&i_rhs, &d_rhs, &mut mediator)));
            let mut a_vars = cmd_vars(&a.lhs);
            a_vars.extend(cmd_vars(&a.rhs));
            let mut dedup_a = BTreeSet::new();
            let a_vars: Vec<MetaVar> = a_vars
                .into_iter()
                .filter(|mv| dedup_a.insert(mv.clone()))
                .collect();
            let a_factored = compose(&mediator, &unifier, &a_vars);
            prop_assert!(bool::from(substs_agree_on(&tau_a, &a_factored, &a_vars)));
            let mut b_vars = cmd_vars(&b.lhs);
            b_vars.extend(cmd_vars(&b.rhs));
            let mut dedup_b = BTreeSet::new();
            let b_vars: Vec<MetaVar> = b_vars
                .into_iter()
                .filter(|mv| dedup_b.insert(mv.clone()))
                .collect();
            let b_factored = compose(&mediator, &unifier, &b_vars);
            prop_assert!(bool::from(substs_agree_on(&b_match, &b_factored, &b_vars)));
        }
    }

    // ---- Axiom (iii): horizontal decomposition -------------------------------

    proptest! {
        #![proptest_config(crdc_config(Cases(1024)))]

        /// Axiom (iii): a cell over a composed seam factors, up to a globular
        /// iso, as a horizontal composition of per-frame cells — the fused
        /// cell decomposes as the two-step derivation, and the factorization
        /// is **strict** (the iso residue is the identity on the boundary).
        #[test]
        fn fused_cells_decompose_as_two_step_derivations(
            (a, b, tau) in cospan_case(OverlapKind::Composition),
        ) {
            let mut store = CellStore::new();
            let a_id = store.insert(a);
            let b_id = store.insert(b);
            let family = enumerate_overlaps(&store);
            let compositions = family
                .iter()
                .filter(|o| o.kind == OverlapKind::Composition)
                .count();
            prop_assume!(compositions > 0);
            for overlap in &family {
                if overlap.kind != OverlapKind::Composition
                    || overlap.left != a_id
                    || overlap.right != b_id
                {
                    continue;
                }
                let HorizontalFactorization { frames, witness } =
                    decompose_horizontal(overlap, &mut store)
                        .expect("the fused cell of a composition overlap exists");
                prop_assert_eq!(
                    frames.as_slice(),
                    &[overlap.left, overlap.right],
                    "the factorization is the per-frame two-step"
                );
                // The globular-iso witness replays (the fused ≡ two-step
                // certificate) and is strict: the recorded two-step path is
                // [left@root, right@seam], the one-step path is [fused@root],
                // and both land on the composite.
                prop_assert!(
                    bool::from(witness.replay(&store)),
                    "the fused ≡ two-step certificate replays"
                );
                prop_assert_eq!(witness.path_a.len(), 2, "the two-step path");
                prop_assert_eq!(witness.path_b.len(), 1, "the fused one-step path");
                // The differential: on ground instances of the peak, firing
                // the fused cell agrees with firing the frames in sequence.
                let instance = tau.apply_cmd(&overlap.peak);
                let fused_id = witness.path_b[0].cell;
                let fused_cell = store.get(fused_id).expect("the fused id is fresh");
                let via_fused = rewrite_at(fused_cell, &instance, &Pos::root());
                let left_cell = store.get(overlap.left).expect("left id");
                let right_cell = store.get(overlap.right).expect("right id");
                let via_two_step = rewrite_at(left_cell, &instance, &Pos::root())
                    .and_then(|mid| rewrite_at(right_cell, &mid, &Pos::root()));
                prop_assert!(via_fused.is_some(), "the fused cell fires on the instance");
                prop_assert_eq!(
                    via_fused,
                    via_two_step,
                    "the fused cell and the two-step composite agree"
                );
            }
        }
    }

    // ---- Axiom (iv): source a strong multi-opfibration ------------------------

    /// A generated cell, an instantiation of its variables, and a grounding
    /// of the instantiated left-hand side — the source-pushforward cases.
    fn pushforward_case() -> BoxedStrategy<(Cell, Subst, Subst)>
    {
        arb_cell()
            .prop_flat_map(|cell| {
                let (p2, c2) = Pool::Two.vars();
                let vars = cmd_vars(&cell.lhs);
                arb_subst(vars, &p2, &c2).prop_map(move |sigma| (cell.clone(), sigma))
            })
            .prop_flat_map(|(cell, sigma)| {
                let lifted_vars = cmd_vars(&sigma.apply_cmd(&cell.lhs));
                arb_ground_subst(lifted_vars).prop_map(move |m| (cell.clone(), sigma.clone(), m))
            })
            .boxed()
    }

    proptest! {
        #![proptest_config(crdc_config(Cases(2048)))]

        /// Axiom (iv): rewriting is substitution-stable — the lifted cell and
        /// the original cell agree on every ground instance of the lifted
        /// left-hand side (the TRS[Σ] discrete lift is sound).
        #[test]
        fn source_pushforward_is_substitution_stable(
            (cell, sigma, m) in pushforward_case(),
        ) {
            let lifts = pushforward_src(&cell, &sigma);
            prop_assert_eq!(lifts.len(), 1, "the source lift is discrete (a singleton)");
            let lifted = &lifts[0].0;
            let instance = m.apply_cmd(&lifted.lhs);
            let via_lifted = rewrite_at(lifted, &instance, &Pos::root());
            let via_original = rewrite_at(&cell, &instance, &Pos::root());
            prop_assert!(via_lifted.is_some(), "the lifted cell fires on its instances");
            prop_assert_eq!(
                via_lifted,
                via_original,
                "the lifted cell and the original agree on the instance"
            );
        }

        /// Axiom (iv): the singleton lift is op-Cartesian — every match of
        /// the lifted left-hand side factors through the substitution,
        /// uniquely on the occurring variables (and matching is
        /// deterministic).
        #[test]
        fn source_pushforward_factors_matches_uniquely(
            (cell, sigma, m) in pushforward_case(),
        ) {
            let lifted = &pushforward_src(&cell, &sigma)[0].0;
            let instance = m.apply_cmd(&lifted.lhs);
            let mut direct = Subst::new();
            prop_assert!(bool::from(match_cmd(&cell.lhs, &instance, &mut direct)));
            let mut through = Subst::new();
            prop_assert!(bool::from(match_cmd(&lifted.lhs, &instance, &mut through)));
            // The factorization: the direct match is the through-match
            // composed with the substitution, on the cell's variables.
            let cell_vars = cmd_vars(&cell.lhs);
            let factored = compose(&through, &sigma, &cell_vars);
            prop_assert!(bool::from(substs_agree_on(&direct, &factored, &cell_vars)));
            // Determinism (uniqueness of the mediator): re-matching agrees.
            let mut again = Subst::new();
            prop_assert!(bool::from(match_cmd(&lifted.lhs, &instance, &mut again)));
            let through_vars = cmd_vars(&lifted.lhs);
            prop_assert!(bool::from(substs_agree_on(&through, &again, &through_vars)));
        }
    }

    // ---- Axiom (v): target a residual multi-opfibration -----------------------

    proptest! {
        #![proptest_config(crdc_config(Cases(256)))]

        /// Axiom (v): the pushed-forward derivation exists on every ground
        /// instance — fire the lifted cell, then normalize; the residue
        /// records the owed post-normalization (Def 2.8's `post`). The
        /// budget is deliberately shallow: generated stores can loop-rewrite,
        /// and a divergent generated system is outside the convergent
        /// fragment, not an axiom failure.
        #[test]
        fn target_pushforward_exists_with_derivation_residue(
            (cell, sigma, m) in pushforward_case(),
        ) {
            let mut store = CellStore::new();
            store.insert(cell.clone());
            let lifted = pushforward_tgt(&cell, &sigma);
            prop_assert_eq!(lifted.len(), 1, "the target lift is a singleton family");
            let lift = lifted[0].clone();
            store.insert(lift.0.clone());
            let instance = m.apply_cmd(&lift.0.lhs);
            let budget = NormalizationBudget::from(12_usize);
            let Some((normal, residue)) = residue_of(&store, &lift, &instance, budget) else {
                // Budget exhaustion is a divergent generated system — outside
                // the convergent fragment, not an axiom failure.
                return Ok(());
            };
            // The residue is the derivation the composite still owes:
            // re-running it from the fired output lands on the normal form.
            let fired = rewrite_at(&lift.0, &instance, &Pos::root())
                .expect("the lifted cell fires on its instances");
            let replayed = run_path(&store, &fired, &residue.post);
            prop_assert_eq!(
                replayed.as_ref(),
                Some(&normal),
                "the residue path replays to the pushed output"
            );
        }

        /// Axiom (v): lifts compose functorially — pushing along τ after σ is
        /// pushing along τ ∘ σ (Def 2.8's `post ∘ f` bookkeeping at the lift
        /// level).
        #[test]
        fn target_pushforward_lifts_compose(
            (cell, sigma, tau) in arb_cell().prop_flat_map(|cell| {
                let (p2, c2) = Pool::Two.vars();
                arb_subst(cmd_vars(&cell.lhs), &p2, &c2)
                    .prop_map(move |sigma| (cell.clone(), sigma))
            })
            .prop_flat_map(|(cell, sigma)| {
                let (p3, c3) = Pool::Three.vars();
                let mut vars = cmd_vars(&sigma.apply_cmd(&cell.lhs));
                vars.extend(cmd_vars(&sigma.apply_cmd(&cell.rhs)));
                let mut dedup = BTreeSet::new();
                let vars: Vec<MetaVar> = vars
                    .into_iter()
                    .filter(|mv| dedup.insert(mv.clone()))
                    .collect();
                arb_subst(vars, &p3, &c3).prop_map(move |tau| {
                    (cell.clone(), sigma.clone(), tau)
                })
            }),
        ) {
            let staged = pushforward_tgt(&pushforward_tgt(&cell, &sigma)[0].0, &tau);
            let mut vars = cmd_vars(&cell.lhs);
            vars.extend(cmd_vars(&cell.rhs));
            let mut dedup = BTreeSet::new();
            let vars: Vec<MetaVar> = vars
                .into_iter()
                .filter(|mv| dedup.insert(mv.clone()))
                .collect();
            let direct = pushforward_tgt(&cell, &compose(&tau, &sigma, &vars));
            prop_assert_eq!(
                &staged[0].0,
                &direct[0].0,
                "τ ∘ (σ-lift) coincides with the (τ ∘ σ)-lift"
            );
        }

        /// Axiom (v): pushed derivations are confluence-unique on the
        /// certified convergent fragment — normalizing an instance directly
        /// and normalizing its pushed output reach the same normal form (the
        /// residual factorization over the convergent store).
        #[test]
        fn target_pushforward_is_confluence_unique(
            (joinable, cell, sigma, m) in (any::<bool>(), 0_usize .. 16)
                .prop_flat_map(|(joinable, pick)| {
                    let store = certified_store(if joinable {
                        CertifiedStore::Joinable
                    }
                    else {
                        CertifiedStore::Peano
                    });
                    let cells: Vec<Cell> =
                        store.iter().map(|(_, cell)| cell.clone()).collect();
                    let cell = cells[pick.checked_rem(cells.len()).expect("cells are nonempty")].clone();
                    let (p2, c2) = Pool::Two.vars();
                    arb_subst(cmd_vars(&cell.lhs), &p2, &c2)
                        .prop_map(move |sigma| (joinable, cell.clone(), sigma))
                })
                .prop_flat_map(|(joinable, cell, sigma)| {
                    let mut vars = cmd_vars(&sigma.apply_cmd(&cell.lhs));
                    vars.extend(cmd_vars(&sigma.apply_cmd(&cell.rhs)));
                    let mut dedup = BTreeSet::new();
                    let vars: Vec<MetaVar> = vars
                        .into_iter()
                        .filter(|mv| dedup.insert(mv.clone()))
                        .collect();
                    arb_ground_subst(vars).prop_map(move |m| {
                        (joinable, cell.clone(), sigma.clone(), m)
                    })
                }),
        ) {
            let store = certified_store(if joinable {
                CertifiedStore::Joinable
            }
            else {
                CertifiedStore::Peano
            });
            let lifted = lift_cell(&cell, &sigma);
            let instance = m.apply_cmd(&lifted.lhs);
            let pushed_output = m.apply_cmd(&lifted.rhs);
            let budget = NormalizationBudget::from(64_usize);
            let direct = normalize(&store, &instance, budget);
            let pushed = normalize(&store, &pushed_output, budget);
            prop_assume!(!direct.exhausted);
            prop_assume!(!pushed.exhausted);
            prop_assert_eq!(
                direct.normal,
                pushed.normal,
                "the instance and its pushed output join on the convergent fragment"
            );
        }
    }

    /// The residue census on the Peano family (deterministic golden):
    /// instantiating (add-Z)'s output with a redex-bearing continuation
    /// forces a three-step residue; a non-redex instantiation owes nothing.
    /// This is the empirical answer to the open question of which cell
    /// classes exercise the residual part of axiom (v): exactly the
    /// instantiations that create redexes.
    #[test]
    fn residue_census_on_the_peano_family()
    {
        let store = peano_store();
        let budget = NormalizationBudget::from(64_usize);
        // σ redex-bearing: n ↦ Succ(Zero), α ↦ add(Zero; ★).
        let mut sigma = Subst::new();
        let _ = sigma.bind_prod(
            MetaVar::producer("n"),
            ProdPat::ctor("Succ", [ProdPat::ctor("Zero", [])]),
        );
        let _ = sigma.bind_cons(
            MetaVar::consumer("alpha"),
            ConsPat::op("add", [ProdPat::ctor("Zero", [])], ConsPat::Top),
        );
        let add_z_cell = add_z();
        let lifted = pushforward_tgt(&add_z_cell, &sigma);
        let instance = sigma.apply_cmd(&add_z_cell.lhs);
        let (normal, residue) = residue_of(&store, &lifted[0], &instance, budget)
            .expect("the Peano fragment converges");
        assert_eq!(
            residue.post.len(),
            3,
            "the residue records three owed steps (add-S, add-Z, frame)"
        );
        assert_eq!(
            normal,
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::ctor("Succ", [ProdPat::ctor("Zero", [])]),
                ConsPat::Top,
            ),
            "the pushed output is the normal form ⟨Succ(Zero) | ★⟩"
        );
        // σ trivial: n ↦ Zero, α ↦ ★ — no redex created, no residue.
        let mut trivial = Subst::new();
        let _ = trivial.bind_prod(MetaVar::producer("n"), ProdPat::ctor("Zero", []));
        let _ = trivial.bind_cons(MetaVar::consumer("alpha"), ConsPat::Top);
        let lifted_trivial = pushforward_tgt(&add_z_cell, &trivial);
        let instance_trivial = trivial.apply_cmd(&add_z_cell.lhs);
        let (_normal, residue_trivial) =
            residue_of(&store, &lifted_trivial[0], &instance_trivial, budget)
                .expect("the Peano fragment converges");
        assert!(
            residue_trivial.post.is_empty(),
            "a non-redex instantiation owes no residue"
        );
    }

    // ---- Virtual-side line items (Thompson–Carlson) ---------------------------

    /// The Peano instance arguments (successor depths).
    #[derive(Clone, Copy, Debug)]
    struct PeanoArgs(u8, u8);

    /// A Peano ground instance `⟨Succ^a(Zero) | Succ⁻(add(Succ^b(Zero); ★))⟩`
    /// with a nontrivial normalization path (frame, then `a+1` (add-S), then
    /// (add-Z), then `a+1` frames).
    fn peano_instance(args: PeanoArgs) -> CmdPat
    {
        CmdPat::cut(
            Polarity::Positive,
            nat_of(NatSuccCount(args.0)),
            ConsPat::frame(
                "Succ",
                ConsPat::op("add", [nat_of(NatSuccCount(args.1))], ConsPat::Top),
            ),
        )
    }

    proptest! {
        #![proptest_config(crdc_config(Cases(512)))]

        /// Virtual side, positive globular decompositions: every derivation
        /// path decomposes uniquely into its one-step cells (the free path
        /// algebra) and recomposes — the recorded path is the decomposition,
        /// and deterministic normalization makes it unique.
        #[test]
        fn paths_decompose_uniquely_into_steps(
            a in 0u8 .. 6,
            b in 0u8 .. 6,
        ) {
            let store = peano_store();
            let instance = peano_instance(PeanoArgs(a, b));
            let budget = NormalizationBudget::from(64_usize);
            let norm = normalize(&store, &instance, budget);
            prop_assume!(!norm.exhausted);
            // Recomposition: the decomposition folds back to the normal form.
            let recomposed = run_path(&store, &instance, &norm.path);
            prop_assert_eq!(
                recomposed.as_ref(),
                Some(&norm.normal),
                "the path recomposes to the normal form"
            );
            // Uniqueness: normalization is deterministic, so the
            // decomposition is the unique one the engine produces.
            let again = normalize(&store, &instance, budget);
            prop_assert_eq!(&again.path, &norm.path, "the decomposition is unique");
            // Positivity: no step is a unit — every recorded step actually
            // fires (changes the term) at its position.
            let mut current = instance;
            for step in &norm.path {
                let cell = store.get(step.cell).expect("path ids are fresh");
                let next = rewrite_at(cell, &current, &step.at)
                    .expect("a recorded step fires");
                prop_assert_ne!(&next, &current, "no step is a unit");
                current = next;
            }
        }

        /// Virtual side, pro-representability: every two-step composite is
        /// representable — the fused cell exists as a cell of the store and
        /// represents the composite (the exponentiability chain's
        /// decomposable ⇔ pro-representable direction, Thm 5.11).
        #[test]
        fn two_step_composites_are_pro_representable(
            (a, b, _tau) in cospan_case(OverlapKind::Composition),
        ) {
            let mut store = CellStore::new();
            store.insert(a);
            store.insert(b);
            let family = enumerate_overlaps(&store);
            let compositions = family
                .iter()
                .filter(|o| o.kind == OverlapKind::Composition)
                .count();
            prop_assume!(compositions > 0);
            for overlap in &family {
                if overlap.kind != OverlapKind::Composition {
                    continue;
                }
                let (fused_id, witness) = derive_fused(overlap, &mut store)
                    .expect("the composite is representable as a fused cell");
                let fused = store.get(fused_id).expect("the fused id is fresh");
                // The representable's boundary is the composite's boundary.
                prop_assert_eq!(&fused.lhs, &overlap.peak, "source is the peak");
                prop_assert_eq!(&fused.rhs, &witness.joins_at, "target is the composite");
            }
        }

        /// Virtual side, cellular Conduché (the 2-layer criterion, Thm 4.14):
        /// path splittings refine — for a path split as p = p₁ · p₂ and a
        /// further split of p₁, the staged replays agree (concatenation is
        /// free, so the criterion holds strictly).
        #[test]
        fn path_splittings_refine(
            a in 1u8 .. 6,
            b in 1u8 .. 6,
            i_raw in any::<u8>(),
            j_raw in any::<u8>(),
        ) {
            let store = peano_store();
            let instance = peano_instance(PeanoArgs(a, b));
            let budget = NormalizationBudget::from(64_usize);
            let norm = normalize(&store, &instance, budget);
            prop_assume!(!norm.exhausted);
            prop_assume!(norm.path.len() >= 2);
            let i = usize::from(i_raw).checked_rem(norm.path.len().saturating_add(1)).expect("saturating_add(1) is nonzero");
            let j = usize::from(j_raw).checked_rem(i.saturating_add(1)).expect("saturating_add(1) is nonzero");
            let (p1, p2) = norm.path.split_at(i);
            let (q1, q2) = p1.split_at(j);
            // Staged replay: q1, then q2, then p2.
            let staged = run_path(&store, &instance, q1)
                .and_then(|mid| run_path(&store, &mid, q2))
                .and_then(|mid| run_path(&store, &mid, p2));
            prop_assert_eq!(
                staged.as_ref(),
                Some(&norm.normal),
                "the refined splitting recomposes to the same normal form"
            );
        }
    }
}
