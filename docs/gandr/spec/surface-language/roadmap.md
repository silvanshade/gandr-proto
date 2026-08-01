# Surface-language roadmap

What remains on the surface, beyond the current rungs of [[../surface-language|the surface-language track]].
Each reserved form's graduation is gated on the semantics that owns it — **no rung's syntax ships before its check is designed**; this file names the gate per form so no later pass rediscovers it.

## Graduation rungs of the reserved forms

| form                                                                         | current                                  | graduates when                                                                                                                                                                            |
| ---------------------------------------------------------------------------- | ---------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `op` operation members (incl. multi-out)                                     | parse-and-decline                        | the cell layer consumes operation frames (the engine's fusion rung); the multi-out result needs the multi-output term face                                                                |
| `rule lhs ~> rhs` members                                                    | parse-and-decline                        | the cell store accepts surface-declared cells (the L2/completion rung); rule **conditions** will grow the form — the condition-syntax seam is a binding forward constraint on the grammar |
| grade-prefixed fields / observations                                         | parse-and-decline                        | the grade discipline enforces per-field usage                                                                                                                                             |
| generalized constructor results                                              | parse-and-decline                        | the indexed/family rung of declared data                                                                                                                                                  |
| per-symbol attribute slots                                                   | parse-and-decline                        | the attribute semantic tier (compile-time metadata consumers)                                                                                                                             |
| parameterized observations                                                   | parse-and-decline                        | functions-as-codata (the L-era codata upgrade)                                                                                                                                            |
| with-view match                                                              | parse-and-decline                        | the view elaboration (a desugaring pass, no core change)                                                                                                                                  |
| `rec { … }` blocks                                                           | parse-and-decline                        | recursive lowering with the bundle encoding ([[recursion#Mutual recursion]])                                                                                                              |
| operator-fixity declarations                                                 | parse-and-decline, `Pbg::extend` unwired | user operators: the table-as-data mutation is designed; the wiring needs the module provenance story                                                                                      |
| copattern default arm `_ =>`                                                 | parse-and-decline                        | the copattern elaboration's coverage discipline                                                                                                                                           |
| instantiation-slot residents (`m<`, `x = e`, `size = e`, `cost = e`, `tail`) | parsed; declined by name                 | the guardedness and sized rungs of [[recursion]], the cost-as-effect lane, the tail-call backend contract, and implicit-argument elaboration respectively                                 |

## The productivity ladder's next rungs

* **Guardedness rung** — the sigils become checked claims (structural descent; at-least-one-observation), total on the fragment, with the per-rung refusal diagnostic contract; gated on recursive lowering landing.
* **Sized rung** — a deliberate design pass first (the bounded-quantification/infinite-size interaction is the standing hedge); sizes enter as indices in their own sort reusing the grade-zero erasure machinery, never as a fresh semiring grade; the well-founded fixed-point former is the single recursion-plus-corecursion former that retires the productivity/termination split; the guardedness check is a two-state flag automaton; the named deep-guardedness programs (higher-order `g f = cons 0 (f (g f))`, abstracted `repeat`, `zipWith`, knot-tying traversals) pin the syntactic-check/sizes cliff as corpus goldens.
* **Derived terminating recursors** — catamorphism/paramorphism eliminators derived per `data` declaration, the first checked-termination lever, bridging the unchecked-`fix` rung to the termination-obligation wave.

## Pending surface lanes

* **The answer-type annotation lane** `run p : F T <- c;` — pending.
* **The term-in-type splice** (rung 2 of the type grammar's value-endpoint growth) — decided with the parser owner.
* **Format specifiers** for string interpolation — deferred.
* **Mixed-word interpolation** for the shell host escape (`"pre $( E ) post"`) — deferred; the standalone-word cut is what landed.
* **Environment assignment** `FOO=bar cmd` — currently unmoldable; wanted working for the daily-driver rung.
* **Typed package imports** — deferred past the module rung; `import` resolution is unwired today.
* **The computation-sort gradual-top spelling** — a named deferral from the module train.
* **The full module family** (structures, signatures, functors, sealing, `pack`/`unpack`, `Package σ`) — the M1+ staging of the modules design, then modules as their own primitive layer (the implementation phase).
* **Effects/control keywords** (`handle`, `perform`, `reset`, `shift`, `resume`, `stk`) — reserved, wired when the effects lane surfaces them; `quote`/`splice` ride the metaprogramming lane.
* **`yield`/`await`** — keyword-only commitments gated on the handler-reification mechanism (generators/async as handler-to-codata reification).
* **Labeled `break 'l`** — needs per-label atoms or named handlers; the `'` sigil collides with the character lexer.
* **Sessions surface** — linear typestate codata with a duality involution, after the codata MVP and the duality engine; no session channel syntax (`(νxy)`, `T ⊥ U`) in any near rung.
* ~~**The `Path`/`Flow` surface spellings for the directed family**~~ — **settled 2026-07-31** (owner decision, closing the metatheory roadmap's open question 9): the eliminator is the shared `walk` (under the motive-covariance side condition), the diagonal intro is `diag`, and directed composition shares `then`.
  The landed vocabulary is [[directed-family]]; the identity-layer phase still owns the landing itself.
* **Display elision of inferable recursion markers** — a projection concern; unowned.
* **`let` (unassigned) and `mut` (unreserved)** — keyword decisions owned by the dependent-rung and value-semantics passes respectively.

## Deferred-with-reasons, collected

The fold-in record's deferred class, each with its structural reason — these are **not** silently dropped surface:

* **user operators / prefix / postfix / sections / open mixfix** — open alphabetic mixfix violates Operator Form structurally; sections and closed/bracketed notation are decided extensions; parser-level fixity reopens only under the named trigger (deterministic incremental capture, exact spans, editor recovery preserved);
* **refinements `{ν : A | φ}`, contracts, refined session payloads** — solver-gated sketch forms;
* **sized-type surface `C^i`, `∀i<a. B`** — the sized rung's design pass; non-ASCII;
* **the wheel marker `↻u`** — enters with the term-face/sessions stage;
* **quotation `‹…›` and the derive semantic tier** — metaprogramming-gated;
* **extern member attributes `@unwind`/`@variadic`/`@repr(c)`** — the bare-`@` form conflicts with the `@[…]` block discipline; the extern-member attribute story is unfixed;
* **angle-bracket generics `Result<Db>`** — rejected: `<`/`>` are comparison operators; `Name(args)` is the type-application form;
* **row-open records `{ℓ : A | ρ}` and set operations `∪`** — need the polymorphism/solver lane; non-ASCII;
* **references `ref`/`:=`, access modes** — a hard foreclosure of the current surface: no lvalue grammar;
* **kernel/metatheory notation** (effect-signature braces, effect rows, stratification arrows, multiparty arrows, term-merge) — not committed authored surface.

## Corpus obligations

* Every surfaced feature lands with its corpus treatment in the same change: a parse-gated `surface/` witness for syntax-only landings (the current posture for the reserved forms), graduating to runnable model plus pathological examples with harness assertions when the semantics lands.
* Named pending witnesses: the recursion model examples (structural recursion and a mutual group running with expected output, once recursive lowering lands); the pathological unmarked-self example (the error naming the marked spelling); the pathological escaping-reference example (the per-rung refusal); the η-hygiene pin (the completion engine consulting cut polarity); the guardedness cliff goldens (the four named deep-guardedness programs).
* The corpus counts and the three-tree split (model / pathological / parse-only surface) are as-built facts, kept honest per feature.
