# Surface language

This track owns the surface language: what gandr programs look like.
It covers the surface grammar's design (precedence-bounded grammars, the molder/melder pipeline, obligations), the declaration forms and their reserved slots, the shell fragment, and the vocabulary decisions — drawn from, and cross-linked to, the [[implementation|implementation track]]'s as-built account.

The main document mentions and sketches **every** language feature with references, so a reader sees the whole language without descending; the sub-documents carry the exhaustive machinery and the full set of sketched examples, so nothing is lost by the split.
If the exhaustiveness claim is ever wrong, that is a defect in this document, not a reason to move content down.

* [[surface-language/grammar]] — the grammar machinery: the PBG, precedence DAGs, the molder/melder pipeline, the obligation taxonomy, the build-time gates, the adaptations registry, and the complete parse-and-decline inventory.
* [[surface-language/declarations]] — the declaration forms in full: the `def` family, `val`/`run`, attributes, `data`/`codata`, `extern`, `import`, `module`, operator declarations, and the elaboration behaviors.
* [[surface-language/attributes]] — the entity-attribute layer in full: the `@[…]` marker and its two positions, the sigil decision and its reversal target, the typed-schema registry and the diagnostics, attribute purity as locality, the hash-neutral side table with its inert and semantic tiers, the report projection, and the consumers the one layer serves.
* [[surface-language/directed-family]] — the directed identity family's surface: `Flow(A, x, y)` spelled `A ~~> B` at the type level, the diagonal intro `diag`, the shared `walk` under the motive-covariance side condition, the shared `then`, and `ua-dir` with what it buys a program.
* [[surface-language/circuit-cells]] — the design sketch for full circuit-algebra cells: reconvergence, disconnection, and wheels at the surface, port-named interfaces with polarity, the wheel guard, holes as contexts, and the cost of each feature.
* [[surface-language/higher-cells]] — the dimension-named `data`-block members in full: the `sort`/`cons`/`oper`/`rule`/`meta` ladder with the reserved `cell` tower, mandatory 2- and 3-cell names, the boundary language and its sphere typing, and the derived `Model(S)` signature-former with its flagship shapes.
* [[surface-language/recursion]] — the (co)recursion surface: `def rec`/`rec { … }`, the instantiation slot with its direction sigils, the productivity ladder, loops, and the as-built scope rung.
* [[surface-language/value-semantics]] — the update surface in full: functional record update, the list update operations, update by construction, the state-visibility red line, in-place execution as a runtime licence, and the four foreclosure rules that keep the mode calculus open.
* [[surface-language/proposed/modules]] — **proposed, only its bottom rung built**: the module system — the core-and-modules unification and its call-by-push-value sorting, the predicativity fence, the module grammar and typing rules, sealing and first-class packages, implicit resolution, located modules and distribution, futures, and the six-rung staging ladder.
* [[surface-language/proposed/modes-and-references]] — **proposed, nothing built**: the access-mode, reference, and region calculus — the sixteen-decision register, the per-problem comparison against the languages that have made those decisions, the foreign-interface consequences, and the literature the answers would come from.
* [[surface-language/shell]] — the shell fragment: shell blocks and jobs, the embedded sub-grammar, the host escape, string interpolation, and the REPL split.
* [[surface-language/roadmap]] — graduation rungs per reserved form, the pending lanes, and the deferred-with-reasons inventory.

## The design stance

Five criteria, all binding, shape every form below:

1. **Small compiled grammar** — parser tables are deployment footprint; size budgets are gates, not hopes.
2. **Never unbounded lookahead** — a latency criterion: every construct is identifiable by its first token in context; zero declared conflicts; no dynamic precedence; no external scanner.
   Lookahead destroys felt latency three ways: parse forking on the edit hot path, distant parse decisions flipping under small edits (ballooning re-typing), and poor error recovery evaporating the derivation the user sees over whole regions.
3. **Unambiguous over terse** — mandatory braces, mandatory `else`, required statement terminators; verbosity buys unrepresentable ambiguity classes.
4. **Coverage** — the surface must express the whole core calculus and every form the machines exercise, and must not paint over the proposal-stage forms.
5. **Recovery and edit locality** — `;` and `}` are unique-in-context synchronization points; flat statement lists keep unchanged siblings outside any changed subtree.

The consequences recur below: **first-token discrimination everywhere; parenthesized calls; no juxtaposition application; no angle-bracket generics; no layout significance; `;`-terminated flat blocks**.

The arm-stealing that untyped ML-family surfaces leave to defensive parentheses is **unrepresentable** here: every multi-arm or two-branch construct is brace-delimited, and `else` is mandatory (`if` is sugar for `case` on `1 + 1`, which needs both branches anyway).

## A first look

```text
#!/usr/bin/env gandr
// A first look at gandr surface syntax: keyword-led definitions, brace-
// delimited bodies, monadic bind written as a statement, and brace-delimited
// case arms that cannot leak into an enclosing case.

def square(x: Integer) -> F Integer {
  ret (x * x)
}

def pipeline(a: A) -> F Integer {
  run x <- f(a);
  run y <- g(x);
  ret (x + y)
}

def classify(v: (A + B) + Integer) {
  case v {
    Inl(x) => inner(x),
    Inr(y) => case y {          // arms cannot leak: braces delimit
      Inl(z) => ret z,
      Inr(_) => ret 0
    }
  }
}
```

gandr is a call-by-push-value language, and that one fact shapes everything on the page: **values _are_, and computations _do_** — the two never mix silently. (The example predates one vocabulary decision: the binder keyword is now `run`, and the function sugar's result type is written in the signature — see below.)

## The lexical layer

* **Identifiers**: `[a-z_][A-Za-z0-9_]*` for terms; `[A-Z][A-Za-z0-9_]*` for constructors and type constructors; lowercase for type variables.
* **Literals**: unit `()`; bare numerics `7`, `1.5` (gradual literal type); suffixed numerics `8080u32`, `2.5f64` (concrete sized atoms); strings `"…"` with backslash escapes and `${ E }` interpolation; characters `'c'`; booleans `true`/`false`.
* **Comments**: `//` line; nestable `/* … */` block; comment tokens lex below shell words, so inside a shell block `//path` stays a path.
* **Whitespace**: indentation is never significant; `;` terminates statements; newline inside shell blocks is a list operator.
* **Keywords**: one table, ~50 live plus six reserved-for-proposals — the full table with the reservation policy is in [[grammar#The keyword and operator tables]].

## Items: what a file is

A source file is a flat list of items, each first-token-discriminated:

* `def` — the definition family (signature, value, function, `def rec`) — [[declarations#The def family]];
* `rec { … }` — a mutual-recursion block (reserved) — [[recursion#Mutual recursion]];
* `data` / `codata` — datatype declarations with their member forms — [[declarations#data declarations]] / [[declarations#codata declarations]];
* `extern "abi" from "lib" { … }` — foreign interface blocks — [[declarations#extern blocks]];
* `import "URI" as name ;` — module imports — [[declarations#import declarations]];
* `module Name (: #{ … })? { … }` — module declarations — [[declarations#module declarations]];
* `op <fixity> <level> "spelling" ;` — operator-fixity declarations (reserved) — [[declarations#Operator-fixity declarations]];
* `@[ … ]` attribute blocks — leading `def` items — [[declarations#Attributes]];
* an expression statement `e ;` — script/REPL mode.

## Statements: what a block is

A block `{ s₁; …; t }` is a flat sibling list of statements ending in an expression — **the block is the bind-chain spine**:

* `run p <- c;` — computation-result bind (the honest rendering of `c >>= p. …`);
* `run p : F T <- c;` — the answer-type annotation lane (pending);
* `val p = v;` — value split (irrefutable patterns only);
* `t;` — sequencing, sugar for `run _ <- t;`;
* session statements: `send(c, v);`, `recv(c) as x;`, `close(c);`, `select(c, l);`;
* sharing/world statements: `fork(c : S) { t } as d;`, `acquire(a) as c;`, `release(c) as a;`, `fork!(a : S) { t };`, `leta x = v;`.

The older spelling `let` is **retired**: `val`/`run` are the binder pair; `let` stays unassigned for a future transparent definitional binder ([[declarations#Statement binders: val and run]]).

## Expressions: every form

* **Variables, constructors, literals** — as in the lexical layer.
* **Typed holes** — `?` and `?name`: first-class goals; a file with holes still lowers and reports (holes are deliberately typeable, so "has holes" is not "incomplete").
* **Lambda** — `fn(x: A) { t }`, `fn(x) { t }`; **type abstraction** — `fn[X] { t }`; **type instantiation** — `t[A]`.
  No juxtaposition application anywhere: calls are `t(v)`, with `t(v, w)` recorded sugar for `t(v)(w)` — see [[recursion#Application syntax, rejected]] for the rejection's full reasoning.
* **Call-position force sugar** — `g(x)` with `g : U_r B` elaborates to `(force g)(x)`:

```text
fn(f: U (B -> F C)) {
  fn(g: U (A -> F B)) {
    fn(x: A) {
      run y <- g(x);     // call-position force sugar; kernel: (force g)(x)
      f(y)
    }
  }
}
```

* **`ret v`** — the returner; binds loosest and right-associative.
* **`if c { t } else { u }`** — mandatory `else`; `else if` chains allowed.
* **`case v { K(x) => t, … }`** — sum elimination; arms comma-separated and brace-delimited; **empty case** `case x {}` over an uninhabited type; **with-view** `case xs with f(xs) { … }` (reserved).
* **Ascription** — `(e : T)`, parenthesized only.
* **Thunks** — `thunk { t }`, graded `thunk[r] { t }` (grade defaults to `ω`); **`force v`**.
* **Lazy products** — `co { fst = t, snd = u }` with projection `t.fst`.
* **Records** — `#{ℓ = v, …}` literals, `#{ℓ : T, …}` types, `r.ℓ` projection, `#{r | ℓ = v, …}` functional update ([[value-semantics#Functional record update]]).
* **Tuples and lists** — `(v, w)`; `[a, b, …]` (check-only: a bare inferred list needs an expected type).
* **Sharing primitives** — `dup(v)`, `drop(v)` as ordinary computations.
* **Worlds** — `hold v` (package at the current world), `leta x = v;` (modal elimination), `migrate[w] { t }`.
* **Sessions** — `send(c, v);`, `recv(c) as x;`, `close(c);`, `select(c, l);`, `offer(c) { l1 => t1, l2 => t2 }`, `fork(c : S) { t } as d;` (the binder gets `dual(S)`), delegation by `send(c, d);`, `acquire(a) as c;`, `release(c) as a;`, `fork!(a : S) { t };`:

```text
def answer_service() -> F Integer {
  fork(c : !Integer. end) {
    send(c, 41);
    close(c);
    ret ()
  } as d;
  recv(d) as x;
  close(d);
  ret x
}
```

* **Loops** — `for x in e { … }`, `while c { … }`, `loop { … }`, `break`, `continue` — sugar over a native fold or `fix`, with `break`/`continue` as effect operations ([[recursion#Loops, and the break/continue discipline]]).
* **Operators** — the fixed infix table `|| && == != < <= > >= ++ + - *` and unary `-`; resolution is deterministic and type-independent against the active operator table; user-declared operators ride the reserved declaration form ([[declarations#Operator-fixity declarations]]).
* **The instantiation slot** — `e[ι₁, …, ιₙ]` with residents `T` (type argument), `<`/`>` (the recursion direction sigils), `m<` (named measure), `x = e` (explicit instantiation), `size = e`, `cost = e`, `tail` — the full design is [[recursion#The instantiation slot]].
* **Extern and host calls** — `m.op(a)` member-call form, elaborating to `perform m.op { p1 = a }`; the reserved namespaces `fs`, `env`, `proc` ride the same surface ([[declarations#extern blocks]]).
* **Shell blocks** — `#!{ … }` and everything in [[surface-language/shell]].

## Patterns: every form

```text
x                      variable (binder)
_                      wildcard
C(p₁, …, pₙ)           constructor (nested)
C                      nullary constructor
(p, q)                 tuple
#{ℓ = p, …}            record
[p₁, …, ..rest]        list with cons-spread
42 / "s" / true        literal (refutable)
p | q                  or-pattern (same variable set both branches, or none)
p as x                 as-pattern
```

* `let`/`val`-position patterns must be **irrefutable** (variable, wildcard, tuple, record, single-constructor); a refutable pattern requires `case`.
* Patterns compile through the case-tree machinery to the existing eliminators — exhaustiveness yields a missing-pattern witness diagnostic, and a redundant arm is a warning naming the shadowing arm.

## Types: every former

Value types:

| former        | spelling                                                                                                                              | notes                                                                     |
| ------------- | ------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------- |
| unit          | `Unit`                                                                                                                                |                                                                           |
| eager product | `A * B`                                                                                                                               | right-associative                                                         |
| sum           | `A + B`                                                                                                                               | right-associative                                                         |
| thunk         | `U[r] B`                                                                                                                              | `U B` ≡ `U[ω] B`                                                          |
| union         | `A \| B`                                                                                                                              | pairwise-incomparable with `/\` and `&` — parentheses required when mixed |
| world         | `A at w`                                                                                                                              |                                                                           |
| record        | `#{ℓ : A, …}`                                                                                                                         |                                                                           |
| application   | `Name(args)`                                                                                                                          | e.g. `List(a)`, `Vec(a)` — **never** angle brackets                       |
| primitives    | `Any`, `Boolean`, `Char`, `Integer`, `Never`, `String`, `Symbol`, `Unit`, `Unknown`, `Void`, `u32`, `u64`, `i32`, `i64`, `f32`, `f64` | sized atoms and the gradual `Unknown`                                     |

Computation types:

| former       | spelling      | notes                                             |
| ------------ | ------------- | ------------------------------------------------- |
| returner     | `F A`         |                                                   |
| arrow        | `A -> B`      | right-associative                                 |
| lazy product | `B & B'`      | right-associative, incomparable with `|` and `/\` |
| intersection | `B /\ B'`     |                                                   |
| polymorphism | `forall a. B` |                                                   |
| stack        | `Stk(B, C)`   | the reified-continuation value type               |

Session types (prefix-discriminated, dot-sequenced): `end`, `!A.S` (send), `?A.S` (receive), `+{l: S, …}` (internal choice), `&{l: S, …}` (external choice), `mu X. S` (recursion); grades are `0`, `1`, `ω` (and numerals).

Deliberate exclusions: **no angle-bracket generics** (`<`/`>` are comparison operators); **no free-floating coercions**; **no return-position inference markers**; annotations on arbitrary expressions require parentheses; `Type : Type` is refused (the universe keyword is `Type`, predicative with reflexive-only identity — see the vocabulary decisions below).

## Declarations: every form

Sketched here, specified in [[surface-language/declarations]]:

```text
def name : T ;
def name = v ;
def name(x: A, y: B) -> B' { t }
def rec fact(n: Integer) -> F Integer { … }

data Maybe(a) { None, Some(x: a) }
data Nat { Zero, Succ(n: Nat), op add(m: Nat, n: Nat) -> Nat, rule add(Zero, n) ==> n }

codata Stream(a) { head: a, tail: Stream(a) }

extern "c" from "m" { type Db; def cos(x: f64) -> f64; }

import "file:///lib/parse.gandr" as parse ;

module Parser { def run(s: String) -> F Result { … } }

op infixl 6 "++" ;

@[doc("…")]
def square(x: Integer) -> F Integer { ret (x * x) }
```

The reserved members and slots (all parse-and-decline, exhaustively inventoried in [[grammar#The parse-and-decline semantics]]): operation members with multi-out results, `rule` rewrite members, grade-prefixed fields and observations, generalized constructor results, per-symbol attribute slots, parameterized observations, with-view matches, `rec` blocks, copattern default arms, and the instantiation slot's reserved residents.

## The (co)recursion surface

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

rec {
  def even(n: Nat) -> Bool { case n { Zero => true,  Succ(p) => odd[<](p)  } }
  def odd (n: Nat) -> Bool { case n { Zero => false, Succ(p) => even[<](p) } }
}
```

The discipline in one paragraph: a **recursive scope** opened by `def rec` or `rec { … }`, and a **direction sigil** in the instantiation slot required at every recursive occurrence — the marked occurrence is the only reference to the fix-bound variable, an unmarked one is a hard error carrying the marked spelling as its suggestion, and outer bindings stay reachable by qualified path.
The markers grow along the **productivity ladder** (scope evidence now; checked guardedness next; erased size applications at the sized rung), and corecursion is the same family — copattern-clause bodies, no `def corec`.
The full design, the as-built scope pass, and the edge cases are [[surface-language/recursion]].

## Value semantics — how a program says "a changed value"

```text
def r  = #{ x = 1, y = 2 };
def r2 = #{ r | x = 9 };        // r STILL denotes #{x = 1, y = 2}
def xs2 = list.set(xs, 1, 99);  // xs is untouched; xs2 is a new list
```

Every update produces a **new** value, and no binding a program already holds can observe a later change to it — the **state-visibility red line**, which holds structurally because the surface has no lvalue, no `:=`, no reference, and no aliasable cell.
The whole mutation surface is three constructs: functional record update `#{ r | ℓ = v, … }`, the `list` module's update operations, and update-by-construction for declared data (match, then rebuild).

Whether the runtime physically copies is a **separate question, answered below the surface**: where a base is provably unique at its update site, in-place mutation is an unobservable optimization.
Access modes, references, and regions are deliberately absent, and four foreclosure rules keep them addable rather than retrofittable.
The full treatment is [[surface-language/value-semantics]]; the calculus that would add them is [[surface-language/proposed/modes-and-references]].

## The shell fragment

```text
def build = thunk {
  #!{
    mkdir -p out;
    echo "building for $USER" > out/log;
    [ cd out; make all ];
    $(notify("build finished"))
  }
}
```

Shell blocks `#!{ … }` (dialect-tagged `#!zsh{ … }`), jobs as thunked values, pipelines, quoting, `$name`/`${name}` expansions, `[ … ]` subshells, fd redirections (`2>`, `2>&1`), `$!{ … }` command substitution, the `$( E )` host escape, `${ E }` string interpolation, and the REPL split (bare `gandr` = shell-REPL; `^` forces a command) — all in [[surface-language/shell]].

## Identity and univalence forms

The identity fragment's surface (rung 1, built):

* `Path(A, x, y)` — the identity type family, with numeric endpoints allowed (`Path(Integer, 2 + 2, 4)` is the honesty example);
* `here(v)` — the introduction form (`refl` is a constructor);
* `walk(p, fn(a, b, q) => C, fn(x) => c)` — the full dinatural eliminator with an explicit motive;
* the **K-rejection witness is a live diagnostic**: a `case` on an identity type is rejected — the reserved here-pattern fragment requires the without-K unification fragment (rung 2), whose solver declines the deletion step itself, so the pathological example reads `//@ expect-diagnostic: without-k`.

The directed family stages beside the groupoid one (designed; the identity-layer phase owns the landing):

* `Flow(A, x, y)` — the directed identity family, spelled `A ~~> B` at the type level: `x` transports into `y`, in one direction — a deprecation, a backend migration, a refinement, never an iso;
* `diag(v)` — the diagonal intro: every point flows to itself;
* `walk` — the **same eliminator**, under the motive-covariance side condition: a motive placing the moving endpoint in the contravariant slot is refused, and the refused motive shape is exactly the symmetry shape — so inversion is underivable by construction, not merely unimplemented;
* `then` — composition, the **same spelling**: directed composition is covariant transport, and directedness costs nothing at dimension 1;
* the **symmetry-derivation witness is the permanent guard**: it must fail elaboration, the directed twin of the K-rejection witness above.

The full treatment — the two families side by side, the worked motives (accepted and refused), `ua-dir` and what it buys a program, and the open items — is [[surface-language/directed-family]].

The ratified spellings of record — `then` (composition, shared by both families), `back` (inversion, groupoid only), `diag` (the diagonal intro), `Step`, `A <~> B` (the isomorphism spelling), `A ~~> B` (the directed former's spelling) — are decided surface vocabulary **not yet landed**; the directed family stages beside the groupoid one with **no kernel coercion between them** (the comparison is a theorem, the core-coincidence obligation of the metatheory track).
The eliminator `walk` and the composition `then` are shared across both families — the settled answer to the metatheory roadmap's open question 9 (owner decision, 2026-07-31).
The former names are the current ratified spellings (a rename from the earlier `Id`/`refl`/`J` family; the rename itself is recorded as a ratified surface decision).

## The vocabulary decisions of record

| decision                 | ruling                                                                                                       | where argued                                      |
| ------------------------ | ------------------------------------------------------------------------------------------------------------ | ------------------------------------------------- |
| the universe keyword     | `Type`, **never** `Set`                                                                                      | the universe design (predicative, reflexive-only) |
| binder keywords          | `val p = v;` / `run p <- c;` (`let` retired, unassigned)                                                     | [[declarations]]                                  |
| type operators           | right-associative; `|`/`/\`/`&` pairwise-incomparable                                                        | [[grammar#The precedence bands as built]]         |
| term operators           | fixed left-associative table; no parser-dependent user fixity, ever                                          | [[declarations#Operator-fixity declarations]]     |
| application              | mandatory parentheses; n-ary as recorded sugar                                                               | [[recursion#Application syntax, rejected]]        |
| generics                 | `Name(args)`, never `Name<args>`                                                                             | this document (types)                             |
| imports                  | `import "URI" as name ;` — plain string, `file` scheme now, others zero-grammar-change later                 | [[declarations]]                                  |
| the semidecision type    | **Sier** (Sierpiński), never spelled Σ, never encoded as boolean                                             | [[../metatheory/exact-reals]]                     |
| numerics                 | `Integer` renames to `Int` with `Nat` added when arbitrary precision lands                                   | the implementation roadmap                        |
| keyword policy           | a small closed set of globally reserved keywords; fixity classes contextual; corpus-swept before reservation | [[grammar]]                                       |
| braces/else/semicolons   | mandatory                                                                                                    | this document (the design stance)                 |
| layout                   | never significant; no ASI, no external scanner                                                               | this document (the design stance)                 |
| `rec` marker             | explicit, never implicit self-reference detection                                                            | [[recursion]]                                     |
| shell subshell           | `[ … ]`, keeping `( … )` free for the `$( E )` host escape                                                   | [[surface-language/shell]]                        |
| `${name}` vs `${E}`      | distinct labeler modes: shell parameter expansion is not string interpolation                                | [[surface-language/shell]]                        |
| `meta` coherence members | **do not exist yet** — named so the absence is visible                                                       | [[declarations#data declarations]]                |

## The manual as a reference

The pre-reboot manual is the largest coherent statement of the language ever written, and this track is written against it chapter by chapter.
What remains true: the whole grammar reference (items, statements, patterns, expressions, types, session types, the shell sub-grammar), the precedence tables, the keyword inventory, the CBPV mapping tables, and the worked examples — the manual's surface-syntax chapter is the fullest single rendering, and its four companion programs are carried in this track's examples above.
What has drifted, marked at the claim:

* the manual's `let` binders are now `val`/`run` (vocabulary decision, landed after the manual's cut);
* its "normative grammar" framing predates the PBG: the normative parser is now the Rust PBG + melder, with tree-sitter demoted to parity tooling (the manual's own as-built amendment says this; the grammar reference it renders is the parity grammar — the PBG-only forms in this track are absent there);
* the module, recursion-marker (`[<]`/`[>]`), and fold-in forms (`data`/`codata`/`def rec`/loops) landed after its chapters, so it carries them only as designed-not-built where it carries them at all;
* its kernel/identity chapters predate the current kernel and the `Path` rename; this track states the current forms.

## Source and confidence

The track is written against: the as-built grammar and engine crates (high confidence — every as-built claim names its crate and module); the surface-syntax, operators, data-patterns, codata-corecursion, attributes, ffi, modules, shell-usage, effects, and parser-interaction proposals (the design records); the syntax-inventory fold-in and the overnight fold-in records (the divergence registry); the recursion-surface `.gfd` component (fidelity-reviewed) and its as-built scope pass; and the pre-reboot manual, marked for drift above.
Where the sources disagree, the as-built tree wins on status and the design record wins on payload — stated at the claim.
