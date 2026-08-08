//! The parser-agnostic item seam, exercised against the real front end
//! (`incremental-pipeline.md` §§2-3).
//!
//! `gandr_core_checker::region::ItemSource` states the boundary the
//! changed-region detector reads across, and `crate::item_source` is its first
//! implementation that is not a test double. Two things need proving about it,
//! and they are different claims.
//!
//! 1. **The projection is faithful.**
//!    [`tests::the_seam_carries_names_ascriptions_and_order`] checks that the
//!    three fields the unchanged-region test compares — name, ascription, and
//!    source order — survive the crossing, since a projection that dropped any
//!    of them would let the detector adopt an item it must re-type while every
//!    from-scratch check still passed.
//! 2. **The two engines agree.** This crate and `gandr-core-checker` each carry
//!    an item-granular checkpoint engine, and they are near-identical by
//!    descent rather than by construction — neither is defined in terms of the
//!    other, so nothing today makes a fix to one reach the other.
//!    [`tests::the_seam_engine_agrees_with_the_surface_engine`] drives both
//!    over the same source and asserts they agree item for item, which turns a
//!    silent divergence into a failing test for as long as both exist.
//!
//! The second is a guard on a known duplication, not an endorsement of it: the
//! question of which engine survives is on this lane's owner-decision queue.

#![cfg_attr(
    dylint_lib = "non_topologically_sorted_functions",
    allow(
        unknown_lints,
        non_topologically_sorted_functions,
        reason = "integration tests share fixture helpers called from tests in per-test orders; no single module arrangement satisfies every caller-before-callee pair, so the ordering rule is waived in test code"
    )
)]

/// The seam's fidelity and the two engines' agreement.
#[cfg(test)]
mod tests
{
    use alloc::string::String;
    use alloc::vec::Vec;

    use gandr_core_checker::region::ItemSource as _;
    use gandr_core_checker::region::Program;
    use gandr_core_checker::types::Ty;
    use gandr_surface_engine::item_source::LoweringItemSource;
    use gandr_surface_engine::item_source::SourceRevision;
    use gandr_surface_engine::lower::Lowered;
    use gandr_surface_engine::lower::lower_source_total;
    use gandr_surface_engine::prelude_ctx;

    use crate::common::TestText;

    /// The comparable shape of one item's typing, projected out of whichever
    /// engine produced it.
    ///
    /// The two engines return two distinct `ItemTyping` enums — one per crate —
    /// so they cannot be compared directly. Both carry the same underlying
    /// `Ty` and classification, and this is that common shape: the classifier,
    /// the bound name where there is one, and the reported type where the item
    /// typed. A failure carries no payload here because the two crates'
    /// `TypeError` values are the same type but the assertion this test makes
    /// is about agreement of outcome, and an error's identity is already
    /// pinned by each engine's own from-scratch gate.
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
        /// An item of unknown sort — reachable only through the surface
        /// engine's forward-compatible variant, which nothing produces today.
        Unknown,
    }

    /// The surface engine's typings for `source`, from scratch.
    fn surface_typings<'text>(source: impl Into<TestText<'text>>) -> Vec<Typing>
    {
        use gandr_surface_engine::checkpoint::ItemTyping;

        let lowered = lower(source);
        gandr_surface_engine::checkpoint::checkpoint_source(&lowered)
            .items
            .into_iter()
            .map(|checkpoint| match checkpoint.typing {
                | ItemTyping::Definition { name, ty, bound } => {
                    Typing::Definition { name, ty, bound }
                },
                | ItemTyping::Expression { ty } => Typing::Expression { ty },
                | ItemTyping::TypeError { .. } => Typing::Failed,
                | ItemTyping::Holey => Typing::Holey,
                | ItemTyping::Unknown => Typing::Unknown,
            })
            .collect()
    }

    /// The core-checker engine's typings for `program`, from scratch, against
    /// the same prelude context the surface engine uses.
    fn seam_typings(program: &Program) -> Vec<Typing>
    {
        use gandr_core_checker::checkpoint::ItemTyping;

        gandr_core_checker::checkpoint::checkpoint_with(program, &prelude_ctx())
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

    /// The source the agreement runs over: a definition, a type-ascribed
    /// definition reading it, a dependent expression, and an item that fails to
    /// type — so the comparison spans every classifier both engines produce.
    fn corpus() -> Vec<TestText<'static>>
    {
        Vec::from([
            TestText("def target(x: Integer) -> F Integer {\n  ret (x + 1)\n}\nprint(target)\n"),
            TestText("def n = 1;\ndef m = n;\nprint(m)\n"),
            TestText("def s = \"text\";\ndef t = s;\nprint(t)\n"),
            TestText("def n = 1;\ndef bad = n(n);\n"),
            TestText("def n = 1;\n"),
        ])
    }

    /// Both item-granular checkpoint engines classify and type every item of
    /// every corpus source identically. They are separate near-identical
    /// implementations, so this is the only thing in the tree that would fail
    /// if one drifted from the other.
    #[test]
    fn the_seam_engine_agrees_with_the_surface_engine()
    {
        for source in corpus() {
            let text = source.0;
            let program = seam_program(source);
            let seam = seam_typings(&program);
            assert!(
                !seam.is_empty(),
                "{text:?} must lower to at least one item, or the agreement below is vacuous"
            );
            assert_eq!(
                seam,
                surface_typings(source),
                "the core-checker and surface-engine checkpoint engines must agree on {text:?}"
            );
        }
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
    /// reproduces a from-scratch re-type — the differential gate, run for the
    /// first time against the real front end rather than a test double.
    #[test]
    fn the_seam_admits_the_differential_gate()
    {
        let base = "def target(x: Integer) -> F Integer {\n  ret (x + 1)\n}\nprint(target)\n";
        let edited = "def target(x: Integer) -> F Integer {\n  ret (x + 2)\n}\nprint(target)\n";

        let checkpoints =
            gandr_core_checker::checkpoint::checkpoint_with(&seam_program(base), &prelude_ctx());
        let resumed = gandr_core_checker::checkpoint::resume_with(
            &checkpoints,
            &seam_program(edited),
            &prelude_ctx(),
        );

        assert_eq!(
            seam_typings(&seam_program(edited)),
            resumed
                .typings
                .into_iter()
                .map(|typing| {
                    use gandr_core_checker::checkpoint::ItemTyping;

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
            resumed.adopted.iter().any(|adopted| *adopted),
            "a body-only edit must adopt at least its type-stable dependent, or the seam is \
             carrying too little for the footprint to clear anything"
        );
    }
}
