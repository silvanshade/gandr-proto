#![expect(
    clippy::result_large_err,
    reason = "TypeError retains full types for diagnostics across machine boundaries."
)]

//! The defunctionalized typing machine (`spec:implementation/typing-machine.md`
//! §"Machine state" through §"The step function", core subset).
//!
//! Derived from [`gandr_core_checker::judgements::checker`] by the functional
//! correspondence: CPS transform the recursive checker, then defunctionalize
//! the continuations. Each [`Frame`] constructor is the defunctionalized image
//! of one pending recursive call site; the stack *is* the continuation (frames
//! carry no continuation pointers), and control is the explicit
//! `Descend`/`Return` register of `spec:implementation/typing-machine.md`
//! §"Control".
//!
//! Frame naming relative to the spec's inventory
//! (`spec:implementation/typing-machine.md` §"The frame inventory"), with the
//! `K` prefix dropped for Rust style:
//!
//! - `KAbs` → [`Frame::Abs`]; `KAppFn`/`KAppArg` → [`Frame::AppFn`] /
//!   [`Frame::AppArg`]; `KPairFst`/`KPairSnd`, `KThunk`, `KForce`, `KRet`,
//!   `KBind`, `KWith1`/`KWith2`, `KPrj`, `KAnnot`, `KSplit`, `KCaseScrut`
//!   likewise.
//! - `KBindBody`/`KSplitBody`/`KCaseArm1`/`KCaseArm2`/`KInj` →
//!   [`Frame::BindBody`] / [`Frame::SplitBody`] / [`Frame::CaseArm1`] /
//!   [`Frame::CaseArm2`] / [`Frame::Inj`]: the inventory corrections this
//!   derivation produced, adopted as ADR-27 decision 2 (the pre-A1 spec reused
//!   the session frame `KSessCont` for bind's context restore, stored an unused
//!   "branch-1 type" for case-arm sequencing, and omitted the checked
//!   injection's payload frame).
//! - Frames that complete a rule carry the originating [`Dir`], and the
//!   subsumption check runs at the frame pop (the spec §"The step function"
//!   `finish` notation) — exactly where the recursive checker's inlined Sub
//!   rule runs. Stage 1 has no solver, so the constraint is decided rather than
//!   emitted there; see ADR-27 decision 1 for the Stage 3+ emission semantics.
//!
//! # Error-path `Γ` contract
//!
//! `Γ` is **not** restored on error. A failing [`step`] returns
//! [`Outcome::Error`] carrying a [`FailureState`] whose `Γ` is the context as
//! it stood at the failure point (`spec:implementation/typing-machine.md`
//! §"Error handling": "the contexts at that point"). The recursive
//! [`gandr_core_checker::judgements::checker`], by contrast, unwinds `Γ` along
//! the host call stack as the error propagates — so the two `Γ`s differ on the
//! error path, which is why the conformance suite
//! compares [`gandr_core_term::error::TypeError`] values, never machine `Γ`.
//!
//! # Frame-pop ordering convention
//!
//! Frame-pop rules that both restore `Γ` and run a fallible check run the
//! **check before the unbind** ([`Frame::Abs`], [`Frame::CaseArm1`], and
//! [`Frame::BindBody`] — whose `combine_bind_row` + `finish_comp` row checks
//! precede its unbind — all follow this). Consequently `Γ` is never mutated
//! before a reachable error, so the `Γ` in [`FailureState`] equals the pre-pop
//! context for every reachable failure. (The unconditional restore frame
//! [`Frame::CaseArm2`] has no fallible check, so the ordering is moot for it.
//! The dependent-eliminator finish frames [`Frame::SplitBody`] (ADR-82) and
//! [`Frame::WalkBase`] (ADR-76) restore `Γ` and *then* finish a **precomputed**
//! answer against the direction — the subsumption there runs post-unbind, as it
//! does in the recursive checker, which likewise unwinds `Γ` before its
//! delivering `finish_comp`.)

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::rc::Rc;

use gandr_core_checker::discipline::subtype::finish_comp;
use gandr_core_checker::discipline::subtype::finish_int_literal;
use gandr_core_checker::discipline::subtype::finish_value;
use gandr_core_checker::discipline::subtype::pick;
use gandr_core_checker::judgements::checker::base_diagonal_type;
use gandr_core_checker::judgements::checker::inferred_binder;
use gandr_core_checker::judgements::checker::instantiate_codomain;
use gandr_core_checker::judgements::checker::motive_result_type;
use gandr_core_checker::judgements::checker::relocate_codomain;
use gandr_core_checker::judgements::checker::split_expectations;
use gandr_core_checker::judgements::checker::split_unknown_expectations;
use gandr_core_checker::judgements::control::Control;
use gandr_core_checker::judgements::control::Dir;
use gandr_core_checker::judgements::control::Trace;
use gandr_core_checker::judgements::stack::arrow_components;
use gandr_core_checker::judgements::stack::returner_components;
use gandr_core_checker::judgements::stack::stk_components;
use gandr_core_checker::judgements::stack::with_component;
use gandr_core_term::boundary::MachineStepCount;
use gandr_core_term::boundary::NameRef;
use gandr_core_term::boundary::StackDepth;
use gandr_core_term::ctx::Ctx;
use gandr_core_term::effect::EffectOp;
use gandr_core_term::effect::EffectRow;
use gandr_core_term::effect::combine_bind_row;
use gandr_core_term::effect::handle_natural_type;
use gandr_core_term::effect::resolve_handler_coverage;
use gandr_core_term::effect::resume_stack_type;
use gandr_core_term::error::TypeError;
use gandr_core_term::error::text;
use gandr_core_term::grade::Grade;
use gandr_core_term::syntax::Comp;
use gandr_core_term::syntax::FlatArena;
use gandr_core_term::syntax::OpClause;
use gandr_core_term::syntax::Side;
use gandr_core_term::syntax::Stack;
use gandr_core_term::syntax::Term;
use gandr_core_term::syntax::Value;
use gandr_core_term::types::CompType;
use gandr_core_term::types::Ty;
use gandr_core_term::types::ValueType;

/// A typing-stack frame: one pending obligation of the suspended derivation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Frame
{
    /// An abstraction's body is pending; yields `A → B` and unbinds.
    Abs
    {
        /// The bound variable's name (carried for the derivation UI, matching
        /// the spec's `KAbs(var, vty)`; [`Frame::Bind`], [`Frame::Split`], and
        /// [`Frame::CaseScrut`] likewise keep their binders).
        var: String,
        /// Whether the yielded function type quantifies, and over which name.
        ///
        /// On the checking side this is the expected type's binder relocated to
        /// `var`; on the inference side it is decided from the body's own type
        /// when the frame pops, so it is `None` here and filled there.
        binder: Option<String>,
        /// The bound argument type `A`.
        arg: ValueType,
        /// The direction the abstraction itself was typed in.
        dir: Dir<CompType>,
    },
    /// An application's head is pending; the argument is stored.
    AppFn
    {
        /// The stored argument value.
        arg: Value,
        /// The direction the application itself was typed in.
        dir: Dir<CompType>,
    },
    /// An application's argument check is pending; yields the result type.
    AppArg
    {
        /// The head's binder, for a dependent codomain.
        binder: Option<String>,
        /// The applied argument, which a dependent codomain is closed at.
        arg: Value,
        /// The function type's result `B`, **before** instantiation.
        result: CompType,
        /// The direction the application itself was typed in.
        dir: Dir<CompType>,
    },
    /// A pair's first component is pending; the second is stored.
    PairFst
    {
        /// The stored second component.
        second: Value,
        /// The direction for the second component.
        second_dir: Dir<ValueType>,
        /// The direction the pair itself was typed in.
        dir: Dir<ValueType>,
    },
    /// A pair's second component is pending; the first's type is stored.
    PairSnd
    {
        /// The first component's type.
        first: ValueType,
        /// The direction the pair itself was typed in.
        dir: Dir<ValueType>,
    },
    /// A checked injection's payload is pending; yields the stored sum.
    Inj
    {
        /// The expected sum type the injection checks against.
        sum: ValueType,
    },
    /// A declared-data constructor's payload is pending; yields the stored data
    /// type (ADR-80 Decision 2). The image of [`Frame::Inj`]: the nominal `id`
    /// check ran at the dispatch, so the pop just returns the result type.
    Ctor
    {
        /// The data type `Data { id, args }` (or `Unknown`) the constructor
        /// checks against.
        result: ValueType,
    },
    /// A packed module's payload is pending; yields the stored package type
    /// (rule Pack⇓). The image of [`Frame::Ctor`]: every check the rule makes —
    /// arity, the payload's grade, and the witness substitution — ran at the
    /// dispatch, so the pop just returns the result type.
    Pack
    {
        /// The package type (or `Unknown`) the pack checks against.
        result: ValueType,
    },
    /// An unpack's scrutinee is pending; the module binder, the type it binds
    /// at, the body, and the expectation are stored (rule Unpack⇓).
    ///
    /// The stored binding type is computed at the **dispatch** rather than
    /// here, because everything it depends on — the ascribed signature and
    /// the recorded atoms — is in the term, so nothing waits on the
    /// scrutinee's own type. That is the difference from [`Frame::Split`],
    /// whose binder types are only known once the scrutinee has been
    /// inferred.
    Unpack
    {
        /// The module variable bound over the body.
        binder: String,
        /// The type the module variable binds at — the payload with each
        /// abstract type component replaced by its minted atom.
        bound: ValueType,
        /// The stored body.
        body: Comp,
        /// The expectation, delivered verbatim once the body checks.
        expected: CompType,
    },
    /// An unpack's body is pending; the pop restores `Γ` and returns the stored
    /// expectation (rule Unpack⇓). The [`Frame::SplitBody`] discipline, minus
    /// the motive: an unpack has no motive by design, since a motive is what
    /// would let a minted atom escape into the answer.
    UnpackBody
    {
        /// The expectation the unpack delivers.
        result: CompType,
    },
    /// A checked list literal's current element is pending; the remaining
    /// elements and the result type are stored (rule List⇓; ADR-40 D3). Each
    /// element is descended in `elem_dir`; when none remain the stored `result`
    /// (`List A` for the list case, `Unknown` for the matched-hole case) is
    /// returned. The machine image of
    /// [`gandr_core_checker::judgements::checker`]'s `rule_list` loop, so
    /// the per-element `Descend`/`Return` sequence matches step for step.
    List
    {
        /// The elements still to type, in order.
        remaining: alloc::vec::Vec<Rc<Value>>,
        /// The direction each element is checked in (`Check(A)` or
        /// `Check(Unknown)`).
        elem_dir: Dir<ValueType>,
        /// The type returned once every element has typed (`List A` /
        /// `Unknown`).
        result: ValueType,
    },
    /// A record literal's fields are pending; each field is typed in its
    /// per-label direction and accumulated into `typed`, then `finish_value`
    /// finishes against `dir` (rule Record⇑/Record⇓; ADR-45 D3). The machine
    /// image of [`gandr_core_checker::judgements::checker`]'s `rule_record`
    /// loop, so the per-field `Descend`/`Return` sequence matches step for
    /// step. Fields are processed in the canonical (sorted-label) order of
    /// the `BTreeMap`.
    Record
    {
        /// The fields still to type, `(label, value)` in sorted-label order.
        remaining: alloc::vec::Vec<(String, Rc<Value>)>,
        /// The label of the field currently being typed.
        current_label: String,
        /// The fields typed so far, accumulating into the record type.
        typed: BTreeMap<String, Rc<ValueType>>,
        /// The direction the record itself was typed in (for the final
        /// `finish_value` width/depth subsumption).
        dir: Dir<ValueType>,
    },
    /// A thunk's body is pending; yields `U_grade B`.
    Thunk
    {
        /// The grade to rebuild with (the expectation's grade in checking
        /// mode, the annotation otherwise).
        grade: Grade,
        /// The direction the thunk itself was typed in.
        dir: Dir<ValueType>,
    },
    /// A pure-computation embedding's body is pending; the embedding delivers
    /// the returner's payload once the empty-row premise holds (rule Run).
    ///
    /// The body rides the frame so a decline names **the term the author
    /// wrote** rather than a placeholder; it is an `Rc` clone, so carrying it
    /// costs a refcount rather than the term.
    Run
    {
        /// The embedded computation, retained for the decline's diagnostic.
        body: Rc<Comp>,
        /// The direction the embedding itself was typed in.
        dir: Dir<ValueType>,
    },
    /// A forced value is pending; yields `B` from `U_r B` after `1 ⊑ r`.
    Force
    {
        /// The direction the forcing itself was typed in.
        dir: Dir<CompType>,
    },
    /// A record projection's record is pending; on return the field `label` is
    /// looked up in the inferred record type and `F A` is finished against
    /// `dir` (rule `RecordProj`⇑; ADR-45 D4). The image of [`Frame::Force`];
    /// the record value is **retained** so a missing-field stuck error carries
    /// the same term as [`gandr_core_checker::judgements::checker`]'s
    /// `rule_record_proj`.
    RecordProj
    {
        /// The record value being projected (retained for the stuck-error
        /// term; the descent types a clone, as the checker does).
        record: Rc<Value>,
        /// The projected field label `ℓ`.
        label: String,
        /// The direction the projection itself was typed in.
        dir: Dir<CompType>,
    },
    /// A `dup`'d value is pending; on return the conservation `r + s ⊑ g` is
    /// checked and `F (U_r B × U_s B)` is finished against the expectation
    /// (rule Dup, `spec:implementation/type-system.md` §"Grades").
    Dup
    {
        /// The first half's grade `r` (read from the expectation).
        r: Grade,
        /// The second half's grade `s` (read from the expectation).
        s: Grade,
        /// The direction the `dup` was typed in — always `Check(F (U_r B ×
        /// U_s B))`, the sole source of `r`/`s` (dup is check-only).
        dir: Dir<CompType>,
    },
    /// A `drop`'d value is pending; yields `F 1`, discarding the budget (rule
    /// Drop, `spec:implementation/type-system.md` §"Grades").
    Drop
    {
        /// The direction the `drop` itself was typed in.
        dir: Dir<CompType>,
    },
    /// A returner's payload is pending; yields `F A`.
    Ret
    {
        /// The direction the returner itself was typed in.
        dir: Dir<CompType>,
    },
    /// A bind's bound computation is pending; the continuation is stored.
    Bind
    {
        /// The variable receiving the produced value.
        var: String,
        /// The stored continuation.
        cont: Comp,
        /// The direction the bind itself was typed in.
        dir: Dir<CompType>,
    },
    /// A bind's continuation is pending; pop restores `Γ`, unions the bound
    /// computation's effect row into the result, and finishes against the
    /// bind's direction (A3.2 `+effects`, via [`combine_bind_row`] then
    /// [`finish_comp`] — the latter is the row-subsumption the union requires).
    BindBody
    {
        /// The bound computation's effect row `ε_bound`, folded into the
        /// continuation's returner at the pop.
        bound_row: EffectRow,
        /// The direction the bind itself was typed in — the union is finished
        /// against it, so a checking-mode bind subsumption-checks its row.
        dir: Dir<CompType>,
    },
    /// A case's scrutinee is pending; both arms are stored.
    CaseScrut
    {
        /// The stored first arm.
        arm_fst: (String, Rc<Comp>),
        /// The stored second arm.
        arm_snd: (String, Rc<Comp>),
        /// The expected type both arms check against.
        expected: CompType,
    },
    /// A case's first arm is pending; the second arm is stored.
    CaseArm1
    {
        /// The stored second arm.
        arm_snd: (String, Rc<Comp>),
        /// The right summand's type, bound for the second arm.
        snd_ty: ValueType,
        /// The expected type both arms check against.
        expected: CompType,
    },
    /// A case's second arm is pending; pop restores `Γ`.
    CaseArm2,
    /// A declared-data case's scrutinee is pending; the arms and expectation
    /// are stored (rule `DataCase`⇓; ADR-80 Decision 3). The
    /// `k`-constructor analogue of [`Frame::CaseScrut`]: on return the
    /// scrutinee type is shape-checked (`Data { … }` / `Unknown`) and the
    /// first arm is bound at `Unknown` and descended (an empty arm list
    /// returns the expectation).
    DataCaseScrut
    {
        /// The stored per-constructor arms, arm `i` handling tag `i`.
        arms: alloc::vec::Vec<(String, Rc<Comp>)>,
        /// The expected type every arm checks against.
        expected: CompType,
    },
    /// A declared-data case's current arm is pending; pop restores `Γ` (the
    /// arm's `Unknown` payload binder), then descends the next arm or returns
    /// the expectation (rule `DataCase`⇓; ADR-80). The `k`-arm analogue of the
    /// fixed [`Frame::CaseArm1`]/[`Frame::CaseArm2`] chain.
    DataCaseArm
    {
        /// The arms still to check, in order.
        remaining: alloc::vec::Vec<(String, Rc<Comp>)>,
        /// The expected type every arm checks against.
        expected: CompType,
    },
    /// A list-case's scrutinee is pending; the arms and binders are stored
    /// (rule `ListCase`⇓; ADR-40 D4). The list analogue of
    /// [`Frame::CaseScrut`].
    ListCaseScrut
    {
        /// The stored `nil` arm body.
        nil: Rc<Comp>,
        /// The `cons` arm's head binder.
        head: String,
        /// The `cons` arm's tail binder.
        tail: String,
        /// The stored `cons` arm body.
        cons: Rc<Comp>,
        /// The expected type both arms check against.
        expected: CompType,
    },
    /// A list-case's `nil` arm is pending; the `cons` arm and its binder types
    /// are stored (rule `ListCase`⇓). On return the `cons` binders are bound
    /// and its body descended.
    ListCaseNil
    {
        /// The `cons` arm's head binder.
        head: String,
        /// The element type `A`, bound to `head`.
        head_ty: ValueType,
        /// The `cons` arm's tail binder.
        tail: String,
        /// The list type `List A` (or `Unknown`), bound to `tail`.
        tail_ty: ValueType,
        /// The stored `cons` arm body.
        cons: Rc<Comp>,
        /// The expected type both arms check against.
        expected: CompType,
    },
    /// A list-case's `cons` arm is pending; pop restores `Γ` (both binders).
    ListCaseCons,
    /// A split's scrutinee is pending; the body, the motive, and the scrutinee
    /// value are stored (rules `SplitMotive`⇑ / Split⇓, ADR-82). On the
    /// scrutinee-pop the motive instantiation is computed — the body's
    /// checked-against type `M[(p, q)/z]` and the precomputed answer `M[v/z]`
    /// (the [`Frame::WalkScrut`] discipline: the answer is computed at the
    /// scrutinee-pop and does not depend on the body).
    Split
    {
        /// The binder for the first component `p`.
        fst_name: String,
        /// The binder for the second component `q`.
        snd_name: String,
        /// The optional dependent motive `(z). M` (ADR-82); `None` is the
        /// check-only motive-less split.
        motive: Option<Box<gandr_core_term::syntax::SplitMotive>>,
        /// The scrutinee value `v` (the motive's `z` is instantiated to it for
        /// the answer `M[v/z]`).
        scrut: Value,
        /// The stored body `t`.
        body: Comp,
        /// The direction the split itself was typed in.
        dir: Dir<CompType>,
    },
    /// A split's body is pending; the pop restores `Γ` (both binders) and
    /// finishes the **precomputed** answer against the direction (ADR-82 D5:
    /// the body-restore frame carries the answer — `M[v/z]` with a motive,
    /// the expectation without — rather than echoing the body type). The
    /// [`Frame::WalkBase`] discipline.
    SplitBody
    {
        /// The precomputed answer `M[v/z]` (motive) or the expectation `C`
        /// (motive-less check).
        result: CompType,
        /// The direction the split itself was typed in.
        dir: Dir<CompType>,
    },
    /// A lazy pair's first component is pending; the second is stored.
    With1
    {
        /// The stored second component.
        second: Comp,
        /// The expected type for the second component.
        second_expected: CompType,
    },
    /// A lazy pair's second component is pending; the first's type is stored.
    With2
    {
        /// The first component's type.
        first: CompType,
    },
    /// A projection's target is pending; yields the chosen conjunct.
    Prj
    {
        /// Which conjunct is projected.
        side: Side,
        /// The direction the projection itself was typed in.
        dir: Dir<CompType>,
    },
    /// An annotation's check is pending; yields the ascription.
    Annot
    {
        /// The direction the annotated value itself was typed in.
        dir: Dir<ValueType>,
    },
    /// A `perform`'s payload check is pending; yields the singleton-row
    /// returner `F^⟨E⟩ B_op` (rule Op, A3.2 `+effects`).
    Perform
    {
        /// The operation's reply type `B_op` (the produced value type).
        reply: ValueType,
        /// The singleton row `⟨E⟩` of the performed operation's signature.
        row: EffectRow,
        /// The direction the `perform` itself was typed in.
        dir: Dir<CompType>,
    },
    /// A handler's handled computation `t` is pending; on return its residual
    /// row is computed and the return clause is descended (rule Handle, A3.2
    /// `+effects`).
    HandleScrut
    {
        /// The handler's answer computation type `F^ε C` (or `Unknown` for the
        /// matched-hole answer); equals the `Dir::Check` target.
        answer: CompType,
        /// The handled signature's name, subtracted from `t`'s row to form the
        /// residual `ε_t ∖ E`.
        sig_name: String,
        /// The return clause `ret x ⇒ t_ret`.
        ret: (String, Rc<Comp>),
        /// The operation clauses resolved against the signature, in canonical
        /// operation order (matching the recursive checker's iteration).
        ops: Vec<(EffectOp, OpClause)>,
    },
    /// A handler's return clause `t_ret` is pending; on return `Γ` is restored
    /// and the first operation clause (if any) is descended (rule Handle).
    HandleRet
    {
        /// The handler's answer type (see [`Frame::HandleScrut`]).
        answer: CompType,
        /// The handled computation's residual row `ε_t ∖ E`.
        residual: EffectRow,
        /// The operation clauses still to check (all of them at this point).
        ops: Vec<(EffectOp, OpClause)>,
    },
    /// A handler's operation clause `t_i` is pending; on return its binders are
    /// restored and the next clause (or the final soundness finish) follows
    /// (rule Handle).
    HandleOp
    {
        /// The handler's answer type (see [`Frame::HandleScrut`]).
        answer: CompType,
        /// The handled computation's residual row `ε_t ∖ E`.
        residual: EffectRow,
        /// The operation clauses still to check *after* the pending one.
        rest: Vec<(EffectOp, OpClause)>,
    },
    /// A `resume`'s reified stack `v` is pending; the fed computation is stored
    /// (rule Resume, A3.3 `+control`). The image of [`Frame::AppFn`] with the
    /// sorts swapped — the "function" is the stack value, inferred first.
    ResumeFn
    {
        /// The computation `t` fed to the resumed stack.
        comp: Comp,
        /// The direction the `resume` itself was typed in.
        dir: Dir<CompType>,
    },
    /// A `resume`'s fed computation `t` is pending; the delivered answer is
    /// stored (rule Resume, A3.3 `+control`). The image of [`Frame::AppArg`].
    ResumeArg
    {
        /// The stack's delivered answer `C` (the resume's result).
        result: CompType,
        /// The direction the `resume` itself was typed in.
        dir: Dir<CompType>,
    },
    /// A `reset`'s body is pending; on return the saved ambient answer is
    /// restored (rule Reset, A3.3 `+control`). `reset` is check-only and
    /// transparent on the type, so the body's *checked* type is returned
    /// directly (a consistent subtype of the answer `C`, equal to `C` only when
    /// the body echoes it — the recursive checker's `rule_reset` returns the
    /// same body type, keeping the two faces lock-step).
    ResetBody
    {
        /// The ambient answer in force *before* this `reset`, restored at the
        /// pop (dynamic scoping; `None` if this `reset` was outermost).
        saved: Option<CompType>,
    },
    /// A `shift`'s body is pending; on return the continuation binder `k` is
    /// restored and the captured type `B` is delivered (rule Shift, A3.3
    /// `+control`).
    ShiftBody
    {
        /// The captured type `B` the `shift` delivers (the body is checked
        /// against the *answer* `C`, not `B`).
        captured: CompType,
    },
    /// A fixpoint's body is pending; the recursion's own computation type is
    /// stored, because the former delivers what it was checked against (rule
    /// Fix⇓).
    FixBody
    {
        /// The recursion's own computation type `B`.
        recursive: CompType,
    },
    /// A reified stack's argument-frame value `v` is pending; the rest of the
    /// stack and the type to continue the walk from are stored (rule Reify, the
    /// stack-judgment walk; A3.3 `+control`).
    StkArg
    {
        /// The rest of the stack, walked from `result_input` on return.
        rest: Stack,
        /// The consumed function type's binder, for a dependent codomain.
        binder: Option<String>,
        /// The argument the frame supplies, which a dependent codomain is
        /// closed at.
        arg: Value,
        /// The consumed function's result type — the input the walk continues
        /// from once the argument value has checked, **before**
        /// instantiation.
        result_input: CompType,
        /// The direction the `stk K` itself was typed in (the original
        /// `Check(Stk(B, C))` / `Check(Unknown)`), carried so the walk's final
        /// `ε` step can finish against it.
        dir: Dir<ValueType>,
    },
    /// A reified stack's bind-frame continuation `u` is pending; the rest of
    /// the stack and the consumed row are stored (rule Reify, the
    /// stack-judgment walk; A3.3 `+control`). On return the consumed row
    /// folds into the continuation's type (via [`combine_bind_row`], as
    /// [`Frame::BindBody`]) before the binder is restored and the walk
    /// continues.
    StkBind
    {
        /// The rest of the stack, walked from the sequenced type on return.
        rest: Stack,
        /// The consumed returner's row `ε`, folded into the continuation's
        /// result at the pop.
        consumed_row: EffectRow,
        /// The direction the `stk K` itself was typed in (see
        /// [`Frame::StkArg`]).
        dir: Dir<ValueType>,
    },
    /// A reflexivity proof's witness is pending (rule `Here`, ADR-76). The
    /// image of [`Frame::Inj`]: the witness value is descended in inference
    /// mode, and on return its inferred type `A` becomes the carrier of the
    /// natural type `Path A v v`, finished against the stored direction.
    Here
    {
        /// The witness value `v`, stored to form the endpoints of `Path A v v`.
        witness: Value,
        /// The direction the `here` itself was typed in.
        dir: Dir<ValueType>,
    },
    /// An identity eliminator's scrutinee is pending (rule `Walk`, ADR-76). The
    /// image of [`Frame::CaseScrut`]: the scrutinee is descended in inference
    /// mode, and on return its `Path A a b` shape drives the diagonal base
    /// check and the result-type instantiation.
    WalkScrut
    {
        /// The motive `(x y q). C`.
        motive: gandr_core_term::syntax::WalkMotive,
        /// The diagonal base `(x). c`.
        base: gandr_core_term::syntax::WalkBase,
        /// The scrutinee value `p` (the motive's `q` is instantiated to it).
        scrut: Value,
        /// The direction the `Walk` itself was typed in.
        dir: Dir<CompType>,
    },
    /// An identity eliminator's base body is pending (rule `Walk`, ADR-76). The
    /// result type was fully computed at the [`Frame::WalkScrut`] pop (it does
    /// not depend on the base's type), so this frame only sort-checks the
    /// base, restores `Γ`, and finishes the stored result against the
    /// direction.
    WalkBase
    {
        /// The precomputed result type `C[a/x][b/y][p/q]`.
        result: CompType,
        /// The direction the `Walk` itself was typed in.
        dir: Dir<CompType>,
    },
}

/// The complete machine state: inspectable, cloneable, resumable.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct State
{
    /// The control register.
    control: Control,
    /// The typing stack; the *last* element is the top.
    stack: Vec<Frame>,
    /// The two-zone typing context `Γ; Σ`.
    ctx: Ctx,
    /// The ambient answer type the nearest enclosing `reset` establishes
    /// (the effects and control record's answer-typing section; A3.3
    /// `+control`): `None` outside any
    /// `reset`. The `reset` rule's `step_comp` sets it and its
    /// [`Frame::ResetBody`] pop restores it (dynamic scoping); `shift` reads
    /// it. The mirror of the recursive checker's `answer` register; like
    /// `Γ`, it is not restored on the error path (the conformance suite
    /// compares [`gandr_core_term::error::TypeError`] values, never the
    /// register).
    answer: Option<CompType>,
    /// Monotone step counter.
    steps: u64,
}

impl State
{
    /// Initial state for typing a value.
    #[inline]
    #[must_use]
    pub fn new_value(
        ctx: Ctx,
        value: Value,
        dir: Dir<ValueType>,
    ) -> Self
    {
        Self {
            control: Control::DescendValue { value, dir },
            stack: Vec::new(),
            ctx,
            answer: None,
            steps: 0,
        }
    }

    /// Initial state for typing a computation.
    #[inline]
    #[must_use]
    pub fn new_comp(
        ctx: Ctx,
        comp: Comp,
        dir: Dir<CompType>,
    ) -> Self
    {
        Self {
            control: Control::DescendComp { comp, dir },
            stack: Vec::new(),
            ctx,
            answer: None,
            steps: 0,
        }
    }

    /// The current control register.
    #[inline]
    #[must_use]
    pub fn control(&self) -> &Control
    {
        &self.control
    }

    /// The current typing context `Γ`.
    ///
    /// Read-only inspection for step-driving consumers — the pipeline's
    /// goals report reads `Γ` at each hole's `Descend` state (A2.2), and the
    /// A2.4 diagnostics surface will read it for context chains.
    #[inline]
    #[must_use]
    pub fn ctx(&self) -> &Ctx
    {
        &self.ctx
    }

    /// The current stack depth.
    #[inline]
    #[must_use]
    pub fn depth(&self) -> StackDepth
    {
        self.stack.len().into()
    }

    /// The number of steps taken so far.
    #[inline]
    #[must_use]
    pub fn steps(&self) -> MachineStepCount
    {
        self.steps.into()
    }
}

/// The machine state captured at the point a step failed.
///
/// Per `spec:implementation/typing-machine.md` §"Error handling", "every error
/// carries the offending expr, the frame stack at failure (the partial
/// derivation), and the contexts at that point". This is the failure analogue
/// of [`State`]: it is the state on which [`step`] failed, so for every
/// *reachable* error it is reproducible — stepping it again raises the same
/// error. The control register holds the offending sub-term (`Descend`) or the
/// value propagating into the failing frame (`Return`); for a `Return` failure
/// the frame under which the failure occurred is restored to the top of
/// [`Self::stack`], so the stack is the complete partial derivation with the
/// failing frame at its top.
///
/// `Γ` error contract (machine module doc): `Γ` is *not* restored on error;
/// [`Self::ctx`] is `Γ` as it stood at the failure point. Frame-pop rules run
/// their fallible sort checks before any `Γ` mutation, so for every reachable
/// error `Γ` matches the pre-pop context.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FailureState
{
    /// The control register being processed when the step failed.
    control: Control,
    /// The frame stack at failure — the partial derivation. For a `Return`
    /// failure the failing frame is restored to the top.
    stack: Vec<Frame>,
    /// The typing context `Γ` at the failure point (not restored).
    ctx: Ctx,
}

impl FailureState
{
    /// The control register being processed when the step failed.
    #[inline]
    #[must_use]
    pub fn control(&self) -> &Control
    {
        &self.control
    }

    /// The frame stack at failure (the partial derivation); the *last* element
    /// is the top.
    #[inline]
    #[must_use]
    pub fn stack(&self) -> &[Frame]
    {
        &self.stack
    }

    /// The typing context `Γ` at the failure point.
    #[inline]
    #[must_use]
    pub fn ctx(&self) -> &Ctx
    {
        &self.ctx
    }
}

/// The result of one machine step (`spec:implementation/typing-machine.md`
/// §"The step function").
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Outcome
{
    /// The derivation is complete: control was `Return` on an empty stack.
    Done(
        /// The subject's type.
        Ty,
    ),
    /// One more state.
    Step(
        /// The successor state.
        State,
    ),
    /// The derivation failed: the error paired with the state at failure
    /// (`spec:implementation/typing-machine.md` §"Error handling").
    Error
    {
        /// The failure.
        error: TypeError,
        /// The machine state at the point of failure (the partial derivation,
        /// `Γ`, and the control register).
        state: FailureState,
    },
}

/// Runs a value state to completion, collecting the control trace.
///
/// # Contract
/// - ensures: drives `step` from the initial value state to a terminal outcome,
///   returning `(Ok(ty), trace)` on success (`ty` is `Ty::Value`-rooted) or
///   `(Err(error), trace)` on the first failure; the `Trace` records every
///   control register visited (equal, event for event, to
///   `gandr_core_checker::judgements::checker::run_value` on the same input).
/// - provides: the `Trace` is returned in both arms; on failure the
///   `FailureState` is dropped here — drive `step` directly to inspect it.
/// - fails: the result arm is `Err` with the first `TypeError`
///   (`UnboundVariable`, `TypeMismatch`, `ShapeMismatch`, `StuckExpr`, or
///   `GradeError`).
/// - panics: none; the frame stack lives on the heap, so adversarial-depth
///   terms do not overflow the host stack (unlike
///   `gandr_core_checker::judgements::checker`).
#[inline]
pub fn run_value(
    ctx: Ctx,
    value: Value,
    dir: Dir<ValueType>,
) -> (Result<Ty, TypeError>, Trace)
{
    run(State::new_value(ctx, value, dir))
}

/// Runs a computation state to completion, collecting the control trace.
///
/// # Contract
/// - ensures: drives `step` from the initial computation state to a terminal
///   outcome, returning `(Ok(ty), trace)` on success (`ty` is
///   `Ty::Comp`-rooted) or `(Err(error), trace)` on the first failure; the
///   `Trace` records every control register visited (equal, event for event, to
///   `gandr_core_checker::judgements::checker::run_comp` on the same input).
/// - provides: the `Trace` is returned in both arms; on failure the
///   `FailureState` is dropped here — drive `step` directly to inspect it.
/// - fails: the result arm is `Err` with the first `TypeError`
///   (`UnboundVariable`, `TypeMismatch`, `ShapeMismatch`, `StuckExpr`, or
///   `GradeError`).
/// - panics: none; the frame stack lives on the heap, so adversarial-depth
///   terms do not overflow the host stack (unlike
///   `gandr_core_checker::judgements::checker`).
#[inline]
pub fn run_comp(
    ctx: Ctx,
    comp: Comp,
    dir: Dir<CompType>,
) -> (Result<Ty, TypeError>, Trace)
{
    run(State::new_comp(ctx, comp, dir))
}

/// Runs a state to completion (`eager` mode,
/// `spec:implementation/typing-machine.md` §"Execution modes"), collecting
/// every control register passed through.
#[inline]
pub fn run(state: State) -> (Result<Ty, TypeError>, Trace)
{
    let mut trace = Trace::new();
    let mut current = state;
    loop {
        trace.push(current.control().clone());
        match step(current) {
            | Outcome::Step(next) => current = next,
            | Outcome::Done(ty) => return (Ok(ty), trace),
            // The conformance contract compares `TypeError` values (the checker
            // has no machine state); the failing state is dropped here and is
            // available to callers that drive [`step`] directly.
            | Outcome::Error { error, state: _ } => return (Err(error), trace),
        }
    }
}

/// Runs a state to completion like [`run`], additionally reporting the final
/// state's step counter and `Γ`.
///
/// # Contract
/// - ensures: drives `step` to a terminal outcome and returns the same control
///   `Trace` [`run`] would, together with the `steps` counter and the `Γ` of
///   the state that produced that outcome; neither terminal outcome mutates
///   either past the reported point.
/// - provides: the two projections the `(Result, Trace)` pair cannot show — the
///   step counter, which pins `steps == trace.len() - 1`, and the final `Γ`,
///   which pins that a successful run matches every bind with an unbind.
/// - fails: never; a failing run is reported like a succeeding one, since the
///   type and the error are [`run`]'s to return and are dropped here.
/// - panics: none; the frame stack lives on the heap, so adversarial-depth
///   terms do not overflow the host stack.
///
/// # Adequacy
/// - hypothesis: L3 only — both projections are pinned pointwise by the
///   conformance suite, which asserts the step-counter identity against the
///   trace length and the final `Γ` against the initial one, over generated
///   terms in both directions.
/// - witness: `gandr-core-checker-tools`
///   `conformance::step_counter_tracks_trace_length_comp`
/// - witness: `gandr-core-checker-tools`
///   `conformance::step_counter_tracks_trace_length_value`
/// - witness: `gandr-core-checker-tools`
///   `conformance::checked_comps_agree_and_succeed`
/// - witness: `gandr-core-checker-tools`
///   `conformance::sigma_stays_empty_through_every_comp_run`
#[inline]
#[must_use]
pub fn run_report(state: State) -> RunReport
{
    let mut trace = Trace::new();
    let mut current = state;
    loop {
        trace.push(current.control().clone());
        // Snapshot before the step: on `Done`/`Error` the stepped state is
        // consumed, but neither outcome mutates `Γ` or `steps` past this point.
        let steps = current.steps;
        let ctx = current.ctx.clone();
        // Both terminal outcomes carry the type/error elsewhere; this runner
        // only reports the trace, the step counter, and the final `Γ`.
        match step(current) {
            | Outcome::Step(next) => current = next,
            | Outcome::Done(_) | Outcome::Error { .. } => {
                return RunReport {
                    trace,
                    steps: steps.into(),
                    ctx,
                };
            },
        }
    }
}

/// An instrumented run report: the trace, plus the step counter and `Γ` of the
/// state that produced the final outcome.
///
/// Exposes [`State::steps`] and the final `Γ`, which is what lets a caller pin
/// two invariants the bare [`run`] result cannot show: that the step counter is
/// load-bearing (`steps == trace.len() - 1`), and that a successful run
/// restores `Γ` (every bind is matched by an unbind, so the final `Γ` equals
/// the initial one). The conformance suite in `gandr-core-checker-tools` is the
/// consumer those two projections exist for.
pub struct RunReport
{
    /// The control trace.
    pub trace: Trace,
    /// The `steps` counter of the state that produced the final outcome.
    pub steps: MachineStepCount,
    /// The `Γ` of the state that produced the final outcome.
    pub ctx: Ctx,
}

/// Performs one machine step, dispatching on the control register and the
/// top of the stack.
///
/// # Contract
/// - ensures: returns an `Outcome` for every input `State` (the totality
///   keystone — `step` is defined on every reachable state and matches every
///   `State` value): `Outcome::Done(ty)` when control was `Return` on an empty
///   stack, `Outcome::Step(next)` for an ordinary transition (with the step
///   counter saturating-incremented), or `Outcome::Error { error, state }`
///   carrying the `TypeError` and the `FailureState` at the failure point — on
///   a `Return` failure the popped frame is pushed back so the partial
///   derivation is complete, and `Γ` is reported as it stood at the failure
///   point (never mutated before the fallible check).
/// - provides: a typing failure surfaces as `Outcome::Error`, never as a panic
///   or an escaping `Result`.
/// - panics: none.
#[inline]
#[must_use]
pub fn step(state: State) -> Outcome
{
    let State {
        control,
        mut stack,
        mut ctx,
        mut answer,
        steps,
    } = state;
    // `control` is retained for failure reporting; the shallow clone (one AST
    // node — children are `Rc`) feeds the dispatch. On a `Return` failure the
    // popped frame is pushed back so the partial derivation is complete.
    let stepped = match control.clone() {
        | Control::DescendValue { value, dir } => step_value(value, dir, &mut stack, &mut ctx),
        | Control::DescendComp { comp, dir } => {
            step_comp(comp, dir, &mut stack, &mut ctx, &mut answer)
        },
        | Control::Return { ty } => match stack.pop() {
            | None => return Outcome::Done(ty),
            | Some(frame) => {
                let stepped = step_return(frame.clone(), ty, &mut stack, &mut ctx, &mut answer);
                if stepped.is_err() {
                    stack.push(frame);
                }
                stepped
            },
        },
    };
    match stepped {
        | Ok(control_next) => Outcome::Step(State {
            control: control_next,
            stack,
            ctx,
            answer,
            steps: steps.saturating_add(1),
        }),
        | Err(error) => Outcome::Error {
            error,
            state: FailureState {
                control,
                stack,
                ctx,
            },
        },
    }
}

/// Transition for `Descend` on a value (the value rules of §"Core rules").
/// Takes `Γ; Σ` mutably for the reified-stack walk's bind frames (rule Reify;
/// A3.3 `+control`), which bind the consumed payload while the continuation is
/// typed.
fn step_value(
    value: Value,
    dir: Dir<ValueType>,
    stack: &mut Vec<Frame>,
    ctx: &mut Ctx,
) -> Result<Control, TypeError>
{
    match value {
        | Value::Var(name) => {
            match ctx.lookup(gandr_core_term::boundary::NameRef::from(name.as_str())) {
                | Some(found) => {
                    let ty = found.clone();
                    finish_value(ctx, ty, dir).map(return_value)
                },
                | None => Err(TypeError::UnboundVariable { name }),
            }
        },
        | Value::Unit => finish_value(ctx, ValueType::Unit, dir).map(return_value),
        // Rule Int⇑/Int⇓ (A2.1 literals extension; ADR-39 D4): a literal axiom,
        // as Unit — no frame is pushed, matching the recursive checker step.
        // `finish_int_literal` carries the Rust `{integer}` checking-mode
        // widening, so machine and checker stay lock-step.
        | Value::Int(literal) => finish_int_literal(
            ctx,
            gandr_core_term::boundary::IntegerLiteral::from(literal),
            dir,
        )
        .map(return_value),
        // Rule Str⇑/Str⇓ (value-model ladder, ADR-38): a literal axiom, as Int
        // — no frame is pushed, matching the recursive checker step for step.
        | Value::Str(_) => finish_value(ctx, ValueType::string(), dir).map(return_value),
        // Rule Num⇑/Num⇓ (value-model ladder, ADR-39): a suffixed numeric
        // literal, monomorphic in its `NumLit` atom — no frame is pushed,
        // matching the recursive checker step.
        | Value::Num(literal) => finish_value(ctx, literal.value_type(), dir).map(return_value),
        // Rule Hole⇑/Hole⇓ (A2.2 holes extension, pipeline spec §"Holes"): an
        // axiom, as Unit/Int — infer `Unknown`, check against anything via
        // consistency; no frame is pushed.
        | Value::Hole(_) => finish_value(ctx, ValueType::Unknown, dir).map(return_value),
        | Value::Pair(fst, snd) => {
            // Rule Sigma⇓ (ADR-81 feature 2), mirroring the recursive checker:
            // a pair checked against `Σ(x:A).B` descends its first component at
            // `A` and its second at `B[v₁/x]`, the value-into-type substitution
            // over the *actual* first component (available here as `fst`); the
            // `PairSnd` frame returns the `Σ` itself (below). Every other
            // direction is the non-dependent Pair rule via `pair_components`.
            let (fst_dir, snd_dir) = match dir {
                | Dir::Check(ValueType::Sigma {
                    fst: ref head,
                    ref binder,
                    snd: ref tail,
                }) => {
                    let tail_ty = gandr_core_term::identity::subst_valuetype(tail, binder, &fst);
                    (Dir::Check(head.as_ref().clone()), Dir::Check(tail_ty))
                },
                | _ => dir.pair_components(),
            };
            stack.push(Frame::PairFst {
                second: Rc::unwrap_or_clone(snd),
                second_dir: snd_dir,
                dir,
            });
            Ok(Control::DescendValue {
                value: Rc::unwrap_or_clone(fst),
                dir: fst_dir,
            })
        },
        | Value::Inj(side, payload) => match dir {
            | Dir::Check(ValueType::Sum(lhs, rhs)) => {
                let payload_ty = pick(side, &lhs, &rhs);
                stack.push(Frame::Inj {
                    sum: ValueType::Sum(lhs, rhs),
                });
                Ok(Control::DescendValue {
                    value: Rc::unwrap_or_clone(payload),
                    dir: Dir::Check(payload_ty),
                })
            },
            // The matched sum (A2.2 holes extension): checking against
            // `Unknown` checks the payload against `Unknown` and returns the
            // expectation (stored in the frame).
            | Dir::Check(ValueType::Unknown) => {
                stack.push(Frame::Inj {
                    sum: ValueType::Unknown,
                });
                Ok(Control::DescendValue {
                    value: Rc::unwrap_or_clone(payload),
                    dir: Dir::Check(ValueType::Unknown),
                })
            },
            | Dir::Infer | Dir::Check(_) => Err(TypeError::StuckExpr {
                expr: Term::Value(Value::Inj(side, payload)),
                hint: text::ANNOTATE_INJECTION,
            }),
        },
        // Rule List⇓ (ADR-40 D3): check-only. Extract the element direction and
        // the result type from the expectation (the matched-hole list uses
        // `Check(Unknown)` / result `Unknown`); descend the first element over a
        // `Frame::List` carrying the rest, or return the result type directly
        // for the empty list. A list in inference mode (or away from a list /
        // `Unknown` expectation) is stuck — exactly the injection discipline.
        | Value::List(elements) => {
            let (elem_dir, result): (Dir<ValueType>, ValueType) = match dir {
                | Dir::Check(ValueType::List(ref elem)) => (
                    Dir::Check(elem.as_ref().clone()),
                    ValueType::List(Rc::clone(elem)),
                ),
                | Dir::Check(ValueType::Unknown) => {
                    (Dir::Check(ValueType::Unknown), ValueType::Unknown)
                },
                | Dir::Infer | Dir::Check(_) => {
                    return Err(TypeError::StuckExpr {
                        expr: Term::Value(Value::List(elements)),
                        hint: text::ANNOTATE_LIST,
                    });
                },
            };
            let mut iter = elements.into_iter();
            match iter.next() {
                | Some(first) => {
                    stack.push(Frame::List {
                        remaining: iter.collect(),
                        elem_dir: elem_dir.clone(),
                        result,
                    });
                    Ok(Control::DescendValue {
                        value: Rc::unwrap_or_clone(first),
                        dir: elem_dir,
                    })
                },
                | None => Ok(return_value(result)),
            }
        },
        // Rule Record⇑/Record⇓ (ADR-45 D3): direction-polymorphic, like the
        // eager pair, generalized to labels. Descend the first field (in sorted
        // order) in its per-label direction over a `Frame::Record` carrying the
        // rest; the empty record finishes immediately. The machine image of
        // `rule_record` — same per-field `Descend`/`Return` order.
        | Value::Record(fields) => {
            let mut iter = fields.into_iter();
            match iter.next() {
                | Some((label, field_value)) => {
                    let field_dir = dir.record_field_dir(
                        gandr_core_term::boundary::FieldName::from(label.as_str()),
                    );
                    stack.push(Frame::Record {
                        remaining: iter.collect(),
                        current_label: label,
                        typed: BTreeMap::new(),
                        dir,
                    });
                    Ok(Control::DescendValue {
                        value: Rc::unwrap_or_clone(field_value),
                        dir: field_dir,
                    })
                },
                | None => {
                    finish_value(ctx, ValueType::Record(BTreeMap::new()), dir).map(return_value)
                },
            }
        },
        // Rule Run: the pure-computation embedding, inference-primary. The
        // computation is inferred and the `Frame::Run` pop reads the returner's
        // payload off it, declining an effectful row or a non-returner by name.
        // Lock-step with the recursive checker's arm.
        | Value::Run(body) => {
            stack.push(Frame::Run {
                body: Rc::clone(&body),
                dir,
            });
            Ok(Control::DescendComp {
                comp: Rc::unwrap_or_clone(body),
                dir: Dir::Infer,
            })
        },
        | Value::Thunk(grade, body) => match dir {
            // The matched thunk (A2.2 holes extension): the body checks
            // against `Unknown`; no grade constraint is emitted (the matched
            // grade is unknown). The frame keeps the literal's grade for the
            // rebuild, exactly as the recursive checker.
            | Dir::Check(ValueType::Unknown) => {
                stack.push(Frame::Thunk {
                    grade,
                    dir: Dir::Check(ValueType::Unknown),
                });
                Ok(Control::DescendComp {
                    comp: Rc::unwrap_or_clone(body),
                    dir: Dir::Check(CompType::Unknown),
                })
            },
            | Dir::Check(ValueType::Thunk(expected_grade, expected_body)) => {
                if !bool::from(expected_grade.leq(grade)) {
                    return Err(TypeError::GradeError {
                        lower: expected_grade,
                        upper: grade,
                    });
                }
                let body_dir = Dir::Check(expected_body.as_ref().clone());
                stack.push(Frame::Thunk {
                    grade: expected_grade,
                    dir: Dir::Check(ValueType::Thunk(expected_grade, expected_body)),
                });
                Ok(Control::DescendComp {
                    comp: Rc::unwrap_or_clone(body),
                    dir: body_dir,
                })
            },
            | other => {
                stack.push(Frame::Thunk { grade, dir: other });
                Ok(Control::DescendComp {
                    comp: Rc::unwrap_or_clone(body),
                    dir: Dir::Infer,
                })
            },
        },
        | Value::Annot(inner, ty) => {
            stack.push(Frame::Annot { dir });
            Ok(Control::DescendValue {
                value: Rc::unwrap_or_clone(inner),
                dir: Dir::Check(Rc::unwrap_or_clone(ty)),
            })
        },
        // Rule Reify (`effects-control-shell.md` §2.1; A3.3 `+control`):
        // check-only against `Stk(B, C)`. Extract the consumed type `B` from
        // the expectation (the matched-hole stack uses `B = Unknown`), then walk
        // the stack forward from `B` — the structural frames are processed
        // inline by [`walk_stack`], descending only the sub-terms, exactly as
        // the recursive checker's `stack_infer`.
        | Value::Stk(reified) => {
            let consumed: CompType = match dir {
                | Dir::Check(ValueType::Stk(ref b, _)) => b.as_ref().clone(),
                | Dir::Check(ValueType::Unknown) => CompType::Unknown,
                | Dir::Infer | Dir::Check(_) => {
                    return Err(TypeError::StuckExpr {
                        expr: Term::Value(Value::Stk(reified)),
                        hint: text::STK_NEEDS_STK_TYPE,
                    });
                },
            };
            walk_stack(Rc::unwrap_or_clone(reified), consumed, dir, stack, ctx)
        },
        // Rule Here (ADR-76): infer the witness, then form `Path A v v` at the
        // frame pop. The image of `Frame::Inj` — a single descended sub-value —
        // with the witness stored for the endpoints.
        | Value::Here(witness) => {
            let witness_value = Rc::unwrap_or_clone(witness);
            stack.push(Frame::Here {
                witness: witness_value.clone(),
                dir,
            });
            Ok(Control::DescendValue {
                value: witness_value,
                dir: Dir::Infer,
            })
        },
        // Rule Ctor⇓ (ADR-80 Decision 2): a declared-data constructor only
        // checks, against its data type `Data { id, args }` (nominal `id`
        // equality — a mismatch is a `TypeMismatch` at dispatch, before the
        // payload is descended, matching the recursive checker's short-circuit)
        // or against `Unknown`; the payload is descended in inference (the
        // frozen core carries only the nominal tag). The image of `Frame::Inj`
        // — a single descended sub-value — with the result type stored.
        | Value::Ctor { id, tag, payload } => match dir {
            | Dir::Check(ValueType::Data {
                id: expected_id,
                args,
            }) => {
                if id != expected_id {
                    return Err(TypeError::type_mismatch(
                        Ty::Value(ValueType::Data {
                            id: expected_id,
                            args,
                        }),
                        Ty::Value(ValueType::Data {
                            id,
                            args: alloc::vec::Vec::new(),
                        }),
                    ));
                }
                stack.push(Frame::Ctor {
                    result: ValueType::Data {
                        id: expected_id,
                        args,
                    },
                });
                Ok(Control::DescendValue {
                    value: Rc::unwrap_or_clone(payload),
                    dir: Dir::Infer,
                })
            },
            | Dir::Check(ValueType::Unknown) => {
                stack.push(Frame::Ctor {
                    result: ValueType::Unknown,
                });
                Ok(Control::DescendValue {
                    value: Rc::unwrap_or_clone(payload),
                    dir: Dir::Infer,
                })
            },
            | Dir::Infer | Dir::Check(_) => Err(TypeError::StuckExpr {
                expr: Term::Value(Value::Ctor { id, tag, payload }),
                hint: text::ANNOTATE_CTOR,
            }),
        },
        // Rule Pack⇓: a packed module only checks, against a package type or
        // `Unknown`. Every refusal — arity, the payload's grade, the witness
        // substitution — fires **here, at the dispatch, before any frame is
        // pushed**, the identical firing point as the recursive checker's rule
        // body, so the two stay lock-step on which error surfaces first.
        | Value::Pack { witnesses, payload } => match dir {
            | Dir::Check(ValueType::Package {
                grade,
                abstracts,
                payload: signature_payload,
            }) => {
                let expected = gandr_core_checker::judgements::package::pack_payload_expectation(
                    grade,
                    &abstracts,
                    signature_payload.as_ref(),
                    &witnesses,
                );
                let expected = match expected {
                    | Ok(expected) => expected,
                    | Err(refusal) => {
                        return Err(gandr_core_checker::judgements::package::refusal_error(
                            refusal,
                            Term::Value(Value::Pack { witnesses, payload }),
                        ));
                    },
                };
                stack.push(Frame::Pack {
                    result: ValueType::Package {
                        grade,
                        abstracts,
                        payload: signature_payload,
                    },
                });
                Ok(Control::DescendValue {
                    value: Rc::unwrap_or_clone(payload),
                    dir: Dir::Check(expected),
                })
            },
            | Dir::Check(ValueType::Unknown) => {
                stack.push(Frame::Pack {
                    result: ValueType::Unknown,
                });
                Ok(Control::DescendValue {
                    value: Rc::unwrap_or_clone(payload),
                    dir: Dir::Check(ValueType::Unknown),
                })
            },
            | Dir::Infer | Dir::Check(_) => Err(TypeError::StuckExpr {
                expr: Term::Value(Value::Pack { witnesses, payload }),
                hint: text::ANNOTATE_PACK,
            }),
        },
    }
}

/// Transition for `Descend` on a computation (the computation rules of §"Core
/// rules"). Takes the ambient answer register mutably for the delimited-control
/// rules (`reset` sets it, `shift` reads it; A3.3 `+control`).
fn step_comp(
    comp: Comp,
    dir: Dir<CompType>,
    stack: &mut Vec<Frame>,
    ctx: &mut Ctx,
    ambient: &mut Option<CompType>,
) -> Result<Control, TypeError>
{
    match comp {
        | Comp::Abs(name, annot, body) => match (annot, dir) {
            | (None, Dir::Check(CompType::Arrow { binder, arg, res })) => {
                ctx.bind(name.clone(), arg.as_ref().clone());
                let body_dir = Dir::Check(relocate_codomain(
                    binder.as_deref().map(NameRef::from),
                    NameRef::from(name.as_str()),
                    &res,
                ));
                stack.push(Frame::Abs {
                    binder: binder.as_ref().map(|_| name.clone()),
                    var: name,
                    arg: arg.as_ref().clone(),
                    dir: Dir::Check(CompType::Arrow { binder, arg, res }),
                });
                Ok(Control::DescendComp {
                    comp: Rc::unwrap_or_clone(body),
                    dir: body_dir,
                })
            },
            | (Some(annot_ty), any_dir) => {
                ctx.bind(name.clone(), annot_ty.as_ref().clone());
                stack.push(Frame::Abs {
                    // Inference decides the binder from the body's own type,
                    // which is not known until this frame pops.
                    binder: None,
                    var: name,
                    arg: annot_ty.as_ref().clone(),
                    dir: any_dir,
                });
                Ok(Control::DescendComp {
                    comp: Rc::unwrap_or_clone(body),
                    dir: Dir::Infer,
                })
            },
            // The matched arrow (A2.2 holes extension): an unannotated
            // binder checked against `Unknown` binds at `Unknown` and checks
            // its body against `Unknown`.
            | (None, Dir::Check(CompType::Unknown)) => {
                ctx.bind(name.clone(), ValueType::Unknown);
                stack.push(Frame::Abs {
                    binder: None,
                    var: name,
                    arg: ValueType::Unknown,
                    dir: Dir::Check(CompType::Unknown),
                });
                Ok(Control::DescendComp {
                    comp: Rc::unwrap_or_clone(body),
                    dir: Dir::Check(CompType::Unknown),
                })
            },
            | (None, Dir::Infer) => Err(TypeError::StuckExpr {
                expr: diagnostic_abs_term(name, body),
                hint: text::ANNOTATE_BINDER,
            }),
            | (None, Dir::Check(_)) => Err(TypeError::StuckExpr {
                expr: diagnostic_abs_term(name, body),
                hint: text::ABS_NEEDS_ARROW,
            }),
        },
        | Comp::App(head, arg) => {
            stack.push(Frame::AppFn {
                arg: Rc::unwrap_or_clone(arg),
                dir,
            });
            Ok(Control::DescendComp {
                comp: Rc::unwrap_or_clone(head),
                dir: Dir::Infer,
            })
        },
        | Comp::Ret(payload) => {
            let payload_dir = dir.ret_payload();
            stack.push(Frame::Ret { dir });
            Ok(Control::DescendValue {
                value: Rc::unwrap_or_clone(payload),
                dir: payload_dir,
            })
        },
        | Comp::Bind(bound, name, cont) => {
            stack.push(Frame::Bind {
                var: name,
                cont: Rc::unwrap_or_clone(cont),
                dir,
            });
            Ok(Control::DescendComp {
                comp: Rc::unwrap_or_clone(bound),
                dir: Dir::Infer,
            })
        },
        | Comp::Force(thunked) => {
            stack.push(Frame::Force { dir });
            Ok(Control::DescendValue {
                value: Rc::unwrap_or_clone(thunked),
                dir: Dir::Infer,
            })
        },
        // Rule RecordProj⇑ (ADR-45 D4): the image of `Comp::Force` — descend the
        // record value in inference mode over a frame retaining the record (for
        // the stuck-error term, matching the checker). The descent types a clone
        // so the trace is identical to `rule_record_proj`.
        | Comp::RecordProj { record, label } => {
            stack.push(Frame::RecordProj {
                record: Rc::clone(&record),
                label,
                dir,
            });
            Ok(Control::DescendValue {
                value: record.as_ref().clone(),
                dir: Dir::Infer,
            })
        },
        | Comp::Case(scrut, arm_fst, arm_snd) => match dir {
            | Dir::Check(expected) => {
                stack.push(Frame::CaseScrut {
                    arm_fst,
                    arm_snd,
                    expected,
                });
                Ok(Control::DescendValue {
                    value: Rc::unwrap_or_clone(scrut),
                    dir: Dir::Infer,
                })
            },
            | Dir::Infer => Err(TypeError::StuckExpr {
                expr: Term::Comp(Comp::Case(scrut, arm_fst, arm_snd)),
                hint: text::CASE_NEEDS_CHECK,
            }),
        },
        // Rule `DataCase`⇓ (ADR-80 Decision 3): check-only, like Case. Store the
        // arms and expectation, then descend the scrutinee in inference mode;
        // the `Frame::DataCaseScrut` pop shape-checks it and begins the arm
        // walk.
        | Comp::DataCase(scrut, arms) => match dir {
            | Dir::Check(expected) => {
                stack.push(Frame::DataCaseScrut { arms, expected });
                Ok(Control::DescendValue {
                    value: Rc::unwrap_or_clone(scrut),
                    dir: Dir::Infer,
                })
            },
            | Dir::Infer => Err(TypeError::StuckExpr {
                expr: Term::Comp(Comp::DataCase(scrut, arms)),
                hint: text::DATA_CASE_NEEDS_CHECK,
            }),
        },
        // Rule ListCase⇓ (ADR-40 D4): check-only, like Case. Store the arms and
        // binders, then descend the scrutinee in inference mode.
        | Comp::ListCase {
            scrut,
            nil,
            head,
            tail,
            cons,
        } => match dir {
            | Dir::Check(expected) => {
                stack.push(Frame::ListCaseScrut {
                    nil,
                    head,
                    tail,
                    cons,
                    expected,
                });
                Ok(Control::DescendValue {
                    value: Rc::unwrap_or_clone(scrut),
                    dir: Dir::Infer,
                })
            },
            | Dir::Infer => Err(TypeError::StuckExpr {
                expr: Term::Comp(Comp::ListCase {
                    scrut,
                    nil,
                    head,
                    tail,
                    cons,
                }),
                hint: text::LIST_CASE_NEEDS_CHECK,
            }),
        },
        // Rules `SplitMotive`⇑ / Split⇓ (ADR-82): store the body, the motive, and
        // the scrutinee value, then descend the scrutinee in inference mode. A
        // **motive-less split in inference mode is stuck** ([`SPLIT_NEEDS_MOTIVE`])
        // — declined **here, at the descend step, before any frame is pushed**,
        // the identical firing point as the recursive checker's rule entry
        // (ADR-82 D3, ADR-48 lock-step).
        | Comp::Split {
            scrut,
            fst_name,
            snd_name,
            motive,
            body,
        } => {
            if motive.is_none() && matches!(dir, Dir::Infer) {
                return Err(TypeError::StuckExpr {
                    expr: Term::Comp(Comp::Split {
                        scrut,
                        fst_name,
                        snd_name,
                        motive,
                        body,
                    }),
                    hint: text::SPLIT_NEEDS_MOTIVE,
                });
            }
            let scrut_value = Rc::unwrap_or_clone(scrut);
            stack.push(Frame::Split {
                fst_name,
                snd_name,
                motive,
                scrut: scrut_value.clone(),
                body: Rc::unwrap_or_clone(body),
                dir,
            });
            Ok(Control::DescendValue {
                value: scrut_value,
                dir: Dir::Infer,
            })
        },
        | Comp::With(fst, snd) => match dir {
            | Dir::Check(CompType::With(lhs, rhs)) => {
                let fst_dir = Dir::Check(lhs.as_ref().clone());
                stack.push(Frame::With1 {
                    second: Rc::unwrap_or_clone(snd),
                    second_expected: rhs.as_ref().clone(),
                });
                Ok(Control::DescendComp {
                    comp: Rc::unwrap_or_clone(fst),
                    dir: fst_dir,
                })
            },
            // The matched with (A2.2 holes extension): both components check
            // against `Unknown`; the rebuild yields `Unknown & Unknown`,
            // exactly as the recursive checker.
            | Dir::Check(CompType::Unknown) => {
                stack.push(Frame::With1 {
                    second: Rc::unwrap_or_clone(snd),
                    second_expected: CompType::Unknown,
                });
                Ok(Control::DescendComp {
                    comp: Rc::unwrap_or_clone(fst),
                    dir: Dir::Check(CompType::Unknown),
                })
            },
            | Dir::Infer | Dir::Check(_) => Err(TypeError::StuckExpr {
                expr: Term::Comp(Comp::With(fst, snd)),
                hint: text::WITH_NEEDS_WITH,
            }),
        },
        | Comp::Prj(side, target) => {
            stack.push(Frame::Prj { side, dir });
            Ok(Control::DescendComp {
                comp: Rc::unwrap_or_clone(target),
                dir: Dir::Infer,
            })
        },
        // Rule Dup (`spec:implementation/type-system.md` §"Grades"): check-only — the split grades
        // `r`/`s` come only from the expectation `F (U_r B × U_s B)`, so a dup
        // away from that shape is stuck before the scrutinee is even descended
        // (matching the recursive checker step for step).
        | Comp::Dup(thunked) => {
            let split = match dir {
                | Dir::Check(CompType::F(ref payload, _)) => match payload.as_ref() {
                    | &ValueType::Prod(ref lhs, ref rhs) => match (lhs.as_ref(), rhs.as_ref()) {
                        | (&ValueType::Thunk(r, _), &ValueType::Thunk(s, _)) => Some((r, s)),
                        | _ => None,
                    },
                    | _ => None,
                },
                | _ => None,
            };
            match split {
                | Some((r, s)) => {
                    stack.push(Frame::Dup { r, s, dir });
                    Ok(Control::DescendValue {
                        value: Rc::unwrap_or_clone(thunked),
                        dir: Dir::Infer,
                    })
                },
                | None => Err(TypeError::StuckExpr {
                    expr: Term::Comp(Comp::Dup(thunked)),
                    hint: text::DUP_NEEDS_RETURNER_PRODUCT,
                }),
            }
        },
        // Rule Drop (`spec:implementation/type-system.md` §"Grades"): infer the thunk, discard the
        // budget.
        | Comp::Drop(thunked) => {
            stack.push(Frame::Drop { dir });
            Ok(Control::DescendValue {
                value: Rc::unwrap_or_clone(thunked),
                dir: Dir::Infer,
            })
        },
        // Rule Op⇑ (`effects-control-shell.md` §1.1; A3.2 `+effects`): resolve
        // the op against the inline signature, then check the payload against
        // its payload type. An absent op is stuck before the payload is
        // descended (matching the recursive checker step for step).
        | Comp::Perform(sig, op, arg) => match sig
            .op(gandr_core_term::boundary::OperationName::from(op.as_str()))
            .cloned()
        {
            | Some(op_def) => {
                let payload_dir = Dir::Check(op_def.payload().clone());
                let row = EffectRow::singleton(*sig);
                stack.push(Frame::Perform {
                    reply: op_def.reply().clone(),
                    row,
                    dir,
                });
                Ok(Control::DescendValue {
                    value: Rc::unwrap_or_clone(arg),
                    dir: payload_dir,
                })
            },
            | None => Err(TypeError::StuckExpr {
                expr: Term::Comp(Comp::Perform(sig, op, arg)),
                hint: text::PERFORM_UNKNOWN_OP,
            }),
        },
        // Rule Handle⇓ (`effects-control-shell.md` §1.1; A3.2 `+effects`):
        // check-only against a returner answer. Validate the direction and
        // clause coverage up front (so a stuck handle errors before any
        // descend, matching the recursive checker), then descend the handled
        // computation in inference mode.
        | Comp::Handle {
            sig,
            scrutinee,
            ret,
            ops,
        } => {
            let answer: CompType = match dir {
                | Dir::Check(CompType::F(payload, eps)) => CompType::F(payload, eps),
                | Dir::Check(CompType::Unknown) => CompType::Unknown,
                | Dir::Infer => {
                    return Err(TypeError::StuckExpr {
                        expr: Term::Comp(Comp::Handle {
                            sig,
                            scrutinee,
                            ret,
                            ops,
                        }),
                        hint: text::HANDLE_NEEDS_CHECK,
                    });
                },
                | Dir::Check(_) => {
                    return Err(TypeError::StuckExpr {
                        expr: Term::Comp(Comp::Handle {
                            sig,
                            scrutinee,
                            ret,
                            ops,
                        }),
                        hint: text::HANDLE_NEEDS_RETURNER,
                    });
                },
            };
            let Some(resolved) = resolve_handler_coverage(&sig, &ops)
            else {
                return Err(TypeError::StuckExpr {
                    expr: Term::Comp(Comp::Handle {
                        sig,
                        scrutinee,
                        ret,
                        ops,
                    }),
                    hint: text::HANDLER_CLAUSES_MISMATCH,
                });
            };
            stack.push(Frame::HandleScrut {
                answer,
                sig_name: sig.name().as_ref().to_owned(),
                ret,
                ops: resolved,
            });
            Ok(Control::DescendComp {
                comp: Rc::unwrap_or_clone(scrutinee),
                dir: Dir::Infer,
            })
        },
        // Rule Resume⇑ (`effects-control-shell.md` §2.1; A3.3 `+control`): the
        // image of `App` with the sorts swapped — the reified stack value is the
        // inferred principal premise, the fed computation the checked argument.
        | Comp::Resume(reified, comp) => {
            stack.push(Frame::ResumeFn {
                comp: Rc::unwrap_or_clone(comp),
                dir,
            });
            Ok(Control::DescendValue {
                value: Rc::unwrap_or_clone(reified),
                dir: Dir::Infer,
            })
        },
        // Rule Reset⇓ (`effects-control-shell.md` §2.2; A3.3 `+control`):
        // check-only against the answer `C`. Save the current ambient answer,
        // set it to `C`, and descend the body in `Check(C)`; the `ResetBody`
        // pop restores the saved answer (dynamic scoping). Stuck in inference.
        | Comp::Reset(body) => match dir {
            | Dir::Check(answer_ty) => {
                let saved = ambient.replace(answer_ty.clone());
                stack.push(Frame::ResetBody { saved });
                Ok(Control::DescendComp {
                    comp: Rc::unwrap_or_clone(body),
                    dir: Dir::Check(answer_ty),
                })
            },
            | Dir::Infer => Err(TypeError::StuckExpr {
                expr: Term::Comp(Comp::Reset(body)),
                hint: text::RESET_NEEDS_CHECK,
            }),
        },
        // Rule Shift⇓ (`effects-control-shell.md` §2.2; A3.3 `+control`):
        // check-only against the captured type `B`. Read the ambient answer `C`
        // (a `shift` outside any `reset` — `answer = None` — is stuck), bind
        // `k : Stk(B, C)`, and descend the body in `Check(C)`. The two stuck
        // checks run direction-first, then ambient-answer, matching the checker.
        | Comp::Shift(k, body) => match dir {
            | Dir::Check(captured) => match ambient.clone() {
                | Some(ans) => {
                    ctx.bind(k, ValueType::stk(captured.clone(), ans.clone()));
                    stack.push(Frame::ShiftBody { captured });
                    Ok(Control::DescendComp {
                        comp: Rc::unwrap_or_clone(body),
                        dir: Dir::Check(ans),
                    })
                },
                | None => Err(TypeError::StuckExpr {
                    expr: Term::Comp(Comp::Shift(k, body)),
                    hint: text::SHIFT_NEEDS_RESET,
                }),
            },
            | Dir::Infer => Err(TypeError::StuckExpr {
                expr: Term::Comp(Comp::Shift(k, body)),
                hint: text::SHIFT_NEEDS_CHECK,
            }),
        },
        // Rule Fix⇓: the recursion former, check-primary. The expectation
        // states the self-reference's type `U_ω B`, the body is checked against
        // `B`, and the `Frame::FixBody` pop delivers `B` unchanged. Inference is
        // stuck: nothing in the term synthesizes the type the body is being
        // checked against, and the ascription coercion is the inference route.
        // Lock-step with the recursive checker's arm.
        | Comp::Fix(x, body) => match dir {
            | Dir::Check(recursive) => {
                ctx.bind(x, ValueType::thunk(Grade::OMEGA, recursive.clone()));
                stack.push(Frame::FixBody {
                    recursive: recursive.clone(),
                });
                Ok(Control::DescendComp {
                    comp: Rc::unwrap_or_clone(body),
                    dir: Dir::Check(recursive),
                })
            },
            | Dir::Infer => Err(TypeError::StuckExpr {
                expr: Term::Comp(Comp::Fix(x, body)),
                hint: text::FIX_NEEDS_CHECK,
            }),
        },
        // Rule Hole⇑/Hole⇓ (A2.2 holes extension): an axiom, as the value
        // hole — no frame is pushed.
        | Comp::Hole(_) => finish_comp(ctx, CompType::Unknown, dir).map(return_comp),
        // Rule Native (ADR-42): an axiom typed by the primitive's residual type
        // (the declared type with the consumed arguments' arrows peeled) — no
        // frame is pushed, lock-step with the recursive checker's arm.
        | Comp::Native { prim, args } => {
            finish_comp(ctx, prim.residual_type(args.len()), dir).map(return_comp)
        },
        // Rule Walk (ADR-76): infer the scrutinee, then the `Frame::WalkScrut` pop
        // forms the diagonal base check and the result. The image of
        // `Frame::CaseScrut` — the scrutinee descended in inference mode.
        | Comp::Walk {
            scrut,
            motive,
            base,
        } => {
            let scrut_value = Rc::unwrap_or_clone(scrut);
            stack.push(Frame::WalkScrut {
                motive: *motive,
                base,
                scrut: scrut_value.clone(),
                dir,
            });
            Ok(Control::DescendValue {
                value: scrut_value,
                dir: Dir::Infer,
            })
        },
        // Rule Unpack⇓: inference is stuck and every signature refusal fires
        // **here, at the descend step, before any frame is pushed** — the
        // identical firing point as the recursive checker's rule body. The
        // scrutinee is then descended in **checking** mode against the
        // ascription, which is the decidability fence: nothing infers a module
        // type from a core term.
        | Comp::Unpack {
            scrut,
            signature,
            atoms,
            binder,
            body,
        } => {
            let Dir::Check(expected) = dir
            else {
                return Err(TypeError::StuckExpr {
                    expr: Term::Comp(Comp::Unpack {
                        scrut,
                        signature,
                        atoms,
                        binder,
                        body,
                    }),
                    hint: text::UNPACK_NEEDS_CHECK,
                });
            };
            let bound = match *signature {
                | ValueType::Package {
                    grade,
                    ref abstracts,
                    payload: ref signature_payload,
                } => {
                    if !bool::from(Grade::ONE.leq(grade)) {
                        return Err(TypeError::GradeError {
                            lower: Grade::ONE,
                            upper: grade,
                        });
                    }
                    let bound = gandr_core_checker::judgements::package::unpack_binding(
                        grade,
                        abstracts,
                        signature_payload.as_ref(),
                        &atoms,
                    );
                    match bound {
                        | Ok(bound) => bound,
                        | Err(refusal) => {
                            return Err(gandr_core_checker::judgements::package::refusal_error(
                                refusal,
                                Term::Comp(Comp::Unpack {
                                    scrut,
                                    signature,
                                    atoms,
                                    binder,
                                    body,
                                }),
                            ));
                        },
                    }
                },
                | ValueType::Unknown => ValueType::Unknown,
                | ref other => {
                    return Err(TypeError::ShapeMismatch {
                        expected: text::SHAPE_PACKAGE,
                        actual: Ty::Value(other.clone()),
                    });
                },
            };
            let ascribed = signature.as_ref().clone();
            stack.push(Frame::Unpack {
                binder,
                bound,
                body: Rc::unwrap_or_clone(body),
                expected,
            });
            Ok(Control::DescendValue {
                value: Rc::unwrap_or_clone(scrut),
                dir: Dir::Check(ascribed),
            })
        },
    }
}
/// Builds the legacy diagnostic expression for an unannotated abstraction
/// through the canonical flat-arena bridge.
///
/// This is an explicit compatibility boundary: the typing-machine hot path has
/// already destructured the abstraction, and the structural [`Comp::Abs`] is
/// reconstructed only to populate the public [`TypeError`] surface. The
/// computation is immediately allocated into [`FlatArena`] and read back
/// through the checked bridge so diagnostics remain source-compatible without
/// making `Rc<Comp>` the machine's internal residual carrier.
///
/// # Contract
/// - ensures: returns a `Term::Comp` equivalent to `λ name. body` for the
///   public error payload.
/// - panics: none.
fn diagnostic_abs_term(
    name: String,
    body: Rc<Comp>,
) -> Term
{
    let structural = Comp::Abs(name, None, body);
    let mut arena = FlatArena::new();
    match arena
        .alloc_comp(&structural)
        .and_then(|root| arena.comp(root))
    {
        | Ok(comp) => Term::Comp(comp),
        | Err(_error) => Term::Comp(structural),
    }
}

/// Transition for `Return` against the popped frame (the frame-pop rules of
/// `spec:implementation/typing-machine.md` §"The step function"). Takes the
/// ambient answer register mutably for the [`Frame::ResetBody`] restore (A3.3
/// `+control`).
fn step_return(
    frame: Frame,
    ty: Ty,
    stack: &mut Vec<Frame>,
    ctx: &mut Ctx,
    ambient: &mut Option<CompType>,
) -> Result<Control, TypeError>
{
    match frame {
        | Frame::PairFst {
            second,
            second_dir,
            dir,
        } => {
            let fst_ty = expect_value(ty)?;
            stack.push(Frame::PairSnd { first: fst_ty, dir });
            Ok(Control::DescendValue {
                value: second,
                dir: second_dir,
            })
        },
        | Frame::PairSnd { first, dir } => {
            let snd_ty = expect_value(ty)?;
            // Rule Sigma⇓ finish (ADR-81 feature 2): a pair checked against a
            // `Σ` returns the `Σ` itself (its components were already checked
            // against the head and the substituted tail). Every other direction
            // is the non-dependent Prod finish.
            match dir {
                | Dir::Check(sigma @ ValueType::Sigma { .. }) => Ok(return_value(sigma)),
                | other => {
                    finish_value(ctx, ValueType::Prod(Rc::new(first), Rc::new(snd_ty)), other)
                        .map(return_value)
                },
            }
        },
        | Frame::Inj { sum } => {
            expect_value(ty)?;
            Ok(return_value(sum))
        },
        // Rule Ctor⇓ (ADR-80 Decision 2): the payload typed (by inference); the
        // nominal `id` check already ran at the dispatch, so return the stored
        // data type. The image of `Frame::Inj`.
        // Rule Pack⇓ pop shares this arm: the payload checked against the
        // instantiated signature, and every other check the pack rule makes —
        // arity, the payload's grade, the witness substitution — ran at the
        // dispatch. Both frames therefore just return their stored type.
        | Frame::Ctor { result } | Frame::Pack { result } => {
            expect_value(ty)?;
            Ok(return_value(result))
        },
        // Rule Unpack⇓ pop: the scrutinee checked against the ascription. Bind
        // the module variable at the type the dispatch computed and descend the
        // body against the expectation, which rides the `UnpackBody` frame.
        | Frame::Unpack {
            binder,
            bound,
            body,
            expected,
        } => {
            expect_value(ty)?;
            ctx.bind(binder, bound);
            stack.push(Frame::UnpackBody {
                result: expected.clone(),
            });
            Ok(Control::DescendComp {
                comp: body,
                dir: Dir::Check(expected),
            })
        },
        // Rule Unpack⇓ pop 2: the body checked; restore `Γ` and return the
        // stored expectation, never the body's echo (the `SplitBody`
        // discipline).
        | Frame::UnpackBody { result } => {
            expect_comp(ty)?;
            ctx.unbind();
            Ok(return_comp(result))
        },
        // Rule List⇓ (ADR-40 D3): the current element typed (sort-checked); the
        // element's own subsumption already ran at its descend. Descend the next
        // element, or return the stored result type when none remain.
        | Frame::List {
            remaining,
            elem_dir,
            result,
        } => {
            expect_value(ty)?;
            let mut iter = remaining.into_iter();
            match iter.next() {
                | Some(next) => {
                    stack.push(Frame::List {
                        remaining: iter.collect(),
                        elem_dir: elem_dir.clone(),
                        result,
                    });
                    Ok(Control::DescendValue {
                        value: Rc::unwrap_or_clone(next),
                        dir: elem_dir,
                    })
                },
                | None => Ok(return_value(result)),
            }
        },
        // Rule Record⇑/Record⇓ (ADR-45 D3): the current field typed (its own
        // subsumption ran at its descend); accumulate it and descend the next
        // field in its per-label direction, or rebuild `{ℓᵢ:Aᵢ}` and finish
        // against the record's direction when none remain (the width/depth Sub
        // rule). The image of `rule_record`'s loop tail.
        | Frame::Record {
            remaining,
            current_label,
            mut typed,
            dir,
        } => {
            let field_ty = expect_value(ty)?;
            typed.insert(current_label, Rc::new(field_ty));
            let mut iter = remaining.into_iter();
            match iter.next() {
                | Some((label, field_value)) => {
                    let field_dir = dir.record_field_dir(
                        gandr_core_term::boundary::FieldName::from(label.as_str()),
                    );
                    stack.push(Frame::Record {
                        remaining: iter.collect(),
                        current_label: label,
                        typed,
                        dir,
                    });
                    Ok(Control::DescendValue {
                        value: Rc::unwrap_or_clone(field_value),
                        dir: field_dir,
                    })
                },
                | None => finish_value(ctx, ValueType::Record(typed), dir).map(return_value),
            }
        },
        | Frame::Thunk { grade, dir } => {
            let body_ty = expect_comp(ty)?;
            finish_value(ctx, ValueType::Thunk(grade, Rc::new(body_ty)), dir).map(return_value)
        },
        | Frame::Run { body, dir } => {
            let body_ty = expect_comp(ty)?;
            let produced = match body_ty {
                | CompType::F(produced, ref row) => {
                    if !bool::from(row.is_empty()) {
                        return Err(TypeError::StuckExpr {
                            expr: Term::Value(Value::Run(body)),
                            hint: text::RUN_NEEDS_PURITY,
                        });
                    }
                    produced.as_ref().clone()
                },
                | CompType::Unknown => ValueType::Unknown,
                | _other => {
                    return Err(TypeError::StuckExpr {
                        expr: Term::Value(Value::Run(body)),
                        hint: text::RUN_NEEDS_RETURNER,
                    });
                },
            };
            finish_value(ctx, produced, dir).map(return_value)
        },
        | Frame::Annot { dir } => {
            let checked = expect_value(ty)?;
            finish_value(ctx, checked, dir).map(return_value)
        },
        | Frame::Abs {
            var,
            binder,
            arg,
            dir,
        } => {
            // Frame-pop ordering convention (machine module doc): fallible sort
            // checks run *before* the `Γ` restore, so `Γ` is never mutated on
            // the error path. Matches [`Frame::CaseArm1`].
            let res_ty = expect_comp(ty)?;
            ctx.unbind();
            // A checked abstraction carries the binder its expectation gave it;
            // an inferred one derives it from the body's type here, which is
            // the first point that type exists. Both agree with the recursive
            // checker's `rule_abs` step for step.
            let binder = match dir {
                | Dir::Check(_) => binder,
                | Dir::Infer => inferred_binder(NameRef::from(var.as_str()), &res_ty),
            };
            finish_comp(
                ctx,
                CompType::Arrow {
                    binder,
                    arg: Rc::new(arg),
                    res: Rc::new(res_ty),
                },
                dir,
            )
            .map(return_comp)
        },
        | Frame::AppFn { arg, dir } => match expect_comp(ty)? {
            | CompType::Arrow {
                binder,
                arg: param,
                res,
            } => {
                stack.push(Frame::AppArg {
                    binder,
                    arg: arg.clone(),
                    result: Rc::unwrap_or_clone(res),
                    dir,
                });
                Ok(Control::DescendValue {
                    value: arg,
                    dir: Dir::Check(param.as_ref().clone()),
                })
            },
            // The matched arrow (A2.2 holes extension): an `Unknown` head
            // applies — argument against `Unknown`, result `Unknown`.
            | CompType::Unknown => {
                stack.push(Frame::AppArg {
                    binder: None,
                    arg: arg.clone(),
                    result: CompType::Unknown,
                    dir,
                });
                Ok(Control::DescendValue {
                    value: arg,
                    dir: Dir::Check(ValueType::Unknown),
                })
            },
            | other => Err(TypeError::ShapeMismatch {
                expected: text::SHAPE_ARROW,
                actual: Ty::Comp(other),
            }),
        },
        | Frame::AppArg {
            binder,
            arg,
            result,
            dir,
        } => {
            expect_value(ty)?;
            // Dependent application: the codomain is closed at the argument the
            // head was applied to, matching the recursive checker's `rule_app`.
            let applied = instantiate_codomain(binder.as_deref().map(NameRef::from), &result, &arg);
            finish_comp(ctx, applied, dir).map(return_comp)
        },
        | Frame::Ret { dir } => {
            let payload_ty = expect_value(ty)?;
            // `ret v` performs no effects: the pure returner `F^⟨⟩ A` (ADR-33 D2).
            finish_comp(ctx, CompType::returner(payload_ty), dir).map(return_comp)
        },
        | Frame::Force { dir } => match expect_value(ty)? {
            | ValueType::Thunk(grade, body) => {
                if !bool::from(Grade::ONE.leq(grade)) {
                    return Err(TypeError::GradeError {
                        lower: Grade::ONE,
                        upper: grade,
                    });
                }
                finish_comp(ctx, Rc::unwrap_or_clone(body), dir).map(return_comp)
            },
            // The matched thunk (A2.2 holes extension): forcing `Unknown`
            // exposes `Unknown`; no `1 ⊑ r` constraint is emitted.
            | ValueType::Unknown => finish_comp(ctx, CompType::Unknown, dir).map(return_comp),
            | other => Err(TypeError::ShapeMismatch {
                expected: text::SHAPE_THUNK,
                actual: Ty::Value(other),
            }),
        },
        // Rule RecordProj⇑ (ADR-45 D4): the record inferred. Look up the field
        // in its record type and deliver `F A`; a matched-`Unknown` record
        // projects `Unknown`; a record lacking the field is stuck (carrying the
        // retained term, as the checker); a non-record is a shape mismatch.
        // The image of `rule_record_proj`.
        | Frame::RecordProj { record, label, dir } => match expect_value(ty)? {
            | ValueType::Record(fields) => match fields.get(&label) {
                | Some(field_ty) => {
                    finish_comp(ctx, CompType::returner(field_ty.as_ref().clone()), dir)
                        .map(return_comp)
                },
                | None => Err(TypeError::StuckExpr {
                    expr: Term::Comp(Comp::RecordProj { record, label }),
                    hint: text::RECORD_NO_FIELD,
                }),
            },
            | ValueType::Unknown => finish_comp(ctx, CompType::Unknown, dir).map(return_comp),
            | other => Err(TypeError::ShapeMismatch {
                expected: text::SHAPE_RECORD,
                actual: Ty::Value(other),
            }),
        },
        | Frame::Dup { r, s, dir } => match expect_value(ty)? {
            | ValueType::Thunk(grade, body) => {
                // Conservation `r + s ⊑ g` (the additive accounting `+` of
                // §"Grades").
                let total = r.plus(s);
                if !bool::from(total.leq(grade)) {
                    return Err(TypeError::GradeError {
                        lower: total,
                        upper: grade,
                    });
                }
                let body = Rc::unwrap_or_clone(body);
                // dup's natural type `F (U_r B_v × U_s B_v)`; the Sub rule
                // discharges body subsumption + the reflexive grade match.
                let natural = CompType::returner(ValueType::Prod(
                    Rc::new(ValueType::Thunk(r, Rc::new(body.clone()))),
                    Rc::new(ValueType::Thunk(s, Rc::new(body))),
                ));
                finish_comp(ctx, natural, dir).map(return_comp)
            },
            // The matched thunk (A2.2 holes extension): `dup ?hole` splits at
            // `Unknown` bodies; no grade constraint is emitted.
            | ValueType::Unknown => {
                let natural = CompType::returner(ValueType::Prod(
                    Rc::new(ValueType::Thunk(r, Rc::new(CompType::Unknown))),
                    Rc::new(ValueType::Thunk(s, Rc::new(CompType::Unknown))),
                ));
                finish_comp(ctx, natural, dir).map(return_comp)
            },
            | other => Err(TypeError::ShapeMismatch {
                expected: text::SHAPE_THUNK,
                actual: Ty::Value(other),
            }),
        },
        | Frame::Drop { dir } => match expect_value(ty)? {
            // A thunk of any grade (the `0 ⊑ r` side condition is vacuous on
            // the default carrier) — and the matched `Unknown` scrutinee
            // alike: `F 1` depends on neither grade nor body.
            | ValueType::Thunk(..) | ValueType::Unknown => {
                finish_comp(ctx, CompType::returner(ValueType::Unit), dir).map(return_comp)
            },
            | other => Err(TypeError::ShapeMismatch {
                expected: text::SHAPE_THUNK,
                actual: Ty::Value(other),
            }),
        },
        | Frame::Bind { var, cont, dir } => match expect_comp(ty)? {
            // The bound computation's row is carried into `BindBody` (alongside
            // the bind's direction) and unioned into the result at the pop
            // (A3.2 `+effects`). The continuation is descended in the bind's
            // direction, so the direction is cloned for the frame.
            | CompType::F(payload, row) => {
                ctx.bind(var, Rc::unwrap_or_clone(payload));
                stack.push(Frame::BindBody {
                    bound_row: row,
                    dir: dir.clone(),
                });
                Ok(Control::DescendComp { comp: cont, dir })
            },
            // The matched returner (A2.2 holes extension): binding an
            // `Unknown` computation binds the variable at `Unknown`, with an
            // empty bound row.
            | CompType::Unknown => {
                ctx.bind(var, ValueType::Unknown);
                stack.push(Frame::BindBody {
                    bound_row: EffectRow::EMPTY,
                    dir: dir.clone(),
                });
                Ok(Control::DescendComp { comp: cont, dir })
            },
            | other => Err(TypeError::ShapeMismatch {
                expected: text::SHAPE_RETURNER,
                actual: Ty::Comp(other),
            }),
        },
        | Frame::BindBody { bound_row, dir } => {
            // Union the bound row into the continuation's result, then finish
            // against the bind's direction (the row-subsumption the union
            // requires; a checking-mode bind decides `ε_bound ∪ ε ⊆ ε`). Both
            // fallible checks run *before* the `Γ` restore, so a failure reports
            // the pre-pop `Γ`.
            let cont_ty = expect_comp(ty)?;
            let combined = combine_bind_row(&bound_row, cont_ty)?;
            let finished = finish_comp(ctx, combined, dir)?;
            ctx.unbind();
            Ok(return_comp(finished))
        },
        | Frame::CaseArm2 => {
            ctx.unbind();
            Ok(Control::Return { ty })
        },
        | Frame::CaseScrut {
            arm_fst,
            arm_snd,
            expected,
        } => match expect_value(ty)? {
            | ValueType::Sum(lhs, rhs) => {
                let (fst_name, fst_body) = arm_fst;
                ctx.bind(fst_name, Rc::unwrap_or_clone(lhs));
                stack.push(Frame::CaseArm1 {
                    arm_snd,
                    snd_ty: Rc::unwrap_or_clone(rhs),
                    expected: expected.clone(),
                });
                Ok(Control::DescendComp {
                    comp: Rc::unwrap_or_clone(fst_body),
                    dir: Dir::Check(expected),
                })
            },
            // The matched sum (A2.2 holes extension): an `Unknown` scrutinee
            // binds both arms at `Unknown`.
            | ValueType::Unknown => {
                let (fst_name, fst_body) = arm_fst;
                ctx.bind(fst_name, ValueType::Unknown);
                stack.push(Frame::CaseArm1 {
                    arm_snd,
                    snd_ty: ValueType::Unknown,
                    expected: expected.clone(),
                });
                Ok(Control::DescendComp {
                    comp: Rc::unwrap_or_clone(fst_body),
                    dir: Dir::Check(expected),
                })
            },
            // A `case` on an identity type: the reserved here-pattern fragment
            // (ADR-76) — the without-k diagnostic, lock-step with the
            // recursive checker's arm.
            | scrut_id @ ValueType::Path { .. } => Err(TypeError::ShapeMismatch {
                expected: text::CASE_ON_PATH_WITHOUT_K,
                actual: Ty::Value(scrut_id),
            }),
            | other => Err(TypeError::ShapeMismatch {
                expected: text::SHAPE_SUM,
                actual: Ty::Value(other),
            }),
        },
        | Frame::CaseArm1 {
            arm_snd,
            snd_ty,
            expected,
        } => {
            expect_comp(ty)?;
            ctx.unbind();
            let (snd_name, snd_body) = arm_snd;
            ctx.bind(snd_name, snd_ty);
            stack.push(Frame::CaseArm2);
            Ok(Control::DescendComp {
                comp: Rc::unwrap_or_clone(snd_body),
                dir: Dir::Check(expected),
            })
        },
        // Rule `DataCase`⇓ (ADR-80 Decision 3): the scrutinee inferred. Require a
        // declared-data (or matched `Unknown`) shape, then begin the arm walk —
        // bind the first arm's payload binder at `Unknown` and descend it, or
        // return the expectation directly when there are no arms (the absurd
        // match). Lock-step with the recursive checker's `rule_data_case`.
        | Frame::DataCaseScrut { arms, expected } => match expect_value(ty)? {
            | ValueType::Data { .. } | ValueType::Unknown => {
                let mut iter = arms.into_iter();
                match iter.next() {
                    | Some((binder, body)) => {
                        ctx.bind(binder, ValueType::Unknown);
                        stack.push(Frame::DataCaseArm {
                            remaining: iter.collect(),
                            expected: expected.clone(),
                        });
                        Ok(Control::DescendComp {
                            comp: Rc::unwrap_or_clone(body),
                            dir: Dir::Check(expected),
                        })
                    },
                    | None => Ok(return_comp(expected)),
                }
            },
            | other => Err(TypeError::ShapeMismatch {
                expected: text::SHAPE_DATA,
                actual: Ty::Value(other),
            }),
        },
        // The current arm checked (sort-checked before any `Γ` restore);
        // restore its `Unknown` payload binder, then descend the next arm or
        // return the expectation (rule `DataCase`⇓; ADR-80).
        | Frame::DataCaseArm {
            remaining,
            expected,
        } => {
            expect_comp(ty)?;
            ctx.unbind();
            let mut iter = remaining.into_iter();
            match iter.next() {
                | Some((binder, body)) => {
                    ctx.bind(binder, ValueType::Unknown);
                    stack.push(Frame::DataCaseArm {
                        remaining: iter.collect(),
                        expected: expected.clone(),
                    });
                    Ok(Control::DescendComp {
                        comp: Rc::unwrap_or_clone(body),
                        dir: Dir::Check(expected),
                    })
                },
                | None => Ok(return_comp(expected)),
            }
        },
        // Rule ListCase⇓ (ADR-40 D4): the scrutinee inferred. Destructure its
        // `List A` into the `cons` binder types (`head : A`, `tail : List A`;
        // the matched `Unknown` binds both at `Unknown`), then descend the `nil`
        // arm (no binders yet — it does not see `head`/`tail`).
        | Frame::ListCaseScrut {
            nil,
            head,
            tail,
            cons,
            expected,
        } => {
            let (head_ty, tail_ty): (ValueType, ValueType) = match expect_value(ty)? {
                | ValueType::List(elem) => (elem.as_ref().clone(), ValueType::List(elem)),
                | ValueType::Unknown => (ValueType::Unknown, ValueType::Unknown),
                | other => {
                    return Err(TypeError::ShapeMismatch {
                        expected: text::SHAPE_LIST,
                        actual: Ty::Value(other),
                    });
                },
            };
            stack.push(Frame::ListCaseNil {
                head,
                head_ty,
                tail,
                tail_ty,
                cons,
                expected: expected.clone(),
            });
            Ok(Control::DescendComp {
                comp: Rc::unwrap_or_clone(nil),
                dir: Dir::Check(expected),
            })
        },
        // The `nil` arm checked (sort-checked before any `Γ` mutation); bind the
        // `cons` arm's `head`/`tail` and descend its body.
        | Frame::ListCaseNil {
            head,
            head_ty,
            tail,
            tail_ty,
            cons,
            expected,
        } => {
            expect_comp(ty)?;
            ctx.bind(head, head_ty);
            ctx.bind(tail, tail_ty);
            stack.push(Frame::ListCaseCons);
            Ok(Control::DescendComp {
                comp: Rc::unwrap_or_clone(cons),
                dir: Dir::Check(expected),
            })
        },
        // Rules `SplitMotive`⇑ / Split⇓ pop (ADR-82): the scrutinee inferred its
        // `Prod` / `Σ` shape. Bind `p` / `q`, compute the body's checked-against
        // type and the (body-independent) answer via [`split_expectations`] /
        // [`split_unknown_expectations`] — mirroring the recursive checker
        // exactly — and descend the body; the answer rides the
        // [`Frame::SplitBody`] frame (the [`Frame::WalkScrut`] discipline).
        | Frame::Split {
            fst_name,
            snd_name,
            motive,
            scrut,
            body,
            dir,
        } => {
            let (fst_ty, snd_ty, body_expected, result): (
                ValueType,
                ValueType,
                Dir<CompType>,
                CompType,
            ) = match expect_value(ty)? {
                | ValueType::Prod(lhs, rhs) => {
                    let (body_expected, result) =
                        split_expectations(motive.as_deref(), &dir, &fst_name, &snd_name, &scrut);
                    (
                        Rc::unwrap_or_clone(lhs),
                        Rc::unwrap_or_clone(rhs),
                        body_expected,
                        result,
                    )
                },
                // Rule Sigma elimination (ADR-81 feature 2): the first binder
                // gets the head `A`, the second the substituted tail `B[p/x]`
                // (the first binder variable for the bound `x`).
                | ValueType::Sigma {
                    fst: head,
                    binder,
                    snd: tail,
                } => {
                    let tail_ty = gandr_core_term::identity::subst_valuetype(
                        &tail,
                        gandr_core_term::boundary::NameRef::from(binder.as_str()),
                        &Value::var(&fst_name),
                    );
                    let (body_expected, result) =
                        split_expectations(motive.as_deref(), &dir, &fst_name, &snd_name, &scrut);
                    (Rc::unwrap_or_clone(head), tail_ty, body_expected, result)
                },
                // The matched product (A2.2 holes extension): an `Unknown`
                // scrutinee binds both components at `Unknown`.
                | ValueType::Unknown => {
                    let (body_expected, result) =
                        split_unknown_expectations(motive.as_deref(), &dir);
                    (
                        ValueType::Unknown,
                        ValueType::Unknown,
                        body_expected,
                        result,
                    )
                },
                | other => {
                    return Err(TypeError::ShapeMismatch {
                        expected: text::SHAPE_PROD,
                        actual: Ty::Value(other),
                    });
                },
            };
            ctx.bind(fst_name, fst_ty);
            ctx.bind(snd_name, snd_ty);
            stack.push(Frame::SplitBody { result, dir });
            Ok(Control::DescendComp {
                comp: body,
                dir: body_expected,
            })
        },
        // Rule `SplitMotive`⇑ / Split⇓ pop 2 (ADR-82): the body checked; restore
        // `Γ` (both binders) and finish the **precomputed** answer against the
        // direction — never the body's echo (the [`Frame::WalkBase`]
        // discipline).
        | Frame::SplitBody { result, dir } => {
            expect_comp(ty)?;
            ctx.unbind();
            ctx.unbind();
            finish_comp(ctx, result, dir).map(return_comp)
        },
        // A list-case's `cons` arm restores `Γ` (two binders) and propagates the
        // body type unchanged (ADR-40 D4).
        | Frame::ListCaseCons => {
            ctx.unbind();
            ctx.unbind();
            Ok(Control::Return { ty })
        },
        | Frame::With1 {
            second,
            second_expected,
        } => {
            let fst_ty = expect_comp(ty)?;
            stack.push(Frame::With2 { first: fst_ty });
            Ok(Control::DescendComp {
                comp: second,
                dir: Dir::Check(second_expected),
            })
        },
        | Frame::With2 { first } => {
            let snd_ty = expect_comp(ty)?;
            Ok(return_comp(CompType::With(Rc::new(first), Rc::new(snd_ty))))
        },
        | Frame::Prj { side, dir } => match expect_comp(ty)? {
            | CompType::With(lhs, rhs) => {
                finish_comp(ctx, pick(side, &lhs, &rhs), dir).map(return_comp)
            },
            // The matched with (A2.2 holes extension): projecting from an
            // `Unknown` target yields `Unknown`.
            | CompType::Unknown => finish_comp(ctx, CompType::Unknown, dir).map(return_comp),
            | other => Err(TypeError::ShapeMismatch {
                expected: text::SHAPE_WITH,
                actual: Ty::Comp(other),
            }),
        },
        // Rule Op⇑ (A3.2 `+effects`): the payload checked; yield the
        // singleton-row returner `F^⟨E⟩ B_op`.
        | Frame::Perform { reply, row, dir } => {
            expect_value(ty)?;
            finish_comp(ctx, CompType::returner_eff(reply, row), dir).map(return_comp)
        },
        // Rule Handle⇓ (A3.2 `+effects`): the handled computation inferred.
        // The fallible returner-shape check runs before any `Γ` mutation; then
        // compute the residual, bind the return binder, and descend `t_ret`.
        | Frame::HandleScrut {
            answer,
            sig_name,
            ret,
            ops,
        } => {
            let (eps_t, payload_a): (EffectRow, ValueType) = match expect_comp(ty)? {
                | CompType::F(payload, row) => (row, Rc::unwrap_or_clone(payload)),
                | CompType::Unknown => (EffectRow::EMPTY, ValueType::Unknown),
                | other => {
                    return Err(TypeError::ShapeMismatch {
                        expected: text::SHAPE_RETURNER,
                        actual: Ty::Comp(other),
                    });
                },
            };
            let residual = eps_t.without(gandr_core_term::boundary::EffectSignatureName::from(
                sig_name.as_str(),
            ));
            let (ret_var, ret_body) = ret;
            ctx.bind(ret_var, payload_a);
            let body_dir = Dir::Check(answer.clone());
            stack.push(Frame::HandleRet {
                answer,
                residual,
                ops,
            });
            Ok(Control::DescendComp {
                comp: Rc::unwrap_or_clone(ret_body),
                dir: body_dir,
            })
        },
        // The return clause checked against the answer; restore the return
        // binder, then start the first operation clause or finish the handle.
        | Frame::HandleRet {
            answer,
            residual,
            ops,
        } => {
            expect_comp(ty)?;
            ctx.unbind();
            handle_advance(answer, residual, ops, stack, ctx)
        },
        // An operation clause checked against the answer; restore `p` and `k`,
        // then advance to the next clause or finish.
        | Frame::HandleOp {
            answer,
            residual,
            rest,
        } => {
            expect_comp(ty)?;
            ctx.unbind();
            ctx.unbind();
            handle_advance(answer, residual, rest, stack, ctx)
        },
        // Rule Resume⇑ (A3.3 `+control`): the reified stack inferred. Destructure
        // its `Stk(B, C)` (matched `Unknown` feeds against `Unknown`, delivers
        // `Unknown`), then descend the fed computation in `Check(B)`.
        | Frame::ResumeFn { comp, dir } => {
            let value_type = expect_value(ty)?;
            let (consumed, delivered) = stk_components(value_type)?;
            stack.push(Frame::ResumeArg {
                result: delivered,
                dir,
            });
            Ok(Control::DescendComp {
                comp,
                dir: Dir::Check(consumed),
            })
        },
        // Rule Resume⇑: the fed computation checked against `B`; finish the
        // delivered answer `C` against the resume's direction.
        | Frame::ResumeArg { result, dir } => {
            expect_comp(ty)?;
            finish_comp(ctx, result, dir).map(return_comp)
        },
        // Rule Reset⇓ (A3.3 `+control`): the body checked against the answer `C`.
        // The fallible sort check runs before the ambient restore, then `reset`
        // returns the body's *checked* type (transparent; a consistent subtype of
        // `C`, equal to `C` only when the body echoes it — the recursive
        // checker's `rule_reset` returns the same body type, ADR-9 lock-step).
        | Frame::ResetBody { saved } => {
            let body_ty = expect_comp(ty)?;
            *ambient = saved;
            Ok(return_comp(body_ty))
        },
        // Rule Shift⇓ (A3.3 `+control`): the body checked against the answer `C`;
        // restore the continuation binder `k` and deliver the captured type `B`.
        // The fallible sort check runs before the `Γ` restore.
        | Frame::ShiftBody { captured } => {
            expect_comp(ty)?;
            ctx.unbind();
            Ok(return_comp(captured))
        },
        // Rule Fix⇓: the body checked against the recursion's own type; restore
        // the self-reference binder and deliver that type unchanged. The
        // fallible sort check runs before the context restore, as `ShiftBody`
        // does.
        | Frame::FixBody { recursive } => {
            expect_comp(ty)?;
            ctx.unbind();
            Ok(return_comp(recursive))
        },
        // Rule Reify (the stack-judgment walk; A3.3 `+control`): an argument
        // frame's value checked against the consumed function's argument type;
        // continue the walk from the function's result type.
        | Frame::StkArg {
            rest,
            binder,
            arg,
            result_input,
            dir,
        } => {
            expect_value(ty)?;
            // The frame supplied the argument, so a dependent codomain closes
            // here before the walk continues from it.
            let applied =
                instantiate_codomain(binder.as_deref().map(NameRef::from), &result_input, &arg);
            walk_stack(rest, applied, dir, stack, ctx)
        },
        // Rule Reify (the stack-judgment walk; A3.3 `+control`): a bind frame's
        // continuation inferred; fold the consumed row in (via
        // [`combine_bind_row`], as [`Frame::BindBody`]) — the fallible row check
        // runs before the binder restore — then continue the walk.
        | Frame::StkBind {
            rest,
            consumed_row,
            dir,
        } => {
            let cont_ty = expect_comp(ty)?;
            let sequenced = combine_bind_row(&consumed_row, cont_ty)?;
            ctx.unbind();
            walk_stack(rest, sequenced, dir, stack, ctx)
        },
        // Rule Here pop (ADR-76): the witness inferred `A`; the natural type is
        // `Path A v v` (both endpoints the stored witness), finished against the
        // direction — the image of `Frame::Inj`.
        | Frame::Here { witness, dir } => {
            let carrier = expect_value(ty)?;
            let natural = ValueType::path(carrier, witness.clone(), witness);
            finish_value(ctx, natural, dir).map(return_value)
        },
        // Rule Walk pop 1 (ADR-76): the scrutinee inferred its `Path A a b` shape.
        // Compute the diagonal base's expected type and the (base-independent)
        // result type, bind the diagonal binder, and descend the base body — the
        // image of `Frame::CaseScrut`.
        | Frame::WalkScrut {
            motive,
            base,
            scrut,
            dir,
        } => {
            let (carrier, base_expected, result): (ValueType, CompType, CompType) =
                match expect_value(ty)? {
                    | ValueType::Path { ty, lhs, rhs } => {
                        let carrier = Rc::unwrap_or_clone(ty);
                        let diagonal = base_diagonal_type(&motive, &base.x);
                        let result =
                            motive_result_type(&motive, lhs.as_ref(), rhs.as_ref(), &scrut);
                        (carrier, diagonal, result)
                    },
                    | ValueType::Unknown => {
                        (ValueType::Unknown, CompType::Unknown, CompType::Unknown)
                    },
                    | other => {
                        return Err(TypeError::ShapeMismatch {
                            expected: text::SHAPE_PATH,
                            actual: Ty::Value(other),
                        });
                    },
                };
            ctx.bind(base.x.clone(), carrier);
            stack.push(Frame::WalkBase { result, dir });
            Ok(Control::DescendComp {
                comp: Rc::unwrap_or_clone(base.body),
                dir: Dir::Check(base_expected),
            })
        },
        // Rule Walk pop 2 (ADR-76): the base body checked; restore `Γ` and finish
        // the precomputed result against the direction — the image of
        // `Frame::SplitBody` followed by the rule's `finish`.
        | Frame::WalkBase { result, dir } => {
            expect_comp(ty)?;
            ctx.unbind();
            finish_comp(ctx, result, dir).map(return_comp)
        },
    }
}

/// Helper: projects a returned type to the value sort.
///
/// By construction every frame is resumed at the sort it suspended on, so
/// this cannot fail on states reachable from [`State::new_value`] /
/// [`State::new_comp`]; the error keeps the machine total.
fn expect_value(ty: Ty) -> Result<ValueType, TypeError>
{
    match ty {
        | Ty::Value(value_ty) => Ok(value_ty),
        | Ty::Comp(_) => Err(TypeError::ShapeMismatch {
            expected: text::SHAPE_VALUE,
            actual: ty,
        }),
    }
}

/// Helper: shorthand for a value `Return` control.
fn return_value(ty: ValueType) -> Control
{
    Control::Return { ty: Ty::Value(ty) }
}

/// Helper: projects a returned type to the computation sort.
///
/// See [`expect_value`] for why this is unreachable on well-formed states.
fn expect_comp(ty: Ty) -> Result<CompType, TypeError>
{
    match ty {
        | Ty::Comp(comp_ty) => Ok(comp_ty),
        | Ty::Value(_) => Err(TypeError::ShapeMismatch {
            expected: text::SHAPE_COMP,
            actual: ty,
        }),
    }
}

/// Helper: shorthand for a computation `Return` control.
fn return_comp(ty: CompType) -> Control
{
    Control::Return { ty: Ty::Comp(ty) }
}

/// Advances a handler past its return clause or an operation clause (rule
/// Handle, A3.2 `+effects`): descends the next operation clause — binding its
/// payload `p` and resumption `k : Stk(F^ε B_i, F^ε C)` (the *deep* discipline,
/// the shared [`resume_stack_type`]) — or, when no clauses remain, finishes the
/// handle's natural type `F^{ε_t ∖ E} C` against the answer (the soundness leg
/// `ε_t ∖ E ⊆ ε` via [`finish_comp`]; the matched-hole answer absorbs any
/// residual).
///
/// Shared by the [`Frame::HandleRet`] and [`Frame::HandleOp`] pops so the
/// machine drives the variable-arity clause fold exactly as the recursive
/// checker's loop does (the step-for-step contract): clauses are consumed in
/// the signature's canonical operation order.
fn handle_advance(
    answer: CompType,
    residual: EffectRow,
    mut remaining: Vec<(EffectOp, OpClause)>,
    stack: &mut Vec<Frame>,
    ctx: &mut Ctx,
) -> Result<Control, TypeError>
{
    if remaining.is_empty() {
        let natural = handle_natural_type(&answer, residual);
        return finish_comp(ctx, natural, Dir::Check(answer)).map(return_comp);
    }
    let (op_def, clause) = remaining.remove(0);
    let resume_ty = resume_stack_type(&answer, op_def.reply().clone());
    ctx.bind(clause.payload, op_def.payload().clone());
    ctx.bind(clause.resume, resume_ty);
    let body = Rc::unwrap_or_clone(clause.body);
    let body_dir = Dir::Check(answer.clone());
    stack.push(Frame::HandleOp {
        answer,
        residual,
        rest: remaining,
    });
    Ok(Control::DescendComp {
        comp: body,
        dir: body_dir,
    })
}

/// Walks a reified stack `K` forward from the consumed type `input`, driving
/// the stack-judgment frames (rule Reify, `effects-control-shell.md` §2.1; A3.3
/// `+control`). The structural frames are consumed **inline** — the empty stack
/// finishes the reified type `Stk(B, output)` against the original direction
/// `dir` (the inlined Sub rule's `output <: C` is the covariant-`C` variance
/// leg), and a projection frame continues the walk with no trace event — while
/// an argument frame descends its value (`Check` against the consumed
/// function's argument) and a bind frame binds the consumed payload and
/// descends its continuation (`Infer`). Only those sub-term descents log
/// control events, so a multi-frame stack with no leading sub-term (`prjᵢ :: …
/// :: ε`) is consumed in one machine step and the trace matches the recursive
/// checker's `stack_infer` exactly.
///
/// Shared by the initial `stk K` step ([`step_value`]) and the
/// [`Frame::StkArg`] / [`Frame::StkBind`] pops; the per-frame type destructures
/// are the `gandr_core_checker::judgements::stack` helpers shared with the
/// checker, so the two faces cannot drift.
fn walk_stack(
    stack: Stack,
    input: CompType,
    dir: Dir<ValueType>,
    frames: &mut Vec<Frame>,
    ctx: &mut Ctx,
) -> Result<Control, TypeError>
{
    let mut current = stack;
    let mut current_input = input;
    loop {
        match current {
            | Stack::Empty => {
                // The synthesized output is `current_input`; rebuild `Stk(B,
                // output)` (B from the expectation, `Unknown` for the matched-
                // hole stack) and finish against the original direction.
                let constructed = match dir {
                    | Dir::Check(ValueType::Stk(ref b, _)) => {
                        ValueType::stk(b.as_ref().clone(), current_input)
                    },
                    | _ => ValueType::stk(CompType::Unknown, current_input),
                };
                return finish_value(ctx, constructed, dir).map(return_value);
            },
            | Stack::Prj(side, rest) => {
                // A projection has no sub-term: continue the walk in this step.
                current_input = with_component(current_input, side)?;
                current = Rc::unwrap_or_clone(rest);
            },
            | Stack::Arg(value, rest) => {
                let (binder, arg_ty, result_input) = arrow_components(current_input)?;
                let value = Rc::unwrap_or_clone(value);
                frames.push(Frame::StkArg {
                    rest: Rc::unwrap_or_clone(rest),
                    binder,
                    arg: value.clone(),
                    result_input,
                    dir,
                });
                return Ok(Control::DescendValue {
                    value,
                    dir: Dir::Check(arg_ty),
                });
            },
            | Stack::Bind(name, cont, rest) => {
                let (payload, consumed_row) = returner_components(current_input)?;
                ctx.bind(name, payload);
                frames.push(Frame::StkBind {
                    rest: Rc::unwrap_or_clone(rest),
                    consumed_row,
                    dir,
                });
                return Ok(Control::DescendComp {
                    comp: Rc::unwrap_or_clone(cont),
                    dir: Dir::Infer,
                });
            },
        }
    }
}

/// Machine-only depth tests: the frame stack lives on the heap, so the
/// machine's nesting bound is memory, not the host call stack. These terms
/// are far beyond what the recursive
/// [`gandr_core_checker::judgements::checker`] could survive (its recursion
/// would need hundreds of megabytes of call stack), so they are deliberately
/// *not* part of the conformance pairing.
#[cfg(test)]
mod tests
{
    use super::*;

    /// Nesting depth for the machine-only test.
    ///
    /// The term is a `bind` chain: nested through the *continuation*, so the
    /// machine accumulates one [`Frame::BindBody`] per level while the result
    /// type stays the shallow `F Unit`.
    ///
    /// Drop constraint: the AST's *derived* `Drop` recurses one call per
    /// `Rc` link, and a plain `drop` of a chain this deep overflows an 8 MiB
    /// thread stack (measured: ~50 000 survives, 100 000 aborts, debug
    /// build). The test stays safe only because no such drop ever happens:
    /// the chain's sole owner is moved into the machine, which dismantles it
    /// level by level, and the [`Trace`] — whose `Descend` entry at each
    /// level holds the last surviving reference to that level's suffix —
    /// drops front-to-back (`Vec` drops elements in order), releasing one
    /// node per entry. Keeping a second owner of the full term alive past
    /// the run (e.g. a pre-run `clone` for a later assertion) would
    /// reintroduce the deep recursive drop and abort the test.
    const DEPTH: usize = 100_000;

    /// A deeply nested `bind` chain types successfully on the machine alone.
    #[test]
    fn deeply_nested_bind_chain_types_on_the_machine()
    {
        let mut comp = Comp::ret(Value::Unit);
        for _ in 0 .. DEPTH {
            comp = Comp::bind(Comp::ret(Value::Unit), "x", comp);
        }
        let (result, trace) = run_comp(Ctx::new(), comp, Dir::Infer);
        assert_eq!(
            result,
            Ok(Ty::Comp(CompType::returner(ValueType::Unit))),
            "a {DEPTH}-deep bind chain must infer F Unit on the heap-stack machine"
        );
        assert!(
            trace.len() > DEPTH,
            "the trace must record at least one control register per level"
        );
        // Release the trace explicitly front-to-back: each `Descend` entry
        // holds the last reference to one suffix of the chain (see [`DEPTH`]),
        // so consuming in order frees one node at a time instead of recursing.
        for control in trace {
            drop(control);
        }
    }
    /// A `Descend`-position failure captures the offending sub-term as the
    /// control register, with an empty partial derivation and the failure-point
    /// `Γ`.
    #[test]
    fn failure_state_carries_the_offending_descend()
    {
        let (error, failure) = run_to_failure(State::new_value(
            Ctx::new(),
            Value::var("ghost"),
            Dir::Infer,
        ))
        .expect("an unbound variable must fail to type");
        assert_eq!(error, TypeError::UnboundVariable {
            name: "ghost".to_owned()
        });
        assert_eq!(
            failure.control(),
            &Control::DescendValue {
                value: Value::var("ghost"),
                dir: Dir::Infer,
            },
            "the control register must be the offending sub-term"
        );
        assert!(
            failure.stack().is_empty(),
            "a top-level descend failure has no pending obligations"
        );
        assert!(failure.ctx().lookup("ghost").is_none());
    }

    /// A `Return`-position failure restores the failing frame to the top of the
    /// partial derivation and carries `Γ` at the failure point — here the inner
    /// binder `x : 1` is still in scope, and the enclosing [`Frame::Abs`] (with
    /// its binder name) sits below the failing [`Frame::AppFn`].
    #[test]
    fn failure_state_preserves_failing_frame_and_ctx()
    {
        // `λx:1. (ret x) x`: the head `ret x` infers `F 1`, which is not an
        // arrow, so the `AppFn` pop fails — inside the lambda body, so `Γ` is
        // non-empty and a frame stack is pending.
        let body = Comp::app(Comp::ret(Value::var("x")), Value::var("x"));
        let lam = Comp::lam_ann("x", ValueType::Unit, body);
        let (error, failure) = run_to_failure(State::new_comp(Ctx::new(), lam, Dir::Infer))
            .expect("applying a non-arrow must fail to type");

        assert_eq!(error, TypeError::ShapeMismatch {
            expected: text::SHAPE_ARROW,
            actual: Ty::Comp(CompType::returner(ValueType::Unit)),
        });
        assert_eq!(
            failure.control(),
            &Control::Return {
                ty: Ty::Comp(CompType::returner(ValueType::Unit)),
            },
            "the control register is the value propagating into the failing frame"
        );
        assert!(
            matches!(failure.stack().last(), Some(Frame::AppFn { .. })),
            "the failing frame must be restored to the top of the partial derivation"
        );
        assert!(
            matches!(
                failure.stack().first(),
                Some(Frame::Abs { var, .. }) if var == "x"
            ),
            "the enclosing abstraction frame must carry its binder name"
        );
        assert_eq!(
            Some(&ValueType::Unit),
            failure.ctx().lookup("x"),
            "Γ is carried at the failure point, not restored: x : 1 is still bound"
        );
    }

    /// Drives a state to its first failure, returning the error and the
    /// captured [`FailureState`], or [`None`] if the run completes.
    fn run_to_failure(state: State) -> Option<(TypeError, FailureState)>
    {
        let mut current = state;
        loop {
            match step(current) {
                | Outcome::Step(next) => current = next,
                | Outcome::Done(_) => return None,
                | Outcome::Error {
                    error,
                    state: failure,
                } => return Some((error, failure)),
            }
        }
    }
}

#[cfg(test)]
mod arena_boundary_tests
{
    use alloc::rc::Rc;

    use gandr_core_term::syntax::Comp;
    use gandr_core_term::syntax::Term;
    use gandr_core_term::syntax::Value;

    use super::diagnostic_abs_term;

    /// The unannotated-abstraction stuck diagnostic is a public compatibility
    /// readback boundary: it reconstructs the legacy term only after routing
    /// the lambda body through `FlatArena`.
    #[test]
    fn abs_stuck_diagnostic_reads_back_through_arena_bridge()
    {
        let body = Rc::new(Comp::ret(Value::Unit));
        let term = diagnostic_abs_term("x".to_owned(), Rc::clone(&body));

        assert_eq!(term, Term::Comp(Comp::Abs("x".to_owned(), None, body)));
    }
}
