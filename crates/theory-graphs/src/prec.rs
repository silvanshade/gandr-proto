//! Named precedence DAGs for parser semantics.
//!
//! # Contract
//! - requires: callers construct precedence groups through [`PrecSpec`], so
//!   node identifiers are dense and edges mention existing groups.
//! - ensures: [`PrecDag::build`] rejects cyclic tighter-to-looser relations
//!   with deterministic closed witnesses and exposes total comparison queries
//!   that return `false` or `None` for invalid identifiers.
//! - provides: named precedence groups, associativity-controlled reflexive
//!   comparisons, virtual bottom/root bounds, deterministic topological order,
//!   and stable metadata-sensitive fingerprints.
//! - fails: [`PrecSpec`] methods return [`PrecSpecError`] for duplicate names,
//!   exhausted dense identifiers, or invalid edge endpoints; [`PrecDag::build`]
//!   returns [`PrecCycle`] for cyclic relations.
//! - panics: none.
//! - intension: edge rows are sorted and deduplicated before graph algorithms,
//!   Kahn's ready set chooses the smallest dense id on ties, and fingerprints
//!   use the crate's fixed FNV accumulator rather than process-random hashing.
//!
//! # Adequacy
//! - hypothesis: L3 pointwise plus L1 cycle evidence — named diamond,
//!   duplicate-edge, size/boundary, virtual-bound one-past validity, bound,
//!   fingerprint, deterministic-extension, and integer-chain witnesses
//!   distinguish every semantic branch; cycle witnesses are validated as closed
//!   walks over adjacent input edges.
//! - witness: `gandr_theory_graphs::prec::contracts::prec_spec_size_and_boundary_contract`
//! - witness: `gandr_theory_graphs::prec::contracts::prec_dag_size_and_boundary_contract`
//! - witness: `gandr_theory_graphs::prec::contracts::prec_dag_contract`
//! - witness: `gandr_theory_graphs::prec::contracts::prec_cycle_witness_contract`
//! - witness: `gandr_theory_graphs::prec::contracts::prec_integer_chain_oracle`
use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;
use core::convert::TryFrom as _;
use core::error::Error;
use core::fmt::Display;
use core::fmt::Formatter;
use core::fmt::Result as FmtResult;

use crate::EdgeSource;
use crate::Fingerprint;
use crate::FingerprintByte;
use crate::FingerprintBytes;
use crate::FingerprintWord16;
use crate::FingerprintWord64;
use crate::NodeCount;
use crate::NodeId;
use crate::NodePosition;
use crate::cycle_witness;
use crate::fingerprint::Fnv64;
use crate::reachability;

/// Dense precedence-group index bits.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PrecIndex(u16);

impl From<u16> for PrecIndex
{
    #[inline]
    fn from(value: u16) -> Self
    {
        Self(value)
    }
}

impl From<PrecIndex> for u16
{
    #[inline]
    fn from(value: PrecIndex) -> Self
    {
        value.0
    }
}

impl From<PrecIndex> for u32
{
    #[inline]
    fn from(value: PrecIndex) -> Self
    {
        Self::from(value.0)
    }
}

impl Display for PrecIndex
{
    #[inline]
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> FmtResult
    {
        Display::fmt(&self.0, f)
    }
}

/// Number of declared precedence groups.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PrecGroupCount(usize);

impl From<usize> for PrecGroupCount
{
    #[inline]
    fn from(value: usize) -> Self
    {
        Self(value)
    }
}

impl From<PrecGroupCount> for usize
{
    #[inline]
    fn from(value: PrecGroupCount) -> Self
    {
        value.0
    }
}

impl Display for PrecGroupCount
{
    #[inline]
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> FmtResult
    {
        Display::fmt(&self.0, f)
    }
}

/// Borrowed precedence group name.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PrecName<'name>(&'name str);

impl<'name> From<&'name str> for PrecName<'name>
{
    #[inline]
    fn from(value: &'name str) -> Self
    {
        Self(value)
    }
}

impl Display for PrecName<'_>
{
    #[inline]
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> FmtResult
    {
        Display::fmt(self.0, f)
    }
}

impl<'name> From<PrecName<'name>> for &'name str
{
    #[inline]
    fn from(value: PrecName<'name>) -> Self
    {
        value.0
    }
}

impl AsRef<str> for PrecName<'_>
{
    #[inline]
    fn as_ref(&self) -> &str
    {
        self.0
    }
}

impl PartialEq<PrecName<'_>> for &str
{
    #[inline]
    fn eq(
        &self,
        other: &PrecName<'_>,
    ) -> bool
    {
        *self == other.0
    }
}

impl PartialEq<&str> for PrecName<'_>
{
    #[inline]
    fn eq(
        &self,
        other: &&str,
    ) -> bool
    {
        self.0 == *other
    }
}

/// Public parser-precedence comparison result.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PrecedenceComparison(bool);

impl From<bool> for PrecedenceComparison
{
    #[inline]
    fn from(value: bool) -> Self
    {
        Self(value)
    }
}

impl From<PrecedenceComparison> for bool
{
    #[inline]
    fn from(value: PrecedenceComparison) -> Self
    {
        value.0
    }
}

impl Display for PrecedenceComparison
{
    #[inline]
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> FmtResult
    {
        Display::fmt(&self.0, f)
    }
}

/// Whether a precedence specification or DAG has no groups.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PrecSetEmpty(bool);

impl From<bool> for PrecSetEmpty
{
    #[inline]
    fn from(value: bool) -> Self
    {
        Self(value)
    }
}

impl From<PrecSetEmpty> for bool
{
    #[inline]
    fn from(value: PrecSetEmpty) -> Self
    {
        value.0
    }
}

impl Display for PrecSetEmpty
{
    #[inline]
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> FmtResult
    {
        Display::fmt(&self.0, f)
    }
}

/// Internal validity result for precedence ids.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct PrecValidity(bool);

impl From<bool> for PrecValidity
{
    #[inline]
    fn from(value: bool) -> Self
    {
        Self(value)
    }
}

impl From<PrecValidity> for bool
{
    #[inline]
    fn from(value: PrecValidity) -> Self
    {
        value.0
    }
}

/// Dense precedence group identifier.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub struct Prec(PrecIndex);

impl Prec
{
    /// Construct a precedence group identifier from its dense index.
    ///
    /// # Contract
    /// - requires: callers use indices obtained from [`PrecSpec`] when querying
    ///   a [`PrecDag`] or adding edges; arbitrary indices are permitted for
    ///   fail-closed boundary probes.
    /// - ensures: preserves `index` exactly and performs no validation.
    /// - provides: the public construction path for non-exhaustive precedence
    ///   ids.
    /// - fails: never; invalid ids are rejected by [`PrecSpec`] and [`PrecDag`]
    ///   operations that know the active group count.
    /// - panics: none.
    /// - intension: keeping construction explicit lets external contract tests
    ///   assert invalid-id behavior without exposing the tuple constructor.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — invalid-id and capacity witnesses observe
    ///   that constructed ids round-trip their dense index while graph queries
    ///   fail closed for ids outside the active dag.
    /// - witness: `gandr_theory_graphs::prec::contracts::prec_dag_contract`
    #[must_use]
    #[inline]
    pub const fn new(index: PrecIndex) -> Self
    {
        Self(index)
    }

    /// Return this precedence group's dense index.
    ///
    /// # Contract
    /// - requires: none.
    /// - ensures: returns the exact index supplied by [`Prec::new`] or
    ///   allocated by [`PrecSpec::insert`].
    /// - provides: the public inspection path for non-exhaustive precedence
    ///   ids.
    /// - fails: never.
    /// - panics: none.
    /// - intension: callers compare or serialize the stable dense id through
    ///   this accessor instead of depending on tuple-field layout.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — the capacity witness observes every
    ///   allocated id and checks that the public accessor reports insertion
    ///   order.
    /// - witness: `gandr_theory_graphs::prec::contracts::capacity_beyond_u16_is_typed`
    #[must_use]
    #[inline]
    pub const fn index(self) -> PrecIndex
    {
        self.0
    }
}

/// Parser associativity declared by one precedence group.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Assoc
{
    /// Left-associative operators admit reflexive greater-than comparisons.
    Left,
    /// Right-associative operators admit reflexive less-than comparisons.
    Right,
}

/// Virtual precedence bound or concrete precedence value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Bound<T>
{
    /// Strictly below every valid concrete precedence.
    Bottom,
    /// Concrete precedence value.
    Value(T),
    /// Strictly above every valid concrete precedence.
    Root,
}

/// A typed validation failure while constructing a precedence specification.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum PrecSpecError
{
    /// Dense `u16` identifiers are exhausted.
    CapacityExceeded,
    /// A group name was inserted more than once.
    DuplicateName
    {
        /// Duplicate group name.
        name: String,
    },
    /// An edge mentioned at least one non-existent precedence id.
    InvalidEdge
    {
        /// Tighter endpoint supplied by the caller.
        tighter: Prec,
        /// Looser endpoint supplied by the caller.
        looser: Prec,
        /// Number of currently declared groups.
        node_count: NodeCount,
    },
}

impl Display for PrecSpecError
{
    #[inline]
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> FmtResult
    {
        match *self {
            | Self::CapacityExceeded => f.write_str("precedence id capacity exceeded"),
            | Self::DuplicateName { ref name } => write!(f, "duplicate precedence name {name}"),
            | Self::InvalidEdge {
                tighter,
                looser,
                node_count,
            } => write!(
                f,
                "precedence edge {}->{} is outside 0..{}",
                tighter.index(),
                looser.index(),
                node_count
            ),
        }
    }
}

impl Error for PrecSpecError
{
}

/// Validated builder for named precedence groups and tighter-to-looser edges.
///
/// # Contract
/// - requires: inserted names identify parser precedence groups and edge
///   endpoints come from earlier successful [`insert`](Self::insert) calls.
/// - ensures: successful insertions return dense [`Prec`] ids in insertion
///   order, names are unique, and stored edges are canonical sorted pairs.
/// - provides: immutable inspection of group names, associativity declarations,
///   and canonical edges for consumers and [`PrecDag::build`].
/// - fails: returns [`PrecSpecError::CapacityExceeded`] after all `u16` ids are
///   used, [`PrecSpecError::DuplicateName`] for repeated names, and
///   [`PrecSpecError::InvalidEdge`] for non-existent endpoints.
/// - panics: none.
/// - intension: duplicate edges are accepted by reinserting into the canonical
///   sorted edge list and observing no semantic change.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — size/boundary, duplicate-edge, name/association
///   access, invalid edge, and stable fingerprint witnesses distinguish every
///   builder branch.
/// - witness: `gandr_theory_graphs::prec::contracts::prec_spec_size_and_boundary_contract`
/// - witness: `gandr_theory_graphs::prec::contracts::prec_dag_contract`
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PrecSpec
{
    /// Group names in dense id order.
    names: Vec<String>,
    /// Group associativity in dense id order; `None` is non-associative.
    assocs: Vec<Option<Assoc>>,
    /// Canonical sorted tighter-to-looser edge pairs.
    edges: Vec<(Prec, Prec)>,
}

impl PrecSpec
{
    /// Construct an empty precedence specification.
    #[must_use]
    #[inline]
    pub const fn new() -> Self
    {
        Self {
            names: Vec::new(),
            assocs: Vec::new(),
            edges: Vec::new(),
        }
    }

    /// Insert one named precedence group.
    ///
    /// # Contract
    /// - requires: `name` is the externally meaningful group name and has not
    ///   already been inserted.
    /// - ensures: returns the next dense [`Prec`] id and records `assoc` for
    ///   reflexive comparisons.
    /// - provides: a node id that may be used in later
    ///   [`add_edge`](Self::add_edge) calls.
    /// - fails: returns [`PrecSpecError::CapacityExceeded`] when no `u16` id is
    ///   available or [`PrecSpecError::DuplicateName`] when `name` already
    ///   exists.
    /// - panics: none.
    /// - intension: capacity is checked by fallible conversion from the current
    ///   length to `u16`, so the failing insertion leaves the spec unchanged.
    ///
    /// # Errors
    /// Returns [`PrecSpecError::CapacityExceeded`] or
    /// [`PrecSpecError::DuplicateName`].
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — name/association access and duplicate-name
    ///   witnesses observe successful dense ids and the exact duplicate branch.
    /// - witness: `gandr_theory_graphs::prec::contracts::prec_dag_contract`
    #[inline]
    pub fn insert<N>(
        &mut self,
        name: N,
        assoc: Option<Assoc>,
    ) -> Result<Prec, PrecSpecError>
    where
        N: Into<String>,
    {
        let name = name.into();
        if self.names.iter().any(|candidate| candidate == &name) {
            return Err(PrecSpecError::DuplicateName { name });
        }
        let id = u16::try_from(self.names.len()).map_err(|conversion_error| {
            let _ = conversion_error;
            PrecSpecError::CapacityExceeded
        })?;
        self.names.push(name);
        self.assocs.push(assoc);
        Ok(Prec::new(PrecIndex::from(id)))
    }

    /// Add a tighter-to-looser precedence edge.
    ///
    /// # Contract
    /// - requires: `tighter` and `looser` are ids returned by this spec.
    /// - ensures: the edge relation contains `tighter -> looser`; duplicate
    ///   insertions do not add duplicate stored pairs.
    /// - provides: canonical edge input for [`PrecDag::build`] and
    ///   [`edges`](Self::edges).
    /// - fails: returns [`PrecSpecError::InvalidEdge`] when either endpoint is
    ///   not currently valid.
    /// - panics: none.
    /// - intension: the edge vector is sorted and deduplicated after each
    ///   successful insertion so later fingerprints are insertion-order
    ///   neutral.
    ///
    /// # Errors
    /// Returns [`PrecSpecError::InvalidEdge`] for invalid endpoints.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — duplicate-edge canonicalization and invalid
    ///   edge witnesses observe both successful idempotence and the error
    ///   branch.
    /// - witness: `gandr_theory_graphs::prec::contracts::prec_dag_contract`
    #[inline]
    pub fn add_edge(
        &mut self,
        tighter: Prec,
        looser: Prec,
    ) -> Result<(), PrecSpecError>
    {
        if !bool::from(self.valid_prec(tighter)) || !bool::from(self.valid_prec(looser)) {
            return Err(PrecSpecError::InvalidEdge {
                tighter,
                looser,
                node_count: self.node_count(),
            });
        }
        self.edges.push((tighter, looser));
        self.edges.sort_unstable();
        self.edges.dedup();
        Ok(())
    }

    /// Return the number of declared groups.
    #[must_use]
    #[inline]
    pub fn len(&self) -> PrecGroupCount
    {
        PrecGroupCount::from(self.names.len())
    }

    /// Return whether no groups are declared.
    #[must_use]
    #[inline]
    pub fn is_empty(&self) -> PrecSetEmpty
    {
        PrecSetEmpty::from(self.names.is_empty())
    }

    /// Return the name for a valid precedence id.
    #[must_use]
    #[inline]
    pub fn name(
        &self,
        prec: Prec,
    ) -> Option<PrecName<'_>>
    {
        self.names
            .get(usize::from(u16::from(prec.index())))
            .map(String::as_str)
            .map(PrecName::from)
    }

    /// Return the associativity declaration for a valid precedence id.
    #[must_use]
    #[inline]
    pub fn assoc(
        &self,
        prec: Prec,
    ) -> Option<Option<Assoc>>
    {
        self.assocs
            .get(usize::from(u16::from(prec.index())))
            .copied()
    }

    /// Iterate over declared groups in dense id order.
    #[inline]
    pub fn groups(&self) -> impl Iterator<Item = (Prec, PrecName<'_>, Option<Assoc>)> + '_
    {
        self.names
            .iter()
            .zip(self.assocs.iter())
            .enumerate()
            .filter_map(|(index, (name, assoc))| {
                u16::try_from(index).ok().map(|id| {
                    (
                        Prec::new(PrecIndex::from(id)),
                        PrecName::from(name.as_str()),
                        *assoc,
                    )
                })
            })
    }

    /// Iterate over canonical tighter-to-looser edges.
    #[inline]
    pub fn edges(&self) -> impl Iterator<Item = (Prec, Prec)> + '_
    {
        self.edges.iter().copied()
    }

    /// Return whether `prec` is valid for this spec.
    fn valid_prec(
        &self,
        prec: Prec,
    ) -> PrecValidity
    {
        PrecValidity::from(usize::from(u16::from(prec.index())) < self.names.len())
    }

    /// Return the group count as `u32` for diagnostics and graph algorithms.
    fn node_count(&self) -> NodeCount
    {
        match u32::try_from(self.names.len()) {
            | Ok(count) => NodeCount::from(count),
            | Err(_conversion_error) => NodeCount::from(u32::MAX),
        }
    }
}

/// Deterministic closed precedence cycle witness.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct PrecCycle
{
    /// Closed walk of precedence ids; first and last entries are equal.
    pub witness: Vec<Prec>,
}

impl Display for PrecCycle
{
    #[inline]
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> FmtResult
    {
        f.write_str("precedence cycle")?;
        for node in &self.witness {
            write!(f, " {}", node.index())?;
        }
        Ok(())
    }
}

impl Error for PrecCycle
{
}

/// Built precedence DAG with precomputed reachability and deterministic order.
///
/// # Contract
/// - requires: input comes from [`PrecSpec`].
/// - ensures: cyclic inputs return [`PrecCycle`]; acyclic inputs preserve group
///   metadata, canonical edges, strict reachability, and a deterministic linear
///   extension.
/// - provides: total parser precedence comparisons, bound comparisons, metadata
///   inspection, canonical edge inspection, and stable fingerprints.
/// - fails: returns [`PrecCycle`] with a closed deterministic witness when a
///   tighter-to-looser cycle exists.
/// - panics: none.
/// - intension: cycle and reachability observations are delegated to the
///   crate's graph algorithms over a private dense [`EdgeSource`], then the
///   public linear extension is recomputed with smallest-id Kahn tie breaking.
///
/// # Errors
/// Returns [`PrecCycle`] when the canonical relation is cyclic.
///
/// # Adequacy
/// - hypothesis: L3 pointwise plus L1 evidence — size/boundary, virtual-bound
///   one-past validity, diamond, incomparability, cycle, bound, fingerprint,
///   deterministic-extension, and integer-chain witnesses distinguish the
///   strict relation, reflexive associativity, virtual-bound equality, and
///   deterministic observations.
/// - witness: `gandr_theory_graphs::prec::contracts::prec_dag_size_and_boundary_contract`
/// - witness: `gandr_theory_graphs::prec::contracts::prec_dag_contract`
/// - witness: `gandr_theory_graphs::prec::contracts::prec_cycle_witness_contract`
/// - witness: `gandr_theory_graphs::prec::contracts::prec_integer_chain_oracle`
#[derive(Clone, Debug)]
pub struct PrecDag
{
    /// Group names in dense id order.
    names: Vec<String>,
    /// Group associativity in dense id order; `None` is non-associative.
    assocs: Vec<Option<Assoc>>,
    /// Canonical sorted tighter-to-looser edge pairs.
    edges: Vec<(Prec, Prec)>,
    /// Strict reachability rows in dense source order.
    reachability: Vec<Vec<Prec>>,
    /// Deterministic topological order.
    linear_extension: Vec<Prec>,
}

impl PrecDag
{
    /// Build an acyclic precedence DAG from a validated specification.
    ///
    /// # Contract
    /// - requires: `spec` was built through [`PrecSpec`].
    /// - ensures: on success, all strict comparisons follow the transitive
    ///   tighter-to-looser closure and
    ///   [`linear_extension`](Self::linear_extension) orders every edge source
    ///   before its target.
    /// - provides: a fully precomputed [`PrecDag`] for parser comparison
    ///   queries.
    /// - fails: returns [`PrecCycle`] when `spec` contains a self-cycle or
    ///   multi-node cycle.
    /// - panics: none.
    /// - intension: edges are fed to [`cycle_witness`] and [`reachability`]
    ///   using canonical dense rows before Kahn's smallest-id tie-breaking
    ///   pass.
    ///
    /// # Errors
    /// Returns [`PrecCycle`] for cyclic precedence relations.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise plus L1 cycle evidence — self-cycle and
    ///   multi-node witnesses validate closed walks, while diamond/chain and
    ///   public size/boundary witnesses observe the closure, one-past validity,
    ///   and extension success path.
    /// - witness: `gandr_theory_graphs::prec::contracts::prec_dag_size_and_boundary_contract`
    /// - witness: `gandr_theory_graphs::prec::contracts::prec_dag_contract`
    /// - witness: `gandr_theory_graphs::prec::contracts::prec_cycle_witness_contract`
    #[inline]
    pub fn build(spec: &PrecSpec) -> Result<Self, PrecCycle>
    {
        let graph = PrecGraphSource::from_spec(spec);
        match cycle_witness(&graph) {
            | Ok(Some(witness)) => return Err(PrecCycle::from_nodes(witness.nodes)),
            | Ok(None) => {},
            | Err(_validation_error) => {
                return Err(PrecCycle {
                    witness: Vec::new(),
                });
            },
        }
        let reachability = match reachability(&graph) {
            | Ok(reachability) => reachability_from_rows(reachability.rows),
            | Err(_validation_error) => {
                return Err(PrecCycle {
                    witness: Vec::new(),
                });
            },
        };
        let linear_extension = linear_extension_from_edges(spec.len(), &spec.edges)?;
        Ok(Self {
            names: spec.names.clone(),
            assocs: spec.assocs.clone(),
            edges: spec.edges.clone(),
            reachability,
            linear_extension,
        })
    }

    /// Return the number of precedence groups.
    #[must_use]
    #[inline]
    pub fn len(&self) -> PrecGroupCount
    {
        PrecGroupCount::from(self.names.len())
    }

    /// Return whether the DAG has no precedence groups.
    #[must_use]
    #[inline]
    pub fn is_empty(&self) -> PrecSetEmpty
    {
        PrecSetEmpty::from(self.names.is_empty())
    }

    /// Return the name for a valid precedence id.
    #[must_use]
    #[inline]
    pub fn name(
        &self,
        prec: Prec,
    ) -> Option<PrecName<'_>>
    {
        self.names
            .get(usize::from(u16::from(prec.index())))
            .map(String::as_str)
            .map(PrecName::from)
    }

    /// Return the associativity declaration for a valid precedence id.
    #[must_use]
    #[inline]
    pub fn assoc(
        &self,
        prec: Prec,
    ) -> Option<Option<Assoc>>
    {
        self.assocs
            .get(usize::from(u16::from(prec.index())))
            .copied()
    }

    /// Iterate over declared groups in dense id order.
    #[inline]
    pub fn groups(&self) -> impl Iterator<Item = (Prec, PrecName<'_>, Option<Assoc>)> + '_
    {
        self.names
            .iter()
            .zip(self.assocs.iter())
            .enumerate()
            .filter_map(|(index, (name, assoc))| {
                u16::try_from(index).ok().map(|id| {
                    (
                        Prec::new(PrecIndex::from(id)),
                        PrecName::from(name.as_str()),
                        *assoc,
                    )
                })
            })
    }

    /// Iterate over canonical tighter-to-looser edges.
    #[inline]
    pub fn edges(&self) -> impl Iterator<Item = (Prec, Prec)> + '_
    {
        self.edges.iter().copied()
    }

    /// Return owned precedence ids in deterministic topological order.
    #[must_use]
    #[inline]
    pub fn linear_extension(&self) -> Vec<Prec>
    {
        self.linear_extension.clone()
    }

    /// Return whether `left` is less-than `right` under parser semantics.
    ///
    /// # Contract
    /// - requires: `left` and `right` may be arbitrary precedence ids; invalid
    ///   ids are permitted inputs.
    /// - ensures: distinct valid ids return `true` when `right` is strictly
    ///   tighter than `left` in the tighter-to-looser reachability closure;
    ///   equal ids return `true` exactly for a right-associative group when
    ///   `assoc == Some(Assoc::Right)`.
    /// - provides: the parser less-than predicate.
    /// - fails: invalid ids return `false`.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — diamond/incomparability and integer-chain
    ///   witnesses distinguish strict reachability from reflexive
    ///   associativity.
    /// - witness: `gandr_theory_graphs::prec::contracts::prec_dag_contract`
    /// - witness: `gandr_theory_graphs::prec::contracts::prec_integer_chain_oracle`
    #[must_use]
    #[inline]
    pub fn lt(
        &self,
        left: Prec,
        right: Prec,
        assoc: Option<Assoc>,
    ) -> PrecedenceComparison
    {
        if !bool::from(self.valid_prec(left)) || !bool::from(self.valid_prec(right)) {
            return PrecedenceComparison::from(false);
        }
        if left == right {
            return PrecedenceComparison::from(
                self.assoc(left) == Some(Some(Assoc::Right)) && assoc == Some(Assoc::Right),
            );
        }
        self.strict_reaches(right, left)
    }

    /// Return whether `left` is greater-than `right` under parser semantics.
    ///
    /// # Contract
    /// - requires: `left` and `right` may be arbitrary precedence ids; invalid
    ///   ids are permitted inputs.
    /// - ensures: distinct valid ids are the exact dual of [`lt`](Self::lt);
    ///   equal ids return `true` exactly for a left-associative group when
    ///   `assoc == Some(Assoc::Left)`.
    /// - provides: the parser greater-than predicate.
    /// - fails: invalid ids return `false`.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — proptest duality and integer-chain
    ///   witnesses distinguish dual strict reachability from reflexive
    ///   associativity.
    /// - witness: `gandr_theory_graphs::prec::contracts::prec_dag_contract`
    /// - witness: `gandr_theory_graphs::prec::contracts::prec_integer_chain_oracle`
    #[must_use]
    #[inline]
    pub fn gt(
        &self,
        left: Prec,
        right: Prec,
        assoc: Option<Assoc>,
    ) -> PrecedenceComparison
    {
        if !bool::from(self.valid_prec(left)) || !bool::from(self.valid_prec(right)) {
            return PrecedenceComparison::from(false);
        }
        if left == right {
            return PrecedenceComparison::from(
                self.assoc(left) == Some(Some(Assoc::Left)) && assoc == Some(Assoc::Left),
            );
        }
        self.strict_reaches(left, right)
    }

    /// Return whether two ids are equal under parser semantics.
    #[must_use]
    #[inline]
    pub fn eq(
        &self,
        left: Prec,
        right: Prec,
        assoc: Option<Assoc>,
    ) -> PrecedenceComparison
    {
        PrecedenceComparison::from(
            bool::from(self.valid_prec(left))
                && bool::from(self.valid_prec(right))
                && left == right
                && self.assoc(left) == Some(None)
                && assoc.is_none(),
        )
    }

    /// Return whether two precedence ids are equality- or
    /// reachability-comparable.
    #[must_use]
    #[inline]
    pub fn comparable(
        &self,
        left: Prec,
        right: Prec,
    ) -> PrecedenceComparison
    {
        if !bool::from(self.valid_prec(left)) || !bool::from(self.valid_prec(right)) {
            return PrecedenceComparison::from(false);
        }
        PrecedenceComparison::from(
            left == right
                || bool::from(self.strict_reaches(left, right))
                || bool::from(self.strict_reaches(right, left)),
        )
    }

    /// Return whether `left` is less-than `right` with virtual bounds.
    #[must_use]
    #[inline]
    pub fn bound_lt(
        &self,
        left: Bound<Prec>,
        right: Bound<Prec>,
        assoc: Option<Assoc>,
    ) -> PrecedenceComparison
    {
        match (left, right) {
            | (Bound::Root, _) | (_, Bound::Bottom) => PrecedenceComparison::from(false),
            | (Bound::Bottom, Bound::Root) => PrecedenceComparison::from(true),
            | (Bound::Bottom, Bound::Value(prec)) | (Bound::Value(prec), Bound::Root) => {
                PrecedenceComparison::from(bool::from(self.valid_prec(prec)))
            },
            | (Bound::Value(left), Bound::Value(right)) => self.lt(left, right, assoc),
        }
    }

    /// Return whether `left` is greater-than `right` with virtual bounds.
    #[must_use]
    #[inline]
    pub fn bound_gt(
        &self,
        left: Bound<Prec>,
        right: Bound<Prec>,
        assoc: Option<Assoc>,
    ) -> PrecedenceComparison
    {
        match (left, right) {
            | (Bound::Bottom, _) | (_, Bound::Root) => PrecedenceComparison::from(false),
            | (Bound::Root, Bound::Bottom) => PrecedenceComparison::from(true),
            | (Bound::Root, Bound::Value(prec)) | (Bound::Value(prec), Bound::Bottom) => {
                PrecedenceComparison::from(bool::from(self.valid_prec(prec)))
            },
            | (Bound::Value(left), Bound::Value(right)) => self.gt(left, right, assoc),
        }
    }

    /// Return whether two bounds are equal under parser semantics.
    ///
    /// # Contract
    /// - ensures: concrete values delegate to [`eq`](Self::eq); virtual bounds
    ///   are equal only when both sides are the same virtual bound.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — virtual-bound equality witnesses cover same
    ///   bottom, same root, bottom/root mismatch, and virtual/concrete mismatch
    ///   quadrants.
    /// - witness: `gandr_theory_graphs::prec::contracts::virtual_bound_comparisons`
    #[must_use]
    #[inline]
    pub fn bound_eq(
        &self,
        left: Bound<Prec>,
        right: Bound<Prec>,
        assoc: Option<Assoc>,
    ) -> PrecedenceComparison
    {
        match (left, right) {
            | (Bound::Value(left), Bound::Value(right)) => self.eq(left, right, assoc),
            | (left, right) => PrecedenceComparison::from(
                matches!(left, Bound::Bottom | Bound::Root) && left == right,
            ),
        }
    }

    /// Return whether two bounds are equality- or reachability-comparable.
    #[must_use]
    #[inline]
    pub fn bound_comparable(
        &self,
        left: Bound<Prec>,
        right: Bound<Prec>,
    ) -> PrecedenceComparison
    {
        match (left, right) {
            | (Bound::Bottom | Bound::Root, Bound::Bottom | Bound::Root) => {
                PrecedenceComparison::from(true)
            },
            | (Bound::Bottom | Bound::Root, Bound::Value(prec))
            | (Bound::Value(prec), Bound::Bottom | Bound::Root) => {
                PrecedenceComparison::from(bool::from(self.valid_prec(prec)))
            },
            | (Bound::Value(left), Bound::Value(right)) => self.comparable(left, right),
        }
    }

    /// Return the fixed FNV-1a fingerprint over metadata and canonical edges.
    ///
    /// # Contract
    /// - ensures: equal names, associativity tags, and canonical relations
    ///   return equal fingerprints independent of duplicate or reordered edge
    ///   insertion; name, associativity, or relation changes alter the observed
    ///   stream.
    /// - provides: a stable [`Fingerprint`] for parser table cache keys.
    /// - panics: none.
    /// - intension: writes group count, length-delimited UTF-8 names,
    ///   associativity tags, and sorted/deduplicated edges into [`Fnv64`].
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — fingerprint sensitivity witnesses separate
    ///   duplicate/reordered edges from name, association, and relation
    ///   changes.
    /// - witness: `gandr_theory_graphs::prec::contracts::prec_dag_contract`
    #[must_use]
    #[inline]
    pub fn fingerprint(&self) -> Fingerprint
    {
        let mut state = Fnv64::new();
        state.write_u64(FingerprintWord64::from(
            match u64::try_from(self.names.len()) {
                | Ok(count) => count,
                | Err(_conversion_error) => u64::MAX,
            },
        ));
        for (name, assoc) in self.names.iter().zip(self.assocs.iter()) {
            let bytes = name.as_bytes();
            state.write_u64(FingerprintWord64::from(match u64::try_from(bytes.len()) {
                | Ok(len) => len,
                | Err(_conversion_error) => u64::MAX,
            }));
            state.write_bytes(FingerprintBytes::from(bytes));
            state.write_byte(assoc_tag(*assoc));
        }
        state.write_u64(FingerprintWord64::from(
            match u64::try_from(self.edges.len()) {
                | Ok(count) => count,
                | Err(_conversion_error) => u64::MAX,
            },
        ));
        for (source, target) in self.edges.iter().copied() {
            state.write_u16(FingerprintWord16::from(u16::from(source.index())));
            state.write_u16(FingerprintWord16::from(u16::from(target.index())));
        }
        state.finish()
    }

    /// Return whether `prec` is a valid node in this DAG.
    fn valid_prec(
        &self,
        prec: Prec,
    ) -> PrecValidity
    {
        PrecValidity::from(usize::from(u16::from(prec.index())) < self.names.len())
    }

    /// Return whether `source` strictly reaches `target`.
    fn strict_reaches(
        &self,
        source: Prec,
        target: Prec,
    ) -> PrecedenceComparison
    {
        self.reachability
            .get(usize::from(u16::from(source.index())))
            .is_some_and(|targets| targets.contains(&target))
            .into()
    }
}

/// Dense graph source over canonical precedence rows.
#[repr(transparent)]
struct PrecGraphSource
{
    /// Sorted successor rows.
    rows: Vec<Vec<NodeId>>,
}

impl PrecGraphSource
{
    /// Build graph rows from a precedence spec.
    fn from_spec(spec: &PrecSpec) -> Self
    {
        let mut rows = Vec::with_capacity(usize::from(spec.len()));
        for _group in spec.groups() {
            rows.push(Vec::new());
        }
        for (source, target) in spec.edges() {
            if let Some(row) = rows.get_mut(usize::from(u16::from(source.index()))) {
                row.push(NodeId::from(u32::from(target.index())));
            }
        }
        for row in &mut rows {
            row.sort_unstable();
            row.dedup();
        }
        Self { rows }
    }
}

impl EdgeSource for PrecGraphSource
{
    type Successors<'successors>
        = core::iter::Copied<core::slice::Iter<'successors, NodeId>>
    where
        Self: 'successors;

    #[inline]
    fn node_count(&self) -> NodeCount
    {
        match u32::try_from(self.rows.len()) {
            | Ok(count) => NodeCount::from(count),
            | Err(_conversion_error) => NodeCount::from(u32::MAX),
        }
    }

    #[inline]
    fn successors(
        &self,
        node: NodeId,
    ) -> Self::Successors<'_>
    {
        static EMPTY: [NodeId; 0] = [];

        NodePosition::try_from(node)
            .ok()
            .and_then(|position| self.rows.get(usize::from(position)))
            .map_or_else(|| EMPTY.iter().copied(), |row| row.iter().copied())
    }
}

/// Convert a graph cycle witness into a precedence cycle.
impl PrecCycle
{
    /// Build a precedence cycle from dense graph nodes.
    fn from_nodes(nodes: Vec<NodeId>) -> Self
    {
        let mut witness = Vec::with_capacity(nodes.len());
        for node in nodes {
            if let Ok(id) = u16::try_from(u32::from(node)) {
                witness.push(Prec::new(PrecIndex::from(id)));
            }
        }
        Self { witness }
    }
}

/// Convert reachability rows into precedence reachability rows.
fn reachability_from_rows(rows: Vec<crate::ReachabilityRow>) -> Vec<Vec<Prec>>
{
    let mut converted = Vec::with_capacity(rows.len());
    for _row in &rows {
        converted.push(Vec::new());
    }
    for row in rows {
        if let Ok(position) = NodePosition::try_from(row.source)
            && let Some(targets) = converted.get_mut(usize::from(position))
        {
            for target in row.targets {
                if let Ok(id) = u16::try_from(u32::from(target)) {
                    targets.push(Prec::new(PrecIndex::from(id)));
                }
            }
        }
    }
    converted
}

/// Compute smallest-id-tie Kahn linear extension.
fn linear_extension_from_edges(
    node_count: PrecGroupCount,
    edges: &[(Prec, Prec)],
) -> Result<Vec<Prec>, PrecCycle>
{
    let mut rows = Vec::with_capacity(usize::from(node_count));
    let mut indegree = Vec::with_capacity(usize::from(node_count));
    for _node in 0 .. usize::from(node_count) {
        rows.push(Vec::<Prec>::new());
        indegree.push(0_usize);
    }
    for (source, target) in edges.iter().copied() {
        if let Some(row) = rows.get_mut(usize::from(u16::from(source.index()))) {
            row.push(target);
        }
        let Some(degree) = indegree.get_mut(usize::from(u16::from(target.index())))
        else {
            return Err(PrecCycle {
                witness: Vec::new(),
            });
        };
        *degree = degree.checked_add(1).ok_or(PrecCycle {
            witness: Vec::new(),
        })?;
    }
    for row in &mut rows {
        row.sort_unstable();
        row.dedup();
    }

    let mut ready = BTreeSet::new();
    for (index, degree) in indegree.iter().enumerate() {
        if *degree == 0
            && let Ok(id) = u16::try_from(index)
        {
            ready.insert(Prec::new(PrecIndex::from(id)));
        }
    }

    let mut order = Vec::with_capacity(usize::from(node_count));
    while let Some(node) = ready.pop_first() {
        order.push(node);
        let Some(targets) = rows.get(usize::from(u16::from(node.index())))
        else {
            return Err(PrecCycle {
                witness: Vec::new(),
            });
        };
        for &target in targets {
            let Some(degree) = indegree.get_mut(usize::from(u16::from(target.index())))
            else {
                return Err(PrecCycle {
                    witness: Vec::new(),
                });
            };
            *degree = degree.checked_sub(1).ok_or(PrecCycle {
                witness: Vec::new(),
            })?;
            if *degree == 0 {
                ready.insert(target);
            }
        }
    }
    if order.len() == usize::from(node_count) {
        Ok(order)
    }
    else {
        Err(PrecCycle {
            witness: Vec::new(),
        })
    }
}

/// Return the canonical byte tag for an associativity declaration.
fn assoc_tag(assoc: Option<Assoc>) -> FingerprintByte
{
    match assoc {
        | None => FingerprintByte::from(0),
        | Some(Assoc::Left) => FingerprintByte::from(1),
        | Some(Assoc::Right) => FingerprintByte::from(2),
    }
}
