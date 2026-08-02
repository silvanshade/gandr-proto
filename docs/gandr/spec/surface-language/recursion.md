# The (co)recursion surface

Where recursion and corecursion are declared, where their termination and productivity evidence lives, and how that evidence grows into sized types, cost annotations, and explicit implicit-instantiation without retrofit.
The core former (`fix x. t`, recursion through a graded thunk) and the loop sugar are the substrate this surface lowers into; this document owns the **surface discipline** — the scope rules, the call-site markers, and the ladder that strengthens what a marker means.
That substrate **does not exist in this tree**: the kernel's computation vocabulary has no fixpoint, so a recursive definition scope-checks and then declines at lowering.
Its design — the former, its checker and operational rules, its machine realization, and the termination ladder past the step budget — is [[../implementation/proposed/recursion-former]].

## The decision: a hybrid of two sites

A **recursive scope** — `def rec` for a single self-referential definition, `rec { … }` for a mutual group — is opened at the declaration.
A **recursion marker** — a **direction sigil** written in the bracketed instantiation slot — is required at every recursive occurrence:

```text
def rec add(m: Nat, n: Nat) -> Nat {
  case m {
    Zero    => n,
    Succ(p) => Succ(add[<](p, n)),     // `<` claims descent into an inductive argument
  }
}

def rec nats(m: Nat) -> Stream(Nat) {
  .head => m,
  .tail => nats[>](m + 1),             // `>` claims production of coinductive output
}
```

* The marked occurrence is the **only** reference to the fix-bound variable: an unmarked occurrence of the definition's own name inside its scope is a **hard error carrying a did-you-mean suggestion naming the marked form** — never a silent resolution.
* The error-not-capture clause is the safety argument: under the naive alternative (a plain name resolves to an outer binding), forgetting one marker silently captures an imported or shadowed binding of the same name — a strictly worse footgun than shadowing confusions, and the rule kills it along with the `nonrec` keyword in one stroke.
* An outer or imported binding of the same name stays reachable **only through a qualified path**, so the legitimate outer-reference idiom survives.
* The declaration-site half keeps independent value: a reader learns from the header alone that a definition is self-referential without scanning the body, and the scope keyword costs no new shared-prefix window (`def rec` folds into the `def` family).

The two sites answer different questions, so **carrying both is not redundancy**: the `def rec` header is scope information; the call-site marker is **evidence information**.

## The instantiation slot

The load-bearing syntactic finding is that the **bracket plane already exists** — the postfix bracketed instantiation form `e[T, …]` — and it already uniformly denotes the erased, second-class tier: type instantiation `e[T]`, type abstraction `fn [a] { … }`, the grade slot `thunk [g] { … }`.

The recursion marker is therefore **not a new syntax class**: it is the existing instantiation slot admitting two new interior atoms.
Under the sized-types rung this stops being a pun and becomes literally true: `f[<]` is instantiation of an erased size argument at a fresh size strictly below the ambient one.

```text
e ::= e [ ι₁, …, ιₙ ] | e ( e₁, …, eₘ ) | …
ι ::= T | < | > | x = e | m < | size = e | cost = e | tail
```

* The slot is a comma list from day one: the grammar accepts the full list, and the engine **declines every resident it does not yet implement, by name**.
* The reserved residents and their intended readings:
  + `<` / `>` — the direction sigils (this document);
  + `m <` — a **named measure**: which argument descends;
  + `size = e` — explicit size instantiation;
  + `cost = e` — a cost bound, in the cost-as-effect reading: recurrences are extracted at recursive calls, so a per-call slot is exactly where a cost bound lives;
  + `tail` — asserts the call must compile as a tail call.
* A function that recurses on an inductive argument while corecursing into its coinductive result marks one call with a list, `f[<, >]` — which is why **the slot cannot be a single atom**.
* The implicit-arguments interaction resolves by unification, not collision: term-level implicits arrive by contextual-metavariable elaboration, and any implicit system eventually needs an explicit-instantiation surface — **this same slot**, `f[i = e]`, so one occurrence-modifier slot serves type arguments, sizes, direction sigils, and explicit implicit instantiation instead of two bracket languages competing for one opener.

## The productivity ladder

The staged strengthening of what a marker means — **no rung's syntax ships before its check is designed**:

| rung            | marker semantics                                                                                                                                                                                                                             | checked obligation                                                                                                                                                                   | status                                                                                                                                     |
| --------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------ |
| **scope**       | scope evidence only; the sigils record direction intent, unchecked                                                                                                                                                                           | a marked occurrence must target a definition of the enclosing recursive scope; reserved residents declined by name; unmarked self-reference is an error carrying the marked spelling | **built** — parse gates and the scope pass are green                                                                                       |
| **guardedness** | the sigils become checked claims: `<` obliges some argument of the marked call to structurally descend; `>` obliges the marked call to sit under at least one copattern observation (the standard guard discipline of copattern corecursion) | the structural-descent and guard checks are total on the fragment; an escaping reference is declined with a diagnostic saying which rung refused it                                  | adopted-unbuilt; go: recursive lowering lands and both checks are total; no-go: any sized-type semantics leaking into scope-checked syntax |
| **sized**       | both sigils elaborate to erased size applications at a fresh size strictly below the ambient one; the explicit `size =` form opens in the same slot; conversion quotients the annotations under size irrelevance                             | the sized-types design pass, including the bounded-quantification and infinite-size interaction                                                                                      | reserved, gated on that design pass — never on enthusiasm                                                                                  |

One normative caveat is fixed now so the sized rung inherits no lie: in the sized-type semantics, **both recursion and corecursion consume size** — a coinductive type sized by observation depth has its corecursive call at strictly smaller depth, exactly as an inductive traversal descends.
Both sigils therefore mean the same thing normatively (instantiate strictly below the ambient size); the `<`/`>` split is **presentational**, keyed to the polarity of the type being traversed or produced — `<` for descent into a positive argument, `>` for production of a negative result.
The split is kept anyway: the consuming-input/producing-output distinction is what a human reader wants at the call, and the design refuses to pretend it is a semantic distinction.

The ladder's ordering is also a soundness hedge: sized types have a troubled implementation history (the interaction of bounded size quantification with the infinite size produced real inconsistencies in Agda's implementation; the underlying MiniAgda-line theory is sound).
Keeping the marker's normative semantics at "scope plus direction claim", with guardedness checked before any size semantics ships, means **nothing downstream depends on sized-type semantics before that rung is deliberately designed**.

## One `def rec` family

There is deliberately **no `def corec`**:

* The body shape already discriminates: a statement body is recursive; a copattern-clause body (`.head => e`, `.tail => e`) is corecursive, elaborating through the cosplit case-tree node to a record of thunks.
* A second keyword would re-encode information the body carries, would cost a second mold in the definition family, and — decisively — would force mixed recursive/corecursive functions to pick a side they do not have: a stream transformer descends an inductive argument while producing guarded coinductive output, and that is one definition whose calls carry `[<, >]`, not two definitions.
* The single `{ … }` body with an interior alternative keeps the two body forms from opening two brace-led branches in one context (the duplicate-tile discipline).

The sequent reading is recorded as a correspondence obligation: in the classical account of (co)recursion, the corecursor's seed sits on the **consumer** side of a cut — dual to the recursor's argument on the producer side — so when the L fragment grows far enough to express it, the `>`-marked occurrence should acquire an L face as the seed side of a coiterator cut.
**The marker discipline must survive that translation unchanged; if it does not, the surface design — not the kernel — is what moves.**

## Mutual recursion

The `rec { … }` block opens one scope for the group:

```text
rec {
  def even(n: Nat) -> Bool { case n { Zero => true,  Succ(p) => odd[<](p)  } }
  def odd (n: Nat) -> Bool { case n { Zero => false, Succ(p) => even[<](p) } }
}
```

* A marked call may target **any member** of the group — which a self-only construct cannot express, and the sized semantics justifies (the group shares the descending size).
* Call-only marking was considered and rejected: mutual recursion needs declaration-site grouping regardless of call-site discipline — inside `even`, a call to `odd` requires `odd` in scope before `odd`'s definition is complete, and something must open the group's scope.
* The alternative that dissolves the problem — the whole file as one implicit recursive scope — is rejected on principle: **implicit recursion everywhere is exactly the default this design exists to remove**.
* The elaboration desugars a group to one `fix` over a bundle (a record of the mutually recursive thunks, each member projecting its sibling).

## Type-level self-reference

The analogous pain at the type level dissolves without new syntax, because its cause is shadowing, not termination:

* Declared data is **generative-nominal**: the declaration mints a fresh nominal id, and inside the block the declared name denotes the block's own fresh nominal — **data is inherently recursive, and that recursion is what declaring a datatype means**.
* An outer binding of the same name is reachable only by qualified path; the re-export idiom is written as a qualified alias instead.
* **No termination evidence attaches to type-level self-reference**, because none is owed at the surface: strict positivity is the checker's obligation over the declaration table, not a claim the author repeats per occurrence.

## Application syntax, rejected

Considered alongside this design and rejected: switching application from `f(a, b)` to juxtaposition `f a b` — a rejection that is **doctrinal and economic, not technical** (the tile calculus already melds juxtaposed atoms inside shell blocks, so the parser could carry it).

* Doctrinally: the data surface rules that a constructor's surface is a field-tuple, and that writing a constructor as an arrow is a polarity lie; curried juxtaposition makes partially applied constructors first-class — turning value introductions into closures mid-application, the exact laundering the constructor rule exists to prevent.
  Adopting juxtaposition would force either breaking that rule or splitting the application syntax (constructors parenthesized, functions juxtaposed) — both worse than either uniform choice.
* Economically: parenthesized application makes an arity mistake a parse-adjacent material obligation — a missing argument or unclosed call is a located hole, the tile parser's signature diagnostic strength; juxtaposition demotes every such mistake to a downstream type error and makes every adjacency a potential meld (the hazard that quarantines melding to shell blocks).
* The genuine notational win of the juxtaposition family is mixfix operators, not application; gandr reserves that outlet in the operator-declaration form, with tiles-as-data keeping open mixfix a priced option — and prover-facing beauty is a projection/display concern over whitespace-invariant identity, not a grammar concern.

## Loops, and the `break`/`continue` discipline

The loop constructs are surface sugar producing `F 1`, spliced into bind chains as statements (the fix-former design lineage):

* `for x in e { body }` desugars to a **native fold** over a concrete (finite) collection — bounded, no `fix`;
* `while c { body }` desugars to `fix self. force c >>= b. case b { false => ret unit | true => body >>= _. force self }`;
* `loop { body }` is `while true { body }`;
* **`break` and `continue` are effect operations, not `reset`/`shift`**: single-prompt delimited control cannot give `break` (outermost) and `continue` (innermost) distinct targets in one loop, so the desugaring installs two op-name-keyed deep handlers — `Break` caught by the outer handler, `Continue` by the inner per-iteration handler — and `reset`/`shift` stay reserved for user-level delimited control.
* Labeled `break 'l` is deferred growth: labels need distinct operation instances (fresh atoms per labeled loop) or named handlers, and the `'` sigil collides with the character-literal lexer.
* The `def rec` marker is the honesty about non-termination: termination is unchecked at the current rung, guarded operationally by the machine's step budget — a divergent `fix` halts at the budget with a stuck outcome; it never hangs, overflows, or mis-caches a bogus value (the black-holing memo discipline caches only a reached pure WHNF).
  The elaborations above are the substrate document's, stated here for the reading; the two-handler nesting that gives `break` and `continue` distinct targets is written out in [[../implementation/proposed/recursion-former#Loops elaborate through the former]].

## The as-built rung

The scope rung is built and verified against the tree:

* The grammar admits the six instantiation rule shapes — type argument, the two direction sigils, named measure (`m<`), explicit named form (`x = e`), the reserved `size = e` and `cost = e` forms, and the `tail` atom — with the sigils molding as atoms inside the instantiation sort, distinct from their binary-expression molds.
* A **pre-lowering, pre-kernel scope pass** (`surface-engine`, `lower/recursion_surface.rs`) validates one parsed item at a time: it resolves which bare names are fix-bound (a `def rec` scope or a `rec` block group) and classifies instantiation-slot residents before ordinary CBPV lowering sees the expression, so "the marked occurrence is the only reference to the fix-bound variable" is an **exact property**, not a convention.
* A lexical-shadow arena keeps inner binders shadowing recursive names, so qualified outer access survives exactly; total-mode recovery guarantees a scope error never consumes its sibling item.
* The finding family is named and structured: `UnmarkedRecursiveReference` (carrying the suggested marked spelling **as a field**, not as message text — the did-you-mean is part of the finding's data, which is what makes the diagnostic quality bar a testable contract), `MarkedReferenceOutsideRecursiveScope`, and one decline variant per reserved resident (`ReservedNamedMeasure`, `ReservedExplicitInstantiation`, `ReservedExplicitSize`, `ReservedCostBound`, `ReservedTailAssertion`) — the decline variant is selected by the resident's name, and the diagnostic quotes the resident text.
* The witnesses are parse-gated corpus programs (`examples/surface/def-rec-and-copatterns.gandr`, `examples/surface/operator-and-rec-block.gandr`), firewalled from execution per the corpus discipline: a syntax-only landing gets a parse-gated surface witness, and the full treatment rides the change that lands the semantics.

## Edge cases and non-decisions

* **Escaping self-reference** — a marked occurrence that is not a call (`fold(add[<], xs)`, the definition passed to a higher-order function): the grammar admits it (the slot is an instantiation form over any operand); the guardedness rung **declines** it, because structural descent through an escaping reference is not syntactically checkable; the sized rung **accepts** it, because a partially size-instantiated reference is meaningful.
  The decline is designed, not accidental, and the diagnostic must say which rung refused it.
* **Qualified call heads** in a recursive body are currently grouped as `(outer.f)(args)`; relaxing that grouping is a parser refinement, not a change to the qualified-path escape semantics.
* **Display-layer elision** of inferable markers in prover-facing renderings is endorsed as a projection concern over whitespace-invariant identity; it constrains nothing in the grammar or the stored term.
* **The `let` keyword stays unassigned** (its strongest claimant is a transparent definitional binder at the dependent rung); **the `mut` keyword stays unreserved** pending the value-semantics/modes design pass; the marker slot deliberately carries no aliasing or mode content.
* **The noise objection is priced**: a prover-flavored language recurses constantly, and every call pays three characters — but a prover requires termination evidence somewhere, and the design question is only its address; three characters at the call site is close to the **minimum honest notation** for evidence the checker needs anyway, and the whitespace-invariant identity of stored terms lets a prover-facing renderer elide inferable markers as a projection.

## Open questions

* **The sequent correspondence** — the productive sigil acquiring an L face as the seed side of a coiterator cut (above): the marker discipline must survive that translation unchanged.
* **Escaping self-reference across rungs** — the per-rung diagnostic contract (the message says which rung refused) is stated; its wording is unowned.
* **Sized-rung soundness posture** — the underlying theory is sound, but the bounded-quantification/infinite-size interaction is the standing hedge; the sized rung stays gated on a deliberate design pass, and the marker's normative semantics stays at scope-plus-direction until then.
* **Display elision** — nothing constrains or implements the projection.
* **Adjacent non-decisions** — `let` unassigned, `mut` unreserved, no aliasing or mode content in the slot.

## Source and confidence

The design record is the recursion-surface component of the parked `.gfd` corpus (high confidence — it passed its own two-axis fidelity review), verified against the as-built scope pass, the grammar's instantiation-sort rules, and the surface witnesses.
The core-former substrate (`fix x. t`, the step-budget termination stance, the black-holing discipline) is [[../implementation/proposed/recursion-former]], which this surface supersedes in presentation without touching in substance.
The earlier loop sugar and the mutual-`fix` bundle encoding are carried here unchanged.
