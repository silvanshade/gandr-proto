//! Directions and the machine control register (`typing-machine.md`
//! §"Control").
//!
//! [`Control`] doubles as the *trace event*: the recursive checker logs one
//! `Descend` event at each call entry and one `Return` event at each call
//! exit, and the typing machine's run is the sequence of control registers it
//! passes through. Step-for-step agreement is equality of these sequences.

use alloc::rc::Rc;

use gandr_core_term::boundary::FieldName;
use gandr_core_term::syntax::Comp;
use gandr_core_term::syntax::Value;
use gandr_core_term::types::CompType;
use gandr_core_term::types::Ty;
use gandr_core_term::types::ValueType;

/// The bidirectional mode: inference (`⇑`) or checking (`⇓`) against an
/// expected type.
///
/// Carrying the expected type inside `Check` is what removes the original
/// design's `KSubtype` pseudo-frames (`typing-machine.md` §"Control").
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Dir<T>
{
    /// Inference mode `⇑`.
    Infer,
    /// Checking mode `⇓` against the carried expected type.
    Check(
        /// The expected type.
        T,
    ),
}

impl Dir<ValueType>
{
    /// Component directions for a pair under this direction.
    ///
    /// Checking against a product distributes componentwise (rule Pair⇓);
    /// checking against `Unknown` distributes `Unknown` componentwise (the
    /// matched product `Unknown ▶× Unknown × Unknown`, A2.2 holes
    /// extension); any other direction infers both components (rule Pair⇑,
    /// with subsumption deciding a non-product expectation at the rebuild).
    #[inline]
    #[must_use]
    pub fn pair_components(&self) -> (Self, Self)
    {
        match self {
            | &Self::Check(ValueType::Prod(ref fst, ref snd)) => (
                Self::Check(fst.as_ref().clone()),
                Self::Check(snd.as_ref().clone()),
            ),
            | &Self::Check(ValueType::Unknown) => (
                Self::Check(ValueType::Unknown),
                Self::Check(ValueType::Unknown),
            ),
            | _ => (Self::Infer, Self::Infer),
        }
    }

    /// The direction for the record field `label` under this direction
    /// (ADR-45 D3 — the per-label generalization of [`Self::pair_components`]).
    ///
    /// Checking against a record `{…}` pushes the expected type of a *matching*
    /// field into it, and *infers* an extra field the expectation does not
    /// mention (so every field is still typed, and width subtyping is left to
    /// the rebuild's Sub rule); checking against `Unknown` distributes
    /// `Unknown` to every field (the matched record `Unknown ▶{} {…:Unknown}`,
    /// A2.2 holes extension); any other direction infers the field (rule
    /// Record⇑, with subsumption deciding a non-record expectation at the
    /// rebuild).
    #[inline]
    #[must_use]
    pub fn record_field_dir<'source, N>(
        &self,
        label: N,
    ) -> Self
    where
        N: Into<FieldName<'source>>,
    {
        let label = label.into();
        match self {
            | &Self::Check(ValueType::Record(ref fields)) => fields
                .get(label.as_ref())
                .map_or(Self::Infer, |ty| Self::Check(ty.as_ref().clone())),
            | &Self::Check(ValueType::Unknown) => Self::Check(ValueType::Unknown),
            | _ => Self::Infer,
        }
    }
}

impl Dir<CompType>
{
    /// The payload direction for `ret v` under this direction.
    ///
    /// Checking against `F A` checks the payload against `A` (rule Ret⇓);
    /// checking against `Unknown` checks the payload against `Unknown` (the
    /// matched returner `Unknown ▶F F Unknown`, A2.2 holes extension); any
    /// other direction infers the payload (rule Ret⇑, with subsumption
    /// deciding a non-returner expectation at the rebuild).
    #[inline]
    #[must_use]
    pub fn ret_payload(&self) -> Dir<ValueType>
    {
        match self {
            | &Self::Check(CompType::F(ref inner, _)) => Dir::Check(inner.as_ref().clone()),
            | &Self::Check(CompType::Unknown) => Dir::Check(ValueType::Unknown),
            | _ => Dir::Infer,
        }
    }
}

/// The control register of the typing machine, and equally the trace-event
/// vocabulary of the recursive checker.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Control
{
    /// About to type a value sub-term.
    DescendValue
    {
        /// The value being typed.
        value: Value,
        /// The direction it is typed in.
        dir: Dir<ValueType>,
    },
    /// About to type a computation sub-term.
    DescendComp
    {
        /// The computation being typed.
        comp: Comp,
        /// The direction it is typed in.
        dir: Dir<CompType>,
    },
    /// A sub-term's type, propagating up the stack.
    Return
    {
        /// The type produced by the completed sub-derivation.
        ty: Ty,
    },
}

/// A trace: the sequence of control states of a complete (or failed) run.
pub type Trace = Vec<Control>;

/// Clones a reference-counted node into an owned term/type.
///
/// Children of the AST are reference counted, so this is cheap: it either
/// unwraps a unique node or shallow-clones one whose own children stay
/// shared.
#[inline]
#[must_use]
pub(crate) fn unrc<T>(node: Rc<T>) -> T
where
    T: Clone,
{
    Rc::unwrap_or_clone(node)
}
