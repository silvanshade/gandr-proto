//! The deterministic export writer (kernel-boundary.md §5, E1/E2/E4/E5/E6):
//! serialize an [`Environment`] to canonical v1 bytes.
//!
//! # v1: the maximal-sharing subterm table (massive-term design §4.2–4.5)
//!
//! Each declaration segment carries a **subterm table** rather than an expanded
//! preorder tree. The writer interns nodes bottom-up (**post-order**, children
//! before parents) with **content-keyed dedup**: a node's key is its encoded
//! entry bytes — the node tag, its children's already-assigned **global** table
//! indices, and its canonical inline payload — so two structurally-equal nodes
//! (whether shared within a declaration, across declarations, or merely
//! coincident) collapse to one global index (**maximal sharing under structural
//! equality**, the Lean `ShareCommon` boundary pass). The first completion of a
//! node assigns the next global index and appends its entry to the *current*
//! declaration's segment; a later occurrence reuses the index without
//! appending.
//!
//! Dedup is **content-keyed, never arena-id-keyed**: the bytes are a function
//! of the *abstract* environment, not of incidental in-memory sharing
//! (in-memory sharing differs between a freshly-built and a decoded
//! environment; E4 determinism demands content keys). The `no_std` dedup map is
//! a `BTreeMap`. The walk is a resumable-frame post-order over an explicit
//! stack (never input-scaled recursion) and is **sharing-aware** — an arena
//! node is visited once (an `id_to_global` memo), so re-encoding a decoded DAG
//! is `O(entries)`, not `O(expanded)` (§4.6).
//!
//! The writer feeds no judgment and is not a trusted fast path: the reader's
//! whole-artifact re-encode-compare (§4.6) is what enforces canonical form, so
//! a buggy or malicious writer is caught, never trusted (C3-compatible).

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use gandr_kernel_strata::Level;

use super::ADMISSION_CHECKED;
use super::ADMISSION_UNCHECKED;
use super::AdmissionMark;
use super::BASE_INTEGER;
use super::BASE_NUMERIC;
use super::BASE_STRING;
use super::FORMAT_VERSION_V1;
use super::KIND_AXIOM;
use super::KIND_DEF;
use super::LITERAL_INTEGER;
use super::LITERAL_NUMERIC;
use super::LITERAL_TEXT;
use super::MAGIC;
use super::NODE_C_APPLICATION;
use super::NODE_C_BIND;
use super::NODE_C_CASE;
use super::NODE_C_FORCE;
use super::NODE_C_LAMBDA;
use super::NODE_C_RETURN;
use super::NODE_CT_ARROW;
use super::NODE_CT_RETURNER;
use super::NODE_V_CONSTANT;
use super::NODE_V_INJECTION;
use super::NODE_V_LIFT;
use super::NODE_V_LITERAL;
use super::NODE_V_PAIR;
use super::NODE_V_THUNK;
use super::NODE_V_UNIT;
use super::NODE_V_VARIABLE;
use super::NODE_VT_BASE;
use super::NODE_VT_LIFT;
use super::NODE_VT_PRODUCT;
use super::NODE_VT_SUM;
use super::NODE_VT_THUNK;
use super::NODE_VT_UNIT;
use super::NODE_VT_UNIVERSE;
use super::RELATION_EQ;
use super::RELATION_LEQ;
use super::SIDE_LEFT;
use super::SIDE_RIGHT;
use super::SIGN_NEGATIVE;
use super::SIGN_NON_NEGATIVE;
use super::SegmentedArtifact;
use crate::arena::CompTypeId;
use crate::arena::ComputationId;
use crate::arena::TermArena;
use crate::arena::ValueId;
use crate::arena::ValueTypeId;
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

/// Serialize an [`Environment`] to a self-contained, canonical v1 byte
/// artifact.
///
/// # Contract
/// - requires: nothing.
/// - ensures: the bytes carry the v1 version tag, the reserved sections, and
///   the admission-ordered declaration segments — each a maximal-sharing
///   subterm table plus root references (E1); iterating an identical *abstract*
///   environment yields a byte-identical result regardless of in-memory sharing
///   (E4, content-keyed dedup); the precomputed audit sets are not written
///   (E3).
/// - provides: the K5 v1 export artifact.
/// - fails: never — serialization is total.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L2 — the round-trip differential pins that every
///   [`super::read()`] of `write(env)` reproduces the environment and retains
///   sharing, and the determinism property pins that structurally-equal but
///   differently-shared inputs write identically; the L3 residues are the empty
///   environment and the bypass-admitted mark.
/// - witness: `export::tests::round_trip_reproduces_the_environment`
/// - witness: `export::tests::structurally_equal_inputs_write_identically`
/// - witness: `export::tests::sharing_collapses_the_table`
/// - witness: `read::tests::decode_retains_cross_declaration_sharing`
#[inline]
#[must_use]
pub fn write(environment: &Environment) -> Vec<u8>
{
    let declarations: Vec<(AdmissionMark, &Declaration)> = environment
        .admitted()
        .map(|(admission, declaration)| (mark_of(admission), declaration))
        .collect();
    encode_artifact(environment.arena(), &declarations)
}

/// Serialize an [`Environment`] to canonical v1 bytes and the byte boundaries
/// of its declaration segments.
///
/// This is the structural companion to [`write()`] the outer content-addressed
/// layer consumes (massive-term design §6, B2.3).
///
/// # Contract
/// - requires: nothing.
/// - ensures: `bytes` equals `write(environment)`; the returned framing splits
///   `bytes` into the header (magic, version, reserved minted-atom table,
///   declaration count) and one self-delimiting segment per admitted
///   declaration in admission order, so the outer layer keys each declaration
///   by its admission index without re-parsing the format (E1/E4). No hash is
///   computed and no payload byte is interpreted — offsets and lengths only,
///   the TCB-wall discipline.
/// - provides: the K5 v1 export artifact plus its declaration-record grain.
/// - fails: never — serialization is total.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L2 — the byte-identity differential pins that
///   [`SegmentedArtifact::bytes`] equals `write(env)`, and the reassembly
///   differential pins that the header followed by every segment reproduces the
///   artifact; the L3 residue is the empty environment (a header, no segments).
/// - witness: `export::tests::write_segmented_matches_write`
/// - witness: `export::tests::segments_reassemble_the_artifact`
/// - witness: `export::tests::the_empty_environment_segments_to_a_header`
#[inline]
#[must_use]
pub fn write_segmented(environment: &Environment) -> SegmentedArtifact
{
    let declarations: Vec<(AdmissionMark, &Declaration)> = environment
        .admitted()
        .map(|(admission, declaration)| (mark_of(admission), declaration))
        .collect();
    let (bytes, header_len, segment_ends) =
        encode_artifact_framed(environment.arena(), &declarations);
    SegmentedArtifact::new(bytes, header_len, segment_ends)
}

/// Encode a marked declaration sequence (its content in `arena`) into the
/// canonical v1 artifact bytes — the shared engine of [`write()`] and the
/// reader's sharing-aware canonical-bytes check (§4.6).
///
/// # Contract
/// - requires: `declarations` is in admission order and their content roots
///   resolve in `arena`.
/// - ensures: the header, the empty reserved sections, the declaration count,
///   and one canonical maximal-sharing segment per declaration.
/// - provides: the deterministic canonical byte image, `O(total arena nodes)`
///   (sharing-aware, never `O(expanded)`).
/// - fails: never.
/// - panics: none.
pub(super) fn encode_artifact(
    arena: &TermArena,
    declarations: &[(AdmissionMark, &Declaration)],
) -> Vec<u8>
{
    let (bytes, _header_len, _segment_ends) = encode_artifact_framed(arena, declarations);
    bytes
}

/// Encode a marked declaration sequence into canonical v1 bytes, recording the
/// header length and each declaration segment's exclusive end offset — the
/// shared engine of [`encode_artifact`] and [`write_segmented`].
///
/// # Contract
/// - requires: `declarations` is in admission order and their content roots
///   resolve in `arena`.
/// - ensures: the returned bytes are the canonical artifact; `header_len` is
///   the offset of the first declaration segment; `segment_ends[k]` is
///   declaration `k`'s exclusive end (strictly ascending, the last equal to the
///   byte length), so segment `k` is `bytes[prev_end .. segment_ends[k]]`.
/// - provides: the canonical byte image plus its declaration-segment framing.
/// - fails: never.
/// - panics: none.
fn encode_artifact_framed(
    arena: &TermArena,
    declarations: &[(AdmissionMark, &Declaration)],
) -> (Vec<u8>, usize, Vec<usize>)
{
    let mut out = Vec::new();
    out.extend_from_slice(&MAGIC);
    out.extend_from_slice(&FORMAT_VERSION_V1.to_be_bytes());
    // R4: the reserved minted-atom table, admission-ordered and empty at v1.
    put_uvarint(&mut out, 0_u64);
    put_uvarint(&mut out, usize_to_u64(declarations.len()));
    let header_len = out.len();
    let mut segment_ends: Vec<usize> = Vec::with_capacity(declarations.len());
    let mut interner = Interner::new();
    for &(mark, declaration) in declarations {
        encode_declaration(&mut out, arena, &mut interner, mark, declaration);
        segment_ends.push(out.len());
    }
    (out, header_len, segment_ends)
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

/// A cross-family arena node reference — the work item of the intern walk.
#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
enum Node
{
    /// A value node.
    Value(ValueId),
    /// A computation node.
    Computation(ComputationId),
    /// A value-type node.
    ValueType(ValueTypeId),
    /// A computation-type node.
    CompType(CompTypeId),
}

/// The content-keyed subterm interner, carrying the global index counter and
/// the dedup and arena-memo maps across declarations.
struct Interner
{
    /// Arena node → its assigned global table index (the sharing-aware memo, so
    /// an in-arena shared node is walked once).
    id_to_global: BTreeMap<Node, u32>,
    /// Encoded entry bytes → its global index (the content dedup; the encoded
    /// bytes are the content key, so structurally-equal nodes collapse).
    key_to_global: BTreeMap<Vec<u8>, u32>,
    /// The next free global table index.
    next_index: u32,
}

impl Interner
{
    /// A fresh interner.
    #[inline]
    fn new() -> Self
    {
        Self {
            id_to_global: BTreeMap::new(),
            key_to_global: BTreeMap::new(),
            next_index: 0,
        }
    }
}

/// The immediate child references of an arena node, in order (empty for a leaf
/// or a dangling id — fail-safe).
#[expect(
    clippy::pattern_type_mismatch,
    reason = "ergonomic matching of the borrowed arena node against value patterns; every binding is a shared reference by intent"
)]
fn node_children(
    arena: &TermArena,
    node: Node,
) -> Vec<Node>
{
    let mut children = Vec::new();
    match node {
        | Node::Value(id) => {
            if let Some(value) = arena.value(id) {
                match value {
                    | Value::Variable(_) | Value::Constant(_) | Value::Unit | Value::Literal(_) => {
                    },
                    | Value::Pair(first, second) => {
                        children.push(Node::Value(*first));
                        children.push(Node::Value(*second));
                    },
                    | Value::Injection(_, body) | Value::Lift { body, .. } => {
                        children.push(Node::Value(*body));
                    },
                    | Value::Thunk(body) => children.push(Node::Computation(*body)),
                }
            }
        },
        | Node::Computation(id) => {
            if let Some(computation) = arena.computation(id) {
                match computation {
                    | Computation::Lambda(body) => children.push(Node::Computation(*body)),
                    | Computation::Application(head, argument) => {
                        children.push(Node::Computation(*head));
                        children.push(Node::Value(*argument));
                    },
                    | Computation::Return(value) | Computation::Force(value) => {
                        children.push(Node::Value(*value));
                    },
                    | Computation::Bind(bound, body) => {
                        children.push(Node::Computation(*bound));
                        children.push(Node::Computation(*body));
                    },
                    | Computation::Case {
                        scrutinee,
                        on_left,
                        on_right,
                    } => {
                        children.push(Node::Value(*scrutinee));
                        children.push(Node::Computation(*on_left));
                        children.push(Node::Computation(*on_right));
                    },
                }
            }
        },
        | Node::ValueType(id) => {
            if let Some(value_type) = arena.value_type(id) {
                match value_type {
                    | ValueType::Base(_) | ValueType::Unit | ValueType::Universe(_) => {},
                    | ValueType::Product(first, second) | ValueType::Sum(first, second) => {
                        children.push(Node::ValueType(*first));
                        children.push(Node::ValueType(*second));
                    },
                    | ValueType::Thunk(body) => children.push(Node::CompType(*body)),
                    | ValueType::Lift { inner, .. } => children.push(Node::ValueType(*inner)),
                }
            }
        },
        | Node::CompType(id) => {
            if let Some(comp_type) = arena.comp_type(id) {
                match comp_type {
                    | CompType::Returner(result) => children.push(Node::ValueType(*result)),
                    | CompType::Arrow { domain, codomain } => {
                        children.push(Node::ValueType(*domain));
                        children.push(Node::CompType(*codomain));
                    },
                }
            }
        },
    }
    children
}

/// Encode one subterm-table entry: its node tag, inline payload, then its
/// children's global indices as uvarints (a dangling id encodes as unit,
/// fail-safe).
#[expect(
    clippy::pattern_type_mismatch,
    reason = "ergonomic matching of the borrowed arena node against value patterns; every binding is a shared reference by intent"
)]
fn encode_entry(
    arena: &TermArena,
    node: Node,
    child_globals: &[u32],
) -> Vec<u8>
{
    let mut out = Vec::new();
    match node {
        | Node::ValueType(id) => match arena.value_type(id) {
            | Some(&ValueType::Base(base)) => {
                out.push(NODE_VT_BASE);
                out.push(base_type_byte(base));
            },
            | Some(&ValueType::Unit) | None => out.push(NODE_VT_UNIT),
            | Some(ValueType::Universe(level)) => {
                out.push(NODE_VT_UNIVERSE);
                encode_level(&mut out, level);
            },
            | Some(&ValueType::Product(..)) => out.push(NODE_VT_PRODUCT),
            | Some(&ValueType::Sum(..)) => out.push(NODE_VT_SUM),
            | Some(&ValueType::Thunk(_)) => out.push(NODE_VT_THUNK),
            | Some(ValueType::Lift { target, .. }) => {
                out.push(NODE_VT_LIFT);
                encode_level(&mut out, target);
            },
        },
        | Node::CompType(id) => match arena.comp_type(id) {
            | Some(&CompType::Returner(_)) | None => out.push(NODE_CT_RETURNER),
            | Some(&CompType::Arrow { .. }) => out.push(NODE_CT_ARROW),
        },
        | Node::Value(id) => match arena.value(id) {
            | Some(&Value::Variable(index)) => {
                out.push(NODE_V_VARIABLE);
                put_uvarint(&mut out, u64::from(u32::from(index)));
            },
            | Some(&Value::Constant(index)) => {
                out.push(NODE_V_CONSTANT);
                put_uvarint(&mut out, usize_to_u64(usize::from(index)));
            },
            | Some(&Value::Unit) | None => out.push(NODE_V_UNIT),
            | Some(Value::Literal(literal)) => {
                out.push(NODE_V_LITERAL);
                encode_literal(&mut out, literal);
            },
            | Some(&Value::Pair(..)) => out.push(NODE_V_PAIR),
            | Some(&Value::Injection(side, _)) => {
                out.push(NODE_V_INJECTION);
                out.push(side_byte(side));
            },
            | Some(&Value::Thunk(_)) => out.push(NODE_V_THUNK),
            | Some(Value::Lift { target, .. }) => {
                out.push(NODE_V_LIFT);
                encode_level(&mut out, target);
            },
        },
        | Node::Computation(id) => match arena.computation(id) {
            | Some(&Computation::Lambda(_)) => out.push(NODE_C_LAMBDA),
            | Some(&Computation::Application(..)) => out.push(NODE_C_APPLICATION),
            | Some(&Computation::Return(_)) | None => out.push(NODE_C_RETURN),
            | Some(&Computation::Bind(..)) => out.push(NODE_C_BIND),
            | Some(&Computation::Force(_)) => out.push(NODE_C_FORCE),
            | Some(&Computation::Case { .. }) => out.push(NODE_C_CASE),
        },
    }
    for &child in child_globals {
        put_uvarint(&mut out, u64::from(child));
    }
    out
}

/// Intern a root node's sub-DAG into `interner`, appending each first-completed
/// entry to `segment`, and return the root's global table index.
///
/// # Contract
/// - requires: `root` resolves in `arena`.
/// - ensures: every node reachable from `root` has a global index in
///   `interner.id_to_global`; entries first completed here are appended to
///   `segment` in post-order first-completion order; the root's global index is
///   returned. The walk is a resumable-frame post-order over an explicit stack,
///   sharing-aware (each arena node is walked once), so it is total on any
///   depth.
/// - provides: the maximal-sharing intern of one declaration root.
/// - fails: never.
/// - panics: none.
fn intern(
    arena: &TermArena,
    interner: &mut Interner,
    segment: &mut Vec<Vec<u8>>,
    root: Node,
) -> u32
{
    let mut stack: Vec<(Node, usize)> = Vec::new();
    stack.push((root, 0_usize));
    while let Some((node, cursor)) = stack.pop() {
        if interner.id_to_global.contains_key(&node) {
            continue;
        }
        let children = node_children(arena, node);
        let mut next = cursor;
        let mut descended = false;
        while let Some(&child) = children.get(next) {
            if interner.id_to_global.contains_key(&child) {
                next = next.saturating_add(1);
            }
            else {
                stack.push((node, next.saturating_add(1)));
                stack.push((child, 0_usize));
                descended = true;
                break;
            }
        }
        if descended {
            continue;
        }
        let child_globals: Vec<u32> = children
            .iter()
            .map(|child| interner.id_to_global.get(child).copied().unwrap_or(0))
            .collect();
        let encoded = encode_entry(arena, node, &child_globals);
        let global = match interner.key_to_global.get(&encoded) {
            | Some(&existing) => existing,
            | None => {
                let assigned = interner.next_index;
                interner.next_index = interner.next_index.saturating_add(1);
                let _prior = interner.key_to_global.insert(encoded.clone(), assigned);
                segment.push(encoded);
                assigned
            },
        };
        let _prior = interner.id_to_global.insert(node, global);
    }
    interner.id_to_global.get(&root).copied().unwrap_or(0)
}

/// Encode one declaration segment: admission mark, kind, name (empty at v1),
/// level signature, its subterm-table entries, and root references.
fn encode_declaration(
    out: &mut Vec<u8>,
    arena: &TermArena,
    interner: &mut Interner,
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
    // R2: the structured name, empty (zero segments) at v1.
    put_uvarint(out, 0_u64);
    encode_level_signature(out, declaration.levels());
    // Intern this declaration's roots, collecting the entries it introduces.
    let mut segment: Vec<Vec<u8>> = Vec::new();
    let roots = match *content {
        | DeclarationContent::Def { declared, body } => {
            let declared = intern(arena, interner, &mut segment, Node::ValueType(declared));
            let body = intern(arena, interner, &mut segment, Node::Value(body));
            (declared, Some(body))
        },
        | DeclarationContent::Axiom { declared } => {
            let declared = intern(arena, interner, &mut segment, Node::ValueType(declared));
            (declared, None)
        },
    };
    put_uvarint(out, usize_to_u64(segment.len()));
    for entry in &segment {
        out.extend_from_slice(entry);
    }
    let (root_declared, root_body) = roots;
    put_uvarint(out, u64::from(root_declared));
    if let Some(root_body) = root_body {
        put_uvarint(out, u64::from(root_body));
        // R3: four per-Def annotation slots, empty at v1.
        put_uvarint(out, 0_u64);
        put_uvarint(out, 0_u64);
        put_uvarint(out, 0_u64);
        put_uvarint(out, 0_u64);
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
pub(super) fn put_uvarint(
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
pub(super) fn usize_to_u64(value: usize) -> u64
{
    u64::try_from(value).unwrap_or(u64::MAX)
}
