//! The deterministic export writer (kernel-boundary.md §5, E1/E2/E4/E5/E6):
//! serialize an [`Environment`] to canonical bytes.
//!
//! The encoding iterates the environment in admission order (E2) and each
//! level's atoms in `BTreeMap`-sorted order (E4) — never a hash-order
//! iteration — so an identical environment yields a byte-identical artifact.
//! Term and type trees are walked **iteratively** over an explicit borrowed
//! stack (never input-scaled recursion), so an environment holding a deeply
//! nested term (admissible through the bypass) is serialized without risking a
//! stack overflow. Every field is length-prefixed or tagged so the reader can
//! validate structurally.

use alloc::vec::Vec;

use gandr_kernel_strata::Level;

use super::ADMISSION_CHECKED;
use super::ADMISSION_UNCHECKED;
use super::AdmissionMark;
use super::BASE_INTEGER;
use super::BASE_NUMERIC;
use super::BASE_STRING;
use super::FORMAT_VERSION_V0;
use super::KIND_AXIOM;
use super::KIND_DEF;
use super::LITERAL_INTEGER;
use super::LITERAL_NUMERIC;
use super::LITERAL_TEXT;
use super::MAGIC;
use super::RELATION_EQ;
use super::RELATION_LEQ;
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
use crate::base::BaseType;
use crate::base::Literal;
use crate::base::Sign;
use crate::decl::Declaration;
use crate::decl::DeclarationContent;
use crate::decl::LevelSignature;
use crate::env::Admission;
use crate::env::Environment;
use crate::term::Computation;
use crate::term::Side;
use crate::term::Value;
use crate::types::CompType;
use crate::types::ValueType;

/// Serialize an [`Environment`] to a self-contained, canonical byte artifact.
///
/// # Contract
/// - requires: nothing.
/// - ensures: the bytes carry the version tag, the reserved sections, and the
///   admission-ordered declaration sequence with per-declaration admission
///   marks — everything a replay needs (E1); iterating an identical environment
///   yields a byte-identical result (E4, deterministic canonical bytes:
///   admission order and sorted level atoms, never hash-order); the precomputed
///   audit sets are not written (E3).
/// - provides: the K5 export artifact.
/// - fails: never — serialization is total.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L2 — the round-trip differential pins that every
///   [`super::read()`] of `write(env)` reproduces the environment
///   (declarations, marks, and recomputed audits), and the determinism property
///   pins that a second `write` is byte-identical; the L3 residues are the
///   empty environment and the bypass-admitted mark, each pinned by a unit
///   golden.
/// - witness: `export::tests::round_trip_reproduces_the_environment`
/// - witness: `export::tests::a_second_write_is_byte_identical`
/// - witness: `export::tests::the_empty_environment_round_trips`
#[inline]
#[must_use]
pub fn write(environment: &Environment) -> Vec<u8>
{
    let declarations: Vec<(AdmissionMark, &Declaration)> = environment
        .admitted()
        .map(|(admission, declaration)| (mark_of(admission), declaration))
        .collect();
    encode_artifact(&declarations)
}

/// Encode a marked declaration sequence into the canonical artifact bytes —
/// the shared engine of [`write()`] and the reader's canonical-bytes check.
///
/// # Contract
/// - requires: `declarations` is in admission order.
/// - ensures: the bytes are the header (magic + version), the reserved
///   minted-atom table (empty at v0), the declaration count, and one canonical
///   record per declaration.
/// - provides: the deterministic canonical byte image of the sequence.
/// - fails: never.
/// - panics: none.
pub(super) fn encode_artifact(declarations: &[(AdmissionMark, &Declaration)]) -> Vec<u8>
{
    let mut out = Vec::new();
    out.extend_from_slice(&MAGIC);
    out.extend_from_slice(&FORMAT_VERSION_V0.to_be_bytes());
    // R4: the reserved minted-atom table, admission-ordered and empty at v0.
    put_uvarint(&mut out, 0_u64);
    put_uvarint(&mut out, usize_to_u64(declarations.len()));
    for &(mark, declaration) in declarations {
        encode_declaration(&mut out, mark, declaration);
    }
    out
}

/// Map an environment admission to its wire mark (E6).
#[inline]
fn mark_of(admission: Admission) -> AdmissionMark
{
    match admission {
        | Admission::Checked => AdmissionMark::Checked,
        | Admission::Unchecked => AdmissionMark::UncheckedBypass,
    }
}

/// Encode one declaration record: admission mark, kind, structured name (empty
/// at v0), level signature, and content.
fn encode_declaration(
    out: &mut Vec<u8>,
    mark: AdmissionMark,
    declaration: &Declaration,
)
{
    out.push(match mark {
        | AdmissionMark::Checked => ADMISSION_CHECKED,
        | AdmissionMark::UncheckedBypass => ADMISSION_UNCHECKED,
    });
    let content = declaration.content();
    out.push(match *content {
        | DeclarationContent::Def { .. } => KIND_DEF,
        | DeclarationContent::Axiom { .. } => KIND_AXIOM,
    });
    // R2: the structured name is a segment sequence, empty (zero segments) at
    // v0 — never a flat dotted string.
    put_uvarint(out, 0_u64);
    encode_level_signature(out, declaration.levels());
    match *content {
        | DeclarationContent::Def {
            ref declared,
            ref body,
        } => {
            encode_value_type(out, declared);
            encode_value(out, body);
            // R3: four per-Def annotation slots, each length-prefixed and empty
            // at v0 (erasure, modes/grades, sealing-provenance,
            // directedness/variance).
            put_uvarint(out, 0_u64);
            put_uvarint(out, 0_u64);
            put_uvarint(out, 0_u64);
            put_uvarint(out, 0_u64);
        },
        | DeclarationContent::Axiom { ref declared } => {
            encode_value_type(out, declared);
        },
    }
}

/// Encode a declaration's prenex level signature: parameter count and declared
/// landmark constraints, in declaration order.
fn encode_level_signature(
    out: &mut Vec<u8>,
    signature: &LevelSignature,
)
{
    put_uvarint(out, u64::from(u32::from(signature.params())));
    let constraints = signature.constraints();
    put_uvarint(out, usize_to_u64(constraints.len()));
    for constraint in constraints {
        out.push(match constraint.relation() {
            | gandr_kernel_strata::ConstraintRelation::Leq => RELATION_LEQ,
            | gandr_kernel_strata::ConstraintRelation::Eq => RELATION_EQ,
        });
        encode_level(out, constraint.left());
        encode_level(out, constraint.right());
    }
}

/// Encode a canonical level: its constant part, then its variable atoms in
/// ascending order (E4: `BTreeMap`-sorted, never hash-order).
fn encode_level(
    out: &mut Vec<u8>,
    level: &Level,
)
{
    put_uvarint(out, u64::from(level.constant_part()));
    let atoms: Vec<(
        gandr_kernel_strata::LevelVar,
        gandr_kernel_strata::LevelOffset,
    )> = level.atoms().collect();
    put_uvarint(out, usize_to_u64(atoms.len()));
    for (variable, offset) in atoms {
        put_uvarint(out, u64::from(u32::from(variable.index())));
        put_uvarint(out, u64::from(offset));
    }
}

/// A borrowed type node awaiting emission (value-type or computation-type).
enum TypeRef<'decl>
{
    /// A value type.
    Value(&'decl ValueType),
    /// A computation type.
    Comp(&'decl CompType),
}

/// Encode a value type in preorder over an explicit stack (no input recursion):
/// each node emits its tag and inline leaf data, then its children follow.
#[expect(
    clippy::pattern_type_mismatch,
    reason = "ergonomic matching of borrowed type nodes; every binding is a shared reference by intent"
)]
fn encode_value_type(
    out: &mut Vec<u8>,
    root: &ValueType,
)
{
    let mut stack: Vec<TypeRef<'_>> = Vec::new();
    stack.push(TypeRef::Value(root));
    while let Some(node) = stack.pop() {
        match node {
            | TypeRef::Value(value_type) => match value_type {
                | ValueType::Base(base) => {
                    out.push(TYPE_BASE);
                    out.push(base_type_byte(*base));
                },
                | ValueType::Unit => out.push(TYPE_UNIT),
                | ValueType::Product(first, second) => {
                    out.push(TYPE_PRODUCT);
                    stack.push(TypeRef::Value(second.as_ref()));
                    stack.push(TypeRef::Value(first.as_ref()));
                },
                | ValueType::Sum(first, second) => {
                    out.push(TYPE_SUM);
                    stack.push(TypeRef::Value(second.as_ref()));
                    stack.push(TypeRef::Value(first.as_ref()));
                },
                | ValueType::Thunk(body) => {
                    out.push(TYPE_THUNK);
                    stack.push(TypeRef::Comp(body.as_ref()));
                },
                | ValueType::Universe(level) => {
                    out.push(TYPE_UNIVERSE);
                    encode_level(out, level);
                },
                | ValueType::Lift { inner, target } => {
                    out.push(TYPE_LIFT);
                    encode_level(out, target);
                    stack.push(TypeRef::Value(inner.as_ref()));
                },
            },
            | TypeRef::Comp(comp_type) => match comp_type {
                | CompType::Returner(result) => {
                    out.push(TYPE_RETURNER);
                    stack.push(TypeRef::Value(result.as_ref()));
                },
                | CompType::Arrow { domain, codomain } => {
                    out.push(TYPE_ARROW);
                    stack.push(TypeRef::Comp(codomain.as_ref()));
                    stack.push(TypeRef::Value(domain.as_ref()));
                },
            },
        }
    }
}

/// A borrowed term node awaiting emission (value or computation).
enum TermRef<'decl>
{
    /// A value.
    Value(&'decl Value),
    /// A computation.
    Comp(&'decl Computation),
}

/// Encode a value term in preorder over an explicit stack (no input recursion).
#[expect(
    clippy::pattern_type_mismatch,
    reason = "ergonomic matching of borrowed term nodes; every binding is a shared reference by intent"
)]
fn encode_value(
    out: &mut Vec<u8>,
    root: &Value,
)
{
    let mut stack: Vec<TermRef<'_>> = Vec::new();
    stack.push(TermRef::Value(root));
    while let Some(node) = stack.pop() {
        match node {
            | TermRef::Value(value) => match value {
                | Value::Variable(index) => {
                    out.push(TERM_VARIABLE);
                    put_uvarint(out, u64::from(u32::from(*index)));
                },
                | Value::Constant(index) => {
                    out.push(TERM_CONSTANT);
                    put_uvarint(out, usize_to_u64(usize::from(*index)));
                },
                | Value::Unit => out.push(TERM_UNIT),
                | Value::Literal(literal) => {
                    out.push(TERM_LITERAL);
                    encode_literal(out, literal);
                },
                | Value::Pair(first, second) => {
                    out.push(TERM_PAIR);
                    stack.push(TermRef::Value(second.as_ref()));
                    stack.push(TermRef::Value(first.as_ref()));
                },
                | Value::Injection(side, body) => {
                    out.push(TERM_INJECTION);
                    out.push(side_byte(*side));
                    stack.push(TermRef::Value(body.as_ref()));
                },
                | Value::Thunk(body) => {
                    out.push(TERM_THUNK);
                    stack.push(TermRef::Comp(body.as_ref()));
                },
                | Value::Lift { target, body } => {
                    out.push(TERM_LIFT);
                    encode_level(out, target);
                    stack.push(TermRef::Value(body.as_ref()));
                },
            },
            | TermRef::Comp(computation) => match computation {
                | Computation::Lambda(body) => {
                    out.push(TERM_LAMBDA);
                    stack.push(TermRef::Comp(body.as_ref()));
                },
                | Computation::Application(head, argument) => {
                    out.push(TERM_APPLICATION);
                    stack.push(TermRef::Value(argument.as_ref()));
                    stack.push(TermRef::Comp(head.as_ref()));
                },
                | Computation::Return(value) => {
                    out.push(TERM_RETURN);
                    stack.push(TermRef::Value(value.as_ref()));
                },
                | Computation::Bind(bound, body) => {
                    out.push(TERM_BIND);
                    stack.push(TermRef::Comp(body.as_ref()));
                    stack.push(TermRef::Comp(bound.as_ref()));
                },
                | Computation::Force(value) => {
                    out.push(TERM_FORCE);
                    stack.push(TermRef::Value(value.as_ref()));
                },
                | Computation::Case {
                    scrutinee,
                    on_left,
                    on_right,
                } => {
                    out.push(TERM_CASE);
                    stack.push(TermRef::Comp(on_right.as_ref()));
                    stack.push(TermRef::Comp(on_left.as_ref()));
                    stack.push(TermRef::Value(scrutinee.as_ref()));
                },
            },
        }
    }
}

/// Encode a literal: its kind, then its canonical payload.
fn encode_literal(
    out: &mut Vec<u8>,
    literal: &Literal,
)
{
    match *literal {
        | Literal::Integer(ref integer) => {
            out.push(LITERAL_INTEGER);
            out.push(sign_byte(integer.sign()));
            encode_text(out, &integer.magnitude().to_digits());
        },
        | Literal::Text(ref text) => {
            out.push(LITERAL_TEXT);
            encode_text(out, &text.to_content());
        },
        | Literal::Numeric(ref numeric) => {
            out.push(LITERAL_NUMERIC);
            out.push(sign_byte(numeric.sign()));
            encode_text(out, &numeric.integer_part().to_digits());
            encode_text(out, &numeric.fraction().to_digits());
        },
    }
}

/// Encode length-prefixed UTF-8 text.
fn encode_text(
    out: &mut Vec<u8>,
    text: &str,
)
{
    let bytes = text.as_bytes();
    put_uvarint(out, usize_to_u64(bytes.len()));
    out.extend_from_slice(bytes);
}

/// The wire byte for a base-type atom.
#[inline]
fn base_type_byte(base: BaseType) -> u8
{
    match base {
        | BaseType::Integer => BASE_INTEGER,
        | BaseType::String => BASE_STRING,
        | BaseType::Numeric => BASE_NUMERIC,
    }
}

/// The wire byte for a literal sign.
#[inline]
fn sign_byte(sign: Sign) -> u8
{
    match sign {
        | Sign::NonNegative => SIGN_NON_NEGATIVE,
        | Sign::Negative => SIGN_NEGATIVE,
    }
}

/// The wire byte for an injection side.
#[inline]
fn side_byte(side: Side) -> u8
{
    match side {
        | Side::Left => SIDE_LEFT,
        | Side::Right => SIDE_RIGHT,
    }
}

/// Append a canonical (minimal) unsigned LEB128 varint.
///
/// # Contract
/// - requires: nothing.
/// - ensures: appends the minimal little-endian base-128 encoding of `value`,
///   so a given value has exactly one byte image (E4).
/// - provides: the writer's integer primitive, matched by the reader's
///   overlong-rejecting decoder.
/// - fails: never.
/// - panics: none.
fn put_uvarint(
    out: &mut Vec<u8>,
    value: u64,
)
{
    let mut remaining = value;
    loop {
        let low = u8::try_from(remaining & 0x7f).unwrap_or(0_u8);
        remaining = remaining.wrapping_shr(7);
        if remaining == 0_u64 {
            out.push(low);
            break;
        }
        out.push(low | 0x80);
    }
}

/// Widen a `usize` count to `u64` for the wire (lossless on every supported
/// platform, where `usize` is at most 64 bits).
#[inline]
fn usize_to_u64(value: usize) -> u64
{
    u64::try_from(value).unwrap_or(u64::MAX)
}
