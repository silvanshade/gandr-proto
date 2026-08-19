//! Read-only occurrence scanning over a readback term.
//!
//! Solving a flex constraint asks three questions of the term a metavariable is
//! about to be bound to, and all three are occurrence questions: does the term
//! mention a variable the metavariable is not allowed to depend on (escape),
//! does it mention the metavariable itself (occurs), and which other
//! metavariables does it mention (the blockers a postponement reports). One
//! pass answers all three.
//!
//! # Why this walk needs no shadowing discipline
//!
//! The scan does not distinguish a bound occurrence from a free one, and that
//! is sound for both of its questions because of **who names the binders**.
//!
//! The terms it walks come from [`QuoteMode::Canonical`] readback, which
//! renames every binder to the de Bruijn level it opened, drawn from the
//! normalizer's own monotone counter. So every binder in a scanned term carries
//! a level **at or above** the counter's value when the quote began, while
//! every variable the solver opened by eta expansion carries a level **below**
//! it. The scan takes that value as its `ceiling` and asks about levels below
//! it only, so no name it can flag is a name the term can bind.
//!
//! A source binder that survives inside a reified stack keeps its source name,
//! which no level name can equal: the bracket characters are outside the
//! identifier grammar the surface parses.
//!
//! Hole identity has no binding form at all, so the occurs and blocker
//! questions never had a shadowing case.
//!
//! [`QuoteMode::Canonical`]: gandr_core_nbe::quote::QuoteMode::Canonical

use alloc::collections::BTreeSet;
use alloc::vec::Vec;

use gandr_core_nbe::quote::parse_level_name;
use gandr_core_term::boundary::EscapeStatus;
use gandr_core_term::boundary::HoleId;
use gandr_core_term::boundary::NameRef;
use gandr_core_term::boundary::OpaqueOccurrence;
use gandr_core_term::boundary::VariableLevel;
use gandr_core_term::syntax::Comp;
use gandr_core_term::syntax::Stack;
use gandr_core_term::syntax::Value;

/// What one scan found.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Occurrences
{
    /// Every hole the term mentions, at either sort, in canonical order.
    holes: BTreeSet<HoleId>,
    /// Whether the term mentions a solver-opened level the query disallowed.
    escaped: EscapeStatus,
    /// Whether the walk reached a reified stack.
    opaque: OpaqueOccurrence,
}

impl Occurrences
{
    /// Every hole the term mentions, at either sort.
    #[inline]
    #[must_use]
    pub(super) fn holes(&self) -> &BTreeSet<HoleId>
    {
        &self.holes
    }

    /// Whether the term mentions a solver-opened variable the query did not
    /// allow.
    #[inline]
    #[must_use]
    pub(super) fn escapes(&self) -> EscapeStatus
    {
        self.escaped
    }

    /// Whether the walk reached a reified stack.
    ///
    /// A reified stack carries source syntax verbatim, binders included, so it
    /// is the one place where a name the scan flags could in principle be bound
    /// rather than free. An escape found in a term carrying one is reported as
    /// a postponement rather than as a refutation: a positive result is safe to
    /// overshoot, and a refutation is not.
    #[inline]
    #[must_use]
    pub(super) fn opaque(&self) -> OpaqueOccurrence
    {
        self.opaque
    }

    /// Whether the term mentions `hole`.
    #[inline]
    #[must_use]
    pub(super) fn mentions(
        &self,
        hole: HoleId,
    ) -> gandr_core_term::boundary::HoleOccurrence
    {
        gandr_core_term::boundary::HoleOccurrence::from(self.holes.contains(&hole))
    }
}

/// Scans a **value** for its holes and for a disallowed solver-opened variable.
///
/// # Contract
/// - requires: `term` is [`QuoteMode::Canonical`] readback output, so every
///   binder in it names a level at or above `ceiling` (the module doc states
///   why that is what makes a shadowing-free walk sound).
/// - ensures: the result names every hole reached at either sort, and reports
///   an escape exactly when the term mentions a variable whose name parses to a
///   level strictly below `ceiling` that is not in `allowed`.
/// - provides: the escape, occurs, and blocker evidence one flex solve needs,
///   from one traversal that allocates nothing per node.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 — four decision surfaces separated pointwise: an allowed
///   level does not escape, a disallowed one below the ceiling does, a level at
///   the ceiling does not (the readback-binder case), and an ordinary source
///   name does not.
/// - witness: `unify::tests::a_scan_allows_the_spine_levels_and_flags_the_others`
/// - witness: `unify::tests::a_scan_ignores_readback_binders_and_source_names`
///
/// [`QuoteMode::Canonical`]: gandr_core_nbe::quote::QuoteMode::Canonical
#[must_use]
pub fn scan_value(
    term: &Value,
    ceiling: VariableLevel,
    allowed: &[VariableLevel],
) -> Occurrences
{
    let mut scan = Scan::new(ceiling, allowed);
    scan.work.push(Node::Value(term));
    scan.run();
    scan.found
}

/// Scans a **computation** for its holes and for a disallowed solver-opened
/// variable (the computation-sorted companion of [`scan_value`], with the
/// identical contract).
///
/// # Contract
/// - requires: `term` is canonical readback output.
/// - ensures: as [`scan_value`], over a computation.
/// - provides: the same evidence where a higher-order solution lands.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 — the computation-sorted image of [`scan_value`], separated
///   by a computation hole under an application spine whose argument carries a
///   disallowed level.
/// - witness: `unify::tests::a_scan_reaches_through_an_application_spine`
#[must_use]
pub fn scan_comp(
    term: &Comp,
    ceiling: VariableLevel,
    allowed: &[VariableLevel],
) -> Occurrences
{
    let mut scan = Scan::new(ceiling, allowed);
    scan.work.push(Node::Comp(term));
    scan.run();
    scan.found
}

/// One pending node on the scan's work stack.
enum Node<'term>
{
    /// A value to visit.
    Value(&'term Value),
    /// A computation to visit.
    Comp(&'term Comp),
    /// A reified stack to visit.
    Stack(&'term Stack),
}

/// The iterative scanner: an explicit work stack over borrowed nodes, so depth
/// follows the heap and nothing is rebuilt.
struct Scan<'term, 'query>
{
    /// The first level a readback binder may carry; every level below it was
    /// opened by the solver.
    ceiling: VariableLevel,
    /// The solver-opened levels the term is allowed to mention.
    allowed: &'query [VariableLevel],
    /// Pending nodes, processed last-in-first-out.
    work: Vec<Node<'term>>,
    /// What the walk has found so far.
    found: Occurrences,
}

impl<'term, 'query> Scan<'term, 'query>
{
    /// Builds an empty scanner for one query.
    fn new(
        ceiling: VariableLevel,
        allowed: &'query [VariableLevel],
    ) -> Self
    {
        Self {
            ceiling,
            allowed,
            work: Vec::new(),
            found: Occurrences::default(),
        }
    }

    /// Drains the work stack.
    fn run(&mut self)
    {
        while let Some(node) = self.work.pop() {
            match node {
                | Node::Value(value) => self.visit_value(value),
                | Node::Comp(comp) => self.visit_comp(comp),
                | Node::Stack(stack) => self.visit_stack(stack),
            }
        }
    }

    /// Records a variable occurrence, flagging a disallowed solver-opened one.
    fn visit_var(
        &mut self,
        name: NameRef<'_>,
    )
    {
        let Some(level) = parse_level_name(name)
        else {
            return;
        };
        if u32::from(level) >= u32::from(self.ceiling) || self.allowed.contains(&level) {
            return;
        }
        self.found.escaped = EscapeStatus::from(true);
    }

    /// Visits one value, pushing its children.
    fn visit_value(
        &mut self,
        value: &'term Value,
    )
    {
        match *value {
            | Value::Var(ref name) => self.visit_var(NameRef::from(name.as_str())),
            | Value::Hole(hole) => {
                self.found.holes.insert(HoleId::from(hole));
            },
            | Value::Unit | Value::Int(_) | Value::Str(_) | Value::Num(_) => {},
            | Value::Pair(ref fst, ref snd) => {
                self.work.push(Node::Value(fst.as_ref()));
                self.work.push(Node::Value(snd.as_ref()));
            },
            // A pack's witnesses and an annotation's type are types, and a
            // type is not a place this scan looks; each of these forms has
            // exactly one value child.
            | Value::Inj(_, ref payload)
            | Value::Here(ref payload)
            | Value::Annot(ref payload, _)
            | Value::Ctor { ref payload, .. }
            | Value::Pack { ref payload, .. } => self.work.push(Node::Value(payload.as_ref())),
            | Value::List(ref elements) => {
                for element in elements {
                    self.work.push(Node::Value(element.as_ref()));
                }
            },
            | Value::Record(ref fields) => {
                for field in fields.values() {
                    self.work.push(Node::Value(field.as_ref()));
                }
            },
            | Value::Thunk(_, ref body) | Value::Run(ref body) => {
                self.work.push(Node::Comp(body.as_ref()));
            },
            | Value::Stk(ref stack) => {
                self.found.opaque = OpaqueOccurrence::from(true);
                self.work.push(Node::Stack(stack.as_ref()));
            },
        }
    }

    /// Visits one computation, pushing its children.
    fn visit_comp(
        &mut self,
        comp: &'term Comp,
    )
    {
        match *comp {
            | Comp::Hole(hole) => {
                self.found.holes.insert(HoleId::from(hole));
            },
            | Comp::Abs(_, _, ref body)
            | Comp::Reset(ref body)
            | Comp::Shift(_, ref body)
            | Comp::Fix(_, ref body)
            | Comp::Prj(_, ref body) => self.work.push(Node::Comp(body.as_ref())),
            | Comp::App(ref head, ref arg) => {
                self.work.push(Node::Comp(head.as_ref()));
                self.work.push(Node::Value(arg.as_ref()));
            },
            | Comp::Ret(ref value)
            | Comp::Force(ref value)
            | Comp::Dup(ref value)
            | Comp::Drop(ref value)
            | Comp::Perform(_, _, ref value) => self.work.push(Node::Value(value.as_ref())),
            | Comp::Bind(ref bound, _, ref body) => {
                self.work.push(Node::Comp(bound.as_ref()));
                self.work.push(Node::Comp(body.as_ref()));
            },
            | Comp::Case(ref scrut, ref arm_fst, ref arm_snd) => {
                self.work.push(Node::Value(scrut.as_ref()));
                self.work.push(Node::Comp(arm_fst.1.as_ref()));
                self.work.push(Node::Comp(arm_snd.1.as_ref()));
            },
            | Comp::ListCase {
                ref scrut,
                ref nil,
                ref cons,
                ..
            } => {
                self.work.push(Node::Value(scrut.as_ref()));
                self.work.push(Node::Comp(nil.as_ref()));
                self.work.push(Node::Comp(cons.as_ref()));
            },
            // An unpack's ascribed signature is a type and its atoms are
            // minted identities — neither is a place this scan looks, and the
            // module binder is a name the shadowing-free discipline of the
            // module doc already covers. Its term children are the scrutinee
            // and the body, exactly as a split's are.
            | Comp::Split {
                ref scrut,
                ref body,
                ..
            }
            | Comp::Unpack {
                ref scrut,
                ref body,
                ..
            } => {
                self.work.push(Node::Value(scrut.as_ref()));
                self.work.push(Node::Comp(body.as_ref()));
            },
            | Comp::DataCase(ref scrut, ref arms) => {
                self.work.push(Node::Value(scrut.as_ref()));
                for arm in arms {
                    self.work.push(Node::Comp(arm.1.as_ref()));
                }
            },
            | Comp::With(ref fst, ref snd) => {
                self.work.push(Node::Comp(fst.as_ref()));
                self.work.push(Node::Comp(snd.as_ref()));
            },
            | Comp::RecordProj { ref record, .. } => {
                self.work.push(Node::Value(record.as_ref()));
            },
            | Comp::Handle {
                ref scrutinee,
                ret: (_, ref ret_body),
                ref ops,
                ..
            } => {
                self.work.push(Node::Comp(scrutinee.as_ref()));
                self.work.push(Node::Comp(ret_body.as_ref()));
                for clause in ops {
                    self.work.push(Node::Comp(clause.body.as_ref()));
                }
            },
            | Comp::Resume(ref reified, ref fed) => {
                self.work.push(Node::Value(reified.as_ref()));
                self.work.push(Node::Comp(fed.as_ref()));
            },
            | Comp::Native { ref args, .. } => {
                for arg in args {
                    self.work.push(Node::Value(arg.as_ref()));
                }
            },
            | Comp::Walk {
                ref scrut,
                ref base,
                ..
            } => {
                self.work.push(Node::Value(scrut.as_ref()));
                self.work.push(Node::Comp(base.body.as_ref()));
            },
        }
    }

    /// Visits one reified stack, pushing its children.
    fn visit_stack(
        &mut self,
        stack: &'term Stack,
    )
    {
        match *stack {
            | Stack::Empty => {},
            | Stack::Arg(ref value, ref rest) => {
                self.work.push(Node::Value(value.as_ref()));
                self.work.push(Node::Stack(rest.as_ref()));
            },
            | Stack::Bind(_, ref body, ref rest) => {
                self.work.push(Node::Comp(body.as_ref()));
                self.work.push(Node::Stack(rest.as_ref()));
            },
            | Stack::Prj(_, ref rest) => self.work.push(Node::Stack(rest.as_ref())),
        }
    }
}
