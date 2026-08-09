//! The surface front end as the checkpoint engine's parser-agnostic item source
//! (A2.3; `incremental-pipeline.md` §"Cold reparse" and §"The structural
//! diff").
//!
//! # What this module is for
//!
//! [`gandr_core_checker::region`] states the boundary between *producing* a
//! revision's top-level items and *consuming* them: the changed-region detector
//! and the checkpoint engine read only [`Item`] / [`Program`], never a concrete
//! parser. That module names no front end on purpose, so until something
//! implements [`ItemSource`] the boundary is a statement of intent that nothing
//! stands on.
//!
//! This module is the first implementation of it that is not a test double.
//! [`LoweringItemSource`] lowers a revision of surface source through the
//! melder push machine (`gandr-surface-parser`) and the CST → core lowering
//! ([`crate::lower`]), and hands the result across the seam as a [`Program`].
//! The engine on the other side depends on this crate for nothing: the
//! dependency runs one way, from here to the seam.
//!
//! # What crossing the seam drops, and why that is the point
//!
//! [`crate::lower::Lowered`] carries what the *surface* needs — an
//! [`crate::origin::OriginMap`], raw attributes, and the `extern` / `data` /
//! `codata` declaration tables — beside the items themselves. None of it
//! reaches the seam, because the unchanged-region test is structural equality
//! over an item's name, ascription, and lowered term, and equality over
//! anything parser-carried would make the detection parser-specific by
//! construction. A consumer that needs the surface faces of a revision holds
//! the [`crate::lower::Lowered`] it lowered; a consumer that needs only to know
//! *what changed* holds the [`Program`].
//!
//! [`Item`]: gandr_core_checker::region::Item
//! [`ItemSource`]: gandr_core_checker::region::ItemSource
//! [`Program`]: gandr_core_checker::region::Program

use alloc::string::String;

use gandr_core_checker::region::Item;
use gandr_core_checker::region::ItemSource;
use gandr_core_checker::region::Program;

use crate::boundary::PipelineSource;
use crate::lower::LowerError;
use crate::lower::LoweredItem;
use crate::lower::lower_source_total;

/// One revision of surface source, as the seam's opaque revision type.
///
/// [`ItemSource::Revision`] is the front end's own business, so it is this
/// crate that says what a revision is: the full text of a submission. The
/// engine never inspects it.
#[repr(transparent)]
#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SourceRevision(pub String);

impl From<String> for SourceRevision
{
    #[inline]
    fn from(value: String) -> Self
    {
        Self(value)
    }
}

impl From<&str> for SourceRevision
{
    #[inline]
    fn from(value: &str) -> Self
    {
        Self(String::from(value))
    }
}

/// The melder-and-lowering front end, as an [`ItemSource`].
///
/// Lowering runs in total mode, so every parseable revision yields a program:
/// an out-of-fragment construct or an error region becomes a hole carrying its
/// note, never a failure (`incremental-pipeline.md` §"Holes"). What survives as
/// an error is the input-independent residue the seam exists to carry — the
/// parser being unavailable, or the parse machinery itself failing.
///
/// This type is a unit: the front end holds no state across revisions, because
/// the state an incremental session accumulates lives in the checkpoint set,
/// not in the lowerer.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LoweringItemSource;

impl ItemSource for LoweringItemSource
{
    type Error = LowerError;
    type Revision = SourceRevision;

    /// Lowers one revision of surface source to its ordered top-level items.
    ///
    /// # Contract
    /// - requires: nothing of `revision`; every text is a revision, and an
    ///   unparseable one lowers to holes rather than failing.
    /// - ensures: the returned [`Program`] holds one [`Item`] per lowered
    ///   top-level item, in source order, carrying only the name, ascription,
    ///   and lowered term — no span, origin, or parser identity.
    /// - provides: the seam's first non-test-double implementation, so the
    ///   changed-region detector runs against the melder front end without
    ///   naming it.
    /// - fails: [`LowerError::ParserUnavailable`] and
    ///   [`LowerError::ParseFailed`] only — the two failures total lowering
    ///   does not recover, neither of which depends on the input text.
    /// - panics: none.
    ///
    /// # Errors
    ///
    /// Returns [`LowerError::ParserUnavailable`] when the grammar or parser
    /// cannot be built, and [`LowerError::ParseFailed`] when the parse machine
    /// itself fails. No input-dependent lowering failure surfaces here.
    ///
    /// # Adequacy
    /// - hypothesis: the seam projection is exactly the three-field item
    ///   identity and nothing else — a mutant that dropped the ascription, the
    ///   name, or the source order would let the changed-region detector adopt
    ///   an item it must re-type. `tests::seam` distinguishes these by driving
    ///   the core-checker engine over seam programs and asserting it agrees
    ///   item-for-item with this crate's own engine over the same source.
    /// - witness: `tests::seam::the_seam_engine_agrees_with_the_surface_engine`
    /// - witness: `tests::seam::the_seam_carries_names_ascriptions_and_order`
    #[inline]
    fn items(
        &self,
        revision: &Self::Revision,
    ) -> Result<Program, Self::Error>
    {
        let source = PipelineSource::from(revision.0.as_str());
        let lowered = lower_source_total(source)?;
        Ok(lowered.items.into_iter().map(seam_item).collect())
    }
}

/// Projects one lowered surface item onto the seam's parser-agnostic [`Item`].
///
/// The projection is a move of the three fields the unchanged-region test
/// compares; everything else a lowering knows about an item stays behind in
/// [`crate::lower::Lowered`].
fn seam_item(item: LoweredItem) -> Item
{
    Item::new(item.name, item.ascription, item.term)
}
