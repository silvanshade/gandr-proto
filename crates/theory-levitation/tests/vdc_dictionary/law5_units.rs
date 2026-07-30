//! **Law 5 — units: path-relation cell-induction** (proposal §3 item 5;
//! expected PARTIAL — the informative one).
//!
//! Path formation is real (the bounded rewriter over the fixture `plus` /
//! `double` rules), and `PathInd` satisfies its β-rule on `refl`. But on a
//! **non-empty** path the raw induction declines: the base cell's shared middle
//! variable requires the chain endpoints to meet, and a path between them
//! cannot be absorbed by any generating instance. That decline **locates the
//! obligation**: the unit universal property's bijectivity requires the
//! instance notion to be closed under path action — a saturation invariant on
//! the future cell store (instances-as-modules over the rewrite-path relation).
//! In metatheory terms this saturation is the store being a **profunctor**: the
//! left/right path action (`actˡ`/`actʳ`) with its module laws is exactly this
//! closure, a `SaturatedInstance` is a free module element, and the value the
//! probe computes is the co-Yoneda (density) reduction of the right action
//! (Loregian, *Coend calculus*, arXiv:1501.02503).
//! The saturation probe shows that with path absorption the induced value is
//! defined and β-compatible, so the invariant is sufficient, not merely
//! necessary.

#[cfg(test)]
mod law5
{
    use alloc::collections::BTreeMap;

    use gandr_theory_levitation::FreeTerm;
    use gandr_theory_levitation::Name;

    use super::fixtures::gen_x;
    use super::fixtures::loose_of;
    use super::fixtures::nat;
    use super::fixtures::nat_desc;
    use super::fixtures::nat_sig;
    use super::fixtures::succ;
    use super::fixtures::unary_relation;
    use super::fixtures::var;
    use super::fixtures::zero;
    use super::harness::BaseInstance;
    use super::harness::Cell;
    use super::harness::CellClause;
    use super::harness::CellKind;
    use super::harness::LooseArrow;
    use super::harness::LooseInstance;
    use super::harness::SaturatedInstance;
    use super::harness::SigMorphism;
    use super::harness::apply_path;
    use super::harness::cells_equal;
    use super::harness::enumerate_paths;
    use super::harness::replay;
    use super::harness::replay_path_ind_saturated;

    #[test]
    fn saturation_makes_the_non_refl_value_defined_and_beta_compatible()
    {
        let desc = nat_desc();
        let cells = &desc.cells;
        let base = base_cell(var("p0.x".into()));
        let a = gen_x(nat(1.into()));
        let b = gen_x(nat(2.into()));

        // A worked non-empty absorbed path: plus(Zero, Zero) reduces to Zero.
        let path_start = FreeTerm::op("plus", [zero(), zero()]);
        let one_step = enumerate_paths(&path_start, cells, 1.into())
            .into_iter()
            .find(|entry| {
                let (ref steps, _) = *entry;
                !steps.is_empty()
            })
            .expect("plus(Zero, Zero) reduces");
        let saturated = SaturatedInstance {
            generator: BaseInstance::Gen {
                generator: 0.into(),
                subst: BTreeMap::new(),
            },
            absorbed: one_step.0,
            path_start,
        };

        // Defined even though the raw induction would decline on a non-empty path.
        let value = replay_path_ind_saturated(&base, &a, &saturated, &b, cells);
        assert!(
            value.is_some(),
            "with path absorption the induced value IS defined"
        );

        // β-compatible: on an empty absorbed path it agrees with the refl value.
        let refl_saturated = SaturatedInstance {
            generator: BaseInstance::Gen {
                generator: 0.into(),
                subst: BTreeMap::new(),
            },
            absorbed: Vec::new(),
            path_start: nat(0.into()),
        };
        let empty_value = replay_path_ind_saturated(&base, &a, &refl_saturated, &b, cells);
        let refl_value = replay(&base, &[a, b]);
        assert_eq!(
            empty_value, refl_value,
            "on refl the saturated value matches the β-value"
        );
    }

    #[test]
    fn path_induction_satisfies_beta_on_refl()
    {
        let base = base_cell(var("p0.x".into()));
        let induction = path_ind_cell(base.clone());
        let a = gen_x(nat(1.into()));
        let b = gen_x(nat(2.into()));
        let refl_at = refl(nat(3.into()));

        let via_induction = replay(&induction, &[a.clone(), refl_at, b.clone()]);
        let via_base = replay(&base, &[a, b]);
        assert_eq!(
            via_induction, via_base,
            "PathInd{{μ}}(a, refl, b) = μ(a, b)"
        );
        assert!(via_induction.is_some(), "the β-value is defined");
    }
    #[test]
    fn path_induction_declines_on_a_non_empty_path()
    {
        let desc = nat_desc();
        let cells = &desc.cells;
        let start = FreeTerm::op("plus", [succ(zero()), zero()]);
        let paths = enumerate_paths(&start, cells, 3.into());
        let (steps, _end) = paths
            .into_iter()
            .find(|entry| {
                let (ref steps, _) = *entry;
                !steps.is_empty()
            })
            .expect("a non-empty path exists");
        let nonrefl = LooseInstance {
            per_factor: vec![BaseInstance::Path { start, steps }],
        };

        let base = base_cell(var("p0.x".into()));
        let induction = path_ind_cell(base);
        let declined = replay(&induction, &[
            gen_x(nat(1.into())),
            nonrefl,
            gen_x(nat(2.into())),
        ]);
        assert_eq!(
            None, declined,
            "PathInd declines on a non-empty path — the located obligation: instances must be \
             closed under path action (a cell-store saturation invariant)"
        );
    }
    #[test]
    fn distinct_bases_give_distinct_inductions_at_refl()
    {
        let induction_one = path_ind_cell(base_cell(var("p0.x".into())));
        let induction_two = path_ind_cell(base_cell(succ(var("p0.x".into()))));
        let corpus: Vec<Vec<LooseInstance>> = (0 ..= 3_usize)
            .map(|k| {
                vec![
                    gen_x(nat(k.into())),
                    refl(nat(k.into())),
                    gen_x(nat(k.saturating_add(1).into())),
                ]
            })
            .collect();
        assert!(
            !bool::from(cells_equal(&induction_one, &induction_two, &corpus)),
            "PathInd cells over replay-distinct bases are replay-distinct at refl"
        );
    }
    /// A two-input base cell `(a, b) ↦ S`: matches generator `0` at both
    /// inputs, emitting generator `0` with `x` bound by the given template
    /// (over `p0.x`) and `y` bound to `p1.x`.
    fn base_cell(template_for_x: FreeTerm) -> Cell
    {
        let mut emit_templates: BTreeMap<Name, FreeTerm> = BTreeMap::new();
        emit_templates.insert("x".into(), template_for_x);
        emit_templates.insert("y".into(), var("p1.x".into()));
        Cell {
            dom: vec![
                loose_of(unary_relation("A".into())),
                loose_of(unary_relation("B".into())),
            ],
            cod: loose_of(unary_relation("S".into())),
            left_frame: SigMorphism::identity(&nat_sig()),
            right_frame: SigMorphism::identity(&nat_sig()),
            kind: CellKind::Clauses(vec![CellClause {
                matches: vec![0.into(), 0.into()],
                emit: vec![(0.into(), emit_templates)],
            }]),
        }
    }
    /// A `PathInd` cell over the given base, with domain `[A, path, B]`.
    fn path_ind_cell(base: Cell) -> Cell
    {
        Cell {
            dom: vec![
                loose_of(unary_relation("A".into())),
                LooseArrow::path(&nat_sig()),
                loose_of(unary_relation("B".into())),
            ],
            cod: loose_of(unary_relation("S".into())),
            left_frame: SigMorphism::identity(&nat_sig()),
            right_frame: SigMorphism::identity(&nat_sig()),
            kind: CellKind::PathInd {
                base: Box::new(base),
            },
        }
    }

    /// A `refl` path instance at `start`.
    fn refl(start: FreeTerm) -> LooseInstance
    {
        LooseInstance {
            per_factor: vec![BaseInstance::Path {
                start,
                steps: Vec::new(),
            }],
        }
    }

    #[test]
    fn path_formation_is_real_and_deterministic()
    {
        let desc = nat_desc();
        let cells = &desc.cells;
        // plus(Succ(Zero), Zero) reduces to Succ(Zero) in two steps.
        let start = FreeTerm::op("plus", [succ(zero()), zero()]);
        let paths = enumerate_paths(&start, cells, 3.into());

        assert!(
            paths.iter().any(|entry| {
                let (ref steps, _) = *entry;
                steps.is_empty()
            }),
            "refl (the empty path) is enumerated"
        );
        assert!(
            paths.iter().any(|entry| {
                let (ref steps, _) = *entry;
                steps.len() == 1
            }),
            "a genuine one-step reduction is enumerated via the plus rules"
        );
        // A full reduction reaches the normal form Succ(Zero) = 1 + 0.
        let full = (0 .. paths.len())
            .find_map(|i| {
                let entry = &paths[i];
                (!entry.0.is_empty() && entry.1 == succ(zero())).then_some(entry)
            })
            .expect("a full reduction to Succ(Zero) exists");
        assert_eq!(
            apply_path(&start, &full.0, cells),
            Some(succ(zero())),
            "replaying a real reduction path reaches Succ(Zero)"
        );
        // Determinism: the enumeration is reproducible.
        assert_eq!(
            paths,
            enumerate_paths(&start, cells, 3.into()),
            "path enumeration is deterministic"
        );
    }
}
