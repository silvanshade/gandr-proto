//! **Circuit rule instantiation** fixtures — the two-redex block whose two
//! sequentializations are identified where the rule is *applied*
//! (`spec:implementation/circuit-terms.md`,
//! `circuit-terms-question-15` and `circuit-terms-spike-07`).
//!
//! # Why these live here, and on the toy alphabet
//!
//! `tests/shift.rs` realizes the `cong2` block's *pair* by hand: it fabricates
//! two applications at two positions and puts them to the guard. These fixtures
//! close the remaining gap — the applications are no longer fabricated. The
//! rule is a [`gandr_theory_levitation::CircuitRule`] with a real two-redex
//! body, the two positions are read from that body's own occurrence record
//! ([`gandr_theory_levitation::redex_occurrences`]), and the two cells arrive
//! as the instantiation of its rewrite-sorted ports. What is under test is the
//! whole path from a written block to an earned identification.
//!
//! The alphabet is the toy one for the reason `tests/shift.rs` records: a
//! sequent command pattern has exactly one command position, so it cannot carry
//! two applications at incomparable positions, and the shift quotient's
//! extension over it is empty. The toy alphabet nests commands, which is what a
//! circuit-shaped body does. The sequent-side declines of the instantiation
//! path — the shape refusals and the instance check — are pinned in the
//! `instantiate` module's own unit tests.
//!
//! # The fixtures
//!
//! - [`an_instantiated_cong2_rule_earns_its_shift_witness`] is the earning: two
//!   independent redexes, instantiated by two cells that share no seam, whose
//!   two sequentializations reach one composite and replay under both orders.
//! - [`the_instantiated_applications_carry_the_records_positions`] is the
//!   provenance pin: the positions the witness fired at are the occurrence
//!   record's argument paths, not positions the caller chose.
//! - [`a_genuinely_overlapping_instantiation_is_refused_at_the_application_site`]
//!   is the refusal that matters: the same body, instantiated by a cell pair the
//!   enumerator reports overlapping, is refused with the shift constructor's own
//!   obstruction — even though the two orders do reach one term at this instance.
//! - [`a_sequential_two_redex_body_is_refused_comparable_positions`] refuses
//!   the body whose second redex consumes the first: that is `ρ then ρ′`, and
//!   the record's two positions say so.
//! - [`a_reconvergent_body_resolves_both_occurrences_through_one_binding`]
//!   keeps the environment condition off the body: a wire consumed twice is one
//!   rewrite at two positions, so both occurrences resolve through that port's
//!   single binding and the two positions stay distinct.
//!
//! [`an_instantiated_cong2_rule_earns_its_shift_witness`]: tests::an_instantiated_cong2_rule_earns_its_shift_witness
//! [`the_instantiated_applications_carry_the_records_positions`]: tests::the_instantiated_applications_carry_the_records_positions
//! [`a_genuinely_overlapping_instantiation_is_refused_at_the_application_site`]: tests::a_genuinely_overlapping_instantiation_is_refused_at_the_application_site
//! [`a_sequential_two_redex_body_is_refused_comparable_positions`]: tests::a_sequential_two_redex_body_is_refused_comparable_positions
//! [`a_reconvergent_body_resolves_both_occurrences_through_one_binding`]: tests::a_reconvergent_body_resolves_both_occurrences_through_one_binding

#[cfg(test)]
mod tests
{
    use alloc::vec::Vec;

    use gandr_theory_computads::Cell;
    use gandr_theory_computads::CellAlphabet as _;
    use gandr_theory_computads::CellApp;
    use gandr_theory_computads::CellId;
    use gandr_theory_computads::CellStore;
    use gandr_theory_computads::CircuitShiftObstruction;
    use gandr_theory_computads::ConvexityDischarge;
    use gandr_theory_computads::PositionOrder;
    use gandr_theory_computads::PositionStep;
    use gandr_theory_computads::RewriteBinding;
    use gandr_theory_computads::ShiftObstruction;
    use gandr_theory_computads::instantiate_two_redex_rule;
    use gandr_theory_computads::rewrite::rewrite_at;
    use gandr_theory_levitation::CircuitBody;
    use gandr_theory_levitation::CircuitFrame;
    use gandr_theory_levitation::CircuitNode;
    use gandr_theory_levitation::CircuitRedex;
    use gandr_theory_levitation::CircuitRule;
    use gandr_theory_levitation::FrameHead;
    use gandr_theory_levitation::FreeTerm;
    use gandr_theory_levitation::RedexOccurrence;
    use gandr_theory_levitation::RuleFace;
    use gandr_theory_levitation::SurfaceSpan;
    use gandr_theory_levitation::redex_occurrences;

    use crate::toy_alphabet::Toy;
    use crate::toy_alphabet::ToyAlphabet;
    use crate::toy_alphabet::ToyNameRef;
    use crate::toy_alphabet::ToyPos;
    use crate::toy_alphabet::toy_cell;

    extern crate alloc;

    /// A toy position from child indices.
    fn at<Steps>(steps: Steps) -> ToyPos
    where
        Steps: IntoIterator<Item = usize>,
    {
        ToyPos(steps.into_iter().collect::<Vec<_>>().into_boxed_slice())
    }

    /// The position the occurrence record's argument path names, read the way
    /// the instantiation site reads it.
    fn recorded_position(occurrence: &RedexOccurrence) -> ToyPos
    {
        ToyAlphabet::position_at_path(
            &occurrence
                .position
                .iter()
                .copied()
                .map(|index| PositionStep::from(usize::from(index)))
                .collect::<Vec<_>>(),
        )
    }

    /// A face carrying the two terms, with no telescope and a zero span.
    fn face(
        lhs: FreeTerm,
        rhs: FreeTerm,
    ) -> RuleFace
    {
        RuleFace::new(
            lhs,
            rhs,
            Vec::new(),
            SurfaceSpan::new(0_usize.into(), 0_usize.into()),
        )
    }

    /// The `cong2` rule: `*p(-x, +x'); *q(-y, +y'); *add(-x', -y', +z);`, whose
    /// two redexes sit at the frame's two argument positions.
    ///
    /// The declared sphere is the pair the wiring derives, which is what the
    /// surface route supplies; the instantiation site never reads it, because a
    /// peak is exercised rather than matched against a boundary term.
    fn cong2_rule() -> CircuitRule
    {
        let body = CircuitBody::new(
            [
                CircuitNode::Redex(CircuitRedex::new(
                    "p",
                    FreeTerm::var("x"),
                    FreeTerm::var("x\u{2032}"),
                    "x\u{2032}",
                )),
                CircuitNode::Redex(CircuitRedex::new(
                    "q",
                    FreeTerm::var("y"),
                    FreeTerm::var("y\u{2032}"),
                    "y\u{2032}",
                )),
                CircuitNode::Frame(CircuitFrame::new(
                    FrameHead::Op("add".into()),
                    [FreeTerm::var("x\u{2032}"), FreeTerm::var("y\u{2032}")],
                    "z",
                )),
            ],
            "z",
        );
        let derived =
            gandr_theory_levitation::derive_boundaries(&body).expect("the cong2 wiring derives");
        CircuitRule::new("cong2", face(derived.source, derived.target), body)
    }

    /// (f): `Succ(Zero) ~> Zero` — the rule the left redex node is instantiated
    /// by.
    fn f_cell() -> Cell<ToyAlphabet>
    {
        toy_cell(Toy::succ(Toy::Zero), Toy::Zero)
    }

    /// (g): `Succ(Succ(Zero)) ~> Zero` — the rule the right redex node is
    /// instantiated by.
    ///
    /// Both faces are ground and neither right-hand side offers a seam the
    /// other left-hand side unifies with, which is the surface reading "f
    /// and g share no ports".
    fn g_cell() -> Cell<ToyAlphabet>
    {
        toy_cell(Toy::succ(Toy::succ(Toy::Zero)), Toy::Zero)
    }

    /// The store the `cong2` ports are instantiated from, and its two ids.
    fn cong2_store() -> (CellStore<ToyAlphabet>, CellId, CellId)
    {
        let mut store = CellStore::new();
        let f = store.insert(f_cell());
        let g = store.insert(g_cell());
        (store, f, g)
    }

    /// The peak the instantiated rule is applied to: `add(Succ(Zero),
    /// Succ(Succ(Zero)))`, an instance of the block's derived source boundary
    /// with an `f`-redex in the left argument and a `g`-redex in the right.
    fn cong2_peak() -> Toy
    {
        Toy::add(Toy::succ(Toy::Zero), Toy::succ(Toy::succ(Toy::Zero)))
    }

    /// (add-Z): `Add(Zero, x) ~> x`.
    fn add_z() -> Cell<ToyAlphabet>
    {
        toy_cell(
            Toy::add(Toy::Zero, Toy::var(ToyNameRef("x"))),
            Toy::var(ToyNameRef("x")),
        )
    }

    /// (add-S): `Add(Succ(m), n) ~> Succ(Add(m, n))`.
    fn add_s() -> Cell<ToyAlphabet>
    {
        toy_cell(
            Toy::add(
                Toy::succ(Toy::var(ToyNameRef("m"))),
                Toy::var(ToyNameRef("n")),
            ),
            Toy::succ(Toy::add(
                Toy::var(ToyNameRef("m")),
                Toy::var(ToyNameRef("n")),
            )),
        )
    }

    /// Run a whole recorded schedule from `start`, failing the test at the step
    /// that does not fire.
    fn run(
        store: &CellStore<ToyAlphabet>,
        start: &Toy,
        schedule: &[CellApp<ToyAlphabet>],
    ) -> Toy
    {
        let mut current = start.clone();
        for step in schedule {
            let cell = store.get(step.cell).expect("the step names a stored cell");
            current =
                rewrite_at(cell, &current, &step.at).expect("the step fires where it was recorded");
        }
        current
    }

    #[test]
    fn an_instantiated_cong2_rule_earns_its_shift_witness()
    {
        let (store, f, g) = cong2_store();
        let shift = instantiate_two_redex_rule(
            &store,
            &cong2_rule(),
            &[RewriteBinding::new("p", f), RewriteBinding::new("q", g)],
            &cong2_peak(),
        )
        .expect("two independent redexes, instantiated by two cells sharing no seam");
        assert_eq!(
            cong2_peak(),
            shift.witness.peak,
            "the witness records the term the rule was applied to"
        );
        assert_eq!(
            Toy::add(Toy::Zero, Toy::Zero),
            shift.witness.joins_at,
            "both sequentializations reach one composite"
        );
        assert_eq!(
            ConvexityDischarge::LeftConnectedOverAcyclicTarget,
            shift.witness.convexity,
            "and the witness carries the warrant its convexity conjunct was skipped under"
        );
        assert!(
            bool::from(shift.replay),
            "the identification is confirmed by replay, not granted by the guard alone"
        );
        // The identification is not a claim about the recorded order: running
        // either sequentialization by hand lands on the same composite.
        assert_eq!(
            shift.witness.joins_at,
            run(&store, &cong2_peak(), &shift.witness.first_then_second()),
            "p then q reaches the composite"
        );
        assert_eq!(
            shift.witness.joins_at,
            run(&store, &cong2_peak(), &shift.witness.second_then_first()),
            "and so does q then p"
        );
    }

    #[test]
    fn the_instantiated_applications_carry_the_records_positions()
    {
        // The provenance pin: nothing in the call names a position, so the two
        // the witness fired at can only have come from the block's own
        // occurrence record.
        let (store, f, g) = cong2_store();
        let rule = cong2_rule();
        let occurrences = redex_occurrences(&rule.body).expect("the cong2 wiring unfolds");
        let shift = instantiate_two_redex_rule(
            &store,
            &rule,
            &[RewriteBinding::new("p", f), RewriteBinding::new("q", g)],
            &cong2_peak(),
        )
        .expect("the instantiated pair earns its witness");
        let [ref left, ref right] = *occurrences
        else {
            panic!("the cong2 body unfolds to exactly two occurrences");
        };
        assert_eq!(
            recorded_position(left),
            shift.witness.first.at,
            "the first application fired at the record's first occurrence"
        );
        assert_eq!(
            recorded_position(right),
            shift.witness.second.at,
            "and the second at the record's second, in the source reading's order"
        );
        assert_eq!(
            (at([0]), at([1])),
            (
                shift.witness.first.at.clone(),
                shift.witness.second.at.clone()
            ),
            "which are the frame's two argument slots"
        );
        assert_eq!(
            (f, g),
            (shift.witness.first.cell, shift.witness.second.cell),
            "and each carries the cell its rewrite port was instantiated by"
        );
    }

    #[test]
    fn a_genuinely_overlapping_instantiation_is_refused_at_the_application_site()
    {
        // The same two-redex body, so the record's two positions are the same
        // incomparable pair; only the instantiation changed. The two orders do
        // reach one term at this peak, and the pair is refused all the same,
        // because the enumerator reports the instantiating cells overlapping.
        // The refusal is the shift constructor's own obstruction: this site
        // holds no overlap oracle of its own.
        let mut store = CellStore::new();
        let z = store.insert(add_z());
        let s = store.insert(add_s());
        let peak = Toy::add(
            Toy::add(Toy::Zero, Toy::succ(Toy::Zero)),
            Toy::add(Toy::succ(Toy::Zero), Toy::Zero),
        );
        let first = CellApp {
            cell: z,
            at: at([0]),
        };
        let second = CellApp {
            cell: s,
            at: at([1]),
        };
        assert_eq!(
            run(&store, &peak, &alloc::vec![first.clone(), second.clone()]),
            run(&store, &peak, &alloc::vec![second, first]),
            "the two orders do reach one term at this instance"
        );
        let refusal = instantiate_two_redex_rule(
            &store,
            &cong2_rule(),
            &[RewriteBinding::new("p", z), RewriteBinding::new("q", s)],
            &peak,
        )
        .expect_err("an overlapping instantiation is refused at the application site");
        let CircuitShiftObstruction::Refused(obstruction) = refusal
        else {
            panic!("the refusal is the guard's, carried verbatim: {refusal:?}");
        };
        let ShiftObstruction::GenuineOverlap { overlap } = *obstruction
        else {
            panic!("the overlap conjunct is what refuses this instantiation: {obstruction:?}");
        };
        assert_eq!(
            (z, s),
            (overlap.left, overlap.right),
            "and it carries an overlap of exactly the two instantiating cells"
        );
    }

    #[test]
    fn a_reconvergent_body_resolves_both_occurrences_through_one_binding()
    {
        // `*p(-x, +w); *add(-w, -w, +z);` — one redex output feeding both
        // arguments of one frame. The wire is unfolded at each consumption, so
        // the record holds the SAME rewrite at two positions. That is a repeated
        // port in the *body*, which is ordinary reconvergence, and not a
        // repeated *binding*: `p` is bound once and both occurrences resolve
        // through that single binding, at two positions that stay distinct.
        let body = CircuitBody::new(
            [
                CircuitNode::Redex(CircuitRedex::new(
                    "p",
                    FreeTerm::var("x"),
                    FreeTerm::ctor("Succ", [FreeTerm::var("x")]),
                    "w",
                )),
                CircuitNode::Frame(CircuitFrame::new(
                    FrameHead::Op("add".into()),
                    [FreeTerm::var("w"), FreeTerm::var("w")],
                    "z",
                )),
            ],
            "z",
        );
        let rule = CircuitRule::new("dup", face(FreeTerm::var("z"), FreeTerm::var("z")), body);
        let occurrences = redex_occurrences(&rule.body).expect("the reconvergent wiring unfolds");
        let [ref left, ref right] = *occurrences
        else {
            panic!("a wire consumed twice is two occurrences");
        };
        assert_eq!(
            (left.rewrite.clone(), right.rewrite.clone()),
            ("p".into(), "p".into()),
            "both occurrences are of the one rewrite the body applies"
        );
        // Only `f` is stored, and `p` is bound to it exactly once.
        let mut store = CellStore::new();
        let f = store.insert(f_cell());
        let peak = Toy::add(Toy::succ(Toy::Zero), Toy::succ(Toy::Zero));
        let shift =
            instantiate_two_redex_rule(&store, &rule, &[RewriteBinding::new("p", f)], &peak)
                .expect("one binding serves both occurrences of its port");
        assert_eq!(
            (f, f),
            (shift.witness.first.cell, shift.witness.second.cell),
            "both applications carry the single cell the port is bound to"
        );
        assert_eq!(
            (recorded_position(left), recorded_position(right)),
            (
                shift.witness.first.at.clone(),
                shift.witness.second.at.clone()
            ),
            "at the record's own two positions"
        );
        assert_ne!(
            shift.witness.first.at, shift.witness.second.at,
            "which stay distinct: reconvergence is one rewrite at two places, not one place twice"
        );
        assert_eq!(
            Toy::add(Toy::Zero, Toy::Zero),
            shift.witness.joins_at,
            "and the two orders reach one composite"
        );
        assert!(
            bool::from(shift.replay),
            "confirmed by replay like any other earned identification"
        );
    }

    #[test]
    fn a_sequential_two_redex_body_is_refused_comparable_positions()
    {
        // `*p(-x, +x'); *q(-x', +y'); *add(-y', -w, +z);` — the second redex
        // consumes the first, so both occurrences unfold at the frame's first
        // argument and the record's two positions are the same position. That
        // is sequential composition, which the boundary language spells
        // `ρ then ρ′` and which this site refuses with the position conjunct
        // rather than identifying.
        let body = CircuitBody::new(
            [
                CircuitNode::Redex(CircuitRedex::new(
                    "p",
                    FreeTerm::var("x"),
                    FreeTerm::var("x\u{2032}"),
                    "x\u{2032}",
                )),
                CircuitNode::Redex(CircuitRedex::new(
                    "q",
                    FreeTerm::var("x\u{2032}"),
                    FreeTerm::var("y\u{2032}"),
                    "y\u{2032}",
                )),
                CircuitNode::Frame(CircuitFrame::new(
                    FrameHead::Op("add".into()),
                    [FreeTerm::var("y\u{2032}"), FreeTerm::var("w")],
                    "z",
                )),
            ],
            "z",
        );
        let rule = CircuitRule::new("seq2", face(FreeTerm::var("z"), FreeTerm::var("z")), body);
        let (store, f, g) = cong2_store();
        let refusal = instantiate_two_redex_rule(
            &store,
            &rule,
            &[RewriteBinding::new("p", f), RewriteBinding::new("q", g)],
            &cong2_peak(),
        )
        .expect_err("one redex feeding the other is not an adjacent pair");
        assert_eq!(
            CircuitShiftObstruction::<ToyAlphabet>::Refused(alloc::boxed::Box::new(
                ShiftObstruction::ComparablePositions {
                    order: PositionOrder::Same,
                }
            )),
            refusal,
            "the record's two positions coincide, and the position conjunct says so"
        );
    }
}
