//! The shared runtime-outcome vocabulary of the gandr machines (ADR-9,
//! ADR-34 D4).
//!
//! An evaluation reaches one of three defined shapes — a **terminal** (a
//! value-producing whnf at the empty continuation), a **blame** ([`Blame`] — a
//! typed runtime halt), or an **undefined stuck** ([`StuckReason`] — reachable
//! only on ill-typed input) — packaged as [`Eval`]. These types are the
//! **durable** boundary the operational semantics is expressed over: they
//! outlive any one machine. The L machine (`gandr-core-sequent`) is the live
//! driver that produces them; the retired CEK oracle produced the *same*
//! [`Eval`] so the two were directly comparable during the L1 migration (the
//! ADR-9 differential). [`STEP_BUDGET`] is the shared step-count net both
//! machines run under.
//!
//! This module is the outcome vocabulary's durable home for the same reason
//! [`crate::host`] is the host seam's: the boundary is
//! representation-independent and survives the machine that realizes it.

use crate::syntax::Comp;

/// A **defined** runtime halt (ADR-34 D4): a typed outcome that is *not* an
/// undefined stuck.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Blame
{
    /// A gradual hole reached an elimination, so it has no value to produce
    /// (the `Canonicity.normalize` blame arm; A2.2).
    Hole,
    /// A `shift` reached no enclosing `KReset` (or its capture would cross a
    /// handler) — the transitional outcome for the over-accepted
    /// escaping-`shift` set (ADR-34 D5); the deferred conservative restriction
    /// will reject these statically.
    ShiftNoReset,
    /// A `perform` reached no matching `KHandle` (or its capture would cross a
    /// delimiter / unrelated handler — the v0 structural single-handler scope).
    PerformNoHandler,
}

/// An **undefined** stuck configuration: reachable only on an ill-typed input.
///
/// The operational soundness oracle pins that a closed well-typed `F A`
/// computation never reaches one (ADR-34 D4).
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum StuckReason
{
    /// A non-function terminal met an argument frame, or a function met a
    /// non-argument frame — an ill-typed application.
    AppliedNonFunction,
    /// A non-returner terminal met a bind frame, or `ret v` met an argument /
    /// projection frame — an ill-typed sequencing.
    SequencedNonReturner,
    /// `force` was applied to a value that is not a thunk.
    ForcedNonThunk,
    /// `case` scrutinized a value that is not an injection.
    CasedNonSum,
    /// A declared-data `case` scrutinized a value that is not a matching
    /// constructor — a non-`Ctor` value, or a `Ctor` whose `tag` is out of the
    /// arm range (an ill-typed / non-exhaustive elimination; ADR-80).
    DataCasedNonCtor,
    /// A list-case scrutinized a value that is not a list (ADR-40 D4).
    ListCasedNonList,
    /// `split` scrutinized a value that is not a pair.
    SplitNonProduct,
    /// A projection met a focus that is not a lazy pair, or a lazy pair met a
    /// non-projection frame.
    ProjectedNonPair,
    /// A record projection `r.ℓ` scrutinized a value that is not a record
    /// (ADR-45 D4).
    RecordProjNonRecord,
    /// A record projection `r.ℓ` scrutinized a record that has no field `ℓ`
    /// (ADR-45 D4).
    RecordProjMissingField,
    /// `resume` was applied to a value that is not a reified stack.
    ResumedNonStack,
    /// The identity eliminator `Walk` scrutinized a value that is not a `here`
    /// (ADR-76) — an ill-typed elimination, mirroring [`Self::CasedNonSum`]. On
    /// a well-typed closed program the scrutinee is always `here(v)`
    /// (canonicity of the identity type), so this is reachable only on
    /// ill-typed input.
    WalkOnNonHere,
    /// A former the focusing translation `𝓕` does not yet realize on the L
    /// machine (the sequent kernel's not-yet-built surface): the machine
    /// reports this defined stuck rather than diverging, and the corpus
    /// differential treats it as a declined item. (Historically also the
    /// sentinel the CEK's recursive reference evaluator returned on a
    /// control / effect form, whence the name; the differential never fed
    /// the reference one.)
    UnsupportedByReference,
    /// A closure body id did not resolve in its arena. This indicates a
    /// corrupted transitional closure adapter, not a well-typed source term.
    InvalidClosureBody,
    /// The continuation-key allocator exhausted its atom identity space while
    /// α-renaming a captured continuation binder.
    FreshContinuationNameExhausted,
    /// The step budget was exhausted (the safety net for the machine's run
    /// loop; not reachable on the bounded, non-recursive v0 fragment — see
    /// [`STEP_BUDGET`]).
    StepLimit,
}

/// The final outcome of a machine run: a terminal, a defined blame, or an
/// undefined stuck.
///
/// A direct-style reference and the step machine were directly comparable over
/// it (ADR-9 differential). The L machine (in `gandr-core-sequent`) is the live
/// producer; the retired CEK oracle produced the same [`Eval`].
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Eval
{
    /// A terminal computation — a value-producing whnf (`ret v`, `λx. t`, or a
    /// lazy pair) reached at the empty continuation.
    Value(
        /// The terminal computation.
        Comp,
    ),
    /// A defined runtime halt (a gradual hole or a control blame).
    Blame(
        /// Which defined halt.
        Blame,
    ),
    /// An undefined stuck configuration (ill-typed input).
    Stuck(
        /// Why the configuration is stuck.
        StuckReason,
    ),
}

/// The maximum number of reduction steps a machine takes before halting with
/// [`StuckReason::StepLimit`].
///
/// The v0 fragment has no recursion / fixpoint and substitutes only closed
/// values, so evaluation of a finite term terminates; this is a pure safety net
/// against a pathological (necessarily ill-typed) input, set far above any term
/// the test fragment produces.
///
/// **Single source of truth.** This is the one budget constant the whole
/// workspace shares: the L machine (`gandr-core-sequent`) and the `gandr-shell`
/// / `gandr-ffi` drivers all run the same budget so their step-count nets stay
/// at parity under the shared bound. Keeping one public constant is the L1
/// de-duplication of the former driver mirrors (`proposal-sequent-kernel.md`
/// §9, phase L1).
pub const STEP_BUDGET: u64 = 1_000_000;
