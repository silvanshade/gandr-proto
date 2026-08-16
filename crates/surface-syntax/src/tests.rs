use core::error::Error;
use std::io;
use std::process::Command;

use fixture_support::assert_build_error;
use fixture_support::changed_first_identifier_mold_tree;
use fixture_support::child_probe_hash;
use fixture_support::fixed_tree;
use fixture_support::ghost_close_token_hash;
use fixture_support::grout_token_hash;
use fixture_support::ident;
use fixture_support::kw_let;
use fixture_support::range;
use fixture_support::reindented_tree;
use fixture_support::space;
use fixture_support::tile;

use super::BuildError;
use super::ClosingClass;
use super::Cst;
use super::GrammarFingerprint;
use super::GroutShape;
use super::GroutSort;
use super::Material;
use super::MoldId;
use super::MoldPayload;
use super::NodeCount;
use super::NodeId;
use super::NodeKind;
use super::NodeSlot;
use super::SourceText;
use super::StableHash;
use super::TextOffset;
use super::TextRange;
use super::builder::CstBuilder;
use super::diff::diff;
use super::model::NodeData;

type TestResult = Result<(), Box<dyn Error>>;

const GOLDEN_ROOT_HASH: StableHash = StableHash(0x7aeb_e5ab_8d67_7764);
const HASH_PROBE_ENV: &str = "GANDR_SYNTAX_HASH_PROBE";
const HASH_PROBE_PREFIX: &str = "GANDR_SYNTAX_HASH_PROBE=";

/// Fingerprint of the fixture grammar whose mold table these ids index.
const GRAMMAR_FP: GrammarFingerprint = GrammarFingerprint(0x1234_5678_9abc_def0);

const IDENT_MOLD_ID: u32 = 11;
const EQ_MOLD_ID: u32 = 12;
const NUMBER_MOLD_ID: u32 = 13;
const CHANGED_IDENT_MOLD_ID: u32 = 99;

#[test]
fn non_grout_token_leaves_round_trip_source_bytes() -> TestResult
{
    let tree = fixed_tree()?;
    let mut token_texts = Vec::new();

    for slot in tree.cst.len().slots()? {
        let id = NodeId::from_raw(slot);
        let view = tree.cst.node(id)?;
        if view.kind() == NodeKind::Token && view.material() != Material::Grout {
            token_texts.push((view.range().start(), view.range().end(), {
                let text = view.text()?;
                text.as_ref().to_owned()
            }));
        }
    }

    token_texts.sort_by_key(|&(start, end, _)| (start, end));
    let concatenated = token_texts
        .into_iter()
        .map(|(_start, _end, text)| text)
        .collect::<String>();

    assert_eq!(
        tree.cst.source().as_ref().as_bytes(),
        concatenated.as_bytes()
    );
    Ok(())
}

#[test]
fn node_count_slots_accept_representable_max_without_allocation() -> TestResult
{
    let max_count = usize::try_from(u32::MAX)?;
    let mut slots = NodeCount(max_count).slots()?;

    assert_eq!(slots.len(), max_count);
    assert_eq!(Some(NodeSlot(0)), slots.next());
    assert_eq!(Some(NodeSlot(u32::MAX.wrapping_sub(1))), slots.next_back());
    assert_eq!(slots.len(), max_count.wrapping_sub(2));
    Ok(())
}

#[test]
fn node_count_slots_reject_first_overflow_before_iteration() -> TestResult
{
    let max_count = usize::try_from(u32::MAX)?;
    let first_overflow = max_count.saturating_add(1);
    let error = match NodeCount(first_overflow).slots() {
        | Ok(_slots) => panic!("overflowing node count unexpectedly produced slots"),
        | Err(error) => error,
    };

    assert_eq!(
        BuildError::NodeCountOverflow {
            len: first_overflow
        },
        error
    );
    Ok(())
}

#[test]
fn fixed_cst_root_hash_matches_golden_oracle() -> TestResult
{
    let tree = fixed_tree()?;

    let root_hash = tree.cst.hash(tree.cst.root())?;
    assert_eq!(GOLDEN_ROOT_HASH, root_hash);
    Ok(())
}

#[test]
fn stable_hash_probe() -> TestResult
{
    if std::env::var_os(HASH_PROBE_ENV).is_none() {
        return Ok(());
    }

    let tree = fixed_tree()?;
    let root_hash = tree.cst.hash(tree.cst.root())?;
    println!("{HASH_PROBE_PREFIX}{root_hash:016x}");
    Ok(())
}

/// Grout payload closing each statement cell.
const SENTINEL_GROUT: MoldPayload = MoldPayload::Grout {
    shape: GroutShape::Postfix,
    sort: GroutSort(90),
};
/// Grout payload carried by statement interiors.
const STATEMENT: MoldPayload = MoldPayload::Grout {
    shape: GroutShape::Convex,
    sort: GroutSort(20),
};
/// Grout payload carried by the program interior.
const PROGRAM: MoldPayload = MoldPayload::Grout {
    shape: GroutShape::Convex,
    sort: GroutSort(30),
};

struct TestTree
{
    cst: Cst,
    first_statement: NodeId,
    second_statement: NodeId,
}

#[derive(Clone, Copy)]
struct StatementRanges
{
    start: TextOffset,
    let_end: TextOffset,
    first_space_end: TextOffset,
    ident_end: TextOffset,
    second_space_end: TextOffset,
    eq_end: TextOffset,
    third_space_end: TextOffset,
    number_end: TextOffset,
    line_end: TextOffset,
}

const FIXED_SECOND_STATEMENT: StatementRanges = StatementRanges {
    start: TextOffset(10),
    let_end: TextOffset(13),
    first_space_end: TextOffset(14),
    ident_end: TextOffset(15),
    second_space_end: TextOffset(16),
    eq_end: TextOffset(17),
    third_space_end: TextOffset(18),
    number_end: TextOffset(19),
    line_end: TextOffset(20),
};

const REINDENTED_SECOND_STATEMENT: StatementRanges = StatementRanges {
    start: TextOffset(10),
    let_end: TextOffset(13),
    first_space_end: TextOffset(14),
    ident_end: TextOffset(15),
    second_space_end: TextOffset(19),
    eq_end: TextOffset(20),
    third_space_end: TextOffset(21),
    number_end: TextOffset(22),
    line_end: TextOffset(23),
};

#[test]
fn child_processes_report_same_golden_hash() -> TestResult
{
    let first = child_probe_hash()?;
    let second = child_probe_hash()?;

    assert_eq!(GOLDEN_ROOT_HASH, first);
    assert_eq!(first, second);
    Ok(())
}

#[test]
fn builder_records_grammar_fingerprint() -> TestResult
{
    let tree = fixed_tree()?;

    assert_eq!(GRAMMAR_FP, tree.cst.grammar_fingerprint());
    Ok(())
}

#[test]
fn space_token_changes_leave_root_hash_unchanged() -> TestResult
{
    let fixed = fixed_tree()?;
    let reindented = reindented_tree()?;

    assert_ne!(fixed.cst.source(), reindented.cst.source());
    let fixed_hash = fixed.cst.hash(fixed.cst.root())?;
    let reindented_hash = reindented.cst.hash(reindented.cst.root())?;
    assert_eq!(fixed_hash, reindented_hash);

    let difference = diff(&fixed.cst, &reindented.cst);
    assert_eq!(difference.matches(), &[super::SubtreeMatch::new(
        fixed.cst.root(),
        reindented.cst.root()
    )]);
    assert!(
        difference.unmatched_old().is_empty(),
        "a whitespace-only edit must leave no unmatched old root"
    );
    assert!(
        difference.unmatched_new().is_empty(),
        "a whitespace-only edit must leave no unmatched new root"
    );
    Ok(())
}

#[test]
fn changing_one_tile_mold_changes_root_hash() -> TestResult
{
    let fixed = fixed_tree()?;
    let changed = changed_first_identifier_mold_tree()?;

    let fixed_root_hash = fixed.cst.hash(fixed.cst.root())?;
    let changed_root_hash = changed.cst.hash(changed.cst.root())?;
    assert_ne!(fixed_root_hash, changed_root_hash);
    let fixed_first_hash = fixed.cst.hash(fixed.first_statement)?;
    let changed_first_hash = changed.cst.hash(changed.first_statement)?;
    assert_ne!(fixed_first_hash, changed_first_hash);
    let fixed_second_hash = fixed.cst.hash(fixed.second_statement)?;
    let changed_second_hash = changed.cst.hash(changed.second_statement)?;
    assert_eq!(fixed_second_hash, changed_second_hash);
    Ok(())
}

#[test]
fn two_tiles_differing_only_in_mold_id_hash_differently() -> TestResult
{
    let mut first = CstBuilder::new(SourceText::from("x"), GRAMMAR_FP);
    let first_tile = tile(&mut first, MoldId::from(1), TextOffset(0), TextOffset(1))?;
    let first_cst = first.finish(first_tile)?;

    let mut second = CstBuilder::new(SourceText::from("x"), GRAMMAR_FP);
    let second_tile = tile(&mut second, MoldId::from(2), TextOffset(0), TextOffset(1))?;
    let second_cst = second.finish(second_tile)?;

    let first_text = first_cst.text(first_tile)?;
    let second_text = second_cst.text(second_tile)?;
    assert_eq!(
        first_text, second_text,
        "the two tiles must cover identical source bytes"
    );
    let first_hash = first_cst.hash(first_tile)?;
    let second_hash = second_cst.hash(second_tile)?;
    assert_ne!(
        first_hash, second_hash,
        "tiles differing only in MoldId must hash differently"
    );
    Ok(())
}

#[test]
fn grout_shape_and_sort_participate_in_hash() -> TestResult
{
    let base = grout_token_hash(GroutShape::Convex, GroutSort(5))?;
    let other_shape = grout_token_hash(GroutShape::Infix, GroutSort(5))?;
    let other_sort = grout_token_hash(GroutShape::Convex, GroutSort(6))?;

    assert_ne!(base, other_shape, "grout shape must reach the hash");
    assert_ne!(base, other_sort, "grout sort must reach the hash");
    Ok(())
}

/// A minted close hashes as itself: distinct from ordinary grout, and
/// class-sensitive within its own variant.
///
/// Both halves matter and they pull opposite ways. Ordinary grout must hash
/// exactly as it always did, because a shared frame byte would move every
/// recorded hash of every existing tree — that is what makes the variant
/// additive rather than a format break. And two minted closes differing only in
/// class must hash differently, because the class is the whole content the
/// variant exists to carry; hashing it away would let a `Paren` ghost and a
/// `Brace` ghost be treated as the same subtree by the structural diff.
#[test]
fn a_minted_close_hashes_distinctly_from_grout_and_by_class() -> TestResult
{
    let grout = grout_token_hash(GroutShape::Postfix, GroutSort(5))?;
    let paren = ghost_close_token_hash(GroutSort(5), ClosingClass::Paren)?;
    let brace = ghost_close_token_hash(GroutSort(5), ClosingClass::Brace)?;
    let other_sort = ghost_close_token_hash(GroutSort(6), ClosingClass::Paren)?;

    assert_ne!(
        grout, paren,
        "a minted close frames under its own tag, so ordinary grout keeps its hash"
    );
    assert_ne!(
        paren, brace,
        "the carried class must reach the hash — it is the content of the variant"
    );
    assert_ne!(
        paren, other_sort,
        "a minted close still carries its grout sort, exactly as grout does"
    );
    Ok(())
}

#[test]
fn diff_prunes_unchanged_subtree_and_reports_statement_local_roots() -> TestResult
{
    let fixed = fixed_tree()?;
    let changed = changed_first_identifier_mold_tree()?;
    let difference = diff(&fixed.cst, &changed.cst);

    assert_eq!(difference.matches(), &[super::SubtreeMatch::new(
        fixed.second_statement,
        changed.second_statement
    )]);
    assert_eq!(difference.unmatched_old(), &[fixed.first_statement]);
    assert_eq!(difference.unmatched_new(), &[changed.first_statement]);
    Ok(())
}

#[test]
fn malformed_ranges_and_material_boundaries_return_typed_errors() -> TestResult
{
    assert_eq!(
        Err(BuildError::InvalidTextRange { start: 4, end: 3 }),
        TextRange::new(TextOffset(4), TextOffset(3))
    );

    let mut builder = CstBuilder::new(SourceText::from("abc"), GRAMMAR_FP);
    let out_of_source_range = range(TextOffset(0), TextOffset(4))?;
    assert_eq!(
        Err(BuildError::InvalidTextRange { start: 0, end: 4 }),
        builder.token(Material::Space, MoldPayload::Space, out_of_source_range)
    );

    let mut builder = CstBuilder::new(SourceText::from("é"), GRAMMAR_FP);
    let split_codepoint_range = range(TextOffset(0), TextOffset(1))?;
    assert_eq!(
        Err(BuildError::InvalidTextBoundary {
            node: NodeId::from_raw(NodeSlot(0)),
            start: 0,
            end: 1,
        }),
        builder.token(Material::Space, MoldPayload::Space, split_codepoint_range)
    );

    let mut builder = CstBuilder::new(SourceText::from(""), GRAMMAR_FP);
    let empty_space_range = range(TextOffset(0), TextOffset(0))?;
    assert_eq!(
        Err(BuildError::MoldPayloadMismatch {
            material: Material::Space,
        }),
        builder.token(Material::Space, SENTINEL_GROUT, empty_space_range)
    );

    let mut builder = CstBuilder::new(SourceText::from("x"), GRAMMAR_FP);
    let tile_payload_range = range(TextOffset(0), TextOffset(1))?;
    assert_eq!(
        Err(BuildError::MoldPayloadMismatch {
            material: Material::Tile,
        }),
        builder.token(Material::Tile, MoldPayload::Space, tile_payload_range)
    );
    let mut builder = CstBuilder::new(SourceText::from(""), GRAMMAR_FP);
    let grout_payload_range = range(TextOffset(0), TextOffset(0))?;
    assert_eq!(
        Err(BuildError::MoldPayloadMismatch {
            material: Material::Grout,
        }),
        builder.token(
            Material::Grout,
            MoldPayload::Tile(ident()),
            grout_payload_range
        )
    );

    Ok(())
}

#[test]
fn material_payload_pairing_round_trips_each_variant() -> TestResult
{
    let mut builder = CstBuilder::new(SourceText::from("x"), GRAMMAR_FP);
    let tile_range = range(TextOffset(0), TextOffset(1))?;
    let tile = builder.token(
        Material::Tile,
        MoldPayload::Tile(MoldId::from(7)),
        tile_range,
    )?;
    let parent = builder.node(
        NodeKind::Cell,
        Material::Grout,
        MoldPayload::Grout {
            shape: GroutShape::Prefix,
            sort: GroutSort(42),
        },
        tile_range,
        [tile],
    )?;
    let cst = builder.finish(parent)?;

    let tile_view = cst.node(tile)?;
    assert_eq!(MoldPayload::Tile(MoldId::from(7)), tile_view.payload());
    let parent_view = cst.node(parent)?;
    assert_eq!(
        MoldPayload::Grout {
            shape: GroutShape::Prefix,
            sort: GroutSort(42),
        },
        parent_view.payload()
    );
    Ok(())
}

#[test]
fn malformed_builder_ordering_and_parentage_return_typed_errors() -> TestResult
{
    let mut builder = CstBuilder::new(SourceText::from(""), GRAMMAR_FP);
    let token_kind_range = range(TextOffset(0), TextOffset(0))?;
    let token_kind_result = builder.node(
        NodeKind::Token,
        Material::Space,
        MoldPayload::Space,
        token_kind_range,
        core::iter::empty::<NodeId>(),
    );
    let token_kind_error = match token_kind_result {
        | Ok(_node) => panic!("builder unexpectedly accepted a token interior"),
        | Err(error) => error,
    };
    assert_eq!(BuildError::TokenKindForInterior, token_kind_error);

    let mut builder = CstBuilder::new(SourceText::from(""), GRAMMAR_FP);
    let tile_interior_range = range(TextOffset(0), TextOffset(0))?;
    let tile_interior_error = match builder.node(
        NodeKind::Cell,
        Material::Tile,
        MoldPayload::Tile(kw_let()),
        tile_interior_range,
        core::iter::empty::<NodeId>(),
    ) {
        | Ok(_node) => panic!("builder unexpectedly accepted tile material for an interior"),
        | Err(error) => error,
    };
    assert_eq!(
        BuildError::TileInterior {
            kind: NodeKind::Cell
        },
        tile_interior_error
    );

    let mut builder = CstBuilder::new(SourceText::from(""), GRAMMAR_FP);
    let out_of_bounds_range = range(TextOffset(0), TextOffset(0))?;
    let out_of_bounds_error = match builder.node(
        NodeKind::Cell,
        Material::Space,
        MoldPayload::Space,
        out_of_bounds_range,
        [NodeId::from_raw(NodeSlot(7))],
    ) {
        | Ok(_node) => panic!("builder unexpectedly accepted an unknown child"),
        | Err(error) => error,
    };
    assert_eq!(
        BuildError::NodeOutOfBounds {
            id: NodeId::from_raw(NodeSlot(7)),
            len: 0,
        },
        out_of_bounds_error
    );

    let mut builder = CstBuilder::new(SourceText::from("x"), GRAMMAR_FP);
    let child = space(&mut builder, TextOffset(0), TextOffset(1))?;
    let duplicate_child_range = range(TextOffset(0), TextOffset(1))?;
    let duplicate_child_error = match builder.node(
        NodeKind::Cell,
        Material::Space,
        MoldPayload::Space,
        duplicate_child_range,
        [child, child],
    ) {
        | Ok(_node) => panic!("builder unexpectedly accepted a duplicate child"),
        | Err(error) => error,
    };
    assert_eq!(BuildError::DuplicateChild { child }, duplicate_child_error);

    let mut builder = CstBuilder::new(SourceText::from("x"), GRAMMAR_FP);
    let child = space(&mut builder, TextOffset(0), TextOffset(1))?;
    let parent_range = range(TextOffset(0), TextOffset(1))?;
    let parent = builder.node(
        NodeKind::Cell,
        Material::Space,
        MoldPayload::Space,
        parent_range,
        [child],
    )?;
    let child_already_parented_range = range(TextOffset(0), TextOffset(1))?;
    let child_already_parented_error = match builder.node(
        NodeKind::Meld,
        Material::Space,
        MoldPayload::Space,
        child_already_parented_range,
        [child],
    ) {
        | Ok(_node) => panic!("builder unexpectedly reparented a child"),
        | Err(error) => error,
    };
    assert_eq!(
        BuildError::ChildAlreadyParented { child, parent },
        child_already_parented_error
    );

    Ok(())
}

#[test]
fn finish_rejects_unknown_parented_and_orphan_roots() -> TestResult
{
    let builder = CstBuilder::new(SourceText::from(""), GRAMMAR_FP);
    assert_build_error(
        builder.finish(NodeId::from_raw(NodeSlot(0))),
        &BuildError::UnknownRoot {
            root: NodeId::from_raw(NodeSlot(0)),
        },
    );

    let mut builder = CstBuilder::new(SourceText::from("x"), GRAMMAR_FP);
    let child = space(&mut builder, TextOffset(0), TextOffset(1))?;
    let parent_range = range(TextOffset(0), TextOffset(1))?;
    let parent = builder.node(
        NodeKind::Cell,
        Material::Space,
        MoldPayload::Space,
        parent_range,
        [child],
    )?;
    assert_build_error(builder.finish(child), &BuildError::RootHasParent {
        root: child,
        parent,
    });

    let mut builder = CstBuilder::new(SourceText::from("xy"), GRAMMAR_FP);
    let root = space(&mut builder, TextOffset(0), TextOffset(1))?;
    let orphan = space(&mut builder, TextOffset(1), TextOffset(2))?;
    assert_build_error(builder.finish(root), &BuildError::OrphanNode {
        node: orphan,
    });

    Ok(())
}

#[test]
fn node_data_remains_compact()
{
    assert!(
        core::mem::size_of::<NodeData>() <= 40,
        "the arena node layout must stay within the 40-byte compactness bound"
    );
}

mod fixture_support
{
    use super::*;

    pub(super) fn fixed_tree() -> Result<TestTree, BuildError>
    {
        build_tree(
            SourceText::from("let x = 1\nlet y = 2\n"),
            StatementRanges {
                start: TextOffset(0),
                let_end: TextOffset(3),
                first_space_end: TextOffset(4),
                ident_end: TextOffset(5),
                second_space_end: TextOffset(6),
                eq_end: TextOffset(7),
                third_space_end: TextOffset(8),
                number_end: TextOffset(9),
                line_end: TextOffset(10),
            },
            FIXED_SECOND_STATEMENT,
            ident(),
        )
    }

    pub(super) fn reindented_tree() -> Result<TestTree, BuildError>
    {
        build_tree(
            SourceText::from("let   x=1\nlet y    = 2\n"),
            StatementRanges {
                start: TextOffset(0),
                let_end: TextOffset(3),
                first_space_end: TextOffset(6),
                ident_end: TextOffset(7),
                second_space_end: TextOffset(7),
                eq_end: TextOffset(8),
                third_space_end: TextOffset(8),
                number_end: TextOffset(9),
                line_end: TextOffset(10),
            },
            REINDENTED_SECOND_STATEMENT,
            ident(),
        )
    }

    pub(super) fn changed_first_identifier_mold_tree() -> Result<TestTree, BuildError>
    {
        build_tree(
            SourceText::from("let x = 1\nlet y = 2\n"),
            StatementRanges {
                start: TextOffset(0),
                let_end: TextOffset(3),
                first_space_end: TextOffset(4),
                ident_end: TextOffset(5),
                second_space_end: TextOffset(6),
                eq_end: TextOffset(7),
                third_space_end: TextOffset(8),
                number_end: TextOffset(9),
                line_end: TextOffset(10),
            },
            FIXED_SECOND_STATEMENT,
            changed_ident(),
        )
    }

    fn build_tree(
        source: SourceText,
        first: StatementRanges,
        second: StatementRanges,
        first_ident_mold: MoldId,
    ) -> Result<TestTree, BuildError>
    {
        let mut builder = CstBuilder::new(source, GRAMMAR_FP);
        let first_statement = statement(&mut builder, first, first_ident_mold, number())?;
        let second_statement = statement(&mut builder, second, ident(), number())?;
        let program_range = range(first.start, second.line_end)?;
        let root = builder.node(NodeKind::Wald, Material::Grout, PROGRAM, program_range, [
            first_statement,
            second_statement,
        ])?;
        let cst = builder.finish(root)?;

        Ok(TestTree {
            cst,
            first_statement,
            second_statement,
        })
    }

    fn changed_ident() -> MoldId
    {
        MoldId::from(CHANGED_IDENT_MOLD_ID)
    }

    fn statement(
        builder: &mut CstBuilder,
        ranges: StatementRanges,
        ident_mold: MoldId,
        number_mold: MoldId,
    ) -> Result<NodeId, BuildError>
    {
        let keyword = tile(builder, kw_let(), ranges.start, ranges.let_end)?;
        let first_space = space(builder, ranges.let_end, ranges.first_space_end)?;
        let ident = tile(
            builder,
            ident_mold,
            ranges.first_space_end,
            ranges.ident_end,
        )?;
        let second_space = space(builder, ranges.ident_end, ranges.second_space_end)?;
        let equals = tile(builder, op_eq(), ranges.second_space_end, ranges.eq_end)?;
        let third_space = space(builder, ranges.eq_end, ranges.third_space_end)?;
        let number = tile(
            builder,
            number_mold,
            ranges.third_space_end,
            ranges.number_end,
        )?;
        let newline = space(builder, ranges.number_end, ranges.line_end)?;
        let sentinel_range = range(ranges.line_end, ranges.line_end)?;
        let grout = builder.token(Material::Grout, SENTINEL_GROUT, sentinel_range)?;

        let statement_range = range(ranges.start, ranges.line_end)?;
        builder.node(
            NodeKind::Cell,
            Material::Grout,
            STATEMENT,
            statement_range,
            [
                keyword,
                first_space,
                ident,
                second_space,
                equals,
                third_space,
                number,
                newline,
                grout,
            ],
        )
    }

    fn number() -> MoldId
    {
        MoldId::from(NUMBER_MOLD_ID)
    }

    pub(super) fn tile(
        builder: &mut CstBuilder,
        mold: MoldId,
        start: TextOffset,
        end: TextOffset,
    ) -> Result<NodeId, BuildError>
    {
        let tile_range = range(start, end)?;
        builder.token(Material::Tile, MoldPayload::Tile(mold), tile_range)
    }

    pub(super) fn kw_let() -> MoldId
    {
        MoldId::from(10)
    }
    pub(super) fn assert_build_error<T>(
        result: Result<T, BuildError>,
        expected: &BuildError,
    )
    {
        match result {
            | Ok(_value) => panic!("builder unexpectedly succeeded"),
            | Err(error) => assert_eq!(&error, expected),
        }
    }

    use super::ClosingClass;

    pub(super) fn ghost_close_token_hash(
        sort: GroutSort,
        class: ClosingClass,
    ) -> Result<StableHash, BuildError>
    {
        let mut builder = CstBuilder::new(SourceText::from(""), GRAMMAR_FP);
        let ghost_range = range(TextOffset(0), TextOffset(0))?;
        let ghost = builder.token(
            Material::Grout,
            MoldPayload::GhostClose { sort, class },
            ghost_range,
        )?;
        let cst = builder.finish(ghost)?;
        cst.hash(ghost)
    }

    pub(super) fn grout_token_hash(
        shape: GroutShape,
        sort: GroutSort,
    ) -> Result<StableHash, BuildError>
    {
        let mut builder = CstBuilder::new(SourceText::from(""), GRAMMAR_FP);
        let grout_range = range(TextOffset(0), TextOffset(0))?;
        let grout = builder.token(
            Material::Grout,
            MoldPayload::Grout { shape, sort },
            grout_range,
        )?;
        let cst = builder.finish(grout)?;
        cst.hash(grout)
    }
    pub(super) fn space(
        builder: &mut CstBuilder,
        start: TextOffset,
        end: TextOffset,
    ) -> Result<NodeId, BuildError>
    {
        let space_range = range(start, end)?;
        builder.token(Material::Space, MoldPayload::Space, space_range)
    }

    fn op_eq() -> MoldId
    {
        MoldId::from(EQ_MOLD_ID)
    }

    pub(super) fn ident() -> MoldId
    {
        MoldId::from(IDENT_MOLD_ID)
    }

    pub(super) fn range(
        start: TextOffset,
        end: TextOffset,
    ) -> Result<TextRange, BuildError>
    {
        TextRange::new(start, end)
    }

    pub(super) fn child_probe_hash() -> Result<StableHash, Box<dyn Error>>
    {
        let current_exe = std::env::current_exe()
            .map_err(|error| {
                io::Error::new(
                    error.kind(),
                    format!("failed to locate current test binary: {error}"),
                )
            })
            .map_err(|error| -> Box<dyn Error> { Box::new(error) })?;
        let output = Command::new(current_exe)
            .arg("--exact")
            .arg("tests::stable_hash_probe")
            .arg("--nocapture")
            .env(HASH_PROBE_ENV, "1")
            .output()
            .map_err(|error| {
                io::Error::new(
                    error.kind(),
                    format!("failed to run hash probe child process: {error}"),
                )
            })
            .map_err(|error| -> Box<dyn Error> { Box::new(error) })?;

        if !output.status.success() {
            return Err(
                io::Error::other(format!("hash probe exited with {}", output.status)).into(),
            );
        }

        let stdout = String::from_utf8(output.stdout)?;
        let Some(hash_text) = stdout
            .lines()
            .find_map(|line| line.strip_prefix(HASH_PROBE_PREFIX))
        else {
            return Err(
                io::Error::new(io::ErrorKind::InvalidData, "hash probe marker missing").into(),
            );
        };

        let parsed_hash_raw = u64::from_str_radix(hash_text, 16)?;
        let parsed_hash = StableHash(parsed_hash_raw);
        Ok(parsed_hash)
    }
}
