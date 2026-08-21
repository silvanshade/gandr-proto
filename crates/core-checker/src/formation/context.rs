//! The scopes formation classifies a type against.

use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;

use gandr_core_term::boundary::FamilyArity;
use gandr_core_term::boundary::TypeAtomName;
use gandr_core_term::boundary::TypeFamilyName;
use gandr_core_term::classifier::Classifier;
use gandr_core_term::classifier::SortExpr;
use gandr_core_term::error::FormationError;
use gandr_core_term::types::SealId;
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
