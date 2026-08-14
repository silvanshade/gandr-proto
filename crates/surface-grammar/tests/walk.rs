#[cfg(test)]
mod contracts
{
    use alloc::collections::BTreeMap;
    use alloc::collections::BTreeSet;
    use core::error::Error;

    use gandr_surface_grammar::Bound;
    use gandr_surface_grammar::Comparison;
    use gandr_surface_grammar::Dir;
    use gandr_surface_grammar::End;
    use gandr_surface_grammar::MAX_WALK_CHAIN_LEN;
    use gandr_surface_grammar::MoldCount;
    use gandr_surface_grammar::MoldId;
    use gandr_surface_grammar::Pbg;
    use gandr_surface_grammar::PbgError;
    use gandr_surface_grammar::Prec;
    use gandr_surface_grammar::PrecDag;
    use gandr_surface_grammar::PrecSpec;
    use gandr_surface_grammar::Regex;
    use gandr_surface_grammar::Rule;
    use gandr_surface_grammar::RuleName;
    use gandr_surface_grammar::SeenKeyVerdict;
    use gandr_surface_grammar::Sort;
    use gandr_surface_grammar::StepSym;
    use gandr_surface_grammar::TileLabel;
    use gandr_surface_grammar::built_in;
    use gandr_surface_grammar::comparison_table;
    use gandr_surface_grammar::reachable_molds;
    use gandr_surface_grammar::seen_key_verdict;
    use gandr_surface_grammar::walk_index;
    use gandr_surface_syntax::GrammarFingerprint;
    use gandr_theory_graphs::WalkChainLength;

    /// The pinned fingerprint of the built-in mold and interned-context tables.
    ///
    /// Covers the full built-in surface: the expression / pattern / type /
    /// instantiation forms; the `data` / `codata` datatype declarations
    /// (members, and observation fields, reserved `op` / `rule` / grade /
    /// GADT / attr slots); `def rec` + copattern clauses folded into the
    /// def family; the `for` / `while` / `loop` / `break` / `continue`
    /// control atoms; the `import` MVP; the reserved operator-fixity
    /// declaration; the reserved `rec` block; the `case … with` view +
    /// empty-arm option; `${ … }` string interpolation; the shell surface
    /// (braced parameter expansion, fragment-lexed `double_quoted_string`,
    /// and the `[ … ]` `subshell` on distinct `subshell_open` /
    /// `subshell_close` bracket tiles, with the dead composite shell rules
    /// folded into `shell_word` / `redirection_operator`); first-class
    /// holes (the standalone `hole_name` folded to the `?` hole's optional
    /// tail); the `number.type` Type-sort atom for literal-endpoint type
    /// holes; right-associative product / sum operators and the
    /// pairwise-incomparable right-associative set tier
    /// (union / intersection / lazy-product); and the keyword-led `module` Item
    /// form with its optional transparent record-type ascription and body-local
    /// non-recursive def/signature member family; the distinct
    /// `run PAT <- E` computation bind; the `val PAT = E` value bind; and the
    /// dedicated instantiation sort for type arguments, direction sigils, named
    /// measures, explicit residents, and `tail`; and the ruled circuit block
    /// form — the `sign` block with its `sort` / `data` / `oper` / `rule`
    /// judgment members, the four-glyph arrow grid, the two-sided port lists
    /// with parameter-side binders, and the top-level `oper` / `rule`
    /// declaration with its `node` / `feed` body statements.
    /// The rule-face migration then made `==>` the description-rule face former
    /// beside the retired `~>`, which stays admissible only so a stale face
    /// reaches the elaborator's decline. The nested generator-block form then
    /// gave the `data` / `codata` heads typed parameter binders and the
    /// `: Idx -> Type` annotation, and the generator member its local
    /// telescope + `-->` signature — while the retired Haskell-style head
    /// (bare parameters, no annotation) and the field-tuple member stay
    /// admissible so a stale declaration reaches the elaborator's retirement
    /// decline with the respelling hint. The sign block's members then became
    /// `;`-terminated (owner directive, gandr-ng9.14): the terminator is
    /// load-bearing at sign item level, closing each member's trailing sort
    /// hole before the next member's lead can cross it.
    const BUILT_IN_FINGERPRINT: GrammarFingerprint = GrammarFingerprint(0xb0b5_edc0_6cc3_d277);

    /// The pinned declared mold count of the built-in surface.
    ///
    /// Block-bearing constructs dominate the count: the `for` / `while` /
    /// `loop` bodies, the `def rec` body, and the `rec` block members each
    /// carry a fresh `block()` / statement-alt copy (a per-occurrence `rctx`
    /// mold identity), plus the `data` / `codata` member families (each
    /// `comma1` clone doubling its field tiles) and the keyword-led form
    /// openers. The shell surface contributes the inlined parameter-expansion
    /// tiles of `double_quoted_string` and the `subshell_open` /
    /// `subshell_close` bracket tiles, while the deleted composite shell rules
    /// (`command`, `argument`, `redirection`, and the standalone
    /// `command_name`) keep their placeholder tiles out of the count.
    /// First-class holes keep `hole_name` at a single mold (the `?` hole's
    /// optional tail); the `number.type` Type-sort atom adds the `number`
    /// lexeme's type realisation; and the `module M (: #{ … })? { def … }`
    /// Item form contributes its keyword-led opener, inline record-type
    /// ascription, and body-local non-recursive definition/signature family.
    /// The one-level nested-module member adds 81 molds: its keyword, name,
    /// braces, optional inline signature, and definition-only body copy.
    /// The distinct `run`- and `val`-led bind rules each contribute one keyword
    /// mold in their 20 expanded statement contexts.
    /// The dedicated instantiation-sort forms add seven molds: two `<`
    /// occurrences and one each for `>`, `=`, `tail`, and the two
    /// named-resident `identifier` occurrences. The circuit block form adds
    /// 238, dominated by the `comma1` clones of its port lists: the `oper` /
    /// `rule` judgment is declared **once** and shared by the `sign` member and
    /// the top-level declaration precisely to keep that number from doubling,
    /// and the parameter-side binders are kept off the result side for the same
    /// reason. The rule-face migration adds four: the `data` and `codata`
    /// members' face arrow becomes a two-way alternation (`==>` ruled, `~>`
    /// retired-but-admissible), and each member family carries two copies of
    /// its arrow through `comma1`. The nested generator-block form adds
    /// fifty: the typed head-parameter binder (its `:` and Type hole, per
    /// `comma1` clone and per block kind), the head's `: Idx -> Type`
    /// annotation, the generator member's local telescope and `-->`
    /// signature, and the member-list's optional migration comma — the
    /// retired bare-parameter and field-tuple tails ride the same
    /// alternations rather than adding rules of their own. The sign member
    /// terminator adds one: the `;` tile after the inlined member family
    /// (gandr-ng9.14).
    const BUILT_IN_MOLD_COUNT: MoldCount = MoldCount(1896);

    /// The declared per-label candidate inventory, sorted and exact.
    ///
    /// One candidate per tile OCCURRENCE (the rctx mold identity), pinned from
    /// the `MoldDef` table. This is the declared inventory;
    /// the reachable multi-mold metric belongs to the generative walk
    /// front-end.
    const DECLARED_CANDIDATE_INVENTORY: &[(&str, usize)] = &[
        ("!", 21),
        ("!=", 1),
        ("\"", 16),
        ("#!{", 1),
        ("#{", 7),
        ("$", 3),
        ("${", 3),
        ("&", 3),
        ("&&", 2),
        ("'", 2),
        ("(", 162),
        (")", 165),
        ("*", 2),
        ("*/", 1),
        ("+", 3),
        ("++", 1),
        (",", 73),
        ("-", 2),
        ("-->", 15),
        ("->", 9),
        (".", 7),
        ("..", 3),
        ("/*", 1),
        ("/\\", 1),
        (":", 138),
        (":>", 2),
        (";", 201),
        ("<", 4),
        ("<&", 1),
        ("<-", 20),
        ("<->", 13),
        ("<=", 1),
        ("<=>", 13),
        ("<>", 1),
        ("=", 53),
        ("==", 1),
        ("==>", 17),
        ("=>", 8),
        (">", 3),
        (">&", 1),
        (">=", 1),
        (">>", 1),
        ("?", 2),
        ("@[", 4),
        ("Any", 1),
        ("Boolean", 1),
        ("Char", 1),
        ("F", 1),
        ("Integer", 1),
        ("Never", 1),
        ("String", 1),
        ("Symbol", 1),
        ("U", 1),
        ("Unit", 1),
        ("Unknown", 1),
        ("Void", 1),
        ("[", 11),
        ("]", 15),
        ("_", 31),
        ("acquire", 20),
        ("as", 82),
        ("at", 1),
        ("block_comment", 1),
        ("block_comment_content", 1),
        ("break", 1),
        ("case", 1),
        ("character", 1),
        ("close", 1),
        ("co", 1),
        ("codata", 1),
        ("command_substitution_start", 1),
        ("constructor", 6),
        ("continue", 1),
        ("data", 8),
        ("def", 5),
        ("double_string_fragment", 1),
        ("drop", 1),
        ("dup", 1),
        ("else", 2),
        ("end", 1),
        ("environment_assignment", 1),
        ("escape_sequence", 9),
        ("extern", 1),
        ("f32", 1),
        ("f64", 1),
        ("false", 3),
        ("feed", 2),
        ("file_descriptor", 1),
        ("fn", 1),
        ("for", 1),
        ("forall", 1),
        ("force", 1),
        ("fork", 40),
        ("from", 1),
        ("hold", 1),
        // One mold — the `?` hole's optional-tail occurrence. The
        // former standalone `hole_name` Expression atom (a second, fresh-slot
        // mold) is folded away; it tied with every bare `identifier` and, at a
        // smaller `MoldId`, would have won.
        ("hole_name", 1),
        ("i32", 1),
        ("i64", 1),
        ("identifier", 285),
        ("if", 2),
        ("import", 1),
        ("in", 1),
        ("infix", 1),
        ("infixl", 1),
        ("infixr", 1),
        ("leta", 20),
        ("line_comment", 1),
        ("list_operator", 3),
        ("loop", 1),
        ("migrate", 1),
        ("module", 2),
        ("mu", 1),
        ("negation", 1),
        ("newline", 1),
        ("node", 2),
        ("number", 21),
        ("offer", 1),
        ("op", 3),
        ("oper", 4),
        ("pipeline_operand", 2),
        ("postfix", 1),
        ("prefix", 1),
        ("rec", 2),
        ("recv", 20),
        ("release", 20),
        ("ret", 1),
        ("rule", 12),
        ("run", 20),
        ("select", 1),
        ("send", 1),
        ("shebang", 1),
        ("shell_and", 2),
        ("shell_list", 1),
        ("shell_or", 2),
        ("shell_word", 1),
        ("sign", 1),
        ("single_quoted_content", 1),
        ("sort", 1),
        ("string_fragment", 7),
        ("subshell_close", 1),
        ("subshell_open", 1),
        ("tail", 1),
        ("thunk", 1),
        ("true", 3),
        ("type", 9),
        ("type_identifier", 16),
        ("type_variable", 9),
        ("typed_number", 3),
        ("u32", 1),
        ("u64", 1),
        ("val", 20),
        ("variable_name", 5),
        ("while", 1),
        ("with", 1),
        ("{", 33),
        ("|", 4),
        ("|&", 1),
        ("||", 2),
        ("}", 47),
        ("~>", 4),
        ("ω", 16),
    ];

    #[test]
    fn declared_mold_candidate_inventory_is_exact() -> Result<(), Box<dyn Error>>
    {
        let pbg = built_in()?;
        let counts = pbg.candidate_counts();

        // Sorted by label, ascending.
        let mut previous: Option<TileLabel> = None;
        for &(label, _count) in &counts {
            if let Some(prev) = previous {
                assert!(prev < label, "candidate inventory must be sorted by label");
            }
            previous = Some(label);
        }

        let observed: Vec<(&str, usize)> = counts
            .iter()
            .map(|&(label, count)| (label.0, count.0))
            .collect();
        let expected: Vec<(&str, usize)> = DECLARED_CANDIDATE_INVENTORY.to_vec();
        assert_eq!(
            observed, expected,
            "declared per-label candidate counts must be exact"
        );

        let total: usize = counts.iter().map(|&(_label, count)| count.0).sum();
        assert_eq!(
            BUILT_IN_MOLD_COUNT.0, total,
            "total occurrences equal mold count"
        );
        assert_eq!(BUILT_IN_MOLD_COUNT, pbg.mold_count());
        Ok(())
    }

    #[test]
    fn walk_index_projects_every_mold_once() -> Result<(), Box<dyn Error>>
    {
        let pbg = built_in()?;
        let index = walk_index(&pbg)?;
        for (mold_id, def) in pbg.iter_molds() {
            let count = index
                .molds(&TileLabel(def.label))
                .iter()
                .filter(|&&(_, projected)| projected == mold_id)
                .count();
            assert_eq!(
                1, count,
                "mold {mold_id:?} for label {:?} must project exactly once",
                def.label
            );
        }
        Ok(())
    }

    #[test]
    fn mold_lookup_checks_bounds() -> Result<(), Box<dyn Error>>
    {
        let (pbg, base) = synthetic_pbg()?;
        let plus = pbg.candidates(TileLabel("+"));
        assert_eq!(1, plus.len());
        let mold = pbg.mold(plus[0])?;
        assert_eq!("+", mold.label);
        assert_eq!(mold.prec, base);
        assert_eq!(Sort::Expression, mold.sort);

        assert_eq!(
            Err(PbgError::UnknownMold {
                id: MoldId::from(u32::MAX)
            }),
            pbg.mold(MoldId::from(u32::MAX))
        );
        Ok(())
    }

    #[test]
    fn mold_bounds_follow_context_nullability() -> Result<(), Box<dyn Error>>
    {
        let (pbg, base) = synthetic_pbg()?;
        let plus = pbg.candidates(TileLabel("+"))[0];
        let plus_bounds = pbg.bounds(plus)?;
        assert_eq!(
            (Bound::Value(base), Bound::Value(base)),
            plus_bounds,
            "an infix operator faces a sort on both sides"
        );

        let atom = pbg.candidates(TileLabel("x"))[0];
        let atom_bounds = pbg.bounds(atom)?;
        assert_eq!(
            (Bound::Root, Bound::Root),
            atom_bounds,
            "a bare atom faces no sort on either side"
        );
        Ok(())
    }

    #[test]
    fn rctx_steps_cross_adjacent_symbols() -> Result<(), Box<dyn Error>>
    {
        let (pbg, _base) = synthetic_pbg()?;
        let plus = pbg.mold(pbg.candidates(TileLabel("+"))[0])?;
        let left_steps = pbg.step(plus.rctx, Dir::Left)?;
        let left: Vec<StepSym> = left_steps.iter().map(|step| step.crossed).collect();
        let right_steps = pbg.step(plus.rctx, Dir::Right)?;
        let right: Vec<StepSym> = right_steps.iter().map(|step| step.crossed).collect();
        assert_eq!(left, vec![StepSym::Sort(Sort::Expression)]);
        assert_eq!(right, vec![StepSym::Sort(Sort::Expression)]);
        Ok(())
    }

    #[test]
    fn pbg_fingerprint_is_stable_and_folds_precdag() -> Result<(), Box<dyn Error>>
    {
        let first = built_in()?;
        let second = built_in()?;
        assert_eq!(
            BUILT_IN_FINGERPRINT,
            first.fingerprint(),
            "pinned fingerprint"
        );
        assert_eq!(
            first.fingerprint(),
            second.fingerprint(),
            "fingerprint is deterministic across builds"
        );
        assert_ne!(
            first.fingerprint(),
            GrammarFingerprint(u64::from(first.dag().fingerprint())),
            "the PBG fingerprint folds more than the precedence DAG"
        );
        Ok(())
    }

    #[test]
    fn generic_walk_projection_keeps_one_label_with_distinct_molds() -> Result<(), Box<dyn Error>>
    {
        let (pbg, _base) = synthetic_pbg()?;
        let index = walk_index(&pbg)?;
        // The atom "x" has one mold; the operator "+" has one mold; both project.
        let x_molds: BTreeSet<MoldId> = index
            .molds(&TileLabel("x"))
            .iter()
            .map(|&(_, mold)| mold)
            .collect();
        assert_eq!(1, x_molds.len());
        let plus_molds: BTreeSet<MoldId> = index
            .molds(&TileLabel("+"))
            .iter()
            .map(|&(_, mold)| mold)
            .collect();
        assert_eq!(1, plus_molds.len());
        assert!(x_molds.is_disjoint(&plus_molds));
        Ok(())
    }

    #[test]
    fn same_form_adjacency_is_the_eq_relation() -> Result<(), Box<dyn Error>>
    {
        // A bracket form yields exactly the `( ≐ )` adjacency, across the hole.
        let paren = paren_pbg()?;
        let open = paren.candidates(TileLabel("("))[0];
        let close = paren.candidates(TileLabel(")"))[0];
        assert_eq!(
            paren.adjacencies(),
            &[(open, close)],
            "( E ) must contribute exactly the ( ≐ ) same-form pair"
        );

        // A bare infix `E + E` contributes no adjacency: its single tile has no
        // consecutive same-form partner.
        let (infix, _base) = synthetic_pbg()?;
        assert!(
            infix.adjacencies().is_empty(),
            "a single-tile infix form has no same-form adjacency"
        );

        // The eq face of the comparison table over the bracket form is the pair.
        let index = walk_index(&paren)?;
        let table = comparison_table(&paren, &index);
        let eq_rows: BTreeSet<(MoldId, MoldId)> = table
            .iter()
            .filter(|row| row.cmp == Comparison::Equal)
            .map(|row| (row.left, row.right))
            .collect();
        assert!(eq_rows.contains(&(open, close)));
        Ok(())
    }

    /// A synthetic PBG with a bracket form `( E )` and a bare atom `x`.
    fn paren_pbg() -> Result<Pbg, Box<dyn Error>>
    {
        let mut spec = PrecSpec::new();
        let base = spec.insert("base", None)?;
        let dag = PrecDag::build(&spec)?;
        let pbg = Pbg::build(dag, vec![
            Rule::new(
                RuleName("group"),
                Sort::Expression,
                base,
                Regex::seq([
                    Regex::tile(TileLabel("(")),
                    Regex::sort(Sort::Expression),
                    Regex::tile(TileLabel(")")),
                ]),
            ),
            Rule::new(
                RuleName("atom"),
                Sort::Expression,
                base,
                Regex::tile(TileLabel("x")),
            ),
        ])?;
        Ok(pbg)
    }

    fn synthetic_pbg() -> Result<(Pbg, Prec), Box<dyn Error>>
    {
        let mut spec = PrecSpec::new();
        let base = spec.insert("base", None)?;
        let dag = PrecDag::build(&spec)?;
        let pbg = Pbg::build(dag, vec![
            Rule::new(
                RuleName("infix"),
                Sort::Expression,
                base,
                Regex::seq([
                    Regex::sort(Sort::Expression),
                    Regex::tile(TileLabel("+")),
                    Regex::sort(Sort::Expression),
                ]),
            ),
            Rule::new(
                RuleName("atom"),
                Sort::Expression,
                base,
                Regex::tile(TileLabel("x")),
            ),
        ])?;
        Ok((pbg, base))
    }

    #[test]
    fn comparison_table_coheres_with_precedence() -> Result<(), Box<dyn Error>>
    {
        // Theorem 3.1 (Annotation-Comparison Coherence), comparable-pair form,
        // over the generated table: `t_L ⋖ t_R ⟺ p_L <_s p_R` and
        // `t_L ⋗ t_R ⟺ p_L >_s p_R`, keyed by the mediating sort; incomparable
        // (cross-sort) pairs derive no comparison.
        let pbg = built_in()?;
        let index = walk_index(&pbg)?;
        let table = comparison_table(&pbg, &index);
        let dag = pbg.dag();

        // Soundness: each derived non-equal row agrees with the precedence DAG,
        // and never crosses sorts (so incomparable pairs derive nothing).
        for row in &table {
            let left = pbg.mold(row.left)?;
            let right = pbg.mold(row.right)?;
            match row.cmp {
                | Comparison::Yields => {
                    assert_eq!(left.sort, right.sort, "⋖ mediates one sort");
                    assert_eq!(row.sort, left.sort);
                    assert!(
                        bool::from(dag.lt(left.prec, right.prec, None)),
                        "⋖ requires p_L <_s p_R"
                    );
                },
                | Comparison::Takes => {
                    assert_eq!(left.sort, right.sort, "⋗ mediates one sort");
                    assert_eq!(row.sort, left.sort);
                    assert!(
                        bool::from(dag.gt(left.prec, right.prec, None)),
                        "⋗ requires p_L >_s p_R"
                    );
                },
                | Comparison::Equal => {
                    assert_eq!(left.sort, right.sort, "≐ is same-form, hence one sort");
                },
            }
        }

        // Completeness: every comparable same-sort form-group pair derives the
        // exact matching relation over its representatives.
        let reps = group_reps(&pbg);
        let mut expected: BTreeSet<(MoldId, MoldId, Comparison)> = BTreeSet::new();
        for (&(sort_l, prec_l), &rep_l) in &reps {
            for (&(sort_r, prec_r), &rep_r) in &reps {
                if sort_l != sort_r {
                    continue;
                }
                if bool::from(dag.lt(prec_l, prec_r, None)) {
                    expected.insert((rep_l, rep_r, Comparison::Yields));
                }
                else if bool::from(dag.gt(prec_l, prec_r, None)) {
                    expected.insert((rep_l, rep_r, Comparison::Takes));
                }
            }
        }
        let observed: BTreeSet<(MoldId, MoldId, Comparison)> = table
            .iter()
            .filter(|row| row.cmp != Comparison::Equal)
            .map(|row| (row.left, row.right, row.cmp))
            .collect();
        assert_eq!(
            observed, expected,
            "the ⋖/⋗ table is exactly the comparable form-group precedence matrix"
        );
        Ok(())
    }

    /// Map every `(sort, prec)` form group to its canonical representative
    /// mold.
    fn group_reps(pbg: &Pbg) -> BTreeMap<(Sort, Prec), MoldId>
    {
        let mut reps: BTreeMap<(Sort, Prec), MoldId> = BTreeMap::new();
        for (id, def) in pbg.iter_molds() {
            reps.entry((def.sort, def.prec)).or_insert(id);
        }
        reps
    }

    #[test]
    fn comparison_table_is_conflict_free() -> Result<(), Box<dyn Error>>
    {
        // Conflict-freedom (Assumption-3 companion): at most one comparison per
        // ordered terminal pair. Any `≐`-conflict (dangling-else family) would
        // surface as a second, differing relation for a pair; there are none.
        let pbg = built_in()?;
        let index = walk_index(&pbg)?;
        let table = comparison_table(&pbg, &index);

        let mut seen: BTreeMap<(MoldId, MoldId), Comparison> = BTreeMap::new();
        for row in &table {
            if let Some(previous) = seen.insert((row.left, row.right), row.cmp) {
                assert_eq!(
                    previous, row.cmp,
                    "pair ({:?}, {:?}) derives two distinct comparisons",
                    row.left, row.right
                );
            }
        }
        Ok(())
    }

    #[test]
    fn walks_terminate_for_every_symbol_pair() -> Result<(), Box<dyn Error>>
    {
        // `walks(l, r)` is total: the engine's closure guarantees a (possibly
        // empty) answer for every ordered pair of the full real vertex set.
        let pbg = built_in()?;
        let index = walk_index(&pbg)?;
        let ends: Vec<End<_>> = index.ends().to_vec();
        let mut probed = 0_u64;
        for left in &ends {
            for right in &ends {
                let _left_walks = index.walks(Dir::Left, left, right);
                let _right_walks = index.walks(Dir::Right, left, right);
                probed = probed.saturating_add(1);
            }
        }
        let count = u64::try_from(ends.len())?;
        assert_eq!(
            probed,
            count.saturating_mul(count),
            "every ordered vertex pair was probed and terminated"
        );
        Ok(())
    }

    #[test]
    fn walk_lengths_respect_the_chain_cap() -> Result<(), Box<dyn Error>>
    {
        // The cap is enforced by the engine's `guard_cap` debug-assert; a walk
        // over the cap surfaces as `ChainLengthExceeded`, never silent
        // truncation. A successful build proves the invariant, and every
        // materialised walk is at or below the cap.
        let pbg = built_in()?;
        let index = walk_index(&pbg)?;
        let ends: Vec<End<_>> = index.ends().to_vec();
        for left in &ends {
            for right in &ends {
                for dir in [Dir::Left, Dir::Right] {
                    for walk in index.walks(dir, left, right) {
                        assert!(
                            walk.chain_len()? <= WalkChainLength::from(MAX_WALK_CHAIN_LEN),
                            "walk exceeds the configured chain cap"
                        );
                    }
                }
            }
        }
        Ok(())
    }

    #[test]
    fn reachable_multi_mold_metric_and_seen_key() -> Result<(), Box<dyn Error>>
    {
        // The reachable multi-mold metric and the seen-key verdict are pinned
        // here. The count reflects the labels that carry more than one
        // reachable mold across the built-in surface: deleting the dead
        // composite shell rules holds `shell_word`, `command_name`,
        // `file_descriptor`, `environment_assignment`, and `negation` to a
        // single reachable mold each (the molder's sole-admissible fast path
        // for every shell word / fd token), and first-class holes keep
        // `hole_name` single-mold (the `?` hole's optional tail).
        //
        // The ruled circuit block form adds eight: the four arrow-grid glyphs
        // (`-->` / `<->` / `==>` / `<=>`, each admissible at every arrow
        // position because the confirmation is the checker's, not the
        // grammar's), the contextual body leads `node` / `feed`, the item-lead
        // `oper`, and `data` — which crosses from single- to multi-mold because
        // a `sign` member and a parameter-telescope binder join the datatype
        // declaration. `sign` and `sort` each stay single-mold. The nested
        // member makes `module` cross from one reachable mold to two.
        //
        // The sealing rung adds two. `:>` arrives multi-mold outright, being
        // admissible at both the outer and the nested module ascription; and
        // `type` crosses from single- to multi-mold, having led only the
        // `extern` block's inline type member and now leading an abstract type
        // component in every module signature. Both are the same fact seen
        // twice: a signature is one form admitted at two sites under two
        // ascriptions.
        let pbg = built_in()?;
        let index = walk_index(&pbg)?;
        let reachable = reachable_molds(&pbg, &index);

        let total: usize = reachable.values().map(BTreeSet::len).sum();
        assert_eq!(
            BUILT_IN_MOLD_COUNT.0, total,
            "every declared mold is reachable through the real walk index"
        );

        let multi = reachable.values().filter(|molds| molds.len() > 1).count();
        assert_eq!(
            75,
            multi,
            "reachable multi-mold labels (PBG {fingerprint:#018x})",
            fingerprint = BUILT_IN_FINGERPRINT.0
        );

        let actual_seen_key = seen_key_verdict(&pbg)?;
        assert_eq!(
            SeenKeyVerdict::Equivalent,
            actual_seen_key,
            "the direct-row front-end runs no swing closure, so the seen-key keyings agree"
        );

        // The reachable projection is exhaustive and non-duplicating: every mold
        // appears exactly once under its own label.
        for (mold_id, def) in pbg.iter_molds() {
            assert!(
                reachable
                    .get(def.label)
                    .is_some_and(|molds| molds.contains(&mold_id)),
                "mold {mold_id:?} for label {:?} must be reachable",
                def.label
            );
        }
        Ok(())
    }
}
