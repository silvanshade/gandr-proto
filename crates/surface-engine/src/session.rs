//! The REPL session engine
//! (`spec:implementation/incremental-pipeline.md` §"The
//! read-evaluate loop").
//!
//! The smallest end-to-end interactive slice: [`Session::submit`] takes one
//! line of source, lowers it totally ([`crate::lower::lower_source_total`]),
//! reports diagnostics and hole goals ([`crate::diag::report`]), and resumes
//! [`gandr_core_incremental`] at the submitted items' boundaries. The validated
//! resume reuses prior checkpoints where their terms and dependency footprints
//! remain valid and re-types the dirty frontier. Hole-free expressions then
//! evaluate on the [`gandr_core_sequent::machine`] L machine, returning the
//! structured [`Submission`] a front-end renders. The engine is
//! presentation-free (no `Display`; the core stays free of presentation
//! dependencies) so every frontend reuses the same session core.
//!
//! # Cross-line definitions
//!
//! Top-level items lower independently. [`Session`] retains their ordered
//! [`Program`] and the matching [`Checkpoints`], then appends each new
//! submission and calls [`resume_with`] against the surface prelude context.
//! The checkpoint engine is therefore the typing authority across lines:
//!
//! - **typing** comes from the resumed [`ItemTyping`] sequence. A bound
//!   definition's validated typing contributes its name and value type to the
//!   session context used by diagnostics for the next submission;
//! - **evaluation** runs each definition to a value once, at definition time
//!   (eager, but safe here: the pure spine is terminating and effect-free), and
//!   stores that value. A later expression folds the stored values into a
//!   `ret`-`Bind` chain ([`eval_chain`]), the same sequencing shape the lowerer
//!   gives a block's `val` / `run` statements, so the L machine extends its
//!   value environment with each stored definition before running the
//!   expression.
//!
//! Storing the evaluated value rather than the definition term keeps the chain
//! a sequence of `ret v >>= …`, which never re-runs a computation. An
//! unevaluable definition, for example one that reaches defined blame instead
//! of `ret`, is bound for typing but carries no stored value, so it cannot make
//! an unrelated later expression stuck. An evaluable definition runs exactly
//! once (REPL memoization).
//!
//! The item checkpoints are the persisted `Γ`-side state of
//! `spec:implementation/incremental-pipeline.md` §"The read-evaluate loop".
//! Only definitions with a **value type** are bindable as variables (CBPV: a
//! variable is a value). A definition of bare computation type, such as an
//! un-thunked `λ`, is reported with `bound = false` and left out of scope
//! (thunk it to name it), matching the lowerer's own `let` discipline.
//!
//! # What evaluates
//!
//! The pure CBPV spine (literals, pairs, lists, injections, `let`/bind,
//! forcing a thunk, `case`/`split` over closed scrutinees, lambdas), **plus**
//! the fixed-table operator builtins and module-qualified native builtins
//! resolved through the eval prelude ([`prelude_env`], module/prelude design):
//! `1 + 2` evaluates through the same native-builtin seam as `force(prim.id) 5`
//! and `list.push([1, 2], 3)`. The session surfaces a definition that does not
//! reduce to `ret` of a value — for example, a handler-less effect that reaches
//! defined blame or another non-value outcome — honestly as a type-only
//! definition, never as a stored value.
//!
//! # The kernel crossing
//!
//! A session also carries a [`KernelAdmissions`] ledger and offers every typed
//! definition to it in source order, so the surface language's own `def` items
//! are what drives [`gandr_core_checker::kernel_bridge`] and the kernel's
//! `add_decl` choke point. The ledger is where the definitions that lower into
//! the closed S1 vocabulary accumulate as a kernel environment, and each
//! submission reports one [`KernelVerdict`] per item beside its
//! [`ItemOutcome`]. [`Session::export_kernel_artifact`] is the storage tier's
//! shipping consumer: it exports that environment through
//! `gandr-storage-artifact` as a content-addressed artifact, so the manifest
//! identity of what a session admitted is available to the caller.
//!
//! **The crossing is observation, never authority.** Typing comes from the
//! resumed checkpoints and evaluation from the L machine, exactly as before: no
//! verdict feeds back into either, and a definition the kernel declines is
//! bound, evaluated, and reported precisely as it was. The kernel sits
//! downstream of the session, so what the ledger makes of a submission cannot
//! change that submission.
//!
//! [`KernelAdmissions`]: crate::kernel::KernelAdmissions
//! [`KernelVerdict`]: crate::kernel::KernelVerdict

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use gandr_core_checker::boundary::ConstructorTag;
use gandr_core_checker::ctx::Ctx;
use gandr_core_checker::error::TypeError;
use gandr_core_checker::outcome::Eval;
use gandr_core_checker::syntax::Comp;
use gandr_core_checker::syntax::Term;
use gandr_core_checker::syntax::Value;
use gandr_core_checker::types::DataId;
use gandr_core_checker::types::Ty;
use gandr_core_incremental::checkpoint::Checkpoints;
use gandr_core_incremental::checkpoint::ItemTyping;
use gandr_core_incremental::checkpoint::resume_with;
use gandr_core_incremental::persistence::BackendArtifact;
use gandr_core_incremental::persistence::CheckpointAddress;
use gandr_core_incremental::persistence::CheckpointObserver;
use gandr_core_incremental::persistence::CheckpointStore;
use gandr_core_incremental::persistence::CheckpointStoreError;
use gandr_core_incremental::persistence::persist;
use gandr_core_incremental::region::Item;
use gandr_core_incremental::region::Program;
use gandr_core_incremental::stream::SynthesisStream;
use gandr_core_sequent::machine::run_comp_with_prelude;
use gandr_storage_artifact::ArtifactError;
use gandr_storage_artifact::BuiltArtifact;
use gandr_storage_prolly_trees::BlockStore;
use gandr_storage_prolly_trees::TreeParams;

use crate::boundary::ConstructorName;
use crate::boundary::DefinitionBindingFlag;
use crate::boundary::DefinitionName;
use crate::boundary::PipelineSource;
use crate::boundary::SourceRange;
use crate::boundary::SubmissionIndex;
use crate::diag;
use crate::ffi::ForeignModule;
use crate::kernel::DefinitionOffer;
use crate::kernel::KernelAdmissions;
use crate::kernel::KernelVerdict;
use crate::kernel::WithheldReason;
use crate::live_match::MatchAnalysis;
use crate::live_match::RetainedSource;
use crate::live_match::SubmissionRecord;
use crate::live_match::liveness;
use crate::lower::ImportDeclaration;
use crate::lower::ImportIndex;
use crate::lower::LowerError;
use crate::lower::LoweringSeed;
use crate::lower::lower_source_total_seeded;
use crate::namespace::NamePath;
use crate::namespace::Scope;
use crate::prelude::Prelude;
use crate::prelude_ctx;
use crate::prelude_env;
use crate::recognition::Recognition;
use crate::recognition::ShadowPolicy;

/// Why a session checkpoint export failed.
#[derive(Debug, thiserror::Error)]
pub enum SessionPersistenceError
{
    /// The kernel environment could not be exported as a storage artifact.
    #[error("kernel artifact export failed")]
    Artifact(#[from] ArtifactError),
    /// The canonical checkpoint could not be persisted.
    #[error("checkpoint persistence failed: {0:?}")]
    Checkpoint(CheckpointStoreError),
}

/// The outcome of one lowered item in a [`Submission`], in source order.
///
/// Typing failures and hole goals are carried by the submission's
/// [`diag::Report`]; the variants here add the success information a front-end
/// needs: a definition's type and whether it entered scope, or an expression's
/// type and evaluation outcome. Markers for non-success items let a consumer
/// iterate items and the report in lock-step.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ItemOutcome
{
    /// A `def name = …` item that typed successfully.
    Definition
    {
        /// The defined name.
        name: String,
        /// The definition's type.
        ty: Ty,
        /// Whether the definition entered scope: `true` for a value-typed
        /// definition (bound into [`Session::ctx`] and the eval prelude),
        /// `false` for a bare-computation-typed one (reported but not bound;
        /// thunk it to name it).
        bound: bool,
    },
    /// An expression item that typed successfully, with its evaluation result.
    Expression
    {
        /// The expression's type (its terminal computation type, or the value
        /// type for a value item).
        ty: Ty,
        /// The L-machine outcome of evaluating the expression under the
        /// accumulated definitions (a terminal value, blame, or stuck result).
        value: Eval,
    },
    /// An item whose typing failed; the matching [`diag::Diagnostic`] in the
    /// submission's report carries the source-ranged detail.
    TypeError
    {
        /// The first typing error, as raised by the checker.
        error: TypeError,
    },
    /// An item carrying one or more holes: evaluation is declined (the
    /// parse-completeness validator) and the holes are listed as goals in the
    /// submission's report.
    Holey,
}

/// The structured result of one [`Session::submit`].
///
/// Pairs the [`diag::Report`] (diagnostics and hole goals, rendered by the
/// front-end) with one [`ItemOutcome`] per lowered item, in source order, and
/// one [`KernelVerdict`] per item beside it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Submission
{
    /// The diagnostics-and-goals report for the submitted source, typed against
    /// the session context as it stood at submission start.
    pub report: diag::Report,
    /// One outcome per lowered item, in source order (aligned with
    /// `report` by item index).
    pub outcomes: Vec<ItemOutcome>,
    /// What the kernel made of each lowered item, in the same source order and
    /// index-aligned with `outcomes`. Every item has an entry: one the kernel
    /// never saw carries [`KernelVerdict::Withheld`] naming why.
    pub kernel: Vec<KernelVerdict>,
    /// What the live pattern analysis concluded about each match expression in
    /// the submitted source, in lowering order.
    ///
    /// These are the elaboration's **obligations**, carried through to the
    /// caller unrendered: exhaustiveness and redundancy facts, per-branch
    /// liveness, and the pattern-position hole marks.
    ///
    /// Each analysis is addressed by its [`crate::live_match::MatchOrigin`] —
    /// the submission it was written in, the source item that owns it, and its
    /// position in that item. That is the same addressing the synthesis stream
    /// publishes, so a consumer reading both needs no translation between them,
    /// and neither has to know how many core items any source item lowered to.
    pub matches: Vec<MatchAnalysis>,
}

/// A REPL session: item-granular typing checkpoints and the definition values
/// that carry definitions across lines (see the module doc).
#[derive(Clone, Debug)]
pub struct Session
{
    /// The surface prelude context against which every checkpoint set starts.
    base_ctx: Ctx,
    /// The typing context after the current checkpoint set, used to report the
    /// next submission's diagnostics.
    ctx: Ctx,
    /// The complete ordered item program whose typing state is checkpointed.
    program: Program,
    /// One validated typing checkpoint per item in [`Self::program`].
    checkpoints: Checkpoints,
    /// The latest successful whole-program synthesis stream.
    latest_synthesis: Option<SynthesisStream>,
    /// The accumulated definition values, in definition order, each the
    /// once-evaluated result of a value-typed `def` ([`Session::define`]).
    defs: Vec<(String, Value)>,
    /// The eval-side prelude binding environment: module-qualified native
    /// builtins consumed by the L machine's prelude focus.
    prelude: Prelude,
    /// The `extern`-declared foreign modules accumulated across submissions,
    /// keyed by namespace (proposal-ffi.md §2).
    foreign: BTreeMap<String, ForeignModule>,
    /// The `codata`-declared observation shapes accumulated across submissions,
    /// keyed by codata type name (codata design §2).
    codata: BTreeMap<String, crate::lower::codata::CodataDecl>,
    /// The `data`-declared datatype shapes accumulated across submissions,
    /// keyed by datatype name (declared-data design Decision 4).
    data: BTreeMap<String, crate::lower::data::DataDecl>,
    /// Import declarations accumulated across submissions, in the same global
    /// source order named by [`ImportIndex`].
    imports: Vec<ImportDeclaration>,
    /// The persistent visible import namespace. Its bindings index
    /// [`Self::imports`].
    import_scope: Scope<ImportIndex, SourceRange>,
    /// The persistent outermost recognition scope: the prelude, host, and
    /// `extern` names plus every declaration earlier submissions bound over
    /// them, so a `def list = 1;` on one line still shadows the prelude `list`
    /// on the next.
    recognition: Recognition,
    /// How a declaration shadowing a prelude or host name is settled for this
    /// session's submissions.
    shadow_policy: ShadowPolicy,
    /// The kernel environment the session's S1-eligible definitions accumulate
    /// in, and the naming environment that resolves one to another.
    kernel: KernelAdmissions,
    /// One entry per submission, in submission order: the source-side match
    /// records the live pattern analysis is recomputed from, or the marker
    /// standing where a submission's records were never retained.
    ///
    /// **This is the only state the analysis needs, and it lives here rather
    /// than in the core.** A checkpoint keeps a lowered term, and patterns are
    /// not in a lowered term — so without this a match submitted on an earlier
    /// line would silently stop being reported the moment a later line arrived.
    /// Keeping it session-side is what leaves checkpoints, their footprints,
    /// and their equality exactly as they were.
    sources: Vec<RetainedSource>,
}

impl Session
{
    /// Opens a fresh session: the surface prelude and an empty checkpoint set.
    ///
    /// # Contract
    /// - ensures: the returned session types against the prelude operators and
    ///   carries no source items, checkpoints, definitions, or source match
    ///   records.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn new() -> Self
    {
        let base_ctx = prelude_ctx();
        Self {
            ctx: base_ctx.clone(),
            base_ctx,
            program: Program::default(),
            checkpoints: Checkpoints::default(),
            latest_synthesis: None,
            defs: Vec::new(),
            prelude: prelude_env(),
            foreign: BTreeMap::new(),
            codata: BTreeMap::new(),
            data: BTreeMap::new(),
            imports: Vec::new(),
            import_scope: Scope::new(),
            recognition: Recognition::new(ShadowPolicy::WarnAndAllow),
            shadow_policy: ShadowPolicy::WarnAndAllow,
            kernel: KernelAdmissions::new(),
            sources: Vec::new(),
        }
    }

    /// Sets how a declaration shadowing a prelude or host name is settled for
    /// every later submission.
    ///
    /// # Contract
    /// - ensures: later submissions lower under `policy`; already-submitted
    ///   sources are untouched.
    /// - provides: the reject-under-policy face of the recognition graduation,
    ///   for an embedding that treats a shadowed builtin as an error.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — one decision surface (which policy the next
    ///   submission takes), separated by submitting the same shadowing source
    ///   under each policy and asserting the outcome exactly.
    /// - witness: `gandr-surface-engine` `tests/recognition.rs` —
    ///   `a_session_rejects_a_shadowed_builtin_under_policy`
    #[inline]
    pub const fn set_shadow_policy(
        &mut self,
        policy: ShadowPolicy,
    )
    {
        self.shadow_policy = policy;
    }

    /// Opens a session on core state restored from a persisted checkpoint
    /// artifact, with **no** source match records.
    ///
    /// This is the record-stripped resume, and what it cannot do is the point.
    /// A checkpoint artifact carries lowered terms and their typings; the
    /// patterns the programmer wrote were never in the core to be persisted. So
    /// the restored program's matches are unrecoverable, and the session says
    /// exactly that: everything restored is recorded as one prior submission
    /// whose source was not retained, and the synthesis stream names it with
    /// [`gandr_core_incremental::stream::SynthesisEvent::SourceNotRetained`].
    /// One marker rather than a reconstructed count is deliberate — the
    /// artifact carries no submission structure, and inventing one would be a
    /// fiction a consumer could not tell from a fact.
    ///
    /// # Contract
    /// - requires: `checkpoints` validated `program`, as
    ///   [`gandr_core_incremental::persistence::restore`] returns them
    ///   together.
    /// - ensures: the typing context is rebuilt from the restored typings, so a
    ///   value-typed bound definition is in scope for the next submission by
    ///   name and type.
    /// - ensures: the restored program is one prior submission reported as
    ///   `SourceNotRetained`, and the next submission is numbered after it and
    ///   analyzed normally.
    /// - ensures: what the artifact does not carry is not invented. Stored
    ///   definition **values** are absent, so a restored definition is bound
    ///   for typing and evaluates nothing — the same standing an unevaluable
    ///   definition already has. The surface declaration registries (`data`,
    ///   `codata`, imports, `extern` modules) are absent too, so a submission
    ///   naming an earlier datatype must redeclare it.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L2 — the distinction that matters is between "no matches
    ///   to report" and "the records are gone", and only a resumed session can
    ///   exhibit the second; a fixture that never resumes cannot tell an
    ///   implementation that omits the marker from one that publishes it.
    /// - witness: `live_match::tests::a_record_stripped_resume_reports_source_not_retained`
    #[inline]
    #[must_use]
    pub fn resume_from_core(
        program: Program,
        checkpoints: Checkpoints,
    ) -> Self
    {
        let mut session = Self::new();
        for typing in checkpoints.items.iter().map(|item| &item.typing) {
            if let ItemTyping::Definition {
                ref name,
                ty: Ty::Value(ref value_type),
                bound: true,
            } = *typing
            {
                session.ctx.bind(name.clone(), value_type.clone());
            }
        }
        session.program = program;
        session.checkpoints = checkpoints;
        session.sources = alloc::vec![RetainedSource::SourceNotRetained {
            submission: SubmissionIndex::from(0_usize),
        }];
        session
    }

    /// The kernel admissions this session has accumulated.
    ///
    /// # Contract
    /// - ensures: returns the ledger holding the kernel environment every
    ///   [`KernelVerdict::Admitted`] definition entered, in admission order.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn kernel(&self) -> &KernelAdmissions
    {
        &self.kernel
    }

    /// Exports the session's kernel environment through
    /// `gandr-storage-artifact` as a content-addressed artifact, minting its
    /// manifest identity.
    ///
    /// # Contract
    /// - requires: `params` is a supported prolly-tree parameter set.
    /// - ensures: `Ok(built)` carries every tree node inserted into `store`
    ///   (verified on insert) and the canonical manifest binding the chunker
    ///   parameter commitment, the record count, the root node hash, and the
    ///   inner kernel export format version, with [`BuiltArtifact::identity`]
    ///   the BLAKE3 digest of that manifest. The build is a deterministic
    ///   function of the admitted record set, so two sessions that admitted the
    ///   same definitions mint the same identity.
    /// - provides: the shipping session export path: the outer
    ///   content-addressed identity of the environment this session's admitted
    ///   definitions accumulated. The identity authenticates the bytes;
    ///   validity is re-derived at replay from the canonical inner encoding,
    ///   never from the hash.
    /// - fails: [`ArtifactError`], as [`gandr_storage_artifact::build`].
    /// - panics: none.
    ///
    /// # Errors
    /// [`ArtifactError`].
    ///
    /// # Adequacy
    /// - hypothesis: L2 — the session-level differentials pin that this path
    ///   exports the real environment, equal environments mint equal
    ///   identities, and changed content changes the identity.
    /// - witness: `export::tests::the_session_export_path_exports_the_admitted_environment`
    /// - witness: `export::tests::independently_built_sessions_mint_byte_equal_identities`
    /// - witness: `export::tests::changed_environment_content_changes_the_identity`
    /// - witness: `export::tests::an_empty_session_exports_an_empty_record_set`
    #[inline]
    pub fn export_kernel_artifact<S>(
        &self,
        params: TreeParams,
        store: &mut S,
    ) -> Result<BuiltArtifact, ArtifactError>
    where
        S: BlockStore + ?Sized,
    {
        gandr_storage_artifact::build_from_environment(self.kernel.environment(), params, store)
    }

    /// Exports the session environment and persists its validated checkpoints.
    ///
    /// # Contract
    /// - requires: `params` is supported by the artifact tree, and both stores
    ///   remain available for the duration of the operation.
    /// - ensures: the artifact is built through
    ///   [`Self::export_kernel_artifact`] and the checkpoints are stored under
    ///   the program address and the artifact manifest identity.
    /// - provides: the built artifact and checkpoint address for a later call
    ///   to [`gandr_core_incremental::persistence::restore`].
    /// - fails: returns [`SessionPersistenceError::Artifact`] when artifact
    ///   construction fails, or [`SessionPersistenceError::Checkpoint`] when
    ///   checkpoint encoding, verification, or storage fails.
    /// - panics: none.
    ///
    /// # Errors
    /// [`SessionPersistenceError`].
    ///
    /// # Adequacy
    /// - hypothesis: L3 — the production path preserves the canonical manifest
    ///   identity, durable checkpoint encoding, and explicit codec rejection
    ///   boundary as one operation.
    /// - witness: `export::tests::session_persists_and_reopens_file_checkpoint`
    /// - witness:
    ///   `gandr_core_incremental::persistence::tests::nested_process_local_and_opaque_forms_report_exact_errors`
    #[inline]
    pub fn persist_kernel_checkpoint<B, S, O>(
        &self,
        params: TreeParams,
        artifact_store: &mut B,
        checkpoint_store: &mut S,
        observer: &mut O,
    ) -> Result<(BuiltArtifact, CheckpointAddress), SessionPersistenceError>
    where
        B: BlockStore + ?Sized,
        S: CheckpointStore,
        O: CheckpointObserver,
    {
        let artifact = self.export_kernel_artifact(params, artifact_store)?;
        let manifest = artifact.manifest().encode();
        let backend = BackendArtifact::from_bytes(manifest.as_ref());
        let address = persist(
            checkpoint_store,
            &self.program,
            backend,
            self.checkpoints.clone(),
            observer,
        )
        .map_err(SessionPersistenceError::Checkpoint)?;
        Ok((artifact, address))
    }

    /// The declared constructor name for a value `Ctor { id, tag, … }` — the
    /// decl-table lookup a front-end renderer uses to print `Some(3)` / `Red`
    /// rather than the structural carrier (declared-data design stage d). The
    /// datatype is keyed by the `id`'s name; the constructor is its
    /// `tag`-th.
    ///
    /// # Contract
    /// - ensures: `Some(name)` when the datatype is registered and `tag` is in
    ///   range; `None` otherwise (an unknown / stale id — the renderer then
    ///   falls back to a structural rendering).
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn constructor_name<T>(
        &self,
        id: &DataId,
        tag: T,
    ) -> Option<ConstructorName<'_>>
    where
        T: Into<ConstructorTag>,
    {
        let tag = usize::from(tag.into());
        self.data
            .get(id.name().as_ref())
            .filter(|decl| decl.id == *id)
            .and_then(|decl| decl.ctors.get(tag))
            .map(ConstructorName::from)
    }

    /// The current typing context (prelude plus bound definitions).
    ///
    /// # Contract
    /// - ensures: returns the context an expression submitted now would type
    ///   against.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn ctx(&self) -> &Ctx
    {
        &self.ctx
    }

    /// The ordered item program whose typing state this session checkpoints.
    ///
    /// # Contract
    /// - ensures: returns every item submitted so far, in program order — the
    ///   same program [`Session::persist_kernel_checkpoint`] stores under and
    ///   [`gandr_core_incremental::persistence::restore`] validates against.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn program(&self) -> &Program
    {
        &self.program
    }

    /// The validated typing checkpoints, one per item of [`Session::program`].
    ///
    /// # Contract
    /// - ensures: returns the checkpoint set the next resume starts from.
    /// - provides: the core state a caller persists and later reopens through
    ///   [`Session::resume_from_core`] — core state only, carrying no source
    ///   match records, which is why reopening reports them as not retained.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn checkpoints(&self) -> &Checkpoints
    {
        &self.checkpoints
    }

    /// Resolves a persistent import path to the declaration that introduced
    /// its binding.
    ///
    /// # Contract
    /// - ensures: returns the declaration indexed by the visible namespace
    ///   binding at `path`.
    /// - ensures: returns `None` when `path` is unbound.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn resolve_import(
        &self,
        path: &NamePath,
    ) -> Option<&ImportDeclaration>
    {
        let index = self.import_scope.resolve(path)?.data.0;
        self.imports.get(index)
    }
    /// Returns an owned stream over the latest successful whole-program typing.
    ///
    /// # Contract
    /// - ensures: returns a fresh stream cursor over the latest successful
    ///   submission's `Started`, source-ordered `Item`, and `Completed` events;
    ///   a failed submission leaves that stream unchanged.
    /// - provides: caller-owned incremental synthesis events without rechecking
    ///   or borrowing the session.
    /// - panics: none.
    /// - intension: clones the single stream retained at the successful submit
    ///   boundary; it performs no lowering or typing work.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise event assertions over the first revision, an
    ///   appended revision, and an injected infrastructure failure distinguish
    ///   missing boundaries, reordered items, wrong adoption flags, and failure
    ///   replacement.
    /// - witness: `tests::session::successful_submissions_publish_whole_program_synthesis`
    /// - witness: `session::tests::failed_submission_retains_latest_synthesis`
    /// - witness: `live_match::tests::the_stream_publishes_per_branch_status`
    /// - witness: `live_match::tests::adoption_does_not_move_a_verdict`
    #[inline]
    #[must_use]
    pub fn latest_synthesis_stream(&self) -> Option<SynthesisStream>
    {
        self.latest_synthesis.clone()
    }

    /// Lowers, reports, incrementally types, and evaluates one line of source.
    ///
    /// # Contract
    /// - ensures: on success returns one [`ItemOutcome`] and one
    ///   [`KernelVerdict`] per newly lowered item, in source order, and retains
    ///   one validated checkpoint per item seen across the whole session;
    ///   value-typed definitions enter the diagnostic and evaluation
    ///   environments before later items are processed, and every typed
    ///   definition is offered to the session's kernel ledger after its own
    ///   outcome is produced.
    /// - provides: the smallest end-to-end interactive slice (read → lower →
    ///   report → validated resume → eval → result), and the engine's crossing
    ///   from checked core into the certified kernel.
    /// - fails: only the infrastructure lowering failures
    ///   ([`LowerError::ParserUnavailable`] / [`LowerError::ParseFailed`]);
    ///   every parseable input lowers totally, with out-of-fragment regions
    ///   becoming holes.
    /// - panics: none; evaluation runs on the environment-backed L machine, and
    ///   a hole, blame, or ill-typed redex surfaces as a non-terminal [`Eval`].
    ///
    /// # Errors
    ///
    /// Returns the lowering [`LowerError`] for an infrastructure failure.
    ///
    /// # Adequacy
    /// - hypothesis: L2 agreement — the session's incremental typing sequence
    ///   must equal a from-scratch typing of the same accumulated program while
    ///   preserving cross-line evaluation. The session differential and the
    ///   core checkpoint differential distinguish stale adoption, missing
    ///   checkpoints, and a return to detached per-item typing.
    /// - witness: `tests::session::checkpointed_session_matches_from_scratch`
    /// - witness: `tests::session::successful_submissions_publish_whole_program_synthesis`
    /// - witness: `session::tests::failed_submission_retains_latest_synthesis`
    /// - witness: `gandr_core_incremental::tests::incremental`
    /// - witness: `tests::kernel::every_item_carries_exactly_one_verdict`
    /// - witness: `tests::kernel::a_later_definition_refers_to_an_earlier_one`
    /// - witness: `live_match::tests::an_inexhaustive_match_does_not_block_its_siblings`
    /// - witness: `live_match::tests::the_submission_and_the_stream_agree_on_the_item`
    #[inline]
    pub fn submit<'source, S>(
        &mut self,
        source: S,
    ) -> Result<Submission, LowerError>
    where
        S: Into<PipelineSource<'source>>,
    {
        let lowered = lower_source_total_seeded(source.into(), LoweringSeed {
            foreign: &self.foreign,
            codata: &self.codata,
            data: &self.data,
            import_scope: &self.import_scope,
            import_index_base: ImportIndex(self.imports.len()),
            recognition: Some(&self.recognition),
            shadow_policy: self.shadow_policy,
        });
        self.finish_submission(lowered)
    }

    /// Completes one attempted lowering as an atomic session submission.
    ///
    /// # Contract
    /// - ensures: an error returns before any session state changes; success
    ///   installs exactly one latest whole-program synthesis stream after the
    ///   existing outcome, checkpoint, declaration, import, and kernel updates.
    /// - provides: the transaction boundary between infrastructure lowering and
    ///   session mutation.
    /// - fails: returns the supplied lowering error unchanged.
    /// - panics: none.
    ///
    /// # Errors
    ///
    /// Returns the infrastructure [`LowerError`] supplied by lowering.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise comparison of every retained event before and
    ///   after an injected error kills mutation-before-error and
    ///   stream-clearing variants.
    /// - witness: `session::tests::failed_submission_retains_latest_synthesis`
    /// - witness: `live_match::tests::a_prior_submissions_matches_survive_the_next_one`
    #[inline]
    fn finish_submission(
        &mut self,
        lowered: Result<crate::lower::Lowered, LowerError>,
    ) -> Result<Submission, LowerError>
    {
        let mut lowered = lowered?;
        let report = diag::report(&lowered, &self.ctx);

        // Retain this submission's source records, and analyze it as one of
        // them. The submission's number is its position in the retained
        // sequence — a count of submissions, never of items — so a source item
        // that lowers to several core items moves nothing here.
        //
        // The lowering error above is this method's only failure, so recording
        // now still leaves the session unchanged on every path that returns
        // one.
        let record = SubmissionRecord {
            submission: SubmissionIndex::from(self.sources.len()),
            sites: core::mem::take(&mut lowered.match_sites),
            table: core::mem::take(&mut lowered.constructors),
        };
        let matches = record.analyze();
        self.sources.push(RetainedSource::Records(record));

        let prior_item_count = self.program.items.len();
        let mut edited = self.program.clone();
        edited.items.extend(lowered.items.iter().cloned());
        let resumed = resume_with(&self.checkpoints, &edited, &self.base_ctx);
        // Whole-program liveness: this submission's analyses merged with every
        // earlier submission's, recomputed from the retained records rather
        // than accumulated, and keyed by origin so each recorded match
        // publishes exactly one entry.
        let synthesis =
            SynthesisStream::from_resume_with_liveness(&resumed, &liveness(&self.sources));

        let mut outcomes = Vec::with_capacity(lowered.items.len());
        let mut verdicts = Vec::with_capacity(lowered.items.len());
        for (item, typing) in lowered
            .items
            .iter()
            .zip(resumed.typings().skip(prior_item_count))
        {
            let (outcome, verdict) = self.process_item(item, typing);
            outcomes.push(outcome);
            verdicts.push(verdict);
        }
        debug_assert_eq!(
            outcomes.len(),
            lowered.items.len(),
            "resume yields one typing and outcome per appended item"
        );
        debug_assert_eq!(
            verdicts.len(),
            outcomes.len(),
            "every item's outcome is paired with exactly one kernel verdict"
        );
        self.program = edited;
        self.checkpoints = resumed.into_checkpoints();

        // Declaration tables become visible only to later submissions, after
        // this submission's outcomes have consumed the prior tables.
        for (name, decl) in lowered.codata() {
            self.codata.insert(name.clone(), decl.clone());
        }
        for (name, decl) in lowered.data() {
            self.data.insert(name.clone(), decl.clone());
        }
        self.imports.extend(lowered.imports.iter().cloned());
        self.import_scope = lowered.import_scope().clone();
        self.recognition = lowered.recognition().clone();
        for module in lowered.foreign {
            self.foreign.insert(module.name.clone(), module);
        }
        self.latest_synthesis = Some(synthesis);
        Ok(Submission {
            report,
            outcomes,
            kernel: verdicts,
            matches,
        })
    }

    /// Projects one engine-owned typing into the session's evaluation outcome
    /// and the kernel's verdict on the same item.
    ///
    /// The two are independent: the outcome is produced first and the kernel is
    /// offered afterwards, so nothing the kernel decides can reach the typing,
    /// the binding, or the evaluation of the item it decided about.
    fn process_item(
        &mut self,
        item: &Item,
        typing: &ItemTyping,
    ) -> (ItemOutcome, KernelVerdict)
    {
        match *typing {
            | ItemTyping::Definition {
                ref name,
                ref ty,
                bound,
            } => {
                let outcome = self.define(name, item, ty.clone(), bound.into());
                let verdict = self.kernel.offer(DefinitionOffer {
                    name: DefinitionName::from(name),
                    term: &item.term,
                    ty,
                });
                (outcome, verdict)
            },
            | ItemTyping::Expression { ref ty } => (
                ItemOutcome::Expression {
                    value: run_comp_with_prelude(
                        &eval_chain(&self.defs, &item.term),
                        self.prelude.as_bindings(),
                    ),
                    ty: ty.clone(),
                },
                KernelVerdict::Withheld {
                    reason: WithheldReason::Expression,
                },
            ),
            | ItemTyping::TypeError { ref error } => (
                ItemOutcome::TypeError {
                    error: error.clone(),
                },
                KernelVerdict::Withheld {
                    reason: WithheldReason::Untyped,
                },
            ),
            | ItemTyping::Holey => (ItemOutcome::Holey, KernelVerdict::Withheld {
                reason: WithheldReason::Holey,
            }),
        }
    }

    /// Records a successfully-typed definition and, when bound, its value.
    fn define<'name>(
        &mut self,
        name: impl Into<DefinitionName<'name>>,
        item: &Item,
        ty: Ty,
        bound: DefinitionBindingFlag,
    ) -> ItemOutcome
    {
        let name = name.into();
        if bool::from(bound)
            && let Ty::Value(ref value_type) = ty
        {
            // Evaluate the definition once under the definitions already in
            // scope. A non-value terminal contributes no eval binding and
            // cannot poison an unrelated later expression.
            if let Eval::Value(Comp::Ret(value)) = run_comp_with_prelude(
                &eval_chain(&self.defs, &item.term),
                self.prelude.as_bindings(),
            ) {
                self.defs.push((name.0.to_owned(), (*value).clone()));
            }
            self.ctx.bind(name.0.to_owned(), value_type.clone());
        }
        ItemOutcome::Definition {
            name: name.0.to_owned(),
            ty,
            bound: bound.into(),
        }
    }
}

impl Default for Session
{
    #[inline]
    fn default() -> Self
    {
        Self::new()
    }
}

/// Folds the stored definition values into a `ret`-`Bind` chain around `expr`,
/// so the L machine extends its value environment with each stored definition
/// before running the expression.
///
/// Each definition `name = v` folds as `ret v >>= name. rest`, the earliest
/// outermost — so a later definition's value may mention an earlier name. Every
/// bound computation is a `ret v` (never a redex), so the chain re-runs no
/// definition and an unrelated stuck definition can never appear in it.
fn eval_chain(
    defs: &[(String, Value)],
    expr: &Term,
) -> Comp
{
    let mut body = term_into_comp(expr);
    for entry in defs.iter().rev() {
        body = Comp::bind(Comp::ret(entry.1.clone()), &entry.0, body);
    }
    body
}

/// Coerces a [`Term`] into the computation the L machine drives: a value `v`
/// becomes `ret v`, while a computation passes through.
fn term_into_comp(term: &Term) -> Comp
{
    match *term {
        | Term::Value(ref value) => Comp::ret(value.clone()),
        | Term::Comp(ref comp) => comp.clone(),
    }
}
#[cfg(test)]
mod tests
{
    use gandr_core_incremental::stream::SynthesisEvent;

    use super::*;

    /// An infrastructure failure cannot clear or replace the last successful
    /// stream retained by the transaction boundary.
    #[test]
    fn failed_submission_retains_latest_synthesis()
    {
        let mut session = Session::new();
        let _submission = session
            .submit("def retained = 40")
            .expect("fixture lowering must succeed");
        let before = session
            .latest_synthesis_stream()
            .expect("successful submission publishes a stream")
            .collect::<Vec<SynthesisEvent>>();

        let failed = session.finish_submission(Err(LowerError::ParseFailed));
        assert_eq!(
            failed,
            Err(LowerError::ParseFailed),
            "the infrastructure failure is returned unchanged"
        );
        let after = session
            .latest_synthesis_stream()
            .expect("failed submission retains the prior stream")
            .collect::<Vec<SynthesisEvent>>();
        assert_eq!(
            after, before,
            "failed submission preserves every prior event and adoption decision"
        );
    }
}
