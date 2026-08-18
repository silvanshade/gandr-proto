//! Source-file program linking for lowered gandr items.
//!
//! The lowerer keeps top-level source items separate: named definitions and
//! modules carry their name, while the script's runnable result is the final
//! unnamed item. This module is the shared shell/FFI bridge that turns that
//! item stream into the one [`Comp`] the core evaluator runs.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use gandr_core_incremental::region::Item;
use gandr_core_term::grade::Grade;
use gandr_core_term::syntax::Comp;
use gandr_core_term::syntax::Term;
use gandr_core_term::syntax::Value;
use gandr_core_term::types::CompType;
use gandr_core_term::types::Ty;
use gandr_core_term::types::ValueType;

use crate::boundary::AscriptionPresence;
use crate::boundary::DefinitionName;
use crate::boundary::ItemIndex;
use crate::lower::Lowered;

/// Result type for linking a lowered source file into one runnable computation.
pub type LinkResult<T> = Result<T, LinkError>;

/// Stable internal binder for checking a computation definition's returned
/// value.
const ASCRIBED_RESULT_BINDER: &str = "__gandr_link_ascribed";
/// Surface discard binder; it must not become a top-level definition name.
const DISCARD_BINDER: &str = "_";

/// A structured program-linking failure.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum LinkError
{
    /// The item stream has no unnamed final run target.
    #[error("no final runnable program after {named_count} named item(s)")]
    NoFinalProgram
    {
        /// Number of named items accepted before end-of-file.
        named_count: usize,
    },
    /// More than one unnamed item wants to be the source file's run target.
    #[error("multiple unnamed runnable targets: first at item {first}, second at item {second}")]
    MultipleRunTargets
    {
        /// The first unnamed runnable item.
        first: usize,
        /// The later unnamed runnable item that made the program ambiguous.
        second: usize,
    },
    /// A named item appeared after the final run target.
    #[error("named item `{name}` at item {index} appears after final runnable item {final_index}")]
    NamedItemAfterRunTarget
    {
        /// The unnamed final target that closed the definition prefix.
        final_index: usize,
        /// The misplaced named item.
        index: usize,
        /// The misplaced item's name.
        name: String,
    },
    /// Two named items define the same binder in one source file.
    #[error("duplicate definition `{name}` at item {second}; first defined at item {first}")]
    DuplicateName
    {
        /// The duplicated binder.
        name: String,
        /// The first definition's item index.
        first: usize,
        /// The second definition's item index.
        second: usize,
    },
    /// A lowered named item carried a binder the linker cannot safely bind.
    #[error("invalid definition name `{name}` at item {index}")]
    InvalidName
    {
        /// The invalid binder.
        name: String,
        /// The offending item index.
        index: usize,
    },
    /// The lowered term or ascription shape cannot become a runnable
    /// computation.
    #[error("unsupported top-level term shape at item {index}")]
    UnsupportedTermShape
    {
        /// The offending item index.
        index: usize,
    },
}

/// One named computation to bind around the final run target.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Binding
{
    /// The source definition/module name.
    name: String,
    /// The computation that evaluates the named item exactly once.
    bound: Comp,
}

/// Links a lowered source file into one runnable computation.
///
/// # Contract
/// - requires: `lowered.items` are in source order.
/// - ensures: accepts zero or more named definition/module items followed by
///   exactly one final unnamed item; named item computations are bound by
///   right-folding [`Comp::Bind`] so source order, lexical visibility, and
///   exactly-once evaluation are preserved.
/// - ensures: a top-level value item is coerced to `ret value`; a computation
///   item is preserved; value-sorted ascriptions annotate only when the exact
///   returned payload boundary is not already annotated, and computation
///   ascriptions use the existing thunk-ascription encoding rather than
///   inventing a core computation annotation.
/// - fails: [`LinkError`] distinguishes a missing final target, multiple final
///   targets, a named item after the final target, duplicate or invalid names,
///   and unsupported future term/ascription shapes.
/// - panics: none.
///
/// # Errors
/// Returns [`LinkError`] when the item stream is not a valid runnable source
/// file, or when a future lowered term/ascription shape cannot be linked.
#[inline]
pub fn link_program(lowered: &Lowered) -> LinkResult<Comp>
{
    let mut bindings: Vec<Binding> = Vec::new();
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    let mut final_target: Option<(usize, Comp)> = None;

    for (index, item) in lowered.items.iter().enumerate() {
        let item_name = item.name.as_ref();
        if let Some(final_entry) = final_target.as_ref() {
            let final_index = final_entry.0;
            match item_name {
                | Some(name) => {
                    return Err(LinkError::NamedItemAfterRunTarget {
                        final_index,
                        index,
                        name: name.clone(),
                    });
                },
                | None => {
                    return Err(LinkError::MultipleRunTargets {
                        first: final_index,
                        second: index,
                    });
                },
            }
        }

        match item_name {
            | Some(name) => {
                validate_name(name.into(), index.into())?;
                if let Some(first) = seen.insert(name.clone(), index) {
                    return Err(LinkError::DuplicateName {
                        name: name.clone(),
                        first,
                        second: index,
                    });
                }
                bindings.push(Binding {
                    name: name.clone(),
                    bound: item_comp(item),
                });
            },
            | None => {
                final_target = Some((index, item_comp(item)));
            },
        }
    }

    let Some((_final_index, mut linked)) = final_target
    else {
        return Err(LinkError::NoFinalProgram {
            named_count: bindings.len(),
        });
    };

    for binding in bindings.into_iter().rev() {
        linked = Comp::bind(binding.bound, &binding.name, linked);
    }
    Ok(linked)
}

/// Validates one lowered top-level binder before it enters a [`Comp::Bind`].
///
/// # Errors
/// Returns [`LinkError::InvalidName`] for the empty name or the discard binder.
fn validate_name(
    name: DefinitionName<'_>,
    index: ItemIndex,
) -> LinkResult<()>
{
    if name.is_empty() || name.0 == DISCARD_BINDER {
        return Err(LinkError::InvalidName {
            name: name.0.to_owned(),
            index: index.0,
        });
    }
    Ok(())
}

/// Converts one lowered item into the computation the outer file bind will run.
fn item_comp(item: &Item) -> Comp
{
    term_to_comp(&item.term, item.ascription.as_ref())
}

/// Converts one lowered term plus its item ascription into a runnable
/// computation.
fn term_to_comp(
    term: &Term,
    ascription: Option<&Ty>,
) -> Comp
{
    match *term {
        | Term::Value(ref value) => value_to_comp(value.clone(), ascription),
        | Term::Comp(ref comp) => comp_to_comp(comp.clone(), ascription),
    }
}

/// Coerces a value item to `ret`, preserving any item ascription.
fn value_to_comp(
    value: Value,
    ascription: Option<&Ty>,
) -> Comp
{
    match ascription {
        | None => Comp::ret(value),
        | Some(&Ty::Value(ref value_ty)) => Comp::ret(ascribe_value(value, value_ty.clone())),
        | Some(&Ty::Comp(ref comp_ty)) => ascribe_comp(Comp::ret(value), comp_ty.clone()),
    }
}

/// Preserves a computation item, adding any item ascription in a sort-correct
/// way.
fn comp_to_comp(
    comp: Comp,
    ascription: Option<&Ty>,
) -> Comp
{
    match ascription {
        | None => comp,
        | Some(&Ty::Comp(ref comp_ty)) => ascribe_comp(comp, comp_ty.clone()),
        | Some(&Ty::Value(ref value_ty)) if comp_payload_has_ascription(&comp, value_ty).0 => comp,
        | Some(&Ty::Value(ref value_ty)) => ascribe_comp_payload(comp, value_ty.clone()),
    }
}

/// Annotates a value unless the same value ascription is already embedded.
fn ascribe_value(
    value: Value,
    ascription: ValueType,
) -> Value
{
    if value_has_ascription(&value, &ascription).0 {
        value
    }
    else {
        Value::annot(value, ascription)
    }
}

/// Encodes a computation ascription with the lowerer's thunk-annotation shape.
fn ascribe_comp(
    comp: Comp,
    ascription: CompType,
) -> Comp
{
    Comp::force(Value::annot(
        Value::thunk(Grade::OMEGA, comp),
        ValueType::thunk(Grade::OMEGA, ascription),
    ))
}

/// Whether a bind-chain computation already returns a value with `ascription`.
fn comp_payload_has_ascription(
    comp: &Comp,
    ascription: &ValueType,
) -> AscriptionPresence
{
    let mut current = comp;
    loop {
        match *current {
            | Comp::Ret(ref value) => {
                return value_has_ascription(value.as_ref(), ascription);
            },
            | Comp::Bind(_, _, ref cont) => current = cont.as_ref(),
            | _ => return AscriptionPresence(false),
        }
    }
}

/// Whether a value is already annotated with `ascription` at its outer
/// boundary.
fn value_has_ascription(
    value: &Value,
    ascription: &ValueType,
) -> AscriptionPresence
{
    AscriptionPresence(match *value {
        | Value::Annot(_, ref existing) => existing.as_ref() == ascription,
        | _ => false,
    })
}

/// Checks the returned value of a computation against a value-sorted
/// ascription.
fn ascribe_comp_payload(
    comp: Comp,
    ascription: ValueType,
) -> Comp
{
    Comp::bind(
        comp,
        ASCRIBED_RESULT_BINDER,
        Comp::ret(Value::annot(Value::var(ASCRIBED_RESULT_BINDER), ascription)),
    )
}

#[cfg(test)]
mod tests
{
    use gandr_core_checker::judgements::control::Dir;
    use gandr_core_incremental::region::Item;
    use gandr_core_sequent::machine::run_comp;
    use gandr_core_term::ctx::Ctx;
    use gandr_core_term::effect::EffectRow;
    use gandr_core_term::grade::Grade;
    use gandr_core_term::outcome::Eval;
    use gandr_core_term::syntax::Comp;
    use gandr_core_term::syntax::Term;
    use gandr_core_term::syntax::Value;
    use gandr_core_term::types::CompType;
    use gandr_core_term::types::Ty;
    use gandr_core_term::types::ValueType;

    use super::ASCRIBED_RESULT_BINDER;
    use super::LinkError;
    use super::link_program;
    use crate::lower::Lowered;
    use crate::lower::lower_source;

    /// Optional test-item binder.
    #[repr(transparent)]
    struct OptionalItemName<'name>(Option<&'name str>);

    impl<'name> From<Option<&'name str>> for OptionalItemName<'name>
    {
        #[inline]
        fn from(value: Option<&'name str>) -> Self
        {
            Self(value)
        }
    }
    #[test]
    fn already_embedded_value_ascriptions_are_not_duplicated()
    {
        let linked = link_program(&lowered(vec![
            item_with_ascription(
                Some("x"),
                Term::Value(Value::annot(Value::int(5), ValueType::integer())),
                Ty::Value(ValueType::integer()),
            ),
            item(None, Term::Value(Value::var("x"))),
        ]))
        .expect("an already-annotated value definition links");

        let expected = Comp::bind(
            Comp::ret(Value::annot(Value::int(5), ValueType::integer())),
            "x",
            Comp::ret(Value::var("x")),
        );
        assert_eq!(
            linked, expected,
            "the linker must not wrap an identical embedded value annotation again"
        );
    }

    fn item_with_ascription<'name>(
        name: impl Into<OptionalItemName<'name>>,
        term: Term,
        ascription: Ty,
    ) -> Item
    {
        Item {
            name: name.into().0.map(str::to_owned),
            ascription: Some(ascription),
            term,
        }
    }
    #[test]
    fn named_prefix_right_folds_into_source_order_binds()
    {
        let linked = link_program(&lowered(vec![
            item(Some("first"), Term::Value(Value::int(1))),
            item(Some("second"), Term::Value(Value::var("first"))),
            item(None, Term::Value(Value::var("second"))),
        ]))
        .expect("a named prefix plus final value links");

        let expected = Comp::bind(
            Comp::ret(Value::int(1)),
            "first",
            Comp::bind(
                Comp::ret(Value::var("first")),
                "second",
                Comp::ret(Value::var("second")),
            ),
        );
        assert_eq!(
            linked, expected,
            "the outer bind runs the first source definition, then the second, then the final target"
        );
    }
    #[test]
    fn value_items_and_computation_items_keep_their_runnable_shape()
    {
        let linked = link_program(&lowered(vec![
            item(Some("value"), Term::Value(Value::int(1))),
            item(Some("comp"), Term::Comp(Comp::ret(Value::var("value")))),
            item(None, Term::Comp(Comp::ret(Value::var("comp")))),
        ]))
        .expect("value and computation items link");

        let expected = Comp::bind(
            Comp::ret(Value::int(1)),
            "value",
            Comp::bind(
                Comp::ret(Value::var("value")),
                "comp",
                Comp::ret(Value::var("comp")),
            ),
        );
        assert_eq!(
            linked, expected,
            "value items coerce to ret, while computation items are preserved"
        );
    }
    #[test]
    fn computation_ascriptions_use_the_existing_thunk_annotation_encoding()
    {
        let linked = link_program(&lowered(vec![
            item_with_ascription(
                Some("x"),
                Term::Comp(Comp::ret(Value::int(9))),
                Ty::Comp(CompType::returner(ValueType::integer())),
            ),
            item(None, Term::Value(Value::var("x"))),
        ]))
        .expect("a computation-sorted signature links");

        let expected_bound = Comp::force(Value::annot(
            Value::thunk(Grade::OMEGA, Comp::ret(Value::int(9))),
            ValueType::thunk(Grade::OMEGA, CompType::returner(ValueType::integer())),
        ));
        let expected = Comp::bind(expected_bound, "x", Comp::ret(Value::var("x")));
        assert_eq!(
            linked, expected,
            "computation ascriptions reuse force((thunk body) : Uω B)"
        );
    }
    #[test]
    fn no_final_target_is_structured()
    {
        let err = link_program(&lowered(vec![item(Some("x"), Term::Value(Value::int(1)))]));
        assert_eq!(Err(LinkError::NoFinalProgram { named_count: 1 }), err);
    }
    #[test]
    fn two_unnamed_targets_are_structured()
    {
        let err = link_program(&lowered(vec![
            item(None, Term::Value(Value::int(1))),
            item(None, Term::Value(Value::int(2))),
        ]));
        assert_eq!(
            Err(LinkError::MultipleRunTargets {
                first: 0,
                second: 1,
            }),
            err
        );
    }
    #[test]
    fn invalid_definition_names_are_structured()
    {
        let err = link_program(&lowered(vec![
            item(Some("_"), Term::Value(Value::int(1))),
            item(None, Term::Value(Value::int(2))),
        ]));
        assert_eq!(
            err,
            Err(LinkError::InvalidName {
                name: "_".to_owned(),
                index: 0,
            })
        );
    }

    fn lowered(items: Vec<Item>) -> Lowered
    {
        let mut lowered = Lowered::default();
        lowered.items = items;
        lowered
    }
    #[test]
    fn lower_source_effectful_ascribed_module_preserves_payload_contract_and_row()
    {
        let mut lowered =
            lower_source("module M : #{ dir: String } { def dir = fs.tempdir(); }".into())
                .expect("an effectful ascribed module lowers");
        lowered.items.push(item(None, Term::Value(Value::var("M"))));
        let module_item = lowered.items.first().expect("the module item is first");
        let module_comp = match &module_item.term {
            | &Term::Comp(ref comp) => Some(comp),
            | _ => None,
        }
        .expect("the module lowers to a computation item");
        let module_ty = match module_item.ascription.as_ref() {
            | Some(&Ty::Value(ref ty)) => Some(ty),
            | _ => None,
        }
        .expect("the module item keeps a value-sorted metadata ascription");
        let module_row = inferred_returner_row(module_comp.clone());
        assert!(
            !bool::from(module_row.is_empty()),
            "the source module body is effectful before linking"
        );

        let linked = link_program(&lowered).expect("the effectful ascribed module links");
        let linked_row = inferred_returner_row(linked.clone());
        assert_eq!(
            linked_row, module_row,
            "linking must not impose an empty row or add effects around the module binding"
        );

        let (bound, name, cont) = match linked {
            | Comp::Bind(ref bound, ref name, ref cont) => Some((bound, name, cont)),
            | _ => None,
        }
        .expect("a named module links as the outer file bind");
        assert_eq!("M", name, "the module binder remains the source name");
        assert_eq!(
            cont.as_ref(),
            &Comp::ret(Value::var("M")),
            "the final target returns the linked module value"
        );
        let returned =
            terminal_return_value(bound.as_ref()).expect("the module bind-chain returns a value");
        let payload = one_root_annotation(returned, module_ty);
        assert!(
            matches!(payload, &Value::Record(_)),
            "the preserved annotation still wraps the lowered module record, not a linker binder"
        );
    }
    #[test]
    fn computation_payload_value_ascriptions_do_not_impose_an_empty_effect_row()
    {
        let linked = link_program(&lowered(vec![
            item_with_ascription(
                Some("x"),
                Term::Comp(Comp::ret(Value::int(7))),
                Ty::Value(ValueType::integer()),
            ),
            item(None, Term::Value(Value::var("x"))),
        ]))
        .expect("a computation definition with a value-result contract links");

        let expected_bound = Comp::bind(
            Comp::ret(Value::int(7)),
            ASCRIBED_RESULT_BINDER,
            Comp::ret(Value::annot(
                Value::var(ASCRIBED_RESULT_BINDER),
                ValueType::integer(),
            )),
        );
        let expected = Comp::bind(expected_bound, "x", Comp::ret(Value::var("x")));
        assert_eq!(
            linked, expected,
            "the payload annotation is sequenced after the computation, preserving the bound row"
        );
    }
    #[test]
    fn named_item_after_final_target_is_structured()
    {
        let err = link_program(&lowered(vec![
            item(None, Term::Value(Value::int(1))),
            item(Some("late"), Term::Value(Value::int(2))),
        ]));
        assert_eq!(
            err,
            Err(LinkError::NamedItemAfterRunTarget {
                final_index: 0,
                index: 1,
                name: "late".to_owned(),
            })
        );
    }
    #[test]
    fn duplicate_definitions_are_structured()
    {
        let err = link_program(&lowered(vec![
            item(Some("x"), Term::Value(Value::int(1))),
            item(Some("x"), Term::Value(Value::int(2))),
            item(None, Term::Value(Value::var("x"))),
        ]));
        assert_eq!(
            err,
            Err(LinkError::DuplicateName {
                name: "x".to_owned(),
                first: 0,
                second: 1,
            })
        );
    }

    fn item<'name>(
        name: impl Into<OptionalItemName<'name>>,
        term: Term,
    ) -> Item
    {
        Item {
            name: name.into().0.map(str::to_owned),
            ascription: None,
            term,
        }
    }

    fn empty_record_ty() -> ValueType
    {
        ValueType::record(Vec::<(String, ValueType)>::new())
    }
    #[test]
    fn lower_source_empty_ascribed_module_links_once_and_runs_on_l_machine()
    {
        let mut lowered =
            lower_source("module M : #{} {}".into()).expect("an empty ascribed module lowers");
        lowered.items.push(item(None, Term::Value(Value::var("M"))));
        let linked = link_program(&lowered).expect("the empty ascribed module links");
        let record_ty = empty_record_ty();

        let expected = Comp::bind(
            Comp::ret(Value::annot(empty_record(), record_ty)),
            "M",
            Comp::ret(Value::var("M")),
        );
        assert_eq!(
            linked, expected,
            "lowering embeds the module record ascription, so linking preserves it exactly once"
        );

        let runtime = run_comp(&linked);
        let returned = match runtime {
            | Eval::Value(Comp::Ret(ref returned)) => Some(returned.as_ref()),
            | _ => None,
        }
        .expect("the linked module returns a value");
        assert_eq!(
            returned,
            &empty_record(),
            "the L machine erases the checked annotation after the linker preserves it exactly once"
        );
    }

    fn empty_record() -> Value
    {
        Value::record(Vec::<(String, Value)>::new())
    }

    fn inferred_returner_row(comp: Comp) -> EffectRow
    {
        let (ty, _trace) = gandr_core_machine::run_comp(Ctx::new(), comp, Dir::Infer);
        match ty.expect("the linked computation infers") {
            | Ty::Comp(CompType::F(_, row)) => Some(row),
            | _ => None,
        }
        .expect("the linked computation infers a returner type")
    }

    fn terminal_return_value(comp: &Comp) -> Option<&Value>
    {
        let mut current = comp;
        loop {
            match *current {
                | Comp::Ret(ref value) => return Some(value.as_ref()),
                | Comp::Bind(_, _, ref cont) => current = cont.as_ref(),
                | _ => return None,
            }
        }
    }

    fn one_root_annotation<'value>(
        value: &'value Value,
        expected: &ValueType,
    ) -> &'value Value
    {
        let (payload, ty) = match value {
            | &Value::Annot(ref payload, ref ty) => Some((payload, ty)),
            | _ => None,
        }
        .expect("expected one root annotation");
        assert_eq!(
            ty.as_ref(),
            expected,
            "the root annotation carries the source ascription"
        );
        assert!(
            !matches!(payload.as_ref(), &Value::Annot(_, _)),
            "the payload must not be annotated again"
        );
        payload.as_ref()
    }

    #[test]
    fn value_ascriptions_are_preserved_on_named_value_bindings()
    {
        let lowered = lower_source("def x : Integer; def x = 5; x".into())
            .expect("signature, value definition, and final reference lower");
        let linked = link_program(&lowered).expect("the signed definition links");

        let expected = Comp::bind(
            Comp::ret(Value::annot(Value::int(5), ValueType::integer())),
            "x",
            Comp::ret(Value::var("x")),
        );
        assert_eq!(
            linked, expected,
            "a value-sorted item ascription annotates the bound value instead of disappearing"
        );
    }

    #[test]
    fn already_embedded_module_ascriptions_are_not_duplicated()
    {
        let lowered = lower_source("module M : #{ x: Integer } { def x = 1; }\nM.x".into())
            .expect("an ascribed module and final reference lower");
        let linked = link_program(&lowered).expect("the ascribed module links");

        let record_ty = ValueType::record([("x".to_owned(), ValueType::integer())]);
        let module_record = Value::annot(
            Value::record([("x".to_owned(), Value::var("x"))]),
            record_ty,
        );
        let module_bound = Comp::bind(Comp::ret(Value::int(1)), "x", Comp::ret(module_record));
        let expected = Comp::bind(module_bound, "M", Comp::record_proj(Value::var("M"), "x"));
        assert_eq!(
            linked, expected,
            "module lowering embeds the record ascription, so linking must not add a second payload annotation"
        );
    }

    #[test]
    fn extern_and_codata_declarations_contribute_no_runnable_items()
    {
        /// The bound integer in the expected linked term.
        const ANSWER: i64 = 42;

        let lowered = lower_source(
            concat!(
                "extern \"c\" from \"m\" {\n  def f(x: i64) -> i64;\n}\n",
                "codata Point : Type { x : Integer; y : Integer; }\n",
                "def answer = 42;\n",
                "answer",
            )
            .into(),
        )
        .expect("declarations plus runnable items lower");
        let linked = link_program(&lowered).expect("declarations do not disturb linking");

        let expected = Comp::bind(
            Comp::ret(Value::int(ANSWER)),
            "answer",
            Comp::ret(Value::var("answer")),
        );
        assert_eq!(linked, expected);
    }

    #[test]
    fn empty_lowered_source_has_zero_named_items()
    {
        let lowered = lower_source("".into()).expect("empty source lowers to an empty item stream");
        let err = link_program(&lowered);
        assert_eq!(Err(LinkError::NoFinalProgram { named_count: 0 }), err);
    }
}
