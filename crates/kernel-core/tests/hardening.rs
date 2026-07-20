//! Adversarial-depth totality (gandr-i3i): the worklist `Drop` and iterative
//! `Clone` of the owned term/type trees survive ~1M-deep chains inside a
//! small-stack thread where the recursive glue they replace would overflow.
//!
//! Each chain is built ITERATIVELY (proptest never reaches this depth); the
//! work under test — dropping, cloning, or decoding it — runs inside a
//! `std::thread::Builder` thread with a small stack, so a regression to
//! recursive glue is a deterministic stack overflow rather than a silent pass
//! on a large main stack. A clone is checked by an ITERATIVE spine-depth walk
//! (the derived recursive `PartialEq` would itself overflow), so a shallow or
//! structurally wrong clone is caught, not just an overflow.

#![cfg_attr(
    test,
    allow(
        clippy::arithmetic_side_effects,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic,
        clippy::unwrap_used,
        reason = "the standard test-allow set keeps the deep-totality tests readable (docs/workflow/rust.md)"
    )
)]

/// The deep-drop, deep-clone, and indirect-drop totality witnesses.
#[cfg(test)]
mod tests
{
    use std::thread;

    use gandr_kernel_core::CompType;
    use gandr_kernel_core::Computation;
    use gandr_kernel_core::Declaration;
    use gandr_kernel_core::Environment;
    use gandr_kernel_core::ExpectedValueShape;
    use gandr_kernel_core::KernelError;
    use gandr_kernel_core::LevelSignature;
    use gandr_kernel_core::Value;
    use gandr_kernel_core::ValueType;
    use gandr_kernel_core::decode;
    use gandr_kernel_core::write;

    /// A chain depth past any plausible recursive-glue stack budget. It is
    /// built iteratively (proptest never reaches this depth).
    const CHAIN_DEPTH: usize = 1_000_000;

    /// A smaller depth for the multi-pass decode witness (write, decode, and
    /// the canonical re-encode each walk the whole tree), still far past
    /// the recursive-overflow point.
    const DECODE_DEPTH: usize = 200_000;

    /// A deliberately small worker-thread stack: recursive glue overflows it
    /// well before the chain depth, the iterative worklist does not.
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

    /// A left-nested pair chain of the given depth (each step wraps the prior
    /// value).
    fn deep_value(depth: usize) -> Value
    {
        let mut value = Value::Unit;
        for _step in 0 .. depth {
            value = Value::Pair(Box::new(value), Box::new(Value::Unit));
        }
        value
    }

    /// A left-nested bind chain of the given depth (each step wraps the prior
    /// computation as the bound sub-computation).
    fn deep_computation(depth: usize) -> Computation
    {
        let mut computation = Computation::Return(Box::new(Value::Unit));
        for _step in 0 .. depth {
            computation = Computation::Bind(
                Box::new(computation),
                Box::new(Computation::Return(Box::new(Value::Unit))),
            );
        }
        computation
    }

    /// A left-nested product chain of the given depth.
    fn deep_value_type(depth: usize) -> ValueType
    {
        let mut value_type = ValueType::Unit;
        for _step in 0 .. depth {
            value_type = ValueType::Product(Box::new(value_type), Box::new(ValueType::Unit));
        }
        value_type
    }

    /// An arrow chain of the given depth (each step nests the prior type as the
    /// codomain).
    fn deep_comp_type(depth: usize) -> CompType
    {
        let mut comp_type = CompType::Returner(Box::new(ValueType::Unit));
        for _step in 0 .. depth {
            comp_type = CompType::Arrow {
                domain: Box::new(ValueType::Unit),
                codomain: Box::new(comp_type),
            };
        }
        comp_type
    }

    /// The left-spine depth of a pair chain, walked iteratively.
    #[expect(
        clippy::pattern_type_mismatch,
        reason = "ergonomic walk of a borrowed value spine; each binding is a shared reference by intent"
    )]
    fn value_spine_depth(root: &Value) -> usize
    {
        let mut depth: usize = 0;
        let mut cursor = root;
        while let Value::Pair(first, _second) = cursor {
            depth += 1;
            cursor = first.as_ref();
        }
        depth
    }

    /// The spine depth of a bind chain, walked iteratively.
    #[expect(
        clippy::pattern_type_mismatch,
        reason = "ergonomic walk of a borrowed computation spine; each binding is a shared reference by intent"
    )]
    fn comp_spine_depth(root: &Computation) -> usize
    {
        let mut depth: usize = 0;
        let mut cursor = root;
        while let Computation::Bind(bound, _body) = cursor {
            depth += 1;
            cursor = bound.as_ref();
        }
        depth
    }

    /// The left-spine depth of a product chain, walked iteratively.
    #[expect(
        clippy::pattern_type_mismatch,
        reason = "ergonomic walk of a borrowed value-type spine; each binding is a shared reference by intent"
    )]
    fn value_type_spine_depth(root: &ValueType) -> usize
    {
        let mut depth: usize = 0;
        let mut cursor = root;
        while let ValueType::Product(first, _second) = cursor {
            depth += 1;
            cursor = first.as_ref();
        }
        depth
    }

    /// The spine depth of an arrow chain (walked through the codomain),
    /// iteratively.
    #[expect(
        clippy::pattern_type_mismatch,
        reason = "ergonomic walk of a borrowed computation-type spine; each binding is a shared reference by intent"
    )]
    fn comp_type_spine_depth(root: &CompType) -> usize
    {
        let mut depth: usize = 0;
        let mut cursor = root;
        while let CompType::Arrow { codomain, .. } = cursor {
            depth += 1;
            cursor = codomain.as_ref();
        }
        depth
    }

    #[test]
    fn deep_value_drop_is_total()
    {
        in_small_stack(|| {
            let value = deep_value(CHAIN_DEPTH);
            drop(value);
        });
    }

    #[test]
    fn deep_computation_drop_is_total()
    {
        in_small_stack(|| {
            let computation = deep_computation(CHAIN_DEPTH);
            drop(computation);
        });
    }

    #[test]
    fn deep_value_type_drop_is_total()
    {
        in_small_stack(|| {
            let value_type = deep_value_type(CHAIN_DEPTH);
            drop(value_type);
        });
    }

    #[test]
    fn deep_comp_type_drop_is_total()
    {
        in_small_stack(|| {
            let comp_type = deep_comp_type(CHAIN_DEPTH);
            drop(comp_type);
        });
    }

    #[test]
    fn deep_value_clone_is_total()
    {
        in_small_stack(|| {
            let value = deep_value(CHAIN_DEPTH);
            let cloned = value.clone();
            assert_eq!(
                value_spine_depth(&cloned),
                CHAIN_DEPTH,
                "the clone preserves the chain depth"
            );
            assert_eq!(
                value_spine_depth(&value),
                value_spine_depth(&cloned),
                "the clone matches the source depth"
            );
            drop(cloned);
            drop(value);
        });
    }

    #[test]
    fn deep_computation_clone_is_total()
    {
        in_small_stack(|| {
            let computation = deep_computation(CHAIN_DEPTH);
            let cloned = computation.clone();
            assert_eq!(
                comp_spine_depth(&cloned),
                CHAIN_DEPTH,
                "the clone preserves the chain depth"
            );
            drop(cloned);
            drop(computation);
        });
    }

    #[test]
    fn deep_value_type_clone_is_total()
    {
        in_small_stack(|| {
            let value_type = deep_value_type(CHAIN_DEPTH);
            let cloned = value_type.clone();
            assert_eq!(
                value_type_spine_depth(&cloned),
                CHAIN_DEPTH,
                "the clone preserves the chain depth"
            );
            drop(cloned);
            drop(value_type);
        });
    }

    #[test]
    fn deep_comp_type_clone_is_total()
    {
        in_small_stack(|| {
            let comp_type = deep_comp_type(CHAIN_DEPTH);
            let cloned = comp_type.clone();
            assert_eq!(
                comp_type_spine_depth(&cloned),
                CHAIN_DEPTH,
                "the clone preserves the chain depth"
            );
            drop(cloned);
            drop(comp_type);
        });
    }

    #[test]
    fn deep_error_payload_drop_is_total()
    {
        // A `KernelError` boxing a deep value type: dropping the error is a deep
        // drop through the error payload (the checker's mismatch arms box types).
        in_small_stack(|| {
            let error = KernelError::ValueShapeMismatch {
                expected: ExpectedValueShape::Thunk,
                actual: Box::new(deep_value_type(CHAIN_DEPTH)),
            };
            drop(error);
        });
    }

    #[test]
    fn deep_decoded_declaration_drop_is_total()
    {
        // A deep body admitted through the bypass, serialized, and decoded back
        // into an owned `DecodedDeclaration` — dropped without ever being checked,
        // the exact i3i hazard path (decode builds a deep tree, the caller
        // discards it).
        in_small_stack(|| {
            let mut environment = Environment::new();
            let _id = environment.add_decl_unchecked(Declaration::def(
                LevelSignature::monomorphic(),
                ValueType::Unit,
                deep_value(DECODE_DEPTH),
            ));
            let bytes = write(&environment);
            let decoded = decode(&bytes).expect("the deep artifact decodes");
            assert_eq!(decoded.len(), 1, "one declaration decodes");
            drop(decoded);
        });
    }
}
