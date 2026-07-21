//! Validation passes for the checked PBG model.

use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;

use gandr_surface_syntax::GrammarFingerprint;

use crate::model::PbgError;
use crate::model::Regex;
use crate::model::Rule;
use crate::model::RuleName;
use crate::model::Sort;
use crate::model::Sym;
use crate::mold::MoldTable;

/// Nullable/FIRST/LAST summary for one regex subtree.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct Summary
{
    /// Whether the expression accepts the empty sequence.
    nullable: bool,
    /// Sorts that can appear first in a generated sequence.
    first: BTreeSet<Sort>,
    /// Sorts that can appear last in a generated sequence.
    last: BTreeSet<Sort>,
}

/// Validate Operator Form for one rule.
///
/// # Contract
/// - requires: `rule` is one grammar rule over the closed [`Regex`] algebra.
/// - ensures: returns `Ok` only when no concatenation can expose adjacent sort
///   symbols, including through nullable optional, repeat, alternative, or
///   empty branches.
/// - provides: the hard Operator Form gate for [`crate::Pbg::build`].
/// - fails: returns [`PbgError::AdjacentSorts`] with the rule and the exposed
///   left/right sorts.
/// - panics: none.
/// - intension: traversal is recursive only over grammar-construction depth;
///   PBG data is constant generator output, not user input, so the recursion
///   bound is the statically emitted regex nesting depth of sibling surface
///   modules.
///
/// # Errors
/// Returns [`PbgError::AdjacentSorts`] for the first deterministic
/// adjacent-sort exposure found in left-to-right traversal.
///
/// # Adequacy
/// - hypothesis: L3 pointwise plus L1 evidence — witnesses cover direct
///   concatenation and nullable optional/repeat/alt/empty bridges, observing
///   the reported rule and left/right sorts.
/// - witness: `gandr_surface_grammar::contracts::operator_form_nullable_contract`
#[inline]
pub fn validate_operator_form(rule: &Rule) -> Result<(), PbgError>
{
    let _summary = summarize(rule.name(), rule.regex())?;
    Ok(())
}

/// Validate Unique Tiles under rctx mold identity across all rules.
///
/// # Contract
/// - requires: `rules` are the full rule set of a PBG candidate.
/// - ensures: returns `Ok` only when every tile occurrence interns to a
///   distinct `(label, rctx)` — cloned element subtrees at different regex
///   positions have distinct zipper contexts, so the `comma1`/`repeat1` class
///   resolves per occurrence.
/// - provides: the hard Unique Tiles gate for [`crate::Pbg::build`], delegated
///   to the authoritative mold-table build.
/// - fails: returns [`PbgError::DuplicateTile`] carrying both rule names.
/// - panics: none.
/// - intension: rules are visited in input order and each rule's regex left to
///   right, so the first duplicate report is deterministic.
///
/// # Errors
/// Returns [`PbgError::DuplicateTile`] for the first duplicated
/// `(label, rctx)` occurrence — genuine redundancy such as an alternation of
/// identical branches.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — an alternation of identical branches collides
///   while distinct positions (including `comma1` element clones) do not.
/// - witness: `gandr_surface_grammar::contracts::unique_tiles_contract`
#[inline]
pub fn validate_unique_tiles(rules: &[Rule]) -> Result<(), PbgError>
{
    MoldTable::build(rules, GrammarFingerprint(0)).map(|_table| ())
}

/// Validate the dissertation's Assumption 3 (precedence conflict-freedom).
///
/// # Contract
/// - requires: `rules` are the full rule set of a PBG candidate.
/// - ensures: returns `Ok` only when no distinct sorts `r ≠ s` satisfy both `s
///   ∈ FIRST(⟦G(r, p)⟧)` and `r ∈ LAST(⟦G(s, q)⟧)` for some precedences `p, q`
///   (diss. p. 87).
/// - provides: the third checked grammar assumption beside Operator Form and
///   Unique Tiles, as an adaptation trigger for [`crate::Pbg::build`].
/// - fails: returns [`PbgError::Assumption3Conflict`] naming the conflicting
///   sorts.
/// - panics: none.
/// - intension: FIRST/LAST sort sets are aggregated per producing sort over all
///   precedences, then sort pairs are examined in deterministic order.
///
/// # Errors
/// Returns [`PbgError::Assumption3Conflict`] for the first conflicting sort
/// pair in deterministic order.
///
/// # Adequacy
/// - hypothesis: L3 — a form beginning with one sort paired with a form ending
///   with the other kills the conflict branch while the built-in surface
///   witnesses the accepted path.
/// - witness: `gandr_surface_grammar::contracts::assumption_3_contract`
#[inline]
pub fn validate_assumption_3(rules: &[Rule]) -> Result<(), PbgError>
{
    let mut first_sorts: BTreeMap<Sort, BTreeSet<Sort>> = BTreeMap::new();
    let mut last_sorts: BTreeMap<Sort, BTreeSet<Sort>> = BTreeMap::new();
    for rule in rules {
        let summary = summarize(rule.name(), rule.regex())?;
        first_sorts
            .entry(rule.sort())
            .or_default()
            .extend(summary.first);
        last_sorts
            .entry(rule.sort())
            .or_default()
            .extend(summary.last);
    }
    for (first_sort, begins) in &first_sorts {
        for second_sort in begins {
            if *second_sort == *first_sort {
                continue;
            }
            if last_sorts
                .get(second_sort)
                .is_some_and(|ends| ends.contains(first_sort))
            {
                return Err(PbgError::Assumption3Conflict {
                    first_sort: *first_sort,
                    second_sort: *second_sort,
                });
            }
        }
    }
    Ok(())
}

/// Summarize one regex subtree.
fn summarize(
    rule: RuleName,
    regex: &Regex,
) -> Result<Summary, PbgError>
{
    enum Frame<'regex>
    {
        Enter(&'regex Regex),
        FinishSeq(usize),
        FinishAlt(usize),
        FinishNullable,
    }

    let mut frames = vec![Frame::Enter(regex)];
    let mut summaries = Vec::new();
    while let Some(frame) = frames.pop() {
        match frame {
            | Frame::Enter(node) => match *node {
                | Regex::Empty => summaries.push(Summary {
                    nullable: true,
                    first: BTreeSet::new(),
                    last: BTreeSet::new(),
                }),
                | Regex::Sym(Sym::Tile(_tile)) => summaries.push(Summary {
                    nullable: false,
                    first: BTreeSet::new(),
                    last: BTreeSet::new(),
                }),
                | Regex::Sym(Sym::Sort(sort)) => {
                    summaries.push(summary_for_sort(sort));
                },
                | Regex::Seq(ref items) => {
                    frames.push(Frame::FinishSeq(items.len()));
                    for item in items.iter().rev() {
                        frames.push(Frame::Enter(item));
                    }
                },
                | Regex::Alt(ref items) => {
                    frames.push(Frame::FinishAlt(items.len()));
                    for item in items.iter().rev() {
                        frames.push(Frame::Enter(item));
                    }
                },
                | Regex::Optional(ref inner) | Regex::Repeat(ref inner) => {
                    frames.push(Frame::FinishNullable);
                    frames.push(Frame::Enter(inner));
                },
            },
            | Frame::FinishSeq(len) => {
                let start = summaries.len().saturating_sub(len);
                let children = summaries.drain(start ..).collect::<Vec<_>>();
                let mut acc = Summary {
                    nullable: true,
                    first: BTreeSet::new(),
                    last: BTreeSet::new(),
                };
                for current in children {
                    reject_adjacent(rule, &acc.last, &current.first)?;
                    let next_first = seq_first(&acc, &current);
                    let next_last = seq_last(&acc, &current);
                    acc = Summary {
                        nullable: acc.nullable && current.nullable,
                        first: next_first,
                        last: next_last,
                    };
                }
                summaries.push(acc);
            },
            | Frame::FinishAlt(len) => {
                let start = summaries.len().saturating_sub(len);
                let children = summaries.drain(start ..).collect::<Vec<_>>();
                let mut acc = Summary::default();
                for current in children {
                    acc.nullable = acc.nullable || current.nullable;
                    acc.first.extend(current.first);
                    acc.last.extend(current.last);
                }
                summaries.push(acc);
            },
            | Frame::FinishNullable => {
                let Some(mut summary) = summaries.pop()
                else {
                    continue;
                };
                summary.nullable = true;
                summaries.push(summary);
            },
        }
    }
    Ok(summaries.pop().unwrap_or_default())
}

/// Summarize one recursive-sort hole.
fn summary_for_sort(sort: Sort) -> Summary
{
    let mut first = BTreeSet::new();
    first.insert(sort);
    let mut last = BTreeSet::new();
    last.insert(sort);
    Summary {
        nullable: false,
        first,
        last,
    }
}

/// Reject an exposed sort boundary.
fn reject_adjacent(
    rule: RuleName,
    left: &BTreeSet<Sort>,
    right: &BTreeSet<Sort>,
) -> Result<(), PbgError>
{
    if let Some(left_sort) = left.iter().next().copied()
        && let Some(right_sort) = right.iter().next().copied()
    {
        return Err(PbgError::AdjacentSorts {
            rule: rule.0,
            left: left_sort,
            right: right_sort,
        });
    }
    Ok(())
}

/// Compute FIRST for a concatenation prefix extended by one item.
fn seq_first(
    left: &Summary,
    right: &Summary,
) -> BTreeSet<Sort>
{
    let mut first = left.first.clone();
    if left.nullable {
        first.extend(right.first.iter().copied());
    }
    first
}

/// Compute LAST for a concatenation prefix extended by one item.
fn seq_last(
    left: &Summary,
    right: &Summary,
) -> BTreeSet<Sort>
{
    let mut last = right.last.clone();
    if right.nullable {
        last.extend(left.last.iter().copied());
    }
    last
}
