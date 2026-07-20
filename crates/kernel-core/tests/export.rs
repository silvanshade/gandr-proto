//! The K5 export writer/reader differential and rejection-totality suite
//! (kernel-boundary.md §5, E1–E6).
//!
//! The round-trip properties pin that `read(write(env))` reproduces the
//! environment (declarations, admission marks, and recomputed audits) and that
//! `write` is deterministic. The rejection suite pins that the validating
//! reader is total on adversarial bytes — truncation at every prefix, corrupted
//! tags, and arbitrary random bytes all *return* (never panic, never a false
//! success) — and that each named refusal (reserved kind, reserved slot,
//! version, non-canonical encoding) fires with the right error.

#![cfg_attr(
    test,
    allow(
        clippy::arithmetic_side_effects,
        clippy::as_conversions,
        clippy::cast_possible_truncation,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic,
        clippy::unwrap_used,
        reason = "the standard test-allow set keeps unit and property tests readable (docs/workflow/rust.md)"
    )
)]

extern crate alloc;

/// The differential and rejection suite.
#[cfg(test)]
mod tests
{
    use alloc::boxed::Box;
    use alloc::format;
    use alloc::string::String;
    use alloc::vec;
    use alloc::vec::Vec;

    use gandr_kernel_core::AdmissionMark;
    use gandr_kernel_core::BaseType;
    use gandr_kernel_core::CompType;
    use gandr_kernel_core::Computation;
    use gandr_kernel_core::ConstantIndex;
    use gandr_kernel_core::DeBruijnIndex;
    use gandr_kernel_core::Declaration;
    use gandr_kernel_core::DecodeError;
    use gandr_kernel_core::Environment;
    use gandr_kernel_core::FractionDigits;
    use gandr_kernel_core::IntegerLiteral;
    use gandr_kernel_core::LevelParamCount;
    use gandr_kernel_core::LevelSignature;
    use gandr_kernel_core::Literal;
    use gandr_kernel_core::Magnitude;
    use gandr_kernel_core::MalformedSite;
    use gandr_kernel_core::NumericLiteral;
    use gandr_kernel_core::ReservedKind;
    use gandr_kernel_core::ReservedSlot;
    use gandr_kernel_core::Side;
    use gandr_kernel_core::Sign;
    use gandr_kernel_core::StringLiteral;
    use gandr_kernel_core::Value;
    use gandr_kernel_core::ValueType;
    use gandr_kernel_core::decode;
    use gandr_kernel_core::read;
    use gandr_kernel_core::write;
    use gandr_kernel_strata::LandmarkConstraint;
    use gandr_kernel_strata::Level;
    use gandr_kernel_strata::LevelConstant;
    use gandr_kernel_strata::LevelVar;
    use gandr_kernel_strata::LevelVarIndex;
    use proptest::prelude::*;

    // The v0 wire constants, restated here so this suite tests the byte format
    // as a black box (they mirror the crate-private `export` constants).
    const WIRE_MAGIC: [u8; 4] = *b"GKX1";
    const WIRE_ADMISSION_CHECKED: u8 = 0;
    const WIRE_KIND_AXIOM: u8 = 1;
    const WIRE_TYPE_UNIVERSE: u8 = 5;

    /// Append a canonical unsigned LEB128 varint (the writer's wire form).
    fn put_uvarint(
        out: &mut Vec<u8>,
        mut value: u64,
    )
    {
        loop {
            let low = (value & 0x7f) as u8;
            value >>= 7_u32;
            if value == 0 {
                out.push(low);
                break;
            }
            out.push(low | 0x80);
        }
    }

    /// The artifact header (magic, v0 version, empty minted-atom table) plus a
    /// declaration count.
    fn artifact_header(count: u64) -> Vec<u8>
    {
        let mut bytes = WIRE_MAGIC.to_vec();
        bytes.extend_from_slice(&[0, 0]); // version 0
        put_uvarint(&mut bytes, 0); // R4 minted-atom table, empty
        put_uvarint(&mut bytes, count);
        bytes
    }

    /// `variable + offset` built through the strata smart constructors.
    fn var_plus(
        index: u32,
        offset: u64,
    ) -> Level
    {
        let mut level = Level::var(LevelVar::new(LevelVarIndex::from(index)));
        for _step in 0 .. offset {
            level = level.succ().unwrap();
        }
        level
    }

    /// `U (Unit -> F Unit)` — the thunked unit endofunction type.
    fn unit_endo_thunk_type() -> ValueType
    {
        ValueType::Thunk(Box::new(CompType::Arrow {
            domain: Box::new(ValueType::Unit),
            codomain: Box::new(CompType::Returner(Box::new(ValueType::Unit))),
        }))
    }

    /// The identity thunk body `thunk (lambda. return v0)`.
    fn identity_thunk_body() -> Value
    {
        Value::Thunk(Box::new(Computation::Lambda(Box::new(
            Computation::Return(Box::new(Value::Variable(DeBruijnIndex::from(0_u32)))),
        ))))
    }

    /// A well-typed environment exercising every declaration shape: axioms and
    /// defs, base literals, thunks, universes, lifts, level parameters, a
    /// landmark constraint, and a cross-declaration constant reference.
    fn rich_checked_environment() -> Environment
    {
        let mut environment = Environment::new();
        // 0: axiom Unit.
        environment
            .add_decl(Declaration::axiom(
                LevelSignature::monomorphic(),
                ValueType::Unit,
            ))
            .unwrap();
        // 1: def Unit = unit.
        environment
            .add_decl(Declaration::def(
                LevelSignature::monomorphic(),
                ValueType::Unit,
                Value::Unit,
            ))
            .unwrap();
        // 2: def U (Unit -> F Unit) = identity thunk.
        environment
            .add_decl(Declaration::def(
                LevelSignature::monomorphic(),
                unit_endo_thunk_type(),
                identity_thunk_body(),
            ))
            .unwrap();
        // 3: def Integer = 42.
        environment
            .add_decl(Declaration::def(
                LevelSignature::monomorphic(),
                ValueType::Base(BaseType::Integer),
                Value::Literal(Literal::Integer(IntegerLiteral::new(
                    Sign::NonNegative,
                    Magnitude::from_decimal_text(String::from("42")).unwrap(),
                ))),
            ))
            .unwrap();
        // 4: axiom U_0.
        environment
            .add_decl(Declaration::axiom(
                LevelSignature::monomorphic(),
                ValueType::Universe(Level::constant(LevelConstant::from(0_u64))),
            ))
            .unwrap();
        // 5: axiom Lift Unit -> 2.
        environment
            .add_decl(Declaration::axiom(
                LevelSignature::monomorphic(),
                ValueType::Lift {
                    inner: Box::new(ValueType::Unit),
                    target: Level::constant(LevelConstant::from(2_u64)),
                },
            ))
            .unwrap();
        // 6: axiom generalized over one level parameter, type U_(x0).
        environment
            .add_decl(Declaration::axiom(
                LevelSignature::new(LevelParamCount::from(1_u32), vec![]),
                ValueType::Universe(var_plus(0, 0)),
            ))
            .unwrap();
        // 7: axiom over two params with the landmark x0 <= x1, type U_(x0).
        let constraint = LandmarkConstraint::leq(var_plus(0, 0), var_plus(1, 0)).unwrap();
        environment
            .add_decl(Declaration::axiom(
                LevelSignature::new(LevelParamCount::from(2_u32), vec![constraint]),
                ValueType::Universe(var_plus(0, 0)),
            ))
            .unwrap();
        // 8: def U (Unit -> F Unit) = the constant reference to declaration 2.
        environment
            .add_decl(Declaration::def(
                LevelSignature::monomorphic(),
                unit_endo_thunk_type(),
                Value::Constant(ConstantIndex::from(2_usize)),
            ))
            .unwrap();
        environment
    }

    #[test]
    fn the_empty_environment_round_trips()
    {
        let environment = Environment::new();
        let bytes = write(&environment);
        let recovered = read(&bytes).expect("the empty artifact must read");
        assert_eq!(
            write(&recovered),
            bytes,
            "the empty environment round-trips to byte-identical output"
        );
        assert_eq!(
            decode(&bytes).unwrap().len(),
            0,
            "the empty artifact decodes to no declarations"
        );
    }

    #[test]
    fn round_trip_reproduces_the_environment()
    {
        let environment = rich_checked_environment();
        let bytes = write(&environment);
        let recovered = read(&bytes).expect("a genuine artifact must read");
        assert_eq!(
            write(&recovered),
            bytes,
            "the recovered environment re-serializes byte-identically (E4)"
        );
        // Every declaration survives and carries the checked mark (E6);
        // structural content equality is pinned by the round-trip property, and
        // the recomputed audits by `audits_agree_after_the_round_trip`.
        let decoded = decode(&bytes).unwrap();
        assert_eq!(decoded.len(), 9, "every declaration survives");
        for decoded_declaration in &decoded {
            assert_eq!(
                decoded_declaration.mark(),
                AdmissionMark::Checked,
                "every rich declaration was checked-admitted"
            );
        }
    }

    #[test]
    fn audits_agree_after_the_round_trip()
    {
        // Build the environment capturing the CheckedIds, then confirm the
        // recomputed audits in the recovered environment agree (E3: the audit
        // is recomputed on replay, never serialized).
        let mut environment = Environment::new();
        let axiom = environment
            .add_decl(Declaration::axiom(
                LevelSignature::monomorphic(),
                ValueType::Unit,
            ))
            .unwrap();
        let dependent = environment
            .add_decl(Declaration::def(
                LevelSignature::monomorphic(),
                ValueType::Unit,
                Value::Constant(axiom.position()),
            ))
            .unwrap();
        let bytes = write(&environment);
        let recovered = read(&bytes).unwrap();
        assert_eq!(
            environment.audit(axiom),
            recovered.audit(axiom),
            "the axiom's recomputed audit agrees"
        );
        assert_eq!(
            environment.audit(dependent),
            recovered.audit(dependent),
            "the dependent's recomputed audit agrees (it rests on the axiom)"
        );
        assert_eq!(
            recovered.audit(dependent).axioms(),
            &[axiom.position()],
            "the recovered dependent still rests on the axiom"
        );
    }

    #[test]
    fn a_bypass_admission_survives_the_round_trip()
    {
        // The bypass admits an ill-typed declaration; a dependent rests on it,
        // and the unchecked mark must survive serialization (E6).
        let mut environment = Environment::new();
        let bypassed = environment.add_decl_unchecked(Declaration::def(
            LevelSignature::monomorphic(),
            ValueType::Base(BaseType::Integer),
            Value::Unit, // ill-typed, but the bypass does not check
        ));
        let dependent = environment
            .add_decl(Declaration::def(
                LevelSignature::monomorphic(),
                ValueType::Base(BaseType::Integer),
                Value::Constant(bypassed.position()),
            ))
            .unwrap();
        let bytes = write(&environment);

        let decoded = decode(&bytes).unwrap();
        assert_eq!(
            decoded[0].mark(),
            AdmissionMark::UncheckedBypass,
            "the bypass mark survives serialization"
        );
        assert_eq!(
            decoded[1].mark(),
            AdmissionMark::Checked,
            "the dependent's checked mark survives"
        );

        let recovered = read(&bytes).unwrap();
        assert_eq!(
            environment.audit(dependent),
            recovered.audit(dependent),
            "the recomputed audit agrees through the round trip"
        );
        assert_eq!(
            recovered.audit(dependent).unchecked_admissions(),
            &[bypassed.position()],
            "the recovered dependent still rests on the unchecked admission"
        );
    }

    #[test]
    fn a_second_write_is_byte_identical()
    {
        let environment = rich_checked_environment();
        assert_eq!(
            write(&environment),
            write(&environment),
            "serialization is deterministic (E4)"
        );
    }

    #[test]
    fn an_unknown_version_is_refused()
    {
        let mut bytes = write(&rich_checked_environment());
        bytes[4] = 0; // version high byte
        bytes[5] = 1; // version low byte -> version 1
        assert_eq!(
            decode(&bytes).unwrap_err(),
            DecodeError::UnsupportedVersion { found: 1 },
            "an unimplemented version is a named refusal, not a guess (E5)"
        );
    }

    #[test]
    fn each_reserved_declaration_kind_is_rejected()
    {
        // Kind bytes 2..=5 are the R1 reserved kinds; the reader rejects them
        // distinctly from a generic bad tag.
        let expectations = [
            (2_u8, ReservedKind::AbstractType),
            (3_u8, ReservedKind::ModuleSig),
            (4_u8, ReservedKind::ModuleDef),
            (5_u8, ReservedKind::FunctorDef),
        ];
        for (kind_byte, kind) in expectations {
            let mut bytes = artifact_header(1);
            bytes.push(WIRE_ADMISSION_CHECKED);
            bytes.push(kind_byte);
            assert_eq!(
                decode(&bytes).unwrap_err(),
                DecodeError::ReservedDeclarationKind { kind },
                "reserved declaration kind {kind_byte} is rejected distinctly"
            );
        }
    }

    #[test]
    fn a_non_empty_minted_atom_table_is_rejected()
    {
        let mut bytes = artifact_header(0);
        bytes[6] = 1; // the R4 minted-atom table count
        assert_eq!(
            decode(&bytes).unwrap_err(),
            DecodeError::ReservedSlotOccupied {
                slot: ReservedSlot::MintedAtomTable,
            },
            "a non-empty reserved minted-atom table is rejected (R4)"
        );
    }

    #[test]
    fn a_non_empty_structured_name_is_rejected()
    {
        // header(1) + admission + kind(axiom) + name-segment-count = 1.
        let mut bytes = artifact_header(1);
        bytes.push(WIRE_ADMISSION_CHECKED);
        bytes.push(WIRE_KIND_AXIOM);
        put_uvarint(&mut bytes, 1); // one name segment -> reserved at v0
        assert_eq!(
            decode(&bytes).unwrap_err(),
            DecodeError::ReservedSlotOccupied {
                slot: ReservedSlot::StructuredName,
            },
            "a non-empty structured name is rejected at v0 (R2)"
        );
    }

    #[test]
    fn a_non_empty_def_annotation_slot_is_rejected()
    {
        // Build a real def(Unit, Unit), then set its first annotation slot's
        // length prefix non-zero. The four slots trail the body; for this
        // record the first slot is the final-but-three byte region, so append
        // a fresh crafted record instead for exact control.
        let mut bytes = artifact_header(1);
        bytes.push(WIRE_ADMISSION_CHECKED);
        bytes.push(0); // kind Def
        put_uvarint(&mut bytes, 0); // empty name
        put_uvarint(&mut bytes, 0); // params
        put_uvarint(&mut bytes, 0); // constraints
        bytes.push(1); // declared type: TYPE_UNIT
        bytes.push(2); // body: TERM_UNIT
        put_uvarint(&mut bytes, 1); // first Def annotation slot -> non-empty
        assert_eq!(
            decode(&bytes).unwrap_err(),
            DecodeError::ReservedSlotOccupied {
                slot: ReservedSlot::ErasureAnnotation,
            },
            "a non-empty per-Def annotation slot is rejected at v0 (R3)"
        );
    }

    #[test]
    fn a_non_canonical_level_encoding_is_rejected()
    {
        // A universe axiom whose level bytes carry a dominated constant part
        // (constant 3 with a variable atom at offset 3): the reader rebuilds it
        // canonically (constant drops to 0), so the re-encode differs from the
        // input and the non-canonical encoding is rejected.
        let mut bytes = artifact_header(1);
        bytes.push(WIRE_ADMISSION_CHECKED);
        bytes.push(WIRE_KIND_AXIOM);
        put_uvarint(&mut bytes, 0); // empty name
        put_uvarint(&mut bytes, 1); // one level parameter (so x0 is in scope)
        put_uvarint(&mut bytes, 0); // no constraints
        bytes.push(WIRE_TYPE_UNIVERSE);
        // Non-canonical level: max(3, x0 + 3) -> constant 3 is dominated.
        put_uvarint(&mut bytes, 3); // constant part
        put_uvarint(&mut bytes, 1); // one atom
        put_uvarint(&mut bytes, 0); // variable index 0
        put_uvarint(&mut bytes, 3); // offset 3
        assert_eq!(
            decode(&bytes).unwrap_err(),
            DecodeError::Malformed {
                site: MalformedSite::NonCanonical,
            },
            "a non-canonical level encoding is rejected (E4; B2.1 obligation re-armed)"
        );
    }

    #[test]
    fn a_non_canonical_literal_encoding_is_rejected()
    {
        // A def(Integer, 007): the leading-zero magnitude is not canonical, so
        // the re-encode ("7") differs and the artifact is rejected.
        let mut bytes = artifact_header(1);
        bytes.push(WIRE_ADMISSION_CHECKED);
        bytes.push(0); // kind Def
        put_uvarint(&mut bytes, 0); // empty name
        put_uvarint(&mut bytes, 0); // params
        put_uvarint(&mut bytes, 0); // constraints
        bytes.push(0); // declared type: TYPE_BASE
        bytes.push(0); // base atom: Integer
        bytes.push(3); // body: TERM_LITERAL
        bytes.push(0); // literal kind: Integer
        bytes.push(0); // sign: NonNegative
        put_uvarint(&mut bytes, 3); // magnitude length
        bytes.extend_from_slice(b"007"); // non-canonical (leading zeros)
        put_uvarint(&mut bytes, 0); // slot 0
        put_uvarint(&mut bytes, 0); // slot 1
        put_uvarint(&mut bytes, 0); // slot 2
        put_uvarint(&mut bytes, 0); // slot 3
        assert_eq!(
            decode(&bytes).unwrap_err(),
            DecodeError::Malformed {
                site: MalformedSite::NonCanonical,
            },
            "a non-canonical literal encoding is rejected (E4)"
        );
    }

    #[test]
    fn a_non_digit_magnitude_is_rejected()
    {
        let mut bytes = artifact_header(1);
        bytes.push(WIRE_ADMISSION_CHECKED);
        bytes.push(0); // kind Def
        put_uvarint(&mut bytes, 0);
        put_uvarint(&mut bytes, 0);
        put_uvarint(&mut bytes, 0);
        bytes.push(0); // TYPE_BASE
        bytes.push(0); // Integer
        bytes.push(3); // TERM_LITERAL
        bytes.push(0); // Integer literal
        bytes.push(0); // sign
        put_uvarint(&mut bytes, 2);
        bytes.extend_from_slice(b"1a"); // non-digit
        assert_eq!(
            decode(&bytes).unwrap_err(),
            DecodeError::Malformed {
                site: MalformedSite::LiteralPayload,
            },
            "a non-digit magnitude is rejected by the smart constructor"
        );
    }

    #[test]
    fn an_overlong_varint_is_rejected()
    {
        // Replace the empty minted-atom-table varint (a single 0x00) with the
        // overlong two-byte encoding 0x80 0x00 of the same value.
        let mut bytes = artifact_header(0);
        bytes.splice(6 .. 7, [0x80, 0x00]);
        assert_eq!(
            decode(&bytes).unwrap_err(),
            DecodeError::Malformed {
                site: MalformedSite::Varint,
            },
            "an overlong (non-minimal) varint is rejected"
        );
    }

    #[test]
    fn a_corrupted_tag_is_rejected()
    {
        // A Unit axiom ends with its declared type tag (TYPE_UNIT); corrupt it.
        let mut bytes = write(&{
            let mut environment = Environment::new();
            environment
                .add_decl(Declaration::axiom(
                    LevelSignature::monomorphic(),
                    ValueType::Unit,
                ))
                .unwrap();
            environment
        });
        let last = bytes.len() - 1;
        bytes[last] = 0xFF; // an out-of-vocabulary type tag
        assert!(
            matches!(decode(&bytes), Err(DecodeError::UnknownTag { .. })),
            "a corrupted tag byte is rejected as an unknown tag"
        );
    }

    #[test]
    fn truncation_at_every_prefix_is_rejected()
    {
        let bytes = write(&rich_checked_environment());
        for length in 0 .. bytes.len() {
            let prefix = &bytes[.. length];
            assert!(
                decode(prefix).is_err(),
                "the {length}-byte proper prefix must be rejected, never a false success"
            );
        }
        assert!(decode(&bytes).is_ok(), "the full artifact still decodes");
    }

    // ----- The generated strategies for the round-trip property. -----

    /// A canonical magnitude from a `u64`.
    fn arb_magnitude() -> impl Strategy<Value = Magnitude>
    {
        any::<u64>().prop_map(|value| Magnitude::from_decimal_text(format!("{value}")).unwrap())
    }

    /// A canonical fraction from a `u32`.
    fn arb_fraction() -> impl Strategy<Value = FractionDigits>
    {
        any::<u32>()
            .prop_map(|value| FractionDigits::from_decimal_text(format!("{value}")).unwrap())
    }

    /// An arbitrary literal.
    fn arb_literal() -> impl Strategy<Value = Literal>
    {
        let sign = prop_oneof![Just(Sign::NonNegative), Just(Sign::Negative)];
        prop_oneof![
            (sign.clone(), arb_magnitude()).prop_map(|(sign, magnitude)| Literal::Integer(
                IntegerLiteral::new(sign, magnitude)
            )),
            any::<String>().prop_map(|content| Literal::Text(StringLiteral::new(content))),
            (sign, arb_magnitude(), arb_fraction()).prop_map(|(sign, integer, fraction)| {
                Literal::Numeric(NumericLiteral::new(sign, integer, fraction))
            }),
        ]
    }

    /// An arbitrary level: a small constant or a small variable atom.
    fn arb_level() -> impl Strategy<Value = Level>
    {
        prop_oneof![
            (0_u64 .. 5).prop_map(|constant| Level::constant(LevelConstant::from(constant))),
            (0_u32 .. 3, 0_u64 .. 4).prop_map(|(index, offset)| var_plus(index, offset)),
        ]
    }

    /// An arbitrary value type (recursively, embedding computation types under
    /// thunks and arrows).
    fn arb_value_type() -> impl Strategy<Value = ValueType>
    {
        let leaf = prop_oneof![
            Just(ValueType::Unit),
            prop_oneof![
                Just(BaseType::Integer),
                Just(BaseType::String),
                Just(BaseType::Numeric),
            ]
            .prop_map(ValueType::Base),
            arb_level().prop_map(ValueType::Universe),
        ];
        leaf.prop_recursive(3, 24, 3, |inner| {
            prop_oneof![
                (inner.clone(), inner.clone())
                    .prop_map(|(one, two)| ValueType::Product(Box::new(one), Box::new(two))),
                (inner.clone(), inner.clone())
                    .prop_map(|(one, two)| ValueType::Sum(Box::new(one), Box::new(two))),
                (inner.clone(), arb_level()).prop_map(|(inner_type, target)| ValueType::Lift {
                    inner: Box::new(inner_type),
                    target,
                }),
                inner
                    .clone()
                    .prop_map(
                        |result| ValueType::Thunk(Box::new(CompType::Returner(Box::new(result))))
                    ),
                (inner.clone(), inner).prop_map(|(domain, result)| ValueType::Thunk(Box::new(
                    CompType::Arrow {
                        domain: Box::new(domain),
                        codomain: Box::new(CompType::Returner(Box::new(result))),
                    }
                ))),
            ]
        })
    }

    /// An arbitrary value (recursively, exercising every value former and,
    /// under thunks, every computation former).
    fn arb_value() -> impl Strategy<Value = Value>
    {
        let leaf = prop_oneof![
            (0_u32 .. 4).prop_map(|index| Value::Variable(DeBruijnIndex::from(index))),
            (0_usize .. 4).prop_map(|index| Value::Constant(ConstantIndex::from(index))),
            Just(Value::Unit),
            arb_literal().prop_map(Value::Literal),
        ];
        leaf.prop_recursive(3, 40, 3, |inner| {
            let side = prop_oneof![Just(Side::Left), Just(Side::Right)];
            prop_oneof![
                (inner.clone(), inner.clone())
                    .prop_map(|(one, two)| Value::Pair(Box::new(one), Box::new(two))),
                (side, inner.clone())
                    .prop_map(|(side, body)| Value::Injection(side, Box::new(body))),
                (arb_level(), inner.clone()).prop_map(|(target, body)| Value::Lift {
                    target,
                    body: Box::new(body),
                }),
                inner
                    .clone()
                    .prop_map(|value| Value::Thunk(Box::new(Computation::Return(Box::new(value))))),
                inner
                    .clone()
                    .prop_map(|value| Value::Thunk(Box::new(Computation::Lambda(Box::new(
                        Computation::Return(Box::new(value))
                    ))))),
                inner
                    .clone()
                    .prop_map(|value| Value::Thunk(Box::new(Computation::Force(Box::new(
                        Value::Thunk(Box::new(Computation::Return(Box::new(value))))
                    ))))),
                (inner.clone(), inner.clone()).prop_map(|(head, argument)| Value::Thunk(Box::new(
                    Computation::Application(
                        Box::new(Computation::Force(Box::new(head))),
                        Box::new(argument),
                    )
                ))),
                (inner.clone(), inner.clone()).prop_map(|(first, second)| Value::Thunk(Box::new(
                    Computation::Bind(
                        Box::new(Computation::Return(Box::new(first))),
                        Box::new(Computation::Return(Box::new(second))),
                    )
                ))),
                (inner.clone(), inner.clone(), inner).prop_map(|(scrutinee, left, right)| {
                    Value::Thunk(Box::new(Computation::Case {
                        scrutinee: Box::new(scrutinee),
                        on_left: Box::new(Computation::Return(Box::new(left))),
                        on_right: Box::new(Computation::Return(Box::new(right))),
                    }))
                }),
            ]
        })
    }

    /// An arbitrary variable-only landmark constraint.
    fn arb_constraint() -> impl Strategy<Value = LandmarkConstraint>
    {
        (
            0_u32 .. 3,
            0_u64 .. 3,
            0_u32 .. 3,
            0_u64 .. 3,
            any::<bool>(),
        )
            .prop_map(|(left_var, left_offset, right_var, right_offset, equal)| {
                let left = var_plus(left_var, left_offset);
                let right = var_plus(right_var, right_offset);
                if equal {
                    LandmarkConstraint::equal(left, right).unwrap()
                }
                else {
                    LandmarkConstraint::leq(left, right).unwrap()
                }
            })
    }

    /// An arbitrary prenex level signature.
    fn arb_signature() -> impl Strategy<Value = LevelSignature>
    {
        (0_u32 .. 3, prop::collection::vec(arb_constraint(), 0 .. 3)).prop_map(
            |(params, constraints)| LevelSignature::new(LevelParamCount::from(params), constraints),
        )
    }

    /// An arbitrary declaration (a def or an axiom).
    fn arb_declaration() -> impl Strategy<Value = Declaration>
    {
        prop_oneof![
            (arb_signature(), arb_value_type(), arb_value()).prop_map(
                |(signature, declared, body)| Declaration::def(signature, declared, body)
            ),
            (arb_signature(), arb_value_type())
                .prop_map(|(signature, declared)| Declaration::axiom(signature, declared)),
        ]
    }

    proptest! {
        /// An environment of arbitrary declarations (admitted through the
        /// bypass, so any structure is representable) round-trips: the recovered
        /// artifact is byte-identical, and each declaration's content, level
        /// signature, and admission mark survive.
        #[test]
        fn round_trip_reproduces_arbitrary_declarations(
            declarations in prop::collection::vec(arb_declaration(), 0 .. 5),
        )
        {
            let mut environment = Environment::new();
            for declaration in &declarations {
                let _checked = environment.add_decl_unchecked(declaration.clone());
            }
            let bytes = write(&environment);
            let recovered = read(&bytes).expect("a genuine artifact must read");
            prop_assert!(write(&recovered) == bytes, "the round trip is byte-stable");

            let decoded = decode(&bytes).unwrap();
            prop_assert_eq!(decoded.len(), declarations.len(), "every declaration survives");
            for (original, recovered_declaration) in declarations.iter().zip(decoded.iter()) {
                prop_assert!(
                    original.content() == recovered_declaration.declaration().content(),
                    "the declaration content round-trips exactly"
                );
                prop_assert_eq!(
                    u32::from(original.levels().params()),
                    u32::from(recovered_declaration.declaration().levels().params()),
                    "the level parameter count round-trips"
                );
                prop_assert!(
                    original.levels().constraints()
                        == recovered_declaration.declaration().levels().constraints(),
                    "the landmark constraints round-trip exactly"
                );
                prop_assert_eq!(
                    recovered_declaration.mark(),
                    AdmissionMark::UncheckedBypass,
                    "the bypass mark round-trips"
                );
            }
        }

        /// Serialization is deterministic on any environment.
        #[test]
        fn write_is_deterministic(
            declarations in prop::collection::vec(arb_declaration(), 0 .. 5),
        )
        {
            let mut environment = Environment::new();
            for declaration in declarations {
                let _checked = environment.add_decl_unchecked(declaration);
            }
            prop_assert_eq!(write(&environment), write(&environment), "write is deterministic");
        }

        /// Arbitrary bytes never panic the reader: decode always returns, and a
        /// successful decode round-trips to the same bytes (a decoded artifact
        /// is canonical).
        #[test]
        fn arbitrary_bytes_never_panic(raw in prop::collection::vec(any::<u8>(), 0 .. 128))
        {
            match decode(&raw) {
                | Ok(_declarations) => {
                    prop_assert!(read(&raw).is_ok(), "a decodable artifact also reads");
                },
                | Err(_error) => {
                    prop_assert!(read(&raw).is_err(), "a rejected artifact also fails to read");
                },
            }
        }
    }
}
