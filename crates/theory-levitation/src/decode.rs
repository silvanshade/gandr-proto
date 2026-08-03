//! Levitation **stage 1**: large elimination — the decoder
//! `decode : Desc → ValueType` (ADR-67 D7 feature 3; ADR-81;
//! `proposal-levitation.md` §6).
//!
//! Stage 0 keeps `decode` host-side as unwritten meta-theory; **stage 1 writes
//! it**: [`decode`] interprets a first-order [`Code`] into a core
//! [`gandr_core_checker::types::ValueType`], and [`decode_desc`] interprets a
//! whole tagged [`SignDesc`] (the μ decoder, [`DeclPolarity::Data`]) into the
//! coproduct over its constructors. This is the single non-negotiable
//! *dependent* capability the simply-typed checker lacked — a function *from
//! data into the universe of types* — realized here as a total host function
//! over the decidable first-order fragment `{1, var, ×, σ}` plus the V5
//! [`Code::Field`] leaf. The decoded types are elements of the code universe
//! [`gandr_core_checker::types::ValueType::Universe`] (feature 1) and use the
//! dependent pair [`gandr_core_checker::types::ValueType::Sigma`] (feature 2)
//! nowhere yet — the current fragment is non-dependent, so it decodes into the
//! non-dependent positive core (`1`, `×`, `+`, atoms); `Σ` is the stage-1
//! capability that becomes `decode`'s target when a genuinely dependent σ
//! *code* (a payload type depending on a tag value) lands with the
//! codes-as-gandr-data step (stage 2). See the ADR-81 consequences.
//!
//! **Decidable equality is preserved** (proposal §3): [`decode`] is a total
//! *function* over the decidable-`Eq` [`Code`] fragment into the
//! structurally-`Eq` [`gandr_core_checker::types::ValueType`], so
//! structurally-equal codes decode to structurally-equal types — the property
//! content-addressing and matching-modulo depend on.
//!
//! **Erasure (V5)**: the value decoder *erases* the [`Code::Field`] grade and
//! attribute decorations (they are read by the separate grade / attribute
//! decoders, proposal §4.3); `decode` reads only the field's core value type.
//!
//! **The `var` carrier**: a recursive occurrence [`Code::Var`] names its
//! target sort (the description universe's sort index), and it decodes to the
//! caller-supplied `self_ty` — the value type standing for "the type being
//! defined" — exactly when it names the decoded description's own sort. A
//! `var` at any other sort is **outside the single-sort decode fragment**
//! ([`DecodeError::ForeignSort`]): cross-sort recursion is representable in
//! the sort-indexed universe but not decodable here. Full recursive `μ⁺`
//! unrolling is the deferred value-model rung (proposal §7: recursive
//! declared data decodes *through* the `μ⁺` rung); until it lands, `decode`
//! interprets a self-sort `var` as the provided carrier (an opaque atom at
//! the datatype's use site), keeping the fragment first-order and total.

use gandr_core_checker::boundary::NameRef;
use gandr_core_checker::types::ValueType;

use crate::code::Code;
use crate::code::Name;
use crate::code::PrimTy;
use crate::code::ValueTypeRef;
use crate::desc::DeclPolarity;
use crate::desc::SignDesc;

/// Why a first-order code (or a whole description) cannot be decoded into a
/// core value type — the honest boundary of the stage-1 decode fragment.
///
/// A [`Code`] *cannot represent* a higher-order (function-typed) field, so that
/// case does not appear here (it is excluded from the fragment, not declined at
/// runtime — proposal §3); what remains are the three genuinely-deferred cases.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum DecodeError
{
    /// A `bind ⟨a⟩T` atom-abstraction field (V5, [`Code::Bind`]): interpreting
    /// it into an atom-abstraction functor over `gandr-nominal` is a later lane
    /// (proposal §4.3), outside the stage-1 decode fragment.
    AtomAbstraction,
    /// An **applied** named type (`Vec(a)` — a [`ValueTypeRef::Ctor`] with
    /// arguments): the frozen core has no generic type application for
    /// arbitrary heads — a declared type enters as a minted
    /// [`gandr_core_checker::types::ValueType::Data`] handle at the *pipeline*
    /// seam, which `gandr-theory-levitation` cannot mint (it holds no
    /// [`gandr_core_checker::types::DataId`] allocator) — so an applied
    /// reference is deferred to that seam. Carries the offending head name
    /// for the diagnostic.
    AppliedType(Name),
    /// A `codata` description: its ν decoder targets the *negative computation*
    /// universe (observations), a later lane (proposal §5). The μ
    /// [`decode_desc`] decodes only [`DeclPolarity::Data`].
    CodataNu,
    /// A description with **no constructors** (an uninhabited datatype): the
    /// frozen core carries no `Void` value type, so [`decode_desc`] declines it
    /// rather than manufacture an empty coproduct.
    Uninhabited,
    /// A recursive occurrence [`Code::Var`] naming a sort **other than** the
    /// decoded description's own: cross-sort recursion lies outside the
    /// single-sort stage-1 decode fragment. Carries the foreign sort's name
    /// for the diagnostic.
    ForeignSort(Name),
    /// A description declaring **several sorts**: the stage-1 μ decoder reads
    /// the single-sort fragment only (a multi-sorted signature decodes
    /// per-sort through a later, genuinely indexed decoder).
    MultiSorted,
}

/// Decodes a whole tagged [`SignDesc`] (the μ decoder) into the coproduct over
/// its constructors — the stage-1 reading of the σ tag as a finite choice
/// (ADR-81 feature 3; proposal §3, §5).
///
/// The `n` constructors decode to the right-nested coproduct
/// `P₀ + (P₁ + … + Pₙ₋₁)` of their per-constructor payload decodings — the
/// finite-tag reading of `Σ (t : Fin n). Pₜ`, non-dependent because the current
/// per-constructor codes are non-dependent (a payload type does not depend on
/// the tag value). The genuinely dependent `Σ`-over-tag packaging (feature 2's
/// target) waits on a dependent σ code and the codes-as-gandr-data step (ADR-81
/// consequences). `self_ty` decodes each constructor's [`Code::Var`]
/// occurrences (proposal §7's `μ⁺` availability).
///
/// # Contract
/// - requires: `desc` is a [`DeclPolarity::Data`] description with at least one
///   constructor.
/// - ensures: returns `P₀` for one constructor, and the right-nested coproduct
///   `P₀ + (P₁ + …)` for several, where `Pᵢ = decode(ctorᵢ.code, self_ty)`.
/// - fails: [`DecodeError::CodataNu`] for a `codata` polarity (ν decoder is a
///   later lane); [`DecodeError::MultiSorted`] for a description declaring
///   several sorts (the μ decoder reads the single-sort fragment only);
///   [`DecodeError::Uninhabited`] for zero constructors; any [`decode`] failure
///   of a constructor's code.
/// - panics: never.
///
/// # Adequacy
/// - hypothesis: L2 — `decode_desc` folds the per-constructor decodings into a
///   deterministic right-nested coproduct; the tag arity and per-constructor
///   codes are both observable in the result (adding, removing, or reordering a
///   constructor changes the decoded coproduct).
/// - witness: `decode::tests::decode_desc_is_the_constructor_coproduct`,
///   `decode::tests::decode_desc_declines_codata_and_the_uninhabited`.
///
/// # Errors
/// Returns [`DecodeError`] — see the variants and the contract.
#[inline]
pub fn decode_desc(
    desc: &SignDesc,
    self_ty: &ValueType,
) -> Result<ValueType, DecodeError>
{
    if matches!(desc.polarity, DeclPolarity::Codata) {
        return Err(DecodeError::CodataNu);
    }
    let self_sort = match *desc.sorts {
        | [ref sort] => sort.name.clone(),
        | _ => return Err(DecodeError::MultiSorted),
    };
    let mut summands: Vec<ValueType> = Vec::with_capacity(desc.ctors.len());
    for ctor in &desc.ctors {
        let summand = decode(&ctor.code, NameRef::from(self_sort.as_ref()), self_ty)?;
        summands.push(summand);
    }
    let Some(last) = summands.pop()
    else {
        return Err(DecodeError::Uninhabited);
    };
    Ok(summands
        .into_iter()
        .rev()
        .fold(last, |acc, summand| ValueType::sum(summand, acc)))
}
/// Decodes a first-order [`Code`] into a core value type — the stage-1 large
/// elimination (ADR-81 feature 3).
///
/// `self_sort` names the decoded description's own sort and `self_ty` is what
/// a recursive occurrence [`Code::Var`] at that sort decodes to (the carrier
/// of the type being defined); a `var` at any other sort is declined as
/// [`DecodeError::ForeignSort`]. The grade and attribute decorations on a
/// [`Code::Field`] are **erased** (V5).
///
/// # Contract
/// - requires: `code` is a first-order code (guaranteed by the [`Code`] type —
///   higher-order fields are unrepresentable).
/// - ensures: returns the value type the code describes — `1` for
///   [`Code::Unit`], `self_ty` for a [`Code::Var`] naming `self_sort`, `× / +`
///   for [`Code::Prod`] / [`Code::Sum`] recursing into children, and the
///   field's core value type for [`Code::Field`] (grade / attrs erased); the
///   result is deterministic, so structurally-equal codes yield
///   structurally-equal types.
/// - fails: [`DecodeError::AtomAbstraction`] for [`Code::Bind`];
///   [`DecodeError::AppliedType`] for an applied named field reference;
///   [`DecodeError::ForeignSort`] for a `var` outside `self_sort`.
/// - panics: never.
///
/// # Adequacy
/// - hypothesis: L2 — `decode` is a total, deterministic map on the decidable
///   first-order fragment: every representable non-`Bind`, non-applied code
///   decodes to a unique value type, and the map respects [`Code`]'s decidable
///   equality (a mutation that swaps `× ↔ +`, reorders factors, or changes a
///   leaf type is observable in the decoded type).
/// - witness: `decode::tests::decode_interprets_the_first_order_fragment`,
///   `decode::tests::decode_respects_code_equality`,
///   `decode::tests::decode_declines_the_deferred_leaves`.
///
/// # Errors
/// Returns [`DecodeError`] for the deferred leaves (atom-abstraction fields;
/// applied named types) — see the variants.
#[inline]
pub fn decode(
    code: &Code,
    self_sort: NameRef<'_>,
    self_ty: &ValueType,
) -> Result<ValueType, DecodeError>
{
    enum DecodeFrame<'code>
    {
        Decode(&'code Code),
        FinishProd,
        FinishSum,
    }

    let mut stack = vec![DecodeFrame::Decode(code)];
    let mut decoded = Vec::new();
    while let Some(frame) = stack.pop() {
        match frame {
            | DecodeFrame::Decode(node) => match *node {
                | Code::Unit => decoded.push(ValueType::Unit),
                | Code::Var(ref sort) => {
                    if sort.as_ref() == self_sort.as_ref() {
                        decoded.push(self_ty.clone());
                    }
                    else {
                        return Err(DecodeError::ForeignSort(sort.clone()));
                    }
                },
                | Code::Prod(ref left, ref right) => {
                    stack.push(DecodeFrame::FinishProd);
                    stack.push(DecodeFrame::Decode(right));
                    stack.push(DecodeFrame::Decode(left));
                },
                | Code::Sum(ref left, ref right) => {
                    stack.push(DecodeFrame::FinishSum);
                    stack.push(DecodeFrame::Decode(right));
                    stack.push(DecodeFrame::Decode(left));
                },
                // V5 erasure: the grade and attribute Σ are read by the grade / attribute
                // decoders, not the value decoder — `decode` reads only the field's type.
                | Code::Field(ref vref, ..) => {
                    let value_type = decode_ref(vref)?;
                    decoded.push(value_type);
                },
                | Code::Bind(..) => return Err(DecodeError::AtomAbstraction),
            },
            | DecodeFrame::FinishProd => {
                let right = decoded.pop().ok_or(DecodeError::Uninhabited)?;
                let left = decoded.pop().ok_or(DecodeError::Uninhabited)?;
                decoded.push(ValueType::prod(left, right));
            },
            | DecodeFrame::FinishSum => {
                let right = decoded.pop().ok_or(DecodeError::Uninhabited)?;
                let left = decoded.pop().ok_or(DecodeError::Uninhabited)?;
                decoded.push(ValueType::sum(left, right));
            },
        }
    }
    decoded.pop().ok_or(DecodeError::Uninhabited)
}

/// Decodes a field's core-value-type reference (the [`Code::Field`] leaf).
fn decode_ref(vref: &ValueTypeRef) -> Result<ValueType, DecodeError>
{
    match *vref {
        | ValueTypeRef::Prim(prim) => Ok(decode_prim(prim)),
        // A type parameter is an opaque atom (Stage 1 represents type variables
        // as rigid atoms compared by equality — the `ValueType::Atom` reading).
        | ValueTypeRef::Param(ref name) => Ok(ValueType::atom(name.as_ref())),
        // A bare named type is an opaque atom; an APPLIED named type needs the
        // pipeline's minted `Data` handle, which this crate cannot allocate.
        | ValueTypeRef::Ctor { ref head, ref args } => {
            if args.is_empty() {
                Ok(ValueType::atom(head.as_ref()))
            }
            else {
                Err(DecodeError::AppliedType(head.clone()))
            }
        },
    }
}

/// Decodes a retrofitted primitive type into its core value type.
fn decode_prim(prim: PrimTy) -> ValueType
{
    match prim {
        | PrimTy::Integer => ValueType::integer(),
        // The retrofit `Boolean = 1 + 1` (proposal §3; the frozen core has no
        // dedicated boolean atom, so decoding lands its sum-of-units carrier).
        | PrimTy::Boolean => ValueType::sum(ValueType::Unit, ValueType::Unit),
        | PrimTy::Unit => ValueType::Unit,
        | PrimTy::StringTy => ValueType::string(),
        | PrimTy::Char => ValueType::atom("Char"),
        | PrimTy::U32 => ValueType::u32(),
        | PrimTy::U64 => ValueType::u64(),
        | PrimTy::I32 => ValueType::i32(),
        | PrimTy::I64 => ValueType::i64(),
        | PrimTy::F32 => ValueType::f32(),
        | PrimTy::F64 => ValueType::f64(),
        | PrimTy::Unknown => ValueType::Unknown,
    }
}

#[cfg(test)]
mod tests
{
    use gandr_core_checker::grade::Grade;

    use super::*;
    use crate::code::AtomSort;
    use crate::code::Attrs;
    use crate::desc::CtorDesc;
    use crate::desc::NominalId;

    #[test]
    fn decode_interprets_the_first_order_fragment()
    {
        assert_eq!(
            Ok(ValueType::Unit),
            decode(&Code::Unit, self_sort(), &carrier()),
            "1 ↦ 1"
        );
        assert_eq!(
            decode(&Code::var("Self"), self_sort(), &carrier()),
            Ok(carrier()),
            "a self-sort var ↦ the supplied carrier"
        );
        assert_eq!(
            decode(
                &Code::prod(field(PrimTy::Integer), Code::var("Self")),
                self_sort(),
                &carrier()
            ),
            Ok(ValueType::prod(ValueType::integer(), carrier())),
            "× ↦ core product, recursing"
        );
        assert_eq!(
            decode(
                &Code::sum(Code::Unit, field(PrimTy::StringTy)),
                self_sort(),
                &carrier()
            ),
            Ok(ValueType::sum(ValueType::Unit, ValueType::string())),
            "σ (inline sum) ↦ core coproduct"
        );
    }
    #[test]
    fn decode_declines_a_foreign_sort_var()
    {
        assert_eq!(
            Err(DecodeError::ForeignSort("Tree".into())),
            decode(&Code::var("Tree"), self_sort(), &carrier()),
            "cross-sort recursion is outside the single-sort decode fragment"
        );
    }
    #[test]
    fn decode_erases_field_grade_and_attributes()
    {
        // Two fields differing only in grade / attributes decode identically —
        // the V5 erasure the value decoder performs.
        let graded = Code::field(
            ValueTypeRef::Prim(PrimTy::Integer),
            Grade::OMEGA,
            Attrs::new([crate::code::Attr::marker("boxed")]),
        );
        assert_eq!(
            decode(&graded, self_sort(), &carrier()),
            decode(&field(PrimTy::Integer), self_sort(), &carrier()),
            "grade and attribute decorations are erased by the value decoder (V5)"
        );
    }
    #[test]
    fn decode_boolean_retrofits_to_one_plus_one()
    {
        assert_eq!(
            decode(&field(PrimTy::Boolean), self_sort(), &carrier()),
            Ok(ValueType::sum(ValueType::Unit, ValueType::Unit)),
            "Boolean retrofits to 1 + 1 (proposal §3)"
        );
    }
    #[test]
    fn decode_declines_the_deferred_leaves()
    {
        assert_eq!(
            Err(DecodeError::AtomAbstraction),
            decode(
                &Code::bind(AtomSort::named("v"), Code::var("Self")),
                self_sort(),
                &carrier()
            ),
            "a bind field is outside the stage-1 decode fragment"
        );
        let applied = Code::field(
            ValueTypeRef::Ctor {
                head: "Vec".into(),
                args: Box::new([ValueTypeRef::Param("a".into())]),
            },
            Grade::ONE,
            Attrs::empty(),
        );
        assert_eq!(
            decode(&applied, self_sort(), &carrier()),
            Err(DecodeError::AppliedType("Vec".into())),
            "an applied named type is deferred to the pipeline's Data seam"
        );
    }
    #[test]
    fn decode_desc_is_the_constructor_coproduct()
    {
        // `data Maybe(a) { None | Some(a) }` — the µ decoder reads the two-tag σ
        // as the coproduct `1 + a` (None's nullary payload, Some's parameter).
        let maybe = SignDesc::new(
            NominalId::new(0.into(), "Maybe"),
            Vec::new(),
            [
                CtorDesc::new("None", Code::Unit, "Maybe", Attrs::empty()),
                CtorDesc::new("Some", field(PrimTy::Integer), "Maybe", Attrs::empty()),
            ],
            Vec::new(),
            Vec::new(),
            DeclPolarity::Data,
            Attrs::empty(),
        );
        assert_eq!(
            decode_desc(&maybe, &carrier()),
            Ok(ValueType::sum(ValueType::Unit, ValueType::integer())),
            "Maybe decodes to 1 + Integer"
        );

        // A single-constructor datatype decodes to its lone payload (no coproduct
        // wrapper) — the degenerate tag.
        let wrapper = SignDesc::new(
            NominalId::new(1.into(), "Wrap"),
            Vec::new(),
            [CtorDesc::new(
                "Wrap",
                field(PrimTy::StringTy),
                "Wrap",
                Attrs::empty(),
            )],
            Vec::new(),
            Vec::new(),
            DeclPolarity::Data,
            Attrs::empty(),
        );
        assert_eq!(
            decode_desc(&wrapper, &carrier()),
            Ok(ValueType::string()),
            "a one-constructor datatype decodes to its payload"
        );
    }
    #[test]
    fn decode_desc_declines_codata_and_the_uninhabited()
    {
        let void = SignDesc::new(
            NominalId::new(0.into(), "Void"),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            DeclPolarity::Data,
            Attrs::empty(),
        );
        assert_eq!(
            Err(DecodeError::Uninhabited),
            decode_desc(&void, &carrier()),
            "an uninhabited datatype has no core coproduct"
        );
        let stream = SignDesc::new(
            NominalId::new(1.into(), "Stream"),
            Vec::new(),
            [CtorDesc::new(
                "Head",
                field(PrimTy::Integer),
                "Stream",
                Attrs::empty(),
            )],
            Vec::new(),
            Vec::new(),
            DeclPolarity::Codata,
            Attrs::empty(),
        );
        assert_eq!(
            Err(DecodeError::CodataNu),
            decode_desc(&stream, &carrier()),
            "the ν decoder is a later lane (proposal §5)"
        );
    }
    #[test]
    fn decode_desc_declines_a_multi_sorted_signature()
    {
        use crate::desc::SortDesc;
        let two_sorted = SignDesc::new(
            NominalId::new(2.into(), "Pair"),
            Vec::new(),
            [CtorDesc::new("Mk", Code::Unit, "Pair", Attrs::empty())],
            Vec::new(),
            Vec::new(),
            DeclPolarity::Data,
            Attrs::empty(),
        )
        .with_sorts([
            SortDesc::new("Pair", DeclPolarity::Data),
            SortDesc::new("Other", DeclPolarity::Data),
        ]);
        assert_eq!(
            Err(DecodeError::MultiSorted),
            decode_desc(&two_sorted, &carrier()),
            "the μ decoder reads the single-sort fragment only"
        );
    }
    #[test]
    fn decode_respects_code_equality()
    {
        // Structurally-equal codes decode to structurally-equal types; a single
        // ×↔+ or factor-order edit is observable in the decoded type (the
        // decidable-equality-preserving property, proposal §3).
        let prod = Code::prod(field(PrimTy::Integer), field(PrimTy::StringTy));
        assert_eq!(
            decode(&prod, self_sort(), &carrier()),
            decode(&prod.clone(), self_sort(), &carrier()),
            "equal codes decode equally"
        );
        let sum = Code::sum(field(PrimTy::Integer), field(PrimTy::StringTy));
        assert_ne!(
            decode(&prod, self_sort(), &carrier()),
            decode(&sum, self_sort(), &carrier()),
            "× and σ decode distinctly"
        );
        let swapped = Code::prod(field(PrimTy::StringTy), field(PrimTy::Integer));
        assert_ne!(
            decode(&prod, self_sort(), &carrier()),
            decode(&swapped, self_sort(), &carrier()),
            "factor order is observable"
        );
    }
    /// The `var` carrier used across the tests — an opaque atom standing for
    /// the datatype being defined.
    fn carrier() -> ValueType
    {
        ValueType::atom("Self")
    }
    /// The self sort the direct `decode` tests decode under (the sort a
    /// `var` must name to reach [`carrier`]).
    fn self_sort() -> NameRef<'static>
    {
        "Self".into()
    }
    /// A leaf field of a primitive type (grade / attrs to be erased).
    fn field(prim: PrimTy) -> Code
    {
        Code::field(ValueTypeRef::Prim(prim), Grade::ONE, Attrs::empty())
    }
}
