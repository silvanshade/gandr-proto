//! Public structural-diff integration tests for flat CST arenas.
//!
//! # Contract
//! - ensures: unchanged significant roots are hash-pruned, layout edits remain
//!   semantic no-ops, statement-local token edits do not escape their
//!   statement, mold changes break alignment even with equal source text, and
//!   ambiguous LCS reconstruction advances the right/new side on ties.
//! - provides: public-API witnesses without inspecting implementation
//!   internals.
//! - panics: when the public CST builder or diff contract deviates from the
//!   documented behavior.
//! - intension: constructs all trees through [`CstBuilder`] and asserts node-id
//!   relationships plus [`Diff`] slices.
//!
//! # Adequacy
//! - hypothesis: L4 — root equality, layout-only drift, a one-token statement
//!   edit, mold drift, and repeated-signature LCS ambiguity each kill a
//!   distinct structural-diff branch.
//! - witness: `gandr_surface_syntax::equal_trees_prune_at_one_root_match`
//! - witness: `gandr_surface_syntax::space_source_and_range_changes_still_match_root`
//! - witness: `gandr_surface_syntax::changed_token_is_unmatched_only_at_its_statement`
//! - witness: `gandr_surface_syntax::same_text_with_changed_mold_is_unmatched`
//! - witness: `gandr_surface_syntax::repeated_sibling_tie_advances_the_right_side`
//!
//! The fixtures carry tiles as opaque [`MoldId`] references and grout interiors
//! as [`MoldPayload::Grout`]; the diff compares the material-governed payload,
//! so a changed tile mold breaks alignment exactly as a changed grout sort
//! does.

#[cfg(test)]
mod gandr_surface_syntax
{
    use gandr_surface_syntax::Cst;
    use gandr_surface_syntax::CstBuilder;
    use gandr_surface_syntax::Diff;
    use gandr_surface_syntax::GrammarFingerprint;
    use gandr_surface_syntax::GroutShape;
    use gandr_surface_syntax::GroutSort;
    use gandr_surface_syntax::Material;
    use gandr_surface_syntax::MoldId;
    use gandr_surface_syntax::MoldPayload;
    use gandr_surface_syntax::NodeId;
    use gandr_surface_syntax::NodeKind;
    use gandr_surface_syntax::NodeSlot;
    use gandr_surface_syntax::SourceText;
    use gandr_surface_syntax::SubtreeMatch;
    use gandr_surface_syntax::TextOffset;
    use gandr_surface_syntax::TextRange;
    use gandr_surface_syntax::diff;

    /// Fingerprint of the fixture grammar whose mold table these ids index.
    const GRAMMAR_FP: GrammarFingerprint = GrammarFingerprint(0x00ab_cdef_0123_4567);
    /// Dense fixture mold-table index used by the second statement token.
    const SECOND_TOKEN_MOLD_ID: u32 = 20;

    /// Layout-only source and range drift still hash-prunes the significant
    /// root.
    #[test]
    fn space_source_and_range_changes_still_match_root()
    {
        let mut old_builder = CstBuilder::new(SourceText::from("a b"), GRAMMAR_FP);
        let old_a = tile(
            &mut old_builder,
            first_token_mold(),
            TextOffset(0),
            TextOffset(1),
        );
        let old_space = space(&mut old_builder, TextOffset(1), TextOffset(2));
        let old_b = tile(
            &mut old_builder,
            second_token_mold(),
            TextOffset(2),
            TextOffset(3),
        );
        let old_root = old_builder
            .node(
                NodeKind::Cell,
                Material::Space,
                MoldPayload::Space,
                range(TextOffset(0), TextOffset(3)),
                [old_a, old_space, old_b],
            )
            .expect("old layout root must be valid");
        let old = finish(old_builder, old_root);

        let mut new_builder = CstBuilder::new(SourceText::from("a   b"), GRAMMAR_FP);
        let new_a = tile(
            &mut new_builder,
            first_token_mold(),
            TextOffset(0),
            TextOffset(1),
        );
        let new_space_lead = space(&mut new_builder, TextOffset(1), TextOffset(2));
        let new_space_tail = space(&mut new_builder, TextOffset(2), TextOffset(4));
        let new_b = tile(
            &mut new_builder,
            second_token_mold(),
            TextOffset(4),
            TextOffset(5),
        );
        let new_root = new_builder
            .node(
                NodeKind::Cell,
                Material::Space,
                MoldPayload::Space,
                range(TextOffset(0), TextOffset(5)),
                [new_a, new_space_lead, new_space_tail, new_b],
            )
            .expect("new layout root must be valid");
        let new = finish(new_builder, new_root);

        assert_eq!(
            Material::Space,
            old.node(old_space)
                .expect("old space must resolve")
                .material(),
            "old layout child must be marked as space"
        );
        assert_eq!(
            new.node(new_space_tail)
                .expect("new trailing space must resolve")
                .range(),
            range(TextOffset(2), TextOffset(4)),
            "new layout range must witness whitespace drift"
        );

        let actual = diff(&old, &new);

        assert_diff(
            &actual,
            &[SubtreeMatch::new(old.root(), new.root())],
            &[],
            &[],
        );
    }

    /// Repeated equal sibling signatures choose right-side advancement on ties.
    #[test]
    fn repeated_sibling_tie_advances_the_right_side()
    {
        let mut old_builder = CstBuilder::new(SourceText::from("b a b"), GRAMMAR_FP);
        let (old_left_b, _old_left_b_token) = statement(
            &mut old_builder,
            REPEATED_B_SORT,
            second_token_mold(),
            TextOffset(0),
            TextOffset(1),
        );
        let (old_a, _old_a_token) = statement(
            &mut old_builder,
            REPEATED_A_SORT,
            first_token_mold(),
            TextOffset(2),
            TextOffset(3),
        );
        let (old_right_b, _old_right_b_token) = statement(
            &mut old_builder,
            REPEATED_B_SORT,
            second_token_mold(),
            TextOffset(4),
            TextOffset(5),
        );
        let old_root = old_builder
            .node(
                NodeKind::Cell,
                Material::Grout,
                grout_payload(ROOT_SORT),
                range(TextOffset(0), TextOffset(5)),
                [old_left_b, old_a, old_right_b],
            )
            .expect("old ambiguous root must be valid");
        let old = finish(old_builder, old_root);

        let mut new_builder = CstBuilder::new(SourceText::from("a b b"), GRAMMAR_FP);
        let (new_a, _new_a_token) = statement(
            &mut new_builder,
            REPEATED_A_SORT,
            first_token_mold(),
            TextOffset(0),
            TextOffset(1),
        );
        let (new_left_b, _new_left_b_token) = statement(
            &mut new_builder,
            REPEATED_B_SORT,
            second_token_mold(),
            TextOffset(2),
            TextOffset(3),
        );
        let (new_right_b, _new_right_b_token) = statement(
            &mut new_builder,
            REPEATED_B_SORT,
            second_token_mold(),
            TextOffset(4),
            TextOffset(5),
        );
        let new_root = new_builder
            .node(
                NodeKind::Cell,
                Material::Grout,
                grout_payload(ROOT_SORT),
                range(TextOffset(0), TextOffset(5)),
                [new_a, new_left_b, new_right_b],
            )
            .expect("new ambiguous root must be valid");
        let new = finish(new_builder, new_root);

        assert_eq!(
            old.children(old.root())
                .expect("old root children must resolve"),
            &[old_left_b, old_a, old_right_b],
            "old root children must preserve fixture order"
        );
        assert_eq!(
            new.children(new.root())
                .expect("new root children must resolve"),
            &[new_a, new_left_b, new_right_b],
            "new root children must preserve fixture order"
        );

        let actual = diff(&old, &new);

        assert_diff(
            &actual,
            &[
                SubtreeMatch::new(old_left_b, new_left_b),
                SubtreeMatch::new(old_right_b, new_right_b),
            ],
            &[old_a],
            &[new_a],
        );
    }
    /// Sort assigned to the first statement in ordinary two-statement fixtures.
    const FIRST_STATEMENT_SORT: GroutSort = GroutSort(30);
    /// Sort assigned to the second statement in ordinary two-statement
    /// fixtures.
    const SECOND_STATEMENT_SORT: GroutSort = GroutSort(40);
    /// Changed first-statement sort used to break a mold-payload match.
    const CHANGED_FIRST_STATEMENT_SORT: GroutSort = GroutSort(31);
    /// Sort of repeated `b` statements in the LCS tie fixture.
    const REPEATED_B_SORT: GroutSort = GroutSort(50);
    /// Sort of the `a` statement in the LCS tie fixture.
    const REPEATED_A_SORT: GroutSort = GroutSort(60);
    /// Sort assigned to fixture root nodes.
    const ROOT_SORT: GroutSort = GroutSort(1);

    /// Equal CSTs emit one root match and prune descendants.
    #[test]
    fn equal_trees_prune_at_one_root_match()
    {
        let (old, old_first, old_second) = two_statements(
            SourceText::from("a b"),
            FIRST_STATEMENT_SORT,
            SECOND_STATEMENT_SORT,
        );
        let (new, new_first, new_second) = two_statements(
            SourceText::from("a b"),
            FIRST_STATEMENT_SORT,
            SECOND_STATEMENT_SORT,
        );

        assert_eq!(
            NodeSlot(4),
            old.root().slot(),
            "old root uses dense id slot 4"
        );
        assert_eq!(
            NodeSlot(4),
            new.root().slot(),
            "new root uses dense id slot 4"
        );
        assert_eq!(
            old.node(old_first)
                .expect("old first statement must resolve")
                .parent(),
            Some(old.root()),
            "old first statement parent must be root"
        );
        assert_eq!(
            new.node(new_second)
                .expect("new second statement must resolve")
                .parent(),
            Some(new.root()),
            "new second statement parent must be root"
        );

        let actual = diff(&old, &new);

        assert_diff(
            &actual,
            &[SubtreeMatch::new(old.root(), new.root())],
            &[],
            &[],
        );
        assert!(
            !actual
                .matches()
                .iter()
                .any(|matched| matched.old_root() == old_first || matched.old_root() == old_second),
            "descendant statements must be pruned by the root match"
        );
        assert!(
            !actual
                .matches()
                .iter()
                .any(|matched| matched.new_root() == new_first || matched.new_root() == new_second),
            "new descendant statements must be pruned by the root match"
        );
    }

    /// Editing one token only reports the edited statement roots as unmatched.
    #[test]
    fn changed_token_is_unmatched_only_at_its_statement()
    {
        let (old, old_changed, old_unchanged) = two_statements(
            SourceText::from("a b"),
            FIRST_STATEMENT_SORT,
            SECOND_STATEMENT_SORT,
        );
        let (new, new_changed, new_unchanged) = two_statements(
            SourceText::from("x b"),
            FIRST_STATEMENT_SORT,
            SECOND_STATEMENT_SORT,
        );

        assert_eq!(
            old.node(old_changed)
                .expect("old changed statement must resolve")
                .parent(),
            Some(old.root()),
            "old changed statement must be a root child"
        );
        assert_eq!(
            new.node(new_changed)
                .expect("new changed statement must resolve")
                .parent(),
            Some(new.root()),
            "new changed statement must be a root child"
        );

        let actual = diff(&old, &new);

        assert_diff(
            &actual,
            &[SubtreeMatch::new(old_unchanged, new_unchanged)],
            &[old_changed],
            &[new_changed],
        );
    }

    /// Equal token bytes with different molds are semantic diff boundaries.
    #[test]
    fn same_text_with_changed_mold_is_unmatched()
    {
        let (old, old_statement, _old_token) = two_statements(
            SourceText::from("a b"),
            FIRST_STATEMENT_SORT,
            SECOND_STATEMENT_SORT,
        );
        let (new, new_statement, _new_token) = two_statements(
            SourceText::from("a b"),
            CHANGED_FIRST_STATEMENT_SORT,
            SECOND_STATEMENT_SORT,
        );

        assert_eq!(
            old.text(old_statement)
                .expect("old statement text must resolve"),
            new.text(new_statement)
                .expect("new statement text must resolve"),
            "fixture must change mold without changing source bytes"
        );
        assert_ne!(
            old.node(old_statement)
                .expect("old statement must resolve")
                .payload(),
            new.node(new_statement)
                .expect("new statement must resolve")
                .payload(),
            "fixture must change the public mold payload"
        );

        let actual = diff(&old, &new);

        assert_diff(
            &actual,
            &[SubtreeMatch::new(
                old.children(old.root())
                    .expect("old root children must resolve")[1],
                new.children(new.root())
                    .expect("new root children must resolve")[1],
            )],
            &[old_statement],
            &[new_statement],
        );
    }

    /// Build a two-statement CST and return `(tree, first_statement,
    /// second_statement)`.
    fn two_statements(
        source: SourceText,
        first_sort: GroutSort,
        second_sort: GroutSort,
    ) -> (Cst, NodeId, NodeId)
    {
        let mut builder = CstBuilder::new(source, GRAMMAR_FP);
        let (first, _first_token) = statement(
            &mut builder,
            first_sort,
            first_token_mold(),
            TextOffset(0),
            TextOffset(1),
        );
        let (second, _second_token) = statement(
            &mut builder,
            second_sort,
            second_token_mold(),
            TextOffset(2),
            TextOffset(3),
        );
        let root = builder
            .node(
                NodeKind::Cell,
                Material::Grout,
                grout_payload(ROOT_SORT),
                range(TextOffset(0), TextOffset(3)),
                [first, second],
            )
            .expect("fixture root must be valid");

        return (finish(builder, root), first, second);
    }

    /// Add a one-token statement node and return `(statement, token)`.
    fn statement(
        builder: &mut CstBuilder,
        statement_sort: GroutSort,
        token_mold: MoldId,
        start: TextOffset,
        end: TextOffset,
    ) -> (NodeId, NodeId)
    {
        let token = tile(builder, token_mold, start, end);
        let statement = builder
            .node(
                NodeKind::Meld,
                Material::Grout,
                grout_payload(statement_sort),
                range(start, end),
                [token],
            )
            .expect("fixture statement must be valid");

        return (statement, token);
    }

    /// Add one significant tile token referencing mold `mold_id`.
    fn tile(
        builder: &mut CstBuilder,
        mold_id: MoldId,
        start: TextOffset,
        end: TextOffset,
    ) -> NodeId
    {
        return builder
            .token(Material::Tile, tile_payload(mold_id), range(start, end))
            .expect("fixture tile must be valid");
    }

    /// Tile payload referencing the fixture grammar mold table.
    const fn tile_payload(mold_id: MoldId) -> MoldPayload
    {
        return MoldPayload::Tile(mold_id);
    }
    /// Mold id used by the first token in two-statement fixtures.
    fn first_token_mold() -> MoldId
    {
        MoldId::from(10)
    }

    /// Add one insignificant layout token.
    fn space(
        builder: &mut CstBuilder,
        start: TextOffset,
        end: TextOffset,
    ) -> NodeId
    {
        return builder
            .token(Material::Space, MoldPayload::Space, range(start, end))
            .expect("fixture space must be valid");
    }

    /// Mold id used by the second token in two-statement fixtures.
    fn second_token_mold() -> MoldId
    {
        MoldId::from(SECOND_TOKEN_MOLD_ID)
    }

    /// Grout payload carrying a fixture sort tag.
    const fn grout_payload(sort: GroutSort) -> MoldPayload
    {
        return MoldPayload::Grout {
            shape: GroutShape::Convex,
            sort,
        };
    }

    /// Checked public text range constructor with fixture diagnostics.
    fn range(
        start: TextOffset,
        end: TextOffset,
    ) -> TextRange
    {
        return TextRange::new(start, end).expect("fixture range must be monotone");
    }

    /// Finish a builder with the supplied root.
    fn finish(
        builder: CstBuilder,
        root: NodeId,
    ) -> Cst
    {
        return builder
            .finish(root)
            .expect("fixture CST must close at root");
    }

    /// Assert a diff contains exactly the supplied slices.
    fn assert_diff(
        actual: &Diff,
        matches: &[SubtreeMatch],
        unmatched_old: &[NodeId],
        unmatched_new: &[NodeId],
    )
    {
        assert_eq!(actual.matches(), matches, "matched subtree roots differ");
        assert_eq!(
            actual.unmatched_old(),
            unmatched_old,
            "old unmatched roots differ"
        );
        assert_eq!(
            actual.unmatched_new(),
            unmatched_new,
            "new unmatched roots differ"
        );
    }
}
