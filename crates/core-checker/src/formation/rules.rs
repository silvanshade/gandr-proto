//! The formation rules, one per type constructor.

use alloc::vec::Vec;

use gandr_core_term::boundary::FamilyArity;
use gandr_core_term::boundary::TypeAtomName;
use gandr_core_term::classifier::Classifier;
use gandr_core_term::classifier::GroundSort;
use gandr_core_term::classifier::SortExpr;
use gandr_core_term::error::FormationError;
use gandr_core_term::error::UnsupportedForm;
use gandr_core_term::grade::Grade;
use gandr_core_term::static_term::StaticArg;
use gandr_core_term::syntax::Value;
use gandr_core_term::types::CompType;
use gandr_core_term::types::ValueType;
use gandr_kernel_strata::Level;

use crate::formation::context::FormationContext;

/// A type that can be formed: it answers with the classifier it is formed at,
/// or with the named reason it is not.
///
/// # Why a trait rather than two functions
///
/// The two ground ASTs stay separate — that is what makes a sort error
/// unrepresentable after checked construction — so the judgement is stated
/// once and implemented twice, rather than flattened into one enum with a
/// runtime tag.
pub trait FormType
{
    /// The classifier this type is formed at.
    ///
    /// # Contract
    /// - requires: every free level, sort, type, and family name occurring in
    ///   the type is bound by `ctx`.
    /// - ensures: the returned classifier's level is derived through the one
    ///   level algebra, never by arithmetic written here.
    /// - provides: the single place a type's sort and level are decided.
    /// - fails: a named [`FormationError`]; never a fallthrough classifier and
    ///   never a fallthrough to the gradual unknown.
    /// - panics: none.
    ///
    /// # Errors
    /// Graded bridges are outside the admitted formation fragment. The
    /// ungraded `+U` and `-F` floors preserve their premise level; a future
    /// grade/effect rule admits the graded forms once its level action is
    /// declared.
    fn infer_classifier(
        &self,
        ctx: &FormationContext,
    ) -> Result<Classifier, FormationError>;
}

/// A pending formation operation in the explicit worklist.
enum Task<'ty>
{
    /// Form a value type.
    Value(&'ty ValueType),
    /// Form a computation type.
    Comp(&'ty CompType),
    /// Consume child results for a value constructor.
    FinishValue(ValueFinish),
    /// Consume child results for a computation constructor.
    FinishComp(CompFinish),
}

/// A typed number of formation results consumed from the worklist.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct ResultCount(usize);

impl From<usize> for ResultCount
{
    #[inline]
    fn from(count: usize) -> Self
    {
        Self(count)
    }
}

impl From<ResultCount> for usize
{
    #[inline]
    fn from(count: ResultCount) -> Self
    {
        count.0
    }
}

/// The value-constructor finish frames.
enum ValueFinish
{
    /// Join child classifiers at the constructor's result sort.
    Join
    {
        /// Number of child results to consume.
        count: ResultCount,
        /// Required sort for every child.
        expected: GroundSort,
        /// Sort assigned to the finished constructor.
        result: GroundSort,
    },
    /// Form a thunk after its computation body.
    Thunk,
}

/// The computation-constructor finish frames.
enum CompFinish
{
    /// Form a returner after its value body.
    Returner,
    /// Form an arrow after its argument and result.
    Arrow,
    /// Form a with-type after both computations.
    With,
}

impl FormType for ValueType
{
    #[inline]
    fn infer_classifier(
        &self,
        ctx: &FormationContext,
    ) -> Result<Classifier, FormationError>
    {
        form_value(self, ctx)
    }
}

impl FormType for CompType
{
    #[inline]
    fn infer_classifier(
        &self,
        ctx: &FormationContext,
    ) -> Result<Classifier, FormationError>
    {
        form_comp(self, ctx)
    }
}

/// Form a value type without host recursion.
fn form_value(
    root: &ValueType,
    ctx: &FormationContext,
) -> Result<Classifier, FormationError>
{
    run(ctx, Task::Value(root))
}

/// Form a computation type without host recursion.
fn form_comp(
    root: &CompType,
    ctx: &FormationContext,
) -> Result<Classifier, FormationError>
{
    run(ctx, Task::Comp(root))
}

/// Drain one type tree with heap frames rather than host recursion.
///
/// # Contract
/// - ensures: returns the same classifier a complete formation derivation
///   yields, or the first named formation refusal.
/// - panics: none.
///
/// # Termination
/// - reason: the walk drains an explicit task stack over one finite type.
/// - measure: pending tasks on the stack.
/// - boundedness: each task pushed for a child refers to a strict child of the
///   finite caller-owned type tree, and every finish frame consumes its child
///   answers before producing one parent answer.
/// - input recursion: none.
fn run(
    ctx: &FormationContext,
    initial: Task<'_>,
) -> Result<Classifier, FormationError>
{
    let mut pending = alloc::vec![initial];
    let mut results = Vec::new();
    while let Some(task) = pending.pop() {
        match task {
            | Task::Value(value_ty) => match *value_ty {
                | ValueType::Atom(ref name) => {
                    let classifier = ctx.type_variable(TypeAtomName::from(name))?;
                    results.push(require_sort(classifier, GroundSort::Value)?);
                },
                | ValueType::Unit | ValueType::Unknown => {
                    results.push(value_classifier(Level::zero()));
                },
                | ValueType::Prod(ref fst, ref snd)
                | ValueType::Sum(ref fst, ref snd)
                | ValueType::Sigma {
                    ref fst, ref snd, ..
                } => {
                    pending.push(Task::FinishValue(ValueFinish::Join {
                        count: ResultCount::from(2_usize),
                        expected: GroundSort::Value,
                        result: GroundSort::Value,
                    }));
                    pending.push(Task::Value(snd));
                    pending.push(Task::Value(fst));
                },
                | ValueType::List(ref element) => {
                    pending.push(Task::FinishValue(ValueFinish::Join {
                        count: ResultCount::from(1_usize),
                        expected: GroundSort::Value,
                        result: GroundSort::Value,
                    }));
                    pending.push(Task::Value(element));
                },
                | ValueType::Record(ref fields) => {
                    pending.push(Task::FinishValue(ValueFinish::Join {
                        count: ResultCount::from(fields.len()),
                        expected: GroundSort::Value,
                        result: GroundSort::Value,
                    }));
                    for field in fields.values().rev() {
                        pending.push(Task::Value(field));
                    }
                },
                | ValueType::Thunk(ref grade, ref body) => {
                    if *grade != Grade::OMEGA {
                        return Err(FormationError::GradedBridge {
                            grade: Some(*grade),
                            effects: None,
                        });
                    }
                    pending.push(Task::FinishValue(ValueFinish::Thunk));
                    pending.push(Task::Comp(body));
                },
                | ValueType::Stk(ref consumed, ref delivered) => {
                    pending.push(Task::FinishValue(ValueFinish::Join {
                        count: ResultCount::from(2_usize),
                        expected: GroundSort::Computation,
                        result: GroundSort::Value,
                    }));
                    pending.push(Task::Comp(delivered));
                    pending.push(Task::Comp(consumed));
                },
                | ValueType::Path { ref ty, .. } => {
                    pending.push(Task::FinishValue(ValueFinish::Join {
                        count: ResultCount::from(1_usize),
                        expected: GroundSort::Value,
                        result: GroundSort::Value,
                    }));
                    pending.push(Task::Value(ty));
                },
                | ValueType::Data { ref args, .. } => {
                    pending.push(Task::FinishValue(ValueFinish::Join {
                        count: ResultCount::from(args.len()),
                        expected: GroundSort::Value,
                        result: GroundSort::Value,
                    }));
                    for arg in args.iter().rev() {
                        pending.push(Task::Value(arg));
                    }
                },
                | ValueType::Universe {
                    ref sort,
                    ref level,
                } => {
                    if !bool::from(ctx.sort_is_bound(sort)) {
                        return Err(FormationError::AbstractSort(sort.clone()));
                    }
                    if matches!(sort, SortExpr::Param(_)) {
                        return Err(FormationError::AbstractSort(sort.clone()));
                    }
                    ctx.check_level(level)?;
                    let level = level.succ().map_err(FormationError::LevelOverflow)?;
                    results.push(value_classifier(level));
                },
                | ValueType::Family(ref application) => {
                    let head = application.neutral().head_name();
                    let Some(signature) = ctx.family_signature(head).cloned()
                    else {
                        return Err(FormationError::UnsupportedForm(
                            UnsupportedForm::UnboundTypeFamily,
                        ));
                    };
                    let expected = signature.arity();
                    let args = application.neutral().arguments();
                    let actual = FamilyArity::from(args.len());
                    if actual != expected {
                        return Err(FormationError::FamilyArity {
                            expected: usize::from(expected),
                            actual: usize::from(actual),
                        });
                    }
                    for (position, (argument, expected)) in args
                        .iter()
                        .zip(signature.argument_classifiers())
                        .enumerate()
                    {
                        let actual = match argument {
                            | &StaticArg::Value(ref value) => {
                                family_argument_classifier(value.as_ref())?
                            },
                            | _ => {
                                return Err(FormationError::UnsupportedForm(
                                    UnsupportedForm::DependentFamilyArgument,
                                ));
                            },
                        };
                        if actual != *expected {
                            return Err(FormationError::FamilyArgumentClassifier {
                                position,
                                expected: expected.clone(),
                                actual,
                            });
                        }
                    }
                    let result = ctx.check_level(signature.result().level()).and_then(|()| {
                        require_sort(signature.result().clone(), GroundSort::Value)
                    })?;
                    results.push(result);
                },
                | ValueType::Sealed(ref seal) => {
                    let classifier = ctx.sealed_type(seal)?;
                    results.push(require_sort(classifier, GroundSort::Value)?);
                },
                | ValueType::Package { ref payload, .. } => {
                    pending.push(Task::FinishValue(ValueFinish::Join {
                        count: ResultCount::from(1_usize),
                        expected: GroundSort::Value,
                        result: GroundSort::Value,
                    }));
                    pending.push(Task::Value(payload));
                },
            },
            | Task::Comp(comp_ty) => match *comp_ty {
                | CompType::F(ref produced, ref row) => {
                    if !bool::from(row.is_empty()) {
                        return Err(FormationError::GradedBridge {
                            grade: None,
                            effects: Some(row.clone()),
                        });
                    }
                    pending.push(Task::FinishComp(CompFinish::Returner));
                    pending.push(Task::Value(produced));
                },
                | CompType::Arrow {
                    ref arg, ref res, ..
                } => {
                    pending.push(Task::FinishComp(CompFinish::Arrow));
                    pending.push(Task::Comp(res));
                    pending.push(Task::Value(arg));
                },
                | CompType::With(ref fst, ref snd) => {
                    pending.push(Task::FinishComp(CompFinish::With));
                    pending.push(Task::Comp(snd));
                    pending.push(Task::Comp(fst));
                },
                | CompType::Family(ref application) => {
                    let result = ctx
                        .check_level(application.result().level())
                        .and_then(|()| {
                            require_sort(application.result().clone(), GroundSort::Computation)
                        })?;
                    results.push(result);
                },
                | CompType::Unknown => {
                    results.push(comp_classifier(Level::zero()));
                },
            },
            | Task::FinishValue(ValueFinish::Join {
                count,
                expected,
                result,
            }) => {
                let children = take_results(&mut results, count)?;
                let mut level = Level::zero();
                for child in children {
                    let child = require_sort(child, expected)?;
                    level = level.max(child.level());
                }
                results.push(Classifier::new(result, level));
            },
            | Task::FinishValue(ValueFinish::Thunk) => {
                let body = take_results(&mut results, ResultCount::from(1_usize))?;
                let body = require_sort(first_result(body)?, GroundSort::Computation)?;
                results.push(value_classifier(body.level().clone()));
            },
            | Task::FinishComp(CompFinish::Returner) => {
                let produced = take_results(&mut results, ResultCount::from(1_usize))?;
                let produced = require_sort(first_result(produced)?, GroundSort::Value)?;
                results.push(comp_classifier(produced.level().clone()));
            },
            | Task::FinishComp(CompFinish::Arrow) => {
                let children = take_results(&mut results, ResultCount::from(2_usize))?;
                let mut children = children.into_iter();
                let arg = require_sort(next_result(&mut children)?, GroundSort::Value)?;
                let res = require_sort(next_result(&mut children)?, GroundSort::Computation)?;
                results.push(comp_classifier(arg.level().max(res.level())));
            },
            | Task::FinishComp(CompFinish::With) => {
                let children = take_results(&mut results, ResultCount::from(2_usize))?;
                let mut children = children.into_iter();
                let fst = require_sort(next_result(&mut children)?, GroundSort::Computation)?;
                let snd = require_sort(next_result(&mut children)?, GroundSort::Computation)?;
                results.push(comp_classifier(fst.level().max(snd.level())));
            },
        }
    }
    first_result(results)
}

/// Classify a family argument, rejecting dependent value arguments at this
/// rung.
fn family_argument_classifier(argument: &Value) -> Result<Classifier, FormationError>
{
    match argument {
        | &Value::Unit | &Value::Int(_) | &Value::Str(_) | &Value::Num(_) => {
            Ok(value_classifier(Level::zero()))
        },
        | _ => Err(FormationError::UnsupportedForm(
            UnsupportedForm::DependentFamilyArgument,
        )),
    }
}

/// Construct a value classifier at `level`.
fn value_classifier(level: Level) -> Classifier
{
    Classifier::new(GroundSort::Value, level)
}

/// Construct a computation classifier at `level`.
fn comp_classifier(level: Level) -> Classifier
{
    Classifier::new(GroundSort::Computation, level)
}

/// Require `classifier` to have the expected ground sort.
fn require_sort(
    classifier: Classifier,
    expected: GroundSort,
) -> Result<Classifier, FormationError>
{
    match classifier.sort() {
        | &SortExpr::Ground(actual) if actual == expected => Ok(classifier),
        | &SortExpr::Ground(actual) => Err(FormationError::WrongSort { expected, actual }),
        | &SortExpr::Param(_) => Err(FormationError::AbstractSort(classifier.sort().clone())),
    }
}

/// Remove the last `count` child results from the worklist result stack.
fn take_results(
    results: &mut Vec<Classifier>,
    count: ResultCount,
) -> Result<Vec<Classifier>, FormationError>
{
    let count: usize = count.into();
    let Some(start) = results.len().checked_sub(count)
    else {
        return Err(FormationError::UnsupportedForm(
            UnsupportedForm::ResultStackUnderflow,
        ));
    };
    Ok(results.split_off(start))
}

/// Return the sole result, rejecting malformed stack cardinality.
fn first_result(mut results: Vec<Classifier>) -> Result<Classifier, FormationError>
{
    if results.len() == 1 {
        return Ok(results.remove(0));
    }
    Err(FormationError::UnsupportedForm(
        UnsupportedForm::ResultStackCardinality,
    ))
}

/// Return the next child result, rejecting underflow.
fn next_result(results: &mut alloc::vec::IntoIter<Classifier>)
-> Result<Classifier, FormationError>
{
    results.next().ok_or(FormationError::UnsupportedForm(
        UnsupportedForm::ResultStackUnderflow,
    ))
}

#[cfg(test)]
mod tests
{
    use alloc::rc::Rc;

    use gandr_core_term::boundary::NameRef;
    use gandr_core_term::static_term::FamilyApp;
    use gandr_core_term::static_term::StaticNeutral;
    use gandr_core_term::static_term::StaticVar;
    use gandr_core_term::types::CompTypeTag;
    use gandr_core_term::types::DataId;
    use gandr_core_term::types::SealId;
    use gandr_core_term::types::ValueTypeTag;
    use gandr_kernel_strata::LevelConstant;

    use super::*;
    use crate::formation::context::FamilySignature;

    fn value_at(level: Level) -> Classifier
    {
        Classifier::new(GroundSort::Value, level)
    }
    fn family(
        name: NameRef<'_>,
        arguments: Vec<Rc<Value>>,
    ) -> ValueType
    {
        let neutral = arguments.into_iter().map(StaticArg::Value).fold(
            StaticNeutral::head(StaticVar::new(name.as_ref())),
            StaticNeutral::app,
        );
        ValueType::family(FamilyApp::new(neutral, value_at(Level::zero())))
    }

    fn context() -> FormationContext
    {
        let zero = value_at(Level::zero());
        let mut ctx = FormationContext::new();
        ctx.bind_type_variable("A", zero.clone());
        ctx.bind_type_variable("Integer", zero.clone());
        ctx.bind_sealed_type(SealId::new(0_u64, "M", "A"), zero.clone());
        ctx.bind_family("F", FamilySignature::new(vec![zero.clone()], zero));
        ctx
    }

    fn value_samples() -> Vec<(ValueTypeTag, ValueType)>
    {
        let sealed = SealId::new(0_u64, "M", "A");
        vec![
            (ValueTypeTag::Atom, ValueType::atom("A")),
            (ValueTypeTag::Unit, ValueType::Unit),
            (
                ValueTypeTag::Prod,
                ValueType::prod(ValueType::Unit, ValueType::Unit),
            ),
            (
                ValueTypeTag::Sum,
                ValueType::sum(ValueType::Unit, ValueType::Unit),
            ),
            (ValueTypeTag::List, ValueType::list(ValueType::Unit)),
            (
                ValueTypeTag::Record,
                ValueType::record([("field".to_owned(), ValueType::Unit)]),
            ),
            (
                ValueTypeTag::Thunk,
                ValueType::thunk(Grade::OMEGA, CompType::returner(ValueType::Unit)),
            ),
            (
                ValueTypeTag::Stk,
                ValueType::stk(
                    CompType::returner(ValueType::Unit),
                    CompType::returner(ValueType::Unit),
                ),
            ),
            (
                ValueTypeTag::Path,
                ValueType::path(ValueType::Unit, Value::Unit, Value::Unit),
            ),
            (
                ValueTypeTag::Data,
                ValueType::data(DataId::new(0_u64, "D"), vec![ValueType::Unit]),
            ),
            (
                ValueTypeTag::Universe,
                ValueType::universe(GroundSort::Value, Level::zero()),
            ),
            (
                ValueTypeTag::Sigma,
                ValueType::sigma(ValueType::Unit, "x", ValueType::Unit),
            ),
            (
                ValueTypeTag::Family,
                family(NameRef::from("F"), vec![Rc::new(Value::Unit)]),
            ),
            (ValueTypeTag::Sealed, ValueType::Sealed(sealed)),
            (
                ValueTypeTag::Package,
                ValueType::package(Grade::OMEGA, ["A"], ValueType::Unit),
            ),
            (ValueTypeTag::Unknown, ValueType::Unknown),
        ]
    }

    fn comp_samples() -> Vec<(CompTypeTag, CompType)>
    {
        vec![
            (CompTypeTag::F, CompType::returner(ValueType::Unit)),
            (
                CompTypeTag::Arrow,
                CompType::arrow(ValueType::Unit, CompType::returner(ValueType::Unit)),
            ),
            (
                CompTypeTag::Family,
                CompType::Family(FamilyApp::new(
                    StaticNeutral::head(StaticVar::new("F")),
                    Classifier::new(GroundSort::Computation, Level::zero()),
                )),
            ),
            (
                CompTypeTag::With,
                CompType::with(
                    CompType::returner(ValueType::Unit),
                    CompType::returner(ValueType::Unit),
                ),
            ),
            (CompTypeTag::Unknown, CompType::Unknown),
        ]
    }

    /// Both universe families form one level up, in the value sort, and are
    /// distinct types at that level.
    #[test]
    fn universe_families_form_one_level_up_in_the_value_sort()
    {
        let ctx = context();
        let value = ValueType::universe(GroundSort::Value, Level::zero());
        let computation = ValueType::universe(GroundSort::Computation, Level::zero());
        assert_ne!(value, computation);
        let expected = value_at(Level::constant(LevelConstant::from(1_u64)));
        assert_eq!(Ok(expected.clone()), value.infer_classifier(&ctx));
        assert_eq!(Ok(expected), computation.infer_classifier(&ctx));
    }

    /// Every value-type constructor in `ValueTypeTag::ALL` is answered by
    /// a rule, and none reaches the catch-all.
    #[test]
    fn every_value_type_constructor_has_a_formation_rule()
    {
        let ctx = context();
        let samples = value_samples();
        assert_eq!(ValueTypeTag::ALL.len(), samples.len());
        for (tag, value_ty) in samples {
            assert_eq!(tag, value_ty.tag());
            assert!(!matches!(
                value_ty.infer_classifier(&ctx),
                Err(FormationError::UnsupportedForm(_))
            ));
        }
    }

    /// Every computation-type constructor in `CompTypeTag::ALL` is answered
    /// by a rule, and none reaches the catch-all.
    #[test]
    fn every_comp_type_constructor_has_a_formation_rule()
    {
        let ctx = context();
        let samples = comp_samples();
        assert_eq!(CompTypeTag::ALL.len(), samples.len());
        for (tag, comp_ty) in samples {
            assert_eq!(tag, comp_ty.tag());
            assert!(!matches!(
                comp_ty.infer_classifier(&ctx),
                Err(FormationError::UnsupportedForm(_))
            ));
        }
    }

    /// Unsupported family arguments expose a nominal failure kind rather than
    /// requiring callers to match on diagnostic prose.
    #[test]
    fn unsupported_forms_have_nominal_kinds()
    {
        let ctx = context();
        let family = family(NameRef::from("F"), vec![Rc::new(Value::Var(
            "x".to_owned(),
        ))]);
        assert_eq!(
            Err(FormationError::UnsupportedForm(
                UnsupportedForm::DependentFamilyArgument,
            )),
            family.infer_classifier(&ctx)
        );
    }

    /// The gradual unknown is answered by its own rule rather than by the
    /// unsupported-form fallthrough.
    #[test]
    fn the_gradual_unknown_is_a_rule_not_a_fallthrough()
    {
        let ctx = context();
        assert_eq!(
            Ok(value_at(Level::zero())),
            ValueType::Unknown.infer_classifier(&ctx)
        );
        assert_eq!(
            Ok(comp_classifier(Level::zero())),
            CompType::Unknown.infer_classifier(&ctx)
        );
    }

    /// An arrow forms at the join of its domain and codomain levels, computed
    /// through the level algebra.
    #[test]
    fn arrow_forms_at_the_join_of_its_premise_levels()
    {
        let mut ctx = context();
        ctx.bind_type_variable("A", value_at(Level::constant(LevelConstant::from(1_u64))));
        ctx.bind_type_variable("B", value_at(Level::constant(LevelConstant::from(2_u64))));
        let arrow = CompType::arrow(
            ValueType::atom("A"),
            CompType::returner(ValueType::atom("B")),
        );
        assert_eq!(
            Ok(comp_classifier(Level::constant(LevelConstant::from(2_u64)))),
            arrow.infer_classifier(&ctx)
        );
    }

    /// A family applied at the wrong arity raises the exact arity variant.
    #[test]
    fn family_applied_at_wrong_arity_raises_the_exact_variant()
    {
        let ctx = context();
        let family = family(NameRef::from("F"), vec![
            Rc::new(Value::Unit),
            Rc::new(Value::Unit),
        ]);
        assert_eq!(
            Err(FormationError::FamilyArity {
                expected: 1,
                actual: 2,
            }),
            family.infer_classifier(&ctx)
        );
    }

    /// A family argument at the wrong classifier raises the exact variant.
    #[test]
    fn family_argument_at_wrong_classifier_raises_the_exact_variant()
    {
        let mut ctx = FormationContext::new();
        ctx.bind_family(
            "F",
            FamilySignature::new(
                vec![Classifier::new(GroundSort::Computation, Level::zero())],
                value_at(Level::zero()),
            ),
        );
        let family = family(NameRef::from("F"), vec![Rc::new(Value::Unit)]);
        assert!(matches!(
            family.infer_classifier(&ctx),
            Err(FormationError::FamilyArgumentClassifier { position: 0, .. })
        ));
    }

    /// An abstract sort has no ground reading and raises the exact variant.
    #[test]
    fn abstract_sort_raises_the_exact_variant()
    {
        let ctx = context();
        let universe = ValueType::universe(
            SortExpr::Param(gandr_core_term::classifier::SortParam::new("s")),
            Level::zero(),
        );
        assert!(matches!(
            universe.infer_classifier(&ctx),
            Err(FormationError::AbstractSort(_))
        ));
    }
}
