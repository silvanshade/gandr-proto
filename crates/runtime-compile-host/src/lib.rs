//! The Rust side of the compilation host boundary.
//!
//! A core computation is checked, lowered into a program image, encoded, and
//! handed to the compilation host through a small C boundary; the run's value
//! and its accounted work come back. That path is the whole crate.
//!
//! The host is **found at run time, never linked**. It is built against a
//! discovered MLIR installation, so a checkout without one still builds and
//! tests this crate: [`CompileHost::discover`] reports an absent host as an
//! ordinary outcome, and the parts that do not need it — the lowering, the
//! encoder, the renderer — are exercised unconditionally.
//!
//! ```no_run
//! use gandr_core_term::syntax::Comp;
//! use gandr_core_term::syntax::Value;
//! use gandr_runtime_compile_host::CompileHost;
//! use gandr_runtime_compile_host::compile_and_run;
//!
//! let host = CompileHost::discover()?;
//! let answer = compile_and_run(&host, &Comp::ret(Value::Int(5)))?;
//! assert_eq!(answer.value.to_string(), "(int 5)");
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

extern crate alloc;

pub mod host;
pub mod image;
pub mod lower;
pub mod render;

use gandr_core_checker::judgements::control::Dir;
use gandr_core_machine::run_comp;
use gandr_core_term::ctx::Ctx;
use gandr_core_term::syntax::Comp;
use gandr_core_term::types::CompType;

pub use crate::host::CompileHost;
pub use crate::host::HostAnswer;
pub use crate::host::HostError;
pub use crate::image::Image;
pub use crate::lower::LowerError;
pub use crate::lower::lower_computation;
pub use crate::render::RenderError;

/// What can go wrong on the way from a core computation to a host answer.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum BridgeError
{
    /// The core checker refused the computation.
    ///
    /// The bridge checks before it lowers, so what crosses the boundary is a
    /// computation the checker accepted rather than one that merely parsed.
    #[error("the core checker refused the computation: {detail}")]
    NotChecked
    {
        /// What the checker said.
        detail: CheckerDetail,
    },
    /// The computation could not be lowered into an image.
    #[error("the computation could not be lowered: {source}")]
    NotLowered
    {
        /// Why the lowering refused.
        #[from]
        source: LowerError,
    },
    /// The host refused, or could not be reached.
    #[error("the compilation host did not answer: {source}")]
    NotRun
    {
        /// What the boundary reported.
        #[from]
        source: HostError,
    },
}

/// What the core checker said about a computation it refused.
#[derive(Clone, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct CheckerDetail(String);

impl core::fmt::Display for CheckerDetail
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

/// Checks a core computation, lowers it, and runs it on the compilation host.
///
/// The three stages are one entry because their order is a property of the
/// bridge rather than a convention a caller keeps: a computation the checker
/// refused has no business becoming an image, and an image the lowering
/// refused has no business crossing the boundary.
///
/// # Contract
/// - requires: `host` is bound; `comp` is a closed core computation.
/// - ensures: on success the answer is the compiled run's value and accounted
///   work, for a computation the core checker accepted.
/// - provides: the driving path from the Rust core into native execution.
/// - fails: [`BridgeError::NotChecked`] when the checker refuses,
///   [`BridgeError::NotLowered`] outside the compiled slice, and
///   [`BridgeError::NotRun`] when the host refuses.
/// - panics: none; the checking runs on the typing machine, whose frame stack
///   is on the heap, and the lowering is an explicit stack for the same reason.
///
/// # Errors
/// The variants above, in stage order.
///
/// # Adequacy
/// - hypothesis: L2 with an L3 residue — the success path is held to the L
///   machine's own answer for the same computation, and each refusal is
///   triggered pointwise by a computation that reaches exactly that stage.
/// - witness: `bridge::the_bridge_agrees_with_the_l_machine_on_every_named_program`
/// - witness: `bridge::a_computation_outside_the_slice_is_refused_before_the_boundary`
#[inline]
pub fn compile_and_run(
    host: &CompileHost,
    comp: &Comp,
) -> Result<HostAnswer, BridgeError>
{
    let image = check_and_lower(comp)?;
    let bytes = image.encode();
    let answer = host.run(&bytes)?;
    Ok(answer)
}

/// Lowers and runs a positive-core **machine** program, without the typed gate.
///
/// The compiled slice lowers the L machine's positive core, and the machine's
/// core is wider than the typed core in exactly one place: the machine's
/// duplication and discard are structural operations over any runtime value,
/// while the core's `dup` and `drop` are the grade rules, which require a
/// graded thunk. So `dup 4` runs on the machine and is not a typed
/// computation at all, and the checker is right to refuse it.
///
/// This entry exists for those programs and for nothing else. The gap closes
/// when the slice can represent a thunk, which is the codata rung: with a
/// thunk node in the image, a typed `dup` lowers and `compile_and_run` covers
/// the whole named set.
///
/// # Contract
/// - requires: `comp` is inside the machine's positive core; nothing checks its
///   type, which is the entire difference from `compile_and_run`.
/// - ensures: on success the answer is the compiled run's value and accounted
///   work.
/// - provides: the path for the machine-level grade fixtures.
/// - fails: [`BridgeError::NotLowered`], [`BridgeError::NotRun`].
/// - panics: none.
///
/// # Errors
/// The variants above.
///
/// # Adequacy
/// - hypothesis: L2 — the answers are held to the L machine's own, which is the
///   only oracle these programs have, since they have no type.
/// - witness: `bridge::the_bridge_agrees_with_the_l_machine_on_every_named_program`
#[inline]
pub fn run_machine_program(
    host: &CompileHost,
    comp: &Comp,
) -> Result<HostAnswer, BridgeError>
{
    let image = lower_computation(comp)?;
    let bytes = image.encode();
    let answer = host.run(&bytes)?;
    Ok(answer)
}

/// Whether a computation passes the bridge's typed gate.
///
/// # Contract
/// - ensures: true exactly when `check_and_lower` would accept `comp`.
/// - provides: the discriminator a caller needs to choose between
///   [`compile_and_run`] and [`run_machine_program`] without catching an error
///   to find out.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 — the two sides are pointwise: a typed program and a
///   machine-level grade program.
/// - witness: `lowering::the_typed_gate_admits_the_typed_programs_and_refuses_the_grade_ones`
#[inline]
#[must_use]
pub fn is_typed(comp: &Comp) -> TypedVerdict
{
    TypedVerdict(check_and_lower(comp).is_ok())
}

/// Whether a computation passes the bridge's typed gate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct TypedVerdict(bool);

impl From<TypedVerdict> for bool
{
    #[inline]
    fn from(verdict: TypedVerdict) -> Self
    {
        verdict.0
    }
}

/// Checks a core computation and lowers it, stopping short of the boundary.
///
/// # Contract
/// - requires: `comp` is a closed core computation.
/// - ensures: the returned image is what `compile_and_run` would have sent.
/// - provides: the half of the path that needs no host, so a checkout with no
///   MLIR still exercises the lowering and the wire form.
/// - fails: [`BridgeError::NotChecked`], [`BridgeError::NotLowered`].
/// - panics: none.
///
/// # Errors
/// The variants above.
///
/// # Adequacy
/// - hypothesis: L3 — a computation the checker refuses and a computation the
///   lowering refuses are distinct pointwise cases, each asserted on its exact
///   variant.
/// - witness: `lowering::a_computation_the_checker_refuses_never_reaches_the_lowering`
/// - witness: `lowering::every_excluded_core_form_is_refused_by_name`
#[inline]
pub fn check_and_lower(comp: &Comp) -> Result<Image, BridgeError>
{
    // Checking rather than inferring, against the unknown computation type.
    // The core's dispatch is check-only — it has no principal type to infer,
    // because its answer type is whatever both arms agree on — so an inferring
    // gate would refuse every program with a `case` in it, which is a third of
    // the slice. Checking against the hole asks the checker for everything it
    // can decide without an expected type, which is what a bridge with no
    // signature to consult can honestly ask for.
    let (checked, _trace) = run_comp(Ctx::new(), comp.clone(), Dir::Check(CompType::Unknown));
    if let Err(error) = checked {
        return Err(BridgeError::NotChecked {
            detail: CheckerDetail(alloc::format!("{error:?}")),
        });
    }
    let image = lower_computation(comp)?;
    Ok(image)
}
