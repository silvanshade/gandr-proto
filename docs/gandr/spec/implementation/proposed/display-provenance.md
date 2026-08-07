# The display-provenance layer

**Proposed.** Nothing in this document is built.
What exists is every premise the design stands on — the renderer firewall, the typing machine's serializable derivations, the origin map that already records desugaring provenance, the engines' replayed certificates, and content-addressed storage with a deterministic interner over the _type_ graph — and each is named with the module that carries it below.
What does not exist is the layer itself: no dubbing judgement, no canonical _term_ index at η-long form, no inverse name map, no display labels, no residual search, and no pattern elaboration that could emit a dubbing today.

The display-provenance layer is **elaboration metadata carried in the derivation** — replayed like every other elaboration step, never held in the kernel — keeping three registers over one canonical-term index:

1. **dub tables**: user pattern names mapped to arbitrary core terms, η-expanded forms included;
2. **display labels**: the labelled-type spellings under which a user programming with eliminators never sees one;
3. **the desugaring provenance gandr already records**, promoted from a per-pipeline map to a register of the same layer.

Its design content is the **inverse** direction: printing a core term under the user's names and labels in goals, diagnostics, and the derivation display.
Forward elaboration emits dubbings against canonical (β-short η-long) forms, so the inverse map is a multimap from content hash to name, populated incrementally as dubbings are emitted — a lookup where the source design reaches for a search.
The residual hard case, a term definitionally but not canonically equal to any dubbing, gets a memoised definitional-equality search over the dub table's canonical representatives, with the result itself interned so the search runs once per (term, table) pair.
That residual is the source design's floor solution with its domain shrunk and its cost amortised, stated as such rather than dissolved ([[#The residual case is a bounded search, not a lookup]]).

## What is built, and what this document describes

**Built, and verified against the tree at write time.**

* **The renderer firewall exists, and renderers reconstruct nothing.** The checker's state is reified as data and shipped to renderers as a bounded, schema-versioned projection; the leaf wire crate `gandr-surface-render-remote` (its `wire` and `present` modules) declares no workspace dependency, and no renderer parses, lowers, types, marks, deduplicates, or reconstructs a semantic fact — [[../inspection-protocol]].
  The desc-render inspection notation already emits ruled surface spellings, so "the machine's vocabulary is read back in the surface's spelling" is an established posture, not a new ask ([[../../surface-language/signatures#The rulings]], ruling 5).
* **The typing machine's state is serializable data, and it builds a derivation tree as frames pop.** Frames record the context deltas they introduced; each frame pop closes a derivation node; the whole state checkpoints and resumes — [[../typing-machine#Derivation tree construction]], [[../typing-machine#Serialization and checkpointing]].
  The module is `gandr-core-checker`'s `machine` module.
* **Desugaring provenance is already recorded, as an origin map.** `gandr-surface-engine`'s `origin` module maps stable origin node IDs to CST node IDs and byte ranges and tags every synthesized node with an `ElabKind` (the `def` sugar, operator elaboration, sequencing, and the rest) — built so elaborations can be un-sugared on demand; each entry carries the node's per-node merkle hash, so a provenance key reproduces across runs ([[interactive-surface]]).
  The standing rule this register generalizes is the corpus-wide constraint that **every new elaboration records its provenance so diagnostics can un-sugar** ([[../feature-staging#Cross-cutting constraints]]; restated per form in [[../../surface-language/declarations#Elaboration behaviors, collected]]).
* **The engines' derivations are certificates, replayed rather than trusted.** A tracelet is a recorded, replayable derivation; certificate identity _is_ replay-equivalence; equality checks replay against the store rather than trust a normal form — [[../../metatheory#The certificate algebra]], built as `gandr-theory-computads`'s tracelet machinery.
  "Carried in the derivation and reproduced by replay" is therefore the house discipline this layer joins, not a mechanism it invents.
* **Content addressing is canonicalize-before-address, and the kernel's export is already maximally shared.** The storage stack canonicalizes first and hashes last ([[../implementation#Storage — content addressing, canonicalize-before-address]]); `gandr-kernel-core`'s `export` module encodes each declaration as a maximal-sharing subterm table by content-keyed dedup, deterministic at the byte level.
* **A deterministic content-hash interner exists — over the type graph, not over terms.** `gandr-core-checker`'s `intern` module hash-conses `Ty` (FNV content hash, collision buckets resolved by structural equality, per-run) so the marking layer's unchanged-type check is O(1).
  It is the shape this layer's index takes, one sort up.

**Designed, and not built.** Everything else: the dubbing judgement and the elaborator that runs it, the canonical-term index over terms, the inverse map and its population rule, the display labels, the residual memoised search, and every display consumer of any of it.

Three premises are easy to overstate, and each is stated here at the strength the tree supports.

* **No η anywhere is built, and the corpus's η discipline is polarity-sorted.** The identity fragment is β-only at rung 1 — no η, no K, endpoint comparison structural ([[../implementation#The checked language]]); codata explicitly has no η ([[../../surface-language/declarations#`codata` declarations]]); and the metatheory record's rule is that data-η is valid only call-by-value and codata-η only call-by-name ([[../../metatheory#The operational substrate — the polarized sequent kernel]]).
  The canonical form this design keys on is therefore stated **relative to the η-laws the core actually adopts**: the η-long half of the index is a premise gated on the definitional-equality phase, and [[#display-question-03]] carries it.
* **"No global hash-consing in the evaluator" is a settled performance commitment** ([[../implementation#The performance discipline]]).
  The index specified here is display-layer state populated at elaboration time, not evaluator hash-consing; it neither shares nor accelerates the evaluator's terms, and it does not ask that commitment to move.
* **The pattern-elaboration substrate is an MVP, and nothing emits a dubbing.** `gandr-surface-engine`'s `lower::codata` module carries one left-hand-side engine, exercised on the projection-copattern axis only; the data-pattern matrix is its planned generalization; the split nest the lowerer builds for tuple patterns emits no naming metadata.
  The forward judgement below is specified ahead of the eliminator elaboration that will run it, deliberately, so that lane elaborates into a display-ready derivation from day one.

## The three registers over one canonical index

One index — core terms interned at canonical form, addressed by content hash — carries all three registers, so a lookup that resolves a name, a label, or a desugaring is the same operation against the same keys.

| register                  | payload                                                   | keyed by                                                                                                     | emitted by                                         | consumed by                                             |
| ------------------------- | --------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ | -------------------------------------------------- | ------------------------------------------------------- |
| **dub table**             | a user pattern name, per dubbing                          | the canonical hash of the dubbed core term                                                                   | the dubbing judgement ([[#The forward judgement]]) | the printer, goals, diagnostics, the derivation display |
| **display labels**        | a labelled-type spelling over an eliminator form          | the canonical hash of the eliminator application spine                                                       | eliminator refinement, when it lands               | the same displays, so an eliminator never surfaces      |
| **desugaring provenance** | the elaboration tag and source span gandr already records | the synthesized node's stable origin identity (built; promotion to content keys is [[#display-question-04]]) | every desugaring elaboration, as today             | diagnostics un-sugaring on demand, as today             |

The registers differ in payload and authority, never in machinery: the dub table answers "what did the user call this term", the label register answers "what should this eliminator form read as", and the provenance register answers "what did the user write that became this node".
Keeping them over one index is what makes the inverse direction one lookup rather than three searches.

## The canonical-term index

The index's key discipline is the one the storage stack already lives by — canonicalize before you address ([[../implementation#Storage — content addressing, canonicalize-before-address]]) — applied to core terms at the checker's boundary:

* **Canonical form is β-short η-long, relative to the adopted η-laws.** Reductions are contracted and introduction forms are η-expanded, so a term and its η-expansion are one key — the normal form the source design's free-variable analysis independently insists on, arriving here because gandr canonicalises anyway.
* **The key is a deterministic content hash**, the shape of `gandr-core-checker`'s `intern` module carried from types to terms: a fixed hash with collision buckets resolved by structural equality, per-run and golden-stable, never a randomized hash.
* **Interning is display-layer state.** It is populated when the elaborator emits metadata and consulted when the renderer side prints; it shares nothing with the evaluator, consistent with the no-global-hash-consing commitment named above.
* **Binders are α-invariant**, as every content-addressed structure in this tree already is — content-addressed identity is structural only, with no arena addresses, generation stamps, or session state in hashed identity ([[../implementation/roadmap]]'s standing constraints).

What the index is **not**: a quotient the kernel decides.
Nothing in this layer is a typing fact; the kernel's conversion is untouched, and the index's notion of sameness serves display and only display ([[#display-rule-04]]).

## The forward judgement

A surface pattern is elaborated _against a core-language input_, not merely scoped [@sterling-2026-pterodactyl-worklog].
The judgement form, with core term $M : A$ as input, surface pattern $p$ as subject, and a dubbing sequence $cal(D)$ as output:

$$ Gamma tack M : A triangle.l p arrow.squiggly cal(D) $$

The rules deconstruct $M$ in lockstep with $p$, **respecting the judgemental equality of $M$** — so an η-expanded form is indistinguishable from the variable it expands — until a base case:

* a dot pattern $.(e)$: elaborate $e$ at $A$ and check it judgementally equals $M$;
* an identifier $a$ not yet dubbed: emit the dubbing $a mapsto M$;
* a wildcard: stop.

Two properties are the design, and both are stated rather than assumed.

* **A name may dub a complex term.** The core label under a user name may be η-expanded, or a projection of a bound variable, and the surface must not syntactically distinguish these from variables.
  This is what makes the table a _dub table_ rather than a renaming context, and it is the whole reason the inverse direction has content.
* **The judgement refines an elaboration discipline, it does not invent one.** It is the low-level, name-distinguishing form of the elaboration whose figure-level presentation abstracts internal and user names away — the elaboration-of-definitions discipline of [@dagand-mcbride-2012-elaborating], with the naming made explicit and judgemental.
  The claim here is lineage, read from the source's own account of what it refines [@sterling-2026-pterodactyl-worklog]; the older system's relabelling algorithm is declined wholesale ([[#The two declines]]).

Dubbings are **emitted against canonical forms**: the $M$ bound to a name is interned at canonical form at emission, which is what the inverse map's population rule keys on.

## The inverse map

The inverse of the dub table is the structure the source design found unsatisfying, and it is where this layer spends its design budget.
Forward emission is a judgement; inverse display is an **index maintained beside elaboration**, never a search over names at print time.

### The population rule

Every dubbing $a mapsto M$ the forward judgement emits inserts one entry into the inverse multimap at emission time:

$$ "hash"(M_"canonical") arrow.squiggly a $$

* **Incremental, never batch.** The map is a function of the emitted dubbing sequence, so it grows entry-by-entry as elaboration proceeds and survives as part of the derivation's metadata ([[#display-rule-01]]).
* **A multimap, not a map.** Two names may dub canonically equal terms (a user may name one term twice); the lookup returns the set and the display policy picks — most-recently-emitted, with the full set available to a hover — a presentation choice, not a typing one.
* **η-expanded dubbings match for free.** Because keys are canonical, $alpha$ and $lambda x . alpha med x$ hash to one key: the dubbing $a mapsto lambda x . alpha med x$ is found by a printer that meets the unexpanded $alpha$.

### The lookup rule

The printer meeting a core term canonicalises it and looks the hash up.
That is the whole common path: **constant-time in the common case**, including every η-expanded dubbing, with no definitional-equality check anywhere on it.
The worked example, in surface spelling:

```text
def uncurry(f : A -> B -> C) : (A × B) -> C
  | uncurry(λ x y => h(x, y)) => h

-- elaboration emits the dubbing
--   h ↦ λ u => f (u.1) (u.2)        (u the internal name for the argument)
-- the inverse table, keyed by canonical hash:
--   hash(λ u => f (u.1) (u.2)) ↦ h
-- a goal displaying f's elaborated use prints h, not the λ-expansion
```

### The residual case is a bounded search, not a lookup

One shape escapes the index, and the source design's account of it is exact: a core term **definitionally but not canonically equal** to any dubbed term.
The `swap` example is the minimal case [@sterling-2026-pterodactyl-worklog]:

```text
def swap (u : A × B) : B × A
  | swap(x, y) => (y, x)

-- u.1 is dubbed x; u.2 is dubbed y.
-- The printer meeting u itself has no dubbing to consult,
-- and pair-η says the answer should print as (x, y).
```

No key helps: `u`'s canonical form is `u`, and the dubbings name its projections.
This layer's answer is the source's floor solution, taken with two tightenings and stated honestly:

* **The search domain is the dub table's canonical representatives only** — the handful of names in scope at the display point — never the naming context at large.
* **The search is memoised, and the memo is interned.** A definitional-equality probe runs once per (term, table) pair; its outcome — name, reconstructed display form like `(x, y)` by pair-η, or no-name — is interned into the inverse map under the term's canonical key, so the search never runs twice for the same pair.

The honest record: this is the part the source calls unsatisfying, and this design does not dissolve it — it shrinks its domain and amortises its cost.
Whether pattern-unification machinery can boil the dubbings down further is an open direction, carried as [[#display-question-02]] and **not claimed**.

## The rules that keep the layer honest

### display-rule-01

**Dubbings are elaboration metadata, carried in the derivation — never typing-context data.** The context stays minimal and kernel-owned; naming is display-layer.
The derivation is already the unit gandr stores, checkpoints, and replays ([[../typing-machine#Serialization and checkpointing]]), so the metadata rides existing machinery and costs the kernel nothing.
The declined alternative — dubbings in the typing context — is recorded at [[#The two declines]].

### display-rule-02

**The inverse map is populated at emission and consulted at display; the common path never searches.** A lookup that needs a definitional-equality probe is the residual case, is bounded to the in-scope dubbings, and is memoised with the memo interned.
A printer that walks the whole dub table on every term is the failure this rule exists to prevent.

### display-rule-03

**Display identity is not typing identity.** The index's content-hash equality, and the residual search's definitional probes, decide _what a term prints as_ and nothing else.
No name, label, or provenance entry may flow back into conversion, the solver, or the kernel — the same wall the corpus keeps between certificate equivalence and type-level identity ([[../implementation/roadmap]]'s standing constraints).

### display-rule-04

**The layer sits behind the renderer firewall and requires no kernel change.** Every register projects through the same checked-frame seam as the rest of the reified state ([[../inspection-protocol#The render bus]]); the kernel's TCB wall ([[../implementation#Architectural commitments]]) is neither widened nor touched — no `kernel-*` crate gains a name, a label, or a hash of one.

### display-rule-05

**Replay reproduces the display metadata exactly.** Each register entry is emitted by a deterministic elaboration step recorded in the derivation, so replaying the derivation re-emits the identical dubbing sequence, labels, and provenance — and the inverse map, a pure function of that sequence, rebuilds entry-for-entry.
This is the same determinism the incremental lane's differential gate enforces between reused and from-scratch checking ([[../incremental-pipeline#The differential gate]]), and the same replayed-not-trusted discipline the certificate layer defines identity by ([[../../metatheory#The certificate algebra]]).
It is a property the design inherits from those two commitments, not a mechanism of its own — and it is the acceptance shape of [[#display-test-03]].

## The acceptance tests the design owes

### display-test-01

**The `swap` pair-η case.** Elaborate the `swap` clause above; display a goal that mentions `u`.
Expected: `u` prints as `(x, y)` under the recorded dubbings of its projections, by exactly one memoised residual probe whose result is thereafter a lookup.
This is the case the source design leaves open [@sterling-2026-pterodactyl-worklog], and it is named here as the layer's first acceptance test so the open case is the thing demonstrated, not the thing avoided.

### display-test-02

**The `uncurry` η-expansion case.** Elaborate the `uncurry` clause above; display a goal that mentions `f`'s use as `λ u => f (u.1) (u.2)` — including after reductions the dubbing table did not witness.
Expected: the name `h` prints, by lookup alone, because interning canonicalises.

### display-test-03

**Replay fidelity.** Checkpoint a derivation mid-clause, resume it, and replay it from the recorded peak.
Expected: the resumed and replayed runs carry byte-identical display metadata — dub table, labels, provenance, and the interned inverse map — against the from-scratch run, the differential-gate discipline applied to this layer's registers.

### display-test-04

**The label register's reason to exist.** A program refined through eliminators displays its goals under labelled-type spellings; no eliminator application spine is ever shown to the user who wrote none.
This test lands with eliminator refinement, not before; it is specified now so that lane inherits the register rather than retrofitting it.

## The labelled-types register

The dub table is one species of a larger genus: **display provenance layered over core terms so the user never meets the machine's vocabulary**.
Its sibling is the labelled-types mechanism of _The View from the Left_ [@mcbride-mckinna-2004-view-left] — the device by which a user programming with eliminators never _sees_ an eliminator in a goal, and by which the intended general-recursive program is recovered from its eliminator form for execution.

* **The foundation is the elimination-operator line**: McBride's thesis is the elimination-operator development the labelled types present [@mcbride-1999-thesis], and the recovery of the general-recursive program from its eliminator form is Brady's thesis's development [@brady-2005-practical-implementation].
* **The connection is a design claim, stated at claim strength.** The three Epigram-line works are held in the register; their abstracts and the source design's own account of the connection were read, and the works' bodies were not re-read at this pass.
  What is claimed here is the genus — names, labels, and desugaring provenance as one layer over one index — not any theorem of those papers.

## The two declines

Both are the source design's own, adopted with its reasons, and both are recorded so the corpus's revisit obligation attaches to them.

* **Dubbings in the typing context — declined on architecture, no reversal condition filed.** The context is kernel data and stays minimal; naming is elaboration metadata.
  The delta that would change this is a demonstrated need for the kernel itself to print, which the renderer firewall ([[#display-rule-04]]) is designed to prevent; the decline therefore rests on architecture, not on cost.
* **The Epigram-1 relabelling algorithm — declined wholesale.** That algorithm abstracts the expected left-hand side, walks the user pattern accumulating a relabelling, and rebinds; the source records it as complicated, error-prone, and probably η-disrespecting [@sterling-2026-pterodactyl-worklog].
  The judgement form of [[#The forward judgement]] replaces it whole: deconstruct the core input against the surface pattern, respecting its judgemental equality, until the dot-pattern, identifier, and wildcard base cases.
  Nothing of the older algorithm is retained, so there is no residual to disposition.

## Open items, dispositioned

Every open item the source leaves carries exactly one disposition here.

### display-question-01

**The residual search's completeness boundary.** Which definitionally-but-not-canonically-equal shapes beyond pair-η reconstruction the memoised probe must cover — record-η, unit-η, and whatever the definitional-equality phase adopts — and at what fuel discipline.
**Disposition: carried.** The probe is bounded to in-scope dubbings and memoised, but its coverage is exactly as wide as the η-laws the core lands, so the question is gated on [[#display-question-03]].

### display-question-02

**Boiling the dubbings down.** The source's hope that nested-pattern-unification-style machinery could reduce the dub table to something the inverse needs no search over at all, with the pattern-matching-unification boundary account recorded as the direction's present literature [@richter-bohler-2026-pattern-matching-unification].
**Disposition: carried as an open direction, explicitly not claimed** — the present design takes the floor solution with a shrunk domain and an amortised cost, and any pattern-unification refinement lands behind [[#display-test-01]] continuing to pass.

### display-question-03

**Which η-laws the canonical form keys on.** The corpus's η discipline is polarity-sorted — data-η valid call-by-value, codata-η valid call-by-name ([[../../metatheory#The operational substrate — the polarized sequent kernel]]) — and nothing η-shaped is built in the core today.
The index's η-long half is therefore specified relative to the definitional equality the identity layer's own phase lands, and this question carries that dependency.
**Disposition: carried**, gated on the definitional-equality phase; [[#display-test-01]] presupposes pair-η and lands with it.

### display-question-04

**Promoting the provenance register's keys.** The origin map's built keys are stable origin node identities with per-node merkle hashes; the other two registers key on canonical content.
Whether the provenance register moves onto content keys outright, or keeps node identity for span-bearing reasons (a span is a property of a node, not of a term), is a representation question the first landing answers by measurement.
**Disposition: carried**; the table above records both registers over "one index" at the discipline level — canonicalize before you address — not at the cost of unifying two key types that may legitimately differ.

### display-question-05

**One judgement instance per injective constructor, or a consolidated rule.** The source notes the annoyance of implementing the deconstruction structure for every definitionally injective constructor, mirroring the term-formation rules, and flags consolidation as tempting but possibly unwise [@sterling-2026-pterodactyl-worklog].
**Disposition: carried**, with the source's hesitation recorded as the design's own.

## Source and confidence

Written against four sources, named because a change with no declared source set cannot be fidelity-reviewed.

1. **The Pterodactyl worklog, trees 01KK and 01KL** [@sterling-2026-pterodactyl-worklog] — the forward judgement form and its three base cases, the dub-table ambition, the memoised definitional-equality floor and the author's dissatisfaction with it, the `swap` and `uncurry` examples, the pattern-unification hope, and both declines — read from the live pages at this pass (2026-08-07).
2. **The tree**, for every as-built claim: `gandr-surface-render-remote`'s `wire` and `present` modules (the firewall); `gandr-core-checker`'s `machine`, `intern`, and `origin`-adjacent modules; `gandr-surface-engine`'s `origin` and `lower::codata` modules; `gandr-kernel-core`'s `export` module; `gandr-theory-computads`'s tracelet machinery.
3. **The corpus documents linked at each claim** — the inspection protocol, the typing machine, the incremental pipeline's differential gate, the metatheory track's certificate algebra and polarity-sorted η discipline, the surface track's declarations and signatures rulings, and the feature-staging constraints — each read at the linked section rather than restated from memory.
4. **The Epigram-line literature** [@mcbride-mckinna-2004-view-left] [@dagand-mcbride-2012-elaborating] [@mcbride-1999-thesis] [@brady-2005-practical-implementation], cited from the register with its abstracts and the source design's account of the connection; the works' bodies were not re-read at this pass.

**Confidence, by class.**

* **High** — the as-built premises, each verified against the named module at write time; the forward judgement, transcribed from the source read at this pass; and the two declines, which are the source's own.
* **High** — the inverse map's population and lookup rules and the residual case's shape, which are this document's design content and stand as design, with their acceptance tests named.
* **Medium** — the labelled-types genus claim, whose citations are held but whose bodies were not re-read; the genus is asserted at design strength and no theorem is cited.
* **Marked at the claim** — every η-dependent statement, which is conditional on the definitional-equality phase the corpus schedules and this document does not.
