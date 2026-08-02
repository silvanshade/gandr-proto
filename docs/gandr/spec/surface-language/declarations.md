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

## `data` declarations

```text
data Maybe(a) { None, Some(x: a) }
data Tree(a) { Leaf, Node(l: Tree(a), x: a, r: Tree(a)) }
data Void {}
data Nat {
  Zero,
  Succ(n: Nat),
  op add(m: Nat, n: Nat) -> Nat,        // reserved
  rule add(Zero, n)    ~> n,            // reserved
  rule add(Succ(m), n) ~> Succ(add(m, n)),
}
```

* Parameters are type variables in parentheses; recursion needs nothing new — `Tree(a)` inside a field is an ordinary type application.
* Members are first-token-discriminated by case: uppercase-led **constructors** (nullary or field-tuple) versus lowercase-led reserved members.
* Declared data is **generative-nominal**: the declaration mints a fresh nominal identity, so two `data Boolean` declarations in different modules are different types; type-level self-reference needs no marker (see [[recursion#Type-level self-reference]]).
* The reserved members, all parse-and-decline:
  + `op name(params) -> R` — an **operation** member, with a multi-output tuple result form `-> (o1: A, o2: B)` kept local to it;
  + `rule lhs ~> rhs` — a **directed rewrite** member, the user 2-cell: it elaborates (when its rung lands) to an oriented command cell on the rule's cut seam, never to an unverified equation — the anti-pragma discipline;
  + a **grade prefix** on a field — `Some(1 x: a)` — grade tile restricted to `{ number, ω }`;
  + a **generalized constructor result** — `Cons(x: a, xs: Vec(a)) : Vec(a)`;
  + a **per-symbol attribute slot** — `[ctor, assoc]`.
* A `meta name: ρ ~>> ρ′` coherence member (a 3-cell, higher-rule form) **does not exist yet** — named here so its absence is visible, per the keyword-ladder lineage (`sort`/`cons`/`oper`/`rule`/`meta`).
  The whole ladder — the respell, mandatory 2- and 3-cell names, the `meta` boundary language, the reserved `cell` tower, and the `Model(S)` signature-former the members exist to feed — is [[higher-cells]]; the `rule` member above is its dimension-2 rung as landed, and its anonymity is precisely what the respell fixes.

## `codata` declarations

```text
codata Stream(a) {
  head: a,
  tail: Stream(a),
}
codata Iter(a) { next: F(Option(#{item: a, rest: Iter(a)})) }
codata Fun(a, b) { ap(x: a): b }     // reserved parameterized observation
```

* `codata` reserves as one whole keyword (never `co` + `data`); `co { … }` stays the lazy-product expression.
* Members are lowercase-led **observation declarations** `π : ResultType` — the dual of field-tuple constructors (named observation, result type).
* Reserved member forms: the **parameterized observation** `ap(x: a): b` (functions-as-codata), the **grade-prefixed observation** `1 next: F(Unit)`, and the **`rule` 2-cell member** shared with `data`.
* Elaboration of codata values: a copattern body elaborates through the `Cosplit` case-tree node to a **record of thunks** over the record former — observation `s.π` becomes `force(s.π)`; the CBPV-faithful negative n-ary product is the reserved upgrade.
* The equality stance: (co)match equality is by unique label plus closure, **no η for codata** — undecidable, and recursive-record η breaks the elaborator's scope invariant.

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
* Operators themselves are resolved deterministically and type-independently against the active table: `x * y` lowers to `((force mul) x) y` in CBPV form; sections `(_ op y)` are decided extension growth; **open alphabetic mixfix is structurally rejected** (the parser could not find the region's boundaries without consulting declarations — an Operator Form violation).

## Elaboration behaviors, collected

The desugarings every form above relies on, each recorded with provenance so diagnostics un-sugar:

| surface                     | elaborates to                                                                                                                                                                                                                 |
| --------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `def f(x: A) -> B { t }`    | `def f = thunk (fix-free) fn(x: A) { t }` (function sugar; see the def family)                                                                                                                                                |
| `f(v, w)`                   | `f(v)(w)` — n-ary call sugar over one-argument CBPV calls, recorded                                                                                                                                                           |
| `f(v)` with `f : U_r B`     | `(force f)(v)` — call-position force sugar                                                                                                                                                                                    |
| `if c { t } else { u }`     | `case c { True => t, False => u }` — `if` is case on `1 + 1`, which is why `else` is mandatory                                                                                                                                |
| `t;`                        | `run _ <- t;` — sequencing                                                                                                                                                                                                    |
| `case v { .. }`             | the case-tree compiler over the existing eliminators (sum case, tuple split, record projection) — no new eliminator node                                                                                                      |
| `.π => t` copattern clauses | `fix self. Cosplit { πᵢ ↦ tᵢ[self] }` → record of thunks                                                                                                                                                                      |
| `for x in e { body }`       | a native fold over `e` — bounded, off the `fix` path                                                                                                                                                                          |
| `while c { body }`          | `fix self. force c >>= b. case b { false => ret unit | true => body >>= _. force self }`                                                                                                                                      |
| `loop { body }`             | `while true { body }`                                                                                                                                                                                                         |
| `break` / `continue`        | `perform Break unit` / `perform Continue unit`, under two op-name-keyed deep handlers (outer catches `Break`, inner per-iteration catches `Continue`) — never `reset`/`shift`, which stay reserved for user delimited control |
| `m.op(a)`                   | `perform m.op { p1 = a }` — host-effect member calls                                                                                                                                                                          |
| `module M { .. }`           | a canonical record over a source-ordered bind chain                                                                                                                                                                           |

Discarded continuations are sound here because the linear zone is vacuous at the current stage; when sessions and stack-owned obligations land, `break`/`continue` inherit the discard-runs-unwind discipline for free, since they already route through the handler mechanism that owns it.

## Source and confidence

The declaration forms are as-built where marked (grammar and lowering crates; the module train; the attribute diagnostics) and designed-reserved where marked (the parse-and-decline inventory).
The manual's values-data, codata, and modules chapters restate the forms at reference depth; where the manual predates a landed divergence (the `val`/`run` binder pair, the folded `def rec`, the narrowed grade tiles), the grammar crate's adaptation registry is authoritative.
