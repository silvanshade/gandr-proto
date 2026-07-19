//! Differential parity (`wyrd-46lo`): on arbitrary input, the normative Rust
//! front-end (`gandr_parser`) must agree with the tree-sitter grammar on both
//! equivalence relations, outside shell blocks (where the frozen PBG tokenizes
//! differently — see the grammar contract-fixtures manifest `parity` section):
//!
//! * E1 (token stream): the labeler's non-trivia token spans and tree-sitter's
//!   non-trivia leaf-token spans coincide, modulo trivia and the declared
//!   string-fragment collapse (tree-sitter's `(string)` node absorbs its
//!   content, so a labeler `string_fragment`/`single_quoted_content` has no
//!   leaf).
//! * E2 (highlight span): `gandr_grammar::highlight` and
//!   `gandr_tree_sitter::highlight` produce an identical `Vec<HlSpan>`.
//!
//! A source tree-sitter cannot parse cleanly (an `ERROR`/`MISSING` node — an
//! incomplete buffer under fuzzing) is a recovery case, not a parity seed, and
//! is skipped: parity is a relation over programs both parsers accept. The
//! seed corpus lives in `fuzz/corpus/parity/` (the base + fixture programs).

fn main()
{
    afl::fuzz!(|data: &[u8]| {
        let Ok(source) = core::str::from_utf8(data)
        else {
            return;
        };
        if u32::try_from(source.len()).is_err() {
            return;
        }

        // Tree-sitter reference. Skip recovery (error/missing) trees.
        let mut parser = tree_sitter::Parser::new();
        if parser
            .set_language(&gandr_tree_sitter::language::gandr())
            .is_err()
        {
            return;
        }
        let Some(tree) = parser.parse(source, None)
        else {
            return;
        };
        if tree.root_node().has_error() {
            return;
        }

        // Mold front-end.
        let Ok(pbg) = gandr_grammar::built_in()
        else {
            return;
        };
        let Ok(result) = gandr_parser::parse(&pbg, source)
        else {
            return;
        };
        // Skip inputs the mold parser only recovers (non-empty obligations):
        // parity is a relation over programs BOTH parsers fully accept. On such
        // well-formed programs the mold and tree-sitter shell-block boundaries
        // align; on malformed shell nesting they do not, and that is the
        // recovery domain.
        if !result.is_clean() {
            return;
        }
        let cst = result.cst();
        let Some(regions) = shell_regions(cst)
        else {
            return;
        };

        // E1: token-stream span agreement outside shell.
        let Some(mut labeler) = labeler_spans(source)
        else {
            return;
        };
        let Some(mut leaves) = tree_sitter_spans(&tree)
        else {
            return;
        };
        labeler = outside(labeler, &regions);
        leaves = outside(leaves, &regions);
        labeler.sort_unstable();
        leaves.sort_unstable();
        assert_eq!(labeler, leaves, "E1 token-stream parity (outside shell)");

        // E2: highlight-span agreement outside shell.
        let mold = gandr_grammar::highlight(&pbg, cst);
        let Ok(highlighter) = gandr_tree_sitter::highlight::Highlighter::new()
        else {
            return;
        };
        let ts = highlighter.spans(&tree, source);
        let Some(mold_bytes) = per_byte(&mold, source.len())
        else {
            return;
        };
        let Some(ts_bytes) = per_byte(&ts, source.len())
        else {
            return;
        };
        for byte in 0 .. source.len() {
            if in_shell(byte, &regions) {
                continue;
            }
            let Some(&ts_role) = ts_bytes.get(byte)
            else {
                return;
            };
            let Some(&mold_role) = mold_bytes.get(byte)
            else {
                return;
            };
            assert_eq!(
                ts_role, mold_role,
                "E2 highlight parity (outside shell) at byte {byte}"
            );
        }
    });
}

/// The labeler's non-trivia token spans, minus the string-content classes
/// tree-sitter's `(string)` node absorbs (a declared E1 collapse).
fn labeler_spans(source: &str) -> Option<Vec<(u32, u32)>>
{
    let mut spans = Vec::new();
    for token in gandr_parser::label(source) {
        if token.material == gandr_syntax::Material::Space
            || matches!(
                token.lexeme,
                gandr_parser::Lexeme::StringFragment | gandr_parser::Lexeme::SingleQuotedContent
            )
        {
            continue;
        }
        if token.start > token.end {
            return None;
        }
        spans.try_reserve(1).ok()?;
        spans.push((token.start, token.end));
    }
    Some(spans)
}

/// Tree-sitter leaf kinds that are atomic trivia (grammar.js `extras`).
const TS_TRIVIA: &[&str] = &["line_comment", "block_comment", "shebang"];

/// Tree-sitter's non-trivia leaf-token spans (comments/shebangs atomic).
fn tree_sitter_spans(tree: &tree_sitter::Tree) -> Option<Vec<(u32, u32)>>
{
    let mut spans = Vec::new();
    let mut worklist = vec![tree.root_node()];
    while let Some(node) = worklist.pop() {
        if TS_TRIVIA.contains(&node.kind()) {
            continue;
        }
        let child_count = node.child_count();
        if child_count == 0 {
            let start = u32::try_from(node.start_byte()).ok()?;
            let end = u32::try_from(node.end_byte()).ok()?;
            if start > end {
                return None;
            }
            if start < end {
                spans.try_reserve(1).ok()?;
                spans.push((start, end));
            }
            continue;
        }
        let child_count_u32 = u32::try_from(child_count).ok()?;
        worklist.try_reserve(child_count).ok()?;
        for index in (0 .. child_count_u32).rev() {
            let child = node.child(index)?;
            worklist.push(child);
        }
    }
    Some(spans)
}

/// Whether a tile text opens a shell block: `#!…{` / `$!…{` (an optional
/// dialect word between the lead and the brace; `gandr_parser`
/// `scan_shell_start`).
fn is_shell_opener(text: &str) -> bool
{
    (text.starts_with("#!") || text.starts_with("$!")) && text.ends_with('{')
}

/// Byte ranges of shell-block regions: the shell opener to its enclosing form's
/// end (or end-of-input for an unclosed block).
fn shell_regions(cst: &gandr_syntax::Cst) -> Option<Vec<(usize, usize)>>
{
    let mut regions = Vec::new();
    let mut worklist = vec![cst.root()];
    while let Some(id) = worklist.pop() {
        let view = cst.node(id).ok()?;
        if view.kind() == gandr_syntax::NodeKind::Token
            && view.text().ok().is_some_and(is_shell_opener)
        {
            let parent = view.parent()?;
            let parent_view = cst.node(parent).ok()?;
            let start = usize::try_from(view.range().start()).ok()?;
            let end = usize::try_from(parent_view.range().end()).ok()?;
            if start > end {
                return None;
            }
            regions.try_reserve(1).ok()?;
            regions.push((start, end));
        }
        let children = view.children().ok()?;
        worklist.try_reserve(children.len()).ok()?;
        for &child in children.iter().rev() {
            worklist.push(child);
        }
    }
    Some(regions)
}

/// Whether `byte` lies in any shell region.
fn in_shell(
    byte: usize,
    regions: &[(usize, usize)],
) -> bool
{
    regions
        .iter()
        .any(|&(low, high)| byte >= low && byte < high)
}

/// Keep only spans whose start is outside every shell region.
fn outside(
    mut spans: Vec<(u32, u32)>,
    regions: &[(usize, usize)],
) -> Vec<(u32, u32)>
{
    spans.retain(|&(start, _end)| {
        usize::try_from(start)
            .ok()
            .is_some_and(|byte| !in_shell(byte, regions))
    });
    spans
}

/// A per-byte role projection of a highlight-span list.
fn per_byte(
    spans: &[gandr_render_proto::present::HlSpan],
    len: usize,
) -> Option<Vec<Option<gandr_render_proto::present::HlRole>>>
{
    let mut map = Vec::new();
    map.try_reserve_exact(len).ok()?;
    map.resize(len, None);
    for span in spans {
        let start = span.range.start.min(len);
        let end = span.range.end.min(len);
        for byte in start .. end {
            let slot = map.get_mut(byte)?;
            *slot = Some(span.role);
        }
    }
    Some(map)
}
