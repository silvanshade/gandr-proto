//! Atom minting for sealing, and the table that makes freshness **checkable**.
//!
//! Opaque ascription mints one fresh nominal atom per abstract type component
//! and returns the module at its signature with those components rebound to the
//! atoms. This module owns the minting and the record of it.
//!
//! # Why there is a table at all
//!
//! An abstract type could have been introduced by quantifying existentially
//! over its identity, and the two presentations are inter-translatable in the
//! fragment gandr restricts itself to. What is *not* symmetric is what an
//! admission point can verify. An existential presentation offers an importer
//! nothing to check — the packing already happened, elsewhere, and the artifact
//! can only be believed. Minting against a table offers a check, because
//! minting is a **function of the sealing site** and a site is recorded in the
//! artifact:
//!
//! > re-elaborate, re-mint, and require the result to equal what was recorded.
//!
//! That is the whole mechanism, and it is the reason the atom route was taken.
//!
//! # What a site is, and why it is the right key
//!
//! A [`SealSite`] is a declaration name and a component label: *this* module
//! declaration's opaque ascription, sealing *that* abstract type component. Two
//! properties follow, and both are load-bearing rather than convenient.
//!
//! **Sealing is declaration-granular**, so a site always has its minting
//! declaration in scope — which is the anchorability invariant, maintained by
//! construction rather than argued for. Every minted atom has an in-scope
//! defining occurrence, and [`SealTable::sites`] is where that is observable.
//!
//! **Re-minting is deterministic**, because elaborating one program meets one
//! sequence of sites in one order. So the serials are a function of the program
//! rather than of the run, and [`SealTable::verify_reminting`] can refute a
//! recorded sequence that a re-elaboration does not reproduce.
//!
//! # The one property this does not give
//!
//! Determinism and *global uniqueness* are different, and only the first is
//! claimed here. Two independently elaborated programs both start their serials
//! at zero, so two sealed values crossing a process boundary can carry
//! colliding identities. Nothing in this module closes that, and the fix is not
//! a wider counter — it belongs with the package boundary, which is where a
//! sealed value first crosses a process at all.

use alloc::collections::BTreeMap;
use alloc::rc::Rc;
use alloc::vec::Vec;
use core::fmt;

use crate::boundary::TypeSerial;
use crate::types::SealId;
pub use crate::types::SealSite;

/// Why a minting or a re-minting check failed.
///
/// Both variants are refusals of a **claim about freshness**, which is why they
/// live together: one catches a program that seals the same component twice,
/// the other catches a recorded sequence that re-elaboration does not
/// reproduce.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SealError
{
    /// Two seals claimed one site.
    ///
    /// The elaborator would otherwise mint two distinct atoms for one
    /// component, and the second would shadow the first in every later
    /// reference — an aliasing hazard that is silent at the point it is
    /// created and only observable much later, as a type error at a use
    /// site far away.
    SiteAlreadyMinted
    {
        /// The site claimed twice.
        site: SealSite,
    },
    /// A re-elaboration did not reproduce a recorded minting.
    ///
    /// Either the sites differ, or the same sites came out in a different order
    /// and therefore at different serials. Both mean the recorded atoms are not
    /// the atoms this program mints, so the record cannot be trusted to name
    /// them.
    RemintingDiverged
    {
        /// The serial at which the recorded and re-minted sequences first
        /// disagreed.
        serial: TypeSerial,
    },
    /// A re-elaboration minted a different number of atoms than was recorded.
    RemintingCountDiverged
    {
        /// How many atoms were recorded.
        recorded: usize,
        /// How many this table holds.
        minted: usize,
    },
}

impl fmt::Display for SealError
{
    #[inline]
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        match *self {
            | Self::SiteAlreadyMinted { ref site } => {
                write!(f, "the sealing site `{site}` was already minted")
            },
            | Self::RemintingDiverged { serial } => {
                write!(
                    f,
                    "re-minting diverged from the recorded sealing at serial {}",
                    u64::from(serial)
                )
            },
            | Self::RemintingCountDiverged { recorded, minted } => {
                write!(
                    f,
                    "re-minting produced {minted} atoms where {recorded} were recorded"
                )
            },
        }
    }
}

impl core::error::Error for SealError
{
}

/// The minted-atom table for one elaboration: every atom sealing minted, in
/// minting order, with the site that produced it.
///
/// It is the elaborator-side half of the freshness discipline. Its kernel-side
/// counterpart is the export format's minted-atom table, and the two check
/// different things on purpose: this one refutes a *program* that seals one
/// component twice, and the kernel's refutes an *artifact* whose recorded atoms
/// are not the atoms its declarations contain.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SealTable
{
    /// The sites minted, in minting order; the index is the serial. Each is
    /// shared with the [`SealId`] minted for it, so a site is stored once.
    order: Vec<Rc<SealSite>>,
    /// Site → the serial minted for it, for the duplicate refusal.
    minted: BTreeMap<SealSite, TypeSerial>,
}

impl SealTable
{
    /// An empty table — no atom minted yet.
    #[inline]
    #[must_use]
    pub fn new() -> Self
    {
        Self::default()
    }

    /// Mint one fresh atom for `site`.
    ///
    /// # Contract
    /// - requires: nothing.
    /// - ensures: `Ok(id)` with the next serial in minting order, recorded
    ///   against `site`, exactly when `site` has not been minted before; the
    ///   table is left unchanged on refusal.
    /// - provides: the one way an abstract type component acquires an identity.
    /// - fails: [`SealError::SiteAlreadyMinted`] when `site` was already minted
    ///   — the duplicate is a defect in the program, refused where it happens
    ///   rather than at a use site later.
    /// - panics: none.
    ///
    /// # Errors
    /// As `- fails:`.
    ///
    /// # Adequacy
    /// - hypothesis: L2 — minting order is the serial order and each site mints
    ///   once; the L3 residue is the duplicate refusal.
    /// - witness: `seal::tests::minting_is_ordered_and_each_site_mints_once`
    /// - witness: `seal::tests::a_repeated_site_is_refused`
    #[inline]
    pub fn mint(
        &mut self,
        site: SealSite,
    ) -> Result<SealId, SealError>
    {
        if self.minted.contains_key(&site) {
            return Err(SealError::SiteAlreadyMinted { site });
        }
        let serial = TypeSerial::from(u64::try_from(self.order.len()).unwrap_or(u64::MAX));
        let shared = Rc::new(site);
        let id = SealId::at_site(serial, Rc::clone(&shared));
        let _prior = self.minted.insert(shared.as_ref().clone(), serial);
        self.order.push(shared);
        Ok(id)
    }

    /// The sites minted, in minting order — the index of a site is its serial.
    ///
    /// Every entry is a declaration that was in scope when it minted, so this
    /// is also where the **anchorability** invariant is observable: a
    /// minted atom with no in-scope defining occurrence cannot appear here,
    /// because a site is named by the declaration that produced it.
    #[inline]
    #[must_use]
    pub fn sites(&self) -> &[Rc<SealSite>]
    {
        &self.order
    }

    /// The identity a site was minted at, if it was minted.
    #[inline]
    #[must_use]
    pub fn lookup(
        &self,
        site: &SealSite,
    ) -> Option<SealId>
    {
        self.minted
            .get(site)
            .map(|&serial| SealId::new(serial, site.declaration(), site.component()))
    }

    /// **The freshness check.** Refute a recorded minting against this one.
    ///
    /// This is what turns freshness from a claim into a property. A recorded
    /// sequence of sites is compared against the sequence this elaboration
    /// actually minted — same sites, same order, therefore same serials, or the
    /// record is refused. Nothing is taken on trust: the comparison is against
    /// what re-elaboration produced, and re-elaboration is the only authority.
    ///
    /// Order is compared, not just membership, because the serial *is* the
    /// position. Two programs minting the same two sites in opposite orders
    /// mint different atoms, and a check that accepted both would be
    /// accepting a record that names the wrong ones.
    ///
    /// # Contract
    /// - requires: `recorded` is a previously recorded minting order.
    /// - ensures: `Ok(())` exactly when `recorded` is this table's minting
    ///   order, element for element.
    /// - provides: the elaborator-side re-minting check, the counterpart of the
    ///   export format's minted-atom table cross-check.
    /// - fails: [`SealError::RemintingCountDiverged`] on a length disagreement,
    ///   [`SealError::RemintingDiverged`] at the first differing serial.
    /// - panics: none.
    ///
    /// # Errors
    /// As `- fails:`.
    ///
    /// # Adequacy
    /// - hypothesis: L2 — a re-elaboration of one program verifies against its
    ///   own record; the L3 residues are the two divergences, each pinned by a
    ///   perturbed record.
    /// - witness: `seal::tests::re_minting_the_same_sites_verifies`
    /// - witness: `seal::tests::a_reordered_record_is_refused`
    /// - witness: `seal::tests::a_truncated_record_is_refused`
    #[inline]
    pub fn verify_reminting(
        &self,
        recorded: &[SealSite],
    ) -> Result<(), SealError>
    {
        if recorded.len() != self.order.len() {
            return Err(SealError::RemintingCountDiverged {
                recorded: recorded.len(),
                minted: self.order.len(),
            });
        }
        for (position, (recorded_site, minted_site)) in
            recorded.iter().zip(self.order.iter()).enumerate()
        {
            if recorded_site != minted_site.as_ref() {
                return Err(SealError::RemintingDiverged {
                    serial: TypeSerial::from(u64::try_from(position).unwrap_or(u64::MAX)),
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests
{
    use alloc::vec;
    use alloc::vec::Vec;

    use super::SealError;
    use super::SealSite;
    use super::SealTable;
    use crate::boundary::TypeSerial;

    /// A two-component sealing: `Counter` sealing `t` and `u`.
    fn counter_sites() -> [SealSite; 2]
    {
        [SealSite::new("Counter", "t"), SealSite::new("Counter", "u")]
    }

    #[test]
    fn minting_is_ordered_and_each_site_mints_once()
    {
        let mut table = SealTable::new();
        let [first_site, second_site] = counter_sites();
        let first = table.mint(first_site.clone()).unwrap();
        let second = table.mint(second_site.clone()).unwrap();
        assert_eq!(
            TypeSerial::from(0_u64),
            first.serial(),
            "the first mint takes serial zero"
        );
        assert_eq!(
            TypeSerial::from(1_u64),
            second.serial(),
            "serials follow minting order"
        );
        assert_ne!(first, second, "two components mint two distinct atoms");
        let recorded: Vec<SealSite> = table
            .sites()
            .iter()
            .map(|site| site.as_ref().clone())
            .collect();
        assert_eq!(
            vec![first_site, second_site],
            recorded,
            "the table records the minting order"
        );
    }

    /// Two sealings of one component are refused where they happen. Minting the
    /// second would shadow the first everywhere later, and the resulting type
    /// error would surface far from its cause.
    #[test]
    fn a_repeated_site_is_refused()
    {
        let mut table = SealTable::new();
        let site = SealSite::new("Counter", "t");
        let _first = table.mint(site.clone()).unwrap();
        assert_eq!(
            Err(SealError::SiteAlreadyMinted { site: site.clone() }),
            table.mint(site),
            "one component may be sealed once"
        );
        assert_eq!(
            1,
            table.sites().len(),
            "the refused mint left the table unchanged"
        );
    }

    /// Two declarations sealing the same component name mint distinct atoms —
    /// sealing is generative and declaration-granular.
    #[test]
    fn two_declarations_sealing_one_component_name_stay_distinct()
    {
        let mut table = SealTable::new();
        let first = table.mint(SealSite::new("Counter", "t")).unwrap();
        let second = table.mint(SealSite::new("Gauge", "t")).unwrap();
        assert_ne!(
            first, second,
            "structurally identical sealings in two declarations do not interchange"
        );
    }

    /// **The freshness property.** Re-elaborating one program meets the same
    /// sites in the same order, so its record verifies.
    #[test]
    fn re_minting_the_same_sites_verifies()
    {
        let sites = counter_sites();
        let mut first_run = SealTable::new();
        for site in sites.clone() {
            let _id = first_run.mint(site).unwrap();
        }
        let mut second_run = SealTable::new();
        for site in sites {
            let _id = second_run.mint(site).unwrap();
        }
        let recorded: Vec<SealSite> = first_run
            .sites()
            .iter()
            .map(|site| site.as_ref().clone())
            .collect();
        assert_eq!(
            Ok(()),
            second_run.verify_reminting(&recorded),
            "re-minting one program reproduces its recorded atoms"
        );
    }

    /// A record whose order differs is refused: the serial *is* the position,
    /// so the same sites in another order name different atoms.
    #[test]
    fn a_reordered_record_is_refused()
    {
        let [first, second] = counter_sites();
        let mut table = SealTable::new();
        let _one = table.mint(first.clone()).unwrap();
        let _two = table.mint(second.clone()).unwrap();
        assert_eq!(
            Err(SealError::RemintingDiverged {
                serial: TypeSerial::from(0_u64),
            }),
            table.verify_reminting(&[second, first]),
            "a reordered record names different atoms and is refused"
        );
    }

    /// A record missing an atom is refused, so no minted atom goes unrecorded.
    #[test]
    fn a_truncated_record_is_refused()
    {
        let [first, second] = counter_sites();
        let mut table = SealTable::new();
        let _one = table.mint(first.clone()).unwrap();
        let _two = table.mint(second).unwrap();
        assert_eq!(
            Err(SealError::RemintingCountDiverged {
                recorded: 1,
                minted: 2,
            }),
            table.verify_reminting(&[first]),
            "a record short of the minting is refused"
        );
    }
}
