//! The interop boundary: resolving the compilation host's C ABI at run time.
//!
//! The host is built against a discovered MLIR installation, so it is found
//! rather than linked. That is the whole reason this is a dynamic boundary: a
//! statically linked bridge would make every Rust build in the workspace
//! depend on an installation the workspace does not pin, which is a much
//! larger claim than this crate makes.

use std::ffi::CStr;
use std::ffi::c_char;
use std::path::Path;
use std::path::PathBuf;

use crate::image::ImageBytes;

/// The boundary version this crate speaks.
///
/// The host declares the same number in its own header; a library built from
/// a different revision is refused rather than called, because a struct whose
/// layout changed has no other symptom across a dynamic boundary.
pub const ABI_VERSION: u32 = 1;

/// The environment variable naming the host library explicitly.
pub const LIBRARY_PATH_VARIABLE: &str = "GANDR_COMPILE_HOST_LIBRARY";

/// The build output the discovery falls back to, relative to a workspace root.
pub const DEFAULT_LIBRARY_DIRECTORY: &str = "runtime/compile-host/build";

/// The library's base name, without the platform's prefix or extension.
pub const LIBRARY_STEM: &str = "gandr-compile-host-abi";

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

/// A value in the canonical rendering both sides compare on.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct RenderedValue(String);

impl AsRef<str> for RenderedValue
{
    #[inline]
    fn as_ref(&self) -> &str
    {
        &self.0
    }
}

impl From<String> for RenderedValue
{
    #[inline]
    fn from(text: String) -> Self
    {
        Self(text)
    }
}

impl core::fmt::Display for RenderedValue
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
    /// No host library could be found.
    ///
    /// This is the ordinary state of a checkout with no MLIR installation, and
    /// it is a report rather than a failure of the build: nothing in the Rust
    /// workspace links the host, so its absence is discovered here and nowhere
    /// earlier.
    #[error("no compilation host library was found; looked at {looked}")]
    Unavailable
    {
        /// Where the discovery looked.
        looked: SearchReport,
    },
    /// The library was found but could not be loaded or bound.
    #[error("the compilation host library at {path} could not be bound: {detail}")]
    NotBindable
    {
        /// The library that was found.
        path: LibraryPath,
        /// What the loader said.
        detail: LoaderDetail,
    },
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

/// A path to a host library.
#[derive(Clone, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct LibraryPath(PathBuf);

impl LibraryPath
{
    /// The path.
    #[inline]
    #[must_use]
    pub fn as_path(&self) -> &Path
    {
        &self.0
    }
}

impl From<PathBuf> for LibraryPath
{
    #[inline]
    fn from(path: PathBuf) -> Self
    {
        Self(path)
    }
}

impl core::fmt::Display for LibraryPath
{
    #[inline]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        write!(f, "{}", self.0.display())
    }
}

/// What the dynamic loader said about a library it would not bind.
#[derive(Clone, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct LoaderDetail(String);

impl core::fmt::Display for LoaderDetail
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

/// The places the discovery looked for a host library.
#[derive(Clone, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct SearchReport(Vec<PathBuf>);

impl core::fmt::Display for SearchReport
{
    #[inline]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        let mut first = true;
        for path in &self.0 {
            if !first {
                f.write_str(", ")?;
            }
            first = false;
            write!(f, "{}", path.display())?;
        }
        if first {
            f.write_str("nowhere")?;
        }
        Ok(())
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

/// The boundary entry that runs an image on a host-sized heap.
type RunEntry = unsafe extern "C" fn(*const u8, usize, *mut RawOutcome) -> i32;

/// The boundary entry that runs an image on a caller-sized heap.
type RunWithHeapEntry = unsafe extern "C" fn(*const u8, usize, u64, *mut RawOutcome) -> i32;

/// The boundary entry that reports the library's version.
type VersionEntry = unsafe extern "C" fn() -> u32;

/// The boundary entry that releases what an outcome owns.
type ReleaseEntry = unsafe extern "C" fn(*mut RawOutcome);

/// A boundary symbol's name, NUL-terminated as the loader wants it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(transparent)]
struct SymbolName(&'static [u8]);

impl core::fmt::Display for SymbolName
{
    /// The name without its terminator.
    ///
    /// The loader reports a failed lookup as "dlsym failed" and says nothing
    /// about what it was looking for, so the name is carried into the message
    /// here — a caller debugging a partial or mismatched library needs the
    /// symbol far more than the verb.
    #[inline]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        let spelled = self
            .0
            .split_last()
            .map_or(self.0, |(_terminator, head)| head);
        match core::str::from_utf8(spelled) {
            | Ok(name) => f.write_str(name),
            | Err(_not_utf8) => f.write_str("a symbol whose name is not UTF-8"),
        }
    }
}

/// The version entry's symbol.
const VERSION_SYMBOL: SymbolName = SymbolName(b"gandr_compile_host_abi_version\0");

/// The ordinary run entry's symbol.
const RUN_SYMBOL: SymbolName = SymbolName(b"gandr_compile_host_run\0");

/// The sized-heap run entry's symbol.
const RUN_WITH_HEAP_SYMBOL: SymbolName = SymbolName(b"gandr_compile_host_run_with_heap\0");

/// The reference-interpreter entry's symbol.
const INTERPRET_SYMBOL: SymbolName = SymbolName(b"gandr_compile_host_interpret\0");

/// The release entry's symbol.
const RELEASE_SYMBOL: SymbolName = SymbolName(b"gandr_compile_host_outcome_release\0");

/// A bound compilation host.
///
/// Holding the library open for the caller's lifetime is deliberate: each run
/// builds its own MLIR context, so the cost of loading is paid once while no
/// state is shared between runs.
#[derive(Debug)]
pub struct CompileHost
{
    /// The loaded library, kept alive for as long as the entries are used.
    library: libloading::Library,
    /// Where it was loaded from.
    path: LibraryPath,
}

impl CompileHost
{
    /// Finds and binds the host library.
    ///
    /// The search is the mirror of the host's own MLIR discovery: an explicit
    /// environment variable wins, then the conventional build output under the
    /// workspace root.
    ///
    /// # Contract
    /// - requires: nothing; an absent host is an ordinary outcome.
    /// - ensures: a returned host has been checked against this crate's
    ///   boundary version.
    /// - provides: the entry every caller uses, so the search order is stated
    ///   in one place.
    /// - fails: [`HostError::Unavailable`] when nothing was found,
    ///   [`HostError::NotBindable`] when a found library would not load, and
    ///   [`HostError::VersionMismatch`] when it speaks another version.
    /// - panics: none.
    ///
    /// # Errors
    /// The variants above.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — the search order and the version check are separated
    ///   by pointwise cases: a variable naming a missing file, a variable
    ///   naming the real library, and no variable at all.
    /// - witness: `bridge::an_absent_host_is_reported_rather_than_fatal`
    /// - witness: `bridge::the_bridge_agrees_with_the_l_machine_on_every_named_program`
    #[inline]
    pub fn discover() -> Result<Self, HostError>
    {
        let mut looked: Vec<PathBuf> = Vec::new();
        if let Some(named) = std::env::var_os(LIBRARY_PATH_VARIABLE) {
            let path = PathBuf::from(named);
            looked.push(path.clone());
            if path.is_file() {
                return Self::open(&path);
            }
        }
        for candidate in default_candidates() {
            looked.push(candidate.clone());
            if candidate.is_file() {
                return Self::open(&candidate);
            }
        }
        Err(HostError::Unavailable {
            looked: SearchReport(looked),
        })
    }

    /// Binds a host library at a known path.
    ///
    /// # Contract
    /// - requires: `path` names a shared library built from this repository's
    ///   compilation host.
    /// - ensures: the library's declared boundary version equals
    ///   [`ABI_VERSION`], or the call fails rather than proceeding.
    /// - provides: the explicit-path entry, for a caller that already located
    ///   the build.
    /// - fails: [`HostError::NotBindable`], [`HostError::VersionMismatch`].
    /// - panics: none.
    ///
    /// # Errors
    /// The variants above.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — the two failure modes are separated by a path that is
    ///   not a library and a library whose version entry disagrees; the success
    ///   case is carried by the differential.
    /// - witness: `bridge::an_absent_host_is_reported_rather_than_fatal`
    #[inline]
    pub fn open(path: &Path) -> Result<Self, HostError>
    {
        // The loader takes the name as text, so a path this platform admits
        // but Unicode does not is refused here rather than lossily converted.
        let Some(name) = path.to_str()
        else {
            return Err(HostError::NotBindable {
                path: LibraryPath(path.to_path_buf()),
                detail: LoaderDetail(String::from("the library path is not valid UTF-8")),
            });
        };

        // SAFETY: loading a shared library runs its initializers, which is why
        // the call is unsafe. The library named here is this repository's own
        // compilation host, built by `mise run compile-host:build`; the caller
        // supplies the path and the version check below refuses anything that
        // does not declare this boundary.
        let library = unsafe { libloading::Library::new(name) };
        let library = match library {
            | Ok(library) => library,
            | Err(error) => {
                return Err(HostError::NotBindable {
                    path: LibraryPath(path.to_path_buf()),
                    detail: LoaderDetail(error.to_string()),
                });
            },
        };

        let host = Self {
            library,
            path: LibraryPath(path.to_path_buf()),
        };
        let found = host.declared_version()?;
        if found != AbiVersion(ABI_VERSION) {
            return Err(HostError::VersionMismatch {
                found,
                expected: AbiVersion(ABI_VERSION),
            });
        }
        Ok(host)
    }

    /// Where this host was loaded from.
    #[inline]
    #[must_use]
    pub fn path(&self) -> &LibraryPath
    {
        &self.path
    }

    /// The boundary version the library declares.
    fn declared_version(&self) -> Result<AbiVersion, HostError>
    {
        // SAFETY: the signature named here is the one `abi.h` declares for
        // this symbol, which is what `entry` requires of its caller.
        let entry = unsafe { self.entry::<VersionEntry>(VERSION_SYMBOL) }?;
        // SAFETY: the entry was resolved from the library bound above and
        // returns a plain integer, so nothing outlives the call.
        let version = unsafe { entry() };
        Ok(AbiVersion(version))
    }

    /// Compiles and runs an encoded image, letting the host size the heap.
    ///
    /// # Contract
    /// - requires: `image` was produced by this crate's encoder.
    /// - ensures: the answer carries the run's value, both ledger counters and
    ///   its arena consumption.
    /// - provides: the ordinary run entry.
    /// - fails: [`HostError::Refused`] naming the stage that refused;
    ///   [`HostError::NotBindable`] if a symbol will not resolve.
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
        // The release entry is resolved **before** the run, and that order is
        // the point: a run allocates the outcome's text, so discovering a
        // missing release afterwards would leak on the way to reporting a
        // library this crate cannot bind.
        let release = self.release_entry()?;
        let mut outcome = RawOutcome::default();
        // SAFETY: the signature named here is the one `abi.h` declares for
        // this symbol.
        let entry = unsafe { self.entry::<RunEntry>(RUN_SYMBOL) }?;
        // SAFETY: `bytes` is a live slice for the duration of the call and its
        // length is passed beside it; `outcome` is a live local of the layout
        // `abi.h` declares. The host copies out of the slice and does not
        // retain it, and the text it writes into `outcome` is released below.
        let status = unsafe { entry(bytes.as_ptr(), bytes.len(), &raw mut outcome) };
        Self::finish(BoundaryStatus::from(status), &mut outcome, &release)
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
        // Resolved before the run, as in `run`.
        let release = self.release_entry()?;
        let mut outcome = RawOutcome::default();
        // SAFETY: the signature named here is the one `abi.h` declares for
        // this symbol.
        let entry = unsafe { self.entry::<RunWithHeapEntry>(RUN_WITH_HEAP_SYMBOL) }?;
        // SAFETY: as `run`, with the heap size passed by value.
        let status = unsafe {
            entry(
                bytes.as_ptr(),
                bytes.len(),
                u64::from(heap),
                &raw mut outcome,
            )
        };
        Self::finish(BoundaryStatus::from(status), &mut outcome, &release)
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
        // Resolved before the run, as in `run`.
        let release = self.release_entry()?;
        let mut outcome = RawOutcome::default();
        // SAFETY: the signature named here is the one `abi.h` declares for
        // this symbol.
        let entry = unsafe { self.entry::<RunEntry>(INTERPRET_SYMBOL) }?;
        // SAFETY: as `run`.
        let status = unsafe { entry(bytes.as_ptr(), bytes.len(), &raw mut outcome) };
        Self::finish(BoundaryStatus::from(status), &mut outcome, &release)
    }

    /// Resolves one boundary entry by name.
    ///
    /// # Safety
    /// The caller states that `Entry` is the signature `abi.h` declares for
    /// `name`. Nothing here can check that, which is why the entry types are
    /// declared once in this module beside the header they mirror.
    unsafe fn entry<Entry>(
        &self,
        name: SymbolName,
    ) -> Result<libloading::Symbol<'_, Entry>, HostError>
    {
        // SAFETY: forwarded from this function's own safety contract; the
        // symbol is looked up in the library this host bound.
        let resolved = unsafe { self.library.get::<Entry>(name.0) };
        match resolved {
            | Ok(symbol) => Ok(symbol),
            | Err(error) => Err(HostError::NotBindable {
                path: self.path.clone(),
                detail: LoaderDetail(alloc::format!("resolving {name}: {error}")),
            }),
        }
    }

    /// Resolves the release entry.
    ///
    /// Every caller does this **before** invoking a run, which is what makes
    /// the release on the way out infallible: [`CompileHost::finish`] takes
    /// the resolved entry rather than looking one up, so there is no path on
    /// which an allocated outcome meets a failing lookup.
    ///
    /// # Contract
    /// - ensures: a returned symbol is the release entry `abi.h` declares.
    /// - provides: the resolution `finish` requires by type.
    /// - fails: [`HostError::NotBindable`] when the library exports no such
    ///   symbol, before anything has been allocated.
    /// - panics: none.
    ///
    /// # Errors
    /// [`HostError::NotBindable`].
    fn release_entry(&self) -> Result<libloading::Symbol<'_, ReleaseEntry>, HostError>
    {
        // SAFETY: the signature named here is the one `abi.h` declares for
        // this symbol.
        unsafe { self.entry::<ReleaseEntry>(RELEASE_SYMBOL) }
    }

    /// Reads a filled outcome, releases what it owns, and reports the answer.
    ///
    /// Taking the release entry as an argument rather than resolving one is
    /// the whole discipline: an outcome reaches this function only after its
    /// caller has already proved the release exists, so the type makes the
    /// leaking order unreachable rather than merely unused.
    fn finish(
        status: BoundaryStatus,
        outcome: &mut RawOutcome,
        release: &libloading::Symbol<'_, ReleaseEntry>,
    ) -> Result<HostAnswer, HostError>
    {
        let text = read_text(outcome);
        // SAFETY: `outcome` was filled by one of the entries above, which is
        // exactly the precondition `gandr_compile_host_outcome_release`
        // states; it clears the pointer, so a second release is inert.
        unsafe { release(&raw mut *outcome) };

        if status != STATUS_OK {
            return Err(HostError::Refused {
                stage: RefusalStage::from_status(status),
                detail: RefusalDetail(text),
            });
        }
        Ok(HostAnswer {
            value: RenderedValue(text),
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

/// The conventional build outputs the discovery falls back to.
///
/// The workspace root is derived from this crate's manifest directory, which
/// Cargo supplies at compile time, so the fallback does not depend on the
/// current working directory.
fn default_candidates() -> Vec<PathBuf>
{
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let Some(crates_directory) = manifest.parent()
    else {
        return Vec::new();
    };
    let Some(workspace) = crates_directory.parent()
    else {
        return Vec::new();
    };
    let directory = workspace.join(DEFAULT_LIBRARY_DIRECTORY);
    let mut candidates = Vec::new();
    for name in library_file_names() {
        candidates.push(directory.join(name));
    }
    candidates
}

/// The platform's spellings of the host library's file name.
fn library_file_names() -> Vec<String>
{
    let mut names = Vec::new();
    if cfg!(target_os = "macos") {
        names.push(alloc::format!("lib{LIBRARY_STEM}.dylib"));
    }
    if cfg!(target_os = "windows") {
        names.push(alloc::format!("{LIBRARY_STEM}.dll"));
    }
    names.push(alloc::format!("lib{LIBRARY_STEM}.so"));
    names
}
