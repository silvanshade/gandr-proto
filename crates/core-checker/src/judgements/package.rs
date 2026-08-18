//! The package layer: discharging a signature's abstract type components.
//!
//! [`ValueType::Package`] binds the abstract type components of a module
//! signature, and every typing rule that touches a package discharges those
//! binders in exactly one way — one **simultaneous** substitution of a witness
//! type per binder, performed here. Three callers drive it and they differ only
//! in what they supply:
//!
//! * **packing** supplies the witness types the term carries, so the payload is
//!   checked at the representation the packer chose;
//! * **unpacking** supplies one freshly minted [`ValueType::Sealed`] atom per
//!   binder, so the client meets an abstraction rather than a representation —
//!   this is where abstraction safety is actually bought;
//! * **subtyping** supplies one canonical binder per position to both sides, so
//!   two package types are compared up to α-renaming without either side's
//!   spelling deciding the answer.
//!
//! # Why the substitution is simultaneous, and why it avoids capture
//!
//! Sequential substitution of a binder list is wrong whenever one witness
//! mentions a later binder: substituting `α` first and `β` second lets a `β`
//! the first witness contributed be captured by the second step. Doing all
//! binders at once removes that failure entirely, so the engine here takes a
//! map and discharges it in one traversal.
//!
//! What one traversal must still handle is a **nested** package inside the
//! payload. Two cases arise and both are handled rather than assumed away: a
//! nested binder that **shadows** an outer one removes it from the substitution
//! for that subtree, and a nested binder that appears free in some **witness**
//! is renamed to a fresh name before the traversal descends, so the witness's
//! own atom cannot be captured on the way in.
//!
//! # The one configuration this refuses, and why refusing beats proceeding
//!
//! A value can occur inside a type at exactly one place — the endpoints of
//! [`ValueType::Path`] — and a value can carry types of its own, in an
//! ascription or in a thunked computation's binder annotations. Substituting
//! type atoms through a term is a whole term-rewriting engine, and this rung
//! does not build one.
//!
//! Passing such an endpoint through **unsubstituted** is not an option, and the
//! reason is specific rather than fastidious: a binder left behind is a free
//! [`ValueType::Atom`] with a source-level name, and a client whose own rigid
//! atom happens to carry that name would then relate to the abstraction. That
//! is an abstraction leak with no symptom, which is precisely the failure mode
//! the whole nominal-atom route exists to make impossible. So the engine
//! **refuses** ([`PackageRefusal::AbstractUnderPathEndpoint`]) when — and only
//! when — an endpoint actually mentions a name being substituted. An endpoint
//! that mentions none is passed through, because then there is nothing to miss.
//!
//! The refusal is reachable only from hand-built core terms: a package built by
//! lowering a surface signature has a thunked-returner payload, where no `Path`
//! endpoint occurs at all.

use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use gandr_core_term::boundary::PackageArity;
use gandr_core_term::boundary::TypeAtomName;
use gandr_core_term::effect::EffectRow;
use gandr_core_term::error::TypeError;
use gandr_core_term::error::text;
use gandr_core_term::grade::Grade;
use gandr_core_term::syntax::Comp;
use gandr_core_term::syntax::Stack;
use gandr_core_term::syntax::Term;
use gandr_core_term::syntax::Value;
use gandr_core_term::types::CompType;
use gandr_core_term::types::DataId;
use gandr_core_term::types::SealId;
use gandr_core_term::types::Ty;
use gandr_core_term::types::ValueType;

use crate::discipline::mark::Mark;

/// Why discharging a package's binders could not be completed.
///
/// Both variants are refusals rather than failures of the substitution: the
/// engine stops with a named reason instead of producing a type whose binders
/// are partly discharged, because a partly discharged package type is exactly
/// an abstraction leak.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PackageRefusal
{
    /// The number of witnesses supplied is not the number of binders the
    /// package abstracts over.
    ///
    /// There is no sensible completion: a missing witness leaves a binder free
    /// and a surplus witness names a binder that does not exist.
    ArityMismatch
    {
        /// How many abstract type components the package declares.
        declared: PackageArity,
        /// How many witnesses were supplied.
        supplied: PackageArity,
    },
    /// The package declares one abstract type component twice.
    ///
    /// The second binder would shadow the first everywhere in the payload, so
    /// one of the two supplied witnesses could never be reached — a defect in
    /// the signature, refused where it is written rather than at a use site.
    DuplicateComponent
    {
        /// The label declared more than once.
        component: String,
    },
    /// An identity type's endpoint inside the payload mentions a binder being
    /// discharged.
    ///
    /// See the module documentation: substituting a type atom through a term is
    /// outside this rung, and passing the endpoint through would leave a free
    /// binder behind under a source-level name.
    AbstractUnderPathEndpoint
    {
        /// The binder the endpoint mentions.
        component: String,
    },
    /// The package's payload is not a thunk graded exactly as the package.
    ///
    /// A package's grade and its payload thunk's grade are the same `r` rather
    /// than two annotations that might disagree, so this is a malformed
    /// signature rather than a subtyping question.
    PayloadNotGradedThunk
    {
        /// The payload the signature declared.
        payload: ValueType,
    },
    /// An elimination's recorded atoms are not one distinct atom per abstract
    /// type component.
    ///
    /// Two components bound to one atom would be interchangeable inside the
    /// body — an abstraction that abstracts less than it says.
    AtomsNotDistinct,
}

impl fmt::Display for PackageRefusal
{
    #[inline]
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        match *self {
            | Self::ArityMismatch { declared, supplied } => {
                write!(
                    f,
                    "the package declares {} abstract type components and {} were supplied",
                    usize::from(declared),
                    usize::from(supplied)
                )
            },
            | Self::DuplicateComponent { ref component } => {
                write!(
                    f,
                    "the package declares the abstract type component `{component}` twice"
                )
            },
            | Self::AbstractUnderPathEndpoint { ref component } => {
                write!(
                    f,
                    "an identity-type endpoint in the payload mentions the abstract type \
                     component `{component}`"
                )
            },
            | Self::PayloadNotGradedThunk { .. } => {
                f.write_str("the package payload is not a thunk graded as the package itself")
            },
            | Self::AtomsNotDistinct => {
                f.write_str("an unpack binds one distinct minted atom per abstract type component")
            },
        }
    }
}

impl core::error::Error for PackageRefusal
{
}

/// The typing error a refusal becomes at `term`.
///
/// One function so that the recursive checker, the typing machine, and the
/// marker cannot drift apart on what a refused package rule reports. The
/// conformance lockstep compares errors, so a second spelling of this mapping
/// would be a lockstep failure waiting for the input that reaches it.
///
/// # Contract
/// - ensures: a shape refusal becomes [`TypeError::ShapeMismatch`] and every
///   other refusal becomes a [`TypeError::StuckExpr`] at `term` carrying that
///   refusal's own hint.
/// - panics: none.
#[inline]
#[must_use]
pub fn refusal_error(
    refusal: PackageRefusal,
    term: Term,
) -> TypeError
{
    match refusal {
        | PackageRefusal::ArityMismatch { .. } => TypeError::StuckExpr {
            expr: term,
            hint: text::PACK_ARITY_MISMATCH,
        },
        | PackageRefusal::DuplicateComponent { .. } => TypeError::StuckExpr {
            expr: term,
            hint: text::PACKAGE_DUPLICATE_COMPONENT,
        },
        | PackageRefusal::AbstractUnderPathEndpoint { .. } => TypeError::StuckExpr {
            expr: term,
            hint: text::PACKAGE_ABSTRACT_UNDER_PATH,
        },
        | PackageRefusal::PayloadNotGradedThunk { payload } => TypeError::ShapeMismatch {
            expected: text::SHAPE_PACKAGE_PAYLOAD,
            actual: Ty::Value(payload),
        },
        | PackageRefusal::AtomsNotDistinct => TypeError::StuckExpr {
            expr: term,
            hint: text::UNPACK_ATOM_MISMATCH,
        },
    }
}

/// The mark a refusal becomes in the total marking layer.
///
/// The sibling of [`refusal_error`], written beside it because the marking
/// oracle binds them together: the checker rejecting forces at least one error
/// mark, so a refusal that becomes a `TypeError` there must become a `Mark`
/// here, with the same reason. Two spellings in two files would drift on the
/// first refusal nobody thought to mirror.
///
/// # Contract
/// - ensures: the mark carries the same reason [`refusal_error`] gives the same
///   refusal.
/// - panics: none.
#[inline]
#[must_use]
pub fn refusal_mark(refusal: PackageRefusal) -> Mark
{
    match refusal {
        | PackageRefusal::ArityMismatch { .. } => Mark::Stuck {
            hint: text::PACK_ARITY_MISMATCH,
        },
        | PackageRefusal::DuplicateComponent { .. } => Mark::Stuck {
            hint: text::PACKAGE_DUPLICATE_COMPONENT,
        },
        | PackageRefusal::AbstractUnderPathEndpoint { .. } => Mark::Stuck {
            hint: text::PACKAGE_ABSTRACT_UNDER_PATH,
        },
        | PackageRefusal::PayloadNotGradedThunk { payload } => Mark::ShapeMismatch {
            expected: text::SHAPE_PACKAGE_PAYLOAD,
            actual: Ty::Value(payload),
        },
        | PackageRefusal::AtomsNotDistinct => Mark::Stuck {
            hint: text::UNPACK_ATOM_MISMATCH,
        },
    }
}

/// The type a `pack`'s payload must check against, given the expected package
/// type's parts and the term's witness types.
///
/// # Contract
/// - requires: `grade`, `abstracts` and `payload` are one package type's parts.
/// - ensures: on `Ok`, the payload type with every abstract component replaced
///   by its witness.
/// - fails: [`PackageRefusal::PayloadNotGradedThunk`] when the signature's
///   payload is not a thunk at the package's own grade, and every refusal
///   [`instantiate`] can raise.
/// - panics: none.
///
/// # Errors
/// As `- fails:`.
#[inline]
pub fn pack_payload_expectation(
    grade: Grade,
    abstracts: &[String],
    payload: &ValueType,
    witnesses: &[Rc<ValueType>],
) -> Result<ValueType, PackageRefusal>
{
    payload_is_graded_as(grade, payload)?;
    instantiate(abstracts, payload, witnesses)
}

/// The type an `unpack` binds its module variable at, given the ascribed
/// package type's parts and the atoms the elimination records.
///
/// This is where abstraction is bought: the witnesses are the minted atoms, so
/// the body meets the payload at abstract types rather than at whatever the
/// packer supplied.
///
/// # Contract
/// - requires: `grade`, `abstracts` and `payload` are one package type's parts.
/// - ensures: on `Ok`, the payload type with abstract component `i` replaced by
///   [`ValueType::Sealed`] at `atoms[i]`.
/// - fails: [`PackageRefusal::PayloadNotGradedThunk`] as above,
///   [`PackageRefusal::ArityMismatch`] when the atom count disagrees,
///   [`PackageRefusal::AtomsNotDistinct`] when two components share an atom,
///   and every refusal [`instantiate`] can raise.
/// - panics: none.
///
/// # Errors
/// As `- fails:`.
#[inline]
pub fn unpack_binding(
    grade: Grade,
    abstracts: &[String],
    payload: &ValueType,
    atoms: &[SealId],
) -> Result<ValueType, PackageRefusal>
{
    payload_is_graded_as(grade, payload)?;
    if atoms.len() != abstracts.len() {
        return Err(PackageRefusal::ArityMismatch {
            declared: PackageArity::from(abstracts.len()),
            supplied: PackageArity::from(atoms.len()),
        });
    }
    let distinct: BTreeSet<&SealId> = atoms.iter().collect();
    if distinct.len() != atoms.len() {
        return Err(PackageRefusal::AtomsNotDistinct);
    }
    let witnesses: Vec<Rc<ValueType>> = atoms
        .iter()
        .map(|atom| Rc::new(ValueType::Sealed(atom.clone())))
        .collect();
    instantiate(abstracts, payload, &witnesses)
}

/// The payload as subtyping should compare it: with the outer thunk's grade
/// normalized away.
///
/// **The grade is one annotation and must be compared once.** A package's grade
/// and its payload thunk's grade are the same `r`, so a subtyping rule that
/// compares the payload invariantly *and* the package's grade contravariantly
/// compares that one annotation twice — and invariance wins, which makes the
/// contravariant leg dead. Measured rather than reasoned: `Package_ω σ` failed
/// to relate to `Package_1 σ` until this normalization landed, so the leg the
/// ruling asks for was present in the code and unreachable in the relation.
///
/// Normalizing rather than stripping keeps a **malformed** payload comparable:
/// a payload whose grade is not the package's is left exactly as it is, so it
/// still fails against a well-formed one rather than being quietly repaired.
///
/// The payload is otherwise compared **invariantly**, the
/// [`ValueType::Sigma`] precedent. Covariance is available and sound for an
/// existential-shaped former, and it is deliberately not taken here: it would
/// relax width and depth under the abstraction barrier, which is a wider claim
/// than the grade leg needs.
///
/// # Contract
/// - ensures: a payload that is a thunk at `grade` returns the same thunk at a
///   fixed canonical grade; every other payload returns unchanged.
/// - panics: none.
#[inline]
#[must_use]
pub fn comparable_payload(
    grade: Grade,
    payload: &ValueType,
) -> ValueType
{
    match *payload {
        | ValueType::Thunk(payload_grade, ref body) if payload_grade == grade => {
            ValueType::Thunk(Grade::OMEGA, Rc::clone(body))
        },
        | ref other => other.clone(),
    }
}

/// The shared payload well-formedness leg: a package's payload is a thunk at
/// the package's own grade.
///
/// `Unknown` passes, because the gradual discipline makes it consistent with
/// every type in both directions and a hole must not turn into a shape refusal.
fn payload_is_graded_as(
    grade: Grade,
    payload: &ValueType,
) -> Result<(), PackageRefusal>
{
    match *payload {
        | ValueType::Unknown => Ok(()),
        | ValueType::Thunk(payload_grade, _) if payload_grade == grade => Ok(()),
        | ref other => Err(PackageRefusal::PayloadNotGradedThunk {
            payload: other.clone(),
        }),
    }
}

/// Discharge `abstracts` with `witnesses`, simultaneously, inside `payload`.
///
/// This is the one operation every package rule is written in terms of. The
/// binder at position `i` is replaced by the witness at position `i`, in a
/// single traversal, with shadowing and capture handled as the module
/// documentation describes.
///
/// # Contract
/// - requires: nothing; every precondition is checked and reported.
/// - ensures: on `Ok`, the result is `payload` with **every** occurrence of
///   every binder replaced by its witness, and no binder of `abstracts` occurs
///   free in it.
/// - provides: the abstraction step for packing (witnesses are representation
///   types), for unpacking (witnesses are freshly minted sealed atoms), and for
///   subtyping (witnesses are canonical positional binders).
/// - fails: [`PackageRefusal::ArityMismatch`] when the lengths disagree,
///   [`PackageRefusal::DuplicateComponent`] when a label is declared twice, and
///   [`PackageRefusal::AbstractUnderPathEndpoint`] when an identity endpoint
///   mentions a discharged binder.
/// - panics: none.
///
/// # Errors
/// As `- fails:`.
///
/// # Adequacy
/// - hypothesis: L2 — a well-formed instantiation replaces every binder and
///   leaves everything else alone; the L3 residues are the three refusals and
///   the two binder interactions (shadowing, capture), each with its own
///   witness.
/// - witness: `package::tests::instantiation_replaces_every_occurrence`
/// - witness: `package::tests::a_nested_binder_shadows_the_outer_one`
/// - witness: `package::tests::a_capturing_nested_binder_is_renamed`
/// - witness: `package::tests::substitution_is_simultaneous_not_sequential`
/// - witness: `package::tests::an_arity_disagreement_is_refused`
/// - witness: `package::tests::a_duplicated_component_is_refused`
/// - witness: `package::tests::a_path_endpoint_naming_a_binder_is_refused`
#[inline]
pub fn instantiate(
    abstracts: &[String],
    payload: &ValueType,
    witnesses: &[Rc<ValueType>],
) -> Result<ValueType, PackageRefusal>
{
    if abstracts.len() != witnesses.len() {
        return Err(PackageRefusal::ArityMismatch {
            declared: PackageArity::from(abstracts.len()),
            supplied: PackageArity::from(witnesses.len()),
        });
    }
    let mut subst = Substitution::new();
    for (component, witness) in abstracts.iter().zip(witnesses.iter()) {
        let prior = subst.insert(component.clone(), Rc::clone(witness));
        if prior.is_some() {
            return Err(PackageRefusal::DuplicateComponent {
                component: component.clone(),
            });
        }
    }
    discharge(payload, Rc::new(subst))
}

/// The canonical binder for position `index`, used to compare two package types
/// up to α-renaming.
///
/// Both sides of a comparison are instantiated at these names, so neither
/// side's own spelling can decide the answer. The `#` is what keeps the name
/// out of the source-level namespace: gandr identifiers do not contain it, so a
/// canonical binder can never coincide with a rigid atom a program wrote.
#[inline]
#[must_use]
pub fn canonical_binder(index: PackageArity) -> String
{
    alloc::format!("package#{}", usize::from(index))
}

/// The canonical binder atoms for a package of `arity`, in position order.
///
/// The witness list [`instantiate`] takes when a caller wants α-alignment
/// rather than instantiation — [`crate::discipline::subtype`]'s package arm is
/// the caller.
#[inline]
#[must_use]
pub fn canonical_witnesses(arity: PackageArity) -> Vec<Rc<ValueType>>
{
    (0 .. usize::from(arity))
        .map(|index| {
            Rc::new(ValueType::atom(TypeAtomName::from(
                canonical_binder(PackageArity::from(index)).as_str(),
            )))
        })
        .collect()
}

/// The simultaneous substitution in force over one subtree: binder label to the
/// type it is discharged with.
type Substitution = BTreeMap<String, Rc<ValueType>>;

/// One task on the [`discharge`] worklist — the defunctionalized image of a
/// recursive traversal (the ADR-47 iterative discipline). A `Value` / `Comp`
/// task visits a source type under the substitution in force there; a `Finish*`
/// frame reassembles one node from the rebuilt children on the result stacks.
enum Task<'type_>
{
    /// Visit a value type under a substitution.
    Value(&'type_ ValueType, Rc<Substitution>),
    /// Visit a computation type under a substitution.
    Comp(&'type_ CompType, Rc<Substitution>),
    /// Reassemble a value type.
    FinishValue(ValueFinish<'type_>),
    /// Reassemble a computation type.
    FinishComp(CompFinish),
}

/// A pending value-type reassembly — what a [`Task::FinishValue`] frame needs
/// once its rebuilt children sit on the result stack.
enum ValueFinish<'type_>
{
    /// Rebuild a product from its rebuilt components.
    Prod,
    /// Rebuild a sum from its rebuilt summands.
    Sum,
    /// Rebuild a list type from its rebuilt element type.
    List,
    /// Rebuild a record type from its rebuilt fields, in the saved label order.
    Record(Vec<String>),
    /// Rebuild a graded thunk from its grade and rebuilt body.
    Thunk(Grade),
    /// Rebuild a reified-stack type from its rebuilt halves.
    Stk,
    /// Rebuild an identity type from its rebuilt carrier and its endpoints,
    /// which are carried through unchanged (the refusal above guarantees they
    /// mention nothing being discharged).
    Path(&'type_ Rc<Value>, &'type_ Rc<Value>),
    /// Rebuild a declared-data application from its id and rebuilt arguments.
    Data(DataId, usize),
    /// Rebuild a dependent pair from its rebuilt head and tail; the value
    /// binder is inert here, since this substitution replaces type atoms.
    Sigma(String),
    /// Rebuild a package from its grade, its (possibly renamed) binder list,
    /// and its rebuilt payload.
    Package(Grade, Vec<String>),
}

/// A pending computation-type reassembly (see [`ValueFinish`]).
enum CompFinish
{
    /// Rebuild a returner from its rebuilt payload and its effect row.
    F(EffectRow),
    /// Rebuild an arrow from its rebuilt argument and result.
    Arrow,
    /// Rebuild a `with` type from its rebuilt components.
    With,
}

/// The iterative engine behind [`instantiate`].
///
/// It drains an explicit LIFO task stack onto one result stack per type sort,
/// so substitution depth follows the heap rather than the host call stack. The
/// substitution in force travels **with** each task rather than in the engine,
/// which is what lets a nested package narrow it for its own subtree without
/// affecting its siblings.
///
/// # Contract
/// - ensures: on `Ok` the value result stack holds exactly one rebuilt type.
/// - fails: as [`instantiate`].
/// - panics: none (a `debug_assert` guards the post-order balance in debug and
///   test builds).
///
/// # Termination
/// - reason: the engine drains a finite task stack.
/// - measure: pending tasks and result frames.
/// - boundedness: source types are finite Rust values, and binder renaming adds
///   no node.
/// - input recursion: none.
fn discharge(
    payload: &ValueType,
    subst: Rc<Substitution>,
) -> Result<ValueType, PackageRefusal>
{
    let mut tasks = alloc::vec![Task::Value(payload, subst)];
    let mut values: Vec<ValueType> = Vec::new();
    let mut comps: Vec<CompType> = Vec::new();
    while let Some(task) = tasks.pop() {
        match task {
            | Task::Value(ty, subst) => {
                if subst.is_empty() {
                    values.push(ty.clone());
                    continue;
                }
                match *ty {
                    | ValueType::Atom(ref name) => match subst.get(name) {
                        | Some(witness) => values.push(witness.as_ref().clone()),
                        | None => values.push(ty.clone()),
                    },
                    | ValueType::Unit
                    | ValueType::Unknown
                    | ValueType::Universe
                    | ValueType::Sealed(_) => values.push(ty.clone()),
                    | ValueType::Prod(ref fst, ref snd) => {
                        tasks.push(Task::FinishValue(ValueFinish::Prod));
                        tasks.push(Task::Value(snd, Rc::clone(&subst)));
                        tasks.push(Task::Value(fst, subst));
                    },
                    | ValueType::Sum(ref lhs, ref rhs) => {
                        tasks.push(Task::FinishValue(ValueFinish::Sum));
                        tasks.push(Task::Value(rhs, Rc::clone(&subst)));
                        tasks.push(Task::Value(lhs, subst));
                    },
                    | ValueType::List(ref element) => {
                        tasks.push(Task::FinishValue(ValueFinish::List));
                        tasks.push(Task::Value(element, subst));
                    },
                    | ValueType::Record(ref fields) => {
                        let labels = fields.keys().cloned().collect::<Vec<_>>();
                        tasks.push(Task::FinishValue(ValueFinish::Record(labels)));
                        for field in fields.values().rev() {
                            tasks.push(Task::Value(field, Rc::clone(&subst)));
                        }
                    },
                    | ValueType::Thunk(grade, ref body) => {
                        tasks.push(Task::FinishValue(ValueFinish::Thunk(grade)));
                        tasks.push(Task::Comp(body, subst));
                    },
                    | ValueType::Stk(ref consumes, ref delivers) => {
                        tasks.push(Task::FinishValue(ValueFinish::Stk));
                        tasks.push(Task::Comp(delivers, Rc::clone(&subst)));
                        tasks.push(Task::Comp(consumes, subst));
                    },
                    | ValueType::Path {
                        ty: ref carrier,
                        ref lhs,
                        ref rhs,
                    } => {
                        let refusal = endpoint_refusal(lhs, rhs, subst.as_ref());
                        if let Some(refusal) = refusal {
                            return Err(refusal);
                        }
                        tasks.push(Task::FinishValue(ValueFinish::Path(lhs, rhs)));
                        tasks.push(Task::Value(carrier, subst));
                    },
                    | ValueType::Data { ref id, ref args } => {
                        tasks.push(Task::FinishValue(ValueFinish::Data(id.clone(), args.len())));
                        for arg in args.iter().rev() {
                            tasks.push(Task::Value(arg, Rc::clone(&subst)));
                        }
                    },
                    | ValueType::Sigma {
                        ref fst,
                        ref binder,
                        ref snd,
                    } => {
                        tasks.push(Task::FinishValue(ValueFinish::Sigma(binder.clone())));
                        tasks.push(Task::Value(snd, Rc::clone(&subst)));
                        tasks.push(Task::Value(fst, subst));
                    },
                    | ValueType::Package {
                        grade,
                        ref abstracts,
                        ref payload,
                    } => {
                        let (renamed, inner) = enter_package(abstracts, payload, subst.as_ref());
                        tasks.push(Task::FinishValue(ValueFinish::Package(grade, renamed)));
                        tasks.push(Task::Value(payload, Rc::new(inner)));
                    },
                }
            },
            | Task::Comp(ty, subst) => {
                if subst.is_empty() {
                    comps.push(ty.clone());
                    continue;
                }
                match *ty {
                    | CompType::Unknown => comps.push(CompType::Unknown),
                    | CompType::F(ref of, ref row) => {
                        tasks.push(Task::FinishComp(CompFinish::F(row.clone())));
                        tasks.push(Task::Value(of, subst));
                    },
                    | CompType::Arrow(ref arg, ref res) => {
                        tasks.push(Task::FinishComp(CompFinish::Arrow));
                        tasks.push(Task::Comp(res, Rc::clone(&subst)));
                        tasks.push(Task::Value(arg, subst));
                    },
                    | CompType::With(ref fst, ref snd) => {
                        tasks.push(Task::FinishComp(CompFinish::With));
                        tasks.push(Task::Comp(snd, Rc::clone(&subst)));
                        tasks.push(Task::Comp(fst, subst));
                    },
                }
            },
            | Task::FinishValue(finish) => match finish {
                | ValueFinish::Prod => {
                    let snd = pop_value(&mut values);
                    let fst = pop_value(&mut values);
                    values.push(ValueType::Prod(Rc::new(fst), Rc::new(snd)));
                },
                | ValueFinish::Sum => {
                    let rhs = pop_value(&mut values);
                    let lhs = pop_value(&mut values);
                    values.push(ValueType::Sum(Rc::new(lhs), Rc::new(rhs)));
                },
                | ValueFinish::List => {
                    let element = pop_value(&mut values);
                    values.push(ValueType::List(Rc::new(element)));
                },
                | ValueFinish::Record(labels) => {
                    let mut rebuilt = Vec::with_capacity(labels.len());
                    for _ in 0 .. labels.len() {
                        rebuilt.push(pop_value(&mut values));
                    }
                    rebuilt.reverse();
                    let mut fields = BTreeMap::new();
                    for (label, field) in labels.into_iter().zip(rebuilt) {
                        let _prior = fields.insert(label, Rc::new(field));
                    }
                    values.push(ValueType::Record(fields));
                },
                | ValueFinish::Thunk(grade) => {
                    let body = pop_comp(&mut comps);
                    values.push(ValueType::Thunk(grade, Rc::new(body)));
                },
                | ValueFinish::Stk => {
                    let delivers = pop_comp(&mut comps);
                    let consumes = pop_comp(&mut comps);
                    values.push(ValueType::Stk(Rc::new(consumes), Rc::new(delivers)));
                },
                | ValueFinish::Path(lhs, rhs) => {
                    let carrier = pop_value(&mut values);
                    values.push(ValueType::Path {
                        ty: Rc::new(carrier),
                        lhs: Rc::clone(lhs),
                        rhs: Rc::clone(rhs),
                    });
                },
                | ValueFinish::Data(id, count) => {
                    let mut args = Vec::with_capacity(count);
                    for _ in 0 .. count {
                        args.push(pop_value(&mut values));
                    }
                    args.reverse();
                    values.push(ValueType::Data {
                        id,
                        args: args.into_iter().map(Rc::new).collect(),
                    });
                },
                | ValueFinish::Sigma(binder) => {
                    let snd = pop_value(&mut values);
                    let fst = pop_value(&mut values);
                    values.push(ValueType::Sigma {
                        fst: Rc::new(fst),
                        binder,
                        snd: Rc::new(snd),
                    });
                },
                | ValueFinish::Package(grade, abstracts) => {
                    let payload = pop_value(&mut values);
                    values.push(ValueType::Package {
                        grade,
                        abstracts,
                        payload: Rc::new(payload),
                    });
                },
            },
            | Task::FinishComp(finish) => match finish {
                | CompFinish::F(row) => {
                    let of = pop_value(&mut values);
                    comps.push(CompType::F(Rc::new(of), row));
                },
                | CompFinish::Arrow => {
                    let res = pop_comp(&mut comps);
                    let arg = pop_value(&mut values);
                    comps.push(CompType::Arrow(Rc::new(arg), Rc::new(res)));
                },
                | CompFinish::With => {
                    let snd = pop_comp(&mut comps);
                    let fst = pop_comp(&mut comps);
                    comps.push(CompType::With(Rc::new(fst), Rc::new(snd)));
                },
            },
        }
    }
    Ok(pop_value(&mut values))
}

/// Narrow a substitution for a nested package's subtree, renaming the nested
/// binders that would capture.
///
/// Returns the nested package's binder list as it must be rebuilt, and the
/// substitution its payload is traversed under. Two adjustments happen, in this
/// order, and the order is what makes them compose: a nested binder first
/// **removes** any outer binder of the same name from the substitution
/// (shadowing), and a nested binder that occurs in some surviving witness is
/// then **renamed** so that witness cannot be captured when it lands inside.
///
/// # Contract
/// - ensures: no name in the returned binder list occurs in any witness of the
///   returned substitution, and no returned binder is a key of it.
/// - panics: none.
#[must_use]
fn enter_package(
    abstracts: &[String],
    payload: &Rc<ValueType>,
    outer: &Substitution,
) -> (Vec<String>, Substitution)
{
    let mut inner: Substitution = outer
        .iter()
        .filter(|&(key, _)| !abstracts.contains(key))
        .map(|(key, witness)| (key.clone(), Rc::clone(witness)))
        .collect();
    if inner.is_empty() {
        return (abstracts.to_vec(), inner);
    }
    let mut occupied: BTreeSet<String> = BTreeSet::new();
    for witness in inner.values() {
        collect_atom_names(witness, &mut occupied);
    }
    let capturing = abstracts
        .iter()
        .any(|binder| occupied.contains(binder.as_str()));
    if !capturing {
        return (abstracts.to_vec(), inner);
    }
    collect_atom_names(payload, &mut occupied);
    occupied.extend(abstracts.iter().cloned());
    occupied.extend(inner.keys().cloned());
    let mut renamed = Vec::with_capacity(abstracts.len());
    for binder in abstracts {
        let mut fresh = binder.clone();
        let mut counter: usize = 0;
        while occupied.contains(&fresh) {
            counter = counter.saturating_add(1);
            fresh = alloc::format!("{binder}#{counter}");
        }
        if fresh == *binder {
            renamed.push(binder.clone());
        }
        else {
            let _prior = inner.insert(
                binder.clone(),
                Rc::new(ValueType::atom(TypeAtomName::from(fresh.as_str()))),
            );
            let _fresh = occupied.insert(fresh.clone());
            renamed.push(fresh);
        }
    }
    (renamed, inner)
}

/// The refusal an identity type's endpoints earn, if any: `Some` exactly when
/// an endpoint mentions a name the substitution discharges.
///
/// See the module documentation for why this is a refusal rather than a
/// pass-through.
#[must_use]
fn endpoint_refusal(
    lhs: &Rc<Value>,
    rhs: &Rc<Value>,
    subst: &Substitution,
) -> Option<PackageRefusal>
{
    let mut mentioned: BTreeSet<String> = BTreeSet::new();
    let mut embedded: Vec<TypeRef<'_>> = Vec::new();
    collect_embedded_types(lhs, &mut embedded);
    collect_embedded_types(rhs, &mut embedded);
    for ty in embedded {
        match ty {
            | TypeRef::Value(ty) => collect_atom_names(ty, &mut mentioned),
            | TypeRef::Comp(ty) => collect_comp_atom_names(ty, &mut mentioned),
        }
    }
    subst
        .keys()
        .find(|component| mentioned.contains(component.as_str()))
        .map(|component| PackageRefusal::AbstractUnderPathEndpoint {
            component: component.clone(),
        })
}

/// A borrowed type of either sort, as [`collect_embedded_types`] reports it.
enum TypeRef<'term>
{
    /// A value type embedded in a term.
    Value(&'term ValueType),
    /// A computation type embedded in a term.
    Comp(&'term CompType),
}

/// One node of a term traversal — the borrowed sorts a term is built from.
enum TermRef<'term>
{
    /// A value node.
    Value(&'term Value),
    /// A computation node.
    Comp(&'term Comp),
    /// A reified-stack node.
    Stack(&'term Stack),
}

/// Collect every type a term embeds, at any depth.
///
/// The traversal is exhaustive over the term language on purpose: a variant
/// added later that carries a type will not compile against this match until it
/// is accounted for, which is what keeps [`endpoint_refusal`] from going
/// quietly blind as the syntax grows.
///
/// # Contract
/// - ensures: `out` gains every [`ValueType`] and [`CompType`] reachable from
///   `value` through term structure.
/// - panics: none.
///
/// # Termination
/// - reason: the traversal drains a finite worklist over a finite term.
/// - measure: pending term nodes.
/// - boundedness: terms are finite Rust values.
/// - input recursion: none.
fn collect_embedded_types<'term>(
    value: &'term Value,
    out: &mut Vec<TypeRef<'term>>,
)
{
    let mut work = alloc::vec![TermRef::Value(value)];
    while let Some(node) = work.pop() {
        match node {
            | TermRef::Value(node) => match *node {
                | Value::Var(_)
                | Value::Unit
                | Value::Int(_)
                | Value::Str(_)
                | Value::Num(_)
                | Value::Hole(_) => {},
                | Value::Pair(ref fst, ref snd) => {
                    work.push(TermRef::Value(fst));
                    work.push(TermRef::Value(snd));
                },
                | Value::Inj(_, ref payload) | Value::Here(ref payload) => {
                    work.push(TermRef::Value(payload));
                },
                | Value::Ctor { ref payload, .. } => work.push(TermRef::Value(payload)),
                | Value::List(ref elements) => {
                    work.extend(elements.iter().map(|element| TermRef::Value(element)));
                },
                | Value::Record(ref fields) => {
                    work.extend(fields.values().map(|field| TermRef::Value(field)));
                },
                | Value::Thunk(_, ref body) => work.push(TermRef::Comp(body)),
                | Value::Annot(ref inner, ref ty) => {
                    out.push(TypeRef::Value(ty));
                    work.push(TermRef::Value(inner));
                },
                | Value::Stk(ref stack) => work.push(TermRef::Stack(stack)),
                | Value::Pack {
                    ref witnesses,
                    ref payload,
                } => {
                    out.extend(witnesses.iter().map(|witness| TypeRef::Value(witness)));
                    work.push(TermRef::Value(payload));
                },
            },
            | TermRef::Comp(node) => match *node {
                | Comp::Hole(_) => {},
                | Comp::Abs(_, ref annot, ref body) => {
                    if let Some(ref ty) = *annot {
                        out.push(TypeRef::Value(ty));
                    }
                    work.push(TermRef::Comp(body));
                },
                | Comp::App(ref head, ref arg) => {
                    work.push(TermRef::Comp(head));
                    work.push(TermRef::Value(arg));
                },
                | Comp::Ret(ref produced) => work.push(TermRef::Value(produced)),
                | Comp::Bind(ref bound, _, ref rest) => {
                    work.push(TermRef::Comp(bound));
                    work.push(TermRef::Comp(rest));
                },
                | Comp::Force(ref thunked) | Comp::Dup(ref thunked) | Comp::Drop(ref thunked) => {
                    work.push(TermRef::Value(thunked));
                },
                | Comp::Case(ref scrut, (_, ref lhs), (_, ref rhs)) => {
                    work.push(TermRef::Value(scrut));
                    work.push(TermRef::Comp(lhs));
                    work.push(TermRef::Comp(rhs));
                },
                | Comp::DataCase(ref scrut, ref arms) => {
                    work.push(TermRef::Value(scrut));
                    work.extend(arms.iter().map(|arm| TermRef::Comp(&arm.1)));
                },
                | Comp::ListCase {
                    ref scrut,
                    ref nil,
                    ref cons,
                    ..
                } => {
                    work.push(TermRef::Value(scrut));
                    work.push(TermRef::Comp(nil));
                    work.push(TermRef::Comp(cons));
                },
                | Comp::Split {
                    ref scrut,
                    ref motive,
                    ref body,
                    ..
                } => {
                    if let Some(ref motive) = *motive {
                        out.push(TypeRef::Comp(&motive.body));
                    }
                    work.push(TermRef::Value(scrut));
                    work.push(TermRef::Comp(body));
                },
                | Comp::RecordProj { ref record, .. } => work.push(TermRef::Value(record)),
                | Comp::With(ref fst, ref snd) => {
                    work.push(TermRef::Comp(fst));
                    work.push(TermRef::Comp(snd));
                },
                | Comp::Prj(_, ref target) | Comp::Reset(ref target) => {
                    work.push(TermRef::Comp(target));
                },
                | Comp::Perform(ref sig, _, ref argument) => {
                    push_signature_types(sig, out);
                    work.push(TermRef::Value(argument));
                },
                | Comp::Handle {
                    ref sig,
                    ref scrutinee,
                    ref ret,
                    ref ops,
                } => {
                    push_signature_types(sig, out);
                    work.push(TermRef::Comp(scrutinee));
                    work.push(TermRef::Comp(&ret.1));
                    work.extend(ops.iter().map(|clause| TermRef::Comp(&clause.body)));
                },
                | Comp::Resume(ref stack, ref fed) => {
                    work.push(TermRef::Value(stack));
                    work.push(TermRef::Comp(fed));
                },
                | Comp::Shift(_, ref body) => work.push(TermRef::Comp(body)),
                | Comp::Native { ref args, .. } => {
                    work.extend(args.iter().map(|arg| TermRef::Value(arg)));
                },
                | Comp::Walk {
                    ref scrut,
                    ref motive,
                    ref base,
                } => {
                    out.push(TypeRef::Comp(&motive.body));
                    work.push(TermRef::Value(scrut));
                    work.push(TermRef::Comp(&base.body));
                },
                | Comp::Unpack {
                    ref scrut,
                    ref signature,
                    ref body,
                    ..
                } => {
                    out.push(TypeRef::Value(signature));
                    work.push(TermRef::Value(scrut));
                    work.push(TermRef::Comp(body));
                },
            },
            | TermRef::Stack(node) => match *node {
                | Stack::Empty => {},
                | Stack::Arg(ref argument, ref rest) => {
                    work.push(TermRef::Value(argument));
                    work.push(TermRef::Stack(rest));
                },
                | Stack::Bind(_, ref continuation, ref rest) => {
                    work.push(TermRef::Comp(continuation));
                    work.push(TermRef::Stack(rest));
                },
                | Stack::Prj(_, ref rest) => work.push(TermRef::Stack(rest)),
            },
        }
    }
}

/// Push an effect signature's operation types onto the embedded-type list.
fn push_signature_types<'term>(
    sig: &'term gandr_core_term::effect::EffectSig,
    out: &mut Vec<TypeRef<'term>>,
)
{
    for op in sig.ops() {
        out.push(TypeRef::Value(op.payload()));
        out.push(TypeRef::Value(op.reply()));
    }
}

/// Collect every type-atom name and package binder occurring in a value type.
///
/// The set is deliberately **conservative** — it does not distinguish a bound
/// binder from a free atom — because both callers want a set of names to avoid,
/// and avoiding one name too many only ever makes a rename more eager.
///
/// # Contract
/// - ensures: `out` gains every [`ValueType::Atom`] name and every package
///   binder reachable from `ty` through type structure, including through the
///   types embedded in identity endpoints.
/// - panics: none.
///
/// # Termination
/// - reason: the traversal drains a finite worklist over a finite type.
/// - measure: pending type nodes.
/// - boundedness: types are finite Rust values.
/// - input recursion: none.
fn collect_atom_names(
    ty: &ValueType,
    out: &mut BTreeSet<String>,
)
{
    let mut work = alloc::vec![TypeRef::Value(ty)];
    while let Some(node) = work.pop() {
        match node {
            | TypeRef::Value(node) => match *node {
                | ValueType::Atom(ref name) => {
                    let _fresh = out.insert(name.clone());
                },
                | ValueType::Unit
                | ValueType::Unknown
                | ValueType::Universe
                | ValueType::Sealed(_) => {},
                // Products, sums and dependent pairs are the same walk: both
                // children are types and neither binds a type name.
                | ValueType::Prod(ref fst, ref snd)
                | ValueType::Sum(ref fst, ref snd)
                | ValueType::Sigma {
                    ref fst, ref snd, ..
                } => {
                    work.push(TypeRef::Value(fst));
                    work.push(TypeRef::Value(snd));
                },
                | ValueType::List(ref element) => work.push(TypeRef::Value(element)),
                | ValueType::Record(ref fields) => {
                    work.extend(fields.values().map(|field| TypeRef::Value(field)));
                },
                | ValueType::Thunk(_, ref body) => work.push(TypeRef::Comp(body)),
                | ValueType::Stk(ref consumes, ref delivers) => {
                    work.push(TypeRef::Comp(consumes));
                    work.push(TypeRef::Comp(delivers));
                },
                | ValueType::Path {
                    ty: ref carrier,
                    ref lhs,
                    ref rhs,
                } => {
                    work.push(TypeRef::Value(carrier));
                    let mut embedded: Vec<TypeRef<'_>> = Vec::new();
                    collect_embedded_types(lhs, &mut embedded);
                    collect_embedded_types(rhs, &mut embedded);
                    work.extend(embedded);
                },
                | ValueType::Data { ref args, .. } => {
                    work.extend(args.iter().map(|arg| TypeRef::Value(arg)));
                },
                | ValueType::Package {
                    ref abstracts,
                    ref payload,
                    ..
                } => {
                    out.extend(abstracts.iter().cloned());
                    work.push(TypeRef::Value(payload));
                },
            },
            | TypeRef::Comp(node) => match *node {
                | CompType::Unknown => {},
                | CompType::F(ref of, _) => work.push(TypeRef::Value(of)),
                | CompType::Arrow(ref arg, ref res) => {
                    work.push(TypeRef::Value(arg));
                    work.push(TypeRef::Comp(res));
                },
                | CompType::With(ref fst, ref snd) => {
                    work.push(TypeRef::Comp(fst));
                    work.push(TypeRef::Comp(snd));
                },
            },
        }
    }
}

/// [`collect_atom_names`] entered at a computation type.
fn collect_comp_atom_names(
    ty: &CompType,
    out: &mut BTreeSet<String>,
)
{
    match *ty {
        | CompType::Unknown => {},
        | CompType::F(ref of, _) => collect_atom_names(of, out),
        | CompType::Arrow(ref arg, ref res) => {
            collect_atom_names(arg, out);
            collect_comp_atom_names_iter(res, out);
        },
        | CompType::With(ref fst, ref snd) => {
            collect_comp_atom_names_iter(fst, out);
            collect_comp_atom_names_iter(snd, out);
        },
    }
}

/// The worklist body behind [`collect_comp_atom_names`], entered at a
/// computation type without re-entering the value-sorted wrapper.
fn collect_comp_atom_names_iter(
    ty: &CompType,
    out: &mut BTreeSet<String>,
)
{
    collect_atom_names(&ValueType::Thunk(Grade::OMEGA, Rc::new(ty.clone())), out);
}

/// Pops the most-recent rebuilt value type, with a balance-invariant guard.
fn pop_value(values: &mut Vec<ValueType>) -> ValueType
{
    debug_assert!(
        !values.is_empty(),
        "package instantiation worklist underflow (post-order balance)"
    );
    values.pop().unwrap_or(ValueType::Unknown)
}

/// Pops the most-recent rebuilt computation type (see [`pop_value`]).
fn pop_comp(comps: &mut Vec<CompType>) -> CompType
{
    debug_assert!(
        !comps.is_empty(),
        "package instantiation worklist underflow (post-order balance)"
    );
    comps.pop().unwrap_or(CompType::Unknown)
}

#[cfg(test)]
mod tests
{
    use alloc::rc::Rc;
    use alloc::vec;
    use alloc::vec::Vec;

    use gandr_core_term::boundary::GradeBound;
    use gandr_core_term::effect::EffectRow;
    use gandr_core_term::syntax::Value;

    use super::*;

    /// `U_ω (F payload)` — the thunked module returner shape every package
    /// payload takes.
    fn returner_thunk(
        grade: Grade,
        payload: ValueType,
    ) -> ValueType
    {
        ValueType::thunk(grade, CompType::F(Rc::new(payload), EffectRow::EMPTY))
    }

    /// A one-component counter signature: `Package_ω ⟨t⟩ U_ω (F {seed: t})`.
    fn counter_signature() -> ValueType
    {
        ValueType::package(
            Grade::OMEGA,
            ["t"],
            returner_thunk(
                Grade::OMEGA,
                ValueType::record([("seed".to_owned(), ValueType::atom("t"))]),
            ),
        )
    }

    fn witnesses(types: Vec<ValueType>) -> Vec<Rc<ValueType>>
    {
        types.into_iter().map(Rc::new).collect()
    }

    /// Every occurrence of a binder is replaced, at any depth, and nothing else
    /// moves.
    #[test]
    fn instantiation_replaces_every_occurrence()
    {
        let ValueType::Package {
            ref abstracts,
            ref payload,
            ..
        } = counter_signature()
        else {
            panic!("the counter signature is a package");
        };
        let instantiated =
            instantiate(abstracts, payload, &witnesses(vec![ValueType::integer()])).unwrap();
        assert_eq!(
            returner_thunk(
                Grade::OMEGA,
                ValueType::record([("seed".to_owned(), ValueType::integer())])
            ),
            instantiated,
            "the abstract component is replaced by its witness wherever it occurs"
        );
    }

    /// A nested package binding the same label shadows the outer one, so the
    /// substitution stops at it.
    #[test]
    fn a_nested_binder_shadows_the_outer_one()
    {
        let abstracts = vec!["t".to_owned()];
        let inner = ValueType::package(Grade::OMEGA, ["t"], ValueType::atom("t"));
        let payload = ValueType::prod(ValueType::atom("t"), inner.clone());
        let instantiated =
            instantiate(&abstracts, &payload, &witnesses(vec![ValueType::integer()])).unwrap();
        assert_eq!(
            ValueType::prod(ValueType::integer(), inner),
            instantiated,
            "the outer occurrence substitutes and the shadowed inner one does not"
        );
    }

    /// **Capture avoidance.** A nested binder that occurs free in a witness is
    /// renamed before the traversal descends, so the witness's own atom keeps
    /// its meaning inside.
    #[test]
    fn a_capturing_nested_binder_is_renamed()
    {
        let abstracts = vec!["t".to_owned()];
        let payload = ValueType::package(
            Grade::OMEGA,
            ["u"],
            ValueType::prod(ValueType::atom("t"), ValueType::atom("u")),
        );
        let instantiated =
            instantiate(&abstracts, &payload, &witnesses(vec![ValueType::atom("u")])).unwrap();
        let ValueType::Package {
            abstracts: ref renamed,
            payload: ref inner,
            ..
        } = instantiated
        else {
            panic!("the result is still a package");
        };
        assert_ne!(
            vec!["u".to_owned()],
            *renamed,
            "the capturing binder is renamed rather than left to capture"
        );
        let fresh = renamed.first().cloned().unwrap_or_default();
        assert_eq!(
            ValueType::prod(ValueType::atom("u"), ValueType::atom(fresh.as_str())),
            *inner.as_ref(),
            "the witness's free atom survives and the renamed binder is distinct from it"
        );
    }

    /// The substitution is **simultaneous**: a witness mentioning a later
    /// binder is not re-substituted by that binder's own step.
    #[test]
    fn substitution_is_simultaneous_not_sequential()
    {
        let abstracts = vec!["a".to_owned(), "b".to_owned()];
        let payload = ValueType::prod(ValueType::atom("a"), ValueType::atom("b"));
        let instantiated = instantiate(
            &abstracts,
            &payload,
            &witnesses(vec![ValueType::atom("b"), ValueType::integer()]),
        )
        .unwrap();
        assert_eq!(
            ValueType::prod(ValueType::atom("b"), ValueType::integer()),
            instantiated,
            "a witness naming a later binder is not caught by that binder's own replacement"
        );
    }

    /// A witness count that is not the declared arity is refused rather than
    /// completed.
    #[test]
    fn an_arity_disagreement_is_refused()
    {
        let abstracts = vec!["t".to_owned(), "u".to_owned()];
        let payload = ValueType::atom("t");
        assert_eq!(
            Err(PackageRefusal::ArityMismatch {
                declared: PackageArity::from(2_usize),
                supplied: PackageArity::from(1_usize),
            }),
            instantiate(&abstracts, &payload, &witnesses(vec![ValueType::integer()])),
            "a missing witness leaves a binder free, so the instantiation is refused"
        );
    }

    /// A signature declaring one component twice is refused: the second binder
    /// would shadow the first and one witness could never be reached.
    #[test]
    fn a_duplicated_component_is_refused()
    {
        let abstracts = vec!["t".to_owned(), "t".to_owned()];
        let payload = ValueType::atom("t");
        assert_eq!(
            Err(PackageRefusal::DuplicateComponent {
                component: "t".to_owned(),
            }),
            instantiate(
                &abstracts,
                &payload,
                &witnesses(vec![ValueType::integer(), ValueType::string()])
            ),
            "a component declared twice is a defect in the signature"
        );
    }

    /// **The refusal that keeps a binder from leaking.** An identity endpoint
    /// mentioning a discharged component cannot be substituted through at this
    /// rung, so it is refused rather than passed on with the component still
    /// free.
    #[test]
    fn a_path_endpoint_naming_a_binder_is_refused()
    {
        let abstracts = vec!["t".to_owned()];
        let endpoint = Value::annot(Value::var("x"), ValueType::atom("t"));
        let payload = ValueType::path(ValueType::integer(), endpoint.clone(), endpoint);
        assert_eq!(
            Err(PackageRefusal::AbstractUnderPathEndpoint {
                component: "t".to_owned(),
            }),
            instantiate(&abstracts, &payload, &witnesses(vec![ValueType::integer()])),
            "an endpoint mentioning the component is refused rather than left unsubstituted"
        );
    }

    /// An endpoint that mentions no discharged component is passed through, so
    /// the refusal is as narrow as it claims.
    #[test]
    fn a_path_endpoint_naming_nothing_discharged_passes_through()
    {
        let abstracts = vec!["t".to_owned()];
        let endpoint = Value::annot(Value::var("x"), ValueType::integer());
        let payload = ValueType::prod(
            ValueType::atom("t"),
            ValueType::path(ValueType::integer(), endpoint.clone(), endpoint.clone()),
        );
        let instantiated =
            instantiate(&abstracts, &payload, &witnesses(vec![ValueType::string()])).unwrap();
        assert_eq!(
            ValueType::prod(
                ValueType::string(),
                ValueType::path(ValueType::integer(), endpoint.clone(), endpoint)
            ),
            instantiated,
            "an endpoint naming nothing discharged is carried through unchanged"
        );
    }

    /// The payload well-formedness leg: a package's grade and its payload
    /// thunk's grade are one `r`, so a payload graded otherwise is refused.
    #[test]
    fn a_payload_graded_otherwise_is_refused()
    {
        let payload = returner_thunk(Grade::ONE, ValueType::integer());
        assert_eq!(
            Err(PackageRefusal::PayloadNotGradedThunk {
                payload: payload.clone(),
            }),
            pack_payload_expectation(Grade::OMEGA, &[], &payload, &[]),
            "the package's grade and its payload thunk's grade are the same r"
        );
        assert!(
            pack_payload_expectation(Grade::ONE, &[], &payload, &[]).is_ok(),
            "the matching grade is accepted"
        );
    }

    /// An unpack whose recorded atoms repeat is refused: two components bound
    /// to one atom would be interchangeable inside the body.
    #[test]
    fn repeated_unpack_atoms_are_refused()
    {
        let payload = returner_thunk(
            Grade::OMEGA,
            ValueType::prod(ValueType::atom("t"), ValueType::atom("u")),
        );
        let abstracts = vec!["t".to_owned(), "u".to_owned()];
        let atom = SealId::new(0_u64, "M", "t");
        assert_eq!(
            Err(PackageRefusal::AtomsNotDistinct),
            unpack_binding(Grade::OMEGA, &abstracts, &payload, &[
                atom.clone(),
                atom.clone()
            ]),
            "one atom cannot stand for two components"
        );
        let second = SealId::new(1_u64, "M", "u");
        let bound = unpack_binding(Grade::OMEGA, &abstracts, &payload, &[
            atom.clone(),
            second.clone(),
        ])
        .unwrap();
        assert_eq!(
            returner_thunk(
                Grade::OMEGA,
                ValueType::prod(ValueType::Sealed(atom), ValueType::Sealed(second))
            ),
            bound,
            "each component binds at its own minted atom"
        );
    }

    /// The canonical binders subtyping aligns at cannot collide with a
    /// source-level atom, because gandr identifiers carry no `#`.
    #[test]
    fn canonical_binders_are_outside_the_source_namespace()
    {
        let binder = canonical_binder(PackageArity::from(0_usize));
        assert!(
            binder.contains('#'),
            "a canonical binder is spelled with a character no source identifier carries"
        );
        let all = canonical_witnesses(PackageArity::from(2_usize));
        assert_eq!(2, all.len(), "one canonical witness per position");
        assert_ne!(
            all.first().map(|first| first.as_ref().clone()),
            all.get(1).map(|second| second.as_ref().clone()),
            "distinct positions take distinct canonical binders"
        );
    }

    /// Grade construction used by the payload-grade witness above stays inside
    /// the semiring's own interface.
    #[test]
    fn a_finite_payload_grade_matches_its_package()
    {
        let grade = Grade::fin(GradeBound::from(2_u64));
        let payload = returner_thunk(grade, ValueType::Unit);
        assert!(
            pack_payload_expectation(grade, &[], &payload, &[]).is_ok(),
            "a finite grade matches its own payload"
        );
    }
}
