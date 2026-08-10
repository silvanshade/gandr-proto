//! The inspection protocol (the sequent-machines design's §9, §4.1 —
//! consumers, and the store are IR data addressable by inspection).
//!
//! L0 has no store yet (that is L1), so the inspection surface here is the
//! static one the phase already affords: node-population [`Stats`], a
//! provenance histogram that un-sugars a focused term back to the source
//! constructs it came from, and a textual [`dump`] combining the
//! [`crate::pretty`] rendering with those figures — the "dumpable / debuggable"
//! view the gate and future goldens read.

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;

use crate::boundary::OriginCount;
use crate::boundary::RenderToken;
use crate::boundary::SequentNodeCount;
use crate::focus::FocusOrigin;
use crate::focus::Focused;
use crate::pretty;

/// The node population of a focused term's arena.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Stats
{
    /// The number of producer nodes.
    pub producers: SequentNodeCount,
    /// The number of consumer nodes.
    pub consumers: SequentNodeCount,
    /// The number of command nodes.
    pub commands: SequentNodeCount,
}

impl Stats
{
    /// The total node count across the three families.
    ///
    /// # Contract
    /// - ensures: the saturating sum of the three family counts (saturating so
    ///   the figure is total even at the astronomically unreachable `usize`
    ///   ceiling).
    #[inline]
    #[must_use]
    pub fn total(&self) -> SequentNodeCount
    {
        usize::from(self.producers)
            .saturating_add(usize::from(self.consumers))
            .saturating_add(usize::from(self.commands))
            .into()
    }
}

/// A histogram of the provenance of every created command — the un-sugaring
/// view (§3 `ElabKind`): how many commands each source construct contributed.
///
/// # Contract
/// - ensures: the values sum to the number of commands `𝓕` created (the cuts
///   and prim invocations), keyed by [`FocusOrigin`] in its declaration order.
#[inline]
#[must_use]
pub fn origin_histogram(focused: &Focused) -> BTreeMap<FocusOrigin, OriginCount>
{
    let mut histogram: BTreeMap<FocusOrigin, OriginCount> = BTreeMap::new();
    for origin in focused.origins().values() {
        let count = histogram.entry(*origin).or_default();
        *count = usize::from(*count).saturating_add(1_usize).into();
    }
    histogram
}

/// A textual dump of a focused term: its rendered root, node population, and
/// root provenance.
///
/// # Contract
/// - ensures: total; a self-contained multi-line debugging view.
#[inline]
#[must_use]
pub fn dump(focused: &Focused) -> String
{
    let population = stats(focused);
    let rendered = pretty::render_command(focused.arena(), focused.root());
    let origin = focused
        .origin(focused.root())
        .map_or_else(|| "(threaded)".into(), origin_label);
    format!(
        "root [{origin}]: {rendered}\nnodes: {} producers, {} consumers, {} commands ({} total)",
        population.producers,
        population.consumers,
        population.commands,
        population.total()
    )
}

/// The node population of a focused term.
///
/// # Contract
/// - ensures: the counts are exactly the arena's per-family lengths.
#[inline]
#[must_use]
pub fn stats(focused: &Focused) -> Stats
{
    let arena = focused.arena();
    Stats {
        producers: arena.producer_count(),
        consumers: arena.consumer_count(),
        commands: arena.command_count(),
    }
}

/// A stable one-word label for a provenance kind (avoids a `Debug` widening).
fn origin_label(origin: FocusOrigin) -> RenderToken<'static>
{
    match origin {
        | FocusOrigin::Ret => "ret",
        | FocusOrigin::Force => "force",
        | FocusOrigin::Abs => "abs",
        | FocusOrigin::Case => "case",
        | FocusOrigin::ListCase => "list-case",
        | FocusOrigin::Split => "split",
        | FocusOrigin::Walk => "walk",
        | FocusOrigin::DataCase => "data-case",
        | FocusOrigin::RecordProj => "record-proj",
        | FocusOrigin::With => "with",
        | FocusOrigin::Perform => "perform",
        | FocusOrigin::Shift => "shift",
        | FocusOrigin::Resume => "resume",
        | FocusOrigin::Handle => "handle",
        | FocusOrigin::Reset => "reset",
        | FocusOrigin::CompHole => "comp-hole",
        | FocusOrigin::Dup => "dup",
        | FocusOrigin::Drop => "drop",
        | FocusOrigin::Native => "native",
        | FocusOrigin::TopValue => "top-value",
        | FocusOrigin::Unsupported => "unsupported",
    }
    .into()
}

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::focus;

    /// Focusing `ret ()` yields one command, one producer, one consumer, tagged
    /// `ret`.
    #[test]
    fn stats_and_dump_report_a_terminal_cut()
    {
        let comp = gandr_core_checker::syntax::Comp::ret(gandr_core_checker::syntax::Value::Unit);
        let focused = focus::focus_comp(&comp).expect("focuses");
        let population = stats(&focused);
        assert_eq!(1, usize::from(population.commands), "one command");
        assert_eq!(1, usize::from(population.producers), "one producer");
        assert_eq!(1, usize::from(population.consumers), "one consumer");
        assert_eq!(3, usize::from(population.total()), "three nodes total");

        let histogram = origin_histogram(&focused);
        assert_eq!(
            Some(&1_usize.into()),
            histogram.get(&FocusOrigin::Ret),
            "the single command's provenance is ret"
        );

        let rendered = dump(&focused);
        assert!(
            rendered.contains("root [ret]:"),
            "dump names the root origin: {rendered}"
        );
        assert!(
            rendered.contains("3 total"),
            "dump reports the node total: {rendered}"
        );
    }
}
