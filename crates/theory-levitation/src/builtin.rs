//! **Retrofitted descriptions** for the primitive formers (proposal §3: "the
//! primitive formers get retrofitted descriptions so generic operations cover
//! builtins and declared data uniformly").
//!
//! Each function returns a [`SignDesc`] for a builtin, so the generic programs
//! of [`crate::generic`] — equality, serialization, inspection — and the
//! [`crate::CodeInterner`] apply to `Boolean`, `Option`, `Pair`, sums, and
//! `List` exactly as they apply to a declared datatype. The retrofits use the
//! same first-order fragment: `Boolean` is `1 + 1` (two nullary constructors);
//! `List` is recursive through [`crate::Code::Var`].

use gandr_core_checker::boundary::NameRef;
use gandr_core_checker::grade::Grade;

use crate::code::Attrs;
use crate::code::Code;
use crate::code::ValueTypeRef;
use crate::desc::CtorDesc;
use crate::desc::DeclPolarity;
use crate::desc::NominalId;
use crate::desc::ParamDesc;
use crate::desc::SignDesc;

/// The retrofit description of **`Option(a)`** — `None = 1`, `Some = a`.
#[inline]
#[must_use]
pub fn option_desc() -> SignDesc
{
    SignDesc::new(
        NominalId::new(0.into(), "Option"),
        [param("a".into())],
        [
            nullary("None".into(), "Option".into()),
            CtorDesc::new("Some", param_field("a".into()), "Option", Attrs::empty()),
        ],
        Vec::new(),
        Vec::new(),
        DeclPolarity::Data,
        Attrs::empty(),
    )
}

/// The retrofit description of **`Boolean`** as `1 + 1` — two nullary
/// constructors `False` and `True` (proposal §5's `Boolean` retrofit target).
#[inline]
#[must_use]
pub fn bool_desc() -> SignDesc
{
    SignDesc::new(
        NominalId::new(0.into(), "Boolean"),
        Vec::new(),
        [
            nullary("False".into(), "Boolean".into()),
            nullary("True".into(), "Boolean".into()),
        ],
        Vec::new(),
        Vec::new(),
        DeclPolarity::Data,
        Attrs::empty(),
    )
}
/// The retrofit description of **`List(a)`** — `Nil = 1`,
/// `Cons = a × var` (recursive through [`Code::Var`]).
#[inline]
#[must_use]
pub fn list_desc() -> SignDesc
{
    SignDesc::new(
        NominalId::new(0.into(), "List"),
        [param("a".into())],
        [
            nullary("Nil".into(), "List".into()),
            CtorDesc::new(
                "Cons",
                Code::prod(param_field("a".into()), Code::var("List")),
                "List",
                Attrs::empty(),
            ),
        ],
        Vec::new(),
        Vec::new(),
        DeclPolarity::Data,
        Attrs::empty(),
    )
}
/// The retrofit description of **`Pair(a, b)`** — one constructor `Pair` whose
/// payload is `a × b`.
#[inline]
#[must_use]
pub fn pair_desc() -> SignDesc
{
    SignDesc::new(
        NominalId::new(0.into(), "Pair"),
        [param("a".into()), param("b".into())],
        [CtorDesc::new(
            "Pair",
            Code::prod(param_field("a".into()), param_field("b".into())),
            "Pair",
            Attrs::empty(),
        )],
        Vec::new(),
        Vec::new(),
        DeclPolarity::Data,
        Attrs::empty(),
    )
}
/// The retrofit description of the binary **sum `Sum(a, b)`** — `Inl = a`,
/// `Inr = b` (the tagged form mirroring gandr's `Inl` / `Inr`).
#[inline]
#[must_use]
pub fn sum_desc() -> SignDesc
{
    SignDesc::new(
        NominalId::new(0.into(), "Sum"),
        [param("a".into()), param("b".into())],
        [
            CtorDesc::new("Inl", param_field("a".into()), "Sum", Attrs::empty()),
            CtorDesc::new("Inr", param_field("b".into()), "Sum", Attrs::empty()),
        ],
        Vec::new(),
        Vec::new(),
        DeclPolarity::Data,
        Attrs::empty(),
    )
}
/// A linear, unattributed parameter of the given name.
fn param(name: NameRef<'_>) -> ParamDesc
{
    ParamDesc::new(name, Grade::ONE, Attrs::empty())
}
/// A nullary constructor of the given name (`1`), targeting the result sort
/// `of`.
fn nullary(
    name: NameRef<'_>,
    of: NameRef<'_>,
) -> CtorDesc
{
    CtorDesc::new(name, Code::Unit, of, Attrs::empty())
}
/// A linear, unattributed field over a type parameter of the given name.
fn param_field(name: NameRef<'_>) -> Code
{
    Code::field(ValueTypeRef::Param(name.into()), Grade::ONE, Attrs::empty())
}

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::generic::DescValue;
    use crate::generic::Payload;
    use crate::generic::generic_eq;
    use crate::generic::serialize_desc;
    use crate::wellformed::check_desc;

    #[test]
    fn every_retrofit_is_well_formed()
    {
        for desc in [
            bool_desc(),
            option_desc(),
            pair_desc(),
            sum_desc(),
            list_desc(),
        ] {
            assert!(
                check_desc(&desc).is_empty(),
                "the retrofit `{}` is well-formed",
                desc.id.name
            );
        }
    }

    #[test]
    fn generic_programs_cover_builtins_uniformly()
    {
        // The generic consumers apply to `Boolean` exactly as to declared data.
        let boolean = bool_desc();
        assert_eq!(
            "data Boolean { False = 1, True = 1 }",
            serialize_desc(&boolean).as_ref(),
            "the builtin renders through the same inspection notation"
        );
        let truth = DescValue::new(1.into(), Payload::Unit);
        let falsity = DescValue::new(0.into(), Payload::Unit);
        assert!(
            bool::from(generic_eq(&boolean, &truth, &truth)),
            "True == True"
        );
        assert!(
            !bool::from(generic_eq(&boolean, &truth, &falsity)),
            "True ≠ False"
        );
    }

    #[test]
    fn list_is_recursive()
    {
        assert!(
            bool::from(list_desc().is_recursive()),
            "List recurses through Cons"
        );
        assert!(
            !bool::from(option_desc().is_recursive()),
            "Option is non-recursive"
        );
    }
}
