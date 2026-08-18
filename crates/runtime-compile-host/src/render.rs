//! The canonical value rendering both sides of the boundary compare on.
//!
//! The host answers with text rather than with a binary value, because the two
//! sides are separate builds with separate toolchains and a shared binary
//! encoding would be a third thing to keep true. This module is the Rust end
//! of that agreement: it projects an L machine terminal value into the same
//! grammar the host prints.
//!
//! The grammar is deliberately partial. A value outside the compiled slice has
//! no spelling here, because the slice's boundary is what the comparison is
//! about; giving such a value a rendering would quietly widen the claim.

use gandr_core_term::syntax::Side;
use gandr_core_term::syntax::Value;

use crate::host::RenderedValue;

/// What can go wrong while rendering a value.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum RenderError
{
    /// The value lies outside the compiled slice's grammar.
    #[error("the value lies outside the compiled slice and has no canonical rendering")]
    OutsideSlice,
}

/// One step of the rendering walk.
enum Step<'value>
{
    /// A value whose rendering has not started.
    Node(&'value Value),
    /// Text to append when the step is reached.
    Text(&'static str),
}

/// Renders a value in the grammar the host prints.
///
/// The grammar is `(int N)`, `(unit)`, `(pair V V)`, `(inl V)`, `(inr V)`.
///
/// # Contract
/// - requires: `value` is a terminal value of the compiled slice.
/// - ensures: a returned rendering is a closed s-expression in that grammar,
///   character for character what the host prints for the same value.
/// - provides: the comparison surface the agreement differential uses.
/// - fails: [`RenderError::OutsideSlice`] for any other value, loudly rather
///   than by inventing a spelling.
/// - panics: none; the traversal is an explicit stack, so a deep value costs
///   heap rather than the host stack.
///
/// # Errors
/// [`RenderError::OutsideSlice`] at the first value form outside the grammar.
///
/// # Adequacy
/// - hypothesis: L2 with an L3 residue — every accepted form is compared
///   against the host's own printing of the same run, and the refusal is the
///   residue, triggered by a value form the slice excludes.
/// - witness: `bridge::the_bridge_agrees_with_the_l_machine_on_every_named_program`
/// - witness: `rendering::a_value_outside_the_slice_has_no_spelling`
#[inline]
pub fn canonical(value: &Value) -> Result<RenderedValue, RenderError>
{
    let mut rendered = String::new();
    let mut pending: Vec<Step<'_>> = alloc::vec![Step::Node(value)];

    while let Some(step) = pending.pop() {
        let node = match step {
            | Step::Text(text) => {
                rendered.push_str(text);
                continue;
            },
            | Step::Node(node) => node,
        };

        match *node {
            | Value::Int(literal) => {
                rendered.push_str("(int ");
                rendered.push_str(&literal.to_string());
                rendered.push(')');
            },
            | Value::Unit => rendered.push_str("(unit)"),
            | Value::Pair(ref first, ref second) => {
                rendered.push_str("(pair ");
                pending.push(Step::Text(")"));
                pending.push(Step::Node(second));
                pending.push(Step::Text(" "));
                pending.push(Step::Node(first));
            },
            | Value::Inj(side, ref payload) => {
                rendered.push_str(if side == Side::Fst { "(inl " } else { "(inr " });
                pending.push(Step::Text(")"));
                pending.push(Step::Node(payload));
            },
            | _ => return Err(RenderError::OutsideSlice),
        }
    }

    Ok(RenderedValue::from(rendered))
}
