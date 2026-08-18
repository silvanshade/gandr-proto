//! The **reduction order** measured against real rule corpora
//! (`gandr-0rb` residual (c)).
//!
//! The residual it discharges said the size-only order left equal-size critical
//! pairs as obstructions, and that a path order would orient more — "unmeasured
//! until real rule corpora exist". These fixtures are that measurement, taken
//! over cells the description route produces from written `data` rules rather
//! than over probes shaped to give an answer.
//!
//! - [`the_path_order_orients_pairs_the_size_order_left_as_obstructions`]
//!   counts the equal-size face pairs a real corpus contains and how many of
//!   them the order now orients.
//! - [`completion_derives_the_fusion_cell_the_size_order_could_not_orient`] is
//!   the end of the same measurement at the engine: the design's worked
//!   deforestation cell is an equal-size divergence, and completion now emits
//!   it with a replayable certificate.

#[cfg(test)]
mod tests
{
    use core::cmp::Ordering;

    use gandr_core_sequent::il::Polarity;
    use gandr_theory_computads::Cell;
    use gandr_theory_computads::CellProvenance;
    use gandr_theory_computads::CellStore;
    use gandr_theory_computads::CmdPat;
    use gandr_theory_computads::CompletionBudget;
    use gandr_theory_computads::CompletionOutcome;
    use gandr_theory_computads::ConsPat;
    use gandr_theory_computads::Orientation;
    use gandr_theory_computads::ProdPat;
    use gandr_theory_computads::complete;
    use gandr_theory_computads::reduction_cmp;

    extern crate alloc;

    #[test]
    fn the_path_order_orients_pairs_the_size_order_left_as_obstructions()
    {
        // The measurement. Every face of every cell a real description
        // elaborates, paired with every other: the pairs whose node counts
        // agree are exactly the ones the size-only order reported `Equal`, and
        // the count below says how many of those the path order now decides.
        let store = description_route_store();
        let faces = store_faces(&store);
        assert!(
            faces.len() >= 8,
            "the corpus carries several real rule faces, not one probe"
        );
        let mut equal_size = 0_usize;
        let mut oriented = 0_usize;
        for left in &faces {
            for right in &faces {
                if gandr_theory_computads::pattern::cmd_size(left)
                    != gandr_theory_computads::pattern::cmd_size(right)
                {
                    continue;
                }
                if left == right {
                    continue;
                }
                equal_size = equal_size.saturating_add(1);
                if reduction_cmp(left, right) != Ordering::Equal {
                    oriented = oriented.saturating_add(1);
                }
            }
        }
        assert!(
            equal_size > 0,
            "the hypothesis: a real corpus does contain equal-size pairs, which is why the \
             residual existed"
        );
        assert_eq!(
            OrientedCount::from(14_usize),
            OrientedCount::from(oriented),
            "measured and pinned: of {equal_size} equal-size ordered face pairs the size order \
             left as obstructions, the path order orients this many"
        );
        // Orientation stays antisymmetric across the whole corpus, which is
        // what makes an orientation a rule rather than a coin toss.
        for left in &faces {
            for right in &faces {
                match reduction_cmp(left, right) {
                    | Ordering::Greater => assert_eq!(
                        Ordering::Less,
                        reduction_cmp(right, left),
                        "an oriented pair is oriented one way"
                    ),
                    | Ordering::Less => assert_eq!(
                        Ordering::Greater,
                        reduction_cmp(right, left),
                        "an oriented pair is oriented one way"
                    ),
                    | Ordering::Equal => assert_eq!(
                        Ordering::Equal,
                        reduction_cmp(right, left),
                        "an obstruction is an obstruction from either side"
                    ),
                }
            }
        }
    }

    /// A count of newly oriented equal-size pairs — the measurement's pinned
    /// figure.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct OrientedCount(usize);

    impl From<usize> for OrientedCount
    {
        fn from(value: usize) -> Self
        {
            Self(value)
        }
    }

    #[test]
    fn completion_derives_the_fusion_cell_the_size_order_could_not_orient()
    {
        // The measurement carried to the engine. `⟨v | Succ⁻(add(n; α))⟩` and
        // `⟨v | add(n; Succ⁻(α))⟩` are the two faces of the design's worked
        // deforestation cell, and they have the same node count: fusing moves a
        // frame rather than removing one. A store whose two cells reduce one
        // command to those two faces is an equal-size divergence, and the
        // size-only order reported it unorientable and left it.
        let mut store = CellStore::new();
        store.insert(diverging_cell(fusion_before()));
        store.insert(diverging_cell(fusion_after()));
        let outcome = complete(
            store,
            CompletionBudget::new(64_usize.into(), 16_usize.into(), 64_usize.into()),
        );
        let CompletionOutcome::Completed {
            ref store,
            ref derived,
            ref certificates,
        } = outcome
        else {
            panic!("the two-cell system completes within budget")
        };
        assert!(
            !derived.is_empty(),
            "completion derives a cell from the equal-size divergence, which it could not orient \
             before"
        );
        let derived_cell = derived
            .first()
            .and_then(|&id| store.get(id))
            .expect("the derived cell is in the returned store");
        // Overlap enumeration renames the right cell's holes apart, so the
        // derived faces carry the unifier's names rather than the fixture's.
        // What is asserted is therefore the shape and the direction, which is
        // what the orientation is about.
        assert!(
            frame_wrapping_operation(&derived_cell.lhs).0,
            "the derived cell's left-hand side is the unfused face: a constructor frame outside \
             the operation frame"
        );
        assert!(
            operation_wrapping_frame(&derived_cell.rhs).0,
            "and its right-hand side the fused face: the constructor frame pushed inside — \
             deforestation, in the direction the design writes it"
        );
        assert_eq!(
            Ordering::Greater,
            reduction_cmp(&derived_cell.lhs, &derived_cell.rhs),
            "and the orientation is the order's, not a coin toss"
        );
        for certificate in certificates {
            assert!(
                bool::from(certificate.replay(store)),
                "every certificate completion emits replays"
            );
        }
    }

    /// The design's worked fusion cell, left face.
    fn fusion_before() -> CmdPat
    {
        CmdPat::cut(
            Polarity::Positive,
            ProdPat::meta("v"),
            ConsPat::frame(
                "Succ",
                ConsPat::op("add", [ProdPat::meta("n")], ConsPat::meta("a")),
            ),
        )
    }

    /// The design's worked fusion cell, right face.
    fn fusion_after() -> CmdPat
    {
        CmdPat::cut(
            Polarity::Positive,
            ProdPat::meta("v"),
            ConsPat::op(
                "add",
                [ProdPat::meta("n")],
                ConsPat::frame("Succ", ConsPat::meta("a")),
            ),
        )
    }

    /// Whether a command's consumer is a constructor frame directly wrapping an
    /// operation frame — the unfused face's shape.
    fn frame_wrapping_operation(cmd: &CmdPat) -> FaceShapeMatch
    {
        let CmdPat::Cut { ref cons, .. } = *cmd;
        FaceShapeMatch(matches!(
            *cons,
            ConsPat::Frame { ref ret, .. } if matches!(**ret, ConsPat::Op { .. })
        ))
    }

    /// Whether a command's consumer is an operation frame whose return
    /// continuation is a constructor frame — the fused face's shape.
    fn operation_wrapping_frame(cmd: &CmdPat) -> FaceShapeMatch
    {
        let CmdPat::Cut { ref cons, .. } = *cmd;
        FaceShapeMatch(matches!(
            *cons,
            ConsPat::Op { ref ret, .. } if matches!(**ret, ConsPat::Frame { .. })
        ))
    }

    /// Whether a command's consumer has the fixture's expected shape.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct FaceShapeMatch(bool);

    /// A cell reducing one shared redex to `face`, so two of them present
    /// completion with a divergent critical pair at the root.
    fn diverging_cell(face: CmdPat) -> Cell
    {
        Cell::new(
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::meta("v"),
                ConsPat::op("seed", [ProdPat::meta("n")], ConsPat::meta("a")),
            ),
            face,
            Orientation::PolarityDerived,
            CellProvenance::SurfaceRule,
        )
    }

    /// Both faces of every cell in a store, in insertion order.
    fn store_faces(store: &CellStore) -> alloc::vec::Vec<CmdPat>
    {
        let mut faces = alloc::vec::Vec::new();
        for (_, cell) in store.iter() {
            faces.push(cell.lhs.clone());
            faces.push(cell.rhs.clone());
        }
        faces
    }

    /// The cell store a real `Nat` description elaborates: two constructors,
    /// `add` and `double`, and their four written faces.
    fn description_route_store() -> CellStore
    {
        use gandr_theory_levitation::Attrs;
        use gandr_theory_levitation::BridgeArity;
        use gandr_theory_levitation::Code;
        use gandr_theory_levitation::CtorDesc;
        use gandr_theory_levitation::DeclPolarity;
        use gandr_theory_levitation::FreeTerm;
        use gandr_theory_levitation::NominalId;
        use gandr_theory_levitation::OperDesc;
        use gandr_theory_levitation::RuleFace;
        use gandr_theory_levitation::SignDesc;
        use gandr_theory_levitation::SortRef;
        use gandr_theory_levitation::SurfaceSpan;

        let face = |lhs: FreeTerm, rhs: FreeTerm| {
            RuleFace::new(
                lhs,
                rhs,
                alloc::vec::Vec::new(),
                SurfaceSpan::new(0_usize.into(), 0_usize.into()),
            )
        };
        let desc = SignDesc::new(
            NominalId::new(0_u64.into(), "Nat"),
            alloc::vec::Vec::new(),
            [
                CtorDesc::new("Zero", Code::Unit, "Nat", Attrs::empty()),
                CtorDesc::new("Succ", Code::var("Nat"), "Nat", Attrs::empty()),
            ],
            [
                OperDesc::new(
                    "add",
                    BridgeArity::single_output(
                        [SortRef::new("m", "Nat"), SortRef::new("n", "Nat")],
                        SortRef::new("out", "Nat"),
                    ),
                    Attrs::empty(),
                ),
                OperDesc::new(
                    "double",
                    BridgeArity::single_output(
                        [SortRef::new("m", "Nat")],
                        SortRef::new("out", "Nat"),
                    ),
                    Attrs::empty(),
                ),
            ],
            [
                face(
                    FreeTerm::op("add", [FreeTerm::ctor("Zero", []), FreeTerm::var("n")]),
                    FreeTerm::var("n"),
                ),
                face(
                    FreeTerm::op("add", [
                        FreeTerm::ctor("Succ", [FreeTerm::var("m")]),
                        FreeTerm::var("n"),
                    ]),
                    FreeTerm::ctor("Succ", [FreeTerm::op("add", [
                        FreeTerm::var("m"),
                        FreeTerm::var("n"),
                    ])]),
                ),
                face(
                    FreeTerm::op("double", [FreeTerm::ctor("Zero", [])]),
                    FreeTerm::ctor("Zero", []),
                ),
                face(
                    FreeTerm::op("double", [FreeTerm::ctor("Succ", [FreeTerm::var("m")])]),
                    FreeTerm::ctor("Succ", [FreeTerm::ctor("Succ", [FreeTerm::op(
                        "double",
                        [FreeTerm::var("m")],
                    )])]),
                ),
            ],
            DeclPolarity::Data,
            Attrs::empty(),
        );
        let elaborated = gandr_theory_computads::elaborate_data_desc(&desc);
        assert!(
            elaborated.declined_faces.is_empty() && elaborated.declined_opers.is_empty(),
            "the corpus description elaborates whole, so the measurement runs over real rules"
        );
        elaborated.store
    }
}
