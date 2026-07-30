//! Effect signatures and the effect-row carrier (`effects-control-shell.md`
//! §1; `type-system.md` §3.1; ADR-14, ADR-33).
//!
//! Effects sit on `F`, the dual of the grade on `U` (`grade.rs`): the returner
//! `F^ε A` carries an [`EffectRow`] `ε`. A row is a concrete finite set of
//! effect signatures (ADR-33 D2): in the bottom-up reading the build is
//! responsible for, `perform` contributes its op's signature, sequencing takes
//! the [`EffectRow::union`], and `handle` subtracts the handled signature with
//! [`EffectRow::without`]; the row leg of `F^ε` subtyping is `ε ⊆ ε′`
//! ([`EffectRow::is_subset`]) — a computation that may do *fewer* effects is
//! usable where *more* are allowed. The bottom-up row arithmetic lands with the
//! `+effects` terms (`perform` / `handle`); this module is the data model the
//! type layer carries.
//!
//! Carrier discipline (ADR-33 D2, mirroring ADR-28's grade seal): [`EffectRow`]
//! is a newtype over a crate-private representation, so its entire cross-module
//! surface is the set signature — [`EffectRow::EMPTY`],
//! [`EffectRow::singleton`], [`EffectRow::union`], [`EffectRow::without`],
//! [`EffectRow::contains`], [`EffectRow::is_subset`], [`EffectRow::is_empty`],
//! and [`EffectRow::signatures`]. A different row representation — in
//! particular the frozen open-tail / row-polymorphic `ρ` (`core-ir-contract.md`
//! §5), reserved for `+poly` — is then an edit to this module alone, not a
//! rule-wide rewrite.
//!
//! Signatures are **inline-carried and environment-free** (ADR-33 D3): an
//! [`EffectSig`] (a named interface of typed [`EffectOp`]s) rides directly on
//! the `perform` / `handle` nodes, so there is no global effect-declaration
//! form and no signature context. A signature is identified by its
//! [`EffectSig::name`]; the build maintains the invariant that one name denotes
//! one signature, so a row keyed by name is well defined.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use crate::boundary::EffectContains;
use crate::boundary::EffectRowEmptyStatus;
use crate::boundary::EffectSignatureName;
use crate::boundary::EffectSubsetDecision;
use crate::boundary::OperationName;
use crate::error::TypeError;
use crate::error::text;
use crate::syntax::OpClause;
use crate::types::CompType;
use crate::types::Ty;
use crate::types::ValueType;

/// A single effect operation `op : A ↠ B` (`effects-control-shell.md` §1.1):
/// the operation `name`, the `payload` it is performed with (`A`), and the
/// `reply` it resumes with (`B`).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct EffectOp
{
    /// The operation's name (e.g. `get`, `put`).
    name: String,
    /// The payload type `A` the operation is performed with (`perform op v`
    /// checks `v ⇓ A`).
    payload: ValueType,
    /// The reply type `B` the operation resumes with (`perform op v ⇑ F^… B`).
    reply: ValueType,
}

impl EffectOp
{
    /// Builds an operation `name : payload ↠ reply`.
    #[inline]
    #[must_use]
    pub fn new(
        name: OperationName<'_>,
        payload: ValueType,
        reply: ValueType,
    ) -> Self
    {
        Self {
            name: String::from(name.as_ref()),
            payload,
            reply,
        }
    }

    /// Return the operation name.
    #[inline]
    #[must_use]
    pub fn name(&self) -> OperationName<'_>
    {
        OperationName::from(self.name.as_str())
    }

    /// Return the operation payload type.
    #[inline]
    #[must_use]
    pub fn payload(&self) -> &ValueType
    {
        &self.payload
    }

    /// Return the operation reply type.
    #[inline]
    #[must_use]
    pub fn reply(&self) -> &ValueType
    {
        &self.reply
    }
}

/// An effect signature `E = { opᵢ : Aᵢ ↠ Bᵢ }` (`effects-control-shell.md`
/// §1.1): a named interface grouping the operations a single effect provides.
///
/// The signature is the unit a row carries and a handler discharges. It is
/// identified by its [`Self::name`]; the operation list is kept in a stable
/// order so structural equality of two equal signatures holds regardless of how
/// they were assembled.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct EffectSig
{
    /// The signature's name (the effect's name, e.g. `State`).
    name: String,
    /// The operations the effect provides, ordered by [`EffectOp::name`].
    ops: Vec<EffectOp>,
}

impl EffectSig
{
    /// Builds a signature `name` over `ops`, normalizing the operation list to
    /// a stable order (sorted by [`EffectOp::name`]) so two signatures with the
    /// same operations compare equal regardless of insertion order.
    #[inline]
    #[must_use]
    pub fn new(
        name: EffectSignatureName<'_>,
        mut ops: Vec<EffectOp>,
    ) -> Self
    {
        ops.sort_by(|lhs, rhs| lhs.name.cmp(&rhs.name));
        Self {
            name: String::from(name.as_ref()),
            ops,
        }
    }

    /// Return the signature name.
    #[inline]
    #[must_use]
    pub fn name(&self) -> EffectSignatureName<'_>
    {
        EffectSignatureName::from(self.name.as_str())
    }

    /// Return the canonical operation list.
    #[inline]
    #[must_use]
    pub fn ops(&self) -> &[EffectOp]
    {
        &self.ops
    }

    /// Looks up an operation by name (the resolution `perform` performs against
    /// the handled signature).
    ///
    /// # Contract
    /// - ensures: returns the operation named `op` if the signature provides
    ///   it, else `None`.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn op(
        &self,
        op: OperationName<'_>,
    ) -> Option<&EffectOp>
    {
        self.ops
            .iter()
            .find(|candidate| candidate.name == op.as_ref())
    }
}

/// An effect row `ε` — a concrete finite set of [`EffectSig`]s (ADR-33 D2).
///
/// The carrier is **representation-sealed**: it is a newtype over the
/// crate-private [`Repr`], so the only way another module observes or builds a
/// row is through the set signature ([`Self::EMPTY`], [`Self::singleton`],
/// [`Self::union`], [`Self::without`], [`Self::contains`], [`Self::is_subset`],
/// [`Self::is_empty`], [`Self::signatures`]) plus the forwarding-free [`Debug`]
/// impl below. The pure returner `F A ≡ F^⟨⟩ A` is exactly the [`Self::EMPTY`]
/// case, and [`crate::types::CompType`]'s `Debug` renders it with no row so the
/// spelling stays `F(payload)` (ADR-33 D1).
#[derive(Clone, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct EffectRow(Repr);

/// The sealed representation of an [`EffectRow`]: the signatures keyed by name.
///
/// A `BTreeMap` gives a canonical (name-ordered) form, so two rows built by
/// different union orders compare equal, and dedups by name (the one-name-one-
/// signature invariant). Crate-private *and* module-private, so the row's whole
/// cross-module surface is [`EffectRow`]'s set signature.
type Repr = BTreeMap<String, EffectSig>;

impl core::fmt::Debug for EffectRow
{
    /// Renders the row as `⟨⟩` when empty and `⟨name₁, name₂, …⟩` otherwise
    /// (the signature names, in canonical order). Diagnostics render rows
    /// through this `Debug`; there is deliberately no `Display`, matching
    /// the grade carrier (`core-ir-contract.md` §4).
    #[inline]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        f.write_str("⟨")?;
        for (index, name) in self.0.keys().enumerate() {
            if index != 0 {
                f.write_str(", ")?;
            }
            f.write_str(name)?;
        }
        f.write_str("⟩")
    }
}

impl EffectRow
{
    /// The empty row `⟨⟩` — the pure returner's effects (`F A ≡ F^⟨⟩ A`).
    pub const EMPTY: Self = Self(BTreeMap::new());

    /// The singleton row `⟨E⟩` containing exactly the signature `sig`.
    #[inline]
    #[must_use]
    pub fn singleton(sig: EffectSig) -> Self
    {
        let mut repr = Repr::new();
        repr.insert(sig.name().as_ref().to_owned(), sig);
        Self(repr)
    }

    /// Whether this row is empty (`ε = ⟨⟩`).
    ///
    /// # Contract
    /// - ensures: returns `true` iff the row carries no signatures — i.e. it is
    ///   the pure returner's row.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> EffectRowEmptyStatus
    {
        self.0.is_empty().into()
    }

    /// The union `ε ∪ ε′` — the accumulated demand of sequencing (bottom-up row
    /// inference: a sequence may perform whatever either side performs).
    ///
    /// # Contract
    /// - ensures: returns the row containing every signature of `self` and of
    ///   `other`, keyed by name; on a name collision `other`'s signature is
    ///   kept (the one-name-one-signature invariant means the two agree, so the
    ///   choice is immaterial).
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn union(
        &self,
        other: &Self,
    ) -> Self
    {
        let mut repr = self.0.clone();
        for (name, sig) in &other.0 {
            repr.insert(name.clone(), sig.clone());
        }
        Self(repr)
    }

    /// The difference `ε ∖ ⟨E⟩` — the residual row a handler leaves after
    /// discharging the signature named `sig_name`.
    ///
    /// # Contract
    /// - ensures: returns the row with the signature named `sig_name` removed
    ///   if present, unchanged otherwise.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn without(
        &self,
        sig_name: EffectSignatureName<'_>,
    ) -> Self
    {
        let mut repr = self.0.clone();
        repr.remove(sig_name.as_ref());
        Self(repr)
    }

    /// Whether the row contains a signature named `sig_name`.
    ///
    /// # Contract
    /// - ensures: returns `true` iff a signature with that name is present.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn contains(
        &self,
        sig_name: EffectSignatureName<'_>,
    ) -> EffectContains
    {
        self.0.contains_key(sig_name.as_ref()).into()
    }

    /// The row-subset order `ε ⊆ ε′` — the row leg of `F^ε` subtyping.
    ///
    /// # Contract
    /// - ensures: returns `true` iff every signature of `self` is present in
    ///   `other` with an equal signature — so a computation whose effects are
    ///   `self` may be used where `other`'s effects are allowed. Reflexive and
    ///   transitive; `⟨⟩` is the bottom.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn is_subset(
        &self,
        other: &Self,
    ) -> EffectSubsetDecision
    {
        self.0
            .iter()
            .all(|(name, sig)| other.0.get(name) == Some(sig))
            .into()
    }

    /// The signatures of the row, in canonical (name-ordered) order.
    ///
    /// Read-only inspection for consumers that render or lower a row (the
    /// pipeline's diagnostics, the surface lowerer).
    #[inline]
    pub fn signatures(&self) -> impl Iterator<Item = &EffectSig>
    {
        self.0.values()
    }
}

/// The bottom-up row union at a `bind` (`effects-control-shell.md` §1.2; A3.2
/// `+effects`): `t >>= x. u` accumulates the demand of both sides, so the bound
/// computation's effect row `bound_row` folds into the continuation's result.
///
/// The continuation type `cont_ty` is the bind's result type:
///
/// - a returner continuation `F^{ε_u} C` becomes `F^{ε_t ∪ ε_u} C` (the
///   sequence may perform whatever either side performs);
/// - an [`CompType::Unknown`] continuation absorbs the row (the gradual hole
///   stands for any row, A2.2);
/// - a **non-returner** continuation (`A → B`, `B & B′`) has no `F` to carry
///   the row. With an *empty* `bound_row` this is the pure `bind` of the
///   pre-`+effects` core — the continuation type is returned unchanged, so all
///   prior behaviour is preserved byte-for-byte; with a *non-empty* `bound_row`
///   the effects would be silently lost, which v0 rejects (effects live only on
///   `F` until a later stage carries them on negative types).
///
/// Shared by the recursive checker and the typing machine (as
/// [`crate::subtype::finish_comp`]) so the two produce *equal* results.
///
/// # Contract
/// - ensures: returns `F^{bound_row ∪ ε_u} C` for a returner `cont_ty = F^{ε_u}
///   C`; `Unknown` for an `Unknown` continuation; `cont_ty` unchanged for a
///   non-returner continuation when `bound_row` is empty.
/// - fails: `ShapeMismatch { expected: SHAPE_RETURNER, actual: cont_ty }` for a
///   non-returner continuation when `bound_row` is non-empty.
/// - panics: none.
///
/// # Errors
///
/// Returns [`TypeError::ShapeMismatch`] when an effectful bound computation is
/// sequenced into a non-returner continuation.
#[inline]
pub(crate) fn combine_bind_row(
    bound_row: &EffectRow,
    cont_ty: CompType,
) -> Result<CompType, TypeError>
{
    match cont_ty {
        | CompType::F(payload, cont_row) => Ok(CompType::F(payload, bound_row.union(&cont_row))),
        // The gradual hole absorbs the accumulated row (A2.2).
        | CompType::Unknown => Ok(CompType::Unknown),
        // A pure bound computation sequences into any continuation unchanged
        // (the pre-`+effects` behaviour); an effectful one cannot, as a negative
        // continuation type has no `F` to carry the row in v0.
        | other if bool::from(bound_row.is_empty()) => Ok(other),
        | other => Err(TypeError::ShapeMismatch {
            expected: text::SHAPE_RETURNER,
            actual: Ty::Comp(other),
        }),
    }
}

/// The resumption (continuation) type `k : Stk(F^ε B_op, F^ε C)` a handler
/// clause binds (`effects-control-shell.md` §1.1 rule Handle; A3.2 `+effects`).
///
/// `answer` is the handler's answer computation type `F^ε C` (or `Unknown` for
/// the matched-hole answer); `reply` is the handled operation's reply type
/// `B_op`. The captured stack consumes a `B_op`-returner at the answer's row
/// `ε` (`⟨⟩` for a hole answer) and *delivers the same answer* — the **deep**
/// discipline (ADR-33 D4): the handler reinstalls itself in `k`.
///
/// Shared by the recursive checker and the typing machine so the continuation
/// each binds is identical (the step-for-step contract).
#[inline]
#[must_use]
pub(crate) fn resume_stack_type(
    answer: &CompType,
    reply: ValueType,
) -> ValueType
{
    let cont_row = match *answer {
        | CompType::F(_, ref eps) => eps.clone(),
        | _ => EffectRow::EMPTY,
    };
    ValueType::stk(CompType::returner_eff(reply, cont_row), answer.clone())
}

/// The handler's **natural type** before subsumption
/// (`effects-control-shell.md` §1.1 rule Handle; A3.2 `+effects`): the answer
/// payload `C` carried at the residual row `ε_t ∖ E` the handled computation
/// may leak.
///
/// Finishing it against the answer `F^ε C` (via
/// [`crate::subtype::finish_comp`]) makes the inlined Sub rule's row leg `ε_t ∖
/// E ⊆ ε` the soundness check — the residual `t` may perform unhandled must fit
/// the answer's row. A matched-hole answer yields `Unknown`, which absorbs any
/// residual. Shared by the checker and the machine.
#[inline]
#[must_use]
pub(crate) fn handle_natural_type(
    answer: &CompType,
    residual: EffectRow,
) -> CompType
{
    match *answer {
        | CompType::F(ref c, _) => CompType::returner_eff(c.as_ref().clone(), residual),
        | _ => CompType::Unknown,
    }
}

/// Resolves a deep handler's operation clauses against its signature
/// (`effects-control-shell.md` §1.1 rule Handle: a clause for *each* `opᵢ ∈ E`;
/// ADR-33 D4; A3.2 `+effects`).
///
/// Returns the clauses paired with their resolved [`EffectOp`]s **in the
/// signature's canonical operation order**, so the recursive checker and the
/// typing machine process the clauses identically (the step-for-step contract).
/// Returns `None` when coverage is not exact — a clause names an operation
/// outside the signature, an operation of the signature has no clause, or two
/// clauses name the same operation. (Equal clause/operation counts plus a
/// clause for every operation force a bijection, so an out-of-signature clause
/// is caught by an operation being left without one.)
///
/// # Contract
/// - ensures: `Some(pairs)` with `pairs.len() == sig.ops.len()`, ordered by
///   [`EffectSig::ops`], iff the clause set is exactly the signature's
///   operations; `None` otherwise.
/// - panics: none.
pub(crate) fn resolve_handler_coverage(
    sig: &EffectSig,
    ops: &[OpClause],
) -> Option<Vec<(EffectOp, OpClause)>>
{
    if ops.len() != sig.ops.len() {
        return None;
    }
    let mut resolved = Vec::with_capacity(sig.ops.len());
    for op_def in &sig.ops {
        let mut found: Option<&OpClause> = None;
        for clause in ops {
            if clause.op == op_def.name {
                if found.is_some() {
                    // A second clause for one operation: coverage is not a
                    // bijection.
                    return None;
                }
                found = Some(clause);
            }
        }
        // No clause for this operation (and, with equal counts, the symmetric
        // out-of-signature clause it would pair with).
        let clause = found?;
        resolved.push((op_def.clone(), clause.clone()));
    }
    Some(resolved)
}

/// Set-law and bottom-up-arithmetic tests for the concrete row carrier.
#[cfg(test)]
mod tests
{
    use super::*;
    /// `EMPTY` is empty and a subset of every row; non-empty rows are not
    /// empty.
    #[test]
    fn empty_is_the_bottom()
    {
        let state = EffectRow::singleton(sig(EffectSignatureName::from("State")));
        assert!(
            bool::from(EffectRow::EMPTY.is_empty()),
            "EMPTY must be empty"
        );
        assert!(
            !bool::from(state.is_empty()),
            "a singleton row must not be empty"
        );
        assert!(
            bool::from(EffectRow::EMPTY.is_subset(&state)),
            "⟨⟩ must be a subset of every row"
        );
        assert!(
            bool::from(EffectRow::EMPTY.is_subset(&EffectRow::EMPTY)),
            "⟨⟩ ⊆ ⟨⟩"
        );
    }

    /// `is_subset` is reflexive and transitive, and respects union (the row leg
    /// of `F^ε` subtyping).
    #[test]
    fn subset_is_a_preorder_with_union_as_join()
    {
        let s = EffectRow::singleton(sig(EffectSignatureName::from("State")));
        let i = EffectRow::singleton(sig(EffectSignatureName::from("IO")));
        let si = s.union(&i);
        assert!(bool::from(s.is_subset(&s)), "⊆ must be reflexive");
        assert!(bool::from(s.is_subset(&si)), "ε ⊆ ε ∪ ε′");
        assert!(bool::from(i.is_subset(&si)), "ε′ ⊆ ε ∪ ε′");
        assert!(
            !bool::from(si.is_subset(&s)),
            "a larger row is not a subset of a smaller"
        );
        // transitivity through the empty bottom
        assert!(
            bool::from(EffectRow::EMPTY.is_subset(&s))
                && bool::from(s.is_subset(&si))
                && bool::from(EffectRow::EMPTY.is_subset(&si)),
            "⊆ must be transitive"
        );
    }

    /// `union` is commutative and idempotent (a canonical set), and `without`
    /// inverts a singleton union.
    #[test]
    fn union_is_a_canonical_set_and_without_subtracts()
    {
        let s = EffectRow::singleton(sig(EffectSignatureName::from("State")));
        let i = EffectRow::singleton(sig(EffectSignatureName::from("IO")));
        assert_eq!(s.union(&i), i.union(&s), "∪ must be commutative");
        assert_eq!(s.union(&s), s, "∪ must be idempotent");
        assert!(
            bool::from(s.union(&i).contains(EffectSignatureName::from("State")))
                && bool::from(s.union(&i).contains(EffectSignatureName::from("IO")))
        );
        assert_eq!(
            s.union(&i).without(EffectSignatureName::from("IO")),
            s,
            "without must subtract the named signature"
        );
        assert_eq!(
            s.without(EffectSignatureName::from("Absent")),
            s,
            "without a missing signature is the identity"
        );
    }

    /// A nullary-payload signature named `name` with one operation `name`
    /// taking `1` and replying `1` — enough to exercise the set algebra without
    /// depending on the rest of the type system.
    fn sig(name: EffectSignatureName<'_>) -> EffectSig
    {
        EffectSig::new(name, alloc::vec![EffectOp::new(
            OperationName::from(name.as_ref()),
            ValueType::Unit,
            ValueType::Unit
        )])
    }

    /// The signature lookup resolves operations by name.
    #[test]
    fn signature_resolves_operations_by_name()
    {
        let state = EffectSig::new(EffectSignatureName::from("State"), alloc::vec![
            EffectOp::new(
                OperationName::from("get"),
                ValueType::Unit,
                ValueType::integer()
            ),
            EffectOp::new(
                OperationName::from("put"),
                ValueType::integer(),
                ValueType::Unit
            ),
        ]);
        assert_eq!(
            state
                .op(OperationName::from("get"))
                .map(|o| o.reply.clone()),
            Some(ValueType::integer())
        );
        assert_eq!(
            state
                .op(OperationName::from("put"))
                .map(|o| o.payload.clone()),
            Some(ValueType::integer())
        );
        assert!(
            state.op(OperationName::from("absent")).is_none(),
            "an absent op resolves to None"
        );
    }
}
