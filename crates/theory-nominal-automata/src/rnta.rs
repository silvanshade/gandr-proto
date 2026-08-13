//! The **RNTA** model inventory — regular nominal tree automata (RNTA Def
//! 4.1, Prucker–Schröder, CONCUR 2024) — the tree/term model of the family
//! (design doc §3).
//!
//! RNTA process α-equivalence classes of **nominal Σ-terms** (RNTA Def
//! 3.1): nodes carry a symbol and either a free name (`a.f(t₁,…,tₙ)`) or an
//! allocated bound name scoped over the children (`νa.f(t₁,…,tₙ)`) — the
//! term language of λ- and π-calculus terms and XML-like structured data
//! (RNTA Ex 3.5, 4.6, 4.7). The automaton `A = (Q, Δ, q₀)` has an
//! orbit-finite state set, an equivariant set of rewrite rules
//! `q(γ.f(x₁,…,xₙ)) → γ.f(q₁(x₁),…,qₙ(xₙ))` with `γ ∈ Ā`, and no final
//! set: acceptance is the existence of a top-down run that rewrites the
//! whole term (leaves are arity-zero rules). Under global/branchwise
//! freshness RNTA generalize session automata; under local freshness they
//! are a lossiness-characterized subclass of register tree automata (RNTA
//! §4 intro, Remark 5.6).
//!
//! The representation shares the family's strong-nominal-set handle
//! (design doc §6.4): states are [`Configuration`]s, and each
//! [`RntaRule`] is one defunctionalized rewrite orbit — a source control,
//! the node symbol, the name shape `γ` ([`NodeKind`]), and one
//! [`ChildTarget`] per child position. The support monotonicity laws (RNTA
//! Lemma 4.2) are enforced by validation: a free-name node reads its name
//! from a register, and only an allocating node's children may receive
//! [`Transfer::Allocated`].
//!
//! This module is the type-level inventory — terms, transitions, the
//! free-names function — as data. The RNTA's decision procedures are
//! catalogue entries ([`crate::catalogue`]); its top-down membership and
//! the S-restriction to classical NFTA (RNTA Lemma 6.2, Thm 6.3) are
//! recorded residuals of this slice.

use alloc::collections::BTreeSet;
use alloc::vec::Vec;

use crate::Atom;
use crate::Sort;
use crate::handle::Arity;
use crate::handle::AutomatonError;
use crate::handle::Configuration;
use crate::handle::Control;
use crate::handle::Controls;
use crate::handle::Degree;
use crate::handle::Register;
use crate::handle::Transfer;

/// A **nominal Σ-term** (RNTA Def 3.1): `t ::= a.f(t₁,…,tₙ) | νa.f(t₁,…,tₙ)`.
///
/// The type is generic over the node's symbol `F` (the signature Σ) beside
/// the atom sort `S`; words are the unary-signature special case (RNTA
/// Remark 3.2).
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Term<S, F>
{
    /// `a.f(t₁,…,tₙ)` — a node whose name occurs **free** (RNTA Def 3.1).
    Free
    {
        /// The free name of the node.
        name: Atom<S>,
        /// The node's signature symbol.
        symbol: F,
        /// The child terms.
        children: Vec<Self>,
    },
    /// `νa.f(t₁,…,tₙ)` — a node **allocating** a fresh bound name, scoped
    /// over the children (RNTA Def 3.1).
    Bound
    {
        /// The allocated bound name of the node.
        name: Atom<S>,
        /// The node's signature symbol.
        symbol: F,
        /// The child terms.
        children: Vec<Self>,
    },
}

impl<S, F> Term<S, F>
where
    S: Sort,
{
    /// A free-name node `a.f(t₁,…,tₙ)`.
    #[inline]
    #[must_use]
    pub fn free(
        name: Atom<S>,
        symbol: F,
        children: Vec<Self>,
    ) -> Self
    {
        return Self::Free {
            name,
            symbol,
            children,
        };
    }

    /// An allocating node `νa.f(t₁,…,tₙ)`.
    #[inline]
    #[must_use]
    pub fn bound(
        name: Atom<S>,
        symbol: F,
        children: Vec<Self>,
    ) -> Self
    {
        return Self::Bound {
            name,
            symbol,
            children,
        };
    }

    /// The name the node carries, free or bound.
    #[inline]
    #[must_use]
    pub fn name(&self) -> Atom<S>
    {
        return match *self {
            | Self::Free { name, .. } | Self::Bound { name, .. } => name,
        };
    }

    /// The node's signature symbol.
    #[inline]
    #[must_use]
    pub fn symbol(&self) -> &F
    {
        return match *self {
            | Self::Free { ref symbol, .. } | Self::Bound { ref symbol, .. } => symbol,
        };
    }

    /// The child terms.
    #[inline]
    #[must_use]
    pub fn children(&self) -> &[Self]
    {
        return match *self {
            | Self::Free { ref children, .. } | Self::Bound { ref children, .. } => children,
        };
    }
}

impl<S, F> Term<S, F>
where
    S: Sort + Ord,
{
    /// The **free names** `FN(t)` of the term (RNTA Def 3.4): the names of
    /// free-name nodes, less the names bound by an enclosing `ν`.
    ///
    /// The computation is an explicit-stack post-order (no input-scaled
    /// recursion, ADR-47): each completed node combines its children's free
    /// -name sets, adds its own name when free, and removes its own name
    /// when binding.
    ///
    /// # Contract
    /// - provides: exactly the names with a free occurrence in the term.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — the post-order is separated from a naive
    ///   collect-all by binder shadowing and by free/bound interleaving,
    ///   exercised on one nested term with a shadowed and an unshadowed name.
    /// - witness: `rnta::tests::free_names_respects_binder_shadowing`
    #[inline]
    #[must_use]
    pub fn free_names(&self) -> BTreeSet<Atom<S>>
    {
        enum Work<'tree, S, F>
        {
            Enter(&'tree Term<S, F>),
            Exit(&'tree Term<S, F>),
        }

        let mut work: Vec<Work<'_, S, F>> = alloc::vec![Work::Enter(self)];
        let mut done: Vec<BTreeSet<Atom<S>>> = Vec::new();
        while let Some(step) = work.pop() {
            match step {
                | Work::Enter(term) => {
                    work.push(Work::Exit(term));
                    for child in term.children().iter().rev() {
                        work.push(Work::Enter(child));
                    }
                },
                | Work::Exit(term) => {
                    let split = done.len().saturating_sub(term.children().len());
                    let mut names: BTreeSet<Atom<S>> =
                        done.split_off(split).into_iter().flatten().collect();
                    match *term {
                        | Self::Free { name, .. } => {
                            names.insert(name);
                        },
                        | Self::Bound { name, .. } => {
                            names.remove(&name);
                        },
                    }
                    done.push(names);
                },
            }
        }
        return done.pop().unwrap_or_default();
    }
}

/// The name shape `γ ∈ Ā` of a rewrite rule's left-hand side (RNTA Def
/// 4.1): the node either carries a free name read from a register or
/// allocates a fresh bound name.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum NodeKind
{
    /// `a.f(…)` — the node's free name is read from the named register
    /// (RNTA Lemma 4.2(1)).
    FreeName
    {
        /// The register the node's name is read from.
        register: Register,
    },
    /// `νa.f(…)` — the node allocates a fresh bound name; the children may
    /// keep it (RNTA Lemma 4.2(2)).
    Allocate,
}

/// Where one child position of a rewrite rule sends its subterm: a target
/// control point and the register transfer populating its store.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ChildTarget
{
    /// The control point the child subterm is rewritten from.
    control: Control,
    /// How the child's store is populated; its length is the child's arity.
    transfer: Vec<Transfer>,
}

impl ChildTarget
{
    /// Pair a child control point with its register transfer.
    #[inline]
    #[must_use]
    pub fn new(
        control: Control,
        transfer: Vec<Transfer>,
    ) -> Self
    {
        return Self { control, transfer };
    }

    /// The control point the child subterm is rewritten from.
    #[inline]
    #[must_use]
    pub fn control(&self) -> Control
    {
        return self.control;
    }

    /// The register transfer populating the child's store.
    #[inline]
    #[must_use]
    pub fn transfer(&self) -> &[Transfer]
    {
        return &self.transfer;
    }
}

/// One rewrite rule `q(γ.f(x₁,…,xₙ)) → γ.f(q₁(x₁),…,qₙ(xₙ))` (RNTA Def
/// 4.1): one orbit of the equivariant rule set `Δ`, defunctionalized.
///
/// The rule's `children` length *is* the symbol's arity at this control
/// point — the signature is carried implicitly by the rule set rather than
/// as a separate table.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RntaRule<F>
{
    /// The control point the rule fires from.
    source: Control,
    /// The node symbol the rule matches.
    symbol: F,
    /// The name shape of the matched node.
    kind: NodeKind,
    /// One target per child position.
    children: Vec<ChildTarget>,
}

impl<F> RntaRule<F>
{
    /// A rewrite rule from `source` on `symbol` with the given name shape
    /// and child targets.
    #[inline]
    #[must_use]
    pub fn new(
        source: Control,
        symbol: F,
        kind: NodeKind,
        children: Vec<ChildTarget>,
    ) -> Self
    {
        return Self {
            source,
            symbol,
            kind,
            children,
        };
    }

    /// The control point the rule fires from.
    #[inline]
    #[must_use]
    pub fn source(&self) -> Control
    {
        return self.source;
    }

    /// The node symbol the rule matches.
    #[inline]
    #[must_use]
    pub fn symbol(&self) -> &F
    {
        return &self.symbol;
    }

    /// The name shape of the matched node.
    #[inline]
    #[must_use]
    pub fn kind(&self) -> NodeKind
    {
        return self.kind;
    }

    /// The child targets, one per child position.
    #[inline]
    #[must_use]
    pub fn children(&self) -> &[ChildTarget]
    {
        return &self.children;
    }
}

/// A **regular nominal tree automaton** (RNTA Def 4.1) in the finitary
/// register presentation of design doc §6.4.
///
/// There is no final set: acceptance is the existence of a top-down run
/// that rewrites the whole term, with leaves matched by arity-zero rules
/// (RNTA Def 4.1, `L(A) = L(q₀)`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Rnta<S, F>
{
    /// The register arity of each control point; the index is the control.
    arities: Vec<Arity>,
    /// The initial configuration `q₀`.
    initial: Configuration<S>,
    /// The symbolic rewrite rules `Δ`.
    rules: Vec<RntaRule<F>>,
}

impl<S, F> Rnta<S, F>
where
    S: Sort,
{
    /// Build an RNTA, checking the finite handle's well-formedness.
    ///
    /// # Contract
    /// - ensures: on success, every rule's source and child targets are control
    ///   points of the automaton, every child transfer length matches its child
    ///   control's arity, every register read is in range, and
    ///   [`Transfer::Allocated`] occurs only in the children of an allocating
    ///   rule, at most once per child (RNTA Lemma 4.2).
    /// - provides: a well-formed RNTA.
    /// - fails: the first violated invariant, as an [`AutomatonError`].
    /// - panics: none.
    ///
    /// # Errors
    /// Returns the [`AutomatonError`] variant naming the first violated
    /// invariant.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — each check is separated from acceptance by one
    ///   mutated handle field, exercised by one rejection test plus one
    ///   accepting construction.
    /// - witness: `rnta::tests::construction_accepts_a_well_formed_rnta`
    /// - witness: `rnta::tests::construction_rejects_unknown_register`
    /// - witness: `rnta::tests::construction_rejects_misplaced_allocated_name`
    #[inline]
    pub fn new(
        arities: Vec<Arity>,
        initial: Configuration<S>,
        rules: Vec<RntaRule<F>>,
    ) -> Result<Self, AutomatonError>
    {
        let controls = Controls::from(arities.len());
        let arity_of = |control: Control| arities.get(usize::from(control)).copied();

        let Some(initial_arity) = arity_of(initial.control())
        else {
            return Err(AutomatonError::InvalidControl {
                control: initial.control(),
                controls: u32::from(controls),
            });
        };
        if initial.store().arity() != initial_arity {
            return Err(AutomatonError::ArityMismatch {
                control: initial.control(),
                expected: initial_arity,
                actual: initial.store().arity(),
            });
        }
        for rule in &rules {
            let Some(source_arity) = arity_of(rule.source())
            else {
                return Err(AutomatonError::InvalidControl {
                    control: rule.source(),
                    controls: u32::from(controls),
                });
            };
            if let NodeKind::FreeName { register } = rule.kind()
                && u32::from(register) >= u32::from(source_arity)
            {
                return Err(AutomatonError::UnknownRegister {
                    control: rule.source(),
                    register,
                });
            }
            for child in rule.children() {
                let Some(child_arity) = arity_of(child.control())
                else {
                    return Err(AutomatonError::InvalidControl {
                        control: child.control(),
                        controls: u32::from(controls),
                    });
                };
                let actual = Arity::from(child.transfer().len());
                if actual != child_arity {
                    return Err(AutomatonError::ArityMismatch {
                        control: child.control(),
                        expected: child_arity,
                        actual,
                    });
                }
                let allocations = child
                    .transfer()
                    .iter()
                    .filter(|entry| matches!(entry, Transfer::Allocated))
                    .count();
                let misplaced = match rule.kind() {
                    | NodeKind::Allocate => allocations > 1,
                    | NodeKind::FreeName { .. } => allocations > 0,
                };
                if misplaced {
                    return Err(AutomatonError::MisplacedAllocatedName {
                        control: rule.source(),
                    });
                }
            }
        }
        return Ok(Self {
            arities,
            initial,
            rules,
        });
    }

    /// The register arity of each control point, in control order.
    #[inline]
    #[must_use]
    pub fn arities(&self) -> &[Arity]
    {
        return &self.arities;
    }

    /// The initial configuration `q₀`.
    #[inline]
    #[must_use]
    pub fn initial(&self) -> &Configuration<S>
    {
        return &self.initial;
    }

    /// The symbolic rewrite rules `Δ`.
    #[inline]
    #[must_use]
    pub fn rules(&self) -> &[RntaRule<F>]
    {
        return &self.rules;
    }

    /// The automaton's degree: the maximum register count over its control
    /// points — the complexity parameter of every catalogue procedure
    /// (RNTA Def 4.1, "corresponds morally to the number of registers";
    /// design doc §5).
    #[inline]
    #[must_use]
    pub fn degree(&self) -> Degree
    {
        return self
            .arities
            .iter()
            .max()
            .map_or(Degree::ZERO, |&arity| Degree::from(arity));
    }
}

#[cfg(test)]
mod tests
{
    use alloc::collections::BTreeSet;
    use alloc::vec;

    use super::Arity;
    use super::AutomatonError;
    use super::ChildTarget;
    use super::Configuration;
    use super::Control;
    use super::NodeKind;
    use super::Register;
    use super::Rnta;
    use super::RntaRule;
    use super::Sort;
    use super::Term;
    use super::Transfer;
    use crate::Gensym;
    use crate::Unifiability;
    use crate::handle::Store;

    /// A single-role sort for the RNTA tests.
    #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
    enum Role
    {
        /// An atom-role sort for binder names.
        Binder,
    }

    impl Sort for Role
    {
        #[inline]
        fn is_unifiable(&self) -> Unifiability
        {
            Unifiability::ATOM_ROLE
        }
    }

    /// A tiny two-symbol signature for the term tests.
    #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
    enum Sig
    {
        /// A binary node symbol.
        Node,
        /// A nullary leaf symbol.
        Leaf,
    }

    /// `FN` sees through binder shadowing: in `νa.f(a.g, b.h)` the `a`
    /// occurrence is bound and the `b` occurrence free, so `FN = {b}`; a
    /// further free use of `a` outside the binder's scope is seen.
    #[test]
    fn free_names_respects_binder_shadowing()
    {
        let mut gensym = Gensym::new(Role::Binder);
        let bound = gensym
            .fresh()
            .expect("a new allocator can mint the first atom");
        let free = gensym
            .fresh()
            .expect("a new allocator can mint the second atom");
        let term = Term::bound(bound, Sig::Node, vec![
            Term::free(bound, Sig::Leaf, vec![]),
            Term::free(free, Sig::Leaf, vec![]),
        ]);
        assert_eq!(
            term.free_names(),
            BTreeSet::from([free]),
            "the shadowed name is not free"
        );
        let term = Term::free(bound, Sig::Node, vec![Term::bound(
            bound,
            Sig::Leaf,
            vec![],
        )]);
        assert_eq!(
            term.free_names(),
            BTreeSet::from([bound]),
            "a use outside the binder's scope is free"
        );
    }

    /// A well-formed RNTA — one rule rewriting a binary allocating node
    /// into its two children — is accepted by the constructor and reports
    /// its degree.
    #[test]
    fn construction_accepts_a_well_formed_rnta()
    {
        let q0 = Control::ZERO;
        let initial: Configuration<Role> =
            Configuration::new(q0, Store::empty(Arity::from(0_usize)));
        let q1 = Control::from(1_u32);
        let rules = vec![
            RntaRule::new(q0, Sig::Node, NodeKind::Allocate, vec![
                ChildTarget::new(q1, vec![Transfer::Allocated]),
                ChildTarget::new(q1, vec![Transfer::Allocated]),
            ]),
            RntaRule::new(
                q1,
                Sig::Leaf,
                NodeKind::FreeName {
                    register: Register::ZERO,
                },
                vec![],
            ),
        ];
        let automaton = Rnta::new(
            vec![Arity::from(0_usize), Arity::from(1_usize)],
            initial,
            rules,
        )
        .expect("the tree automaton is well-formed");
        assert_eq!(1, u32::from(automaton.degree()));
        assert_eq!(2, automaton.rules().len());
    }

    /// A free-name rule reading a register its source does not have is
    /// rejected.
    #[test]
    fn construction_rejects_unknown_register()
    {
        let q0 = Control::ZERO;
        let initial: Configuration<Role> =
            Configuration::new(q0, Store::empty(Arity::from(0_usize)));
        let rules = vec![RntaRule::new(
            q0,
            Sig::Leaf,
            NodeKind::FreeName {
                register: Register::ZERO,
            },
            vec![],
        )];
        let result = Rnta::new(vec![Arity::from(0_usize)], initial, rules);
        assert_eq!(
            result.expect_err("a free-name read of a missing register is invalid"),
            AutomatonError::UnknownRegister {
                control: q0,
                register: Register::ZERO,
            }
        );
    }

    /// `Transfer::Allocated` in a child of a free-name rule is rejected.
    #[test]
    fn construction_rejects_misplaced_allocated_name()
    {
        let q0 = Control::ZERO;
        let initial: Configuration<Role> =
            Configuration::new(q0, Store::empty(Arity::from(1_usize)));
        let rules = vec![RntaRule::new(
            q0,
            Sig::Node,
            NodeKind::FreeName {
                register: Register::ZERO,
            },
            vec![ChildTarget::new(q0, vec![Transfer::Allocated])],
        )];
        let result = Rnta::new(vec![Arity::from(1_usize)], initial, rules);
        assert_eq!(
            result.expect_err("an allocated name under a free-name node is invalid"),
            AutomatonError::MisplacedAllocatedName { control: q0 }
        );
    }
}
