//! The **U3.0 certificate realization** — `CodeIso`, a paired value translator
//! between two monomorphic descriptions, with the invertible-mode groupoid
//! operations and the replay evidence discipline.
//!
//! # What is real, and what is a stand-in
//!
//! Every translator here maps real [`gandr_theory_levitation::DescValue`]s,
//! every round-trip is judged by the landed
//! [`gandr_theory_levitation::generic_eq`], and every description is
//! a real [`gandr_theory_levitation::SignDesc`]. There is **no new production
//! machinery (design note §4.3 U3.0b: "rendered with zero new machinery"): a
//! [`CodeIso`] is a thin certificate wrapper over the stage-0 generic programs,
//! exactly as the `vdc_dictionary` suite's `Cell` is a thin wrapper over the
//! landed cell structures. The landed `gandr-theory-virtual-doctrines`
//! `ProtypeIso` (Nasu §3.2.3 — "codes for the two proterms mutually inverse to
//! each other"; cited in full at this suite's root) is this shape at the
//! judgment layer; the monomorphic-code round-trip discipline this harness pins
//! carries over to it.
//!
//! The reference to that type is deliberately plain rather than an intra-doc
//! link: `gandr-theory-virtual-doctrines` depends on this crate, so the edge a
//! link would need is a cycle the layering rule refuses.
//!
//! # The evidence discipline (ADR-69 D1)
//!
//! A [`CodeIso`] is **data (inspectable), not a proof**. Its evidence is the
//! *replay* of its round trips against `generic_eq` over a sample of values:
//! [`CodeIso::round_trips`] checks `back ∘ fwd ≡ id` and `fwd ∘ back ≡ id` up
//! to [`gandr_theory_levitation::generic_eq`], and two certificates count as
//! the *same* transformation exactly when they are [`replay_equivalent`]
//! (ADR-69 D1: "identity is replay-equivalence"). Neither is a decidable
//! equality on codes — that distinction is the whole point of the U3.0c guard.
//!
//! # Composition mode (ADR-69 D3)
//!
//! The groupoid operations compose in the **invertible mode**
//! ([`CodeIso::compose_invertible`]): unconditional — there is *no* acyclicity
//! gate (that gate governs `compose_directed`, the oriented band, which does
//! not arise here because both members carry an inverse by construction). The
//! only way composition declines is a boundary-object mismatch, which is a
//! domain error, not a composability refusal.

use alloc::rc::Rc;
use core::fmt;

use gandr_theory_levitation::DescValue;
use gandr_theory_levitation::MonomorphicStatus;
use gandr_theory_levitation::Name;
use gandr_theory_levitation::ReplayEquivalence;
use gandr_theory_levitation::RoundTripSampleCount;
use gandr_theory_levitation::RoundTripStatus;
use gandr_theory_levitation::SignDesc;
use gandr_theory_levitation::generic_eq;

/// A **description-driven value translator**: one direction of a [`CodeIso`],
/// mapping a [`DescValue`] of the source description to a [`DescValue`] of the
/// target description.
///
/// Reference-counted so the groupoid operations ([`CodeIso::inverse`],
/// [`CodeIso::compose_invertible`]) can share and re-wrap the underlying maps
/// without re-deriving them. At U3.0 the translator is an opaque closure: the
/// **code-edit generator vocabulary** that would make a translator itself
/// inspectable data is a design item deferred to the ornament conversation
/// (design note §4.3, "do NOT invent it ad hoc for ua-base") and is
/// deliberately absent here.
pub type Translate = Rc<dyn Fn(&DescValue) -> DescValue>;

/// A **`CodeIso` certificate**: a paired pair of description-driven value
/// translators (`forward` / `backward`) between two monomorphic
/// [`SignDesc`]s, whose evidence is the replay of its round trips
/// (design note §4.3 U3.0a; ADR-69).
///
/// The certificate is *data*: it carries a provenance `label`, the two boundary
/// descriptions, and the two translators. It is **not** a proof — its validity
/// is established by [`CodeIso::round_trips`] replaying `back ∘ fwd` and
/// `fwd ∘ back` against [`generic_eq`] over generated values.
#[derive(Clone)]
pub struct CodeIso
{
    /// A provenance label for inspection (`negation`, `Boolean ⨟ BoolSum`, …).
    label: Name,
    /// The monomorphic source description the forward translator reads.
    source: SignDesc,
    /// The monomorphic target description the forward translator writes.
    target: SignDesc,
    /// The forward translator `source → target`.
    forward: Translate,
    /// The backward translator `target → source`.
    backward: Translate,
}

impl fmt::Debug for CodeIso
{
    /// Render the inspectable skeleton — label and boundary description names —
    /// eliding the opaque translator closures.
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        f.debug_struct("CodeIso")
            .field("label", &self.label)
            .field("source", &self.source.id.name)
            .field("target", &self.target.id.name)
            .finish_non_exhaustive()
    }
}

impl CodeIso
{
    /// A certificate from its label, boundary descriptions, and the two
    /// translators.
    pub fn new<L>(
        label: L,
        source: SignDesc,
        target: SignDesc,
        forward: Translate,
        backward: Translate,
    ) -> Self
    where
        L: Into<Name>,
    {
        Self {
            label: label.into(),
            source,
            target,
            forward,
            backward,
        }
    }

    /// The **identity `CodeIso`** on `desc` — the groupoid unit; both
    /// translators are the identity on values.
    pub fn identity<L>(
        label: L,
        desc: SignDesc,
    ) -> Self
    where
        L: Into<Name>,
    {
        let forward: Translate = Rc::new(DescValue::clone);
        let backward: Translate = Rc::new(DescValue::clone);
        Self::new(label, desc.clone(), desc, forward, backward)
    }

    /// The certificate's provenance label.
    pub fn label(&self) -> &Name
    {
        &self.label
    }

    /// The monomorphic source description.
    pub fn source(&self) -> &SignDesc
    {
        &self.source
    }

    /// The monomorphic target description.
    pub fn target(&self) -> &SignDesc
    {
        &self.target
    }

    /// Whether both boundary descriptions are monomorphic (parameter-free) —
    /// the U3.0 scope constraint (design note §4.3: "monomorphic codes
    /// only").
    pub fn is_monomorphic(&self) -> MonomorphicStatus
    {
        (self.source.params.is_empty() && self.target.params.is_empty()).into()
    }

    /// Apply the forward translator to a source value.
    pub fn forward_value(
        &self,
        value: &DescValue,
    ) -> DescValue
    {
        (self.forward)(value)
    }

    /// Apply the backward translator to a target value.
    pub fn backward_value(
        &self,
        value: &DescValue,
    ) -> DescValue
    {
        (self.backward)(value)
    }

    /// The **inverse `CodeIso`** — the groupoid inverse; swaps the two
    /// translators and the two boundary descriptions.
    pub fn inverse(&self) -> Self
    {
        Self {
            label: format!("{}⁻¹", self.label).into_boxed_str().into(),
            source: self.target.clone(),
            target: self.source.clone(),
            forward: Rc::clone(&self.backward),
            backward: Rc::clone(&self.forward),
        }
    }

    /// **Invertible-mode composition** `self ⨟ next` (ADR-69 D3
    /// `compose_invertible`): the groupoid composite `source → next.target`.
    ///
    /// # Contract
    /// - requires: `self` and `next` share a boundary object — `self.target`
    ///   equals `next.source` as descriptions.
    /// - ensures: [`Some`] carrying the composite whose forward is
    ///   `next.forward ∘ self.forward` and whose backward is `self.backward ∘
    ///   next.backward`; [`None`] exactly on a boundary-object mismatch. There
    ///   is **no acyclicity gate** — invertible composition is unconditional
    ///   given a shared boundary (ADR-69 D3: the gate governs
    ///   `compose_directed`, not this mode).
    /// - fails: never; a boundary mismatch is a declined [`None`], not a panic.
    pub fn compose_invertible(
        &self,
        next: &Self,
    ) -> Option<Self>
    {
        if self.target != next.source {
            return None;
        }
        let self_forward = Rc::clone(&self.forward);
        let next_forward = Rc::clone(&next.forward);
        let self_backward = Rc::clone(&self.backward);
        let next_backward = Rc::clone(&next.backward);
        let forward: Translate = Rc::new(move |value| next_forward(&self_forward(value)));
        let backward: Translate = Rc::new(move |value| self_backward(&next_backward(value)));
        Some(Self {
            label: format!("{} ⨟ {}", self.label, next.label)
                .into_boxed_str()
                .into(),
            source: self.source.clone(),
            target: next.target.clone(),
            forward,
            backward,
        })
    }

    /// **Replay the round trips** of this certificate against `generic_eq` over
    /// the given value samples — the evidence discipline (ADR-69 D1).
    ///
    /// # Contract
    /// - requires: `source_samples` are values of [`Self::source`] and
    ///   `target_samples` are values of [`Self::target`].
    /// - ensures: returns a [`RoundTripReport`] recording, for every source
    ///   sample `v`, whether `generic_eq(source, backward(forward(v)), v)`, and
    ///   for every target sample `w`, whether `generic_eq(target,
    ///   forward(backward(w)), w)`; a certificate is valid exactly when the
    ///   report [`RoundTripReport::holds`].
    /// - fails: never; `generic_eq` is total, so a mis-built value yields a
    ///   recorded failure, not a panic.
    pub fn round_trips(
        &self,
        source_samples: &[DescValue],
        target_samples: &[DescValue],
    ) -> RoundTripReport
    {
        let mut failures = Vec::new();
        for value in source_samples {
            let round = self.backward_value(&self.forward_value(value));
            if !bool::from(generic_eq(&self.source, &round, value)) {
                failures.push(RoundTripFailure {
                    direction: Direction::BackAfterForward,
                    original: value.clone(),
                    round_tripped: round,
                });
            }
        }
        for value in target_samples {
            let round = self.forward_value(&self.backward_value(value));
            if !bool::from(generic_eq(&self.target, &round, value)) {
                failures.push(RoundTripFailure {
                    direction: Direction::ForwardAfterBackward,
                    original: value.clone(),
                    round_tripped: round,
                });
            }
        }
        RoundTripReport {
            forward_checked: source_samples.len().into(),
            backward_checked: target_samples.len().into(),
            failures,
        }
    }
}

/// Which round-trip direction a [`RoundTripFailure`] records.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Direction
{
    /// `back ∘ fwd` on a source value failed to recover it.
    BackAfterForward,
    /// `fwd ∘ back` on a target value failed to recover it.
    ForwardAfterBackward,
}

/// One recorded round-trip failure — the original value, what it round-tripped
/// to, and in which direction (inspectable evidence, not a bare boolean).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoundTripFailure
{
    /// The direction whose round trip failed.
    pub direction: Direction,
    /// The value fed into the round trip.
    pub original: DescValue,
    /// The value the round trip produced (unequal to `original` under
    /// `generic_eq`).
    pub round_tripped: DescValue,
}

/// The result of replaying a certificate's round trips: how many samples were
/// checked in each direction, and every failure witnessed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoundTripReport
{
    /// The number of source samples checked through `back ∘ fwd`.
    pub forward_checked: RoundTripSampleCount,
    /// The number of target samples checked through `fwd ∘ back`.
    pub backward_checked: RoundTripSampleCount,
    /// Every round-trip failure witnessed (empty means the certificate holds).
    pub failures: Vec<RoundTripFailure>,
}

impl RoundTripReport
{
    /// Whether every replayed round trip held under `generic_eq`.
    pub fn holds(&self) -> RoundTripStatus
    {
        self.failures.is_empty().into()
    }
}

/// Whether two certificates over the **same boundary** are the *same*
/// transformation — ADR-69 D1's identity-is-replay-equivalence, decided over
/// the given samples.
///
/// # Contract
/// - requires: `left` and `right` share a boundary — equal source and equal
///   target descriptions; `source_samples` / `target_samples` are values of
///   those descriptions.
/// - ensures: returns `true` exactly when the two certificates' forward images
///   agree under `generic_eq(target, …)` on every source sample **and** their
///   backward images agree under `generic_eq(source, …)` on every target
///   sample.
/// - fails: panics only on a boundary mismatch (a test-author error, not a data
///   verdict).
pub fn replay_equivalent(
    left: &CodeIso,
    right: &CodeIso,
    source_samples: &[DescValue],
    target_samples: &[DescValue],
) -> ReplayEquivalence
{
    assert!(
        left.source == right.source && left.target == right.target,
        "replay-equivalence is only defined between certificates over one boundary"
    );
    replay_disagreement(left, right, source_samples, target_samples)
        .is_none()
        .into()
}

/// The first point at which two same-boundary certificates *disagree* under
/// replay — the inspectable witness behind [`replay_equivalent`] and the
/// U3.0c guard.
///
/// Returns `Some((input, left_image, right_image))` at the first sample where
/// the two forward images (or backward images) differ under `generic_eq`, or
/// [`None`] when the two are replay-equivalent over the samples.
///
/// # Contract
/// - requires: as [`replay_equivalent`].
/// - ensures: [`None`] iff [`replay_equivalent`] would return `true` over the
///   same samples; otherwise the earliest disagreeing `(input, left_image,
///   right_image)`, source samples before target samples.
/// - fails: panics only on a boundary mismatch.
pub fn replay_disagreement(
    left: &CodeIso,
    right: &CodeIso,
    source_samples: &[DescValue],
    target_samples: &[DescValue],
) -> Option<(DescValue, DescValue, DescValue)>
{
    assert!(
        left.source == right.source && left.target == right.target,
        "replay-disagreement is only defined between certificates over one boundary"
    );
    for value in source_samples {
        let left_image = left.forward_value(value);
        let right_image = right.forward_value(value);
        if !bool::from(generic_eq(&left.target, &left_image, &right_image)) {
            return Some((value.clone(), left_image, right_image));
        }
    }
    for value in target_samples {
        let left_image = left.backward_value(value);
        let right_image = right.backward_value(value);
        if !bool::from(generic_eq(&left.source, &left_image, &right_image)) {
            return Some((value.clone(), left_image, right_image));
        }
    }
    None
}
