//! Levitation **stage 1**: *typed* 2-cell faces (the levitation design's
//! stage ladder and its VDC addendum, §A/§4.1).
//!
//! Stage 0's [`RuleFace`] stores a rewrite `lhs ==> rhs` as an untyped pair of
//! [`crate::FreeTerm`]s with host-side well-formedness (`crate::check_desc`).
//! The addendum sharpens the stage-1 refinement: the typed cell face is a
//! **protype** of the reflected judgment layer, and a typed rule is a
//! **proterm** — the checking that "moves from Rust into the checker" moves,
//! precisely, into a bidirectional checker over the *decoded signature*
//! (proposal §4.1; addendum §4.1). Concretely, a typed face is the stage-0 face
//! together with a **signature context**: each pattern variable paired with the
//! core [`ValueType`] it ranges over — the **decoded** type of the field the
//! variable fills (stage-1 large elimination, [`crate::decode()`]). This is the
//! "dependent Σ over the signature" V3 named: the signature context is the Σ
//! head, the two `D⋆` terms the tail.
//!
//! Everything here is **additive** — the stage-0 [`RuleFace`] and
//! [`RuleVarMeta`](crate::rule::RuleVarMeta) are reused whole, never rewritten
//! (a sibling lane owns `cell.rs`). A typed face *wraps* a stage-0 face;
//! nothing about the stage-0 encoding changes.

use gandr_core_checker::discipline::boundary::NameRef;
use gandr_core_checker::term::types::ValueType;

use crate::boundary::ContextTotality;
use crate::code::Code;
use crate::code::Name;
use crate::decode::DecodeError;
use crate::decode::decode;
use crate::rule::RuleFace;

/// A **signature context** for a cell face (addendum §4.1): each pattern
/// variable paired with the core value type it ranges over — the Σ head of the
/// typed face.
///
/// The types are obtained by **decoding** each variable's field code (stage-1
/// large elimination), so the context is a genuine bridge from the description
/// layer into the core type universe.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct PatternContext
{
    /// The pattern variables and their decoded core value types, in declaration
    /// order.
    pub vars: Box<[(Name, ValueType)]>,
}

impl PatternContext
{
    /// A context from explicit `{variable, decoded type}` bindings.
    #[inline]
    #[must_use]
    pub fn new<V>(vars: V) -> Self
    where
        V: Into<Box<[(Name, ValueType)]>>,
    {
        Self { vars: vars.into() }
    }

    /// Builds a signature context by **decoding** each pattern variable's field
    /// code into its core value type (stage-1 large elimination,
    /// [`crate::decode()`]).
    ///
    /// `self_sort` names the enclosing description's own sort, and `self_ty`
    /// decodes a recursive occurrence ([`Code::Var`]) at that sort — the
    /// carrier of the type being defined.
    ///
    /// # Contract
    /// - requires: `bindings` pairs each pattern variable with the first-order
    ///   field code it fills.
    /// - ensures: returns a context binding each variable to `decode(code,
    ///   self_sort, self_ty)`, in the given order.
    /// - fails: the first [`DecodeError`] a field code raises (a deferred
    ///   atom-abstraction or applied-named-type leaf, or a `var` at a foreign
    ///   sort).
    /// - panics: never.
    ///
    /// # Errors
    /// Returns [`DecodeError`] when a field code lies outside the stage-1
    /// decode fragment (see [`crate::decode`](mod@crate::decode)).
    #[inline]
    pub fn from_field_codes(
        bindings: &[(Name, Code)],
        self_sort: NameRef<'_>,
        self_ty: &ValueType,
    ) -> Result<Self, DecodeError>
    {
        let mut vars = Vec::with_capacity(bindings.len());
        for binding in bindings {
            let decoded = decode(&binding.1, self_sort, self_ty)?;
            vars.push((binding.0.clone(), decoded));
        }
        Ok(Self { vars: vars.into() })
    }

    /// The decoded core value type bound to `var`, or [`None`] when the
    /// variable is not in the context.
    #[inline]
    #[must_use]
    pub fn type_of(
        &self,
        var: NameRef<'_>,
    ) -> Option<&ValueType>
    {
        self.vars.iter().find_map(|entry| {
            let (ref entry_name, ref ty) = *entry;
            (entry_name.as_ref() == var.as_ref()).then_some(ty)
        })
    }
}

/// A **typed 2-cell face** (stage 1; addendum §4.1) — a stage-0 [`RuleFace`]
/// refined with a decoded [`PatternContext`].
///
/// The "protype" of the reflected judgment layer; a typed rule is its
/// "proterm". **Additive**: the stage-0 [`RuleFace`] is reused whole (the
/// untyped `D⋆` term pair, its derived [`crate::RuleVarMeta`], and its
/// provenance), never rewritten.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct TypedRuleFace
{
    /// The stage-0 face (the untyped `D⋆` term pair, reused whole).
    pub face: RuleFace,
    /// The decoded signature context typing the face's pattern variables.
    pub context: PatternContext,
}

impl TypedRuleFace
{
    /// A typed face from a stage-0 face and its decoded signature context.
    #[inline]
    #[must_use]
    pub fn new(
        face: RuleFace,
        context: PatternContext,
    ) -> Self
    {
        Self { face, context }
    }

    /// Whether **every** pattern variable the stage-0 face declares (its
    /// derived [`crate::RuleVarMeta`] list) is typed by the context — the
    /// stage-1 well-typedness link (each `D⋆` pattern variable is decoded to a
    /// core type).
    ///
    /// # Contract
    /// - ensures: `true` iff every `self.face.vars[i].var` has a
    ///   [`PatternContext::type_of`] entry.
    /// - panics: never.
    #[inline]
    #[must_use]
    pub fn is_context_total(&self) -> ContextTotality
    {
        self.face
            .vars
            .iter()
            .all(|meta| self.context.type_of(meta.var.as_ref().into()).is_some())
            .into()
    }
}

#[cfg(test)]
mod tests
{
    use gandr_core_checker::discipline::grade::Grade;

    use super::*;
    use crate::code::Attrs;
    use crate::code::PrimTy;
    use crate::code::ValueTypeRef;
    use crate::desc::SurfaceSpan;
    use crate::rule::FreeTerm;
    use crate::rule::RuleVarMeta;
    use crate::rule::Variance;

    #[test]
    fn signature_context_decodes_field_codes()
    {
        let ctx = PatternContext::from_field_codes(
            &[("x".into(), integer_field()), ("y".into(), Code::Unit)],
            "Self".into(),
            &ValueType::atom("Self"),
        )
        .expect("first-order fields decode");
        assert_eq!(ctx.type_of("x".into()), Some(&ValueType::integer()));
        assert_eq!(Some(&ValueType::Unit), ctx.type_of("y".into()));
        assert_eq!(
            None,
            ctx.type_of("z".into()),
            "an unbound variable has no type"
        );
    }
    #[test]
    fn typed_face_context_totality_tracks_declared_variables()
    {
        // The stage-0 face `f(x) ==> x` declares the pattern variable `x`.
        let face = RuleFace::new(
            FreeTerm::op("f", [FreeTerm::var("x")]),
            FreeTerm::var("x"),
            [RuleVarMeta::new("x", Variance::Producer, true.into())],
            SurfaceSpan::new(0.into(), 4.into()),
        );
        let typed = TypedRuleFace::new(
            face.clone(),
            PatternContext::from_field_codes(
                &[("x".into(), integer_field())],
                "Self".into(),
                &ValueType::atom("Self"),
            )
            .expect("decodes"),
        );
        assert!(
            bool::from(typed.is_context_total()),
            "every declared pattern variable is typed"
        );

        // A context missing `x` is not total.
        let untyped = TypedRuleFace::new(face, PatternContext::new(Vec::new()));
        assert!(
            !bool::from(untyped.is_context_total()),
            "a variable with no decoded type breaks totality"
        );
    }
    fn integer_field() -> Code
    {
        Code::field(
            ValueTypeRef::Prim(PrimTy::Integer),
            Grade::ONE,
            Attrs::empty(),
        )
    }

    #[test]
    fn signature_context_propagates_decode_failure()
    {
        let bind = Code::bind(crate::code::AtomSort::named("a"), Code::var("Self"));
        let result = PatternContext::from_field_codes(
            &[("x".into(), bind)],
            "Self".into(),
            &ValueType::atom("Self"),
        );
        assert_eq!(
            Err(DecodeError::AtomAbstraction),
            result,
            "a deferred field code fails the context build"
        );
    }
}
