# Metatheory roadmap

What remains to realize the accepted directions of [[../metatheory|the metatheory track]], in executable detail.
Ordered within each section by what unblocks the most; costs are estimates against the current tree.

## Spikes — cheap experiments that decide design questions

Each spike is a heading so it can be linked into directly — `[[metatheory/roadmap#meta-spike-04]]` and the like.
The numbering is stable and its gaps are meaningful: a missing number is a spike that has been executed and retired, and its verdict is recorded where the decision it settled lives.

### meta-spike-01

**Make the `theory-computads` enumerator alphabet-polymorphic.** EXECUTED on the Rust side — the engines are generic over `CellAlphabet` with `SequentAlphabet` as the first inhabitant, and an external toy alphabet drives all three engines; the guard-plus-witness half of the warning below is tracked as `gandr-s9q`.
`enumerate_overlaps`, `completion::complete`, and `rewrite::normalize` were gate-tested but monomorphic over the sequent-kernel command-pattern alphabet.
**Days.**

Settles the highest-leverage engineering item on the board: the coherence grinds ahead — the four-layer exchange identity, the shape-layer interchange equation — are exactly the hand work the tool exists to prevent.
Carry the warning verbatim: off-TCB applies to the **enumerator only**; the `cells_equal` normal-form fast path is TCB-adjacent and needs a guard plus a soundness witness, never documentation.

### meta-spike-02

**Is the measured cellular-Conduché condition the exponentiability condition the convolution face waits on?** **≈ 1 day.**

Closes a gate on the convolution face using a measurement already taken; the one open axiom row (cylindrical decomposition) is what convolution needs _beyond_ exponentiability.

### meta-spike-03

**The four offset functions.** Check the monotone-rung placement the directed arena is built on — the computations [[layout-and-coherence]] cites as unchecked — against `Gandr.Arena.Offset`.
**½ day.** Settles the arithmetic substrate of the arena's directed generalization: which of the four one-way generator classes lands in the monotone rung, and where the codiagonal breaks order.

Partially executed in `Gandr.Arena.Directed`: the six realizations; the offset-fixed boundary with proofs and pinned counterexample cells (`inl` fixed verbatim, `inr` fixed modulo the right-injection shift, the diagonal fixing exactly offset 0, the projections fixed exactly at the unit laws, the codiagonal fixing exactly the left leg); and the shift-0 core of `RigidMono`, closed as a category with one-sided whiskering.
Remaining: the named offset transforms (the right injection's shift, the projections' floor-division, the diagonal's `b · i + i`), the four monotonicity proofs, the codiagonal's order-break witness at a size-2 code, and `RigidMono` carrying the shift as data.

### meta-spike-04

**Is gandr's pattern _analytic_?** Check the two conditions (strict Segal morphism to pointed finite sets; conservative interior) with underlying-legs as the candidate functor.
**½ day.**

Settles whether arity approximation applies — whether circuit algebras at `Set` are determined by arity-≤k data, a truncation result more useful than raw finiteness.

### meta-spike-05

**Is the operadic partition complex built from the graphical category's slice?** Compute at a corolla and at the diamond.
**½ day.**

If yes, the Morita-restriction failure, the elegance gate, and the coherence-connectivity criterion are three faces of one condition.

### meta-spike-06

**Is the graphical category _elegant_** (Bergner–Rezk [@bergner-rezk-2013-comparison])?
**Unmeasured.**

The gate on univalence transfer to the diagram model.
If elegance fails for the site but holds for its rigidified form, `Rigid` is load-bearing for transfer — the fourth appearance of one decision.

### meta-spike-07

**The descent corolla-restriction lemma** — reflect an isomorphism of free algebras back to the generating species.
**Small.**

Per-stratum `ua` is a citation **plus this lemma**; easier at the nonunital rung; must not be discovered at implementation time.

### meta-spike-08

**Exhibit the protype whose tabulation is funext over cellular data**, and confirm both ends are objects of one equipment.
A named candidate answer exists from the synthetic-calculi analysis: the protype is the **loose unit**, and funext is **unit-pureness** — full faithfulness of the unit — so the spike may be a verification rather than a search.
The fallback template is the syntactic lax-cones-over-computads construction ([@mikhail-2025-thesis] ch. 1; globular contexts only, a limit not a tabulator, general case a conjecture).
**Unmeasured.**

The one genuine construction the equipment join rests on.

### meta-spike-09

**Is the removal/rebuild pair an instance of the context comonad for tree-like types**, and does the spanning-tree traversal give a shape for `canon`?
[@altenmuller-2026-string-diagrams] **½ day.**

Generalizes a bespoke device; the only concrete lead for the _form_ of a canonical linearization.

**Two concrete candidate shapes are now on the table, and they bracket the family** (2026-08-01): for one syntax over unrooted trees, orienting its cut equation one way makes normal forms **corolla decompositions** — pick a vertex and recurse into the components its removal leaves — and orienting it the other way makes them **edge decompositions** — pick an edge and recurse into the two components its removal leaves, which is the spanning-tree traversal this spike went looking for; the source exhibits both and observes that they are the two extremes of a mixed style [@obradovic-2017-thesis, sec. 2.4.2].
The same source is a caution as well as a lead: its rewriting system is **non-confluent** at exactly the cut symmetry — a redex and both its reducts denote one tree — and its worked example exhibits **five** distinct normal forms for a single tree, so a decomposition shape is a canonical-form _candidate_ and owes its own confluence argument.

### meta-spike-12

**The finiteness/simple-connectivity measurement, re-specified.** The gate must measure the _semantic_ shape (count cells with a repeated metavariable), not the as-built shape — nominal sharing makes the as-built measurement circular.
**1 day.**

Settles what fraction of real cells leave the dioperad fragment once sharing is a wire.

### meta-spike-13

**Supply the tile relation** and instantiate the axiomatic-rewriting axiom interface non-vacuously, resolving the contested axiom count first (nine axioms in one presentation, a ten-item interface in another — both may be right for different presentations).
**2 days.**

Four standardization theorems by citation; turns a vacuous pass into real inheritance.

### meta-spike-16

**Re-decide the double-pushout scoping at the circuit rung.** **EXECUTED 2026-08-01.** The verdict is **re-scoped, not retired**, and its full statement with citations is [[../implementation/circuit-terms#The correspondence at gandr's own rung, at theorem grade]]; the track's own sentence is restated at [[../metatheory#The operational substrate — the polarized sequent kernel]].

In one paragraph: the applicable instance at gandr's rung is **convex** DPO with interfaces over **monogamous acyclic** hypergraphs, and the correspondence with syntactic rewriting is an **iff for arbitrary symmetric monoidal theories, coloured ones included** [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-ii, thm 25, thm 35, thm 39].
The fragment's two conditions are exactly gandr's two standing declines — monogamy is the fan-in (and fan-out) refusal, acyclicity is wheel-freeness — so many-out, reconvergence and disconnection are inside it and wheels are not.
The Frobenius hypothesis is not the obstacle it looked like: it is what the _first_ paper assumes, and the sequel discharges it.
What replaces the mono-left-leg condition is the **boundary complement**, unique whenever it exists, explicitly including rules that are not left-linear [ibid., prop 31] — which matters because gandr's patterns are not left-linear in general.

Two things the spike did not settle and that are now carried as named questions rather than as this spike's residue: whether gandr's cell application is **convex**, and what covers the **traced** rung the arity ruling has made gandr's destination ([[../implementation/circuit-terms#The design questions]], `circuit-terms-question-15` and `circuit-terms-question-19`).

### meta-spike-17

**Does decidable confluence transfer, and does the critical-pair procedure?** **EXECUTED 2026-08-01**, jointly with [[#meta-spike-16|meta-spike-16]].

The result transfers, and the mechanism is the part worth carrying.
Local confluence of DPO-with-interfaces follows from joinability of all pre-critical pairs, and for a **computable terminating** such system confluence is **decidable** [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-iii, thm 3.1, cor 3.1] — where _computable_ is a defined condition (computable pullbacks; a finite computable set of quotients of $L_i + L_j$ per rule pair; enumerable one-step rewrites) and the ambient hypotheses hold in any presheaf category and are stable under slice [ibid., asm 3.1].
**The interface is what saves Knuth–Bendix, and the empty-interface case is the undecidable one**: the authors' own framing is that hypergraphs with empty interface are the graphical analogue of ground terms, and that ground confluence is undecidable for terms and graphs alike while confluence is decidable for both [ibid., secs. 3 and 7].

**What it means for gandr's contract.** gandr's completion engine already enumerates critical pairs and already carries seam data, so it is on the decidable side of that line; its _completed means the worklist drained_ caveat is therefore about **budget**, not about undecidability, and saying so is a material sharpening of the contract.
What the engine does **not** have is the other half: the published decision requires termination, and gandr's completion has no termination argument — its reduction order is plain node count, not a substitution-stable order.
Without Frobenius there are additionally two routes and gandr must pick one: **left-connected** systems (left-linear, ma-rules, strongly connected left-hand sides), where the notion of critical pair is unchanged and confluence of a terminating system is decidable [ibid., def 5.6, thm 5.3, cor 5.1]; or **path joinability**, which checks a critical pair under every maximal path relation via three formal path generators and is necessary as well as sufficient over the extended signature [ibid., def 5.7–5.10, thm 5.4, thm 5.5].
The residual obligation is therefore **a termination argument and a route choice**, both of which belong to the engine and are recorded on the implementation lane rather than here.
The route choice has since been taken for the **as-built** cell alphabet and re-opened by its growth — every expressible cell left-hand side is strongly connected, so the left-connected route is the one to take, its two remaining conjuncts are build items rather than research questions, and the multi-output and disconnection axes each break the verdict ([[../implementation/circuit-terms#The correspondence at gandr's own rung, at theorem grade]]) — while the termination argument stands open.

### meta-spike-15

**Map the landed description constructors onto a graphical-species profile.** The six code-grammar variants against the tiny base of finite sets, bijections, and the input/output involution.
**1 day.**

The first thing to test on the pasting side — everything in the univalence section assumes it; the falsifier is a description needing dependency or indexing the base cannot express.

## Standing obligations — what must be proved or built

* **The wire half of the composition law.** The four-layer exchange coherence's **cut half is closed** — `insert-swap-coh⁴` is proved and the cut composition lemma is proved in all five clauses, with no hypothesis and no parameter — and the live ladder is the wire half, precisely located: the wire apart-lemma at arity three (needs the braid), the cut apart-lemma at arity four (needs the four-layer coherence, held), then the removal composition law and its unanalysed unhit mirror, dropping the wiring category's associator parameter, grafting associativity, and the shape layer's `Monoidal` instance with its interchange **equation** — the duoid target.
  Nothing of the ladder is written yet: the two apart-lemmas are unstarted, and the two glue lemmas of the unit's plan vocabulary (relating removal-tail/removal-recap to removal composition) have no statements in the tree — so no branch of them is open, and none should be read as closed.
  The halves differ for a stated structural reason worth keeping: a cut leaves the second wiring's sources alone, so both sides consult it at the same position, while threading a wire _moves the lookup_, so the two sides consult the second wiring at two different positions.
  The standing prediction is that **no further coherence is owed** on this ladder — a prediction, not a measurement, falsified if either apart-lemma needs a layer the file does not have.
  Keep acyclicity, tractability, and termination as separate named obligations with separate suppliers, never one monolithic convergence proof.
  **The upper rungs are re-scoped by the former ruling of the arity-interface item below.** Substitution is primitive, so the closure subsumes `match-comp`: the removal composition law and grafting associativity are reached through the monad law rather than through the accumulator, and what stays owed here is the two apart-lemmas and the interchange equation.
  Build the apart-lemmas — they are owed either way — and reach the rungs above them through the closure rather than through `match-comp`'s accumulator.
* **Unit five's interchange as a `Tower` consumer**: the interchange equation's middle profile is an existential intermediate — the coend variable, appearing on one side only — which is exactly the shape the `Tower` device packages (carry the intermediates as fields, not indices); reach for it before stating the equation.
* **`Rigid.canon-sound` at the circuit rung**: the construction-term normal form (permutations outermost, ordered tree monomials, unique minimal representative), with the **monomial-to-monomial rewriting condition** checked before anything leans on it.
  **A worked precedent and a bounded residual are recorded against it, and the precedent is the larger half** (2026-08-01, reframed the same day after the reading rule was stated).
  The precedent first, because an earlier draft of this entry led with the hazard and thereby made the same mistake the rule names.
  Categorified cyclic operads are given a coherence theorem **non-skeletally**, and skeletal coherence is then obtained **by reducing to it** along the skeletal/non-skeletal equivalence the same work constructs [@obradovic-2017-thesis, app.
  A].
  That is [[../metatheory#The representation is not the theory|the section discipline]] carried out at an adjacent rung by someone else: the theorem is proved at the quotient and consumed at the ordered presentation, which is exactly what `Rigid` is for.
  It is therefore evidence that gandr's whole representation discipline is workable, not evidence against it.
  **The residual is one named proof move, and it is a cost rather than an obstruction.** The same appendix states that non-skeletality is crucial for the rewriting its proof uses: non-skeletally a symmetric-group action can always be pushed _inward_, from a composite onto its operands, by orienting the equivariance law that way, and the first reduction depends on it; skeletally that distribution "doesn't work in general", exhibited at a three-element composite, and the authors add that they do not know whether orienting the equivariance the **other** way would serve. gandr's recipe pushes permutations **outward**, which is the direction the source declines to claim about in either direction — so the residual is precisely that **a gandr proof may not reach for the inward move, and must route through the equivalence instead, as the source itself does.** What is on the record is therefore a price, not a warning: an ordered representation is not coherence-neutral, and the move it forgoes is named here rather than discovered at proof time.
* **The arity interface at the circuit kit.** The interface is settled and landed universe-style in `Gandr.Arity.Universe`, with the linear instance complete and eight of thirteen fields inhabited at the circuit kit ([[../metatheory#The arity interface, universe-style]]).
  The five that remain all descend from **graph substitution**, whose statement is carried as a type in that module so the next pass starts from a signature.
  Its residual, the **two-sided closure**, is scoped and **the former is ruled**: substitution is primitive and grafting is derived from it, by substituting into a two-corolla series shape.
  The closure decomposes as the merger plus a single-wire closure that does not recurse; it costs four new auxiliaries and retires four, including the kit's only well-founded recursion.
  Its circles are **counted, not discarded** — a code is a shape with its number of closed components, which is the source's own definition of a Brauer diagram rather than a gandr device, and `Match` and `Shape` are untouched.
  Buildable now, in this order: `match-close` and its shape-level lift, the block iteration, `sub` and `pair`, then the three laws.
  Two obligations travel with it — the **agreement lemma** against the built `graft` (without which `verts-graft`, the two unit laws and the merger's incidence theorems do not transfer) and the **count law** for associativity.
  The interpretation is already derived — the profile-indexed vertex family is the generic listing-occurrence family at the vertex listing, so it is not a second induction over the shape and must not be re-declared as one.
  The obligation the presentation promotes rather than discharges is the representation map, which at this rung _is_ `Rigid.canon-sound`; it is a field of the interface, so no circuit instance exists before it does.
  The linear kit is the worked precedent for what that field asks: refuted against the bare interpretation, proved against the ordered one, with the two bracketing what the interpretation must remember.
  Two spikes are retired here rather than carried: the control experiment at the linear kit (the presentation is cheaper — the unit and associativity laws are the existing lemmas read off the graph by functionality, the whiskering lemma has no counterpart, and one new lemma is the whole price), and the coherence-law count (three, because the former is a monad multiplication, and homogeneous because the codes are indexed).
* **The presentation of the graphical category** $Θ_(T^times, "Gr")$ — degree, the two subcategories, computable factorization through canonicalization, decidable morphism equality — and **the oriented-slice arities restatement** (a paragraph: transfer the arities claim along the slice equivalence to the oriented monad).
* **The Segal certification ladder**: per-degree Segal checks; equivalence certificates and per-degree checking on a toy signature with one nontrivial automorphism; fuelled transport with cost accounting (do the two cost measures separate cleanly in code?).
* **The two-cell coherence of the layout univalence map** (the typoid-function obligation), and the _statement_ of its pasting-side analogue.
* **The directed convergence pass** for the directed rule layer — **re-quote its price against the arena presentation first** (the recorded transformation-monoid price was quoted against the superseded tree presentation), and evaluate a focusing-staged normal form before any raw completion pass.
* **The carrier-to-source translation lemma** (the shape carrier against the graphical-species presentation).
* **The decomposition-space edge**: verify the measured strict pullbacks are the stability condition, establish or refuse a set-level shadow, and record the certificate-layer/doctrine-layer identification with its citations.
  The unitality half is dissolved by citation — every 2-Segal space is unital [@feller-garner-kock-proulx-weber-2019-unital]; the pita-nerve result — the nerve of a strictly factorisable operadic category with invertible quasibijections is a decomposition space, with an explicit non-Segal counterexample when they are not invertible [@batanin-kock-weber-2018-regular-patterns] — is adjacent input; and the decomposition-space line's _locally discrete_ grading is the literature's closest analog of a witness h-level, logically independent of Segal versus 2-Segal, and bears on [[#Open questions|meta-question-08]].
  Two adjacent set-level facts from the held properads literature frame the same edge [@hackney-robertson-yau-2015-properads]: the strict-Segal nerve theorem (a fully faithful nerve for properads, with the strict properadic Segal condition and unique inner-horn fillers) is a set-level warrant for the nerve direction at the properadic rung, decoupled from the polynomial-interpretation obstruction; and the finiteness wall — the free many-to-many term set over a cell is finite exactly when the cell is simply connected — is the computable-layout-relevant boundary, more than automorphisms or the nerve theorem itself.
* **The core-coincidence theorem** of the directed statement (the groupoid statement as the invertible core), and the directed normal-form/faithfulness wall behind it.
* **Deleting the cell record's simple-connectivity field** (or re-carrying it as a consumer-side predicate) under the generality ruling, with the surface-language question — whether the _surface_ still hides wheels and disconnection — as its own design pass.
* **The non-unitality condition check** against its polygraph source (cheap, high value; on its face gandr's pattern-to-pattern rules satisfy it).
* **Standing obligations inherited from the identity/reflection arcs**, each a named hole until discharged: J-as-tabulator-elimination is a **theorem obligation, not an identification**; higher-stratum transport owes explicit lifting/cumulativity coherence; protype-isomorphism certificates stay **separate** from equivalence/univalence certificates until the bridge theorem lands; the variance accounting of directed observations and corecursion owes a theorem rather than a description; unfolding and certificate replay must be defined as an **operational relation** before justifying temporal transport with them; and the base-stage identity law must be stated honestly (never implying full path induction where only the saturated-instance eliminator exists).

## The wager's falsifiers

The coherence-debt arity law (debt arity = threaded positions + the head met; blocks contribute nothing) is the architecture's central bet.
Its three falsifiers, all scheduled or spikeable:

1. the residue after the epi–mono and Coxeter decompositions cannot be decided cheaply — the codiagonal alone forces the transformation monoid, which has no register row and no formalized rewriting twin;
2. the next unit's interchange requires two cuts to commute — the ladder is not finite, and gandr enters the measured-blowup regime knowing the growth law rather than merely measuring it.

The third falsifier the law once carried — that the graph former's coherence laws might not stay finite, making the universe route a rename — is retired.
The former is the arity monad's multiplication, so its laws are the monad laws and there are three of them whatever it multiplies; the interface index makes all three homogeneous.
What that retirement does **not** buy is a cheap construction: the coherence count and the construction cost are different questions, and the second is the listing algebra, which is untouched by how the interface is presented.

## Parked deliberately, with reasons

The free-bifibration STOP (gated on the factorization-preorder check, not scheduled); the coproduct-as-cache-keys and antipode-as-rollback directions (need their own design passes); the acceleration band (until the certificate relation has non-empty extension); the session/protocol code stratum (identity at a universe of protocols needs sessions reflected as codes; the cross-stratum seam must be flagged before either stratum hardens — passing to components across an ordering/no-ordering boundary is known not to be conservative); the permission-monoid question against the grade design; higher-order cells and the second hole theory (conditional on wanting them); instance-level keying of the overlap-support relation (the obvious first improvement once the relation is non-empty).
Also parked with their design substance pinned: **the sized/termination direction** — sizes enter as _indices in their own sort_ reusing the grade-zero erasure machinery, never as a fresh semiring grade (they need a well-founded order the resource semiring cannot express); bounded size quantification is unsound without a consistency gate on reduction; the well-founded fixed-point former is the single recursion-plus-corecursion former that retires the productivity/termination split; the guardedness check is a two-state flag automaton over observation-record introductions; and four named deep-guardedness programs pin the syntactic-check/sizes cliff for the corpus.
And **the codata dependent slots** — self-dependent projection result types, indexed codata, the without-K unifier extension, forced copatterns, and the empty cosplit — with the elaboration hazard pinned: lowering codata to positive records of thunks smuggles a computation into the value zone unless mediated by the shift, so the value-side and computation-side readings must be resolved deliberately, and codata has **no η** (undecidable, and recursive-record η breaks the elaborator's scope invariant).
And **the exact-reals / synthetic-topology track** — a lateral, firewalled line whose design, staged plan (stages A–G with gates), decision register, and open obligations are dispositioned in [[exact-reals]]; the metatheory-side carries are the equipment reading (the modal-law checklist as cartesian-equipment conditions, stated once when stages E/G approach), the temporal reading (observational backend equivalence as a temporal certificate), and the `ua_topo` statement shape with its precedent stack, none of which schedules work on the minimal-kernel path.

## Open questions

1. **meta-question-01** — Is the free-rig monad cartesian?
   — the single technical question on which any nerve route for the _layout_ universe turns; no published answer.
2. **meta-question-02** — Does the description universe fit a graphical species?
   — gated by its named spike; falsifier: dependency or indexing.
3. **meta-question-03** — Where does shift equivalence sit in certificate identity — should the _store_ key on the normal form, or only the comparator?
4. **meta-question-04** — Does the reflection face's cartesian-fibrational target restrict to the double-theory cartesian notions where both apply?
5. **meta-question-05** — ~~With non-linear patterns admitted, is the overlap family still finite and multi-universal, or does an occurs-check fragment need fencing?~~ — **dissolved by ruling 2026-08-01**, not answered: cell patterns are linear, so the premise no longer holds ([[../implementation/circuit-terms#The design questions|circuit-terms-question-17]]).
   Two things the dissolution changes rather than removes.
   The **globularity-above-the-base trigger** was "a non-linear pattern producing a genuine, non-singleton multi-sum family", which the ruling now prevents; it must be restated against the per-type-comonoid generalization, because that is the construction under which a genuine family could reappear.
   And the question **returns unchanged** if that generalization lands, so it is retired-with-a-reversal-condition rather than tombstoned.
6. **meta-question-06** — Does gandr _state_ the trek-to-tracelet seam (a publishable convergence of two disconnected literatures) or merely use it?
7. **meta-question-07** — Which cell classes exercise the residual part of the target-opfibration axiom beyond redex-creating instantiations?
8. **meta-question-08** — Is there operational content in the correspondence between the determinism axis (Segal → 2-Segal) and the symmetry axis, or is it orientation only?
9. **meta-question-09** — ~~The directed eliminator and diagonal-intro spellings; whether directed composition shares the groupoid composition name — surface vocabulary, cheapest settled before the identity-layer rules land~~ — **settled 2026-07-31 (owner decision)**: the eliminator is the shared `walk` (under the motive-covariance side condition), the diagonal intro is `diag`, and directed composition shares `then`.
   Landed at [[../surface-language/directed-family]].
10. **meta-question-10** — Where do the constant-map and constant-literal witnesses land as permanent negative tests — the first phase carrying a directed certificate stock over codes.
11. **meta-question-11** — The variance/directedness annotation slot in the export format: reserved early (recommended) or deferred to the certificate phase at the price of a coordinated format bump on the TCB.
12. **meta-question-12** — The doctrine complex's carrier shape: does it want **two node sorts per dimension** — signatures and relations, with a higher graph per pair of signatures and only the tight cells carrying cross-dimensional meaning — or does the single-sorted telescope suffice?
    (An open owner question of record, with a runnable sketch; the sketch's source file is in the pending sweep, so it re-lands when that sweep runs.
    Adjacent to, not settled by, the three-role split.)
13. **meta-question-13** — A two-point relevant/irrelevant variance record in place of a four-point lattice — co/contravariance presupposes cumulative subtyping, which gandr rejects, so only the irrelevant fragment transfers; and the elaborator will still meet stuck max-plus level equations (the oracle gives entailment and benign loops, not most general unifiers) — an unsolved user-experience surface gandr must own.
14. **meta-question-14** — **Cauchy completion as the representability axis** — how Cauchyness and Cauchisation sit under both univalence statements, and the equipment-level Rezk completion; state before either statement's representability is claimed.
15. **meta-question-15** — **The contraction locus** — what adopting the internal-logic equipment costs (no endo-coends), with its honesty gate; state the cost where the equipment is adopted, not after.
16. **meta-question-16** — **The Σ-former at the multi-output face** — the Σ-η direction is where fan-out actually bites (the dual of the data-η discipline), and premise-form statement is what keeps associative–commutative completion out of the rule layer; design before the term face hardens.
17. **meta-question-17** — **The Tietze ancestry note** — the edit-polygraph fullness statement ("complete up to a located obstruction") has a classical ancestor in Tietze-transformation completeness, with the simple-homotopy line as the cautionary instance above dimension one; record the lineage when the layout statement is next touched.
18. **meta-question-18** — ~~Pending targeted reads before import~~ — **resolved by the phase-2 sweep** (each item read at import grade against its primary and folded): the statement-blocker lesson with its blanket-base instance, the frame-bound impossibility, and the observation-grade ledger are carried in [[../proof-engineering#Lessons with no other home|the proof-engineering lessons]], as is the compare-site four-class taxonomy with the shape/witness grading ledger; the per-level cost law of the alphabet discipline (the square-compatibility cylinder one level up is the shape of term the naturality meta-operation generates, and any filler works there) is stated in [[ambient-and-primitives#The technology cluster|the technology cluster]].
19. **meta-question-19** — **The strictness warrant at the circuit rung.** The old licence — the rectification theorem, which said strict semantics is provably adequate at the dioperad rung and not above it — lapsed with the rung change, and nothing has replaced it: either show the rectification dichotomy has no set-level shadow (gandr's cells are finite ordered data compared by content address, not models of an ∞-object), or re-warrant strictness by a coherence-by-decision-procedure story — the skew-monoidal focusing line, the duploid line, or the Schwarz-paper Koszul machine at gandr's own rung [@kaufmann-ward-2024-schwarz].
    This was the substrate arc's deepest open question and it is still open; the re-read of the rectification paper against the shadow question is the named next step, and no consumer may silently assume strictness is adequate.
    **A worked instance of the second route is now on the record** (2026-08-01): a coherence theorem for categorified cyclic operads is proved by **three staged reductions** — get rid of symmetries, get rid of cyclicity, establish skeletality — which is coherence discharged by a decision procedure rather than by a coherence-term generator, at a rung adjacent to gandr's [@obradovic-2017-thesis, sec. 4.1].
    It is an instance to read for its shape, not a candidate re-warrant: it is one-output and unoriented, and its first reduction is the one whose skeletal failure is recorded against `Rigid.canon-sound` above.
20. **meta-question-20** — **A derived `Path`→`Flow` coercion**, once `ua-dir` lands — wanted as a derived surface form, and at which stratum?
    (No kernel coercion: the comparison is the core-coincidence theorem, and a coercion before it would assume the theorem as an axiom.)
21. **meta-question-21** — **Does the certificate layer's one-directional interchange force laxity into the statement's rule layer?** The directed statement keeps the dimension-2 rules an equivalence at this stratum; if mixed-polarity boundaries eventually force one-directional interchange down into the statement, the η grade needs restating — a new design pass, not an amendment.
22. **meta-question-22** — **Variance as a shared kind** carried by the reflected universe, decided when directed univalence is scoped on the reflection face.
23. **meta-question-23** — **The canonical (co)end representation**: does the finite-carrier diagram become the canonical reflected end-object once the description layer can express dipresheaves, or does it remain a property-test vehicle?
24. **meta-question-24** — **What does an adequacy claim for the certificate layer look like?** The diagrammatic-calculus literature has worked examples of the shape — a calculus, a semantics, and a completeness proof that the rewriting is exactly right for it — and gandr has no fixed semantic target, so the theorems do not transfer but the **argument shape** is the closest available model.
    **Carried**, with the stabilizer and Clifford+T completeness results and the ZW calculus recorded as the worked examples to read when the certificate layer's own adequacy statement is drafted; specific examples are more useful here than stronger general statements.

## Reading queue, by leverage

**Next.** The parametricity-cluster entry point [@vanmuylder-2026-thesis]; the pretype-theory report and slides [@nuyts-2026-natpt]; transpension, sections 1–2 [@nuyts-devriese-2024-transpension]; the discrete-Conduché paper, sections 1–2 (for [[#meta-spike-02|meta-spike-02]]) [@guetta-2020-conduche]; the polygraph shape category, introduction and section 5 [@hadzihasanovic-2020-shape]; the (∞,∞)-thesis ch. 1 part 2 (lax cones, for [[#meta-spike-08|meta-spike-08]]) and ch. 2 (the decomposition-space equivalence) [@mikhail-2025-thesis].

**The univalence-transfer chain.** The Univalence Principle, graphs-and-nets chapter first [@ahrens-north-shulman-tsementzis-2021-univalence-principle]; inverse diagrams, sections 11–12 (for [[#meta-spike-06|meta-spike-06]]) [@shulman-2015-inverse-diagrams]; the synthetic line as vocabulary only [@riehl-shulman-2017-synthetic].

**The hypergraph-rewriting sweep**, opened by [[#meta-spike-16|meta-spike-16]], **executed 2026-08-01 for the part that decided the spike** and reduced here to what remains.
Consumed at theorem grade, with what was taken recorded at the spikes above and in [[../implementation/circuit-terms]]: parts III and II of the series [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-iii] [@bonchi-gadducci-kissinger-sobocinski-zanasi-2022-string-diagram-rewriting-ii], and the two implementations read as artifacts, Cartographer and Chyp [@sobocinski-wilson-zanasi-2019-cartographer] [@chyp].
The ESOP statement of the decidability claim needs no separate read — part III is its journal successor and says so [@bonchi-gadducci-kissinger-sobocinski-zanasi-2017-confluence-interfaces].

What the sweep still owes, in order of leverage:

* **the traced/wheeled gap** — nothing consumed reaches gandr's destination rung, and no candidate source has been identified.
  This is the sweep's open end, not a pending read;
* equational reasoning with **context-free families** of string diagrams, the published mechanism for rule schemas over unbounded arity, which a many-out rule with a variable port count would need [@kissinger-zamdzhiev-2015-context-free-families], with its language-theoretic successor beside it [@earnshaw-roman-2024-context-free-languages];
* initial-algebra semantics for **cyclic sharing** structures, the wheel axis from the syntax side rather than the carrier side [@hamana-2009-cyclic-sharing];
* the **multi-device** direction, which is where gandr's disconnection axis lands: effectful categories over several devices, presented by resourceful traces, with a commuting tensor product [@earnshaw-nester-roman-2025-resourceful-traces], read after the thesis that collects the single-device case [@earnshaw-2025-thesis].

The interface-literature map itself is already assembled in the related-work section of the combinatorial string-diagram thesis and should be read there first rather than reconstructed [@altenmuller-2026-string-diagrams].

**The arc's own outstanding reads.** Schwarz modular operads revisited — the one paper running this machine at gandr's exact rung [@kaufmann-ward-2024-schwarz]; circuit algebras are wheeled props (check whether the equivalence survives at `Set` — stated for linear wheeled props) [@dancso-halacheva-robertson-2021-circuit-wheeled]; the graphs/hypergraphs translation source [@kock-2016-graphs-hypergraphs]; the naturality meta-operation [@benjamin-markakis-offord-sarti-vicary-2025-naturality]; monoidal context theory, re-read at the sections the hole identification selects [@roman-2023-monoidal-context].

**Calibration, not adoption.** The combinatorial string-diagram thesis (the closest existing Agda encoding; three independent arrivals at gandr's decisions, one instructive fork, and the context comonad) [@altenmuller-2026-string-diagrams].

**Consumed.** The HoTT-operads internalization [@hewer-2025-hott-operads] — sections 4 to 6 read at import grade, and its generalised-operad-universe record is the shape the arity interface now takes ([[../metatheory#The arity interface, universe-style]]).
Its _development_ remains calibration rather than import for the reasons it was filed under: the wrong rung (one output), truncation to h-sets to avoid higher coherences, and a higher inductive type for the free construction — all three declined here, and the setoid substitutes recorded at the interface.
What was taken is the record, not the mathematics around it; what does not transfer is the representation map, refuted at both of gandr's kits.

**Technique, on demand.** Operads, clones, and distributive laws [@curien-2012-operads-clones]; topological coherence proofs [@curien-laplante-anfossi-2023-topological]; coinserters versus coequifiers [@lucatelli-nunes-2026-freely]; the non-unital presheaf criterion [@henry-2019-nonunital-polygraphs]; cubical internal parametricity (admissible under the trusted-surface criterion) [@cavallo-harper-2021-internal-parametricity]; the skew-coherence-by-focusing line [@veltri-2021-coherence-focusing]; real-cohesion as the cost sheet if a modality revisit condition ever fires.
