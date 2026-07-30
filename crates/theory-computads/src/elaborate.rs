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
//! # The ADR-54 acceptance path
//!
//! Elaborating a face into the store is precisely the "flip `rule` from
//! parse-and-decline to accepted-behind-gate" of the L2 gate row (§9): this
//! crate provides the acceptance target. Feeding surface `rule` members through
//! the pipeline lowering (`gandr-pipeline`'s `codata.rs`, which still declines
//! them) to reach here is a cross-crate wiring step left as a reported
//! residual.

use alloc::boxed::Box;
use alloc::vec::Vec;

use gandr_core_sequent::il::Polarity;
use gandr_theory_levitation::CellFace;
use gandr_theory_levitation::DataDesc;
use gandr_theory_levitation::FreeTerm;

use crate::boundary::DeclinedFaceIndex;
use crate::cell::Cell;
use crate::cell::CellProvenance;
use crate::cell::CellStore;
use crate::cell::Orientation;
use crate::cell::frame_defining_cell;
use crate::pattern::CmdPat;
use crate::pattern::ConsPat;
use crate::pattern::MetaVar;
use crate::pattern::ProdPat;
use crate::pattern::Sym;

/// The reserved return-continuation metavariable name a rule's cut binds — the
/// `$`-prefix keeps it disjoint from every user pattern variable.
const RETURN_CONT: &str = "$ret";

/// Why a face could not be elaborated into a command cell
/// (`proposal-sequent-kernel.md` §7.4, the declined shapes).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
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
}

/// Elaborate a whole datatype description's operations and rules into a store
/// (`proposal-sequent-kernel.md` §7.1).
///
/// # Contract
/// - ensures: the returned store holds a [`frame_defining_cell`] for every
///   declared constructor (so return-side `K⁻` frames reduce) plus every
///   successfully-elaborated rule cell; the returned vector pairs each declined
///   face (by index into `desc.cells`) with its [`ElaborateError`]. Never
///   panics.
/// - panics: none.
#[inline]
#[must_use]
pub fn elaborate_data_desc(desc: &DataDesc)
-> (CellStore, Vec<(DeclinedFaceIndex, ElaborateError)>)
{
    let mut store = CellStore::new();
    for ctor in &desc.ctors {
        store.insert(frame_defining_cell(&Sym::new(ctor.name.clone())));
    }
    let mut declines = Vec::new();
    for (index, face) in desc.cells.iter().enumerate() {
        match elaborate_rule(face) {
            | Ok(cell) => {
                store.insert(cell);
            },
            | Err(error) => declines.push((DeclinedFaceIndex::from(index), error)),
        }
    }
    (store, declines)
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
        use gandr_theory_levitation::Attrs;
        use gandr_theory_levitation::Code;
        use gandr_theory_levitation::CtorDesc;
        use gandr_theory_levitation::DataDesc;
        use gandr_theory_levitation::DeclPolarity;
        use gandr_theory_levitation::NominalId;

        // A Nat description carrying the two add rules.
        let desc = DataDesc::new(
            NominalId::new(0_u64.into(), "Nat"),
            Vec::new(),
            [
                CtorDesc::new("Zero", Code::Unit, None, Attrs::empty()),
                CtorDesc::new("Succ", Code::Var, None, Attrs::empty()),
            ],
            Vec::new(),
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
            DeclPolarity::Data,
            Attrs::empty(),
        );
        let (store, declines) = elaborate_data_desc(&desc);
        assert!(
            declines.is_empty(),
            "both rules are in the supported fragment"
        );
        // Two frame cells (Zero⁻, Succ⁻) plus the two rule cells.
        assert_eq!(
            crate::boundary::CellCount::from(4_usize),
            store.len(),
            "two frame cells and two rule cells"
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
}
