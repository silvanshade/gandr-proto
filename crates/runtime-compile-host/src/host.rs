//! The interop boundary: the compilation host's C ABI, bound by the linker.
//!
//! MLIR is pinned and `compile-host:wall` requires it on every checkout, so
//! the host is linked rather than looked up. The linker resolves every entry
//! this module declares, which means a boundary that drifts is a build
//! failure at the point of the drift instead of a refusal at the point of the
//! call. Nothing here searches, opens, or resolves a symbol by name.
//!
//! The link itself sits behind the `full` feature, and `build.rs` is where
//! that is decided. Without it the crate acquires no MLIR and no C++
//! toolchain, and this module's host-bearing half does not exist.
//!
//! **The layout authority is elsewhere and stays there.** The linker proves
//! that every symbol is present and bound; it cannot see that a struct field
//! moved. `tests/contract.rs` holds this crate's mirror of the boundary to the
//! host's own headers, and neither check substitutes for the other.

#![allow(
    unknown_lints,
    reason = "The local dylint library supplies primitive_signature, and the stable compiler does not know the name."
)]
#![allow(
    primitive_signature,
    reason = "The declarations below mirror a C header; their primitives ARE the boundary's types, and a wrapper here would describe a different ABI."
)]

use core::ffi::CStr;
use core::ffi::c_char;

use crate::ABI_VERSION;
use crate::image::ImageBytes;
use crate::render::RenderedValue;

/// What one run produced, as the boundary reports it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HostAnswer
{
    /// The produced value, in the canonical rendering.
    pub value: RenderedValue,
    /// The duplications the run executed.
    pub duplications: LedgerCount,
    /// The discards the run executed.
    pub discards: LedgerCount,
    /// The arena words the run consumed.
    pub allocated: ArenaWords,
}

/// One of the run's accounted-work counters.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct LedgerCount(i64);

impl From<i64> for LedgerCount
{
    #[inline]
    fn from(count: i64) -> Self
    {
        Self(count)
    }
}

impl From<LedgerCount> for i64
{
    #[inline]
    fn from(count: LedgerCount) -> Self
    {
        count.0
    }
}

/// How many arena words a run consumed.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct ArenaWords(u64);

impl From<u64> for ArenaWords
{
    #[inline]
    fn from(words: u64) -> Self
    {
        Self(words)
    }
}

impl From<ArenaWords> for u64
{
    #[inline]
    fn from(words: ArenaWords) -> Self
    {
        words.0
    }
}

/// How many machine words a caller offers the run as its heap.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct HeapWords(u64);

impl From<u64> for HeapWords
{
    #[inline]
    fn from(words: u64) -> Self
    {
        Self(words)
    }
}

impl From<HeapWords> for u64
{
    #[inline]
    fn from(words: HeapWords) -> Self
    {
        words.0
    }
}

/// Which stage of the host refused a run.
///
/// The numbering is the boundary's rather than the host's C++ enumeration, so
/// a caller matches on a stage without parsing a message.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RefusalStage
{
    /// The byte image did not decode.
    MalformedImage,
    /// The emitted module failed the verifier wall.
    VerifierRejected,
    /// A lowering stage could not translate an operation.
    LoweringFailed,
    /// The conversion to the LLVM dialect failed.
    ConversionFailed,
    /// The execution engine could not be built or resolved.
    ExecutionFailed,
    /// The run produced a heap the renderer could not read.
    ResultUnreadable,
    /// A resource limit was reached: depth, heap size, or image size.
    LimitExceeded,
    /// A file the host was asked to read could not be read.
    FixtureUnreadable,
    /// The boundary itself refused the call.
    BadCall,
    /// The boundary reported a status this crate does not know.
    Unknown,
}

/// A status as the boundary reports it.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct BoundaryStatus(i32);

impl From<i32> for BoundaryStatus
{
    #[inline]
    fn from(status: i32) -> Self
    {
        Self(status)
    }
}

/// The status a successful run reports.
const STATUS_OK: BoundaryStatus = BoundaryStatus(0);

impl RefusalStage
{
    /// The stage a boundary status names.
    ///
    /// # Contract
    /// - ensures: total over every status value, with an unknown status
    ///   reported as such rather than mapped onto a neighbour.
    /// - panics: none.
    #[inline]
    #[must_use]
    fn from_status(status: BoundaryStatus) -> Self
    {
        match status.0 {
            | 1 => Self::MalformedImage,
            | 2 => Self::VerifierRejected,
            | 3 => Self::LoweringFailed,
            | 4 => Self::ConversionFailed,
            | 5 => Self::ExecutionFailed,
            | 6 => Self::ResultUnreadable,
            | 7 => Self::LimitExceeded,
            | 8 => Self::FixtureUnreadable,
            | 100 => Self::BadCall,
            | _ => Self::Unknown,
        }
    }
}

impl core::fmt::Display for RefusalStage
{
    #[inline]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        f.write_str(match *self {
            | Self::MalformedImage => "the image decoder",
            | Self::VerifierRejected => "the verifier wall",
            | Self::LoweringFailed => "the lowering",
            | Self::ConversionFailed => "the conversion to LLVM",
            | Self::ExecutionFailed => "the execution engine",
            | Self::ResultUnreadable => "the value renderer",
            | Self::LimitExceeded => "a resource limit",
            | Self::FixtureUnreadable => "a fixture read",
            | Self::BadCall => "the boundary itself",
            | Self::Unknown => "a stage this crate does not know",
        })
    }
}

/// What can go wrong at the boundary.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum HostError
{
    /// The library speaks a different boundary version.
    #[error(
        "the compilation host library declares ABI version {found}, and this crate speaks {expected}"
    )]
    VersionMismatch
    {
        /// The version the library declared.
        found: AbiVersion,
        /// The version this crate speaks.
        expected: AbiVersion,
    },
    /// The host ran and refused.
    #[error("{stage} refused the program: {detail}")]
    Refused
    {
        /// Which stage refused.
        stage: RefusalStage,
        /// What that stage said.
        detail: RefusalDetail,
    },
}

/// A version of the boundary.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct AbiVersion(u32);

impl From<u32> for AbiVersion
{
    #[inline]
    fn from(version: u32) -> Self
    {
        Self(version)
    }
}

impl core::fmt::Display for AbiVersion
{
    #[inline]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        write!(f, "{}", self.0)
    }
}

/// What a refusing stage said.
#[derive(Clone, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct RefusalDetail(String);

impl AsRef<str> for RefusalDetail
{
    #[inline]
    fn as_ref(&self) -> &str
    {
        &self.0
    }
}

impl core::fmt::Display for RefusalDetail
{
    #[inline]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        f.write_str(&self.0)
    }
}

/// The boundary's outcome record, laid out as the host's header declares it.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct RawOutcome
{
    /// Zero on success, else the refusing stage's status.
    status: i32,
    /// The duplications the run executed.
    duplications: i64,
    /// The discards the run executed.
    discards: i64,
    /// The arena words the run consumed.
    allocated_words: u64,
    /// The rendered value, or the failure detail.
    text: *const c_char,
}

impl Default for RawOutcome
{
    #[inline]
    fn default() -> Self
    {
        Self {
            status: 0,
            duplications: 0,
            discards: 0,
            allocated_words: 0,
            text: core::ptr::null(),
        }
    }
}

// The host's C boundary, declared once and resolved by the linker.
//
// Each signature is the one `include/gandr/compile_host/abi.h` declares. A name
// or a signature that drifts from that header is a link error naming the
// symbol, which is the property the `full` feature exists to buy.
//
// Every entry is `extern "C"` over plain old data. A pointer a caller passes is
// read for the duration of the call and never retained, and the text an outcome
// carries is owned by the host until `gandr_compile_host_outcome_release`
// clears it; each call site below carries the obligation that applies to it.
unsafe extern "C" {
    /// The boundary version the linked host implements.
    fn gandr_compile_host_abi_version() -> u32;

    /// Compiles and runs an image, sizing the heap from the image.
    fn gandr_compile_host_run(
        bytes: *const u8,
        length: usize,
        outcome: *mut RawOutcome,
    ) -> i32;

    /// Compiles and runs an image on a heap of the caller's size.
    fn gandr_compile_host_run_with_heap(
        bytes: *const u8,
        length: usize,
        heap_words: u64,
        outcome: *mut RawOutcome,
    ) -> i32;

    /// Runs an image on the host's reference interpreter.
    fn gandr_compile_host_interpret(
        bytes: *const u8,
        length: usize,
        outcome: *mut RawOutcome,
    ) -> i32;

    /// Releases what a filled outcome owns.
    fn gandr_compile_host_outcome_release(outcome: *mut RawOutcome);
}

/// The linked compilation host.
///
/// A zero-sized handle rather than an open library: the entries are bound by
/// the linker, so there is nothing to keep alive and nothing to close. Holding
/// one is a statement that the boundary version was checked.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct CompileHost;

impl CompileHost
{
    /// Binds the linked host after checking its boundary version.
    ///
    /// There is no search and no fallback. The linker has already resolved
    /// every entry, so the only thing left to establish is that the host
    /// agrees with this crate about what the fields mean.
    ///
    /// # Contract
    /// - requires: nothing.
    /// - ensures: a returned host declares [`ABI_VERSION`].
    /// - provides: the one entry every caller uses.
    /// - fails: [`HostError::VersionMismatch`] when the linked host speaks
    ///   another version.
    /// - panics: none.
    ///
    /// # Errors
    /// The variant above.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — the version check is the only decision here, and it
    ///   is exercised by every bridge case that binds before running.
    /// - witness: `bridge::the_bridge_agrees_with_the_l_machine_on_every_named_program`
    #[inline]
    pub fn bind() -> Result<Self, HostError>
    {
        // SAFETY: the linked entry takes no arguments and returns a plain
        // integer, so nothing outlives the call.
        let found = AbiVersion(unsafe { gandr_compile_host_abi_version() });
        if found != AbiVersion(ABI_VERSION) {
            return Err(HostError::VersionMismatch {
                found,
                expected: AbiVersion(ABI_VERSION),
            });
        }
        Ok(Self)
    }

    /// Compiles and runs an encoded image, letting the host size the heap.
    ///
    /// # Contract
    /// - requires: `image` was produced by this crate's encoder.
    /// - ensures: the answer carries the run's value, both ledger counters and
    ///   its arena consumption.
    /// - provides: the ordinary run entry.
    /// - fails: [`HostError::Refused`] naming the stage that refused;
    /// - panics: none.
    ///
    /// # Errors
    /// The variants above.
    ///
    /// # Adequacy
    /// - hypothesis: L2 — the answer is compared against the L machine's on the
    ///   same computation, which is an oracle in another crate reached by
    ///   another path entirely.
    /// - witness: `bridge::the_bridge_agrees_with_the_l_machine_on_every_named_program`
    #[inline]
    pub fn run(
        &self,
        image: &ImageBytes,
    ) -> Result<HostAnswer, HostError>
    {
        let bytes: &[u8] = image.as_ref();
        let mut outcome = RawOutcome::default();
        // SAFETY: `bytes` is a live slice for the duration of the call and its
        // length is passed beside it; `outcome` is a live local of the layout
        // `abi.h` declares. The host copies out of the slice and does not
        // retain it, and the text it writes into `outcome` is released below.
        let status =
            unsafe { gandr_compile_host_run(bytes.as_ptr(), bytes.len(), &raw mut outcome) };
        Self::finish(BoundaryStatus::from(status), &mut outcome)
    }

    /// Compiles and runs an encoded image on a heap of the caller's size.
    ///
    /// # Contract
    /// - requires: `image` was produced by this crate's encoder.
    /// - ensures: a heap too small for the run yields
    ///   [`RefusalStage::LimitExceeded`] rather than an answer.
    /// - provides: the entry that reaches the compiled bounds check from the
    ///   Rust side.
    /// - fails: as [`CompileHost::run`].
    /// - panics: none.
    ///
    /// # Errors
    /// As [`CompileHost::run`].
    ///
    /// # Adequacy
    /// - hypothesis: L3 — the boundary case is the exact word: the run's own
    ///   measured consumption, and one word less than it.
    /// - witness: `bridge::the_bridge_sees_the_compiled_bounds_check`
    #[inline]
    pub fn run_with_heap(
        &self,
        image: &ImageBytes,
        heap: HeapWords,
    ) -> Result<HostAnswer, HostError>
    {
        let bytes: &[u8] = image.as_ref();
        let mut outcome = RawOutcome::default();
        // SAFETY: as `run`, with the heap size passed by value.
        let status = unsafe {
            gandr_compile_host_run_with_heap(
                bytes.as_ptr(),
                bytes.len(),
                u64::from(heap),
                &raw mut outcome,
            )
        };
        Self::finish(BoundaryStatus::from(status), &mut outcome)
    }

    /// Runs an encoded image on the host's reference interpreter.
    ///
    /// # Contract
    /// - requires: `image` was produced by this crate's encoder.
    /// - ensures: the reference walk's answer, in the same shape.
    /// - provides: the differential's other side without a second MLIR build,
    ///   so a caller can compare the host's two paths from here.
    /// - fails: as [`CompileHost::run`].
    /// - panics: none.
    ///
    /// # Errors
    /// As [`CompileHost::run`].
    ///
    /// # Adequacy
    /// - hypothesis: L2 — the reference answer is compared against the compiled
    ///   one for every named program.
    /// - witness: `bridge::the_two_host_paths_agree_through_the_bridge`
    #[inline]
    pub fn interpret(
        &self,
        image: &ImageBytes,
    ) -> Result<HostAnswer, HostError>
    {
        let bytes: &[u8] = image.as_ref();
        let mut outcome = RawOutcome::default();
        // SAFETY: as `run`.
        let status =
            unsafe { gandr_compile_host_interpret(bytes.as_ptr(), bytes.len(), &raw mut outcome) };
        Self::finish(BoundaryStatus::from(status), &mut outcome)
    }

    /// Reads a filled outcome, releases what it owns, and reports the answer.
    ///
    /// The release is a linked call rather than a resolution, so the order
    /// that used to matter cannot go wrong: there is no lookup that could fail
    /// after a run has already allocated the outcome's text.
    fn finish(
        status: BoundaryStatus,
        outcome: &mut RawOutcome,
    ) -> Result<HostAnswer, HostError>
    {
        let text = read_text(outcome);
        // SAFETY: `outcome` was filled by one of the entries above, which is
        // exactly the precondition `gandr_compile_host_outcome_release`
        // states; it clears the pointer, so a second release is inert.
        unsafe { gandr_compile_host_outcome_release(&raw mut *outcome) };

        if status != STATUS_OK {
            return Err(HostError::Refused {
                stage: RefusalStage::from_status(status),
                detail: RefusalDetail(text),
            });
        }
        Ok(HostAnswer {
            value: RenderedValue::from(text),
            duplications: LedgerCount(outcome.duplications),
            discards: LedgerCount(outcome.discards),
            allocated: ArenaWords(outcome.allocated_words),
        })
    }
}

/// Copies a NUL-terminated boundary string into an owned one.
///
/// A null pointer or invalid UTF-8 becomes an empty string rather than a
/// failure: the text is a message beside a status, and losing the message must
/// not lose the status.
fn read_text(outcome: &RawOutcome) -> String
{
    if outcome.text.is_null() {
        return String::new();
    }
    // SAFETY: the boundary promises a NUL-terminated string that stays valid
    // until released, and the release happens after this read in `finish`.
    let borrowed = unsafe { CStr::from_ptr(outcome.text) };
    borrowed.to_str().map(str::to_owned).unwrap_or_default()
}

#[cfg(test)]
mod tests
{
    use super::*;

    /// Every boundary status names its own stage.
    ///
    /// An error vocabulary nobody can read is one nobody can act on, so each
    /// arm is asserted on its exact variant rather than on "some refusal".
    #[test]
    fn every_boundary_status_names_its_own_stage()
    {
        let mapping = [
            (1_i32, RefusalStage::MalformedImage),
            (2_i32, RefusalStage::VerifierRejected),
            (3_i32, RefusalStage::LoweringFailed),
            (4_i32, RefusalStage::ConversionFailed),
            (5_i32, RefusalStage::ExecutionFailed),
            (6_i32, RefusalStage::ResultUnreadable),
            (7_i32, RefusalStage::LimitExceeded),
            (8_i32, RefusalStage::FixtureUnreadable),
            (100_i32, RefusalStage::BadCall),
        ];
        for (status, expected) in mapping {
            assert_eq!(
                RefusalStage::from_status(BoundaryStatus::from(status)),
                expected
            );
            assert!(!expected.to_string().is_empty(), "every stage renders");
        }

        // A status this crate does not know is reported as unknown rather
        // than mapped onto a neighbour, which is the failure a numbering
        // change would otherwise hide.
        for unknown in [9_i32, 42_i32, -1_i32, 101_i32] {
            assert_eq!(
                RefusalStage::from_status(BoundaryStatus::from(unknown)),
                RefusalStage::Unknown
            );
        }
        assert_eq!(
            RefusalStage::Unknown.to_string(),
            "a stage this crate does not know"
        );
    }

    /// Every failure renders a message that names what went wrong.
    #[test]
    fn every_host_failure_renders_its_own_message()
    {
        let mismatched = HostError::VersionMismatch {
            found: AbiVersion::from(7),
            expected: AbiVersion::from(ABI_VERSION),
        };
        let rendered = mismatched.to_string();
        assert!(rendered.contains('7'), "{rendered}");

        let refused = HostError::Refused {
            stage: RefusalStage::LimitExceeded,
            detail: RefusalDetail(String::from("would not fit")),
        };
        let rendered = refused.to_string();
        assert!(rendered.contains("a resource limit"), "{rendered}");
        assert!(rendered.contains("would not fit"), "{rendered}");
        let detail = RefusalDetail(String::from("would not fit"));
        let borrowed: &str = detail.as_ref();
        assert_eq!(borrowed, "would not fit");
    }

    /// The boundary's values round-trip through their wrappers.
    #[test]
    fn the_boundaries_values_round_trip_through_their_wrappers()
    {
        assert_eq!(i64::from(LedgerCount::from(3_i64)), 3_i64);
        assert_eq!(u64::from(ArenaWords::from(11_u64)), 11_u64);
        assert_eq!(u64::from(HeapWords::from(64_u64)), 64_u64);
        assert_eq!(AbiVersion::from(1_u32), AbiVersion(1));

        let rendered = RenderedValue::from(String::from("(int 5)"));
        assert_eq!(rendered.to_string(), "(int 5)");
        let borrowed: &str = rendered.as_ref();
        assert_eq!(borrowed, "(int 5)");
    }

    /// A message the boundary did not write is read as empty rather than as a
    /// dereference of nothing.
    #[test]
    fn a_null_boundary_message_reads_as_empty()
    {
        let outcome = RawOutcome::default();
        assert!(outcome.text.is_null());
        assert_eq!(read_text(&outcome), String::new());

        // A message the boundary did write is read whole, terminator and all.
        let owned = alloc::ffi::CString::new("a detail").expect("no interior NUL");
        let outcome = RawOutcome {
            text: owned.as_ptr(),
            ..RawOutcome::default()
        };
        assert_eq!(read_text(&outcome), String::from("a detail"));
    }
}
