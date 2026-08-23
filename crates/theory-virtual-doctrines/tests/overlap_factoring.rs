//! **The overlap-factoring experiment** — does the acyclicity verdict on a
//! composite factor through pairwise overlap data?
//!
//! The engine offers no n-ary composition: `directed_cut` is binary, so the
//! composite of a family of certificates is the **left fold** of the gate over
//! its members. That fact is what the experiment turns on. `graft` concatenates
//! both legs, so the accumulator's recorded cell support is the **union** of
//! every member folded so far, while its recorded join is the **last** member's
//! alone. The gate's input at fold step `k` is therefore
//! `(union support of m₀…mₖ) × (seam holes of mₖ's join) × (support of mₖ₊₁)`,
//! and neither factor is a datum of any pair.
//!
//! # The three readings, and what the sweep measures
//!
//! A "pairwise verdict" is not one question, because a pair does not determine
//! which seam term the gate reads. Three readings are compared against the fold
//! over the same generated families:
//!
//! - **adjacent** — the conjunction over consecutive pairs `(mₖ, mₖ₊₁)`. This
//!   is the neighbour-local reading, the one an incremental composition scheme
//!   would want: adding a certificate to a valid composite touches only its
//!   neighbour.
//! - **own-join** — the conjunction over every ordered pair `(mᵢ, mⱼ)`, `i <
//!   j`, each asked at `mᵢ`'s own recorded join. This is the cover reading:
//!   every part checked against every other.
//! - **fold-seam** — the same ordered pairs, but each asked at the join the
//!   fold would actually present when `mⱼ` is grafted, namely `mⱼ₋₁`'s. This is
//!   the maximally generous reading: pairwise cell supports, plus the one datum
//!   the fold has and a pair does not.
//!
//! # Why the pairwise verdicts are all of one class
//!
//! A non-adjacent pair is not sequentially seated, so a composite of it would
//! not replay — but the gate has no verdict for that. `compose_directed`
//! declines for exactly one reason, a variable-flow cycle, and the sequential
//! seam is a precondition on **replay**, never on admissibility. So every
//! consultation in every reading returns a verdict of the same class, and the
//! conjunctions compare like with like. `acyclicity_class` proves it
//! per decline rather than trusting the reading: every declined consultation
//! must carry a non-empty cycle whose every node names a cell one of the two
//! operands fires and a hole that cell classifies `Mixed`. A decline outside
//! that class is counted separately and never folded into a conjunction.
//!
//! # The families
//!
//! Each family is a sequentially seated chain of one-step certificates over one
//! store. Member `k` fires a single cell whose left-hand side is headed by the
//! chain operation `opₖ` and whose right-hand side is headed by `opₖ₊₁`, so
//! exactly one cell in the store matches at each peak and each member records
//! exactly one cell. Three member kinds vary the seam structure:
//!
//! - `Mixed` wears the chain's producer hole `s` at both polarities, which is
//!   the dinaturality shape the gate reads as `CellVariance::Mixed`;
//! - `Split` wears two fresh single-polarity holes and no `s` at all;
//! - `Drop` also wears no `s`, and additionally rewrites the producer position
//!   to a ground constructor — so from that member onward the chain's recorded
//!   joins no longer carry `s`, and the gate stops reading that hole.
//!
//! The sweep is exhaustive over all `3ⁿ` shapes at `n = 3` and `n = 4`.

#[cfg(test)]
mod tests
{
    use gandr_core_sequent::il::Polarity;
    use gandr_theory_cell_complexes::Cell;
    use gandr_theory_cell_complexes::CellId;
    use gandr_theory_cell_complexes::CellProvenance;
    use gandr_theory_cell_complexes::CellStore;
    use gandr_theory_cell_complexes::CellVariance;
    use gandr_theory_cell_complexes::CmdPat;
    use gandr_theory_cell_complexes::ConsPat;
    use gandr_theory_cell_complexes::Orientation;
    use gandr_theory_cell_complexes::ProdPat;
    use gandr_theory_coherent_resolutions::Overlap;
    use gandr_theory_coherent_resolutions::OverlapKind;
    use gandr_theory_coherent_resolutions::Tracelet;
    use gandr_theory_coherent_resolutions::enumerate_overlaps;
    use gandr_theory_decomposition_spaces::CompositionObstruction;
    use gandr_theory_virtual_doctrines::CutOutcome;
    use gandr_theory_virtual_doctrines::directed_cut;

    // ---- fixture vocabulary ------------------------------------------------

    /// A metavariable-hole name used by the fixture builders.
    #[repr(transparent)]
    #[derive(Clone, Copy)]
    struct FixtureHoleName<'fixture>(&'fixture str);

    /// An operation/frame name used by the fixture builders.
    #[repr(transparent)]
    #[derive(Clone, Copy)]
    struct OperationName<'fixture>(&'fixture str);

    /// A member's position in a generated chain, zero-based.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct ChainIndex(usize);

    /// The number of members a generated family carries.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct FamilyLength(usize);

    /// Whether the chain's producer seam hole is still readable at a position.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct SeamHoleVisibility(bool);

    impl From<bool> for SeamHoleVisibility
    {
        #[inline]
        fn from(value: bool) -> Self
        {
            Self(value)
        }
    }

    impl From<SeamHoleVisibility> for bool
    {
        #[inline]
        fn from(value: SeamHoleVisibility) -> Self
        {
            value.0
        }
    }

    /// Whether a decline belongs to the acyclicity class the readings conjoin.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct DeclineIsAcyclicity(bool);

    /// A pinned count from the sweep.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct SweepCount(usize);

    impl From<usize> for SweepCount
    {
        #[inline]
        fn from(value: usize) -> Self
        {
            Self(value)
        }
    }

    /// The seam structure one member of a generated family contributes.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum MemberKind
    {
        /// Wears the chain's producer hole `s` at both polarities — the
        /// dinaturality shape, classified `CellVariance::Mixed`.
        Mixed,
        /// Wears two fresh single-polarity holes and no `s`.
        Split,
        /// Wears two fresh single-polarity holes and rewrites the producer
        /// position to a ground constructor, so every later recorded join in
        /// the chain loses the hole `s`.
        Drop,
    }

    /// A verdict of the gate over a whole family, under one reading.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum FactoringVerdict
    {
        /// Every consultation the reading made was admitted.
        Admits,
        /// At least one consultation was declined.
        Refuses,
    }

    /// What one gate consultation returned.
    enum Consultation
    {
        /// The gate admitted, yielding the composite certificate.
        Admitted(Box<Tracelet>),
        /// The gate declined.
        Declined,
    }

    // ---- the ledger --------------------------------------------------------

    /// What a run of the gate over one family actually did.
    ///
    /// The counts are assertions rather than telemetry: a factoring comparison
    /// whose consultations never reach the gate is green and says nothing.
    #[derive(Default)]
    struct GateLedger
    {
        /// Every `directed_cut` call the run made.
        consultations: usize,
        /// Calls that consulted the acyclicity gate.
        gated: usize,
        /// Calls that took the invertible bypass instead of the gate.
        coherent: usize,
        /// Calls the gate declined.
        declines: usize,
        /// Declines carrying an obstruction of the acyclicity class.
        acyclicity_declines: usize,
    }

    impl GateLedger
    {
        /// Fold another run's counts into this one.
        ///
        /// # Contract
        /// - ensures: each field is the saturating sum of the two runs'.
        /// - panics: none.
        fn absorb(
            &mut self,
            other: &Self,
        )
        {
            self.consultations = self.consultations.saturating_add(other.consultations);
            self.gated = self.gated.saturating_add(other.gated);
            self.coherent = self.coherent.saturating_add(other.coherent);
            self.declines = self.declines.saturating_add(other.declines);
            self.acyclicity_declines = self
                .acyclicity_declines
                .saturating_add(other.acyclicity_declines);
        }
    }

    /// Consult the gate on one ordered pair, recording what it did.
    ///
    /// # Contract
    /// - ensures: the ledger gains one consultation, classified into the
    ///   invertible-bypass lane or the gated lane, and a decline is classified
    ///   against the acyclicity class before it is counted as one.
    /// - panics: none.
    fn consult(
        ledger: &mut GateLedger,
        left: &Tracelet,
        right: &Tracelet,
        store: &CellStore,
    ) -> Consultation
    {
        ledger.consultations = ledger.consultations.saturating_add(1);
        match directed_cut(left, right, store) {
            | CutOutcome::Coherent(composite) => {
                ledger.coherent = ledger.coherent.saturating_add(1);
                Consultation::Admitted(Box::new(composite))
            },
            | CutOutcome::Directed(composite) => {
                ledger.gated = ledger.gated.saturating_add(1);
                Consultation::Admitted(Box::new(composite))
            },
            | CutOutcome::Declined(obstruction) => {
                ledger.gated = ledger.gated.saturating_add(1);
                ledger.declines = ledger.declines.saturating_add(1);
                if acyclicity_class(&obstruction, left, right, store).0 {
                    ledger.acyclicity_declines = ledger.acyclicity_declines.saturating_add(1);
                }
                Consultation::Declined
            },
        }
    }

    /// Whether a decline's obstruction is a variable-flow cycle over the two
    /// operands' own mixed-variance seam holes — the only decline class the
    /// conjunctions may contain.
    ///
    /// # Contract
    /// - ensures: positive iff the cycle is non-empty and every node names a
    ///   cell one of the operands fires together with a hole that cell's live
    ///   metadata classifies `CellVariance::Mixed`.
    /// - panics: none.
    fn acyclicity_class(
        obstruction: &CompositionObstruction,
        left: &Tracelet,
        right: &Tracelet,
        store: &CellStore,
    ) -> DeclineIsAcyclicity
    {
        let mut fired = participating(left);
        fired.extend(participating(right));
        DeclineIsAcyclicity(
            !obstruction.cycle.is_empty()
                && obstruction.cycle.iter().all(|node| {
                    let cell = node.0;
                    let hole = &node.1;
                    fired.contains(&cell)
                        && store.get(cell).is_some_and(|entry| {
                            entry.meta.vars.iter().any(|var| {
                                var.var.name == hole.name && var.variance == CellVariance::Mixed
                            })
                        })
                }),
        )
    }

    // ---- the four readings -------------------------------------------------

    /// The composite verdict — the left fold of the gate over the family.
    ///
    /// Every admitted intermediate graft is replayed, so a refusal is always
    /// the gate refusing and never a malformed chain.
    ///
    /// # Contract
    /// - ensures: `Refuses` at the first declined graft, with the ledger
    ///   carrying exactly the consultations made up to and including it;
    ///   `Admits` after `members.len() - 1` admitted grafts.
    /// - panics: none in the contract sense; a non-replaying intermediate
    ///   composite fails the fixture's assertion, which is its purpose.
    fn composite_verdict(
        members: &[Tracelet],
        store: &CellStore,
        ledger: &mut GateLedger,
    ) -> FactoringVerdict
    {
        let Some((first, rest)) = members.split_first()
        else {
            return FactoringVerdict::Admits;
        };
        let mut accumulator = first.clone();
        for member in rest {
            match consult(ledger, &accumulator, member, store) {
                | Consultation::Declined => return FactoringVerdict::Refuses,
                | Consultation::Admitted(composite) => {
                    assert!(
                        bool::from(composite.replay(store)),
                        "an admitted intermediate graft is a real certificate — it replays, so a \
                         later refusal is the gate refusing and not a malformed chain"
                    );
                    accumulator = *composite;
                },
            }
        }
        FactoringVerdict::Admits
    }

    /// The **adjacent** conjunction — consecutive pairs, each at the left
    /// member's own recorded join.
    ///
    /// # Contract
    /// - ensures: every consecutive pair is consulted (no short-circuit, so the
    ///   consultation count is a function of the family's length alone);
    ///   `Refuses` iff any of them declined.
    /// - panics: none.
    fn adjacent_verdict(
        members: &[Tracelet],
        store: &CellStore,
        ledger: &mut GateLedger,
    ) -> FactoringVerdict
    {
        let mut verdict = FactoringVerdict::Admits;
        for window in members.windows(2) {
            let (Some(left), Some(right)) = (window.first(), window.get(1))
            else {
                continue;
            };
            if matches!(consult(ledger, left, right, store), Consultation::Declined) {
                verdict = FactoringVerdict::Refuses;
            }
        }
        verdict
    }

    /// The **own-join** conjunction — every ordered pair, each asked at the
    /// left member's own recorded join.
    ///
    /// # Contract
    /// - ensures: every ordered pair `(i, j)` with `i < j` is consulted;
    ///   `Refuses` iff any of them declined.
    /// - panics: none.
    fn own_join_verdict(
        members: &[Tracelet],
        store: &CellStore,
        ledger: &mut GateLedger,
    ) -> FactoringVerdict
    {
        let mut verdict = FactoringVerdict::Admits;
        for (index, left) in members.iter().enumerate() {
            for right in members.iter().skip(index.saturating_add(1)) {
                if matches!(consult(ledger, left, right, store), Consultation::Declined) {
                    verdict = FactoringVerdict::Refuses;
                }
            }
        }
        verdict
    }

    /// The **fold-seam** conjunction — every ordered pair, each asked at the
    /// join the fold would present when the right member is grafted.
    ///
    /// The left member is re-seated on `mⱼ₋₁`'s join before the consultation:
    /// same recorded cells, the seam term the fold has. This is the reading
    /// that hands a pairwise checker the one datum a pair does not determine.
    ///
    /// # Contract
    /// - ensures: every ordered pair `(i, j)` with `i < j` is consulted with
    ///   the left operand re-seated on member `j - 1`'s join; `Refuses` iff any
    ///   of them declined.
    /// - panics: none.
    fn fold_seam_verdict(
        members: &[Tracelet],
        store: &CellStore,
        ledger: &mut GateLedger,
    ) -> FactoringVerdict
    {
        let mut verdict = FactoringVerdict::Admits;
        for (index, left) in members.iter().enumerate() {
            for (offset, right) in members.iter().enumerate().skip(index.saturating_add(1)) {
                let Some(seam) = members.get(offset.saturating_sub(1))
                else {
                    continue;
                };
                let reseated = Tracelet {
                    joins_at: seam.joins_at.clone(),
                    ..left.clone()
                };
                if matches!(
                    consult(ledger, &reseated, right, store),
                    Consultation::Declined
                ) {
                    verdict = FactoringVerdict::Refuses;
                }
            }
        }
        verdict
    }

    // ---- the closed form ---------------------------------------------------

    /// Whether the chain's producer seam hole `s` still appears in the recorded
    /// join of the member at `upto`.
    ///
    /// It does exactly while no `Drop` member has fired at or before that
    /// position: a `Drop` rewrites the producer position to a ground
    /// constructor, and every later join inherits that ground producer.
    ///
    /// # Contract
    /// - ensures: positive iff no prefix member through `upto` is `Drop`.
    /// - panics: none.
    fn seam_hole_survives(
        shape: &[MemberKind],
        upto: ChainIndex,
    ) -> SeamHoleVisibility
    {
        SeamHoleVisibility::from(
            shape
                .iter()
                .take(upto.0.saturating_add(1))
                .all(|&kind| kind != MemberKind::Drop),
        )
    }

    /// The composite verdict predicted from the shape alone.
    ///
    /// The gate declines at fold step `k` exactly when the hole `s` is still
    /// readable in member `k`'s join, some member of the accumulated prefix
    /// wears it `Mixed`, and member `k + 1` does too.
    ///
    /// # Contract
    /// - ensures: the closed form of the fold's verdict over a generated chain.
    /// - panics: none.
    fn predicted_composite(shape: &[MemberKind]) -> FactoringVerdict
    {
        for step in 0 .. shape.len().saturating_sub(1) {
            let next_is_mixed = shape.get(step.saturating_add(1)) == Some(&MemberKind::Mixed);
            let prefix_has_mixed = shape
                .iter()
                .take(step.saturating_add(1))
                .any(|&kind| kind == MemberKind::Mixed);
            if next_is_mixed
                && prefix_has_mixed
                && bool::from(seam_hole_survives(shape, ChainIndex(step)))
            {
                return FactoringVerdict::Refuses;
            }
        }
        FactoringVerdict::Admits
    }

    /// The adjacent conjunction predicted from the shape alone.
    ///
    /// # Contract
    /// - ensures: `Refuses` iff two consecutive members are `Mixed` while the
    ///   hole is still readable at the left one's join.
    /// - panics: none.
    fn predicted_adjacent(shape: &[MemberKind]) -> FactoringVerdict
    {
        for step in 0 .. shape.len().saturating_sub(1) {
            let left_is_mixed = shape.get(step) == Some(&MemberKind::Mixed);
            let right_is_mixed = shape.get(step.saturating_add(1)) == Some(&MemberKind::Mixed);
            if left_is_mixed
                && right_is_mixed
                && bool::from(seam_hole_survives(shape, ChainIndex(step)))
            {
                return FactoringVerdict::Refuses;
            }
        }
        FactoringVerdict::Admits
    }

    /// The own-join conjunction predicted from the shape alone.
    ///
    /// # Contract
    /// - ensures: `Refuses` iff two members are `Mixed` while the hole is still
    ///   readable at the earlier one's own join.
    /// - panics: none.
    fn predicted_own_join(shape: &[MemberKind]) -> FactoringVerdict
    {
        for earlier in 0 .. shape.len() {
            for later in earlier.saturating_add(1) .. shape.len() {
                if shape.get(earlier) == Some(&MemberKind::Mixed)
                    && shape.get(later) == Some(&MemberKind::Mixed)
                    && bool::from(seam_hole_survives(shape, ChainIndex(earlier)))
                {
                    return FactoringVerdict::Refuses;
                }
            }
        }
        FactoringVerdict::Admits
    }

    /// The fold-seam conjunction predicted from the shape alone.
    ///
    /// # Contract
    /// - ensures: `Refuses` iff two members are `Mixed` while the hole is still
    ///   readable at the join the fold presents when the later one is grafted.
    /// - panics: none.
    fn predicted_fold_seam(shape: &[MemberKind]) -> FactoringVerdict
    {
        for earlier in 0 .. shape.len() {
            for later in earlier.saturating_add(1) .. shape.len() {
                if shape.get(earlier) == Some(&MemberKind::Mixed)
                    && shape.get(later) == Some(&MemberKind::Mixed)
                    && bool::from(seam_hole_survives(
                        shape,
                        ChainIndex(later.saturating_sub(1)),
                    ))
                {
                    return FactoringVerdict::Refuses;
                }
            }
        }
        FactoringVerdict::Admits
    }

    // ---- the generator -----------------------------------------------------

    /// The chain operation name at a position.
    ///
    /// # Contract
    /// - ensures: distinct names at distinct positions, so exactly one store
    ///   cell matches at each peak of the chain.
    /// - panics: none.
    fn chain_op(index: ChainIndex) -> String
    {
        format!("op{}", index.0)
    }

    /// The cell a member of `kind` at `index` fires.
    ///
    /// # Contract
    /// - ensures: a cell whose left-hand side is headed by `opᵢ` and whose
    ///   right-hand side is headed by `opᵢ₊₁`, carrying the seam structure the
    ///   kind names.
    /// - panics: none.
    fn member_cell(
        kind: MemberKind,
        index: ChainIndex,
    ) -> Cell
    {
        let in_op = chain_op(index);
        let out_op = chain_op(ChainIndex(index.0.saturating_add(1)));
        let producer = format!("p{}", index.0);
        let consumer = format!("c{}", index.0);
        match kind {
            | MemberKind::Mixed => mixed_step(
                FixtureHoleName("s"),
                OperationName(&in_op),
                OperationName(&out_op),
            ),
            | MemberKind::Split => split_step(
                FixtureHoleName(&producer),
                FixtureHoleName(&consumer),
                OperationName(&in_op),
                OperationName(&out_op),
            ),
            | MemberKind::Drop => drop_step(
                FixtureHoleName(&producer),
                FixtureHoleName(&consumer),
                OperationName(&in_op),
                OperationName(&out_op),
            ),
        }
    }

    /// A **mixed**-variance step cell `⟨r | in(; r)⟩ ~> ⟨r | out(; r)⟩`: one
    /// name worn by a producer *and* a consumer metavariable, so the live
    /// derivation classifies it `CellVariance::Mixed`.
    fn mixed_step(
        hole: FixtureHoleName<'_>,
        in_op: OperationName<'_>,
        out_op: OperationName<'_>,
    ) -> Cell
    {
        Cell::new(
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::meta(hole.0),
                ConsPat::op(in_op.0, [], ConsPat::meta(hole.0)),
            ),
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::meta(hole.0),
                ConsPat::op(out_op.0, [], ConsPat::meta(hole.0)),
            ),
            Orientation::PolarityDerived,
            CellProvenance::SurfaceRule,
        )
    }

    /// A **split**-variance step cell `⟨p | in(; c)⟩ ~> ⟨p | out(; c)⟩`: the
    /// producer and the consumer wear different names, so neither is `Mixed`.
    fn split_step(
        producer: FixtureHoleName<'_>,
        consumer: FixtureHoleName<'_>,
        in_op: OperationName<'_>,
        out_op: OperationName<'_>,
    ) -> Cell
    {
        Cell::new(
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::meta(producer.0),
                ConsPat::op(in_op.0, [], ConsPat::meta(consumer.0)),
            ),
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::meta(producer.0),
                ConsPat::op(out_op.0, [], ConsPat::meta(consumer.0)),
            ),
            Orientation::PolarityDerived,
            CellProvenance::SurfaceRule,
        )
    }

    /// A **hole-dropping** step cell `⟨p | in(; c)⟩ ~> ⟨Zero() | out(; c)⟩`.
    ///
    /// Well-formed as a rewrite rule — the right-hand side's metavariables are
    /// a subset of the left's — and single-polarity like `split_step`. What it
    /// adds is that its right-hand side rewrites the producer position to a
    /// ground constructor, so the recorded join it hands the rest of the chain
    /// no longer carries the chain's producer seam hole.
    fn drop_step(
        producer: FixtureHoleName<'_>,
        consumer: FixtureHoleName<'_>,
        in_op: OperationName<'_>,
        out_op: OperationName<'_>,
    ) -> Cell
    {
        Cell::new(
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::meta(producer.0),
                ConsPat::op(in_op.0, [], ConsPat::meta(consumer.0)),
            ),
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::ctor("Zero", []),
                ConsPat::op(out_op.0, [], ConsPat::meta(consumer.0)),
            ),
            Orientation::PolarityDerived,
            CellProvenance::SurfaceRule,
        )
    }

    /// Every shape of the given length over the three member kinds.
    ///
    /// # Contract
    /// - ensures: all `3ⁿ` shapes, in a deterministic order.
    /// - panics: none.
    fn shapes(length: FamilyLength) -> Vec<Vec<MemberKind>>
    {
        let kinds = [MemberKind::Mixed, MemberKind::Split, MemberKind::Drop];
        let mut built: Vec<Vec<MemberKind>> = vec![Vec::new()];
        for _ in 0 .. length.0 {
            let mut extended = Vec::new();
            for prefix in &built {
                for &kind in &kinds {
                    let mut candidate = prefix.clone();
                    candidate.push(kind);
                    extended.push(candidate);
                }
            }
            built = extended;
        }
        built
    }

    /// A generated family: the store its members fire against, and the members
    /// in fold order.
    struct Family
    {
        /// The cell store every member is read against.
        store: CellStore,
        /// The members, sequentially seated in fold order.
        members: Vec<Tracelet>,
    }

    /// Build the family a shape names.
    ///
    /// # Contract
    /// - requires: at least two members, so the store carries a composition
    ///   overlap to serve as the recorded peak's carrier.
    /// - ensures: a sequentially seated chain — each member's recorded peak is
    ///   its predecessor's recorded join — in which every member records
    ///   exactly its own cell and replays.
    /// - panics: none in the contract sense; a chain that fails to seat or
    ///   replay fails the fixture's assertions, which is their purpose.
    fn build_family(shape: &[MemberKind]) -> Family
    {
        let mut store = CellStore::new();
        let mut cells = Vec::new();
        for (position, &kind) in shape.iter().enumerate() {
            cells.push(store.insert(member_cell(kind, ChainIndex(position))));
        }
        let template = enumerate_overlaps(&store)
            .into_iter()
            .find(|candidate| candidate.kind == OverlapKind::Composition)
            .expect("consecutive chain cells overlap at their shared operation");
        let mut peak = CmdPat::cut(
            Polarity::Positive,
            ProdPat::meta("s"),
            ConsPat::op("op0", [], ConsPat::meta("t")),
        );
        let mut members = Vec::new();
        for (position, &cell) in cells.iter().enumerate() {
            let member = one_step_certificate(&store, &template, &peak)
                .expect("exactly one store cell fires at the chain's next peak");
            assert_eq!(
                vec![cell],
                participating(&member),
                "member {position} records its own cell and nothing else"
            );
            assert!(
                bool::from(member.replay(&store)),
                "member {position} is a real certificate — it replays"
            );
            peak = member.joins_at.clone();
            members.push(member);
        }
        Family { store, members }
    }

    /// A one-step certificate over a cloned `template` overlap, applying the
    /// one store cell that fires at `peak`.
    ///
    /// The join is the **schematic** reduct — one budgeted normalization step,
    /// which matches and instantiates rather than skolemizing. That matters
    /// because the gate reads the seam variables off the recorded join: a join
    /// taken from a replay is ground and would present the gate an empty seam.
    ///
    /// The overlap is a carrier for the recorded peak: the gate and replay both
    /// read `overlap.peak`, and what these fixtures are about is the boundary a
    /// certificate records rather than the critical pair that produced it.
    fn one_step_certificate(
        store: &CellStore,
        template: &Overlap,
        peak: &CmdPat,
    ) -> Option<Tracelet>
    {
        let stepped = gandr_theory_coherent_resolutions::normalize(store, peak, 1_usize.into());
        let step = stepped.path.first()?.clone();
        let mut overlap = template.clone();
        overlap.peak = peak.clone();
        Some(Tracelet {
            overlap,
            path_a: vec![step.clone()],
            path_b: vec![step],
            joins_at: stepped.normal,
        })
    }

    /// The distinct metavariable **names** of a command pattern — the seam
    /// holes, keyed exactly as the gate keys them.
    ///
    /// # Contract
    /// - ensures: one entry per distinct metavariable name, in first-occurrence
    ///   order, across both categories.
    /// - panics: none.
    fn seam_hole_names(cmd: &CmdPat) -> Vec<String>
    {
        let mut occurrences = Vec::new();
        gandr_theory_cell_complexes::pattern::collect_cmd_metavars(cmd, &mut occurrences);
        let mut names: Vec<String> = Vec::new();
        for var in occurrences {
            if !names.iter().any(|held| held.as_str() == &*var.name) {
                names.push(String::from(&*var.name));
            }
        }
        names
    }

    /// Whether a certificate's recorded support wears `hole` at both
    /// polarities in some cell it fires.
    ///
    /// # Contract
    /// - ensures: positive iff some fired cell's live metadata classifies
    ///   `hole` as `CellVariance::Mixed`.
    /// - panics: none.
    fn support_wears_mixed(
        tracelet: &Tracelet,
        hole: FixtureHoleName<'_>,
        store: &CellStore,
    ) -> SeamHoleVisibility
    {
        SeamHoleVisibility::from(participating(tracelet).into_iter().any(|cell| {
            store.get(cell).is_some_and(|entry| {
                entry
                    .meta
                    .vars
                    .iter()
                    .any(|var| &*var.var.name == hole.0 && var.variance == CellVariance::Mixed)
            })
        }))
    }

    /// The distinct cells a certificate fires (`path_a` then `path_b`).
    fn participating(tracelet: &Tracelet) -> Vec<CellId>
    {
        let mut cells = Vec::new();
        for step in tracelet.path_a.iter().chain(&tracelet.path_b) {
            if !cells.contains(&step.cell) {
                cells.push(step.cell);
            }
        }
        cells
    }

    // ---- the sweep ---------------------------------------------------------

    /// The four verdicts one family produces, with the ledger that produced
    /// them.
    struct FamilyRun
    {
        /// The left fold of the gate over the whole family.
        composite: FactoringVerdict,
        /// The conjunction over consecutive pairs.
        adjacent: FactoringVerdict,
        /// The conjunction over all ordered pairs at their own joins.
        own_join: FactoringVerdict,
        /// The conjunction over all ordered pairs at the fold's seam terms.
        fold_seam: FactoringVerdict,
        /// What the gate did across all four readings.
        ledger: GateLedger,
    }

    /// Run all four readings over the family a shape names.
    ///
    /// # Contract
    /// - ensures: each reading's verdict, plus one ledger covering every
    ///   consultation all four made.
    /// - panics: none in the contract sense; the family's own assertions fire
    ///   on a malformed chain.
    fn run_family(shape: &[MemberKind]) -> FamilyRun
    {
        let family = build_family(shape);
        let mut ledger = GateLedger::default();
        let composite = composite_verdict(&family.members, &family.store, &mut ledger);
        let adjacent = adjacent_verdict(&family.members, &family.store, &mut ledger);
        let own_join = own_join_verdict(&family.members, &family.store, &mut ledger);
        let fold_seam = fold_seam_verdict(&family.members, &family.store, &mut ledger);
        FamilyRun {
            composite,
            adjacent,
            own_join,
            fold_seam,
            ledger,
        }
    }

    #[test]
    fn the_acyclicity_verdict_does_not_factor_through_pairwise_overlap_data()
    {
        // THE EXPERIMENT. Over every generated family, the gate's verdict on
        // the full composite is compared against three pairwise readings. Two
        // of them disagree with it, in OPPOSITE directions — the neighbour
        // reading admits composites the fold refuses, and the cover reading
        // refuses composites the fold admits. So the verdict is neither a
        // conjunction of neighbour verdicts nor a conjunction of all-pairs
        // verdicts, and no choice between the two rescues it.
        //
        // The third reading is the sharpening. Give a pairwise checker the one
        // datum a pair does not determine — which recorded join sits at the
        // seam when the right member is grafted — and it agrees with the fold
        // on every family. So the obstruction IS pairwise in the cells; what is
        // not pairwise is the seam term, and that is a function of the fold's
        // prefix.
        let mut families = 0_usize;
        let mut composite_refusals = 0_usize;
        let mut adjacent_misses = 0_usize;
        let mut adjacent_false_alarms = 0_usize;
        let mut own_join_misses = 0_usize;
        let mut own_join_false_alarms = 0_usize;
        let mut fold_seam_disagreements = 0_usize;
        let mut ledger = GateLedger::default();

        for length in [FamilyLength(3), FamilyLength(4)] {
            for shape in shapes(length) {
                let run = run_family(&shape);
                families = families.saturating_add(1);
                ledger.absorb(&run.ledger);

                // The closed form, cross-checked against every measurement.
                // Without it the counts below are numbers nobody can read; with
                // it, each reading's verdict is pinned to a stated mechanism.
                assert_eq!(
                    predicted_composite(&shape),
                    run.composite,
                    "the fold's verdict on {shape:?} matches the closed form"
                );
                assert_eq!(
                    predicted_adjacent(&shape),
                    run.adjacent,
                    "the adjacent conjunction on {shape:?} matches the closed form"
                );
                assert_eq!(
                    predicted_own_join(&shape),
                    run.own_join,
                    "the own-join conjunction on {shape:?} matches the closed form"
                );
                assert_eq!(
                    predicted_fold_seam(&shape),
                    run.fold_seam,
                    "the fold-seam conjunction on {shape:?} matches the closed form"
                );

                // The consultation count is a function of the family's shape,
                // asserted rather than reported: a reading that silently skips
                // its pairs would otherwise agree with anything.
                let members = shape.len();
                let pairs = members
                    .saturating_mul(members.saturating_sub(1))
                    .saturating_div(2);
                let conjunctions = members
                    .saturating_sub(1)
                    .saturating_add(pairs)
                    .saturating_add(pairs);
                let folds = match run.composite {
                    | FactoringVerdict::Admits => members.saturating_sub(1),
                    | FactoringVerdict::Refuses => {
                        run.ledger.consultations.saturating_sub(conjunctions)
                    },
                };
                assert!(
                    folds >= 1 && folds <= members.saturating_sub(1),
                    "the fold on {shape:?} consulted the gate once per graft up to its verdict"
                );
                assert_eq!(
                    conjunctions.saturating_add(folds),
                    run.ledger.consultations,
                    "every reading consulted the gate exactly as often as {shape:?} has pairs"
                );

                if run.composite == FactoringVerdict::Refuses {
                    composite_refusals = composite_refusals.saturating_add(1);
                }
                match (run.composite, run.adjacent) {
                    | (FactoringVerdict::Refuses, FactoringVerdict::Admits) => {
                        adjacent_misses = adjacent_misses.saturating_add(1);
                    },
                    | (FactoringVerdict::Admits, FactoringVerdict::Refuses) => {
                        adjacent_false_alarms = adjacent_false_alarms.saturating_add(1);
                    },
                    | _ => {},
                }
                match (run.composite, run.own_join) {
                    | (FactoringVerdict::Refuses, FactoringVerdict::Admits) => {
                        own_join_misses = own_join_misses.saturating_add(1);
                    },
                    | (FactoringVerdict::Admits, FactoringVerdict::Refuses) => {
                        own_join_false_alarms = own_join_false_alarms.saturating_add(1);
                    },
                    | _ => {},
                }
                if run.composite != run.fold_seam {
                    fold_seam_disagreements = fold_seam_disagreements.saturating_add(1);
                }
            }
        }

        // Non-vacuity, as assertions. A factoring comparison over families that
        // can never refuse is green and proves nothing, and one whose
        // consultations bypass the gate compares two implementations of a
        // bypass.
        assert_eq!(
            SweepCount::from(108_usize),
            SweepCount::from(families),
            "the sweep is exhaustive over the three member kinds at length three and four"
        );
        assert_eq!(
            SweepCount::from(0_usize),
            SweepCount::from(ledger.coherent),
            "no consultation took the invertible bypass, so every verdict is the acyclicity \
             gate's: {} consultations, all gated",
            ledger.consultations
        );
        assert_eq!(
            SweepCount::from(ledger.declines),
            SweepCount::from(ledger.acyclicity_declines),
            "every decline carries a variable-flow cycle over the operands' own mixed holes, so \
             the conjunctions compare like with like"
        );
        assert!(
            ledger.declines > 0,
            "the gate declined at all — {} declines over {} consultations",
            ledger.declines,
            ledger.consultations
        );
        assert!(
            composite_refusals > 0,
            "the generated families include composites the gate refuses"
        );

        // THE VERDICT, pinned. The two disagreement counts are the finding; the
        // zero is the sharpening.
        assert_eq!(
            SweepCount::from(23_usize),
            SweepCount::from(composite_refusals),
            "composites the fold refuses, of {families}"
        );
        assert_eq!(
            SweepCount::from(5_usize),
            SweepCount::from(adjacent_misses),
            "families whose every ADJACENT pair is admitted while the composite is refused — the \
             neighbour reading is not a sound substitute for the gate"
        );
        assert_eq!(
            SweepCount::from(0_usize),
            SweepCount::from(adjacent_false_alarms),
            "and the neighbour reading never refuses what the composite admits"
        );
        assert_eq!(
            SweepCount::from(0_usize),
            SweepCount::from(own_join_misses),
            "the all-pairs reading never admits what the composite refuses"
        );
        assert_eq!(
            SweepCount::from(8_usize),
            SweepCount::from(own_join_false_alarms),
            "while it refuses {own_join_false_alarms} composites the gate admits — the cover \
             reading is not the gate either, and it errs the other way"
        );
        assert_eq!(
            SweepCount::from(0_usize),
            SweepCount::from(fold_seam_disagreements),
            "given the fold's own seam term, the pairwise reading agrees on every family: the \
             obstruction is pairwise in the cells, the seam term is not pairwise data"
        );
    }

    #[test]
    fn a_store_mediated_reconvergence_refuses_a_composite_every_adjacent_pair_admits()
    {
        // THE COUNTEREXAMPLE to the neighbour-local reading, pinned on its own.
        //
        // Three members: a mixed-variance cell, a single-polarity cell, another
        // mixed-variance cell. Neither adjacent pair shares a mixed hole, so
        // both are admitted and an incremental scheme checking only neighbours
        // would admit the whole chain. The fold refuses at the second graft:
        // `graft` concatenates both legs, so the accumulator records the FIRST
        // member's cell as well, and that cell's mixed hole meets the third
        // member's across the seam. The two cells never meet as a pair; they
        // meet through the store when the composite's support accumulates.
        let shape = [MemberKind::Mixed, MemberKind::Split, MemberKind::Mixed];
        let family = build_family(&shape);
        let mut ledger = GateLedger::default();

        assert_eq!(
            FactoringVerdict::Admits,
            adjacent_verdict(&family.members, &family.store, &mut ledger),
            "every adjacent pair is admitted — the neighbour reading sees nothing"
        );
        assert_eq!(
            SweepCount::from(2_usize),
            SweepCount::from(ledger.consultations),
            "both adjacent pairs really were consulted"
        );

        let (Some(first), Some(second), Some(third)) = (
            family.members.first(),
            family.members.get(1),
            family.members.get(2),
        )
        else {
            panic!("the family has three members")
        };
        let CutOutcome::Directed(prefix) = directed_cut(first, second, &family.store)
        else {
            panic!("the first graft is admitted through the gate, not the invertible bypass")
        };
        assert!(
            bool::from(prefix.replay(&family.store)),
            "and the prefix composite is a real certificate — it replays"
        );

        let CutOutcome::Declined(obstruction) = directed_cut(&prefix, third, &family.store)
        else {
            panic!("the second graft is refused: the accumulated support closes the flow cycle")
        };
        assert!(
            acyclicity_class(&obstruction, &prefix, third, &family.store).0,
            "the refusal is a variable-flow cycle over mixed seam holes, not another decline class"
        );
        let recorded = participating(first);
        assert!(
            obstruction
                .cycle
                .iter()
                .any(|node| recorded.contains(&node.0)),
            "the cycle runs through the FIRST member's cell — the one no adjacent pair ever put \
             in front of the gate"
        );
    }

    #[test]
    fn the_criterion_reads_the_seam_holes_of_the_left_certificates_recorded_join()
    {
        // WHICH join supplies the seam holes, pinned on a pair whose two joins
        // DISAGREE about a hole that has endpoints on both sides. On a fixture
        // family where every hole is named by both joins the question is
        // unanswerable — the two candidate readings agree everywhere — so this
        // fixture is built where they cannot.
        //
        // The `Drop` member rewrites the producer position to a ground
        // constructor, so the first member's recorded join still carries the
        // hole `s` and the third member's no longer does, while BOTH members
        // fire a cell that wears `s` at both polarities. The unordered pair is
        // therefore one pair with two verdicts, and which one it gets is
        // decided entirely by which certificate is on the left.
        let shape = [MemberKind::Mixed, MemberKind::Drop, MemberKind::Mixed];
        let family = build_family(&shape);
        let (Some(first), Some(third)) = (family.members.first(), family.members.get(2))
        else {
            panic!("the family has three members")
        };

        // The hypothesis, asserted rather than assumed: the joins disagree
        // about `s`, and both supports wear it `Mixed`.
        assert!(
            seam_hole_names(&first.joins_at)
                .iter()
                .any(|name| name.as_str() == "s"),
            "the first member's recorded join carries the seam hole `s`"
        );
        assert!(
            !seam_hole_names(&third.joins_at)
                .iter()
                .any(|name| name.as_str() == "s"),
            "the third member's recorded join does not, because a Drop member ran between them"
        );
        for (label, member) in [("the first", first), ("the third", third)] {
            assert!(
                bool::from(support_wears_mixed(
                    member,
                    FixtureHoleName("s"),
                    &family.store
                )),
                "{label} member fires a cell wearing `s` at both polarities, so each side can \
                 supply half of a flow loop"
            );
        }

        // One pair, two orders, opposite verdicts — and the only thing that
        // moved is whose recorded join is on the left.
        let mut ledger = GateLedger::default();
        assert!(
            matches!(
                consult(&mut ledger, first, third, &family.store),
                Consultation::Declined
            ),
            "with the hole-carrying join on the left the criterion reads `s`, finds a mixed \
             endpoint on each side, and declines"
        );
        assert!(
            matches!(
                consult(&mut ledger, third, first, &family.store),
                Consultation::Admitted(_)
            ),
            "with the hole-dropping join on the left the criterion never reads `s` at all, so the \
             same two supports are admitted: the seam holes are the LEFT certificate's join's"
        );
        assert_eq!(
            SweepCount::from(ledger.declines),
            SweepCount::from(ledger.acyclicity_declines),
            "and the decline is an acyclicity decline, not a different verdict class"
        );
        assert_eq!(
            SweepCount::from(0_usize),
            SweepCount::from(ledger.coherent),
            "neither order took the invertible bypass, so both verdicts are the gate's"
        );
    }

    #[test]
    fn a_dropped_seam_hole_admits_a_composite_the_all_pairs_reading_refuses()
    {
        // THE COUNTEREXAMPLE to the cover reading, and it fails the other way.
        //
        // The middle member rewrites the producer position to a ground
        // constructor, so from there on the chain's recorded joins no longer
        // carry the seam hole `s`. The fold reads the seam holes off the join
        // it actually holds — the middle member's — so when the third member is
        // grafted the hole is not there to cycle and the composite is admitted.
        // The all-pairs reading asks the pair (first, third) at the FIRST
        // member's join, which still carries `s`, and refuses.
        //
        // Neither reading is wrong about its own question. What the pair does
        // not determine is which recorded join sits at the seam, and that is
        // the datum the fold has.
        let shape = [MemberKind::Mixed, MemberKind::Drop, MemberKind::Mixed];
        let family = build_family(&shape);
        let mut composite_ledger = GateLedger::default();
        let mut pairs_ledger = GateLedger::default();

        assert_eq!(
            FactoringVerdict::Admits,
            composite_verdict(&family.members, &family.store, &mut composite_ledger),
            "the fold admits: the middle member's join carries no mixed seam hole to cycle"
        );
        assert_eq!(
            SweepCount::from(2_usize),
            SweepCount::from(composite_ledger.consultations),
            "both grafts really were consulted"
        );
        assert_eq!(
            FactoringVerdict::Refuses,
            own_join_verdict(&family.members, &family.store, &mut pairs_ledger),
            "the all-pairs reading refuses a composite the gate admits"
        );
        assert_eq!(
            SweepCount::from(pairs_ledger.declines),
            SweepCount::from(pairs_ledger.acyclicity_declines),
            "and its refusal is an acyclicity decline, not a different verdict smuggled in"
        );

        // Which pair supplies the refusal, named rather than inferred: the
        // outer two, whose cells never meet at any seam the fold presents.
        let (Some(first), Some(third)) = (family.members.first(), family.members.get(2))
        else {
            panic!("the family has three members")
        };
        let mut outer_ledger = GateLedger::default();
        assert!(
            matches!(
                consult(&mut outer_ledger, first, third, &family.store),
                Consultation::Declined
            ),
            "the outer pair declines at the first member's own join"
        );

        // And the same two members, asked at the seam term the fold presents,
        // agree with the fold. The cells are unchanged; only the recorded join
        // moved.
        let Some(middle) = family.members.get(1)
        else {
            panic!("the family has three members")
        };
        let reseated = Tracelet {
            joins_at: middle.joins_at.clone(),
            ..first.clone()
        };
        assert!(
            matches!(
                consult(&mut outer_ledger, &reseated, third, &family.store),
                Consultation::Admitted(_)
            ),
            "re-seated on the fold's own seam term, the same pair is admitted"
        );
    }
}
