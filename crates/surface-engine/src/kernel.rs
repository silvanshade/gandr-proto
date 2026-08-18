//! The session's **kernel admission ledger** — the engine's shipping consumer
//! of the elaborator-side kernel bridge.
//!
//! [`gandr_core_checker::kernel_bridge`] lowers checked-core CBPV forms into
//! the certified kernel's closed S1 vocabulary, and [`Environment::add_decl`]
//! is the choke point that re-derives every typing obligation before a
//! declaration enters the kernel. This module is what stands a *running*
//! pipeline on that pair: [`KernelAdmissions`] accumulates one kernel
//! environment across a session's definitions, so the surface language's own
//! `def` items are the bridge's production input rather than a fixture corpus
//! replayed by a test.
//!
//! # The dependency direction
//!
//! This crate depends on [`gandr_kernel_core`]; the reverse is forbidden. The
//! bridge is untrusted elaborator-side code and so is everything here: a
//! verdict recorded below reports what the kernel decided and carries no weight
//! of its own. An [`Admitted`](KernelVerdict::Admitted) verdict exists only
//! because a [`CheckedId`] came back from the choke point.
//!
//! # A non-admission is not a verdict on the program
//!
//! Only [`Admitted`](KernelVerdict::Admitted) says anything about the program.
//! The other three verdicts say what the kernel can take *today*: S1 is a
//! standing subset that grows, and a form outside it is a form the certified
//! vocabulary has not reached yet, never a form the checked language got wrong.
//! A session's typing, binding, and evaluation are complete without the kernel
//! and are unchanged by it.
//!
//! The asymmetry is the point and it is worth stating where a reader meets it:
//! failing to admit something admissible costs coverage, and admitting
//! something inadmissible would cost soundness — so every path here is written
//! to fail toward the first. Nothing in this module can produce an
//! [`Admitted`](KernelVerdict::Admitted) verdict without a checked id.
//!
//! # Cross-declaration references, which is what a session adds
//!
//! A definition body naming an earlier definition lowers to a kernel constant
//! through the bridge's [`BridgeContext`]. A session is the first consumer that
//! populates that map: it admits definitions in source order and binds each
//! admitted name to the admission index it came back with, so
//! `def a = 3; def b = a;` reaches the kernel as two declarations, the second
//! referring to the first. A single-item harness cannot exercise that path at
//! all, because its naming environment is always empty.
//!
//! # A rejected lowering leaves nothing behind
//!
//! [`Environment::stage`] mints content directly into the environment's
//! arena, and a staged builder that is dropped without a finisher truncates
//! the arena back to its content-start watermark (the builder's rollback —
//! [`DeclarationBuilder::discard`] is its explicit form). A bridge rejection
//! therefore abandons the partial lowering in place: the definition is
//! lowered exactly once, and the environment the artifact writer later walks
//! holds only admitted content.
//!
//! # Levels
//!
//! A bridged declaration is level-monomorphic, exactly as the bridge documents:
//! checked core carries no universe levels, so the kernel's levelled universe
//! and its explicit lifts stay kernel-native and are never reached from here.
//!
//! [`CheckedId`]: gandr_kernel_core::CheckedId
//! [`DeclarationBuilder::discard`]: gandr_kernel_core::DeclarationBuilder::discard

use alloc::string::String;
use alloc::string::ToString as _;

use gandr_core_checker::kernel_bridge::BridgeContext;
use gandr_core_checker::kernel_bridge::BridgeRejection;
use gandr_core_checker::kernel_bridge::lower_computation_definition;
use gandr_core_checker::kernel_bridge::lower_value_definition;
use gandr_core_checker::term::syntax::Term;
use gandr_core_checker::term::types::Ty;
use gandr_kernel_core::ConstantIndex;
use gandr_kernel_core::Environment;
use gandr_kernel_core::KernelError;
use gandr_kernel_core::LevelSignature;
use gandr_kernel_core::TermArena;
use gandr_kernel_core::ValueId;
use gandr_kernel_core::ValueTypeId;

use crate::boundary::DefinitionName;

/// The number of declarations a ledger has admitted.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AdmittedCount(usize);

impl From<usize> for AdmittedCount
{
    #[inline]
    fn from(value: usize) -> Self
    {
        Self(value)
    }
}

impl From<AdmittedCount> for usize
{
    #[inline]
    fn from(value: AdmittedCount) -> Self
    {
        value.0
    }
}

/// Why a session item never reached the choke point at all.
///
/// These are the items the kernel never saw, kept apart from the items it saw
/// and turned away ([`KernelVerdict::OutsideS1`] and
/// [`KernelVerdict::Refused`]) so a consumer measuring kernel coverage is not
/// reading one undifferentiated failure bucket.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum WithheldReason
{
    /// The item is an expression rather than a `def`, so it declares nothing.
    Expression,
    /// The item's typing failed, so no declared type exists to admit against.
    Untyped,
    /// The item carries a hole, so typing was declined before a type existed.
    Holey,
    /// The item is a definition whose lowered term and reported type have
    /// opposite polarity, which is how a session reports a computation
    /// definition of returner type: it names the returner's *payload* as the
    /// value the definition binds, and the returner's effect row does not
    /// survive that projection. Reconstructing the row would be a guess, and a
    /// guessed-empty row would present an effectful computation to the kernel
    /// as a pure one, so the item is withheld instead.
    IndeterminateDeclaredType,
}

/// A kernel admission failure, rendered for a consumer that does not link the
/// kernel.
///
/// [`KernelError`] carries kernel term ids that mean nothing outside the
/// environment that minted them and that go stale as soon as that environment
/// moves on. The rendering is kept and the ids are not, so a verdict a
/// front-end holds stays meaningful for as long as it holds it.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct KernelRefusal(String);

impl AsRef<str> for KernelRefusal
{
    #[inline]
    fn as_ref(&self) -> &str
    {
        self.0.as_str()
    }
}

impl From<&KernelError> for KernelRefusal
{
    #[inline]
    fn from(error: &KernelError) -> Self
    {
        Self(error.to_string())
    }
}

impl core::fmt::Display for KernelRefusal
{
    #[inline]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        f.write_str(self.0.as_str())
    }
}

/// What became of one session item at the kernel boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum KernelVerdict
{
    /// The definition lowered into S1 and crossed the choke point, entering the
    /// environment at this admission index.
    Admitted
    {
        /// The declaration's position in the kernel environment.
        position: ConstantIndex,
    },
    /// The definition has no image in the closed S1 vocabulary.
    OutsideS1
    {
        /// The exact form the bridge refused.
        rejection: BridgeRejection,
    },
    /// The definition lowered, and the kernel's own re-derivation turned it
    /// away — the choke point granting the bridge no credence, visibly.
    Refused
    {
        /// Why the declaration did not admit.
        error: KernelRefusal,
    },
    /// The item never reached the choke point.
    Withheld
    {
        /// Which kind of item it was.
        reason: WithheldReason,
    },
}

/// One session definition offered to the kernel.
#[derive(Clone, Copy, Debug)]
pub struct DefinitionOffer<'item>
{
    /// The defined name, which later definitions resolve through.
    pub name: DefinitionName<'item>,
    /// The item's lowered core term — the definition's body.
    pub term: &'item Term,
    /// The type the session's typing reported for the item.
    pub ty: &'item Ty,
}

/// The kernel environment a session accumulates, with the naming environment
/// that lets a later definition refer to an earlier one.
#[derive(Clone, Debug, Default)]
pub struct KernelAdmissions
{
    /// The kernel environment, appended to in admission order.
    environment: Environment,
    /// Admitted definition name → its admission index.
    context: BridgeContext,
    /// How many declarations have been admitted.
    admitted: AdmittedCount,
}

impl KernelAdmissions
{
    /// An empty ledger: no environment content and no resolvable names.
    ///
    /// # Contract
    /// - ensures: the returned ledger has admitted nothing and resolves no
    ///   name.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn new() -> Self
    {
        Self::default()
    }

    /// The accumulated kernel environment.
    ///
    /// # Contract
    /// - ensures: returns the environment holding exactly the declarations
    ///   whose offers returned [`KernelVerdict::Admitted`], in admission order.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn environment(&self) -> &Environment
    {
        &self.environment
    }

    /// How many declarations have been admitted.
    ///
    /// # Contract
    /// - ensures: returns the number of [`KernelVerdict::Admitted`] verdicts
    ///   this ledger has returned, which is also the environment's declaration
    ///   count.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn admitted(&self) -> AdmittedCount
    {
        self.admitted
    }

    /// Offer one definition to the kernel, admitting it when it lowers.
    ///
    /// # Contract
    /// - requires: `offer.ty` is the type the session's typing reported for
    ///   `offer.term`, and offers arrive in the session's source order, so a
    ///   body naming an earlier definition resolves against an already-admitted
    ///   one.
    /// - ensures: [`KernelVerdict::Admitted`] exactly when the whole definition
    ///   lowers into S1 and the choke point returns a checked id, and the name
    ///   then resolves for every later offer; on every other verdict the
    ///   environment is left exactly as it was, holding no content from a
    ///   definition that did not admit.
    /// - provides: the surface engine's crossing from checked core into the
    ///   certified kernel.
    /// - fails: an out-of-S1 form surfaces as [`KernelVerdict::OutsideS1`], a
    ///   kernel refusal as [`KernelVerdict::Refused`], and a polarity the
    ///   reported type does not determine as
    ///   [`WithheldReason::IndeterminateDeclaredType`].
    /// - panics: none.
    /// - intension: the definition lowers exactly once, into the staged
    ///   environment arena; a rejection abandons the staged builder, whose
    ///   rollback truncates the partial mint (see the module doc), so the
    ///   rejection the bridge returned is the verdict this function returns.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — the verdicts this function returns are separated by
    ///   definitions a session can actually submit: an integer literal that
    ///   admits, a machine-numeric literal with no S1 image, and a computation
    ///   definition of returner type, whose reported type does not determine
    ///   its polarity. The cross-declaration residue is a second definition
    ///   whose body is the first definition's name, which admits only when the
    ///   naming environment was populated; the environment-unchanged residue is
    ///   a rejected definition followed by an admitted one taking position
    ///   zero.
    /// - hypothesis: the bridge's unresolved-name rejection is reachable only
    ///   through the gap between the two name spaces, because a name nothing
    ///   binds fails typing and never arrives here: a definition the session
    ///   binds but the kernel withheld leaves the name resolvable to the one
    ///   and free to the other. A name no prior declaration binds at all is a
    ///   typing failure, and the session withholds it as
    ///   [`WithheldReason::Untyped`] without calling this function.
    /// - witness: `kernel::a_value_definition_is_admitted`
    /// - witness: `kernel::a_later_definition_refers_to_an_earlier_one`
    /// - witness: `kernel::an_out_of_s1_definition_is_reported_outside_s1`
    /// - witness: `kernel::a_body_naming_a_withheld_definition_has_no_s1_image`
    /// - witness: `kernel::an_untyped_item_is_withheld`
    /// - witness: `kernel::a_returner_definition_is_withheld`
    /// - witness: `kernel::a_rejected_definition_leaves_the_environment_unchanged`
    /// - witness: `kernel::a_partially_lowered_rejection_leaves_no_arena_content`
    #[inline]
    pub fn offer(
        &mut self,
        offer: DefinitionOffer<'_>,
    ) -> KernelVerdict
    {
        let Self {
            ref mut environment,
            ref mut context,
            ref mut admitted,
        } = *self;
        let mut builder = environment.stage();
        let staged = lower_definition(context, builder.arena(), offer);
        let ids = match staged {
            | Some(Ok(ids)) => ids,
            // A rejection abandons the staged builder here, and its rollback
            // truncates whatever the partial lowering minted — the environment
            // is unchanged, and the rejection is the verdict returned.
            | Some(Err(rejection)) => return KernelVerdict::OutsideS1 { rejection },
            | None => {
                return KernelVerdict::Withheld {
                    reason: WithheldReason::IndeterminateDeclaredType,
                };
            },
        };
        let (declared, body) = ids;
        let declaration = builder.def(LevelSignature::monomorphic(), declared, body);
        match environment.add_decl(declaration) {
            | Ok(checked) => {
                let position = checked.position();
                *context = core::mem::take(context).with_constant(offer.name.0, position);
                *admitted = AdmittedCount(admitted.0.saturating_add(1));
                KernelVerdict::Admitted { position }
            },
            | Err(ref error) => KernelVerdict::Refused {
                error: KernelRefusal::from(error),
            },
        }
    }
}

/// Lower one definition's declared type and body into `arena`, dispatching on
/// the polarity the reported type determines.
///
/// # Contract
/// - requires: `arena` is the environment's staged arena; a rejection leaves
///   whatever was minted to the staged builder's rollback.
/// - ensures: `Some(Ok(roots))` with the S1 declared-type and body roots when
///   the whole definition lowers.
/// - provides: the one place the value/computation dispatch is written.
/// - fails: `Some(Err(rejection))` for an out-of-S1 form, and [`None`] for the
///   polarity the reported type does not determine — a computation term
///   reported at a value type, which is
///   [`WithheldReason::IndeterminateDeclaredType`].
/// - panics: none.
fn lower_definition(
    context: &BridgeContext,
    arena: &mut TermArena,
    offer: DefinitionOffer<'_>,
) -> Option<Result<(ValueTypeId, ValueId), BridgeRejection>>
{
    match (offer.term, offer.ty) {
        | (&Term::Value(ref value), &Ty::Value(ref declared)) => {
            Some(lower_value_definition(context, arena, value, declared))
        },
        | (&Term::Comp(ref comp), &Ty::Comp(ref declared)) => {
            Some(lower_computation_definition(context, arena, comp, declared))
        },
        | _ => None,
    }
}
