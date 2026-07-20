//! The validating export reader (kernel-boundary.md §5, E1–E6): decode
//! canonical bytes back to a declaration sequence and replay it through the
//! choke point.
//!
//! # Decode through constructors, total on adversarial bytes
//!
//! The reader is a closed-vocabulary parser. Every read is bounds-checked (a
//! short artifact surfaces [`DecodeError::Truncated`]); every tag is matched
//! against a closed set (an out-of-vocabulary byte surfaces
//! [`DecodeError::UnknownTag`]); every value is rebuilt through its domain
//! **constructor**, so a non-canonical state never materializes:
//!
//! * **Levels** are rebuilt through the [`gandr_kernel_strata`] smart
//!   constructors (`constant`/`var`/`succ`/`max`), which keep canonical form —
//!   a non-canonical level is unrepresentable in memory. This re-arms, at the
//!   B2.2 decode boundary, the B2.1 obligation that the kernel rejects
//!   non-canonical levels (the crate has no non-canonical-level error variant
//!   because such a level cannot be built).
//! * **Landmark constraints** are rebuilt through `LandmarkConstraint::leq` /
//!   `equal`, which reject a non-variable-only side.
//! * **Literals** are rebuilt through `Magnitude` / `FractionDigits` /
//!   `IntegerLiteral` / `NumericLiteral` / `StringLiteral`, which normalize and
//!   reject non-digit payloads.
//! * **Terms and types** are rebuilt through the kernel-core constructors, with
//!   the whole artifact held to a **canonical-bytes check**: the decoded
//!   sequence is re-encoded and compared to the input, so a non-canonical
//!   *encoding* (a padded literal, an unsorted or dominated level atom, an
//!   overlong varint) is rejected as [`DecodeError::Malformed`] rather than
//!   silently normalized (E4).
//!
//! Term and type trees decode **iteratively** over an explicit frame worklist
//! (the [`crate::conv`] precedent), never input-scaled recursion, so an
//! adversarially deep artifact is rejected or decoded without a stack overflow.
//! Level reconstruction is bounded by [`super::MAX_DECODED_LEVEL_OFFSET`].

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

use gandr_kernel_strata::ConstraintRelation;
use gandr_kernel_strata::LandmarkConstraint;
use gandr_kernel_strata::Level;
use gandr_kernel_strata::LevelConstant;
use gandr_kernel_strata::LevelVar;
use gandr_kernel_strata::LevelVarIndex;

use super::ADMISSION_CHECKED;
use super::ADMISSION_UNCHECKED;
use super::AdmissionMark;
use super::BASE_INTEGER;
use super::BASE_NUMERIC;
use super::BASE_STRING;
use super::DecodeError;
use super::DecodedDeclaration;
use super::FORMAT_VERSION_V0;
use super::KIND_ABSTRACT_TYPE;
use super::KIND_AXIOM;
use super::KIND_DEF;
use super::KIND_FUNCTOR_DEF;
use super::KIND_MODULE_DEF;
use super::KIND_MODULE_SIG;
use super::LITERAL_INTEGER;
use super::LITERAL_NUMERIC;
use super::LITERAL_TEXT;
use super::MAGIC;
use super::MAX_DECODED_LEVEL_OFFSET;
use super::MalformedSite;
use super::RELATION_EQ;
use super::RELATION_LEQ;
use super::ReadError;
use super::ReservedKind;
use super::ReservedSlot;
use super::SIDE_LEFT;
use super::SIDE_RIGHT;
use super::SIGN_NEGATIVE;
use super::SIGN_NON_NEGATIVE;
use super::TERM_APPLICATION;
use super::TERM_BIND;
use super::TERM_CASE;
use super::TERM_CONSTANT;
use super::TERM_FORCE;
use super::TERM_INJECTION;
use super::TERM_LAMBDA;
use super::TERM_LIFT;
use super::TERM_LITERAL;
use super::TERM_PAIR;
use super::TERM_RETURN;
use super::TERM_THUNK;
use super::TERM_UNIT;
use super::TERM_VARIABLE;
use super::TYPE_ARROW;
use super::TYPE_BASE;
use super::TYPE_LIFT;
use super::TYPE_PRODUCT;
use super::TYPE_RETURNER;
use super::TYPE_SUM;
use super::TYPE_THUNK;
use super::TYPE_UNIT;
use super::TYPE_UNIVERSE;
use super::TagSite;
use super::write::encode_artifact;
use crate::base::BaseType;
use crate::base::FractionDigits;
use crate::base::IntegerLiteral;
use crate::base::Literal;
use crate::base::Magnitude;
use crate::base::NumericLiteral;
use crate::base::Sign;
use crate::base::StringLiteral;
use crate::decl::Declaration;
use crate::decl::LevelSignature;
use crate::env::Environment;
use crate::levels::LevelParamCount;
use crate::term::Computation;
use crate::term::ConstantIndex;
use crate::term::DeBruijnIndex;
use crate::term::Side;
use crate::term::Value;
use crate::types::CompType;
use crate::types::ValueType;

/// Decode a byte artifact into its admission-ordered declaration sequence
/// (kernel-boundary.md §5, the validating reader).
///
/// # Contract
/// - requires: nothing — `bytes` may be arbitrary/adversarial.
/// - ensures: `Ok(sequence)` with the declarations in admission order, each
///   carrying its admission mark (E6), exactly when the bytes are the canonical
///   encoding of a v0 artifact whose levels, constraints, literals, terms, and
///   types all rebuild through their constructors; every level is canonical by
///   construction (E4).
/// - provides: the K5 re-checkable decode — a total parser over the closed
///   vocabulary.
/// - fails: [`DecodeError`] — the rejection triple (truncation, an unknown tag,
///   a structural violation including a non-canonical encoding), a reserved
///   declaration kind (R1), a non-empty reserved slot/section (R2/R3/R4), or an
///   unsupported version (E5). Never panics, never loops unboundedly.
/// - panics: none.
///
/// # Errors
/// Any [`DecodeError`].
///
/// # Adequacy
/// - hypothesis: L2 — the rejection-totality property pins that truncation at
///   every prefix and arbitrary random bytes return (never panic), and the
///   round-trip differential pins acceptance of every genuine artifact; the L3
///   residues are each named rejection (reserved kind, reserved slot, version
///   refusal, non-canonical level), pinned by hand-crafted goldens.
/// - witness: `export::tests::truncation_at_every_prefix_is_rejected`
/// - witness: `export::tests::arbitrary_bytes_never_panic`
/// - witness: `export::tests::a_non_canonical_level_encoding_is_rejected`
/// - witness: `export::tests::each_reserved_declaration_kind_is_rejected`
#[inline]
pub fn decode(bytes: &[u8]) -> Result<Vec<DecodedDeclaration>, DecodeError>
{
    let mut reader = ByteReader::new(bytes);
    reader.expect_magic()?;
    reader.expect_version()?;
    reader.expect_empty_minted_atom_table()?;
    let count = reader.read_uvarint()?;
    let mut declarations: Vec<DecodedDeclaration> = Vec::new();
    let mut remaining = count;
    while remaining > 0_u64 {
        let declaration = decode_declaration(&mut reader)?;
        declarations.push(declaration);
        remaining = remaining.wrapping_sub(1_u64);
    }
    if !reader.at_end() {
        return Err(DecodeError::Malformed {
            site: MalformedSite::TrailingBytes,
        });
    }
    let pairs: Vec<(AdmissionMark, &Declaration)> = declarations
        .iter()
        .map(|decoded| (decoded.mark, &decoded.declaration))
        .collect();
    if encode_artifact(&pairs) != bytes {
        return Err(DecodeError::Malformed {
            site: MalformedSite::NonCanonical,
        });
    }
    Ok(declarations)
}

/// Read a byte artifact into an [`Environment`] by replaying each declaration
/// through the choke point (kernel-boundary.md §5 E2/E3/E6).
///
/// # Contract
/// - requires: nothing — `bytes` may be arbitrary/adversarial.
/// - ensures: `Ok(environment)` exactly when the bytes [`decode`] and every
///   recovered declaration re-admits; a checked mark replays through
///   [`Environment::add_decl`] (re-deriving every obligation, K2) and a bypass
///   mark through [`Environment::add_decl_unchecked`], so the transitive audit
///   is recomputed rather than trusted (E3) and the bypass marks survive (E6).
/// - provides: the K5 replay — an independent re-checker's admission path.
/// - fails: [`ReadError::Decode`] on a format failure, [`ReadError::Admit`]
///   when a recovered declaration does not re-admit (a typing failure, held
///   distinct from a decode failure).
/// - panics: none.
///
/// # Errors
/// [`ReadError::Decode`] or [`ReadError::Admit`].
///
/// # Adequacy
/// - hypothesis: L2 — the round-trip differential pins that `read(write(env))`
///   reproduces the environment (declarations, marks, and recomputed audits);
///   the L3 residue is the bypass replay path, pinned by a bypass-admitted
///   fixture whose unchecked mark survives into the recomputed audit.
/// - witness: `export::tests::round_trip_reproduces_the_environment`
/// - witness: `export::tests::a_bypass_admission_survives_the_round_trip`
#[inline]
pub fn read(bytes: &[u8]) -> Result<Environment, ReadError>
{
    let declarations = decode(bytes)?;
    let mut environment = Environment::new();
    for decoded in declarations {
        let DecodedDeclaration { mark, declaration } = decoded;
        match mark {
            | AdmissionMark::Checked => {
                let _checked = environment.add_decl(declaration)?;
            },
            | AdmissionMark::UncheckedBypass => {
                let _checked = environment.add_decl_unchecked(declaration);
            },
        }
    }
    Ok(environment)
}

/// A forward byte cursor with bounds-checked reads — the reader's totality
/// substrate (an over-read surfaces [`DecodeError::Truncated`]).
struct ByteReader<'bytes>
{
    /// The artifact bytes.
    bytes: &'bytes [u8],
    /// The next unread offset.
    position: usize,
}

impl<'bytes> ByteReader<'bytes>
{
    /// A cursor at the start of `bytes`.
    #[inline]
    fn new(bytes: &'bytes [u8]) -> Self
    {
        Self { bytes, position: 0 }
    }

    /// Whether every byte has been consumed.
    #[inline]
    fn at_end(&self) -> bool
    {
        self.position >= self.bytes.len()
    }

    /// Read one byte, or [`DecodeError::Truncated`] at the end.
    #[inline]
    fn next_byte(&mut self) -> Result<u8, DecodeError>
    {
        let byte = *self
            .bytes
            .get(self.position)
            .ok_or(DecodeError::Truncated)?;
        self.position = self.position.checked_add(1).ok_or(DecodeError::Truncated)?;
        Ok(byte)
    }

    /// Read `count` bytes as a borrowed slice, or [`DecodeError::Truncated`].
    #[inline]
    fn take(
        &mut self,
        count: usize,
    ) -> Result<&'bytes [u8], DecodeError>
    {
        let end = self
            .position
            .checked_add(count)
            .ok_or(DecodeError::Truncated)?;
        let slice = self
            .bytes
            .get(self.position .. end)
            .ok_or(DecodeError::Truncated)?;
        self.position = end;
        Ok(slice)
    }

    /// Verify the four-byte magic (E1: the artifact is a gandr kernel export).
    #[inline]
    fn expect_magic(&mut self) -> Result<(), DecodeError>
    {
        let head = self.take(MAGIC.len())?;
        if head == MAGIC {
            Ok(())
        }
        else {
            Err(DecodeError::Malformed {
                site: MalformedSite::Header,
            })
        }
    }

    /// Verify the version tag, refusing an unimplemented version (E5).
    #[inline]
    fn expect_version(&mut self) -> Result<(), DecodeError>
    {
        let high = self.next_byte()?;
        let low = self.next_byte()?;
        let found = u16::from_be_bytes([high, low]);
        if found == FORMAT_VERSION_V0 {
            Ok(())
        }
        else {
            Err(DecodeError::UnsupportedVersion { found })
        }
    }

    /// Verify the reserved minted-atom table is empty (R4).
    #[inline]
    fn expect_empty_minted_atom_table(&mut self) -> Result<(), DecodeError>
    {
        let count = self.read_uvarint()?;
        if count == 0_u64 {
            Ok(())
        }
        else {
            Err(DecodeError::ReservedSlotOccupied {
                slot: ReservedSlot::MintedAtomTable,
            })
        }
    }

    /// Read a canonical (minimal) unsigned LEB128 varint.
    ///
    /// # Contract
    /// - requires: nothing.
    /// - ensures: `Ok(value)` for the unique minimal little-endian base-128
    ///   encoding of a `u64`.
    /// - provides: the reader's integer primitive.
    /// - fails: [`DecodeError::Truncated`] at the end;
    ///   [`DecodeError::Malformed`] (`Varint`) on an overlong (non-minimal)
    ///   encoding or one exceeding the `u64` range.
    /// - panics: none.
    fn read_uvarint(&mut self) -> Result<u64, DecodeError>
    {
        let overlong = DecodeError::Malformed {
            site: MalformedSite::Varint,
        };
        let mut result: u64 = 0;
        let mut shift: u32 = 0;
        let mut count: u32 = 0;
        loop {
            let byte = self.next_byte()?;
            count = count.checked_add(1).ok_or(overlong)?;
            if count > 10 {
                return Err(overlong);
            }
            let low = u64::from(byte & 0x7f);
            if shift == 63 && low > 1 {
                return Err(overlong);
            }
            let contribution = low.checked_shl(shift).ok_or(overlong)?;
            result |= contribution;
            if byte & 0x80 == 0 {
                if count > 1 && byte == 0 {
                    return Err(overlong);
                }
                return Ok(result);
            }
            shift = shift.checked_add(7).ok_or(overlong)?;
        }
    }

    /// Read a `u32`, rejecting an out-of-range varint.
    #[inline]
    fn read_u32(&mut self) -> Result<u32, DecodeError>
    {
        let value = self.read_uvarint()?;
        u32::try_from(value).map_err(|_error| DecodeError::Malformed {
            site: MalformedSite::IndexRange,
        })
    }

    /// Read a `usize`, rejecting an out-of-range varint.
    #[inline]
    fn read_usize(&mut self) -> Result<usize, DecodeError>
    {
        let value = self.read_uvarint()?;
        usize::try_from(value).map_err(|_error| DecodeError::Malformed {
            site: MalformedSite::IndexRange,
        })
    }

    /// Read length-prefixed UTF-8 text through a validating conversion.
    #[inline]
    fn read_text(&mut self) -> Result<String, DecodeError>
    {
        let length = self.read_usize()?;
        let bytes = self.take(length)?;
        let text = core::str::from_utf8(bytes).map_err(|_error| DecodeError::Malformed {
            site: MalformedSite::LiteralPayload,
        })?;
        Ok(String::from(text))
    }
}

/// Decode one declaration record.
fn decode_declaration(reader: &mut ByteReader<'_>) -> Result<DecodedDeclaration, DecodeError>
{
    let mark = decode_admission(reader)?;
    let kind = reader.next_byte()?;
    reject_reserved_kind(kind)?;
    decode_empty_name(reader)?;
    let signature = decode_level_signature(reader)?;
    let declaration = match kind {
        | KIND_DEF => {
            let declared = decode_value_type(reader)?;
            let body = decode_value(reader)?;
            decode_empty_def_slots(reader)?;
            Declaration::def(signature, declared, body)
        },
        | KIND_AXIOM => {
            let declared = decode_value_type(reader)?;
            Declaration::axiom(signature, declared)
        },
        | other => {
            return Err(DecodeError::UnknownTag {
                site: TagSite::DeclarationKind,
                tag: other,
            });
        },
    };
    Ok(DecodedDeclaration::new(mark, declaration))
}

/// Decode a declaration's admission mark (E6).
#[inline]
fn decode_admission(reader: &mut ByteReader<'_>) -> Result<AdmissionMark, DecodeError>
{
    match reader.next_byte()? {
        | ADMISSION_CHECKED => Ok(AdmissionMark::Checked),
        | ADMISSION_UNCHECKED => Ok(AdmissionMark::UncheckedBypass),
        | other => Err(DecodeError::UnknownTag {
            site: TagSite::Admission,
            tag: other,
        }),
    }
}

/// Reject a reserved declaration kind (R1), distinctly from a bad tag.
#[inline]
fn reject_reserved_kind(kind: u8) -> Result<(), DecodeError>
{
    let reserved = match kind {
        | KIND_ABSTRACT_TYPE => ReservedKind::AbstractType,
        | KIND_MODULE_SIG => ReservedKind::ModuleSig,
        | KIND_MODULE_DEF => ReservedKind::ModuleDef,
        | KIND_FUNCTOR_DEF => ReservedKind::FunctorDef,
        | _ => return Ok(()),
    };
    Err(DecodeError::ReservedDeclarationKind { kind: reserved })
}

/// Decode the structured-name record, requiring it empty at v0 (R2).
#[inline]
fn decode_empty_name(reader: &mut ByteReader<'_>) -> Result<(), DecodeError>
{
    if reader.read_uvarint()? == 0_u64 {
        Ok(())
    }
    else {
        Err(DecodeError::ReservedSlotOccupied {
            slot: ReservedSlot::StructuredName,
        })
    }
}

/// Decode the four per-Def annotation slots, requiring each empty at v0 (R3).
#[inline]
fn decode_empty_def_slots(reader: &mut ByteReader<'_>) -> Result<(), DecodeError>
{
    let slots = [
        ReservedSlot::ErasureAnnotation,
        ReservedSlot::ModeGradeAnnotation,
        ReservedSlot::SealingProvenance,
        ReservedSlot::DirectednessVariance,
    ];
    for slot in slots {
        if reader.read_uvarint()? != 0_u64 {
            return Err(DecodeError::ReservedSlotOccupied { slot });
        }
    }
    Ok(())
}

/// Decode a prenex level signature: parameter count and landmark constraints,
/// each rebuilt through the strata smart constructor.
fn decode_level_signature(reader: &mut ByteReader<'_>) -> Result<LevelSignature, DecodeError>
{
    let params = reader.read_u32()?;
    let count = reader.read_uvarint()?;
    let mut constraints: Vec<LandmarkConstraint> = Vec::new();
    let mut remaining = count;
    while remaining > 0_u64 {
        let relation = decode_relation(reader)?;
        let left = decode_level(reader)?;
        let right = decode_level(reader)?;
        let constraint = match relation {
            | ConstraintRelation::Leq => LandmarkConstraint::leq(left, right),
            | ConstraintRelation::Eq => LandmarkConstraint::equal(left, right),
        };
        let constraint = constraint.map_err(|_error| DecodeError::Malformed {
            site: MalformedSite::ConstraintForm,
        })?;
        constraints.push(constraint);
        remaining = remaining.wrapping_sub(1_u64);
    }
    Ok(LevelSignature::new(
        LevelParamCount::from(params),
        constraints,
    ))
}

/// Decode a landmark-constraint relation byte.
#[inline]
fn decode_relation(reader: &mut ByteReader<'_>) -> Result<ConstraintRelation, DecodeError>
{
    match reader.next_byte()? {
        | RELATION_LEQ => Ok(ConstraintRelation::Leq),
        | RELATION_EQ => Ok(ConstraintRelation::Eq),
        | other => Err(DecodeError::UnknownTag {
            site: TagSite::ConstraintRelation,
            tag: other,
        }),
    }
}

/// Decode a canonical level, rebuilt through the strata smart constructors so a
/// non-canonical level is unrepresentable (E4; the B2.1 obligation re-armed).
///
/// # Contract
/// - requires: nothing.
/// - ensures: `Ok(level)` — always canonical, since it is built from
///   `constant`/`var`/`succ`/`max` — when the constant, atom count, and each
///   `(variable, offset)` pair decode and no offset meets
///   [`MAX_DECODED_LEVEL_OFFSET`].
/// - provides: the level decoder for universes, lifts, and constraint sides.
/// - fails: [`DecodeError::Truncated`]; [`DecodeError::Malformed`]
///   (`LevelOffset`) on an over-cap offset or a reconstruction overflow,
///   (`IndexRange`) on an out-of-range variable index.
/// - panics: none.
fn decode_level(reader: &mut ByteReader<'_>) -> Result<Level, DecodeError>
{
    let constant = reader.read_uvarint()?;
    let atom_count = reader.read_uvarint()?;
    let mut level = Level::constant(LevelConstant::from(constant));
    let mut remaining = atom_count;
    while remaining > 0_u64 {
        let variable = reader.read_u32()?;
        let offset = reader.read_uvarint()?;
        if offset >= MAX_DECODED_LEVEL_OFFSET {
            return Err(DecodeError::Malformed {
                site: MalformedSite::LevelOffset,
            });
        }
        let atom = build_variable_atom(variable, offset)?;
        level = level.max(&atom);
        remaining = remaining.wrapping_sub(1_u64);
    }
    Ok(level)
}

/// Rebuild the level atom `variable + offset` through `var` and `succ` (strata
/// exposes no direct offset constructor); `offset` is bounded by the caller.
#[inline]
fn build_variable_atom(
    variable: u32,
    offset: u64,
) -> Result<Level, DecodeError>
{
    let mut atom = Level::var(LevelVar::new(LevelVarIndex::from(variable)));
    let mut remaining = offset;
    while remaining > 0_u64 {
        atom = atom.succ().map_err(|_error| DecodeError::Malformed {
            site: MalformedSite::LevelOffset,
        })?;
        remaining = remaining.wrapping_sub(1_u64);
    }
    Ok(atom)
}

/// A decoded type node, tagged by polarity.
enum TypeNode
{
    /// A value type.
    Value(ValueType),
    /// A computation type.
    Comp(CompType),
}

/// A pending type-constructor frame awaiting its remaining children.
enum TypeFrame
{
    /// A product awaiting its first (`None`) or second (`Some`) value type.
    Product(Option<ValueType>),
    /// A sum awaiting its first (`None`) or second (`Some`) value type.
    Sum(Option<ValueType>),
    /// A thunk awaiting its computation type.
    Thunk,
    /// A lift awaiting its inner value type; its target level is decoded.
    Lift(Level),
    /// A returner awaiting its result value type.
    Returner,
    /// An arrow awaiting its domain value type (`None`) or codomain
    /// computation type (`Some`).
    Arrow(Option<ValueType>),
}

/// Decode a value type — the declaration's declared type is always value
/// polarity.
#[inline]
fn decode_value_type(reader: &mut ByteReader<'_>) -> Result<ValueType, DecodeError>
{
    expect_value_type(decode_type_tree(reader)?)
}

/// Decode a type tree iteratively over an explicit frame worklist.
fn decode_type_tree(reader: &mut ByteReader<'_>) -> Result<TypeNode, DecodeError>
{
    let mut frames: Vec<TypeFrame> = Vec::new();
    'read: loop {
        let tag = reader.next_byte()?;
        let mut produced = match tag {
            | TYPE_BASE => TypeNode::Value(ValueType::Base(decode_base_type(reader)?)),
            | TYPE_UNIT => TypeNode::Value(ValueType::Unit),
            | TYPE_UNIVERSE => TypeNode::Value(ValueType::Universe(decode_level(reader)?)),
            | TYPE_PRODUCT => {
                frames.push(TypeFrame::Product(None));
                continue 'read;
            },
            | TYPE_SUM => {
                frames.push(TypeFrame::Sum(None));
                continue 'read;
            },
            | TYPE_THUNK => {
                frames.push(TypeFrame::Thunk);
                continue 'read;
            },
            | TYPE_LIFT => {
                let target = decode_level(reader)?;
                frames.push(TypeFrame::Lift(target));
                continue 'read;
            },
            | TYPE_RETURNER => {
                frames.push(TypeFrame::Returner);
                continue 'read;
            },
            | TYPE_ARROW => {
                frames.push(TypeFrame::Arrow(None));
                continue 'read;
            },
            | other => {
                return Err(DecodeError::UnknownTag {
                    site: TagSite::Type,
                    tag: other,
                });
            },
        };
        loop {
            let Some(frame) = frames.pop()
            else {
                return Ok(produced);
            };
            match frame {
                | TypeFrame::Product(None) => {
                    frames.push(TypeFrame::Product(Some(expect_value_type(produced)?)));
                    continue 'read;
                },
                | TypeFrame::Product(Some(first)) => {
                    let second = expect_value_type(produced)?;
                    produced =
                        TypeNode::Value(ValueType::Product(Box::new(first), Box::new(second)));
                },
                | TypeFrame::Sum(None) => {
                    frames.push(TypeFrame::Sum(Some(expect_value_type(produced)?)));
                    continue 'read;
                },
                | TypeFrame::Sum(Some(first)) => {
                    let second = expect_value_type(produced)?;
                    produced = TypeNode::Value(ValueType::Sum(Box::new(first), Box::new(second)));
                },
                | TypeFrame::Thunk => {
                    let body = expect_comp_type(produced)?;
                    produced = TypeNode::Value(ValueType::Thunk(Box::new(body)));
                },
                | TypeFrame::Lift(target) => {
                    let inner = expect_value_type(produced)?;
                    produced = TypeNode::Value(ValueType::Lift {
                        inner: Box::new(inner),
                        target,
                    });
                },
                | TypeFrame::Returner => {
                    let result = expect_value_type(produced)?;
                    produced = TypeNode::Comp(CompType::Returner(Box::new(result)));
                },
                | TypeFrame::Arrow(None) => {
                    frames.push(TypeFrame::Arrow(Some(expect_value_type(produced)?)));
                    continue 'read;
                },
                | TypeFrame::Arrow(Some(domain)) => {
                    let codomain = expect_comp_type(produced)?;
                    produced = TypeNode::Comp(CompType::Arrow {
                        domain: Box::new(domain),
                        codomain: Box::new(codomain),
                    });
                },
            }
        }
    }
}

/// Require a value-type node, rejecting a polarity mismatch.
#[inline]
fn expect_value_type(node: TypeNode) -> Result<ValueType, DecodeError>
{
    match node {
        | TypeNode::Value(value_type) => Ok(value_type),
        | TypeNode::Comp(_) => Err(DecodeError::Malformed {
            site: MalformedSite::Polarity,
        }),
    }
}

/// Require a computation-type node, rejecting a polarity mismatch.
#[inline]
fn expect_comp_type(node: TypeNode) -> Result<CompType, DecodeError>
{
    match node {
        | TypeNode::Comp(comp_type) => Ok(comp_type),
        | TypeNode::Value(_) => Err(DecodeError::Malformed {
            site: MalformedSite::Polarity,
        }),
    }
}

/// Decode a base-type atom byte.
#[inline]
fn decode_base_type(reader: &mut ByteReader<'_>) -> Result<BaseType, DecodeError>
{
    match reader.next_byte()? {
        | BASE_INTEGER => Ok(BaseType::Integer),
        | BASE_STRING => Ok(BaseType::String),
        | BASE_NUMERIC => Ok(BaseType::Numeric),
        | other => Err(DecodeError::UnknownTag {
            site: TagSite::BaseType,
            tag: other,
        }),
    }
}

/// A decoded term node, tagged by polarity.
enum TermNode
{
    /// A value.
    Value(Value),
    /// A computation.
    Comp(Computation),
}

/// A pending term-constructor frame awaiting its remaining children.
enum TermFrame
{
    /// A pair awaiting its first (`None`) or second (`Some`) value.
    Pair(Option<Value>),
    /// An injection awaiting its body value; its side is decoded.
    Injection(Side),
    /// A thunk awaiting its computation.
    Thunk,
    /// A value lift awaiting its body value; its target level is decoded.
    Lift(Level),
    /// A lambda awaiting its body computation.
    Lambda,
    /// An application awaiting its head computation (`None`) or argument value
    /// (`Some`).
    Application(Option<Computation>),
    /// A returner awaiting its value.
    Return,
    /// A bind awaiting its bound computation (`None`) or body computation
    /// (`Some`).
    Bind(Option<Computation>),
    /// A force awaiting its value.
    Force,
    /// A case awaiting its scrutinee value.
    CaseScrutinee,
    /// A case awaiting its left branch; its scrutinee is decoded.
    CaseLeft(Value),
    /// A case awaiting its right branch; its scrutinee and left branch are
    /// decoded.
    CaseRight(Value, Computation),
}

/// Decode a value term — a definition body is always value polarity.
#[inline]
fn decode_value(reader: &mut ByteReader<'_>) -> Result<Value, DecodeError>
{
    expect_value_term(decode_term_tree(reader)?)
}

/// Decode a term tree iteratively over an explicit frame worklist.
#[expect(
    clippy::too_many_lines,
    reason = "the flat iterative term decoder enumerates every value and computation former in one worklist loop; splitting it would obscure the single reduction machine"
)]
fn decode_term_tree(reader: &mut ByteReader<'_>) -> Result<TermNode, DecodeError>
{
    let mut frames: Vec<TermFrame> = Vec::new();
    'read: loop {
        let tag = reader.next_byte()?;
        let mut produced = match tag {
            | TERM_VARIABLE => {
                TermNode::Value(Value::Variable(DeBruijnIndex::from(reader.read_u32()?)))
            },
            | TERM_CONSTANT => {
                TermNode::Value(Value::Constant(ConstantIndex::from(reader.read_usize()?)))
            },
            | TERM_UNIT => TermNode::Value(Value::Unit),
            | TERM_LITERAL => TermNode::Value(Value::Literal(decode_literal(reader)?)),
            | TERM_PAIR => {
                frames.push(TermFrame::Pair(None));
                continue 'read;
            },
            | TERM_INJECTION => {
                let side = decode_side(reader)?;
                frames.push(TermFrame::Injection(side));
                continue 'read;
            },
            | TERM_THUNK => {
                frames.push(TermFrame::Thunk);
                continue 'read;
            },
            | TERM_LIFT => {
                let target = decode_level(reader)?;
                frames.push(TermFrame::Lift(target));
                continue 'read;
            },
            | TERM_LAMBDA => {
                frames.push(TermFrame::Lambda);
                continue 'read;
            },
            | TERM_APPLICATION => {
                frames.push(TermFrame::Application(None));
                continue 'read;
            },
            | TERM_RETURN => {
                frames.push(TermFrame::Return);
                continue 'read;
            },
            | TERM_BIND => {
                frames.push(TermFrame::Bind(None));
                continue 'read;
            },
            | TERM_FORCE => {
                frames.push(TermFrame::Force);
                continue 'read;
            },
            | TERM_CASE => {
                frames.push(TermFrame::CaseScrutinee);
                continue 'read;
            },
            | other => {
                return Err(DecodeError::UnknownTag {
                    site: TagSite::Term,
                    tag: other,
                });
            },
        };
        loop {
            let Some(frame) = frames.pop()
            else {
                return Ok(produced);
            };
            match frame {
                | TermFrame::Pair(None) => {
                    frames.push(TermFrame::Pair(Some(expect_value_term(produced)?)));
                    continue 'read;
                },
                | TermFrame::Pair(Some(first)) => {
                    let second = expect_value_term(produced)?;
                    produced = TermNode::Value(Value::Pair(Box::new(first), Box::new(second)));
                },
                | TermFrame::Injection(side) => {
                    let body = expect_value_term(produced)?;
                    produced = TermNode::Value(Value::Injection(side, Box::new(body)));
                },
                | TermFrame::Thunk => {
                    let body = expect_comp_term(produced)?;
                    produced = TermNode::Value(Value::Thunk(Box::new(body)));
                },
                | TermFrame::Lift(target) => {
                    let body = expect_value_term(produced)?;
                    produced = TermNode::Value(Value::Lift {
                        target,
                        body: Box::new(body),
                    });
                },
                | TermFrame::Lambda => {
                    let body = expect_comp_term(produced)?;
                    produced = TermNode::Comp(Computation::Lambda(Box::new(body)));
                },
                | TermFrame::Application(None) => {
                    frames.push(TermFrame::Application(Some(expect_comp_term(produced)?)));
                    continue 'read;
                },
                | TermFrame::Application(Some(head)) => {
                    let argument = expect_value_term(produced)?;
                    produced = TermNode::Comp(Computation::Application(
                        Box::new(head),
                        Box::new(argument),
                    ));
                },
                | TermFrame::Return => {
                    let value = expect_value_term(produced)?;
                    produced = TermNode::Comp(Computation::Return(Box::new(value)));
                },
                | TermFrame::Bind(None) => {
                    frames.push(TermFrame::Bind(Some(expect_comp_term(produced)?)));
                    continue 'read;
                },
                | TermFrame::Bind(Some(bound)) => {
                    let body = expect_comp_term(produced)?;
                    produced = TermNode::Comp(Computation::Bind(Box::new(bound), Box::new(body)));
                },
                | TermFrame::Force => {
                    let value = expect_value_term(produced)?;
                    produced = TermNode::Comp(Computation::Force(Box::new(value)));
                },
                | TermFrame::CaseScrutinee => {
                    frames.push(TermFrame::CaseLeft(expect_value_term(produced)?));
                    continue 'read;
                },
                | TermFrame::CaseLeft(scrutinee) => {
                    frames.push(TermFrame::CaseRight(scrutinee, expect_comp_term(produced)?));
                    continue 'read;
                },
                | TermFrame::CaseRight(scrutinee, on_left) => {
                    let on_right = expect_comp_term(produced)?;
                    produced = TermNode::Comp(Computation::Case {
                        scrutinee: Box::new(scrutinee),
                        on_left: Box::new(on_left),
                        on_right: Box::new(on_right),
                    });
                },
            }
        }
    }
}

/// Require a value node, rejecting a polarity mismatch.
#[inline]
fn expect_value_term(node: TermNode) -> Result<Value, DecodeError>
{
    match node {
        | TermNode::Value(value) => Ok(value),
        | TermNode::Comp(_) => Err(DecodeError::Malformed {
            site: MalformedSite::Polarity,
        }),
    }
}

/// Require a computation node, rejecting a polarity mismatch.
#[inline]
fn expect_comp_term(node: TermNode) -> Result<Computation, DecodeError>
{
    match node {
        | TermNode::Comp(computation) => Ok(computation),
        | TermNode::Value(_) => Err(DecodeError::Malformed {
            site: MalformedSite::Polarity,
        }),
    }
}

/// Decode an injection side byte.
#[inline]
fn decode_side(reader: &mut ByteReader<'_>) -> Result<Side, DecodeError>
{
    match reader.next_byte()? {
        | SIDE_LEFT => Ok(Side::Left),
        | SIDE_RIGHT => Ok(Side::Right),
        | other => Err(DecodeError::UnknownTag {
            site: TagSite::Side,
            tag: other,
        }),
    }
}

/// Decode a literal, rebuilt through the base-type smart constructors.
fn decode_literal(reader: &mut ByteReader<'_>) -> Result<Literal, DecodeError>
{
    match reader.next_byte()? {
        | LITERAL_INTEGER => {
            let sign = decode_sign(reader)?;
            let magnitude = decode_magnitude(reader)?;
            Ok(Literal::Integer(IntegerLiteral::new(sign, magnitude)))
        },
        | LITERAL_TEXT => {
            let content = reader.read_text()?;
            Ok(Literal::Text(StringLiteral::new(content)))
        },
        | LITERAL_NUMERIC => {
            let sign = decode_sign(reader)?;
            let integer_part = decode_magnitude(reader)?;
            let fraction = decode_fraction(reader)?;
            Ok(Literal::Numeric(NumericLiteral::new(
                sign,
                integer_part,
                fraction,
            )))
        },
        | other => Err(DecodeError::UnknownTag {
            site: TagSite::LiteralKind,
            tag: other,
        }),
    }
}

/// Decode a literal sign byte.
#[inline]
fn decode_sign(reader: &mut ByteReader<'_>) -> Result<Sign, DecodeError>
{
    match reader.next_byte()? {
        | SIGN_NON_NEGATIVE => Ok(Sign::NonNegative),
        | SIGN_NEGATIVE => Ok(Sign::Negative),
        | other => Err(DecodeError::UnknownTag {
            site: TagSite::Sign,
            tag: other,
        }),
    }
}

/// Decode a canonical magnitude through its smart constructor.
#[inline]
fn decode_magnitude(reader: &mut ByteReader<'_>) -> Result<Magnitude, DecodeError>
{
    let digits = reader.read_text()?;
    Magnitude::from_decimal_text(digits).ok_or(DecodeError::Malformed {
        site: MalformedSite::LiteralPayload,
    })
}

/// Decode a canonical fraction through its smart constructor.
#[inline]
fn decode_fraction(reader: &mut ByteReader<'_>) -> Result<FractionDigits, DecodeError>
{
    let digits = reader.read_text()?;
    FractionDigits::from_decimal_text(digits).ok_or(DecodeError::Malformed {
        site: MalformedSite::LiteralPayload,
    })
}
