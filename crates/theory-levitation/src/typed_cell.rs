//! Levitation **stage 1**: *typed* 2-cell faces (ADR-81; addendum §A/§4.1).
//!
//! Stage 0's [`CellFace`] stores a rewrite `lhs ~> rhs` as an untyped pair of
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
//! Everything here is **additive** — the stage-0 [`CellFace`] and
//! [`CellVarMeta`](crate::cell::CellVarMeta) are reused whole, never rewritten
//! (a sibling lane owns `cell.rs`). A typed face *wraps* a stage-0 face;
//! nothing about the stage-0 encoding changes.

use gandr_core_checker::boundary::NameRef;
use gandr_core_checker::types::ValueType;

use crate::boundary::ContextTotality;
use crate::cell::CellFace;
use crate::code::Code;
use crate::code::Name;
use crate::decode::DecodeError;
use crate::decode::decode;

/// A **signature context** for a cell face (addendum §4.1): each pattern
/// variable paired with the core value type it ranges over — the Σ head of the
/// typed face.
///
/// The types are obtained by **decoding** each variable's field code (stage-1
/// large elimination), so the context is a genuine bridge from the description
/// layer into the core type universe.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct SignatureContext
{
    /// The pattern variables and their decoded core value types, in declaration
    /// order.
    pub vars: Box<[(Name, ValueType)]>,
}

impl SignatureContext
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
    /// `self_ty` decodes any recursive occurrence ([`Code::Var`]) — the carrier
    /// of the type being defined.
    ///
    /// # Contract
    /// - requires: `bindings` pairs each pattern variable with the first-order
    ///   field code it fills.
    /// - ensures: returns a context binding each variable to `decode(code,
    ///   self_ty)`, in the given order.
    /// - fails: the first [`DecodeError`] a field code raises (a deferred
    ///   atom-abstraction or applied-named-type leaf).
    /// - panics: never.
    ///
    /// # Errors
    /// Returns [`DecodeError`] when a field code lies outside the stage-1
    /// decode fragment (see [`crate::decode`](mod@crate::decode)).
    #[inline]
    pub fn from_field_codes(
        bindings: &[(Name, Code)],
        self_ty: &ValueType,
    ) -> Result<Self, DecodeError>
    {
        let mut vars = Vec::with_capacity(bindings.len());
        for binding in bindings {
            let decoded = decode(&binding.1, self_ty)?;
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

/// A **typed 2-cell face** (stage 1; addendum §4.1) — a stage-0 [`CellFace`]
/// refined with a decoded [`SignatureContext`].
///
/// The "protype" of the reflected judgment layer; a typed rule is its
/// "proterm". **Additive**: the stage-0 [`CellFace`] is reused whole (the
/// untyped `D⋆` term pair, its derived [`crate::CellVarMeta`], and its
/// provenance), never rewritten.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct TypedCellFace
{
    /// The stage-0 face (the untyped `D⋆` term pair, reused whole).
    pub face: CellFace,
    /// The decoded signature context typing the face's pattern variables.
    pub context: SignatureContext,
}

impl TypedCellFace
{
    /// A typed face from a stage-0 face and its decoded signature context.
    #[inline]
    #[must_use]
    pub fn new(
        face: CellFace,
        context: SignatureContext,
    ) -> Self
    {
        Self { face, context }
    }

    /// Whether **every** pattern variable the stage-0 face declares (its
    /// derived [`crate::CellVarMeta`] list) is typed by the context — the
    /// stage-1 well-typedness link (each `D⋆` pattern variable is decoded to a
    /// core type).
    ///
    /// # Contract
    /// - ensures: `true` iff every `self.face.vars[i].var` has a
    ///   [`SignatureContext::type_of`] entry.
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
    use gandr_core_checker::grade::Grade;

    use super::*;
    use crate::cell::CellVarMeta;
    use crate::cell::FreeTerm;
    use crate::cell::Variance;
    use crate::code::Attrs;
    use crate::code::PrimTy;
    use crate::code::ValueTypeRef;
    use crate::desc::SurfaceSpan;

    #[test]
    fn signature_context_decodes_field_codes()
    {
        let ctx = SignatureContext::from_field_codes(
            &[("x".into(), integer_field()), ("y".into(), Code::Unit)],
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
        // The stage-0 face `f(x) ~> x` declares the pattern variable `x`.
        let face = CellFace::new(
            FreeTerm::op("f", [FreeTerm::var("x")]),
            FreeTerm::var("x"),
            [CellVarMeta::new("x", Variance::Producer, true.into())],
            SurfaceSpan::new(0.into(), 4.into()),
        );
        let typed = TypedCellFace::new(
            face.clone(),
            SignatureContext::from_field_codes(
                &[("x".into(), integer_field())],
                &ValueType::atom("Self"),
            )
            .expect("decodes"),
        );
        assert!(
            bool::from(typed.is_context_total()),
            "every declared pattern variable is typed"
        );

        // A context missing `x` is not total.
        let untyped = TypedCellFace::new(face, SignatureContext::new(Vec::new()));
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
        let bind = Code::bind(crate::code::AtomSort::named("a"), Code::Var);
        let result =
            SignatureContext::from_field_codes(&[("x".into(), bind)], &ValueType::atom("Self"));
        assert_eq!(
            Err(DecodeError::AtomAbstraction),
            result,
            "a deferred field code fails the context build"
        );
    }
}
