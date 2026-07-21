//! The K5 re-checkable export (kernel-boundary.md §5, obligations E1–E6).
//!
//! The [`write()`] writer serializes an [`Environment`](crate::Environment) to
//! canonical, self-contained bytes; the [`read()`] validating reader decodes
//! them back through the choke point. The obligations, as this module holds
//! them:
//!
//! * **E1 — self-contained.** The artifact carries names, levels, terms, and
//!   declaration order in the bytes; replay needs no access to the producing
//!   process.
//! * **E2 — declaration-granular, admission-ordered.** The artifact is the
//!   sequence of declarations in admission order; [`read()`] replays them by
//!   re-running the choke point on each in turn —
//!   [`Environment::add_decl`](crate::Environment::add_decl) for a checked
//!   mark, [`Environment::add_decl_unchecked`](crate::Environment::add_decl_unchecked)
//!   for a bypass mark.
//! * **E3 — no derived data trusted.** The precomputed transitive audit sets
//!   (`rested_on`) are **not** in the bytes; the reader recomputes them by
//!   re-admitting each declaration, so a forged audit cannot ride along (the
//!   design decision is recorded in the crate STATUS).
//! * **E4 — canonical bytes, validating reader.** Encoding iterates in
//!   admission order and over `BTreeMap`-sorted level atoms — **never** a
//!   hash-order iteration — so an identical environment yields a byte-identical
//!   artifact. The reader is a closed-vocabulary parser with the rejection
//!   triple as a closed error vocabulary ([`DecodeError`]): truncation, an
//!   unknown tag, or a structural violation. It **decodes through
//!   constructors**: levels rebuild through the [`gandr_kernel_strata`] smart
//!   constructors so a non-canonical level is unrepresentable, and the whole
//!   artifact is held to a canonical-bytes check (re-encode and compare) so a
//!   non-canonical *encoding* is **rejected** — discharging the B2.1 obligation
//!   that the non-canonical-level guard re-arms at the decode boundary. Terms,
//!   types, and literals decode only through the kernel-core constructors.
//! * **E5 — versioned, with refusal.** A version tag rides from artifact one;
//!   an unknown version is a named refusal
//!   ([`DecodeError::UnsupportedVersion`]), never a guess.
//! * **E6 — unchecked admissions visible.** Each declaration's admission mark
//!   (checked or unchecked-bypass) is in the bytes, and `Axiom`s are visible as
//!   `Axiom` declarations, so the §3 audit survives serialization.
//!
//! # Totality
//!
//! The reader parses adversarial bytes inside the TCB and is **total**: every
//! read is bounds-checked (truncation surfaces), every tag is matched against a
//! closed set (an unknown tag surfaces), and term/type decoding is
//! **iterative** over an explicit worklist (the [`crate::conv`] precedent),
//! never input-scaled recursion — so an adversarially deep artifact never
//! overflows the stack. The writer is likewise iterative for the same reason
//! (an environment admitted through the bypass may hold a deeply nested term).
//! Level reconstruction is bounded by a decode-time offset cap
//! ([`MAX_DECODED_LEVEL_OFFSET`]).
//!
//! # The seven ratified reservations (format-plane, zero S1 typing consequence)
//!
//! * **R1 — reserved declaration kinds.** `AbstractType` / `ModuleSig` /
//!   `ModuleDef` / `FunctorDef` have concrete reserved kind tags the writer
//!   never emits; the reader rejects them as a distinct
//!   [`DecodeError::ReservedDeclarationKind`] (more honest than a generic bad
//!   tag).
//! * **R2 — structured names.** A name is a sequence of segments, never a flat
//!   dotted string. S1 declarations carry no name, so the per-declaration name
//!   record is a segment count pinned to zero at v0; a non-empty name is
//!   rejected as [`DecodeError::ReservedSlotOccupied`].
//! * **R3 — four per-`Def` annotation slots** (erasure, modes/grades,
//!   sealing-provenance, directedness/variance) are present from birth and
//!   empty at v0; a non-empty slot is rejected as unimplemented.
//! * **R4 — reserved minted-atom table**, admission-ordered and empty at v0; a
//!   non-empty table is rejected.

mod read;
mod write;

use core::error::Error;
use core::fmt;

pub use self::read::decode;
pub use self::read::read;
pub use self::write::write;
use crate::arena::TermArena;
use crate::decl::Declaration;
use crate::error::KernelError;

/// The four-byte artifact magic — Gandr Kernel eXport, v-family `1`.
pub const MAGIC: [u8; 4] = *b"GKX1";

/// The v0 format version tag (E5: the version rides from artifact one).
pub const FORMAT_VERSION_V0: u16 = 0;

/// The decode-time cap on a level variable atom's successor offset.
///
/// A canonical level `max(c, x + o, …)` is rebuilt through the strata smart
/// constructors, and the only way to raise a variable atom to offset `o` is `o`
/// applications of `succ` (strata exposes no direct offset constructor). A
/// small adversarial varint could otherwise demand unbounded reconstruction
/// work, so an atom offset at or above this cap is rejected at decode — bounded
/// work on adversarial input, the reader's totality posture. Real universe
/// levels carry variable offsets of `0` or `1`; a level beyond the cap is a
/// documented non-round-tripping case (crate STATUS), liftable when strata
/// exposes an `O(1)` offset constructor.
pub const MAX_DECODED_LEVEL_OFFSET: u64 = 4096;

/// Admission-mark byte: admitted through the checked choke point (E6).
pub const ADMISSION_CHECKED: u8 = 0;
/// Admission-mark byte: admitted through the warned bypass (E6).
pub const ADMISSION_UNCHECKED: u8 = 1;

/// Declaration-kind byte: a typed definition.
pub const KIND_DEF: u8 = 0;
/// Declaration-kind byte: a tracked typed hole.
pub const KIND_AXIOM: u8 = 1;
/// Declaration-kind byte: the R1 reserved abstract-type kind.
pub const KIND_ABSTRACT_TYPE: u8 = 2;
/// Declaration-kind byte: the R1 reserved module-signature kind.
pub const KIND_MODULE_SIG: u8 = 3;
/// Declaration-kind byte: the R1 reserved module-definition kind.
pub const KIND_MODULE_DEF: u8 = 4;
/// Declaration-kind byte: the R1 reserved functor-definition kind.
pub const KIND_FUNCTOR_DEF: u8 = 5;

/// Base-type-atom byte: the integer atom.
pub const BASE_INTEGER: u8 = 0;
/// Base-type-atom byte: the string atom.
pub const BASE_STRING: u8 = 1;
/// Base-type-atom byte: the numeric atom.
pub const BASE_NUMERIC: u8 = 2;

/// Literal-sign byte: non-negative (the canonical zero is non-negative).
pub const SIGN_NON_NEGATIVE: u8 = 0;
/// Literal-sign byte: negative.
pub const SIGN_NEGATIVE: u8 = 1;

/// Injection-side byte: the left injection.
pub const SIDE_LEFT: u8 = 0;
/// Injection-side byte: the right injection.
pub const SIDE_RIGHT: u8 = 1;

/// Literal-kind byte: an integer literal.
pub const LITERAL_INTEGER: u8 = 0;
/// Literal-kind byte: a string literal.
pub const LITERAL_TEXT: u8 = 1;
/// Literal-kind byte: a numeric literal.
pub const LITERAL_NUMERIC: u8 = 2;

/// Landmark-constraint-relation byte: `left ≤ right`.
pub const RELATION_LEQ: u8 = 0;
/// Landmark-constraint-relation byte: `left = right`.
pub const RELATION_EQ: u8 = 1;

/// Type-stream tag: the base-type former.
pub const TYPE_BASE: u8 = 0;
/// Type-stream tag: the unit type.
pub const TYPE_UNIT: u8 = 1;
/// Type-stream tag: the product former.
pub const TYPE_PRODUCT: u8 = 2;
/// Type-stream tag: the sum former.
pub const TYPE_SUM: u8 = 3;
/// Type-stream tag: the thunk former.
pub const TYPE_THUNK: u8 = 4;
/// Type-stream tag: the universe former.
pub const TYPE_UNIVERSE: u8 = 5;
/// Type-stream tag: the lift former.
pub const TYPE_LIFT: u8 = 6;
/// Type-stream tag: the returner (computation-type) former.
pub const TYPE_RETURNER: u8 = 7;
/// Type-stream tag: the arrow (computation-type) former.
pub const TYPE_ARROW: u8 = 8;

/// Term-stream tag: a bound value variable.
pub const TERM_VARIABLE: u8 = 0;
/// Term-stream tag: a constant reference.
pub const TERM_CONSTANT: u8 = 1;
/// Term-stream tag: the unit value.
pub const TERM_UNIT: u8 = 2;
/// Term-stream tag: a literal value.
pub const TERM_LITERAL: u8 = 3;
/// Term-stream tag: a pair value.
pub const TERM_PAIR: u8 = 4;
/// Term-stream tag: a sum injection value.
pub const TERM_INJECTION: u8 = 5;
/// Term-stream tag: a thunk value.
pub const TERM_THUNK: u8 = 6;
/// Term-stream tag: a value lift.
pub const TERM_LIFT: u8 = 7;
/// Term-stream tag: a lambda computation.
pub const TERM_LAMBDA: u8 = 8;
/// Term-stream tag: an application computation.
pub const TERM_APPLICATION: u8 = 9;
/// Term-stream tag: a returner computation.
pub const TERM_RETURN: u8 = 10;
/// Term-stream tag: a bind computation.
pub const TERM_BIND: u8 = 11;
/// Term-stream tag: a force computation.
pub const TERM_FORCE: u8 = 12;
/// Term-stream tag: a case computation.
pub const TERM_CASE: u8 = 13;

/// A declaration's admission mark as it rides in the artifact (E6): a single
/// checked/unchecked bit, never a trust lattice (kernel-boundary.md §3 K3).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "a single checked/unchecked bit by design (kernel-boundary.md K3)"
)]
pub enum AdmissionMark
{
    /// Admitted through the checked choke point
    /// ([`Environment::add_decl`](crate::Environment::add_decl)).
    Checked,
    /// Admitted through the warned bypass
    /// ([`Environment::add_decl_unchecked`](crate::Environment::add_decl_unchecked)).
    UncheckedBypass,
}

/// A declaration recovered from an artifact, with its admission mark.
///
/// It is the unit [`decode`] yields and [`read()`] replays through the choke
/// point.
#[derive(Clone, Debug)]
pub struct DecodedDeclaration
{
    /// How the declaration was admitted in the producing environment (E6).
    mark: AdmissionMark,
    /// The recovered declaration.
    declaration: Declaration,
}

impl DecodedDeclaration
{
    /// Pair an admission mark with a recovered declaration.
    #[inline]
    #[must_use]
    pub(crate) fn new(
        mark: AdmissionMark,
        declaration: Declaration,
    ) -> Self
    {
        Self { mark, declaration }
    }

    /// The declaration's admission mark (E6).
    #[inline]
    #[must_use]
    pub const fn mark(&self) -> AdmissionMark
    {
        self.mark
    }

    /// The recovered declaration (its content roots address the owning
    /// [`DecodedArtifact`]'s arena).
    #[inline]
    #[must_use]
    pub const fn declaration(&self) -> &Declaration
    {
        &self.declaration
    }
}

/// A fully decoded artifact: the [`TermArena`] its declarations' content was
/// decoded into, and the admission-ordered declaration sequence addressing it.
///
/// [`decode`] yields this self-contained structure (the term/type content lives
/// in [`Self::arena`], not in owned trees); [`read`] re-admits it through the
/// choke point by importing each declaration's content into a fresh
/// environment.
#[derive(Clone, Debug, Default)]
pub struct DecodedArtifact
{
    /// The arena every decoded declaration's content lives in.
    arena: TermArena,
    /// The decoded declarations, in admission order.
    declarations: alloc::vec::Vec<DecodedDeclaration>,
}

impl DecodedArtifact
{
    /// Pair a decode arena with its admission-ordered declaration sequence.
    #[inline]
    #[must_use]
    pub(crate) fn new(
        arena: TermArena,
        declarations: alloc::vec::Vec<DecodedDeclaration>,
    ) -> Self
    {
        Self {
            arena,
            declarations,
        }
    }

    /// The arena the declarations' content was decoded into.
    #[inline]
    #[must_use]
    pub const fn arena(&self) -> &TermArena
    {
        &self.arena
    }

    /// The decoded declarations, in admission order.
    #[inline]
    #[must_use]
    pub fn declarations(&self) -> &[DecodedDeclaration]
    {
        &self.declarations
    }
}

/// One of the four R1 reserved declaration kinds — a shape the writer never
/// emits and the reader rejects distinctly.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "the reserved declaration-kind set is fixed by the ratified reservations (R1)"
)]
pub enum ReservedKind
{
    /// A reserved abstract-type declaration kind.
    AbstractType,
    /// A reserved module-signature declaration kind.
    ModuleSig,
    /// A reserved module-definition declaration kind.
    ModuleDef,
    /// A reserved functor-definition declaration kind.
    FunctorDef,
}

impl fmt::Display for ReservedKind
{
    #[inline]
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        f.write_str(match *self {
            | Self::AbstractType => "AbstractType",
            | Self::ModuleSig => "ModuleSig",
            | Self::ModuleDef => "ModuleDef",
            | Self::FunctorDef => "FunctorDef",
        })
    }
}

/// A reserved slot or section that must be empty at v0 (R2/R3/R4) but was
/// found occupied.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "the reserved-slot set is fixed by the ratified reservations (R2/R3/R4)"
)]
pub enum ReservedSlot
{
    /// The R4 reserved minted-atom table was non-empty.
    MintedAtomTable,
    /// The R2 structured-name record carried a segment (S1 declarations are
    /// nameless, so v0 pins the segment count to zero).
    StructuredName,
    /// The R3 erasure annotation slot was non-empty.
    ErasureAnnotation,
    /// The R3 modes/grades annotation slot was non-empty.
    ModeGradeAnnotation,
    /// The R3 sealing-provenance annotation slot was non-empty.
    SealingProvenance,
    /// The R3 directedness/variance annotation slot was non-empty.
    DirectednessVariance,
}

impl fmt::Display for ReservedSlot
{
    #[inline]
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        f.write_str(match *self {
            | Self::MintedAtomTable => "the reserved minted-atom table",
            | Self::StructuredName => "the structured-name record",
            | Self::ErasureAnnotation => "the erasure annotation slot",
            | Self::ModeGradeAnnotation => "the modes/grades annotation slot",
            | Self::SealingProvenance => "the sealing-provenance annotation slot",
            | Self::DirectednessVariance => "the directedness/variance annotation slot",
        })
    }
}

/// Where in the closed grammar an unknown tag byte was met.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "the tagged grammar positions are a closed set by design"
)]
pub enum TagSite
{
    /// A declaration's admission mark byte.
    Admission,
    /// A declaration's kind byte (a value outside `Def`/`Axiom` and the R1
    /// reserved kinds).
    DeclarationKind,
    /// A type-stream node tag.
    Type,
    /// A term-stream node tag.
    Term,
    /// A base-type atom byte.
    BaseType,
    /// A literal sign byte.
    Sign,
    /// A literal kind byte.
    LiteralKind,
    /// An injection side byte.
    Side,
    /// A landmark-constraint relation byte.
    ConstraintRelation,
}

impl fmt::Display for TagSite
{
    #[inline]
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        f.write_str(match *self {
            | Self::Admission => "an admission mark",
            | Self::DeclarationKind => "a declaration kind",
            | Self::Type => "a type node",
            | Self::Term => "a term node",
            | Self::BaseType => "a base-type atom",
            | Self::Sign => "a literal sign",
            | Self::LiteralKind => "a literal kind",
            | Self::Side => "an injection side",
            | Self::ConstraintRelation => "a constraint relation",
        })
    }
}

/// Which structural invariant a malformed artifact violated (the third leg of
/// the rejection triple).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "the structural-violation vocabulary is closed by design (kernel-boundary.md K1)"
)]
pub enum MalformedSite
{
    /// The magic bytes did not match a gandr kernel export.
    Header,
    /// A varint was overlong (non-minimal) or exceeded the `u64` range.
    Varint,
    /// A decoded index did not fit its target width.
    IndexRange,
    /// A level variable atom's offset met or exceeded
    /// [`MAX_DECODED_LEVEL_OFFSET`], or reconstruction overflowed.
    LevelOffset,
    /// A decoded landmark-constraint side was not variable-only (the strata
    /// smart constructor rejected it).
    ConstraintForm,
    /// A literal payload was not reconstructible: a magnitude or fraction
    /// carried a non-digit byte (the smart constructor rejected it), or a
    /// string literal's bytes were not valid UTF-8.
    LiteralPayload,
    /// A decoded child node's polarity did not fit the slot its parent
    /// constructor required.
    Polarity,
    /// The bytes decoded but were not the canonical encoding of the recovered
    /// artifact (E4: a non-canonical encoding is rejected).
    NonCanonical,
    /// Bytes remained after a complete artifact was decoded.
    TrailingBytes,
}

impl fmt::Display for MalformedSite
{
    #[inline]
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        f.write_str(match *self {
            | Self::Header => "the artifact magic did not match",
            | Self::Varint => "a varint was overlong or out of range",
            | Self::IndexRange => "a decoded index did not fit its width",
            | Self::LevelOffset => "a level atom offset exceeded the decode cap",
            | Self::ConstraintForm => "a constraint side was not variable-only",
            | Self::LiteralPayload => "a literal payload was not reconstructible",
            | Self::Polarity => "a decoded node had the wrong polarity for its slot",
            | Self::NonCanonical => "the bytes were not the canonical encoding",
            | Self::TrailingBytes => "bytes remained after a complete artifact",
        })
    }
}

/// Why decoding an artifact failed.
///
/// The closed rejection vocabulary of the validating reader (kernel-boundary.md
/// §5 E4): a decode failure is a **format** failure, held apart from a typing
/// failure ([`KernelError`]) so the two never blur.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "the reader's rejection vocabulary is closed by design (kernel-boundary.md K1/E4)"
)]
pub enum DecodeError
{
    /// The artifact ended mid-field (the rejection triple's *truncated*).
    Truncated,
    /// A tag byte fell outside the closed vocabulary at a tagged position (the
    /// rejection triple's *bad tag*).
    UnknownTag
    {
        /// Where the tag was read.
        site: TagSite,
        /// The offending byte.
        tag: u8,
    },
    /// The bytes were structurally decodable but violated an invariant (the
    /// rejection triple's *malformed*).
    Malformed
    {
        /// Which invariant was violated.
        site: MalformedSite,
    },
    /// A reserved declaration kind (R1) was present.
    ReservedDeclarationKind
    {
        /// The reserved kind met.
        kind: ReservedKind,
    },
    /// A reserved slot or section (R2/R3/R4) was non-empty at v0.
    ReservedSlotOccupied
    {
        /// The occupied reserved slot.
        slot: ReservedSlot,
    },
    /// The artifact declared a format version the reader does not implement
    /// (E5: a named refusal, never a guess).
    UnsupportedVersion
    {
        /// The version the artifact declared.
        found: u16,
    },
}

impl fmt::Display for DecodeError
{
    #[inline]
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        match *self {
            | Self::Truncated => f.write_str("the export artifact ended mid-field"),
            | Self::UnknownTag { site, tag } => {
                write!(f, "unknown tag byte {tag} where {site} was expected")
            },
            | Self::Malformed { site } => write!(f, "malformed export artifact: {site}"),
            | Self::ReservedDeclarationKind { kind } => {
                write!(f, "reserved declaration kind {kind} is not admitted at v0")
            },
            | Self::ReservedSlotOccupied { slot } => {
                write!(f, "{slot} is non-empty but must be empty at v0")
            },
            | Self::UnsupportedVersion { found } => {
                write!(f, "unsupported export format version {found}")
            },
        }
    }
}

impl Error for DecodeError
{
}

/// Why reading an artifact into an [`Environment`](crate::Environment) failed.
///
/// Either the bytes did not decode ([`DecodeError`]) or a recovered declaration
/// did not re-admit through the choke point ([`KernelError`]); the two failure
/// planes stay distinct.
#[derive(Clone, Debug, Eq, PartialEq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "reading fails on exactly the decode plane or the re-admission plane"
)]
pub enum ReadError
{
    /// The bytes did not decode.
    Decode(DecodeError),
    /// A recovered declaration failed to re-admit through the choke point (E2:
    /// replay is re-running `add_decl`).
    Admit(KernelError),
}

impl fmt::Display for ReadError
{
    #[inline]
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        match *self {
            | Self::Decode(ref error) => write!(f, "export decode failed: {error}"),
            | Self::Admit(ref error) => write!(f, "export replay failed to re-admit: {error}"),
        }
    }
}

impl Error for ReadError
{
    #[inline]
    fn source(&self) -> Option<&(dyn Error + 'static)>
    {
        match *self {
            | Self::Decode(ref error) => Some(error),
            | Self::Admit(ref error) => Some(error),
        }
    }
}

impl From<DecodeError> for ReadError
{
    #[inline]
    fn from(error: DecodeError) -> Self
    {
        Self::Decode(error)
    }
}

impl From<KernelError> for ReadError
{
    #[inline]
    fn from(error: KernelError) -> Self
    {
        Self::Admit(error)
    }
}
