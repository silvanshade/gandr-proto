//! The append-only environment and its single choke point (K3):
//! [`Environment::add_decl`] is the **only** way a declaration enters
//! checked, [`Environment::add_decl_unchecked`] is the **one** warned bypass,
//! and [`Environment::audit`] is the `#print axioms` analogue.
//!
//! A [`CheckedId`] is **unforgeable outside this crate** — its constructor is
//! private — so "this declaration was admitted" is a type-level fact in
//! consuming code, not a runtime claim (K3, the adequacy ladder's L0 applied to
//! trust itself). There is a **single checked/unchecked bit**, never a trust
//! lattice.
//!
//! # The arena and the admission watermark (D1(C))
//!
//! The environment owns the one [`TermArena`] every declaration's content lives
//! in. Content is built by a [`DeclarationBuilder`] ([`Environment::stage`])
//! that records the **content-start** watermark; admission enforces the
//! ratified invariant:
//!
//! > After `add_decl` returns — success **or** rejection — the arena holds
//! > exactly the nodes admitted by prior declarations plus, on success, this
//! > declaration's content; checker intermediates never persist.
//!
//! The mechanism is a **two-mark** scheme (impl-models §1.4/§5.3, the Idris
//! commit-on-success staging overlay). The builder records content-start;
//! `add_decl` reads content-end on entry (the checker's synthesized
//! intermediates allocate strictly after it). The checker runs, then the arena
//! is truncated: to **content-end** on success (dropping intermediates, keeping
//! content) or to **content-start** on rejection (dropping both). The
//! `a_rejected_declaration_leaves_the_environment_unchanged` witness asserts
//! the arena length is restored on rejection.
//!
//! # The admission floor (why rejection clamps its rollback)
//!
//! A [`Declaration`] outlives the builder's borrow, so **the order content is
//! staged in need not be the order it is admitted in**: a producer may stage
//! one declaration, admit a second, and only then offer the first. Its
//! content-start mark then lies *below* content the environment has already
//! admitted, and a rejection that truncated to it would delete an admitted
//! declaration's nodes — leaving `entries` intact but its content roots
//! dangling. That is the rejection path corrupting state the caller can
//! observe, which is precisely what the ratified invariant forbids.
//!
//! The environment therefore carries a third mark, the **admission floor**: the
//! arena watermark as of the most recent admission, checked or bypassed. A
//! rejection truncates to this declaration's content-start **clamped into
//! `[admission floor, content-end]`** ([`ArenaWatermark::clamped_into`]), so it
//! never reaches below committed content and never leaves a checker
//! intermediate behind. For the ordinary stage-then-admit producer the
//! content-start mark already lies inside that interval and the clamp changes
//! nothing; for the out-of-order producer the rejected declaration's nodes are
//! retained as unreachable orphans, which no admitted entry, audit, or export
//! walk can observe. **Retaining garbage is the failure this trades for
//! deleting evidence.**

use alloc::collections::BTreeSet;
use alloc::vec::Vec;

use crate::arena::ArenaWatermark;
use crate::arena::CompTypeId;
use crate::arena::TermArena;
use crate::arena::ValueId;
use crate::arena::ValueTypeId;
use crate::check;
use crate::decl::Declaration;
use crate::decl::DeclarationBuilder;
use crate::decl::DeclarationContent;
use crate::error::KernelError;
use crate::levels::LevelContext;
use crate::term::Computation;
use crate::term::ConstantIndex;
use crate::term::Value;
use crate::types::CompType;
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
pub struct AdmittedDeclaration
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

impl AdmittedDeclaration
{
    /// The declared value-type root id — the constant rule's resolution target.
    #[inline]
    #[must_use]
    pub fn declared_id(&self) -> ValueTypeId
    {
        self.declaration.declared_id()
    }

    /// The admitted declaration's content — what an abstract-type reference is
    /// resolved against, so an atom's claim to be one is checked rather than
    /// believed.
    #[inline]
    #[must_use]
    pub(crate) const fn content(&self) -> &DeclarationContent
    {
        self.declaration.content()
    }
}

/// The transitive audit of a declaration: the axioms and unchecked admissions
/// it rests on (the `#print axioms` analogue).
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

/// The append-only kernel environment: the one term arena and a sequence of
/// admitted declarations in admission order (the export
/// format's E2 ordering).
#[derive(Clone, Debug, Default)]
pub struct Environment
{
    /// The one arena every declaration's content lives in.
    arena: TermArena,
    /// The admitted declarations, in admission order.
    entries: Vec<AdmittedDeclaration>,
    /// The **admission floor**: the arena watermark as of the most recent
    /// admission, checked or bypassed. No rejection truncates below it, so an
    /// admitted declaration's content survives every later error return
    /// whatever order the rejected declaration was staged in (the module's
    /// admission-floor section). The default is the empty arena's watermark.
    admission_floor: ArenaWatermark,
}

impl Environment
{
    /// An empty environment.
    #[inline]
    #[must_use]
    pub fn new() -> Self
    {
        Self::default()
    }

    /// Begin building one declaration's content into the environment arena.
    ///
    /// # Contract
    /// - requires: the returned builder is used to mint exactly one
    ///   declaration's content, then finalized and passed to `add_decl` /
    ///   `add_decl_unchecked` before any further arena mutation (the content-
    ///   start watermark records the arena length now).
    /// - ensures: a [`DeclarationBuilder`] borrowing the arena. If the producer
    ///   abandons the staging — an error return with the builder still live, or
    ///   an explicit [`DeclarationBuilder::discard`] — the builder's `Drop`
    ///   truncates the arena back to the content-start watermark, so no
    ///   partially minted content survives and no probe-before-stage discipline
    ///   is owed.
    /// - provides: the minimal construction surface tests and the
    ///   checker-to-kernel bridge use to build arena-resident declarations.
    /// - fails: never.
    /// - panics: none.
    #[inline]
    pub fn stage(&mut self) -> DeclarationBuilder<'_>
    {
        DeclarationBuilder::new(&mut self.arena)
    }

    /// The environment arena, for the export writer's content walk.
    #[inline]
    #[must_use]
    pub(crate) fn arena(&self) -> &TermArena
    {
        &self.arena
    }

    /// Admit a declaration through the checked choke point.
    ///
    /// # Contract
    /// - requires: the declaration's content was built into this environment's
    ///   arena (typically via [`Self::stage`]); every well-formedness and
    ///   typing obligation is re-derived here (K2), granting the producer no
    ///   credence.
    /// - ensures: `Ok(id)` with an unforgeable [`CheckedId`] exactly when the
    ///   declaration's level signature admits, its declared type is
    ///   well-formed, and (for a `Def`) its body checks against that type; the
    ///   declaration is appended, its audit precomputed, the arena holds prior
    ///   content plus this declaration's content (checker intermediates
    ///   truncated), and the admission floor rises to that watermark. On any
    ///   failure the arena is truncated to this declaration's content-start
    ///   watermark **clamped into `[admission floor, checker-entry mark]`**, so
    ///   every caller-observable face of the environment — the entries, their
    ///   content, the audit, the export image — is exactly what it was before
    ///   the call, whatever order the declaration was staged in (the module's
    ///   admission-floor section).
    /// - provides: the K3 choke point — the only checked entry into the kernel.
    /// - fails: any [`KernelError`] the level admission or the checker
    ///   surfaces.
    /// - panics: none.
    ///
    /// # Errors
    /// Any [`KernelError`].
    ///
    /// # Adequacy
    /// - hypothesis: L1/L2 — a `CheckedId` is unforgeable (L0 type-level), and
    ///   the corpus differential pins acceptance of well-typed declarations and
    ///   rejection of ill-typed ones; the L3 residues are the
    ///   constant-reference resolution and the
    ///   arena-length-restored-on-rejection boundary.
    /// - witness: `env::tests::a_definition_referencing_a_prior_one_checks`
    /// - witness: `env::tests::a_forward_constant_reference_is_unbound`
    /// - witness: `env::tests::a_rejected_declaration_leaves_the_environment_unchanged`
    /// - witness: `env::tests::a_rejection_keeps_a_prior_admission_resolvable`
    /// - witness: `env::tests::a_rejection_keeps_bypassed_content_resolvable`
    #[inline]
    pub fn add_decl(
        &mut self,
        declaration: Declaration,
    ) -> Result<CheckedId, KernelError>
    {
        let position = ConstantIndex::from(self.entries.len());
        let content_end = self.arena.watermark();
        // The rejection mark: this declaration's content-start, clamped into
        // `[admission floor, content-end]` so a rollback can neither delete an
        // admitted declaration's content nor retain a checker intermediate.
        let rejected = declaration
            .content_start()
            .clamped_into(self.admission_floor, content_end);
        let levels = match LevelContext::admit(
            declaration.levels().params(),
            declaration.levels().constraints().to_vec(),
        ) {
            | Ok(levels) => levels,
            | Err(error) => {
                self.arena.truncate_to(rejected);
                return Err(error);
            },
        };
        let outcome = {
            let Self {
                ref mut arena,
                ref entries,
                admission_floor: _,
            } = *self;
            check::check_declaration(arena, entries, &levels, &declaration)
        };
        match outcome {
            | Ok(()) => {
                // Keep this declaration's content; drop the checker intermediates
                // that allocated past it.
                self.arena.truncate_to(content_end);
                self.admission_floor = content_end;
                let rested_on =
                    self.transitive_rest(declaration.content(), position, Admission::Checked);
                self.entries.push(AdmittedDeclaration {
                    declaration,
                    admission: Admission::Checked,
                    rested_on,
                });
                Ok(CheckedId::new(position))
            },
            | Err(error) => {
                // Drop both the intermediates and this declaration's content,
                // down to the admission floor and no further.
                self.arena.truncate_to(rejected);
                Err(error)
            },
        }
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
    /// that transitively rests on it.
    ///
    /// # Contract
    /// - requires: the declaration's content was built into this environment's
    ///   arena; the caller vouches for the declaration; nothing is verified.
    /// - ensures: the declaration is appended, marked [`Admission::Unchecked`],
    ///   and included in the audit of every dependent; a [`CheckedId`] is
    ///   returned. The arena is unchanged (the bypass mints no intermediates,
    ///   so the built content is exactly what persists) and the admission floor
    ///   rises to its watermark, so a later rejection cannot truncate through
    ///   what the bypass committed.
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
        // The bypass commits whatever content the arena holds, so the floor
        // rises to the current watermark — which never sits below the floor,
        // because every truncation clamps to it. Without this, a later
        // rejection could truncate through bypassed content.
        self.admission_floor = self.arena.watermark();
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
    /// admission mark — the export writer's E2/E6 source.
    /// The content roots address [`Self::arena`].
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

    /// Precompute the transitive set of axioms and unchecked admissions a
    /// declaration rests on.
    ///
    /// **A declaration depends on what its declared type names as well as on
    /// what its body names**, and the two are separate arenas to walk. Until
    /// [`ValueType::Abstract`] there was no such thing as a type-level
    /// constant reference, so scanning bodies alone was complete; a sealed
    /// atom is the first type-level edge, and formation resting on one is
    /// exactly as load-bearing as a body resting on an axiom. Every arm's
    /// declared root is scanned, including an abstract type's own kind.
    fn transitive_rest(
        &self,
        content: &DeclarationContent,
        position: ConstantIndex,
        admission: Admission,
    ) -> BTreeSet<ConstantIndex>
    {
        let mut direct = collect_type_constants(&self.arena, content.declared_id());
        if let DeclarationContent::Def { body, .. } = *content {
            direct.append(&mut collect_constants(&self.arena, body));
        }
        let mut set = BTreeSet::new();
        for referenced in direct {
            if let Some(entry) = self.entries.get(usize::from(referenced)) {
                for &ancestor in &entry.rested_on {
                    let _fresh = set.insert(ancestor);
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

/// Collect the constant references a value body mentions, iteratively over the
/// arena.
///
/// # Contract
/// - requires: `root` resolves in `arena`.
/// - ensures: exactly the set of [`ConstantIndex`]es reachable in the term (a
///   dangling id contributes nothing — fail-closed).
/// - provides: the direct-dependency edges of the audit graph.
/// - fails: never.
/// - panics: none.
fn collect_constants(
    arena: &TermArena,
    root: ValueId,
) -> BTreeSet<ConstantIndex>
{
    let mut found = BTreeSet::new();
    let mut values: Vec<ValueId> = Vec::new();
    let mut computations: Vec<crate::arena::ComputationId> = Vec::new();
    values.push(root);
    loop {
        while let Some(id) = values.pop() {
            let Some(value) = arena.value(id)
            else {
                continue;
            };
            match *value {
                | Value::Constant(index) => {
                    let _fresh = found.insert(index);
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
        let Some(id) = computations.pop()
        else {
            break;
        };
        let Some(computation) = arena.computation(id)
        else {
            continue;
        };
        match *computation {
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

/// Collect the constant references a value type mentions, iteratively over the
/// arena.
///
/// The type graph carries exactly one reference form —
/// [`ValueType::Abstract`], a sealed atom naming its declaration's admission
/// position — and it reaches through products, sums, lifts, and the thunked
/// computation types, so the walk is the whole type rather than its head.
///
/// # Contract
/// - requires: `root` resolves in `arena`.
/// - ensures: exactly the set of [`ConstantIndex`]es reachable in the type (a
///   dangling id contributes nothing — fail-closed).
/// - provides: the type-level half of the audit graph's direct-dependency
///   edges, which formation rests on.
/// - fails: never.
/// - panics: none.
fn collect_type_constants(
    arena: &TermArena,
    root: ValueTypeId,
) -> BTreeSet<ConstantIndex>
{
    let mut found = BTreeSet::new();
    let mut value_types: Vec<ValueTypeId> = Vec::new();
    let mut comp_types: Vec<CompTypeId> = Vec::new();
    value_types.push(root);
    loop {
        while let Some(id) = value_types.pop() {
            let Some(value_type) = arena.value_type(id)
            else {
                continue;
            };
            match *value_type {
                | ValueType::Abstract(index) => {
                    let _fresh = found.insert(index);
                },
                | ValueType::Base(_) | ValueType::Unit | ValueType::Universe(_) => {},
                | ValueType::Product(first, second) | ValueType::Sum(first, second) => {
                    value_types.push(first);
                    value_types.push(second);
                },
                | ValueType::Lift { inner, .. } => value_types.push(inner),
                | ValueType::Thunk(body) => comp_types.push(body),
            }
        }
        let Some(id) = comp_types.pop()
        else {
            break;
        };
        let Some(comp_type) = arena.comp_type(id)
        else {
            continue;
        };
        match *comp_type {
            | CompType::Returner(value_type) => value_types.push(value_type),
            | CompType::Arrow { domain, codomain } => {
                value_types.push(domain);
                comp_types.push(codomain);
            },
        }
    }
    found
}

#[cfg(test)]
mod tests
{
    use gandr_kernel_strata::Level;
    use gandr_kernel_strata::LevelConstant;

    use super::Environment;
    use crate::base::BaseType;
    use crate::decl::LevelSignature;
    use crate::error::KernelError;
    use crate::term::ConstantIndex;
    use crate::term::DeBruijnIndex;

    /// Stage and finalize the identity thunk `U (Unit → F Unit) = thunk (λ.
    /// return v0)`, admitting it and returning the environment.
    fn admit_identity(environment: &mut Environment) -> Result<super::CheckedId, KernelError>
    {
        let mut builder = environment.stage();
        let arena = builder.arena();
        let domain = arena.value_type_unit();
        let result = arena.value_type_unit();
        let returner = arena.comp_type_returner(result);
        let arrow = arena.comp_type_arrow(domain, returner);
        let declared = arena.value_type_thunk(arrow);
        let variable = arena.value_variable(DeBruijnIndex::from(0_u32));
        let ret = arena.computation_return(variable);
        let lambda = arena.computation_lambda(ret);
        let body = arena.value_thunk(lambda);
        let declaration = builder.def(LevelSignature::monomorphic(), declared, body);
        environment.add_decl(declaration)
    }

    #[cfg_attr(
        dylint_lib = "non_local_effect_before_unhandled_error",
        allow(
            unknown_lints,
            non_local_effect_before_unhandled_error,
            reason = "the flagged effect is add_decl's own rollback truncation; the environment is local to this test, this admission is asserted to succeed, and no state outlives the test"
        )
    )]
    #[test]
    fn a_definition_referencing_a_prior_one_checks()
    {
        let mut environment = Environment::new();
        let first = admit_identity(&mut environment).unwrap();
        // A second definition of type U (Unit → F Unit) whose body is a constant
        // reference to the first — exercises constant resolution.
        let mut builder = environment.stage();
        let arena = builder.arena();
        let domain = arena.value_type_unit();
        let result = arena.value_type_unit();
        let returner = arena.comp_type_returner(result);
        let arrow = arena.comp_type_arrow(domain, returner);
        let declared = arena.value_type_thunk(arrow);
        let body = arena.value_constant(first.position());
        let declaration = builder.def(LevelSignature::monomorphic(), declared, body);
        assert!(
            environment.add_decl(declaration).is_ok(),
            "a definition may reference a prior declaration by constant"
        );
    }

    #[cfg_attr(
        dylint_lib = "non_local_effect_before_unhandled_error",
        allow(
            unknown_lints,
            non_local_effect_before_unhandled_error,
            reason = "the flagged effect is add_decl's own rollback truncation; the environment is local to this test, is not read after the rejection, and is dropped at scope exit"
        )
    )]
    #[test]
    fn a_forward_constant_reference_is_unbound()
    {
        let mut environment = Environment::new();
        let mut builder = environment.stage();
        let arena = builder.arena();
        let declared = arena.value_type_unit();
        let body = arena.value_constant(ConstantIndex::from(0_usize));
        let declaration = builder.def(LevelSignature::monomorphic(), declared, body);
        assert_eq!(
            environment.add_decl(declaration),
            Err(KernelError::UnboundConstant {
                index: ConstantIndex::from(0_usize),
            }),
            "a constant referencing the not-yet-admitted self is unbound"
        );
    }

    #[cfg_attr(
        dylint_lib = "non_local_effect_before_unhandled_error",
        allow(
            unknown_lints,
            non_local_effect_before_unhandled_error,
            reason = "the flagged effect is add_decl's own rollback truncation, and asserting that it restores the pre-call arena is what this test exists to do"
        )
    )]
    #[test]
    fn a_rejected_declaration_leaves_the_environment_unchanged()
    {
        let mut environment = Environment::new();
        let _first = admit_identity(&mut environment).unwrap();
        let before = environment.arena().watermark();
        // A definition whose body (unit) does not match its declared type.
        let mut builder = environment.stage();
        let arena = builder.arena();
        let declared = arena.value_type_base(BaseType::Integer);
        let body = arena.value_unit();
        let ill_typed = builder.def(LevelSignature::monomorphic(), declared, body);
        assert!(
            environment.add_decl(ill_typed).is_err(),
            "an ill-typed def is rejected"
        );
        assert_eq!(
            environment.arena().watermark(),
            before,
            "rejection restores the arena length (content and intermediates truncated)"
        );
        // The rejected declaration was not appended: a fresh reference to
        // position 1 is still unbound.
        let mut builder = environment.stage();
        let arena = builder.arena();
        let declared = arena.value_type_unit();
        let body = arena.value_constant(ConstantIndex::from(1_usize));
        let referencing = builder.def(LevelSignature::monomorphic(), declared, body);
        assert_eq!(
            environment.add_decl(referencing),
            Err(KernelError::UnboundConstant {
                index: ConstantIndex::from(1_usize),
            }),
            "the rejected declaration left no entry at position 1"
        );
    }

    /// **A rejection never truncates through a prior admission.** A producer
    /// may stage one declaration, admit a second, and only then offer the
    /// first, because a [`Declaration`](crate::Declaration) outlives the
    /// builder's borrow — and the stale declaration's content-start mark then
    /// lies below content the environment has already admitted. Rolling back
    /// to that mark would delete the admitted declaration's nodes while its
    /// entry survived, leaving its content roots dangling; the admission floor
    /// is what stops the rollback there.
    ///
    /// Both assertions fail if the floor clamp is removed from the rejection
    /// path (the arena is truncated to the empty watermark, so the admitted
    /// definition stops resolving), and the first fails if the rejection stops
    /// rolling back at all (the checker intermediates survive).
    #[cfg_attr(
        dylint_lib = "non_local_effect_before_unhandled_error",
        allow(
            unknown_lints,
            non_local_effect_before_unhandled_error,
            reason = "the flagged effect is add_decl's own rollback truncation, and asserting that it stops at the admission floor is what this test exists to do"
        )
    )]
    #[test]
    fn a_rejection_keeps_a_prior_admission_resolvable()
    {
        let mut environment = Environment::new();
        // Staged first and held back: an ill-typed def whose content-start mark
        // is the empty arena's.
        let stale = {
            let mut builder = environment.stage();
            let arena = builder.arena();
            let declared = arena.value_type_base(BaseType::Integer);
            let body = arena.value_unit();
            builder.def(LevelSignature::monomorphic(), declared, body)
        };
        let first = admit_identity(&mut environment).unwrap();
        let after_admission = environment.arena().watermark();
        assert!(
            environment.add_decl(stale).is_err(),
            "the stale ill-typed def is still rejected"
        );
        assert_eq!(
            environment.arena().watermark(),
            after_admission,
            "the rejection rolled back to the admission floor and no further"
        );
        // The admitted definition is still resolvable: a later definition may
        // reference it by constant, which reads its declared type out of the
        // arena the rejection would otherwise have truncated.
        let mut builder = environment.stage();
        let arena = builder.arena();
        let domain = arena.value_type_unit();
        let result = arena.value_type_unit();
        let returner = arena.comp_type_returner(result);
        let arrow = arena.comp_type_arrow(domain, returner);
        let declared = arena.value_type_thunk(arrow);
        let body = arena.value_constant(first.position());
        let referencing = builder.def(LevelSignature::monomorphic(), declared, body);
        assert!(
            environment.add_decl(referencing).is_ok(),
            "the admitted definition's content survived the rejection"
        );
    }

    /// The admission floor rises at the **warned bypass** too: content admitted
    /// through [`Environment::add_decl_unchecked`] is committed exactly as
    /// checked content is, so a later rejection cannot truncate through it.
    ///
    /// Both assertions fail if the bypass stops raising the floor: the stale
    /// declaration's content-start mark is the empty arena's, so the rejection
    /// would empty the arena and strand the bypassed axiom's declared type.
    #[cfg_attr(
        dylint_lib = "non_local_effect_before_unhandled_error",
        allow(
            unknown_lints,
            non_local_effect_before_unhandled_error,
            reason = "the flagged effect is add_decl's own rollback truncation, and asserting that it stops at the floor the bypass raised is what this test exists to do"
        )
    )]
    #[test]
    fn a_rejection_keeps_bypassed_content_resolvable()
    {
        let mut environment = Environment::new();
        // Staged first and held back, as above.
        let stale = {
            let mut builder = environment.stage();
            let arena = builder.arena();
            let declared = arena.value_type_base(BaseType::Integer);
            let body = arena.value_unit();
            builder.def(LevelSignature::monomorphic(), declared, body)
        };
        let bypassed = {
            let mut builder = environment.stage();
            let declared = builder.arena().value_type_unit();
            let axiom = builder.axiom(LevelSignature::monomorphic(), declared);
            environment.add_decl_unchecked(axiom)
        };
        let after_bypass = environment.arena().watermark();
        assert!(
            environment.add_decl(stale).is_err(),
            "the stale ill-typed def is still rejected"
        );
        assert_eq!(
            environment.arena().watermark(),
            after_bypass,
            "the rejection rolled back to the floor the bypass raised"
        );
        let mut builder = environment.stage();
        let arena = builder.arena();
        let declared = arena.value_type_unit();
        let body = arena.value_constant(bypassed.position());
        let referencing = builder.def(LevelSignature::monomorphic(), declared, body);
        assert!(
            environment.add_decl(referencing).is_ok(),
            "the bypassed axiom's declared type survived the rejection"
        );
    }

    /// A producer that abandons a staged builder — the error return before any
    /// finisher — leaves no orphan content: the builder's `Drop` truncates the
    /// partial mint back to the content-start watermark.
    #[test]
    fn an_abandoned_staging_restores_the_arena()
    {
        let mut environment = Environment::new();
        let _first = admit_identity(&mut environment).unwrap();
        let before = environment.arena().watermark();
        {
            let mut builder = environment.stage();
            let arena = builder.arena();
            let _declared = arena.value_type_base(BaseType::Integer);
            let _body = arena.value_unit();
            // Scope exit without a finisher: the producer's failure path.
        }
        assert_eq!(
            environment.arena().watermark(),
            before,
            "an abandoned builder truncates what it minted"
        );
        // The environment is fully usable afterwards: the next admitted
        // declaration takes the next position.
        let mut builder = environment.stage();
        let arena = builder.arena();
        let declared = arena.value_type_unit();
        let body = arena.value_unit();
        let next = builder.def(LevelSignature::monomorphic(), declared, body);
        let next = environment.add_decl(next).unwrap();
        assert_eq!(
            next.position(),
            ConstantIndex::from(1_usize),
            "the abandonment left nothing behind"
        );
    }

    /// The explicit discard path restores the arena exactly as scope exit
    /// does.
    #[test]
    fn a_discarded_staging_restores_the_arena()
    {
        let mut environment = Environment::new();
        let _first = admit_identity(&mut environment).unwrap();
        let before = environment.arena().watermark();
        let mut builder = environment.stage();
        let arena = builder.arena();
        let _declared = arena.value_type_base(BaseType::Integer);
        let _body = arena.value_unit();
        builder.discard();
        assert_eq!(
            environment.arena().watermark(),
            before,
            "discard truncates what the builder minted"
        );
    }

    #[test]
    fn a_closed_definition_rests_on_nothing()
    {
        let mut environment = Environment::new();
        let id = admit_identity(&mut environment).unwrap();
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

    // ----- Sealed abstract types: the kernel handshake. -----

    /// Admit one sealed atom at universe level zero, returning its position.
    fn admit_atom(environment: &mut Environment) -> super::CheckedId
    {
        let mut builder = environment.stage();
        let kind = builder
            .arena()
            .value_type_universe(Level::constant(LevelConstant::from(0_u64)));
        let atom = builder.abstract_type(LevelSignature::monomorphic(), kind);
        environment
            .add_decl(atom)
            .expect("an abstract type at a universe kind admits")
    }

    /// A sealed atom admits, and the kernel takes on **no** obligation for it:
    /// it is not an axiom, so nothing typed at it is reported as resting on
    /// one.
    ///
    /// The distinction is the whole reason the atom route was chosen over an
    /// existential. An axiom claims an inhabitant and the audit must surface
    /// it; an atom introduces an uninterpreted type constant, which is a
    /// conservative extension and claims nothing.
    #[test]
    fn a_sealed_atom_is_not_an_axiom_and_adds_no_obligation()
    {
        let mut environment = Environment::new();
        let atom = admit_atom(&mut environment);
        let report = environment.audit(atom);
        assert!(
            report.axioms().is_empty(),
            "a sealed atom is not an axiom: it claims no inhabitant"
        );
        assert!(
            report.unchecked_admissions().is_empty(),
            "and it crossed the checked choke point"
        );
    }

    /// A sealed member's body is **certified** against its atom: the identity
    /// `U (t → F t)` checks without the kernel ever unfolding `t`.
    #[cfg_attr(
        dylint_lib = "non_local_effect_before_unhandled_error",
        allow(
            unknown_lints,
            non_local_effect_before_unhandled_error,
            reason = "the flagged effect is add_decl's own rollback truncation; the environment is local to this test, this admission is asserted to succeed, and no state outlives the test"
        )
    )]
    #[test]
    fn a_sealed_member_checks_at_its_atom()
    {
        let mut environment = Environment::new();
        let atom = admit_atom(&mut environment);
        let mut builder = environment.stage();
        let arena = builder.arena();
        let opaque = arena.value_type_abstract(atom.position());
        let returner = arena.comp_type_returner(opaque);
        let arrow = arena.comp_type_arrow(opaque, returner);
        let declared = arena.value_type_thunk(arrow);
        let variable = arena.value_variable(DeBruijnIndex::from(0_u32));
        let ret = arena.computation_return(variable);
        let lambda = arena.computation_lambda(ret);
        let body = arena.value_thunk(lambda);
        let member =
            builder.sealed_def(LevelSignature::monomorphic(), declared, body, alloc::vec![
                atom.position()
            ]);
        assert!(
            environment.add_decl(member).is_ok(),
            "the identity at a sealed atom checks, with the atom never unfolded"
        );
    }

    /// **The abstraction-leak refutation.** A declaration claiming to inhabit a
    /// sealed atom with a value of the representation is refused.
    ///
    /// Nothing here is special-cased for sealing: the atom simply has no
    /// conversion arm relating it to `Unit`, so the ordinary mode-switch
    /// conversion refuses. Opacity falls out of the closed vocabulary rather
    /// than out of a guard someone remembered to write.
    #[cfg_attr(
        dylint_lib = "non_local_effect_before_unhandled_error",
        allow(
            unknown_lints,
            non_local_effect_before_unhandled_error,
            reason = "the flagged effect is add_decl's own rollback truncation; the environment is local to this test, is not read after the rejection, and is dropped at scope exit"
        )
    )]
    #[test]
    fn a_value_of_the_representation_does_not_inhabit_the_atom()
    {
        let mut environment = Environment::new();
        let atom = admit_atom(&mut environment);
        let mut builder = environment.stage();
        let arena = builder.arena();
        let declared = arena.value_type_abstract(atom.position());
        let body = arena.value_unit();
        let leak = builder.def(LevelSignature::monomorphic(), declared, body);
        assert!(
            matches!(
                environment.add_decl(leak),
                Err(KernelError::ValueTypeMismatch(_))
            ),
            "unit does not inhabit a sealed atom: the representation cannot leak through"
        );
    }

    /// An atom reference naming an ordinary definition is refused: being an
    /// atom is resolved against the environment, never asserted by the
    /// reference.
    #[cfg_attr(
        dylint_lib = "non_local_effect_before_unhandled_error",
        allow(
            unknown_lints,
            non_local_effect_before_unhandled_error,
            reason = "the flagged effect is add_decl's own rollback truncation; the environment is local to this test, is not read after the rejection, and is dropped at scope exit"
        )
    )]
    #[test]
    fn an_atom_naming_a_definition_is_refused()
    {
        let mut environment = Environment::new();
        let definition = admit_identity(&mut environment).unwrap();
        let mut builder = environment.stage();
        let arena = builder.arena();
        let forged = arena.value_type_abstract(definition.position());
        let declaration = builder.axiom(LevelSignature::monomorphic(), forged);
        assert_eq!(
            environment.add_decl(declaration),
            Err(KernelError::NotAnAbstractType {
                index: definition.position(),
            }),
            "an atom may not name a definition"
        );
    }

    /// An atom reference to a position not yet admitted — including its own —
    /// is refused, so no atom can be self-referential or forward.
    #[cfg_attr(
        dylint_lib = "non_local_effect_before_unhandled_error",
        allow(
            unknown_lints,
            non_local_effect_before_unhandled_error,
            reason = "the flagged effect is add_decl's own rollback truncation; the environment is local to this test, is not read after the rejection, and is dropped at scope exit"
        )
    )]
    #[test]
    fn a_forward_atom_reference_is_refused()
    {
        let mut environment = Environment::new();
        let mut builder = environment.stage();
        let arena = builder.arena();
        let forged = arena.value_type_abstract(ConstantIndex::from(0_usize));
        let declaration = builder.axiom(LevelSignature::monomorphic(), forged);
        assert_eq!(
            environment.add_decl(declaration),
            Err(KernelError::NotAnAbstractType {
                index: ConstantIndex::from(0_usize),
            }),
            "an atom may not name the declaration currently being admitted"
        );
    }

    /// An abstract type whose kind is not a universe is refused, so every
    /// atom's level is a lookup rather than an inference.
    #[cfg_attr(
        dylint_lib = "non_local_effect_before_unhandled_error",
        allow(
            unknown_lints,
            non_local_effect_before_unhandled_error,
            reason = "the flagged effect is add_decl's own rollback truncation; the environment is local to this test, is not read after the rejection, and is dropped at scope exit"
        )
    )]
    #[test]
    fn an_atom_kind_must_be_a_universe()
    {
        let mut environment = Environment::new();
        let mut builder = environment.stage();
        let kind = builder.arena().value_type_unit();
        let declaration = builder.abstract_type(LevelSignature::monomorphic(), kind);
        assert!(
            matches!(
                environment.add_decl(declaration),
                Err(KernelError::AbstractTypeKindNotUniverse { .. })
            ),
            "an atom's kind must be a universe"
        );
    }

    /// Sealing provenance is **re-derived**: an atom claimed but not projected
    /// onto is refused, so the slot cannot record a projection that left no
    /// trace in the type.
    #[cfg_attr(
        dylint_lib = "non_local_effect_before_unhandled_error",
        allow(
            unknown_lints,
            non_local_effect_before_unhandled_error,
            reason = "the flagged effect is add_decl's own rollback truncation; the environment is local to this test, is not read after the rejection, and is dropped at scope exit"
        )
    )]
    #[test]
    fn sealing_provenance_must_occur_in_the_declared_type()
    {
        let mut environment = Environment::new();
        let atom = admit_atom(&mut environment);
        let mut builder = environment.stage();
        let arena = builder.arena();
        let declared = arena.value_type_unit();
        let body = arena.value_unit();
        let overclaimed =
            builder.sealed_def(LevelSignature::monomorphic(), declared, body, alloc::vec![
                atom.position()
            ]);
        assert_eq!(
            environment.add_decl(overclaimed),
            Err(KernelError::SealingProvenanceNotProjected {
                atom: atom.position(),
            }),
            "provenance naming an atom absent from the declared type is refused"
        );
    }

    /// Sealing provenance must be strictly ascending, so a repeat cannot
    /// inflate a projection's apparent extent and one provenance has one
    /// spelling.
    #[cfg_attr(
        dylint_lib = "non_local_effect_before_unhandled_error",
        allow(
            unknown_lints,
            non_local_effect_before_unhandled_error,
            reason = "the flagged effect is add_decl's own rollback truncation; the environment is local to this test, is not read after the rejection, and is dropped at scope exit"
        )
    )]
    #[test]
    fn sealing_provenance_must_be_strictly_ascending()
    {
        let mut environment = Environment::new();
        let atom = admit_atom(&mut environment);
        let mut builder = environment.stage();
        let arena = builder.arena();
        let declared = arena.value_type_abstract(atom.position());
        let body = arena.value_unit();
        let repeated =
            builder.sealed_def(LevelSignature::monomorphic(), declared, body, alloc::vec![
                atom.position(),
                atom.position()
            ]);
        assert_eq!(
            environment.add_decl(repeated),
            Err(KernelError::SealingProvenanceNotCanonical {
                atom: atom.position(),
            }),
            "a repeated provenance entry is refused"
        );
    }

    #[test]
    fn audit_reports_the_transitive_axiom()
    {
        let mut environment = Environment::new();
        let mut builder = environment.stage();
        let declared = builder.arena().value_type_unit();
        let axiom = builder.axiom(LevelSignature::monomorphic(), declared);
        let axiom = environment.add_decl(axiom).unwrap();
        // A def whose body references the axiom.
        let mut builder = environment.stage();
        let arena = builder.arena();
        let declared = arena.value_type_unit();
        let body = arena.value_constant(axiom.position());
        let dependent = builder.def(LevelSignature::monomorphic(), declared, body);
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
        let mut builder = environment.stage();
        let declared = builder.arena().value_type_unit();
        let body = builder.arena().value_unit();
        let bypassed = builder.def(LevelSignature::monomorphic(), declared, body);
        let bypassed = environment.add_decl_unchecked(bypassed);
        let mut builder = environment.stage();
        let arena = builder.arena();
        let declared = arena.value_type_unit();
        let body = arena.value_constant(bypassed.position());
        let dependent = builder.def(LevelSignature::monomorphic(), declared, body);
        let dependent = environment.add_decl(dependent).unwrap();
        let report = environment.audit(dependent);
        assert_eq!(
            report.unchecked_admissions(),
            &[bypassed.position()],
            "the dependent def rests on the unchecked admission"
        );
    }

    /// The audit follows a dependency that runs through the *declared type*,
    /// not only one that runs through a body.
    ///
    /// A sealed atom admitted by the warned bypass, then an axiom whose
    /// declared type is that atom: the axiom's formation rests on unchecked
    /// content while its body — it has none — rests on nothing. An audit that
    /// scanned bodies alone would report the axiom's own position and omit the
    /// atom's, which is the whole trust question the report exists to answer.
    #[test]
    fn audit_follows_the_declared_type_to_an_unchecked_atom()
    {
        let mut environment = Environment::new();
        let mut builder = environment.stage();
        let kind = builder
            .arena()
            .value_type_universe(Level::constant(LevelConstant::from(0_u64)));
        let sealed = builder.abstract_type(LevelSignature::monomorphic(), kind);
        let sealed = environment.add_decl_unchecked(sealed);

        let mut builder = environment.stage();
        let declared = builder.arena().value_type_abstract(sealed.position());
        let axiom = builder.axiom(LevelSignature::monomorphic(), declared);
        let axiom = environment.add_decl(axiom).unwrap();

        let report = environment.audit(axiom);
        assert_eq!(
            report.unchecked_admissions(),
            &[sealed.position()],
            "formation rests on the unchecked atom, so the audit reports it"
        );
    }
}
