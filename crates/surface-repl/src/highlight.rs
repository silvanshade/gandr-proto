//! Highlight spans for a submitted buffer.
//!
//! The spans come from the grammar's normative mold-driven highlighter,
//! [`gandr_surface_grammar::highlight`] — the same producer the
//! language-server face reads. This module re-parses the submitted buffer to
//! obtain the committed CST that highlighter classifies; the session engine
//! parses for its own purposes and does not hand its tree back.
//!
//! The role vocabulary is
//! [`HlRole`](gandr_surface_render_remote::present::HlRole), shared through the
//! presentation seam. This crate consumes that vocabulary and defines no second
//! one: it never speaks the language server's integer token legend, and
//! `gandr-surface-repl` carries no dependency on `gandr-surface-lsp`.

use gandr_surface_grammar::highlight;
use gandr_surface_parser::parse;
use gandr_surface_render_remote::present::HlSpan;
use gandr_surface_syntax::SourceSlice;

use crate::completeness::grammar;

/// Classify `source` into highlight spans.
///
/// The spans are passed through exactly as the highlighter returns them — no
/// filter, sort, clamp, or split — so every face consuming this seam sees the
/// identical span sequence the language-server face sees.
///
/// # Contract
/// - ensures: returns the highlighter's spans for `source`; a grammar or commit
///   failure yields no spans rather than aborting the loop.
/// - provides: the transcript's highlight input, so the terminal painter has
///   something to paint.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 — a keyword-bearing submission is classified, and the
///   classified spans are sorted and pairwise disjoint as the producer's
///   contract states.
/// - witness: `gandr_surface_repl::highlight::tests::a_keyword_is_classified`
/// - witness: `gandr_surface_repl::highlight::tests::spans_are_sorted_and_disjoint`
/// - witness: `gandr_surface_repl::highlight::tests::the_disjointness_predicate_rejects_an_overlap`
#[inline]
#[must_use]
pub fn highlight_source(source: SourceSlice<'_>) -> Vec<HlSpan>
{
    let Ok(pbg) = grammar()
    else {
        return Vec::new();
    };
    let Ok(parsed) = parse(pbg, source)
    else {
        return Vec::new();
    };
    highlight(pbg, parsed.cst())
}

#[cfg(test)]
mod tests
{
    use gandr_surface_render_remote::present::ByteOffset;
    use gandr_surface_render_remote::present::HlRole;
    use gandr_surface_render_remote::present::HlSpan;
    use gandr_surface_syntax::SourceSlice;

    use super::highlight_source;

    #[test]
    fn a_keyword_is_classified()
    {
        let spans = highlight_source(SourceSlice::from("def one() -> F Integer { ret 1 }"));
        assert!(
            spans.iter().any(|span| span.role == HlRole::Keyword),
            "a def form carries a keyword span: {spans:?}"
        );
    }

    /// The first span that starts before the previous span ended, if any.
    ///
    /// Returning the offending span rather than a verdict keeps the failure
    /// message specific and keeps a primitive off the helper's signature.
    fn first_disorder(spans: &[HlSpan]) -> Option<HlSpan>
    {
        let mut cursor = 0_usize;
        for span in spans {
            if usize::from(span.range.start) < cursor {
                return Some(span.clone());
            }
            cursor = usize::from(span.range.end);
        }
        None
    }

    /// The producer's contract states sorted, disjoint spans; nothing in the
    /// producing crate witnesses it, and a witness written inside the
    /// producer's own picture could not refute it. This asserts the property
    /// from the consuming side, where a violation is an LSP protocol
    /// violation on the sibling face and a silently dropped span here.
    ///
    /// Unwiring the encoder does not exercise this witness's own clause — it
    /// kills it at the non-empty precondition instead — so the discriminating
    /// half is established directly by the negative control below rather than
    /// left unverified.
    #[test]
    fn spans_are_sorted_and_disjoint()
    {
        let spans = highlight_source(SourceSlice::from(
            r#"def one() -> F Integer { ret 1 } // trailing
val two = "text"
"#,
        ));
        assert!(!spans.is_empty(), "the fixture classifies something");
        assert_eq!(
            first_disorder(&spans),
            None,
            "the producer's spans are sorted and disjoint: {spans:?}"
        );
    }

    /// The negative control for the witness above: the predicate it asserts
    /// rejects an overlapping pair, so a green there is a statement about the
    /// spans rather than about a predicate that cannot fail.
    #[test]
    fn the_disjointness_predicate_rejects_an_overlap()
    {
        let overlapping = [
            HlSpan::new(ByteOffset::from(0) .. ByteOffset::from(5), HlRole::Keyword),
            HlSpan::new(ByteOffset::from(3) .. ByteOffset::from(8), HlRole::Variable),
        ];
        assert!(
            first_disorder(&overlapping).is_some(),
            "an overlap is rejected"
        );
        let unsorted = [
            HlSpan::new(ByteOffset::from(5) .. ByteOffset::from(8), HlRole::Keyword),
            HlSpan::new(ByteOffset::from(0) .. ByteOffset::from(3), HlRole::Variable),
        ];
        assert!(
            first_disorder(&unsorted).is_some(),
            "an inversion is rejected"
        );
    }

    #[test]
    fn an_unclassifiable_buffer_yields_no_panic()
    {
        drop(highlight_source(SourceSlice::from("@@@ !! nonsense")));
    }
}
