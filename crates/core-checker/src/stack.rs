//! The stack-typing judgment for reified stacks (`effects-control-shell.md`
//! §2.1; contract §6.3; ADR-33 D6/D7; A3.3 `+control`).
//!
//! Levy's CBPV has a third syntactic sort beside values and computations:
//! **stacks** (evaluation contexts), typed `K : B ⇒ C` ("`K` consumes a
//! `B`-computation and delivers a `C`"). gandr internalizes the judgment as the
//! value type [`crate::types::ValueType::Stk`] `Stk(B, C)`, inhabited by the
//! reified-stack term [`crate::syntax::Value::Stk`] (`stk K`).
//!
//! # The forward (synthesizing) reading the checker and machine share
//!
//! `stk K` is **check-only** against an expected `Stk(B, C)` (the consumed type
//! `B` is the input; the empty stack is `B ⇒ B` for *any* `B`, so the input is
//! never inferable). Both implementations type the stack by running it
//! **forward** from the consumed type `B`: the empty stack delivers its input
//! unchanged; an argument frame `v :: K` consumes a function `A → B′`, checks
//! `v ⇓ A`, and continues from `B′`; a bind frame `(x. u) :: K` consumes a
//! returner `F^ε A`, binds `x : A`, infers `u`, and continues from the
//! sequenced type (`ε` folds in exactly as at [`crate::syntax::Comp::Bind`],
//! via [`crate::effect::combine_bind_row`]); a projection frame `prjᵢ :: K`
//! consumes a lazy product `B₁ & B₂` and continues from `Bᵢ`. The synthesized
//! output is then fit to the expected `C` by the inlined Sub rule (so the
//! contravariant- `B` / covariant-`C` variance of ADR-33 D6 is discharged
//! through [`crate::subtype::finish_value`]).
//!
//! The per-frame type destructures live **here**, shared verbatim, so the
//! recursive checker's stack recursion and the typing machine's frame walk
//! cannot drift (the A3.2 soundness hole was exactly a check that diverged
//! between the two faces under a shared-implementation test).
//!
//! # The two-zone context `Γ; Σ` and one-shot linearity (ADR-33 D5)
//!
//! The §2.4 one-shot/linear discipline — a `Stk` capturing `Σ`-resident
//! obligations is itself `Σ`-resident; discarding it without `resume` runs the
//! recorded unwind; duplication is permitted only when the captured `Σ` is
//! empty — is **typing-side** and rides the linear zone [`crate::ctx::Sigma`].
//! It is **vacuous in v0**: every `Σ`-obligation source (session endpoints,
//! held capabilities, acquired channels) is a deferred `+feature` (contract
//! §9), so a source-level `stk K` captures no obligations, a reified stack is
//! never `Σ`-resident, and `resume` / `discard` / duplication are unrestricted.
//! The discipline's machinery is exercised directly over [`crate::ctx::Sigma`]
//! (its own unit tests), so it is not "green because vacuous"; no v0 typing
//! rule populates `Σ`, which a conformance meta-invariant pins.

use crate::effect::EffectRow;
use crate::error::TypeError;
use crate::error::text;
use crate::syntax::Side;
use crate::types::CompType;
use crate::types::Ty;
use crate::types::ValueType;

/// Destructures the consumed type of an argument frame `v :: K`: a function
/// `A → B′` yields its argument type `A` (the value `v` is checked against it)
/// and result type `B′` (the rest of the stack runs from it).
///
/// The matched arrow `Unknown ▶→ Unknown → Unknown` (A2.2 holes extension): an
/// `Unknown` consumed type yields `(Unknown, Unknown)`, so a hole flowing into
/// a stack frame localizes rather than cascading — exactly as at
/// [`crate::syntax::Comp::App`].
///
/// # Contract
/// - ensures: `(A, B′)` for `consumed = A → B′`; `(Unknown, Unknown)` for
///   `consumed = Unknown`.
/// - fails: `ShapeMismatch { expected: SHAPE_ARROW, actual: consumed }`
///   otherwise.
/// - panics: none.
///
/// # Errors
///
/// Returns [`TypeError::ShapeMismatch`] when the consumed type is neither a
/// function nor `Unknown`.
#[inline]
pub(crate) fn arrow_components(consumed: CompType) -> Result<(ValueType, CompType), TypeError>
{
    match consumed {
        | CompType::Arrow(arg, res) => Ok((crate::control::unrc(arg), crate::control::unrc(res))),
        | CompType::Unknown => Ok((ValueType::Unknown, CompType::Unknown)),
        | other => Err(TypeError::ShapeMismatch {
            expected: text::SHAPE_ARROW,
            actual: Ty::Comp(other),
        }),
    }
}

/// Destructures the consumed type of a bind frame `(x. u) :: K`: a returner
/// `F^ε A` yields the payload `A` (bound to `x`) and the row `ε` (folded into
/// the continuation's result by [`crate::effect::combine_bind_row`], as at
/// [`crate::syntax::Comp::Bind`]).
///
/// The matched returner (A2.2): an `Unknown` consumed type binds `x` at
/// `Unknown` with an empty row.
///
/// # Contract
/// - ensures: `(A, ε)` for `consumed = F^ε A`; `(Unknown, ⟨⟩)` for `consumed =
///   Unknown`.
/// - fails: `ShapeMismatch { expected: SHAPE_RETURNER, actual: consumed }`
///   otherwise.
/// - panics: none.
///
/// # Errors
///
/// Returns [`TypeError::ShapeMismatch`] when the consumed type is neither a
/// returner nor `Unknown`.
#[inline]
pub(crate) fn returner_components(consumed: CompType) -> Result<(ValueType, EffectRow), TypeError>
{
    match consumed {
        | CompType::F(payload, row) => Ok((crate::control::unrc(payload), row)),
        | CompType::Unknown => Ok((ValueType::Unknown, EffectRow::EMPTY)),
        | other => Err(TypeError::ShapeMismatch {
            expected: text::SHAPE_RETURNER,
            actual: Ty::Comp(other),
        }),
    }
}

/// Destructures the consumed type of a projection frame `prjᵢ :: K`: a lazy
/// product `B₁ & B₂` yields the projected conjunct `Bᵢ` (the rest of the stack
/// runs from it).
///
/// The matched with (A2.2): an `Unknown` consumed type projects to `Unknown`.
///
/// # Contract
/// - ensures: `Bᵢ` (`side`-selected) for `consumed = B₁ & B₂`; `Unknown` for
///   `consumed = Unknown`.
/// - fails: `ShapeMismatch { expected: SHAPE_WITH, actual: consumed }`
///   otherwise.
/// - panics: none.
///
/// # Errors
///
/// Returns [`TypeError::ShapeMismatch`] when the consumed type is neither a
/// lazy product nor `Unknown`.
#[inline]
pub(crate) fn with_component(
    consumed: CompType,
    side: Side,
) -> Result<CompType, TypeError>
{
    match consumed {
        | CompType::With(fst, snd) => Ok(crate::subtype::pick(side, &fst, &snd)),
        | CompType::Unknown => Ok(CompType::Unknown),
        | other => Err(TypeError::ShapeMismatch {
            expected: text::SHAPE_WITH,
            actual: Ty::Comp(other),
        }),
    }
}

/// Destructures a resumed value's type (rule Resume, `effects-control-shell.md`
/// §2.1): a reified stack `Stk(B, C)` yields the consumed type `B` (the fed
/// computation is checked against it) and the delivered answer `C` (the result
/// of the resumption).
///
/// The matched stack (A2.2): an `Unknown` resumed value feeds its computation
/// against `Unknown` and delivers `Unknown`, so a hole in stack position
/// localizes rather than cascading — exactly as a hole head at
/// [`crate::syntax::Comp::App`].
///
/// # Contract
/// - ensures: `(B, C)` for `stk = Stk(B, C)`; `(Unknown, Unknown)` for `stk =
///   Unknown`.
/// - fails: `ShapeMismatch { expected: SHAPE_STK, actual: stk }` otherwise.
/// - panics: none.
///
/// # Errors
///
/// Returns [`TypeError::ShapeMismatch`] when the resumed value is neither a
/// reified stack nor `Unknown`.
#[inline]
pub(crate) fn stk_components(stk: ValueType) -> Result<(CompType, CompType), TypeError>
{
    match stk {
        | ValueType::Stk(consumes, delivers) => Ok((
            crate::control::unrc(consumes),
            crate::control::unrc(delivers),
        )),
        | ValueType::Unknown => Ok((CompType::Unknown, CompType::Unknown)),
        | other => Err(TypeError::ShapeMismatch {
            expected: text::SHAPE_STK,
            actual: Ty::Value(other),
        }),
    }
}
