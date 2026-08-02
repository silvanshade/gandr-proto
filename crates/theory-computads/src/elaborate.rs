//! **Elaboration** of surface `rule` 2-cell faces into command cells
//! (`proposal-sequent-kernel.md` §7.1, "what a `rule` becomes").
//!
//! A [`gandr_theory_levitation::CellFace`] is a rewrite `lhs ~> rhs` over
//! [`FreeTerm`]s (the reserved `op`/`rule` productions of ADR-54, carried as
//! data by `gandr-theory-levitation`). [`elaborate_rule`] turns it into an
//! oriented [`Cell`] whose left side is a **cut between the matched producer
//! and an operation frame** (§7.1): an operation application `f(head, rest…)`
//! becomes `⟨head | f(rest…; $ret)⟩`, and the right side sends the result term
//! to the same return continuation `$ret`, flattening a constructor that wraps
//! an operation into a **return-side constructor frame** `K⁻` — the standard
//! sequent-machine flattening (§7.1, "Elaborating recursion on the argument
//! into an accumulator frame").
//!
//! The supported fragment is the direct one (a matched producer, an operation
//! frame, and result terms that are variables, constructors of producers, tail
//! operations, or a single constructor wrapping an operation). Shapes outside
//! it — a nested operation in producer position, a multi-argument constructor
//! with a nested operation — are **declined** with an [`ElaborateError`] rather
//! than mis-elaborated, mirroring the honest-limits posture of §7.4.
//!
//! # The operation alphabet a description supplies
//!
//! A description also declares **operations**
//! ([`gandr_theory_levitation::OpDesc`], `DataDesc::ops`), each with a
//! multi-out [`BridgeArity`]. Those arities decide what the cell layer can
//! hold, so [`elaborate_data_desc`] reads them before it reads the faces:
//!
//! * an operation whose arity is the degenerate one-monomial, one-output shape
//!   ([`BridgeArity::single_output`]) is **admitted** — its applications cut
//!   against the operation frame `f(p̄; c)`, which is what
//!   [`elaborate_operation_cut`] builds;
//! * every other arity is **declined** with an [`OpElaborateError`], because
//!   [`ConsPat::Op`] carries exactly **one** return continuation
//!   *structurally*: a many-out operation has no frame in this grammar, and a
//!   Σ-aggregating one would need the commutative monoid the Σ-zone firewall
//!   keeps out of the Π-layer (proposal-levitation.md §4.2).
//!
//! A face that applies a declined operation is declined too
//! ([`ElaborateError::UnrepresentableOperation`]) rather than elaborated into a
//! single-continuation frame that silently drops the operation's other outputs.
//! Growing the alphabet so those arities *are* representable is a cell-alphabet
//! change, not an elaboration change, and is deliberately not taken here.
//!
//! **Division of labour with the description layer.** This module checks only
//! what the *cell* layer must decide. Whether a face's symbols are in the
//! datatype's signature at all, and whether an arity's three maps compose, are
//! [`gandr_theory_levitation::check_desc`]'s checks; a caller that wants both
//! verdicts runs both passes (as `gandr-surface-engine`'s elaboration path
//! does).
//!
//! # The admission boundary
//!
//! This module is where cells **enter a store from a description**, so it is
//! where the linearity ruling binds (owner ruling 2026-08-01, placement decided
//! 2026-08-02): a rule whose left-hand side copies a hole is refused here, by
//! [`crate::linearity::admit_linear_cell`], with a diagnostic naming the copy
//! and the respelling. The refusal is deliberately *not* in
//! [`crate::sequent::CellMeta::derive`] — non-linear command patterns remain
//! constructible, because the multi-sum contract witnesses and unification
//! goals are legitimate internal shapes; what the ruling governs is admission.
//!
//! # The ADR-54 acceptance path
//!
//! Elaborating a face into the store is precisely the "flip `rule` from
//! parse-and-decline to accepted-behind-gate" of the L2 gate row (§9): this
//! crate provides the acceptance target. Feeding surface `rule` members through
//! the surface lowering (`gandr-surface-engine`'s `lower/codata.rs`, which
//! still declines them) to reach here is a cross-crate wiring step left as a
//! reported residual; description-carried rules reach here today through
//! `gandr-surface-engine`'s description cell pass.
//!
//! [`BridgeArity`]: gandr_theory_levitation::BridgeArity
//! [`BridgeArity::single_output`]: gandr_theory_levitation::BridgeArity::single_output

use alloc::boxed::Box;
use alloc::vec::Vec;

use gandr_core_sequent::il::Polarity;
use gandr_theory_levitation::CellFace;
use gandr_theory_levitation::DataDesc;
use gandr_theory_levitation::FreeTerm;
use gandr_theory_levitation::Name;
use gandr_theory_levitation::OpDesc;

use crate::boundary::DeclinedFaceIndex;
use crate::boundary::DeclinedOpIndex;
use crate::boundary::OperationInputCount;
use crate::cell::Cell;
use crate::cell::CellStore;
use crate::linearity::NonLinearPattern;
use crate::linearity::admit_linear_cell;
use crate::pattern::CmdPat;
use crate::pattern::ConsPat;
use crate::pattern::MetaVar;
use crate::pattern::ProdPat;
use crate::pattern::Sym;
use crate::sequent::CellProvenance;
use crate::sequent::Orientation;
use crate::sequent::frame_defining_cell;

/// The reserved return-continuation metavariable name a rule's cut binds — the
/// `$`-prefix keeps it disjoint from every user pattern variable.
const RETURN_CONT: &str = "$ret";

/// Why a face could not be elaborated into a command cell, or could not be
/// admitted once elaborated (`proposal-sequent-kernel.md` §7.4, the declined
/// shapes).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum ElaborateError
{
    /// The rule's left-hand side is not an operation application (a `rule`
    /// rewrites an operation redex `f(…)`).
    LhsNotOperation,
    /// An operation application carried no arguments (there is no matched
    /// producer to cut against).
    EmptyOperation,
    /// A term shape outside the supported flattening fragment (a nested
    /// operation in producer position, or a multi-argument constructor
    /// wrapping an operation).
    UnsupportedShape,
    /// The face applies an operation whose declared arity the
    /// single-continuation command-pattern grammar cannot hold (see
    /// [`OpElaborateError`]).
    UnrepresentableOperation,
    /// The face elaborated, but its left-hand side copies a hole — refused at
    /// the admission boundary, because cell patterns are linear (owner ruling,
    /// 2026-08-01; [`crate::linearity`]).
    NonLinear(NonLinearPattern),
}

impl From<NonLinearPattern> for ElaborateError
{
    #[inline]
    fn from(refusal: NonLinearPattern) -> Self
    {
        Self::NonLinear(refusal)
    }
}

/// Why a declared operation gets no cell-layer operation frame.
///
/// Each variant names a way the operation's [`BridgeArity`] leaves the shape
/// [`ConsPat::Op`] can express: one operation frame carries exactly one
/// producer-argument list and exactly one return continuation, so the only
/// arity it holds is the degenerate one-monomial, one-output shape
/// [`BridgeArity::single_output`] builds.
///
/// [`BridgeArity`]: gandr_theory_levitation::BridgeArity
/// [`BridgeArity::single_output`]: gandr_theory_levitation::BridgeArity::single_output
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum OpElaborateError
{
    /// The operation declares no output port, so its frame's return
    /// continuation would carry nothing.
    NoOutput,
    /// The operation declares **more than one** output port — the many-out
    /// arity of proposal-levitation.md §4.2, which needs one continuation per
    /// output port and therefore a wider consumer grammar.
    ManyOutput,
    /// The operation's arity feeds its single output port from **several
    /// monomials** — the Σ-layer aggregation that independently requires a
    /// commutative monoid on the target (the ADR-49 Σ-zone firewall).
    AggregatedOutput,
}

/// A declared operation the cell layer **admits**: the symbol its applications
/// cut against, and how many input ports it reads.
///
/// The elaborated operation frame carries one fewer producer argument than
/// `inputs`, because a rule's first argument is the matched producer the cut is
/// against ([`elaborate_operation_cut`]).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct OpFrame
{
    /// The operation symbol.
    pub op: Sym,
    /// The operation's declared input ports `|A|`.
    pub inputs: OperationInputCount,
}

/// The cell-layer elaboration of one whole datatype description: what reached
/// the store, and everything that did not.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DescElaboration
{
    /// The elaborated cells — a [`frame_defining_cell`] per declared
    /// constructor plus every admitted rule cell.
    pub store: CellStore,
    /// The operations the cell layer admits, in declaration order.
    pub ops: Vec<OpFrame>,
    /// Each declined operation, by index into `desc.ops`, with its reason.
    pub declined_ops: Vec<(DeclinedOpIndex, OpElaborateError)>,
    /// Each declined face, by index into `desc.cells`, with its reason.
    pub declined_faces: Vec<(DeclinedFaceIndex, ElaborateError)>,
}

/// Elaborate a whole datatype description's operations and rules into a store
/// (`proposal-sequent-kernel.md` §7.1).
///
/// # Contract
/// - ensures: `store` holds a [`frame_defining_cell`] for every declared
///   constructor (so return-side `K⁻` frames reduce) plus every rule cell that
///   passed the operation gate, elaborated, and passed the linearity admission
///   boundary ([`admit_cell`]); `ops` holds one [`OpFrame`] per operation of
///   `desc.ops` the cell layer admits and `declined_ops` the rest, both in
///   declaration order; `declined_faces` pairs each declined face (by index
///   into `desc.cells`) with its [`ElaborateError`]. A face applying a declined
///   operation is itself declined
///   ([`ElaborateError::UnrepresentableOperation`]) and never reaches the
///   store.
/// - provides: the single admission point for the store's contents — a face
///   passes the operation gate before [`elaborate_rule`] shapes it and the
///   linearity admission ([`admit_cell`]) before it enters the store, so a
///   further admission condition is one more check at this seam.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 — the operation pass is separated from the face pass by a
///   description whose only operation is admitted (its faces become cells) and
///   one whose only operation is many-out (no cell, one declined operation, and
///   any face over it declined); the two face decline channels (fragment shape
///   and admission) are separated by one description that elaborates cleanly
///   and one whose rule copies a hole.
/// - witness: `elaborate::tests::a_whole_description_elaborates_frame_and_rule_cells`
/// - witness: `elaborate::tests::a_many_out_operation_is_declined_and_declines_its_faces`
/// - witness: `elaborate::tests::an_admitted_operation_reports_its_declared_inputs`
/// - witness: `elaborate::tests::a_description_whose_rule_copies_a_hole_is_refused`
#[inline]
#[must_use]
pub fn elaborate_data_desc(desc: &DataDesc) -> DescElaboration
{
    let mut store = CellStore::new();
    // Frame cells are generated here rather than read from a description, and
    // are linear by construction (`v` and `beta` occur once each), so they do
    // not pass the admission boundary — nothing user-authored reaches it.
    for ctor in &desc.ctors {
        store.insert(frame_defining_cell(&Sym::new(ctor.name.clone())));
    }
    let mut ops = Vec::with_capacity(desc.ops.len());
    let mut declined_ops = Vec::new();
    // The names of the operations no cell may mention, gathered while the
    // operation pass runs so the face pass can consult them.
    let mut unrepresentable: Vec<&Name> = Vec::new();
    for (index, op) in desc.ops.iter().enumerate() {
        match admit_op(op) {
            | Ok(frame) => ops.push(frame),
            | Err(error) => {
                unrepresentable.push(&op.name);
                declined_ops.push((DeclinedOpIndex::from(index), error));
            },
        }
    }
    let mut declined_faces = Vec::new();
    for (index, face) in desc.cells.iter().enumerate() {
        // The admission gate: every reason a face may not become a cell is
        // decided here — the operation gate before `elaborate_rule` shapes it,
        // and the linearity admission (`admit_cell`) before the cell enters the
        // store.
        let admitted = match declined_operation(face, &unrepresentable) {
            | Some(error) => Err(error),
            | None => match elaborate_rule(face) {
                | Ok(cell) => admit_cell(&mut store, cell),
                | Err(error) => Err(error),
            },
        };
        if let Err(error) = admitted {
            declined_faces.push((DeclinedFaceIndex::from(index), error));
        }
    }
    DescElaboration {
        store,
        ops,
        declined_ops,
        declined_faces,
    }
}

/// Admit one declared operation into the cell layer's operation alphabet.
///
/// # Contract
/// - ensures: `Ok(frame)` exactly when the arity is the one-monomial,
///   one-output shape an operation frame `f(p̄; c)` can hold; the frame carries
///   the operation's symbol and its declared input count.
/// - fails: [`OpElaborateError::NoOutput`] with no output port,
///   [`OpElaborateError::ManyOutput`] with several, and
///   [`OpElaborateError::AggregatedOutput`] when several monomials feed the one
///   output port.
/// - panics: none.
///
/// # Errors
/// See the `- fails:` clause above.
#[inline]
fn admit_op(op: &OpDesc) -> Result<OpFrame, OpElaborateError>
{
    match op.arity.outputs.len() {
        | 1 => {},
        | 0 => return Err(OpElaborateError::NoOutput),
        | _ => return Err(OpElaborateError::ManyOutput),
    }
    if usize::from(op.arity.monomials()) != 1 {
        return Err(OpElaborateError::AggregatedOutput);
    }
    Ok(OpFrame {
        op: Sym::new(op.name.clone()),
        inputs: OperationInputCount::from(op.arity.inputs.len()),
    })
}

/// The decline a face earns for applying an operation the cell layer refused.
///
/// # Contract
/// - ensures: `Some(ElaborateError::UnrepresentableOperation)` iff some
///   operation application in either face mentions a name in `declined`; `None`
///   otherwise. Traversal is an explicit worklist, so face depth costs no stack
///   (ADR-47).
/// - panics: none.
#[inline]
fn declined_operation(
    face: &CellFace,
    declined: &[&Name],
) -> Option<ElaborateError>
{
    let mut stack: Vec<&FreeTerm> = alloc::vec![&face.lhs, &face.rhs];
    while let Some(current) = stack.pop() {
        match *current {
            | FreeTerm::Var(_) => {},
            | FreeTerm::Op(ref name, ref args) => {
                if declined.contains(&name) {
                    return Some(ElaborateError::UnrepresentableOperation);
                }
                stack.extend(args.iter());
            },
            | FreeTerm::Ctor(_, ref args) => stack.extend(args.iter()),
        }
    }
    None
}

/// Admit one elaborated cell into `store`, refusing a non-linear left-hand side
/// at the boundary (owner ruling, 2026-08-01: cell patterns are linear).
///
/// This is the **single admission seam** for every cell a description
/// contributes. The rule (`desc.cells`) path above calls it; any further
/// description-sourced cell path admits through the same call rather than
/// through a second copy of the check.
///
/// # Contract
/// - ensures: `Ok(())` with `cell` inserted (deduplicated as
///   [`CellStore::insert`] specifies) when its left-hand side copies no hole;
///   otherwise `store` is left untouched.
/// - fails: [`ElaborateError::NonLinear`], carrying the copied hole.
/// - panics: none.
///
/// # Errors
/// See the `- fails:` clause above.
///
/// # Adequacy
/// - hypothesis: L1 evidence — one predicate, so the refused copy (store does
///   not grow) and the admitted description separate it.
/// - witness: `elaborate::tests::a_description_whose_rule_copies_a_hole_is_refused`
/// - witness: `elaborate::tests::a_whole_description_elaborates_frame_and_rule_cells`
#[inline]
fn admit_cell(
    store: &mut CellStore,
    cell: Cell,
) -> Result<(), ElaborateError>
{
    match admit_linear_cell(&cell) {
        | Ok(()) => {
            store.insert(cell);
            Ok(())
        },
        | Err(refusal) => Err(ElaborateError::from(refusal)),
    }
}

/// Elaborate one surface `rule` face into an oriented command cell
/// (`proposal-sequent-kernel.md` §7.1).
///
/// # Contract
/// - ensures: `Ok(cell)` for a face whose left-hand side is an operation
///   `f(head, rest…)` and whose result term is in the supported fragment — the
///   cell is `⟨head | f(rest…; $ret)⟩ ~> 𝓡⟦rhs⟧$ret`, positive, provenance
///   [`CellProvenance::SurfaceRule`], with derived metadata.
/// - fails: [`ElaborateError::LhsNotOperation`] when the LHS is not an
///   operation, [`ElaborateError::EmptyOperation`] on a nullary operation LHS,
///   [`ElaborateError::UnsupportedShape`] on a term outside the fragment.
/// - panics: none.
///
/// # Errors
/// See the `- fails:` clause above.
#[inline]
pub fn elaborate_rule(face: &CellFace) -> Result<Cell, ElaborateError>
{
    let cont = ConsPat::Meta(MetaVar::consumer(RETURN_CONT));
    let lhs = elaborate_operation_cut(&face.lhs, cont.clone())?;
    let rhs = elaborate_result(&face.rhs, cont)?;
    Ok(Cell::new(
        lhs,
        rhs,
        Orientation::PolarityDerived,
        CellProvenance::SurfaceRule,
    ))
}

/// Elaborate the operation-application left-hand side into a cut against the
/// matched producer.
///
/// # Contract
/// - ensures: `Ok(⟨head | f(rest…; cont)⟩)` when `term` is `Op(f, [head,
///   rest…])` with a producer `head` and producer `rest`.
/// - fails: as [`elaborate_rule`].
/// - panics: none.
#[inline]
fn elaborate_operation_cut(
    term: &FreeTerm,
    cont: ConsPat,
) -> Result<CmdPat, ElaborateError>
{
    let (name, args) = match *term {
        | FreeTerm::Op(ref name, ref args) => (name, args),
        | FreeTerm::Var(_) | FreeTerm::Ctor(..) => return Err(ElaborateError::LhsNotOperation),
    };
    let (head, rest) = args.split_first().ok_or(ElaborateError::EmptyOperation)?;
    let producer = elaborate_producer(head)?;
    let frame_args = elaborate_producers(rest)?;
    Ok(CmdPat::cut(Polarity::Positive, producer, ConsPat::Op {
        op: Sym::new(name.clone()),
        args: frame_args,
        ret: Box::new(cont),
    }))
}

/// Elaborate a result term, sending its value to the return continuation `cont`
/// (`proposal-sequent-kernel.md` §7.1, the sequent-machine flattening).
///
/// # Contract
/// - ensures: `Ok(command)` for a variable (`⟨x | cont⟩`), a constructor of
///   producers (`⟨K(p̄) | cont⟩`), a tail operation (`⟨head | g(rest…; cont)⟩`),
///   or a single constructor wrapping an operation (`K(op(…))`, flattened to a
///   `K⁻` return frame).
/// - fails: [`ElaborateError::UnsupportedShape`] otherwise.
/// - panics: none.
#[inline]
fn elaborate_result(
    term: &FreeTerm,
    cont: ConsPat,
) -> Result<CmdPat, ElaborateError>
{
    let mut current = term;
    let mut current_cont = cont;
    loop {
        match *current {
            | FreeTerm::Var(ref name) => {
                return Ok(CmdPat::cut(
                    Polarity::Positive,
                    ProdPat::meta(name.clone()),
                    current_cont,
                ));
            },
            | FreeTerm::Op(ref name, ref args) => {
                let (head, rest) = args.split_first().ok_or(ElaborateError::EmptyOperation)?;
                let producer = elaborate_producer(head)?;
                let frame_args = elaborate_producers(rest)?;
                return Ok(CmdPat::cut(Polarity::Positive, producer, ConsPat::Op {
                    op: Sym::new(name.clone()),
                    args: frame_args,
                    ret: Box::new(current_cont),
                }));
            },
            | FreeTerm::Ctor(ref name, ref args) => {
                if let Ok(producers) = elaborate_producers(args) {
                    return Ok(CmdPat::cut(
                        Polarity::Positive,
                        ProdPat::Ctor {
                            ctor: Sym::new(name.clone()),
                            args: producers,
                        },
                        current_cont,
                    ));
                }

                match args.first() {
                    | Some(inner) if args.len() == 1 => {
                        current_cont = ConsPat::Frame {
                            ctor: Sym::new(name.clone()),
                            ret: Box::new(current_cont),
                        };
                        current = inner;
                    },
                    | _ => return Err(ElaborateError::UnsupportedShape),
                }
            },
        }
    }
}

/// Elaborate a term as a **producer pattern** — a variable or a constructor of
/// producers.
///
/// # Contract
/// - ensures: `Ok(ProdPat)` for a variable or a constructor whose arguments are
///   all producers.
/// - fails: [`ElaborateError::UnsupportedShape`] for an operation (which is not
///   a producer) or a constructor with a non-producer argument.
/// - panics: none.
#[inline]
fn elaborate_producer(term: &FreeTerm) -> Result<ProdPat, ElaborateError>
{
    enum Frame<'term>
    {
        Enter(&'term FreeTerm),
        ExitCtor(Sym, usize),
    }

    let mut stack = alloc::vec![Frame::Enter(term)];
    let mut out: Vec<ProdPat> = Vec::new();
    while let Some(frame) = stack.pop() {
        match frame {
            | Frame::Enter(node) => match *node {
                | FreeTerm::Var(ref name) => out.push(ProdPat::meta(name.clone())),
                | FreeTerm::Ctor(ref name, ref args) => {
                    stack.push(Frame::ExitCtor(Sym::new(name.clone()), args.len()));
                    stack.extend(args.iter().rev().map(Frame::Enter));
                },
                | FreeTerm::Op(..) => return Err(ElaborateError::UnsupportedShape),
            },
            | Frame::ExitCtor(name, arity) => {
                let split_at = out
                    .len()
                    .checked_sub(arity)
                    .ok_or(ElaborateError::UnsupportedShape)?;
                let args = out.split_off(split_at).into_boxed_slice();
                out.push(ProdPat::Ctor { ctor: name, args });
            },
        }
    }
    out.pop().ok_or(ElaborateError::UnsupportedShape)
}

/// Elaborate a term slice as producer patterns, failing if any is not a
/// producer.
///
/// # Contract
/// - ensures: `Ok` of one producer per term, in order, iff every term is a
///   producer.
/// - fails: [`ElaborateError::UnsupportedShape`] on the first non-producer.
/// - panics: none.
#[inline]
fn elaborate_producers(terms: &[FreeTerm]) -> Result<Box<[ProdPat]>, ElaborateError>
{
    let mut out = Vec::with_capacity(terms.len());
    for term in terms {
        let producer = elaborate_producer(term)?;
        out.push(producer);
    }
    Ok(out.into_boxed_slice())
}

#[cfg(test)]
mod tests
{
    use gandr_theory_levitation::Attrs;
    use gandr_theory_levitation::BridgeArity;
    use gandr_theory_levitation::Code;
    use gandr_theory_levitation::CtorDesc;
    use gandr_theory_levitation::DeclPolarity;
    use gandr_theory_levitation::NominalId;
    use gandr_theory_levitation::SortRef;
    use gandr_theory_levitation::SurfaceSpan;

    use super::*;
    use crate::pattern::ConsPat;

    #[test]
    fn add_zero_elaborates_to_a_cut_against_the_operation_frame()
    {
        // rule add(Zero, n) ~> n.
        let f = face(
            FreeTerm::op("add", [FreeTerm::ctor("Zero", []), FreeTerm::var("n")]),
            FreeTerm::var("n"),
        );
        let cell = elaborate_rule(&f).expect("the direct case elaborates");
        let expected_lhs = CmdPat::cut(
            Polarity::Positive,
            ProdPat::ctor("Zero", []),
            ConsPat::op("add", [ProdPat::meta("n")], ConsPat::meta(RETURN_CONT)),
        );
        assert_eq!(cell.lhs, expected_lhs, "⟨Zero | add(n; $ret)⟩");
        assert_eq!(
            cell.rhs,
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::meta("n"),
                ConsPat::meta(RETURN_CONT)
            ),
            "⟨n | $ret⟩"
        );
    }

    #[test]
    fn add_succ_flattens_the_wrapping_constructor_into_a_frame()
    {
        // rule add(Succ(m), n) ~> Succ(add(m, n)).
        let f = face(
            FreeTerm::op("add", [
                FreeTerm::ctor("Succ", [FreeTerm::var("m")]),
                FreeTerm::var("n"),
            ]),
            FreeTerm::ctor("Succ", [FreeTerm::op("add", [
                FreeTerm::var("m"),
                FreeTerm::var("n"),
            ])]),
        );
        let cell = elaborate_rule(&f).expect("the flattening case elaborates");
        let expected_rhs = CmdPat::cut(
            Polarity::Positive,
            ProdPat::meta("m"),
            ConsPat::op(
                "add",
                [ProdPat::meta("n")],
                ConsPat::frame("Succ", ConsPat::meta(RETURN_CONT)),
            ),
        );
        assert_eq!(cell.rhs, expected_rhs, "⟨m | add(n; Succ⁻($ret))⟩");
    }

    #[test]
    fn a_non_operation_lhs_is_declined()
    {
        let f = face(FreeTerm::var("x"), FreeTerm::var("x"));
        assert_eq!(
            Err(ElaborateError::LhsNotOperation),
            elaborate_rule(&f),
            "a rule rewrites an op"
        );
    }

    #[test]
    fn a_whole_description_elaborates_frame_and_rule_cells()
    {
        // A Nat description declaring `add` and carrying its two rules.
        let desc = nat_with(
            [op("add", [
                SortRef::new("m", "Nat"),
                SortRef::new("n", "Nat"),
            ])],
            [
                face(
                    FreeTerm::op("add", [FreeTerm::ctor("Zero", []), FreeTerm::var("n")]),
                    FreeTerm::var("n"),
                ),
                face(
                    FreeTerm::op("add", [
                        FreeTerm::ctor("Succ", [FreeTerm::var("m")]),
                        FreeTerm::var("n"),
                    ]),
                    FreeTerm::ctor("Succ", [FreeTerm::op("add", [
                        FreeTerm::var("m"),
                        FreeTerm::var("n"),
                    ])]),
                ),
            ],
        );
        let elaborated = elaborate_data_desc(&desc);
        assert!(
            elaborated.declined_faces.is_empty(),
            "both rules are in the supported fragment"
        );
        assert!(
            elaborated.declined_ops.is_empty(),
            "a single-output `add` is admitted"
        );
        // Two frame cells (Zero⁻, Succ⁻) plus the two rule cells.
        assert_eq!(
            crate::boundary::CellCount::from(4_usize),
            elaborated.store.len(),
            "two frame cells and two rule cells"
        );
    }

    #[test]
    fn a_description_whose_rule_copies_a_hole_is_refused()
    {
        use gandr_theory_levitation::Attrs;
        use gandr_theory_levitation::Code;
        use gandr_theory_levitation::CtorDesc;
        use gandr_theory_levitation::DataDesc;
        use gandr_theory_levitation::DeclPolarity;
        use gandr_theory_levitation::NominalId;

        // `rule and(x, x) ~> x` — the idempotence law written with a repeated
        // hole. It elaborates to ⟨x | and(x; $ret)⟩ ~> ⟨x | $ret⟩, whose left
        // side copies the producer hole `x`, so the admission boundary refuses
        // it rather than letting it into the store.
        let desc = DataDesc::new(
            NominalId::new(0_u64.into(), "Bit"),
            Vec::new(),
            [CtorDesc::new("Off", Code::Unit, None, Attrs::empty())],
            Vec::new(),
            [face(
                FreeTerm::op("and", [FreeTerm::var("x"), FreeTerm::var("x")]),
                FreeTerm::var("x"),
            )],
            DeclPolarity::Data,
            Attrs::empty(),
        );
        let elaborated = elaborate_data_desc(&desc);
        assert_eq!(
            crate::boundary::CellCount::from(1_usize),
            elaborated.store.len(),
            "only the Off frame cell is admitted; the copying rule is not"
        );
        let &(index, ref error) = elaborated
            .declined_faces
            .first()
            .expect("the face is declined");
        assert_eq!(
            DeclinedFaceIndex::from(0_usize),
            index,
            "the decline is reported against the face's index"
        );
        let ElaborateError::NonLinear(ref refusal) = *error
        else {
            panic!("the decline is the linearity refusal, not a fragment decline")
        };
        assert_eq!(
            &*refusal.copied.name, "x",
            "the diagnostic names the copied hole"
        );
    }

    #[test]
    fn an_admitted_operation_reports_its_declared_inputs()
    {
        let desc = nat_with(
            [op("add", [
                SortRef::new("m", "Nat"),
                SortRef::new("n", "Nat"),
            ])],
            [],
        );
        let elaborated = elaborate_data_desc(&desc);
        assert_eq!(
            vec![OpFrame {
                op: Sym::new("add"),
                inputs: OperationInputCount::from(2_usize),
            }],
            elaborated.ops,
            "the admitted operation carries its symbol and its declared |A|"
        );
    }

    #[test]
    fn a_many_out_operation_is_declined_and_declines_its_faces()
    {
        // `op divmod(m, n) -> (q, r)` — two output ports, so two monomials, and
        // an operation frame has exactly one return continuation.
        let divmod = OpDesc::new(
            "divmod",
            BridgeArity::new(
                [SortRef::new("m", "Nat"), SortRef::new("n", "Nat")],
                [2u32, 2u32],
                [0u32, 1u32, 0u32, 1u32],
                [0u32, 1u32],
                [SortRef::new("q", "Nat"), SortRef::new("r", "Nat")],
            ),
            Attrs::empty(),
        );
        let desc = nat_with([divmod], [face(
            FreeTerm::op("divmod", [FreeTerm::ctor("Zero", []), FreeTerm::var("n")]),
            FreeTerm::ctor("Zero", []),
        )]);
        let elaborated = elaborate_data_desc(&desc);
        assert_eq!(
            vec![(DeclinedOpIndex::from(0_usize), OpElaborateError::ManyOutput)],
            elaborated.declined_ops,
            "a many-out arity has no operation frame in this grammar"
        );
        assert!(elaborated.ops.is_empty(), "nothing was admitted");
        assert_eq!(
            vec![(
                DeclinedFaceIndex::from(0_usize),
                ElaborateError::UnrepresentableOperation
            )],
            elaborated.declined_faces,
            "a face over a declined operation is declined, not silently narrowed"
        );
        assert_eq!(
            crate::boundary::CellCount::from(2_usize),
            elaborated.store.len(),
            "only the two constructor frame cells reached the store"
        );
    }

    #[test]
    fn an_aggregating_arity_and_an_outputless_one_are_declined_apart()
    {
        // One output port fed by two monomials: the Σ-layer's monoid obligation.
        let aggregated = OpDesc::new(
            "merge",
            BridgeArity::new(
                [SortRef::new("p", "Nat"), SortRef::new("q", "Nat")],
                [1u32, 1u32],
                [0u32, 1u32],
                [0u32, 0u32],
                [SortRef::new("r", "Nat")],
            ),
            Attrs::empty(),
        );
        let outputless = OpDesc::new(
            "sink",
            BridgeArity::new([SortRef::new("p", "Nat")], [], [], [], []),
            Attrs::empty(),
        );
        let elaborated = elaborate_data_desc(&nat_with([aggregated, outputless], []));
        assert_eq!(
            vec![
                (
                    DeclinedOpIndex::from(0_usize),
                    OpElaborateError::AggregatedOutput
                ),
                (DeclinedOpIndex::from(1_usize), OpElaborateError::NoOutput),
            ],
            elaborated.declined_ops,
            "the Σ-aggregating and the outputless arity decline for their own reasons"
        );
    }

    #[test]
    fn a_face_over_an_admitted_operation_survives_the_gate()
    {
        // The declined operation is not the one this face applies, so the gate
        // lets it through to the fragment rules.
        let desc = nat_with(
            [
                op("id", [SortRef::new("x", "Nat")]),
                OpDesc::new(
                    "divmod",
                    BridgeArity::new(
                        [SortRef::new("m", "Nat")],
                        [1u32, 1u32],
                        [0u32, 0u32],
                        [0u32, 1u32],
                        [SortRef::new("q", "Nat"), SortRef::new("r", "Nat")],
                    ),
                    Attrs::empty(),
                ),
            ],
            [face(
                FreeTerm::op("id", [FreeTerm::ctor("Zero", [])]),
                FreeTerm::ctor("Zero", []),
            )],
        );
        let elaborated = elaborate_data_desc(&desc);
        assert!(
            elaborated.declined_faces.is_empty(),
            "the face applies `id`, which is admitted"
        );
        assert_eq!(
            crate::boundary::CellCount::from(3_usize),
            elaborated.store.len(),
            "two frame cells and the one rule cell"
        );
    }

    fn face(
        lhs: FreeTerm,
        rhs: FreeTerm,
    ) -> CellFace
    {
        CellFace::new(
            lhs,
            rhs,
            Vec::new(),
            SurfaceSpan::new(0_usize.into(), 0_usize.into()),
        )
    }

    /// A single-output operation over the given input ports.
    fn op<N, I>(
        name: N,
        inputs: I,
    ) -> OpDesc
    where
        N: Into<gandr_theory_levitation::Name>,
        I: Into<Box<[SortRef]>>,
    {
        OpDesc::new(
            name.into(),
            BridgeArity::single_output(inputs, SortRef::new("out", "Nat")),
            Attrs::empty(),
        )
    }

    /// A `Nat`-like description with two constructors plus the given operations
    /// and faces.
    fn nat_with<O, C>(
        ops: O,
        cells: C,
    ) -> DataDesc
    where
        O: Into<Box<[OpDesc]>>,
        C: Into<Box<[CellFace]>>,
    {
        DataDesc::new(
            NominalId::new(0_u64.into(), "Nat"),
            Vec::new(),
            [
                CtorDesc::new("Zero", Code::Unit, None, Attrs::empty()),
                CtorDesc::new("Succ", Code::Var, None, Attrs::empty()),
            ],
            ops,
            cells,
            DeclPolarity::Data,
            Attrs::empty(),
        )
    }
}
