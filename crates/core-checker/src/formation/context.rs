//! The scopes formation classifies a type against.

use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;

use gandr_core_term::boundary::FamilyArity;
use gandr_core_term::boundary::TypeAtomName;
use gandr_core_term::boundary::TypeFamilyName;
use gandr_core_term::classifier::Classifier;
use gandr_core_term::classifier::GroundSort;
use gandr_core_term::classifier::SortExpr;
use gandr_core_term::ctx::Ctx;
use gandr_core_term::error::FormationError;
use gandr_core_term::types::SealId;
use gandr_core_term::types::ValueType;
use gandr_kernel_strata::Level;
use gandr_kernel_strata::LevelVar;

/// Whether an abstract sort parameter is bound in a formation context.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct SortBoundStatus(bool);

impl From<bool> for SortBoundStatus
{
    #[inline]
    fn from(value: bool) -> Self
    {
        Self(value)
    }
}

impl From<SortBoundStatus> for bool
{
    #[inline]
    fn from(status: SortBoundStatus) -> Self
    {
        status.0
    }
}

/// The declared telescope of a type family.
///
/// Formation keeps the telescope's classifier contract beside the family
/// name. The arguments are checked in declaration order before the result is
/// returned.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FamilySignature
{
    /// Classifiers required by the family telescope, in declaration order.
    argument_classifiers: Vec<Classifier>,
    /// Classifier produced by the family after its arguments are formed.
    result: Classifier,
}

impl FamilySignature
{
    /// Builds a family signature from its argument classifiers and result.
    #[must_use]
    #[inline]
    pub fn new(
        argument_classifiers: Vec<Classifier>,
        result: Classifier,
    ) -> Self
    {
        Self {
            argument_classifiers,
            result,
        }
    }

    /// The classifiers required by the family telescope.
    #[must_use]
    #[inline]
    pub fn argument_classifiers(&self) -> &[Classifier]
    {
        &self.argument_classifiers
    }

    /// The family result classifier.
    #[must_use]
    #[inline]
    pub const fn result(&self) -> &Classifier
    {
        &self.result
    }

    /// The number of arguments the family accepts.
    #[must_use]
    #[inline]
    pub fn arity(&self) -> FamilyArity
    {
        FamilyArity::from(self.argument_classifiers.len())
    }
}

/// What is in scope when a type is formed.
///
/// It carries four scopes and deliberately not a fifth:
///
/// - **level variables**, so a level mentioning a bound variable is in scope
///   and one mentioning a freed variable is the escaping-level failure;
/// - **sort variables**, so an abstract sort is a named refusal rather than a
///   guessed ground reading;
/// - **type variables**, each mapped to the classifier it was bound at, which
///   is what makes a variable's rule a lookup rather than an inference;
/// - **family signatures**, so a family application is checked against a
///   declared telescope for arity and for each argument's classifier.
///
/// There is no value scope for dependent term indices at this rung. Dependent
/// value indices arrive with the erased static core, and an empty scope now
/// would be a path with no producer.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct FormationContext
{
    /// Bound level variables permitted in classifiers.
    level_variables: BTreeSet<LevelVar>,
    /// Declared abstract sort parameters.
    sort_variables: BTreeSet<String>,
    /// Type variables and the classifiers recorded for them.
    type_variables: BTreeMap<String, Classifier>,
    /// Sealed nominal types and the classifiers recorded for them.
    sealed_types: BTreeMap<SealId, Classifier>,
    /// Declared type-family signatures.
    family_signatures: BTreeMap<String, FamilySignature>,
}

impl FormationContext
{
    /// An empty formation context.
    ///
    /// # Contract
    /// - ensures: every scope is empty, so every lookup fails with its own
    ///   named error.
    /// - panics: none.
    #[must_use]
    #[inline]
    pub fn new() -> Self
    {
        Self::default()
    }

    /// The formation scopes a checking context supplies.
    ///
    /// This is the producer the judgement was missing. Formation was landed
    /// total and correct over five scopes — levels, sorts, type variables,
    /// seals, families — and the only context that had ever been built for it
    /// was the one its own unit tests hand-assemble, so nothing on the
    /// checking path could call it.
    ///
    /// # What it binds, and on whose authority
    ///
    /// - **The rigid base atoms**, at `value/0`. These are axioms: the checker
    ///   and the machine type every literal with them, and `core-term` calls
    ///   each constructor the single point of truth for its spelling. They are
    ///   read off those constructors rather than restated, so a renamed atom
    ///   cannot silently start reading here as an undeclared name.
    /// - **Every universe-typed hypothesis.** A hypothesis `x : Type[s, l]`
    ///   says `x` is a type at level `l`, so `x` enters the type-variable scope
    ///   at that classifier and `l`'s variables enter the level scope. This is
    ///   a fact the context already carries, not an inference about the type
    ///   being formed.
    ///
    /// # What it deliberately does not bind, which is the load-bearing part
    ///
    /// **Nothing from the type being formed.** A producer that seeded itself
    /// from its own subject would bind every name the subject mentions, and
    /// [`FormationError::UnboundName`] would become unreachable — the
    /// judgement would answer `Ok` for every type it was ever handed, and the
    /// consumer built on it would be a rule whose trigger condition can never
    /// be met. That is the exact defect this whole arc exists to remove, and
    /// it is reachable from here by an edit that looks like a convenience.
    ///
    /// The witness holding this property is
    /// `tests::seeding_the_context_from_its_own_subject_silences_the_refusal`,
    /// which performs that edit and shows the refusal disappear.
    ///
    /// # Contract
    /// - ensures: every rigid base atom resolves, and every universe-typed
    ///   hypothesis of `ctx` resolves at the classifier its universe states.
    /// - ensures: a name `ctx` does not declare is left unbound, so formation
    ///   refuses it by name.
    /// - panics: none.
    #[must_use]
    #[inline]
    pub fn from_checking_context(ctx: &Ctx) -> Self
    {
        let mut formation = Self::new();
        for atom in rigid_base_atoms() {
            if let ValueType::Atom(ref name) = atom {
                formation.bind_type_variable(
                    name.clone(),
                    Classifier::new(GroundSort::Value, Level::zero()),
                );
            }
        }
        for &(ref name, ref ty) in ctx.bindings() {
            if let ValueType::Universe {
                sort: SortExpr::Ground(ground),
                ref level,
            } = *ty
            {
                for (variable, _) in level.atoms() {
                    formation.bind_level_variable(variable);
                }
                formation.bind_type_variable(name.clone(), Classifier::new(ground, level.clone()));
            }
        }
        formation
    }

    /// Bind a level variable in this context.
    #[inline]
    pub fn bind_level_variable(
        &mut self,
        variable: LevelVar,
    ) -> &mut Self
    {
        self.level_variables.insert(variable);
        self
    }

    /// Bind a sort parameter in this context.
    #[inline]
    pub fn bind_sort_variable<Name>(
        &mut self,
        name: Name,
    ) -> &mut Self
    where
        Name: Into<String>,
    {
        self.sort_variables.insert(name.into());
        self
    }

    /// Bind a type variable at `classifier`.
    #[inline]
    pub fn bind_type_variable<Name>(
        &mut self,
        name: Name,
        classifier: Classifier,
    ) -> &mut Self
    where
        Name: Into<String>,
    {
        self.type_variables.insert(name.into(), classifier);
        self
    }

    /// Bind a sealed nominal type at `classifier`.
    #[inline]
    pub fn bind_sealed_type(
        &mut self,
        seal: SealId,
        classifier: Classifier,
    ) -> &mut Self
    {
        self.sealed_types.insert(seal, classifier);
        self
    }

    /// Bind a family name to its declared signature.
    #[inline]
    pub fn bind_family<Name>(
        &mut self,
        name: Name,
        signature: FamilySignature,
    ) -> &mut Self
    where
        Name: Into<String>,
    {
        self.family_signatures.insert(name.into(), signature);
        self
    }

    /// The classifier a type variable was bound at.
    ///
    /// # Contract
    /// - ensures: returns the classifier the binder recorded for `name`.
    /// - fails: a name no binder introduced is a named refusal, never a guessed
    ///   classifier.
    /// - panics: none.
    ///
    /// # Errors
    /// [`FormationError::UnboundName`] when `name` is not bound.
    /// [`FormationError::EscapingLevel`] when the binding carries a free level
    /// variable.
    #[inline]
    pub fn type_variable(
        &self,
        name: TypeAtomName<'_>,
    ) -> Result<Classifier, FormationError>
    {
        let name: &str = name.as_ref();
        let Some(classifier) = self.type_variables.get(name)
        else {
            return Err(FormationError::UnboundName(name.to_owned()));
        };
        self.check_level(classifier.level())?;
        Ok(classifier.clone())
    }

    /// The classifier a sealed nominal type was bound at.
    pub(crate) fn sealed_type(
        &self,
        seal: &SealId,
    ) -> Result<Classifier, FormationError>
    {
        let Some(classifier) = self.sealed_types.get(seal)
        else {
            return Err(FormationError::UnboundName(alloc::format!("{seal:?}")));
        };
        self.check_level(classifier.level())?;
        Ok(classifier.clone())
    }

    /// The signature declared for a family name.
    pub(crate) fn family_signature(
        &self,
        name: TypeFamilyName<'_>,
    ) -> Option<&FamilySignature>
    {
        let name: &str = name.as_ref();
        self.family_signatures.get(name)
    }

    /// Check that every level atom is bound in this context.
    pub(crate) fn check_level(
        &self,
        level: &Level,
    ) -> Result<(), FormationError>
    {
        if level
            .atoms()
            .any(|(variable, _)| !self.level_variables.contains(&variable))
        {
            return Err(FormationError::EscapingLevel(level.clone()));
        }
        Ok(())
    }

    /// Check whether an abstract sort parameter is declared in this context.
    pub(crate) fn sort_is_bound(
        &self,
        sort: &SortExpr,
    ) -> SortBoundStatus
    {
        match sort {
            | &SortExpr::Ground(_) => SortBoundStatus::from(true),
            | &SortExpr::Param(ref parameter) => {
                SortBoundStatus::from(self.sort_variables.contains(parameter.name().as_ref()))
            },
        }
    }
}

/// The rigid base atoms, read off the constructors that own their spellings.
///
/// `core-term` documents each of these as "the single point of truth for the
/// atom's spelling". Restating the eight strings here would create a second
/// copy to drift, and the drift would be silent in the worst direction: a
/// renamed atom would stop being declared and start being refused as a name
/// nobody wrote down.
fn rigid_base_atoms() -> [ValueType; 8]
{
    [
        ValueType::integer(),
        ValueType::string(),
        ValueType::u32(),
        ValueType::u64(),
        ValueType::i32(),
        ValueType::i64(),
        ValueType::f32(),
        ValueType::f64(),
    ]
}

#[cfg(test)]
mod tests
{
    use gandr_core_term::types::CompType;

    use super::*;
    use crate::formation::rules::FormType as _;

    fn value_zero() -> Classifier
    {
        Classifier::new(GroundSort::Value, Level::zero())
    }

    /// The producer declares the rigid base atoms, so the most common type in
    /// the corpus classifies rather than being refused as an undeclared name.
    ///
    /// This is the positive control for every refusal witness below: without
    /// it, a refusal would be indistinguishable from a producer that declares
    /// nothing at all.
    #[test]
    fn the_producer_declares_the_rigid_base_atoms()
    {
        let formation = FormationContext::from_checking_context(&Ctx::new());
        for atom in rigid_base_atoms() {
            assert_eq!(
                Ok(value_zero()),
                atom.infer_classifier(&formation),
                "a rigid base atom is an axiom the checker types literals with, so formation \
                 must classify it: {atom:?}"
            );
        }
    }

    /// A name no scope declares is refused **by name**, which is the whole
    /// point: the elaborator accepts an undeclared type name as a rigid atom,
    /// so `Atom("NoSuchType")` and `Atom("Integer")` are structurally
    /// identical and only a declaring scope can tell them apart.
    #[test]
    fn an_undeclared_type_name_is_refused_by_name()
    {
        let formation = FormationContext::from_checking_context(&Ctx::new());
        assert_eq!(
            Err(FormationError::UnboundName("NoSuchType".to_owned())),
            ValueType::atom("NoSuchType").infer_classifier(&formation),
            "an undeclared type name is a named refusal, never a guessed classifier"
        );
    }

    /// **The ablation for the producer's load-bearing property.**
    ///
    /// [`FormationContext::from_checking_context`] binds nothing from the type
    /// being formed. This test performs the edit that would break that — it
    /// seeds the subject's own atom into the context — and shows the refusal
    /// disappear.
    ///
    /// The mutation is deliberately one that **never existed**: this is a
    /// reachability probe for the guard, not a reconstruction of an earlier
    /// engine, so the requirement that an ablation reproduce a real historical
    /// configuration does not apply and would make the property untestable.
    /// The question is only whether the property is doing work, and the answer
    /// is the pair of values below: refused without the seeding, `Ok` with it.
    #[test]
    fn seeding_the_context_from_its_own_subject_silences_the_refusal()
    {
        let subject = ValueType::atom("NoSuchType");

        let honest = FormationContext::from_checking_context(&Ctx::new());
        let refused = subject.infer_classifier(&honest);
        assert_eq!(
            Err(FormationError::UnboundName("NoSuchType".to_owned())),
            refused,
            "the producer under test refuses a name nothing declared"
        );

        let mut seeded = FormationContext::from_checking_context(&Ctx::new());
        seeded.bind_type_variable("NoSuchType", value_zero());
        assert_eq!(
            Ok(value_zero()),
            subject.infer_classifier(&seeded),
            "seeding the subject's own name makes the refusal unreachable"
        );
    }

    /// A hypothesis at a universe declares a type variable: `x : Type` means
    /// `x` is a type, so `Atom("x")` classifies.
    #[test]
    fn a_universe_typed_hypothesis_becomes_a_type_variable()
    {
        let mut ctx = Ctx::new();
        ctx.bind(
            "x".to_owned(),
            ValueType::universe(SortExpr::value(), Level::zero()),
        );
        let formation = FormationContext::from_checking_context(&ctx);
        assert_eq!(
            Ok(value_zero()),
            ValueType::atom("x").infer_classifier(&formation),
            "a hypothesis at `Type` declares its name as a type variable"
        );
    }

    /// **The discriminator.** Only a *universe-typed* hypothesis declares a
    /// type variable. A hypothesis at an ordinary type is a term, and its name
    /// stays undeclared as a type — otherwise the producer would be binding
    /// every name in scope and the refusal would be unreachable again by a
    /// second route.
    #[test]
    fn a_value_typed_hypothesis_does_not_become_a_type_variable()
    {
        let mut ctx = Ctx::new();
        ctx.bind("n".to_owned(), ValueType::integer());
        let formation = FormationContext::from_checking_context(&ctx);
        assert_eq!(
            Err(FormationError::UnboundName("n".to_owned())),
            ValueType::atom("n").infer_classifier(&formation),
            "a term hypothesis is not a type declaration"
        );
    }

    /// A graded bridge stays a refusal from the judgement — the producer does
    /// not admit it — and it is the consumer that classifies it as a
    /// capability boundary rather than as a fact about the source.
    #[test]
    fn the_producer_does_not_admit_a_graded_bridge()
    {
        let formation = FormationContext::from_checking_context(&Ctx::new());
        let graded = ValueType::thunk(
            gandr_core_term::grade::Grade::ONE,
            CompType::returner(ValueType::integer()),
        );
        assert!(
            matches!(
                graded.infer_classifier(&formation),
                Err(FormationError::GradedBridge { .. })
            ),
            "a non-omega thunk grade is outside the admitted fragment"
        );
    }
}
