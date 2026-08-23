//! The sharing-collapse probe: a test-only tally of what the checker actually
//! walked.
//!
//! # What it measures
//!
//! The term representation is a **graph** — a node's children are ids, so a
//! subterm named by two parents is stored once — and checking it without a
//! memo walks it as a **tree**, expanding every occurrence. This module counts
//! the two halves of that gap directly: **expansions**, the goals the machine
//! actually did work for, and **hits**, the goals a memo answered outright.
//!
//! Run the same checker at [`NullMemo`] and every goal is an expansion, which
//! is the tree count. Run it at the live memo and expansions fall to the
//! number of distinct supports, with the rest served as hits. The ratio of the
//! two expansion counts is the collapse.
//!
//! **Distinctness is not counted here.** The memo already knows how many
//! supports it answered ([`VerdictMemo::entry_count`]), and deriving the same
//! number twice from two mechanisms would let them disagree silently. The
//! probe counts work; the memo counts questions.
//!
//! # It changes no verdict
//!
//! Nothing here is consulted by a rule. The whole module is `cfg(test)` and is
//! absent from any build of the shipped crate; a tally asserts that this
//! process expanded a goal, and nothing about whether the goal holds.
//!
//! [`NullMemo`]: gandr_kernel_check_memo::NullMemo
//! [`VerdictMemo::entry_count`]: gandr_kernel_check_memo::VerdictMemo::entry_count

/// How many goals the machine did work for.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct ExpansionCount(u64);

impl From<ExpansionCount> for u64
{
    #[inline]
    fn from(value: ExpansionCount) -> Self
    {
        value.0
    }
}

/// How many goals a memo answered without work.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct HitCount(u64);

impl From<HitCount> for u64
{
    #[inline]
    fn from(value: HitCount) -> Self
    {
        value.0
    }
}

/// Whether one goal was expanded or served from the memo.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExpansionKind
{
    /// The machine read the node and applied a rule.
    Expanded,
    /// The memo answered; no child goal was entered.
    Recalled,
}

/// What one recording window observed.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ProbeReport
{
    /// Goals the machine did work for.
    expansions: ExpansionCount,
    /// Goals the memo answered.
    hits: HitCount,
}

impl ProbeReport
{
    /// Goals the machine did work for — the checker's real node count.
    #[inline]
    pub fn expansions(&self) -> ExpansionCount
    {
        self.expansions
    }

    /// Goals a memo answered without entering the node.
    #[inline]
    pub fn hits(&self) -> HitCount
    {
        self.hits
    }
}

std::thread_local! {
    /// This thread's tally, armed by [`begin`] and taken by [`end`].
    ///
    /// Absent means not recording, which is the state every test that does not
    /// measure runs in. Each test thread has its own, so a measuring test
    /// cannot see another test's goals.
    static TALLY: core::cell::Cell<Option<ProbeReport>> =
        const { core::cell::Cell::new(None) };
}

/// Arm the tally for this thread, discarding any previous window.
///
/// # Contract
/// - requires: nothing.
/// - ensures: subsequent [`record`] calls on this thread accumulate until
///   [`end`].
/// - provides: the start of a measurement window.
/// - fails: never.
/// - panics: none.
#[inline]
pub fn begin()
{
    TALLY.with(|cell| cell.set(Some(ProbeReport::default())));
}

/// Close this thread's window and report what it observed.
///
/// # Contract
/// - requires: nothing — an unarmed thread reports zeroes rather than failing,
///   so a caller that forgot [`begin`] gets a falsifiable answer instead of a
///   panic.
/// - ensures: the counts accumulated since [`begin`]; the tally is disarmed.
/// - provides: the measurement's result.
/// - fails: never.
/// - panics: none.
#[inline]
pub fn end() -> ProbeReport
{
    TALLY.with(|cell| cell.replace(None)).unwrap_or_default()
}

/// Tally one goal, when this thread is recording.
///
/// # Contract
/// - requires: nothing.
/// - ensures: the matching count rises by one when armed; nothing happens
///   otherwise.
/// - provides: the machine's observation seam. It reads and never answers: no
///   verdict, rule, or control decision consults it.
/// - fails: never.
/// - panics: none.
#[inline]
pub fn record(kind: ExpansionKind)
{
    TALLY.with(|cell| {
        if let Some(mut report) = cell.replace(None) {
            match kind {
                | ExpansionKind::Expanded => {
                    report.expansions = ExpansionCount(report.expansions.0.saturating_add(1));
                },
                | ExpansionKind::Recalled => {
                    report.hits = HitCount(report.hits.0.saturating_add(1));
                },
            }
            cell.set(Some(report));
        }
    });
}
