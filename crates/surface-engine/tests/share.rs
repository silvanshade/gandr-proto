#![allow(
    unknown_lints,
    reason = "The local dylint policy is unavailable to rustc outside its owning check."
)]
//! Sharing-overlay integration witnesses: the erasure fold's output against
//! the existing unshared checking path, and the export pipeline's
//! indifference to how that spelling was produced.

#![cfg_attr(
    dylint_lib = "non_topologically_sorted_functions",
    allow(
        unknown_lints,
        non_topologically_sorted_functions,
        reason = "integration tests share fixture helpers called from tests in per-test orders; no single module arrangement satisfies every caller-before-callee pair, so the ordering rule is waived in test code pending a test-layout redesign"
    )
)]

/// Sharing-overlay integration witnesses.
#[cfg(test)]
mod tests
{
    use alloc::rc::Rc;
    use alloc::string::String;
    use alloc::vec;

    use gandr_core_checker::syntax::Term;
    use gandr_core_checker::syntax::Value;
    use gandr_core_checker::types::Ty;
    use gandr_core_checker::types::ValueType;
    use gandr_kernel_core::read;
    use gandr_kernel_core::write;
    use gandr_surface_engine::boundary::DefinitionName;
    use gandr_surface_engine::kernel::DefinitionOffer;
    use gandr_surface_engine::kernel::KernelAdmissions;
    use gandr_surface_engine::kernel::KernelVerdict;
    use gandr_surface_engine::share::AnyShareId;
    use gandr_surface_engine::share::Arity;
    use gandr_surface_engine::share::BinderDistance;
    use gandr_surface_engine::share::Bound;
    use gandr_surface_engine::share::Graft;
    use gandr_surface_engine::share::SeamIndex;
    use gandr_surface_engine::share::ShareArena;
    use gandr_surface_engine::share::ShareId;
    use gandr_surface_engine::share::Sharing;
    use gandr_surface_engine::share::VectorPosition;
    use gandr_surface_engine::share::erase_value;
    use gandr_surface_engine::share::seam_name;
    use gandr_surface_engine::share::validate;

    #[expect(
        primitive_signature,
        reason = "Test fixtures construct semantic values from compact primitive witnesses."
    )]
    fn bound(
        distance: u32,
        position: u32,
    ) -> Bound
    {
        Bound {
            distance: BinderDistance::from(distance),
            position: VectorPosition::from(position),
        }
    }

    #[expect(
        primitive_signature,
        reason = "Test fixtures construct semantic values from compact primitive witnesses."
    )]
    fn seam(index: u32) -> String
    {
        seam_name(SeamIndex::from(index))
    }

    /// The share-carrying `(3, 3)`: one shared literal bound at arity two.
    fn share_carrying_pair() -> (ShareArena, ShareId<Value>)
    {
        let mut arena = ShareArena::new();
        let shared = arena
            .value_opaque(Value::int(3))
            .expect("mint value opaque");
        let first = arena.value_bound(bound(0, 0)).expect("mint value bound");
        let second = arena.value_bound(bound(0, 1)).expect("mint value bound");
        let graft = arena
            .value_graft(Graft {
                template: Value::pair(Value::Var(seam(0)), Value::Var(seam(1))),
                children: vec![AnyShareId::from(first), AnyShareId::from(second)],
            })
            .expect("mint value graft");
        let root = arena
            .value_share(Sharing {
                arity: Arity::from(2),
                shared: AnyShareId::from(shared),
                body: graft,
            })
            .expect("mint value share");
        (arena, root)
    }

    /// The erasure of a share-carrying value and the ordinary unshared
    /// spelling are the same term, and the kernel's verdict on the two is
    /// identical — the whole point of the overlay's erasure discipline.
    #[test]
    fn a_share_carrying_value_checks_identically_to_its_unshared_spelling()
    {
        let (arena, root) = share_carrying_pair();
        assert_eq!(validate(&arena, AnyShareId::from(root)), Ok(()));
        let erased = erase_value(&arena, root).expect("the overlay erases");
        let unshared = Value::pair(Value::int(3), Value::int(3));
        assert_eq!(
            erased, unshared,
            "the fold produces exactly the unshared spelling"
        );
        let ty = Ty::Value(ValueType::Prod(
            Rc::new(ValueType::integer()),
            Rc::new(ValueType::integer()),
        ));
        let mut shared_ledger = KernelAdmissions::new();
        let shared_verdict = shared_ledger.offer(DefinitionOffer {
            name: DefinitionName::from("twice"),
            term: &Term::Value(erased),
            ty: &ty,
        });
        let mut plain_ledger = KernelAdmissions::new();
        let plain_verdict = plain_ledger.offer(DefinitionOffer {
            name: DefinitionName::from("twice"),
            term: &Term::Value(unshared),
            ty: &ty,
        });
        assert!(
            matches!(shared_verdict, KernelVerdict::Admitted { .. }),
            "the share-carrying spelling admits: {shared_verdict:?}"
        );
        assert_eq!(
            shared_verdict, plain_verdict,
            "the kernel cannot tell the erasure and the unshared spelling apart"
        );
        let bytes = write(shared_ledger.environment());
        let reread = read(bytes.as_ref().into()).expect("the admitted environment replays");
        assert_eq!(
            bytes,
            write(&reread),
            "the admitted environment round-trips byte-identically"
        );
    }
}
