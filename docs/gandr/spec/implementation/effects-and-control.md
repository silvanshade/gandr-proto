# Effects, control, and first-class stacks

This document owns two constructions that are usually built separately and are **one mechanism here**: an effect system on the returner with handlers, and the internalization of the evaluation context as a value.

They are one mechanism because a handler clause's resumption **is** a reified stack.
Write the handler rule with that in mind and delimited control is not a second feature to add later — it is the same feature under a different surface idiom.

The type formers this document gives rules for are declared in [[type-system]]: the effect-graded returner and the reified stack, with its variance.
This document carries the terms, the judgments, the interaction discipline, and the dynamics.

## What is built

| feature                                    | status                                                                                             |
| ------------------------------------------ | -------------------------------------------------------------------------------------------------- |
| effect signatures and operations           | **built** — `gandr-core-checker`'s `effect` module, inline-carried and environment-free            |
| the effect row on the returner             | **built as a sealed concrete set**, not as an open row with a row variable                         |
| the operation rule                         | **built** — `Perform`, with the operation's signature carried on the node                          |
| the handle rule                            | **built, deep only** — shallow handling is designed and absent                                     |
| row arithmetic                             | **built bottom-up** — union at sequencing, subtraction at handling, inclusion as the subtyping leg |
| row unification as a solver domain         | **not built** — nothing emits a row constraint, because rows are closed                            |
| the ambient-ability surface                | **not built** — the row kernel is the only presentation                                            |
| the stack judgment                         | **built, three frames plus the empty one** — the type-application frame awaits polymorphism        |
| the reified stack type and its elimination | **built** — `Stk(B, C)`, with reification check-only and resumption an inference form              |
| reset and shift with answer typing         | **built** — the ambient answer register is the v0 form of the answer-modifying generalization      |
| one-shot linearity and unwind obligations  | **not built** — a reified stack is not resident in the linear zone, so nothing is enforced         |
| the sequential dynamics                    | **built, by a different machine** — see [[#Dynamics]]                                              |
| the process soup                           | **not built**                                                                                      |
| the focused-reduction optimization seam    | **not built**                                                                                      |

**Two of those rows are the load-bearing divergences**, and both are stated in full below: the row is closed where the design's is open ([[#The row as built]]), and the stack carries no obligations where the design's owns them ([[#One-shot linearity, and what it resolves]]).

## The effect calculus

Effects on the returner are the **symmetric completion of grades on the thunk**: a grade says how often a computation may be run, and a row says what it does when it is.

### The grammar

```text
effect signatures   E ::= { opᵢ : Aᵢ ↠ Bᵢ }        -- op: payload Aᵢ, reply Bᵢ
effect rows         ε ::= ⟨⟩ | ⟨E | ε⟩ | ρ          -- ρ a row variable, for polymorphism

computation types   B ::= F^ε A | A → B | …        -- F A ≡ F^⟨⟩ A
```

**The pure returner is the empty-row case rather than a separate former**, which is what keeps the effect extension from being a second type system beside the first.

### The operation and handle rules

```text
op ∈ E      Γ ⊢ v ⇓ A_op
─────────────────────────────────── (Op)
Γ; Σ ⊢ perform op v ⇑ F^⟨E|ε⟩ B_op

Γ; Σ₁ ⊢ t ⇑ F^⟨E|ε⟩ A
Γ, x:A; Σ₂ ⊢ t_ret ⇓ F^ε C
Γ, p:Aᵢ, k:Stk(F^ε Bᵢ, F^ε C); Σ₂ ⊢ tᵢ ⇓ F^ε C      for each opᵢ ∈ E
──────────────────────────────────────────────────────── (Handle)
Γ; Σ₁, Σ₂ ⊢ handle t { ret x ⇒ t_ret | opᵢ p k ⇒ tᵢ } ⇓ F^ε C
```

**Read the handle rule's third premise carefully, because it is the whole design.** The clause binder `k` has the reified-stack type, so the resumption a handler receives is a first-class value of the value layer, not a privileged closure the rule invents.
That is why handlers and control operators are one mechanism here.

The construction descends from the algebraic-effects line [@plotkin-power-2002-notions-of-computation; @plotkin-pretnar-2013-handling-algebraic-effects], whose tutorial presentation and reference implementation are [@pretnar-2015-algebraic-effects-tutorial; @bauer-pretnar-2015-programming-algebraic-effects], and the row presentation follows the row-polymorphic effect line [@leijen-2014-koka-row-effects].

### Deep and shallow

The rule as written is **deep**: the handler reinstalls itself in the resumption, so a resumed computation is still handled.
A **shallow** variant differs in one type — its clause receives `k : Stk(F^⟨E|ε⟩ Bᵢ, …)`, with the handled signature still in the resumption's row, because the handler does not reinstall.

**The design's disposition is to provide deep as the default and shallow as the primitive the deep form desugars from.** The ordering is the substantive part: deriving deep from shallow is a definition, and deriving shallow from deep is not available at all, so making the convenient form primitive would foreclose the other.

### The ambient-ability surface

The row grammar above is the **kernel**, and it is not the surface the design commits to.
The surface presentation is an **ambient ability** in the sense of the Frank line [@lindley-mcbride-mclaughlin-2017-frank]: one implicit row threaded through the judgment and **adjusted** at handling positions, rather than row variables the programmer writes.

**This is the same ambient-index pattern the tree already uses for worlds**, which is the argument for it: a reader who has met one has met the other, and a second ambient index costs no new concept.

### Rows as a solver domain

Row equality constraints between two rows form **their own solver domain**, and the design's claim about it is specific and load-bearing: row unification in the standard style needs **no backtracking**.

That matters because the solver's trail exists for the set-operation search, and a domain that never backtracks can be decided without touching it — so adding effects does not enlarge the search the checker performs.

The algorithm the claim refers to is the classical one for extensible records and variants, due to Rémy, and **the held paper carries the property by construction rather than by name** [@remy-1989-typechecking-records].
Its section 2 makes record and variant types **kinded regular trees**, and unification on them ordinary first-order unification — "just the usual algorithm where the occur test has been removed" — returning a **most general unifier**.
Milner's `W` then applies unchanged, with soundness, completeness, and **principal sort schemes** as that section's Theorems 1 to 3.
A domain whose problems have principal solutions has no choice point, so there is nothing for a trail to record and nothing to revisit; that is the no-backtracking property, and it is **derived here rather than quoted, because the paper never uses the word**.

**Two parts of the claim that paper does not reach, and the surviving mark covers exactly those two.** It does not present the row-unification algorithm itself — the field-by-field decomposition against a row variable — but delegates to the standard regular-tree procedure, recalling its results without proof and attributing most of them to Huet's thesis.
And its results are proved for a **finite** label set only: its opening section says an extension to a denumerable set of labels "is suggested", and its closing section says kinded regular trees "could be extended" to finitely generated ones, with a unification theorem one "could prove" — which is the very extension the compact row form with a row variable needs, and the paper says plainly that the same unproved theorem is what would justify the compact form its own implementation already used.

So the open row's no-backtracking property rests on the finite-label result plus the attribution, not on a read proof.
The work that carries the algorithm in detail is Rémy's INRIA research report, which is **locator-pending and declined for acquisition** ([[#Works cited here that this corpus does not hold]]).

### How effects interact with everything else

| feature    | interaction                                                                                                                                      |
| ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| grades     | **orthogonal** — a graded thunk of an effectful returner separates how often a computation may run from what it does when run                    |
| sessions   | session actions stay **structural**, in the linear zone, and are never effects, because protocol fidelity must not be handleable away            |
| sessions   | but an operation may be **implemented** over a session, by a handler that holds an endpoint                                                      |
| worlds     | rows are **located** — performing an operation requires a handler at the current world, or an explicitly distributed one behind a shared session |
| the solver | row equality is a new domain, decided without backtracking                                                                                       |

**The session row is deliberately two rows**, because the two halves are opposite and both are needed: a session action is not an effect, and yet an effect may be realized by one.
Collapsing them in either direction loses a rule.

### The row as built

The tree carries a **sealed, concrete, finite set** of signatures, and the seal is the design decision rather than an implementation shortcut.

`gandr-core-checker`'s `effect` module makes the row a newtype whose entire cross-module surface is the set signature — empty, singleton, union, subtraction, containment, subset, and enumeration.
The arithmetic is bottom-up: an operation contributes its signature, sequencing takes the union, handling subtracts the handled signature, and the row leg of returner subtyping is inclusion, so **a computation that may do fewer effects is usable where more are allowed**.

**What the seal buys is the divergence's own remedy.** Replacing this row with the open, row-polymorphic form — the row variable of the grammar above — is an edit to one module rather than a rewrite across every rule, precisely because no rule ever sees the representation.

Signatures are **inline-carried and environment-free**: a signature rides directly on the operation and handle nodes, so there is no global effect-declaration form and no signature context, and a row keyed by signature name is well defined because the build maintains one name to one signature.

## First-class stacks

### The stack judgment

Call-by-push-value has a third syntactic category beside values and computations — **stacks**, the evaluation contexts — with its own typing judgment [@levy-cbpv]:

```text
Γ; Σ ⊢ K : B ⇒ C          -- K consumes a B-computation and delivers C

ε                         Γ; Σ ⊢ ε : B ⇒ C   requires B = C     (the empty stack)
v :: K                    consumes A → B given v ⇓ A, then K
(x. u) :: K               consumes F^ε A: binds x:A, runs u, then K
prjᵢ :: K, [A] :: K       for lazy products and for polymorphism
```

**The tree builds four of these**, and the omission is not arbitrary: `gandr-core-checker`'s stack node carries the empty stack, the argument frame, the bind frame, and the projection frame, with the bind frame folding the consumed row into its continuation exactly as the ordinary sequencing rule does.
The type-application frame waits on polymorphism, which is not built.

### Internalizing it as a value type

The design's central move is to make that judgment a **value type**:

```text
A ::= … | Stk(B, C)          -- a reified stack from B to C

Γ; Σ ⊢ K : B ⇒ C                    Γ ⊢ v ⇑ Stk(B, C)    Γ; Σ ⊢ t ⇓ B
────────────────────── (Reify)      ──────────────────────────────────── (Resume)
Γ; Σ ⊢ stk K ⇓ Stk(B, C)            Γ; Σ ⊢ resume v t ⇑ C
```

**This is the precise sense in which a stack crosses the value-computation boundary**: a stack is _consumed_ in the computation layer and _held_ in the value layer, which makes it the exact dual of a thunk, a computation held as a value.

As built, reification is **check-only** against an expected stack type, and resumption is an **inference** form structurally identical to application — the stack value is the principal premise, the fed computation checks against what the stack consumes, and the result is what it delivers.

### Control operators and answer typing

Capture is **delimited**, and a delimiter is typed by its answer:

```text
Γ; Σ ⊢ t ⇓ C
──────────────────────── (Reset)
Γ; Σ ⊢ reset t ⇑ C            -- pushes a delimiter frame carrying the answer C

Γ, k : Stk(B, C); Σ ⊢ t ⇓ C
──────────────────────────── (Shift)
Γ; Σ ⊢ shift k. t ⇓ B          -- captures up to the nearest delimiter, reified as k
```

The delimiter is a prompt in the sense the prompt calculus fixes [@felleisen-1988-first-class-prompts], the answer-type discipline is the classical one [@danvy-filinski-1990-abstracting-control], and the design's bookkeeping for it **rides the effect row of the calculus above**, as a control effect carrying the answer.
The subtyping refinement for delimited continuations is the named next step in that direction [@materzok-biernacki-2011-subtyping-delimited].

As built, `reset` is check-only against the answer and transparent on the type, and the answer-type bookkeeping rides an **ambient answer register** rather than a row entry — the v0 form of the row-carried version, whose answer-modifying generalization is reserved.

### Handling is a generalized reset

**The handle rule _is_ a reset whose clauses receive the captured stack**, which is the design's statement of the handlers-and-delimited-control correspondence [@forster-kammar-lindley-pretnar-2017-expressive-power], with the generalized-continuation reading of handlers as its semantic companion [@hillerstrom-lindley-atkey-2020-generalised-continuations].

One mechanism, two surface idioms — and the reason the two are specified in one document rather than two.

### Stacks are located

A captured stack is **immobile**, like a thunk: a continuation is the most world-bound object there is, since it names frames that exist in one place.

The consequence for the shell's job migration is direct: **jobs move by reference or by re-execution, never by shipping raw stacks.** Genuinely mobile code is a separate mechanism and a separate extension, recorded as such in [[type-system]].

### One-shot linearity, and what it resolves

If a captured stack contains linear-zone obligations — open endpoints, held capabilities, acquired channels — then three rules follow, and together they settle a question the effect-handler literature leaves open.

- The reified stack value is itself **linear-zone resident**, because it _owns_ those obligations.
- **Discarding it without resuming is a linearity error**, unless the discard is explicit: an abandonment operation runs the **unwind obligations** — close, release, drop — recorded in the captured frames.
- **Duplication is prohibited** — one-shot continuations [@bruggeman-waddell-dybvig-1996-one-shot-continuations] — and multi-shot is permitted only for a stack whose captured linear zone is **empty**.

**What this resolves is the handler-versus-protocol tension**: a handler that drops or duplicates its continuation would silently drop or duplicate a session's obligations, and the rule above makes that a typing error rather than a runtime surprise.
It is enforced by machinery the design already has — linear-zone residency and frame-recorded context deltas — rather than by a new mechanism.

**As built, none of this is enforced.** A reified stack is not resident in the linear zone, so resumption, abandonment, and duplication are all unrestricted, and the abandonment operation does not exist.
[[../surface-language/proposed/modes-and-references]] carries the same rule from the surface side, with its own open items, and states the same absence.

### The exceptional path, deferred

The rule above covers **explicit** abandonment.
The complementary case is **exceptional** unwinding — an abort propagating _past_ linear-zone obligations — and two questions in it are open: whether the abort path runs the unwind obligations on what it unwinds past, and whether cleanup may itself fail.

**Neither is settled here, and both have a disposition rather than a silence.** [[../surface-language/proposed/modes-and-references#The decision register]] carries them as open decisions with recommendations attached — wire the abort path to the abandonment operation, and make unwind run to completion with recursively linear sub-obligations — and adds a cancel-during-cancel question the design record never asked.

The candidate treatment types exception propagation as a context-erasing unwind that runs each captured frame's destructor in reverse order, which is the same shape as the abandonment rule [@congard-munch-maccagnoni-douence-2025-linear-effects-exceptions].
**Its default is the one to notice:** it takes destructors never failing as a deliberate design choice, so it _forbids_ fallible cleanup rather than typing it — which is the question, not the answer.

## Involutive negation, placed honestly

The design began from classical control, and the placement below is the part most easily got wrong by someone reasoning from the linearity rule alone.

With an abstract answer, the continuation negation is definable as a reified stack into that answer, and the classical principles become derivable in the fragment with control.
**But the full involution is not what this core delivers.** It delivers only the map _out of_ the double negation, making a type a retract of its own double negation rather than isomorphic to it.

**The obstruction is the cartesian value layer's lack of a dualizing answer object, and it is _not_ one-shot linearity.** That distinction is the whole content of the paragraph: a reader who assumes the one-shot rule is what blocks the involution will look for the fix in the linearity discipline, where it is not.
**Selinger's control categories locate the obstruction exactly, and the location below is read from the paper rather than attributed to it** [@selinger-2001-control-categories, sec 3.5].
Double-negation introduction `∂ᴀ : A → ⊥^(⊥^A)` and elimination `θᴀ` compose to `θᴀ ∘ ∂ᴀ = idᴀ` (Lemma 3.9(2)) — a type is a **retract** of its double negation, which is the map this core delivers and no more.
And the obstruction is the next clause's proof: `∂ᴀ` is natural but **not in general central**, because if it were central then _every_ morphism would be central (Lemma 3.9(4)), collapsing the effectful layer altogether.

**Centrality is the value layer's property, not the linearity discipline's**, which is why that paragraph's warning is the right one: nothing in a one-shot rule is what blocks the involution.

Thielecke's thesis is the second classical result and it stays **locator-pending** — named by author and subject only, with no verified entry — and it is **declined for acquisition** ([[#Works cited here that this corpus does not hold]]).

**The genuine involution belongs to a different negation.** An _inspectable_-stack negation, rather than the continuation type into falsity, is what makes negation involutive [@munch-maccagnoni-2014-involutive-negation], in the non-associative setting whose models are duploids [@munch-maccagnoni-duploids].
The adjunctional unification behind the comparison is the classical extension of the Hasegawa-Thielecke theorem [@mangel-mellies-munch-maccagnoni-2026-hasegawa-thielecke], and the published adjunction model of effects _and_ resources locates this design's core as its cartesian corner [@curien-fiore-munch-maccagnoni-2016-effects-and-resources].

**The standing division of labour, which is the decision this section exists to record.** The call-by-push-value core is what the language is built on, for compositional effects and for tooling; the focused and duploid account enters twice and only twice — as the **semantic sanity check** that the reified stack and the thunk are the two shifts of a polarized adjunction, and as **licence for focused-reduction optimization passes** in the runtime.
Never as core syntax.

## The shell as the motivating application

The shell language embeds in the computation fragment as **an effect signature, session protocols, and the control operators, with no shell-specific typing rules at all**.

[[../surface-language/shell#The POSIX-to-typed mapping, and the deferred DSL]] owns that mapping row by row, including both of the observations that make it more than a curiosity — that job control literally _is_ first-class one-shot stacks, and that every shell footgun lands on a static discipline this calculus already has.

It is recorded here because of what it says about **this** document: the shell is the design's motivating application for first-class stacks rather than a beneficiary of them, so the linearity rules above were derived from a concrete discipline rather than proposed abstractly.
A terminated job holding open pipes is exactly the unwind-obligation case.

Asynchronous effects — signals delivered as interrupts, a trap as an installed handler clause — are the one part of that mapping needing machinery beyond this document [@ahman-pretnar-2021-asynchronous-effects].

## Dynamics

### The pipeline the record specified

```text
surface → core → defunctionalization → a stack machine on ⟨term | stack⟩ → per-world processes
```

The design's sequential half is the standard call-by-push-value machine over configurations of a term and a stack, with **the stack being the same datatype the reified stack type internalizes** — so reification is exactly "stop and take the current stack as a value".
It was to be derived by the same functional correspondence as the typing machine ([[typing-machine#The method, which is the point]]): one method, two machines, shared frame infrastructure.

### What replaced it, and why this is a supersession rather than a gap

**That machine is physically removed, and the polarized command machine is the sole evaluator.** The dynamics of effects and control are built — in `gandr-core-sequent`, over the polarized intermediate language rather than over call-by-push-value terms — and the correspondence is exact where it matters:

- a deep handler is a **consumer** that pattern-matches operation constructors and binds the resumption as a covalue, rather than an operation-keyed handler frame;
- a delimiter is a **prompt** consumer, and capture is a binding up to it;
- performing walks the reified frame stack to the nearest handler for the operation, and resuming splices the captured resumption;
- the reified stack crosses into value position as the same covalue.

[[../metatheory#The operational substrate — the polarized sequent kernel]] carries that reading from the metatheory side, where it is stated as one mechanism replacing three.
**An unhandled operation is a defined blame outcome rather than a panic**, and the same seam offers an unclaimed operation to an ambient host handler, which is the runtime host's boundary ([[../implementation#The runtime host]]).

**One restriction is worth carrying because it has a reversal trigger.** A capture that outlives its delimiter would leak its answer type; the built machine resolves this conservatively by clearing the ambient answer at suspension boundaries, and the principled alternative — the answer-carrying control effect of [[#Control operators and answer typing]] — is reserved as the trigger that would replace the restriction.

### The process soup

**Not built.** The design's concurrent half is a soup of per-world configurations: forking spawns, channels connect configurations, acquiring is a mutex on the providing configuration, and migration ships a closed configuration to another world's soup.

Every one of those depends on a feature that is not built — sessions, sharing, or worlds — so the soup is not separately schedulable; it lands with them.

### The optimization seam

**Not built, and deliberately bounded.** Focused and duploid-guided reduction — administrative-redex-free splicing, polarized normal forms — belongs here, as passes over the **abstract** machine's configurations.

**The bound is what "machine" means in the previous sentence, and it is a decided boundary rather than an omission.** The machine these passes optimize is the abstract one, whose configurations are a term and a stack; compiling the checked core further, to native code behind a backend seam, is a separate engineering direction with its own construction obligation, and nothing in this pipeline implies it.

## Staging

Five rungs, each depending on the one before.

### effects-rung-01

**The effect layer.** Effect rows on the returner, deep and shallow handlers, the row-unification solver domain, and the ambient-ability surface.

**Built in part:** rows and deep handlers exist; shallow handlers, the solver domain, and the ambient surface do not.

### effects-rung-02

**The control layer.** The stack judgment, the reified stack type, reification and resumption, reset and shift — plus one-shot linearity and the abandonment operation with its unwind obligations.

**Built in part:** every typing form exists; the linearity half does not, because a reified stack is not linear-zone resident.

### effects-rung-03

**The sequential machine and its evaluation surface.**

**Built, by the polarized machine rather than the designed one.**

### effects-rung-04

**The process soup:** sessions, sharing, and worlds executing, with asynchronous effects for signals.

**Not built**, and gated on the features it executes.

### effects-rung-05

**The shell language's elaboration** and a virtual host layer under it, with job control as the flagship worked example.

**Not built.** The surface fragment exists and its operations run through the host seam ([[../surface-language/shell]]); the semantic language does not.

## Open items

### effects-question-01

**Does the row stay sealed, or does the row variable land?**

The sealed set is sound and cheap, and it is enough for every rule this document states.
What it cannot express is **row polymorphism** — a function polymorphic in the effects of its argument — which is the whole reason the grammar has a row variable.
The remedy is scoped: the carrier is a newtype, so this is an edit to one module, and the decision is when rather than whether.

### effects-question-02

**Is shallow handling built as the primitive, or is deep left primitive because it is already there?**

The design's ordering makes shallow the primitive and deep its desugaring.
The tree built deep first, which is the opposite order, so landing shallow now means either re-deriving deep from it — the design's intent, and a real change to a working checker — or admitting two primitives.
**Naming the choice matters more than making it here**, because the cheap path is the one that silently forecloses the design.

### effects-question-03

**What makes a reified stack linear-zone resident, and what does that cost the built forms?**

This is the single largest gap between the design and the tree.
It cannot be closed before the linear zone is populated, which waits on sessions; but the shape is known — the stack owns the obligations of the frames it captured — and the three rules it forces are stated above.

### effects-question-04

**Where does the answer type live once suspension is real?**

The ambient answer register is a v0 whose stated reversal trigger is the answer-carrying control effect.
The trigger has not fired because nothing suspends yet, so this question is a dependency on the process soup rather than an open design choice.

### effects-question-05

**Does an operation's cost graduate to an effect?**

A cost-as-effect reading — a step-counting effect layer over the returner — is a live research direction elsewhere in this tree, and if it graduates it lands as a row entry rather than as a new judgment.
Recorded here because this is the document its landing would change.

## Source and confidence

Written against five sources, named because a change with no declared source set cannot be fidelity-reviewed.

1. The **pre-reboot effects, control, and shell design record** in full — its provenance section, the effect grammar and both rules, the deep-versus-shallow disposition, the ambient-ability decision, the interaction table, the stack judgment and its internalization, the control operators and the answer-typing discipline, the location rule, the linearity rules with their deferred exceptional path, the involutive-negation placement, the shell mapping, the dynamics, the staging, and its reference list.
2. **The tree**, for every as-built claim: `gandr-core-checker`'s `effect` module for the row carrier and the signature model, its `syntax` and `types` modules for the operation, handle, resume, reset, and shift nodes and the two type formers, and its `stack` module for the stack-typing judgment and its frames; and `gandr-core-sequent`'s intermediate language and machine for the dynamics, the handler consumer, the prompt, and the host seam.
3. **[[type-system]]**, which declares the two type formers and carries the feature table whose effect and control rows this document is the content behind.
4. The **corpus documents that already carry parts of this record** — the shell mapping, the mode and reference calculus's unwinding rules and open items, and the metatheory track's consumer-side reading of handlers — read against the record and linked from the claims they carry rather than restated.
5. **Two of the cited papers themselves**, added at the 2026-08-08 revision and read against the claims that name them rather than taken from the design record's attribution: [@remy-1989-typechecking-records] for the row-unification claim, and [@selinger-2001-control-categories] for the double-negation obstruction.

**Confidence, by class.**

- **High** — the grammar, both rules, the stack judgment, the control operators, the linearity rules, the interaction table, and the staging, all transcribed from the design record rather than re-derived; and every as-built claim, each read from a definition rather than from a doc comment.
- **Medium** — the correspondence drawn in [[#What replaced it, and why this is a supersession rather than a gap]] between the design's machine and the built one, which neither the record nor the crate states as a correspondence.
- **Read against the work** — Selinger's location of the double-negation obstruction (his section 3.5, Lemma 3.9) and the no-backtracking property of row unification (Rémy's section 2 and its Theorems 1 to 3), each discharged from a held paper at the 2026-08-08 revision rather than left on the design record's attribution.
- **Marked at the claim** — Thielecke's result, and the detail and denumerable-label case of the row-unification algorithm.
  These are what survives the revision above: each is named by author and subject with no verified locator, and each rests on a work that is **declined for acquisition rather than unexamined** ([[#Works cited here that this corpus does not hold]]).

### Works cited here that this corpus does not hold

Three works this document leans on are **declined for acquisition**, which is a different state from _not yet looked at_, and recording the difference is the whole point of this section: a later reader should not re-open a search that has already been run and closed.
The decline is an owner ruling — `gandr-fid.14.7-answer-11`, 2026-08-08 — taken on the ground that the works are old and not realistically obtainable.
The same ruling fixed what happens to each claim left behind: re-ground it on held work where one exists, **having read the held work against the claim first**, and leave the rest marked permanently.

| work                                                                                                | what rests on it here                                                            | disposition                                                                                                                                                                                |
| --------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Thielecke, _Categorical Structure of Continuation Passing Style_ (doctoral thesis, Edinburgh, 1997) | the second of the two classical results locating the double-negation obstruction | **locator-pending permanently.** No register entry: even the author, title, and year above are carried from a search rather than checked against the artifact, so nothing here is asserted |
| Rémy, _Type inference for records in a natural extension of ML_ (INRIA research report)             | the detailed row-unification algorithm, and the denumerable-label case           | **narrowed.** The rest of the claim is re-grounded on the held companion paper [@remy-1989-typechecking-records], which reaches the finite-label case and no further                       |
| Felleisen, _The Theory and Practice of First-Class Prompts_ (POPL 1988)                             | the sense of "prompt" the delimiter above is claimed to have                     | **cited, unread.** The register entry [@felleisen-1988-first-class-prompts] is sound and its identifier resolves, so the citation stands; what is unpaid is the content check              |

**The Felleisen row is deliberately not re-grounded, and the temptation to do so is right there in the sentence.** [@danvy-filinski-1990-abstracting-control] is cited beside it and _is_ held — but it carries the answer-type discipline, not the prompt calculus, so pointing the prompt claim at it would make this document say less while looking like it says more.

**Reversal is cheap and the condition is one line**: obtain the work, read it against the claim, and the mark comes off.
Nothing above rests on the decline being correct — only on its being visible.
