//! The render-bus projection of the engine's report: the producer side of the
//! leaf wire crate's presentation seam.
//!
//! [`gandr_surface_render_remote`] parses nothing and types nothing — it holds
//! the plain data a renderer or a bus client consumes. Something has to fill
//! that data from the pipeline, and this module is where the filling lives for
//! the surfaces the engine can supply today: the parse's recovery obligation
//! rows, and the capability statement that says the bus carries them.
//!
//! # What this module deliberately does not do
//!
//! It does not build a whole [`ReportView`]. The other fields of that view —
//! highlights, marks, diagnostic and goal cards — are the local renderer's
//! projection, and the crate that owns it (`surface-render`, the in-tree
//! sibling of the `-remote` leaf) is not built. Filling them here would put
//! their spelling in the wrong crate; leaving them empty behind a
//! whole-view-shaped function would advertise a projection that does not exist.
//! So the seam is per-surface: a producer assembling a view takes the
//! obligation rows from [`obligation_cards`] and supplies the rest itself, and
//! [`server_caps`] states exactly which of those surfaces this crate backs.
//!
//! [`ReportView`]: gandr_surface_render_remote::wire::ReportView

use alloc::vec::Vec;

use gandr_surface_render_remote::present::ByteOffset;
use gandr_surface_render_remote::present::ObligationCard;
use gandr_surface_render_remote::wire::DeltaStreaming;
use gandr_surface_render_remote::wire::ObligationRows;
use gandr_surface_render_remote::wire::ServerCaps;
use gandr_surface_render_remote::wire::SessionBadges;

use crate::diag::Report;

/// Projects a report's obligation rows onto the render-bus cards.
///
/// The projection is total and lossless: one card per row, class and byte range
/// carried across unchanged, in the report's source order. The report is the
/// authority — this reads [`Report::obligations`] rather than the lowering or
/// the parse, so the bus and the agent-stream JSON cannot disagree about what a
/// source's obligations are.
///
/// # Contract
/// - requires: none.
/// - ensures: returns one [`ObligationCard`] per [`Report::obligations`] row,
///   preserving each row's class and exact byte span, in the report's order;
///   returns an empty vector exactly when the report has no obligations.
/// - provides: the live obligation-row production path into the render bus —
///   what [`server_caps`] advertises.
/// - fails: never.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 — a recovering source and a clean source separate the
///   populated and empty card set, and comparing each card against its report
///   row separates a preserved class/span pair from a dropped or re-spanned
///   one.
/// - witness: `diag_obligations::tests::bus::cards_preserve_the_report_rows`
/// - witness: `diag_obligations::tests::bus::a_clean_source_produces_no_cards`
#[inline]
#[must_use]
pub fn obligation_cards(report: &Report) -> Vec<ObligationCard>
{
    report
        .obligations
        .iter()
        .map(|row| {
            ObligationCard::new(
                row.class,
                ByteOffset::from(row.span.start) .. ByteOffset::from(row.span.end),
            )
        })
        .collect()
}

/// The render-bus capabilities a server built on this engine may advertise.
///
/// One flag is set, and it is set because the machinery behind it is in this
/// module: [`obligation_cards`] fills [`ReportView::obligations`], so
/// [`ServerCaps::obligations`] is true. Delta streaming and session badges have
/// no producer here, so they stay false — a capability is a claim about what
/// the server can send, and an unbacked claim costs a renderer a resync it
/// cannot satisfy.
///
/// Note that the flag is about the *path*, not about a document: a clean source
/// yields an empty row set through the same live path, and the server still
/// advertises the capability.
///
/// # Contract
/// - requires: none.
/// - ensures: returns capabilities at the crate's wire schema version with the
///   obligation-row flag set and the delta and session flags clear.
/// - provides: the `Hello`-frame capability statement for a bus server whose
///   projection comes from this crate.
/// - fails: never.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 — asserting the advertised flag together with rows actually
///   produced from a source separates an honest advertisement from a flag set
///   ahead of its machinery (the state this rung replaced) and from one left
///   clear behind it.
/// - witness: `diag_obligations::tests::bus::advertised_capabilities_match_the_live_path`
///
/// [`ReportView::obligations`]: gandr_surface_render_remote::wire::ReportView::obligations
#[inline]
#[must_use]
pub fn server_caps() -> ServerCaps
{
    ServerCaps::new(
        DeltaStreaming::from(false),
        ObligationRows::from(true),
        SessionBadges::from(false),
    )
}
