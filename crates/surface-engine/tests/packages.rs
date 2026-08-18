//! The package surface: `package [ T ] payload` as a type, `pack [ Ā ] E` as
//! an expression, and `unpack m : Sig = E ;` as a statement binding the module
//! over the rest of its block.
//!
//! Each test lowers a real source and then **checks the lowered core**, because
//! the two halves of the rung meet exactly there: the surface's job is to hand
//! the checker a package the checker accepts, and a lowering that produces a
//! well-formed-looking term the checker then rejects would be a green test and
//! a broken feature.

#![cfg_attr(
    dylint_lib = "non_topologically_sorted_functions",
    allow(
        unknown_lints,
        non_topologically_sorted_functions,
        reason = "integration tests share fixture helpers called from tests in per-test orders; no single module arrangement satisfies every caller-before-callee pair, so the ordering rule is waived in test code pending a test-layout redesign"
    )
)]

#[cfg(test)]
mod tests
{
    use gandr_core_checker::judgements::checker::run_comp;
    use gandr_core_checker::machine::control::Dir;
    use gandr_core_checker::outcome::Eval;
    use gandr_core_checker::term::ctx::Ctx;
    use gandr_core_checker::term::syntax::Comp;
    use gandr_core_checker::term::syntax::Value;
    use gandr_core_checker::term::types::CompType;
    use gandr_core_checker::term::types::Ty;
    use gandr_core_checker::term::types::ValueType;
    use gandr_core_sequent::machine::run_comp as evaluate;
    use gandr_surface_engine::lower::LowerError;
    use gandr_surface_engine::lower::lower_source;

    use crate::common::TestText;

    /// The counter signature, spelled once and reused: the surface writes the
    /// grade **only** on the payload thunk, and the package reads it off.
    const COUNTER: &str = "package [ T ] U[ω] (F #{ read: U[ω] (T -> F Integer), seed: T })";

    /// A program that packs an integer counter, unpacks it, and reads its seed
    /// through the signature's own operation.
    fn dispatch_source<'text>(
        witness: TestText<'text>,
        seed: TestText<'text>,
    ) -> String
    {
        let witness = witness.0;
        let seed = seed.0;
        alloc::format!(
            "def make : {COUNTER};
             def make = pack [ {witness} ] thunk {{
               ret #{{ read = thunk {{ fn (n: {witness}) {{ ret {seed} }} }}, seed = {seed} }}
             }};

             def answer() -> F Integer {{
               unpack m : {COUNTER} = pack [ {witness} ] thunk {{
                 ret #{{ read = thunk {{ fn (n: {witness}) {{ ret {seed} }} }}, seed = {seed} }}
               }};
               run r <- force m;
               run f <- r.read;
               run s <- r.seed;
               f(s)
             }}

             answer()"
        )
    }

    /// The item's lowered term, by name.
    fn item_term(
        source: TestText<'_>,
        name: TestText<'_>,
    ) -> Result<gandr_core_checker::term::syntax::Term, LowerError>
    {
        let source = source.0;
        let name = name.0;
        let lowered = lower_source(source.into())?;
        lowered
            .items
            .into_iter()
            .find(|item| item.name.as_deref() == Some(name))
            .map(|item| item.term)
            .ok_or(LowerError::ParseFailed)
    }

    /// The computation a zero-argument `def name() -> …` item holds.
    ///
    /// The def-function surface stores a **thunk**, which is why a function can
    /// be passed around at all in this core; the body is what these tests are
    /// about.
    fn thunked_body(
        source: TestText<'_>,
        name: TestText<'_>,
    ) -> Comp
    {
        let name = name.0;
        let term = item_term(source, name.into()).expect("the item lowers");
        let gandr_core_checker::term::syntax::Term::Value(Value::Thunk(_, body)) = term
        else {
            panic!("`{name}` lowers to a thunked function, got {term:?}");
        };
        body.as_ref().clone()
    }

    /// **The surface reaches the core former.** A `pack` lowers to
    /// [`Value::Pack`] carrying its witness types, and the `unpack` statement
    /// lowers to [`Comp::Unpack`] carrying its ascription and one minted atom
    /// per abstract component.
    #[test]
    fn the_three_forms_lower_to_the_core_package_nodes()
    {
        let source = dispatch_source("Integer".into(), "7".into());
        let make = item_term((&source).into(), "make".into()).expect("the packing item lowers");
        let gandr_core_checker::term::syntax::Term::Value(Value::Pack { ref witnesses, .. }) = make
        else {
            panic!("`make` lowers to a pack, got {make:?}");
        };
        assert_eq!(
            vec![ValueType::integer()],
            witnesses
                .iter()
                .map(|witness| witness.as_ref().clone())
                .collect::<Vec<_>>(),
            "the bracketed witness reaches the core node"
        );

        let comp = thunked_body((&source).into(), "answer".into());
        let Comp::Unpack {
            ref signature,
            ref atoms,
            ref binder,
            ..
        } = comp
        else {
            panic!("`answer`'s body is an unpack, got {comp:?}");
        };
        assert_eq!("m", binder, "the module variable is the surface binder");
        assert!(
            matches!(*signature.as_ref(), ValueType::Package { .. }),
            "the ascription lowers to the package former"
        );
        assert_eq!(1, atoms.len(), "one atom per abstract type component");
        let atom = atoms.first().expect("the single minted atom");
        assert_eq!(
            "m",
            atom.declaration().as_ref(),
            "the minting site's declaration is the module binder"
        );
        assert_eq!(
            "T",
            atom.component().as_ref(),
            "and its component is the signature's own label"
        );
    }

    /// **The lowered program type-checks and runs.** This is the join the rung
    /// exists for: the surface hands the checker a package it accepts, and the
    /// machine then answers through the signature.
    #[test]
    fn a_packed_module_checks_and_evaluates_through_its_signature()
    {
        let source = dispatch_source("Integer".into(), "7".into());
        let comp = thunked_body((&source).into(), "answer".into());
        let expected = CompType::returner(ValueType::integer());
        let (checked, _) = run_comp(Ctx::new(), comp.clone(), Dir::Check(expected.clone()));
        assert_eq!(
            checked,
            Ok(Ty::Comp(expected)),
            "the lowered consumer checks at F Integer"
        );
        assert_eq!(
            evaluate(&comp),
            Eval::Value(Comp::ret(Value::int(7_i64))),
            "and it answers what the packed implementation gives"
        );
    }

    /// **A second representation, the same signature.** The consumer is
    /// unchanged; only the packed module differs, which is the whole point of
    /// packing one.
    #[test]
    fn a_different_representation_checks_at_the_same_signature()
    {
        let source = dispatch_source("String".into(), "\"seven\"".into());
        let make = item_term((&source).into(), "make".into()).expect("the packing item lowers");
        let gandr_core_checker::term::syntax::Term::Value(Value::Pack { ref witnesses, .. }) = make
        else {
            panic!("`make` lowers to a pack");
        };
        assert_eq!(
            vec![ValueType::string()],
            witnesses
                .iter()
                .map(|witness| witness.as_ref().clone())
                .collect::<Vec<_>>(),
            "the witness is the string representation this time"
        );
    }

    /// A package payload that is not a graded thunk is refused at formation:
    /// the package's grade is the payload's, so there would be none to read.
    #[test]
    fn a_payload_that_is_not_a_graded_thunk_is_refused()
    {
        let source = "def bad : package [ T ] Integer; def bad = ret 1;";
        assert!(
            matches!(
                lower_source(source.into()),
                Err(LowerError::PackagePayloadNotGradedThunk { .. })
            ),
            "a package payload must be the thunk whose grade the package reads"
        );
    }

    /// A signature declaring one component twice is refused where it is
    /// written: the second binder would shadow the first, leaving one supplied
    /// witness unreachable.
    #[test]
    fn a_duplicated_component_is_refused()
    {
        let source = "def bad : package [ T, T ] U[ω] (F Integer); def bad = ret 1;";
        assert!(
            matches!(
                lower_source(source.into()),
                Err(LowerError::DuplicatePackageComponent { .. })
            ),
            "each abstract type component is declared once"
        );
    }

    /// An `unpack` ascribed at something other than a package is refused rather
    /// than read as a nearby form — nothing infers a module type from the
    /// expression, so there is nothing to fall back to.
    #[test]
    fn an_unpack_ascribed_at_a_non_package_is_refused()
    {
        let source = "def bad() -> F Integer {
                        unpack m : U[ω] (F Integer) = thunk { ret 1 };
                        force m
                      }
                      bad()";
        assert!(
            matches!(
                lower_source(source.into()),
                Err(LowerError::UnpackNeedsPackageSignature { .. })
            ),
            "an unpack ascription is a package type or nothing"
        );
    }

    /// **Per-elimination freshness at the surface.** Two `unpack`s in one
    /// source mint different atoms even when they share a module binder name,
    /// so their abstract types do not interchange.
    #[test]
    fn two_unpacks_mint_distinct_atoms()
    {
        let source = alloc::format!(
            "def make : {COUNTER};
             def make = pack [ Integer ] thunk {{
               ret #{{ read = thunk {{ fn (n: Integer) {{ ret n }} }}, seed = 7 }}
             }};

             def answer() -> F Integer {{
               unpack m : {COUNTER} = make;
               run a <- force m;
               unpack m : {COUNTER} = make;
               run b <- force m;
               ret 0
             }}

             answer()"
        );
        let outer = thunked_body((&source).into(), "answer".into());
        let outer_atom = sole_unpack_atom(&outer);
        let inner_atom = nested_unpack_atom(&outer);
        assert_ne!(
            outer_atom, inner_atom,
            "two eliminations mint two atoms, however alike their sites"
        );
    }

    /// The single atom of the outermost `unpack` in a computation.
    fn sole_unpack_atom(comp: &Comp) -> gandr_core_checker::term::types::SealId
    {
        let Comp::Unpack { ref atoms, .. } = *comp
        else {
            panic!("expected an unpack, got {comp:?}");
        };
        atoms.first().expect("one minted atom").clone()
    }

    /// The single atom of the next `unpack` below the outermost one.
    fn nested_unpack_atom(comp: &Comp) -> gandr_core_checker::term::types::SealId
    {
        let mut current = comp.clone();
        let mut seen_outer = false;
        loop {
            match current {
                | Comp::Unpack { ref body, .. } if seen_outer => {
                    return sole_unpack_atom(&current.clone());
                },
                | Comp::Unpack { ref body, .. } => {
                    seen_outer = true;
                    let next = body.as_ref().clone();
                    current = next;
                },
                | Comp::Bind(_, _, ref rest) => {
                    let next = rest.as_ref().clone();
                    current = next;
                },
                | ref other => panic!("no second unpack below the first: {other:?}"),
            }
        }
    }
}
