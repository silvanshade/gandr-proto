# The signature surface — `sign` canonical, `data`/`codata` as sugar

The signature unification's ruling of record (owner, 2026-08-02, in design dialogue; executed under gandr-ng9.18 after its two reading sweeps closed): what the canonical signature block is, what the `data` and `codata` declarations desugar to, where polarity lives, and which signature-interaction mechanisms the design commits to first.
The block form's concrete spellings — judgment-style members, the four-glyph arrow grid, ports and body statements — are ruled at [[circuit-cells#The block form, ruled]] and are not restated here; the declaration forms the sugar starts from are [[declarations]]; the description universe this surface elaborates into is the [[../metatheory#Cellular data — descriptions, cells, and computads|metatheory track's cellular-data section]].

## The rulings

Six commitments, taken so execution sessions execute rather than re-litigate.

1. **`sign` is the canonical block form**, uniform across the corpus.
   Naming the signature is meaningful — multi-sorted signatures, named presentations for higher cells, first-class descriptions — and `sign Nat` is the degenerate single-sort case, the block's own name its one implicit sort.
2. **`data` and `codata` blocks are the polarity-carrying sugar** over single-sort signatures, desugaring to the nested generator block plus the polarity token — never to item-level members.
   The constructor-block form — `data Nat { Zero, Succ(n : Nat) }` — and the item-level `data` member are **retired**: the nested block is the one data-declaration form, and the separation argument (three side conditions the separated form must check where the nested form cannot express the violation) is recorded at [[declarations#data declarations]].
   Polarity is load-bearing (the guardedness/delay licence reads off codata-ness) and must survive every desugaring; the discipline that carries it is [[#The sorting discipline carries the licence|below]].
3. **Mixed inductive and coinductive sorts in one signature are ruled in**, under the ergonomics lens — a system too rigid from the signature notion will not work out.
   The ruling executes as the **cheap reading**: several polarity-homogeneous sorts in one `sign` block, mutual where needed; the expensive reading — one sort whose fixpoint alternates internally — is a deliberate universe change this design does **not** take ([[#The internal-alternation fence]]).
4. **Internal vocabulary mirrors the surface, with one exception**: `ctors` stays `ctors` (renaming it to `data` would reintroduce the ambiguity the surface avoids); `ops` respells to `opers`, `cells` to rule faces, type names following — as built, the description table is `SignDesc` with `opers` (`OperDesc`) and `rules` (`RuleFace`), and the pattern-variable context vacated the signature vocabulary (`PatternContext`).
5. **The desc-render inspection notation emits the ruled surface spelling** — every description reads back as its `sign` normal form, which subsumes the inspection-notation half of gandr-r38.
   The normal form's members are sorts, operations, and rules only: the item-level `data` / `codata` member is retired, so constructor and observation descriptors have no member spelling and the render omits them (they stay inspectable through the description itself).
6. **[[higher-cells]]'s member ladder aligns to the ruled form**, re-derived rather than mechanically respelled — its 3-cell-arrow question dissolves into the arrow grid, where dimension is read off the endpoints, never off the arrow.

## One sort-indexed description

The swept literature converges from independent directions on one shape: a multi-sorted signature is **one description** — a flat operator table over a sort set, each operation carrying its result sort — and the per-sort view is the fibre of the sort map, available as surface presentation only [@fiore-szamozvancev-2022-formal-metatheory] [@fiore-kammar-moser-staton-2025-mast].
The description universe therefore grows a **sort index on the same universe** — never a signature stratum and never a mere grouping — and the `sign` block is surface grouping notation over one tagged, sort-indexed description.

As built (`gandr-theory-levitation`): `SignDesc.sorts` is the declared sort set, `Code::Var` carries the sort a recursive occurrence targets, `CtorDesc.result` is the real result-sort slot (the block sort in the degenerate case; a generator's result head, which the nested form constrains to the family), and `check_desc` enforces the sorting discipline — distinct sort names, declared result and `var` sorts, and the stage-0 polarity-homogeneity of ruling 3's cheap reading.
The constructor layer and the operation layer now demonstrably carry **one container shape**: `CtorDesc::arity` reads every constructor as a single-output bridge arity over the sort set, the agreement the sweep found implicit in `BridgeArity` made checkable.

Two hazards are priced into the index, and both are load-bearing rather than stylistic:

- **No self-hosting.** A `sign` form must never be a first-class description quantifying over all descriptions in its own universe — the Girard-collapse variant met in the swept signature calculi [@sterling-2022-existential] — so the index is a genuine universe move, and a sort is never itself a description.
- **Finite declarations only.** Only finitely presented declarations are importable, never induced endofunctors, or decidable code equality dies [@fiore-hamana-2013-multiversal] — and decidable code equality is what content-addressing interns on.

The answer stratifies by how rich the sorts are: simple sort sets take the index (adopted); sorts that are themselves a theory's algebras make the index dependent — the "grows a level" setting [@fiore-hamana-2013-multiversal], relevant only if gandr's sorts become theory-algebras; and rules-as-signatures is a genuine register level that decodes to categories of models rather than types — a different object, recorded as a live design fork, deliberately not taken here.

## The desugarings

The sugar ladder, stated normatively; each row's target is the form the elaborator sees, and all surface forms reach **one description shape** (as built: the `data`/`codata` path and the `sign` path both produce a `SignDesc` whose degenerate sort set is the block's own name at the block's polarity).
The retired rows are kept visible because the declines reference them; the stage-0 elaborator declines a retired spelling with the respelling hint rather than repairing it.

| surface                                                      | desugars to                                                                                                              |
| ------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------ |
| `data Maybe(a : Type) : Type { … }`                          | the nested block itself, at μ polarity: one sort `Maybe`, one generator per member, parameters bound once at the head    |
| `codata Stream(a : Type) : Type { … }`                       | the nested block itself, at ν polarity: one sort `Stream`, one observation per member                                    |
| `oper add(m : Nat, n : Nat) -> Nat` (data block)             | `oper add : (m : Nat, n : Nat) --> (Nat)` — one member form shared with the sign block's judgment style                  |
| `rule lhs ==> rhs` (either block)                            | the same rule-face member the sign block writes                                                                          |
| ~~`data Nat { Zero, Succ(n : Nat) }`~~ (retired)             | ~~`sign Nat { sort Nat : Type; data Zero : Nat; data Succ : Nat --> Nat }`~~ — declines with the nested-block respelling |
| ~~`data Zero : Nat` (sign member)~~ (retired)                | ~~an individual constructor member~~ — the item-level member is gone; declare the family with the nested block           |
| ~~`codata head : Stream --> a` (sign member)~~ (never built) | ~~the stated normal form~~ — superseded by the retirement before its grammar landed                                      |

- The **member keyword carried the polarity** in the retired item-level form; in the nested form the polarity lives on the BLOCK (`data` μ, `codata` ν), which is what the desugaring preserves — the polarity token must survive every desugaring, and here it does so by construction.
- The operation member respelled from `op` to **`oper`** in the data-block sugar; `op` is the operator-fixity declaration only, and the retired lead parses-and-declines with the respelling hint (the retired-`~>` precedent).
- Every desugaring is recorded with provenance, so diagnostics and the derivation UI un-sugar on demand — the standing rule of [[declarations#Elaboration behaviors, collected]].

## The sorting discipline carries the licence

Polarity's home is fixed by the mixed induction-coinduction sweep's findings (gandr-ng9.20, mixed-sweep-verdict-02): the tag is **per-declaration** — a polarity token naming which fixpoint a sort's declaration takes, exactly gandr's built `DeclPolarity` — and a sort-level _partition_ of the universe is refuted by worked calculi that tag polarity per fixpoint rule and fold nothing into sorts but strict positivity [@basold-geuvers-2016-dependent] [@basold-2018-mixed].
The flag only **names** the fixpoint; what keeps the system honest is that the **polarity-specific obligations discharge in the sorting discipline** over the declared sort set — positivity for μ-sorts, productivity for ν-sorts — at the sign block's sorting rules, never at the tag itself (as built at stage 0: `check_desc`'s sort checks are the discharge point's home; positivity and productivity checks land with their rungs).

**The guardedness/delay licence is a typing obligation, never a right-hand-side syntactic check.** The licence reads off codata-ness and must survive every desugaring, and the sweep put that commitment at theorem grade: desugaring preserves well-covering in the anchor calculus [@basold-2018-mixed], and the categorical dichotomy says exactly that a syntactic right-hand-side condition does not survive desugaring while a typing obligation does.
Nothing in this surface may therefore specify productivity as a shape condition on equation right-hand sides; the licence is carried by the sorting/typing discipline the desugaring targets.

## The internal-alternation fence

Ruling 3 hides two different rulings, and this design takes only the first.

- **Cheap (taken):** several polarity-homogeneous sorts in one `sign` block, mutual where needed, sharing an erased measure — well precedented, and what `check_desc`'s homogeneity check currently scopes each declaration to (the per-sort tags in `SignDesc.sorts` are the general carrier as mixed blocks land).
- **Expensive (fenced):** one sort whose fixpoint alternates internally requires **internal fixpoint codes** — a universe change, not a decoder change.
  The one swept universe carrying internal codes for both fixpoints pays positivity in the sorting judgement and keeps first-order code equality decidable, but its conversion rests on a confluence problem the source itself leaves open [@basold-2018-mixed]; the Morris universe the earlier premise attributed both fixpoints to carries an internal μ only [@morris-2007-universes].
  Taking the expensive reading is its own future decision against that machinery, never a consequence of this one.

## The interaction ladder

No swept source has polymorphism over signatures; interaction decomposes into separable mechanisms, and the design commits to specifying them **cheapest first**:

1. **parameters as polymorphism** — uniform datatype parameters, already the surface's `data Maybe(a)` shape;
2. **reindexing and transport along sort maps**, with signature composition;
3. **free-monad terms** of one signature over another's carriers;
4. **dependence** — one signature's rules stated over another's semantics (typing needs and fulfillments, strictly cheaper than any morphism notion [@fiore-kammar-moser-staton-2025-mast]);
5. **prenex schema closure** — rules polymorphic in sorts as schemas closed under sort substitution, with the sharp boundary that prenex needs no new machinery and impredicative quantification breaks the reduction [@sterling-2022-existential];
6. **cross-sort translation**, now cheaply available rather than deferred: a translation between signatures over different sort sets is finite data — a label map and a sort map with one arity equation — acting contravariantly on models with a computed form and a recursion principle by initiality [@ahrens-lafont-lamiaux-2025-2functoriality]; the result is 1-functorial (the paper says so itself) and locked to the untyped-arity setting by its own open-problem list, and it gates on **the same sort index** as the unification — one more payer at the gate the index already paid for.

What stays deferred is **interpretation** — sending an operation to a derived term.
The translation carrier moves sort renaming and label matching only; the cross-sort paper's own worked delimiter is the proof (its PCF-to-untyped-λ morphism fails for want of a suitable sort map, and the interpretation is then built by hand outside the morphism), so the refinement-versus-interpretation wall stands where the ornaments record put it, with interpretation living in the declined general container morphism [@gambino-kock-2013-polynomial] and the multiversal translation apparatus [@fiore-hamana-2013-multiversal].

## As-built rung and witnesses

- The `sign` grammar, arrow grid, and sign-to-description lowering are landed ([[circuit-cells#The block form, ruled]]; `surface-engine`'s circuit lowering), with `sort` members now recorded into the declared sort set and a port at a declared sort read as a genuine recursive occurrence.
- The sort index, result slots, sorting-discipline checks, and container agreement are landed in `gandr-theory-levitation` with crate tests; the μ decoder reads the single-sort fragment and declines multi-sorted descriptions and foreign-sort recursion by name.
- The inspection notation emits the sign normal form (ruling 5), witnessed at the desc-mode corpus (`model/desc/desc-inspect.gandr`, `model/circuit/circuit-rule-block.gandr`, and `surface-engine`'s elaboration tests) — sorts, operations, and rules, with no constructor member spelling.
- The `oper` respell and its retired-`op` decline are witnessed at `surface/data-operation-members.gandr` and the engine's decline test.
- The nested generator block is landed end to end: the grammar admits it (the retired head and member spellings stay admissible for their declines), the stage-0 elaborator reads it with head uniformity enforced, and the corpus migrated in the same change (`model/data/`, `model/codata/`, the decline goldens under `pathological/`).

## Open questions and residuals

- **The rules fork** — rules pushed _up_ into a register level versus _down_ into transition sorts are incompatible designs, both mapped by the signature sweep; this document deliberately takes neither, and the fork is decided when rules-as-signatures earns a customer.
- **The multi-sorted decoder** — `decode_desc` reads the single-sort fragment; the genuinely indexed decoder is the indexed-universe lane's, alongside the multi-output generalization ([[../metatheory#Cellular data — descriptions, cells, and computads]]).
- **The ornament gate** — the ornament requirements ledger is design input for the same sort-index gate, paid once; its owed constructions ride the ornaments proposal (now in the project's research vault — the corpus README's migration banner).

## Source and confidence

The rulings are the owner's, 2026-08-02, transcribed in the tracker's ruling of record (gandr-ng9.18) after two reading sweeps (gandr-ng9.17, the polymorphic-theory sweep; gandr-ng9.20, the mixed induction-coinduction sweep) whose verdicts and design inputs this document's claims cite by register key.
Every as-built claim was verified against the tree at write time, with the module named; the sweep-derived claims carry their sources inline, and a claim resting on a read rather than a source says so at the claim.
