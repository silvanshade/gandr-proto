# Operators and notation

The architecture behind operator syntax: what the grammar may know, what a table may know, what the precedence model is and why it is a graph rather than a number, what the model languages do and what gandr takes from each, and the staged boundary between the fixed table that exists and the user-declared notation that does not.

The **machinery** this rides on — the precedence-bounded grammar, the molder and melder, the build-time gates, the precedence bands as built — is [[grammar]].
The **declaration form** is [[declarations#Operator-fixity declarations]].
This document owns the architecture and the design space around both.

## What is built, and what this document describes

**Built, and verified against the tree at write time.**

* **A fixed table of twelve binary operators and one unary.** `gandr-surface-engine`'s `lower/node_kinds` module declares the table as twelve spelling-to-prelude-name pairs — `||` `&&` `==` `!=` `<` `<=` `>` `>=` `++` `+` `-` `*`, mapping to `or` `and` `eq` `ne` `lt` `le` `gt` `ge` `concat` `add` `sub` `mul` — plus the unary negation's prelude name, and it mirrors the grammar crate's own table.
* **The lowering is the forced-prelude application, and it is tagged.** `gandr-surface-engine`'s `lower` module lowers a binary expression to an application of a forced prelude variable, in curried form, with every synthesized node tagged as an operator elaboration in the origin map.
* **The origin record is richer than the design asked for.** `gandr-surface-engine`'s `origin` module records, per synthesized node, the originating node's identity, its **per-node merkle hash**, its byte range, the elaboration tag, and a hole note — and the merkle hash is reproducible across runs and processes, which the parse-tree address it replaced was not.
* **The precedence model is a named graph.** The grammar's precedence structure is named groups with binds-tighter edges, per-group associativity, a precomputed reachability closure, and a fingerprint; incomparable pairs are honoured rather than resolved, and an ambiguous mix raises the taxonomy's maximum-severity obligation.
* **The fixity declaration parses and declines.** `op <fixity> <level> "spelling" ;` is admitted with the fixity classes as contextual tiles and the spelling as a string literal, so the labeler never has to tokenize a user operator.
* **The extension seam is implemented and unwired.** `gandr-surface-grammar`'s `model` module has `Pbg::extend`, which folds declared operators into a fresh grammar — changed fingerprint, added mold candidate, base mold identities preserved — with named contract witnesses.
  **Nothing outside its own tests calls it.**
* **The seam supports four fixities and the grammar admits five.** `Fixity` has left- and right-associative infix, prefix, and postfix, each with the precedence band it lands at; the grammar's fixity tile alternation additionally admits the non-associative `infix`, which has no band.

**Designed, and not built.** User-declared operators reaching the grammar at all; import and scope provenance for a declared operator; operator sections; closed and bracketed notation; programmable un-expanders; type-directed overload selection; and every tier above the fixed table.

## Capture and resolution are separate, and that is the whole architecture

The one commitment everything else follows from: **what the parser recognizes must not depend on what has been declared.**

Stated as two obligations rather than as one slogan, because they fail separately:

* **Capture is declaration-independent.** The grammar recognizes an operator's _shape_ from its tokens and their contexts alone, never by consulting a fixity declaration, an import, or an inferred type.
* **Resolution reads a table.** Precedence, associativity, and the target an operator denotes come from a machine-resident table that is ordinary serializable data, and the table is what extends.

**Under the current substrate the split lands in an unusual place, and the difference is worth stating precisely because it changes what the seam is.** The pre-reboot design put capture in a generated parser that produced a flat, unresolved operator sequence, and resolution in a separate resumable pass over the machine's operator table.
The precedence-bounded grammar dissolves that separation by absorbing the resolver: the melder _is_ a resumable push machine over a named precedence DAG, so it fixes operator **shape** during the parse, and there is no flat sequence to re-resolve afterwards.
What remains downstream is **name and overload resolution** — which prelude or module binding an operator denotes — which was always the type-dependent half and always belonged after the shape was fixed.

So the architecture survives the substitution and the mechanism does not: the grammar still knows nothing about declarations, because a declaration extends the grammar's **data** through a rebuild rather than steering the parse.

## The precedence model is a named graph, not a number line

Precedence is a **directed acyclic graph over named groups** — edges for binds-tighter, associativity carried per group, reachability precomputed, and pairs that no path relates left **incomparable**.
Mixing two incomparable operators without parentheses is an error, not a default.

**This is the design's single most load-bearing borrowing, and it is a specific published one** [@danielsson-norell-2011-mixfix]: the precedence relation restricted to a graph rather than a total order, so a language never has to assert a relative precedence between two operators that have nothing to do with each other, and so a proof that unique name-parts over an acyclic graph yield an unambiguous grammar is available at all.

Two consequences that a number line cannot deliver:

**Integer levels are a degenerate case of the graph, not an alternative to it.** A total order is a fully connected component; every system that exposes integer levels is therefore expressible in the graph, which is what lets one resolver serve both a fixed builtin table and a future user-declared one.

**Incomparability is a feature, and it is the one that scales.** A total order must decide every pair, so it decides pairs nobody thought about — silently, and in whichever direction the numbers happened to fall.
A graph declines them, and the decline is a diagnostic at a source span.

**gandr's surface can therefore exceed Agda's**, which is the system that implements this scheme.
Agda's user-facing precedence is a single floating-point number, which collapses the graph its own engine runs on back to a total order; its only surfaced incomparability is equal precedence with differing associativity, so any two Agda operators are otherwise comparable.
Exposing the graph itself is strictly more expressive, and it is what the current bands already do — the union, intersection, and lazy-product type operators sit in pairwise-incomparable bands precisely so that mixing them requires parentheses ([[grammar#The precedence bands as built]]).

## What the model languages do

The catalog the architecture was chosen against, across the ten dimensions an operator design has to answer.
Cells are terse.
The rightmost column is the published graph scheme itself [@danielsson-norell-2011-mixfix], which is theory rather than a system, and is what the four implementations are being read against.

| dimension                    | Agda                                                                                   | Idris 2                                                                                  | Lean 4                                                                                  | Swift                                                                                                   | the graph scheme                                                                         |
| ---------------------------- | -------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| **fixity and associativity** | left, right, non, plus a float precedence; prefix and postfix are one-sided mixfix     | left, right, non, prefix — **no postfix**; fixity decoupled from the definition          | infix, prefix, postfix all sugar over a general notation form; precedence is mandatory  | prefix, infix, postfix declarations; associativity belongs to the **group**, defaulting to none         | four classes — prefix, infix with associativity, postfix, closed                         |
| **precedence model**         | surface is a float, so a **total** order; the engine underneath is the graph           | integer levels, total order, no graph                                                    | integer levels with precedence arithmetic, total order                                  | **named groups with a partial order**; incomparable is a compile error; integers deliberately abandoned | the relation restricted to a graph; incomparable is a parse error                        |
| **mixfix and holes**         | full — the name splits at holes; closed, prefix, postfix, infix, and multi-part        | **none**; dropped from the previous version                                              | general, through the notation form                                                      | none; operators are single tokens                                                                       | alternating name-parts and holes, no adjacent holes; a closed operator is a bracket pair |
| **sections**                 | any subset of holes may be left open, **including interior ones**                      | left and right sections become lambdas                                                   | a placeholder becomes a lambda over the nearest parentheses                             | none; the parenthesized operator is a function value                                                    | discussed, not in the core; a section is partial application                             |
| **scope and import**         | per-name, with fixity **changeable on import** by renaming                             | namespaced, with export and private, and a fixity-qualified hide for clashes             | environment data, exported transitively, with scoped and local variants                 | file and global scope, auto-exported; joining two imported groups is an error                           | modularity is the motivation; the mechanism is unspecified                               |
| **notation against macros**  | three tiers — mixfix rearranges, a syntax form reorders and binds, reflection computes | notation is pure sugar; a **separate** elaborator tier computes                          | a continuum — "everything is a macro", with the syntax and rules split kept for tooling | operators rearrange into calls; macros are an unrelated feature                                         | pure notation; output is operator applications; no hook                                  |
| **unparse**                  | a scope-sensitive mechanical inverse; no user printer                                  | fixity-driven infix redisplay; no user un-parser                                         | an automatic un-expander when each hole appears once, and an attribute for the rest     | no contract                                                                                             | a proved display function with a round-trip theorem                                      |
| **type-directed resolution** | parsing is type-independent; **name** overload resolves by type later                  | overload by concrete argument and return types, bounded and non-backtracking             | overload by type classes, plus type-directed sugar through elaborators                  | meaning is type-directed; **precedence is not**                                                         | the core is type-free; type-directed disambiguation is a composable layer                |
| **ambiguity policy**         | reject, do not guess — all parses shown                                                | a fixity clash **warns and picks by import order**, which its own docs flag as dangerous | longest match, then priority, then keep all and elaborate all                           | incomparable or non-associative adjacency is an error; never a priority guess                           | reject; unique name-parts over an acyclic graph give unambiguity as a theorem            |
| **hygiene**                  | mixfix binds nothing, so not applicable; syntax-form binders are user-written          | binders are user-supplied; fresh names only in the elaborator tier                       | **scope-set** hygiene — an ordered macro-scope list plus a top-level set                | not applicable                                                                                          | operators bind nothing; binding notation is an acknowledged limitation                   |

Two familiar baselines, because they bracket the space from the other side.

| dimension                    | Haskell                                                                           | Coq and its successor                                                                      |
| ---------------------------- | --------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| **fixity and associativity** | left, right, non, at integer levels zero through nine, defaulting to left at nine | infix and general notation declarations, with left, right, or no associativity             |
| **precedence model**         | integer levels, total order                                                       | integer levels, total order, plus **per-argument sub-levels**                              |
| **mixfix and holes**         | none — symbolic infix only                                                        | full multi-token notation with holes, including recursive notations                        |
| **scope and import**         | top level, exported and imported with the name                                    | notation **scopes**, opened and closed, with delimiters selecting one                      |
| **unparse**                  | redisplays infix by fixity; no user printer                                       | separable parse and print directives — a print-only notation is expressible                |
| **ambiguity policy**         | same-level incompatible-associativity adjacency is an error; no guessing          | levels resolve most; residual ambiguity needs an explicit scope or a parse-only annotation |

**What the baselines settle.** Integer levels really are the degenerate graph, and operator hygiene really is a non-issue while operators bind nothing.
The scoped-notation baseline is the cautionary one: it has the most expressive notation in the table and the most acute ambiguity management, and the two are the same fact.

## The feasibility split

One row per concrete feature, each carrying exactly one tag.
**The tags are postures against the current substrate, not schedule commitments**, and they are re-tagged from the pre-reboot analysis rather than transcribed — the substitution moved several rows, and the moves are the informative part.

| feature                                         | tag                           | why, and what it costs                                                                                                                                                                                                                                       |
| ----------------------------------------------- | ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| the fixed builtin table                         | **built**                     | the twelve binary operators and the unary lower through the tagged forced-prelude application; the pre-reboot analysis tagged this cheap-and-unbuilt, and it has since landed                                                                                |
| precedence and associativity over a fixed graph | **built**                     | the melder resolves shape against the named DAG during the parse; what the analysis costed as a separate memoized resolver is the parse itself                                                                                                               |
| reject-don't-guess ambiguity                    | **built**                     | incomparable precedence raises the taxonomy's maximum-severity obligation with candidates                                                                                                                                                                    |
| user-declared operators reaching the grammar    | **seam built, unwired**       | the extension function exists with contract witnesses; wiring needs the module provenance story — which declarations are in scope where, and what a rebuild costs                                                                                            |
| the non-associative fixity                      | **band owed**                 | the grammar admits the spelling and the model has no band for it, so a declaration using it would parse and could not be folded even once the seam is wired; the spelling stays and the band is owed                                                         |
| closed and bracketed notation                   | **grammar-shaped**            | the first and last name-parts are ordinary tiles, so a closed operator satisfies the adjacency gate structurally and is an ordinary form rather than a special case                                                                                          |
| operator sections                               | **lowering**                  | detecting an unfilled position is cheap; synthesizing the abstraction is a lowering step building a lambda over the forced target, and only an interior hole needs reordering                                                                                |
| type-directed overload selection                | **lowering**                  | runs after shape is fixed, so capture stays type-independent; this is the safe half of type direction                                                                                                                                                        |
| scoped and renamable-on-import fixity           | **needs provenance**          | the active table becomes location-dependent, so both resolution **and** unparse must know which table was active at a position                                                                                                                               |
| hygiene for binding notation                    | **needs a fresh-name source** | notation that mints a binder needs capture-avoiding names; the binder-free common case needs no hygiene at all                                                                                                                                               |
| a procedural or reflective notation tier        | **needs the parked lane**     | programmable syntax-to-syntax and elaborator hooks are genuine new build, and they belong to the metaprogramming lane rather than here                                                                                                                       |
| programmable un-expanders                       | **needs the parked lane**     | beyond inverting the recorded elaboration — the non-linear and custom cases                                                                                                                                                                                  |
| open alphabetic mixfix                          | **structurally refused**      | an open mixfix _form_ exposes two adjacent sort holes, which the grammar's first build-time gate rejects outright; the gate constrains forms, so this is a different reason from the pre-reboot one rather than a stronger one, and it is `ours` — see below |
| type-directed **parsing**                       | **refused**                   | it would make capture depend on inferred types, breaking the determinism every reuse key and every diagnostic rests on                                                                                                                                       |
| warn-and-guess ambiguity                        | **refused**                   | an expression's meaning would depend on import order; the language that ships it flags it as dangerous itself                                                                                                                                                |
| runtime grammar mutation from inside the parse  | **refused**                   | the grammar is built once and fingerprinted, and a CST records the fingerprint that produced it; mutation during a parse has nothing to record                                                                                                               |

**Three rows moved when the substrate changed, and the direction is the same in all three.** Precedence resolution, ambiguity rejection, and the fixed table were all costed as future work over a machine-resident resolver; the precedence-bounded grammar delivered them as the parse.
**One row moved the other way**: user declarations were costed as "not a parser change" and are now precisely a grammar rebuild, because the table the melder reads _is_ the grammar.

## Ambiguity: reject, never guess

An operator sequence resolves to exactly one tree or to a diagnostic.
**Nothing in the pipeline is permitted to pick.**

The two named anti-patterns, kept because each is a live language's shipped behaviour rather than a hypothetical:

**Warning and then picking by import order** makes an expression's meaning depend on the order modules were imported, which is not a property of the program.

**Keeping every parse and letting the type-checker select** is deterministic but couples the parse outcome to the type context, which makes every reuse key carry the type context too.

The gandr position is Agda's and Swift's: **reject with a source span and, where a parenthesization would resolve it, say which one.** The graph scheme's theorem is what makes that policy affordable rather than what argues for it — unique name-parts over an acyclic graph give unambiguity outright, so rejection is the residual case rather than the common one.

## Lowering

Operators are surface syntax.
**They add no core node**, and this is a commitment rather than an observation: an operator node in the core would have to be given a meaning by the kernel, and its meaning is an application.

```text
x ⊕ y   ⇒   ((force ⊕̂) x) y
⊕ x     ⇒   (force ⊕̂) x
x ⊕     ⇒   (force ⊕̂) x
```

The target `⊕̂` is a prelude or module binding of thunk type, so the lowering forces it and applies it in curried form, and the polarity split is preserved throughout: the operands are values, the application is a computation.

**Operator names are ordinary surface strings.** They are not machine-minted atoms, and the fresh-name substrate is deliberately not a dependency of operator support — it is the right substrate only as a fresh-name source for a future notation tier that mints binders, and even then it supplies names rather than implementing hygiene.

A section, when one lands, lowers to an ordinary lambda over the missing operand, with hygiene applied to the minted binder:

```text
(_ ⊕ y)   ⇒   fn (x) { x ⊕ y }
(x ⊕ _)   ⇒   fn (y) { x ⊕ y }
```

**An interior or reordered hole is not a section.** It needs an explicit argument-ordering abstraction, and it is the one place in the section design where the shape of the notation and the shape of the lambda come apart.

## Origin and unparse

Every operator lowering emits enough to explain or reverse itself.
The record as built carries the originating node's identity and its content hash, the byte range, and the elaboration tag; the design adds, for a declared-operator future, the **active table's fingerprint** and the target chosen after name resolution.

**The display rule is a fallback rule, and the fallback is the point.** A lowered operator expression may be rendered back in source form when the source span is still available, the table fingerprint matches the one it was resolved under, and reversing the elaboration is unambiguous.
**When any of those fails, the renderer shows the explicit lowered form** — it never invents a source operator.

That the content hash is reproducible across processes is what makes the whole scheme sound rather than best-effort: a provenance key that is a memory address is valid only within one run, and un-parsing is exactly the operation that wants to outlive one.

## Hygiene and tier ownership

**Operators bind nothing, so the common case needs no hygiene at all** — a fact all four surveyed implementations confirm independently: where their notation does bind, the binders are written by the user at the call site, and fresh names appear only at a macro or elaborator tier.

Where notation _does_ bind — a section that synthesizes a lambda, a future binding-construct notation, or a tier that mints binders — hygiene is the **scope-set** discipline [@flatt-2016-binding-sets-of-scopes]: an ordered list of macro scopes plus a top-level set, resolved by equality.
Scope sets are the discipline; a fresh-atom source is a component of it, not a substitute for it.

**Which tier may declare an operator is not this document's decision to take.** The tier ladder — declarative table entries, procedural generation, elaborator reflection — belongs to the metaprogramming lane, which is parked.
What this document fixes is the **constraint that ladder must satisfy**: whichever tier declares an operator, the declaration mutates table data and is recorded and replayable, and no tier registers a grammar rule during a parse.

## What the substrate substitution retired

The pre-reboot architecture was written against a generated static parser, and the reboot replaced that parser.
**Each retirement below names what took its place**, so that a reader meeting the old design elsewhere can tell superseded from dropped.

| retired                                                                | tombstone                                                                                                                                                                                                  |
| ---------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| the separate resumable resolution phase over a captured flat sequence  | the melder is itself a resumable machine over the precedence DAG and fixes shape during the parse; only name and overload resolution remain downstream                                                     |
| the parser-side size and state-count budgets                           | the three build-time gates — hole adjacency, tile uniqueness, and precedence conflict-freedom — which are structural rather than budgetary and fail the build rather than a threshold                      |
| the zero-declared-conflicts and no-dynamic-precedence requirements     | the same properties, now structural: the grammar has no conflict mechanism to declare and no dynamic precedence to forbid                                                                                  |
| the no-external-scanner requirement                                    | there is no scanner to be external to; the labeler is a hand-rolled byte automaton and the fixity spelling is a string literal, so a user operator never becomes a token                                   |
| the operator-table fingerprint in the incremental checkpoint footprint | the fingerprint moved onto the grammar itself, and a CST records the fingerprint that produced it; **there is no incremental parsing** to carry a footprint, which is a measured finding rather than a gap |
| the per-sequence memoization and graph-pruning performance study       | the reachability closure and the mold candidate menus are precomputed once at grammar build and content-addressed                                                                                          |

**One of those is a withdrawal rather than a substitution, and it should not be read as one.** The incremental-parsing claim the checkpoint footprint depended on was found to be untrue of this pipeline and was withdrawn: cold reparse is inside the latency budget at gandr's file scale, so incrementality is an optimization nobody has needed rather than a correctness property ([[grammar#Performance and reuse discipline]]).
The footprint obligation therefore has no successor, and inventing one would be worse than recording that it lapsed.

## Sections, closed notation, and open mixfix

**Closed and bracketed notation is ordinary, and the current grammar is why.** A notation whose first and last name-parts are tiles separates every hole with at least one tile, which is exactly what the hole-adjacency gate demands.
So a closed operator is a grammar form like any other, and its interior holes restart at the loosest precedence.

**Open alphabetic mixfix is refused structurally, and the reason is different from the one the pre-reboot design gave.** That design refused it because a generated parser could not find an open region's boundaries without consulting declarations — a fact about the parser.
The current refusal is a fact about the grammar: an open mixfix _form_ exposes two sort holes with nothing between them, which the hole-adjacency gate rejects at build.
That gate is the one the grammar document calls **Operator Form** ([[grammar#The three build-time gates]]), and the name is a collision worth knowing about — it is about sort holes and tiles, and has nothing to do with the user-facing operators this document is named for.
**The gate refuses a form, not adjacency, and that distinction is load-bearing enough to state.** The shell sub-grammar reaches juxtaposition with no form at all — each shell word is its own single-tile `Expression` atom and the melder joins the juxtaposed atoms, with grouping deferred to the semantic stage — so the gate does not settle whether an open region could be recognized by some other route.
It follows that this refusal and the separate rejection of juxtaposition application are **not one refusal**: that one is doctrinal and economic, and explicitly not technical ([[recursion#Application syntax, rejected]], which [[../surface-language]] routes to as its full reasoning).
Both premises here are facts about gandr's own grammar, which the premise test of [`docs/workflow/review.md`](../../../workflow/review.md) tags `ours`, so this refusal is recorded with no reopening delta and re-grading it is the owner's; the question is filed as `gandr-fid.14.7-question-12`.

**The genuine notational win of that family is mixfix, not application.** The fixity declaration is where it is reserved, and a future tile-level substrate — where a grammar is data rather than a compiled table — is where dynamic registration could genuinely live.
That path is deliberately separate and must not be back-ported as parse-time mutation.

## The conformance checklist

An implementation of this architecture is conforming only if all of the following hold.
**The list is rewritten against the gates that exist**, and where a pre-reboot item named a generated-parser property, the current structural equivalent stands in its place.

* the grammar is built once, fingerprinted, and never mutated during a parse;
* every form clears the three build-time gates, so no operator form exposes adjacent sort holes;
* a CST records the fingerprint of the grammar that produced it;
* capture is declaration-independent — no operator form's recognition consults a declaration, an import, or an inferred type;
* precedence is a named graph with reachability and honoured incomparability, never a total order imposed on it;
* an ambiguous mix raises a diagnostic at a source span and is never resolved by priority, import order, or type;
* open alphabetic mixfix is not admitted;
* a fixed builtin table and a future declared table read the **same** structure, so neither is a throwaway;
* operators lower to existing core forms only, and add no core node;
* the origin record preserves enough to explain or reverse the elaboration, keyed on a fingerprint that outlives the run;
* the display path falls back to the lowered form rather than inventing a source rendering; and
* any tier that affects the table does so through recorded, replayable declarations.

## Open questions

### operators-question-01

**Where name and overload resolution sits, now that shape resolution has moved into the parse.** The pre-reboot question was whether one resolver phase should straddle a type-free structural pass and a type-directed selection pass.
Half of it is answered — the structural half is the parse — and the other half is unowned: which component selects an operator's target, and whether it does so before or during type-checking.
**Disposition: carried**, and it is a prerequisite for the declared-operator wiring rather than for the fixed table.

### operators-question-02

**Whether the graph needs a richer representation than reachability data.** Exposing the full named graph, built up per import, was asked as a possible argument for a constraint representation rather than inert data with a query.
The current answer is inert data with a precomputed closure, and it is adequate for a fixed table.
**Disposition: declined for now, with a reversal condition** — it reopens if per-import build-up makes the closure's rebuild cost visible, which cannot be measured until the extension seam is wired.

### operators-question-03

**What delimits a user-declared operator's region.** The pre-reboot form of this question was about a flat capture region's boundaries, and the grammar's hole-adjacency gate has since answered it for both horns: symbolic infix is an ordinary form, and open alphabetic mixfix is refused at build.
**Disposition: retired with a tombstone.** The question presupposed a capture phase that no longer exists; what survives of it is the closed-notation design, which is not a boundary question.

### operators-question-04

**What a declared-operator grammar rebuild costs, and what it invalidates.** The pre-reboot form asked where an operator-table watermark enters an incremental checkpoint footprint; there is no incremental parsing, so that form has lapsed.
The live form is different and sharper: an extension produces a **new grammar with a new fingerprint**, and every CST carrying the old fingerprint is keyed to a grammar that no longer describes the file.
**Disposition: carried**, and it is the substance of the wiring work rather than a side question.

### operators-question-05

**Which tier may declare an operator.** Whether a purely declarative tier may mutate the table at all, or whether table-affecting power is reserved higher.
**Disposition: parked, with a reason and an owner** — the tier ladder belongs to the metaprogramming lane, which is parked, and deciding tier ownership inside this document would decide part of that lane's design without its context.

### operators-question-06

**Whether any type-directed disambiguation is admitted as an opt-in.** Structural rejection is the policy; the open part is whether an explicit opt-in layer may select among parses by type, as three of the surveyed systems do in some form.
**Disposition: declined, with a reversal condition** — it stays refused while it would couple parse outcome to type context; it reopens only behind an explicit per-site opt-in whose cost is paid by that site, never as a default.

### operators-question-07

**The non-associative fixity has a spelling and no band.** The grammar's fixity tile alternation admits five classes and the extension seam's fixity type carries four, each with the precedence band its form lands at, so a non-associative declaration parses today and could not be folded even once the wiring seam is wired.

**Disposition: carried, and the spelling stays.** Removing the tile was the other option and it is declined: the spelling being admitted is evidence that someone intended the fixity to exist, non-associative infix is one of the four classes the precedence-graph scheme itself distinguishes [@danielsson-norell-2011-mixfix], and discarding an intention on the strength of an incomplete implementation is the one direction of this trade that is hard to notice afterwards.

What is owed is the band, and it is small, self-contained, and independent of the rest of the wiring.

## Source and confidence

The architecture descends from the pre-reboot operator-and-notation design record and from the cross-language research catalog that record names as its grounding — a verifier-checked survey whose per-lane verdicts were recorded separately, and whose corrections to its own sources were folded in before the design consumed it.
The catalog's ten dimensions, its two baseline columns, its feasibility split, and its seven carried questions are all transferred; the questions are dispositioned above, and the split is **re-tagged against this tree** rather than copied, because four of its rows changed status when the parser was replaced.

**The as-built account is high confidence and was read from definitions**: the operator table and its prelude names, the lowering's shape and its elaboration tag, the origin record's fields, the extension seam and its callers, and the fixity enumeration against the grammar's fixity tiles were each verified against the named module.

**Two claims rest on the surveyed systems' own documentation rather than on this project's verification**, and are marked here rather than at every cell: the catalog's per-language cells, and the two named ambiguity anti-patterns.
The catalog was assembled with an independent verification pass per source, which is why it is carried at all; that pass is not this project's, and its corrections are recorded in the research record rather than here.

**The precedence-graph borrowing and the hygiene discipline are cited to their published sources** and both entries carry resolvable identifiers.
The remaining named systems are cited by name only, as implementations rather than as literature.
