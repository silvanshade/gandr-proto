//! Context-frame coverage (`src/diag.rs::frame_description`): a type error
//! nested at each checker frame position surfaces that frame in the
//! diagnostic's `context_chain`, so every frame's `(role, prose, binder)`
//! description is exercised. Each program plants an unbound variable (`zzz`) at
//! a distinct structural position; the enclosing frames localize the failure.

#![cfg_attr(
    dylint_lib = "non_topologically_sorted_functions",
    allow(
        unknown_lints,
        non_topologically_sorted_functions,
        reason = "integration tests share fixture helpers called from tests in per-test orders; no single module arrangement satisfies every caller-before-callee pair, so the ordering rule is waived in test code pending a test-layout redesign"
    )
)]

#[cfg(test)]
mod tests
{
    use gandr_surface_engine::diag::ContextFrame;
    use gandr_surface_engine::diag::report;
    use gandr_surface_engine::lower::lower_source_total;
    use gandr_surface_engine::prelude_ctx;

    use crate::TestDecision;
    use crate::TestText;
    #[test]
    fn each_checker_frame_localizes_its_nested_failure()
    {
        // Every frame role, with a program that plants an unbound `zzz` (or an
        // ill-typed scrutinee) at that structural position.
        let cases: &[(&str, &str)] = &[
            ("Abs", "def f = fn(x: Integer) { ret zzz };"),
            ("Ret", "def f = fn(x: Integer) { ret zzz };"),
            ("Annot", "def d = (Inl(zzz) : Integer + Integer);"),
            ("PairFst", "def d = ((zzz, 2) : Integer * Integer);"),
            ("PairSnd", "def d = ((1, zzz) : Integer * Integer);"),
            ("Inj", "def d = (Inl(zzz) : Integer + Integer);"),
            ("Thunk", "def d = (thunk { ret zzz } : U (F Integer));"),
            ("Force", "def d = ((force zzz) : F Integer);"),
            ("Bind", "def d = thunk { run x <- ret zzz; ret x };"),
            ("BindBody", "def d = thunk { run x <- ret 1; ret zzz };"),
            (
                "CaseScrut",
                "def d = (case zzz { Inl(x) => ret 1, Inr(y) => ret 2 } : F Integer);",
            ),
            (
                "CaseArm1",
                "def d = (case (Inl(1) : Integer + Integer) { Inl(x) => ret zzz, Inr(y) => ret y } : F \
                 Integer);",
            ),
            (
                "CaseArm2",
                "def d = (case (Inl(1) : Integer + Integer) { Inl(x) => ret x, Inr(y) => ret zzz } : F \
                 Integer);",
            ),
            // A split is check-only without a motive (rule Split⇓, the split rule), so
            // both frame cases ascribe the thunk to reach the split in checking
            // position (a bare inferred split is stuck-needs-motive before its
            // `Frame::Split` is pushed).
            (
                "Split",
                "def d = (thunk { val (a, b) = zzz; ret 1 } : U (F Integer));",
            ),
            (
                "SplitBody",
                "def d = (thunk { val (a, b) = (1, 2); ret zzz } : U (F Integer));",
            ),
            (
                "With1",
                "def d = (co { fst = ret zzz, snd = ret 2 } : F Integer & F Integer);",
            ),
            (
                "With2",
                "def d = (co { fst = ret 1, snd = ret zzz } : F Integer & F Integer);",
            ),
            ("Prj", "def d = ((1, 2).fst);"),
            ("AppFn", "def d = (zzz(1) : F Integer);"),
            (
                "AppArg",
                "def g(f: U (Integer -> F Integer)) { (force f)(zzz) };",
            ),
        ];

        for &(role, source) in cases {
            assert!(
                bool::from(has_role(source, role)),
                "`{source}` must localize its failure in a `{role}` frame, saw {:?}",
                frames(source)
                    .iter()
                    .map(|frame| frame.role.clone())
                    .collect::<Vec<_>>()
            );
        }
    }

    /// Whether some diagnostic frame carries `role` with a non-empty prose.
    fn has_role<'source, 'role>(
        source: impl Into<TestText<'source>>,
        role: impl Into<TestText<'role>>,
    ) -> TestDecision
    {
        let source = source.into().0;
        let role = role.into().0;
        frames(source)
            .iter()
            .any(|frame| frame.role == role && !frame.prose.is_empty())
            .into()
    }
    #[test]
    fn binder_naming_frames_carry_their_binder()
    {
        // The `Abs` frame records the abstraction's binder; the `Bind` frame the
        // `<-` binder. The prose embeds the name and the `binder` field carries it.
        let abs = frames("def f = fn(item: Integer) { ret zzz };");
        assert!(
            abs.iter().any(|frame| frame.role == "Abs"
                && frame.binder.as_deref() == Some("item")
                && frame.prose.contains("item")),
            "the Abs frame names its binder: {abs:?}"
        );
        let bind = frames("def d = thunk { run slot <- ret zzz; ret slot };");
        assert!(
            bind.iter().any(|frame| frame.role == "Bind"
                && frame.binder.as_deref() == Some("slot")
                && frame.prose.contains("slot")),
            "the Bind frame names its `<-` binder: {bind:?}"
        );
    }

    /// The context frames of every diagnostic of a totally-lowered source.
    fn frames<'text>(source: impl Into<TestText<'text>>) -> Vec<ContextFrame>
    {
        let source = source.into().0;
        let lowered = lower_source_total(source.into()).expect("total lowering never errs");
        report(&lowered, &prelude_ctx())
            .diagnostics
            .into_iter()
            .flat_map(|diagnostic| diagnostic.context_chain)
            .collect()
    }
}
