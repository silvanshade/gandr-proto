//! **Direct command-level rewriting** — applying cells to command
//! configurations and normalizing under a budget, generic over the
//! [`CellAlphabet`] (the executed meta-spike-01).
//!
//! Spec: the sequent-machines design's §7.3.4, §4.1 — the "direct command-level
//! rewriting" the fused≡two-step differential runs on.
//!
//! Because the cell-visible fragment has no nested commands (a cut's children
//! are a producer and a consumer, never another command —
//! [`gandr_theory_cell_complexes::pattern`]), every redex sits at a **command
//! position**, and for a single cut that is the root. [`apply_once`] tries each
//! cell at each command position (deterministic cell order, outermost position
//! first); [`normalize`] iterates to a normal form or a **budget** exhaustion —
//! the same decline-and-report posture as the completion budget (§7.3.3) and
//! the machine's step budget.
//!
//! # Firing discipline
//!
//! A cell fires only where its provenance permits
//! ([`CellAlphabet::may_fire`]) — the sequent alphabet's η-polarity
//! discipline (§5, K2): an η cell
//! ([`gandr_theory_cell_complexes::sequent::CellProvenance::Eta`]) may fire
//! **only** at a cut whose polarity matches its
//! [`gandr_theory_cell_complexes::sequent::EtaKind`]. [`rewrite_at`] consults
//! the hook before contracting, so an η-step at the wrong polarity is rejected
//! — the `eta-wrong-polarity` pathological pin (§11).

use alloc::vec::Vec;

use gandr_theory_cell_complexes::alphabet::CellAlphabet;
use gandr_theory_cell_complexes::boundary::NormalizationBudget;
use gandr_theory_cell_complexes::cell::Cell;
use gandr_theory_cell_complexes::cell::CellId;
use gandr_theory_cell_complexes::cell::CellStore;
use gandr_theory_cell_complexes::sequent::SequentAlphabet;

/// One **rewrite step** — a cell applied at a position (`CellApp` of the
/// tracelet sketch, `proposal-sequent-kernel.md` §7.3).
///
/// A step records only *which* cell fired *where*, never the matched
/// substitution: replay ([`crate::tracelet`]) re-matches and re-contracts, so a
/// certificate is re-executed rather than trusted (ADR-69).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct CellApp<A: CellAlphabet = SequentAlphabet>
{
    /// The cell that fired.
    pub cell: CellId,
    /// The position it fired at.
    pub at: A::Pos,
}

/// The result of a single successful rewrite — the step and the rewritten term.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Rewrite<A: CellAlphabet = SequentAlphabet>
{
    /// The step that fired.
    pub step: CellApp<A>,
    /// The whole term after the contraction.
    pub result: A::Cmd,
}

/// The outcome of [`normalize`] — the normal form, the path taken, and whether
/// the budget was exhausted before a normal form was reached.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Normalization<A: CellAlphabet = SequentAlphabet>
{
    /// The term reached (a normal form when `!exhausted`).
    pub normal: A::Cmd,
    /// The rewrite path from the input to `normal`.
    pub path: Vec<CellApp<A>>,
    /// Whether the budget ran out before a normal form was reached.
    pub exhausted: bool,
}

/// Normalize `term` under `store`, taking at most `budget` steps.
///
/// # Contract
/// - requires: the ground/skolemized fragment (matching binds every LHS
///   metavariable).
/// - ensures: the returned [`Normalization`] reaches a normal form with
///   `exhausted == false` within `budget` steps, or stops at `budget` steps
///   with `exhausted == true`; `path` records every fired step in order and
///   re-running it reproduces `normal`. Never diverges or panics (the budget is
///   the guard).
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L1 evidence — the sequent-alphabet pins (frame reduction,
///   η-polarity rejection, zero-budget report) hold verbatim through the
///   generic loop, and the toy alphabet normalizes a ground term to the same
///   normal form the two constituent steps reach.
/// - witness: `rewrite::tests::a_frame_defining_cell_fires_at_the_root`
/// - witness: `rewrite::tests::an_eta_cell_is_rejected_at_the_wrong_polarity`
/// - witness: `rewrite::tests::a_budget_of_zero_reports_a_pending_redex`
/// - witness: `toy_alphabet::tests::the_normalizer_runs_over_the_toy_alphabet`
#[inline]
#[must_use]
pub fn normalize<A>(
    store: &CellStore<A>,
    term: &A::Cmd,
    budget: NormalizationBudget,
) -> Normalization<A>
where
    A: CellAlphabet,
{
    let mut current = term.clone();
    let mut path = Vec::new();
    let mut remaining = usize::from(budget);
    loop {
        if remaining == 0 {
            let exhausted = apply_once(store, &current).is_some();
            return Normalization {
                normal: current,
                path,
                exhausted,
            };
        }
        match apply_once(store, &current) {
            | Some(rewrite) => {
                path.push(rewrite.step);
                current = rewrite.result;
                remaining = remaining.saturating_sub(1);
            },
            | None => {
                return Normalization {
                    normal: current,
                    path,
                    exhausted: false,
                };
            },
        }
    }
}

/// Fire the first applicable cell at the outermost command position.
///
/// # Contract
/// - ensures: `Some(rewrite)` for the first `(position, cell)` pair — outermost
///   position, then store order — whose [`rewrite_at`] succeeds; `None` when
///   the term is a normal form (no cell fires anywhere).
/// - panics: none.
#[inline]
#[must_use]
pub fn apply_once<A>(
    store: &CellStore<A>,
    term: &A::Cmd,
) -> Option<Rewrite<A>>
where
    A: CellAlphabet,
{
    for pos in A::command_positions(term) {
        for (id, cell) in store.iter() {
            if let Some(result) = rewrite_at(cell, term, &pos) {
                return Some(Rewrite {
                    step: CellApp { cell: id, at: pos },
                    result,
                });
            }
        }
    }
    None
}

/// Try to fire `cell` at `pos` in `term`, returning the rewritten term.
///
/// # Contract
/// - ensures: `Some(result)` when the subterm at `pos` is a command matching
///   `cell.lhs` (ground matching, polarity included) **and** the firing
///   discipline ([`CellAlphabet::may_fire`]) permits it; `None` when there is
///   no redex, the position is not a command, or the discipline refuses the
///   firing (an η cell at the wrong-polarity cut, in the sequent alphabet).
/// - panics: none.
#[inline]
#[must_use]
pub fn rewrite_at<A>(
    cell: &Cell<A>,
    term: &A::Cmd,
    pos: &A::Pos,
) -> Option<A::Cmd>
where
    A: CellAlphabet,
{
    let sub = A::subterm_cmd_at(term, pos)?;
    // The firing discipline (§5): an η cell fires only at its required polarity.
    if !bool::from(A::may_fire(&cell.provenance, &sub)) {
        return None;
    }
    let mut subst = A::Subst::default();
    if !bool::from(A::match_cmd(&cell.lhs, &sub, &mut subst)) {
        return None;
    }
    let contractum = A::apply_subst(&subst, &cell.rhs);
    A::splice_cmd_at(term, pos, contractum)
}

#[cfg(test)]
mod tests
{
    use gandr_core_sequent::il::Polarity;
    use gandr_theory_cell_complexes::pattern::CmdPat;
    use gandr_theory_cell_complexes::pattern::ConsPat;
    use gandr_theory_cell_complexes::pattern::Pos;
    use gandr_theory_cell_complexes::pattern::ProdPat;
    use gandr_theory_cell_complexes::pattern::Sym;
    use gandr_theory_cell_complexes::sequent::CellProvenance;
    use gandr_theory_cell_complexes::sequent::EtaKind;
    use gandr_theory_cell_complexes::sequent::Orientation;
    use gandr_theory_cell_complexes::sequent::frame_defining_cell;

    use super::*;

    #[test]
    fn a_frame_defining_cell_fires_at_the_root()
    {
        // ⟨Zero | Succ⁻(★)⟩ ~> ⟨Succ(Zero) | ★⟩.
        let mut store = CellStore::new();
        store.insert(frame_defining_cell(&Sym::new("Succ")));
        let term = CmdPat::cut(
            Polarity::Positive,
            ProdPat::ctor("Zero", []),
            ConsPat::frame("Succ", ConsPat::Top),
        );
        let out = normalize(&store, &term, 16_usize.into());
        let expected = CmdPat::cut(
            Polarity::Positive,
            ProdPat::ctor("Succ", [ProdPat::ctor("Zero", [])]),
            ConsPat::Top,
        );
        assert_eq!(out.normal, expected, "the μ̃ reduction wrapped Zero in Succ");
        assert_eq!(1, out.path.len(), "exactly one step");
        assert!(!out.exhausted, "reached a normal form");
    }

    #[test]
    fn an_eta_cell_is_rejected_at_the_wrong_polarity()
    {
        // A data-η cell (requires positive) built over a negative cut is refused.
        let lhs = CmdPat::cut(Polarity::Negative, ProdPat::meta("x"), ConsPat::meta("a"));
        let rhs = lhs.clone();
        let eta: Cell = Cell::new(
            lhs.clone(),
            rhs,
            Orientation::PolarityDerived,
            CellProvenance::Eta(EtaKind::Data),
        );
        // The target is a negative cut; data-η requires positive.
        assert_eq!(
            None,
            rewrite_at(&eta, &lhs, &Pos::root()),
            "data-η must not fire at a negative cut"
        );
    }

    #[test]
    fn a_budget_of_zero_reports_a_pending_redex()
    {
        let mut store = CellStore::new();
        store.insert(frame_defining_cell(&Sym::new("Succ")));
        let term = CmdPat::cut(
            Polarity::Positive,
            ProdPat::ctor("Zero", []),
            ConsPat::frame("Succ", ConsPat::Top),
        );
        let out = normalize(&store, &term, 0_usize.into());
        assert!(out.exhausted, "a redex remained but the budget was zero");
        assert_eq!(0, out.path.len(), "no step was taken");
    }
}
