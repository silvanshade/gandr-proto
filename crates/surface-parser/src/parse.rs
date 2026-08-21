//! The batch parser: `labeler ∘ molder ∘ fold(push) ∘ commit(finalize)`.
//!
//! [`parse`](fn@crate::parse) is the whole front-end composed:
//! it labels the source, molds each
//! token to its obligation-minimizing [`gandr_surface_syntax::MoldId`], folds
//! the molded stream through the melder's `push`, records trivia for
//! losslessness, and commits the batch [`Cst`]. The result carries the buffered
//! obligations beside the tree so the obligation surface is never lost — a
//! total function: any [`SourceSlice`] yields a well-formed [`ParseResult`],
//! never a panic.

use alloc::vec::Vec;

use gandr_surface_grammar::Pbg;
use gandr_surface_syntax::ClosingClass;
use gandr_surface_syntax::Cst;
use gandr_surface_syntax::MoldPayload;
use gandr_surface_syntax::NodeId;
use gandr_surface_syntax::NodeKind;
use gandr_surface_syntax::SourceSlice;

use crate::MeldError;
use crate::MeldState;
use crate::Molder;
use crate::label::label;
use crate::mold::SourceText;
use crate::oblig::ObligationInstance;

/// Whether a batch parse produced no obligations.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ParseCleanStatus(bool);

impl From<bool> for ParseCleanStatus
{
    #[inline]
    fn from(is_clean: bool) -> Self
    {
        Self(is_clean)
    }
}

impl From<ParseCleanStatus> for bool
{
    #[inline]
    fn from(is_clean: ParseCleanStatus) -> Self
    {
        is_clean.0
    }
}

/// The result of a batch parse: the committed tree and its obligations.
///
/// The obligations are the melder's buffer at commit — every convex/ghost/
/// incomparable repair the parse made — severity-ordered by
/// [`obligations`](ParseResult::obligations) so consumers render the most
/// serious first. A clean parse has an empty obligation slice.
///
/// # Contract
/// - requires: constructed by [`parse`](fn@crate::parse).
/// - ensures: preserves the committed [`Cst`] and the parse's obligations
///   exactly; [`obligations`](ParseResult::obligations) is severity-ordered
///   (highest first) then by span.
/// - provides: the batch parse's carrier so obligations are not lost.
/// - fails: never.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 — a clean parse and an obligation-bearing parse distinguish
///   the empty and non-empty obligation slice.
/// - witness: `gandr_surface_parser::parse::tests::corpus_parses_with_zero_obligations`
#[derive(Clone, Debug)]
pub struct ParseResult
{
    /// The committed concrete syntax tree.
    cst: Cst,
    /// The parse's obligations, severity-ordered (highest first) then by span.
    obligations: Vec<ObligationInstance>,
}

impl ParseResult
{
    /// Return the committed concrete syntax tree.
    #[inline]
    #[must_use]
    pub const fn cst(&self) -> &Cst
    {
        &self.cst
    }

    /// Return the parse's obligations, severity-ordered (highest first).
    ///
    /// # Contract
    /// - requires: none.
    /// - ensures: returns the buffered obligations sorted by descending
    ///   [`crate::Oblig`] severity, then ascending span.
    /// - provides: the batch obligation surface.
    /// - fails: never.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — a mixed-severity parse observes the descending order.
    /// - witness: `gandr_surface_parser::parse::tests::obligations_are_severity_ordered`
    #[inline]
    #[must_use]
    pub fn obligations(&self) -> &[ObligationInstance]
    {
        &self.obligations
    }

    /// Return whether the parse produced no obligations (a clean parse).
    #[inline]
    #[must_use]
    pub fn is_clean(&self) -> ParseCleanStatus
    {
        ParseCleanStatus::from(self.obligations.is_empty())
    }

    /// Return the class of every minted close in the whole tree, in tree
    /// order.
    ///
    /// A ghost's class is **carried**, never reconstructed: the melder records
    /// it when it mints the ghost, from the form-level closing class the
    /// grammar derives, and a ghost whose form has no single such class carries
    /// none. Reading the sequence back is what distinguishes *no ghost was
    /// minted* from *a ghost was minted and carried no class* — which a repair
    /// witness must tell apart, because only the second says the grammar could
    /// not name the closer.
    ///
    /// # Contract
    /// - requires: none.
    /// - ensures: returns one entry per [`MoldPayload::GhostClose`] token, in
    ///   tree order; unclassed ghosts contribute nothing.
    /// - provides: the minted-close surface a witness reads directly.
    /// - fails: never.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — a two-class repair, an unclassed repair, and a clean
    ///   parse distinguish the sequence.
    /// - witness: `gandr_surface_parser::acceptance::a_repaired_container_keeps_its_member`
    #[inline]
    #[must_use]
    pub fn minted_close_classes(&self) -> Vec<ClosingClass>
    {
        self.minted_closes(self.cst.root())
    }

    /// Collect the carried class of every minted close in the subtree at `id`.
    #[inline]
    fn minted_closes(
        &self,
        id: NodeId,
    ) -> Vec<ClosingClass>
    {
        let mut classes = Vec::new();
        let mut pending = vec![id];
        while let Some(next) = pending.pop() {
            let Ok(view) = self.cst.node(next)
            else {
                continue;
            };
            if view.kind() == NodeKind::Token
                && let MoldPayload::GhostClose { class, .. } = view.payload()
            {
                classes.push(class);
            }
            let children = view.children().unwrap_or(&[]);
            pending.extend(children.iter().rev().copied());
        }
        classes
    }

    /// Consume the result and return the committed tree.
    #[inline]
    #[must_use]
    pub fn into_cst(self) -> Cst
    {
        self.cst
    }
}

/// Batch-parse `src` over the checked grammar `pbg`.
///
/// The whole front-end, composed: label → mold → fold(push) → commit. Trivia
/// are recorded for losslessness (the committed tree reconstructs the source),
/// and the obligations ride the result.
///
/// # Contract
/// - requires: `pbg` is a checked PBG; `src` is any UTF-8 source slice.
/// - ensures: returns a well-formed [`Cst`] recording `pbg`'s fingerprint plus
///   the parse's severity-ordered obligations; the committed tree's leaf spans,
///   ordered by offset, reconstruct `src` (losslessness — trivia are recorded
///   as [`gandr_surface_syntax::Material::Space`] leaves, hash-skipped but
///   byte-preserving); total over every input.
/// - provides: `pub fn parse` — the batch parse entry point (proposal §4.1).
/// - fails: returns [`MeldError`] only for an arena-construction failure at
///   commit (arena size or coordinate overflow), never for ungrammatical input.
/// - panics: none.
/// - intension: labels are folded in source order; each non-space token is
///   molded by the obligation-minimizing [`Molder`]; space tokens are recorded
///   verbatim.
///
/// # Errors
/// Returns [`MeldError::Build`] when the flat arena cannot be assembled.
///
/// # Adequacy
/// - hypothesis: L4 — the corpus (zero obligations), arbitrary byte soup
///   (totality), incomplete input (statement-local obligations), and
///   losslessness each exercise a distinct property.
/// - witness: `gandr_surface_parser::parse::tests::corpus_parses_with_zero_obligations`
/// - witness: `gandr_surface_parser::parse::tests::parse_is_lossless_and_hash_stable`
#[inline]
pub fn parse(
    pbg: &Pbg,
    src: SourceSlice<'_>,
) -> Result<ParseResult, MeldError>
{
    let source_text = src.as_ref();
    let mut molder = Molder::new(pbg);
    let mut state = MeldState::new(pbg);
    let tokens = label(src);
    molder.mold_stream(&mut state, &tokens, SourceText::from(source_text));
    // `commit_with_obligations` captures the completion's repairs (force-close
    // and missing-operand obligations flagged while closing the input), which a
    // bare `commit` would drop.
    let (cst, mut obligations) = state.commit_with_obligations()?;
    // Severity-ordered (highest first) then by span, so consumers render the
    // most serious repair first.
    obligations.sort_by(|a, b| {
        b.class
            .cmp(&a.class)
            .then_with(|| a.span.start().cmp(&b.span.start()))
            .then_with(|| a.span.end().cmp(&b.span.end()))
    });
    Ok(ParseResult { cst, obligations })
}

#[cfg(test)]
mod tests
{
    use core::error::Error;
    use core::fmt;
    use std::path::Path;
    use std::path::PathBuf;

    use gandr_surface_grammar::Pbg;
    use gandr_surface_grammar::built_in;
    use gandr_surface_syntax::Cst;
    use gandr_surface_syntax::NodeId;
    use gandr_surface_syntax::NodeKind;
    use gandr_surface_syntax::SourceSlice;
    use gandr_surface_syntax::TextOffset;
    use proptest::prelude::*;

    use super::parse;

    /// File-read failure with the corpus path preserved.
    #[derive(Debug)]
    struct ReadSourceError
    {
        path: PathBuf,
        source: std::io::Error,
    }

    impl fmt::Display for ReadSourceError
    {
        fn fmt(
            &self,
            f: &mut fmt::Formatter<'_>,
        ) -> fmt::Result
        {
            write!(f, "failed to read {}: {}", self.path.display(), self.source)
        }
    }

    impl Error for ReadSourceError
    {
        fn source(&self) -> Option<&(dyn Error + 'static)>
        {
            Some(&self.source)
        }
    }

    #[test]
    fn corpus_parses_totally() -> Result<(), Box<dyn Error>>
    {
        // Every committed corpus program parses totally — no panic, a
        // well-formed CST recording the grammar fingerprint. (The stronger
        // ZERO-obligation gate is `acceptance::corpus_molds_to_zero_obligations`
        // — this test is the weaker totality + losslessness floor.)
        let pbg = built_in()?;
        let mut files = Vec::new();
        gandr_files(
            &workspace_root().join("crates/surface-corpus/examples"),
            &mut files,
        );
        files.sort();
        assert!(!files.is_empty(), "the corpus is populated");

        for path in &files {
            let src = read_source(path)?;
            let result = parse(&pbg, SourceSlice::from(src.as_str()))?;
            assert_eq!(result.cst().grammar_fingerprint(), pbg.fingerprint());
            let mut rebuilt = String::new();
            collect_leaves_ordered(result.cst(), result.cst().root(), &mut rebuilt)?;
            assert_eq!(
                &rebuilt,
                &src,
                "{:?} reconstructs losslessly",
                path.file_name()
            );
        }
        Ok(())
    }
    /// Collect every `.gandr` file under `dir`.
    fn gandr_files(
        dir: &Path,
        out: &mut Vec<PathBuf>,
    )
    {
        let mut pending = vec![dir.to_path_buf()];
        while let Some(next_dir) = pending.pop() {
            if let Ok(entries) = std::fs::read_dir(next_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        pending.push(path);
                    }
                    else if path.extension().is_some_and(|ext| ext == "gandr") {
                        out.push(path);
                    }
                }
            }
        }
    }
    /// The workspace root (two parents up from this crate manifest).
    fn workspace_root() -> PathBuf
    {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .map(Path::to_path_buf)
            .expect("workspace root")
    }

    /// Read a source file while retaining path context in the error.
    fn read_source(path: &Path) -> Result<String, ReadSourceError>
    {
        std::fs::read_to_string(path).map_err(|source| ReadSourceError {
            path: path.to_path_buf(),
            source,
        })
    }

    /// Reconstruct the source (offset-ordered leaves) into `out`.
    fn collect_leaves_ordered(
        cst: &Cst,
        root: NodeId,
        out: &mut String,
    ) -> Result<(), Box<dyn Error>>
    {
        let reconstructed = reconstruct(cst, root)?;
        out.push_str(&reconstructed);
        Ok(())
    }
    #[test]
    fn parse_is_lossless_and_hash_stable() -> Result<(), Box<dyn Error>>
    {
        // Losslessness: the committed tree's tile + space leaves reconstruct the
        // exact source, including interleaved trivia.
        let pbg = built_in()?;
        let src = "// a comment\ndef greeting = \"hi\";\n\nret greeting\n";
        let result = parse(&pbg, SourceSlice::from(src))?;
        let rebuilt = reconstruct(result.cst(), result.cst().root())?;
        assert_eq!(rebuilt, src, "the committed tree reconstructs the source");

        // Determinism: a second parse hashes identically.
        let again = parse(&pbg, SourceSlice::from(src))?;
        let result_hash = result.cst().hash(result.cst().root())?;
        let again_hash = again.cst().hash(again.cst().root())?;
        assert_eq!(result_hash, again_hash);
        Ok(())
    }

    #[test]
    fn declaration_prefix_round_trips_and_snapshots_model_state() -> Result<(), Box<dyn Error>>
    {
        let pbg = built_in()?;
        let src = r#"def nth @[A : Type, i : Fin(length(xs))] (xs : List(A)) -> A {
  ret xs
}
"#;
        let result = parse(&pbg, SourceSlice::from(src))?;
        assert!(
            result.obligations().is_empty(),
            "the declaration prefix has no parse recovery: {:?}",
            result.obligations()
        );
        assert_eq!(
            result.cst().grammar_fingerprint(),
            pbg.fingerprint(),
            "the CST records the grammar snapshot that produced it"
        );
        assert_eq!(
            reconstruct(result.cst(), result.cst().root())?,
            src,
            "the surface model round-trips the raw multiline source"
        );

        let again = parse(&pbg, SourceSlice::from(src))?;
        assert_eq!(
            result.cst().hash(result.cst().root())?,
            again.cst().hash(again.cst().root())?,
            "the declaration-prefix model snapshot is deterministic"
        );
        Ok(())
    }
    /// A committed leaf span with its source bytes.
    #[derive(Clone, Debug, Eq, PartialEq)]
    struct LeafText
    {
        start: TextOffset,
        end: TextOffset,
        text: String,
    }

    /// Reconstruct the source from the committed tree's leaf spans, ordered by
    /// offset (the losslessness contract: every byte is a leaf with a correct
    /// range; grout leaves are empty and contribute nothing).
    fn reconstruct(
        cst: &Cst,
        root: NodeId,
    ) -> Result<String, Box<dyn Error>>
    {
        let mut leaves = Vec::new();
        collect_leaves(cst, root, &mut leaves)?;
        leaves.sort_by_key(|leaf: &LeafText| (leaf.start, leaf.end));
        let mut out = String::new();
        for leaf in leaves {
            out.push_str(&leaf.text);
        }
        Ok(out)
    }

    /// Collect every token leaf's `(start, end, text)`. Empty-range grout
    /// (convex/ghost repair) contributes nothing; an unmolded token carries its
    /// exact bytes, so all token leaves are byte-preserving.
    fn collect_leaves(
        cst: &Cst,
        id: NodeId,
        out: &mut Vec<LeafText>,
    ) -> Result<(), Box<dyn Error>>
    {
        let mut pending = vec![id];
        while let Some(next) = pending.pop() {
            let view = cst.node(next)?;
            if view.kind() == NodeKind::Token {
                out.push(LeafText {
                    start: view.range().start(),
                    end: view.range().end(),
                    text: {
                        let text = view.text()?;
                        text.as_ref().to_owned()
                    },
                });
            }
            else {
                let children = view.children()?;
                pending.extend(children.iter().rev().copied());
            }
        }
        Ok(())
    }

    /// A biased strategy: byte soup, corpus mutations, and truncations.
    fn hostile_source() -> impl Strategy<Value = String>
    {
        // A pool of gandr fragments to mutate and truncate.
        let fragments = vec![
            "def x = 1;",
            "fn(a) { ret a }",
            "case v { Inl(x) => x, Inr(y) => y }",
            "if c { ret 1 } else { ret 2 }",
            "#{ a = 1, b = \"s\" }",
            "[1, 2, 3] ++ [4]",
            "#!{ echo hi; }",
            "@[doc(\"d\")] def y = 2;",
        ];
        prop_oneof![
            // Byte soup.
            prop::collection::vec(any::<u8>(), 0 .. 64)
                .prop_map(|bytes| String::from_utf8_lossy(&bytes).into_owned()),
            // A fragment, possibly truncated.
            (prop::sample::select(fragments), 0_usize .. 40).prop_map(|(frag, cut)| {
                let end = cut.min(frag.len());
                frag.get(.. end).unwrap_or(frag).to_owned()
            }),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(400))]

        #[test]
        fn arbitrary_source_parses_totally(src in hostile_source()) {
            use std::sync::OnceLock;
            static PBG: OnceLock<Pbg> = OnceLock::new();
            let pbg = PBG.get_or_init(|| built_in().expect("built-in grammar"));

            // Parse never panics and always yields a well-formed CST.
            let parse_result = parse(pbg, SourceSlice::from(src.as_str())).expect("commit is total");
            prop_assert_eq!(parse_result.cst().grammar_fingerprint(), pbg.fingerprint());

            // Losslessness holds over arbitrary input.
            let rebuilt = reconstruct(parse_result.cst(), parse_result.cst().root()).expect("well-formed");
            prop_assert_eq!(rebuilt, src);
        }
    }
}
