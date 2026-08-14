//! The K5 export writer/reader differential and rejection-totality suite for
//! the v1 maximal-sharing subterm-table format (kernel-boundary.md §5, E1–E6;
//! massive-term design §4, §7).
//!
//! The round-trip properties pin that `read(write(env))` reproduces the
//! environment and that `write` is deterministic and content-keyed
//! (structurally equal, differently-shared inputs write identically). The
//! rejection suite pins reader totality on adversarial bytes and each named
//! refusal: the v0 version refusal (E5), the reserved kinds/slots, a
//! non-canonical level/literal, the canonical-form violations (a duplicate,
//! mis-ordered, or dead entry, a forward/self child reference), and the
//! amplification defences (a repeated-diamond DAG rejected by the per-
//! declaration expanded-work budget before the checker, a many-cheap-segments
//! DAG rejected by the artifact-total budget, and the table-size cap), with
//! boundary goldens for all three budget constants.

/// The differential and rejection suite.
#[cfg(test)]
mod tests
{
    use gandr_kernel_core::AdmissionMark;
    use gandr_kernel_core::BaseType;
    use gandr_kernel_core::ConstantIndex;
    use gandr_kernel_core::DeBruijnIndex;
    use gandr_kernel_core::DecodeError;
    use gandr_kernel_core::Environment;
    use gandr_kernel_core::IntegerLiteral;
    use gandr_kernel_core::KernelError;
    use gandr_kernel_core::LevelParamCount;
    use gandr_kernel_core::LevelSignature;
    use gandr_kernel_core::Literal;
    use gandr_kernel_core::MAX_ARTIFACT_EXPANDED_WORK;
    use gandr_kernel_core::MAX_EXPANDED_TERM_WORK;
    use gandr_kernel_core::MAX_TABLE_ENTRIES;
    use gandr_kernel_core::Magnitude;
    use gandr_kernel_core::MalformedSite;
    use gandr_kernel_core::ReadError;
    use gandr_kernel_core::ReservedKind;
    use gandr_kernel_core::ReservedSlot;
    use gandr_kernel_core::Sign;
    use gandr_kernel_core::decode;
    use gandr_kernel_core::read;
    use gandr_kernel_core::write;
    use gandr_kernel_core::write_segmented;
    use gandr_kernel_strata::LandmarkConstraint;
    use gandr_kernel_strata::Level;
    use gandr_kernel_strata::LevelConstant;
    use gandr_kernel_strata::LevelOffset;
    use gandr_kernel_strata::LevelVar;
    use gandr_kernel_strata::LevelVarIndex;
    use proptest::prelude::*;

    use crate::common;
    use crate::common::CompTypeSpec;
    use crate::common::ComputationSpec;
    use crate::common::ValueSpec;
    use crate::common::ValueTypeSpec;

    // The v1 wire constants, restated here so this suite tests the byte format as
    // a black box (they mirror the crate-private `export` constants).
    const WIRE_MAGIC: [u8; 4] = *b"GKX1";
    const WIRE_ADMISSION_CHECKED: u8 = 0;
    const WIRE_KIND_DEF: u8 = 0;
    const WIRE_KIND_AXIOM: u8 = 1;
    const NODE_VT_BASE: u8 = 0x00;
    const NODE_VT_UNIT: u8 = 0x01;
    const NODE_VT_UNIVERSE: u8 = 0x02;
    const NODE_V_VARIABLE: u8 = 0x09;
    const NODE_V_UNIT: u8 = 0x0B;
    const NODE_V_LITERAL: u8 = 0x0C;
    const NODE_V_PAIR: u8 = 0x0D;

    /// Mutable bytes of one hand-crafted wire fixture.
    #[repr(transparent)]
    struct WireFixture(Vec<u8>);

    impl core::ops::Deref for WireFixture
    {
        type Target = Vec<u8>;

        fn deref(&self) -> &Self::Target
        {
            &self.0
        }
    }

    impl core::ops::DerefMut for WireFixture
    {
        fn deref_mut(&mut self) -> &mut Self::Target
        {
            &mut self.0
        }
    }

    /// Unsigned integer carried by one hand-crafted wire fixture.
    #[repr(transparent)]
    #[derive(Clone, Copy)]
    struct WireValue(u64);

    /// Declaration-kind byte carried by one hand-crafted wire fixture.
    #[repr(transparent)]
    #[derive(Clone, Copy)]
    struct WireTag(u8);

    /// Depth of one generated amplification fixture.
    #[repr(transparent)]
    #[derive(Clone, Copy)]
    struct FixtureDepth(usize);

    /// Declaration count of one generated amplification fixture.
    #[repr(transparent)]
    #[derive(Clone, Copy)]
    struct FixtureCount(usize);

    /// Append a canonical unsigned LEB128 varint (the writer's wire form).
    fn put_uvarint(
        out: &mut WireFixture,
        value: WireValue,
    )
    {
        let mut remaining = value.0;
        loop {
            let low = (remaining & 0x7f).to_le_bytes()[0];
            remaining = remaining.wrapping_shr(7);
            if remaining == 0 {
                out.push(low);
                break;
            }
            out.push(low | 0x80);
        }
    }

    /// The v1 artifact header (magic, v1 version, empty minted-atom table) plus
    /// a declaration count.
    fn v1_header(count: WireValue) -> WireFixture
    {
        let mut bytes = WireFixture(WIRE_MAGIC.to_vec());
        bytes.extend_from_slice(&[0, 1]); // version 1
        put_uvarint(&mut bytes, WireValue(0)); // R4 minted-atom table, empty
        put_uvarint(&mut bytes, count);
        bytes
    }

    /// Append a declaration segment header (admission checked, kind, empty
    /// name, monomorphic level signature).
    fn segment_head(
        bytes: &mut WireFixture,
        kind: WireTag,
    )
    {
        bytes.push(WIRE_ADMISSION_CHECKED);
        bytes.push(kind.0);
        put_uvarint(bytes, WireValue(0)); // name segments
        put_uvarint(bytes, WireValue(0)); // level params
        put_uvarint(bytes, WireValue(0)); // level constraints
    }

    /// `variable + offset` built through the strata smart constructors.
    fn var_plus(
        index: LevelVarIndex,
        offset: LevelOffset,
    ) -> Level
    {
        let mut level = Level::var(LevelVar::new(index));
        for _step in 0 .. u64::from(offset) {
            level = level.succ().unwrap();
        }
        level
    }

    /// `U (Unit -> F Unit)` — the thunked unit endofunction type spec.
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
            &ValueTypeSpec::Universe(var_plus(0.into(), 0.into())),
        );
        environment.add_decl(axiom6).unwrap();
        let constraint =
            LandmarkConstraint::leq(var_plus(0.into(), 0.into()), var_plus(1.into(), 0.into()))
                .unwrap();
        let axiom7 = common::stage_axiom(
            &mut environment,
            LevelSignature::new(LevelParamCount::from(2_u32), vec![constraint]),
            &ValueTypeSpec::Universe(var_plus(0.into(), 0.into())),
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
        let recovered = read(bytes.as_ref().into()).expect("the empty artifact must read");
        assert_eq!(
            write(&recovered),
            bytes,
            "the empty environment round-trips to byte-identical output"
        );
        assert_eq!(
            0,
            decode(bytes.as_ref().into()).unwrap().declarations().len(),
            "the empty artifact decodes to no declarations"
        );
    }

    #[test]
    fn round_trip_reproduces_the_environment()
    {
        let environment = rich_checked_environment();
        let bytes = write(&environment);
        let recovered = read(bytes.as_ref().into()).expect("a genuine artifact must read");
        assert_eq!(
            write(&recovered),
            bytes,
            "the recovered environment re-serializes byte-identically (E4)"
        );
        let artifact = decode(bytes.as_ref().into()).unwrap();
        assert_eq!(
            9,
            artifact.declarations().len(),
            "every declaration survives"
        );
        for decoded_declaration in artifact.declarations() {
            assert_eq!(
                AdmissionMark::Checked,
                decoded_declaration.mark(),
                "every rich declaration was checked-admitted"
            );
        }
    }

    // ----- Sealing: the minted-atom table and the freshness property. -----

    /// A sealed environment: one minted atom, then a member typed at it.
    ///
    /// Declaration 0 is the atom `t` at universe zero; declaration 1 is the
    /// identity `U (t -> F t)`, whose sealing provenance records that its type
    /// was projected onto `t`. Nothing in it mentions a representation, which
    /// is the point — a sealed member reaches the kernel already at the
    /// atom.
    fn sealed_environment() -> Environment
    {
        let mut environment = Environment::new();
        let atom = common::stage_abstract_type(
            &mut environment,
            LevelSignature::monomorphic(),
            Level::constant(LevelConstant::from(0_u64)),
        );
        environment
            .add_decl(atom)
            .expect("an abstract type at a universe kind admits");
        let member = common::stage_sealed_def(
            &mut environment,
            LevelSignature::monomorphic(),
            &ValueTypeSpec::Thunk(Box::new(CompTypeSpec::Arrow(
                Box::new(ValueTypeSpec::Abstract(0)),
                Box::new(CompTypeSpec::Returner(Box::new(ValueTypeSpec::Abstract(0)))),
            ))),
            &ValueSpec::Thunk(Box::new(ComputationSpec::Lambda(Box::new(
                ComputationSpec::Return(Box::new(ValueSpec::Variable(0))),
            )))),
            &[0],
        );
        environment
            .add_decl(member)
            .expect("the identity at a sealed atom admits");
        environment
    }

    /// The header's minted-atom table, and the offset its first entry sits at.
    ///
    /// The header is magic, version, the table (count then entries), and the
    /// declaration count; for the one-atom fixture the count and the single
    /// entry are one byte each, so the entry is at offset 7.
    const MINTED_ATOM_TABLE_COUNT_OFFSET: usize = 6;
    /// The offset of the first minted-atom entry in the one-atom fixture.
    const MINTED_ATOM_FIRST_ENTRY_OFFSET: usize = 7;

    /// A sealed artifact round-trips, and its atoms come back as atoms.
    ///
    /// The byte-identity assertion is the load-bearing half: it says the
    /// recovered environment re-serializes to the *same* minted-atom table, so
    /// replay re-minted every atom to the position the artifact recorded. That
    /// is deterministic re-minting, observed rather than assumed.
    #[test]
    fn a_sealed_artifact_round_trips_and_re_mints_deterministically()
    {
        let environment = sealed_environment();
        let bytes = write(&environment);
        let recovered = read(bytes.as_ref().into()).expect("a genuine sealed artifact must read");
        assert_eq!(
            write(&recovered),
            bytes,
            "replay re-mints every atom to its recorded position (the table re-serializes identically)"
        );
        assert_eq!(
            1, bytes[MINTED_ATOM_TABLE_COUNT_OFFSET],
            "the header records exactly one minted atom"
        );
        assert_eq!(
            0, bytes[MINTED_ATOM_FIRST_ENTRY_OFFSET],
            "and it sits at admission position zero"
        );
    }

    /// **Distinctness.** A table repeating an atom is refused, so two atoms can
    /// never be spelled at one position — the aliasing hazard, caught rather
    /// than trusted.
    #[test]
    fn a_minted_atom_table_with_a_repeat_is_refused()
    {
        let mut bytes = Vec::from(write(&sealed_environment()));
        bytes[MINTED_ATOM_TABLE_COUNT_OFFSET] = 2;
        bytes.insert(MINTED_ATOM_FIRST_ENTRY_OFFSET, 0);
        assert_eq!(
            DecodeError::ReservedSlotOccupied {
                slot: ReservedSlot::MintedAtomTable,
            },
            decode(bytes.as_slice().into()).unwrap_err(),
            "a minted-atom table repeating a position is refused"
        );
    }

    /// **Accounting.** A table omitting an atom the declarations do contain is
    /// refused, so no atom is smuggled past it.
    #[test]
    fn a_minted_atom_table_omitting_an_atom_is_refused()
    {
        let mut bytes = Vec::from(write(&sealed_environment()));
        bytes[MINTED_ATOM_TABLE_COUNT_OFFSET] = 0;
        let _entry = bytes.remove(MINTED_ATOM_FIRST_ENTRY_OFFSET);
        assert_eq!(
            DecodeError::ReservedSlotOccupied {
                slot: ReservedSlot::MintedAtomTable,
            },
            decode(bytes.as_slice().into()).unwrap_err(),
            "a minted-atom table omitting an atom is refused"
        );
    }

    /// **No forgery.** A table naming a position that holds an ordinary
    /// definition is refused, so the header cannot conjure an atom the
    /// declarations do not contain.
    #[test]
    fn a_minted_atom_table_naming_a_definition_is_refused()
    {
        let mut bytes = Vec::from(write(&sealed_environment()));
        bytes[MINTED_ATOM_FIRST_ENTRY_OFFSET] = 1; // position 1 is the member `Def`
        assert_eq!(
            DecodeError::ReservedSlotOccupied {
                slot: ReservedSlot::MintedAtomTable,
            },
            decode(bytes.as_slice().into()).unwrap_err(),
            "a minted-atom table naming a definition is refused"
        );
    }

    /// **The forged atom dies at the choke point.** Rewriting a sealed artifact
    /// so its atom is an ordinary axiom — and repairing the table to match, so
    /// the format plane is satisfied — still fails to replay: the member typed
    /// at that position no longer names an abstract type.
    ///
    /// This is the property that makes opacity re-derived rather than imported.
    /// The artifact is internally consistent and the reader accepts its
    /// *bytes*; it is the kernel that refuses, because being an atom is
    /// resolved against the environment rather than asserted by the
    /// reference.
    #[test]
    fn an_atom_downgraded_to_an_axiom_fails_to_replay()
    {
        let segmented = write_segmented(&sealed_environment());
        let header_len = segmented.header().as_ref().len();
        let mut bytes = Vec::from(segmented.bytes().as_ref());
        // The atom's segment leads the declarations: [admission, kind, ...].
        assert_eq!(
            2,
            bytes[header_len + 1],
            "the first segment declares the abstract-type kind"
        );
        bytes[header_len + 1] = WIRE_KIND_AXIOM;
        // Repair the table so the refusal cannot come from the format plane.
        bytes[MINTED_ATOM_TABLE_COUNT_OFFSET] = 0;
        let _entry = bytes.remove(MINTED_ATOM_FIRST_ENTRY_OFFSET);
        assert!(
            decode(bytes.as_slice().into()).is_ok(),
            "the rewritten artifact is well-formed on the format plane"
        );
        assert!(
            matches!(read(bytes.as_slice().into()), Err(ReadError::Admit(_))),
            "but the member typed at the downgraded position does not re-admit"
        );
    }

    /// Sealing provenance survives the round trip, and a rewritten provenance
    /// is refused at replay rather than silently believed.
    #[test]
    fn sealing_provenance_round_trips_and_cannot_be_rewritten()
    {
        let environment = sealed_environment();
        let bytes = write(&environment);
        let recovered = read(bytes.as_ref().into()).expect("the sealed artifact reads");
        assert_eq!(
            write(&recovered),
            bytes,
            "the recovered environment re-serializes its provenance identically"
        );
        // The member segment ends `… | erasure 0 | modes 0 | provenance 1, 0 |
        // directedness 0`, so the provenance entry is the artifact's
        // second-to-last byte. Re-point it at position 1 — the member itself —
        // which the member's own declared type does not project onto.
        let mut rewritten = Vec::from(bytes.as_ref());
        let entry = rewritten.len() - 2;
        assert_eq!(
            0, rewritten[entry],
            "the provenance entry names the atom at position zero"
        );
        rewritten[entry] = 1;
        assert_eq!(
            ReadError::Admit(KernelError::SealingProvenanceNotProjected {
                atom: ConstantIndex::from(1_usize),
            }),
            read(rewritten.as_slice().into()).unwrap_err(),
            "a provenance rewritten to an unprojected atom is refused at the choke point"
        );
    }

    #[test]
    fn write_segmented_matches_write()
    {
        let environment = rich_checked_environment();
        let segmented = write_segmented(&environment);
        assert_eq!(
            segmented.bytes().as_ref(),
            write(&environment).as_ref(),
            "write_segmented's bytes are byte-identical to write (B2.3 outer layer)"
        );
        assert_eq!(
            9,
            usize::from(segmented.segment_count()),
            "one declaration segment per admitted declaration"
        );
    }

    #[test]
    fn segments_reassemble_the_artifact()
    {
        let environment = rich_checked_environment();
        let segmented = write_segmented(&environment);
        let mut reassembled = segmented.header().as_ref().to_vec();
        for segment in segmented.segments() {
            reassembled.extend_from_slice(segment);
        }
        assert_eq!(
            reassembled,
            segmented.bytes().as_ref(),
            "header followed by every declaration segment reproduces the artifact"
        );
    }

    #[test]
    fn the_empty_environment_segments_to_a_header()
    {
        let environment = Environment::new();
        let segmented = write_segmented(&environment);
        assert_eq!(
            0,
            usize::from(segmented.segment_count()),
            "the empty environment has no declaration segments"
        );
        assert_eq!(
            segmented.header().as_ref(),
            segmented.bytes().as_ref(),
            "the empty artifact is exactly its header"
        );
        assert_eq!(
            0,
            segmented.segments().count(),
            "iterating the empty artifact's segments yields nothing"
        );
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
        let recovered = read(bytes.as_ref().into()).unwrap();
        assert_eq!(
            environment.audit(axiom),
            recovered.audit(axiom),
            "the axiom's recomputed audit agrees"
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

        let artifact = decode(bytes.as_ref().into()).unwrap();
        assert_eq!(
            AdmissionMark::UncheckedBypass,
            artifact.declarations()[0].mark(),
            "the bypass mark survives serialization"
        );

        let recovered = read(bytes.as_ref().into()).unwrap();
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

    // ----- Sharing determinism and retention (§7 items 1, 2). -----

    #[test]
    fn structurally_equal_inputs_write_identically()
    {
        // The bytes are a function of the ABSTRACT environment: a body built with
        // maximal in-arena sharing (`pair(x, x)`, x shared) and one built with no
        // sharing (`pair(x1, x2)`, distinct units) are structurally equal and
        // write to identical bytes (content-keyed, never ptr-keyed dedup).
        let mut shared_env = Environment::new();
        {
            let mut builder = shared_env.stage();
            let arena = builder.arena();
            let x = arena.value_unit();
            let body = arena.value_pair(x, x);
            let declared = arena.value_type_unit();
            let declaration = builder.def(LevelSignature::monomorphic(), declared, body);
            shared_env.add_decl_unchecked(declaration);
        };
        let mut unshared_env = Environment::new();
        {
            let mut builder = unshared_env.stage();
            let arena = builder.arena();
            let x1 = arena.value_unit();
            let x2 = arena.value_unit();
            let body = arena.value_pair(x1, x2);
            let declared = arena.value_type_unit();
            let declaration = builder.def(LevelSignature::monomorphic(), declared, body);
            unshared_env.add_decl_unchecked(declaration);
        };
        assert_eq!(
            write(&shared_env),
            write(&unshared_env),
            "structurally-equal, differently-shared inputs write identically (content-keyed dedup)"
        );
    }

    #[test]
    fn sharing_collapses_the_table()
    {
        // A body `pair(x, x)` with a shared `x` produces one shared entry, not
        // two: the artifact is strictly smaller than the same shape with two
        // DISTINCT (structurally different) children.
        let build = |same: bool| {
            let mut environment = Environment::new();
            let mut builder = environment.stage();
            let arena = builder.arena();
            let left = arena.value_variable(DeBruijnIndex::from(0_u32));
            let right = if same {
                left
            }
            else {
                arena.value_variable(DeBruijnIndex::from(1_u32))
            };
            let body = arena.value_pair(left, right);
            let declared = arena.value_type_unit();
            let declaration = builder.def(LevelSignature::monomorphic(), declared, body);
            environment.add_decl_unchecked(declaration);
            write(&environment)
        };
        assert!(
            build(true).len() < build(false).len(),
            "sharing `pair(x, x)` collapses to one entry, shrinking the artifact"
        );
    }

    // ----- The v0 version refusal (E5, §7 item 6). -----

    #[test]
    fn a_v0_artifact_is_refused()
    {
        // A v0-magic/v0-version artifact is refused UnsupportedVersion{found:0} —
        // the old v0 goldens repurposed as the refusal fixture (E5).
        let mut bytes = WireFixture(WIRE_MAGIC.to_vec());
        bytes.extend_from_slice(&[0, 0]); // version 0
        put_uvarint(&mut bytes, WireValue(0));
        put_uvarint(&mut bytes, WireValue(0));
        assert_eq!(
            DecodeError::UnsupportedVersion { found: 0 },
            decode(bytes.as_slice().into()).unwrap_err(),
            "a v0 artifact is a named refusal, not a guess (E5)"
        );
    }

    #[test]
    fn an_unknown_future_version_is_refused()
    {
        let mut bytes = Vec::from(write(&rich_checked_environment()));
        assert!(bytes.len() > 5, "the canonical header carries a version");
        bytes[4] = 0;
        bytes[5] = 2; // version 2
        assert_eq!(
            DecodeError::UnsupportedVersion { found: 2 },
            decode(bytes.as_slice().into()).unwrap_err(),
            "an unimplemented future version is refused"
        );
    }

    // ----- Reserved sections (R1–R4). -----

    /// R1 reserved four declaration kinds, and the sealing rung made
    /// `AbstractType` (byte 2) live — so it is decoded rather than refused and
    /// is deliberately absent here. The other three are still reserved.
    #[test]
    fn each_still_reserved_declaration_kind_is_rejected()
    {
        let expectations = [
            (3_u8, ReservedKind::ModuleSig),
            (4_u8, ReservedKind::ModuleDef),
            (5_u8, ReservedKind::FunctorDef),
        ];
        for (kind_byte, kind) in expectations {
            let mut bytes = v1_header(WireValue(1));
            bytes.push(WIRE_ADMISSION_CHECKED);
            bytes.push(kind_byte);
            assert_eq!(
                DecodeError::ReservedDeclarationKind { kind },
                decode(bytes.as_slice().into()).unwrap_err(),
                "reserved declaration kind {kind_byte} is rejected distinctly"
            );
        }
    }

    /// An artifact declaring an atom it does not contain is refused.
    ///
    /// This is the R4 table's refutation in its simplest form: the header
    /// claims one minted atom, the (empty) declaration sequence supplies
    /// none, and the reader believes the declarations rather than the
    /// claim.
    #[test]
    fn a_minted_atom_table_claiming_an_absent_atom_is_rejected()
    {
        let mut bytes = v1_header(WireValue(0));
        bytes[6] = 1; // the R4 minted-atom table count
        bytes.insert(7, 0); // the claimed atom: admission position 0
        assert_eq!(
            DecodeError::ReservedSlotOccupied {
                slot: ReservedSlot::MintedAtomTable,
            },
            decode(bytes.as_slice().into()).unwrap_err(),
            "a minted-atom table naming an atom the declarations do not contain is refused (R4)"
        );
    }

    #[test]
    fn a_non_empty_structured_name_is_rejected()
    {
        let mut bytes = v1_header(WireValue(1));
        bytes.push(WIRE_ADMISSION_CHECKED);
        bytes.push(WIRE_KIND_AXIOM);
        put_uvarint(&mut bytes, WireValue(1)); // one name segment -> reserved at v1
        assert_eq!(
            DecodeError::ReservedSlotOccupied {
                slot: ReservedSlot::StructuredName,
            },
            decode(bytes.as_slice().into()).unwrap_err(),
            "a non-empty structured name is rejected at v1 (R2)"
        );
    }

    #[test]
    fn a_non_empty_def_annotation_slot_is_rejected()
    {
        let mut bytes = v1_header(WireValue(1));
        segment_head(&mut bytes, WireTag(WIRE_KIND_DEF));
        put_uvarint(&mut bytes, WireValue(2)); // entry_count
        bytes.push(NODE_VT_UNIT); // entry 0: declared type
        bytes.push(NODE_V_UNIT); // entry 1: body
        put_uvarint(&mut bytes, WireValue(0)); // root_declared
        put_uvarint(&mut bytes, WireValue(1)); // root_body
        put_uvarint(&mut bytes, WireValue(1)); // first Def annotation slot -> non-empty
        assert_eq!(
            DecodeError::ReservedSlotOccupied {
                slot: ReservedSlot::ErasureAnnotation,
            },
            decode(bytes.as_slice().into()).unwrap_err(),
            "a non-empty per-Def annotation slot is rejected at v1 (R3)"
        );
    }

    // ----- Non-canonical encodings (E4). -----

    #[test]
    fn a_non_canonical_level_encoding_is_rejected()
    {
        let mut bytes = v1_header(WireValue(1));
        bytes.push(WIRE_ADMISSION_CHECKED);
        bytes.push(WIRE_KIND_AXIOM);
        put_uvarint(&mut bytes, WireValue(0)); // name
        put_uvarint(&mut bytes, WireValue(1)); // one level parameter (so x0 is in scope)
        put_uvarint(&mut bytes, WireValue(0)); // constraints
        put_uvarint(&mut bytes, WireValue(1)); // entry_count
        bytes.push(NODE_VT_UNIVERSE);
        // Non-canonical level: max(3, x0 + 3) -> constant 3 is dominated.
        put_uvarint(&mut bytes, WireValue(3)); // constant part
        put_uvarint(&mut bytes, WireValue(1)); // one atom
        put_uvarint(&mut bytes, WireValue(0)); // variable index 0
        put_uvarint(&mut bytes, WireValue(3)); // offset 3
        put_uvarint(&mut bytes, WireValue(0)); // root_declared
        assert_eq!(
            DecodeError::Malformed {
                site: MalformedSite::NonCanonical,
            },
            decode(bytes.as_slice().into()).unwrap_err(),
            "a non-canonical level encoding is rejected (E4)"
        );
    }

    #[test]
    fn a_non_canonical_literal_encoding_is_rejected()
    {
        let mut bytes = v1_header(WireValue(1));
        segment_head(&mut bytes, WireTag(WIRE_KIND_DEF));
        put_uvarint(&mut bytes, WireValue(2)); // entry_count
        bytes.push(NODE_VT_BASE);
        bytes.push(0); // base atom: Integer
        bytes.push(NODE_V_LITERAL);
        bytes.push(0); // literal kind: Integer
        bytes.push(0); // sign: NonNegative
        put_uvarint(&mut bytes, WireValue(3)); // magnitude length
        bytes.extend_from_slice(b"007"); // non-canonical (leading zeros)
        put_uvarint(&mut bytes, WireValue(0)); // root_declared
        put_uvarint(&mut bytes, WireValue(1)); // root_body
        core::iter::repeat_n(0u64, 4usize).for_each(|v| {
            put_uvarint(&mut bytes, WireValue(v)); // R3 slots
        });
        assert_eq!(
            DecodeError::Malformed {
                site: MalformedSite::NonCanonical,
            },
            decode(bytes.as_slice().into()).unwrap_err(),
            "a non-canonical literal encoding is rejected (E4)"
        );
    }

    #[test]
    fn a_non_digit_magnitude_is_rejected()
    {
        let mut bytes = v1_header(WireValue(1));
        segment_head(&mut bytes, WireTag(WIRE_KIND_DEF));
        put_uvarint(&mut bytes, WireValue(2));
        bytes.push(NODE_VT_BASE);
        bytes.push(0); // Integer
        bytes.push(NODE_V_LITERAL);
        bytes.push(0); // Integer literal
        bytes.push(0); // sign
        put_uvarint(&mut bytes, WireValue(2));
        bytes.extend_from_slice(b"1a"); // letters are not digits
        put_uvarint(&mut bytes, WireValue(0));
        put_uvarint(&mut bytes, WireValue(1));
        core::iter::repeat_n(0u64, 4usize).for_each(|v| {
            put_uvarint(&mut bytes, WireValue(v));
        });
        assert_eq!(
            DecodeError::Malformed {
                site: MalformedSite::LiteralPayload,
            },
            decode(bytes.as_slice().into()).unwrap_err(),
            "a non-digit magnitude is rejected by the smart constructor"
        );
    }

    // ----- Canonical-form violations (§7 item 3). -----

    #[test]
    fn a_duplicate_entry_is_rejected()
    {
        // Two structurally-equal entries (unit values) — the maximal-sharing
        // re-encoder merges them, so the table is not canonical.
        let mut bytes = v1_header(WireValue(1));
        segment_head(&mut bytes, WireTag(WIRE_KIND_DEF));
        put_uvarint(&mut bytes, WireValue(4)); // entry_count
        bytes.push(NODE_VT_UNIT); // 0: declared
        bytes.push(NODE_V_UNIT); // 1: unit value
        bytes.push(NODE_V_UNIT); // 2: DUPLICATE unit value
        bytes.push(NODE_V_PAIR); // 3: pair(1, 2)
        put_uvarint(&mut bytes, WireValue(1));
        put_uvarint(&mut bytes, WireValue(2));
        put_uvarint(&mut bytes, WireValue(0)); // root_declared
        put_uvarint(&mut bytes, WireValue(3)); // root_body
        core::iter::repeat_n(0u64, 4usize).for_each(|v| {
            put_uvarint(&mut bytes, WireValue(v));
        });
        assert_eq!(
            DecodeError::Malformed {
                site: MalformedSite::NonCanonical,
            },
            decode(bytes.as_slice().into()).unwrap_err(),
            "a non-maximally-shared (duplicate-entry) table is rejected (§4.6)"
        );
    }

    #[test]
    fn a_mis_ordered_table_is_rejected()
    {
        // pair(var0, var1) with the two variable entries in the wrong (non
        // post-order-first-completion) order.
        let mut bytes = v1_header(WireValue(1));
        segment_head(&mut bytes, WireTag(WIRE_KIND_DEF));
        put_uvarint(&mut bytes, WireValue(4));
        bytes.push(NODE_VT_UNIT); // 0: declared
        bytes.push(NODE_V_VARIABLE); // 1: var1 (should be second)
        put_uvarint(&mut bytes, WireValue(1));
        bytes.push(NODE_V_VARIABLE); // 2: var0 (should be first)
        put_uvarint(&mut bytes, WireValue(0));
        bytes.push(NODE_V_PAIR); // 3: pair(var0=2, var1=1)
        put_uvarint(&mut bytes, WireValue(2));
        put_uvarint(&mut bytes, WireValue(1));
        put_uvarint(&mut bytes, WireValue(0)); // root_declared
        put_uvarint(&mut bytes, WireValue(3)); // root_body
        core::iter::repeat_n(0u64, 4usize).for_each(|v| {
            put_uvarint(&mut bytes, WireValue(v));
        });
        assert_eq!(
            DecodeError::Malformed {
                site: MalformedSite::NonCanonical,
            },
            decode(bytes.as_slice().into()).unwrap_err(),
            "a mis-ordered (non-post-order) table is rejected (§4.6)"
        );
    }

    #[test]
    fn a_dead_entry_is_rejected()
    {
        // An entry reachable from no root — the re-encoder drops it.
        let mut bytes = v1_header(WireValue(1));
        segment_head(&mut bytes, WireTag(WIRE_KIND_DEF));
        put_uvarint(&mut bytes, WireValue(3));
        bytes.push(NODE_VT_UNIT); // 0: declared
        bytes.push(NODE_V_UNIT); // 1: body
        bytes.push(NODE_V_VARIABLE); // 2: DEAD (unreferenced)
        put_uvarint(&mut bytes, WireValue(0));
        put_uvarint(&mut bytes, WireValue(0)); // root_declared
        put_uvarint(&mut bytes, WireValue(1)); // root_body
        core::iter::repeat_n(0u64, 4usize).for_each(|v| {
            put_uvarint(&mut bytes, WireValue(v));
        });
        assert_eq!(
            DecodeError::Malformed {
                site: MalformedSite::NonCanonical,
            },
            decode(bytes.as_slice().into()).unwrap_err(),
            "a dead (unreferenced) entry is rejected (§4.6)"
        );
    }

    #[test]
    fn a_self_or_forward_child_reference_is_rejected()
    {
        // pair(1, 1) as entry 1 references its own global index — not strictly
        // earlier (acyclicity).
        let mut bytes = v1_header(WireValue(1));
        segment_head(&mut bytes, WireTag(WIRE_KIND_DEF));
        put_uvarint(&mut bytes, WireValue(2));
        bytes.push(NODE_VT_UNIT); // 0: declared
        bytes.push(NODE_V_PAIR); // 1: pair(1, 1) -> self reference
        put_uvarint(&mut bytes, WireValue(1));
        put_uvarint(&mut bytes, WireValue(1));
        put_uvarint(&mut bytes, WireValue(0));
        put_uvarint(&mut bytes, WireValue(1));
        core::iter::repeat_n(0u64, 4usize).for_each(|v| {
            put_uvarint(&mut bytes, WireValue(v));
        });
        assert_eq!(
            DecodeError::Malformed {
                site: MalformedSite::ChildOrder,
            },
            decode(bytes.as_slice().into()).unwrap_err(),
            "a self/forward child reference is rejected (acyclicity)"
        );
    }

    // ----- The amplification defences (§4.4, §7 item 4). -----

    /// Build a bypass-admitted environment whose body is a `depth`-deep
    /// repeated pair diamond `pair(x, x)` (each level shares the prior),
    /// the declared type an ordinary unit.
    fn diamond_environment(depth: FixtureDepth) -> Environment
    {
        let mut environment = Environment::new();
        let mut builder = environment.stage();
        let arena = builder.arena();
        let mut node = arena.value_unit();
        for _step in 0 .. depth.0 {
            node = arena.value_pair(node, node);
        }
        let declared = arena.value_type_unit();
        let declaration = builder.def(LevelSignature::monomorphic(), declared, node);
        environment.add_decl_unchecked(declaration);
        environment
    }

    #[test]
    fn a_repeated_diamond_dag_is_rejected_before_the_checker()
    {
        // A ~28-level diamond has a small table but an astronomical expanded
        // size (2^29); the reader rejects it by the expanded-work budget BEFORE
        // replay. The differential: `read` fails on the DECODE plane
        // (ReadError::Decode), never reaching the choke point / checker.
        /// The diamond depth whose expanded size exceeds the expanded-work
        /// budget.
        const DIAMOND_DEPTH: FixtureDepth = FixtureDepth(28);
        let environment = diamond_environment(DIAMOND_DEPTH);
        let bytes = write(&environment);
        assert!(
            bytes.len() < 512,
            "the diamond artifact is small ({} bytes) despite huge expanded size",
            bytes.len()
        );
        assert_eq!(
            DecodeError::Malformed {
                site: MalformedSite::ExpandedWork,
            },
            decode(bytes.as_ref().into()).unwrap_err(),
            "the repeated-diamond DAG is rejected by the expanded-work budget"
        );
        match read(bytes.as_ref().into()) {
            | Err(ReadError::Decode(DecodeError::Malformed {
                site: MalformedSite::ExpandedWork,
            })) => {},
            | other => {
                panic!("the diamond must fail on the decode plane, not the checker: {other:?}")
            },
        }
    }

    #[test]
    fn the_expanded_work_boundary_accepts_just_under_and_rejects_just_over()
    {
        // A `depth`-level diamond has expanded size 2^(depth+1) - 1; with the
        // cap a power of two, `depth = log2(cap) - 1` lands the diamond exactly
        // at MAX_EXPANDED_TERM_WORK - 1 (accepted), and wrapping it in one more
        // pair with a unit pushes it to MAX + 1 (rejected). Tests the decode
        // plane (structure + budget); the depth tracks the D3-tuned constant.
        assert!(
            MAX_EXPANDED_TERM_WORK.is_power_of_two(),
            "the boundary golden assumes a power-of-two expanded-work cap"
        );
        let depth = usize::try_from(MAX_EXPANDED_TERM_WORK.trailing_zeros().saturating_sub(1))
            .expect("the depth fits");
        let under = write(&diamond_environment(FixtureDepth(depth)));
        assert!(
            decode(under.as_ref().into()).is_ok(),
            "expanded size just under the cap decodes"
        );
        let mut over_env = Environment::new();
        {
            let mut builder = over_env.stage();
            let arena = builder.arena();
            let mut node = arena.value_unit();
            for _step in 0 .. depth {
                node = arena.value_pair(node, node);
            }
            let extra = arena.value_unit();
            let body = arena.value_pair(node, extra); // expanded size lands one over the cap
            let declared = arena.value_type_unit();
            let declaration = builder.def(LevelSignature::monomorphic(), declared, body);
            over_env.add_decl_unchecked(declaration);
        };
        let over = write(&over_env);
        assert_eq!(
            DecodeError::Malformed {
                site: MalformedSite::ExpandedWork,
            },
            decode(over.as_ref().into()).unwrap_err(),
            "expanded size just over the cap is rejected"
        );
    }

    #[test]
    fn the_table_size_boundary_accepts_at_the_cap_and_rejects_over()
    {
        // A left-nested pair chain of distinct entries: the accept case has
        // exactly MAX_TABLE_ENTRIES entries, the reject case one more. The chain
        // expands to ~2·cap tree-nodes, which stays under MAX_EXPANDED_TERM_WORK
        // (the table cap is the smaller, distinct-node axis), so only the
        // table-size arm fires. Tracks the D3-tuned constant.
        let cap = MAX_TABLE_ENTRIES;
        let build = |entries: usize| {
            let mut environment = Environment::new();
            let mut builder = environment.stage();
            let arena = builder.arena();
            // entries = 1 declared-type unit + 1 body-unit + (entries - 2) pairs.
            let mut node = arena.value_unit();
            let pairs = entries.saturating_sub(2);
            for _step in 0 .. pairs {
                let unit = arena.value_unit();
                node = arena.value_pair(node, unit);
            }
            let declared = arena.value_type_unit();
            let declaration = builder.def(LevelSignature::monomorphic(), declared, node);
            environment.add_decl_unchecked(declaration);
            write(&environment)
        };
        assert!(
            decode(build(cap).as_ref().into()).is_ok(),
            "a table at exactly MAX_TABLE_ENTRIES decodes"
        );
        assert_eq!(
            DecodeError::Malformed {
                site: MalformedSite::TableSize,
            },
            decode(build(cap.saturating_add(1)).as_ref().into()).unwrap_err(),
            "a table one entry over the cap is rejected"
        );
    }

    // ----- The artifact-total amplification defence (§4.4, gandr-4p3). -----

    /// Build a bypass-admitted environment of `count` declarations, each a
    /// `def unit = <a fresh depth-`depth` pair diamond>`. The diamonds are
    /// structurally identical, so the content-keyed writer shares one diamond
    /// and every declaration root references it (cross-declaration sharing) —
    /// the `N`-cheap-segments-sharing-one-near-cap-root shape gandr-4p3
    /// defends.
    fn shared_diamond_environment(
        count: FixtureCount,
        depth: FixtureDepth,
    ) -> Environment
    {
        let mut environment = Environment::new();
        for _decl in 0 .. count.0 {
            let mut builder = environment.stage();
            let body = {
                let arena = builder.arena();
                let mut node = arena.value_unit();
                for _step in 0 .. depth.0 {
                    node = arena.value_pair(node, node);
                }
                node
            };
            let declared = builder.arena().value_type_unit();
            let declaration = builder.def(LevelSignature::monomorphic(), declared, body);
            environment.add_decl_unchecked(declaration);
        }
        environment
    }

    #[test]
    fn the_artifact_work_boundary_accepts_just_under_and_rejects_just_over()
    {
        // Each `def unit = diamond(depth)` contributes 1 (declared unit) +
        // (2^(depth+1) - 1) (shared body) = 2^(depth+1) to the artifact-total.
        // With `depth = log2(MAX_EXPANDED_TERM_WORK) - 2` the per-declaration
        // body (2^(depth+1) - 1) stays strictly under MAX_EXPANDED_TERM_WORK, so
        // the per-declaration arm never fires; `count` declarations sum to
        // count·2^(depth+1), and choosing count so the sum is exactly
        // MAX_ARTIFACT_EXPANDED_WORK accepts, one more rejects. Both parameters
        // track the D3-tuned constants.
        assert!(
            MAX_ARTIFACT_EXPANDED_WORK.is_power_of_two()
                && MAX_EXPANDED_TERM_WORK.is_power_of_two(),
            "the boundary golden assumes power-of-two work caps"
        );
        let depth = usize::try_from(MAX_EXPANDED_TERM_WORK.trailing_zeros().saturating_sub(2))
            .expect("the depth fits");
        let shift = u32::try_from(depth.saturating_add(1)).expect("the shift fits in u32");
        let contribution = 1_u64.checked_shl(shift).expect("the shift is in range"); // = MAX_EXPANDED_TERM_WORK / 2
        let count = usize::try_from(MAX_ARTIFACT_EXPANDED_WORK.div_euclid(contribution))
            .expect("count fits");
        // Just at the cap: accepts (the artifact arm rejects strictly over).
        let at_cap = write(&shared_diamond_environment(
            FixtureCount(count),
            FixtureDepth(depth),
        ));
        assert!(
            decode(at_cap.as_ref().into()).is_ok(),
            "artifact-total work at exactly the cap decodes"
        );
        // One more declaration pushes the total over.
        let over = write(&shared_diamond_environment(
            FixtureCount(count.saturating_add(1)),
            FixtureDepth(depth),
        ));
        assert_eq!(
            DecodeError::Malformed {
                site: MalformedSite::ArtifactExpandedWork,
            },
            decode(over.as_ref().into()).unwrap_err(),
            "artifact-total work over the cap is rejected"
        );
    }

    #[test]
    fn a_many_segment_amplification_is_rejected_before_the_checker()
    {
        // Many cheap declaration segments sharing one near-per-declaration-cap
        // diamond: the artifact is small in bytes but its artifact-total
        // expanded work is astronomical. The reader rejects it on the DECODE
        // plane by the artifact-total budget BEFORE any replay reaches the
        // checker (the gandr-4p3 differential).
        let depth = usize::try_from(MAX_EXPANDED_TERM_WORK.trailing_zeros().saturating_sub(1))
            .expect("the depth fits");
        let bytes = write(&shared_diamond_environment(
            FixtureCount(512),
            FixtureDepth(depth),
        ));
        assert!(
            bytes.len() < 8192,
            "the many-segment artifact is small ({} bytes) despite astronomical artifact work",
            bytes.len()
        );
        assert_eq!(
            DecodeError::Malformed {
                site: MalformedSite::ArtifactExpandedWork,
            },
            decode(bytes.as_ref().into()).unwrap_err(),
            "the many-segment amplification is rejected by the artifact-total budget"
        );
        match read(bytes.as_ref().into()) {
            | Err(ReadError::Decode(DecodeError::Malformed {
                site: MalformedSite::ArtifactExpandedWork,
            })) => {},
            | other => {
                panic!(
                    "the amplification must fail on the decode plane, not the checker: {other:?}"
                )
            },
        }
    }

    // ----- Reader totality on adversarial bytes. -----

    #[test]
    fn an_overlong_varint_is_rejected()
    {
        let mut bytes = v1_header(WireValue(0));
        bytes.splice(6 .. 7, [0x80, 0x00]);
        assert_eq!(
            DecodeError::Malformed {
                site: MalformedSite::Varint,
            },
            decode(bytes.as_slice().into()).unwrap_err(),
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
        let mut bytes = Vec::from(write(&environment));
        // The single entry's tag is a NODE_VT_UNIT byte; corrupt it.
        let position = bytes
            .iter()
            .rposition(|&byte| byte == NODE_VT_UNIT)
            .expect("the unit entry tag is present");
        bytes[position] = 0xFF; // an out-of-vocabulary node tag
        assert!(
            matches!(
                decode(bytes.as_slice().into()),
                Err(DecodeError::UnknownTag { .. })
            ),
            "a corrupted node tag is rejected as an unknown tag"
        );
    }

    #[test]
    fn truncation_at_every_prefix_is_rejected()
    {
        let bytes = write(&rich_checked_environment());
        for length in 0 .. bytes.len() {
            let prefix = &bytes[.. length];
            assert!(
                decode(prefix.into()).is_err(),
                "the {length}-byte proper prefix must be rejected, never a false success"
            );
        }
        assert!(
            decode(bytes.as_ref().into()).is_ok(),
            "the full artifact still decodes"
        );
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
                let left = var_plus(left_var.into(), left_offset.into());
                let right = var_plus(right_var.into(), right_offset.into());
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
            | &DeclSpec::Def(ref signature, ref declared, ref body) => {
                let declaration = common::stage_def(environment, signature.clone(), declared, body);
                let _id = environment.add_decl_unchecked(declaration);
                signature.clone()
            },
            | &DeclSpec::Axiom(ref signature, ref declared) => {
                let declaration = common::stage_axiom(environment, signature.clone(), declared);
                let _id = environment.add_decl_unchecked(declaration);
                signature.clone()
            },
        }
    }

    proptest! {
        /// An environment of arbitrary declarations round-trips: the recovered
        /// artifact is byte-identical (which witnesses content equality via the
        /// canonical form, E4), and each level signature and admission mark
        /// survives.
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
            let recovered =
                read(bytes.as_ref().into()).expect("a genuine artifact must read");
            prop_assert!(write(&recovered) == bytes, "the round trip is byte-stable");

            let artifact = decode(bytes.as_ref().into()).unwrap();
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
        /// successful decode round-trips to the same bytes.
        #[test]
        fn arbitrary_bytes_never_panic(raw in prop::collection::vec(any::<u8>(), 0 .. 128))
        {
            match decode(raw.as_slice().into()) {
                | Ok(_artifact) => {
                    prop_assert!(
                        read(raw.as_slice().into()).is_ok(),
                        "a decodable artifact also reads"
                    );
                },
                | Err(_error) => {
                    prop_assert!(
                        read(raw.as_slice().into()).is_err(),
                        "a rejected artifact also fails to read"
                    );
                },
            }
        }
    }
}
