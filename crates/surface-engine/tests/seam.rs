//! The parser-agnostic item seam, exercised against the real front end
//! (`incremental-pipeline.md` §"Cold reparse" and §"The structural diff").
//!
//! `gandr_core_incremental::region::ItemSource` states the boundary the
//! changed-region detector reads across, and `crate::item_source` is the
//! implementation of it that is not a test double. Two things need proving
//! about it, and they are different claims.
//!
//! 1. **The crossing is faithful.**
//!    [`tests::the_seam_carries_names_ascriptions_and_order`] checks that the
//!    three fields the unchanged-region test compares — name, ascription, and
//!    source order — survive the crossing, since dropping any of them would let
//!    the detector adopt an item it must re-type while every from-scratch check
//!    still passed.
//! 2. **The engine runs across it.**
//!    [`tests::the_seam_admits_the_differential_gate`] resumes over an edit
//!    made through the seam and asserts the result equals a from-scratch
//!    re-type, and that reuse actually occurred — so the seam is shown to carry
//!    enough for the footprint to clear anything at all, not merely enough to
//!    type.

#![cfg_attr(
    dylint_lib = "non_topologically_sorted_functions",
    allow(
        unknown_lints,
        non_topologically_sorted_functions,
        reason = "integration tests share fixture helpers called from tests in per-test orders; no single module arrangement satisfies every caller-before-callee pair, so the ordering rule is waived in test code"
    )
)]

/// The seam's fidelity, and the engine running across it.
#[cfg(test)]
mod tests
{
    use alloc::string::String;
    use alloc::vec::Vec;

    use gandr_core_incremental::region::ItemSource as _;
    use gandr_core_incremental::region::Program;
    use gandr_core_term::types::Ty;
    use gandr_surface_engine::item_source::LoweringItemSource;
    use gandr_surface_engine::item_source::SourceRevision;
    use gandr_surface_engine::lower::Lowered;
    use gandr_surface_engine::lower::lower_source_total;
    use gandr_surface_engine::prelude_ctx;

    use crate::common::TestText;

    /// The comparable shape of one item's typing.
    ///
    /// A typing failure carries no payload here: what these assertions compare
    /// is the *classification* an item receives across the seam, and an error's
    /// identity is already pinned by the engine's own from-scratch gate.
    #[derive(Clone, Debug, Eq, PartialEq)]
    enum Typing
    {
        /// A definition that typed, with its name and reported type.
        Definition
        {
            /// The defined name.
            name: String,
            /// The reported type.
            ty: Ty,
            /// Whether the definition entered scope.
            bound: bool,
        },
        /// An expression that typed, with its terminal type.
        Expression
        {
            /// The expression's type.
            ty: Ty,
        },
        /// An item whose typing failed.
        Failed,
        /// An item declined for carrying a hole.
        Holey,
    }

    /// The engine's typings for `program`, from scratch, against the surface
    /// prelude the front end elaborates into.
    fn seam_typings(program: &Program) -> Vec<Typing>
    {
        use gandr_core_incremental::checkpoint::ItemTyping;

        gandr_core_incremental::checkpoint::checkpoint_with(program, &prelude_ctx())
            .items
            .into_iter()
            .map(|checkpoint| match checkpoint.typing {
                | ItemTyping::Definition { name, ty, bound } => {
                    Typing::Definition { name, ty, bound }
                },
                | ItemTyping::Expression { ty } => Typing::Expression { ty },
                | ItemTyping::TypeError { .. } => Typing::Failed,
                | ItemTyping::Holey => Typing::Holey,
            })
            .collect()
    }

    /// The seam's program for `source`.
    fn seam_program<'text>(source: impl Into<TestText<'text>>) -> Program
    {
        let source = source.into().0;
        LoweringItemSource
            .items(&SourceRevision::from(source))
            .expect("total lowering fails only when the parser is unavailable")
    }

    /// Totally lowers `source` through the melder front end.
    fn lower<'text>(source: impl Into<TestText<'text>>) -> Lowered
    {
        let source = source.into().0;
        lower_source_total(source.into()).expect("total lowering never fails structurally")
    }

    /// The seam carries exactly the item identity the unchanged-region test
    /// compares: the name, the ascription, and the source order. A projection
    /// that dropped any of them would still pass every from-scratch check while
    /// silently making adoption unsound.
    #[test]
    fn the_seam_carries_names_ascriptions_and_order()
    {
        let source = "def n: Integer = 1;\ndef m = n;\nprint(m)\n";
        let program = seam_program(source);
        let lowered = lower(source);

        assert_eq!(
            lowered.items.len(),
            program.items.len(),
            "the seam yields one item per lowered item"
        );
        for (index, lowered_item) in lowered.items.iter().enumerate() {
            let item = program
                .items
                .get(index)
                .expect("the seam preserves the item count");
            assert_eq!(
                lowered_item.name, item.name,
                "item {index} keeps its definition name across the seam"
            );
            assert_eq!(
                lowered_item.ascription, item.ascription,
                "item {index} keeps its ascription across the seam"
            );
            assert_eq!(
                lowered_item.term, item.term,
                "item {index} keeps its lowered term across the seam"
            );
        }
        assert!(
            program
                .items
                .first()
                .is_some_and(|item| item.ascription.is_some()),
            "the ascribed definition arrives ascribed, so the seam is not dropping the field \
             this corpus exists to observe"
        );
    }

    /// The changed-region detector, driven through the seam over an edit,
    /// reproduces a from-scratch re-type — the differential gate against the
    /// real front end rather than a test double.
    #[test]
    fn the_seam_admits_the_differential_gate()
    {
        let base = "def target(x: Integer) -> F Integer {\n  ret (x + 1)\n}\nprint(target)\n";
        let edited = "def target(x: Integer) -> F Integer {\n  ret (x + 2)\n}\nprint(target)\n";

        let checkpoints = gandr_core_incremental::checkpoint::checkpoint_with(
            &seam_program(base),
            &prelude_ctx(),
        );
        let resumed = gandr_core_incremental::checkpoint::resume_with(
            &checkpoints,
            &seam_program(edited),
            &prelude_ctx(),
        );

        assert_eq!(
            seam_typings(&seam_program(edited)),
            resumed
                .typings()
                .cloned()
                .map(|typing| {
                    use gandr_core_incremental::checkpoint::ItemTyping;

                    match typing {
                        | ItemTyping::Definition { name, ty, bound } => {
                            Typing::Definition { name, ty, bound }
                        },
                        | ItemTyping::Expression { ty } => Typing::Expression { ty },
                        | ItemTyping::TypeError { .. } => Typing::Failed,
                        | ItemTyping::Holey => Typing::Holey,
                    }
                })
                .collect::<Vec<Typing>>(),
            "resuming across the seam must equal a from-scratch re-type"
        );
        assert!(
            resumed.adopted().any(bool::from),
            "a body-only edit must adopt at least its type-stable dependent, or the seam is \
             carrying too little for the footprint to clear anything"
        );
    }
}
