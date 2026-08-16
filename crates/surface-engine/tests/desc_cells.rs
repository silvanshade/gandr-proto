//! End-to-end **description → cell layer** tests: parse a `data` block,
//! elaborate it to a `gandr_theory_levitation::SignDesc`, then elaborate that
//! description into the `gandr-theory-computads` cell store.
//!
//! These exercise the whole wire the description layer's `op` members travel:
//! surface `op` → [`gandr_theory_levitation::BridgeArity`] → the cell layer's
//! admission verdict → cells (or an inspectable decline).

/// End-to-end cell-layer elaboration tests.
#[cfg(test)]
mod tests
{
    use gandr_surface_engine::desc_cells::elaborate_desc_cells;
    use gandr_surface_engine::desc_elab::elaborate_data_descs;
    use gandr_theory_computads::CellCount;
    use gandr_theory_computads::CellId;
    use gandr_theory_levitation::check_desc;

    use crate::common::TestText;

    /// A `Nat` theory whose `add` is declared single-output and whose two rules
    /// are in the supported flattening fragment.
    const NAT_ADD: &str = "data NatAdd : Type {\n  Zero : NatAdd;\n  Succ : (n : NatAdd) --> \
                           NatAdd;\n  oper add(m : NatAdd, n : NatAdd) -> NatAdd;\n  rule \
                           add(Zero, n) ==> n;\n  rule add(Succ(m), n) ==> Succ(add(m, n));\n}";

    /// The promoted many-out witness: a well-formed description whose `divmod`
    /// the single-continuation cell grammar cannot hold.
    const NAT_DIV: &str = "data NatDiv : Type {\n  Zero : NatDiv;\n  oper divmod(m : NatDiv, n : \
                           NatDiv) -> (q : NatDiv, r : NatDiv);\n}";

    #[test]
    fn a_declared_single_output_operation_lets_its_rule_become_a_cell()
    {
        let elab = elaborate_data_descs(NAT_ADD);
        assert!(
            elab.diagnostics.is_empty(),
            "the declaration is well-formed: {:?}",
            elab.diagnostics
        );
        let desc = elab.descs.first().expect("NatAdd elaborated");
        assert_eq!(1, desc.opers.len(), "one declared operation");
        assert_eq!(2, desc.rules.len(), "two declared 2-cell faces");

        let cells = elaborate_desc_cells(&elab.descs);
        assert!(
            cells.diagnostics.is_empty(),
            "nothing is declined at the cell layer: {:?}",
            cells.diagnostics
        );
        let store = cells.stores.first().expect("one store per description");
        // Two constructor frame cells (Zero⁻, Succ⁻) plus the two rule cells.
        assert_eq!(
            CellCount::from(4_usize),
            store.len(),
            "the declared rules reached the cell store"
        );
    }

    #[test]
    fn a_many_out_operation_is_a_well_formed_description_and_an_unrepresentable_cell()
    {
        let elab = elaborate_data_descs(NAT_DIV);
        let desc = elab.descs.first().expect("NatDiv elaborated");
        // The levitation layer accepts the many-out arity: its three maps
        // compose (one monomial per output port, each reading every input).
        assert_eq!(
            2,
            desc.opers
                .first()
                .expect("divmod elaborated")
                .arity
                .outputs
                .len(),
            "divmod declares a two-port result tuple"
        );
        assert!(
            check_desc(desc).is_empty(),
            "the multi-out arity is well-formed: {:?}",
            check_desc(desc)
        );

        // The cell layer declines it, and says why.
        let cells = elaborate_desc_cells(&elab.descs);
        assert_eq!(
            1,
            cells.diagnostics.len(),
            "exactly one cell-layer decline: {:?}",
            cells.diagnostics
        );
        let decline = cells.diagnostics.first().expect("one decline");
        assert!(
            decline.message.contains("divmod") && decline.message.contains("many-out"),
            "the decline names the operation and the reason: {}",
            decline.message
        );
        let store = cells.stores.first().expect("one store per description");
        assert_eq!(
            CellCount::from(1_usize),
            store.len(),
            "only `Zero`'s frame-defining cell reached the store"
        );
    }

    #[test]
    fn a_rule_over_a_many_out_operation_is_declined_rather_than_narrowed()
    {
        // The rule's left-hand side applies `divmod`, whose result the operation
        // frame's single return continuation cannot carry: elaborating it would
        // silently drop the second output port.
        let source = "data NatDiv2 : Type {\n  Zero : NatDiv2;\n  oper divmod(m : NatDiv2, n : \
                      NatDiv2) -> (q : NatDiv2, r : NatDiv2);\n  rule divmod(Zero, n) ==> Zero;\n}";
        let elab = elaborate_data_descs(source);
        let cells = elaborate_desc_cells(&elab.descs);
        assert!(
            cells.diagnostics.iter().any(|diagnostic| diagnostic
                .message
                .contains("rule of `NatDiv2`")
                && diagnostic.message.contains("declined")),
            "the rule is declined for its operation, not elaborated: {:?}",
            cells.diagnostics
        );
        let store = cells.stores.first().expect("one store per description");
        assert_eq!(
            CellCount::from(1_usize),
            store.len(),
            "no rule cell reached the store"
        );
    }

    #[test]
    fn a_source_with_no_declarations_elaborates_to_no_stores()
    {
        let cells = elaborate_desc_cells(&elaborate_data_descs("ret ()").descs);
        assert!(cells.stores.is_empty(), "no declaration, no store");
        assert!(cells.diagnostics.is_empty(), "and nothing to decline");
    }

    /// The cell identities a source lowers to, as `(id, lhs, rhs)` triples per
    /// store, flattened in store order.
    fn identities(source: TestText<'_>) -> Vec<(CellId, String, String)>
    {
        elaborate_desc_cells(&elaborate_data_descs(source.0).descs)
            .stores
            .iter()
            .flat_map(|store| {
                store
                    .iter()
                    .map(|(id, cell)| (id, format!("{:?}", cell.lhs), format!("{:?}", cell.rhs)))
            })
            .collect()
    }

    #[test]
    fn cell_identities_are_a_deterministic_function_of_the_source()
    {
        // A `CellId` is an insertion-order address, and insertion order is the
        // declaration order this seam walks: constructors first, then rule
        // faces, both in the order the block wrote them. So the identities are
        // a function of the source text and of nothing else — not of a hash
        // iteration order, not of an allocation address, not of how many times
        // the pass has run in this process.
        let once = identities(TestText(NAT_ADD));
        let twice = identities(TestText(NAT_ADD));
        assert_eq!(once, twice, "the same source lowers to the same identities");
        assert_eq!(
            vec![CellId(0), CellId(1), CellId(2), CellId(3)],
            once.iter().map(|entry| entry.0).collect::<Vec<CellId>>(),
            "ids are the dense insertion-order addresses of the store"
        );
    }

    #[test]
    fn declaration_order_is_the_cell_order()
    {
        // Swapping the two rule faces swaps the two rule cells and leaves the
        // constructor frame cells where they were. That is the property a
        // consumer reading a store by id depends on: the order is the source's,
        // so it is reviewable and diffable rather than incidental.
        const SWAPPED: &str = "data NatAdd : Type {\n  Zero : NatAdd;\n  Succ : (n : NatAdd) --> \
                               NatAdd;\n  oper add(m : NatAdd, n : NatAdd) -> NatAdd;\n  rule \
                               add(Succ(m), n) ==> Succ(add(m, n));\n  rule add(Zero, n) ==> n;\n}";
        let declared = identities(TestText(NAT_ADD));
        let swapped = identities(TestText(SWAPPED));
        assert_eq!(4, declared.len(), "two frame cells and two rule cells");
        assert_eq!(4, swapped.len(), "the same four cells");
        assert_eq!(
            declared[.. 2],
            swapped[.. 2],
            "the constructor frame cells are untouched by the rule order"
        );
        assert_eq!(
            (declared[2].1.clone(), declared[2].2.clone()),
            (swapped[3].1.clone(), swapped[3].2.clone()),
            "the first declared rule is the last one after the swap"
        );
        assert_eq!(
            (declared[3].1.clone(), declared[3].2.clone()),
            (swapped[2].1.clone(), swapped[2].2.clone()),
            "and the second is the first"
        );
    }

    #[test]
    fn a_face_declared_twice_is_one_cell()
    {
        // `CellStore::insert` deduplicates on structural identity, so a face
        // written twice occupies one address rather than two. The store's
        // length is therefore a count of distinct cells, which is what the
        // corpus `expect-desc-store-cells` expectation is asserting.
        let twice = "data Idem : Type {\n  Zero : Idem;\n  oper f(x : Idem) -> Idem;\n  rule \
                     f(Zero) ==> Zero;\n  rule f(Zero) ==> Zero;\n}";
        let elab = elaborate_data_descs(twice);
        assert_eq!(
            2,
            elab.descs.first().expect("Idem elaborated").rules.len(),
            "the description carries both written faces"
        );
        let cells = elaborate_desc_cells(&elab.descs);
        assert!(
            cells.diagnostics.is_empty(),
            "a repeat is a duplicate, not a decline: {:?}",
            cells.diagnostics
        );
        let store = cells.stores.first().expect("one store per description");
        assert_eq!(
            CellCount::from(2_usize),
            store.len(),
            "one frame cell and one rule cell: the repeat dedups to its first address"
        );
    }
}
