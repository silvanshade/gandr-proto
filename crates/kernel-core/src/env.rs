//! The append-only environment and its single choke point (kernel-boundary.md
//! §3, K3): [`Environment::add_decl`] is the **only** way a declaration enters
//! checked, [`Environment::add_decl_unchecked`] is the **one** warned bypass,
//! and [`Environment::audit`] is the `#print axioms` analogue.
//!
//! A [`CheckedId`] is **unforgeable outside this crate** — its constructor is
//! private — so "this declaration was admitted" is a type-level fact in
//! consuming code, not a runtime claim (K3, the adequacy ladder's L0 applied to
//! trust itself). There is a **single checked/unchecked bit**, never a trust
//! lattice.

use alloc::collections::BTreeSet;
use alloc::vec::Vec;

use crate::check::Checker;
use crate::decl::Declaration;
use crate::decl::DeclarationContent;
use crate::error::KernelError;
use crate::levels::LevelContext;
use crate::term::Computation;
use crate::term::ConstantIndex;
use crate::term::Value;
use crate::types::ValueType;

/// An unforgeable handle to a declaration admitted into an [`Environment`].
///
/// The wrapped position is private and the constructor is `pub(crate)`, so no
/// code outside this crate can mint a `CheckedId` — holding one is proof the
/// declaration crossed the choke point (K3).
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CheckedId(ConstantIndex);

impl CheckedId
{
    /// Mint a checked id for an admitted position (crate-internal only).
    #[inline]
    #[must_use]
    pub(crate) const fn new(position: ConstantIndex) -> Self
    {
        Self(position)
    }

    /// The declaration's admission position in the environment.
    #[inline]
    #[must_use]
    pub const fn position(self) -> ConstantIndex
    {
        self.0
    }
}

/// How a declaration was admitted: through the checked choke point or the
/// warned bypass. A single bit — never a trust lattice (K3).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Admission
{
    /// Admitted through [`Environment::add_decl`] — fully checked.
    Checked,
    /// Admitted through [`Environment::add_decl_unchecked`] — trusted, tracked.
    Unchecked,
}

/// One admitted declaration and its audit metadata.
#[derive(Clone, Debug)]
struct AdmittedDeclaration
{
    /// The declaration itself.
    declaration: Declaration,
    /// Whether it was checked or bypassed.
    admission: Admission,
    /// The transitive set of positions of axioms and unchecked admissions this
    /// declaration rests on (including itself when it is one). Precomputed at
    /// admission, so the audit is a lookup.
    rested_on: BTreeSet<ConstantIndex>,
}

/// The transitive audit of a declaration: the axioms and unchecked admissions
/// it rests on (kernel-boundary.md §3, the `#print axioms` analogue).
///
/// A declaration whose report is empty on both faces rests on nothing outside
/// the checked kernel.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AxiomReport
{
    /// The positions of axioms it transitively rests on, ascending.
    axioms: Vec<ConstantIndex>,
    /// The positions of unchecked admissions it transitively rests on,
    /// ascending.
    unchecked: Vec<ConstantIndex>,
}

impl AxiomReport
{
    /// The axioms this declaration transitively rests on, ascending.
    #[inline]
    #[must_use]
    pub fn axioms(&self) -> &[ConstantIndex]
    {
        &self.axioms
    }

    /// The unchecked admissions this declaration transitively rests on,
    /// ascending.
    #[inline]
    #[must_use]
    pub fn unchecked_admissions(&self) -> &[ConstantIndex]
    {
        &self.unchecked
    }
}

/// The append-only kernel environment: a sequence of admitted declarations in
/// admission order (kernel-boundary.md §3; the export format's E2 ordering).
#[derive(Clone, Debug, Default)]
pub struct Environment
{
    /// The admitted declarations, in admission order.
    entries: Vec<AdmittedDeclaration>,
}

impl Environment
{
    /// An empty environment.
    #[inline]
    #[must_use]
    pub fn new() -> Self
    {
        Self {
            entries: Vec::new(),
        }
    }

    /// Admit a declaration through the checked choke point.
    ///
    /// # Contract
    /// - requires: nothing — every well-formedness and typing obligation is
    ///   re-derived here (K2), granting the producer no credence.
    /// - ensures: `Ok(id)` with an unforgeable [`CheckedId`] exactly when the
    ///   declaration's level signature admits (a consistent landmark poset),
    ///   its declared type is well-formed, and (for a `Def`) its body checks
    ///   against that type; the declaration is appended and its audit
    ///   precomputed.
    /// - provides: the K3 choke point — the only checked entry into the kernel.
    /// - fails: any [`KernelError`] the level admission or the checker
    ///   surfaces; the environment is left unchanged on failure.
    /// - panics: none.
    ///
    /// # Errors
    /// Any [`KernelError`].
    ///
    /// # Adequacy
    /// - hypothesis: L1/L2 — a `CheckedId` is unforgeable (L0 type-level), and
    ///   the corpus differential pins acceptance of well-typed declarations and
    ///   rejection of ill-typed ones; the L3 residues are the
    ///   constant-reference resolution (a forward reference is rejected) and
    ///   the append-on-success / unchanged-on-failure boundary.
    /// - witness: `env::tests::a_definition_referencing_a_prior_one_checks`
    /// - witness: `env::tests::a_forward_constant_reference_is_unbound`
    /// - witness: `env::tests::a_rejected_declaration_leaves_the_environment_unchanged`
    #[inline]
    pub fn add_decl(
        &mut self,
        declaration: Declaration,
    ) -> Result<CheckedId, KernelError>
    {
        let position = ConstantIndex::from(self.entries.len());
        let levels = LevelContext::admit(
            declaration.levels().params(),
            declaration.levels().constraints().to_vec(),
        )?;
        Self::check_content(self, &levels, &declaration)?;
        let rested_on = self.transitive_rest(declaration.content(), position, Admission::Checked);
        self.entries.push(AdmittedDeclaration {
            declaration,
            admission: Admission::Checked,
            rested_on,
        });
        Ok(CheckedId::new(position))
    }

    /// Admit a declaration through the **single warned bypass**, unchecked.
    ///
    /// # Soundness warning
    ///
    /// This is the one escape hatch, present from day one so no second bypass
    /// is ever improvised (K3). It performs **no** type checking and **no**
    /// level admission: the declaration is trusted verbatim. A wrong
    /// declaration admitted here can make the kernel prove anything —
    /// exactly the Lean `addDecl`-unchecked / `native_decide` hazard. Every
    /// admission through it is **tracked**: the declaration is marked
    /// unchecked and surfaces in the [`Environment::audit`] of everything
    /// that transitively rests on it. Use it only for a deliberately
    /// trusted axiomatic base, never as a shortcut around a checker
    /// rejection.
    ///
    /// # Contract
    /// - requires: the caller vouches for the declaration; nothing is verified.
    /// - ensures: the declaration is appended, marked [`Admission::Unchecked`],
    ///   and included in the audit of every dependent; a [`CheckedId`] is
    ///   returned.
    /// - provides: the tracked, warned bypass of K3.
    /// - fails: never — the bypass does not check.
    /// - panics: none.
    #[inline]
    pub fn add_decl_unchecked(
        &mut self,
        declaration: Declaration,
    ) -> CheckedId
    {
        let position = ConstantIndex::from(self.entries.len());
        let rested_on = self.transitive_rest(declaration.content(), position, Admission::Unchecked);
        self.entries.push(AdmittedDeclaration {
            declaration,
            admission: Admission::Unchecked,
            rested_on,
        });
        CheckedId::new(position)
    }

    /// The transitive audit of an admitted declaration.
    ///
    /// # Contract
    /// - requires: `id` was returned by this environment.
    /// - ensures: the axioms and unchecked admissions the declaration
    ///   transitively rests on, each ascending; an id this environment did not
    ///   issue yields an empty report.
    /// - provides: the `#print axioms` analogue — every escape hatch auditable
    ///   per declaration (K3).
    /// - fails: never.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L2 — the transitive closure is pinned by a chain (a def on
    ///   a def on an axiom must report the axiom) and by an unchecked admission
    ///   surfacing in a dependent; the L3 residue is the checked-def-on-nothing
    ///   empty report.
    /// - witness: `env::tests::audit_reports_the_transitive_axiom`
    /// - witness: `env::tests::audit_reports_a_transitive_unchecked_admission`
    /// - witness: `env::tests::a_closed_definition_rests_on_nothing`
    #[inline]
    #[must_use]
    pub fn audit(
        &self,
        id: CheckedId,
    ) -> AxiomReport
    {
        let mut axioms = Vec::new();
        let mut unchecked = Vec::new();
        if let Some(entry) = self.entries.get(usize::from(id.position())) {
            for &position in &entry.rested_on {
                if let Some(ancestor) = self.entries.get(usize::from(position)) {
                    if matches!(ancestor.admission, Admission::Unchecked) {
                        unchecked.push(position);
                    }
                    if matches!(
                        ancestor.declaration.content(),
                        DeclarationContent::Axiom { .. }
                    ) {
                        axioms.push(position);
                    }
                }
            }
        }
        AxiomReport { axioms, unchecked }
    }

    /// The admitted declarations in admission order, each paired with its
    /// admission mark — the export writer's E2/E6 source (kernel-boundary.md
    /// §5).
    ///
    /// # Contract
    /// - requires: nothing.
    /// - ensures: yields every admitted declaration exactly once, in admission
    ///   order, with its checked/unchecked mark; the precomputed transitive
    ///   audit sets are **not** exposed (E3: derived data is recomputed on
    ///   replay, never serialized and trusted).
    /// - provides: the export writer's read access to the append-only log.
    /// - fails: never.
    /// - panics: none.
    #[inline]
    pub(crate) fn admitted(&self) -> impl Iterator<Item = (Admission, &Declaration)> + '_
    {
        self.entries
            .iter()
            .map(|entry| (entry.admission, &entry.declaration))
    }

    /// The declared value type of a prior admitted declaration, for the
    /// checker's constant-reference rule.
    ///
    /// # Contract
    /// - requires: nothing.
    /// - ensures: `Some(ty)` when `index` names an admitted declaration.
    /// - provides: the constant rule's resolver (a not-yet-admitted index is
    ///   `None`, which the checker turns into `UnboundConstant`).
    /// - fails: `None` for an out-of-range index.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub(crate) fn declared_value_type(
        &self,
        index: ConstantIndex,
    ) -> Option<&ValueType>
    {
        self.entries
            .get(usize::from(index))
            .map(|entry| entry.declaration.declared_type())
    }

    /// Check a declaration's content against a fresh checker over `levels`.
    #[expect(
        clippy::pattern_type_mismatch,
        reason = "ergonomic matching of a borrowed declaration node; every binding is a shared reference by intent"
    )]
    fn check_content(
        environment: &Self,
        levels: &LevelContext,
        declaration: &Declaration,
    ) -> Result<(), KernelError>
    {
        let checker = Checker::new(environment, levels);
        checker.check_value_type(declaration.declared_type())?;
        match declaration.content() {
            | DeclarationContent::Def { declared, body } => {
                checker.check_definition(declared, body)
            },
            | DeclarationContent::Axiom { .. } => Ok(()),
        }
    }

    /// Precompute the transitive set of axioms and unchecked admissions a
    /// declaration rests on.
    #[expect(
        clippy::pattern_type_mismatch,
        reason = "ergonomic matching of a borrowed declaration node; every binding is a shared reference by intent"
    )]
    fn transitive_rest(
        &self,
        content: &DeclarationContent,
        position: ConstantIndex,
        admission: Admission,
    ) -> BTreeSet<ConstantIndex>
    {
        let mut set = BTreeSet::new();
        if let DeclarationContent::Def { body, .. } = content {
            for referenced in collect_constants(body) {
                if let Some(entry) = self.entries.get(usize::from(referenced)) {
                    for &ancestor in &entry.rested_on {
                        let _fresh = set.insert(ancestor);
                    }
                }
            }
        }
        let is_axiom = matches!(*content, DeclarationContent::Axiom { .. });
        if is_axiom || matches!(admission, Admission::Unchecked) {
            let _fresh = set.insert(position);
        }
        set
    }
}

/// Collect the constant references a value body mentions, iteratively.
///
/// # Contract
/// - requires: nothing.
/// - ensures: exactly the set of [`ConstantIndex`]es reachable in the term.
/// - provides: the direct-dependency edges of the audit graph.
/// - fails: never.
/// - panics: none.
#[expect(
    clippy::pattern_type_mismatch,
    reason = "ergonomic matching of borrowed term nodes; every binding is a shared reference by intent"
)]
fn collect_constants(root: &Value) -> BTreeSet<ConstantIndex>
{
    let mut found = BTreeSet::new();
    let mut values: Vec<&Value> = Vec::new();
    let mut computations: Vec<&Computation> = Vec::new();
    values.push(root);
    loop {
        while let Some(value) = values.pop() {
            match value {
                | Value::Constant(index) => {
                    let _fresh = found.insert(*index);
                },
                | Value::Variable(_) | Value::Unit | Value::Literal(_) => {},
                | Value::Pair(first, second) => {
                    values.push(first);
                    values.push(second);
                },
                | Value::Injection(_, body) | Value::Lift { body, .. } => values.push(body),
                | Value::Thunk(body) => computations.push(body),
            }
        }
        let Some(computation) = computations.pop()
        else {
            break;
        };
        match computation {
            | Computation::Lambda(body) => computations.push(body),
            | Computation::Application(head, argument) => {
                computations.push(head);
                values.push(argument);
            },
            | Computation::Return(value) | Computation::Force(value) => values.push(value),
            | Computation::Bind(bound, body) => {
                computations.push(bound);
                computations.push(body);
            },
            | Computation::Case {
                scrutinee,
                on_left,
                on_right,
            } => {
                values.push(scrutinee);
                computations.push(on_left);
                computations.push(on_right);
            },
        }
    }
    found
}

#[cfg(test)]
mod tests
{
    use alloc::boxed::Box;

    use super::Environment;
    use crate::decl::Declaration;
    use crate::decl::LevelSignature;
    use crate::error::KernelError;
    use crate::term::Computation;
    use crate::term::ConstantIndex;
    use crate::term::Value;
    use crate::types::CompType;
    use crate::types::ValueType;

    /// The declared type `U (Unit → F Unit)` and the identity thunk body.
    fn identity_declaration() -> Declaration
    {
        let arrow = CompType::Arrow {
            domain: Box::new(ValueType::Unit),
            codomain: Box::new(CompType::Returner(Box::new(ValueType::Unit))),
        };
        let declared = ValueType::Thunk(Box::new(arrow));
        let body = Value::Thunk(Box::new(Computation::Lambda(Box::new(
            Computation::Return(Box::new(Value::Variable(crate::term::DeBruijnIndex::from(
                0_u32,
            )))),
        ))));
        Declaration::def(LevelSignature::monomorphic(), declared, body)
    }

    #[test]
    fn a_definition_referencing_a_prior_one_checks()
    {
        let mut environment = Environment::new();
        let first = environment.add_decl(identity_declaration()).unwrap();
        // A second definition of type Unit whose body ignores the first is
        // trivial; reference the first through a constant to exercise resolution.
        let referencing = Declaration::def(
            LevelSignature::monomorphic(),
            ValueType::Thunk(Box::new(CompType::Arrow {
                domain: Box::new(ValueType::Unit),
                codomain: Box::new(CompType::Returner(Box::new(ValueType::Unit))),
            })),
            Value::Constant(first.position()),
        );
        assert!(
            environment.add_decl(referencing).is_ok(),
            "a definition may reference a prior declaration by constant"
        );
    }

    #[test]
    fn a_forward_constant_reference_is_unbound()
    {
        let mut environment = Environment::new();
        let declaration = Declaration::def(
            LevelSignature::monomorphic(),
            ValueType::Unit,
            Value::Constant(ConstantIndex::from(0_usize)),
        );
        assert_eq!(
            environment.add_decl(declaration),
            Err(KernelError::UnboundConstant {
                index: ConstantIndex::from(0_usize),
            }),
            "a constant referencing the not-yet-admitted self is unbound"
        );
    }

    #[test]
    fn a_rejected_declaration_leaves_the_environment_unchanged()
    {
        let mut environment = Environment::new();
        let _first = environment.add_decl(identity_declaration()).unwrap();
        // A definition whose body (unit) does not match its declared type.
        let ill_typed = Declaration::def(
            LevelSignature::monomorphic(),
            ValueType::Base(crate::base::BaseType::Integer),
            Value::Unit,
        );
        assert!(
            environment.add_decl(ill_typed).is_err(),
            "an ill-typed def is rejected"
        );
        // The rejected declaration was not appended: a fresh reference to
        // position 1 is still unbound.
        let referencing = Declaration::def(
            LevelSignature::monomorphic(),
            ValueType::Unit,
            Value::Constant(ConstantIndex::from(1_usize)),
        );
        assert_eq!(
            environment.add_decl(referencing),
            Err(KernelError::UnboundConstant {
                index: ConstantIndex::from(1_usize),
            }),
            "the rejected declaration left no entry at position 1"
        );
    }

    #[test]
    fn a_closed_definition_rests_on_nothing()
    {
        let mut environment = Environment::new();
        let id = environment.add_decl(identity_declaration()).unwrap();
        let report = environment.audit(id);
        assert!(
            report.axioms().is_empty(),
            "a checked def rests on no axiom"
        );
        assert!(
            report.unchecked_admissions().is_empty(),
            "a checked def rests on no unchecked admission"
        );
    }

    #[test]
    fn audit_reports_the_transitive_axiom()
    {
        let mut environment = Environment::new();
        let axiom = environment.add_decl(Declaration::axiom(
            LevelSignature::monomorphic(),
            ValueType::Unit,
        ));
        let axiom = axiom.unwrap();
        // A def whose body references the axiom.
        let dependent = Declaration::def(
            LevelSignature::monomorphic(),
            ValueType::Unit,
            Value::Constant(axiom.position()),
        );
        let dependent = environment.add_decl(dependent).unwrap();
        let report = environment.audit(dependent);
        assert_eq!(
            report.axioms(),
            &[axiom.position()],
            "the dependent def rests on the axiom"
        );
    }

    #[test]
    fn audit_reports_a_transitive_unchecked_admission()
    {
        let mut environment = Environment::new();
        let bypassed = environment.add_decl_unchecked(Declaration::def(
            LevelSignature::monomorphic(),
            ValueType::Unit,
            Value::Unit,
        ));
        let dependent = Declaration::def(
            LevelSignature::monomorphic(),
            ValueType::Unit,
            Value::Constant(bypassed.position()),
        );
        let dependent = environment.add_decl(dependent).unwrap();
        let report = environment.audit(dependent);
        assert_eq!(
            report.unchecked_admissions(),
            &[bypassed.position()],
            "the dependent def rests on the unchecked admission"
        );
    }
}
