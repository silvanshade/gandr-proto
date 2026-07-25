//! Deterministic finite walk-index construction.
//!
//! # Contract
//! - requires: callers describe a finite declarative walk machine through
//!   [`WalkSpec`].
//! - ensures: [`WalkIndex::build`] canonicalizes insertion order, uses the full
//!   `(sort, bounds)` swing key, and exposes ordered direct, transitive, mold,
//!   and fingerprint observations.
//! - provides: a small petgraph-free walk materialization engine for gandr's
//!   grammar graph layer.
//! - fails: malformed public shapes, useless swing arcs, zero caps, arithmetic
//!   overflow, and cap exceedance surface as [`WalkBuildError`].
//! - panics: none.
//! - intension: both closure layers are iterative breadth-first traversals; the
//!   outer layer expands each path-local endpoint-simple prefix, records
//!   terminal cycle-closing rows, and never re-expands a visited endpoint.
//!
//! # Adequacy
//! - hypothesis: L3 pointwise plus L2 generative — the integration witness
//!   covers paper-trace fragments, filter/order cases, query orientation,
//!   cycles, cap errors, seen-key comparison, deterministic canonicalization,
//!   duplicate elimination, molds, and small insertion permutations; the
//!   subprocess fingerprint witness is pairwise/self-comparison only, not an
//!   external semantic oracle.
//! - witness: `gandr_theory_graphs::walk::contracts::walk_index_contract`
//! - witness: `gandr_theory_graphs::determinism::gandr_theory_graphs::subprocess_determinism_contract`

use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;
use alloc::collections::VecDeque;
use alloc::vec;
use alloc::vec::Vec;
use core::cmp::Ordering;
use core::convert::TryFrom as _;
use core::error::Error;
use core::fmt::Debug;
use core::fmt::Display;
use core::fmt::Formatter;
use core::fmt::Result as FmtResult;

use crate::Fingerprint;
use crate::FingerprintByte;
use crate::FingerprintWord64;
use crate::SwingHeight;
use crate::WalkChainLength;
use crate::WalkHeight;
use crate::fingerprint::Fnv64;

/// A direction for source-machine walks.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum Dir
{
    /// Left-facing source relation.
    Left,
    /// Right-facing source relation.
    Right,
}

/// An exact walk boundary.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum End<T>
{
    /// The distinguished root boundary.
    Root,
    /// A concrete finite boundary node.
    Node(T),
}

/// Whether a stance belongs to a tile-sorted class.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StanceTileSorted(bool);

impl From<bool> for StanceTileSorted
{
    #[inline]
    fn from(value: bool) -> Self
    {
        Self(value)
    }
}

impl From<StanceTileSorted> for bool
{
    #[inline]
    fn from(value: StanceTileSorted) -> Self
    {
        value.0
    }
}

/// Stable key for fingerprinting walk symbols.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WalkSymbolKey(u64);

impl From<u64> for WalkSymbolKey
{
    #[inline]
    fn from(value: u64) -> Self
    {
        Self(value)
    }
}

impl From<WalkSymbolKey> for u64
{
    #[inline]
    fn from(value: WalkSymbolKey) -> Self
    {
        value.0
    }
}

impl From<WalkSymbolKey> for FingerprintWord64
{
    #[inline]
    fn from(value: WalkSymbolKey) -> Self
    {
        Self::from(u64::from(value))
    }
}

/// Whether a materialized walk survives minimality filtering.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct WalkMinimal(bool);

impl From<bool> for WalkMinimal
{
    #[inline]
    fn from(value: bool) -> Self
    {
        Self(value)
    }
}

impl From<WalkMinimal> for bool
{
    #[inline]
    fn from(value: WalkMinimal) -> Self
    {
        value.0
    }
}

/// Symbol vocabulary used by the generic walk index.
pub trait WalkSym: Clone + Debug + Eq + Ord
{
    /// Grammar nonterminal carried inside swings.
    type Nonterminal: Clone + Eq + Ord;
    /// Intervening stance or tile identity.
    type Stance: Clone + Eq + Ord;
    /// Sort key shared by nonterminals and stances.
    type Sort: Clone + Eq + Ord;
    /// Bounds key that refines nonterminal sorts for safe swing closure.
    type Bounds: Clone + Eq + Ord;
    /// Tile label used by the molds projection.
    type Label: Clone + Eq + Ord;
    /// Mold payload used by the molds projection.
    type Mold: Clone + Eq + Ord;

    /// Return a nonterminal's sort.
    fn nonterminal_sort(nonterminal: &Self::Nonterminal) -> Self::Sort;

    /// Return a nonterminal's bounds key.
    fn nonterminal_bounds(nonterminal: &Self::Nonterminal) -> Self::Bounds;

    /// Return a stance's sort.
    fn stance_sort(stance: &Self::Stance) -> Self::Sort;

    /// Return whether a stance sort is tile-sorted for minimality filtering.
    fn stance_tile_sorted(stance: &Self::Stance) -> StanceTileSorted;

    /// Project an optional label and mold from a tile stance.
    fn label_mold(stance: &Self::Stance) -> Option<(Self::Label, Self::Mold)>;

    /// Return a stable key for fingerprinting a nonterminal.
    fn nonterminal_key(nonterminal: &Self::Nonterminal) -> WalkSymbolKey;

    /// Return a stable key for fingerprinting a stance.
    fn stance_key(stance: &Self::Stance) -> WalkSymbolKey;
}

/// One grammar walk endpoint.
type WalkEnd<S> = End<<S as WalkSym>::Stance>;
/// One materialized grammar walk.
type MachineWalk<S> = Walk<<S as WalkSym>::Nonterminal, <S as WalkSym>::Stance>;
/// One explicitly supplied direct walk.
type DirectStep<S> = (Dir, WalkEnd<S>, WalkEnd<S>, MachineWalk<S>);
/// One swing-machine closure seed.
type SwingSeed<S> = (Dir, WalkEnd<S>, <S as WalkSym>::Nonterminal);
/// Key for a direction-sensitive walk row.
type DirectKey<S> = (Dir, WalkEnd<S>, WalkEnd<S>);
/// Key for an oriented public query row.
type QueryKey<S> = (WalkEnd<S>, WalkEnd<S>);
/// Canonical direction-sensitive walk rows.
type DirectRows<S> = BTreeMap<DirectKey<S>, Vec<MachineWalk<S>>>;
/// Canonical public query rows.
type QueryRows<S> = BTreeMap<QueryKey<S>, Vec<MachineWalk<S>>>;
/// One label-projected endpoint and mold.
type MoldRow<S> = (WalkEnd<S>, <S as WalkSym>::Mold);
/// Canonical label-indexed mold projection rows.
type MoldRows<S> = BTreeMap<<S as WalkSym>::Label, Vec<MoldRow<S>>>;
/// All destinations and walks emitted by one swing closure.
type SwingClosureOutput<S> = Vec<(WalkEnd<S>, MachineWalk<S>)>;
/// One queued endpoint-simple outer-closure prefix and its path-local ends.
type ClosureQueueItem<S> = (WalkEnd<S>, MachineWalk<S>, BTreeSet<WalkEnd<S>>);

/// Direct rows indexed by `(direction, source end)`: each entry lists the
/// `(destination, walks)` successors, in canonical key order.
type RowsBySource<'rows, S> =
    BTreeMap<(Dir, &'rows WalkEnd<S>), Vec<(&'rows WalkEnd<S>, &'rows Vec<MachineWalk<S>>)>>;
/// Total-order key for one canonical walk.
type WalkSortKey<S> =
    CanonicalWalkKey<<S as WalkSym>::Nonterminal, <S as WalkSym>::Stance, <S as WalkSym>::Sort>;

/// A non-empty source-to-destination swing.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Swing<N>
{
    /// Nonterminal sequence in source-to-destination order.
    nonterminals: Vec<N>,
}

impl<N> Swing<N>
{
    /// Construct a non-empty swing.
    ///
    /// # Contract
    /// - requires: `nonterminals` names the source-to-destination swing chain.
    /// - ensures: returns a swing preserving the supplied order when non-empty.
    /// - provides: checked public construction for [`Swing`].
    /// - fails: returns [`WalkBuildError::EmptySwing`] for an empty sequence.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`WalkBuildError::EmptySwing`] when `nonterminals` is empty.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — the walk-index witness constructs both
    ///   valid singleton/compound swings and invalid empty swings through this
    ///   guard.
    /// - witness: `gandr_theory_graphs::walk::contracts::walk_index_contract`
    #[inline]
    pub fn new(nonterminals: Vec<N>) -> Result<Self, WalkBuildError>
    {
        if nonterminals.is_empty() {
            Err(WalkBuildError::EmptySwing)
        }
        else {
            Ok(Self { nonterminals })
        }
    }

    /// Return the nonterminals in source-to-destination order.
    #[inline]
    #[must_use]
    pub fn nonterminals(&self) -> &[N]
    {
        &self.nonterminals
    }

    /// Return `node_count - 1` for this non-empty swing.
    ///
    /// # Contract
    /// - requires: construction went through [`Swing::new`].
    /// - ensures: singleton swings have height zero and longer swings report
    ///   their checked predecessor count.
    /// - provides: the public height observation used by filtering and tests.
    /// - fails: returns [`WalkBuildError::ArithmeticOverflow`] if the length
    ///   cannot be represented as `u32`.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`WalkBuildError::ArithmeticOverflow`] when conversion to `u32`
    /// fails.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — filter/order witnesses distinguish zero and
    ///   non-zero swing heights through exact public walk rows.
    /// - witness: `gandr_theory_graphs::walk::contracts::walk_index_contract`
    #[inline]
    pub fn height(&self) -> Result<SwingHeight, WalkBuildError>
    {
        let count = u32::try_from(self.nonterminals.len())?;
        count
            .checked_sub(1)
            .map(SwingHeight::from)
            .ok_or(WalkBuildError::InvalidWalkShape)
    }
}

impl<N: Ord> Ord for Swing<N>
{
    #[inline]
    fn cmp(
        &self,
        other: &Self,
    ) -> Ordering
    {
        self.nonterminals.cmp(&other.nonterminals)
    }
}

impl<N: Ord> PartialOrd for Swing<N>
{
    #[inline]
    fn partial_cmp(
        &self,
        other: &Self,
    ) -> Option<Ordering>
    {
        Some(self.cmp(other))
    }
}

/// One alternating walk step used by checked construction.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum WalkStep<N, T>
{
    /// A swing segment.
    Swing(Swing<N>),
    /// A stance between two swings.
    Stance(T),
}

/// An alternating source-to-destination walk.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Walk<N, T>
{
    /// Non-empty swing sequence.
    swings: Vec<Swing<N>>,
    /// Intervening stances; exactly one fewer than swings.
    stances: Vec<T>,
}

impl<N, T> Walk<N, T>
{
    /// Construct a checked alternating walk.
    ///
    /// # Contract
    /// - requires: `swings` and `stances` are source-to-destination sequences.
    /// - ensures: accepts exactly non-empty alternating shapes with
    ///   `swings.len() == stances.len() + 1`.
    /// - provides: checked public construction for [`Walk`].
    /// - fails: returns [`WalkBuildError::InvalidWalkShape`] on malformed
    ///   alternating shapes or arithmetic overflow while checking the shape.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`WalkBuildError::InvalidWalkShape`] for empty or
    /// non-alternating shapes, or [`WalkBuildError::ArithmeticOverflow`] if
    /// length arithmetic overflows.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — paper and filter witnesses observe accepted
    ///   singleton and multi-swing walks; shape errors are separated by the cap
    ///   and construction guard cases.
    /// - witness: `gandr_theory_graphs::walk::contracts::walk_index_contract`
    #[inline]
    pub fn new(
        swings: Vec<Swing<N>>,
        stances: Vec<T>,
    ) -> Result<Self, WalkBuildError>
    {
        if swings.is_empty() {
            return Err(WalkBuildError::InvalidWalkShape);
        }
        let expected = stances
            .len()
            .checked_add(1)
            .ok_or(WalkBuildError::ArithmeticOverflow)?;
        if swings.len() == expected {
            Ok(Self { swings, stances })
        }
        else {
            Err(WalkBuildError::InvalidWalkShape)
        }
    }

    /// Construct a checked walk from alternating steps.
    ///
    /// # Contract
    /// - requires: steps are intended to alternate swing/stance/swing.
    /// - ensures: accepts exactly a non-empty sequence beginning and ending
    ///   with a swing and alternating at every position.
    /// - provides: a checked step-based constructor for clients that stream the
    ///   declarative shape.
    /// - fails: returns [`WalkBuildError::InvalidWalkShape`] for malformed step
    ///   order or length overflow.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`WalkBuildError::InvalidWalkShape`] or
    /// [`WalkBuildError::ArithmeticOverflow`] as described above.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — the walk-index contract witness exercises
    ///   the same alternating invariant through exact public rows.
    /// - witness: `gandr_theory_graphs::walk::contracts::walk_index_contract`
    #[inline]
    pub fn from_steps(steps: Vec<WalkStep<N, T>>) -> Result<Self, WalkBuildError>
    {
        let mut swings = Vec::new();
        let mut stances = Vec::new();
        let mut want_swing = true;
        for step in steps {
            match (want_swing, step) {
                | (true, WalkStep::Swing(swing)) => {
                    swings.push(swing);
                    want_swing = false;
                },
                | (false, WalkStep::Stance(stance)) => {
                    stances.push(stance);
                    want_swing = true;
                },
                | _ => return Err(WalkBuildError::InvalidWalkShape),
            }
        }
        if want_swing {
            return Err(WalkBuildError::InvalidWalkShape);
        }
        Self::new(swings, stances)
    }

    /// Return this walk's swings.
    #[inline]
    #[must_use]
    pub fn swings(&self) -> &[Swing<N>]
    {
        &self.swings
    }

    /// Return this walk's intervening stances.
    #[inline]
    #[must_use]
    pub fn stances(&self) -> &[T]
    {
        &self.stances
    }

    /// Return whether all swings have height zero.
    ///
    /// # Contract
    /// - requires: construction went through [`Walk::new`].
    /// - ensures: returns `true` exactly for all-zero-height walks.
    /// - provides: the equality predicate used by direct query filters.
    /// - fails: returns [`WalkBuildError::ArithmeticOverflow`] if a swing
    ///   height cannot fit `u32`.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`WalkBuildError::ArithmeticOverflow`] for hostile vector
    /// lengths.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — query-orientation witnesses distinguish eq
    ///   and neq direct rows.
    /// - witness: `gandr_theory_graphs::walk::contracts::walk_index_contract`
    #[inline]
    pub fn is_eq(&self) -> Result<WalkEquality, WalkBuildError>
    {
        for swing in &self.swings {
            if swing.height()? != SwingHeight::default() {
                return Ok(WalkEquality::from(false));
            }
        }
        Ok(WalkEquality::from(true))
    }

    /// Return whether the destination-adjacent swing has non-zero height.
    ///
    /// # Contract
    /// - requires: construction went through [`Walk::new`].
    /// - ensures: returns `true` exactly when the final swing height is
    ///   nonzero.
    /// - provides: the non-equality predicate used by direct query filters.
    /// - fails: returns [`WalkBuildError::ArithmeticOverflow`] if a swing
    ///   height cannot fit `u32`.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`WalkBuildError::ArithmeticOverflow`] for hostile vector
    /// lengths.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — query-orientation witnesses distinguish lt
    ///   and gt non-equality rows.
    /// - witness: `gandr_theory_graphs::walk::contracts::walk_index_contract`
    #[inline]
    pub fn is_neq(&self) -> Result<WalkInequality, WalkBuildError>
    {
        let mut last = None;
        for swing in &self.swings {
            last = Some(swing);
        }
        match last {
            | Some(swing) => {
                let height = swing.height()?;
                Ok(WalkInequality::from(height != SwingHeight::default()))
            },
            | None => Err(WalkBuildError::InvalidWalkShape),
        }
    }

    /// Return the count of non-zero-height swings.
    ///
    /// # Contract
    /// - requires: construction went through [`Walk::new`].
    /// - ensures: counts exactly the non-zero swing heights.
    /// - provides: the primary canonical comparator key.
    /// - fails: returns [`WalkBuildError::ArithmeticOverflow`] if height or
    ///   count arithmetic overflows.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`WalkBuildError::ArithmeticOverflow`] for hostile lengths.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — §4.1 order witnesses distinguish height
    ///   before stance summaries and lexicographic fallback.
    /// - witness: `gandr_theory_graphs::walk::contracts::walk_index_contract`
    #[inline]
    pub fn height(&self) -> Result<WalkHeight, WalkBuildError>
    {
        let mut height = 0_u32;
        for swing in &self.swings {
            if swing.height()? != SwingHeight::default() {
                height = height
                    .checked_add(1)
                    .ok_or(WalkBuildError::ArithmeticOverflow)?;
            }
        }
        Ok(WalkHeight::from(height))
    }

    /// Return the alternating chain length `2 * swings - 1`.
    ///
    /// # Contract
    /// - requires: construction went through [`Walk::new`].
    /// - ensures: reports the alternating chain length represented by the walk.
    /// - provides: a checked cap and tie-break observation.
    /// - fails: returns [`WalkBuildError::ArithmeticOverflow`] on conversion or
    ///   multiplication overflow.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`WalkBuildError::ArithmeticOverflow`] when the length cannot be
    /// represented.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — cap and chain-length order witnesses
    ///   observe this exact value through build errors and ordered rows.
    /// - witness: `gandr_theory_graphs::walk::contracts::walk_index_contract`
    #[inline]
    pub fn chain_len(&self) -> Result<WalkChainLength, WalkBuildError>
    {
        let swings = u32::try_from(self.swings.len())?;
        swings
            .checked_mul(2)
            .and_then(|value| value.checked_sub(1))
            .map(WalkChainLength::from)
            .ok_or(WalkBuildError::ArithmeticOverflow)
    }

    /// Append `right` through exact intermediate `mid`.
    fn append_through(
        &self,
        mid: &End<T>,
        right: &Self,
    ) -> Result<Self, WalkBuildError>
    where
        N: Clone,
        T: Clone,
    {
        let mut swings = self.swings.clone();
        swings.extend(right.swings.iter().cloned());
        let mut stances = self.stances.clone();
        if let End::Node(ref stance) = *mid {
            stances.push(stance.clone());
        }
        else {
            return Err(WalkBuildError::InvalidWalkShape);
        }
        stances.extend(right.stances.iter().cloned());
        Self::new(swings, stances)
    }
}

/// Whether all swings in a walk have height zero.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WalkEquality(bool);

impl From<bool> for WalkEquality
{
    #[inline]
    fn from(value: bool) -> Self
    {
        Self(value)
    }
}

impl From<WalkEquality> for bool
{
    #[inline]
    fn from(value: WalkEquality) -> Self
    {
        value.0
    }
}

/// Whether a walk has a destination-adjacent non-zero swing.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WalkInequality(bool);

impl From<bool> for WalkInequality
{
    #[inline]
    fn from(value: bool) -> Self
    {
        Self(value)
    }
}

impl From<WalkInequality> for bool
{
    #[inline]
    fn from(value: WalkInequality) -> Self
    {
        value.0
    }
}

impl<N: Ord, T: Ord> Ord for Walk<N, T>
{
    #[inline]
    fn cmp(
        &self,
        other: &Self,
    ) -> Ordering
    {
        match self.swings.cmp(&other.swings) {
            | Ordering::Equal => self.stances.cmp(&other.stances),
            | ordering => ordering,
        }
    }
}

impl<N: Ord, T: Ord> PartialOrd for Walk<N, T>
{
    #[inline]
    fn partial_cmp(
        &self,
        other: &Self,
    ) -> Option<Ordering>
    {
        Some(self.cmp(other))
    }
}

/// A swing-machine transition action.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum SwingAdvance<N, T>
{
    /// Preserve the current swing.
    Stay,
    /// Extend the current swing by one nonterminal.
    Extend(N),
    /// Cross a stance and start a new swing at `next`.
    Cross
    {
        /// Intervening stance.
        stance: T,
        /// First nonterminal of the new swing.
        next: N,
    },
}

/// A declarative swing-machine arc.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SwingArc<S: WalkSym>
{
    /// Source nonterminal state.
    from: S::Nonterminal,
    /// Swing action.
    advance: SwingAdvance<S::Nonterminal, S::Stance>,
    /// Optional destination emitted before seen-key suppression.
    emit: Option<End<S::Stance>>,
    /// Optional next nonterminal to continue from after the action.
    continue_from: Option<S::Nonterminal>,
}

impl<S: WalkSym> SwingArc<S>
{
    /// Construct a checked swing-machine arc.
    ///
    /// # Contract
    /// - requires: `from`, `advance`, `emit`, and `continue_from` describe one
    ///   finite transition.
    /// - ensures: returns an arc that either emits a destination, continues
    ///   closure, or both.
    /// - provides: checked public construction for swing closure transitions.
    /// - fails: returns [`WalkBuildError::UselessArc`] when the arc neither
    ///   emits nor continues.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`WalkBuildError::UselessArc`] for arcs with no observable
    /// effect.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — seen-key and cyclic witnesses require arcs
    ///   that emit, continue, and do both, while the builder guard rejects the
    ///   useless case.
    /// - witness: `gandr_theory_graphs::walk::contracts::walk_index_contract`
    #[inline]
    pub fn new(
        from: S::Nonterminal,
        advance: SwingAdvance<S::Nonterminal, S::Stance>,
        emit: Option<End<S::Stance>>,
        continue_from: Option<S::Nonterminal>,
    ) -> Result<Self, WalkBuildError>
    {
        if emit.is_none() && continue_from.is_none() {
            Err(WalkBuildError::UselessArc)
        }
        else {
            Ok(Self {
                from,
                advance,
                emit,
                continue_from,
            })
        }
    }
}

/// Declarative finite walk-machine specification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WalkSpec<S: WalkSym>
{
    /// Canonical exact ends (sorted, deduplicated by the set).
    ends: BTreeSet<End<S::Stance>>,
    /// Explicit direct transitions (sorted, deduplicated by the set).
    ///
    /// A `BTreeSet` keeps the documented canonical invariant ("insertion
    /// order is erased by sorted deduplication") structurally, at O(log n)
    /// per insert — the former `Vec` + full sort-and-dedup on EVERY insert
    /// made spec construction O(n² log n) in expensive `Walk` comparisons,
    /// the dominant cost of the real grammar's index build.
    direct_steps: BTreeSet<DirectStep<S>>,
    /// Swing seeds per direction/source end (sorted, deduplicated).
    swing_seeds: BTreeSet<SwingSeed<S>>,
    /// Swing-machine arcs (sorted, deduplicated).
    swing_arcs: BTreeSet<SwingArc<S>>,
    /// Optional nonterminal used to enter the mold projection.
    root_entry: Option<S::Nonterminal>,
    /// Positive maximum alternating chain length.
    max_chain_len: WalkChainLength,
}

impl<S: WalkSym> WalkSpec<S>
{
    /// Construct an empty finite walk-machine spec.
    ///
    /// # Contract
    /// - requires: `max_chain_len` is the desired cap for materialized walks.
    /// - ensures: returns a spec containing [`End::Root`] and no other entries
    ///   when `max_chain_len` is positive.
    /// - provides: the public builder root for [`WalkIndex`].
    /// - fails: returns [`WalkBuildError::ZeroMaxChainLen`] for a zero cap.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`WalkBuildError::ZeroMaxChainLen`] when the cap is zero.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — zero-cap rejection, exact accepted-cap
    ///   observation, and positive-cap overflow witnesses distinguish the
    ///   stored maximum from sentinel replacements.
    /// - witness: `gandr_theory_graphs::walk::contracts::max_chain_len_reports_exact_accepted_cap`
    /// - witness: `gandr_theory_graphs::walk::contracts::walk_index_contract`
    #[inline]
    pub fn new(max_chain_len: WalkChainLength) -> Result<Self, WalkBuildError>
    {
        if max_chain_len == WalkChainLength::default() {
            return Err(WalkBuildError::ZeroMaxChainLen);
        }
        let mut ends = BTreeSet::new();
        ends.insert(End::Root);
        Ok(Self {
            ends,
            direct_steps: BTreeSet::new(),
            swing_seeds: BTreeSet::new(),
            swing_arcs: BTreeSet::new(),
            root_entry: None,
            max_chain_len,
        })
    }

    /// Insert an exact end.
    #[inline]
    pub fn insert_end(
        &mut self,
        end: End<S::Stance>,
    )
    {
        self.ends.insert(end);
    }

    /// Insert an explicit direct transition.
    ///
    /// # Contract
    /// - requires: `walk` is a checked source-to-destination walk.
    /// - ensures: stores the transition and both endpoint values; duplicate
    ///   insertions are semantically ignored by canonical build.
    /// - provides: the special-terminal direct-step layer.
    /// - fails: never.
    /// - panics: none.
    /// - intension: insertion order is erased by sorted deduplication.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — Figure 33 and duplicate-canonicalization
    ///   witnesses observe exact explicit direct rows.
    /// - witness: `gandr_theory_graphs::walk::contracts::walk_index_contract`
    #[inline]
    pub fn insert_direct(
        &mut self,
        dir: Dir,
        src: End<S::Stance>,
        dst: End<S::Stance>,
        walk: Walk<S::Nonterminal, S::Stance>,
    )
    {
        self.insert_end(src.clone());
        self.insert_end(dst.clone());
        self.direct_steps.insert((dir, src, dst, walk));
    }

    /// Insert a swing seed for one direction and source end.
    #[inline]
    pub fn insert_swing_seed(
        &mut self,
        dir: Dir,
        src: End<S::Stance>,
        start: S::Nonterminal,
    )
    {
        self.insert_end(src.clone());
        self.swing_seeds.insert((dir, src, start));
    }

    /// Insert a checked swing-machine arc.
    #[inline]
    pub fn insert_swing_arc(
        &mut self,
        arc: SwingArc<S>,
    )
    {
        if let Some(end) = arc.emit.clone() {
            self.insert_end(end);
        }
        self.swing_arcs.insert(arc);
    }

    /// Set the optional root-entry nonterminal used by [`WalkIndex::molds`].
    #[inline]
    pub fn set_root_entry(
        &mut self,
        root_entry: S::Nonterminal,
    )
    {
        self.root_entry = Some(root_entry);
    }

    /// Return this spec's exact maximum chain length.
    #[inline]
    #[must_use]
    pub const fn max_chain_len(&self) -> WalkChainLength
    {
        self.max_chain_len
    }
}

/// Diagnostic verdict comparing legacy sort-only and production swing keys.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SeenKeyVerdict
{
    /// Legacy sort-only closure reaches the same canonical direct rows.
    Equivalent,
    /// Legacy sort-only closure differs from production `(sort, bounds)` rows.
    Divergent,
}

/// Typed walk-index construction failure.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum WalkBuildError
{
    /// A swing was constructed with no nonterminals.
    EmptySwing,
    /// A walk's alternating shape was malformed.
    InvalidWalkShape,
    /// A specification used a zero maximum chain length.
    ZeroMaxChainLen,
    /// A swing arc neither emitted nor continued.
    UselessArc,
    /// A materialized walk exceeded the configured chain-length cap.
    ChainLengthExceeded
    {
        /// Configured maximum chain length.
        max: WalkChainLength,
        /// Observed chain length.
        actual: WalkChainLength,
    },
    /// Checked integer conversion or arithmetic failed.
    ArithmeticOverflow,
}

impl From<core::num::TryFromIntError> for WalkBuildError
{
    #[inline]
    fn from(_: core::num::TryFromIntError) -> Self
    {
        Self::ArithmeticOverflow
    }
}

impl Display for WalkBuildError
{
    #[inline]
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> FmtResult
    {
        match *self {
            | Self::EmptySwing => f.write_str("walk swing is empty"),
            | Self::InvalidWalkShape => f.write_str("walk shape is invalid"),
            | Self::ZeroMaxChainLen => f.write_str("walk maximum chain length is zero"),
            | Self::UselessArc => f.write_str("walk swing arc neither emits nor continues"),
            | Self::ChainLengthExceeded { max, actual } => {
                write!(f, "walk chain length {actual} exceeds maximum {max}")
            },
            | Self::ArithmeticOverflow => f.write_str("walk arithmetic overflow"),
        }
    }
}

impl Error for WalkBuildError
{
}

/// Canonical deterministic walk index.
#[derive(Clone)]
pub struct WalkIndex<S: WalkSym>
{
    /// Canonical exact ends.
    ends: Vec<End<S::Stance>>,
    /// Left-facing direct equality rows.
    eq_rows: QueryRows<S>,
    /// Left-facing direct non-equality rows.
    lt_rows: QueryRows<S>,
    /// Right-facing direct non-equality rows in query orientation.
    gt_rows: QueryRows<S>,
    /// Canonical transitive rows.
    transitive_rows: DirectRows<S>,
    /// Canonical mold rows by label.
    mold_rows: MoldRows<S>,
    /// Stable framed fingerprint.
    fingerprint: Fingerprint,
}

impl<S: WalkSym> WalkIndex<S>
{
    /// Build a production walk index with safe `(sort, bounds)` swing keys.
    ///
    /// # Contract
    /// - requires: `spec` is finite and was built through [`WalkSpec`].
    /// - ensures: returns canonical direct/transitive rows, mold rows, and a
    ///   stable fingerprint independent of insertion order.
    /// - provides: the production deterministic walk-index engine.
    /// - fails: returns [`WalkBuildError`] for malformed shapes, cap
    ///   exceedance, or checked arithmetic failure.
    /// - panics: none.
    /// - intension: production swing closure uses `(sort, bounds)` keys and the
    ///   outer closure uses exact [`End`] keys.
    ///
    /// # Errors
    /// Returns [`WalkBuildError::ChainLengthExceeded`] when a materialized walk
    /// exceeds the cap, or [`WalkBuildError::ArithmeticOverflow`] for checked
    /// arithmetic failures.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise + L2 generative — exact trace, query
    ///   orientation, endpoint-simple transitive queueing, filter/order, cycle,
    ///   seen-key divergence, permutation determinism, duplicate, and mold
    ///   witnesses distinguish every closure layer and canonicalization key
    ///   without fingerprint goldens.
    /// - witness: `gandr_theory_graphs::walk::contracts::query_orientation_filters_direct_rows`
    /// - witness: `gandr_theory_graphs::walk::contracts::transitive_queue_prunes_only_node_endpoints_not_already_seen`
    /// - witness: `gandr_theory_graphs::walk::contracts::canonical_height_counts_nonzero_swings_only`
    /// - witness: `gandr_theory_graphs::walk::contracts::canonical_top_key_requires_zero_height_prefix`
    /// - witness: `gandr_theory_graphs::walk::contracts::canonical_mid_key_requires_strict_interior_count`
    /// - witness: `gandr_theory_graphs::walk::contracts::walk_index_contract`
    #[inline]
    pub fn build(spec: &WalkSpec<S>) -> Result<Self, WalkBuildError>
    {
        Self::build_with_key(spec, SwingKeyMode::SortBounds)
    }

    /// Compare production and legacy diagnostic swing keys on one spec.
    ///
    /// # Contract
    /// - requires: `spec` is finite and was built through [`WalkSpec`].
    /// - ensures: returns [`SeenKeyVerdict::Equivalent`] exactly when the
    ///   canonical direct rows match under both swing key modes.
    /// - provides: diagnostic evidence for the legacy sort-only key hazard.
    /// - fails: propagates build errors from either closure.
    /// - panics: none.
    /// - intension: the legacy mode is private to this comparison and is never
    ///   used by [`WalkIndex::build`].
    ///
    /// # Errors
    /// Returns any [`WalkBuildError`] raised by direct-row construction.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise + L2 generative — chain agreement and
    ///   adversarial same-sort/different-bounds fixtures distinguish equivalent
    ///   and divergent verdicts.
    /// - witness: `gandr_theory_graphs::walk::contracts::walk_index_contract`
    #[inline]
    pub fn compare_seen_keys(spec: &WalkSpec<S>) -> Result<SeenKeyVerdict, WalkBuildError>
    {
        let production = direct_rows(spec, SwingKeyMode::SortBounds)?;
        let legacy = direct_rows(spec, SwingKeyMode::SortOnly)?;
        if production == legacy {
            Ok(SeenKeyVerdict::Equivalent)
        }
        else {
            Ok(SeenKeyVerdict::Divergent)
        }
    }

    /// Query transitive walks for one direction and exact endpoint pair.
    #[inline]
    #[must_use]
    pub fn walks(
        &self,
        dir: Dir,
        src: &End<S::Stance>,
        dst: &End<S::Stance>,
    ) -> &[Walk<S::Nonterminal, S::Stance>]
    {
        self.transitive_rows
            .get(&(dir, src.clone(), dst.clone()))
            .map_or(&[], Vec::as_slice)
    }

    /// Query left-facing direct equality walks.
    #[inline]
    #[must_use]
    pub fn eq(
        &self,
        left: &End<S::Stance>,
        right: &End<S::Stance>,
    ) -> &[Walk<S::Nonterminal, S::Stance>]
    {
        self.eq_rows
            .get(&(left.clone(), right.clone()))
            .map_or(&[], Vec::as_slice)
    }

    /// Query left-facing direct non-equality walks.
    #[inline]
    #[must_use]
    pub fn lt(
        &self,
        left: &End<S::Stance>,
        right: &End<S::Stance>,
    ) -> &[Walk<S::Nonterminal, S::Stance>]
    {
        self.lt_rows
            .get(&(left.clone(), right.clone()))
            .map_or(&[], Vec::as_slice)
    }

    /// Query right-facing direct non-equality walks from `right` to `left`.
    #[inline]
    #[must_use]
    pub fn gt(
        &self,
        left: &End<S::Stance>,
        right: &End<S::Stance>,
    ) -> &[Walk<S::Nonterminal, S::Stance>]
    {
        self.gt_rows
            .get(&(left.clone(), right.clone()))
            .map_or(&[], Vec::as_slice)
    }

    /// Query mold projections for `label`.
    #[inline]
    #[must_use]
    pub fn molds(
        &self,
        label: &S::Label,
    ) -> &[(End<S::Stance>, S::Mold)]
    {
        self.mold_rows.get(label).map_or(&[], Vec::as_slice)
    }

    /// Return the canonical fingerprint.
    #[inline]
    #[must_use]
    pub const fn fingerprint(&self) -> Fingerprint
    {
        self.fingerprint
    }

    /// Return canonical exact ends.
    #[inline]
    #[must_use]
    pub fn ends(&self) -> &[End<S::Stance>]
    {
        &self.ends
    }

    /// Private build with selected swing key mode.
    fn build_with_key(
        spec: &WalkSpec<S>,
        mode: SwingKeyMode,
    ) -> Result<Self, WalkBuildError>
    {
        let direct_rows = direct_rows(spec, mode)?;
        let transitive_rows = transitive_rows(spec, &direct_rows)?;
        let direct_rows = filter_direct_rows::<S>(direct_rows)?;
        let transitive_rows = filter_transitive_rows::<S>(transitive_rows)?;
        let eq_rows = query_rows::<S>(&direct_rows, QueryKind::Eq)?;
        let lt_rows = query_rows::<S>(&direct_rows, QueryKind::Lt)?;
        let gt_rows = query_rows::<S>(&direct_rows, QueryKind::Gt)?;
        let mold_rows = mold_rows(spec, &transitive_rows);
        let fingerprint = fingerprint_index::<S>(
            spec.max_chain_len,
            &direct_rows,
            &transitive_rows,
            &mold_rows,
        )?;
        Ok(Self {
            ends: canonical_ends(spec),
            eq_rows,
            lt_rows,
            gt_rows,
            transitive_rows,
            mold_rows,
            fingerprint,
        })
    }
}

/// Swing seen-key mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SwingKeyMode
{
    /// Production `(sort, bounds)` key.
    SortBounds,
    /// Diagnostic legacy sort-only key.
    SortOnly,
}

/// Build filtered transitive rows.
fn filter_transitive_rows<S>(rows: DirectRows<S>) -> Result<DirectRows<S>, WalkBuildError>
where
    S: WalkSym,
{
    filter_direct_rows::<S>(rows)
}
/// Build filtered direct rows.
fn filter_direct_rows<S>(rows: DirectRows<S>) -> Result<DirectRows<S>, WalkBuildError>
where
    S: WalkSym,
{
    let mut filtered = BTreeMap::new();
    for (key, walks) in rows {
        let kept = canonical_walks::<S>(walks)?;
        if !kept.is_empty() {
            filtered.insert(key, kept);
        }
    }
    Ok(filtered)
}
/// Construct all direct rows from explicit steps and swing closures.
fn direct_rows<S>(
    spec: &WalkSpec<S>,
    mode: SwingKeyMode,
) -> Result<DirectRows<S>, WalkBuildError>
where
    S: WalkSym,
{
    let mut rows: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for &(dir, ref src, ref dst, ref walk) in &spec.direct_steps {
        guard_cap(walk, spec.max_chain_len)?;
        rows.entry((dir, src.clone(), dst.clone()))
            .or_default()
            .push(walk.clone());
    }
    for &(dir, ref src, ref start) in &spec.swing_seeds {
        for (dst, walk) in swing_closure(spec, dir, start, mode)? {
            guard_cap(&walk, spec.max_chain_len)?;
            rows.entry((dir, src.clone(), dst)).or_default().push(walk);
        }
    }
    for walks in rows.values_mut() {
        canonicalize(walks);
    }
    Ok(rows)
}
/// Build all exact-End outer closures.
fn transitive_rows<S>(
    spec: &WalkSpec<S>,
    direct_rows: &DirectRows<S>,
) -> Result<DirectRows<S>, WalkBuildError>
where
    S: WalkSym,
{
    let mut rows: BTreeMap<_, Vec<_>> = BTreeMap::new();
    let ends = canonical_ends(spec);
    // Index the direct rows by `(dir, src)` once, so the closure's successor
    // lookups are O(log rows + out-degree) instead of a full-table scan per
    // source and per queued path — the dominant cost of the eager closure on
    // the real grammar (observed in the index-build and eager-closure
    // profiles). Entries keep
    // the map's key order, so the enumeration order (and therefore every
    // canonicalized row) is unchanged.
    let mut by_src: RowsBySource<'_, S> = BTreeMap::new();
    for (key, walks) in direct_rows {
        let &(row_dir, ref row_src, ref dst) = key;
        by_src
            .entry((row_dir, row_src))
            .or_default()
            .push((dst, walks));
    }
    for dir in [Dir::Left, Dir::Right] {
        for src in &ends {
            let mut queue: VecDeque<ClosureQueueItem<S>> = VecDeque::new();
            let mut initial_visited = BTreeSet::new();
            initial_visited.insert(src.clone());
            for &(dst, walks) in by_src.get(&(dir, src)).into_iter().flatten() {
                for walk in walks {
                    guard_cap(walk, spec.max_chain_len)?;
                    // Sticky-non-minimal prune (remedy 1): the
                    // source-minimality filter is monotone under extension —
                    // an interior tile-sorted stance stays interior in every
                    // continuation — so a non-minimal path can neither be
                    // kept by `filter_transitive_rows` nor extend into a
                    // minimal one. Pruning it HERE (from the rows and the
                    // queue) leaves the filtered output exactly unchanged
                    // while the enumeration stops materializing the doomed
                    // subtree.
                    let is_minimal = minimal::<S>(walk)?;
                    if !bool::from(is_minimal) {
                        continue;
                    }
                    let mut visited = initial_visited.clone();
                    let should_queue = matches!(dst, End::Node(_)) && visited.insert(dst.clone());
                    rows.entry((dir, src.clone(), dst.clone()))
                        .or_default()
                        .push(walk.clone());
                    if should_queue {
                        queue.push_back((dst.clone(), walk.clone(), visited));
                    }
                }
            }
            while let Some((mid, path, visited)) = queue.pop_front() {
                for &(dst, walks) in by_src.get(&(dir, &mid)).into_iter().flatten() {
                    for suffix in walks {
                        let combined = path.append_through(&mid, suffix)?;
                        guard_cap(&combined, spec.max_chain_len)?;
                        // The same sticky prune over the combined path.
                        let is_minimal = minimal::<S>(&combined)?;
                        if !bool::from(is_minimal) {
                            continue;
                        }
                        let mut next_visited = visited.clone();
                        let should_queue =
                            matches!(dst, End::Node(_)) && next_visited.insert(dst.clone());
                        rows.entry((dir, src.clone(), dst.clone()))
                            .or_default()
                            .push(combined.clone());
                        if should_queue {
                            queue.push_back((dst.clone(), combined, next_visited));
                        }
                    }
                }
            }
        }
    }
    for walks in rows.values_mut() {
        canonicalize(walks);
    }
    Ok(rows)
}
/// Return canonical ends including root.
fn canonical_ends<S>(spec: &WalkSpec<S>) -> Vec<End<S::Stance>>
where
    S: WalkSym,
{
    let mut ends: Vec<End<S::Stance>> = spec.ends.iter().cloned().collect();
    ends.push(End::Root);
    canonicalize(&mut ends);
    ends
}
/// Guard a materialized walk against the configured cap.
fn guard_cap<N, T>(
    walk: &Walk<N, T>,
    max: WalkChainLength,
) -> Result<(), WalkBuildError>
{
    let actual = walk.chain_len()?;
    if actual > max {
        Err(WalkBuildError::ChainLengthExceeded { max, actual })
    }
    else {
        debug_assert!(
            actual <= max,
            "guard_cap accepted chain length {actual} above maximum {max}"
        );
        Ok(())
    }
}
/// Run one seeded swing closure.
fn swing_closure<S>(
    spec: &WalkSpec<S>,
    dir: Dir,
    start: &S::Nonterminal,
    mode: SwingKeyMode,
) -> Result<SwingClosureOutput<S>, WalkBuildError>
where
    S: WalkSym,
{
    let mut arcs_by_source: BTreeMap<S::Nonterminal, Vec<SwingArc<S>>> = BTreeMap::new();
    for arc in &spec.swing_arcs {
        arcs_by_source
            .entry(arc.from.clone())
            .or_default()
            .push(arc.clone());
    }
    for arcs in arcs_by_source.values_mut() {
        canonicalize(arcs);
    }

    let initial_swing = Swing::new(vec![start.clone()])?;
    let initial = PartialWalk {
        current: start.clone(),
        swings: vec![initial_swing],
        stances: Vec::new(),
    };
    let mut queue = VecDeque::new();
    queue.push_back(initial);
    let mut seen = BTreeSet::new();
    let mut output = Vec::new();
    while let Some(partial) = queue.pop_front() {
        let key = seen_key::<S>(&partial.current, mode);
        let first_visit = seen.insert(key);
        if let Some(arcs) = arcs_by_source.get(&partial.current) {
            for arc in arcs {
                let next_partial = partial.apply(&arc.advance)?;
                if let Some(dst) = arc.emit.clone() {
                    let walk =
                        Walk::new(next_partial.swings.clone(), next_partial.stances.clone())?;
                    output.push((dst, walk));
                }
                if first_visit && let Some(continue_from) = arc.continue_from.clone() {
                    let continued = next_partial.with_current(continue_from);
                    queue.push_back(continued);
                }
            }
        }
    }
    canonicalize(&mut output);
    let _ = dir;
    Ok(output)
}

/// Canonicalize valid minimal walks.
fn canonical_walks<S>(walks: Vec<MachineWalk<S>>) -> Result<Vec<MachineWalk<S>>, WalkBuildError>
where
    S: WalkSym,
{
    let mut kept = Vec::new();
    for walk in walks {
        let has_equality_shape = walk.is_eq()?;
        let has_inequality_shape = walk.is_neq()?;
        let is_canonical_shape = bool::from(has_equality_shape) || bool::from(has_inequality_shape);
        let is_minimal = minimal::<S>(&walk)?;
        if is_canonical_shape && bool::from(is_minimal) {
            let key = canonical_walk_key::<S>(&walk)?;
            kept.push((key, walk));
        }
    }
    kept.sort_by(|left, right| {
        let (ref left_key, _) = *left;
        let (ref right_key, _) = *right;
        left_key.cmp(right_key)
    });
    let mut output = Vec::new();
    let mut last_key = None;
    for (key, walk) in kept {
        if last_key.as_ref() != Some(&key) {
            output.push(walk);
            last_key = Some(key);
        }
    }
    Ok(output)
}
/// Return whether the source minimality filter retains a walk.
fn minimal<S>(walk: &MachineWalk<S>) -> Result<WalkMinimal, WalkBuildError>
where
    S: WalkSym,
{
    let has_equality_shape = walk.is_eq()?;
    if bool::from(has_equality_shape) {
        return Ok(WalkMinimal::from(true));
    }
    let height = walk.height()?;
    let mut count = 0_u32;
    let mut stance_iter = walk.stances.iter();
    for swing in &walk.swings {
        if swing.height()? != SwingHeight::default() {
            count = count
                .checked_add(1)
                .ok_or(WalkBuildError::ArithmeticOverflow)?;
        }
        if let Some(stance) = stance_iter.next()
            && count > 0
            && WalkHeight::from(count) < height
            && bool::from(S::stance_tile_sorted(stance))
        {
            return Ok(WalkMinimal::from(false));
        }
    }
    Ok(WalkMinimal::from(true))
}
/// Build canonical mold projection rows.
fn mold_rows<S>(
    spec: &WalkSpec<S>,
    transitive_rows: &DirectRows<S>,
) -> MoldRows<S>
where
    S: WalkSym,
{
    let mut rows: MoldRows<S> = BTreeMap::new();
    if spec.root_entry.is_some() {
        let root_end = End::Root;
        for (key, walks) in transitive_rows {
            let &(dir, ref src, ref dst) = key;
            if dir == Dir::Left
                && *src == root_end
                && let End::Node(ref stance) = *dst
                && let Some((label, mold)) = S::label_mold(stance)
            {
                for _walk in walks {
                    rows.entry(label.clone())
                        .or_default()
                        .push((dst.clone(), mold.clone()));
                }
            }
        }
    }
    for row in rows.values_mut() {
        canonicalize(row);
    }
    rows
}
/// Insert-order canonicalization helper.
fn canonicalize<T>(values: &mut Vec<T>)
where
    T: Ord,
{
    values.sort();
    values.dedup();
}

/// Direct query projection kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum QueryKind
{
    /// Left-facing equality.
    Eq,
    /// Left-facing non-equality.
    Lt,
    /// Reversed right-facing non-equality.
    Gt,
}

/// Build stable query rows from canonical direct rows.
fn query_rows<S>(
    direct_rows: &DirectRows<S>,
    kind: QueryKind,
) -> Result<QueryRows<S>, WalkBuildError>
where
    S: WalkSym,
{
    let mut rows: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for (direct_key, walks) in direct_rows {
        let &(dir, ref src, ref dst) = direct_key;
        for walk in walks {
            let has_equality_shape = walk.is_eq()?;
            let has_inequality_shape = walk.is_neq()?;
            let keep = match kind {
                | QueryKind::Eq => dir == Dir::Left && bool::from(has_equality_shape),
                | QueryKind::Lt => dir == Dir::Left && bool::from(has_inequality_shape),
                | QueryKind::Gt => dir == Dir::Right && bool::from(has_inequality_shape),
            };
            if keep {
                let query_key = match kind {
                    | QueryKind::Gt => (dst.clone(), src.clone()),
                    | QueryKind::Eq | QueryKind::Lt => (src.clone(), dst.clone()),
                };
                rows.entry(query_key).or_default().push(walk.clone());
            }
        }
    }
    Ok(rows)
}

/// A partial swing-closure path.
#[derive(Clone, Debug, Eq, PartialEq)]
struct PartialWalk<N, T>
{
    /// Current machine nonterminal.
    current: N,
    /// Built swings.
    swings: Vec<Swing<N>>,
    /// Built intervening stances.
    stances: Vec<T>,
}

impl<N: Clone, T: Clone> PartialWalk<N, T>
{
    /// Apply one swing advance.
    fn apply(
        &self,
        advance: &SwingAdvance<N, T>,
    ) -> Result<Self, WalkBuildError>
    {
        match advance {
            | &SwingAdvance::Stay => Ok(self.clone()),
            | &SwingAdvance::Extend(ref next) => {
                let mut swings = self.swings.clone();
                match swings.pop() {
                    | Some(mut swing) => {
                        swing.nonterminals.push(next.clone());
                        swings.push(swing);
                        Ok(Self {
                            current: next.clone(),
                            swings,
                            stances: self.stances.clone(),
                        })
                    },
                    | None => Err(WalkBuildError::InvalidWalkShape),
                }
            },
            | &SwingAdvance::Cross {
                ref stance,
                ref next,
            } => {
                let mut swings = self.swings.clone();
                let next_swing = Swing::new(vec![next.clone()])?;
                swings.push(next_swing);
                let mut stances = self.stances.clone();
                stances.push(stance.clone());
                Ok(Self {
                    current: next.clone(),
                    swings,
                    stances,
                })
            },
        }
    }

    /// Override the current continuation state.
    fn with_current(
        mut self,
        current: N,
    ) -> Self
    {
        self.current = current;
        self
    }
}

/// Compute a swing seen key.
fn seen_key<S>(
    nonterminal: &S::Nonterminal,
    mode: SwingKeyMode,
) -> (S::Sort, Option<S::Bounds>)
where
    S: WalkSym,
{
    match mode {
        | SwingKeyMode::SortBounds => (
            S::nonterminal_sort(nonterminal),
            Some(S::nonterminal_bounds(nonterminal)),
        ),
        | SwingKeyMode::SortOnly => (S::nonterminal_sort(nonterminal), None),
    }
}

/// Full canonical walk sort key.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct CanonicalWalkKey<N: Ord, T: Ord, Sort: Ord>
{
    /// Count of non-zero-height swings.
    height: WalkHeight,
    /// First top-level stance sort.
    top: Option<Sort>,
    /// Intermediate stance sorts in source order.
    mids: Vec<Sort>,
    /// First bottom-level stance sort.
    bottom: Option<Sort>,
    /// Alternating chain length tie-break.
    chain_len: WalkChainLength,
    /// Destination-to-source swing fallback.
    fallback_swings: Vec<FallbackSwing<N>>,
    /// Destination-to-source stance fallback.
    fallback_stances: Vec<T>,
}

/// Destination-to-source swing fallback key.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct FallbackSwing<N: Ord>
{
    /// Swing height.
    height: SwingHeight,
    /// Nonterminals in destination-to-source order.
    nonterminals: Vec<N>,
}

/// Build a canonical walk sort key.
fn canonical_walk_key<S>(walk: &MachineWalk<S>) -> Result<WalkSortKey<S>, WalkBuildError>
where
    S: WalkSym,
{
    let height = walk.height()?;
    let mut count = 0_u32;
    let mut top = None;
    let mut mids = Vec::new();
    let mut bottom = None;
    let mut stance_iter = walk.stances.iter();
    for swing in &walk.swings {
        let swing_height = swing.height()?;
        if swing_height != SwingHeight::default() {
            count = count
                .checked_add(1)
                .ok_or(WalkBuildError::ArithmeticOverflow)?;
        }
        if let Some(stance) = stance_iter.next() {
            let sort = S::stance_sort(stance);
            if count == 0 && top.is_none() {
                top = Some(sort);
            }
            else if WalkHeight::from(count) < height {
                mids.push(sort);
            }
            else if WalkHeight::from(count) == height && bottom.is_none() {
                bottom = Some(sort);
            }
        }
    }
    let chain_len = walk.chain_len()?;
    let mut fallback_swings = Vec::new();
    for swing in walk.swings.iter().rev() {
        let swing_height = swing.height()?;
        fallback_swings.push(FallbackSwing {
            height: swing_height,
            nonterminals: swing.nonterminals.iter().rev().cloned().collect(),
        });
    }
    let fallback_stances = walk.stances.iter().rev().cloned().collect();
    Ok(CanonicalWalkKey {
        height,
        top,
        mids,
        bottom,
        chain_len,
        fallback_swings,
        fallback_stances,
    })
}

/// Compute the stable framed FNV fingerprint.
fn fingerprint_index<S>(
    max_chain_len: WalkChainLength,
    direct_rows: &DirectRows<S>,
    transitive_rows: &DirectRows<S>,
    mold_rows: &MoldRows<S>,
) -> Result<Fingerprint, WalkBuildError>
where
    S: WalkSym,
{
    let mut hash = Fnv64::new();
    hash.write_bytes(b"gandr.walk.v1");
    hash.write_u32(u32::from(max_chain_len));
    hash_walk_rows::<S>(&mut hash, FingerprintByte::from(b'D'), direct_rows)?;
    hash_walk_rows::<S>(&mut hash, FingerprintByte::from(b'T'), transitive_rows)?;
    hash.write_byte(b'M');
    let mut label_ordinal = 0_u64;
    for row in mold_rows.values() {
        hash.write_byte(b'L');
        hash.write_u64(label_ordinal);
        label_ordinal = label_ordinal.wrapping_add(1);
        let row_len = u32::try_from(row.len())?;
        hash.write_u32(row_len);
        let mut mold_ordinal = 0_u64;
        for entry in row {
            hash_end::<S>(&mut hash, &entry.0);
            hash.write_u64(mold_ordinal);
            mold_ordinal = mold_ordinal.wrapping_add(1);
        }
    }
    Ok(hash.finish())
}

/// Hash walk rows.
fn hash_walk_rows<S>(
    hash: &mut Fnv64,
    tag: FingerprintByte,
    rows: &DirectRows<S>,
) -> Result<(), WalkBuildError>
where
    S: WalkSym,
{
    hash.write_byte(tag);
    let row_count = u32::try_from(rows.len())?;
    hash.write_u32(row_count);
    for (key, walks) in rows {
        let &(dir, ref src, ref dst) = key;
        hash.write_byte(match dir {
            | Dir::Left => b'L',
            | Dir::Right => b'R',
        });
        hash_end::<S>(hash, src);
        hash_end::<S>(hash, dst);
        let walk_count = u32::try_from(walks.len())?;
        hash.write_u32(walk_count);
        for walk in walks {
            hash_walk::<S>(hash, walk)?;
        }
    }
    Ok(())
}

/// Hash one exact end.
fn hash_end<S>(
    hash: &mut Fnv64,
    end: &End<S::Stance>,
) where
    S: WalkSym,
{
    match *end {
        | End::Root => hash.write_byte(0),
        | End::Node(ref stance) => {
            hash.write_byte(1);
            hash.write_u64(S::stance_key(stance));
        },
    }
}

/// Hash one walk shape.
fn hash_walk<S>(
    hash: &mut Fnv64,
    walk: &MachineWalk<S>,
) -> Result<(), WalkBuildError>
where
    S: WalkSym,
{
    let chain_len = walk.chain_len()?;
    let swing_count = u32::try_from(walk.swings.len())?;
    hash.write_u32(u32::from(chain_len));
    hash.write_u32(swing_count);
    for swing in &walk.swings {
        let swing_height = swing.height()?;
        let nonterminal_count = u32::try_from(swing.nonterminals.len())?;
        hash.write_u32(u32::from(swing_height));
        hash.write_u32(nonterminal_count);
        for nonterminal in &swing.nonterminals {
            hash.write_u64(S::nonterminal_key(nonterminal));
        }
    }
    let stance_count = u32::try_from(walk.stances.len())?;
    hash.write_u32(stance_count);
    for stance in &walk.stances {
        hash.write_u64(S::stance_key(stance));
    }
    Ok(())
}
