//! The **certificate/transport adapter** — the narrow boundary at which an
//! in-process primitive label is replaced by a durable transport-step
//! identity (tracker item `gandr-4o8a`).
//!
//! [`prim_address`], [`cell_address`], and [`causal_past_address`] mint
//! **process-local** labels: fixed-seed FNV-1a over [`core::hash::Hash`]
//! writes, stable for one build of one target and no further. They key the
//! factorization, order the schedule, and label the flow, and none of that
//! changes. What they may never do is leave the process. This module is the
//! separation: [`CanonicalStepEncoding`] derives **canonical cell and
//! application-position bytes**, [`transport_step_id`] mints a fixed-width
//! [`TransportStepId`] over them through `gandr-storage-artifact`'s versioned,
//! domain-separated BLAKE3 framing, and [`transport_step_index`] is the
//! enforcement seam — the only construction that keys a certificate's
//! primitives by transport identity, refusing a collision rather than
//! believing it.
//!
//! The split is total: there is no conversion between [`PrimId`] and
//! [`TransportStepId`] in either direction, no shared representation (128-bit
//! target-width digest against 256-bit canonical digest), and no second
//! portable product — the transport boundary addresses a **step** (a cell
//! together with where it fired); a position-free portable cell address
//! deliberately does not exist.
//!
//! # Canonicality obligations
//!
//! The encoding a [`CanonicalStepEncoding`] implementation emits is the whole
//! portability story, so its contract is stronger than
//! [`core::hash::Hash`]'s: structurally equal cells and positions must produce
//! byte-identical images on **every** target and in **every** process, and
//! structurally distinct ones must produce distinct images (injectivity), so
//! that an identity collision is a BLAKE3 collision and nothing else. The
//! framing (big-endian, fixed u64 widths, length prefixes, version, domain) is
//! pinned by `gandr-storage-artifact`; the implementation pins only field
//! order and selection. The shipped implementation covers all five structural
//! [`Cell`] fields — `meta` included, because [`Cell`] equality and store
//! deduplication include it and its fields are public — and metavariable
//! names are encoded structurally, with no α-quotient.
//!
//! [`prim_address`]: crate::normal_form::prim_address
//! [`cell_address`]: crate::normal_form::cell_address
//! [`causal_past_address`]: crate::normal_form::causal_past_address
//! [`PrimId`]: crate::normal_form::PrimId

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::collections::btree_map::Entry;
use alloc::vec::Vec;

use gandr_core_sequent::il::Polarity;
use gandr_storage_artifact::CanonicalBytes;
use gandr_storage_artifact::CanonicalU64;
use gandr_storage_artifact::StepIdEncoder;
use gandr_storage_artifact::StepIdError;
use gandr_storage_artifact::TransportStepId;

use crate::alphabet::CellAlphabet;
use crate::boundary::PrimMultiplicity;
use crate::cell::Cell;
use crate::cell::CellId;
use crate::cell::CellStore;
use crate::normal_form::PrimCert;
use crate::normal_form::TraceletNf;
use crate::pattern::Cat;
use crate::pattern::CmdPat;
use crate::pattern::ConsPat;
use crate::pattern::MetaVar;
use crate::pattern::Pos;
use crate::pattern::ProdPat;
use crate::pattern::Sym;
use crate::sequent::CellMeta;
use crate::sequent::CellProvenance;
use crate::sequent::CellVariance;
use crate::sequent::EtaKind;
use crate::sequent::Orientation;
use crate::sequent::SequentAlphabet;

/// A **canonical variant tag** — the nominal boundary every enum discriminant
/// and flag crosses on its way into the framing.
///
/// Tags are `u64`-valued so the framing sees one integer width everywhere;
/// the wrapper keeps them nominally distinct from the counts and lengths that
/// share that width.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct CanonicalTag(u64);

impl From<u64> for CanonicalTag
{
    #[inline]
    fn from(value: u64) -> Self
    {
        Self(value)
    }
}

impl From<CanonicalTag> for u64
{
    #[inline]
    fn from(value: CanonicalTag) -> Self
    {
        value.0
    }
}

/// The canonical tag of the single [`CmdPat`] variant (a cut).
const TAG_CMD_CUT: CanonicalTag = CanonicalTag(0);
/// The canonical tag of a producer metavariable leaf.
const TAG_PROD_META: CanonicalTag = CanonicalTag(0);
/// The canonical tag of a constructor application.
const TAG_PROD_CTOR: CanonicalTag = CanonicalTag(1);
/// The canonical tag of a consumer metavariable leaf.
const TAG_CONS_META: CanonicalTag = CanonicalTag(0);
/// The canonical tag of an operation frame.
const TAG_CONS_OP: CanonicalTag = CanonicalTag(1);
/// The canonical tag of a return-side constructor frame.
const TAG_CONS_FRAME: CanonicalTag = CanonicalTag(2);
/// The canonical tag of the terminal consumer.
const TAG_CONS_TOP: CanonicalTag = CanonicalTag(3);
/// The canonical tag of a positive cut polarity.
const TAG_POLARITY_POSITIVE: CanonicalTag = CanonicalTag(0);
/// The canonical tag of a negative cut polarity.
const TAG_POLARITY_NEGATIVE: CanonicalTag = CanonicalTag(1);
/// The canonical tag of a producer-category metavariable.
const TAG_CAT_PRODUCER: CanonicalTag = CanonicalTag(0);
/// The canonical tag of a consumer-category metavariable.
const TAG_CAT_CONSUMER: CanonicalTag = CanonicalTag(1);
/// The canonical tag of a polarity-derived orientation.
const TAG_ORIENTATION_POLARITY: CanonicalTag = CanonicalTag(0);
/// The canonical tag of a completion-derived orientation.
const TAG_ORIENTATION_COMPLETION: CanonicalTag = CanonicalTag(1);
/// The canonical tag of a surface-rule provenance.
const TAG_PROVENANCE_SURFACE: CanonicalTag = CanonicalTag(0);
/// The canonical tag of the μ/μ̃ critical-pair provenance.
const TAG_PROVENANCE_MU_MU_TILDE: CanonicalTag = CanonicalTag(1);
/// The canonical tag of a frame-defining-cell provenance.
const TAG_PROVENANCE_FRAME: CanonicalTag = CanonicalTag(2);
/// The canonical tag of a data-η provenance.
const TAG_PROVENANCE_ETA_DATA: CanonicalTag = CanonicalTag(3);
/// The canonical tag of a codata-η provenance.
const TAG_PROVENANCE_ETA_CODATA: CanonicalTag = CanonicalTag(4);
/// The canonical tag of a completion-derived provenance.
const TAG_PROVENANCE_COMPLETION: CanonicalTag = CanonicalTag(5);
/// The canonical tag of a producer-side variance.
const TAG_VARIANCE_PRODUCER: CanonicalTag = CanonicalTag(0);
/// The canonical tag of a consumer-side variance.
const TAG_VARIANCE_CONSUMER: CanonicalTag = CanonicalTag(1);
/// The canonical tag of a mixed (dinatural) variance.
const TAG_VARIANCE_MIXED: CanonicalTag = CanonicalTag(2);
/// The canonical encoding of a `false` flag.
const TAG_FLAG_FALSE: CanonicalTag = CanonicalTag(0);
/// The canonical encoding of a `true` flag.
const TAG_FLAG_TRUE: CanonicalTag = CanonicalTag(1);

/// The **canonical step encoding** of an alphabet — the one deep method of the
/// transport boundary, deriving canonical bytes for a resolved cell's content
/// and an application position.
///
/// This is an **opt-in extension** of the [`CellAlphabet`] contract, not a
/// growth of it: the generic engines keep their [`core::hash::Hash`]-only
/// discipline, and an alphabet joins the transport boundary by implementing
/// this trait.
///
/// # Contract
/// - ensures: every field is streamed through `encoder`'s canonical framing
///   (fixed-width big-endian integers, length-prefixed bytes), so the byte
///   image is a deterministic function of the **structural** content of `cell`
///   and `at` alone — never of a store index, an insertion order, a pointer, a
///   hash-seed, or a target width. Structurally equal `(cell, at)` pairs
///   produce byte-identical images on every target and in every process.
/// - ensures: the encoding is **injective** over the alphabet's structural
///   content: distinct `(cell, at)` pairs produce distinct images, so a
///   [`TransportStepId`] collision is a BLAKE3 collision and nothing else.
/// - fails: [`StepIdError`] when a count, length, or position step exceeds the
///   canonical u64 width (unreachable on a 64-bit target; the check is the
///   width pin, not an expectation).
/// - panics: none.
///
/// # Errors
/// [`StepIdError`].
pub trait CanonicalStepEncoding: CellAlphabet
{
    /// Stream the canonical bytes of `cell` followed by `at` into `encoder`,
    /// in the implementation's fixed field order.
    ///
    /// # Errors
    /// [`StepIdError`].
    fn encode_step(
        cell: &Cell<Self>,
        at: &Self::Pos,
        encoder: &mut StepIdEncoder,
    ) -> Result<(), StepIdError>;
}

/// Why minting or indexing by a transport-step identity failed.
///
/// The mirror of [`crate::normal_form::NormalFormObstruction`] at the
/// transport boundary: refusal is data, and a collision is declined rather
/// than believed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransportStepObstruction<A: CellAlphabet = SequentAlphabet>
{
    /// A recorded primitive names a cell the store does not hold.
    UnknownCell
    {
        /// The identifier that resolved to nothing.
        cell: CellId,
    },
    /// A canonical field exceeded the fixed u64 width.
    Encoding(StepIdError),
    /// Two distinct primitives minted one transport-step identity.
    ///
    /// Refused for the same reason
    /// [`crate::normal_form::NormalFormObstruction::ContentAddressCollision`]
    /// is: the identity is an index and an ordering device, never an identity
    /// witness, so where two primitives cannot be told apart the honest
    /// outcome is to decline the index.
    ContentAddressCollision
    {
        /// The identity both primitives minted.
        address: TransportStepId,
        /// The primitive already recorded under it.
        held: Box<PrimCert<A>>,
        /// The primitive that collided with it.
        offered: Box<PrimCert<A>>,
    },
}

impl<A: CellAlphabet> From<StepIdError> for TransportStepObstruction<A>
{
    #[inline]
    fn from(error: StepIdError) -> Self
    {
        Self::Encoding(error)
    }
}

/// A normal form's primitive factorization keyed by **transport-step
/// identity** — the portable projection of [`TraceletNf::primitives`].
///
/// Multiplicity and keyed-map semantics are preserved exactly: one entry per
/// distinct identity, each carrying its [`PrimCert`] and its occurrence count,
/// in the map's deterministic key order. The map key order is the transport
/// identity's, which — unlike the in-process label order — is the same in
/// every process.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransportStepIndex<A: CellAlphabet = SequentAlphabet>
{
    /// The graded primitives, keyed by transport-step identity.
    entries: BTreeMap<TransportStepId, (PrimCert<A>, PrimMultiplicity)>,
}

impl<A: CellAlphabet> TransportStepIndex<A>
{
    /// The graded primitive recorded under `address`, if any.
    #[inline]
    #[must_use]
    pub fn get(
        &self,
        address: &TransportStepId,
    ) -> Option<&(PrimCert<A>, PrimMultiplicity)>
    {
        self.entries.get(address)
    }
}

/// Mint the **transport-step identity** of one primitive — the resolved
/// `cell` together with the position `at` it fires at.
///
/// This is the only sanctioned mint of a portable step identity: the
/// versioned step-domain framing is applied by [`StepIdEncoder::begin`], and
/// the canonical bytes are the alphabet's [`CanonicalStepEncoding`]. There is
/// no path from a [`crate::normal_form::PrimId`] to a [`TransportStepId`] —
/// the in-process label is never an input, here or anywhere.
///
/// # Contract
/// - requires: `cell` is the resolved cell content (as the storing
///   [`CellStore`] returns it), `at` the position the step fires at.
/// - ensures: equal structural `(cell, at)` content mints one identity on every
///   target and in every process, independently of how the store assigned its
///   [`CellId`]s; distinct content mints distinct identities up to BLAKE3
///   collision resistance.
/// - provides: the durable, portable identity a persistence or transport
///   consumer may serialize.
/// - fails: [`StepIdError`] from the canonical encoding's width checks.
/// - panics: none.
///
/// # Errors
/// [`StepIdError`].
///
/// # Adequacy
/// - hypothesis: L3 pointwise — the decision surface is "same canonical
///   content, same identity, everywhere", separated by a hardcoded golden
///   digest, an independently rebuilt equal cell, the same cell at two
///   positions, two cells of different content, and two insertion orders of one
///   store's content.
/// - witness: `transport::tests::the_v1_golden_step_identity_is_stable`
/// - witness: `transport::tests::an_independently_rebuilt_cell_mints_the_same_identity`
/// - witness: `transport::tests::the_identity_reads_the_position`
/// - witness: `transport::tests::the_identity_reads_the_cell_content`
/// - witness: `transport::tests::the_identity_is_stable_across_store_insertion_orders`
#[inline]
pub fn transport_step_id<A>(
    cell: &Cell<A>,
    at: &A::Pos,
) -> Result<TransportStepId, StepIdError>
where
    A: CanonicalStepEncoding,
{
    let mut encoder = StepIdEncoder::begin();
    A::encode_step(cell, at, &mut encoder)?;
    Ok(encoder.finish())
}

/// Project a normal form's primitive factorization onto transport-step
/// identities — the **enforcement seam** of the transport boundary.
///
/// Each recorded primitive is re-minted from its resolved cell content and
/// position and keyed by the result; the grading is carried verbatim, and
/// merging two records of one primitive under one identity **sums** their
/// multiplicities exactly as [`crate::normal_form::normalize`]'s
/// factorization does (saturating at the width). Two **distinct** primitives
/// under one identity are a collision and are refused.
///
/// # Contract
/// - requires: `normal_form` was taken against `store`, so every [`CellId`] its
///   primitives name resolves there — the discharge [`transport_step_id`]
///   documents for the resolved content.
/// - ensures: `Ok(index)` whose entries are the normal form's graded
///   factorization re-keyed by transport identity, multiplicities preserved;
///   the iteration reads [`TraceletNf::primitives`] in its own deterministic
///   order, so the result is a function of the factorization alone.
/// - provides: the only construction that keys certificates by transport
///   identity — a persistence or transport consumer builds this, serializes it,
///   and resolves back to local [`CellId`]s by content lookup before replay.
/// - fails: [`TransportStepObstruction::UnknownCell`] when a recorded cell is
///   absent from `store`, [`TransportStepObstruction::Encoding`] on a width
///   overflow, and [`TransportStepObstruction::ContentAddressCollision`] when
///   two distinct primitives mint one identity.
/// - panics: none.
///
/// # Errors
/// [`TransportStepObstruction`].
///
/// # Adequacy
/// - hypothesis: L2 — the outcomes own separate decision surfaces: the
///   projection of a replay-verified normal form (twice-normalized, and against
///   a fused/two-step certificate pair), and the refusal of a fabricated shared
///   identity at the insert seam, with the merge arm separated by summing two
///   gradings of one primitive.
/// - witness: `transport::tests::the_index_preserves_the_graded_factorization`
/// - witness: `transport::tests::the_index_is_deterministic_across_repeated_normalization`
/// - witness: `transport::tests::distinct_factorizations_index_distinctly`
/// - witness: `transport::tests::a_shared_identity_with_distinct_content_is_refused`
/// - witness: `transport::tests::a_shared_identity_with_equal_content_sums_the_grading`
#[inline]
pub fn transport_step_index<A>(
    normal_form: &TraceletNf<A>,
    store: &CellStore<A>,
) -> Result<TransportStepIndex<A>, TransportStepObstruction<A>>
where
    A: CanonicalStepEncoding,
{
    let mut entries = BTreeMap::new();
    for graded in normal_form.primitives.values() {
        let cert = &graded.0;
        let multiplicity = &graded.1;
        let step = cert.step();
        let Some(cell) = store.get(step.cell)
        else {
            return Err(TransportStepObstruction::UnknownCell { cell: step.cell });
        };
        let address = transport_step_id::<A>(cell, &step.at)?;
        insert_step(&mut entries, address, cert.clone(), *multiplicity)?;
    }
    Ok(TransportStepIndex { entries })
}

/// Insert one graded primitive into a transport-keyed factorization,
/// mirroring [`crate::normal_form::normalize`]'s insert: a vacant key takes
/// the entry, an equal certificate under a shared key has the gradings summed
/// (saturating at the width), and a distinct certificate under a shared key
/// is a refused collision.
///
/// # Contract
/// - ensures: `Ok(())` with the entry recorded or merged;
///   `Err(ContentAddressCollision)` carrying both certificates otherwise.
/// - panics: none.
///
/// # Errors
/// [`TransportStepObstruction`].
fn insert_step<A>(
    entries: &mut BTreeMap<TransportStepId, (PrimCert<A>, PrimMultiplicity)>,
    address: TransportStepId,
    cert: PrimCert<A>,
    multiplicity: PrimMultiplicity,
) -> Result<(), TransportStepObstruction<A>>
where
    A: CellAlphabet,
{
    match entries.entry(address) {
        | Entry::Vacant(slot) => {
            slot.insert((cert, multiplicity));
            Ok(())
        },
        | Entry::Occupied(mut slot) => {
            let graded = slot.get_mut();
            if graded.0 != cert {
                return Err(TransportStepObstruction::ContentAddressCollision {
                    address,
                    held: Box::new(graded.0.clone()),
                    offered: Box::new(cert),
                });
            }
            graded.1 =
                PrimMultiplicity::from(u32::from(graded.1).saturating_add(u32::from(multiplicity)));
            Ok(())
        },
    }
}

/// One pending pattern node in the explicit pre-order work stack the sequent
/// encoding drains — the no-recursion discipline for a tree whose depth is
/// author-controlled but not statically bounded.
enum PatternWork<'cell>
{
    /// A producer pattern awaiting its tag and fields.
    Prod(&'cell ProdPat),
    /// A consumer pattern awaiting its tag and fields.
    Cons(&'cell ConsPat),
    /// A metavariable leaf awaiting its name and category.
    Var(&'cell MetaVar),
}

impl CanonicalStepEncoding for SequentAlphabet
{
    /// The sequent canonical step encoding: the cell's five structural fields
    /// in fixed order (`lhs`, `rhs`, `orient`, `provenance`, `meta`), then the
    /// application position.
    ///
    /// # Errors
    /// [`StepIdError`].
    #[inline]
    fn encode_step(
        cell: &Cell<Self>,
        at: &Self::Pos,
        encoder: &mut StepIdEncoder,
    ) -> Result<(), StepIdError>
    {
        encode_sequent_cmd(&cell.lhs, encoder)?;
        encode_sequent_cmd(&cell.rhs, encoder)?;
        encode_sequent_orientation(cell.orient, encoder);
        encode_sequent_provenance(cell.provenance, encoder);
        encode_sequent_meta(&cell.meta, encoder)?;
        encode_sequent_pos(at, encoder)
    }
}

/// Stream one canonical tag field.
///
/// # Contract
/// - ensures: `tag` is framed as one canonical u64.
/// - panics: none.
#[inline]
fn put_tag(
    tag: CanonicalTag,
    encoder: &mut StepIdEncoder,
)
{
    encoder.put_u64(CanonicalU64::from(u64::from(tag)));
}

/// Stream a symbol as one length-prefixed UTF-8 byte field.
///
/// # Contract
/// - fails: [`StepIdError`] on a width overflow of the name's length.
/// - panics: none.
///
/// # Errors
/// [`StepIdError`].
#[inline]
fn encode_sequent_sym(
    sym: &Sym,
    encoder: &mut StepIdEncoder,
) -> Result<(), StepIdError>
{
    let name = CanonicalBytes::try_from(sym.as_ref().as_bytes())?;
    encoder.put_bytes(name);
    Ok(())
}

/// Stream a metavariable as its length-prefixed name followed by its category
/// tag — structural, with no α-quotient: replay keys holes by name, and the
/// transport identity inherits that identity discipline.
///
/// # Contract
/// - fails: [`StepIdError`] on a width overflow of the name's length.
/// - panics: none.
///
/// # Errors
/// [`StepIdError`].
#[inline]
fn encode_sequent_metavar(
    var: &MetaVar,
    encoder: &mut StepIdEncoder,
) -> Result<(), StepIdError>
{
    let name = CanonicalBytes::try_from(var.name.as_bytes())?;
    encoder.put_bytes(name);
    let tag = match var.cat {
        | Cat::Producer => TAG_CAT_PRODUCER,
        | Cat::Consumer => TAG_CAT_CONSUMER,
    };
    put_tag(tag, encoder);
    Ok(())
}

/// Stream a command pattern in pre-order from an explicit work stack.
///
/// # Contract
/// - ensures: every node emits its variant tag followed by its fields in
///   declaration order, so the image is a deterministic pre-order walk of the
///   pattern tree.
/// - fails: [`StepIdError`] from a symbol, metavariable, or argument-count
///   width check.
/// - panics: none.
///
/// # Errors
/// [`StepIdError`].
fn encode_sequent_cmd(
    cmd: &CmdPat,
    encoder: &mut StepIdEncoder,
) -> Result<(), StepIdError>
{
    let mut work = Vec::new();
    match *cmd {
        | CmdPat::Cut {
            ref pol,
            ref prod,
            ref cons,
        } => {
            put_tag(TAG_CMD_CUT, encoder);
            let tag = match *pol {
                | Polarity::Positive => TAG_POLARITY_POSITIVE,
                | Polarity::Negative => TAG_POLARITY_NEGATIVE,
            };
            put_tag(tag, encoder);
            work.push(PatternWork::Cons(cons));
            work.push(PatternWork::Prod(prod));
        },
    }
    while let Some(next) = work.pop() {
        match next {
            | PatternWork::Prod(prod) => match *prod {
                | ProdPat::Meta(ref var) => {
                    put_tag(TAG_PROD_META, encoder);
                    work.push(PatternWork::Var(var));
                },
                | ProdPat::Ctor { ref ctor, ref args } => {
                    put_tag(TAG_PROD_CTOR, encoder);
                    encode_sequent_sym(ctor, encoder)?;
                    let count = CanonicalU64::try_from(args.len())?;
                    encoder.put_u64(count);
                    for arg in args.iter().rev() {
                        work.push(PatternWork::Prod(arg));
                    }
                },
            },
            | PatternWork::Cons(cons) => match *cons {
                | ConsPat::Meta(ref var) => {
                    put_tag(TAG_CONS_META, encoder);
                    work.push(PatternWork::Var(var));
                },
                | ConsPat::Op {
                    ref op,
                    ref args,
                    ref ret,
                } => {
                    put_tag(TAG_CONS_OP, encoder);
                    encode_sequent_sym(op, encoder)?;
                    let count = CanonicalU64::try_from(args.len())?;
                    encoder.put_u64(count);
                    work.push(PatternWork::Cons(ret.as_ref()));
                    for arg in args.iter().rev() {
                        work.push(PatternWork::Prod(arg));
                    }
                },
                | ConsPat::Frame { ref ctor, ref ret } => {
                    put_tag(TAG_CONS_FRAME, encoder);
                    encode_sequent_sym(ctor, encoder)?;
                    work.push(PatternWork::Cons(ret.as_ref()));
                },
                | ConsPat::Top => {
                    put_tag(TAG_CONS_TOP, encoder);
                },
            },
            | PatternWork::Var(var) => {
                encode_sequent_metavar(var, encoder)?;
            },
        }
    }
    Ok(())
}

/// Stream an application position as its step count followed by each child
/// index, every integer width-checked into the canonical u64 width.
///
/// # Contract
/// - fails: [`StepIdError::WidthOverflow`] when a step or the count exceeds the
///   canonical width (unreachable on a 64-bit target; the check is the width
///   pin).
/// - panics: none.
///
/// # Errors
/// [`StepIdError`].
fn encode_sequent_pos(
    at: &Pos,
    encoder: &mut StepIdEncoder,
) -> Result<(), StepIdError>
{
    let steps = at.as_ref();
    let count = CanonicalU64::try_from(steps.len())?;
    encoder.put_u64(count);
    for step in steps {
        let step = CanonicalU64::try_from(*step)?;
        encoder.put_u64(step);
    }
    Ok(())
}

/// Stream an orientation tag.
///
/// # Contract
/// - panics: none.
#[inline]
fn encode_sequent_orientation(
    orient: Orientation,
    encoder: &mut StepIdEncoder,
)
{
    let tag = match orient {
        | Orientation::PolarityDerived => TAG_ORIENTATION_POLARITY,
        | Orientation::CompletionDerived => TAG_ORIENTATION_COMPLETION,
    };
    put_tag(tag, encoder);
}

/// Stream a provenance tag, with the η kind flattened into the tag (the two
/// η laws are distinct provenances of one cell).
///
/// # Contract
/// - panics: none.
#[inline]
fn encode_sequent_provenance(
    provenance: CellProvenance,
    encoder: &mut StepIdEncoder,
)
{
    let tag = match provenance {
        | CellProvenance::SurfaceRule => TAG_PROVENANCE_SURFACE,
        | CellProvenance::MuMuTilde => TAG_PROVENANCE_MU_MU_TILDE,
        | CellProvenance::FrameDefining => TAG_PROVENANCE_FRAME,
        | CellProvenance::Eta(EtaKind::Data) => TAG_PROVENANCE_ETA_DATA,
        | CellProvenance::Eta(EtaKind::Codata) => TAG_PROVENANCE_ETA_CODATA,
        | CellProvenance::DerivedByCompletion => TAG_PROVENANCE_COMPLETION,
    };
    put_tag(tag, encoder);
}

/// Stream the derived metadata — encoded because [`Cell`] equality and store
/// deduplication include the public `meta` field, so the canonical image must
/// see it too.
///
/// # Contract
/// - ensures: the per-metavariable entries in their recorded order, each as
///   metavariable, variance tag, and linearity flag, then the invertibility
///   flag.
/// - fails: [`StepIdError`] from a metavariable or count width check.
/// - panics: none.
///
/// # Errors
/// [`StepIdError`].
fn encode_sequent_meta(
    meta: &CellMeta,
    encoder: &mut StepIdEncoder,
) -> Result<(), StepIdError>
{
    let count = CanonicalU64::try_from(meta.vars.len())?;
    encoder.put_u64(count);
    for var in &meta.vars {
        encode_sequent_metavar(&var.var, encoder)?;
        let tag = match var.variance {
            | CellVariance::Producer => TAG_VARIANCE_PRODUCER,
            | CellVariance::Consumer => TAG_VARIANCE_CONSUMER,
            | CellVariance::Mixed => TAG_VARIANCE_MIXED,
        };
        put_tag(tag, encoder);
        let linear = if bool::from(var.linear) {
            TAG_FLAG_TRUE
        }
        else {
            TAG_FLAG_FALSE
        };
        put_tag(linear, encoder);
    }
    let invertible = if bool::from(meta.invertible) {
        TAG_FLAG_TRUE
    }
    else {
        TAG_FLAG_FALSE
    };
    put_tag(invertible, encoder);
    Ok(())
}

/// The golden, determinism, sensitivity, insertion-order, index, and
/// collision witnesses of the transport boundary.
#[cfg(test)]
mod tests
{
    use alloc::string::ToString as _;

    use super::*;
    use crate::normal_form::normalize;
    use crate::overlap::OverlapKind;
    use crate::overlap::enumerate_overlaps;
    use crate::rewrite::CellApp;
    use crate::sequent::frame_defining_cell;
    use crate::tracelet::Tracelet;
    use crate::tracelet::derive_fused;

    /// (add-S): ⟨Succ(m) | add(n; α)⟩ ~> ⟨m | add(n; Succ⁻(α))⟩.
    fn add_s() -> Cell
    {
        let lhs = CmdPat::cut(
            Polarity::Positive,
            ProdPat::ctor("Succ", [ProdPat::meta("m")]),
            ConsPat::op("add", [ProdPat::meta("n")], ConsPat::meta("alpha")),
        );
        let rhs = CmdPat::cut(
            Polarity::Positive,
            ProdPat::meta("m"),
            ConsPat::op(
                "add",
                [ProdPat::meta("n")],
                ConsPat::frame("Succ", ConsPat::meta("alpha")),
            ),
        );
        Cell::new(
            lhs,
            rhs,
            Orientation::PolarityDerived,
            CellProvenance::SurfaceRule,
        )
    }

    /// The fused-cell store and the tracelet `derive_fused` certifies — the
    /// same fixture `normal_form::tests` normalizes.
    fn fused_fixture() -> (CellStore, Tracelet)
    {
        let mut store = CellStore::new();
        let frame = store.insert(frame_defining_cell(&Sym::new("Succ")));
        let add = store.insert(add_s());
        let composition = enumerate_overlaps(&store)
            .into_iter()
            .find(|candidate| {
                candidate.kind == OverlapKind::Composition
                    && candidate.left == frame
                    && candidate.right == add
            })
            .expect("the composition overlap exists");
        let (_fused, tracelet) =
            derive_fused(&composition, &mut store).expect("the fused cell is derived");
        (store, tracelet)
    }

    #[test]
    fn the_v1_golden_step_identity_is_stable()
    {
        let frame = frame_defining_cell(&Sym::new("Succ"));
        let identity =
            transport_step_id::<SequentAlphabet>(&frame, &Pos::root()).expect("the cell encodes");
        assert_eq!(
            "c04ba7d738000bed6f41d3115d5bfaf7d92a4566e0d7b92ced60d5a3cbd3fdc8",
            identity.to_string(),
            "the v1 canonical encoding mints the recorded golden digest",
        );
    }

    #[test]
    fn an_independently_rebuilt_cell_mints_the_same_identity()
    {
        let first = frame_defining_cell(&Sym::new("Succ"));
        let second = frame_defining_cell(&Sym::new("Succ"));
        assert_ne!(
            core::ptr::addr_of!(first),
            core::ptr::addr_of!(second),
            "the fixture needs two distinct values, not one value twice"
        );
        let one =
            transport_step_id::<SequentAlphabet>(&first, &Pos::root()).expect("the first encodes");
        let two = transport_step_id::<SequentAlphabet>(&second, &Pos::root())
            .expect("the second encodes");
        assert_eq!(one, two, "the identity reads content, never allocation");
    }

    #[test]
    fn the_identity_reads_the_position()
    {
        let frame = frame_defining_cell(&Sym::new("Succ"));
        let nested = Pos(alloc::vec![0_usize].into_boxed_slice());
        let root =
            transport_step_id::<SequentAlphabet>(&frame, &Pos::root()).expect("the root encodes");
        let deep =
            transport_step_id::<SequentAlphabet>(&frame, &nested).expect("the nested encodes");
        assert_ne!(root, deep, "one cell at two positions is two steps");
    }

    #[test]
    fn the_identity_reads_the_cell_content()
    {
        let succ = frame_defining_cell(&Sym::new("Succ"));
        let pred = frame_defining_cell(&Sym::new("Pred"));
        let left =
            transport_step_id::<SequentAlphabet>(&succ, &Pos::root()).expect("the first encodes");
        let right =
            transport_step_id::<SequentAlphabet>(&pred, &Pos::root()).expect("the second encodes");
        assert_ne!(left, right, "two cells of distinct content are two steps");
    }

    #[test]
    fn the_identity_is_stable_across_store_insertion_orders()
    {
        let frame = frame_defining_cell(&Sym::new("Succ"));
        let add = add_s();
        let mut forward = CellStore::new();
        let forward_frame = forward.insert(frame.clone());
        let mut reverse = CellStore::new();
        let reverse_add = reverse.insert(add);
        let reverse_frame = reverse.insert(frame);
        assert_ne!(
            forward_frame, reverse_frame,
            "the fixture needs the two stores to assign the frame cell different ids"
        );
        assert_ne!(
            reverse_add, reverse_frame,
            "and the reversed store to order them oppositely"
        );
        let resolved_forward = forward
            .get(forward_frame)
            .expect("the forward store holds it");
        let resolved_reverse = reverse
            .get(reverse_frame)
            .expect("the reverse store holds it");
        let one = transport_step_id::<SequentAlphabet>(resolved_forward, &Pos::root())
            .expect("the forward resolution encodes");
        let two = transport_step_id::<SequentAlphabet>(resolved_reverse, &Pos::root())
            .expect("the reverse resolution encodes");
        assert_eq!(
            one, two,
            "the transport identity is over resolved content, never the insertion-order CellId"
        );
    }

    #[test]
    fn the_index_preserves_the_graded_factorization()
    {
        let (store, tracelet) = fused_fixture();
        let normal = normalize(
            &store,
            &tracelet.overlap.peak,
            &tracelet.joins_at,
            &tracelet.path_a,
        )
        .expect("the two-step derivation normalizes");
        let index = transport_step_index(&normal, &store).expect("the factorization indexes");
        for graded in normal.primitives.values() {
            let cert = &graded.0;
            let multiplicity = &graded.1;
            let cell = store.get(cert.step().cell).expect("the cell resolves");
            let address =
                transport_step_id::<SequentAlphabet>(cell, &cert.step().at).expect("it encodes");
            assert_eq!(
                Some(&(cert.clone(), *multiplicity)),
                index.get(&address),
                "every graded primitive survives the re-keying exactly"
            );
        }
    }

    #[test]
    fn the_index_is_deterministic_across_repeated_normalization()
    {
        let (store, tracelet) = fused_fixture();
        let first = normalize(
            &store,
            &tracelet.overlap.peak,
            &tracelet.joins_at,
            &tracelet.path_a,
        )
        .expect("the first normalization replays");
        let second = normalize(
            &store,
            &tracelet.overlap.peak,
            &tracelet.joins_at,
            &tracelet.path_a,
        )
        .expect("the second normalization replays");
        let one = transport_step_index(&first, &store).expect("the first indexes");
        let two = transport_step_index(&second, &store).expect("the second indexes");
        assert_eq!(one, two, "one derivation indexes one way, every time");
    }

    #[test]
    fn distinct_factorizations_index_distinctly()
    {
        let (store, tracelet) = fused_fixture();
        let two_step = normalize(
            &store,
            &tracelet.overlap.peak,
            &tracelet.joins_at,
            &tracelet.path_a,
        )
        .expect("the two-step leg normalizes");
        let fused = normalize(
            &store,
            &tracelet.overlap.peak,
            &tracelet.joins_at,
            &tracelet.path_b,
        )
        .expect("the fused leg normalizes");
        let left = transport_step_index(&two_step, &store).expect("the two-step leg indexes");
        let right = transport_step_index(&fused, &store).expect("the fused leg indexes");
        assert_ne!(
            left, right,
            "the fused and two-step factorizations are distinct, and stay distinct re-keyed"
        );
    }

    #[test]
    fn a_shared_identity_with_distinct_content_is_refused()
    {
        let address = TransportStepId::from([7_u8; 32]);
        let held = PrimCert::<SequentAlphabet>(CellApp {
            cell: CellId(0_usize),
            at: Pos::root(),
        });
        let offered = PrimCert::<SequentAlphabet>(CellApp {
            cell: CellId(1_usize),
            at: Pos::root(),
        });
        let mut entries = BTreeMap::new();
        insert_step(&mut entries, address, held, PrimMultiplicity::from(1_u32))
            .expect("the first insert is vacant");
        let refusal = insert_step(
            &mut entries,
            address,
            offered,
            PrimMultiplicity::from(1_u32),
        );
        assert!(
            matches!(
                &refusal,
                Err(TransportStepObstruction::ContentAddressCollision { .. })
            ),
            "distinct content under one identity is refused, never merged: {refusal:?}"
        );
    }

    #[test]
    fn a_shared_identity_with_equal_content_sums_the_grading()
    {
        let address = TransportStepId::from([9_u8; 32]);
        let cert = PrimCert::<SequentAlphabet>(CellApp {
            cell: CellId(0_usize),
            at: Pos::root(),
        });
        let mut entries = BTreeMap::new();
        insert_step(
            &mut entries,
            address,
            cert.clone(),
            PrimMultiplicity::from(1_u32),
        )
        .expect("the first insert is vacant");
        insert_step(
            &mut entries,
            address,
            cert.clone(),
            PrimMultiplicity::from(2_u32),
        )
        .expect("equal content under one identity merges");
        assert_eq!(
            Some(&(cert, PrimMultiplicity::from(3_u32))),
            entries.get(&address),
            "the gradings sum exactly as the normal form's own factorization sums them"
        );
    }
}
