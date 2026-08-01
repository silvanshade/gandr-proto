//! A **second, minimal [`CellAlphabet`] inhabitant** — the adequacy witness
//! that the engines are alphabet-polymorphic, not sequent special-casing
//! (metatheory roadmap spike S1).
//!
//! The toy alphabet is a single-sorted first-order term language (`Zero` /
//! `Succ` / `Add` with metavariables), instantiated **externally** (from the
//! integration-test crate, exactly the path a future shape-layer or directed
//! rule-layer alphabet takes). The three engines run over it unmodified:
//! the enumerator finds a real composition overlap, the normalizer rewrites a
//! ground term, and completion orients a divergence into a derived cell and
//! certifies the resulting join — with the budgeted decline intact.

#[cfg(test)]
mod tests
{
    use alloc::boxed::Box;
    use alloc::collections::BTreeMap;
    use alloc::string::ToString as _;
    use alloc::vec::Vec;

    use gandr_theory_computads::Cell;
    use gandr_theory_computads::CellAlphabet;
    use gandr_theory_computads::CellInvertibility;
    use gandr_theory_computads::CellStore;
    use gandr_theory_computads::CompletionBudget;
    use gandr_theory_computads::CompletionOutcome;
    use gandr_theory_computads::FiringPermission;
    use gandr_theory_computads::OverlapKind;
    use gandr_theory_computads::SeamRole;
    use gandr_theory_computads::SubstitutionDecision;
    use gandr_theory_computads::complete;
    use gandr_theory_computads::derive_fused;
    use gandr_theory_computads::enumerate_overlaps;
    use gandr_theory_computads::normalize;

    extern crate alloc;

    /// The toy alphabet marker — a minimal single-sorted term language.
    #[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
    struct ToyAlphabet;

    /// A borrowed toy variable name (the semantic boundary for `str`).
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug)]
    struct ToyNameRef<'source>(&'source str);

    /// A toy metavariable name.
    #[repr(transparent)]
    #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    struct ToyVar(Box<str>);

    impl ToyVar
    {
        /// A variable from a name.
        fn named(name: ToyNameRef<'_>) -> Self
        {
            Self(name.0.into())
        }
    }

    /// The node count of a toy term (the reduction-order measure).
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
    struct ToySize(usize);

    /// A toy term — `Zero` / `Succ` / `Add` with metavariable and
    /// skolem-constant leaves.
    #[derive(Clone, Debug, Eq, Hash, PartialEq)]
    enum Toy
    {
        /// A metavariable leaf.
        Var(ToyVar),
        /// A skolem constant leaf (replay freshening; irreducible).
        Konst(ToyVar),
        /// The nullary constructor.
        Zero,
        /// The unary constructor.
        Succ(Box<Self>),
        /// The binary operation.
        Add(Box<Self>, Box<Self>),
    }

    impl Toy
    {
        /// A metavariable leaf from a name.
        fn var(name: ToyNameRef<'_>) -> Self
        {
            Self::Var(ToyVar::named(name))
        }

        /// A unary constructor application.
        fn succ(arg: Self) -> Self
        {
            Self::Succ(Box::new(arg))
        }

        /// A binary operation application.
        fn add(
            lhs: Self,
            rhs: Self,
        ) -> Self
        {
            Self::Add(Box::new(lhs), Box::new(rhs))
        }

        /// The immediate children, left to right.
        ///
        /// # Contract
        /// - ensures: empty for a leaf; `[arg]` for `Succ`; `[lhs, rhs]` for
        ///   `Add`.
        /// - panics: none.
        fn children(&self) -> Vec<Self>
        {
            match *self {
                | Self::Var(_) | Self::Konst(_) | Self::Zero => Vec::new(),
                | Self::Succ(ref arg) => alloc::vec![(**arg).clone()],
                | Self::Add(ref lhs, ref rhs) => alloc::vec![(**lhs).clone(), (**rhs).clone()],
            }
        }

        /// Rebuild with the immediate children replaced, preserving the shape.
        ///
        /// # Contract
        /// - ensures: `Some` iff the replacement count matches the arity.
        /// - panics: none.
        fn with_children(
            &self,
            children: &[Self],
        ) -> Option<Self>
        {
            match *self {
                | Self::Var(_) | Self::Konst(_) | Self::Zero => {
                    children.is_empty().then(|| self.clone())
                },
                | Self::Succ(_) => {
                    let arg = children.first()?.clone();
                    (children.len() == 1).then(|| Self::succ(arg))
                },
                | Self::Add(..) => {
                    if children.len() != 2 {
                        return None;
                    }
                    let lhs = children.first()?.clone();
                    let rhs = children.get(1)?.clone();
                    Some(Self::add(lhs, rhs))
                },
            }
        }

        /// The node count (the reduction-order measure).
        ///
        /// # Contract
        /// - ensures: a positive count, monotone under subterm inclusion.
        /// - panics: none.
        fn size(&self) -> ToySize
        {
            let mut count = 0_usize;
            let mut stack = alloc::vec![self.clone()];
            while let Some(node) = stack.pop() {
                count = count.saturating_add(1);
                stack.extend(node.children());
            }
            ToySize(count)
        }
    }

    /// A toy position — a path of child indices (root is the empty path).
    #[repr(transparent)]
    #[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
    struct ToyPos(Box<[usize]>);

    /// A toy substitution — an ordered map from metavariables to terms.
    #[repr(transparent)]
    #[derive(Clone, Debug, Default, Eq, PartialEq)]
    struct ToySubst(BTreeMap<ToyVar, Toy>);

    /// The toy orientation tag.
    #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
    enum ToyOrient
    {
        /// Given with the cell.
        Given,
        /// Chosen by the completion reduction order.
        Derived,
    }

    /// The toy provenance tag.
    #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
    enum ToyProv
    {
        /// A rule cell.
        Rule,
        /// Derived by completion.
        Derived,
    }

    /// The toy metadata — the toy alphabet tracks no per-variable data.
    #[derive(Clone, Debug, Eq, Hash, PartialEq)]
    struct ToyMeta;

    /// Collect `term`'s metavariables, left to right with repeats.
    ///
    /// # Contract
    /// - ensures: one entry per [`Toy::Var`] occurrence, in left-to-right
    ///   order.
    /// - panics: none.
    fn toy_metavars(term: &Toy) -> Vec<ToyVar>
    {
        let mut out = Vec::new();
        let mut stack = alloc::vec![term.clone()];
        while let Some(node) = stack.pop() {
            match node {
                | Toy::Var(var) => out.push(var),
                | other => stack.extend(other.children().into_iter().rev()),
            }
        }
        out
    }

    /// Bind `var` to `value`, failing on a conflicting rebind.
    ///
    /// # Contract
    /// - ensures: a positive decision iff `var` was unbound or bound to
    ///   `value`.
    /// - panics: none.
    fn bind(
        subst: &mut ToySubst,
        var: ToyVar,
        value: Toy,
    ) -> SubstitutionDecision
    {
        SubstitutionDecision::from(match subst.0.get(&var) {
            | Some(existing) => *existing == value,
            | None => {
                subst.0.insert(var, value);
                true
            },
        })
    }

    /// Resolve a term's head variable through `subst` (a walk).
    ///
    /// # Contract
    /// - ensures: the first non-variable or unbound head of the binding chain.
    /// - panics: none.
    fn walk(
        term: &Toy,
        subst: &ToySubst,
    ) -> Toy
    {
        let mut current = term.clone();
        while let Toy::Var(ref var) = current {
            match subst.0.get(var) {
                | Some(bound) => current = bound.clone(),
                | None => break,
            }
        }
        current
    }

    /// Whether `var` occurs in `term` after walking through `subst` (the
    /// occurs-check).
    ///
    /// # Contract
    /// - ensures: positive iff `var` reaches a leaf of the fully-walked `term`.
    /// - panics: none.
    fn occurs(
        var: &ToyVar,
        term: &Toy,
        subst: &ToySubst,
    ) -> SubstitutionDecision
    {
        let mut stack = alloc::vec![term.clone()];
        while let Some(node) = stack.pop() {
            match walk(&node, subst) {
                | Toy::Var(found) if found == *var => return SubstitutionDecision::from(true),
                | other => stack.extend(other.children()),
            }
        }
        SubstitutionDecision::from(false)
    }

    impl CellAlphabet for ToyAlphabet
    {
        type Cmd = Toy;
        type Hole = ToyVar;
        type Meta = ToyMeta;
        type Orientation = ToyOrient;
        type Pos = ToyPos;
        type Provenance = ToyProv;
        type Subst = ToySubst;
        type Var = ToyVar;

        fn match_cmd(
            pattern: &Self::Cmd,
            target: &Self::Cmd,
            subst: &mut Self::Subst,
        ) -> SubstitutionDecision
        {
            let mut goals = alloc::vec![(pattern.clone(), target.clone())];
            while let Some((pat, ground)) = goals.pop() {
                match (pat, ground) {
                    | (Toy::Var(var), value) => {
                        if !bool::from(bind(subst, var, value)) {
                            return SubstitutionDecision::from(false);
                        }
                    },
                    | (Toy::Zero, Toy::Zero) => {},
                    | (Toy::Succ(p), Toy::Succ(g)) => goals.push((*p, *g)),
                    | (Toy::Add(pl, pr), Toy::Add(gl, gr)) => {
                        goals.push((*pr, *gr));
                        goals.push((*pl, *gl));
                    },
                    | _ => return SubstitutionDecision::from(false),
                }
            }
            SubstitutionDecision::from(true)
        }

        fn unify_cmd(
            lhs: &Self::Cmd,
            rhs: &Self::Cmd,
            subst: &mut Self::Subst,
        ) -> SubstitutionDecision
        {
            let mut goals = alloc::vec![(lhs.clone(), rhs.clone())];
            while let Some((lhs, rhs)) = goals.pop() {
                let left = walk(&lhs, subst);
                let right = walk(&rhs, subst);
                match (left, right) {
                    | (Toy::Var(x), Toy::Var(y)) if x == y => {},
                    | (Toy::Var(x), other) | (other, Toy::Var(x)) => {
                        if bool::from(occurs(&x, &other, subst)) {
                            return SubstitutionDecision::from(false);
                        }
                        if !bool::from(bind(subst, x, other)) {
                            return SubstitutionDecision::from(false);
                        }
                    },
                    | (Toy::Konst(a), Toy::Konst(b)) if a == b => {},
                    | (Toy::Zero, Toy::Zero) => {},
                    | (Toy::Succ(a), Toy::Succ(b)) => goals.push((*a, *b)),
                    | (Toy::Add(al, ar), Toy::Add(bl, br)) => {
                        goals.push((*ar, *br));
                        goals.push((*al, *bl));
                    },
                    | _ => return SubstitutionDecision::from(false),
                }
            }
            SubstitutionDecision::from(true)
        }

        fn apply_subst(
            subst: &Self::Subst,
            cmd: &Self::Cmd,
        ) -> Self::Cmd
        {
            // Passes to a fixpoint, so triangular bindings resolve fully; each
            // pass strictly reduces the number of bound variables still
            // occurring in the result, bounding the loop by the binding count.
            let mut current = cmd.clone();
            for _ in 0 ..= subst.0.len() {
                let mut stack = alloc::vec![(current.clone(), ToyPos::default())];
                let mut next = current.clone();
                let mut changed = false;
                while let Some((node, pos)) = stack.pop() {
                    if let Toy::Var(ref var) = node
                        && let Some(bound) = subst.0.get(var)
                    {
                        let Some(rebuilt) = Self::splice_cmd_at(&next, &pos, bound.clone())
                        else {
                            continue;
                        };
                        next = rebuilt;
                        changed = true;
                        continue;
                    }
                    for (index, child) in node.children().into_iter().enumerate() {
                        let mut path = pos.0.to_vec();
                        path.push(index);
                        stack.push((child, ToyPos(path.into_boxed_slice())));
                    }
                }
                current = next;
                if !changed {
                    return current;
                }
            }
            current
        }

        fn metavariables(cmd: &Self::Cmd) -> Vec<Self::Var>
        {
            toy_metavars(cmd)
        }

        fn command_positions(cmd: &Self::Cmd) -> Vec<Self::Pos>
        {
            // Every subterm is a toy command; positions come out outer-to-inner.
            let mut out = Vec::new();
            let mut work = alloc::vec![(ToyPos::default(), cmd.clone())];
            while let Some((pos, node)) = work.pop() {
                out.push(pos.clone());
                for (index, child) in node.children().into_iter().enumerate() {
                    let mut path = pos.0.to_vec();
                    path.push(index);
                    work.push((ToyPos(path.into_boxed_slice()), child));
                }
            }
            out.sort_unstable_by_key(|pos| pos.0.len());
            out
        }

        fn root_position() -> Self::Pos
        {
            ToyPos::default()
        }

        fn subterm_cmd_at(
            cmd: &Self::Cmd,
            pos: &Self::Pos,
        ) -> Option<Self::Cmd>
        {
            let mut cursor = cmd.clone();
            for &step in &pos.0 {
                let children = cursor.children();
                cursor = children.get(step)?.clone();
            }
            Some(cursor)
        }

        fn splice_cmd_at(
            cmd: &Self::Cmd,
            pos: &Self::Pos,
            replacement: Self::Cmd,
        ) -> Option<Self::Cmd>
        {
            let mut cursor = cmd.clone();
            let mut frames: Vec<(Toy, Vec<Toy>, usize)> = Vec::new();
            for &step in &pos.0 {
                let children = cursor.children();
                let child = children.get(step)?.clone();
                frames.push((cursor, children, step));
                cursor = child;
            }
            let mut rebuilt = replacement;
            for (parent, mut children, step) in frames.into_iter().rev() {
                let destination = children.get_mut(step)?;
                *destination = rebuilt;
                rebuilt = parent.with_children(&children)?;
            }
            Some(rebuilt)
        }

        fn reduction_cmp(
            lhs: &Self::Cmd,
            rhs: &Self::Cmd,
        ) -> core::cmp::Ordering
        {
            lhs.size().cmp(&rhs.size())
        }

        fn rename_apart(
            anchor: (&Self::Cmd, &Self::Cmd),
            renamed: (&Self::Cmd, &Self::Cmd),
        ) -> (Self::Cmd, Self::Cmd)
        {
            let mut taken: Vec<ToyVar> = toy_metavars(anchor.0);
            taken.extend(toy_metavars(anchor.1));
            let mut mapping: Vec<(ToyVar, ToyVar)> = Vec::new();
            let mut occurrences = toy_metavars(renamed.0);
            occurrences.extend(toy_metavars(renamed.1));
            for var in occurrences {
                if mapping.iter().any(|pair| pair.0 == var) {
                    continue;
                }
                let mut fresh = var.clone();
                while taken.contains(&fresh) {
                    let mut name = fresh.0.to_string();
                    name.push('\'');
                    fresh = ToyVar(name.into_boxed_str());
                }
                taken.push(fresh.clone());
                mapping.push((var, fresh));
            }
            (
                rename_toy(renamed.0, &mapping),
                rename_toy(renamed.1, &mapping),
            )
        }

        fn skolemize(cmd: &Self::Cmd) -> Self::Cmd
        {
            let mut out = cmd.clone();
            for var in toy_metavars(cmd) {
                let mut stack = alloc::vec![(out.clone(), ToyPos::default())];
                while let Some((node, pos)) = stack.pop() {
                    if matches!(node, Toy::Var(ref found) if *found == var) {
                        let konst = Toy::Konst(var.clone());
                        let Some(rebuilt) = Self::splice_cmd_at(&out, &pos, konst)
                        else {
                            continue;
                        };
                        out = rebuilt;
                        continue;
                    }
                    for (index, child) in node.children().into_iter().enumerate() {
                        let mut path = pos.0.to_vec();
                        path.push(index);
                        stack.push((child, ToyPos(path.into_boxed_slice())));
                    }
                }
            }
            out
        }

        fn hole_of(var: &Self::Var) -> Self::Hole
        {
            var.clone()
        }

        fn completion_certificate(provenance: &Self::Provenance) -> CellInvertibility
        {
            CellInvertibility::from(matches!(provenance, ToyProv::Derived))
        }

        fn derive_meta(
            _lhs: &Self::Cmd,
            _rhs: &Self::Cmd,
            _invertible: CellInvertibility,
        ) -> Self::Meta
        {
            ToyMeta
        }

        fn hole_flow(
            _meta: &Self::Meta,
            _hole: &Self::Hole,
        ) -> Vec<(Self::Var, SeamRole)>
        {
            // The toy alphabet has no polarity split: no flow roles to report.
            Vec::new()
        }

        fn may_fire(
            _provenance: &Self::Provenance,
            _target: &Self::Cmd,
        ) -> FiringPermission
        {
            FiringPermission::from(true)
        }

        fn derived_orientation() -> Self::Orientation
        {
            ToyOrient::Derived
        }

        fn derived_provenance() -> Self::Provenance
        {
            ToyProv::Derived
        }
    }

    /// Rename a toy term's metavariables through `mapping`.
    ///
    /// # Contract
    /// - ensures: every metavariable in `mapping`'s domain is replaced by its
    ///   image; others are left in place.
    /// - panics: none.
    fn rename_toy(
        term: &Toy,
        mapping: &[(ToyVar, ToyVar)],
    ) -> Toy
    {
        // Iterative traversal over the original shape; splicing preserves it,
        // so the recorded positions stay valid against the rebuilt term.
        let mut out = term.clone();
        let mut stack = alloc::vec![(term.clone(), ToyPos::default())];
        while let Some((node, pos)) = stack.pop() {
            if let Toy::Var(ref var) = node {
                if let Some(pair) = mapping.iter().find(|pair| pair.0 == *var) {
                    let renamed = Toy::Var(pair.1.clone());
                    if let Some(rebuilt) = ToyAlphabet::splice_cmd_at(&out, &pos, renamed) {
                        out = rebuilt;
                    }
                }
                continue;
            }
            for (index, child) in node.children().into_iter().enumerate() {
                let mut path = pos.0.to_vec();
                path.push(index);
                stack.push((child, ToyPos(path.into_boxed_slice())));
            }
        }
        out
    }

    /// A toy rule cell over the toy alphabet.
    ///
    /// # Contract
    /// - ensures: a `Given` / `Rule` cell with the two faces.
    /// - panics: none.
    fn toy_cell(
        lhs: Toy,
        rhs: Toy,
    ) -> Cell<ToyAlphabet>
    {
        Cell::new(lhs, rhs, ToyOrient::Given, ToyProv::Rule)
    }

    /// (add-Z): `Add(Zero, x) ~> x`.
    fn add_z() -> Cell<ToyAlphabet>
    {
        toy_cell(
            Toy::add(Toy::Zero, Toy::var(ToyNameRef("x"))),
            Toy::var(ToyNameRef("x")),
        )
    }

    /// (add-S): `Add(Succ(m), n) ~> Succ(Add(m, n))`.
    fn add_s() -> Cell<ToyAlphabet>
    {
        toy_cell(
            Toy::add(
                Toy::succ(Toy::var(ToyNameRef("m"))),
                Toy::var(ToyNameRef("n")),
            ),
            Toy::succ(Toy::add(
                Toy::var(ToyNameRef("m")),
                Toy::var(ToyNameRef("n")),
            )),
        )
    }

    #[test]
    fn the_enumerator_finds_the_toy_composition_overlap()
    {
        let mut store = CellStore::new();
        let z = store.insert(add_z());
        let s = store.insert(add_s());
        let overlaps = enumerate_overlaps(&store);
        let composition = overlaps
            .iter()
            .find(|o| o.kind == OverlapKind::Composition && o.left == s && o.right == z)
            .expect("add-S's RHS subterm Add(m, n) unifies with add-Z's LHS");
        // Peak: Add(Succ(Zero), x) — the superposition at m = Zero.
        assert_eq!(
            Toy::add(Toy::succ(Toy::Zero), Toy::var(ToyNameRef("x"))),
            composition.peak,
            "the superposition instantiates m to Zero"
        );
        // Composite: Succ(x) — left fires, then add-Z at the seam.
        let composite = composition.composite(&store).expect("the composite exists");
        assert_eq!(
            Toy::succ(Toy::var(ToyNameRef("x"))),
            composite,
            "the fused right-hand side drops the intermediate Add"
        );
        // The derived fused cell replays as a certificate.
        let (_fused, tracelet) =
            derive_fused(composition, &mut store).expect("the fused cell derives");
        assert!(
            bool::from(tracelet.replay(&store)),
            "the toy fused≡two-step certificate replays"
        );
    }

    #[test]
    fn the_normalizer_runs_over_the_toy_alphabet()
    {
        let mut store = CellStore::new();
        store.insert(add_z());
        store.insert(add_s());
        let term = Toy::add(Toy::succ(Toy::Zero), Toy::Zero);
        let out = normalize(&store, &term, 16_usize.into());
        assert_eq!(Toy::succ(Toy::Zero), out.normal, "add-S then add-Z");
        assert_eq!(2, out.path.len(), "two steps");
        assert!(!out.exhausted, "a normal form was reached");
    }

    #[test]
    fn completion_orients_and_certifies_over_the_toy_alphabet()
    {
        // c1: Add(Zero, x) ~> x and c2: Add(Zero, x) ~> Add(x, Zero) share a
        // left-hand side — a confluence critical pair whose reducts diverge.
        let mut store = CellStore::new();
        store.insert(add_z());
        store.insert(toy_cell(
            Toy::add(Toy::Zero, Toy::var(ToyNameRef("x"))),
            Toy::add(Toy::var(ToyNameRef("x")), Toy::Zero),
        ));
        let outcome = complete(
            store,
            CompletionBudget::new(64_usize.into(), 16_usize.into(), 64_usize.into()),
        );
        let CompletionOutcome::Completed {
            store: completed,
            derived,
            certificates,
        } = outcome
        else {
            panic!("the toy system completes within budget");
        };
        assert_eq!(
            1,
            derived.len(),
            "the divergence oriented into one new cell"
        );
        assert!(
            !certificates.is_empty(),
            "the derived cell joined the original pair"
        );
        assert!(
            certificates
                .iter()
                .all(|tracelet| bool::from(tracelet.replay(&completed))),
            "every toy certificate replays"
        );
    }

    #[test]
    fn a_starved_toy_budget_declines_with_pending()
    {
        let mut store = CellStore::new();
        store.insert(add_z());
        store.insert(toy_cell(
            Toy::add(Toy::Zero, Toy::var(ToyNameRef("x"))),
            Toy::add(Toy::var(ToyNameRef("x")), Toy::Zero),
        ));
        let outcome = complete(
            store,
            CompletionBudget::new(0_usize.into(), 16_usize.into(), 64_usize.into()),
        );
        match outcome {
            | CompletionOutcome::Declined { pending, .. } => {
                assert!(!pending.is_empty(), "the pending overlaps are carried");
            },
            | CompletionOutcome::Completed { .. } => {
                panic!("a zero step budget must decline")
            },
        }
    }
}
