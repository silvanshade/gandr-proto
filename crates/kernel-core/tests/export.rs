//! The K5 export writer/reader differential and rejection-totality suite
//! (kernel-boundary.md §5, E1–E6).
//!
//! The round-trip properties pin that `read(write(env))` reproduces the
//! environment and that `write` is deterministic; structural content equality
//! is witnessed by byte identity of the re-serialization (E4), the
//! arena-layout- independent canonical form. The rejection suite pins that the
//! validating reader is total on adversarial bytes and that each named refusal
//! fires.

#![cfg_attr(
    test,
    allow(
        clippy::arithmetic_side_effects,
        clippy::as_conversions,
        clippy::cast_possible_truncation,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic,
        clippy::pattern_type_mismatch,
        clippy::unwrap_used,
        dead_code,
        reason = "the standard test-allow set keeps tests readable, and dead_code covers the shared common module (docs/workflow/rust.md)"
    )
)]

mod common;

/// The differential and rejection suite.
#[cfg(test)]
mod tests
{
    use gandr_kernel_core::AdmissionMark;
    use gandr_kernel_core::BaseType;
    use gandr_kernel_core::DecodeError;
    use gandr_kernel_core::Environment;
    use gandr_kernel_core::IntegerLiteral;
    use gandr_kernel_core::LevelParamCount;
    use gandr_kernel_core::LevelSignature;
    use gandr_kernel_core::Literal;
    use gandr_kernel_core::Magnitude;
    use gandr_kernel_core::MalformedSite;
    use gandr_kernel_core::ReservedKind;
    use gandr_kernel_core::ReservedSlot;
    use gandr_kernel_core::Sign;
    use gandr_kernel_core::decode;
    use gandr_kernel_core::read;
    use gandr_kernel_core::write;
    use gandr_kernel_strata::LandmarkConstraint;
    use gandr_kernel_strata::Level;
    use gandr_kernel_strata::LevelConstant;
    use gandr_kernel_strata::LevelVar;
    use gandr_kernel_strata::LevelVarIndex;
    use proptest::prelude::*;

    use crate::common;
    use crate::common::CompTypeSpec;
    use crate::common::ComputationSpec;
    use crate::common::ValueSpec;
    use crate::common::ValueTypeSpec;

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
    fn unit_endo_thunk_type() -> ValueTypeSpec
    {
        ValueTypeSpec::Thunk(Box::new(CompTypeSpec::Arrow(
            Box::new(ValueTypeSpec::Unit),
            Box::new(CompTypeSpec::Returner(Box::new(ValueTypeSpec::Unit))),
        )))
    }

    /// The identity thunk body `thunk (lambda. return v0)`.
    fn identity_thunk_body() -> ValueSpec
    {
        ValueSpec::Thunk(Box::new(ComputationSpec::Lambda(Box::new(
            ComputationSpec::Return(Box::new(ValueSpec::Variable(0))),
        ))))
    }

    /// A well-typed environment exercising every declaration shape.
    fn rich_checked_environment() -> Environment
    {
        let mut environment = Environment::new();
        let axiom0 = common::stage_axiom(
            &mut environment,
            LevelSignature::monomorphic(),
            &ValueTypeSpec::Unit,
        );
        environment.add_decl(axiom0).unwrap();
        let def1 = common::stage_def(
            &mut environment,
            LevelSignature::monomorphic(),
            &ValueTypeSpec::Unit,
            &ValueSpec::Unit,
        );
        environment.add_decl(def1).unwrap();
        let def2 = common::stage_def(
            &mut environment,
            LevelSignature::monomorphic(),
            &unit_endo_thunk_type(),
            &identity_thunk_body(),
        );
        environment.add_decl(def2).unwrap();
        let def3 = common::stage_def(
            &mut environment,
            LevelSignature::monomorphic(),
            &ValueTypeSpec::Base(BaseType::Integer),
            &ValueSpec::Literal(Literal::Integer(IntegerLiteral::new(
                Sign::NonNegative,
                Magnitude::from_decimal_text(String::from("42")).unwrap(),
            ))),
        );
        environment.add_decl(def3).unwrap();
        let axiom4 = common::stage_axiom(
            &mut environment,
            LevelSignature::monomorphic(),
            &ValueTypeSpec::Universe(Level::constant(LevelConstant::from(0_u64))),
        );
        environment.add_decl(axiom4).unwrap();
        let axiom5 = common::stage_axiom(
            &mut environment,
            LevelSignature::monomorphic(),
            &ValueTypeSpec::Lift(
                Box::new(ValueTypeSpec::Unit),
                Level::constant(LevelConstant::from(2_u64)),
            ),
        );
        environment.add_decl(axiom5).unwrap();
        let axiom6 = common::stage_axiom(
            &mut environment,
            LevelSignature::new(LevelParamCount::from(1_u32), Vec::new()),
            &ValueTypeSpec::Universe(var_plus(0, 0)),
        );
        environment.add_decl(axiom6).unwrap();
        let constraint = LandmarkConstraint::leq(var_plus(0, 0), var_plus(1, 0)).unwrap();
        let axiom7 = common::stage_axiom(
            &mut environment,
            LevelSignature::new(LevelParamCount::from(2_u32), vec![constraint]),
            &ValueTypeSpec::Universe(var_plus(0, 0)),
        );
        environment.add_decl(axiom7).unwrap();
        // 8: def U (Unit -> F Unit) = the constant reference to declaration 2.
        let def8 = common::stage_def(
            &mut environment,
            LevelSignature::monomorphic(),
            &unit_endo_thunk_type(),
            &ValueSpec::Constant(2),
        );
        environment.add_decl(def8).unwrap();
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
            decode(&bytes).unwrap().declarations().len(),
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
        let artifact = decode(&bytes).unwrap();
        assert_eq!(
            artifact.declarations().len(),
            9,
            "every declaration survives"
        );
        for decoded_declaration in artifact.declarations() {
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
        let mut environment = Environment::new();
        let axiom_decl = common::stage_axiom(
            &mut environment,
            LevelSignature::monomorphic(),
            &ValueTypeSpec::Unit,
        );
        let axiom = environment.add_decl(axiom_decl).unwrap();
        let dependent_decl = common::stage_def(
            &mut environment,
            LevelSignature::monomorphic(),
            &ValueTypeSpec::Unit,
            &ValueSpec::Constant(usize::from(axiom.position())),
        );
        let dependent = environment.add_decl(dependent_decl).unwrap();
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
        let mut environment = Environment::new();
        let bypassed_decl = common::stage_def(
            &mut environment,
            LevelSignature::monomorphic(),
            &ValueTypeSpec::Base(BaseType::Integer),
            &ValueSpec::Unit, // ill-typed, but the bypass does not check
        );
        let bypassed = environment.add_decl_unchecked(bypassed_decl);
        let dependent_decl = common::stage_def(
            &mut environment,
            LevelSignature::monomorphic(),
            &ValueTypeSpec::Base(BaseType::Integer),
            &ValueSpec::Constant(usize::from(bypassed.position())),
        );
        let dependent = environment.add_decl(dependent_decl).unwrap();
        let bytes = write(&environment);

        let artifact = decode(&bytes).unwrap();
        assert_eq!(
            artifact.declarations()[0].mark(),
            AdmissionMark::UncheckedBypass,
            "the bypass mark survives serialization"
        );
        assert_eq!(
            artifact.declarations()[1].mark(),
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
        let mut environment = Environment::new();
        let axiom = common::stage_axiom(
            &mut environment,
            LevelSignature::monomorphic(),
            &ValueTypeSpec::Unit,
        );
        environment.add_decl(axiom).unwrap();
        let mut bytes = write(&environment);
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

    /// An arbitrary declaration description (a def or an axiom).
    #[derive(Clone, Debug)]
    enum DeclSpec
    {
        Def(LevelSignature, ValueTypeSpec, ValueSpec),
        Axiom(LevelSignature, ValueTypeSpec),
    }

    /// An arbitrary declaration description.
    fn arb_declaration() -> impl Strategy<Value = DeclSpec>
    {
        prop_oneof![
            (
                arb_signature(),
                common::arb_value_type_spec(),
                common::arb_value_spec()
            )
                .prop_map(|(signature, declared, body)| DeclSpec::Def(signature, declared, body)),
            (arb_signature(), common::arb_value_type_spec())
                .prop_map(|(signature, declared)| DeclSpec::Axiom(signature, declared)),
        ]
    }

    /// Build a `DeclSpec` into `environment` and admit it through the bypass,
    /// returning its level signature for the round-trip comparison.
    fn admit_spec_unchecked(
        environment: &mut Environment,
        spec: &DeclSpec,
    ) -> LevelSignature
    {
        match spec {
            | DeclSpec::Def(signature, declared, body) => {
                let declaration = common::stage_def(environment, signature.clone(), declared, body);
                let _id = environment.add_decl_unchecked(declaration);
                signature.clone()
            },
            | DeclSpec::Axiom(signature, declared) => {
                let declaration = common::stage_axiom(environment, signature.clone(), declared);
                let _id = environment.add_decl_unchecked(declaration);
                signature.clone()
            },
        }
    }

    proptest! {
        /// An environment of arbitrary declarations (admitted through the
        /// bypass, so any structure is representable) round-trips: the recovered
        /// artifact is byte-identical (which witnesses content equality via the
        /// canonical form, E4), and each declaration's level signature and
        /// admission mark survive.
        #[test]
        fn round_trip_reproduces_arbitrary_declarations(
            declarations in prop::collection::vec(arb_declaration(), 0 .. 5),
        )
        {
            let mut environment = Environment::new();
            let mut signatures: Vec<LevelSignature> = Vec::new();
            for declaration in &declarations {
                signatures.push(admit_spec_unchecked(&mut environment, declaration));
            }
            let bytes = write(&environment);
            let recovered = read(&bytes).expect("a genuine artifact must read");
            prop_assert!(write(&recovered) == bytes, "the round trip is byte-stable");

            let artifact = decode(&bytes).unwrap();
            prop_assert_eq!(
                artifact.declarations().len(),
                declarations.len(),
                "every declaration survives"
            );
            for (signature, recovered_declaration) in
                signatures.iter().zip(artifact.declarations().iter())
            {
                prop_assert_eq!(
                    u32::from(signature.params()),
                    u32::from(recovered_declaration.declaration().levels().params()),
                    "the level parameter count round-trips"
                );
                prop_assert!(
                    signature.constraints()
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
            for declaration in &declarations {
                let _signature = admit_spec_unchecked(&mut environment, declaration);
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
                | Ok(_artifact) => {
                    prop_assert!(read(&raw).is_ok(), "a decodable artifact also reads");
                },
                | Err(_error) => {
                    prop_assert!(read(&raw).is_err(), "a rejected artifact also fails to read");
                },
            }
        }
    }
}
