//! Dependency-footprint capture for the incremental typing pipeline (A2.3;
//! `incremental-pipeline.md` §4; `incremental-checkpoint work`).
//!
//! # What a footprint is
//!
//! `incremental-pipeline.md` §4 tags every checkpoint with a **dependency
//! footprint** — "the dynamic dependency edges an analysis consulted" (Reps,
//! Teitelbaum & Demers 1983, the §7 acknowledged ancestor) — and reuses the
//! checkpoint iff those edges are untouched. The spec states the footprint in
//! the *solver's* vocabulary (`{tyvars, gradeVars, trailDepth, stepId}`),
//! because its checkpoints live inside the stepping typing machine.
//!
//! This module captures the footprint the *current* architecture actually
//! exposes. With the tree-sitter-free melder push machine, the runnable typing
//! surface is item-granular: top-level items lower independently and are typed
//! against an accumulating context ([`Ctx`]) that a [`crate::session::Session`]
//! threads from item to item — a processed `def name = A` binds `name : A` for
//! later items to read. The one piece of shared, edit-mutable state threaded
//! across items is therefore that name → type context, and an item's footprint
//! is exactly **the set of context names its lowered term read** (its free
//! variables). This is the §4 footprint specialized to the item granularity:
//! condition 2 ("no `tyvars` in the footprint were re-assigned") becomes "no
//! *name* the item read had its binding change", the same soundness shape one
//! stratum up.
//!
//! [`Ctx`]: gandr_core_checker::ctx::Ctx
//!
//! # Soundness direction
//!
//! For invalidation to be sound the footprint must **over**-approximate the
//! true read-set — never miss a dependency (a missed edge lets a stale
//! checkpoint survive an input change that should have killed it).
//! [`footprint_of`] therefore captures every free variable exactly, and marks
//! the footprint [`Footprint::opaque`] on any core node it cannot represent (a
//! reified stack, an identity form, a declared-data node, or a native
//! builtin): an opaque footprint is treated as reading *everything*, forcing
//! the item to be re-typed rather than adopted. Over-approximation only costs
//! reuse; it never loses soundness.

use crate::boundary::DefinitionName;
use crate::boundary::MatchDecision;
extern crate alloc;

use alloc::collections::BTreeSet;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;

use gandr_core_checker::syntax::Comp;
use gandr_core_checker::syntax::Term;
use gandr_core_checker::syntax::Value;

/// The dependency footprint of one lowered item: the context names its term
/// read, plus the two conservative flags that force re-typing.
///
/// The [`Self::names`] set is the item's free variables — every name it read
/// from the ambient context (prelude operators and prior definitions alike).
/// The intersection of this set with the names whose binding an edit changed is
/// what decides reuse (`crate::checkpoint`): an item whose footprint avoids
/// every changed binding types identically against the new context, so its
/// cached result is still valid.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Footprint
{
    /// The free variable names the term read — the dependency edges into the
    /// ambient typing context.
    pub names: BTreeSet<String>,
    /// Set when the scan met a core node it cannot represent as a read-set (a
    /// reified [`Value::Stk`], an identity proof [`Value::Here`], a declared-
    /// data node, or a native builtin). An opaque footprint reads *everything*
    /// conservatively, so its item is never adopted — the safe
    /// over-approximation.
    pub opaque: bool,
    /// Set when the term carries a hole ([`Value::Hole`] / [`Comp::Hole`]): the
    /// item is parse-incomplete, matching the session's "holey items are not
    /// typed" discipline (`incremental-pipeline.md` §7).
    pub has_hole: bool,
}

impl Footprint
{
    /// Whether this footprint reads any name in `changed` — the reuse-blocking
    /// test. An [`Self::opaque`] footprint conservatively reads everything, so
    /// it intersects any non-empty change set (and, treated as reading
    /// everything, blocks reuse even against an empty one).
    ///
    /// # Contract
    /// - ensures: returns `true` iff reuse must be blocked — the footprint is
    ///   opaque, or some read name is in `changed`.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn intersects(
        &self,
        changed: &BTreeSet<String>,
    ) -> MatchDecision
    {
        if self.opaque {
            return MatchDecision(true);
        }
        MatchDecision(self.names.iter().any(|name| changed.contains(name)))
    }
}

/// A lexical scope: a persistent cons-list of the binder names in effect at a
/// term position, so the iterative scan can extend a scope for a child in O(1)
/// without mutating its parent's.
enum Scope<'term>
{
    /// The empty scope (the item root; a free variable here reads the ambient
    /// context).
    Root,
    /// A binder `name` in effect, over the enclosing `parent` scope.
    Bind
    {
        /// The bound name (borrowed from the term for the scan's duration).
        name: &'term str,
        /// The enclosing scope.
        parent: Rc<Self>,
    },
}

impl Scope<'_>
{
    /// Whether `name` is bound in this scope (an O(depth) walk of the
    /// cons-list; depth is the term's binder nesting).
    fn binds<'name>(
        &self,
        name: impl Into<DefinitionName<'name>>,
    ) -> MatchDecision
    {
        let name = name.into();
        let mut cursor = self;
        loop {
            match *cursor {
                | Self::Root => return MatchDecision(false),
                | Self::Bind {
                    name: bound,
                    ref parent,
                } => {
                    if bound == name.0 {
                        return MatchDecision(true);
                    }
                    cursor = parent;
                },
            }
        }
    }
}

/// One pending node in the iterative scan: a value or a computation, paired
/// with the lexical scope in force at its position (iterative-traversal design:
/// the traversal is a heap work-list, never host recursion on the user-sized
/// term).
enum Work<'term>
{
    /// A value node to scan under `scope`.
    Val(&'term Value, Rc<Scope<'term>>),
    /// A computation node to scan under `scope`.
    Cmp(&'term Comp, Rc<Scope<'term>>),
}

/// Borrows a computation node behind an `Rc` child (the [`as_val`] dual).
fn as_cmp(comp: &Rc<Comp>) -> &Comp
{
    comp
}

/// Scans one computation node: pushes children under the scope each extends
/// (binders add names), and sets the conservative flags.
fn scan_comp<'term>(
    comp: &'term Comp,
    scope: &Rc<Scope<'term>>,
    stack: &mut Vec<Work<'term>>,
    footprint: &mut Footprint,
)
{
    match *comp {
        | Comp::Abs(ref name, _, ref body) => {
            stack.push(Work::Cmp(as_cmp(body), extend(scope, name)));
        },
        | Comp::App(ref head, ref arg) => {
            stack.push(Work::Val(as_val(arg), Rc::clone(scope)));
            stack.push(Work::Cmp(as_cmp(head), Rc::clone(scope)));
        },
        | Comp::Ret(ref value)
        | Comp::Force(ref value)
        | Comp::Dup(ref value)
        | Comp::Drop(ref value) => {
            stack.push(Work::Val(as_val(value), Rc::clone(scope)));
        },
        | Comp::Bind(ref bound, ref name, ref cont) => {
            stack.push(Work::Cmp(as_cmp(bound), Rc::clone(scope)));
            stack.push(Work::Cmp(as_cmp(cont), extend(scope, name)));
        },
        | Comp::Case(
            ref scrut,
            (ref left_name, ref left_body),
            (ref right_name, ref right_body),
        ) => {
            stack.push(Work::Val(as_val(scrut), Rc::clone(scope)));
            stack.push(Work::Cmp(as_cmp(left_body), extend(scope, left_name)));
            stack.push(Work::Cmp(as_cmp(right_body), extend(scope, right_name)));
        },
        // The motive is a computation *type* (dependent-split design) carrying no free-variable
        // footprint obligation `footprint` tracks — as `Walk`'s motive does not;
        // only the scrutinee value and the body (under `p`/`q`) are walked.
        | Comp::Split {
            ref scrut,
            fst_name: ref first,
            snd_name: ref second,
            ref body,
            ..
        } => {
            stack.push(Work::Val(as_val(scrut), Rc::clone(scope)));
            stack.push(Work::Cmp(
                as_cmp(body),
                extend(&extend(scope, first), second),
            ));
        },
        | Comp::ListCase {
            ref scrut,
            ref nil,
            ref head,
            ref tail,
            ref cons,
        } => {
            stack.push(Work::Val(as_val(scrut), Rc::clone(scope)));
            stack.push(Work::Cmp(as_cmp(nil), Rc::clone(scope)));
            stack.push(Work::Cmp(as_cmp(cons), extend(&extend(scope, head), tail)));
        },
        | Comp::With(ref fst, ref snd) => {
            stack.push(Work::Cmp(as_cmp(fst), Rc::clone(scope)));
            stack.push(Work::Cmp(as_cmp(snd), Rc::clone(scope)));
        },
        | Comp::Prj(_, ref target) => {
            stack.push(Work::Cmp(as_cmp(target), Rc::clone(scope)));
        },
        | Comp::RecordProj { ref record, .. } => {
            stack.push(Work::Val(as_val(record), Rc::clone(scope)));
        },
        | Comp::Perform(_, _, ref arg) => {
            stack.push(Work::Val(as_val(arg), Rc::clone(scope)));
        },
        | Comp::Handle {
            ref scrutinee,
            ref ret,
            ref ops,
            ..
        } => {
            stack.push(Work::Cmp(as_cmp(scrutinee), Rc::clone(scope)));
            stack.push(Work::Cmp(as_cmp(&ret.1), extend(scope, &ret.0)));
            for clause in ops {
                stack.push(Work::Cmp(
                    as_cmp(&clause.body),
                    extend(&extend(scope, &clause.payload), &clause.resume),
                ));
            }
        },
        | Comp::Resume(ref stack_value, ref body) => {
            stack.push(Work::Val(as_val(stack_value), Rc::clone(scope)));
            stack.push(Work::Cmp(as_cmp(body), Rc::clone(scope)));
        },
        | Comp::Reset(ref body) => {
            stack.push(Work::Cmp(as_cmp(body), Rc::clone(scope)));
        },
        | Comp::Shift(ref binder, ref body) => {
            stack.push(Work::Cmp(as_cmp(body), extend(scope, binder)));
        },
        | Comp::Hole(_) => footprint.has_hole = true,
        // A declared-data case, native builtin, or identity walk — the forms
        // the read-set walk does not represent: conservatively opaque.
        | _ => footprint.opaque = true,
    }
}

/// Extends `scope` with one more bound `name`.
fn extend<'term>(
    scope: &Rc<Scope<'term>>,
    name: impl Into<DefinitionName<'term>>,
) -> Rc<Scope<'term>>
{
    let name = name.into();
    Rc::new(Scope::Bind {
        name: name.0,
        parent: Rc::clone(scope),
    })
}

/// Scans one value node: records a free occurrence, pushes children under the
/// same scope, and sets the conservative flags for holes and opaque nodes.
fn scan_value<'term>(
    value: &'term Value,
    scope: &Rc<Scope<'term>>,
    stack: &mut Vec<Work<'term>>,
    footprint: &mut Footprint,
)
{
    match *value {
        | Value::Var(ref name) => {
            if !scope.binds(name).0 {
                let _inserted = footprint.names.insert(name.clone());
            }
        },
        // Leaves that read no name.
        | Value::Unit | Value::Int(_) | Value::Str(_) | Value::Num(_) => {},
        | Value::Hole(_) => footprint.has_hole = true,
        | Value::Pair(ref fst, ref snd) => {
            stack.push(Work::Val(as_val(fst), Rc::clone(scope)));
            stack.push(Work::Val(as_val(snd), Rc::clone(scope)));
        },
        | Value::Inj(_, ref payload) => {
            stack.push(Work::Val(as_val(payload), Rc::clone(scope)));
        },
        | Value::List(ref elements) => {
            for element in elements {
                stack.push(Work::Val(as_val(element), Rc::clone(scope)));
            }
        },
        | Value::Record(ref fields) => {
            for field in fields.values() {
                stack.push(Work::Val(as_val(field), Rc::clone(scope)));
            }
        },
        | Value::Thunk(_, ref body) => {
            stack.push(Work::Cmp(as_cmp(body), Rc::clone(scope)));
        },
        | Value::Annot(ref inner, _) => {
            stack.push(Work::Val(as_val(inner), Rc::clone(scope)));
        },
        // A reified stack ([`Value::Stk`]), an identity proof
        // ([`Value::Here`]), or a data constructor ([`Value::Ctor`]): read-set
        // unrepresentable, so conservatively opaque (never adopted).
        | _ => footprint.opaque = true,
    }
}

/// Borrows a value node behind an `Rc` child (deref coercion at return),
/// avoiding a same-name rebinding at each push site.
fn as_val(value: &Rc<Value>) -> &Value
{
    value
}

/// Captures the dependency footprint of a lowered item's `term`: the free
/// variables it reads from the ambient context, plus the conservative
/// [`Footprint::opaque`] / [`Footprint::has_hole`] flags.
///
/// The scan is a single heap-work-list pass (iterative-traversal design) that
/// threads a persistent [`Scope`] so a variable occurrence counts as *free*
/// only when no enclosing binder captures it — the exact cross-item read-set.
///
/// # Contract
/// - ensures: [`Footprint::names`] is precisely the term's free variables;
///   [`Footprint::opaque`] is set iff a non-representable node was met;
///   [`Footprint::has_hole`] is set iff a hole was met.
/// - provides: a sound over-approximation of the item's dependency edges — the
///   §4 footprint at item granularity.
/// - panics: none — the work-list is heap-allocated, so binder nesting scaled
///   to the input does not consume the host call stack.
///
/// # Adequacy
/// - hypothesis: the scan must (i) count a shadowed occurrence as bound (not a
///   context read), (ii) count a genuinely free occurrence as a read, and (iii)
///   descend every var-bearing constructor of both sorts. A term binding a name
///   that also appears free elsewhere, and one free variable under a chain of
///   binders, distinguish these.
/// - witness: `crate::footprint::tests::shadowed_binder_is_not_a_read` and
///   `crate::footprint::tests::free_occurrence_under_binders_is_read`, plus the
///   integration gate (`tests::incremental`) whose from-scratch/resume
///   equivalence would break under any missed edge.
#[inline]
#[must_use]
pub fn footprint_of(term: &Term) -> Footprint
{
    let mut footprint = Footprint::default();
    let root: Rc<Scope<'_>> = Rc::new(Scope::Root);
    let mut stack: Vec<Work<'_>> = Vec::new();
    match *term {
        | Term::Value(ref value) => stack.push(Work::Val(value, root)),
        | Term::Comp(ref comp) => stack.push(Work::Cmp(comp, root)),
    }
    while let Some(work) = stack.pop() {
        match work {
            | Work::Val(value, scope) => scan_value(value, &scope, &mut stack, &mut footprint),
            | Work::Cmp(comp, scope) => scan_comp(comp, &scope, &mut stack, &mut footprint),
        }
    }
    footprint
}

#[cfg(test)]
mod tests
{
    use alloc::rc::Rc;

    use gandr_core_checker::syntax::Comp;
    use gandr_core_checker::syntax::Term;
    use gandr_core_checker::syntax::Value;

    use super::footprint_of;
    use crate::boundary::DefinitionName;
    /// A binder's occurrence in its own body is *bound*, not a context read, so
    /// it never enters the footprint.
    #[test]
    fn shadowed_binder_is_not_a_read()
    {
        let footprint = footprint_of(&lambda_over("x"));
        assert!(
            footprint.names.is_empty(),
            "the bound `x` is not a read: {:?}",
            footprint.names
        );
        assert!(!footprint.opaque, "an ordinary lambda is representable");
    }
    /// A genuinely free occurrence under a chain of unrelated binders is a
    /// context read.
    #[test]
    fn free_occurrence_under_binders_is_read()
    {
        let footprint = footprint_of(&lambda_over("free"));
        assert!(
            footprint.names.contains("free"),
            "the free `free` is read: {:?}",
            footprint.names
        );
        assert!(
            !footprint.names.contains("x"),
            "the bound `x` is still not read"
        );
    }

    /// `λx. ret x` under `Comp::Abs` — the binder captures `x`.
    fn lambda_over<'name>(body_var: impl Into<DefinitionName<'name>>) -> Term
    {
        let body_var = body_var.into();
        Term::Comp(Comp::Abs(
            "x".to_owned(),
            None,
            Rc::new(Comp::ret(Value::var(body_var.0))),
        ))
    }

    /// A hole sets the parse-incomplete flag.
    #[test]
    fn hole_sets_has_hole()
    {
        let footprint = footprint_of(&Term::Value(Value::hole(0)));
        assert!(footprint.has_hole, "a hole is recorded");
    }
}
