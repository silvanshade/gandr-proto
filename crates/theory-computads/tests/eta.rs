//! The **η cells a declaration mints**, and what makes them safe to mint
//! (`gandr-0rb` residual (e)).
//!
//! `CellProvenance::Eta` and its polarity gate have been in the alphabet since
//! the engine landed, exercised only by hand-built fixtures: no declaration
//! produced one. These fixtures cover the route that now does, and the two
//! obligations it carries.
//!
//! - [`the_two_routes_out_of_the_eta_redex_agree`] is the soundness witness:
//!   the η cell shortcuts a two-step route that already exists, so the critical
//!   pair between them must be joinable. If the law were not a law, the two
//!   routes would land in different normal forms.
//! - [`the_eta_cell_reaches_a_redex_the_projection_route_cannot`] is why it is
//!   worth minting rather than a restatement: the projection rule needs a
//!   producer literally built by the constructor, and the η cell quantifies
//!   over the producer.
//! - [`a_data_eta_cell_does_not_fire_at_a_negative_cut`] and
//!   [`a_codata_eta_cell_does_not_fire_at_a_positive_cut`] drive the
//!   strategy-tied η discipline from a **minted** cell rather than a hand-built
//!   one, which is what the discipline was missing.
//! - [`an_eta_step_replays`] pins that a certificate recording an η step is
//!   re-executable, so the cell is inside the replay discipline rather than
//!   beside it.

#[cfg(test)]
mod tests
{
    use gandr_core_sequent::il::Polarity;
    use gandr_theory_computads::CellStore;
    use gandr_theory_computads::CmdPat;
    use gandr_theory_computads::ConsPat;
    use gandr_theory_computads::ProdPat;
    use gandr_theory_computads::normalize;
    use gandr_theory_levitation::DeclPolarity;

    extern crate alloc;

    /// The normalization budget these fixtures run under — generous enough that
    /// exhaustion means a loop rather than a short ceiling.
    const BUDGET: usize = 64_usize;

    #[test]
    fn the_two_routes_out_of_the_eta_redex_agree()
    {
        // Why minting the cell is safe, run rather than argued. Both routes out
        // of `⟨MkWrap(v) | unwrap(; MkWrap⁻(★))⟩` exist: the projection rule
        // strips the constructor and the frame-defining cell puts it back, and
        // the η cell says those two steps cancel. That is a critical pair, and
        // if the law were not a law the two routes would land in different
        // normal forms.
        //
        // The engine tries cells in insertion order, so which route it takes is
        // decided by where the η cell sits. Both orders are built and both are
        // normalized.
        let (projection_first, eta_ids) = wrapper_store(DeclPolarity::Data);
        assert!(
            !eta_ids.is_empty(),
            "the wrapper declaration mints an η cell"
        );
        let eta_first = store_with_eta_first(&projection_first, &eta_ids);
        let redex = eta_redex(Polarity::Positive);

        let by_projection = normalize(&projection_first, &redex, BUDGET.into());
        let by_eta = normalize(&eta_first, &redex, BUDGET.into());
        assert!(
            !by_projection.exhausted && !by_eta.exhausted,
            "both routes reach a normal form rather than looping"
        );
        assert_eq!(
            by_projection.normal, by_eta.normal,
            "the critical pair is joinable: destructing and rebuilding lands where cancelling \
             does"
        );
        assert!(
            by_eta.path.len() < by_projection.path.len(),
            "and the η route is the shorter one, which is what makes the cell worth minting"
        );
        assert!(
            by_eta
                .path
                .iter()
                .any(|step| Some(step.cell) == store_eta_id(&eta_first)),
            "the short route really is the η step"
        );
    }

    #[test]
    fn the_eta_cell_reaches_a_redex_the_projection_route_cannot()
    {
        // What the law adds, and why it is not a restatement of the projection
        // rule. The projection rule matches only a producer literally built by
        // the constructor; the η cell quantifies over the producer, which is
        // the extensional content — `unwrap` followed by rebuilding cancels for
        // *any* `w`, not only for a value in hand.
        let (store, eta_ids) = wrapper_store(DeclPolarity::Data);
        let without_eta = store_without(&store, &eta_ids);
        let opaque = CmdPat::cut(
            Polarity::Positive,
            ProdPat::ctor("Opaque", []),
            ConsPat::op("unwrap", [], ConsPat::frame("MkWrap", ConsPat::Top)),
        );
        let stuck = normalize(&without_eta, &opaque, BUDGET.into());
        assert_eq!(
            opaque, stuck.normal,
            "without the η cell the redex is stuck: no rule matches a producer that is not a \
             `MkWrap` application"
        );
        let reduced = normalize(&store, &opaque, BUDGET.into());
        assert_eq!(
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::ctor("Opaque", []),
                ConsPat::Top
            ),
            reduced.normal,
            "with it, the destructor and the rebuild cancel and the producer is handed on"
        );
    }

    #[test]
    fn a_data_eta_cell_does_not_fire_at_a_negative_cut()
    {
        // The strategy-tied discipline, driven from a minted cell: data η is a
        // call-by-value law, so the same redex at a negative cut is a normal
        // form. Nothing else in the store fires there either, so the term comes
        // back untouched.
        let (store, _) = wrapper_store(DeclPolarity::Data);
        let redex = eta_redex(Polarity::Negative);
        let outcome = normalize(&store, &redex, BUDGET.into());
        assert_eq!(
            redex, outcome.normal,
            "data η must not fire at a negative cut, so the redex is a normal form"
        );
        assert!(outcome.path.is_empty(), "and no step was taken");
    }

    #[test]
    fn a_codata_eta_cell_does_not_fire_at_a_positive_cut()
    {
        // The mirror. A `codata` declaration mints its η at a negative cut, so
        // the positive redex leaves it alone — and the negative one does not.
        let (store, _) = wrapper_store(DeclPolarity::Codata);
        let positive = eta_redex(Polarity::Positive);
        let negative = eta_redex(Polarity::Negative);
        let at_negative = normalize(&store, &negative, BUDGET.into());
        assert!(
            !at_negative.path.is_empty(),
            "codata η fires at the negative cut it declares"
        );
        let at_positive = normalize(&store, &positive, BUDGET.into());
        assert!(
            at_positive
                .path
                .iter()
                .all(|step| Some(step.cell) != store_eta_id(&store)),
            "and never at a positive one, however well its left-hand side matches"
        );
    }

    #[test]
    fn an_eta_step_replays()
    {
        // The η cell is an ordinary cell as far as the certificate discipline is
        // concerned: a recorded η step re-executes rather than being trusted.
        let (projection_first, eta_ids) = wrapper_store(DeclPolarity::Data);
        let store = store_with_eta_first(&projection_first, &eta_ids);
        let redex = eta_redex(Polarity::Positive);
        let outcome = normalize(&store, &redex, BUDGET.into());
        assert!(
            outcome
                .path
                .iter()
                .any(|step| Some(step.cell) == store_eta_id(&store)),
            "the normalization did take the η step"
        );
        let overlap_source = gandr_theory_computads::enumerate_overlaps(&store)
            .into_iter()
            .next()
            .expect("the store enumerates at least one overlap to carry a peak");
        let mut overlap = overlap_source;
        overlap.peak = redex;
        let certificate = gandr_theory_computads::Tracelet {
            overlap,
            path_a: outcome.path.clone(),
            path_b: outcome.path,
            joins_at: outcome.normal,
        };
        assert!(
            bool::from(certificate.replay(&store)),
            "a certificate recording the η step replays over the store"
        );
    }

    /// The η redex `⟨MkWrap(Zero) |ε unwrap(; MkWrap⁻(★))⟩` — the destructor
    /// applied and its result rebuilt.
    fn eta_redex(polarity: Polarity) -> CmdPat
    {
        CmdPat::cut(
            polarity,
            ProdPat::ctor("MkWrap", [ProdPat::ctor("Zero", [])]),
            ConsPat::op("unwrap", [], ConsPat::frame("MkWrap", ConsPat::Top)),
        )
    }

    /// A copy of `store` whose η cells are inserted first, so the engine's
    /// insertion-order cell choice takes the η route.
    fn store_with_eta_first(
        store: &CellStore,
        eta_ids: &[gandr_theory_computads::CellId],
    ) -> CellStore
    {
        let mut out = CellStore::new();
        for &id in eta_ids {
            if let Some(cell) = store.get(id) {
                out.insert(cell.clone());
            }
        }
        for (id, cell) in store.iter() {
            if eta_ids.contains(&id) {
                continue;
            }
            out.insert(cell.clone());
        }
        out
    }

    /// The id of the store's η cell, if it holds one.
    fn store_eta_id(store: &CellStore) -> Option<gandr_theory_computads::CellId>
    {
        store.iter().find_map(|(id, cell)| {
            matches!(
                cell.provenance,
                gandr_theory_computads::CellProvenance::Eta(_)
            )
            .then_some(id)
        })
    }

    /// A copy of `store` with the cells at `dropped` left out.
    fn store_without(
        store: &CellStore,
        dropped: &[gandr_theory_computads::CellId],
    ) -> CellStore
    {
        let mut out = CellStore::new();
        for (id, cell) in store.iter() {
            if dropped.contains(&id) {
                continue;
            }
            out.insert(cell.clone());
        }
        out
    }

    /// The elaborated store of a single-constructor `Wrap` declaration whose
    /// `unwrap` operation carries the inverse face, with the ids of the η cells
    /// it minted.
    fn wrapper_store(
        polarity: DeclPolarity
    ) -> (CellStore, alloc::vec::Vec<gandr_theory_computads::CellId>)
    {
        use gandr_theory_levitation::Attrs;
        use gandr_theory_levitation::BridgeArity;
        use gandr_theory_levitation::Code;
        use gandr_theory_levitation::CtorDesc;
        use gandr_theory_levitation::FreeTerm;
        use gandr_theory_levitation::NominalId;
        use gandr_theory_levitation::OperDesc;
        use gandr_theory_levitation::RuleFace;
        use gandr_theory_levitation::SignDesc;
        use gandr_theory_levitation::SortRef;
        use gandr_theory_levitation::SurfaceSpan;

        let desc = SignDesc::new(
            NominalId::new(0_u64.into(), "Wrap"),
            alloc::vec::Vec::new(),
            [CtorDesc::new(
                "MkWrap",
                Code::var("Nat"),
                "Wrap",
                Attrs::empty(),
            )],
            [OperDesc::new(
                "unwrap",
                BridgeArity::single_output([SortRef::new("w", "Wrap")], SortRef::new("out", "Nat")),
                Attrs::empty(),
            )],
            [RuleFace::new(
                FreeTerm::op("unwrap", [FreeTerm::ctor("MkWrap", [FreeTerm::var("x")])]),
                FreeTerm::var("x"),
                alloc::vec::Vec::new(),
                SurfaceSpan::new(0_usize.into(), 0_usize.into()),
            )],
            polarity,
            Attrs::empty(),
        );
        let elaborated = gandr_theory_computads::elaborate_data_desc(&desc);
        assert!(
            elaborated.declined_faces.is_empty() && elaborated.declined_opers.is_empty(),
            "the wrapper declaration elaborates whole"
        );
        (elaborated.store, elaborated.eta)
    }
}
