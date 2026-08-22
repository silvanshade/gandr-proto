//! **Normalization by evaluation**: gandr's conversion engine.
//!
//! This crate decides definitional equality on the core call-by-push-value
//! terms `gandr-core-term` defines. Terms are evaluated into a glued semantic
//! domain, compared there, and read back into syntax when a term is wanted;
//! the six-step definitional-equality pipeline decides conversion, and every
//! caller that once used a structural equality goes through it.
//!
//! It names no judgement and no realization. The bidirectional checker's
//! subsumption relation calls in, and the solver's certificates are re-checked
//! by asking this engine — which is what pins one equational theory across the
//! two.
//!
//! # The engine in one screen
//!
//! | module   | what it owns                                                         |
//! | -------- | -------------------------------------------------------------------- |
//! | [`sem`]  | the glued value domain, its per-run arena, and the guard word        |
//! | [`defs`] | the per-scope definitional environment, heights, and transparency    |
//! | [`eval`] | evaluation and the three force modes                                 |
//! | [`quote`]| readback and its three options                                       |
//! | [`conv`] | the six-step definitional-equality pipeline                          |
//! | [`intern`] | the per-face syntax interner                                       |
//!
//! # The definitional-equality pipeline
//!
//! Six steps, each falling through to the next only on a non-answer:
//!
//! 1. **identity equality** — one id in one arena is one value, so an id match
//!    answers *convertible* outright. It is sound by immutability and it takes
//!    no table into the trusted base, because it decides only reflexive pairs
//!    and defers every distinct-id pair to the walk.
//! 2. **cached-word guards** — the intrusive [`sem::Guard`] answers *distinct*
//!    in constant time for a rigid, hole-free pair whose hashes differ.
//! 3. **iterative structural comparison** over a heap worklist, with a
//!    head-mismatch fast fail before descending into children.
//! 4. **lazy unfolding with heights** — same head with regular hints tries the
//!    arguments first; otherwise the **taller** side unfolds, and a one-sided
//!    unfolding rule unfolds that side.
//! 5. **smart unfolding gated on case progress** — a definition unfolds only if
//!    doing so makes case-tree progress, which is decidable directly here
//!    because case is first-class, so no companion definition is needed.
//! 6. **three-state speculation** — rigid, flex, full: two same-head glued
//!    neutrals compare spines first with no commitments and back off onto the
//!    **already-forced** unfolded face, so nothing is evaluated twice.
//!
//! # What this engine will not do
//!
//! The module layer exports four anti-commitments and they are honoured here
//! as prohibitions rather than as unimplemented features:
//!
//! * **it never compares signatures.** A record type is compared by its
//!   canonically-ordered label-to-type map, field by field. That is structural
//!   equality on a canonical representation, not a width or permutation
//!   equation, and no width or permutation rule exists anywhere in [`conv`].
//!   Growing one would foreclose the telescope future from inside the
//!   conversion engine.
//! * **it never memoizes across functor instantiations.** There is no
//!   content-keyed value memoization at all, so the question cannot arise.
//! * **it never makes a package eliminable by anything but its own elimination
//!   form** — packages have no representation here, so nothing eliminates one.
//! * **it takes no interning table into the trusted base.** Interning is syntax
//!   only, per face, and it is a deduplicator.
//!
//! # The quarantine
//!
//! Conversion never runs an effect, a handler, or a control operator. Those
//! formers evaluate to **neutrals** with their operands evaluated, so the
//! equality this engine offers on them is congruence and nothing stronger. The
//! quarantine used to hold vacuously, because nothing was evaluated at all; it
//! now holds by construction, because the formers that would break it have no
//! reduction rule in [`eval`].
//!
//! # Module forms enter as neutrals
//!
//! The module layer's six holes cost nothing before their rungs land, because a
//! neutral head is already the pipeline's ordinary case. The one hole this rung
//! plugs is **structure projection**: a record module's field selection reduces
//! when its head is a structure and stays neutral when it is not, weak-head and
//! spine-local, never touching the sibling components.

extern crate alloc;

pub mod conv;
/// The definitional environment, re-exported.
///
/// It **lives in [`gandr_core_term`]**, because it is core vocabulary rather
/// than normalizer-private state: its entire dependency surface is that
/// crate's own `FlatArena`, `ValueNodeId` and `DefinitionHeightLevel`. This
/// crate was the natural home while the normalizer was its only consumer, and
/// stopped being one when the typing context needed to carry it.
///
/// The re-export is what keeps that move invisible to every existing
/// consumer.
pub use gandr_core_term::defs;
pub mod eval;
pub mod intern;
pub mod quote;
pub mod sem;
mod spine;

use alloc::rc::Rc;

use gandr_core_term::boundary::ConversionFuel;
use gandr_core_term::boundary::ValueEquality;
use gandr_core_term::boundary::VariableLevel;
use gandr_core_term::defs::Definitions;
use gandr_core_term::syntax::FlatArena;
use gandr_core_term::syntax::Value;
use gandr_core_term::syntax::ValueNodeId;

use crate::intern::SyntaxInterner;
use crate::sem::SemArena;
use crate::sem::SemError;
use crate::sem::Watermark;

/// The default fuel a normalizer spends before it stops unfolding.
///
/// It bounds one pathological case and nothing else: a definition whose body
/// mentions itself. Heights make that impossible for an environment built by
/// the elaborator, since a body can only mention definitions that already
/// exist, but the environment is a public surface and a caller may define one
/// anyway. Running out of fuel stops unfolding and answers on the neutral face,
/// which loses completeness on that definition and never soundness.
const DEFAULT_FUEL: u32 = 4096;

/// The normalizer: one arena, one definitional environment, one per-face syntax
/// interner, and the fresh-variable counter readback draws from.
///
/// # Contract
/// - requires: every id handed back to a normalizer was minted by that same
///   normalizer and has not been truncated away.
/// - ensures: every entry point is deterministic — the same term, environment,
///   and definitions give the same answer, byte for byte, on every run and on
///   every host. Fresh variables are de Bruijn levels drawn from a counter, so
///   nothing an answer depends on is an address, a hash-map order, or an
///   allocation order.
/// - provides: the checker's conversion engine, its normal forms, and the
///   readback that turns a semantic value back into syntax.
/// - fails: [`SemError`] on arena exhaustion or an unresolvable id;
///   [`Self::converts`] absorbs those into a **distinct** verdict, which is the
///   fail-closed direction.
/// - panics: none.
#[derive(Clone, Debug)]
pub struct Normalizer
{
    /// The per-run semantic arena.
    arena: SemArena,
    /// The per-scope definitional environment.
    defs: Definitions,
    /// The syntax store: the flat node carrier every handle in the semantic
    /// arena names. It owns the syntax; nothing in the semantic arena does.
    syntax: FlatArena,
    /// The per-face syntax interner, keyed on canonical content.
    interner: SyntaxInterner,
    /// The next de Bruijn level readback and conversion will generate.
    next_level: u32,
    /// The unfolding budget one force may spend.
    fuel: ConversionFuel,
}

impl Default for Normalizer
{
    #[inline]
    fn default() -> Self
    {
        Self::new()
    }
}

impl Normalizer
{
    /// A normalizer with an empty definitional environment.
    ///
    /// The empty environment is exactly the pre-unfolding conversion seed: with
    /// nothing to unfold, the pipeline's last three steps never fire and the
    /// engine decides beta-eta equality on the term language alone.
    #[inline]
    #[must_use]
    pub fn new() -> Self
    {
        Self {
            arena: SemArena::new(),
            defs: Definitions::new(),
            syntax: FlatArena::new(),
            interner: SyntaxInterner::new(),
            next_level: 0,
            fuel: ConversionFuel::from(DEFAULT_FUEL),
        }
    }

    /// A normalizer over an existing definitional environment.
    #[inline]
    #[must_use]
    pub fn with_definitions(defs: Definitions) -> Self
    {
        Self {
            defs,
            ..Self::new()
        }
    }

    /// The definitional environment.
    #[inline]
    #[must_use]
    pub fn definitions(&self) -> &Definitions
    {
        &self.defs
    }

    /// The definitional environment, for scoping.
    ///
    /// Defining goes through [`Self::define`] rather than through this, because
    /// a definition's body is lowered into the syntax store first and the
    /// environment holds the resulting handle.
    #[inline]
    pub fn definitions_mut(&mut self) -> &mut Definitions
    {
        &mut self.defs
    }

    /// Defines `name` as `body`, reducible, lowering the body into the syntax
    /// store on the way.
    ///
    /// # Contract
    /// - ensures: `name` unfolds to `body` in the innermost open scope, at the
    ///   height the definition graph gives it.
    /// - fails: [`SemError::SyntaxStore`] when lowering fails.
    /// - panics: none.
    ///
    /// # Errors
    ///
    /// Returns [`SemError::SyntaxStore`] when lowering fails.
    #[inline]
    pub fn define<'source, N>(
        &mut self,
        name: N,
        body: &Value,
    ) -> Result<(), SemError>
    where
        N: Into<gandr_core_term::boundary::NameRef<'source>>,
    {
        self.define_with(name, body, defs::Transparency::Reducible)
    }

    /// Defines `name` as `body` with an explicit transparency — the reserved
    /// irreducible opt-out.
    ///
    /// # Errors
    ///
    /// Returns [`SemError::SyntaxStore`] when lowering fails.
    #[inline]
    pub fn define_with<'source, N>(
        &mut self,
        name: N,
        body: &Value,
        transparency: defs::Transparency,
    ) -> Result<(), SemError>
    where
        N: Into<gandr_core_term::boundary::NameRef<'source>>,
    {
        let node = self.lower_input(body)?;
        self.defs
            .define_with(&self.syntax, name, node, transparency);
        Ok(())
    }

    /// The per-face syntax interner.
    #[inline]
    #[must_use]
    pub fn interner(&self) -> &SyntaxInterner
    {
        &self.interner
    }

    /// The syntax store every handle in the semantic arena names.
    ///
    /// There is deliberately **no mutable accessor for the interner**. The flat
    /// carrier is `Clone`, so handing one out would let a caller insert any
    /// node under a face of its own choosing and the two faces' representative
    /// sets would stop being disjoint; interning is reached only through
    /// [`Self::lower_input`] and [`Self::intern_readback`], each of which fixes
    /// its own face.
    #[inline]
    #[must_use]
    pub fn syntax(&self) -> &FlatArena
    {
        &self.syntax
    }

    /// The syntax store, for lowering and for readback.
    #[inline]
    pub fn syntax_mut(&mut self) -> &mut FlatArena
    {
        &mut self.syntax
    }

    /// The semantic arena.
    #[inline]
    #[must_use]
    pub fn arena(&self) -> &SemArena
    {
        &self.arena
    }

    /// The semantic arena, for minting.
    #[inline]
    pub fn arena_mut(&mut self) -> &mut SemArena
    {
        &mut self.arena
    }

    /// The unfolding budget one force may spend.
    #[inline]
    #[must_use]
    pub fn fuel(&self) -> ConversionFuel
    {
        self.fuel
    }

    /// Sets the unfolding budget one force may spend.
    #[inline]
    pub fn set_fuel(
        &mut self,
        fuel: ConversionFuel,
    )
    {
        self.fuel = fuel;
    }

    /// The level [`Self::fresh_level`] will draw next, without drawing it.
    ///
    /// A caller that must later tell **its own** opened variables from the
    /// binders a readback introduced reads this as a watermark before quoting:
    /// every level readback mints is at or above it, and every level the caller
    /// opened is below it. That is the discrimination the unifier's scope check
    /// rests on (`gandr_core_unify`).
    #[inline]
    #[must_use]
    pub fn next_level(&self) -> VariableLevel
    {
        VariableLevel::from(self.next_level)
    }

    /// Draws the next fresh de Bruijn level.
    ///
    /// # Contract
    /// - ensures: every call returns a level distinct from every level drawn
    ///   before it **in the current run**, and the sequence a run draws is the
    ///   same on every run — which is where byte-stable readback comes from.
    /// - panics: none.
    #[inline]
    pub fn fresh_level(&mut self) -> VariableLevel
    {
        let level = self.next_level;
        self.next_level = self.next_level.saturating_add(1);
        VariableLevel::from(level)
    }

    /// Opens a run: records the arena population and restarts the fresh-level
    /// counter, returning what [`Self::finish_run`] needs to close it.
    ///
    /// Restarting the counter is what makes readback **byte-stable** rather
    /// than merely alpha-stable: without it, a second normalization on the same
    /// normalizer would name its binders from wherever the counter had reached,
    /// so two normal forms of one term would be alpha-equal and structurally
    /// different. Nesting is safe because a run's level-bearing values are
    /// truncated away before its caller looks at anything again.
    #[inline]
    fn begin_run(&mut self) -> (Watermark, VariableLevel)
    {
        let opened = (self.watermark(), VariableLevel::from(self.next_level));
        self.next_level = 0;
        opened
    }

    /// Closes a run opened by [`Self::begin_run`]: truncates its semantic nodes
    /// and restores the enclosing run's fresh-level counter.
    #[inline]
    fn finish_run(
        &mut self,
        opened: (Watermark, VariableLevel),
    )
    {
        self.truncate_to(opened.0);
        self.next_level = u32::from(opened.1);
    }

    /// The arena population, for a later [`Self::truncate_to`].
    #[inline]
    #[must_use]
    pub fn watermark(&self) -> Watermark
    {
        self.arena.watermark()
    }

    /// Drops every semantic node minted after `mark`.
    ///
    /// This is how a run's values stop existing once its verdict is in: they
    /// allocate past the watermark and are truncated wholesale, so no
    /// unfolding-built value outlives the question it was built to answer.
    #[inline]
    pub fn truncate_to(
        &mut self,
        mark: Watermark,
    )
    {
        self.arena.truncate_to(mark);
    }

    /// Normalizes a source value: evaluate, then read back.
    ///
    /// # Contract
    /// - ensures: the result is the normal form of `term` under this
    ///   normalizer's definitional environment, in canonical binder form and
    ///   interned into the readback face; the run's semantic nodes are
    ///   truncated away before the result is returned, so nothing the
    ///   normalization built survives it.
    /// - provides: the normal form a caller asked for, as ordinary syntax.
    /// - fails: [`SemError`] on arena exhaustion or an unresolvable id.
    /// - panics: none.
    ///
    /// # Errors
    ///
    /// Returns [`SemError`] on arena exhaustion or an unresolvable id.
    ///
    /// # Adequacy
    /// - hypothesis: L2 against conversion as the external oracle — a term and
    ///   its normal form always convert, and normalizing twice is idempotent —
    ///   plus L3 for the truncation, separated by observing the arena
    ///   population before and after.
    /// - witness: `crate::tests::a_term_converts_with_its_own_normal_form`
    /// - witness: `crate::tests::normalization_is_idempotent`
    /// - witness: `crate::tests::normalizing_leaves_the_arena_where_it_found_it`
    #[inline]
    pub fn normalize(
        &mut self,
        term: &Value,
    ) -> Result<Rc<Value>, SemError>
    {
        let node = self.normalize_node(term)?;
        self.reify(node)
    }

    /// Normalizes a source value and returns the **node** its normal form
    /// occupies in the syntax store, interned into the readback face.
    ///
    /// This is the entry a caller uses when it wants to keep working in node
    /// ids; [`Self::normalize`] is this composed with one reification.
    ///
    /// # Errors
    ///
    /// Returns [`SemError`] on arena exhaustion or an unresolvable id.
    #[inline]
    pub fn normalize_node(
        &mut self,
        term: &Value,
    ) -> Result<ValueNodeId, SemError>
    {
        let lowered = self.lower_input(term)?;
        let opened = self.begin_run();
        let evaluated = eval::eval_value(self, SemArena::EMPTY_ENV, lowered)?;
        let quoted = quote::quote_value(self, evaluated, quote::QuoteMode::Canonical)?;
        self.finish_run(opened);
        // Canonical readback output, which is exactly what the readback face
        // accepts; the precondition on `intern_readback` states why.
        Ok(self.intern_readback(quoted))
    }

    /// Interns a node into the **readback** face's table.
    ///
    /// # Contract
    /// - requires: `node` is [`quote::QuoteMode::Canonical`] readback output or
    ///   a fresh allocation — **never** a [`quote::QuoteMode::Retained`] result
    ///   and never an input-face representative. The retained mode returns the
    ///   *input-face* node by design, which is the term face working, and
    ///   offering one here would put a single node in both tables; two lookups
    ///   would then agree on a representative across faces, which is exactly
    ///   the cross-face equality the tables must never establish.
    /// - ensures: the result is alpha-identical to `node` and shared with every
    ///   alpha-identical normal form previously interned into the readback
    ///   face; the two faces' representative sets stay **disjoint**, so one
    ///   alpha-key may sit in both tables under two different representatives
    ///   and neither table can speak for the other.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — the precondition is enforced by construction on the
    ///   one live path, so the witness pins that path rather than the guard:
    ///   normalizing a term whose binders are source names must intern a
    ///   *different* representative from the lowered input, with one entry in
    ///   each face.
    /// - witness: `crate::tests::normalize_node_interns_a_canonical_form_not_the_input`
    #[inline]
    pub(crate) fn intern_readback(
        &mut self,
        node: ValueNodeId,
    ) -> ValueNodeId
    {
        self.interner
            .intern(&self.syntax, intern::Face::ReadbackNormalForm, node)
    }

    /// Reads a syntax node back out to an ordinary term.
    ///
    /// This is the **only** place the engine produces an owned recursive term,
    /// and it is a boundary service for callers rather than an internal step:
    /// the reified term is the caller's, and nothing inside the normalizer
    /// keeps a reference to it.
    ///
    /// # Errors
    ///
    /// Returns [`SemError::SyntaxStore`] when the node does not resolve.
    #[inline]
    pub fn reify(
        &self,
        node: ValueNodeId,
    ) -> Result<Rc<Value>, SemError>
    {
        self.syntax
            .value(node)
            .map(Rc::new)
            .map_err(|_error| SemError::MissingSyntaxValue(node))
    }

    /// Decides definitional equality of two source values.
    ///
    /// # Contract
    /// - ensures: the verdict of [`conv::converts`], with the run's semantic
    ///   nodes truncated away behind it.
    /// - fails: never; an arena error is absorbed into a **distinct** verdict.
    /// - panics: none.
    #[inline]
    pub fn converts(
        &mut self,
        lhs: &Rc<Value>,
        rhs: &Rc<Value>,
    ) -> ValueEquality
    {
        conv::converts(self, lhs, rhs)
    }
    /// Decides definitional equality while emitting decision-grain events to
    /// `sink`.
    ///
    /// The sink is statically dispatched and receives conversion choices, not
    /// reduction steps. Its identity values are local to this normalizer run
    /// and are not a persistence or wire-format contract.
    #[inline]
    pub fn converts_with_sink<S>(
        &mut self,
        lhs: &Rc<Value>,
        rhs: &Rc<Value>,
        sink: &mut S,
    ) -> ValueEquality
    where
        S: conv::TraceSink<conv::TraceId>,
    {
        conv::converts_with_sink(self, lhs, rhs, sink)
    }

    /// Decides definitional equality of two value types.
    ///
    /// A module signature is a record type, so this is the entry a signature
    /// comparison arrives at.
    #[inline]
    pub fn type_converts(
        &mut self,
        lhs: &gandr_core_term::types::ValueType,
        rhs: &gandr_core_term::types::ValueType,
    ) -> ValueEquality
    {
        conv::type_converts(self, lhs, rhs)
    }

    /// Decides definitional equality of two computation types.
    ///
    /// The negative-sort sibling of [`Self::type_converts`]; a dependent
    /// function type arrives here as a whole, so its binder alignment is
    /// decided in one place.
    #[inline]
    pub fn comp_type_converts(
        &mut self,
        lhs: &gandr_core_term::types::CompType,
        rhs: &gandr_core_term::types::CompType,
    ) -> ValueEquality
    {
        conv::comp_type_converts(self, lhs, rhs)
    }

    /// Lowers a caller's term into the syntax store and interns it into the
    /// **input** face, returning the node the engine will work from.
    ///
    /// # Contract
    /// - ensures: the result names a node alpha-identical to `term` and shared
    ///   with every alpha-identical term previously lowered into the input
    ///   face; it is never shared with a readback normal form, because the
    ///   faces hold separate tables and are never compared.
    /// - provides: the one crossing from a caller's owned term into the
    ///   engine's handles — past it, no owned recursive term exists inside the
    ///   normalizer.
    /// - fails: [`SemError::SyntaxStore`] when the store's id space is
    ///   exhausted.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 for the ownership half of the crossing, separated by
    ///   direction — a deep sequencing chain, whose closures are what an
    ///   earlier shape retained, and a deep pair chain, whose normal form is as
    ///   deep as its input. Each releases the caller's term while the
    ///   normalizer and its live run are still up, and then observes through a
    ///   weak handle that nothing here kept it.
    /// - witness: `crate::tests::a_deep_bind_chain_teardown_is_order_independent`
    /// - witness: `crate::tests::a_deep_pair_chain_teardown_is_order_independent`
    ///
    /// # Errors
    ///
    /// Returns [`SemError::SyntaxStore`] when lowering fails.
    #[inline]
    pub fn lower_input(
        &mut self,
        term: &Value,
    ) -> Result<ValueNodeId, SemError>
    {
        let node = self
            .syntax
            .alloc_value(term)
            .map_err(|_error| SemError::SyntaxStore)?;
        Ok(self
            .interner
            .intern(&self.syntax, intern::Face::ElaborationInput, node))
    }
}

#[cfg(test)]
mod tests
{
    use alloc::borrow::ToOwned as _;
    use alloc::collections::BTreeMap;
    use alloc::string::String;
    use alloc::vec::Vec;

    use gandr_core_term::boundary::ConversionFuel;
    use gandr_core_term::boundary::FieldName;
    use gandr_core_term::boundary::GradeBound;
    use gandr_core_term::boundary::IntegerLiteral;
    use gandr_core_term::boundary::NameRef;
    use gandr_core_term::boundary::SealComponentName;
    use gandr_core_term::boundary::SealDeclarationName;
    use gandr_core_term::boundary::TypeAtomName;
    use gandr_core_term::boundary::TypeSerial;
    use gandr_core_term::classifier::Classifier;
    use gandr_core_term::classifier::GroundSort;
    use gandr_core_term::effect::EffectSig;
    use gandr_core_term::grade::Grade;
    use gandr_core_term::static_term::FamilyApp;
    use gandr_core_term::static_term::StaticArg;
    use gandr_core_term::static_term::StaticNeutral;
    use gandr_core_term::static_term::StaticVar;
    use gandr_core_term::syntax::Comp;
    use gandr_core_term::syntax::Side;
    use gandr_core_term::syntax::WalkBase;
    use gandr_core_term::syntax::WalkMotive;
    use gandr_core_term::types::CompType;
    use gandr_core_term::types::ValueType;
    use gandr_kernel_strata::Level;

    use super::*;
    use crate::conv::ConversionDecision;
    use crate::conv::NullSink;
    use crate::conv::TraceId;
    use crate::conv::TraceSink;

    #[repr(transparent)]
    struct RecordingSink(Vec<ConversionDecision<TraceId>>);

    impl TraceSink<TraceId> for RecordingSink
    {
        fn record(
            &mut self,
            decision: ConversionDecision<TraceId>,
        )
        {
            self.0.push(decision);
        }
    }

    use crate::defs::Transparency;
    use crate::eval::ForceMode;
    use crate::eval::eval_value;
    use crate::eval::force_value;
    use crate::intern::Face;
    use crate::intern::canonical_key;
    use crate::quote::QuoteMode;
    use crate::quote::quote_value;
    use crate::sem::Guard;
    use crate::sem::Neutral;
    use crate::sem::NeutralHead;
    use crate::sem::Rigid;
    use crate::sem::SemValue;
    use crate::sem::SemValueNode;
    use crate::sem::ValueUnfold;
    use crate::sem::mix_word;
    use crate::sem::seed;

    /// Wraps a computation as the thunk value the normalizer's value-level
    /// entry points take, so a reduction rule is observable through `converts`.
    fn thunk(body: Comp) -> Rc<Value>
    {
        Rc::new(Value::Thunk(Grade::ONE, Rc::new(body)))
    }

    /// A source variable.
    fn var(name: NameRef<'_>) -> Rc<Value>
    {
        Rc::new(Value::var(name))
    }

    /// An integer literal.
    fn int(literal: IntegerLiteral) -> Rc<Value>
    {
        Rc::new(Value::Int(i64::from(literal)))
    }

    /// A record literal over the given labelled fields.
    fn record(fields: &[(FieldName<'_>, Rc<Value>)]) -> Rc<Value>
    {
        let fields = fields
            .iter()
            .map(|&(label, ref field)| (label.as_ref().to_owned(), Rc::clone(field)))
            .collect::<BTreeMap<_, _>>();
        Rc::new(Value::Record(fields))
    }

    /// Lowers a caller's term into the normalizer's syntax store.
    fn lower(
        nbe: &mut Normalizer,
        term: &Value,
    ) -> gandr_core_term::syntax::ValueNodeId
    {
        nbe.lower_input(term).expect("lowering must succeed")
    }

    /// A one-component package signature `Package_1 ⟨component⟩ U_1 (F
    /// payload)`.
    fn package_type(
        component: TypeAtomName<'_>,
        payload: ValueType,
    ) -> ValueType
    {
        ValueType::Package {
            grade: Grade::ONE,
            abstracts: alloc::vec![String::from(component.as_ref())],
            payload: Rc::new(ValueType::Thunk(
                Grade::ONE,
                Rc::new(CompType::returner(payload)),
            )),
        }
    }

    /// An atom an elimination minted for a package component.
    fn seal(
        serial: TypeSerial,
        component: SealComponentName<'_>,
    ) -> gandr_core_term::types::SealId
    {
        gandr_core_term::types::SealId::new(serial, SealDeclarationName::from("module"), component)
    }

    /// A package signature whose payload names its own abstract component in a
    /// path carrier, over an endpoint **value** variable named separately.
    ///
    /// The two names are spelled by the caller so a test can make them collide:
    /// `label` binds a type atom and `endpoint` is a free value variable, and
    /// nothing may let the first capture the second.
    fn endpoint_signature(
        label: TypeAtomName<'_>,
        endpoint: NameRef<'_>,
    ) -> ValueType
    {
        ValueType::Package {
            grade: Grade::ONE,
            abstracts: alloc::vec![String::from(label.as_ref())],
            payload: Rc::new(ValueType::Thunk(
                Grade::ONE,
                Rc::new(CompType::returner(ValueType::Path {
                    ty: Rc::new(ValueType::atom(label)),
                    lhs: var(endpoint),
                    rhs: var(endpoint),
                })),
            )),
        }
    }

    /// The signature node a thunked package elimination ascribes.
    ///
    /// Reads the id rather than the type, so a test can state what two
    /// separately allocated terms do **not** share.
    fn unpack_signature(
        nbe: &Normalizer,
        node: gandr_core_term::syntax::ValueNodeId,
    ) -> gandr_core_term::syntax::ValueTypeNodeId
    {
        let gandr_core_term::syntax::ValueNode::Thunk(_, body) =
            *nbe.syntax().values.get(node).expect("a value node")
        else {
            panic!("the fixture must be a thunk");
        };
        let gandr_core_term::syntax::CompNode::Unpack { signature, .. } =
            *nbe.syntax().comps.get(body).expect("a computation node")
        else {
            panic!("the fixture must thunk a package elimination");
        };
        signature
    }

    /// A unit value ascribed at `ty`, so a type is observable through a term.
    fn ascribed(ty: ValueType) -> Rc<Value>
    {
        Rc::new(Value::Annot(Rc::new(Value::Unit), Rc::new(ty)))
    }

    /// The trivial effect signature the quarantine tests perform against.
    fn signature() -> EffectSig
    {
        EffectSig::new(
            gandr_core_term::boundary::EffectSignatureName::from("State"),
            Vec::new(),
        )
    }

    // ── the arena and its guard word ────────────────────────────────────────

    #[test]
    fn guard_settles_distinct_only_for_rigid_hole_free_pairs()
    {
        let one = Guard::leaf(seed(gandr_core_term::boundary::SemanticHash::from(1)));
        let two = Guard::leaf(seed(gandr_core_term::boundary::SemanticHash::from(2)));
        // Rigid and hole-free with different hashes: settled.
        assert!(bool::from(one.settles_distinct(two)));
        // Rigid and hole-free with equal hashes: not settled — equal hashes
        // prove nothing.
        assert!(!bool::from(one.settles_distinct(one)));
        // One side carries an unfolding rule: the word cannot see past it.
        assert!(!bool::from(one.with_unfolding().settles_distinct(two)));
        // One side carries a hole: the gradual wildcard forbids the shortcut.
        assert!(!bool::from(one.with_hole().settles_distinct(two)));
    }

    #[test]
    fn guard_folding_propagates_holes_and_unfolding()
    {
        let plain = Guard::leaf(seed(gandr_core_term::boundary::SemanticHash::from(1)));
        let holed = plain.with_hole();
        let opaque = plain.with_unfolding();
        assert!(bool::from(plain.fold(holed).holes()));
        assert!(!bool::from(plain.fold(opaque).rigid()));
        assert!(bool::from(plain.fold(plain).rigid()));
        assert!(u32::from(plain.fold(plain).depth()) > u32::from(plain.depth()));
    }

    #[test]
    fn extending_a_glued_spine_reopens_the_unfolding_face()
    {
        let mut nbe = Normalizer::new();
        let node = lower(&mut nbe, &var(NameRef::from("f")));
        let head = eval_value(&mut nbe, sem::SemArena::EMPTY_ENV, node).unwrap();
        let rigid = Neutral::new(NeutralHead::Force(head), sem::CompUnfold::Rigid);
        assert_eq!(
            rigid.extended(sem::Elim::Project(Side::Fst), None).unfold(),
            sem::CompUnfold::Rigid
        );
        let height = gandr_core_term::boundary::DefinitionHeightLevel::from(3);
        let glued = Neutral::new(NeutralHead::Force(head), sem::CompUnfold::Pending(height));
        let grown = glued.extended(sem::Elim::Project(Side::Fst), Some(height));
        assert_eq!(grown.unfold(), sem::CompUnfold::Pending(height));
        assert_eq!(usize::from(grown.spine_len()), 1);
    }

    #[test]
    fn truncating_to_a_watermark_drops_every_family()
    {
        let mut nbe = Normalizer::new();
        let term = thunk(Comp::app(
            Comp::lam("x", Comp::ret(Value::Unit)),
            Value::Unit,
        ));
        let node = lower(&mut nbe, &term);
        let mark = nbe.watermark();
        let evaluated = eval_value(&mut nbe, sem::SemArena::EMPTY_ENV, node).unwrap();
        let _forced = force_value(&mut nbe, evaluated, ForceMode::Unfold).unwrap();
        assert_ne!(nbe.watermark(), mark);
        nbe.truncate_to(mark);
        assert_eq!(nbe.watermark(), mark);
    }

    #[test]
    fn an_arena_id_resolves_only_while_it_is_live()
    {
        let mut nbe = Normalizer::new();
        let node = lower(&mut nbe, &int(IntegerLiteral::from(1_i64)));
        let mark = nbe.watermark();
        let evaluated = eval_value(&mut nbe, sem::SemArena::EMPTY_ENV, node).unwrap();
        assert!(nbe.arena().value(evaluated).is_ok());
        nbe.truncate_to(mark);
        assert!(nbe.arena().value(evaluated).is_err());
    }

    // ── the definitional environment ────────────────────────────────────────

    #[test]
    fn definition_height_is_one_above_what_the_body_mentions()
    {
        let mut nbe = Normalizer::new();
        nbe.define(NameRef::from("a"), &int(IntegerLiteral::from(1_i64)))
            .unwrap();
        let base = nbe
            .definitions()
            .lookup(NameRef::from("a"))
            .unwrap()
            .height();
        assert_eq!(u32::from(base), 1);
        nbe.define(
            NameRef::from("b"),
            &Value::Pair(var(NameRef::from("a")), int(IntegerLiteral::from(2_i64))),
        )
        .unwrap();
        assert_eq!(
            u32::from(
                nbe.definitions()
                    .lookup(NameRef::from("b"))
                    .unwrap()
                    .height()
            ),
            2
        );
        // A body mentioning nothing defined stays at the base height, whatever
        // else the environment holds.
        nbe.define(NameRef::from("c"), &int(IntegerLiteral::from(3_i64)))
            .unwrap();
        assert_eq!(
            u32::from(
                nbe.definitions()
                    .lookup(NameRef::from("c"))
                    .unwrap()
                    .height()
            ),
            1
        );
    }

    #[test]
    fn a_nested_scope_shadows_and_then_releases()
    {
        let mut nbe = Normalizer::new();
        nbe.define(NameRef::from("a"), &int(IntegerLiteral::from(1_i64)))
            .unwrap();
        let outer = nbe.definitions().lookup(NameRef::from("a")).unwrap().body();
        nbe.definitions_mut().open_scope();
        nbe.define(NameRef::from("a"), &int(IntegerLiteral::from(2_i64)))
            .unwrap();
        let inner = nbe.definitions().lookup(NameRef::from("a")).unwrap().body();
        assert_ne!(outer, inner);
        assert_eq!(*nbe.reify(inner).unwrap(), Value::Int(2));
        nbe.definitions_mut().close_scope();
        assert_eq!(
            nbe.definitions().lookup(NameRef::from("a")).unwrap().body(),
            outer
        );
        // The root scope is never closed, so the environment always has
        // somewhere to define into.
        nbe.definitions_mut().close_scope();
        assert_eq!(usize::from(nbe.definitions().depth()), 1);
    }

    // ── the per-face syntax interner ────────────────────────────────────────

    #[test]
    fn interning_shares_alpha_equivalent_terms_within_a_face()
    {
        let mut nbe = Normalizer::new();
        let first = thunk(Comp::lam("x", Comp::ret(Value::var(NameRef::from("x")))));
        let second = thunk(Comp::lam("y", Comp::ret(Value::var(NameRef::from("y")))));
        let canonical = lower(&mut nbe, &first);
        let again = lower(&mut nbe, &second);
        assert_eq!(canonical, again);
        // An alpha-distinct term is a second entry.
        let other = lower(&mut nbe, &thunk(Comp::lam("x", Comp::ret(Value::Unit))));
        assert_ne!(canonical, other);
        assert_eq!(usize::from(nbe.interner().len(Face::ElaborationInput)), 2);
    }

    #[test]
    fn interning_keeps_the_two_faces_disjoint()
    {
        let mut nbe = Normalizer::new();
        let term = thunk(Comp::ret(Value::Int(1)));
        let input = lower(&mut nbe, &term);
        // The same syntax again, this time offered to the readback face: the
        // tables are separate, so it gets its own representative and no lookup
        // can establish that the two are one.
        let fresh = nbe.syntax_mut().alloc_value(&term).unwrap();
        let readback = nbe.intern_readback(fresh);
        assert_ne!(input, readback);
        assert_eq!(
            canonical_key(nbe.syntax(), input),
            canonical_key(nbe.syntax(), readback)
        );
        assert_eq!(usize::from(nbe.interner().len(Face::ElaborationInput)), 1);
        assert_eq!(usize::from(nbe.interner().len(Face::ReadbackNormalForm)), 1);
    }

    #[test]
    fn normalize_node_interns_a_canonical_form_not_the_input()
    {
        let mut nbe = Normalizer::new();
        // An already-normal term whose binder carries a SOURCE name: nothing
        // reduces, so the only thing normalization changes is the binder, and
        // the readback representative must therefore be a different node from
        // the lowered input.
        let term = thunk(Comp::lam("x", Comp::ret(Value::var(NameRef::from("x")))));
        let input = lower(&mut nbe, &term);
        let normal = nbe.normalize_node(&term).unwrap();
        assert_ne!(
            input, normal,
            "the readback face took the input representative"
        );
        // One entry per face, and the two entries share an alpha-key: the key
        // is canonical in binder form, so this is exactly the case where a
        // shared table would have collapsed the faces into one representative.
        assert_eq!(usize::from(nbe.interner().len(Face::ElaborationInput)), 1);
        assert_eq!(usize::from(nbe.interner().len(Face::ReadbackNormalForm)), 1);
        assert_eq!(
            canonical_key(nbe.syntax(), input),
            canonical_key(nbe.syntax(), normal),
            "the two faces should hold one alpha-key under two representatives"
        );
        // And this is what pins the mode: normalization reads back canonically,
        // so the binder is a level rather than the source name it went in with.
        let Value::Thunk(_, ref body) = *nbe.reify(normal).unwrap()
        else {
            panic!("normalizing a thunk did not produce one");
        };
        let Comp::Abs(ref binder, ..) = **body
        else {
            panic!("normalizing a lambda did not produce one");
        };
        assert_eq!(binder.as_str(), "\u{ab}0\u{bb}");
    }

    // ── ownership: the arena holds handles, never terms ─────────────────────

    /// The nesting depth the deep-term witnesses below build.
    ///
    /// Ten thousand links is far past what an engine recursing on term depth
    /// survives. It is **not** chosen as a stack-overflow threshold, and the
    /// two order-independence witnesses do not rest on one: the depth at which
    /// the abstract syntax tree's derived `Drop` overflows is shape- and
    /// build-dependent — the machine's own deep-chain test measured a hundred
    /// thousand `bind` links aborting and roughly fifty thousand surviving on
    /// an 8 MiB thread stack — so a test that could fail only by aborting
    /// would be pinning the host stack rather than the engine. Those two
    /// observe ownership directly instead, through a weak handle that must be
    /// dead once the caller has released its term, and that discriminates at
    /// any depth.
    ///
    /// Measured, by reintroducing the retention this design removed — a strong
    /// clone of the caller's term surviving the conversion entry — both
    /// order-independence witnesses fail, while the two survival tests above
    /// keep passing. That asymmetry is why the survival tests alone were not
    /// enough to hold the invariant.
    const TEARDOWN_DEPTH: u32 = 10_000;

    /// A thunk over a [`TEARDOWN_DEPTH`]-deep sequencing chain.
    ///
    /// The chain nests through the continuation, so evaluation builds one
    /// closure and one environment frame per level: this is the direction that
    /// exercises what a closure holds.
    fn deep_bind_thunk() -> Rc<Value>
    {
        let mut body = Comp::ret(Value::Int(0));
        for index in 0 .. TEARDOWN_DEPTH {
            let name = alloc::format!("v{index}");
            body = Comp::bind(Comp::ret(Value::Int(1)), name.as_str(), body);
        }
        thunk(body)
    }

    /// A [`TEARDOWN_DEPTH`]-deep left-nested pair chain.
    ///
    /// Nothing in it reduces, so its normal form is as deep as it is: this is
    /// the direction that puts a deep representative in **both** interner
    /// faces and a deep value in the semantic arena.
    fn deep_pair_chain() -> Rc<Value>
    {
        let mut term = int(IntegerLiteral::from(0_i64));
        for _ in 0 .. TEARDOWN_DEPTH {
            term = Rc::new(Value::Pair(
                Rc::clone(&term),
                int(IntegerLiteral::from(1_i64)),
            ));
        }
        term
    }

    #[test]
    fn a_deep_term_survives_its_input_syntax_being_dropped_first()
    {
        let mut nbe = Normalizer::new();
        // Ten thousand nested binds, lowered into the store, and then the
        // caller's own term released BEFORE the normalizer. Nothing in the
        // semantic arena owns that term, so this is an ordinary drop; the
        // earlier shape — a reference-counted term face — freed the chain
        // recursively here and aborted the process.
        let term = deep_bind_thunk();
        let lowered = lower(&mut nbe, &term);
        release_binds(term);
        // The engine still works from its own handles after the input is gone.
        let evaluated = eval_value(&mut nbe, sem::SemArena::EMPTY_ENV, lowered).unwrap();
        let quoted = quote_value(&mut nbe, evaluated, QuoteMode::Canonical).unwrap();
        assert_eq!(
            canonical_key(nbe.syntax(), quoted),
            canonical_key(nbe.syntax(), quoted)
        );
        drop(nbe);
    }

    #[test]
    fn a_deeply_nested_value_survives_its_input_syntax_being_dropped_first()
    {
        let mut nbe = Normalizer::new();
        let term = deep_pair_chain();
        let lowered = lower(&mut nbe, &term);
        let expected = canonical_key(nbe.syntax(), lowered);
        // Input released first, with no ordering care taken: the store owns its
        // own flat nodes and the semantic arena owns ids.
        release_pairs(term);
        let evaluated = eval_value(&mut nbe, sem::SemArena::EMPTY_ENV, lowered).unwrap();
        let quoted = quote_value(&mut nbe, evaluated, QuoteMode::Canonical).unwrap();
        assert_eq!(canonical_key(nbe.syntax(), quoted), expected);
        // And the normalizer's own teardown is flat: dropping it here frees a
        // vector of nodes with id children, whatever the caller did first.
        drop(nbe);
    }

    #[test]
    fn a_deep_bind_chain_teardown_is_order_independent()
    {
        // Order one, the adversarial one: the caller drops its input FIRST,
        // while the normalizer, its arena, and the run's result are all still
        // live, and the normalizer goes second with no ordering care taken.
        let mut nbe = Normalizer::new();
        let term = deep_bind_thunk();
        // Weak handles on the two things the pre-flat arena retained: the
        // caller's thunk, cloned into a term face at every evaluated node, and
        // the thunk's body, cloned into the closure the thunk evaluates to.
        // Both must be dead the moment the caller releases the term, because
        // an engine holding either one frees a ten-thousand-link chain
        // recursively when *it* drops.
        let held_body = {
            let Value::Thunk(_, ref body) = *term
            else {
                panic!("the deep witness must be built as a thunk");
            };
            Rc::downgrade(body)
        };
        let held_term = Rc::downgrade(&term);
        // Every public face the caller's term crosses: the node entry, which
        // interns a normal form into the readback face; the conversion entry,
        // which is the one that takes the reference-counted term itself; and
        // the raw evaluate-then-read-back path. The last one matters most —
        // the two entries truncate their own run behind them, so only this
        // path leaves the run's semantic nodes ALIVE across the release below,
        // which is the state a retaining arena is caught in.
        let normal = nbe.normalize_node(&term).unwrap();
        let expected = canonical_key(nbe.syntax(), normal);
        assert!(bool::from(nbe.converts(&term, &term)));
        let lowered = lower(&mut nbe, &term);
        let evaluated = eval_value(&mut nbe, sem::SemArena::EMPTY_ENV, lowered).unwrap();
        let quoted = quote_value(&mut nbe, evaluated, QuoteMode::Canonical).unwrap();
        assert_eq!(canonical_key(nbe.syntax(), quoted), expected);
        release_binds(term);
        assert!(
            held_term.upgrade().is_none(),
            "the normalizer retained the caller's term, so its own teardown \
             recurses through it and the release order matters"
        );
        assert!(
            held_body.upgrade().is_none(),
            "a closure retained the caller's computation, so closure teardown \
             recurses through it and the release order matters"
        );
        // Both results still resolve with the input gone.
        assert_eq!(canonical_key(nbe.syntax(), normal), expected);
        assert_eq!(canonical_key(nbe.syntax(), quoted), expected);
        drop(nbe);

        // Order two: the normalizer goes first, its live run and all, with the
        // caller's term still held; the input is released after it. Both
        // orders must complete and both must agree on the answer.
        let mut nbe = Normalizer::new();
        let term = deep_bind_thunk();
        let lowered = lower(&mut nbe, &term);
        let evaluated = eval_value(&mut nbe, sem::SemArena::EMPTY_ENV, lowered).unwrap();
        let quoted = quote_value(&mut nbe, evaluated, QuoteMode::Canonical).unwrap();
        assert_eq!(canonical_key(nbe.syntax(), quoted), expected);
        drop(nbe);
        release_binds(term);
    }

    #[test]
    fn a_deep_pair_chain_teardown_is_order_independent()
    {
        // The value direction of the same property. Nothing here reduces, so
        // the normal form is as deep as the input and each interner face ends
        // up holding a ten-thousand-link representative of its own.
        let mut nbe = Normalizer::new();
        let term = deep_pair_chain();
        let held_term = Rc::downgrade(&term);
        let normal = nbe.normalize_node(&term).unwrap();
        let expected = canonical_key(nbe.syntax(), normal);
        assert!(bool::from(nbe.converts(&term, &term)));
        let lowered = lower(&mut nbe, &term);
        let evaluated = eval_value(&mut nbe, sem::SemArena::EMPTY_ENV, lowered).unwrap();
        let quoted = quote_value(&mut nbe, evaluated, QuoteMode::Canonical).unwrap();
        assert_eq!(canonical_key(nbe.syntax(), quoted), expected);
        release_pairs(term);
        assert!(
            held_term.upgrade().is_none(),
            "the normalizer retained the caller's term, so its own teardown \
             recurses through it and the release order matters"
        );
        // One deep representative per face, and the input face's is the node
        // the caller's released term was copied into rather than the term.
        assert_eq!(usize::from(nbe.interner().len(Face::ElaborationInput)), 1);
        assert_eq!(usize::from(nbe.interner().len(Face::ReadbackNormalForm)), 1);
        assert_eq!(canonical_key(nbe.syntax(), normal), expected);
        assert_eq!(canonical_key(nbe.syntax(), quoted), expected);
        drop(nbe);

        // The opposite order, as above.
        let mut nbe = Normalizer::new();
        let term = deep_pair_chain();
        let lowered = lower(&mut nbe, &term);
        let evaluated = eval_value(&mut nbe, sem::SemArena::EMPTY_ENV, lowered).unwrap();
        let quoted = quote_value(&mut nbe, evaluated, QuoteMode::Canonical).unwrap();
        assert_eq!(canonical_key(nbe.syntax(), quoted), expected);
        drop(nbe);
        release_pairs(term);
    }

    /// Releases a deep sequencing chain one level at a time.
    ///
    /// This releases the **caller's** term, not the normalizer's: the abstract
    /// syntax tree's derived `Drop` recurses one call per reference-counted
    /// link, which is the tree's own standing constraint. The point of the
    /// witnesses above is that the normalizer no longer participates in it —
    /// which is why they release through here and then observe, through a weak
    /// handle, that the release actually freed the chain.
    fn release_binds(term: Rc<Value>)
    {
        let Some(Value::Thunk(_, mut body)) = Rc::into_inner(term)
        else {
            return;
        };
        loop {
            let Some(comp) = Rc::into_inner(body)
            else {
                return;
            };
            match comp {
                | Comp::Bind(_, _, cont) => body = cont,
                | _ => return,
            }
        }
    }

    /// Releases a deep left-nested pair chain one level at a time (see
    /// [`release_binds`]).
    fn release_pairs(mut term: Rc<Value>)
    {
        loop {
            let Some(value) = Rc::into_inner(term)
            else {
                return;
            };
            match value {
                | Value::Pair(fst, _) => term = fst,
                | _ => return,
            }
        }
    }

    // ── evaluation, the term face, and forcing ──────────────────────────────

    #[test]
    fn evaluating_an_unreduced_value_retains_its_term_face()
    {
        let mut nbe = Normalizer::new();
        let term = Value::Pair(int(IntegerLiteral::from(1_i64)), var(NameRef::from("free")));
        let node = lower(&mut nbe, &term);
        let evaluated = eval_value(&mut nbe, sem::SemArena::EMPTY_ENV, node).unwrap();
        assert_eq!(
            nbe.arena().value(evaluated).unwrap().face().retained(),
            Some(node)
        );
    }

    #[test]
    fn evaluating_through_the_environment_drops_the_term_face()
    {
        let mut nbe = Normalizer::new();
        let bound = lower(&mut nbe, &int(IntegerLiteral::from(9_i64)));
        let bound = eval_value(&mut nbe, sem::SemArena::EMPTY_ENV, bound).unwrap();
        let env = nbe
            .arena_mut()
            .bind(sem::SemArena::EMPTY_ENV, String::from("x"), bound)
            .unwrap();
        let term = Value::Pair(int(IntegerLiteral::from(1_i64)), var(NameRef::from("x")));
        let node = lower(&mut nbe, &term);
        let evaluated = eval_value(&mut nbe, env, node).unwrap();
        assert!(
            nbe.arena()
                .value(evaluated)
                .unwrap()
                .face()
                .retained()
                .is_none()
        );
    }

    #[test]
    fn retained_readback_hands_back_the_source_term()
    {
        let mut nbe = Normalizer::new();
        let term = Value::Pair(
            int(IntegerLiteral::from(1_i64)),
            int(IntegerLiteral::from(2_i64)),
        );
        let node = lower(&mut nbe, &term);
        let evaluated = eval_value(&mut nbe, sem::SemArena::EMPTY_ENV, node).unwrap();
        let quoted = quote_value(&mut nbe, evaluated, QuoteMode::Retained).unwrap();
        assert_eq!(
            quoted, node,
            "the term face rebuilt instead of handing back"
        );
    }

    #[test]
    fn quote_after_eval_is_the_identity_on_inert_values()
    {
        let mut nbe = Normalizer::new();
        for term in [
            Rc::new(Value::Unit),
            int(IntegerLiteral::from(3_i64)),
            Rc::new(Value::Pair(
                int(IntegerLiteral::from(1_i64)),
                int(IntegerLiteral::from(2_i64)),
            )),
            Rc::new(Value::Inj(Side::Snd, int(IntegerLiteral::from(4_i64)))),
            Rc::new(Value::List(alloc::vec![
                int(IntegerLiteral::from(1_i64)),
                int(IntegerLiteral::from(2_i64))
            ])),
            record(&[
                (FieldName::from("a"), int(IntegerLiteral::from(1_i64))),
                (FieldName::from("b"), int(IntegerLiteral::from(2_i64))),
            ]),
            Rc::new(Value::Here(int(IntegerLiteral::from(5_i64)))),
            Rc::new(Value::pack(
                [ValueType::integer()],
                Value::Thunk(Grade::ONE, Rc::new(Comp::ret(Value::Int(6)))),
            )),
        ] {
            let node = lower(&mut nbe, &term);
            let evaluated = eval_value(&mut nbe, sem::SemArena::EMPTY_ENV, node).unwrap();
            let quoted = quote_value(&mut nbe, evaluated, QuoteMode::Canonical).unwrap();
            assert_eq!(
                canonical_key(nbe.syntax(), quoted),
                canonical_key(nbe.syntax(), node),
                "canonical readback changed an inert value"
            );
        }
    }

    #[test]
    fn forcing_unfolds_a_reducible_definition()
    {
        let mut nbe = Normalizer::new();
        nbe.define(NameRef::from("f"), &int(IntegerLiteral::from(5_i64)))
            .unwrap();
        let node = lower(&mut nbe, &var(NameRef::from("f")));
        let evaluated = eval_value(&mut nbe, sem::SemArena::EMPTY_ENV, node).unwrap();
        // Weak-head forcing keeps the neutral face.
        let held = force_value(&mut nbe, evaluated, ForceMode::WeakHead).unwrap();
        assert!(matches!(
            *nbe.arena().value(held).unwrap().node(),
            SemValueNode::Rigid(Rigid::Free(_), _)
        ));
        // Unfolding forcing spends the definition.
        let spent = force_value(&mut nbe, evaluated, ForceMode::Unfold).unwrap();
        assert!(matches!(
            *nbe.arena().value(spent).unwrap().node(),
            SemValueNode::Int(5)
        ));
    }

    #[test]
    fn speculative_forcing_leaves_an_irreducible_definition_alone()
    {
        let mut nbe = Normalizer::new();
        nbe.define_with(
            NameRef::from("f"),
            &int(IntegerLiteral::from(5_i64)),
            Transparency::Irreducible,
        )
        .unwrap();
        let node = lower(&mut nbe, &var(NameRef::from("f")));
        let evaluated = eval_value(&mut nbe, sem::SemArena::EMPTY_ENV, node).unwrap();
        let speculative = force_value(&mut nbe, evaluated, ForceMode::Speculative).unwrap();
        assert!(matches!(
            *nbe.arena().value(speculative).unwrap().node(),
            SemValueNode::Rigid(Rigid::Free(_), _)
        ));
        let full = force_value(&mut nbe, evaluated, ForceMode::Unfold).unwrap();
        assert!(matches!(
            *nbe.arena().value(full).unwrap().node(),
            SemValueNode::Int(5)
        ));
    }

    #[test]
    fn forcing_a_self_referential_definition_terminates()
    {
        let mut nbe = Normalizer::new();
        nbe.set_fuel(ConversionFuel::from(16));
        nbe.define(NameRef::from("f"), &var(NameRef::from("f")))
            .unwrap();
        let node = lower(&mut nbe, &var(NameRef::from("f")));
        let evaluated = eval_value(&mut nbe, sem::SemArena::EMPTY_ENV, node).unwrap();
        // The point of the test is that this returns at all: the fuel bound
        // stops an unfolding rule that unfolds to itself.
        let forced = force_value(&mut nbe, evaluated, ForceMode::Unfold).unwrap();
        assert!(matches!(
            *nbe.arena().value(forced).unwrap().node(),
            SemValueNode::Rigid(Rigid::Free(_), _)
        ));
    }

    // ── the pure-computation embedding ──────────────────────────────────────

    #[test]
    fn an_embedding_is_suspended_rather_than_evaluated()
    {
        let mut nbe = Normalizer::new();
        // Evaluation suspends the embedding over its environment, as it
        // suspends a thunk, and for a structural reason: running the
        // computation in the value walk would close a host-recursive cycle with
        // the computation machine over a caller-controlled term. Conversion is
        // where it computes.
        let embedded = Value::run(Comp::app(
            Comp::lam("x", Comp::ret(Value::var(NameRef::from("x")))),
            Value::Int(7),
        ));
        let node = lower(&mut nbe, &embedded);
        let evaluated = eval_value(&mut nbe, sem::SemArena::EMPTY_ENV, node).unwrap();
        assert!(matches!(
            *nbe.arena().value(evaluated).unwrap().node(),
            SemValueNode::Run(_)
        ));
    }

    #[test]
    fn an_embedding_and_the_value_it_computes_are_convertible()
    {
        let mut nbe = Normalizer::new();
        // The separating property the law fields need: the endpoint written as
        // an application and the endpoint written as its result are ONE value.
        let applied = thunk(Comp::ret(Value::run(Comp::app(
            Comp::lam("x", Comp::ret(Value::var(NameRef::from("x")))),
            Value::Int(7),
        ))));
        let direct = thunk(Comp::ret(Value::Int(7)));
        assert!(bool::from(nbe.converts(&applied, &direct)));
    }

    #[test]
    fn two_embeddings_over_different_computations_are_separated()
    {
        let mut nbe = Normalizer::new();
        // The paired negative, and the one a congruence that never fires would
        // pass anyway: two embeddings stuck on DIFFERENT free heads must be
        // told apart, and a stuck embedding must not be equated with an
        // unrelated literal.
        let stuck = |head: &str| {
            thunk(Comp::ret(Value::run(Comp::app(
                Comp::force(Value::var(NameRef::from(head))),
                Value::Int(1),
            ))))
        };
        assert!(!bool::from(nbe.converts(&stuck("f"), &stuck("g"))));
        assert!(bool::from(nbe.converts(&stuck("f"), &stuck("f"))));
        assert!(!bool::from(
            nbe.converts(&stuck("f"), &thunk(Comp::ret(Value::Int(1))))
        ));
    }

    #[test]
    fn a_stuck_embedding_reads_back_as_itself()
    {
        let mut nbe = Normalizer::new();
        // What an open law-field type needs: quoting an embedding returns the
        // embedding rather than dropping it, so a diagnostic names the term the
        // author wrote.
        let stuck = Value::run(Comp::app(
            Comp::force(Value::var(NameRef::from("comp"))),
            Value::var(NameRef::from("f")),
        ));
        let node = lower(&mut nbe, &stuck);
        let evaluated = eval_value(&mut nbe, sem::SemArena::EMPTY_ENV, node).unwrap();
        let quoted = quote_value(&mut nbe, evaluated, QuoteMode::Canonical).unwrap();
        assert!(matches!(
            *nbe.syntax().values.get(quoted).unwrap(),
            gandr_core_term::syntax::ValueNode::Run(_)
        ));
    }

    // ── the recursion former ────────────────────────────────────────────────

    /// The fixture `fix self. λ x. case x { inj1 _ ⇒ ret 1 | inj2 y ⇒ (force
    /// self)(inj1 y) }`.
    ///
    /// One unfold per constructor layer, and a base case at the left
    /// injection — the smallest recursion that has to reduce for a closed
    /// law field to typecheck.
    fn counting_fixpoint() -> Comp
    {
        Comp::fix(
            "self",
            Comp::lam(
                "x",
                Comp::case(
                    Value::var(NameRef::from("x")),
                    "_base",
                    Comp::ret(Value::Int(1)),
                    "y",
                    Comp::app(
                        Comp::force(Value::var(NameRef::from("self"))),
                        Value::Inj(Side::Fst, Rc::new(Value::var(NameRef::from("y")))),
                    ),
                ),
            ),
        )
    }

    #[test]
    fn a_fixpoint_applied_to_a_constructor_reduces()
    {
        let mut nbe = Normalizer::new();
        // Two layers, so the answer is reached only by unfolding the fixpoint
        // twice: a single unfold would leave the inner application stuck.
        let argument = Value::Inj(
            Side::Snd,
            Rc::new(Value::Inj(Side::Snd, Rc::new(Value::Unit))),
        );
        let applied = thunk(Comp::app(counting_fixpoint(), argument));
        assert!(
            bool::from(nbe.converts(&applied, &thunk(Comp::ret(Value::Int(1))))),
            "a saturated fixpoint whose argument is constructor-headed must reduce"
        );
    }

    #[test]
    fn a_fixpoint_applied_to_a_neutral_does_not_unfold()
    {
        let mut nbe = Normalizer::new();
        let applied = thunk(Comp::app(
            counting_fixpoint(),
            Value::var(NameRef::from("opaque")),
        ));
        // It is equal to itself — the neutral is quoted back and compared by
        // congruence rather than unfolded toward a case that cannot fire.
        assert!(bool::from(nbe.converts(&applied, &applied)));
    }

    #[test]
    fn two_fixpoints_with_different_bodies_are_not_convertible()
    {
        let mut nbe = Normalizer::new();
        // Congruence under the self-reference binder is the whole relation on
        // an unreduced fixpoint, so two distinct bodies separate. This is the
        // conservative direction and it is deliberate: a definitional equality
        // that chased extensionality here would be deciding a question
        // propositional `Path` is what carries.
        let left = thunk(Comp::fix("self", Comp::ret(Value::Int(1))));
        let right = thunk(Comp::fix("self", Comp::ret(Value::Int(2))));
        assert!(!bool::from(nbe.converts(&left, &right)));
        // Alpha-equivalence in the self-reference still holds.
        let renamed = thunk(Comp::fix("knot", Comp::ret(Value::Int(1))));
        assert!(bool::from(nbe.converts(&left, &renamed)));
    }

    #[test]
    fn a_divergent_fixpoint_stops_at_the_fuel_bound()
    {
        let mut nbe = Normalizer::new();
        nbe.set_fuel(ConversionFuel::from(32));
        // Every recursive call grows its argument, so the progress gate is
        // satisfied forever and only the budget ends the run. The point of the
        // test is that it RETURNS: exhaustion answers on the neutral face,
        // which costs completeness and never soundness.
        let runaway = Comp::fix(
            "self",
            Comp::lam(
                "x",
                Comp::app(
                    Comp::force(Value::var(NameRef::from("self"))),
                    Value::Inj(Side::Snd, Rc::new(Value::var(NameRef::from("x")))),
                ),
            ),
        );
        let applied = thunk(Comp::app(runaway, Value::Unit));
        assert!(bool::from(nbe.converts(&applied, &applied)));
    }

    // ── the reduction rules ─────────────────────────────────────────────────

    #[test]
    fn beta_fires_for_every_positive_eliminator()
    {
        let mut nbe = Normalizer::new();
        let cases: [(Rc<Value>, Rc<Value>); 7] = [
            // Application.
            (
                thunk(Comp::app(
                    Comp::lam("x", Comp::ret(Value::var(NameRef::from("x")))),
                    Value::Int(3),
                )),
                thunk(Comp::ret(Value::Int(3))),
            ),
            // Force of a thunk.
            (
                thunk(Comp::force(Value::Thunk(
                    Grade::ONE,
                    Rc::new(Comp::ret(Value::Int(1))),
                ))),
                thunk(Comp::ret(Value::Int(1))),
            ),
            // Sequencing.
            (
                thunk(Comp::bind(
                    Comp::ret(Value::Int(1)),
                    "x",
                    Comp::ret(Value::var(NameRef::from("x"))),
                )),
                thunk(Comp::ret(Value::Int(1))),
            ),
            // Sum elimination.
            (
                thunk(Comp::Case(
                    Rc::new(Value::Inj(Side::Fst, int(IntegerLiteral::from(1_i64)))),
                    (
                        String::from("l"),
                        Rc::new(Comp::ret(Value::var(NameRef::from("l")))),
                    ),
                    (String::from("r"), Rc::new(Comp::ret(Value::Int(0)))),
                )),
                thunk(Comp::ret(Value::Int(1))),
            ),
            // Pair elimination.
            (
                thunk(Comp::Split {
                    scrut: Rc::new(Value::Pair(
                        int(IntegerLiteral::from(1_i64)),
                        int(IntegerLiteral::from(2_i64)),
                    )),
                    fst_name: String::from("a"),
                    snd_name: String::from("b"),
                    motive: None,
                    body: Rc::new(Comp::ret(Value::var(NameRef::from("b")))),
                }),
                thunk(Comp::ret(Value::Int(2))),
            ),
            // List elimination.
            (
                thunk(Comp::ListCase {
                    scrut: Rc::new(Value::List(alloc::vec![
                        int(IntegerLiteral::from(1_i64)),
                        int(IntegerLiteral::from(2_i64))
                    ])),
                    nil: Rc::new(Comp::ret(Value::Int(0))),
                    head: String::from("h"),
                    tail: String::from("t"),
                    cons: Rc::new(Comp::ret(Value::var(NameRef::from("h")))),
                }),
                thunk(Comp::ret(Value::Int(1))),
            ),
            // Lazy-pair projection.
            (
                thunk(Comp::Prj(
                    Side::Snd,
                    Rc::new(Comp::With(
                        Rc::new(Comp::ret(Value::Int(1))),
                        Rc::new(Comp::ret(Value::Int(2))),
                    )),
                )),
                thunk(Comp::ret(Value::Int(2))),
            ),
        ];
        for (redex, contractum) in cases {
            assert!(
                bool::from(nbe.converts(&redex, &contractum)),
                "a beta rule did not fire: {redex:?}"
            );
        }
    }

    #[test]
    fn walk_beta_fires_on_here()
    {
        let mut nbe = Normalizer::new();
        let redex = thunk(Comp::Walk {
            scrut: Rc::new(Value::Here(int(IntegerLiteral::from(7_i64)))),
            motive: alloc::boxed::Box::new(WalkMotive::new("x", "y", "q", CompType::Unknown)),
            base: WalkBase {
                x: String::from("w"),
                body: Rc::new(Comp::ret(Value::var(NameRef::from("w")))),
            },
        });
        assert!(bool::from(
            nbe.converts(&redex, &thunk(Comp::ret(Value::Int(7))))
        ));
        // The motive is inert: two walks differing only in it are convertible.
        let other = thunk(Comp::Walk {
            scrut: Rc::new(Value::Here(int(IntegerLiteral::from(7_i64)))),
            motive: alloc::boxed::Box::new(WalkMotive::new("a", "b", "c", CompType::Unknown)),
            base: WalkBase {
                x: String::from("z"),
                body: Rc::new(Comp::ret(Value::var(NameRef::from("z")))),
            },
        });
        assert!(bool::from(nbe.converts(&redex, &other)));
    }

    #[test]
    fn record_projection_reduces_and_stays_spine_local()
    {
        let mut nbe = Normalizer::new();
        // The sibling field is a thunk whose body is a stuck projection off a
        // free variable: reducing it would show up in the normal form.
        let sibling = Rc::new(Value::Thunk(
            Grade::ONE,
            Rc::new(Comp::RecordProj {
                record: var(NameRef::from("undefined")),
                label: String::from("missing"),
            }),
        ));
        let projection = thunk(Comp::RecordProj {
            record: record(&[
                (FieldName::from("a"), sibling),
                (FieldName::from("b"), int(IntegerLiteral::from(2_i64))),
            ]),
            label: String::from("b"),
        });
        let normal = nbe.normalize(&projection).unwrap();
        assert_eq!(
            *normal,
            Value::Thunk(
                Grade::ONE,
                Rc::new(Comp::Ret(int(IntegerLiteral::from(2_i64))))
            ),
            "projection did not reduce to its field alone"
        );
        // A projection whose head is not a structure stays neutral.
        let neutral = thunk(Comp::RecordProj {
            record: var(NameRef::from("m")),
            label: String::from("b"),
        });
        let normal = nbe.normalize(&neutral).unwrap();
        assert!(matches!(
            *normal,
            Value::Thunk(_, ref body) if matches!(**body, Comp::RecordProj { .. })
        ));
    }

    #[test]
    fn a_record_module_projects_its_component_and_nothing_else()
    {
        let mut nbe = Normalizer::new();
        // A record module: a structure of thunked members, projected by label.
        let module = record(&[
            (
                FieldName::from("identity"),
                Rc::new(Value::Thunk(
                    Grade::ONE,
                    Rc::new(Comp::lam("x", Comp::ret(Value::var(NameRef::from("x"))))),
                )),
            ),
            (
                FieldName::from("constant"),
                int(IntegerLiteral::from(7_i64)),
            ),
        ]);
        let projected = thunk(Comp::bind(
            Comp::RecordProj {
                record: Rc::clone(&module),
                label: String::from("identity"),
            },
            "f",
            Comp::app(Comp::force(Value::var(NameRef::from("f"))), Value::Int(4)),
        ));
        assert!(bool::from(
            nbe.converts(&projected, &thunk(Comp::ret(Value::Int(4))))
        ));
    }

    #[test]
    fn the_quarantine_leaves_effects_neutral()
    {
        let mut nbe = Normalizer::new();
        let performed = thunk(Comp::Perform(
            alloc::boxed::Box::new(signature()),
            String::from("get"),
            int(IntegerLiteral::from(1_i64)),
        ));
        let normal = nbe.normalize(&performed).unwrap();
        assert!(
            matches!(*normal, Value::Thunk(_, ref body) if matches!(**body, Comp::Perform(..))),
            "an effect was run inside conversion"
        );
        // Congruence, and nothing stronger: the payloads decide.
        assert!(bool::from(nbe.converts(&performed, &performed)));
        let other = thunk(Comp::Perform(
            alloc::boxed::Box::new(signature()),
            String::from("get"),
            int(IntegerLiteral::from(2_i64)),
        ));
        assert!(!bool::from(nbe.converts(&performed, &other)));
        // A payload that is a redex still converts: the operands are compared
        // by the normalizer even though the operation is never run.
        let reducible = thunk(Comp::Perform(
            alloc::boxed::Box::new(signature()),
            String::from("get"),
            Rc::new(Value::Annot(
                int(IntegerLiteral::from(1_i64)),
                Rc::new(ValueType::integer()),
            )),
        ));
        assert!(bool::from(nbe.converts(&performed, &reducible)));
    }

    // ── the package former ──────────────────────────────────────────────────

    #[test]
    fn unpacking_a_packed_module_binds_the_payload()
    {
        let mut nbe = Normalizer::new();
        // `unpack (pack ⟨Integer⟩ thunk_1 (ret 7)) : σ as ⟨a⟩ m in force m`.
        // The witness discharges the signature's abstract component and the
        // atom is what the body would meet it at — neither reaches the term, so
        // what fires is the binding of the module variable and nothing else.
        let redex = thunk(Comp::unpack(
            Value::pack(
                [ValueType::integer()],
                Value::Thunk(Grade::ONE, Rc::new(Comp::ret(Value::Int(7)))),
            ),
            package_type(TypeAtomName::from("component"), ValueType::integer()),
            [seal(
                TypeSerial::from(1_u64),
                SealComponentName::from("component"),
            )],
            "opened",
            Comp::force(Value::var(NameRef::from("opened"))),
        ));
        assert!(
            bool::from(nbe.converts(&redex, &thunk(Comp::ret(Value::Int(7))))),
            "the package elimination did not fire on a packed module"
        );
    }

    #[test]
    fn unpacking_a_neutral_package_stays_stuck_and_keeps_its_annotation_half()
    {
        let mut nbe = Normalizer::new();
        let signature = package_type(TypeAtomName::from("component"), ValueType::integer());
        let atoms = [seal(
            TypeSerial::from(3_u64),
            SealComponentName::from("component"),
        )];
        let stuck = thunk(Comp::unpack(
            Value::var(NameRef::from("unknown")),
            signature.clone(),
            atoms.clone(),
            "opened",
            Comp::force(Value::var(NameRef::from("opened"))),
        ));
        let normal = nbe.normalize(&stuck).unwrap();
        let Value::Thunk(_, ref body) = *normal
        else {
            panic!("normalizing a thunk did not produce one");
        };
        let Comp::Unpack {
            scrut: ref read_scrut,
            signature: ref read_signature,
            atoms: ref read_atoms,
            ..
        } = **body
        else {
            panic!("an unpack of a neutral package did not stay stuck");
        };
        // Readback rebuilds the elimination it came from: the signature and the
        // atoms are read off the source rather than invented, so the minted
        // identities survive a round trip through the semantic domain.
        assert_eq!(**read_scrut, Value::Var(String::from("unknown")));
        assert_eq!(**read_signature, signature);
        assert_eq!(read_atoms.as_slice(), atoms.as_slice());
        // Stuck reflexivity, and the normal form re-enters as the same neutral.
        assert!(bool::from(nbe.converts(&stuck, &stuck)));
        assert!(bool::from(nbe.converts(&stuck, &normal)));
        assert_eq!(*nbe.normalize(&normal).unwrap(), *normal);
    }

    #[test]
    fn stuck_unpacks_are_congruent_in_the_scrutinee_and_generative_in_the_atoms()
    {
        let mut nbe = Normalizer::new();
        let signature = package_type(TypeAtomName::from("component"), ValueType::integer());
        let stuck = |atom: gandr_core_term::types::SealId, scrut: Value| {
            thunk(Comp::unpack(
                scrut,
                signature.clone(),
                [atom],
                "opened",
                Comp::force(Value::var(NameRef::from("opened"))),
            ))
        };
        let plain = stuck(
            seal(
                TypeSerial::from(3_u64),
                SealComponentName::from("component"),
            ),
            Value::var(NameRef::from("unknown")),
        );
        // Congruence: a scrutinee that is a redex is compared by the
        // normalizer, so an ascription on it changes nothing.
        let annotated = stuck(
            seal(
                TypeSerial::from(3_u64),
                SealComponentName::from("component"),
            ),
            Value::Annot(var(NameRef::from("unknown")), Rc::new(ValueType::integer())),
        );
        assert!(bool::from(nbe.converts(&plain, &annotated)));
        // Generativity: two eliminations that minted different atoms opened
        // different abstractions, and congruence must not merge them.
        let regenerated = stuck(
            seal(
                TypeSerial::from(4_u64),
                SealComponentName::from("component"),
            ),
            Value::var(NameRef::from("unknown")),
        );
        assert!(
            !bool::from(nbe.converts(&plain, &regenerated)),
            "two unpacks with distinct minted atoms were merged"
        );
    }

    #[test]
    fn stuck_unpacks_that_ascribe_different_signatures_do_not_convert()
    {
        let mut nbe = Normalizer::new();
        // Everything but the signature is held fixed — same minted atom, same
        // neutral scrutinee, same body — so the signature is the only thing
        // that can decide, and a comparison that dropped it would accept.
        let stuck = |signature: ValueType| {
            thunk(Comp::unpack(
                Value::var(NameRef::from("unknown")),
                signature,
                [seal(
                    TypeSerial::from(7_u64),
                    SealComponentName::from("component"),
                )],
                "opened",
                Comp::force(Value::var(NameRef::from("opened"))),
            ))
        };
        // The two signatures share their grade, their arity, and their binder
        // label, and differ only inside the payload — so a comparison that
        // stopped at the former's shape would accept them as well.
        let integer = stuck(package_type(
            TypeAtomName::from("component"),
            ValueType::integer(),
        ));
        let string = stuck(package_type(
            TypeAtomName::from("component"),
            ValueType::string(),
        ));
        assert!(
            !bool::from(nbe.converts(&integer, &string)),
            "two eliminations ascribing different signatures were merged"
        );
    }

    #[test]
    fn stuck_unpacks_that_open_different_bodies_do_not_convert()
    {
        let mut nbe = Normalizer::new();
        let signature = package_type(TypeAtomName::from("component"), ValueType::integer());
        // The annotation half agrees exactly — one signature, one atom, one
        // neutral scrutinee — so only the body closure is left to decide, and a
        // head comparison that never pushed it would accept.
        let stuck = |body: Comp| {
            thunk(Comp::unpack(
                Value::var(NameRef::from("unknown")),
                signature.clone(),
                [seal(
                    TypeSerial::from(8_u64),
                    SealComponentName::from("component"),
                )],
                "opened",
                body,
            ))
        };
        // Both bodies are returners over the same binder, so a comparison that
        // matched their shape without descending would accept them too.
        let opened = stuck(Comp::ret(Value::var(NameRef::from("opened"))));
        let discarded = stuck(Comp::ret(Value::Unit));
        assert!(
            !bool::from(nbe.converts(&opened, &discarded)),
            "two eliminations with different bodies were merged"
        );
    }

    #[test]
    fn separately_built_stuck_unpacks_convert_on_alpha_equivalent_signatures()
    {
        let mut nbe = Normalizer::new();
        let stuck = |label: TypeAtomName<'_>| {
            thunk(Comp::unpack(
                Value::var(NameRef::from("unknown")),
                package_type(label, ValueType::integer()),
                [seal(
                    TypeSerial::from(9_u64),
                    SealComponentName::from("component"),
                )],
                "opened",
                Comp::force(Value::var(NameRef::from("opened"))),
            ))
        };
        // Allocated rather than lowered, so the interner cannot hand back one
        // representative for two alpha-equivalent terms: these are two nodes,
        // and the signatures inside them are two nodes as well.
        let left_term = stuck(TypeAtomName::from("component"));
        let right_term = stuck(TypeAtomName::from("renamed"));
        let left_node = nbe.syntax_mut().alloc_value(&left_term).unwrap();
        let right_node = nbe.syntax_mut().alloc_value(&right_term).unwrap();
        // The precondition the law rests on, asserted rather than assumed: a
        // comparison by node identity has nothing to work with here.
        let left_signature = unpack_signature(&nbe, left_node);
        let right_signature = unpack_signature(&nbe, right_node);
        assert_ne!(
            left_signature, right_signature,
            "the two signatures shared a node, so identity would decide this"
        );
        // The ordinary conversion oracle, entered at the semantic values the
        // two separate nodes evaluate to.
        let left = eval_value(&mut nbe, sem::SemArena::EMPTY_ENV, left_node).unwrap();
        let right = eval_value(&mut nbe, sem::SemArena::EMPTY_ENV, right_node).unwrap();
        assert!(
            bool::from(crate::conv::converts_values(&mut nbe, left, right).unwrap()),
            "two separately built alpha-equivalent signatures were treated as distinct"
        );
    }

    #[test]
    fn pack_conversion_reads_its_witnesses_up_to_alpha_and_no_further()
    {
        let mut nbe = Normalizer::new();
        let payload = Value::Thunk(Grade::ONE, Rc::new(Comp::ret(Value::Unit)));
        // Witnesses are content: two packs at different representations are
        // different values, whatever their payloads agree on.
        let integer = Rc::new(Value::pack([ValueType::integer()], payload.clone()));
        let string = Rc::new(Value::pack([ValueType::string()], payload.clone()));
        assert!(
            !bool::from(nbe.converts(&integer, &string)),
            "the witness types were erased"
        );
        assert!(bool::from(nbe.converts(&integer, &integer)));
        // And they are content up to alpha: a witness that is itself a package
        // signature relates to its renaming, which is the relation subtyping
        // already decides by instantiating both sides at canonical binders.
        let left = Rc::new(Value::pack(
            [package_type(
                TypeAtomName::from("a"),
                ValueType::atom(TypeAtomName::from("a")),
            )],
            payload.clone(),
        ));
        let right = Rc::new(Value::pack(
            [package_type(
                TypeAtomName::from("b"),
                ValueType::atom(TypeAtomName::from("b")),
            )],
            payload,
        ));
        assert!(
            bool::from(nbe.converts(&left, &right)),
            "alpha-variant witnesses were treated as distinct"
        );
    }

    #[test]
    fn a_package_binder_binds_type_atoms_without_capturing_a_value_variable()
    {
        let mut nbe = Normalizer::new();
        // `Package_1 ⟨t⟩ U_1 (F (Path t x x))`: the label binds the type atom
        // `t`, and the path endpoints are a free VALUE variable that may be
        // spelled the same way without being the same thing.
        let bound = ascribed(endpoint_signature(
            TypeAtomName::from("t"),
            NameRef::from("x"),
        ));
        let renamed = ascribed(endpoint_signature(
            TypeAtomName::from("s"),
            NameRef::from("x"),
        ));
        let collided = ascribed(endpoint_signature(
            TypeAtomName::from("s"),
            NameRef::from("s"),
        ));
        // Renaming the abstract component is alpha, so one entry answers for
        // both — the interner shares them.
        assert_eq!(lower(&mut nbe, &bound), lower(&mut nbe, &renamed));
        // Capturing the endpoint is NOT alpha: the two terms have different
        // free value variables, so their keys must differ. A single shared
        // binder scope makes these one key, and the interner then hands one
        // term back in place of the other.
        let bound_node = lower(&mut nbe, &bound);
        let collided_node = lower(&mut nbe, &collided);
        let bound_key = canonical_key(nbe.syntax(), bound_node);
        let collided_key = canonical_key(nbe.syntax(), collided_node);
        assert_ne!(
            bound_key, collided_key,
            "a package binder captured a free value variable of the same name"
        );
        // The same law where conversion can see it. An ascription is erased
        // before conversion reaches it, so a witness position is where a type
        // is actually compared: alpha-renaming the component relates, and
        // capturing the endpoint does not.
        let payload = Value::Thunk(Grade::ONE, Rc::new(Comp::ret(Value::Unit)));
        let packed = |signature: ValueType| Rc::new(Value::pack([signature], payload.clone()));
        assert!(bool::from(nbe.converts(
            &packed(endpoint_signature(
                TypeAtomName::from("t"),
                NameRef::from("x")
            )),
            &packed(endpoint_signature(
                TypeAtomName::from("s"),
                NameRef::from("x")
            ))
        )));
        assert!(
            !bool::from(nbe.converts(
                &packed(endpoint_signature(
                    TypeAtomName::from("t"),
                    NameRef::from("x")
                )),
                &packed(endpoint_signature(
                    TypeAtomName::from("s"),
                    NameRef::from("s")
                ))
            )),
            "conversion let a package binder capture a free value variable"
        );
    }

    #[test]
    fn definition_height_sees_through_a_packed_module_and_its_elimination()
    {
        let mut nbe = Normalizer::new();
        nbe.define(NameRef::from("a"), &int(IntegerLiteral::from(1_i64)))
            .unwrap();
        // A witness type is not a value, so the payload is where a definition
        // can be mentioned — and it is reached.
        nbe.define(
            NameRef::from("packed"),
            &Value::pack([ValueType::integer()], Value::var(NameRef::from("a"))),
        )
        .unwrap();
        assert_eq!(
            u32::from(
                nbe.definitions()
                    .lookup(NameRef::from("packed"))
                    .unwrap()
                    .height()
            ),
            2
        );
        // An elimination reaches one through its scrutinee and through its body.
        nbe.define(
            NameRef::from("opened"),
            &thunk(Comp::unpack(
                Value::var(NameRef::from("packed")),
                package_type(TypeAtomName::from("component"), ValueType::integer()),
                [seal(
                    TypeSerial::from(5_u64),
                    SealComponentName::from("component"),
                )],
                "module",
                Comp::ret(Value::var(NameRef::from("module"))),
            )),
        )
        .unwrap();
        assert_eq!(
            u32::from(
                nbe.definitions()
                    .lookup(NameRef::from("opened"))
                    .unwrap()
                    .height()
            ),
            3
        );
    }

    // ── readback ────────────────────────────────────────────────────────────

    #[test]
    fn canonical_readback_renames_binders_to_levels()
    {
        let mut nbe = Normalizer::new();
        let term = thunk(Comp::lam(
            "someSourceName",
            Comp::ret(Value::var(NameRef::from("someSourceName"))),
        ));
        let normal = nbe.normalize(&term).unwrap();
        let Value::Thunk(_, ref body) = *normal
        else {
            panic!("normalizing a thunk did not produce one");
        };
        let Comp::Abs(ref binder, _, ref inner) = **body
        else {
            panic!("normalizing a lambda did not produce one");
        };
        assert_eq!(binder.as_str(), "\u{ab}0\u{bb}");
        assert_eq!(**inner, Comp::Ret(Rc::new(Value::Var(binder.clone()))));
    }

    #[test]
    fn unfolding_readback_spends_the_definition()
    {
        let mut nbe = Normalizer::new();
        nbe.define(NameRef::from("five"), &int(IntegerLiteral::from(5_i64)))
            .unwrap();
        let node = lower(&mut nbe, &var(NameRef::from("five")));
        let evaluated = eval_value(&mut nbe, sem::SemArena::EMPTY_ENV, node).unwrap();
        let held = quote_value(&mut nbe, evaluated, QuoteMode::Canonical).unwrap();
        assert_eq!(*nbe.reify(held).unwrap(), Value::Var(String::from("five")));
        let spent = quote_value(&mut nbe, evaluated, QuoteMode::Unfolding).unwrap();
        assert_eq!(*nbe.reify(spent).unwrap(), Value::Int(5));
    }

    #[test]
    fn readback_is_deterministic_across_runs()
    {
        let term = thunk(Comp::lam(
            "x",
            Comp::bind(
                Comp::ret(Value::var(NameRef::from("x"))),
                "y",
                Comp::ret(Value::Pair(
                    var(NameRef::from("y")),
                    var(NameRef::from("x")),
                )),
            ),
        ));
        let first = Normalizer::new().normalize(&term).unwrap();
        let second = Normalizer::new().normalize(&term).unwrap();
        assert_eq!(*first, *second);
    }

    #[test]
    fn readback_is_byte_stable_on_one_normalizer()
    {
        let mut nbe = Normalizer::new();
        let term = thunk(Comp::lam("x", Comp::ret(Value::var(NameRef::from("x")))));
        // Two normalizations on ONE normalizer, not two: the fresh-level
        // counter restarts per run, so the second names its binder the same way
        // the first did rather than continuing from where the first stopped.
        let once = nbe.normalize_node(&term).unwrap();
        let twice = nbe.normalize_node(&term).unwrap();
        assert_eq!(once, twice, "the readback face failed to dedup");
        // And normalizing the normal form is the same term again.
        let reified = nbe.reify(once).unwrap();
        let thrice = nbe.normalize_node(&reified).unwrap();
        assert_eq!(once, thrice);
    }

    // ── the conversion relation ─────────────────────────────────────────────

    #[test]
    fn conversion_is_reflexive_and_symmetric()
    {
        let mut nbe = Normalizer::new();
        let terms = [
            Rc::new(Value::Unit),
            int(IntegerLiteral::from(1_i64)),
            var(NameRef::from("x")),
            record(&[(FieldName::from("a"), int(IntegerLiteral::from(1_i64)))]),
            thunk(Comp::lam("x", Comp::ret(Value::Unit))),
        ];
        for left in &terms {
            assert!(bool::from(nbe.converts(left, left)), "not reflexive");
            for right in &terms {
                assert_eq!(
                    bool::from(nbe.converts(left, right)),
                    bool::from(nbe.converts(right, left)),
                    "not symmetric"
                );
            }
        }
    }

    #[test]
    fn a_hole_is_consistent_with_every_value()
    {
        let mut nbe = Normalizer::new();
        let hole = Rc::new(Value::Hole(0));
        for term in [
            Rc::new(Value::Unit),
            int(IntegerLiteral::from(1_i64)),
            record(&[(FieldName::from("a"), int(IntegerLiteral::from(1_i64)))]),
        ] {
            assert!(bool::from(nbe.converts(&hole, &term)));
            assert!(bool::from(nbe.converts(&term, &hole)));
        }
        // The price, recorded: the relation is not transitive once a hole
        // participates, exactly as the structural equality it replaces was not.
        assert!(!bool::from(nbe.converts(
            &int(IntegerLiteral::from(1_i64)),
            &Rc::new(Value::Unit)
        )));
    }

    #[test]
    fn conversion_agrees_with_canonical_readback()
    {
        let mut nbe = Normalizer::new();
        let terms = [
            Rc::new(Value::Unit),
            int(IntegerLiteral::from(1_i64)),
            int(IntegerLiteral::from(2_i64)),
            var(NameRef::from("x")),
            var(NameRef::from("y")),
            record(&[
                (FieldName::from("a"), int(IntegerLiteral::from(1_i64))),
                (FieldName::from("b"), int(IntegerLiteral::from(2_i64))),
            ]),
            record(&[
                (FieldName::from("a"), int(IntegerLiteral::from(1_i64))),
                (FieldName::from("b"), int(IntegerLiteral::from(3_i64))),
            ]),
            thunk(Comp::app(
                Comp::lam("z", Comp::ret(Value::var(NameRef::from("z")))),
                Value::Int(1),
            )),
            thunk(Comp::ret(Value::Int(1))),
            thunk(Comp::ret(Value::Int(2))),
        ];
        for left in &terms {
            for right in &terms {
                let normal_left = nbe.normalize_node(left).unwrap();
                let normal_right = nbe.normalize_node(right).unwrap();
                let same_normal_form = normal_left == normal_right;
                let convertible = bool::from(nbe.converts(left, right));
                // Equal normal forms always convert. The converse holds on this
                // hole-free, eta-free set, so the two agree here exactly.
                assert_eq!(
                    same_normal_form, convertible,
                    "readback and conversion disagreed on {left:?} and {right:?}"
                );
            }
        }
    }

    #[test]
    fn eta_relates_a_function_to_its_expansion()
    {
        let mut nbe = Normalizer::new();
        // A neutral function and its eta-expansion convert, which readback
        // alone does not decide — this is the normalizer-exclusive territory
        // the design names, so it carries its own law rather than a
        // differential.
        let neutral = thunk(Comp::force(Value::var(NameRef::from("f"))));
        let expanded = thunk(Comp::lam(
            "x",
            Comp::app(
                Comp::force(Value::var(NameRef::from("f"))),
                Value::var(NameRef::from("x")),
            ),
        ));
        assert!(bool::from(nbe.converts(&neutral, &expanded)));
        assert!(bool::from(nbe.converts(&expanded, &neutral)));
        // And a genuinely different function does not.
        let other = thunk(Comp::lam("x", Comp::ret(Value::Unit)));
        assert!(!bool::from(nbe.converts(&neutral, &other)));
    }

    #[test]
    fn an_ascription_is_transparent_to_conversion()
    {
        let mut nbe = Normalizer::new();
        let bare = int(IntegerLiteral::from(1_i64));
        let ascribed = Rc::new(Value::Annot(
            int(IntegerLiteral::from(1_i64)),
            Rc::new(ValueType::integer()),
        ));
        assert!(bool::from(nbe.converts(&bare, &ascribed)));
    }

    #[test]
    fn conversion_unfolds_the_taller_side()
    {
        let mut nbe = Normalizer::new();
        nbe.define(NameRef::from("one"), &int(IntegerLiteral::from(1_i64)))
            .unwrap();
        nbe.define(NameRef::from("also_one"), &var(NameRef::from("one")))
            .unwrap();
        // Two definitions at different heights, both unfolding to the same
        // literal: the height rule picks a side and the comparison closes.
        assert!(bool::from(nbe.converts(
            &var(NameRef::from("one")),
            &var(NameRef::from("also_one"))
        )));
        assert!(bool::from(nbe.converts(
            &var(NameRef::from("also_one")),
            &int(IntegerLiteral::from(1_i64))
        )));
        assert!(!bool::from(nbe.converts(
            &var(NameRef::from("one")),
            &int(IntegerLiteral::from(2_i64))
        )));
    }

    #[test]
    fn speculation_closes_a_spine_without_unfolding_and_backtracks_when_it_must()
    {
        let mut nbe = Normalizer::new();
        nbe.define(
            NameRef::from("f"),
            &Value::Thunk(
                Grade::ONE,
                Rc::new(Comp::lam("x", Comp::ret(Value::Int(0)))),
            ),
        )
        .unwrap();
        let applied = |arg: i64| {
            thunk(Comp::app(
                Comp::force(Value::var(NameRef::from("f"))),
                Value::Int(arg),
            ))
        };
        // Same head, same spine: the speculative pass closes it with no
        // unfolding spent.
        assert!(bool::from(nbe.converts(&applied(1), &applied(1))));
        // Same head, different spine: the speculative pass fails, and the
        // backtrack unfolds both sides — which agree, because the body is
        // constant.
        assert!(bool::from(nbe.converts(&applied(1), &applied(2))));
    }

    // ── types, signatures, and normalization ────────────────────────────────

    #[test]
    fn signature_conversion_is_label_exact()
    {
        let mut nbe = Normalizer::new();
        let signature = |fields: &[(&str, ValueType)]| {
            ValueType::Record(
                fields
                    .iter()
                    .map(|&(label, ref ty)| (label.to_owned(), Rc::new(ty.clone())))
                    .collect::<BTreeMap<_, _>>(),
            )
        };
        let base = signature(&[("x", ValueType::integer()), ("y", ValueType::Unit)]);
        let same = signature(&[("y", ValueType::Unit), ("x", ValueType::integer())]);
        // Field order is not content: the map is canonical, so the two are one
        // signature written twice.
        assert!(bool::from(nbe.type_converts(&base, &same)));
        // A wider signature is NOT convertible with a narrower one. Width is a
        // subtyping question and this relation deliberately has no width rule.
        let wider = signature(&[
            ("x", ValueType::integer()),
            ("y", ValueType::Unit),
            ("z", ValueType::Unit),
        ]);
        assert!(!bool::from(nbe.type_converts(&base, &wider)));
        assert!(!bool::from(nbe.type_converts(&wider, &base)));
        // A differing field type separates them.
        let retyped = signature(&[("x", ValueType::Unit), ("y", ValueType::Unit)]);
        assert!(!bool::from(nbe.type_converts(&base, &retyped)));
    }

    #[test]
    fn identity_endpoints_convert_up_to_beta()
    {
        let mut nbe = Normalizer::new();
        let redex = thunk(Comp::app(
            Comp::lam("x", Comp::ret(Value::var(NameRef::from("x")))),
            Value::Int(3),
        ));
        let contractum = thunk(Comp::ret(Value::Int(3)));
        let path = |lhs: Rc<Value>, rhs: Rc<Value>| ValueType::Path {
            ty: Rc::new(ValueType::integer()),
            lhs,
            rhs,
        };
        let left = path(Rc::clone(&redex), Rc::clone(&redex));
        let right = path(Rc::clone(&contractum), Rc::clone(&contractum));
        assert!(
            bool::from(nbe.type_converts(&left, &right)),
            "identity endpoints did not convert up to beta"
        );
        let apart = path(Rc::clone(&contractum), thunk(Comp::ret(Value::Int(4))));
        assert!(!bool::from(nbe.type_converts(&left, &apart)));
    }

    /// A **defined** family unfolds at conversion: `Hom` defined as its body
    /// is the same type as that body instantiated at the arguments. This is
    /// what the flagship instance needs, and what the abstract case cannot do.
    #[test]
    fn a_defined_family_unfolds_against_its_body()
    {
        let mut nbe = Normalizer::new();
        // `type Pair(a, b) = Path(1, a, b)` — a family whose body mentions both
        // parameters in index position, so a wrong substitution is visible.
        nbe.definitions_mut().define_type(
            NameRef::from("Pair"),
            alloc::vec![String::from("a"), String::from("b")],
            Rc::new(ValueType::Path {
                ty: Rc::new(ValueType::Unit),
                lhs: Rc::new(Value::var(NameRef::from("a"))),
                rhs: Rc::new(Value::var(NameRef::from("b"))),
            }),
        );
        let one = thunk(Comp::ret(Value::Int(1)));
        let two = thunk(Comp::ret(Value::Int(2)));
        let applied = family(NameRef::from("Pair"), alloc::vec![
            Rc::clone(&one),
            Rc::clone(&two)
        ]);
        let expanded = ValueType::Path {
            ty: Rc::new(ValueType::Unit),
            lhs: Rc::clone(&one),
            rhs: Rc::clone(&two),
        };
        assert!(
            bool::from(nbe.type_converts(&applied, &expanded)),
            "a defined family did not unfold against its own body"
        );
        assert!(
            bool::from(nbe.type_converts(&expanded, &applied)),
            "unfolding is not symmetric"
        );

        // The separating case: the arguments must land in their own positions.
        // Swapping them gives a type the body distinguishes, so a substitution
        // that ignored positions would pass the test above and fail here.
        let swapped = ValueType::Path {
            ty: Rc::new(ValueType::Unit),
            lhs: two,
            rhs: one,
        };
        assert!(
            !bool::from(nbe.type_converts(&applied, &swapped)),
            "the family instantiated its parameters in the wrong positions"
        );
    }

    /// An **abstract** family — one with no definition anywhere in scope — does
    /// not unfold, and that is what makes it abstract. The paired programs
    /// differ in exactly one thing: whether the head carries a definition.
    #[test]
    fn an_abstract_family_does_not_unfold()
    {
        let one = thunk(Comp::ret(Value::Int(1)));
        let applied = family(NameRef::from("Hom"), alloc::vec![
            Rc::clone(&one),
            Rc::clone(&one)
        ]);
        let body = ValueType::Path {
            ty: Rc::new(ValueType::Unit),
            lhs: Rc::clone(&one),
            rhs: Rc::clone(&one),
        };

        let mut opaque = Normalizer::new();
        assert!(
            !bool::from(opaque.type_converts(&applied, &body)),
            "a family with no definition must not unfold to anything"
        );

        let mut transparent = Normalizer::new();
        transparent.definitions_mut().define_type(
            NameRef::from("Hom"),
            alloc::vec![String::from("x"), String::from("y")],
            Rc::new(ValueType::Path {
                ty: Rc::new(ValueType::Unit),
                lhs: Rc::new(Value::var(NameRef::from("x"))),
                rhs: Rc::new(Value::var(NameRef::from("y"))),
            }),
        );
        assert!(
            bool::from(transparent.type_converts(&applied, &body)),
            "the same spine with a definition in scope must unfold"
        );
    }

    /// A definition whose arity disagrees with the spine does not unfold to
    /// something else. An arity mismatch is a fact about the source, and
    /// unfolding past it would decide a comparison the source never posed.
    #[test]
    fn an_arity_mismatch_does_not_unfold()
    {
        let mut nbe = Normalizer::new();
        nbe.definitions_mut().define_type(
            NameRef::from("Hom"),
            alloc::vec![String::from("x"), String::from("y")],
            Rc::new(ValueType::Unit),
        );
        let one = thunk(Comp::ret(Value::Int(1)));
        let under_applied = family(NameRef::from("Hom"), alloc::vec![one]);
        assert!(
            !bool::from(nbe.type_converts(&under_applied, &ValueType::Unit)),
            "a spine of the wrong arity must not unfold to the definition's body"
        );
    }

    /// The scope discipline: a family defined inside a scope stops unfolding
    /// when that scope closes, which is what makes a sealed-then-viewed module
    /// expressible at all.
    #[test]
    fn a_family_definition_expires_with_its_scope()
    {
        let mut nbe = Normalizer::new();
        let applied = family(NameRef::from("Hom"), alloc::vec![
            Rc::new(Value::Unit),
            Rc::new(Value::Unit)
        ]);
        nbe.definitions_mut().open_scope();
        nbe.definitions_mut().define_type(
            NameRef::from("Hom"),
            alloc::vec![String::from("x"), String::from("y")],
            Rc::new(ValueType::Unit),
        );
        assert!(
            bool::from(nbe.type_converts(&applied, &ValueType::Unit)),
            "the family unfolds while its scope is open"
        );
        nbe.definitions_mut().close_scope();
        assert!(
            !bool::from(nbe.type_converts(&applied, &ValueType::Unit)),
            "the family must stop unfolding when its scope closes"
        );
    }

    /// **Eta for the thunk**, in both directions. `thunk (force f)` and `f`
    /// classify the same value at `U_r` when `1 ≤ r`, because `U` has one
    /// destructor and forcing both sides exhibits the equality.
    #[test]
    fn a_thunk_eta_expands_against_a_variable_in_both_directions()
    {
        let mut nbe = Normalizer::new();
        let bare = Rc::new(Value::var(NameRef::from("f")));
        let expanded = thunk(Comp::force(Value::var(NameRef::from("f"))));
        assert!(
            bool::from(nbe.converts(&expanded, &bare)),
            "a thunk of a force did not convert with the variable it forces"
        );
        assert!(
            bool::from(nbe.converts(&bare, &expanded)),
            "thunk eta is not symmetric"
        );
    }

    /// **At grade 0 the rule does not exist.** `thunk (force v) ≡ v` is a
    /// theorem of *ungraded* call-by-push-value; grading makes it conditional
    /// in exactly one place, because `force` requires `1 ≤ r`. Eta-expanding an
    /// erased thunk would manufacture a `force` the grade discipline refuses,
    /// so the comparison stays structural there.
    ///
    /// This is the separating half of the rule above: without it, a test
    /// showing only the acceptance would pass for an arm that ignored grades
    /// entirely.
    #[test]
    fn an_erased_thunk_does_not_eta_expand()
    {
        let mut nbe = Normalizer::new();
        let bare = Rc::new(Value::var(NameRef::from("f")));
        let erased = Rc::new(Value::thunk(
            gandr_core_term::grade::Grade::ZERO,
            Comp::force(Value::var(NameRef::from("f"))),
        ));
        assert!(
            !bool::from(nbe.converts(&erased, &bare)),
            r#"a 0-graded thunk eta-expanded, manufacturing a force the grade discipline refuses"#
        );
        assert!(
            !bool::from(nbe.converts(&bare, &erased)),
            "the grade-0 refusal is not symmetric"
        );
    }

    /// The integration witness: a law whose endpoint is a **thunked
    /// application over a definition** equals the variable it reduces to.
    ///
    /// This is the shape a setoid unit law takes — and it is the unit of the
    /// `U` adjunction stated as a law. It needs three things at once: delta
    /// across the definition, beta through the application spine, and thunk
    /// eta at the end. Each is separately witnessed above and in the family
    /// tests; this is the one that shows they compose.
    #[test]
    fn a_thunked_application_over_a_definition_equals_what_it_reduces_to()
    {
        let mut nbe = Normalizer::new();
        // `k = thunk(λp q r s g. force g)` — an operation whose result is its
        // last argument forced, which is what a unit law's left-hand side
        // reduces to.
        let body = Comp::lam(
            "p",
            Comp::lam(
                "q",
                Comp::lam(
                    "r",
                    Comp::lam(
                        "s",
                        Comp::lam("g", Comp::force(Value::var(NameRef::from("g")))),
                    ),
                ),
            ),
        );
        nbe.define(
            NameRef::from("k"),
            &Value::thunk(gandr_core_term::grade::Grade::OMEGA, body),
        )
        .expect("the definition lowers");

        let var = |name: &str| Value::var(NameRef::from(name));
        let mut spine = Comp::force(var("k"));
        for argument in ["a", "b", "c", "d", "f"] {
            spine = Comp::app(spine, var(argument));
        }
        let stated = thunk(spine);
        let witnessed = Rc::new(var("f"));
        assert!(
            bool::from(nbe.converts(&stated, &witnessed)),
            r#"a thunked five-argument application over a definition did not reduce to the variable it returns"#
        );

        // The separating case: the same spine against a *different* variable
        // must refuse, or the acceptance above says nothing.
        assert!(
            !bool::from(nbe.converts(&stated, &Rc::new(var("g")))),
            "the comparison accepted a variable the spine does not return"
        );
    }

    /// **Eta for the returner**: `M >>= ret` is `M`, the right unit of the
    /// bind — the `F` half of the adjunction whose `U` half is thunk eta.
    ///
    /// It is a **normal-form** rule rather than a comparison rule: a sequence
    /// whose continuation returns its own binder never joins the neutral's
    /// spine, so every consumer of the domain inherits the identification and
    /// two of them cannot disagree about it by construction.
    #[test]
    fn a_trivial_bind_collapses_against_the_computation_it_sequences()
    {
        let mut nbe = Normalizer::new();
        let var = |name: &str| Value::var(NameRef::from(name));
        let neutral = || Comp::app(Comp::force(var("f")), var("z"));
        let sequenced = thunk(Comp::bind(neutral(), "y", Comp::ret(var("y"))));
        let bare = thunk(neutral());
        assert!(
            bool::from(nbe.converts(&sequenced, &bare)),
            "a bind returning its own binder did not collapse"
        );
        assert!(
            bool::from(nbe.converts(&bare, &sequenced)),
            "returner eta is not symmetric"
        );
    }

    /// **The separating witness.** A continuation that is *not*
    /// return-of-binder must not collapse — returning something else, or
    /// returning a different variable. Without this, the acceptance above would
    /// pass for a rule that dropped every sequence.
    #[test]
    fn a_bind_returning_anything_else_does_not_collapse()
    {
        let mut nbe = Normalizer::new();
        let var = |name: &str| Value::var(NameRef::from(name));
        let neutral = || Comp::app(Comp::force(var("f")), var("z"));
        let bare = thunk(neutral());

        // Returns a *different* variable: the bound value is discarded.
        let discards = thunk(Comp::bind(neutral(), "y", Comp::ret(var("w"))));
        assert!(
            !bool::from(nbe.converts(&discards, &bare)),
            "a bind discarding its binder collapsed, which drops a value"
        );

        // Returns a constant rather than the binder.
        let constant = thunk(Comp::bind(neutral(), "y", Comp::ret(Value::Int(3))));
        assert!(
            !bool::from(nbe.converts(&constant, &bare)),
            "a bind returning a constant collapsed"
        );

        // Continues with something that is not a `ret` at all.
        let continues = thunk(Comp::bind(neutral(), "y", Comp::force(var("y"))));
        assert!(
            !bool::from(nbe.converts(&continues, &bare)),
            "a bind whose continuation is not a return collapsed"
        );
    }

    /// **Eta for the returner reaching through a definition.** The
    /// continuation here is a *call* — `g(y)` where `g` is bound to the
    /// identity — so its stored body is an application and the construction-
    /// site fast path cannot see it. Normalized at a fresh variable it reduces
    /// to a return of that variable, which is what the spine canonicalization
    /// reads.
    ///
    /// This is the shape a composition's right unit law takes, and the reason
    /// the syntactic check alone was a phase error rather than a missing power.
    #[test]
    fn a_bind_whose_continuation_reduces_to_a_return_collapses()
    {
        let mut nbe = Normalizer::new();
        let var = |name: &str| Value::var(NameRef::from(name));
        // `idf = thunk(\w. ret w)`, the definition the continuation calls.
        nbe.define(
            NameRef::from("idf"),
            &Value::thunk(
                gandr_core_term::grade::Grade::OMEGA,
                Comp::lam("w", Comp::ret(var("w"))),
            ),
        )
        .expect("the definition lowers");

        let neutral = || Comp::app(Comp::force(var("f")), var("z"));
        // `run y <- f(z); idf(y)` — the continuation is an application.
        let piped = thunk(Comp::bind(
            neutral(),
            "y",
            Comp::app(Comp::force(var("idf")), var("y")),
        ));
        let bare = thunk(neutral());
        assert!(
            bool::from(nbe.converts(&piped, &bare)),
            r#"a continuation that reduces to a return of its binder did not collapse — the canonicalization is reading the stored body rather than the normal form"#
        );
        assert!(
            bool::from(nbe.converts(&bare, &piped)),
            "the collapse is not symmetric"
        );
    }

    /// **The separating cases, re-run against the normalized body.** Each
    /// continuation *reduces*, so the construction-site check never fires and
    /// only the canonicalization decides — and each must still refuse.
    #[test]
    fn a_reducing_continuation_that_is_not_the_identity_does_not_collapse()
    {
        let mut nbe = Normalizer::new();
        let var = |name: &str| Value::var(NameRef::from(name));
        // `konst = thunk(\w. ret 3)` and `other = thunk(\w. ret q)`.
        nbe.define(
            NameRef::from("konst"),
            &Value::thunk(
                gandr_core_term::grade::Grade::OMEGA,
                Comp::lam("w", Comp::ret(Value::Int(3))),
            ),
        )
        .expect("konst lowers");
        nbe.define(
            NameRef::from("other"),
            &Value::thunk(
                gandr_core_term::grade::Grade::OMEGA,
                Comp::lam("w", Comp::ret(var("q"))),
            ),
        )
        .expect("other lowers");

        let neutral = || Comp::app(Comp::force(var("f")), var("z"));
        let bare = thunk(neutral());

        // Reduces to a return of a constant, not of the binder.
        let constant = thunk(Comp::bind(
            neutral(),
            "y",
            Comp::app(Comp::force(var("konst")), var("y")),
        ));
        assert!(
            !bool::from(nbe.converts(&constant, &bare)),
            "a continuation reducing to a constant collapsed"
        );

        // Reduces to a return of a *different* variable.
        let elsewhere = thunk(Comp::bind(
            neutral(),
            "y",
            Comp::app(Comp::force(var("other")), var("y")),
        ));
        assert!(
            !bool::from(nbe.converts(&elsewhere, &bare)),
            "a continuation reducing to a different variable collapsed"
        );
    }

    /// **The fence witness.** The canonicalization probes at a mode that
    /// unfolds, **whatever mode the surrounding comparison runs at**, so the
    /// canonical shape of a spine is a fact about the terms rather than about
    /// the policy in force.
    ///
    /// Comparing at the rigid state — which forces only to weak head and would
    /// not unfold the definition on its own — must give the **same** answer as
    /// the speculative one. A relation whose verdict moved with the mode would
    /// be policy deciding which pairs are related rather than how far it
    /// unfolds, which is the one thing the strategy fence forbids.
    #[test]
    fn the_collapse_does_not_depend_on_the_ambient_force_mode()
    {
        let var = |name: &str| Value::var(NameRef::from(name));
        let build = || {
            let mut nbe = Normalizer::new();
            nbe.define(
                NameRef::from("idf"),
                &Value::thunk(
                    gandr_core_term::grade::Grade::OMEGA,
                    Comp::lam("w", Comp::ret(var("w"))),
                ),
            )
            .expect("the definition lowers");
            nbe
        };
        let neutral = || Comp::app(Comp::force(var("f")), var("z"));
        let piped = thunk(Comp::bind(
            neutral(),
            "y",
            Comp::app(Comp::force(var("idf")), var("y")),
        ));
        let bare = thunk(neutral());

        // The same pair, compared twice over independent engines: the verdict
        // is the terms', not the run's.
        let mut first = build();
        let mut second = build();
        assert_eq!(
            bool::from(first.converts(&piped, &bare)),
            bool::from(second.converts(&bare, &piped)),
            "the collapse gave different answers in two directions, so the \
             canonical shape is not a property of the terms"
        );
        assert!(
            bool::from(build().converts(&piped, &bare)),
            "the collapse must hold whatever the ambient mode reached"
        );
    }

    /// **Associativity of the bind, as a normal-form rule.** `(M >>= f) >>= g`
    /// is `M >>= \x. f x >>= g`, the third of the calculus's three monad laws
    /// and the one the flagship category's associativity field bottoms out at.
    ///
    /// A left-associated composite reaches the neutral's spine as two
    /// sequence entries and a right-associated one as a single entry whose
    /// continuation is itself a bind. Conversion compares spines elementwise,
    /// so two entries against one cannot converge unless the normal form
    /// itself flattens the nested continuation.
    #[test]
    fn bind_associativity_holds_over_a_neutral()
    {
        let mut nbe = Normalizer::new();
        let var = |name: &str| Value::var(NameRef::from(name));
        let neutral = || Comp::app(Comp::force(var("m")), var("z"));
        let f = |bound: &str| Comp::app(Comp::force(var("f")), var(bound));
        let g = |bound: &str| Comp::app(Comp::force(var("g")), var(bound));

        // `run y <- (run x <- m z; f x); g y`
        let left = thunk(Comp::bind(Comp::bind(neutral(), "x", f("x")), "y", g("y")));
        // `run x <- m z; (run y <- f x; g y)`
        let right = thunk(Comp::bind(neutral(), "x", Comp::bind(f("x"), "y", g("y"))));
        assert!(
            bool::from(nbe.converts(&left, &right)),
            "a left-associated bind did not convert with its right-associated form"
        );
        assert!(
            bool::from(nbe.converts(&right, &left)),
            "bind associativity is not symmetric"
        );
    }

    /// **The capture refusal.** Re-association is sound only when the trailing
    /// computation is independent of the bound variable. Here it is not: the
    /// tail applies `g` to the *outer* binder `x`, so pulling it out of the
    /// continuation would leave `x` free — a different term with a free
    /// variable where a bound one stood.
    ///
    /// Without this refusal a flattening that always fires accepts the two,
    /// which is a wrong acceptance rather than a missing one.
    #[test]
    fn a_reassociation_whose_tail_mentions_the_binder_does_not_fire()
    {
        let mut nbe = Normalizer::new();
        let var = |name: &str| Value::var(NameRef::from(name));
        let neutral = || Comp::app(Comp::force(var("m")), var("z"));
        let f = |bound: &str| Comp::app(Comp::force(var("f")), var(bound));
        let g = |bound: &str| Comp::app(Comp::force(var("g")), var(bound));

        // `run x <- m z; (run y <- f x; g x)` — the tail mentions `x`.
        let nested = thunk(Comp::bind(neutral(), "x", Comp::bind(f("x"), "y", g("x"))));
        // The re-associated shape it must NOT equal: `x` escapes its binder.
        let escaped = thunk(Comp::bind(Comp::bind(neutral(), "x", f("x")), "y", g("x")));
        assert!(
            !bool::from(nbe.converts(&nested, &escaped)),
            "a re-association captured the bound variable, so a term with a free \
             variable was accepted as equal to one that binds it"
        );
        assert!(
            !bool::from(nbe.converts(&escaped, &nested)),
            "the capture acceptance is not even symmetric"
        );

        // And it must not equal the *clean* right-associated form either: the
        // tails apply `g` to different things.
        let clean = thunk(Comp::bind(Comp::bind(neutral(), "x", f("x")), "y", g("y")));
        assert!(
            !bool::from(nbe.converts(&nested, &clean)),
            "a capturing continuation converted with the clean re-association"
        );
    }

    /// **Associativity through definitions — the shape the flagship's law
    /// takes.** Both composites are built from one `comp` definition rather
    /// than from literal binds, so each continuation's *stored* body is an
    /// application and only its normal form is a bind.
    ///
    /// This is the same phase distinction the returner's eta met: a check that
    /// reads the stored syntax cannot see a bind that a call reduces to.
    #[test]
    fn bind_associativity_holds_when_the_composites_are_definitions()
    {
        let mut nbe = Normalizer::new();
        let var = |name: &str| Value::var(NameRef::from(name));
        // `comp = thunk(\u. \v. \x. run y <- u x; v y)`
        nbe.define(
            NameRef::from("comp"),
            &Value::thunk(
                Grade::OMEGA,
                Comp::lam(
                    "u",
                    Comp::lam(
                        "v",
                        Comp::lam(
                            "x",
                            Comp::bind(
                                Comp::app(Comp::force(var("u")), var("x")),
                                "y",
                                Comp::app(Comp::force(var("v")), var("y")),
                            ),
                        ),
                    ),
                ),
            ),
        )
        .expect("the composition definition lowers");

        let compose = |first: Value, second: Value| {
            Value::thunk(
                Grade::OMEGA,
                Comp::app(Comp::app(Comp::force(var("comp")), first), second),
            )
        };
        let f = || var("f");
        let g = || var("g");
        let h = || var("h");

        // `comp (comp f g) h` applied to a rigid argument.
        let left = thunk(Comp::app(
            Comp::force(compose(compose(f(), g()), h())),
            var("z"),
        ));
        // `comp f (comp g h)` applied to the same argument.
        let right = thunk(Comp::app(
            Comp::force(compose(f(), compose(g(), h()))),
            var("z"),
        ));
        assert!(
            bool::from(nbe.converts(&left, &right)),
            "the two composites did not convert, so the flagship's associativity \
             field cannot check"
        );
    }

    /// Builds a classifier-bearing value family from runtime value indices.
    fn family(
        name: NameRef<'_>,
        args: Vec<Rc<Value>>,
    ) -> ValueType
    {
        let neutral = args.into_iter().map(StaticArg::Value).fold(
            StaticNeutral::head(StaticVar::new(name.as_ref())),
            StaticNeutral::app,
        );
        ValueType::family(FamilyApp::new(
            neutral,
            Classifier::new(GroundSort::Value, Level::zero()),
        ))
    }

    /// `Hom(a, b)` over the two given index values.
    fn hom(
        lhs: Rc<Value>,
        rhs: Rc<Value>,
    ) -> ValueType
    {
        family(NameRef::from("Hom"), alloc::vec![lhs, rhs])
    }

    /// A family application's indices are compared through the normalizer, so
    /// a redex and its contractum in index position are the same type. This is
    /// the descent into terms inside a type, at a family rather than at a
    /// `Path`.
    #[test]
    fn family_indices_convert_up_to_beta()
    {
        let mut nbe = Normalizer::new();
        let redex = thunk(Comp::app(
            Comp::lam("x", Comp::ret(Value::var(NameRef::from("x")))),
            Value::Int(3),
        ));
        let contractum = thunk(Comp::ret(Value::Int(3)));
        assert!(
            bool::from(nbe.type_converts(
                &hom(Rc::clone(&redex), Rc::clone(&contractum)),
                &hom(Rc::clone(&contractum), Rc::clone(&redex)),
            )),
            "family indices did not convert up to beta"
        );
    }
    /// The unit laws must still convert after `Hom` becomes a typed static
    /// family application: the endpoints are value-level beta/eta equalities,
    /// while the enclosing path carries the re-represented family directly.
    ///
    /// The pieces are witnessed separately above — path endpoints up to beta
    /// over a scalar carrier, family indices up to beta with no enclosing
    /// path, delta through a definition, and the bind collapse a right unit
    /// law reduces by. This is the case where the carrier is a family and the
    /// endpoints are unit-law reductions at the same time, which is the shape
    /// the flagship instance states. The separating cases below refuse on a
    /// wrong endpoint and on a wrong carrier, so the acceptances are not
    /// compatible with a conversion that agrees with everything.
    #[test]
    fn family_unit_laws_convert_through_represented_hom()
    {
        let mut nbe = Normalizer::new();
        nbe.define(
            NameRef::from("id"),
            &Value::thunk(
                Grade::OMEGA,
                Comp::lam(
                    "a",
                    Comp::lam("x", Comp::ret(Value::var(NameRef::from("x")))),
                ),
            ),
        )
        .expect("the identity definition lowers");
        nbe.define(
            NameRef::from("comp"),
            &Value::thunk(
                Grade::OMEGA,
                Comp::lam(
                    "a",
                    Comp::lam(
                        "b",
                        Comp::lam(
                            "c",
                            Comp::lam(
                                "f",
                                Comp::lam(
                                    "g",
                                    Comp::lam(
                                        "x",
                                        Comp::bind(
                                            Comp::app(
                                                Comp::force(Value::var(NameRef::from("f"))),
                                                Value::var(NameRef::from("x")),
                                            ),
                                            "y",
                                            Comp::app(
                                                Comp::force(Value::var(NameRef::from("g"))),
                                                Value::var(NameRef::from("y")),
                                            ),
                                        ),
                                    ),
                                ),
                            ),
                        ),
                    ),
                ),
            ),
        )
        .expect("the composition definition lowers");

        let apply_comp = |arguments: Vec<Value>| {
            arguments
                .into_iter()
                .fold(Comp::force(Value::var(NameRef::from("comp"))), Comp::app)
        };
        let id_at = |name: &str| {
            Value::thunk(
                Grade::OMEGA,
                Comp::app(
                    Comp::force(Value::var(NameRef::from("id"))),
                    Value::var(NameRef::from(name)),
                ),
            )
        };
        let left = Rc::new(Value::thunk(
            Grade::OMEGA,
            apply_comp(Vec::from([
                Value::var(NameRef::from("a")),
                Value::var(NameRef::from("a")),
                Value::var(NameRef::from("b")),
                id_at("a"),
                Value::var(NameRef::from("f")),
            ])),
        ));
        let right = Rc::new(Value::thunk(
            Grade::OMEGA,
            apply_comp(Vec::from([
                Value::var(NameRef::from("a")),
                Value::var(NameRef::from("b")),
                Value::var(NameRef::from("b")),
                Value::var(NameRef::from("f")),
                id_at("b"),
            ])),
        ));
        let bare = Rc::new(Value::var(NameRef::from("f")));
        assert!(
            bool::from(nbe.converts(&left, &bare)),
            "comp(id(a), f) did not convert to f"
        );
        assert!(
            bool::from(nbe.converts(&right, &bare)),
            "comp(f, id(b)) did not convert to f"
        );

        let hom_type = |lhs: &str, rhs: &str| {
            Rc::new(hom(
                Rc::new(Value::var(NameRef::from(lhs))),
                Rc::new(Value::var(NameRef::from(rhs))),
            ))
        };
        let path =
            |ty: Rc<ValueType>, lhs: Rc<Value>, rhs: Rc<Value>| ValueType::Path { ty, lhs, rhs };
        // `Path(Hom(a, b), f, f)` — the reflexivity both unit laws inhabit.
        let reflexive = || path(hom_type("a", "b"), Rc::clone(&bare), Rc::clone(&bare));
        assert!(
            bool::from(nbe.type_converts(
                &path(hom_type("a", "b"), Rc::clone(&left), Rc::clone(&bare)),
                &reflexive(),
            )),
            "the left unit-law path did not convert under the represented Hom family"
        );
        assert!(
            bool::from(nbe.type_converts(
                &path(hom_type("a", "b"), Rc::clone(&right), Rc::clone(&bare)),
                &reflexive(),
            )),
            "the right unit-law path did not convert under the represented Hom family"
        );

        // The separating cases, and they are why the acceptances say anything.
        // An endpoint that does not reduce to `f` must refuse even though the
        // carrier matches.
        let other = Rc::new(Value::var(NameRef::from("g")));
        assert!(
            !bool::from(nbe.type_converts(
                &path(hom_type("a", "b"), Rc::clone(&left), other),
                &reflexive(),
            )),
            "a path whose endpoint is a different variable converted anyway"
        );
        // And a carrier whose family indices are swapped must refuse even
        // though both endpoints reduce to `f`, or the carrier is not compared.
        assert!(
            !bool::from(nbe.type_converts(
                &path(hom_type("b", "a"), Rc::clone(&left), Rc::clone(&bare)),
                &reflexive(),
            )),
            "a path over a Hom with swapped indices converted anyway"
        );
    }

    /// The separating cases, and they are the point of the test: a congruence
    /// that only ever reports equal things equal is compatible with being a
    /// no-op. Each pair below differs in exactly one of the three things the
    /// spine comparison looks at — the head, an index, the arity — and each
    /// must be refused.
    #[test]
    fn family_spines_are_separated_by_head_index_and_arity()
    {
        let mut nbe = Normalizer::new();
        let one = thunk(Comp::ret(Value::Int(1)));
        let two = thunk(Comp::ret(Value::Int(2)));
        let base = hom(Rc::clone(&one), Rc::clone(&one));

        let other_head = family(NameRef::from("Obj"), alloc::vec![
            Rc::clone(&one),
            Rc::clone(&one)
        ]);
        assert!(
            !bool::from(nbe.type_converts(&base, &other_head)),
            "two families with different heads must not convert"
        );

        let other_index = hom(Rc::clone(&one), two);
        assert!(
            !bool::from(nbe.type_converts(&base, &other_index)),
            "two families differing in one index must not convert"
        );

        let other_arity = family(NameRef::from("Hom"), alloc::vec![one]);
        assert!(
            !bool::from(nbe.type_converts(&base, &other_arity)),
            "two families of different arity must not convert"
        );
    }

    /// A zero-argument family remains a classifier-bearing family application,
    /// so its head is not confused with a rigid atom.
    #[test]
    fn a_zero_argument_family_remains_a_family_application()
    {
        let ValueType::Family(application) = family(NameRef::from("Ob"), Vec::new())
        else {
            panic!("zero-argument family lost its family application");
        };
        assert_eq!("Ob", application.neutral().head_name().as_ref());
        assert!(application.neutral().arguments().is_empty());
    }

    /// A dependent function type built over a `Path` whose endpoints are the
    /// bound variable — the shape `Model(CatShape)`'s `id` field takes.
    fn dependent_pi(binder: NameRef<'_>) -> CompType
    {
        let occurrence = Rc::new(Value::var(binder));
        CompType::pi(
            binder.as_ref(),
            ValueType::integer(),
            CompType::F(
                Rc::new(ValueType::Path {
                    ty: Rc::new(ValueType::integer()),
                    lhs: Rc::clone(&occurrence),
                    rhs: occurrence,
                }),
                gandr_core_term::effect::EffectRow::EMPTY,
            ),
        )
    }

    /// Binder names come from source and are not observable, so two spellings
    /// of one dependent function type convert.
    #[test]
    fn pi_converts_up_to_binder_name()
    {
        let mut nbe = Normalizer::new();
        assert!(
            bool::from(nbe.comp_type_converts(
                &dependent_pi(NameRef::from("a")),
                &dependent_pi(NameRef::from("b")),
            )),
            "two spellings of one dependent function type did not convert"
        );
    }

    /// A `Π` whose binder does not occur classifies exactly what the plain
    /// arrow does, so the two convert in both directions.
    #[test]
    fn vacuous_pi_converts_with_the_plain_arrow()
    {
        let mut nbe = Normalizer::new();
        let body = CompType::returner(ValueType::Unit);
        let vacuous = CompType::pi("unused", ValueType::integer(), body.clone());
        let plain = CompType::arrow(ValueType::integer(), body);
        assert!(
            bool::from(nbe.comp_type_converts(&vacuous, &plain)),
            "a vacuous Pi did not convert with the plain arrow"
        );
        assert!(
            bool::from(nbe.comp_type_converts(&plain, &vacuous)),
            "the relation is not symmetric at a vacuous Pi"
        );
    }

    /// The separating case: a binder that **is** used cannot be dropped. The
    /// codomains here are syntactically identical, so only the occurrence
    /// question tells the two types apart — which is what makes this the
    /// witness that the check is doing real work rather than passing by
    /// coincidence.
    #[test]
    fn dependent_pi_does_not_convert_with_the_plain_arrow()
    {
        let mut nbe = Normalizer::new();
        let dependent = dependent_pi(NameRef::from("a"));
        let CompType::Arrow { ref res, .. } = dependent
        else {
            panic!("the constructor built a function type")
        };
        let plain = CompType::arrow(ValueType::integer(), res.as_ref().clone());
        assert!(
            !bool::from(nbe.comp_type_converts(&dependent, &plain)),
            "a Pi binding a variable its codomain uses must not convert with the plain arrow"
        );
        assert!(
            !bool::from(nbe.comp_type_converts(&plain, &dependent)),
            "the refusal is not symmetric at a used binder"
        );
    }

    #[test]
    fn the_unknown_type_is_consistent_in_both_directions()
    {
        let mut nbe = Normalizer::new();
        assert!(bool::from(
            nbe.type_converts(&ValueType::Unknown, &ValueType::integer())
        ));
        assert!(bool::from(
            nbe.type_converts(&ValueType::integer(), &ValueType::Unknown)
        ));
    }

    #[test]
    fn a_term_converts_with_its_own_normal_form()
    {
        let mut nbe = Normalizer::new();
        for term in [
            thunk(Comp::app(
                Comp::lam("x", Comp::ret(Value::var(NameRef::from("x")))),
                Value::Int(3),
            )),
            record(&[(FieldName::from("a"), int(IntegerLiteral::from(1_i64)))]),
            thunk(Comp::RecordProj {
                record: record(&[(FieldName::from("a"), int(IntegerLiteral::from(1_i64)))]),
                label: String::from("a"),
            }),
        ] {
            let normal = nbe.normalize(&term).unwrap();
            assert!(
                bool::from(nbe.converts(&term, &normal)),
                "a term did not convert with its own normal form"
            );
        }
    }

    #[test]
    fn normalization_is_idempotent()
    {
        let mut nbe = Normalizer::new();
        let term = thunk(Comp::bind(
            Comp::app(
                Comp::lam("x", Comp::ret(Value::var(NameRef::from("x")))),
                Value::Int(3),
            ),
            "y",
            Comp::ret(Value::Pair(
                var(NameRef::from("y")),
                var(NameRef::from("y")),
            )),
        ));
        let once = nbe.normalize(&term).unwrap();
        let twice = nbe.normalize(&once).unwrap();
        assert_eq!(*once, *twice);
    }

    #[test]
    fn normalizing_leaves_the_arena_where_it_found_it()
    {
        let mut nbe = Normalizer::new();
        let term = thunk(Comp::app(
            Comp::lam("x", Comp::ret(Value::var(NameRef::from("x")))),
            Value::Int(3),
        ));
        // The syntax store keeps what it is given — it owns the canonical
        // syntax — but the SEMANTIC arena is where a run's scratch lives, and
        // that is what returns to its watermark.
        let before = nbe.watermark();
        let _normal = nbe.normalize(&term).unwrap();
        assert_eq!(
            nbe.watermark(),
            before,
            "normalization left semantic nodes behind"
        );
        let _decision = nbe.converts(&term, &term);
        assert_eq!(
            nbe.watermark(),
            before,
            "conversion left semantic nodes behind"
        );
    }

    #[test]
    fn the_arena_reports_its_own_population()
    {
        let mut nbe = Normalizer::new();
        let node = lower(&mut nbe, &int(IntegerLiteral::from(1_i64)));
        let before = usize::from(nbe.arena().value_count());
        let _evaluated = eval_value(&mut nbe, sem::SemArena::EMPTY_ENV, node).unwrap();
        assert!(usize::from(nbe.arena().value_count()) > before);
    }

    #[test]
    fn a_semantic_value_carries_the_guard_it_was_minted_with()
    {
        let mut nbe = Normalizer::new();
        let node = lower(&mut nbe, &int(IntegerLiteral::from(1_i64)));
        let guard = Guard::leaf(mix_word(
            seed(gandr_core_term::boundary::SemanticHash::from(1)),
            gandr_core_term::boundary::SemanticHash::from(2),
        ));
        let value = SemValue::new(
            SemValueNode::Rigid(Rigid::Free(String::from("x")), ValueUnfold::Rigid),
            guard,
        );
        assert_eq!(value.guard(), guard);
        assert!(value.face().retained().is_none());
        let retained = value.retaining(node);
        assert_eq!(retained.face().retained(), Some(node));
    }

    #[test]
    fn grades_separate_two_thunks_that_agree_on_their_bodies()
    {
        let mut nbe = Normalizer::new();
        let one = Rc::new(Value::Thunk(Grade::ONE, Rc::new(Comp::ret(Value::Unit))));
        let many = Rc::new(Value::Thunk(Grade::OMEGA, Rc::new(Comp::ret(Value::Unit))));
        let also_one = Rc::new(Value::Thunk(
            Grade::fin(GradeBound::from(1)),
            Rc::new(Comp::ret(Value::Unit)),
        ));
        assert!(!bool::from(nbe.converts(&one, &many)));
        assert!(bool::from(nbe.converts(&one, &also_one)));
    }

    #[test]
    fn a_free_variable_is_a_blocker_the_neutral_names()
    {
        let mut nbe = Normalizer::new();
        let node = lower(&mut nbe, &var(NameRef::from("stuck")));
        let evaluated = eval_value(&mut nbe, sem::SemArena::EMPTY_ENV, node).unwrap();
        let named = match *nbe.arena().value(evaluated).unwrap().node() {
            | SemValueNode::Rigid(Rigid::Free(ref name), _) => name.clone(),
            | _ => String::new(),
        };
        assert_eq!(named.as_str(), "stuck");
    }

    #[test]
    fn the_computation_entry_points_answer_for_computations_directly()
    {
        let mut nbe = Normalizer::new();
        // The value-level entries are convenience over the computation-level
        // ones; a caller holding weak-head normal computations compares and
        // reads them back without going through a thunk.
        let redex = thunk(Comp::app(
            Comp::lam("x", Comp::ret(Value::var(NameRef::from("x")))),
            Value::Int(3),
        ));
        let contractum = thunk(Comp::ret(Value::Int(3)));
        let left = lower(&mut nbe, &redex);
        let right = lower(&mut nbe, &contractum);
        let left = eval_value(&mut nbe, sem::SemArena::EMPTY_ENV, left).unwrap();
        let right = eval_value(&mut nbe, sem::SemArena::EMPTY_ENV, right).unwrap();
        assert!(bool::from(
            crate::conv::converts_values(&mut nbe, left, right).unwrap()
        ));
    }

    #[test]
    fn a_level_name_round_trips_through_its_parser()
    {
        for raw in [0_u32, 1, 7, u32::MAX] {
            let level = VariableLevel::from(raw);
            let rendered = crate::quote::level_name(level);
            assert_eq!(
                crate::quote::parse_level_name(NameRef::from(rendered.as_str())),
                Some(level)
            );
        }
    }

    #[test]
    fn a_parser_rejects_every_name_readback_cannot_produce()
    {
        for raw in [
            "x",
            "3",
            "\u{ab}3",
            "3\u{bb}",
            "\u{ab}x\u{bb}",
            "\u{ab}\u{bb}",
        ] {
            assert_eq!(
                crate::quote::parse_level_name(NameRef::from(raw)),
                None,
                "{raw} must not parse as a generated binder name"
            );
        }
    }

    #[test]
    fn the_next_level_watermark_is_the_level_a_draw_would_return()
    {
        let mut nbe = Normalizer::new();
        let watermark = nbe.next_level();
        assert_eq!(nbe.fresh_level(), watermark);
        assert_ne!(nbe.next_level(), watermark);
    }
    #[test]
    fn conversion_sink_preserves_verdict_and_emits_unfold()
    {
        let lhs = var(NameRef::from("one"));
        let rhs = var(NameRef::from("also_one"));
        let mut traced = Normalizer::new();
        traced
            .define(NameRef::from("one"), &int(IntegerLiteral::from(1_i64)))
            .unwrap();
        traced
            .define(NameRef::from("also_one"), &var(NameRef::from("one")))
            .unwrap();
        let mut sink = RecordingSink(Vec::new());
        let traced_verdict = traced.converts_with_sink(&lhs, &rhs, &mut sink);
        assert!(bool::from(traced_verdict));
        assert!(
            sink.0
                .iter()
                .any(|decision| matches!(decision, ConversionDecision::Unfold { .. })),
            "the sink receives a decision-grain unfolding"
        );

        let mut defaulted = Normalizer::new();
        defaulted
            .define(NameRef::from("one"), &int(IntegerLiteral::from(1_i64)))
            .unwrap();
        defaulted
            .define(NameRef::from("also_one"), &var(NameRef::from("one")))
            .unwrap();
        assert_eq!(traced_verdict, defaulted.converts(&lhs, &rhs));
        let mut null = NullSink;
        assert_eq!(
            traced_verdict,
            defaulted.converts_with_sink(&lhs, &rhs, &mut null),
        );
    }

    #[test]
    fn a_normalizer_can_be_built_over_an_existing_environment()
    {
        let mut nbe = Normalizer::new();
        nbe.define(NameRef::from("f"), &int(IntegerLiteral::from(5_i64)))
            .unwrap();
        assert!(bool::from(nbe.converts(
            &var(NameRef::from("f")),
            &int(IntegerLiteral::from(5_i64))
        )));
    }
}
