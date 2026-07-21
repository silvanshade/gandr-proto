//! The validating export reader (kernel-boundary.md §5, E1–E6): decode
//! canonical v1 bytes back to a declaration sequence and replay it through the
//! choke point.
//!
//! # v1: streaming subterm-table decode is arena construction (§4.4–4.6)
//!
//! The reader is a closed-vocabulary parser. Every read is bounds-checked
//! ([`DecodeError::Truncated`]); every tag is matched against the closed
//! `NODE_*` set ([`DecodeError::UnknownTag`]); every value is rebuilt through
//! its domain constructor. Each subterm-table entry is **validated as it
//! accrues** and **minted once into a [`TermArena`]** (post-order completion is
//! allocation order), so **decode retains sharing** — a table index referenced
//! twice reuses one arena id, never expanding:
//!
//! * **Per-entry validation**: a known tag; each child index **strictly
//!   earlier** than the entry's own global index ([`MalformedSite::ChildOrder`]
//!   — acyclicity) and **polarity-correct** by table lookup (replacing v0's
//!   `expect_value_term`/`expect_comp_term`); [`MAX_TABLE_ENTRIES`] enforced as
//!   entries accrue.
//! * **The amplification defence (§4.4)**: after the whole table, one forward
//!   scan computes each entry's memoized `expanded_size` (a saturating `u64`);
//!   any declaration whose declared-type or body root exceeds
//!   [`MAX_EXPANDED_TERM_WORK`], **or** whose artifact-total (the saturating
//!   sum over every declaration root) exceeds [`MAX_ARTIFACT_EXPANDED_WORK`]
//!   (gandr-4p3 — the many-cheap-segments-sharing-one-root amplification), is
//!   rejected **before replay** — bounding the recursive checker's
//!   tree-expanded work over the shared DAG without touching the checker. The
//!   scan also yields the deterministic [`DecodeMetrics`] the B2.3 exit gate
//!   records.
//! * **Canonical form (E4, §4.6)**: the whole-artifact **sharing-aware**
//!   re-encode-compare (the maximal-sharing writer is the re-encoder,
//!   `O(entries)`) rejects any non-canonical table — a non-maximally-shared
//!   duplicate, a mis-ordered table, or a dead (unreferenced) entry — as
//!   [`MalformedSite::NonCanonical`]; strictly-earlier children are checked
//!   structurally during decode, before re-encode.
//!
//! [`decode`] yields a self-contained [`DecodedArtifact`] (the shared arena
//! plus the declaration sequence); [`read`] imports each declaration's content
//! into a fresh environment arena and re-admits through the choke point. The
//! decode is iterative over the flat table, never input-scaled recursion.

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
use super::DecodeMetrics;
use super::DecodedArtifact;
use super::DecodedDeclaration;
use super::FORMAT_VERSION_V1;
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
use super::MAX_ARTIFACT_EXPANDED_WORK;
use super::MAX_DECODED_LEVEL_OFFSET;
use super::MAX_EXPANDED_TERM_WORK;
use super::MAX_TABLE_ENTRIES;
use super::MalformedSite;
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
use super::ReadError;
use super::ReservedKind;
use super::ReservedSlot;
use super::SIDE_LEFT;
use super::SIDE_RIGHT;
use super::SIGN_NEGATIVE;
use super::SIGN_NON_NEGATIVE;
use super::TagSite;
use super::write::encode_artifact;
use crate::arena::CompTypeId;
use crate::arena::ComputationId;
use crate::arena::TermArena;
use crate::arena::ValueId;
use crate::arena::ValueTypeId;
use crate::base::BaseType;
use crate::base::FractionDigits;
use crate::base::IntegerLiteral;
use crate::base::Literal;
use crate::base::Magnitude;
use crate::base::NumericLiteral;
use crate::base::Sign;
use crate::base::StringLiteral;
use crate::decl::Declaration;
use crate::decl::DeclarationBuilder;
use crate::decl::DeclarationContent;
use crate::decl::LevelSignature;
use crate::env::Environment;
use crate::levels::LevelParamCount;
use crate::term::ConstantIndex;
use crate::term::DeBruijnIndex;
use crate::term::Side;

/// The polarity family of a decoded subterm-table entry (from its tag).
#[derive(Clone, Copy, Eq, PartialEq)]
enum Family
{
    /// A value type.
    ValueType,
    /// A computation type.
    CompType,
    /// A value.
    Value,
    /// A computation.
    Computation,
}

/// A decoded subterm-table entry's arena id, tagged by family.
#[derive(Clone, Copy)]
enum DecodedNode
{
    /// A value-type node.
    ValueType(ValueTypeId),
    /// A computation-type node.
    CompType(CompTypeId),
    /// A value node.
    Value(ValueId),
    /// A computation node.
    Computation(ComputationId),
}

/// The running decode state for the global subterm table: the arena the entries
/// mint into, and per-entry parallel vectors indexed by global table index.
struct Table
{
    /// The arena every entry mints into (sharing retained).
    arena: TermArena,
    /// Global index → the entry's arena id (typed).
    nodes: Vec<DecodedNode>,
    /// Global index → the entry's polarity family (for child-polarity checks).
    families: Vec<Family>,
    /// Global index → the entry's child global indices (for `expanded_size`).
    children: Vec<Vec<u32>>,
}

impl Table
{
    /// A fresh empty table.
    #[inline]
    fn new() -> Self
    {
        Self {
            arena: TermArena::new(),
            nodes: Vec::new(),
            families: Vec::new(),
            children: Vec::new(),
        }
    }

    /// The number of entries decoded so far, as a `u32` global index (capped at
    /// [`MAX_TABLE_ENTRIES`] well below `u32::MAX`, so the widening is exact).
    #[inline]
    fn len_index(&self) -> u32
    {
        u32::try_from(self.nodes.len()).unwrap_or(u32::MAX)
    }
}

/// Per-declaration metadata gathered during decode, resolved to declarations
/// after the arena is built.
struct DeclMeta
{
    /// The admission mark (E6).
    mark: AdmissionMark,
    /// The prenex level signature.
    levels: LevelSignature,
    /// The declared value type's global table index.
    root_declared: u32,
    /// The body value's global table index (`Def` only).
    root_body: Option<u32>,
}

/// Decode a byte artifact into its admission-ordered declaration sequence,
/// building the content into a self-contained shared arena (the v1 validating
/// reader).
///
/// # Contract
/// - requires: nothing — `bytes` may be arbitrary/adversarial.
/// - ensures: `Ok(artifact)` — a [`DecodedArtifact`] whose arena holds the
///   maximal-shared subterm table and whose declarations carry their admission
///   mark (E6) — exactly when the bytes are the canonical v1 encoding of an
///   artifact whose entries all validate (known tag, strictly-earlier and
///   polarity-correct children, within the table, per-declaration
///   expanded-work, and artifact-total expanded-work caps) and whose levels,
///   constraints, and literals rebuild through their constructors (E4). The
///   returned artifact carries the deterministic [`DecodeMetrics`] (D3
///   telemetry) computed en route.
/// - provides: the K5 re-checkable decode — a total parser over the closed
///   vocabulary.
/// - fails: [`DecodeError`] — the rejection triple (truncation, an unknown tag,
///   a structural violation including non-canonical, child-order, table-size,
///   per-declaration expanded-work, and artifact-total expanded-work), a
///   reserved declaration kind (R1), a non-empty reserved slot/section
///   (R2/R3/R4), or an unsupported version (E5, including v0). Never panics,
///   never loops unboundedly.
/// - panics: none.
///
/// # Errors
/// Any [`DecodeError`].
///
/// # Adequacy
/// - hypothesis: L2 — the rejection-totality property pins that truncation at
///   every prefix and arbitrary random bytes return (never panic), and the
///   round-trip differential pins acceptance of every genuine artifact; the L3
///   residues are each named rejection (v0 refusal, non-canonical, child-order,
///   table-size, per-declaration expanded-work, artifact-total expanded-work),
///   pinned by hand-crafted goldens.
/// - witness: `export::tests::truncation_at_every_prefix_is_rejected`
/// - witness: `export::tests::arbitrary_bytes_never_panic`
/// - witness: `export::tests::a_repeated_diamond_dag_is_rejected_before_the_checker`
/// - witness: `export::tests::the_artifact_work_boundary_accepts_just_under_and_rejects_just_over`
/// - witness: `export::tests::a_v0_artifact_is_refused`
#[inline]
pub fn decode(bytes: &[u8]) -> Result<DecodedArtifact, DecodeError>
{
    let mut reader = ByteReader::new(bytes);
    reader.expect_magic()?;
    reader.expect_version()?;
    reader.expect_empty_minted_atom_table()?;
    let count = reader.read_uvarint()?;
    let mut table = Table::new();
    let mut metas: Vec<DeclMeta> = Vec::new();
    let mut remaining = count;
    while remaining > 0_u64 {
        let meta = decode_declaration(&mut reader, &mut table)?;
        metas.push(meta);
        remaining = remaining.wrapping_sub(1_u64);
    }
    if !reader.at_end() {
        return Err(DecodeError::Malformed {
            site: MalformedSite::TrailingBytes,
        });
    }
    let metrics = budget_report(&table, &metas);
    check_budget(&metrics)?;
    let declarations = build_declarations(&mut table, &metas);
    let pairs: Vec<(AdmissionMark, &Declaration)> = declarations
        .iter()
        .map(|decoded| (decoded.mark(), decoded.declaration()))
        .collect();
    if encode_artifact(&table.arena, &pairs) != bytes {
        return Err(DecodeError::Malformed {
            site: MalformedSite::NonCanonical,
        });
    }
    Ok(DecodedArtifact::new(table.arena, declarations, metrics))
}

/// Compute the artifact's decode-budget report in one forward scan of memoized
/// saturating `expanded_size` — the deterministic D3 telemetry the
/// amplification defence rides (§4.4).
///
/// # Contract
/// - requires: every entry's children are strictly-earlier (checked at decode),
///   so a forward scan resolves each child's size before the parent's.
/// - ensures: a [`DecodeMetrics`] whose `table_entries` is the table size,
///   `max_declaration_expanded_work` is the maximum expanded (tree) size over
///   every declaration root, and `artifact_expanded_work` is the saturating sum
///   of those root sizes — all functions of the canonical bytes alone.
/// - provides: the one scan both [`check_budget`] (the acceptance gate) and the
///   exit-gate telemetry read; there is no second descent.
/// - fails: never (a saturating scan is total).
/// - panics: none.
fn budget_report(
    table: &Table,
    metas: &[DeclMeta],
) -> DecodeMetrics
{
    let mut expanded: Vec<u64> = Vec::with_capacity(table.children.len());
    for child_globals in &table.children {
        let mut size: u64 = 1;
        for &child in child_globals {
            let child_size = expanded
                .get(usize::try_from(child).unwrap_or(usize::MAX))
                .copied()
                .unwrap_or(u64::MAX);
            size = size.saturating_add(child_size);
        }
        expanded.push(size);
    }
    let size_of = |global: u32| -> u64 {
        expanded
            .get(usize::try_from(global).unwrap_or(usize::MAX))
            .copied()
            .unwrap_or(u64::MAX)
    };
    let mut max_declaration_expanded_work: u64 = 0;
    let mut artifact_expanded_work: u64 = 0;
    for meta in metas {
        let declared = size_of(meta.root_declared);
        max_declaration_expanded_work = max_declaration_expanded_work.max(declared);
        artifact_expanded_work = artifact_expanded_work.saturating_add(declared);
        if let Some(root_body) = meta.root_body {
            let body = size_of(root_body);
            max_declaration_expanded_work = max_declaration_expanded_work.max(body);
            artifact_expanded_work = artifact_expanded_work.saturating_add(body);
        }
    }
    DecodeMetrics::new(
        table.nodes.len(),
        max_declaration_expanded_work,
        artifact_expanded_work,
    )
}

/// Reject an artifact whose expanded work exceeds either the per-declaration
/// cap [`MAX_EXPANDED_TERM_WORK`] or the artifact-total cap
/// [`MAX_ARTIFACT_EXPANDED_WORK`] (gandr-4p3) — the amplification defence,
/// before replay (§4.4).
///
/// # Contract
/// - requires: `metrics` is the [`budget_report`] of the decoded table.
/// - ensures: `Ok(())` exactly when the maximum per-declaration-root expanded
///   size is within [`MAX_EXPANDED_TERM_WORK`] **and** the artifact-total
///   expanded size is within [`MAX_ARTIFACT_EXPANDED_WORK`].
/// - provides: the checker-time bound, enforced structurally on the table — the
///   per-declaration cap first (the tighter, more specific rejection), then the
///   artifact-total cap.
/// - fails: [`DecodeError::Malformed`] (`ExpandedWork` or
///   `ArtifactExpandedWork`).
/// - panics: none.
fn check_budget(metrics: &DecodeMetrics) -> Result<(), DecodeError>
{
    if metrics.max_declaration_expanded_work() > MAX_EXPANDED_TERM_WORK {
        return Err(DecodeError::Malformed {
            site: MalformedSite::ExpandedWork,
        });
    }
    if metrics.artifact_expanded_work() > MAX_ARTIFACT_EXPANDED_WORK {
        return Err(DecodeError::Malformed {
            site: MalformedSite::ArtifactExpandedWork,
        });
    }
    Ok(())
}

/// The value-type id at a global index, if it is a value-type entry.
#[inline]
fn value_type_id_at(
    nodes: &[DecodedNode],
    global: u32,
) -> Option<ValueTypeId>
{
    match nodes.get(usize::try_from(global).unwrap_or(usize::MAX)) {
        | Some(&DecodedNode::ValueType(id)) => Some(id),
        | _ => None,
    }
}

/// The value id at a global index, if it is a value entry.
#[inline]
fn value_id_at(
    nodes: &[DecodedNode],
    global: u32,
) -> Option<ValueId>
{
    match nodes.get(usize::try_from(global).unwrap_or(usize::MAX)) {
        | Some(&DecodedNode::Value(id)) => Some(id),
        | _ => None,
    }
}

/// Resolve each declaration's global roots to arena ids and build the decoded
/// declaration sequence.
///
/// # Contract
/// - requires: each meta's roots resolved to entries of the correct family at
///   decode; `table.arena` holds the built nodes.
/// - ensures: one [`DecodedDeclaration`] per meta, its content roots addressing
///   `table.arena`.
/// - provides: the [`DecodedArtifact`]'s declaration sequence and the re-encode
///   input.
/// - fails: never (a family mismatch is impossible after decode's checks; a
///   fresh unit leaf is the fail-safe fallback, never a panic).
/// - panics: none.
fn build_declarations(
    table: &mut Table,
    metas: &[DeclMeta],
) -> Vec<DecodedDeclaration>
{
    let mut declarations: Vec<DecodedDeclaration> = Vec::new();
    for meta in metas {
        let declared_id = value_type_id_at(&table.nodes, meta.root_declared);
        let body_id = meta.root_body.map(|root| value_id_at(&table.nodes, root));
        let mut builder = DeclarationBuilder::new(&mut table.arena);
        let declared = declared_id.unwrap_or_else(|| builder.arena().value_type_unit());
        let declaration = match body_id {
            | Some(body_id) => {
                let body = body_id.unwrap_or_else(|| builder.arena().value_unit());
                builder.def(meta.levels.clone(), declared, body)
            },
            | None => builder.axiom(meta.levels.clone(), declared),
        };
        declarations.push(DecodedDeclaration::new(meta.mark, declaration));
    }
    declarations
}

/// Read a byte artifact into an [`Environment`] by importing each decoded
/// declaration's content into a fresh environment arena and replaying it
/// through the choke point (kernel-boundary.md §5 E2/E3/E6).
///
/// # Contract
/// - requires: nothing — `bytes` may be arbitrary/adversarial.
/// - ensures: `Ok(environment)` exactly when the bytes [`decode`] (within the
///   expanded-work budget, so the checker's work is bounded) and every
///   recovered declaration re-admits; a checked mark replays through
///   [`Environment::add_decl`] (K2) and a bypass mark through
///   [`Environment::add_decl_unchecked`], so the audit is recomputed (E3) and
///   the bypass marks survive (E6); the imported content retains the format's
///   sharing.
/// - provides: the K5 replay.
/// - fails: [`ReadError::Decode`] on a format failure (including the pre-replay
///   expanded-work rejection), [`ReadError::Admit`] when a declaration does not
///   re-admit.
/// - panics: none.
///
/// # Errors
/// [`ReadError::Decode`] or [`ReadError::Admit`].
///
/// # Adequacy
/// - hypothesis: L2 — the round-trip differential pins that `read(write(env))`
///   reproduces the environment; the L3 residues are the bypass replay and the
///   amplification differential (an over-budget artifact fails on the decode
///   plane, never reaching the checker).
/// - witness: `export::tests::round_trip_reproduces_the_environment`
/// - witness: `export::tests::a_repeated_diamond_dag_is_rejected_before_the_checker`
#[inline]
pub fn read(bytes: &[u8]) -> Result<Environment, ReadError>
{
    let artifact = decode(bytes)?;
    let decode_arena = artifact.arena();
    let mut environment = Environment::new();
    for decoded in artifact.declarations() {
        let mark = decoded.mark();
        let declaration = decoded.declaration();
        let signature = declaration.levels().clone();
        let mut builder = environment.stage();
        let recovered = match *declaration.content() {
            | DeclarationContent::Def { declared, body } => {
                let (declared, body) = decode_arena.import_def(builder.arena(), declared, body);
                builder.def(signature, declared, body)
            },
            | DeclarationContent::Axiom { declared } => {
                let declared = decode_arena.import_axiom(builder.arena(), declared);
                builder.axiom(signature, declared)
            },
        };
        match mark {
            | AdmissionMark::Checked => {
                let _checked = environment.add_decl(recovered)?;
            },
            | AdmissionMark::UncheckedBypass => {
                let _checked = environment.add_decl_unchecked(recovered);
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

    /// Verify the version tag, refusing any version other than v1 — including
    /// v0, whose refusal exercises the E5 machinery against a real
    /// predecessor.
    #[inline]
    fn expect_version(&mut self) -> Result<(), DecodeError>
    {
        let high = self.next_byte()?;
        let low = self.next_byte()?;
        let found = u16::from_be_bytes([high, low]);
        if found == FORMAT_VERSION_V1 {
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
    ///   [`DecodeError::Malformed`] (`Varint`) on an overlong or out-of-range
    ///   encoding.
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

/// Decode one declaration segment's header, its subterm-table entries, and its
/// root references, minting the entries into `table.arena`.
fn decode_declaration(
    reader: &mut ByteReader<'_>,
    table: &mut Table,
) -> Result<DeclMeta, DecodeError>
{
    let mark = decode_admission(reader)?;
    let kind = reader.next_byte()?;
    reject_reserved_kind(kind)?;
    decode_empty_name(reader)?;
    let levels = decode_level_signature(reader)?;
    let entry_count = reader.read_uvarint()?;
    let mut remaining = entry_count;
    while remaining > 0_u64 {
        decode_entry(reader, table)?;
        remaining = remaining.wrapping_sub(1_u64);
    }
    let root_declared = decode_root(reader, table, Family::ValueType)?;
    let root_body = match kind {
        | KIND_DEF => {
            let body = decode_root(reader, table, Family::Value)?;
            decode_empty_def_slots(reader)?;
            Some(body)
        },
        | KIND_AXIOM => None,
        | other => {
            return Err(DecodeError::UnknownTag {
                site: TagSite::DeclarationKind,
                tag: other,
            });
        },
    };
    Ok(DeclMeta {
        mark,
        levels,
        root_declared,
        root_body,
    })
}

/// Decode a declaration's root reference: a global index that must resolve to
/// an already-decoded entry of the required family.
fn decode_root(
    reader: &mut ByteReader<'_>,
    table: &Table,
    required: Family,
) -> Result<u32, DecodeError>
{
    let index = reader.read_u32()?;
    let family = family_at(table, index)?;
    if family == required {
        Ok(index)
    }
    else {
        Err(DecodeError::Malformed {
            site: MalformedSite::Polarity,
        })
    }
}

/// The family of the entry at `index`, requiring `index` to name an already-
/// decoded entry (a forward or out-of-range reference is a child-order fault).
#[inline]
fn family_at(
    table: &Table,
    index: u32,
) -> Result<Family, DecodeError>
{
    if index >= table.len_index() {
        return Err(DecodeError::Malformed {
            site: MalformedSite::ChildOrder,
        });
    }
    table
        .families
        .get(usize::try_from(index).unwrap_or(usize::MAX))
        .copied()
        .ok_or(DecodeError::Malformed {
            site: MalformedSite::ChildOrder,
        })
}

/// Decode one subterm-table entry: enforce the table cap, read its tag, inline
/// payload, and strictly-earlier polarity-correct child indices, mint it into
/// the arena, and record its family and children.
#[expect(
    clippy::too_many_lines,
    reason = "the flat entry decoder enumerates every NODE_* former in one match; splitting it would obscure the single validated mint"
)]
fn decode_entry(
    reader: &mut ByteReader<'_>,
    table: &mut Table,
) -> Result<(), DecodeError>
{
    if table.nodes.len() >= MAX_TABLE_ENTRIES {
        return Err(DecodeError::Malformed {
            site: MalformedSite::TableSize,
        });
    }
    let this_global = table.len_index();
    let tag = reader.next_byte()?;
    let mut child_globals: Vec<u32> = Vec::new();
    let (node, family) = match tag {
        | NODE_VT_BASE => {
            let base = decode_base_type(reader)?;
            let id = table.arena.value_type_base(base);
            (DecodedNode::ValueType(id), Family::ValueType)
        },
        | NODE_VT_UNIT => (
            DecodedNode::ValueType(table.arena.value_type_unit()),
            Family::ValueType,
        ),
        | NODE_VT_UNIVERSE => {
            let level = decode_level(reader)?;
            let id = table.arena.value_type_universe(level);
            (DecodedNode::ValueType(id), Family::ValueType)
        },
        | NODE_VT_PRODUCT => {
            let first = read_value_type(reader, table, this_global, &mut child_globals)?;
            let second = read_value_type(reader, table, this_global, &mut child_globals)?;
            let id = table.arena.value_type_product(first, second);
            (DecodedNode::ValueType(id), Family::ValueType)
        },
        | NODE_VT_SUM => {
            let first = read_value_type(reader, table, this_global, &mut child_globals)?;
            let second = read_value_type(reader, table, this_global, &mut child_globals)?;
            let id = table.arena.value_type_sum(first, second);
            (DecodedNode::ValueType(id), Family::ValueType)
        },
        | NODE_VT_THUNK => {
            let body = read_comp_type(reader, table, this_global, &mut child_globals)?;
            let id = table.arena.value_type_thunk(body);
            (DecodedNode::ValueType(id), Family::ValueType)
        },
        | NODE_VT_LIFT => {
            let target = decode_level(reader)?;
            let inner = read_value_type(reader, table, this_global, &mut child_globals)?;
            let id = table.arena.value_type_lift(inner, target);
            (DecodedNode::ValueType(id), Family::ValueType)
        },
        | NODE_CT_RETURNER => {
            let result = read_value_type(reader, table, this_global, &mut child_globals)?;
            let id = table.arena.comp_type_returner(result);
            (DecodedNode::CompType(id), Family::CompType)
        },
        | NODE_CT_ARROW => {
            let domain = read_value_type(reader, table, this_global, &mut child_globals)?;
            let codomain = read_comp_type(reader, table, this_global, &mut child_globals)?;
            let id = table.arena.comp_type_arrow(domain, codomain);
            (DecodedNode::CompType(id), Family::CompType)
        },
        | NODE_V_VARIABLE => {
            let index = reader.read_u32()?;
            let id = table.arena.value_variable(DeBruijnIndex::from(index));
            (DecodedNode::Value(id), Family::Value)
        },
        | NODE_V_CONSTANT => {
            let index = reader.read_usize()?;
            let id = table.arena.value_constant(ConstantIndex::from(index));
            (DecodedNode::Value(id), Family::Value)
        },
        | NODE_V_UNIT => (DecodedNode::Value(table.arena.value_unit()), Family::Value),
        | NODE_V_LITERAL => {
            let literal = decode_literal(reader)?;
            let id = table.arena.value_literal(literal);
            (DecodedNode::Value(id), Family::Value)
        },
        | NODE_V_PAIR => {
            let first = read_value(reader, table, this_global, &mut child_globals)?;
            let second = read_value(reader, table, this_global, &mut child_globals)?;
            let id = table.arena.value_pair(first, second);
            (DecodedNode::Value(id), Family::Value)
        },
        | NODE_V_INJECTION => {
            let side = decode_side(reader)?;
            let body = read_value(reader, table, this_global, &mut child_globals)?;
            let id = table.arena.value_injection(side, body);
            (DecodedNode::Value(id), Family::Value)
        },
        | NODE_V_THUNK => {
            let body = read_computation(reader, table, this_global, &mut child_globals)?;
            let id = table.arena.value_thunk(body);
            (DecodedNode::Value(id), Family::Value)
        },
        | NODE_V_LIFT => {
            let target = decode_level(reader)?;
            let body = read_value(reader, table, this_global, &mut child_globals)?;
            let id = table.arena.value_lift(target, body);
            (DecodedNode::Value(id), Family::Value)
        },
        | NODE_C_LAMBDA => {
            let body = read_computation(reader, table, this_global, &mut child_globals)?;
            let id = table.arena.computation_lambda(body);
            (DecodedNode::Computation(id), Family::Computation)
        },
        | NODE_C_APPLICATION => {
            let head = read_computation(reader, table, this_global, &mut child_globals)?;
            let argument = read_value(reader, table, this_global, &mut child_globals)?;
            let id = table.arena.computation_application(head, argument);
            (DecodedNode::Computation(id), Family::Computation)
        },
        | NODE_C_RETURN => {
            let value = read_value(reader, table, this_global, &mut child_globals)?;
            let id = table.arena.computation_return(value);
            (DecodedNode::Computation(id), Family::Computation)
        },
        | NODE_C_BIND => {
            let bound = read_computation(reader, table, this_global, &mut child_globals)?;
            let body = read_computation(reader, table, this_global, &mut child_globals)?;
            let id = table.arena.computation_bind(bound, body);
            (DecodedNode::Computation(id), Family::Computation)
        },
        | NODE_C_FORCE => {
            let value = read_value(reader, table, this_global, &mut child_globals)?;
            let id = table.arena.computation_force(value);
            (DecodedNode::Computation(id), Family::Computation)
        },
        | NODE_C_CASE => {
            let scrutinee = read_value(reader, table, this_global, &mut child_globals)?;
            let on_left = read_computation(reader, table, this_global, &mut child_globals)?;
            let on_right = read_computation(reader, table, this_global, &mut child_globals)?;
            let id = table.arena.computation_case(scrutinee, on_left, on_right);
            (DecodedNode::Computation(id), Family::Computation)
        },
        | other => {
            return Err(DecodeError::UnknownTag {
                site: TagSite::Node,
                tag: other,
            });
        },
    };
    table.nodes.push(node);
    table.families.push(family);
    table.children.push(child_globals);
    Ok(())
}

/// Read one child index, validating strictly-earlier order and the required
/// polarity, and return the resolved child node.
#[inline]
fn read_child_node(
    reader: &mut ByteReader<'_>,
    table: &Table,
    this_global: u32,
    required: Family,
    child_globals: &mut Vec<u32>,
) -> Result<DecodedNode, DecodeError>
{
    let index = reader.read_u32()?;
    if index >= this_global {
        return Err(DecodeError::Malformed {
            site: MalformedSite::ChildOrder,
        });
    }
    let position = usize::try_from(index).unwrap_or(usize::MAX);
    let family = table
        .families
        .get(position)
        .copied()
        .ok_or(DecodeError::Malformed {
            site: MalformedSite::ChildOrder,
        })?;
    if family != required {
        return Err(DecodeError::Malformed {
            site: MalformedSite::Polarity,
        });
    }
    let node = table
        .nodes
        .get(position)
        .copied()
        .ok_or(DecodeError::Malformed {
            site: MalformedSite::ChildOrder,
        })?;
    child_globals.push(index);
    Ok(node)
}

/// Read a value-type child id (order and polarity validated).
#[inline]
fn read_value_type(
    reader: &mut ByteReader<'_>,
    table: &Table,
    this_global: u32,
    child_globals: &mut Vec<u32>,
) -> Result<ValueTypeId, DecodeError>
{
    match read_child_node(reader, table, this_global, Family::ValueType, child_globals)? {
        | DecodedNode::ValueType(id) => Ok(id),
        | _ => Err(DecodeError::Malformed {
            site: MalformedSite::Polarity,
        }),
    }
}

/// Read a computation-type child id (order and polarity validated).
#[inline]
fn read_comp_type(
    reader: &mut ByteReader<'_>,
    table: &Table,
    this_global: u32,
    child_globals: &mut Vec<u32>,
) -> Result<CompTypeId, DecodeError>
{
    match read_child_node(reader, table, this_global, Family::CompType, child_globals)? {
        | DecodedNode::CompType(id) => Ok(id),
        | _ => Err(DecodeError::Malformed {
            site: MalformedSite::Polarity,
        }),
    }
}

/// Read a value child id (order and polarity validated).
#[inline]
fn read_value(
    reader: &mut ByteReader<'_>,
    table: &Table,
    this_global: u32,
    child_globals: &mut Vec<u32>,
) -> Result<ValueId, DecodeError>
{
    match read_child_node(reader, table, this_global, Family::Value, child_globals)? {
        | DecodedNode::Value(id) => Ok(id),
        | _ => Err(DecodeError::Malformed {
            site: MalformedSite::Polarity,
        }),
    }
}

/// Read a computation child id (order and polarity validated).
#[inline]
fn read_computation(
    reader: &mut ByteReader<'_>,
    table: &Table,
    this_global: u32,
    child_globals: &mut Vec<u32>,
) -> Result<ComputationId, DecodeError>
{
    match read_child_node(
        reader,
        table,
        this_global,
        Family::Computation,
        child_globals,
    )? {
        | DecodedNode::Computation(id) => Ok(id),
        | _ => Err(DecodeError::Malformed {
            site: MalformedSite::Polarity,
        }),
    }
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

/// Decode the structured-name record, requiring it empty at v1 (R2).
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

/// Decode the four per-Def annotation slots, requiring each empty at v1 (R3).
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
/// - ensures: `Ok(level)` — always canonical — when the constant, atom count,
///   and each `(variable, offset)` pair decode and no offset meets
///   [`MAX_DECODED_LEVEL_OFFSET`].
/// - provides: the level decoder for universes, lifts, and constraint sides.
/// - fails: [`DecodeError::Truncated`]; [`DecodeError::Malformed`]
///   (`LevelOffset`) on an over-cap offset or overflow, (`IndexRange`) on an
///   out-of-range index.
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

/// Rebuild the level atom `variable + offset` through `var` and `succ`;
/// `offset` is bounded by the caller.
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

#[cfg(test)]
mod tests
{
    use super::super::write::write;
    use super::DecodedDeclaration;
    use super::decode;
    use crate::decl::DeclarationContent;
    use crate::decl::LevelSignature;
    use crate::env::Environment;
    use crate::term::DeBruijnIndex;
    use crate::term::Value;

    /// Decode retains within-declaration sharing: a body `pair(x, x)` decodes
    /// to a pair whose two children are the SAME arena id (the format's
    /// shared entry reuses one id — decode is arena construction, never
    /// expansion).
    #[test]
    fn decode_retains_within_declaration_sharing()
    {
        let mut environment = Environment::new();
        let mut builder = environment.stage();
        let arena = builder.arena();
        let x = arena.value_variable(DeBruijnIndex::from(0_u32));
        let body = arena.value_pair(x, x);
        let declared = arena.value_type_unit();
        let declaration = builder.def(LevelSignature::monomorphic(), declared, body);
        let _id = environment.add_decl_unchecked(declaration);
        let bytes = write(&environment);
        let artifact = decode(&bytes).expect("the shared artifact decodes");
        let decoded = artifact
            .declarations()
            .first()
            .expect("one declaration decodes");
        let body_id = match *decoded.declaration().content() {
            | DeclarationContent::Def { body, .. } => body,
            | DeclarationContent::Axiom { .. } => panic!("a Def was decoded"),
        };
        match artifact.arena().value(body_id) {
            | Some(&Value::Pair(first, second)) => assert_eq!(
                first, second,
                "the shared pair children decode to one arena id (sharing retained)"
            ),
            | _ => panic!("the body decodes to a pair"),
        }
    }

    /// Decode retains cross-declaration sharing: two declarations whose bodies
    /// are the same `unit` value share one arena entry, so the second
    /// declaration's body id equals the first's.
    #[test]
    fn decode_retains_cross_declaration_sharing()
    {
        let mut environment = Environment::new();
        // The first declaration's body is a bare unit.
        let mut builder = environment.stage();
        let first_declared = builder.arena().value_type_unit();
        let first_body = builder.arena().value_unit();
        let first = builder.def(LevelSignature::monomorphic(), first_declared, first_body);
        let _first_id = environment.add_decl_unchecked(first);
        // The second's body is pair(unit, unit) — the units share the first's entry.
        let mut builder = environment.stage();
        let second_declared = builder.arena().value_type_unit();
        let left = builder.arena().value_unit();
        let right = builder.arena().value_unit();
        let second_body = builder.arena().value_pair(left, right);
        let second = builder.def(LevelSignature::monomorphic(), second_declared, second_body);
        let _second_id = environment.add_decl_unchecked(second);

        let bytes = write(&environment);
        let artifact = decode(&bytes).expect("the shared artifact decodes");
        let decls = artifact.declarations();
        let body_of = |declaration: &DecodedDeclaration| match *declaration.declaration().content()
        {
            | DeclarationContent::Def { body, .. } => body,
            | DeclarationContent::Axiom { .. } => panic!("a Def was decoded"),
        };
        let unit_entry = body_of(decls.first().expect("the first declaration decodes"));
        let pair_entry = body_of(decls.get(1).expect("the second declaration decodes"));
        match artifact.arena().value(pair_entry) {
            | Some(&Value::Pair(left_child, right_child)) => {
                assert_eq!(
                    left_child, right_child,
                    "the pair's own children share one id"
                );
                assert_eq!(
                    left_child, unit_entry,
                    "the second declaration's unit shares the first's arena id (cross-declaration sharing)"
                );
            },
            | _ => panic!("the second declaration decodes to a pair"),
        }
    }
}
