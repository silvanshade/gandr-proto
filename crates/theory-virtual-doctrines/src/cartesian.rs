//! Checked runtime witnesses for the VDC cartesian structure.

use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::boundary::CartesianDiagonalPreservation;
use crate::boundary::CartesianProjectionPreservation;
use crate::boundary::CartesianStructurePreservation;
use crate::vdc::SigMorphism;
use crate::vdc::SignatureRef;

/// A componentwise action on a finite product of VDC signatures.
///
/// The action is intentionally first-order: one tight action is recorded per
/// product factor. This is the runtime shadow of the W-action used by the
/// cartesian law; no second law interface is introduced.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WCartesianAction
{
    /// The source product acted on.
    source: SignatureRef,
    /// The target product produced by the action.
    target: SignatureRef,
    /// One factor action per product component, in declaration order.
    components: Box<[SigMorphism]>,
}

impl WCartesianAction
{
    /// Construct a W-action over the supplied product factors.
    ///
    /// # Contract
    /// - requires: `source` and `target` are product signatures; `components`
    ///   is the proposed factor action in declaration order.
    /// - ensures: stores the action without assuming that its factor boundaries
    ///   or cartesian laws are valid; [`Self::checked_witness`] performs those
    ///   checks deterministically.
    /// - provides: a first-order action suitable for adversarial validation.
    /// - fails: never; malformed actions are represented so the checker can
    ///   report which law they violate.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn new(
        source: SignatureRef,
        target: SignatureRef,
        components: Vec<SigMorphism>,
    ) -> Self
    {
        Self {
            source,
            target,
            components: components.into_boxed_slice(),
        }
    }

    /// Check all cartesian obligations and return their runtime witness.
    ///
    /// # Contract
    /// - requires: the action describes a finite product on both sides.
    /// - ensures: succeeds only when factor projections, the diagonal, and the
    ///   complete product structure are all preserved.
    /// - provides: deterministic law statuses on every failure.
    /// - fails: returns [`CartesianLawError`] for a non-product boundary or a
    ///   failed cartesian obligation.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`CartesianLawError::SourceNotProduct`] or
    /// [`CartesianLawError::TargetNotProduct`] for non-product boundaries, and
    /// [`CartesianLawError::Violation`] when any law status is false.
    #[inline]
    pub fn checked_witness(&self) -> Result<CartesianWitness, CartesianLawError>
    {
        let source = product_parts(&self.source).ok_or(CartesianLawError::SourceNotProduct)?;
        let target = product_parts(&self.target).ok_or(CartesianLawError::TargetNotProduct)?;
        let same_width = source.len() == target.len() && source.len() == self.components.len();
        let projections = same_width
            && self
                .components
                .iter()
                .zip(source.iter().zip(target.iter()))
                .all(|(component, (source_factor, target_factor))| {
                    component.src == *source_factor && component.tgt == *target_factor
                });
        let diagonal = same_width
            && self.components.windows(2).all(|pair| {
                pair.first().is_some_and(|first| {
                    pair.get(1)
                        .is_some_and(|second| first.src == second.src && first.tgt == second.tgt)
                })
            });
        let structure = same_width && projections && diagonal;
        let witness = CartesianWitness {
            projections: CartesianProjectionPreservation::from(projections),
            diagonal: CartesianDiagonalPreservation::from(diagonal),
            structure: CartesianStructurePreservation::from(structure),
        };
        if !structure {
            return Err(CartesianLawError::Violation(witness));
        }
        Ok(witness)
    }
}

/// The checked status of the three LAW-VDC-CARTESIAN obligations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CartesianWitness
{
    /// Preservation of every product projection.
    pub projections: CartesianProjectionPreservation,
    /// Preservation of the product diagonal.
    pub diagonal: CartesianDiagonalPreservation,
    /// Preservation of the complete product structure.
    pub structure: CartesianStructurePreservation,
}

/// Failure reported by the checked W-cartesian action.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CartesianLawError
{
    /// The source boundary is not a product signature.
    SourceNotProduct,
    /// The target boundary is not a product signature.
    TargetNotProduct,
    /// At least one cartesian status is false.
    Violation(CartesianWitness),
}

/// Return product factors without exposing an unchecked slice operation.
#[inline]
fn product_parts(signature: &SignatureRef) -> Option<&[SignatureRef]>
{
    match signature {
        | &SignatureRef::Product(ref parts) => Some(parts),
        | &SignatureRef::Single(_) => None,
    }
}
