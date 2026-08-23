//! Dependency-footprint capture for the incremental typing pipeline
//! (`incremental-pipeline.md` §"Checkpoints and the reuse rule").
//!
//! # What a footprint is
//!
//! `incremental-pipeline.md` §"Checkpoints and the reuse rule" tags every
//! checkpoint with a **dependency footprint** — "the dynamic dependency edges
//! an analysis consulted" (Reps, Teitelbaum & Demers 1983, the
//! §"pipeline-decision-08" acknowledged ancestor) — and reuses the checkpoint
//! iff those edges are untouched. The spec states the footprint in the
//! *solver's* vocabulary (`{tyvars, gradeVars, trailDepth, stepId}`), because
//! its checkpoints live inside the stepping typing machine.
//!
//! This module captures that footprint at the item granularity this crate
//! works in. Top-level items ([`crate::region::Item`]) lower independently and
//! are typed against an accumulating context ([`Ctx`]) that
//! [`crate::checkpoint`] threads item to item — a processed `def name = A`
//! binds `name : A` for later items to read. The one piece of shared,
//! edit-mutable state threaded across items is therefore that name → type
//! context, and an item's footprint is exactly **the set of context names its
//! lowered term read** (its free variables). This is the §"Checkpoints and the
//! reuse rule" footprint specialized to the item granularity: condition 2 ("no
//! `tyvars` in the footprint were re-assigned") becomes "no *name* the item
//! read had its binding change", the same soundness shape one stratum up.
//!
//! [`Ctx`]: gandr_core_term::ctx::Ctx
//!
//! # Soundness direction
//!
//! For invalidation to be sound the footprint must **over**-approximate the
//! true read-set — never miss a dependency (a missed edge lets a stale
//! checkpoint survive an input change that should have killed it).
//! [`footprint_of`] therefore captures every free variable exactly, and marks
//! the footprint [`Footprint::opaque`] on any core node it cannot represent (a
//! reified stack, an identity form, a declared-data or native form, or a future
//! non-exhaustive variant): an opaque footprint is treated as reading
//! *everything*, forcing the item to be re-typed rather than adopted.
//! Over-approximation only costs reuse; it never loses soundness.

use alloc::collections::BTreeSet;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;

use gandr_core_term::syntax::Comp;
use gandr_core_term::syntax::Term;
use gandr_core_term::syntax::Value;
use gandr_core_term::types::CompType;
use gandr_core_term::types::Ty;
use gandr_core_term::types::ValueType;

use crate::boundary::DefinitionName;
use crate::boundary::MatchDecision;
use crate::region::Item;

/// The dependency footprint of one lowered item: the context names its term
/// read, plus the two conservative flags that force re-typing.
///
/// The [`Self::names`] set is the item's free variables — every name it read
/// from the ambient context (the base context, plus the prior definitions
/// threaded in ahead of it). The intersection of this set with the names whose
/// binding an edit changed is
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
    /// reified [`Value::Stk`], an identity form, a declared-data or native
    /// form, or a future non-exhaustive variant). An opaque footprint reads
    /// *everything* conservatively, so its item is never adopted — the safe
    /// over-approximation.
    pub opaque: bool,
    /// Set when the term carries a hole ([`Value::Hole`] / [`Comp::Hole`]): the
    /// item is parse-incomplete, so typing is declined rather than attempted
    /// (`incremental-pipeline.md` §"Holes").
    pub has_hole: bool,
}

impl Footprint
{
    /// Whether this footprint reads any name in `changed` — the reuse-blocking
    /// test. An [`Self::opaque`] footprint conservatively reads everything, so
    /// it intersects any change set (and, treated as reading everything, blocks
    /// reuse even against an empty one).
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
            return MatchDecision::from(true);
        }
        MatchDecision::from(self.names.iter().any(|name| changed.contains(name)))
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
        let name: DefinitionName<'name> = name.into();
        let needle: &str = name.into();
        let mut cursor = self;
        loop {
            match *cursor {
                | Self::Root => return MatchDecision::from(false),
                | Self::Bind {
                    name: bound,
                    ref parent,
                } => {
                    if bound == needle {
                        return MatchDecision::from(true);
                    }
                    cursor = parent;
                },
            }
        }
    }
}

/// Whether a scanned node sits inside a **type**.
///
/// The distinction is what separates the two read sets an item has. A name read
/// in a value position is consulted for its **type**; a name read inside a type
/// may additionally be consulted for its **value**, because definitional
/// equality unfolds definitions when it compares types, and a type can carry
/// values ([`ValueType::Path`]'s endpoints).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Position
{
    /// An ordinary term position.
    Value,
    /// Inside a type, where definitional equality can consult a body.
    Type,
}

/// Which reads a scan collects.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Collect
{
    /// Every free name the item reads, wherever it occurs.
    EveryRead,
    /// Only the free names occurring inside a type — the item's *type support*.
    TypePositionsOnly,
}

/// One pending node in the iterative scan, paired with the lexical scope in
/// force at its position (ADR-47: the traversal is a heap work-list, never host
/// recursion on the user-sized term).
enum Work<'term>
{
    /// A value node to scan under `scope`, at `position`.
    Val(&'term Value, Rc<Scope<'term>>, Position),
    /// A computation node to scan under `scope`, at `position`.
    Cmp(&'term Comp, Rc<Scope<'term>>, Position),
    /// A value type to scan under `scope`; everything inside a type is at
    /// [`Position::Type`], so the position is implied.
    ValTy(&'term ValueType, Rc<Scope<'term>>),
    /// A computation type to scan under `scope`.
    CompTy(&'term CompType, Rc<Scope<'term>>),
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
    position: Position,
    stack: &mut Vec<Work<'term>>,
    footprint: &mut Footprint,
)
{
    match *comp {
        | Comp::Abs(ref name, ref annotation, ref body) => {
            stack.push(Work::Cmp(as_cmp(body), extend(scope, name), position));
            // The binder's annotation is a type in the *enclosing* scope, and
            // checking against it compares types — so its free names are type
            // reads.
            if let Some(ref annotation) = *annotation {
                stack.push(Work::ValTy(as_value_type(annotation), Rc::clone(scope)));
            }
        },
        | Comp::App(ref head, ref arg) => {
            stack.push(Work::Val(as_val(arg), Rc::clone(scope), position));
            stack.push(Work::Cmp(as_cmp(head), Rc::clone(scope), position));
        },
        | Comp::Ret(ref value)
        | Comp::Force(ref value)
        | Comp::Dup(ref value)
        | Comp::Drop(ref value) => {
            stack.push(Work::Val(as_val(value), Rc::clone(scope), position));
        },
        | Comp::Bind(ref bound, ref name, ref cont) => {
            stack.push(Work::Cmp(as_cmp(bound), Rc::clone(scope), position));
            stack.push(Work::Cmp(as_cmp(cont), extend(scope, name), position));
        },
        | Comp::Case(
            ref scrut,
            (ref left_name, ref left_body),
            (ref right_name, ref right_body),
        ) => {
            stack.push(Work::Val(as_val(scrut), Rc::clone(scope), position));
            stack.push(Work::Cmp(
                as_cmp(left_body),
                extend(scope, left_name),
                position,
            ));
            stack.push(Work::Cmp(
                as_cmp(right_body),
                extend(scope, right_name),
                position,
            ));
        },
        // The motive is a computation *type* (ADR-82). Its shape is not walked
        // here: a motive present is conservatively opaque rather than scanned,
        // which costs reuse for an explicitly-motivated split and keeps the
        // read set complete.
        | Comp::Split {
            ref scrut,
            ref fst_name,
            ref snd_name,
            ref motive,
            ref body,
        } => {
            stack.push(Work::Val(as_val(scrut), Rc::clone(scope), position));
            stack.push(Work::Cmp(
                as_cmp(body),
                extend(&extend(scope, fst_name), snd_name),
                position,
            ));
            if motive.is_some() {
                footprint.opaque = true;
            }
        },
        | Comp::ListCase {
            ref scrut,
            ref nil,
            ref head,
            ref tail,
            ref cons,
        } => {
            stack.push(Work::Val(as_val(scrut), Rc::clone(scope), position));
            stack.push(Work::Cmp(as_cmp(nil), Rc::clone(scope), position));
            stack.push(Work::Cmp(
                as_cmp(cons),
                extend(&extend(scope, head), tail),
                position,
            ));
        },
        | Comp::With(ref fst, ref snd) => {
            stack.push(Work::Cmp(as_cmp(fst), Rc::clone(scope), position));
            stack.push(Work::Cmp(as_cmp(snd), Rc::clone(scope), position));
        },
        | Comp::Prj(_, ref target) => {
            stack.push(Work::Cmp(as_cmp(target), Rc::clone(scope), position));
        },
        | Comp::RecordProj { ref record, .. } => {
            stack.push(Work::Val(as_val(record), Rc::clone(scope), position));
        },
        | Comp::Perform(_, _, ref arg) => {
            stack.push(Work::Val(as_val(arg), Rc::clone(scope), position));
        },
        // The inline effect signature carries operation types this scan does
        // not walk, so a handler is conservatively opaque.
        | Comp::Handle {
            ref scrutinee,
            ref ret,
            ref ops,
            ..
        } => {
            footprint.opaque = true;
            stack.push(Work::Cmp(as_cmp(scrutinee), Rc::clone(scope), position));
            stack.push(Work::Cmp(as_cmp(&ret.1), extend(scope, &ret.0), position));
            for clause in ops {
                stack.push(Work::Cmp(
                    as_cmp(&clause.body),
                    extend(&extend(scope, &clause.payload), &clause.resume),
                    position,
                ));
            }
        },
        | Comp::Resume(ref stack_value, ref body) => {
            stack.push(Work::Val(as_val(stack_value), Rc::clone(scope), position));
            stack.push(Work::Cmp(as_cmp(body), Rc::clone(scope), position));
        },
        | Comp::Reset(ref body) => {
            stack.push(Work::Cmp(as_cmp(body), Rc::clone(scope), position));
        },
        | Comp::Shift(ref binder, ref body) | Comp::Fix(ref binder, ref body) => {
            stack.push(Work::Cmp(as_cmp(body), extend(scope, binder), position));
        },
        | Comp::Hole(_) => footprint.has_hole = true,
        // A declared-data eliminator, a native form, the identity eliminator,
        // or a future non-exhaustive `Comp` variant: conservatively opaque.
        | _ => footprint.opaque = true,
    }
}

/// Borrows a value type behind an [`Rc`] child.
fn as_value_type(ty: &Rc<ValueType>) -> &ValueType
{
    ty
}

/// Borrows a computation type behind an [`Rc`] child.
fn as_comp_type(ty: &Rc<CompType>) -> &CompType
{
    ty
}

/// Scans one value type: pushes its component types, and its embedded
/// **values** at [`Position::Type`], since those are the occurrences
/// definitional equality can consult a body for.
fn scan_value_type<'term>(
    ty: &'term ValueType,
    scope: &Rc<Scope<'term>>,
    stack: &mut Vec<Work<'term>>,
    footprint: &mut Footprint,
)
{
    match *ty {
        // An atom is a rigid base type or type variable, compared by name; it
        // names no definition. A sealed atom is nominal, `Unknown` is the
        // absent classifier, and a universe's sort and level live in the level
        // algebra rather than the value namespace.
        | ValueType::Atom(_)
        | ValueType::Unit
        | ValueType::Sealed(_)
        | ValueType::Unknown
        | ValueType::Universe { .. } => {},
        | ValueType::Prod(ref fst, ref snd) | ValueType::Sum(ref fst, ref snd) => {
            stack.push(Work::ValTy(as_value_type(fst), Rc::clone(scope)));
            stack.push(Work::ValTy(as_value_type(snd), Rc::clone(scope)));
        },
        | ValueType::List(ref element) => {
            stack.push(Work::ValTy(as_value_type(element), Rc::clone(scope)));
        },
        | ValueType::Record(ref fields) => {
            for field in fields.values() {
                stack.push(Work::ValTy(as_value_type(field), Rc::clone(scope)));
            }
        },
        | ValueType::Thunk(_, ref comp) => {
            stack.push(Work::CompTy(as_comp_type(comp), Rc::clone(scope)));
        },
        | ValueType::Stk(ref consumed, ref delivered) => {
            stack.push(Work::CompTy(as_comp_type(consumed), Rc::clone(scope)));
            stack.push(Work::CompTy(as_comp_type(delivered), Rc::clone(scope)));
        },
        // The endpoints are values *inside* a type, compared by definitional
        // equality — the occurrence class this whole distinction exists for.
        | ValueType::Path {
            ty: ref carrier,
            ref lhs,
            ref rhs,
        } => {
            stack.push(Work::ValTy(as_value_type(carrier), Rc::clone(scope)));
            stack.push(Work::Val(as_val(lhs), Rc::clone(scope), Position::Type));
            stack.push(Work::Val(as_val(rhs), Rc::clone(scope), Position::Type));
        },
        | ValueType::Data { ref args, .. } => {
            for arg in args {
                stack.push(Work::ValTy(as_value_type(arg), Rc::clone(scope)));
            }
        },
        | ValueType::Sigma {
            ref fst,
            ref binder,
            ref snd,
        } => {
            stack.push(Work::ValTy(as_value_type(fst), Rc::clone(scope)));
            stack.push(Work::ValTy(as_value_type(snd), extend(scope, binder)));
        },
        | ValueType::Package {
            ref abstracts,
            ref payload,
            ..
        } => {
            let mut inner = Rc::clone(scope);
            for abstracted in abstracts {
                inner = extend(&inner, abstracted);
            }
            stack.push(Work::ValTy(as_value_type(payload), inner));
        },
        // A family application's neutral head, or a future non-exhaustive
        // variant: conservatively opaque.
        | _ => footprint.opaque = true,
    }
}

/// Scans one computation type.
fn scan_comp_type<'term>(
    ty: &'term CompType,
    scope: &Rc<Scope<'term>>,
    stack: &mut Vec<Work<'term>>,
    footprint: &mut Footprint,
)
{
    match *ty {
        | CompType::F(ref payload, _) => {
            stack.push(Work::ValTy(as_value_type(payload), Rc::clone(scope)));
        },
        | CompType::Arrow {
            ref binder,
            ref arg,
            ref res,
        } => {
            stack.push(Work::ValTy(as_value_type(arg), Rc::clone(scope)));
            let inner = match *binder {
                | Some(ref binder) => extend(scope, binder),
                | None => Rc::clone(scope),
            };
            stack.push(Work::CompTy(as_comp_type(res), inner));
        },
        | CompType::With(ref fst, ref snd) => {
            stack.push(Work::CompTy(as_comp_type(fst), Rc::clone(scope)));
            stack.push(Work::CompTy(as_comp_type(snd), Rc::clone(scope)));
        },
        | CompType::Unknown => {},
        // A family application, or a future non-exhaustive variant.
        | _ => footprint.opaque = true,
    }
}

/// Extends `scope` with one more bound `name`.
fn extend<'term>(
    scope: &Rc<Scope<'term>>,
    name: impl Into<DefinitionName<'term>>,
) -> Rc<Scope<'term>>
{
    let name: DefinitionName<'term> = name.into();
    let bound: &'term str = name.into();
    Rc::new(Scope::Bind {
        name: bound,
        parent: Rc::clone(scope),
    })
}

/// Scans one value node: records a free occurrence, pushes children under the
/// same scope, and sets the conservative flags for holes and opaque nodes.
fn scan_value<'term>(
    value: &'term Value,
    scope: &Rc<Scope<'term>>,
    position: Position,
    collect: Collect,
    stack: &mut Vec<Work<'term>>,
    footprint: &mut Footprint,
)
{
    match *value {
        | Value::Var(ref name) => {
            if !bool::from(scope.binds(name)) {
                record(
                    footprint,
                    DefinitionName::from(name.as_str()),
                    position,
                    collect,
                );
            }
        },
        // Leaves that read no name.
        | Value::Unit | Value::Int(_) | Value::Str(_) | Value::Num(_) => {},
        | Value::Hole(_) => footprint.has_hole = true,
        | Value::Pair(ref fst, ref snd) => {
            stack.push(Work::Val(as_val(fst), Rc::clone(scope), position));
            stack.push(Work::Val(as_val(snd), Rc::clone(scope), position));
        },
        | Value::Inj(_, ref payload) => {
            stack.push(Work::Val(as_val(payload), Rc::clone(scope), position));
        },
        | Value::List(ref elements) => {
            for element in elements {
                stack.push(Work::Val(as_val(element), Rc::clone(scope), position));
            }
        },
        | Value::Record(ref fields) => {
            for field in fields.values() {
                stack.push(Work::Val(as_val(field), Rc::clone(scope), position));
            }
        },
        | Value::Thunk(_, ref body) | Value::Run(ref body) => {
            stack.push(Work::Cmp(as_cmp(body), Rc::clone(scope), position));
        },
        // The ascribed type is scanned too: checking against it compares types,
        // so the names it mentions are read, and read in a **type** position.
        | Value::Annot(ref inner, ref ty) => {
            stack.push(Work::Val(as_val(inner), Rc::clone(scope), position));
            stack.push(Work::ValTy(as_value_type(ty), Rc::clone(scope)));
        },
        // A reified stack ([`Value::Stk`]), an identity form ([`Value::Here`]),
        // a declared-data constructor ([`Value::Ctor`]), or a future
        // non-exhaustive variant: read-set unrepresentable, so conservatively
        // opaque (never adopted).
        | _ => footprint.opaque = true,
    }
}

/// Borrows a value node behind an `Rc` child (deref coercion at return),
/// avoiding a same-name rebinding at each push site.
fn as_val(value: &Rc<Value>) -> &Value
{
    value
}

/// Records one free occurrence, subject to what this scan collects.
fn record<'name>(
    footprint: &mut Footprint,
    name: impl Into<DefinitionName<'name>>,
    position: Position,
    collect: Collect,
)
{
    let name: DefinitionName<'name> = name.into();
    let name: &str = name.into();
    let keep = match collect {
        | Collect::EveryRead => true,
        | Collect::TypePositionsOnly => matches!(position, Position::Type),
    };
    if keep {
        let _inserted = footprint.names.insert(name.to_owned());
    }
}

/// Drives the work-list over one item's term and its ascription.
fn scan_item(
    term: &Term,
    ascription: Option<&Ty>,
    collect: Collect,
) -> Footprint
{
    let mut footprint = Footprint::default();
    let root: Rc<Scope<'_>> = Rc::new(Scope::Root);
    let mut stack: Vec<Work<'_>> = Vec::new();
    match *term {
        | Term::Value(ref value) => stack.push(Work::Val(value, Rc::clone(&root), Position::Value)),
        | Term::Comp(ref comp) => stack.push(Work::Cmp(comp, Rc::clone(&root), Position::Value)),
    }
    if let Some(ascription) = ascription {
        match *ascription {
            | Ty::Value(ref value_type) => {
                stack.push(Work::ValTy(value_type, Rc::clone(&root)));
            },
            | Ty::Comp(ref comp_type) => {
                stack.push(Work::CompTy(comp_type, Rc::clone(&root)));
            },
        }
    }
    while let Some(work) = stack.pop() {
        match work {
            | Work::Val(value, scope, position) => {
                scan_value(value, &scope, position, collect, &mut stack, &mut footprint);
            },
            | Work::Cmp(comp, scope, position) => {
                scan_comp(comp, &scope, position, &mut stack, &mut footprint);
            },
            | Work::ValTy(ty, scope) => scan_value_type(ty, &scope, &mut stack, &mut footprint),
            | Work::CompTy(ty, scope) => scan_comp_type(ty, &scope, &mut stack, &mut footprint),
        }
    }
    footprint
}

/// Captures the dependency footprint of a lowered item.
///
/// The footprint is the free variables the item reads from the ambient context
/// — in its term **and in its ascription** — plus the conservative
/// [`Footprint::opaque`] / [`Footprint::has_hole`] flags.
///
/// The scan is a single heap-work-list pass (ADR-47) that threads a persistent
/// [`Scope`] so a variable occurrence counts as *free* only when no enclosing
/// binder captures it — the exact cross-item read-set.
///
/// **The ascription and every type embedded in the term are scanned**, because
/// checking an item against a type compares types, and a comparison reads every
/// name the compared types mention. A scan of the term alone misses those reads
/// entirely, which is one half of the over-adoption defect this crate carries.
///
/// # Contract
/// - ensures: [`Footprint::names`] is precisely the free variables of the
///   item's term and ascription together; [`Footprint::opaque`] is set iff a
///   non-representable node was met; [`Footprint::has_hole`] is set iff a hole
///   was met.
/// - provides: a sound over-approximation of the item's dependency edges — the
///   §"Checkpoints and the reuse rule" footprint at item granularity.
/// - panics: none — the work-list is heap-allocated, so binder nesting scaled
///   to the input does not consume the host call stack.
///
/// # Adequacy
/// - hypothesis: the scan must (i) count a shadowed occurrence as bound (not a
///   context read), (ii) count a genuinely free occurrence as a read, (iii)
///   descend every var-bearing constructor of both sorts, and (iv) descend into
///   types, where a name occurrence is a read the term alone does not show. A
///   term binding a name that also appears free elsewhere, one free variable
///   under a chain of binders, and an ascription mentioning a name the term
///   does not, distinguish these.
/// - witness: `crate::footprint::tests::shadowed_binder_is_not_a_read`,
///   `crate::footprint::tests::free_occurrence_under_binders_is_read`, and
///   `crate::footprint::tests::ascription_names_are_reads`, plus the
///   integration gate (`tests/incremental`) whose from-scratch/resume
///   equivalence would break under any missed edge.
#[inline]
#[must_use]
pub fn footprint_of(item: &Item) -> Footprint
{
    scan_item(&item.term, item.ascription.as_ref(), Collect::EveryRead)
}

/// The item's **type support**: the free names it reads from inside a type,
/// where definitional equality can consult a definition's *body* rather than
/// only its type.
///
/// This is the read set that must be tested against value-only edits. A name
/// read solely in a value position is consulted for its type alone, so an edit
/// that changes a definition's body while preserving its type leaves such a
/// reader adoptable — which is the reuse this crate exists to buy. A name read
/// inside a type is different: comparing two types normalizes them, and
/// normalization unfolds definitions.
///
/// # Contract
/// - ensures: the returned [`Footprint::names`] are exactly the free names
///   occurring inside a type reachable from the item, and are a subset of
///   [`footprint_of`]'s; [`Footprint::opaque`] is set iff a type this scan
///   cannot represent was met.
/// - provides: the support the value-changed test consults.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: an ascription's names must appear here and a value-position
///   read must not, or the test either misses the defect class or forfeits
///   type-stable dependent reuse.
/// - witness: `crate::footprint::tests::type_support_holds_only_type_positions`.
#[inline]
#[must_use]
pub fn type_support_of(item: &Item) -> Footprint
{
    scan_item(
        &item.term,
        item.ascription.as_ref(),
        Collect::TypePositionsOnly,
    )
}

/// The free names of one bound value type — the support carried by a name a
/// reader reads.
///
/// A reader's typing manipulates the types of the names it reads, so those
/// types' free names are consulted even when the reader's own term and
/// ascription mention none of them.
///
/// # Contract
/// - ensures: the returned [`Footprint::names`] are the type's free value
///   names; [`Footprint::opaque`] is set iff an unrepresentable type was met.
/// - provides: the one-step propagation the adoption test needs through the
///   context.
/// - panics: none.
#[inline]
#[must_use]
pub fn type_support_of_value_type(ty: &ValueType) -> Footprint
{
    let mut footprint = Footprint::default();
    let root: Rc<Scope<'_>> = Rc::new(Scope::Root);
    let mut stack: Vec<Work<'_>> = alloc::vec![Work::ValTy(ty, root)];
    while let Some(work) = stack.pop() {
        match work {
            | Work::Val(value, scope, position) => {
                scan_value(
                    value,
                    &scope,
                    position,
                    Collect::EveryRead,
                    &mut stack,
                    &mut footprint,
                );
            },
            | Work::Cmp(comp, scope, position) => {
                scan_comp(comp, &scope, position, &mut stack, &mut footprint);
            },
            | Work::ValTy(ty, scope) => scan_value_type(ty, &scope, &mut stack, &mut footprint),
            | Work::CompTy(ty, scope) => scan_comp_type(ty, &scope, &mut stack, &mut footprint),
        }
    }
    footprint
}

#[cfg(test)]
mod tests
{
    use alloc::rc::Rc;

    use gandr_core_term::syntax::Comp;
    use gandr_core_term::syntax::Term;
    use gandr_core_term::syntax::Value;
    use gandr_core_term::types::Ty;
    use gandr_core_term::types::ValueType;

    use super::footprint_of;
    use super::type_support_of;
    use crate::boundary::DefinitionName;
    use crate::region::Item;

    /// An unascribed item carrying `term`.
    fn item_of(term: Term) -> Item
    {
        Item::new(None, None, term)
    }

    /// A binder's occurrence in its own body is *bound*, not a context read, so
    /// it never enters the footprint.
    #[test]
    fn shadowed_binder_is_not_a_read()
    {
        let footprint = footprint_of(&item_of(lambda_over("x")));
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
        let footprint = footprint_of(&item_of(lambda_over("free")));
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
        let body_var: DefinitionName<'name> = body_var.into();
        let body_var: &str = body_var.into();
        Term::Comp(Comp::Abs(
            "x".to_owned(),
            None,
            Rc::new(Comp::ret(Value::var(body_var))),
        ))
    }

    /// A hole sets the parse-incomplete flag.
    #[test]
    fn hole_sets_has_hole()
    {
        let footprint = footprint_of(&item_of(Term::Value(Value::hole(0_u32))));
        assert!(footprint.has_hole, "a hole is recorded");
    }

    /// An item whose ascription mentions a name reads that name, even when its
    /// term does not: checking against the ascription compares types, and the
    /// comparison consults everything the types mention.
    #[test]
    fn ascription_names_are_reads()
    {
        let item = path_ascribed_item();
        let footprint = footprint_of(&item);
        assert!(
            footprint.names.contains("endpoint"),
            "the ascription's endpoint is read: {:?}",
            footprint.names
        );
        assert!(
            footprint.names.contains("witness"),
            "the term's own read survives: {:?}",
            footprint.names
        );
    }

    /// The type support holds the ascription's names and **not** a name read
    /// only in a value position — the separation that keeps a value-only edit
    /// from invalidating an ordinary type-stable dependent.
    #[test]
    fn type_support_holds_only_type_positions()
    {
        let item = path_ascribed_item();
        let support = type_support_of(&item);
        assert!(
            support.names.contains("endpoint"),
            "the ascription's endpoint is type support: {:?}",
            support.names
        );
        assert!(
            !support.names.contains("witness"),
            "a value-position read is not type support: {:?}",
            support.names
        );
    }

    /// `def _ : Path Integer endpoint 1 = witness`.
    fn path_ascribed_item() -> Item
    {
        Item::new(
            None,
            Some(Ty::Value(ValueType::path(
                ValueType::integer(),
                Value::var("endpoint"),
                Value::int(1_i64),
            ))),
            Term::Value(Value::var("witness")),
        )
    }
}
