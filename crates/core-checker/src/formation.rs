//! **Type formation**: the judgement that says what classifier a type is
//! formed at, and refuses when it is formed at none.
//!
//! Every value type and every computation type has a rule here, and the rule
//! answers with a `Classifier` — a sort and a level — or with a named
//! `FormationError`. Nothing falls through, and nothing guesses: a former
//! outside the admitted fragment is a named refusal, not a silent
//! `ValueType::Unknown`, which is the gradual hole an author wrote and has a
//! rule of its own.
//!
//! # Why it is one interface
//!
//! Before this module, a caller that needed a type's level inferred a sort
//! from an enum name and then guessed a level beside it. That is two
//! judgements in the caller's head and neither of them is checked. Formation
//! is where both are decided, once, so universe lifting, level joins, family
//! telescope checking, and cumulativity are one implementation rather than a
//! convention.
//!
//! # The one level algebra
//!
//! Every successor and every join goes through `gandr_kernel_strata::Level`.
//! There is no arithmetic on levels in this module and there will not be: the
//! rule that decides a level and the oracle that orders one are the same code,
//! which is what makes a formation answer checkable against the kernel's.

pub mod context;
pub mod rules;

pub use context::FamilySignature;
pub use context::FormationContext;
use gandr_core_term::classifier::Classifier;
use gandr_core_term::ctx::Ctx;
use gandr_core_term::error::FormationError;
use gandr_core_term::error::UnsupportedForm;
use gandr_core_term::types::Ty;
pub use rules::FormType;

/// What formation delivers to a consumer on the checking path.
///
/// # Why four arms rather than a `Result`
///
/// A `Result` collapses two questions a consumer must keep apart: *did the
/// type form*, and *whose fact does the failure record*. The classifier is the
/// buildout map's capability-boundary rule — a diagnostic states a fact about
/// the source or a state of the engine, and the two never share a class — and
/// a consumer that raised every `Err` as a typing error would report the
/// engine's own admitted fragment as the author's mistake.
///
/// The concrete instance, measured rather than imagined: `U[1] (Integer -> F
/// Integer)` appears as a paired signature in `27-paired-signatures.gandr`, a
/// model example whose corpus expectation is `clean`. Formation refuses it as
/// a [`FormationError::GradedBridge`], because the graded bridges are outside
/// the fragment it admits and its module documentation says so. Raising that
/// as a typing error would turn a documented non-admission into a red corpus.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FormationVerdict
{
    /// The type is formed, at this classifier.
    Formed(Classifier),
    /// The type is outside the fragment formation admits.
    ///
    /// Formation records **no fact about the source**: the author may have
    /// written something perfectly good that this rung cannot yet classify.
    /// A consumer abstains — it neither trusts nor refuses the type on
    /// formation's word.
    OutsideFragment(FormationError),
    /// The type is malformed: a named fact **about the source**.
    ///
    /// Something is wrong rather than missing, and the consumer refuses.
    Malformed(FormationError),
    /// Formation reached a state that records a fact about **the engine**,
    /// not about the source.
    ///
    /// A result-stack imbalance is a defect in formation itself. It is kept
    /// out of [`Self::Malformed`] because folding it there would report an
    /// engine defect as the author's mistake.
    EngineFault(FormationError),
}

/// Classify a declared type against the scopes a checking context supplies.
///
/// This is the judgement's entry point for a consumer on the checking path:
/// it builds the formation scopes from `ctx`
/// ([`FormationContext::from_checking_context`]) and sorts the answer by whose
/// fact it records.
///
/// # The fragment narrowing, stated where the claim is made
///
/// Two arms are classified as [`FormationVerdict::OutsideFragment`] because of
/// what the producer declares rather than because of what the judgement can
/// decide:
///
/// - [`UnsupportedForm::UnboundTypeFamily`] and
///   [`UnsupportedForm::DependentFamilyArgument`] — the producer declares no
///   family signatures at this rung, so *every* family application would
///   otherwise be reported as malformed. That would be a false accusation about
///   the source on the strength of a scope nothing populates.
/// - [`UnsupportedForm::TypeBinder`] — the judgement has no binder rule, so a
///   dependent arrow, a sigma, or a package is refused at the former rather
///   than walked into. The alternative is the same false accusation one level
///   down: the body's mention of a bound name reported as an undeclared name.
/// - [`UnsupportedForm::DataSignature`] — a data application stores its
///   arguments as value types, so a value index is an atom the judgement cannot
///   tell from a type name, and the producer declares no data scope that would
///   settle it.
///
/// When a family-signature producer lands, `UnboundTypeFamily` becomes a
/// genuine fact about the source and moves to [`FormationVerdict::Malformed`].
/// `TypeBinder` lifts when formation gains binder scoping — which is the
/// dependent-classifier work, since the surface's `Type` annotation lowers to
/// a rigid atom today and there is no universe to read a binder's classifier
/// off. Until then these narrowings are stated here, at the claim, rather than
/// left for a reader to infer from the producer.
///
/// # Contract
/// - ensures: `Formed` carries the classifier the judgement derived.
/// - ensures: a refusal is sorted by whose fact it records, never by severity.
/// - panics: none.
#[must_use]
#[inline]
pub fn classify_declared_type(
    ty: &Ty,
    ctx: &Ctx,
) -> FormationVerdict
{
    let formation = FormationContext::from_checking_context(ctx);
    let answer = match *ty {
        | Ty::Value(ref value) => value.infer_classifier(&formation),
        | Ty::Comp(ref comp) => comp.infer_classifier(&formation),
    };
    match answer {
        | Ok(classifier) => FormationVerdict::Formed(classifier),
        | Err(error) => match error {
            | FormationError::GradedBridge { .. }
            | FormationError::UnsupportedForm(
                UnsupportedForm::UnboundTypeFamily
                | UnsupportedForm::DependentFamilyArgument
                | UnsupportedForm::DataSignature
                | UnsupportedForm::TypeBinder,
            ) => FormationVerdict::OutsideFragment(error),
            | FormationError::UnsupportedForm(
                UnsupportedForm::ResultStackUnderflow | UnsupportedForm::ResultStackCardinality,
            ) => FormationVerdict::EngineFault(error),
            | _ => FormationVerdict::Malformed(error),
        },
    }
}
