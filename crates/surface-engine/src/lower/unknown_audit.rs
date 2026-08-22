//! The gradual-unknown construction-site audit.
//!
//! `buildout-standing-02` closes with the rule this module enforces: **the
//! gradual `Unknown` is constructed at the raw-decode boundary only, never by
//! lowering, and its construction sites stay enumerable.** `gandr-x6t6` asks
//! for that enumeration as a deliverable, pinned by something that fails when
//! a new site appears.
//!
//! # Why the sources are embedded rather than read
//!
//! An audit that finds no unclassified sites and an audit that cannot see the
//! sources are the same green. Reading files at run time makes that failure
//! representable — a wrong relative path, a working directory that is not the
//! crate root, a file moved out from under the scan — and each one reports a
//! clean sweep. [`include_str!`] resolves at **compile time** against this
//! file's own directory, so a path that does not resolve is a build failure
//! rather than an empty scan. The blindness is unrepresentable instead of
//! guarded against, which is where the invariant ladder says it belongs.
//!
//! A positive control still runs, because compile-time inclusion proves the
//! text arrived and proves nothing about the scanner reading it.
//!
//! # What the audit measured, and the part that is not in `gandr-x6t6`
//!
//! The bead asks each site to be classified **as the raw-decode boundary or a
//! defect**. The tree needs a third answer and two of the five kinds below are
//! neither: a site may *copy* an unknown that already exists, constructing
//! nothing new, and a site may degrade from a real **author-side absence**,
//! which is the one degradation the standing entry permits.
//!
//! # The two keys, and why one of them is not enough
//!
//! A failure of this class has three representations in the lowering paths and
//! **no single key sees them all**:
//!
//! 1. a constructed `ValueType::Unknown`,
//! 2. a constructed `CompType::Unknown` — the sibling, invisible to any check
//!    keyed to the value sort alone,
//! 3. **no construction at all** — `lower.rs` skips an unlowerable case arm
//!    with `continue`, so the arm simply vanishes and the result is a shorter
//!    list that reads as a correct success everywhere.
//!
//! Keying on the **discarded error** finds the third and misses every site
//! that degrades from a `None` or an `unwrap_or` rather than from an `Err`.
//! Keying on the **construction** finds those and misses the third. The counts
//! are stated on [`SITES`]: this module owns the construction key, and the
//! sites only the discard key sees are named there so the gap is legible
//! rather than implied.

use alloc::collections::BTreeSet;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

/// What a site does with the gradual unknown.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum SiteKind
{
    /// The raw-decode boundary: the author wrote `?` and this is where the
    /// spelling becomes the type. The only construction the standing entry
    /// admits, and the reason the rule says *boundary only* rather than
    /// *never*.
    AuthorWritten,

    /// Pattern position. The site reads an unknown and builds none.
    Reads,

    /// The site copies an unknown that already exists — structural sharing, a
    /// cross-sort re-read of an author-written `?`. No new unknown enters the
    /// program, so the fact recorded is whatever the original site recorded.
    Propagation,

    /// A degradation from a real author-side absence: something optional the
    /// source did not write. This is `buildout-standing-02`'s first class and
    /// it is legitimate — the hole records a true fact about the source.
    Absence,

    /// **A defect by the standing entry.** An engine limit, a discarded error,
    /// or an unresolved internal lookup recorded as the gradual top. The
    /// author wrote a form; the engine could not read it; the unknown says the
    /// author left it open. Every site here is a candidate repair, and the
    /// repair belongs to `gandr-hxsj`'s slice two rather than to this audit.
    Degradation,
}

/// One classified construction site.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Site
{
    /// The source file, as spelled in [`SOURCES`].
    file: &'static str,
    /// Which occurrence of this exact text within the file, one-based.
    ///
    /// The key is text-plus-ordinal rather than either alone, and both
    /// alternatives were measured failing. **Text alone collapses siblings**:
    /// `lower/types.rs` carries three sites whose trimmed text is the
    /// identical `Ok(Ty::Value(ValueType::Unknown))` and whose classifications
    /// differ — one propagates a payload that was already unknown, two discard
    /// an error — so a text-keyed audit reads clean while classifying a third
    /// of what it claims to. **A line number is not stable enough to be a
    /// name**: adding one site shifted every site beneath it, so a single new
    /// construction reported eleven unclassified ones and buried the real one.
    ///
    /// The ordinal survives an unrelated insertion and moves only when another
    /// occurrence of the same text appears, which is exactly the edit whose
    /// classification nobody can infer.
    ordinal: usize,
    /// The exact source line, trimmed, as the scanner reports it.
    line: &'static str,
    /// What the site does.
    kind: SiteKind,
    /// Why it is classified that way, in one clause.
    because: &'static str,
}

/// The lowering sources this audit covers, embedded at compile time.
///
/// The audit's own source is deliberately **not** among them: it quotes every
/// pinned line verbatim, so scanning it would find this module's inventory as
/// a set of construction sites and the audit would report itself.
const SOURCES: &[(&str, &str)] = &[
    ("lower.rs", include_str!("../lower.rs")),
    ("lower/types.rs", include_str!("types.rs")),
    ("lower/data.rs", include_str!("data.rs")),
    ("lower/matrix.rs", include_str!("matrix.rs")),
];

/// Every gradual-unknown site in the lowering paths, classified.
///
/// **Ten sites are `Degradation` and each is a candidate defect.** Two more
/// exist that this key cannot see, because they construct nothing at all:
/// `lower.rs`'s two `Err(_) if total => continue` arms, which drop an
/// unlowerable case arm and a list case arm outright. Twelve distinct
/// degradation sites therefore exist across the two keys, and neither key
/// alone reaches more than ten.
const SITES: &[Site] = &[
    // ---- lower.rs ----
    Site {
        file: "lower.rs",
        ordinal: 1,
        line: "| node_kinds::HOLE => Ok(ValueType::Unknown),",
        kind: SiteKind::AuthorWritten,
        because: "a `?` the author typed is a legitimate axiom, not a recovery",
    },
    Site {
        file: "lower.rs",
        ordinal: 1,
        line: "| Ty::Value(ValueType::Unknown) if bool::from(self.total()) => Ok(ty),",
        kind: SiteKind::Reads,
        because: "matches an existing unknown and returns the type unchanged",
    },
    // ---- lower/types.rs ----
    Site {
        file: "lower/types.rs",
        ordinal: 1,
        line: "| node_kinds::NAME_UNKNOWN_TYPE => Ok(ValueType::Unknown),",
        kind: SiteKind::AuthorWritten,
        because: "the `Unknown` type keyword decoding to the gradual top",
    },
    Site {
        file: "lower/types.rs",
        ordinal: 1,
        line: "return Ok(Ty::Value(ValueType::Unknown));",
        kind: SiteKind::Degradation,
        because: "the blanket total-mode arm of `lower_ty_manifest`: every named \
                  decline in the tree dies here, unread (gandr-hxsj, gandr-zsqp)",
    },
    Site {
        file: "lower/types.rs",
        ordinal: 1,
        line: "results.push(Ok(Ty::Value(ValueType::Unknown)));",
        kind: SiteKind::AuthorWritten,
        because: "the `?` atom node lowering to the sort-free value default",
    },
    Site {
        file: "lower/types.rs",
        ordinal: 1,
        line: ".map(|_| Ty::Value(ValueType::Unknown)),",
        kind: SiteKind::Reads,
        because: "reached only where the field is absent, so the `map` never runs \
                  and the pushed result is the error; the unknown is a type ascription",
    },
    Site {
        file: "lower/types.rs",
        ordinal: 1,
        line: "| ValueType::Unknown if matches!(strictness, Strictness::Total) => {",
        kind: SiteKind::Reads,
        because: "the guard on the package-payload arm below",
    },
    Site {
        file: "lower/types.rs",
        ordinal: 1,
        line: "Ok(Ty::Value(ValueType::Unknown))",
        kind: SiteKind::Propagation,
        because: "a package payload that is already unknown stays unknown",
    },
    Site {
        file: "lower/types.rs",
        ordinal: 2,
        line: "Ok(Ty::Value(ValueType::Unknown))",
        kind: SiteKind::Degradation,
        because: "the package-type assembly discards its error under totality; \
                  identical in text to the propagation three lines above and \
                  opposite in meaning",
    },
    Site {
        file: "lower/types.rs",
        ordinal: 3,
        line: "Ok(Ty::Value(ValueType::Unknown))",
        kind: SiteKind::Degradation,
        because: "the member-record assembly discards its error under totality, \
                  by an `or_else` that no search for a discarded `Err(_)` finds",
    },
    Site {
        file: "lower/types.rs",
        ordinal: 1,
        line: "| Ok(_) | Err(_) if total => Ok(ValueType::Unknown),",
        kind: SiteKind::Degradation,
        because: "`value_result` swallows every error including the `TypeSortMismatch` \
                  the identity type's carrier already raises correctly (gandr-zsqp)",
    },
    Site {
        file: "lower/types.rs",
        ordinal: 1,
        line: "| Ok(_) | Err(_) if total => Ok(CompType::Unknown),",
        kind: SiteKind::Degradation,
        because: "`comp_result`, the computation-sorted sibling of the line above",
    },
    Site {
        file: "lower/types.rs",
        ordinal: 1,
        line: "| Ok(Ty::Value(ValueType::Unknown)) if is_unknown_atom(node).0 => Ok(CompType::Unknown),",
        kind: SiteKind::Propagation,
        because: "an author-written `?` re-read at the computation sort",
    },
    // ---- lower/data.rs ----
    Site {
        file: "lower/data.rs",
        ordinal: 1,
        line: "alloc::vec![ValueType::Unknown; usize::from(arity)]",
        kind: SiteKind::Degradation,
        because: "no expected arguments were available, so every field type of the \
                  constructor becomes the gradual top at once",
    },
    Site {
        file: "lower/data.rs",
        ordinal: 1,
        line: "| _ => ValueType::Unknown,",
        kind: SiteKind::Degradation,
        because: "a wildcard over description codes: any code that is not a field \
                  degrades without naming what it was",
    },
    Site {
        file: "lower/data.rs",
        ordinal: 1,
        line: ".unwrap_or(ValueType::Unknown),",
        kind: SiteKind::Degradation,
        because: "a type parameter that resolves to no argument position degrades \
                  rather than declining",
    },
    Site {
        file: "lower/data.rs",
        ordinal: 1,
        line: "| None => ValueType::Unknown,",
        kind: SiteKind::Degradation,
        because: "an applied non-declared former in a field is documented as `out of \
                  this first cut, interpreted permissively` — a capability boundary \
                  written down as the gradual top, which is the exact substitution \
                  buildout-standing-02 forbids",
    },
    Site {
        file: "lower/data.rs",
        ordinal: 1,
        line: "| PrimTy::Unknown => ValueType::Unknown,",
        kind: SiteKind::AuthorWritten,
        because: "the decoded `?` primitive mapping to the gradual top",
    },
    // ---- lower/matrix.rs ----
    Site {
        file: "lower/matrix.rs",
        ordinal: 1,
        line: "ty = CompType::arrow(ValueType::Unknown, ty);",
        kind: SiteKind::Degradation,
        because: "each case-arm binder's domain is built as the gradual top rather \
                  than recovered, so an arm's parameter type is never checked",
    },
    Site {
        file: "lower/matrix.rs",
        ordinal: 1,
        line: ".unwrap_or(CompType::Unknown);",
        kind: SiteKind::Absence,
        because: "the eliminator's `-> T` answer annotation is optional, so its \
                  absence is the author's and the unknown records a true fact",
    },
];

/// Extracts every gradual-unknown mention from a source, ignoring comments.
///
/// A doc comment and a line comment both begin `//` once trimmed, so one test
/// excludes prose without excluding code that merely trails a comment.
fn mentions(text: &str) -> Vec<(usize, String)>
{
    let mut seen: alloc::collections::BTreeMap<String, usize> = alloc::collections::BTreeMap::new();
    text.lines()
        .map(str::trim)
        .filter(|line| !line.starts_with("//"))
        .filter(|line| line.contains("ValueType::Unknown") || line.contains("CompType::Unknown"))
        .map(|line| {
            let text = String::from(line);
            let ordinal = seen.entry(text.clone()).or_insert(0);
            *ordinal = ordinal.saturating_add(1);
            (*ordinal, text)
        })
        .collect()
}

#[cfg(test)]
mod tests
{
    use super::*;

    /// The scanner can see the sources it was given.
    ///
    /// A pinned inventory that matches an empty scan is a clean sweep, and an
    /// empty scan is what every way of getting the sources wrong produces.
    /// `include_str!` rules out an unresolved path at compile time; this rules
    /// out a scanner that reads the text and matches nothing in it.
    #[test]
    fn the_audit_can_see_the_sites_it_claims_to_classify() -> Result<(), String>
    {
        for &(file, text) in SOURCES {
            assert!(
                !text.is_empty(),
                "{file} was included as empty text, so any sweep over it is vacuous"
            );
            let found = mentions(text);
            assert!(
                !found.is_empty(),
                "the scanner found no gradual-unknown line in {file}, which is \
                 indistinguishable from a file that has none; every source listed \
                 here was measured to contain at least one"
            );
        }
        let types = SOURCES
            .iter()
            .find(|&&(file, _)| file == "lower/types.rs")
            .ok_or_else(|| String::from("lower/types.rs must be among the audited sources"))?;
        assert!(
            mentions(types.1).iter().any(|&(ordinal, ref line)| {
                ordinal == 1 && line == "return Ok(Ty::Value(ValueType::Unknown));"
            }),
            "the scanner must find the blanket total-mode arm of `lower_ty_manifest`, \
             the site this whole audit exists for; not finding it means the scanner \
             is blind rather than that the tree is clean"
        );
        Ok(())
    }

    /// Every site in the lowering paths is classified, and nothing is pinned
    /// that the tree no longer contains.
    ///
    /// This is the enumeration `gandr-x6t6` asks for. It fails in **both**
    /// directions on purpose: an unpinned line is a site somebody added
    /// without answering what fact it records, and a pinned line the scanner
    /// cannot find is an inventory describing a tree that has moved.
    #[test]
    fn every_gradual_unknown_site_is_enumerated_and_classified()
    {
        for &(file, text) in SOURCES {
            let found: BTreeSet<(usize, String)> = mentions(text).into_iter().collect();
            let pinned: BTreeSet<(usize, String)> = SITES
                .iter()
                .filter(|site| site.file == file)
                .map(|site| (site.ordinal, String::from(site.line)))
                .collect();

            let unpinned: Vec<&(usize, String)> = found.difference(&pinned).collect();
            assert!(
                unpinned.is_empty(),
                "{file} constructs or reads the gradual unknown at a site this audit \
                 does not classify: {unpinned:?}. Add it to SITES with the fact it \
                 records — buildout-standing-02 admits construction at the raw-decode \
                 boundary only."
            );

            let stale: Vec<&(usize, String)> = pinned.difference(&found).collect();
            assert!(
                stale.is_empty(),
                "{file} no longer contains sites this audit pins: {stale:?}. \
                 An inventory that outlives its sites describes a tree that moved."
            );
        }
    }

    /// The audit's own count of defects, stated so a repair is visible as one.
    ///
    /// Slice two of `gandr-hxsj` repairs these, and each repair moves a site
    /// out of `Degradation`. Pinning the count is what makes that movement
    /// show up as a deliberate edit here rather than as silence.
    #[test]
    fn ten_sites_degrade_a_written_form_into_the_gradual_top()
    {
        let degradations: Vec<&Site> = SITES
            .iter()
            .filter(|site| site.kind == SiteKind::Degradation)
            .collect();
        let named: Vec<String> = degradations
            .iter()
            .map(|site| format!("{} #{} {}", site.file, site.ordinal, site.line))
            .collect();
        assert_eq!(
            degradations.len(),
            10,
            "the audit classifies ten construction sites as degradations; \
             it now finds {}: {named:#?}",
            degradations.len()
        );
    }

    /// The raw-decode boundary is where the author's `?` becomes a type, and
    /// there are four such sites.
    ///
    /// Stated separately from the degradation count because the two move for
    /// opposite reasons: a new boundary site is a new surface spelling for the
    /// gradual top, while a new degradation is a defect.
    #[test]
    fn the_raw_decode_boundary_has_four_sites()
    {
        let boundary: Vec<&Site> = SITES
            .iter()
            .filter(|site| site.kind == SiteKind::AuthorWritten)
            .collect();
        let named: Vec<String> = boundary
            .iter()
            .map(|site| format!("{} #{} {}", site.file, site.ordinal, site.line))
            .collect();
        assert_eq!(
            boundary.len(),
            4,
            "four sites decode an author-written unknown; it now finds {}: {named:#?}",
            boundary.len()
        );
    }
}
