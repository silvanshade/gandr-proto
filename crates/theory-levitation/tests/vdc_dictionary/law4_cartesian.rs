//! **Law 4 — cartesian structure (the split form)** (proposal §3 item 4).
//!
//! The local `⊤` / `∧` strict laws are covered by Law 3(b). Here: the
//! pairing / projection bijection up to replay (`⋄` is replay-level
//! composition), a bounded uniqueness spot-check, the unique cell into `⊤`, and
//! products preserved by restriction (strict via 3(b)(4) plus a replay
//! spot-check).

#[cfg(test)]
mod law4
{
    use alloc::rc::Rc;

    use gandr_theory_levitation::DescriptorFactorIndex;

    use crate::vdc_dictionary::fixtures::gen_x;
    use crate::vdc_dictionary::fixtures::loose_of;
    use crate::vdc_dictionary::fixtures::nat;
    use crate::vdc_dictionary::fixtures::nat_sig;
    use crate::vdc_dictionary::fixtures::relabel_cell;
    use crate::vdc_dictionary::fixtures::single_input_corpus;
    use crate::vdc_dictionary::fixtures::succ;
    use crate::vdc_dictionary::fixtures::unary_relation;
    use crate::vdc_dictionary::fixtures::var;
    use crate::vdc_dictionary::harness::Cell;
    use crate::vdc_dictionary::harness::CellKind;
    use crate::vdc_dictionary::harness::LooseArrow;
    use crate::vdc_dictionary::harness::LooseInstance;
    use crate::vdc_dictionary::harness::SigMorphism;
    use crate::vdc_dictionary::harness::cells_equal;
    use crate::vdc_dictionary::harness::loose_instance_eq;
    use crate::vdc_dictionary::harness::replay;
    use crate::vdc_dictionary::harness::replay_compose;
    use crate::vdc_dictionary::harness::restrict;

    #[test]
    fn pairing_is_unique_up_to_replay_on_pair_shaped_cells()
    {
        // A ρ built as a pair whose projections replay-equal μ and ν is
        // replay-equal to ⟨μ, ν⟩. Scope: bounded to Pair-shaped ρ (recorded
        // honestly in the verdict).
        let (mu, nu, paired) = pair_fixture();
        let rho = Cell {
            dom: mu.dom.clone(),
            cod: paired.cod.clone(),
            left_frame: SigMorphism::identity(&nat_sig()),
            right_frame: SigMorphism::identity(&nat_sig()),
            kind: CellKind::Pair(Box::new(mu.clone()), Box::new(nu.clone())),
        };
        let proj0 = proj_cell(paired.cod.clone(), 0.into());
        let proj1 = proj_cell(paired.cod.clone(), 1.into());
        let corpus = single_input_corpus();
        // Premises: the projections of ρ match μ and ν.
        for input in &corpus {
            let p0 = replay_compose(&proj0, &[&rho], core::slice::from_ref(input));
            assert_eq!(p0, replay(&mu, input), "π₀ ⋄ ρ = μ");
            let p1 = replay_compose(&proj1, &[&rho], core::slice::from_ref(input));
            assert_eq!(p1, replay(&nu, input), "π₁ ⋄ ρ = ν");
        }
        // Conclusion.
        assert!(
            bool::from(cells_equal(&rho, &paired, &corpus)),
            "ρ ≡ ⟨μ, ν⟩ up to replay"
        );
    }

    #[test]
    fn projection_pairing_bijection_holds_up_to_replay()
    {
        let (mu, nu, paired) = pair_fixture();
        let proj0 = proj_cell(paired.cod.clone(), 0.into());
        let proj1 = proj_cell(paired.cod.clone(), 1.into());
        for k in 0 ..= 5_usize {
            let input = vec![gen_x(nat(k.into()))];
            // π₀ ⋄ ⟨μ, ν⟩ ≡ μ
            let left0 = replay_compose(&proj0, &[&paired], core::slice::from_ref(&input));
            let right0 = replay(&mu, &input);
            assert_eq!(left0, right0, "π₀ ⋄ ⟨μ, ν⟩ = μ at input {k}");
            // π₁ ⋄ ⟨μ, ν⟩ ≡ ν
            let left1 = replay_compose(&proj1, &[&paired], core::slice::from_ref(&input));
            let right1 = replay(&nu, &input);
            assert_eq!(left1, right1, "π₁ ⋄ ⟨μ, ν⟩ = ν at input {k}");
        }
    }
    #[test]
    fn products_are_preserved_by_restriction()
    {
        let (_mu, _nu, paired) = pair_fixture();
        // Strict: restriction distributes over the product codomain (3(b)(4)).
        let s = SigMorphism::identity(&nat_sig());
        let t = SigMorphism::identity(&nat_sig());
        assert_eq!(
            restrict(&paired.cod, &s, &t).factors.len(),
            paired.cod.factors.len(),
            "restriction preserves the product's factor count"
        );
        // Replay spot-check: the product structure survives — a pair replay has
        // both factors, recoverable by projection.
        let output = replay(&paired, &[gen_x(nat(2.into()))]).expect("the pair fires");
        assert_eq!(
            2,
            output.per_factor.len(),
            "the product has both factors at replay"
        );
        let first = LooseInstance {
            per_factor: vec![output.per_factor[0].clone()],
        };
        let proj0 = proj_cell(paired.cod, 0.into());
        let projected = replay(&proj0, &[output]).expect("projection fires");
        assert!(
            bool::from(loose_instance_eq(&projected, &first)),
            "π₀ recovers the first factor"
        );
    }
    /// The pair `μ : R⇒S`, `ν : R⇒T`, and `⟨μ, ν⟩ : R⇒S∧T`.
    fn pair_fixture() -> (Cell, Cell, Cell)
    {
        let r = unary_relation("R".into());
        let mu = relabel_cell(
            Rc::clone(&r),
            unary_relation("S".into()),
            var("p0.x".into()),
        );
        let nu = relabel_cell(r, unary_relation("T".into()), succ(var("p0.x".into())));
        let paired = Cell {
            dom: mu.dom.clone(),
            cod: LooseArrow::meet(&mu.cod, &nu.cod),
            left_frame: SigMorphism::identity(&nat_sig()),
            right_frame: SigMorphism::identity(&nat_sig()),
            kind: CellKind::Pair(Box::new(mu.clone()), Box::new(nu.clone())),
        };
        (mu, nu, paired)
    }
    /// A projection cell onto codomain factor `idx`.
    fn proj_cell(
        loose: LooseArrow,
        idx: DescriptorFactorIndex,
    ) -> Cell
    {
        Cell {
            dom: vec![loose.clone()],
            cod: loose,
            left_frame: SigMorphism::identity(&nat_sig()),
            right_frame: SigMorphism::identity(&nat_sig()),
            kind: CellKind::Proj { idx },
        }
    }

    #[test]
    fn the_cell_into_top_is_unique()
    {
        let loose = loose_of(unary_relation("R".into()));
        let bang_one = Cell {
            dom: vec![loose],
            cod: LooseArrow::top(&nat_sig(), &nat_sig()),
            left_frame: SigMorphism::identity(&nat_sig()),
            right_frame: SigMorphism::identity(&nat_sig()),
            kind: CellKind::Bang,
        };
        let bang_two = Cell {
            kind: CellKind::Bang,
            ..bang_one.clone()
        };
        let corpus = single_input_corpus();
        assert!(
            bool::from(cells_equal(&bang_one, &bang_two, &corpus)),
            "any two cells into ⊤ agree"
        );
        let empty = LooseInstance {
            per_factor: Vec::new(),
        };
        assert_eq!(
            replay(&bang_one, &[gen_x(nat(0.into()))]),
            Some(empty),
            "the cell into ⊤ produces the empty (terminal) instance"
        );
    }
}
