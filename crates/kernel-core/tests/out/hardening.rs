//! Adversarial-depth totality (gandr-i3i, gandr-5t3): under the D1(C) arena the
//! term/type node children are `Copy` ids, so a whole [`TermArena`] tears down
//! in flat `Vec` drops — no recursive drop glue. These witnesses build ~1M-deep
//! chains **iteratively** into an arena inside a small-stack thread and confirm
//! that **construction**, **checking**, **decoding**, and **teardown** stay
//! total where the retired recursive owned-tree glue would overflow.
//!
//! A checker/decoder that regressed to input-scaled recursion, or a node that
//! regained an owned `Box` child (reviving recursive drop glue), is a
//! deterministic stack overflow here rather than a silent pass on a large main
//! stack.

/// The deep construction, check, decode, and teardown totality witnesses.
#[cfg(test)]
mod tests
{
    use std::thread;

    use gandr_kernel_core::CompTypeId;
    use gandr_kernel_core::ComputationId;
    use gandr_kernel_core::Environment;
    use gandr_kernel_core::KernelError;
    use gandr_kernel_core::LevelSignature;
    use gandr_kernel_core::TermArena;
    use gandr_kernel_core::ValueId;
    use gandr_kernel_core::ValueTypeId;
    use gandr_kernel_core::decode;
    use gandr_kernel_core::write;

    /// Requested depth of one iterative hardening fixture.
    #[repr(transparent)]
    #[derive(Clone, Copy)]
    struct FixtureDepth(usize);

    /// A chain depth past any plausible recursive-glue stack budget. Built
    /// iteratively (proptest never reaches this depth).
    const CHAIN_DEPTH: FixtureDepth = FixtureDepth(1_000_000);

    /// A smaller depth for the multi-pass decode witness (write, decode, and
    /// the canonical re-encode each walk the whole tree).
    const DECODE_DEPTH: FixtureDepth = FixtureDepth(200_000);

    /// A depth for the checker-totality witnesses (gandr-98o): the machine
    /// descends this far, well past the retired 512 depth budget.
    const CHECK_DEPTH: FixtureDepth = FixtureDepth(200_000);

    /// A deliberately small worker-thread stack: recursive glue overflows it
    /// well before the chain depth; the flat arena teardown does not.
    const SMALL_STACK: usize = 512 * 1024;

    /// Run `work` in a thread with a small stack, propagating a panic or
    /// overflow.
    fn in_small_stack<F>(work: F)
    where
        F: FnOnce() + Send + 'static,
    {
        thread::Builder::new()
            .stack_size(SMALL_STACK)
            .spawn(work)
            .expect("the worker thread spawns")
            .join()
            .expect("the worker thread does not overflow or panic");
    }

    /// A left-nested pair chain of the given depth, minted into `arena`.
    fn deep_value(
        arena: &mut TermArena,
        depth: FixtureDepth,
    ) -> ValueId
    {
        let mut value = arena.value_unit();
        for _step in 0 .. depth.0 {
            let unit = arena.value_unit();
            value = arena.value_pair(value, unit);
        }
        value
    }

    /// A left-nested bind chain of the given depth (each step wraps the prior
    /// computation as the bound sub-computation), minted into `arena`.
    fn deep_computation(
        arena: &mut TermArena,
        depth: FixtureDepth,
    ) -> ComputationId
    {
        let unit = arena.value_unit();
        let mut computation = arena.computation_return(unit);
        for _step in 0 .. depth.0 {
            let inner_unit = arena.value_unit();
            let inner_return = arena.computation_return(inner_unit);
            computation = arena.computation_bind(computation, inner_return);
        }
        computation
    }

    /// A left-nested product chain of the given depth, minted into `arena`.
    fn deep_value_type(
        arena: &mut TermArena,
        depth: FixtureDepth,
    ) -> ValueTypeId
    {
        let mut value_type = arena.value_type_unit();
        for _step in 0 .. depth.0 {
            let unit = arena.value_type_unit();
            value_type = arena.value_type_product(value_type, unit);
        }
        value_type
    }

    /// An arrow chain of the given depth (each step nests the prior type as the
    /// codomain), minted into `arena`.
    fn deep_comp_type(
        arena: &mut TermArena,
        depth: FixtureDepth,
    ) -> CompTypeId
    {
        let unit = arena.value_type_unit();
        let mut comp_type = arena.comp_type_returner(unit);
        for _step in 0 .. depth.0 {
            let domain = arena.value_type_unit();
            comp_type = arena.comp_type_arrow(domain, comp_type);
        }
        comp_type
    }

    #[test]
    fn deep_value_arena_teardown_is_total()
    {
        in_small_stack(|| {
            let mut arena = TermArena::new();
            let _root = deep_value(&mut arena, CHAIN_DEPTH);
            drop(arena);
        });
    }

    #[test]
    fn deep_computation_arena_teardown_is_total()
    {
        in_small_stack(|| {
            let mut arena = TermArena::new();
            let _root = deep_computation(&mut arena, CHAIN_DEPTH);
            drop(arena);
        });
    }

    #[test]
    fn deep_value_type_arena_teardown_is_total()
    {
        in_small_stack(|| {
            let mut arena = TermArena::new();
            let _root = deep_value_type(&mut arena, CHAIN_DEPTH);
            drop(arena);
        });
    }

    #[test]
    fn deep_comp_type_arena_teardown_is_total()
    {
        in_small_stack(|| {
            let mut arena = TermArena::new();
            let _root = deep_comp_type(&mut arena, CHAIN_DEPTH);
            drop(arena);
        });
    }

    #[test]
    fn deep_error_payload_drop_is_total()
    {
        // A deep declared type with a mismatching (unit) body: the checker's
        // conversion builds a `ValueTypeMismatch` carrying an arena SNAPSHOT of
        // the deep type, reified iteratively; dropping the error tears the
        // snapshot down flatly (the owned-tree payload would overflow).
        in_small_stack(|| {
            let mut environment = Environment::new();
            let mut builder = environment.stage();
            let declared = deep_value_type(builder.arena(), CHECK_DEPTH);
            let body = builder.arena().value_unit();
            let declaration = builder.def(LevelSignature::monomorphic(), declared, body);
            let error = environment
                .add_decl(declaration)
                .expect_err("unit does not inhabit a deep product");
            assert!(
                matches!(error, KernelError::ValueTypeMismatch(_)),
                "a deep type mismatch is the reified-snapshot payload"
            );
            drop(error);
        });
    }

    #[test]
    fn deep_decoded_declaration_drop_is_total()
    {
        // A deep body admitted through the bypass, serialized, and decoded back
        // into a `DecodedArtifact` — dropped without ever being checked, the
        // exact i3i hazard path (decode builds a deep arena, the caller
        // discards it).
        in_small_stack(|| {
            let mut environment = Environment::new();
            let mut builder = environment.stage();
            let declared = builder.arena().value_type_unit();
            let body = deep_value(builder.arena(), DECODE_DEPTH);
            let declaration = builder.def(LevelSignature::monomorphic(), declared, body);
            let _id = environment.add_decl_unchecked(declaration);
            let bytes = write(&environment);
            let decoded = decode(bytes.as_ref().into()).expect("the deep artifact decodes");
            assert_eq!(1, decoded.declarations().len(), "one declaration decodes");
            drop(decoded);
        });
    }

    #[test]
    fn a_deep_pair_definition_checks_totally()
    {
        // A ~200k-deep well-typed pair : product admits inside a small-stack
        // thread where the retired recursive descent overflows (gandr-98o).
        in_small_stack(|| {
            let mut environment = Environment::new();
            let mut builder = environment.stage();
            let declared = deep_value_type(builder.arena(), CHECK_DEPTH);
            let body = deep_value(builder.arena(), CHECK_DEPTH);
            let declaration = builder.def(LevelSignature::monomorphic(), declared, body);
            assert!(
                environment.add_decl(declaration).is_ok(),
                "the deep well-typed pair definition admits totally"
            );
        });
    }

    #[test]
    fn a_deep_bind_definition_checks_totally()
    {
        // A ~200k-deep bind chain checked against U (F Unit): the machine's
        // explicit typing-context stack grows and unwinds 200k deep through
        // scope-exit frames without overflowing the small thread stack.
        in_small_stack(|| {
            let mut environment = Environment::new();
            let mut builder = environment.stage();
            let arena = builder.arena();
            // declared = U (F Unit).
            let unit_type = arena.value_type_unit();
            let returner = arena.comp_type_returner(unit_type);
            let declared = arena.value_type_thunk(returner);
            // body = thunk (bind _ <- return unit; ...; return unit).
            let inner_unit = arena.value_unit();
            let mut chain = arena.computation_return(inner_unit);
            for _step in 0 .. CHECK_DEPTH.0 {
                let bound_unit = arena.value_unit();
                let bound = arena.computation_return(bound_unit);
                chain = arena.computation_bind(bound, chain);
            }
            let body = arena.value_thunk(chain);
            let declaration = builder.def(LevelSignature::monomorphic(), declared, body);
            assert!(
                environment.add_decl(declaration).is_ok(),
                "the deep bind definition admits totally"
            );
        });
    }
}
