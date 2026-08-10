# The declaration forms

Every top-level and member declaration form, with its elaboration behavior.
Grammar machinery (bands, gates, adaptations, reserved-slot semantics) is [[grammar]]; the (co)recursion discipline (`def rec`, `rec { … }`, the direction sigils) is [[recursion]].

## The `def` family

One form-first mold with tail discrimination — `def` leads, and the token after the name selects the form:

```text
def name : T ;                        signature
def name = v ;                        value binding (top level binds VALUES)
def name(x: A, y: B) -> B' { t }      function sugar
def rec name(x: A) -> B { t }         recursive definition (see [[recursion]])
```

* An `@[…]` attribute block may precede any of them.
* The function sugar is where the U/F adjunction is hidden for everyday code and surfaced everywhere else:

```text
def name(x: A, y: B) -> B' { t }
  ⤳  def name : U[ω] (A -> B -> B')
     = thunk { fn(x: A) { fn(y: B) { t } } }
```

* The elaboration is recorded with its provenance, so diagnostics and the derivation UI un-sugar on demand.
* `def` binds **values**, sequentially, with **no self-scope** — self-reference exists only through the marked recursive forms ([[recursion]]).

## Statement binders: `val` and `run`

Two binder forms, one per CBPV sort:

```text
val p = v ;        value split (pattern p; irrefutable only)
run p <- c ;       computation-result bind
run p : F T <- c ; the answer-type annotation lane (pending)
```

* `let` is retired — the `let` keyword stays unassigned; its strongest claimant is a transparent definitional binder at the dependent rung, semantically distinct from `val`'s opaque bind.
* `run p <- c;` inside a block is the honest rendering of `c >>= p. …`: **the block is the bind-chain spine**, each statement one frame.
* Sequencing `t;` is sugar for `run _ <- t;`.
* `val` takes a pattern but only an irrefutable one (variable, wildcard, tuple, record, single-constructor); a refutable pattern requires `case`.

## Attributes

```text
@[doc("the square function")]
@[deprecated(#{ since = "0.3", note = "use sq" })]
def square(x: Integer) -> F Integer {
  ret (x * x)
}
```

* Grammar: `attr ::= name | name ( payload )`, stacked in `@[…]` blocks; payload is an expression (record payload for structured data).
* As built, attribute blocks lead `def` value/function definitions; diagnostics name the failure (`UnknownAttribute`, `DuplicateAttribute`, `MissingPayload`, `NonValuePayload`).
* **Manifest attributes** — `@[package(#{ name = "acme/parser", version = "1.4.0" })]`, `@[dependency(#{…})]`, `@[toolchain(#{ gandr = ">=0.9" })]` — parse as attribute blocks with record payloads; their `module` host declaration is the module family's business.
* **Per-symbol attribute slots** inside `data` blocks are a different, declaration-position form: `Add(l: Expr, r: Expr) [ctor, assoc, comm]` — no `@` sigil, reserved parse-and-decline.
* The quotation payload `‹Eq, Show›` (a derive-style semantic tier) is deferred growth.
* The layer behind the surface — the schema registry, the payload-typing path, attribute purity as locality, the hash-neutral side table and its inert/semantic tiers, and the report projection tooling reads attributes through — is [[attributes]].

## `data` declarations

**The one data-declaration form is the nested generator block**: a family declared once with its parameters, its index arity, and all of its generators inside one braced block.

```text
data Maybe(a : Type) : Type {
  None : Maybe(a);
  Some : (x : a) --> Maybe(a);
}
data Vec(a : Type) : Nat -> Type {
  Nil : Vec(a, 0);
  Cons : (n : Nat, x : a, xs : Vec(a, n)) --> Vec(a, 1 + n);
}
data Nat : Type {
  Zero : Nat;
  Succ : (n : Nat) --> Nat;
  oper add(m : Nat, n : Nat) -> Nat;      // reserved
  rule add(Zero, n) ==> n;                // reserved
}
data Empty : Type {}
```

* **The head binds the family's parameters once**, each a typed binder `a : Type`, and declares the index arity as the head annotation `: Idx -> Type` (`: Type` when unindexed).
  The annotation is mandatory: it is what the sort index carries, and this declaration is the indexed decoder lane's first surface customer.
* **Every generator member is a judgment** `Ctor : Side (--> Result)?`: no arrow means the side IS the result (`Nil : Vec(a, 0)` declares no fields); an arrow makes the side the payload — a parenthesized binder telescope or a bare single-field sort — and the post-arrow type the result.
  The generator's telescope is local to the member, and members are first-token-discriminated by case (uppercase-led constructors versus the lowercase-led reserved `oper` / `rule` members).
* **Head uniformity is structural, not a side condition.** Every generator's result head is the family applied to the parameter VARIABLES in order — the syntax offers no slot in which to write anything else, and the elaborator enforces it executably (index arguments are admitted unread; their semantics are the arity-substitution lane's).
  An instantiated head (`Mk : Bad(Integer)`) is declined: instantiation is uninferable.
* **Why the nested form, and not item-level members** — the separation argument.
  Item-level `data` members translate the same as the nested form if and only if three side conditions hold: head uniformity (every result head is the family applied to the parameter variables, never instantiations), one head per block (all generators construct the same sort), and family-wide positivity (the strict-positivity check ranges over the whole generator set, not per member).
  Each is a property the separated form must CHECK (against earlier members, using ordering) where the nested form CANNOT EXPRESS the violation — parameters are bound once outside the generator list, the block declares one family, and the check has exactly one scope.
  Since the violation is never wanted, forbidding is better: the item-level `data` member is retired from `sign` blocks entirely (and cleanly — in shape mode there is no `cons`/`oper` distinction at all, so no shape loses anything; see [[higher-cells]]).
* **A `data` block is the polarity-carrying sugar over a single-sort signature** (the signature unification's ruling 2): the block desugars to the nested block plus the polarity token, never to item-level members.
  The canonical block, the desugaring table, and the sorting discipline the sugar's polarity survives into are [[signatures]].
* Declared data is **generative-nominal**: the declaration mints a fresh nominal identity, so two `data Boolean` declarations in different modules are different types; type-level self-reference needs no marker (see [[recursion#Type-level self-reference]]).
  Recursion needs nothing new — `Tree(a)` inside a field is an ordinary type application.
* The reserved members, all parse-and-decline:
  + `oper name(params) -> R` — an **operation** member, with a multi-output tuple result form `-> (o1: A, o2: B)` kept local to it.
    The member respelled from `op` under the signature unification (`oper` is the 1-cell member shared with the sign block's judgment form; `op` is the operator-fixity declaration only), and the retired `op` lead still parses so the elaborator declines it with the respelling hint rather than silently accepting a synonym — the same migration device as the retired `~>` below;
  + `rule lhs ==> rhs` — a **directed rewrite** member, the user 2-cell: it elaborates (when its rung lands) to an oriented command cell on the rule's cut seam, never to an unverified equation — the anti-pragma discipline.
    The face former was respelled from `~>` to `==>` by the block-form ruling ([[circuit-cells#The block form, ruled]]), which makes `==>` the rewrite face at every position; `~>` still parses in this slot and the stage-0 elaborator declines it with the respelling, so a stale program is told what to write rather than silently accepted;
  + a **grade prefix** on a field — `Cons : (1 x : a, xs : Vec(a)) --> Vec(a)` — grade tile restricted to `{ number, ω }`;
  + a **per-symbol attribute slot** — `Lit : (x : Integer) --> Expr(a) [ctor, assoc]`.
* A declared **3-cell member** (a coherence between rewrite composites) **does not exist yet** — named here so its absence is visible.
  Under the ruled arrow grid a 3-cell needs no new member kind or arrow: a `rule` between rule-sorted endpoints _is_ a 3-cell, with dimension read off the endpoints ([[circuit-cells#The block form, ruled]]).
  The higher-dimensional design — mandatory 2- and 3-cell names, the boundary language for composites, the reserved `cell` tower, and the `Model(S)` signature-former the members exist to feed — is [[higher-cells]]; the `rule` member above is its dimension-2 rung as landed, and its anonymity is precisely what that lane's naming rung fixes.

### What is declined, with deltas

* **The Haskell/OCaml form, entirely** — bare head parameters, no head annotation, field-tuple members (`data Maybe(a) { None, Some(x: a) }`).
  There is no compatibility reader: the grammar keeps the retired shapes admissible so the stage-0 elaborator declines them with the respelling hint (the retired-`~>` precedent) rather than the parser repairing a token — a stale head declines the whole declaration, a stale member declines individually.
  Delta that would reopen: a corpus body large enough that mechanical migration cost dominates — measurable, and currently false by inspection of the migrated corpus (the migration is per-declaration local).
* **Layout-based syntax for the new blocks** — declined; the precedence-bounded grammar's item-position discrimination and no-unbounded-lookahead criteria are the existing contract, and braced blocks meet them.
  A representation decline with no reopening delta filed: the grammar gate is machinery.
* **Item-level `data` members in `sign` blocks** — declined with the separation argument above; the description route declines a stale member with the nested block's respelling.
  The reversal condition would be a demonstrated need to add generators to an existing family, which is a new family rather than an extension, so the delta would have to be a use case that is genuinely _extension of an inductive family_ rather than presentation inclusion.
  None is known.
* **The unseparated member list** — members are terminated by `;` (the surface's declaration terminator), with the retired `,` separator admissible so a stale declaration parses whole.
  An unseparated list is a clean parse of the WRONG tree: a member ends in a sort hole, the walk's `≐`-relation crosses the hole to whatever may follow, and at the hole's fill position the next member's lead outranks the hole's own content in the molder's local key — `Nil : Vec` would read as a member `Nil` whose signature is missing plus a nullary member `Vec`.
  The `sign` block's member list joined the terminated discipline under gandr-ng9.14 (owner directive: `;` load-bearing at the sign member level, the retired `,` NOT admissible there — it stays admissible only inside these generator/observation lists), after the graduation rung's `add(x, x)` collapse showed the keyword-led exemption did not hold ([[circuit-cells#The block form, ruled]]).
  The reopening condition for the comma-free, terminator-free spelling is a molder key change: hole-fill must outrank `≐`-continuation.

## `codata` declarations

```text
codata Stream(a : Type) : Type {
  head : a;
  tail : Stream(a);
}
codata Iter(a : Type) : Type { next : F(Option(#{item : a, rest : Iter(a)})); }
codata Fun(a : Type, b : Type) : Type { ap(x : a) : b; }     // reserved parameterized observation
```

* **A `codata` block takes the same head discipline as `data`** (the signature unification's ruling 2, retargeted): typed parameter binders bound once, plus the mandatory index-arity annotation; the block desugars to the nested block plus the ν polarity token, never to item-level members.
  The members are lowercase-led **observation declarations** `π : ResultType` — the dual of generators (named observation, result type).
  Observations are not generators, so no result-head discipline applies to them; a constructor-led member of a `codata` block is declined.
* `codata` reserves as one whole keyword (never `co` + `data`); `co { … }` stays the lazy-product expression.
* Reserved member forms: the **parameterized observation** `ap(x : a) : b` (functions-as-codata), the **grade-prefixed observation** `1 step : F(Unit)`, and the **`rule` 2-cell member** shared with `data`.
* Elaboration of codata values: a copattern body elaborates through the `Cosplit` case-tree node to a **record of thunks** over the record former — observation `s.π` becomes `force(s.π)`; the CBPV-faithful negative n-ary product is the reserved upgrade.
* The equality stance: (co)match equality is by unique label plus closure, **no η for codata** — undecidable, and recursive-record η breaks the elaborator's scope invariant.

## Eliminators, decided (2026-08-07)

**All pattern matching elaborates to eliminators, eliminator generation is a fold over `SignDesc` consuming the nested generator block, and the `match … by …` surface quantifies over any constant in motive–methods–target form — not only datatype eliminators.** The specification itself lives in the project's research vault (the corpus README's migration banner); what this repository binds is:

* **The user never sees an eliminator.** Display-provenance is a day-one elaborator constraint, not a polish item: a refinement the display layer cannot display is one the elaborator may not emit (the display-provenance design lives in the project's research vault — the corpus README's migration banner).
* **The four-tier elaboration policy is the standing answer to unification-steps-as-terms.** Don't generate (solve in the elaborator, record the solution); dissolve (the standard chains — singleton contraction, transport composition — discharged once at the doctrine level as fusion cells); decide (constructor-form targets collapse by the `walk` computation rule); generate the residue as derived cells that are replayable AND decomposable — specialisation chains compose canonically, and a left inverse is a decomposition with certificates, never a term computed.
  Fiat reduction rules in the kernel are declined; the reopening delta is measured certificate-replay cost dominating the dependent-core elaboration wall.
* **General recursion is carried by memo structures, not accessibility predicates**, typed against the polarity discipline: the definition is a value-side thunk, the memo structure is codata, and the guardedness/delay licence reads off the ν-sort in the sorting discipline.
  There is no termination checker: termination evidence is ordinary data with a certificate, checked by replay, and the effect quarantine's fixed-point and step budgets are the same refusal.
  Reopening delta: a demonstrated class of definitions whose termination evidence the memo discipline cannot express as ordinary data.
* **The derived eliminators are the payoffs the surface exists for**: univalence as induction on equivalences, funext as induction on homotopies, restricted-motive quotient eliminators.
* **The acceptance instance is the worked pasting derivation** — two identifications pasted, the four naive based-path-induction calls disposed through the tier ledger, the collapse demonstrated on constructor targets; its executable demonstration is owed to the dependent-core build lane, not to this record.

## `extern` blocks

```text
extern "c" from "m" {
  type Db;
  def cos(x: f64) -> f64;
}
```

* The ABI string (`"c"`, with `"c-unwind"`, `"stdcall"`, `"wasm"` as the slot's intended residents) and the library namespace are fixed; the namespace must be a lowercase identifier.
* Members are opaque type declarations (`type Db;`) and bodiless function signatures; member **attributes** (`@unwind`, `@variadic`, `@repr(c)`) are deferred — the block accepts un-attributed members only.
* A member call elaborates through the host-effect seam: `m.cos(x)` becomes `perform m.cos { p1 = x }`, with the reserved namespaces `fs`, `env`, `proc` riding the same member-call surface (`fs.read(p)`).
* Boundary type conventions: `CStr` ↔ `String`, opaque handles ↔ `u64`, `Void` ↔ `Unit`; the sized scalar atoms (`u32` … `f64`) are primitive types.
* The safety model behind the surface — the foreign call as an effect operation, loading as a capability, the library lifetime as a linear resource, abort-on-unwind, the full boundary mapping, and the two execution paths — is [[../implementation/foreign-interface]].
  **The native handler does not exist in this tree**: the block declares and type-checks a boundary that nothing crosses yet.

## `import` declarations

```text
import "file:///lib/parse.gandr" as parse ;
```

* The URI is a plain string literal — the `file` scheme today; other schemes extend later with zero grammar change.
* Resolution is unwired at the current rung (parse and surface witnesses only); typed package imports are a named deferral.

## `module` declarations

```text
module Parser {
  def run(s: String) -> F Result { … }
}
```

* As built (the M1-lite rung): checked-PBG module declarations with **optional record ascription** (`module M : #{ field: Type, … } { … }`), ordered member fields, and attribute-aware projection.
* Module bodies lower to source-ordered, exactly-once bind chains returning **canonical records**; modules are **value-only** (member ascriptions do not constrain eager effect rows); nested paths route through record projection.
* The full module family — structures, signatures, functors `fun (x:σ) ⇒ m`, ascription `:>` (sealing, generative), `pack`/`unpack`, `Package σ ≜ ∃β̄. U_r (F σ)` — is [[proposed/modules|the module system's]] staging above the built rung, not the current rung; modules as their own primitive layer is the implementation track's phase commitment, and neither older reading (compile-time namespaces, nor modules-elaborating-to-records as the final story) is current.
* A module **member** may carry a leading `@[…]` block in the grammar, but the lowering collects attributes for top-level items only, so a member's attributes parse and are then neither projected nor diagnosed; the module declaration itself takes no attribute block ([[proposed/modules#module-question-04]]).

## Operator-fixity declarations

```text
op infixl 6 "++" ;
```

* Reserved parse-and-decline: fixity classes (`infixl`, `infixr`, `infix`, `prefix`, `postfix`) are contextual tiles, not reserved keywords; the spelling is a string literal, so the labeler never tokenizes a user operator.
* The declaration mutates the **operator table as data** (fixity, associativity, precedence group, DAG edges) — never the grammar; the per-module wiring seam is deliberately unwired.
* Operators themselves are resolved deterministically and type-independently against the active table: `x * y` lowers to `((force mul) x) y` in CBPV form; sections `(_ op y)` are decided extension growth; **open alphabetic mixfix is structurally rejected** — an open mixfix form exposes two adjacent sort holes, which is an Operator Form violation and fails at grammar build.
* The architecture this declaration serves — the capture-and-resolution split, the named precedence graph, the cross-language catalog, the ambiguity policy, and the tier ownership question — is [[operators]].

## Elaboration behaviors, collected

The desugarings every form above relies on, each recorded with provenance so diagnostics un-sugar.

**The loop rows are designed, not built.** No fixpoint former exists in the kernel and the lowerer handles no loop node kind, so `for`, `while`, `loop`, `break`, and `continue` parse and then reach the total-mode hole; their targets are stated here because the surface owns their reading, and their design is [[../implementation/proposed/recursion-former#Loops elaborate through the former]].
Every other row is as-built.

| surface                     | elaborates to                                                                                                                                                                                                                                                                                     |
| --------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `def f(x: A) -> B { t }`    | `def f = thunk (fix-free) fn(x: A) { t }` (function sugar; see the def family)                                                                                                                                                                                                                    |
| `f(v, w)`                   | `f(v)(w)` — n-ary call sugar over one-argument CBPV calls, recorded                                                                                                                                                                                                                               |
| `f(v)` with `f : U_r B`     | `(force f)(v)` — call-position force sugar                                                                                                                                                                                                                                                        |
| `if c { t } else { u }`     | `case c { True => t, False => u }` — `if` is case on `1 + 1`, which is why `else` is mandatory                                                                                                                                                                                                    |
| `t;`                        | `run _ <- t;` — sequencing                                                                                                                                                                                                                                                                        |
| `case v { .. }`             | the case-tree compiler over the existing eliminators (sum case, tuple split, record projection) — no new eliminator node                                                                                                                                                                          |
| `.π => t` copattern clauses | `Cosplit { πᵢ ↦ tᵢ }` → record of thunks. **No self-reference is bound**: the lowering emits the `Cosplit` alone, because the kernel has no fixpoint former to bind one ([[../implementation/proposed/recursion-former]]) — a corecursive clause body therefore scope-checks and does not resolve |
| `for x in e { body }`       | a native fold over `e` — bounded, off the `fix` path                                                                                                                                                                                                                                              |
| `while c { body }`          | `fix self. force c >>= b. case b { false => ret unit \| true => body >>= _. force self }`                                                                                                                                                                                                         |
| `loop { body }`             | `while true { body }`                                                                                                                                                                                                                                                                             |
| `break` / `continue`        | `perform Break unit` / `perform Continue unit`, under two op-name-keyed deep handlers (outer catches `Break`, inner per-iteration catches `Continue`) — never `reset`/`shift`, which stay reserved for user delimited control                                                                     |
| `m.op(a)`                   | `perform m.op { p1 = a }` — host-effect member calls                                                                                                                                                                                                                                              |
| `module M { .. }`           | a canonical record over a source-ordered bind chain                                                                                                                                                                                                                                               |

Discarded continuations are sound here because the linear zone is vacuous at the current stage; when sessions and stack-owned obligations land, `break`/`continue` inherit the discard-runs-unwind discipline for free, since they already route through the handler mechanism that owns it.

## Source and confidence

The declaration forms are as-built where marked (grammar and lowering crates; the module train; the attribute diagnostics) and designed-reserved where marked (the parse-and-decline inventory).
The manual's values-data, codata, and modules chapters restate the forms at reference depth; where the manual predates a landed divergence (the `val`/`run` binder pair, the folded `def rec`, the narrowed grade tiles), the grammar crate's adaptation registry is authoritative.
