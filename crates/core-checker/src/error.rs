//! Typing errors (`typing-machine.md` §"Error handling", core subset).
//!
//! One constructor per *user-facing* failure mode of core CBPV. The lone
//! exception is [`TypeError::ShapeMismatch`], which carries a second, internal
//! role: the machine's [`text::SHAPE_VALUE`] / [`text::SHAPE_COMP`]
//! descriptions guard the polarity invariant of [`crate::machine`] (a frame is
//! always resumed at the sort it suspended on). Those two are *unreachable by
//! construction* on states reachable from the public entry points — a
//! conformance meta-test asserts they never surface in a generated run — so in
//! practice each reachable failure still maps to one constructor. Both
//! implementations must produce *equal* errors on the same input; this is
//! asserted by the conformance property tests.

use thiserror::Error;

use crate::grade::Grade;
use crate::syntax::Term;
use crate::types::Ty;

/// Result type for this crate's typing operations.
pub type GandrCoreResult<T> = Result<T, TypeError>;

/// Hints and shape descriptions shared verbatim by the recursive checker and
/// the typing machine, so that the two implementations produce *equal* errors.
pub mod text
{
    /// Hint for an injection in inference mode (`typing-machine.md` §"The step
    /// function").
    pub const ANNOTATE_INJECTION: &str = "annotate this injection";
    /// Hint for a list literal in inference mode (rule List⇓ is check-only,
    /// like an injection: the element type comes only from the expectation, and
    /// the empty list `[]` cannot infer one; ADR-40 D3).
    pub const ANNOTATE_LIST: &str = "annotate this list or check against a list type List A";
    /// Hint for an unannotated abstraction in inference mode.
    pub const ANNOTATE_BINDER: &str = "annotate the binder or check against an arrow type";
    /// Hint for an abstraction checked against a non-arrow type.
    pub const ABS_NEEDS_ARROW: &str = "an abstraction only checks against an arrow type";
    /// Hint for a case analysis in inference mode (rule Case⇓ is check-only).
    pub const CASE_NEEDS_CHECK: &str = "case only checks; annotate or supply an expected type";
    /// Hint for a list-case in inference mode (rule `ListCase`⇓ is check-only,
    /// like `case`; ADR-40 D4).
    pub const LIST_CASE_NEEDS_CHECK: &str =
        "list-case only checks; annotate or supply an expected type";
    /// Hint for a declared-data constructor in inference mode (rule Ctor⇓ is
    /// check-only, like an injection; ADR-80 Decision 2).
    pub const ANNOTATE_CTOR: &str = "annotate this constructor or check against its data type D(ā)";
    /// Hint for a declared-data case in inference mode (rule `DataCase`⇓ is
    /// check-only, like `case`; ADR-80 Decision 3).
    pub const DATA_CASE_NEEDS_CHECK: &str =
        "data-case only checks; annotate or supply an expected type";
    /// Hint for a motive-less split in inference mode (rule Split⇓ is
    /// check-only; ADR-82 D3).
    ///
    /// A split *infers* only with an explicit dependent motive (rule
    /// `SplitMotive`⇑). This fires at rule entry, before the scrutinee premise
    /// — identically in the checker and the typing machine.
    pub const SPLIT_NEEDS_MOTIVE: &str = "a motive-less split only checks; supply a dependent motive (z. M) to infer, or an \
         expected type";
    /// Hint for a lazy pair away from a with-type (rule With⇓ is check-only).
    pub const WITH_NEEDS_WITH: &str = "a lazy pair only checks against a with-type";
    /// Hint for `dup` away from a returner-of-thunk-product (rule Dup is
    /// check-only: the split grades `r`/`s` come only from the expectation).
    pub const DUP_NEEDS_RETURNER_PRODUCT: &str =
        "dup only checks against a returner of a graded-thunk product F (U_r B × U_s B)";
    /// Hint for `perform op v` whose op is not declared by its inline-carried
    /// signature (rule Op's side condition `op ∈ E` fails; A3.2 `+effects`).
    pub const PERFORM_UNKNOWN_OP: &str = "the operation is not declared by this effect signature";
    /// Hint for a handler in inference mode (rule Handle is check-only against
    /// a returner answer; A3.2 `+effects`).
    pub const HANDLE_NEEDS_CHECK: &str =
        "handle only checks; supply an expected returner answer type F^ε C";
    /// Hint for a handler checked against a non-returner answer type (the
    /// clauses and the continuation `k : Stk(F^ε B_i, F^ε C)` need an `F^ε C`
    /// answer; A3.2 `+effects`).
    pub const HANDLE_NEEDS_RETURNER: &str =
        "handle only checks against a returner answer type F^ε C";
    /// Hint for a handler whose operation clauses do not cover exactly the
    /// signature's operations (deep-handler coverage is exact; A3.2
    /// `+effects`).
    pub const HANDLER_CLAUSES_MISMATCH: &str =
        "the handler clauses must cover exactly the signature's operations";
    /// Hint for a reified stack `stk K` away from a `Stk(B, C)` expectation
    /// (rule Reify is check-only: the consumed type `B` comes only from the
    /// expectation; A3.3 `+control`).
    pub const STK_NEEDS_STK_TYPE: &str = "stk only checks against a reified-stack type Stk(B, C)";
    /// Hint for a delimiter `reset t` in inference mode (rule Reset is
    /// check-only against the answer `C`, like a handler; A3.3 `+control`).
    pub const RESET_NEEDS_CHECK: &str = "reset only checks; supply an expected answer type C";
    /// Hint for a capture `shift k. t` in inference mode (rule Shift is
    /// check-only against the captured type `B`; A3.3 `+control`).
    pub const SHIFT_NEEDS_CHECK: &str = "shift only checks; supply an expected type B";
    /// Hint for a capture `shift k. t` with no enclosing `reset` (the ambient
    /// answer type is undetermined; A3.3 `+control`).
    pub const SHIFT_NEEDS_RESET: &str =
        "shift must appear inside a reset that fixes its answer type";
    /// Hint for a `pack` in inference mode (rule Pack⇓ is check-only).
    ///
    /// Stronger than the injection's reason: the abstract type components live
    /// only in the signature, so inferring a package type from the payload
    /// would mean guessing which of its types were meant to be abstract.
    pub const ANNOTATE_PACK: &str =
        "pack only checks; annotate it or check against a package type Package_r ⟨ᾱ⟩ A";
    /// Hint for a `pack` whose witness count disagrees with the signature's
    /// abstract type components.
    pub const PACK_ARITY_MISMATCH: &str =
        "supply exactly one witness type per abstract type component of the signature";
    /// Hint for an `unpack` in inference mode (the elimination is check-only,
    /// which is also the avoidance fence: an expectation formed outside the
    /// unpack cannot mention the atoms minted inside it).
    pub const UNPACK_NEEDS_CHECK: &str = "unpack only checks; supply an expected answer type, which is what keeps a minted \
         abstract type from escaping its scope";
    /// Hint for an `unpack` whose recorded atoms do not match the signature's
    /// abstract type components one for one.
    pub const UNPACK_ATOM_MISMATCH: &str =
        "an unpack binds exactly one distinct minted atom per abstract type component";
    /// Expected shape of a package's payload.
    ///
    /// A package's own grade and its payload thunk's grade are the same `r`
    /// rather than two independent annotations, so a payload graded otherwise
    /// is a malformed signature rather than a subtyping question. Fills the
    /// [`TypeError::ShapeMismatch`](crate::error::TypeError::ShapeMismatch)
    /// `expected` slot ("expected {this}").
    pub const SHAPE_PACKAGE_PAYLOAD: &str = "a thunk graded exactly as the package itself";
    /// Expected shape of a `pack`'s expectation and an `unpack`'s ascription.
    pub const SHAPE_PACKAGE: &str = "a package type";
    /// Hint for an instantiation refused because an identity endpoint in the
    /// payload mentions an abstract type component.
    pub const PACKAGE_ABSTRACT_UNDER_PATH: &str = "an identity-type endpoint in the payload mentions an abstract type component; \
         substituting a type through a term is outside this rung, and passing the endpoint \
         through would leave the component free";
    /// Hint for a package signature that declares one abstract type component
    /// twice.
    pub const PACKAGE_DUPLICATE_COMPONENT: &str =
        "a package signature declares each abstract type component once";
    /// Expected shape of an application head.
    pub const SHAPE_ARROW: &str = "an arrow type";
    /// Expected shape of a forced value.
    pub const SHAPE_THUNK: &str = "a thunk type";
    /// Expected shape of a bound computation.
    pub const SHAPE_RETURNER: &str = "a returner type";
    /// Expected shape of a case scrutinee.
    pub const SHAPE_SUM: &str = "a sum type";
    /// Expected shape of a declared-data case scrutinee (rule `DataCase`⇓;
    /// ADR-80 Decision 3).
    pub const SHAPE_DATA: &str = "a declared-data type";
    /// Expected shape of a list-case scrutinee (rule `ListCase`⇓; ADR-40 D4).
    pub const SHAPE_LIST: &str = "a list type";
    /// Expected shape of a split scrutinee.
    pub const SHAPE_PROD: &str = "a product type";
    /// Expected shape of a record-projection target (rule `RecordProj`;
    /// ADR-45 D4).
    pub const SHAPE_RECORD: &str = "a record type";
    /// Hint for a record projection `r.ℓ` whose record type has no field `ℓ`
    /// (the inferred record is well-shaped but lacks the projected label; rule
    /// `RecordProj`, ADR-45 D4).
    pub const RECORD_NO_FIELD: &str = "the record has no field with this label";
    /// Expected shape of a projection target.
    pub const SHAPE_WITH: &str = "a with-type";
    /// Expected shape of a resumed value (rule Resume; A3.3 `+control`).
    pub const SHAPE_STK: &str = "a reified-stack type";
    /// Expected polarity: a value type (internal invariant of the machine).
    pub const SHAPE_VALUE: &str = "a value type";
    /// Expected polarity: a computation type (internal invariant of the
    /// machine).
    pub const SHAPE_COMP: &str = "a computation type";
    /// Expected shape of an identity eliminator's scrutinee (rule `Walk`,
    /// ADR-76).
    pub const SHAPE_PATH: &str = "a path type";
    /// A `case` scrutinizing an identity type (ADR-76).
    ///
    /// The reserved `here`-pattern fragment's rung-1 K-rejection diagnostic,
    /// kept lock-step with the rung-2 lhs engine's own decline; fills the
    /// [`TypeError::ShapeMismatch`](crate::error::TypeError::ShapeMismatch)
    /// `expected` slot ("expected {this}").
    ///
    /// The message MUST contain the literal substring `without-k`: the
    /// K-rejection corpus witness (`pathological/identity/k-derivation.gandr`)
    /// asserts it on every diagnostic of its elaboration path, and the rung-2
    /// pattern engine inherits the same spelling when it declines the deletion
    /// step itself.
    pub const CASE_ON_PATH_WITHOUT_K: &str = "a sum type (case analysis on an identity type is reserved: a here pattern requires the \
         without-k unification fragment (rung 2) — solving its reflexive endpoint equation by \
         deletion is exactly the step without-k forbids (ADR-76); eliminate with the walk primitive \
         instead)";
}

/// A typing failure.
///
/// `ShapeMismatch` is the elimination-form refinement of `TypeMismatch`
/// adopted as ADR-27 decision 3 (now in the spec's §"Error handling"
/// inventory): with no unification variables yet, an elimination whose
/// principal premise infers the wrong *constructor* (e.g. applying a non-arrow)
/// has no complete "expected" type to report, only an expected shape.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum TypeError
{
    /// Subsumption failed: the term's type is not a subtype of the expected
    /// type.
    #[error("type mismatch: expected {expected:?}, actual {actual:?}")]
    TypeMismatch
    {
        /// The expected (checked-against) type.
        expected: Ty,
        /// The type the term actually has.
        actual: Ty,
    },

    /// An elimination's principal premise inferred a type of the wrong shape.
    #[error("type shape mismatch: expected {expected}, actual {actual:?}")]
    ShapeMismatch
    {
        /// A description of the expected type constructor.
        expected: &'static str,
        /// The type actually inferred.
        actual: Ty,
    },

    /// No typing rule applies to this term in this direction.
    #[error("no typing rule applies to term {expr:?}: {hint}")]
    StuckExpr
    {
        /// The stuck term.
        expr: Term,
        /// A hint for making progress (e.g. "annotate this injection").
        hint: &'static str,
    },

    /// A variable was used without a hypothesis in scope.
    #[error("variable is unbound: {name}")]
    UnboundVariable
    {
        /// The variable's name.
        name: String,
    },

    /// A grade-order requirement `lower ⊑ upper` failed.
    #[error("grade order requirement failed: {lower:?} ⊑ {upper:?} does not hold")]
    GradeError
    {
        /// The left-hand side of the failed `⊑`.
        lower: Grade,
        /// The right-hand side of the failed `⊑`.
        upper: Grade,
    },
}
