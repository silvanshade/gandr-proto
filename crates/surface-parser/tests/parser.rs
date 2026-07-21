#![cfg_attr(
    test,
    allow(
        clippy::arithmetic_side_effects,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic,
        clippy::unwrap_used,
        reason = "the standard test-allow set keeps parser acceptance tests readable (docs/workflow/rust.md)"
    )
)]

#[cfg(test)]
mod contracts
{
    use core::error::Error;
    use std::sync::OnceLock;

    use gandr_surface_grammar::Assoc;
    use gandr_surface_grammar::Pbg;
    use gandr_surface_grammar::PrecDag;
    use gandr_surface_grammar::PrecSpec;
    use gandr_surface_grammar::Regex;
    use gandr_surface_grammar::Rule;
    use gandr_surface_grammar::RuleName;
    use gandr_surface_grammar::Sort;
    use gandr_surface_grammar::TileLabel;
    use gandr_surface_grammar::built_in;
    use gandr_surface_parser::Checkpoint;
    use gandr_surface_parser::CheckpointBytesRef;
    use gandr_surface_parser::MeldState;
    use gandr_surface_parser::MoldedTile;
    use gandr_surface_parser::Oblig;
    use gandr_surface_parser::TileText;
    use gandr_surface_parser::parse;
    use gandr_surface_syntax::Cst;
    use gandr_surface_syntax::Material;
    use gandr_surface_syntax::MoldId;
    use gandr_surface_syntax::NodeId;
    use gandr_surface_syntax::NodeKind;
    use gandr_surface_syntax::SourceSlice;
    use proptest::prelude::*;

    /// The real built-in grammar, built once and shared across proptest cases.
    static BUILT_IN: OnceLock<Pbg> = OnceLock::new();
    /// Count of CST nodes reachable from a root.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug)]
    struct ReachableNodeCount(usize);

    impl From<ReachableNodeCount> for usize
    {
        #[inline]
        fn from(count: ReachableNodeCount) -> Self
        {
            count.0
        }
    }

    /// Whether a direct tile lookup found its target.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug)]
    struct TilePresence(bool);

    impl From<TilePresence> for bool
    {
        #[inline]
        fn from(presence: TilePresence) -> Self
        {
            presence.0
        }
    }

    /// Number of grammar molds available to the fuzz strategy.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug)]
    struct MoldCount(u32);

    impl From<u32> for MoldCount
    {
        #[inline]
        fn from(count: u32) -> Self
        {
            Self(count)
        }
    }

    impl From<MoldCount> for u32
    {
        #[inline]
        fn from(count: MoldCount) -> Self
        {
            count.0
        }
    }

    #[test]
    fn module_declarations_mold_zero_obligation() -> Result<(), Box<dyn Error>>
    {
        let pbg = built();
        let cases = [
            "module M { def x = 1; def y : Integer; }",
            "module M : #{ x: Integer, f: F Integer } { \
             def x = 1; \
             def f(a: Integer) -> F Integer { ret a } \
             }",
        ];
        for src in cases {
            let result = parse(pbg, SourceSlice::from(src))?;
            assert!(
                bool::from(result.is_clean()),
                "module source {src:?} molds clean; obligations: {:?}",
                result
                    .obligations()
                    .iter()
                    .map(|o| o.class)
                    .collect::<Vec<_>>()
            );
            assert_eq!(result.cst().grammar_fingerprint(), pbg.fingerprint());
            assert_eq!(
                NodeKind::Wald,
                {
                    let root = result.cst().node(result.cst().root())?;
                    root.kind()
                },
                "module source {src:?} commits to a Wald"
            );
        }
        Ok(())
    }
    #[test]
    fn malformed_module_member_uses_ordinary_recovery() -> Result<(), Box<dyn Error>>
    {
        let pbg = built();
        let src = "module M { def bad = 1 ~ 2; def good = 1; }";
        let result = parse(pbg, SourceSlice::from(src))?;
        assert!(
            !bool::from(result.is_clean()),
            "a malformed module member must flag an ordinary repair obligation"
        );
        assert!(
            result.obligations().iter().any(|o| matches!(
                o.class,
                Oblig::MissingMeld | Oblig::MissingTile | Oblig::UnmoldedTok
            )),
            "module member repair should be an ordinary missing-meld/tile/unmolded obligation; got {:?}",
            result
                .obligations()
                .iter()
                .map(|o| o.class)
                .collect::<Vec<_>>()
        );
        assert!(
            result
                .obligations()
                .iter()
                .all(|o| o.class != Oblig::AmbiguousPrec),
            "malformed module member recovery must not introduce precedence ambiguity"
        );
        assert_eq!(
            NodeKind::Wald,
            {
                let root = result.cst().node(result.cst().root())?;
                root.kind()
            },
            "malformed module source still commits"
        );
        Ok(())
    }
    /// Return the shared built-in grammar.
    fn built() -> &'static Pbg
    {
        BUILT_IN.get_or_init(|| built_in().expect("built-in grammar assembles"))
    }

    #[test]
    fn trace_left_associates_like_figure_24() -> Result<(), Box<dyn Error>>
    {
        // `n + n + n` left-associates to `((n + n) + n)`: the first `+` Reduces
        // when the second (equal, left-associative) `+` arrives.
        let pbg = arith_pbg()?;
        let cst = commit_stream(
            &pbg,
            &stream(&pbg, &[
                TileLabel("n"),
                TileLabel("+"),
                TileLabel("n"),
                TileLabel("+"),
                TileLabel("n"),
            ]),
        )?;

        let outer = sole_meld(&cst)?;
        let outer_has_plus = has_tile(&cst, outer, TileText::from("+"))?;
        assert!(bool::from(outer_has_plus));
        let outer_view = cst.node(outer)?;
        let outer_child_slice = outer_view.children()?;
        let outer_children = outer_child_slice.to_vec();
        assert_eq!(3, outer_children.len());
        // Left child is the inner `(n + n)` meld; right child is the bare atom.
        let inner = outer_children[0];
        let inner_view = cst.node(inner)?;
        assert_eq!(NodeKind::Meld, inner_view.kind());
        let inner_has_plus = has_tile(&cst, inner, TileText::from("+"))?;
        assert!(bool::from(inner_has_plus));
        let right_child = cst.node(outer_children[2])?;
        assert_eq!(NodeKind::Token, right_child.kind());
        Ok(())
    }
    #[test]
    fn no_obligations_on_well_formed_fragments() -> Result<(), Box<dyn Error>>
    {
        let pbg = arith_pbg()?;
        for labels in [
            vec![TileLabel("n")],
            vec![
                TileLabel("n"),
                TileLabel("+"),
                TileLabel("n"),
                TileLabel("*"),
                TileLabel("n"),
            ],
            vec![
                TileLabel("n"),
                TileLabel("+"),
                TileLabel("n"),
                TileLabel("+"),
                TileLabel("n"),
            ],
            vec![
                TileLabel("("),
                TileLabel("n"),
                TileLabel("+"),
                TileLabel("n"),
                TileLabel(")"),
                TileLabel("*"),
                TileLabel("n"),
            ],
        ] {
            let mut state = MeldState::new(&pbg);
            for tile in stream(&pbg, &labels) {
                state.push(&tile);
            }
            assert!(
                state.obligations().is_empty(),
                "well-formed fragment {labels:?} melds without obligations"
            );
            state.commit()?;
        }
        Ok(())
    }

    /// The number of nodes reachable from `id`, proving the tree is
    /// well-formed.
    fn count_nodes(
        cst: &Cst,
        id: NodeId,
    ) -> Result<ReachableNodeCount, Box<dyn Error>>
    {
        let mut total = 0_usize;
        let mut pending = vec![id];
        while let Some(next) = pending.pop() {
            total = total.saturating_add(1);
            let view = cst.node(next)?;
            let children = view.children()?;
            pending.extend(children.iter().rev().copied());
        }
        Ok(ReachableNodeCount(total))
    }

    #[test]
    fn trace_brackets_reset_precedence_like_figure_33() -> Result<(), Box<dyn Error>>
    {
        // `( n + n ) * n` melds to `((n + n) * n)`: the bracket closes on its
        // matching delimiter (a `≐` step) and resets precedence, so `+` binds
        // inside the parens despite being looser than the enclosing `*`.
        let pbg = arith_pbg()?;
        let cst = commit_stream(
            &pbg,
            &stream(&pbg, &[
                TileLabel("("),
                TileLabel("n"),
                TileLabel("+"),
                TileLabel("n"),
                TileLabel(")"),
                TileLabel("*"),
                TileLabel("n"),
            ]),
        )?;

        let mul = sole_meld(&cst)?;
        let mul_has_star = has_tile(&cst, mul, TileText::from("*"))?;
        assert!(bool::from(mul_has_star), "the outer meld is the `*` term");
        let mul_view = cst.node(mul)?;
        let mul_child_slice = mul_view.children()?;
        let mul_children = mul_child_slice.to_vec();
        assert_eq!(3, mul_children.len());
        let paren = mul_children[0];
        // The paren meld's direct tiles are `(` and `)`, enclosing the `+` meld.
        let paren_tiles = direct_tiles(&cst, paren)?;
        assert_eq!(paren_tiles, vec!["(".to_owned(), ")".to_owned()]);
        assert!(
            {
                let paren_view = cst.node(paren)?;
                let paren_children = paren_view.children()?;
                let has_plus = has_tile(&cst, paren_children[1], TileText::from("+"))?;
                bool::from(has_plus)
            },
            "the `+` term nests inside the parens"
        );
        Ok(())
    }
    #[test]
    fn trace_precedence_climbs_like_figure_23() -> Result<(), Box<dyn Error>>
    {
        // `n + n * n` melds to `(n + (n * n))`: Shift, then Reduce the tighter
        // `*` handle before the looser `+` — no stuck states, no grout. Atoms
        // stay bare tokens, so the `+` meld's children are `[n, +, (n * n)]`.
        let pbg = arith_pbg()?;
        let cst = commit_stream(
            &pbg,
            &stream(&pbg, &[
                TileLabel("n"),
                TileLabel("+"),
                TileLabel("n"),
                TileLabel("*"),
                TileLabel("n"),
            ]),
        )?;

        let add = sole_meld(&cst)?;
        let add_view = cst.node(add)?;
        assert_eq!(NodeKind::Meld, add_view.kind());
        let add_has_plus = has_tile(&cst, add, TileText::from("+"))?;
        assert!(bool::from(add_has_plus), "the outer meld is the `+` term");
        let add_has_star = has_tile(&cst, add, TileText::from("*"))?;
        assert!(
            !bool::from(add_has_star),
            "`*` binds tighter, one level down"
        );
        let add_child_slice = add_view.children()?;
        let add_children = add_child_slice.to_vec();
        assert_eq!(3, add_children.len());
        // The right operand is the `*` meld; the left operand is the bare atom.
        let left_child = cst.node(add_children[0])?;
        assert_eq!(NodeKind::Token, left_child.kind());
        let mul = add_children[2];
        let mul_view = cst.node(mul)?;
        assert_eq!(NodeKind::Meld, mul_view.kind());
        let mul_has_star = has_tile(&cst, mul, TileText::from("*"))?;
        assert!(bool::from(mul_has_star), "the nested meld is the `*` term");
        Ok(())
    }

    /// A synthetic arithmetic grammar: atoms, `*` (tight), `+` (loose), parens.
    fn arith_pbg() -> Result<Pbg, Box<dyn Error>>
    {
        let mut spec = PrecSpec::new();
        let atom = spec.insert("atom", None)?;
        let mul = spec.insert("mul", Some(Assoc::Left))?;
        let add = spec.insert("add", Some(Assoc::Left))?;
        spec.add_edge(atom, mul)?;
        spec.add_edge(mul, add)?;
        let dag = PrecDag::build(&spec)?;
        let pbg = Pbg::build(dag, vec![
            Rule::new(
                RuleName("num"),
                Sort::Expression,
                atom,
                Regex::tile(TileLabel("n")),
            ),
            Rule::new(
                RuleName("paren"),
                Sort::Expression,
                atom,
                Regex::seq([
                    Regex::tile(TileLabel("(")),
                    Regex::sort(Sort::Expression),
                    Regex::tile(TileLabel(")")),
                ]),
            ),
            Rule::new(
                RuleName("mul"),
                Sort::Expression,
                mul,
                Regex::seq([
                    Regex::sort(Sort::Expression),
                    Regex::tile(TileLabel("*")),
                    Regex::sort(Sort::Expression),
                ]),
            ),
            Rule::new(
                RuleName("add"),
                Sort::Expression,
                add,
                Regex::seq([
                    Regex::sort(Sort::Expression),
                    Regex::tile(TileLabel("+")),
                    Regex::sort(Sort::Expression),
                ]),
            ),
        ])?;
        Ok(pbg)
    }
    /// Push a whole stream and commit.
    fn commit_stream(
        pbg: &Pbg,
        tiles: &[MoldedTile],
    ) -> Result<Cst, Box<dyn Error>>
    {
        let mut state = MeldState::new(pbg);
        for tile in tiles {
            state.push(tile);
        }
        let cst = state.commit()?;
        Ok(cst)
    }
    /// Build a molded-tile stream from `(label)` pairs over `pbg`.
    fn stream(
        pbg: &Pbg,
        labels: &[TileLabel],
    ) -> Vec<MoldedTile>
    {
        labels
            .iter()
            .map(|&label| MoldedTile::new(only(pbg, label), TileText::from(label.0)))
            .collect()
    }

    /// The sole mold id declared for `label`.
    fn only(
        pbg: &Pbg,
        label: TileLabel,
    ) -> MoldId
    {
        let molds = pbg.candidates(label);
        assert_eq!(1, molds.len(), "label {} must have one mold", label.0);
        molds[0]
    }
    /// The single meld child of the root wald.
    fn sole_meld(cst: &Cst) -> Result<NodeId, Box<dyn Error>>
    {
        let children = cst.children(cst.root())?;
        assert_eq!(1, children.len(), "one top-level term");
        Ok(children[0])
    }
    /// Whether the direct tile children of `id` include `label`.
    fn has_tile(
        cst: &Cst,
        id: NodeId,
        label: TileText<'_>,
    ) -> Result<TilePresence, Box<dyn Error>>
    {
        let label_text = <&str>::from(label);
        let tiles = direct_tiles(cst, id)?;
        Ok(TilePresence(tiles.iter().any(|tile| tile == label_text)))
    }
    /// The label texts of a meld's direct tile children, left to right.
    fn direct_tiles(
        cst: &Cst,
        id: NodeId,
    ) -> Result<Vec<String>, Box<dyn Error>>
    {
        let mut out = Vec::new();
        let parent = cst.node(id)?;
        for child in parent.children()? {
            let view = cst.node(*child)?;
            if view.kind() == NodeKind::Token && view.material() == Material::Tile {
                let text = view.text()?;
                out.push(text.as_ref().to_owned());
            }
        }
        Ok(out)
    }

    // T3: paper-trace fixtures on equivalent gandr fragments (figures 23, 24, and
    // 33).

    // T1: totality / panic-freedom over arbitrary molded-tile streams.

    /// A biased tile-index strategy over the real mold table plus out-of-range
    /// sentinels (boundary bias: extremes and the out-of-range edge).
    fn tile_index(mold_count: MoldCount) -> impl Strategy<Value = MoldId>
    {
        let mold_count = u32::from(mold_count);
        prop_oneof![
            6 => 0_u32 .. mold_count,
            1 => Just(0_u32),
            1 => Just(mold_count.saturating_sub(1)),
            2 => mold_count .. mold_count.saturating_add(8),
        ]
        .prop_map(MoldId::from)
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]

        #[test]
        fn arbitrary_real_mold_streams_parse_totally(
            indices in {
                let count = match u32::try_from(built().mold_count().0) {
                    | Ok(count) => count,
                    | Err(_error) => u32::MAX,
                };
                prop::collection::vec(tile_index(MoldCount::from(count)), 0 .. 48)
            }
        ) {
            let pbg = built();
            let tiles: Vec<MoldedTile> = indices
                .iter()
                .map(|&mold| {
                    let label = pbg.mold(mold).map_or("?", |def| def.label);
                    MoldedTile::new(mold, TileText::from(label))
                })
                .collect();

            // Every push is total; finalize is a non-destructive query.
            let mut state = MeldState::new(pbg);
            for tile in &tiles {
                state.push(tile);
                let _completion = state.finalize();
            }

            // Commit yields a well-formed CST recording the grammar fingerprint.
            let obligations = state.obligations().to_vec();
            let cst = state.commit().expect("commit is total on a well-formed slope");
            prop_assert_eq!(cst.grammar_fingerprint(), pbg.fingerprint());
            let reachable = count_nodes(&cst, cst.root()).expect("tree is well-formed");
            prop_assert!(usize::from(reachable) >= 1);

            // Determinism: an identical run yields an identical committed tree.
            let mut replay = MeldState::new(pbg);
            for tile in &tiles {
                replay.push(tile);
            }
            prop_assert_eq!(replay.obligations().to_vec(), obligations);
            let replayed = replay.commit().expect("replay commits");
            prop_assert_eq!(
                cst.hash(cst.root()).expect("root hash"),
                replayed.hash(replayed.root()).expect("replayed root hash")
            );
        }
    }

    /// A bracket-heavy vocabulary strategy over the arithmetic grammar.
    fn arith_label() -> impl Strategy<Value = TileLabel>
    {
        prop_oneof![
            3 => Just(TileLabel("n")),
            2 => Just(TileLabel("+")),
            2 => Just(TileLabel("*")),
            2 => Just(TileLabel("(")),
            2 => Just(TileLabel(")")),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]

        #[test]
        fn arbitrary_bracket_streams_parse_totally(
            labels in prop::collection::vec(arith_label(), 0 .. 32)
        ) {
            let pbg = arith_pbg().expect("arith grammar");
            let tiles: Vec<MoldedTile> = labels
                .iter()
                .map(|&label| MoldedTile::new(only(&pbg, label), TileText::from(label.0)))
                .collect();
            let mut state = MeldState::new(&pbg);
            for tile in &tiles {
                state.push(tile);
            }
            let cst = state.commit().expect("bracket stream commits");
            let reachable = count_nodes(&cst, cst.root()).expect("well-formed");
            prop_assert!(usize::from(reachable) >= 1);
        }
    }

    // T4: checkpoint round-trip and resume-equivalence over synthesized streams.

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]

        #[test]
        fn checkpoint_resume_equals_uninterrupted(
            labels in prop::collection::vec(arith_label(), 0 .. 24),
            split in 0_usize .. 24
        ) {
            let pbg = arith_pbg().expect("arith grammar");
            let tiles: Vec<MoldedTile> = labels
                .iter()
                .map(|&label| MoldedTile::new(only(&pbg, label), TileText::from(label.0)))
                .collect();
            let cut = split.min(tiles.len());

            // Uninterrupted run.
            let mut plain = MeldState::new(&pbg);
            for tile in &tiles {
                plain.push(tile);
            }
            let plain_obligations = plain.obligations().to_vec();
            let plain_cst = plain.commit().expect("plain commits");
            let plain_hash = plain_cst.hash(plain_cst.root()).expect("plain root hash");

            // Checkpoint at the split, round-trip through bytes, resume.
            let mut prefix = MeldState::new(&pbg);
            for tile in tiles.get(.. cut).unwrap_or(&[]) {
                prefix.push(tile);
            }
            let checkpoint = prefix.checkpoint();
            let bytes = checkpoint.to_bytes();
            let restored = Checkpoint::from_bytes(CheckpointBytesRef::from(&bytes))
                .expect("round-trips");
            prop_assert_eq!(&restored, &checkpoint);

            let mut resumed = MeldState::resume(&pbg, &restored);
            for tile in tiles.get(cut ..).unwrap_or(&[]) {
                resumed.push(tile);
            }
            prop_assert_eq!(resumed.obligations().to_vec(), plain_obligations);
            let resumed_cst = resumed.commit().expect("resumed commits");
            let resumed_hash = resumed_cst.hash(resumed_cst.root()).expect("resumed root hash");
            prop_assert_eq!(resumed_hash, plain_hash);
        }
    }
}

#[cfg(test)]
#[path = "acceptance.rs"]
mod acceptance;
