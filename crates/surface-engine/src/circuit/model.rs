//! `Model(S)` — the module signature a shape's models inhabit, **computed from
//! the shape's elaborated description**.
//!
//! # What this is, and why it is a computation rather than a spelling
//!
//! A `sign` block *presents* a theory; `Model(S)` *structures* what it takes to
//! be a model of it. The design's own sentence is that `Model(S)` is a
//! signature expression computed by elaboration, so the computed former is what
//! `Model(S)` **is** — a hand-written signature exercises its shape and never
//! its definition.
//!
//! The hand-written `Model(CatShape)` lives beside the flagship's own witness
//! and is this computation's **oracle**: the acceptance condition for what is
//! built here is definitional agreement with it, on the same shape. Read them
//! as a pair. Where they disagree, the disagreement is a finding rather than a
//! failure, and which side is wrong is decided by reading each against the
//! staged design — the hand-written signature has no privilege beyond being
//! the one a human checked.
//!
//! # The clauses
//!
//! | member of `S`                                | field of `Model(S)`                                          |
//! | -------------------------------------------- | ------------------------------------------------------------ |
//! | `sort X(Δ)`                                  | `type X : Δ → Type`                                           |
//! | `oper f : T̄ --> X`                           | `val f : U_ω (T̄ → F X)`                                       |
//! | `rule r : l ==> t` at sort `X`               | `val r : U_ω (Π Γ_r → F Path(X, ⟦l⟧, ⟦t⟧))`                   |
//! | `rule m : ρ ==> ρ′` at rule-sorted endpoints | `val m : U_ω (Π Γ_m → F Path(Path(X, ⟦l⟧, ⟦t⟧), ⟦ρ⟧, ⟦ρ′⟧))`  |
//!
//! Three things the table does not say, which the hand-written signature
//! decided and which this computation must decide the same way or explain why:
//!
//! - **Where an operation's free sort variables are bound.** `oper comp : (f :
//!   Hom(a, b), g : Hom(b, c)) --> Hom(a, c)` mentions `a`, `b`, `c` and binds
//!   none of them. Every free sort variable of an operation's declaration
//!   becomes a `Π` parameter of its field, in order of first occurrence left to
//!   right through the arguments and then the result.
//! - **What a rule's context `Γ_r` contains.** The sort variables its endpoints
//!   mention plus the 1-cell variables, in the same first-occurrence order.
//! - **What an endpoint's implicit index arguments are.** An operation's field
//!   takes its sort parameters explicitly, so `⟦comp(id(a), f)⟧` is `comp(a, a,
//!   b, id(a), f)`: the identity's own index fixes the middle one.
//!
//! # What must land before this can
//!
//! - **A description must be able to hold an indexed sort.** A `SortDesc`
//!   carries a name and a polarity and no index telescope, so `sort Hom(dom :
//!   Ob, cod : Ob)` has nowhere to put its indices — and the first clause is
//!   exactly `Δ → Type`, which is `Δ`. `gandr-wvd.6.1.2`.
//! - **A `sign` block's rule members must reach the description.** The stage-0
//!   route reads a `rule` member as a circuit rule needing a filler and
//!   declines the term-face spelling `lhs ==> rhs` by name, so no law field has
//!   an endpoint to be computed from. `gandr-wvd.6.1.3`.
//! - **The signature grammar must be able to say what the clauses produce**:
//!   kinded type components for the first clause, and the type-family
//!   applications the later clauses mention. `gandr-wvd.6.2`.
//!
//! Each of those is a **named** prerequisite rather than a discovered
//! obstruction: none of them is a thing this module may work around, because
//! working around one means emitting a field shape the design refused in
//! advance — shipping `Model` early through ad-hoc non-dependent encodings
//! would freeze a crippled field shape that instances then depend on.

use crate::lower::KindedComponent;
use crate::lower::TypeComponent;

/// The signature a shape's models inhabit.
///
/// Held as the two component kinds the module signature grammar has rather
/// than as one list, because the two answer different questions and a consumer
/// entitled to only one must not reach the other by construction: a kinded
/// component is **matched against**, a manifest one **expands**.
pub struct ModelSignature
{
    /// The abstract type components, one per sort, in declaration order.
    pub sorts: Vec<KindedComponent>,
    /// The manifest type components, if the shape's own presentation fixes
    /// any. A `sign` block fixes none today; the field exists so a later
    /// presentation that does has somewhere to put them rather than a reason
    /// to reshape this type.
    pub manifest: Vec<TypeComponent>,
    /// The value components, one per operation and one per rule, in
    /// declaration order — operations before rules, because a rule's field
    /// type mentions the operation fields and a signature's components scope
    /// left to right.
    pub values: Vec<ModelField>,
}

/// One value component of a model signature, with the shape member it came
/// from.
///
/// The provenance is carried rather than derived, because the acceptance
/// condition for this computation is agreement with a hand-written signature
/// **member by member**, and a disagreement is only actionable if it names
/// which member disagreed.
pub struct ModelField
{
    /// The field's label, which is the shape member's own name.
    pub label: String,
    /// Which clause produced it.
    pub clause: ModelClause,
}

/// Which clause of the `Model(S)` table a field came from.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelClause
{
    /// `oper f : T̄ --> X`.
    Operation,
    /// `rule r : l ==> t` at a sort.
    Rule,
    /// `rule m : ρ ==> ρ′` at rule-sorted endpoints.
    Coherence,
}

/// Compute `Model(S)` from a shape's elaborated description.
///
/// # Contract
/// - requires: `desc` is the description a `sign` block elaborated to, with its
///   sorts, operations, and rules all read — a description missing a member is
///   a description of a different shape, and computing over it would produce a
///   signature for a theory nobody wrote.
/// - ensures: one abstract type component per sort, kinded at the sort's index
///   telescope; one value component per operation and per rule, in declaration
///   order, operations first.
/// - ensures: every field's type mentions only this signature's own components
///   and the shape's own members — never a name from the elaboration
///   environment, because a model signature that depends on where it was
///   computed is not a signature of the shape.
/// - fails: **by name**, for any member whose field this cannot construct. A
///   coherence rule whose faces are boundary-language composites is the known
///   case: an identity-type endpoint has no spelling for `then` or `cong`, so
///   emitting anything for it would state a weaker theorem under its name.
/// - panics: never.
///
/// # Adequacy
/// - hypothesis: the four clauses are a total function of the description, so
///   the computation needs no information the `sign` block did not carry.
/// - mutants: emit sorts in reverse order; bind an operation's sort variables
///   alphabetically rather than by first occurrence; omit a rule's sort
///   variables from its context; emit a degraded field for a coherence rule
///   rather than declining.
/// - witnesses: `the_computed_model_agrees_with_the_hand_written_signature`,
///   `a_coherence_rule_declines_by_name_rather_than_degrading`, and
///   `an_operations_sort_variables_are_bound_in_first_occurrence_order`.
pub fn model_signature(_desc: &gandr_theory_levitation::SignDesc) -> ModelSignature
{
    todo!("gandr-0ika: compute Model(S) from the elaborated description")
}
