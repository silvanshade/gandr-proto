# The interactive and toolchain surface

**Proposed.
One of the faces this document designs is built; the rest are not.** `gandr <file>` runs one source file and reports the run as a process status — that is the script-runner face — while the read-evaluate loop, the language-server adapter, and the formatter land with crates that do not exist yet.
The next section separates what is already built underneath all of them from what is not.

This document fixes the surface that makes gandr usable as a shell and as a scripting language: the read-evaluate loop, structured data, command history, completion, highlighting, diagnostics, a language-server adapter, and a formatter.

Its organizing commitment is a single one, and everything else is a consequence: **the checker's state is data, and every surface here is an encoder over that data.** Nothing in this document re-implements typing, and the moment one of them would need to, the fix goes into the pipeline instead.

## What is built, and what this document describes

**Built, and verified against the tree at write time.**

* **The driver is a working script runner, and running scripts is its whole contract.** The `gandr` binary — the package the toolchain driver crate builds — takes `gandr <file>` and hands the path to `gandr-surface-engine`'s `run` module, which lowers, links, prelude-checks, and runs the program under the host-effect handler; the driver owns no pipeline of its own and is the process boundary and nothing else.
  It turns the run's outcome into a process status — `0` for a value terminal, a `proc.exit` code reduced to a byte the way a shell reduces one, `1` for a blame, a stuck configuration, or a fatal host abort, and `2` for a malformed command line or a source that never reached the machine.
  **A run prints nothing of the driver's own**, so a script's output reaches its consumer unmixed; the usage text is the one thing the driver writes to standard output, and only when it was asked for rather than as a complaint.
  Its production consumer is `scripts/agda-deps.gandr`, which provisions the Agda proof vehicle in the language the toolchain is for; its test suite drives the real binary, because what is under test is what a calling shell observes.
* **The renderer firewall exists as a crate.** `gandr-surface-render-remote` holds **wire types only** — plain, serialization-ready data that the pipeline projects and renderers consume.
  It parses, lowers, types, and marks nothing, and it depends on no other workspace crate, so an adapter can link it without pulling in the checker.
  Its two layers are the presentation seam (highlight and mark spans, diagnostic and goal cards, preview and transcript frames, the byte-to-position projection) and the versioned bus frame with its delta schema.
  Serialization rides a default-off feature, so an in-process renderer pays nothing for it.
* **The report is a versioned envelope carrying diagnostics and goals together.** `gandr-surface-engine`'s `diag` module maps a typing failure to a diagnostic and carries hole goals in the same envelope, shaped as an agent-consumable stream.
* **The origin map's keys survive the process.** Each entry carries the originating node's per-node merkle hash alongside its identity and byte range, so a provenance key is reproducible across runs rather than valid only inside one.

**Designed, and not built.** Everything else: the loop itself, the codecs, history, completion, the language-server adapter, and the formatter.

**One caveat about the driver's manifest, because its deferred-face list reads like a parking lot and is not one.** The list names what each deferred face waits on, and the three cases differ: the REPL waits on a line editor, its other named dependencies — the checked grammar its highlighter molds against, the parser whose obligation queries drive its validator and completer, and the checked borrowed source slice that parser takes — being workspace crates that already exist; `tui` waits on the terminal programming environment, which has no crate in this tree at all; and `lsp`, `mcp`, `fmt`, and `build` are subcommand slots with no implementing crate.
A face therefore arrives with its real dependency edge in the same change, never by uncommenting a line.

## The one machine state, and the firewall

The checker's state is reified as serializable data, and the interactive surface, the language server, and an agent stream are **three renderers of one machine state**.

The pipeline already emits the presentation payload as plain data: a report of diagnostics and goals with reserved slots for marks and obligations, and a per-node marking.
Every surface in this document is an **encoder** over that payload.

**The firewall is what keeps them from forking**, and it is a rule about where fixes go rather than a rule about layering: _if a renderer needs information the report cannot supply, the fix goes into the pipeline, so all three renderers gain it at once._

That rule has one structural consequence worth stating separately, because it is what makes it enforceable rather than aspirational: **none of these crates may parse, lower, type, or mark.** The renderer-firewall crate already obeys it by construction — it is a leaf holding wire types and nothing else — and the rule extends to every face built on it.

## The typing surface is the enabling prerequisite

**The shell is not usable until structured shell data is statically typed**, so building out the value model leads the critical path rather than trailing it.

**The gradual unknown type is a boundary, never a representation.** It is reserved for the raw-decode edge — the result of decoding a payload _before_ it is refined to a schema, or genuinely heterogeneous input — and it is explicitly not the de-facto type of shell values.
A value left unknown carries no static guarantee and no runtime shape check, so it is refined into a typed record before use and never threaded through a pipeline.

**The value-model ladder**, in dependency order, with what has since landed:

| rung                                                          | status                                                       |
| ------------------------------------------------------------- | ------------------------------------------------------------ |
| a string value carrier — record keys, paths, decoded strings  | landed                                                       |
| numeric widening                                              | landed                                                       |
| a positive list former with its eliminators                   | landed                                                       |
| a positive record former                                      | landed, as the record literal with its projection eliminator |
| polarity-sorted unions and intersections                      | specified, not built                                         |
| a value-level positive fixpoint for a structured decoded type | not built, and it depends on the rung above                  |

**The record former's polarity argument is worth keeping even though the decision landed**, because it is the reason the decision is stable.
Records are **positive** — a value former, by analogy with products.
The intersection encoding was refuted rather than declined: intersection is a **computation** connective, and intersection introduction types the _same_ computation at both types, which is overloading and not field combination, so it cannot build a value record at all.
Three options were live — a row-typed open record with a row variable, the refuted intersection encoding, and a closed positive record with width and depth subtyping — and the closed record was adopted, with the row variable kept as a **refinement rather than a retrofit**, since a closed record is the empty-tail case of a row-typed one.
The surface that landed is [[../../surface-language/value-semantics|the update surface]]'s.

**Optional fields ride that same fork**, and the fork is real: presence-polymorphism needs the row variable, while the alternative of a field typed as a union with a null carrier needs the set operations and is **untagged**, so it can be eliminated only under both arms at a bind and can never be tag-dispatched.

## The pillars

Each rides the shared machine-state-to-presentation projection.

**The loop.** The line editor owns raw text, editing, history, and menus; on submit the validator gates on **parse completeness only**, because holes are deliberately typeable and "has holes" is not "incomplete".
The loop then lowers hole-tolerantly, reports diagnostics and goals, and evaluates only a hole-free rigid term.
Cross-line definitions carry over by re-lowering an accumulated session prelude — **an interim simplification** of a persisted typed context, and it is marked as one so that nobody mistakes it for the end state.

**Structured data.** A pure codec layer over the value API, one format required and the rest following.
It targets the typed value model above; the unknown type is the boundary for an undecoded payload, refined immediately.

**History.** One history implementation over a backend trait.
The entry is **content-addressed** — canonicalize, then hash — and the log is an append-only root chain.
A relational backend now, behind the trait, so a content-addressed store is a swap rather than a rewrite; plaintext history is explicitly out.
The richer-than-usual entry — exit status, working directory, duration — is in scope; persisting evaluated **result values** waits on the value model.

**Completion, highlighting, diagnostics.** Completion is a **query over machine state**: splice a synthetic hole at the cursor, read its goal type and in-scope context, and rank candidates against them.
That is the payoff the whole reified-state design is for, and it is fundamentally stronger than text completion — but it is **partial**, and saying so is part of the design: the goal is determined at a checking-position cursor, while identifier and external-command completion do not need it at all.
Highlighting is lexical plus a semantic overlay that tints marks at the offending node — **errors as marks, not aborts**.
Diagnostics are the monotone "what you still owe", goals and obligations together, and they are **never a submission gate**.

**The language-server adapter.** A thin protocol shim: every request is a projection of the document's parse tree, lowered form with its origin map, and report, through the span lens.
The streaming checker is itself a session protocol and the adapter is one concrete client of it.
Its first cut is diagnostics, hover, completion, and delegated formatting on a whole-file recheck, within the latency budget at script scale; incrementality is a transparent scaling lever behind the same report interface rather than a redesign.
The adapter is only one of **two** channels, and the protocol that carries the other — the heavy interactive payload the editor protocol cannot subscribe to — is [[../inspection-protocol]].

**The formatter.** It formats the **concrete syntax tree, never the core** — the core drops comments and re-inflates the verbosity the surface exists to hide.
Idempotence and meaning-preservation under lower-after-parse are binding, and **error and hole regions are left byte-identical**, which is the same fidelity-over-best-effort posture the project's other formatters take.
It is three components rather than one: a shared layout engine [@porncharoenwase-pombrio-torlak-2023-pretty-expressive], the source formatter over the concrete tree, and a separate core printer for the loop and for diagnostics.

## What a usable shell surface still requires

The requirement list, stated as capability gaps rather than as a schedule.

* **The host operation handlers** — the process, filesystem, environment, and exit families — through the preserved host-effect seam ([[../../implementation#The runtime host]]).
* **A record and table combinator library** — filter, map, project, insert, reduce, unique, sort, and optional-cell access — implemented as **native higher-order builtins**, so the iteration lives in the host language and applies a gandr closure per element.
* **A string and pattern library** with named-capture extraction.
* **The codecs**, one required format first.
* **A typed error and exit model**, largely present through the effect layer.
* **An assertion builtin with a handler-based test runner** — and the runner is itself a handler catching the assertion effect, which is the effect layer dogfooding itself.

**One thing is explicitly not on this path, and it is the one most readers assume is.** Core recursion is **not** required for a scripting surface: script-shaped programs are combinator-shaped, and the one genuinely iterative construct — a worklist frontier — is a native builtin.
Recursion is needed for **user-level recursive functions**, which is a different capability with its own design ([[recursion-former]]).

The dependency order among the requirements is fixed even though the schedule is not: **the host seam and the process and filesystem families come first**, because they prove the syscall loop; then the string carrier and pattern library, which need no codec; then the first codec together with content verification; then the remaining codecs; and last the test runner, which is the capstone precisely because a mock handler replacing a real one is the strongest evidence that the handler boundary is real.

## Decisions with reversal conditions

Each is a conservative default with its reversal trigger named, and none blocks the path.

| decision                                    | current                                                                                                   | reversal condition                                                                              |
| ------------------------------------------- | --------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| **record-former carrier**                   | the closed positive record with width and depth subtyping — landed                                        | the row variable lands with the polymorphism rung and its biunification experiment              |
| **history backend**                         | a relational backend behind a trait                                                                       | a persistent, **traversable** content-addressed block store ships                               |
| **host-effect API**                         | a seam in the core that admits a reply originating outside the term                                       | none — the zero-core-change alternative is refuted, below                                       |
| **decoded-data typing**                     | decoding is fallible, so it returns an effect row over the unknown type rather than a total returner      | the structured decoded type lands, once the value fixpoint and records are both available       |
| **protocol, layout, and storage libraries** | a synchronous protocol loop, a document-algebra layout engine, an embedded synchronous relational binding | concurrency pressure, or footprint and maintenance signals — each a fresh survey at commit time |

**One of those is a refutation rather than a default, and it is recorded as one.** A source-level handler clause cannot interpose on a host operation, because a clause body is _in-term_ and a syscall reply is not: the host can observe a performed operation at the focus but has no way to inject a reply originating outside the term and splice the resumption.
So the seam is required rather than preferred, and the "zero core change" alternative is refuted on the machinery rather than declined on cost.

## The host-effect seam, and why it is not this document's

The seam that makes any of this runnable is a **core** item, not a toolchain one, and it is owned separately.
What belongs here is only its consequence: the seam is the **earliest** gate on the whole interactive surface, and it is one of the small number of core additions the surface depends on rather than a library it can be written around.

Its current form — the preserved boundary over public values and the operation name, the four operation families with two-level dispatch, representable outcomes as ordinary replies, and the ambient always-resume posture with **no capability model yet** — is [[../../implementation#The runtime host|the implementation track's]], and the grant model that would price it is [[../capability-model|the capability model]].

## Open questions

### interactive-surface-question-01

**Whether the session prelude survives contact with a persisted typed context.** Cross-line definitions carry over by re-lowering an accumulated prelude, which is explicitly an interim for a persisted context and its checkpoint driver.
What is not stated anywhere is whether the interim is _compatible_ with the end state or merely cheaper — whether a session built by re-lowering can be migrated to a persisted context without the user observing a change.
**Disposition: carried.**

### interactive-surface-question-02

**How partial completion presents its own partiality.** Goal-directed completion is determined only at a checking-position cursor, and the design says so; what it does not say is what the surface shows at a cursor where the goal is undetermined — degrade silently to identifier completion, or say that the stronger ranking is unavailable here.
**Disposition: carried**, and it is a diagnostic-quality question rather than an architectural one.

### interactive-surface-question-03

**Whether the renderer firewall's rule is checkable.** "None of these crates may parse, lower, type, or mark" is currently a discipline the leaf crate happens to satisfy by having no workspace dependencies.
Whether it should be a **gate** — a dependency fence like the ones the workspace already runs — is unowned.
**Disposition: carried.** The rule is the load-bearing part and it is stated; making it mechanical is a separate call with its own cost.

### interactive-surface-question-04

**What the formatter does with a shell block.** Idempotence and meaning-preservation are binding, and error and hole regions are byte-identical — but a shell block is an embedded sub-grammar whose interior is neither an error nor a hole, and nothing records whether it is formatted, left alone, or delegated.
**Disposition: carried**, and it interacts with the shell fragment's own boundary rules ([[../../surface-language/shell]]).

## Source and confidence

The design is absorbed from the pre-reboot shell-usage and toolchain design record, as a superset of its technical content.

**Its project-status framing is deliberately not carried**, per owner ruling.
The record carried a great deal of it — which programme this surface was the governing focus of, which decision superseded that role, which lettered build-track stage each pillar rode, and a bootstrap milestone framed around porting this project's own gate scripts.
None of that is design, all of it has moved at least once, and the corpus register states what the design _is_.
What is carried from that material is its technical residue: the capability requirements, their dependency order, and the observation that a mock handler replacing a real one is what proves the handler boundary.
The one status statement that survives is the one at the top: **of the faces designed here, only the script runner is built**.

**The as-built account is high confidence and was read from definitions**: the driver's argument surface, its outcome-to-status contract, and its deferred-face manifest; the renderer-firewall crate's two layers and its leaf position; the report envelope; and the origin map's cross-process-stable keys.
The driver's outcome-to-status contract was additionally exercised through the built binary, on the value, exit, blame, and refusal paths; its stuck and host-abort arms are read from the classifier and carry no binary witness.

**Two claims are marked rather than resolved.** The value-model ladder's landed rungs are stated from the corpus documents that own them rather than re-verified against the crates here.
And the layout-engine citation is registered with a resolvable identifier but its content was not checked against this document's use of it, which is the standing residual for this document.
