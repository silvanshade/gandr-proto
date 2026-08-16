//! Finite-alphabet reductions for nominal automata.
//!
//! The nominal reduction first fixes a finite name set `S`; this module is the
//! deliberately boring finite backend used after that reduction. Traversals
//! use ordered maps and sets, so witnesses are reproducible across runs.

use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;
use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::fmt;

use crate::Atom;
use crate::Sort;
use crate::letter::Letter;

/// A position in a bounded name alphabet.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NameIndex(u32);

impl From<NameIndex> for u32
{
    #[inline]
    fn from(value: NameIndex) -> Self
    {
        value.0
    }
}

/// The four lifecycle operations over a bounded name alphabet.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum BoundedLetter
{
    Free(NameIndex),
    Open(NameIndex),
    Close(NameIndex),
    OpenClose(NameIndex),
}

/// Failure while applying a finite S-restriction.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RestrictionError
{
    UnknownAtom,
    TooManyNames,
}

/// A finite name set `S` and its deterministic encoding into symbols.
#[repr(transparent)]
pub struct SRestriction<S>
where
    S: Sort + Ord,
{
    /// The sorted finite support used by the restriction.
    names: BTreeMap<Atom<S>, NameIndex>,
}

impl<S> SRestriction<S>
where
    S: Sort + Ord,
{
    /// Build an S-restriction from a finite support set.
    ///
    /// # Contract
    /// - requires: `names` is finite.
    /// - ensures: atoms are assigned indices in sorted order.
    /// - provides: a total encoder for lifecycle letters over `names`.
    /// - fails: encoding an atom outside `names` returns
    ///   [`RestrictionError::UnknownAtom`].
    /// # Errors
    /// Returns [`RestrictionError::TooManyNames`] when the index space is
    /// exhausted.
    #[inline]
    pub fn new<I>(names: I) -> Result<Self, RestrictionError>
    where
        I: IntoIterator<Item = Atom<S>>,
    {
        let ordered = names.into_iter().collect::<BTreeSet<_>>();
        let mut map = BTreeMap::new();
        for (index, atom) in ordered.into_iter().enumerate() {
            let index = u32::try_from(index).map_err(|error| {
                let _ = error;
                RestrictionError::TooManyNames
            })?;
            map.insert(atom, NameIndex(index));
        }
        Ok(Self { names: map })
    }

    /// Encode one nominal letter.
    ///
    /// # Contract
    /// - requires: the letter's atom belongs to `S`.
    /// - ensures: the lifecycle constructor and its deterministic index are
    ///   preserved.
    /// - provides: a finite alphabet symbol.
    /// - fails: returns [`RestrictionError::UnknownAtom`] outside `S`.
    /// # Errors
    /// Returns [`RestrictionError::UnknownAtom`] when the atom is outside `S`.
    #[inline]
    pub fn encode(
        &self,
        letter: Letter<S>,
    ) -> Result<BoundedLetter, RestrictionError>
    {
        let atom = letter.atom();
        let index = self
            .names
            .get(&atom)
            .copied()
            .ok_or(RestrictionError::UnknownAtom)?;
        Ok(match letter {
            | Letter::Free(_) => BoundedLetter::Free(index),
            | Letter::Open(_) => BoundedLetter::Open(index),
            | Letter::Close(_) => BoundedLetter::Close(index),
            | Letter::OpenClose(_) => BoundedLetter::OpenClose(index),
        })
    }
}

/// A finite NFA state identity.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct StateId(u32);

impl From<u32> for StateId
{
    #[inline]
    fn from(value: u32) -> Self
    {
        Self(value)
    }
}

/// A deterministic acceptance witness for a finite automaton.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WordWitness
{
    /// The finite word symbols.
    symbols: Vec<BoundedLetter>,
}

impl WordWitness
{
    /// The symbols in the witness word.
    #[must_use]
    #[inline]
    pub fn symbols(&self) -> &[BoundedLetter]
    {
        &self.symbols
    }
}

/// A finite NFA produced after S-restriction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Nfa
{
    /// The complete finite state set.
    states: BTreeSet<StateId>,
    /// The finite lifecycle alphabet.
    alphabet: BTreeSet<BoundedLetter>,
    /// The initial state subset.
    initial: BTreeSet<StateId>,
    /// The accepting state subset.
    finals: BTreeSet<StateId>,
    /// The transition relation.
    transitions: BTreeMap<(StateId, BoundedLetter), BTreeSet<StateId>>,
}

impl Nfa
{
    /// Construct a finite NFA, rejecting references to unknown states.
    ///
    /// # Contract
    /// - requires: every transition endpoint and initial/final state is in
    ///   `states`.
    /// - ensures: transitions are canonicalized in ordered sets.
    /// - provides: a finite automaton suitable for reachability and inclusion.
    /// - fails: returns `None` when a state reference is unknown.
    #[must_use]
    #[inline]
    pub fn new(
        states: BTreeSet<StateId>,
        alphabet: BTreeSet<BoundedLetter>,
        initial: BTreeSet<StateId>,
        finals: BTreeSet<StateId>,
        transitions: BTreeMap<(StateId, BoundedLetter), BTreeSet<StateId>>,
    ) -> Option<Self>
    {
        if !initial.is_subset(&states) || !finals.is_subset(&states) {
            return None;
        }
        if transitions.iter().any(|(key, targets)| {
            let source = key.0;
            let symbol = key.1;
            !states.contains(&source) || !alphabet.contains(&symbol) || !targets.is_subset(&states)
        }) {
            return None;
        }
        Some(Self {
            states,
            alphabet,
            initial,
            finals,
            transitions,
        })
    }

    /// Return a shortest lexicographically least accepted word, if one exists.
    ///
    /// # Contract
    /// - ensures: `Some` is an accepted word and `None` means no accepting
    ///   state is reachable from the initial subset.
    /// - provides: a deterministic emptiness certificate.
    #[must_use]
    #[inline]
    pub fn emptiness_witness(&self) -> Option<WordWitness>
    {
        let start = self.initial.clone();
        let mut queue = VecDeque::from([(start.clone(), Vec::new())]);
        let mut seen = BTreeSet::from([start]);
        while let Some((subset, word)) = queue.pop_front() {
            if subset.iter().any(|state| self.finals.contains(state)) {
                return Some(WordWitness { symbols: word });
            }
            for symbol in &self.alphabet {
                let next = successors(&self.transitions, &subset, *symbol);
                if !next.is_empty() && seen.insert(next.clone()) {
                    let mut next_word = word.clone();
                    next_word.push(*symbol);
                    queue.push_back((next, next_word));
                }
            }
        }
        None
    }

    #[must_use]
    #[inline]
    pub fn inclusion_counterexample(
        &self,
        other: &Self,
    ) -> Option<WordWitness>
    {
        if self.alphabet != other.alphabet {
            return None;
        }
        let start = (self.initial.clone(), other.initial.clone());
        let mut queue = VecDeque::from([(start.clone(), Vec::new())]);
        let mut seen = BTreeSet::from([start]);
        while let Some(((left, right), word)) = queue.pop_front() {
            let left_final = left.iter().any(|state| self.finals.contains(state));
            let right_final = right.iter().any(|state| other.finals.contains(state));
            if left_final && !right_final {
                return Some(WordWitness { symbols: word });
            }
            for symbol in &self.alphabet {
                let next = (
                    successors(&self.transitions, &left, *symbol),
                    successors(&other.transitions, &right, *symbol),
                );
                if seen.insert(next.clone()) {
                    let mut next_word = word.clone();
                    next_word.push(*symbol);
                    queue.push_back((next, next_word));
                }
            }
        }
        None
    }
}

/// Compute one deterministic successor subset.
#[inline]
fn successors(
    transitions: &BTreeMap<(StateId, BoundedLetter), BTreeSet<StateId>>,
    states: &BTreeSet<StateId>,
    symbol: BoundedLetter,
) -> BTreeSet<StateId>
{
    let mut result = BTreeSet::new();
    for state in states {
        if let Some(targets) = transitions.get(&(*state, symbol)) {
            result.extend(targets);
        }
    }
    result
}

impl fmt::Display for RestrictionError
{
    #[inline]
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        match *self {
            | Self::UnknownAtom => f.write_str("atom is outside the bounded name set"),
            | Self::TooManyNames => f.write_str("bounded name set exceeds index space"),
        }
    }
}

/// A finite tree symbol identified by its deterministic alphabet index.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TreeSymbol(u32);

impl From<u32> for TreeSymbol
{
    #[inline]
    fn from(value: u32) -> Self
    {
        Self(value)
    }
}

/// A finite tree used as an NFTA witness.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Tree
{
    Node
    {
        symbol: TreeSymbol,
        children: Vec<Self>,
    },
}

/// A deterministic witness for finite-tree emptiness.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TreeWitness(Tree);

impl TreeWitness
{
    /// The witness tree.
    #[must_use]
    #[inline]
    pub fn tree(&self) -> &Tree
    {
        &self.0
    }
}

/// A finite bottom-up nondeterministic tree automaton.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Nfta
{
    /// The finite state set.
    states: BTreeSet<StateId>,
    /// The accepting root states.
    finals: BTreeSet<StateId>,
    /// The bottom-up transition relation.
    transitions: BTreeMap<(TreeSymbol, Vec<StateId>), BTreeSet<StateId>>,
}

impl Nfta
{
    /// Construct an NFTA with canonical transition keys.
    ///
    /// # Contract
    /// - requires: every transition state and final state belongs to `states`.
    /// - ensures: transitions are finite and ordered.
    /// - provides: a bottom-up finite-tree automaton.
    /// - panics: none.
    #[must_use]
    #[inline]
    pub fn new(
        states: BTreeSet<StateId>,
        finals: BTreeSet<StateId>,
        transitions: BTreeMap<(TreeSymbol, Vec<StateId>), BTreeSet<StateId>>,
    ) -> Option<Self>
    {
        if !finals.is_subset(&states)
            || transitions.iter().any(|(key, targets)| {
                let children = &key.1;
                !children.iter().all(|state| states.contains(state)) || !targets.is_subset(&states)
            })
        {
            return None;
        }
        Some(Self {
            states,
            finals,
            transitions,
        })
    }

    /// Return a deterministic accepted tree, or `None` when the language is
    /// empty.
    #[inline]
    pub fn emptiness_witness(&self) -> Option<TreeWitness>
    {
        let mut known: BTreeMap<StateId, Tree> = BTreeMap::new();
        let mut changed = true;
        while changed {
            changed = false;
            for (key, targets) in &self.transitions {
                let symbol = key.0;
                let children = &key.1;
                if children.iter().all(|state| known.contains_key(state)) {
                    let trees = children
                        .iter()
                        .filter_map(|state| known.get(state).cloned())
                        .collect::<Vec<_>>();
                    for target in targets {
                        if !known.contains_key(target) {
                            known.insert(*target, Tree::Node {
                                symbol,
                                children: trees.clone(),
                            });
                            changed = true;
                        }
                    }
                }
            }
        }
        self.finals
            .iter()
            .find_map(|state| known.get(state).cloned())
            .map(TreeWitness)
    }
}

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::Gensym;
    use crate::Sort;

    #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
    enum Role
    {
        Name,
    }
    impl Sort for Role
    {
        fn is_unifiable(&self) -> crate::Unifiability
        {
            crate::Unifiability::ATOM_ROLE
        }
    }

    fn automaton(finals: BTreeSet<StateId>) -> Nfa
    {
        let states = BTreeSet::from([StateId(0), StateId(1)]);
        let alphabet = BTreeSet::from([BoundedLetter::Open(NameIndex(0))]);
        let transitions = BTreeMap::from([(
            (StateId(0), BoundedLetter::Open(NameIndex(0))),
            BTreeSet::from([StateId(1)]),
        )]);
        Nfa::new(
            states,
            alphabet,
            BTreeSet::from([StateId(0)]),
            finals,
            transitions,
        )
        .expect("valid test NFA")
    }

    #[test]
    fn s_restriction_orders_atoms_and_encodes_letters()
    {
        let mut gensym = Gensym::new(Role::Name);
        let first = gensym.fresh().expect("first atom");
        let second = gensym.fresh().expect("second atom");
        let restriction = SRestriction::new([second, first]).expect("finite set");
        assert_eq!(2, restriction.names.len());
        assert_eq!(
            BoundedLetter::Open(NameIndex(0)),
            restriction.encode(Letter::Open(first)).expect("in S")
        );
    }

    #[test]
    fn emptiness_returns_shortest_deterministic_witness()
    {
        let witness = automaton(BTreeSet::from([StateId(1)]))
            .emptiness_witness()
            .expect("accepted");
        assert_eq!([BoundedLetter::Open(NameIndex(0))], witness.symbols());
        assert_eq!(
            witness,
            automaton(BTreeSet::from([StateId(1)]))
                .emptiness_witness()
                .expect("accepted")
        );
    }

    #[test]
    fn inclusion_returns_deterministic_counterexample()
    {
        let empty = automaton(BTreeSet::new());
        let accepting = automaton(BTreeSet::from([StateId(1)]));
        assert!(empty.inclusion_counterexample(&accepting).is_none());
        assert!(accepting.inclusion_counterexample(&empty).is_some());
    }
}
