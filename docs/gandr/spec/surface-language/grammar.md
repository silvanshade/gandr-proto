# The grammar machinery

The surface grammar's design: the precedence-bounded grammar (PBG), the molder/melder pipeline, the obligation taxonomy, and the parse-and-decline semantics.
What the pipeline produces downstream — CST, lowering, the machines — is the [[../implementation#The surface pipeline|implementation track]]; what the forms _mean_ is the other surface-language sub-documents.

## The parsing calculus

The normative parser is **not BNF, PEG, tree-sitter, or GF**: a checked **precedence-bounded grammar** over named precedence DAGs, hand-written constructor code with no codegen, in the tylr lineage [@moon-blinn-porter-omar-2025-tylr].

* A **grammar** is a set of forms: regexes over lexical tiles and recursive sorts, each attached to a `(sort, precedence)` pair.
* A **mold** is a zipper into the grammar, interned to a compact `MoldId(u32)`: `{ label, rctx, prec, sort }`, where `rctx` is the regex-context zipper.
  Molds are deterministic and fingerprint-scoped: ids are assigned in canonical table order at grammar build; the mold table folds into the grammar's fingerprint; **a CST records the fingerprint of the grammar that produced it**, so mold ids never migrate silently across grammar revisions.
* The **precedence DAG** is a named graph, not a number line: named precedence groups, edges for binds-tighter, per-group associativity, a precomputed reachability closure, and a fingerprint; strict precedence is reachability, and incomparable pairs are honored — an ambiguous mix is an error, never a guess.
  The choice of a graph over a number line, its literature grounding, and what it buys over the integer-level systems are [[operators#The precedence model is a named graph, not a number line]].
  **The DAG and the walk relation are not parser-internal notions**: both are public artifacts of the shared graph substrate, which also carries the analysis justifying the generalization and the re-proof obligations it leaves open ([[../implementation/graph-substrate#The precedence DAG]]).
* The pipeline:

```text
source text → lexer (hand-rolled byte DFA) → molder (obligation-minimizing dry run over the grammar)
  → melder (resumable first-order push machine) → commit → flat-arena CST + obligations
  → named-AST read adapter → lowering (total, with origin map) → linking → one computation
  → the typing machine + the L machine → outcome
```

* The **melder** is a resumable push machine over persistent, first-order, serializable state: `push` is total (Shift / Reduce / Degrout — incomparable precedences complete-and-reduce with grout at bottom, guaranteeing termination); `finalize` is a _query_ computing the completion that would close the input now, without committing it; batch `parse` is the derived fold `labeler ∘ molder ∘ fold(push) ∘ commit(finalize)`.
  The stack is one slope of terraces; the emission log is append-only, so rollback is truncation and checkpoints are cheap.
* **Error recovery is the obligation taxonomy, not panic-and-resync**, with lexical ambiguity resolved by obligation minimum rather than in the lexer.
* The CST's node kinds are deliberately **form-name-free**; declaration forms live in grammar rules at the item sort and in node-kind constants the lowerer dispatches on — there is no typed top-level declaration enum.
* Tree-sitter survives as **parity/reference tooling** (editor grammars, highlight preprocessing) under a differential parity harness; grammar growth is PBG-only, and the PBG-only construct kinds are a parity exemption registry (`PBG_ONLY_KINDS`, below).

## The three build-time gates

Every form must clear three structural checks at grammar build; a violating rule fails the build, so collisions surface immediately as fixtures rather than silent ambiguity.

1. **Operator Form** — no concatenation may expose two sort holes adjacent, even through nullable segments: **every sort hole is separated by at least one tile.** Mandatory call/ctor parentheses and keyword-led forms satisfy this structurally; juxtaposition application and open mixfix violate it — the structural reason those forms cannot be folded.
2. **Unique Tiles** — every tile occurrence interns to a distinct `(label, rctx)`: an alternation of two identical tile-led branches, or two structurally identical rules at the same sort, collides and is rejected.
   This is the molder-determinism seam, and it shapes the grammar: member forms like `ident : Type` stay **inline inside their container rules** (their left context gives a distinct `rctx`), never standalone helper rules that would collide by menu.
3. **Assumption 3** — precedence conflict-freedom: no distinct sorts `r ≠ s` with `s ∈ FIRST(forms of r)` and `r ∈ LAST(forms of s)`.
   The source conjecture is not assumed: the generated comparison table is additionally **tested** for conflict-freedom directly (every terminal pair has at most one derivable comparison).

Molder determinism also folds two derived tables that grow with every shared-prefix form: the per-label candidate menu and the `≐` same-form adjacency set.
More forms sharing a leading keyword means more molds under one label and heavier reliance on `rctx` to disambiguate — so new forms are kept keyword-led with genuinely distinct contexts.

## The obligation taxonomy

Obligations are the parser's error surface, the completion surface, and the recovery mechanism at once — severity-ordered, statement-local spans.
The taxonomy, in severity order:

| obligation        | meaning                                       | lowers to                                                  |
| ----------------- | --------------------------------------------- | ---------------------------------------------------------- |
| `MissingMeld`     | convex grout: no term where one was expected  | a typed hole                                               |
| `MissingTile`     | ghost tile: an absent delimiter               | hole + missing-delimiter note                              |
| `IncompleteTile`  | a partially typed keyword                     | hole + incomplete-keyword note                             |
| `UnmoldedTok`     | token not in the grammar at all               | unrecognized-token note                                    |
| `InconMeld`       | pre/postfix grout: a term of the wrong sort   | the sort-mismatch diagnostic, localized                    |
| `ExtraMeld`       | infix grout: two terms where one was expected | adjacent-terms note                                        |
| `ReservedKeyword` | a reserved keyword used as an identifier      | reserved-keyword note                                      |
| `AmbiguousPrec`   | incomparable precedence                       | maximum severity: the ambiguity diagnostic with candidates |

Two queries expose the machinery to every interactive surface: `obligations()` (the buffered obligations of the committed prefix) and `expected()` (the completion the melder would insert to close the input here — ordered tiles, grout, and holes with sorts).
The REPL's continuation decision and hints (`expected: ')' <expression> 'in'`), the TUI's ghost rendering, and the editor's hole materialization are the same data presented differently; materialization follows the execute-point policy — obligations are shown as annotations while typing and committed as structure only at submission, so the user always sees how the system chose to repair.

## The precedence bands as built

The four sort bands are mutually incomparable except for the declared within-band chains:

```text
item.singleton                                              (no assoc)
expression:  atom < postfix < unary < mul < add < cmp < and < or < ret
pattern:     atom < as < or
type:        atom < application < product < sum < set < arrow
```

* New item forms land at `item.singleton`; new block-leading expression forms (loops, `for`) land at `expression.atom` beside `if`/`case`/`thunk`/`co`; patterns extend the `pattern.*` chain.
* `ret` binds loosest and is right-associative; postfix (call, instantiation, projection) is tightest.
* Term-level binary operators are left-associative; **type operators are right-associative** (an unparenthesized flat chain like `A * B * C` written against the grain is a user-visible error).
* The union `|`, intersection `/\`, and lazy product `&` type operators sit in **pairwise-incomparable right-associative bands** — mixing them requires parentheses, so there are no set-operation precedence puzzles.
* The shell sub-grammar carries its own three-level ladder: `|` (pipe) tightest, then `&&`, then `||`.

## The keyword and operator tables

The live keyword set (one table; a rename is a one-table change):

| group                              | keywords                                                                                                                                                               |
| ---------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| declarations                       | `def`, `val`, `run`, `leta`, `as`, `extern`, `from`, `type`                                                                                                            |
| computations & values              | `fn`, `ret`, `thunk`, `force`, `case`, `if`, `else`, `co`, `hold`, `dup`, `drop`                                                                                       |
| sessions & sharing                 | `send`, `recv`, `close`, `select`, `offer`, `fork`, `acquire`, `release`                                                                                               |
| worlds                             | `migrate`, `at`                                                                                                                                                        |
| literals                           | `true`, `false`                                                                                                                                                        |
| types                              | `forall`, `F`, `U`, `mu`, `end`                                                                                                                                        |
| W4d/W4e growth                     | `data`, `codata`, `for`, `while`, `loop`, `break`, `continue`, `import`, `module`, `in`, `rec`, `op`, `rule`, `with`, `infix`, `infixl`, `infixr`, `prefix`, `postfix` |
| circuit block form                 | `sign`, `oper` (globally reserved); `sort`, `node`, `feed` (contextual)                                                                                                |
| reserved for proposals (not wired) | `handle`, `perform`, `reset`, `shift`, `quote`, `splice`                                                                                                               |

* Reservation policy: a **small closed set of globally reserved keywords**, with fixity classes contextual; new keywords are swept against the corpus for identifier collisions first — the fold-in reserved thirteen (`data codata rec op rule for in while loop break continue with`) globally, collision-free.
* `rec` was a legal identifier before its reservation — an accepted, intended source break (the explicit-marker decision); `codata` reserves as one whole keyword so it never lexes as `co` + `data`.
* The circuit block form reserves exactly its two **item-position** leads, `sign` and `oper`: at a fresh top-level slot a lowercase word is otherwise an expression statement, so an unreserved form-first lead would tie its own tile against `identifier` at every declaration.
  Its other leads stay contextual — the member keyword `sort` and the body statements `node` / `feed` are `≐`-successors inside an open block, so a program may still bind them as ordinary names (`list.sort` is a live projection in the corpus, so `sort` could not be reserved even if wanted).
  Contextual is **not** inadmissible-everywhere-else: because members carry no separator, a member lead is admissible exactly where the previous member's bare-sort side would sit, so `sign S { oper f : sort --> Nat … }` regroups cleanly and wrongly.
  The collision is bounded to that slot and to `sort`; the grammar module records it.
* The fixed term operator table: `||`, `&&`, `==`, `!=`, `<`, `<=`, `>`, `>=`, `++`, `+`, `-`, `*` (binary, left-associative), unary `-`; comparison chains ride the comparison band.
* The circuit arrow grid is four tiles and never more — `-->` / `<->` (circuit 1-cell formers) and `==>` / `<=>` (rewrite faces at every dimension) ([[circuit-cells#The block form, ruled]]).
  Each strictly extends a live shorter tile (`->`, `<-`, `==`, `<=`), so all four sit ahead of their prefixes in the labeler's longest-first table; `=>` never competes (neither glyph prefixes the other), and `--` is not a comment lead here.
  A word may carry trailing primes (`′`), the ruled form's spelling for a rewrite's target endpoint; ASCII `'` stays the shell single-quote opener.
* The primitive type set: `Any`, `Boolean`, `Char`, `Integer`, `Never`, `String`, `Symbol`, `Unit`, `Unknown`, `Void`, and the sized atoms `u32`, `u64`, `i32`, `i64`, `f32`, `f64`.

## The adaptations registry

Every divergence from a sketched design is recorded in-code as an `Adaptation` record on the grammar rule — the authoritative registry of surface shapings, each with its rationale:

* **`def rec` folded into the `def` family** as a fourth tail discriminating on the `rec` keyword after `def`, so `def` keeps one form-first mold and the rec-vs-name choice is locally decidable (a second `def` form-start would tie every top-level `def`).
* **Grade prefixes restricted to `{ number, ω }`** — the bare-identifier grade spelling is dropped, because a graded field `Cons(1 x: a)` must stay first-token-distinct from an ungraded field named `n`.
* **One `{` with an interior alternative** for `def rec` bodies (copattern clauses versus statement body), so two brace-led branches never share a context.
* **Operator-fixity declarations spelled `op <fixity> <level> "spelling" ;`** — designed fresh (no sketch existed), reusing the reserved `op` lead with a string-literal spelling so the labeler needs no user-operator token; the per-module wiring seam (`Pbg::extend`) is deliberately unwired.
  What wiring that seam costs, and the architecture the declaration serves, are [[operators]].
* **Loops reuse the block form**; **string interpolation lives only in expression-position strings**; **the shell subshell is spelled with square brackets** (POSIX parentheses stay free for the `$( E )` host escape); **braced parameter expansion `${name}` is a distinct labeler mode from interpolation `${E}`**; **spaced hole names `? name`** attach where an immediate-token rule could not hold.
* Member forms are **first-token-discriminated by case** inside `data`/`codata` blocks: uppercase-led constructors versus lowercase-led `oper`/`rule` keywords (the retired `op` member lead still parses and is declined with its respelling).
* **The `oper` / `rule` circuit judgment is declared once** and shared by the `sign` member and the top-level declaration, and telescope binders are confined to the parameter side — a duplicated tail would clone the signature, the port lists, and the body statements into a second set of molds, widening the hottest menus (`identifier`, `(`, `:`) twice as far.
* **The circuit arrow grid is admitted whole at every arrow position**, with the confirmation left to the checker: a body line's arrow comes from the applied head's kind, which is an environment fact no grammar sees, and a grammar restricted to the matching glyph would turn a nameable disagreement into a parse failure.
* **A top-level circuit declaration takes parenthesized sides**, so the sugar ladder's bare-sort rungs are `sign`-member-only.
  An Item-sort form that can end in a **sort hole** does not close — the melder has no following tile of an enclosing form to close it against, so a bare-sort side detaches and the declaration silently keeps only its prefix, a clean parse of the wrong tree that the zero-obligation gate cannot see.
  No other Item form in this grammar ends in a sort hole either: every `def` / `module` / `import` / `data` tail ends in `;`, `}`, or `)`.
  Inside a `sign` block the bare rungs stay available, because there the member's sort hole is form-**interior**.

## The parse-and-decline semantics

Reserved forms parse and **decline by name**: the grammar accepts the full shape from day one (the anti-retrofit posture), and the engine rejects each resident it does not yet implement with a named diagnostic quoting the resident text.
Decline is never a parse failure — the form is grammatical; the decline is a typed finding at the boundary the form crosses (scope pass, lowering, or elaborator), so the diagnostic names the rung that refused it.

The complete reserved-slot inventory, as built:

| form                           | shape                                                               | status                                                                                                                                    |
| ------------------------------ | ------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| operation member               | `op name(params) -> R` inside `data`                                | parse-and-decline                                                                                                                         |
| multi-output result            | `op name(..) -> (o1: A, o2: B)`                                     | parse-and-decline; the named-tuple result is **local to the op member**, never a general type form                                        |
| directed rewrite member        | `rule lhs ==> rhs` inside `data` or `codata`                        | parse-and-decline (`==>` is a fixed token); the retired `~>` also parses here, and the stage-0 elaborator declines it with the respelling |
| grade-prefixed field           | `Cons(1 x: a)`                                                      | parse-and-decline; grade tile restricted to `{ number, ω }`                                                                               |
| generalized constructor result | `C(..) : Vec(..)`                                                   | parse-and-decline                                                                                                                         |
| per-symbol attribute slot      | `C(..) [ctor, assoc]`                                               | parse-and-decline; declaration-position only, no `@` sigil inside the block                                                               |
| parameterized observation      | `ap(x: a): b`                                                       | parse-and-decline                                                                                                                         |
| grade-prefixed observation     | `1 next: F(Unit)`                                                   | parse-and-decline                                                                                                                         |
| with-view match                | `case xs with length(xs) { .. }`                                    | parse-and-decline                                                                                                                         |
| mutual-recursion block         | `rec { def ..; def .. }`                                            | parse-and-decline                                                                                                                         |
| operator-fixity declaration    | `op infixl 6 "++" ;`                                                | parse-and-decline; wiring seam unwired                                                                                                    |
| copattern default arm          | `_ => e`                                                            | parse-and-decline                                                                                                                         |
| reversible circuit former      | `oper f : (a : A) <-> (b : B)`                                      | parse-and-decline; the circuit surface check names the reversible-oper lane that owes its discipline                                      |
| circuit block members          | `sign`, `oper`, and `rule` declarations with `node` / `feed` bodies | parsed and arrow-confirmed; parse-and-decline at lowering                                                                                 |
| instantiation-slot residents   | `f[m<]`, `f[x = e]`, `f[size = e]`, `f[cost = e]`, `f[tail]`        | parsed by the grammar; **declined by name** by the recursion-surface scope pass (see [[recursion]])                                       |

## The PBG-only kinds registry

The constructs the parity grammar does not produce, registered as the parity exemption — twelve construct kinds (new rule provenances) and fourteen member surfaces (adaptation surfaces):

* construct kinds: `break_expression`, `circuit_declaration`, `codata_declaration`, `continue_expression`, `data_declaration`, `for_expression`, `import_declaration`, `loop_expression`, `operator_declaration`, `rec_block`, `sign_declaration`, `while_expression`;
* member surfaces: `braced_variable_expansion`, `case_with_view`, `circuit_body`, `circuit_member`, `circuit_signature`, `codata_observation`, `def_rec`, `feed_statement`, `grade_prefix`, `node_statement`, `op_member`, `parameterized_observation`, `rule_member`, `string_interpolation`.

## Performance and reuse discipline

* **No incremental parsing** — a measured finding, not a gap: at gandr's file scale, cold melder reparse is inside the latency budget (measured: ~253 µs largest corpus file, ~816 µs p99 in the pre-reboot pipeline; streaming push ~116 ns/token; obligation queries ~53 ns), and incremental locality is inherited from operator-precedence parsing's bounded-context property — statement-level resync by merkle identity, with sub-statement resync a later optimization, never a correctness feature.
* **Precomputation at grammar build**: the walk relation, the mold-candidate menus, the zipper steps and nullability bounds, and the precedence closure are all built once and content-addressed; a precomputed decision table may later replace dry-run search where the choice is context-free, gated on a differential proof of equivalence.
* Reuse keys carry the grammar fingerprint, so a top-of-file grammar-affecting edit invalidates exactly the right decisions.
* The melder's state is first-order, flat, and serializable — checkpoints, arena-resident stacks, allocation-free push.

## Source and confidence

The machinery is as-built (high confidence): the pipeline, gates, taxonomy, bands, and registries are verified against the `surface-grammar` crate (`surface.rs`, `surface/term.rs`, `surface/type_shell.rs`, `surface/circuit.rs`, `highlight.rs`) and the implementation track's surface-pipeline account.
The design rationale traces to the surface-syntax, operators, graph-core, and parser-interaction proposals and the syntax-inventory fold-in record; the manual's parser-core chapter restates it at reference depth.
The one claim resting on a conjecture — the precedence-DAG generalization of the calculus's metatheory — is marked as such wherever it is load-bearing (the engineering gates and the tested conflict-freedom table hold regardless).
