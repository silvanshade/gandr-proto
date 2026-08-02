# Entity attributes

An **entity attribute** is a named, typed datum attached to a declaration through the leading `@[…]` marker: a doc string, a deprecation note, a package coordinate, an editor hint, a code-generation request.

The layer exists because every area that wants such a place would otherwise invent its own.
Modules and packages want manifest metadata, builds want configuration, error rendering wants doc objects, the foreign interface wants capability gates, and code generation wants a request surface.
Five bespoke slots is five parsers, five typing paths, and five storage schemes; **one typed attribute layer is the alternative**, and this is it.

The load-bearing stance, stated once: **an attribute is data _about_ an entity, not a new kind of entity.** It rides the record former for its types, a side table keyed by the entity's identity for its storage, and the plain-data report projection for its query surface — three mechanisms that already exist, composed, with as little new machinery as the job admits.

## What is built

The layer is built and runs.
Every claim in this section names the module it was read from, and the corpus examples named below were executed rather than inspected.

* **The marker parses as a fresh single-character token.** `gandr-surface-grammar`'s `surface::term` module contributes an `attribute_block` rule at `Item` sort whose body is `@[ attr (, attr)* ]`, with `attr ::= identifier | identifier ( expression )`.
  Blocks stack: a `def` item takes any number of them, and so does a module member.
* **Attributes are collected for top-level items only.** `gandr-surface-engine`'s `lower` module records one raw attribute per marker entry, keyed to the item's index, in the top-level item loop.
  A module member's attribute block therefore **parses and is then dropped** — it is neither projected nor diagnosed ([[proposed/modules#module-question-04]]).
* **Eight schemas are registered, and every one is inert.** `gandr-surface-engine`'s `attributes` module holds the registry as a `const` table: `doc` and `deprecated` from the entity vocabulary, and `name`, `license`, `authors`, `package`, `dependency`, and `toolchain` from the manifest vocabulary, which shares this one registry rather than carrying its own.
  `dependency` is repeatable; the other seven are single-valued.
* **Payload typing is ordinary value typing, driven by the iterative typing machine.** The pass builds a checking state against the schema's value type and steps `gandr-core-checker`'s `machine` module to completion, so it never recurses on the host stack — the same driver the diagnostics and goals passes use.
* **Five diagnostics are realized**: an unknown name with a did-you-mean over the registry (a two-row Levenshtein program with a maximum distance of three), a repeated single-valued attribute, a bare marker where the schema needs a payload, a payload outside the **value fragment** — the literals, records, lists, and constructor applications a payload may be, as against a computation — and the ordinary type error of an ill-typed payload.
* **Storage never touches the item's term.** The pass returns a side table of resolved attributes plus the findings, and the lowered items are neither read nor mutated by it, so an item's content-address is unchanged by adding, editing, or removing an attribute.
* **The projection is an additive report field.** `gandr-surface-engine`'s `diag` module carries `attributes` as a field of the report envelope at schema version 2, with no version bump: a consumer that ignores it reads the report unchanged.
  Each row is a node id, a schema name, a payload, a tier, and a span.
  The payload is rendered by the module's deterministic `Debug` projection **pending the shared surface pretty-printer**, which is the one place the projection is not yet in its final form.
* **The projection includes a payload that fails to type.** A well-formed attachment whose payload is ill-typed still produces a row — the value is present, and a renderer shows it beside the diagnostic.
  An unknown, duplicate, missing, or non-value attribute is a diagnostic only.

Three things the design describes and this tree does **not** have.
A **misplaced**-attribute diagnostic is designed and unbuilt, and it is unreachable rather than merely unwritten: items are the only attribute target today, so no attribute can be applied to an entity kind its schema does not target.
An **expression-position** `@[…]` does not parse at all — the block rule is `Item`-sorted, so there is no reserved slot to decline from, only an ordinary parse failure.
And the **semantic tier** — the opt-in identity-bearing tier — exists as a schema field with no schema selecting it and no reflection behind it.

## The design space

Five axes fix the design, and each is decided below.

| axis                  | the question                                                 | decided as                                                                |
| --------------------- | ------------------------------------------------------------ | ------------------------------------------------------------------------- |
| **surface marker**    | how is an attribute written, and where may it appear?        | a leading `@[…]` block on items, plus a bracket slot inside `data` blocks |
| **payload semantics** | what is a payload, and how is it checked?                    | a schema-typed value, checked by the ordinary bidirectional checker       |
| **evaluation stance** | when, and with what power, does a payload evaluate?          | staged, value fragment only, effect-free **by locality**                  |
| **storage identity**  | where is an attribute stored, and does it change the entity? | a side table keyed by the entity's stable id; hash-neutral by default     |
| **query surface**     | how do tooling, the shell, and agents read attributes back?  | projected into the reified report, behind the renderer firewall           |

The last row's **renderer firewall** is the standing rule that a renderer reads the projected report and never re-resolves anything itself; the attribute field obeys it like every other field.

The through-line is **conservatism of identity**.
The default attribute is inert metadata that never changes what an entity _is_, and the escape hatch — an attribute that _does_ change identity — is explicit, declared per schema, and realized by reflecting the attribute into syntax.

## The marker

### One production, two positions

An attribute is a name, optionally applied to a payload.
The same production serves two syntactic positions.

```text
attr    ::= attr_name | attr_name "(" payload ")"
payload ::= value_expression        // the value fragment only
```

**Position one is the leading marker on a declaration item.** An attribute block `@[ attr (, attr)* ]` precedes a declaration and annotates it, and blocks stack, mirroring the way an item takes several independent annotations.

```text
@[doc("Squares its argument.")]
@[deprecated(#{ since = "0.2", note = "use `square_checked`" })]
def square(x: Integer) -> F Integer {
  ret (x * x)
}
```

**Position two is a per-symbol slot inside a `data` block**, where a bracket slot `[ attr (, attr)* ]` trails a declared symbol.

```text
data Expr {
  Lit(Integer)                 [ctor]
  Add(Expr, Expr)              [ctor, assoc, comm]
}
```

The slot omits the `@` sigil because its position — trailing a symbol declaration inside a `data` block — is already unambiguous.
The discipline it follows is the one a rewriting-logic system uses for its operator declarations, where a symbol carries its own attribute list in brackets: `op … : … -> … [ctor assoc comm prec 41] .` [@clavel-duran-eker-lincoln-martioliet-meseguer-talcott-2007-maude].

**Ownership is split, deliberately.** This document owns the `attr` production and the schema, typing, storage, and query semantics that both positions share; where the slot sits in the `data` grammar is the data-and-patterns design's business.
As built, the slot parses and is declined ([[declarations#data declarations]]), and it graduates when this layer's semantic tier lands ([[roadmap#Graduation rungs of the reserved forms]]).

### attr-decision-01

**The sigil is `@[…]`, and `#[…]` is the recorded reversal target.**

`@` is the decisive first token: it appears **nowhere else** in the surface — the world modality is spelled `at`, not `@` — so an attribute block is discriminated by a single token with zero lookahead, the strongest available form of the surface's never-unbounded-lookahead criterion ([[../surface-language#The design stance]]).
It is also the mainstream annotation marker across Java, Scala, Kotlin, Python decorators, and TypeScript, so it reads as "metadata about the following item" at the lowest unfamiliarity cost.

The case _against_ `#[…]` — Rust's marker, and the reason it was a candidate at all — is a **category** argument rather than a taste one.
The `#` prefix is already spent on two forms that both denote _values or embedded sub-grammars_: `#{ … }` is a record value or type, and `#!{ … }` and `$!{ … }` embed the shell sub-grammar ([[shell]]).
An attribute is neither.
It is metadata _about_ the following item, a different category, so `#[ … ]` would risk reading as a `#`-family data literal — a hash-bracket set or array, by analogy to `#{ … }` — rather than as an annotation.
Keeping `#` for value and reader forms and `@` for annotations makes the sigil carry the category distinction.

The one real cost of declining `#[…]` is that a code-generation request is spelled `#[derive(…)]` in Rust; `@[derive(…)]` is a one-character transposition and loses nothing but the exact pixels.

_Status:_ adopted, with a recorded disagreement.
If the data-declaration design or a later coherence pass concludes that the `#` family should absorb attributes for a uniform compile-directive reader story, `#[…]` is the reversal target, and the semantics below are sigil-independent and transfer unchanged.

### Grammar-gate compliance

The marker is designed to hold the surface's grammar gates — zero declared conflicts, no dynamic precedence, no external scanner, and the table size and state budgets ([[grammar#The three build-time gates]]) — for three reasons.

**`@` is a fresh single-character token, legal only where an item may begin**, and after `@` a `[` deterministically opens the block.
That is first-token discrimination on the standard machine, with no lookahead.

**`]` is unique-in-context as the block's closing delimiter**, so it is an error-recovery synchronization point: a malformed attribute block resynchronizes at `]` instead of swallowing the item that follows.

**Attribute blocks are keyword-and-delimiter-shaped items**, which is the cheap kind for table size, so the parser budget is not at risk.

**The claim is a design expectation, and the gate is the measurement.** The pre-reboot record carried a state-count delta measured against a generated tree-sitter parser; that package is not in this tree, so the number is not carried.
What is carried is the shape of the check: the binding evidence is the grammar gate on the regenerated parser, and if a budget is exceeded the surface is simplified until it holds.

## Typed schemas

### The registry

An attribute name resolves, in a dedicated **attribute registry**, to a **schema**: a value type — a record type `#{ℓ : A}`, a base scalar, a list type, or a declared data type.
The payload is checked against that type by the **ordinary bidirectional checker**, in check mode.

**No new typing rule is introduced.** A payload is a value, and the record, scalar, list, and data rules already type it.
That is the whole of the semantics, and it is why the layer costs a registry and a checker call rather than a type system extension.

The registry binds names to schemas the way the prelude binds native builtins to their types.
As built it is a flat, global `const` table shared by the entity and manifest vocabularies:

| attribute    | schema                                                    | arity      | tier  |
| ------------ | --------------------------------------------------------- | ---------- | ----- |
| `doc`        | `String`                                                  | single     | inert |
| `deprecated` | `#{ since : String, note : String }`                      | single     | inert |
| `name`       | `String`                                                  | single     | inert |
| `license`    | `String`                                                  | single     | inert |
| `authors`    | `[String]`                                                | single     | inert |
| `package`    | `#{ name : String, version : String }`                    | single     | inert |
| `dependency` | `#{ name : String, alias : String, constraint : String }` | repeatable | inert |
| `toolchain`  | `#{ gandr : String }`                                     | single     | inert |

The last six are the manifest vocabulary, and the reason they live in the entity registry rather than beside a manifest parser is the whole point of the layer: a manifest is not a file format, it is typed data on a declaration.

User-declared attribute schemas are the growth path, and they reopen the namespacing question ([[#attr-question-01]]).

### Diagnostics

A schema declares an **arity** — single-valued or repeatable — and targets a set of entity kinds.
Six diagnostics follow, and each localizes to the attribute node through the provenance the pipeline already records, so they render through the ordinary error surface rather than through anything attribute-specific.

| diagnostic          | fires when                                                            | built |
| ------------------- | --------------------------------------------------------------------- | ----- |
| unknown attribute   | the name resolves to no registry entry; carries a did-you-mean        | yes   |
| duplicate attribute | a single-valued attribute is repeated on one entity                   | yes   |
| missing payload     | a schema that requires a payload is written as a bare marker          | yes   |
| non-value payload   | the payload is a computation rather than a value                      | yes   |
| ill-typed payload   | the payload fails to check against the schema                         | yes   |
| misplaced attribute | the attribute targets an entity kind its schema does not              | no    |

The last two are worth separating, because they fail differently.
**An ill-typed payload needs no attribute-specific machinery to explain it** — it is the ordinary type error of the record, scalar, and list rules, surfaced at the payload node, and that is a design property rather than an accident of implementation.
**A misplaced attribute is unreachable today**, since items are the only target; it becomes reachable when the `data`-block slot admits non-item targets.

The two diagnostics the design did not anticipate and the value-fragment stance forced are the **missing payload** and the **non-value payload**.
The second is the executable form of the next section's claim.

### attr-decision-02

**A payload is data, not computation, and its purity is a consequence of _locality_ rather than a side condition.**

Concretely, a payload is restricted to the **value fragment** — literals, records, lists, constructor applications, and unit — and, reserved for growth, to **quoted syntax** for code-carrying attributes.
It is never an arbitrary computation.

Payloads are checked, and for a semantic attribute reduced, at a distinguished **attribute phase** whose located context carries an **empty runtime context**: no endpoints, no capabilities.
So a payload _cannot_ consume a runtime endpoint or perform a host or session effect, because those located hypotheses are simply not in scope at that phase.
Nothing forbids the effect; there is nothing to perform it with.

Two consequences follow and both are load-bearing.
**Attribute resolution is total and effect-free**, so it can run in the checker and pipeline, where there is no runtime at all.
And its result is a **stable, cacheable datum**, which is what lets an attribute survive incremental reuse.

The executable witness is `crates/surface-corpus`'s `examples/pathological/attributes/effectful-payload.gandr`, which puts a shell block in a `doc` payload and pins its rejection by the value-fragment discipline — before any effect could run.

Code-carrying attributes are where quoted syntax enters: a code-generation request carries _syntax_, not values, and the phasing machinery is what types and stages it.
The current registry restricts payloads to the value fragment and reserves the quoted-syntax payload, so no built schema needs the phase tower.

_Status:_ adopted and built for the value fragment; the quoted-syntax payload is reserved and unbuilt.

## Storage

### Attachment by stable id

An attribute attaches to the **stable identity** of the entity it annotates, in a side table mapping that identity to a list of resolved attributes — the same shape as the marks, origin, and diagnostics side tables, serialized alongside checkpoints keyed by the same identities, so attributes survive incremental reuse without any new identity mechanism.

**As built, that identity is the item's index in the lowered item list**, which is the same key the report's goal, diagnostic, and mark projections already localize by.
The arena-of-node-id re-key the design specifies is a **lossless swap, not a redesign**: the side table's shape, its serialization, and the projection all key on whatever the pipeline's item identity is, and only that definition changes.

### attr-decision-03

**An attribute never perturbs an entity's syntax identity unless it is explicitly modeled as syntax.**

This is the invariant the layer exists to keep precise, and it is a two-tier rule rather than a slogan.

**Inert is the default tier.** An inert attribute is metadata that does not change what the entity _is_: a doc string, a deprecation note, a source-location hint, an editor or agent annotation, a package coordinate.
It lives **only** in the side table.
The entity's content-address is computed over its core-IR syntax and is **unchanged** by adding, removing, or editing an inert attribute, so two entities identical in syntax but differing in their inert attributes have the **same** content hash.

**Semantic is the opt-in tier.** A semantic attribute is one whose meaning is a _transformation_ of the entity: an operator fixity that changes resolution, a code-generation request that generates terms, a foreign link or capability gate that changes what type-checks.
Two entities that differ in such an attribute are genuinely different entities, so it **must** participate in identity — and it does so by being **reflected into the core IR**, lowered to a term or recorded as a canonical elaboration input, and therefore entering the content hash _through syntax_.

The tier is a per-schema declaration, and that is what makes the content-address function's contract exact: **it reads the core-IR term plus the semantic-attribute reflections, and never reads the inert side table.**

_Status:_ the inert tier is adopted and built, so the content-address function is unchanged today.
The semantic tier is designed and unbuilt: the tier field exists in the schema table, no schema selects it, and no reflection stands behind it.
Its shape is [[#attr-question-03]].

### The entity-component graduation path

The side table is an entity-component store in embryo: the stable identity is an **entity** id, and each schema is a **component** type.
The graduation is to generalize the flat table into a **columnar** store — one column per schema — over which the attribute queries below become component queries and joins.

The eventual backend is the project's own storage substrate: a content-addressed component store, with provenance carried beside it.
Kept substrate-agnostic now, that is a backend swap rather than a redesign.

This is reserved, not built; what exists is the flat table.

## The query surface

Tooling, the shell, and agents read attributes **only** through the reified report — the plain-data projection that already carries diagnostics, goals, marks, and obligations.

```text
attributes : [ { node : NodeId, schema : SchemaRef, payload : Value, tier : (inert | semantic) } ]
```

Renderers — an editor hover, the interactive surface, an agent stream, a shell command that lists an item's attributes — read that field and render it.
**They parse nothing, lower nothing, type nothing.** If a renderer needs attribute information the report does not carry, the fix goes into the pipeline so that _every_ renderer gains it; that is the firewall rule, unchanged for this consumer.

The query shape is read-only and entity-component flavoured: **by node** and **by schema**.
Richer joins — "every deprecated public item", a doc-against-signature table an agent wants — are the graduation above; what is exposed is by-node and by-schema.

The executable image of the firewall is `crates/surface-corpus`'s `examples/model/attributes/query-via-report.gandr`, whose harness assertion reads the projected row rather than re-resolving the attribute.

## The cut and the growth path

The cut is chosen so the mechanism is provable end to end on the smallest surface, with every consumer's hook designed in and unbuilt rather than retrofitted later.

| concern   | built                                                 | designed, unbuilt                                                            |
| --------- | ----------------------------------------------------- | ---------------------------------------------------------------------------- |
| surface   | leading `@[ attr, … ]` on `def` items                 | the `data`-block per-symbol slot; expression-position blocks                 |
| registry  | eight built-in inert schemas in one flat global table | user-declared schemas, coordinated with data declarations and modules        |
| payload   | the value fragment                                    | quoted-syntax payloads for code generation and notation                      |
| tier      | inert only, hash-neutral                              | the semantic tier and its modeled-as-syntax hashing boundary                 |
| storage   | a flat side table keyed by item index                 | the arena-of-node-id re-key; the columnar graduation and its storage backend |
| query     | the report field, by node and by schema               | joins; agent-facing attribute queries                                        |
| consumers | the manifest vocabulary                               | build configuration, doc objects, imported documentation, code generation    |

The one row that has moved since the design was written is **consumers**: the design's cut had none, and the manifest vocabulary is now registered and checked, which makes packaging the layer's first real consumer ([[#Consumers]]).

## Consumers

Each of these is a hook this design must not foreclose, and most are consumers of it.

* **Data declarations and patterns** — the per-symbol in-block slot is a position of _this_ document's `attr` production, owned by the data design; constructor attributes such as a constructor marker, a fixity, or a reserved grade slot are its first users.
* **Modules and packaging** — module and package metadata is typed attribute data on a unit's root declaration, and the manifest **consumes** the attachment mechanism rather than inventing a format.
  A package coordinate is an inert attribute, and the content-addressed cache hashes the unit's _syntax_, so inert package metadata is hash-neutral and the manifest does not perturb the address.
  The manifest's intended host — a module root that takes a leading attribute block — does not exist, so the schemas validate on a top-level definition as the unit-root stand-in ([[proposed/modules#module-question-04]]).
* **Build configuration** — build-target configuration is attribute data on a build target: the same typed-schema mechanism, a different entity, a different registry, so packaging and build configuration do not collide.
* **Readable errors** — the attribute diagnostics render through the same adapter as every other diagnostic, and attribute provenance rides the origin and elaboration-kind machinery.
* **The foreign interface** — an `extern` block's link target, capability, and abort-on-unwind policy change what type-checks and how a symbol is bound, so they are **semantic** attributes: modeled as syntax and hash-participating.
  The member-attribute surface is currently deferred, and the reason is a surface collision rather than a semantic one — the bare `@` spelling the member form wanted conflicts with the `@[…]` block discipline ([[roadmap#Deferred-with-reasons, collected]], [[../implementation/foreign-interface]]).
* **Code generation** — the natural first _semantic_ consumer: a generating attribute carries quoted syntax, rides the elaborator interface, and records its elaboration inputs in the dependency footprint so it stays checkpoint-sound.
* **Operators and notation** — operator fixity and precedence are semantic per-symbol attributes, hash-participating, and they realize one shape of the operator footprint the fixity declaration already reserves ([[declarations#Operator-fixity declarations]]).
* **Doc objects and imported documentation** — `doc` is the doc-object's home, and importing documentation from a foreign toolchain maps its comments onto `doc` attributes on the imported entities.
* **Value semantics** — the functional record-update stance informs an eventual attribute-editing surface ([[value-semantics#Functional record update]]); minor, and off the built path.

## The corpus examples

The layer's corpus treatment is split between a **model** subtree that teaches the feature and a **pathological** subtree where each example pins a diagnostic or an invariant.

Built and passing, under `crates/surface-corpus/examples`:

| example                                       | what it pins                                                         |
| --------------------------------------------- | -------------------------------------------------------------------- |
| `model/attributes/doc-and-deprecated`         | the leading marker, a scalar payload, a record payload, and stacking |
| `model/attributes/module-metadata`            | the manifest vocabulary on a unit root, with its inert coordinates   |
| `model/attributes/query-via-report`           | the projected row a renderer reads — the firewall, executable        |
| `pathological/attributes/unknown-attribute`   | the unknown name and its did-you-mean                                |
| `pathological/attributes/ill-typed-payload`   | the ordinary type error at the payload node                          |
| `pathological/attributes/missing-payload`     | a bare marker where the schema needs a payload                       |
| `pathological/attributes/duplicate-attribute` | the repeat of a single-valued attribute                              |
| `pathological/attributes/effectful-payload`   | rejection by locality — attribute purity, executable                 |

Planned and **not** built, each with the milestone that activates it:

* a **declared-record-schema payload** example, showing that attribute typing _is_ ordinary value typing field by field — writable today, and the only planned model example with no gate in front of it;
* a **hash-neutrality** example asserting that two entities differing only in an inert attribute hash equal while a semantic difference hashes distinct — gated on the semantic tier, since half of it is unstateable without one;
* a **semantic-attribute** example carrying a code-generation request — gated on the same tier;
* a **misplaced-attribute** example — gated on the `data`-block slot admitting non-item targets, which is what makes the diagnostic reachable;
* an **expression-position** example pinning a reserved-not-implemented parse diagnostic — gated on that slot being _reserved_, which it currently is not: the block rule is `Item`-sorted, so the position fails to parse rather than declining.

## Why this shape

Four properties are what the layer is for, and they are worth stating positively because each is a thing that would otherwise be paid for repeatedly.

**Maximal reuse.** Schemas ride the record former, storage rides the stable-id side tables, and the query rides the renderer firewall, so the layer adds a marker, a checker path, and a report field — nothing structural.

**One layer, several consumers.** Manifest metadata, build configuration, doc objects, imported documentation, and code generation all attach to the same typed mechanism, so the language does not accrete a bespoke slot per consumer.

**A checkable identity rule.** Inert never hashes; semantic hashes via syntax.
That is precise enough to be tested rather than merely asserted, and the hash-neutrality example is the test.

**A cheap and unambiguous surface.** `@` is the strongest first-token discriminator available and the mainstream annotation sigil, so the marker is both cheap for the parser and familiar to a reader.

### The risk, and it is one risk

**The inert-semantic boundary is the whole design's exposure.** It is clean while the layer is inert-only, and the _first_ semantic attribute forces the modeled-as-syntax reflection to be built **and** proven hash-correct against the two instruments that would catch it being wrong: the **differential** oracle, which compares an incremental run against a from-scratch one, and the **checkpoint** oracle, which replays a serialized state and expects the same answer.
If that reflection turns out not to compose with the content-address function, **the boundary moves rather than merely extends**.
That is real work the built layer only reserves, and it is the thing to watch when the semantic tier is scheduled.

Two smaller costs are worth naming honestly.
Two positions, and later two tiers, are two axes of "one mechanism, several shapes", and each is a place a future reader can be surprised; the mitigation is that exactly one position and one tier are built.
And declining `#[…]` spends a sliver of muscle memory on the most famous attribute feature in a neighbouring language — churn, if the data design later wants `#`-family coherence.

### The named dead-ends

Three alternatives were rejected, each with the reason that binds.

**A payload as arbitrary computation.** Rejected: it would make attribute resolution effectful and non-total, which breaks cacheability and the phase discipline together.
The reversal condition is a use case that needs a payload to observe something only a computation can reach, and that case would have to price the loss of both properties.

**Attributes as extra fields on the entity's type**, an intersection-style encoding.
Rejected on polarity grounds: intersection is negative-only and models overloading, not attachment, which is the same reason the intersection record was refused as a record former.

**A bespoke per-consumer manifest format.** Rejected: it is precisely the fragmentation this layer exists to prevent, and it would need its own parser, typing path, and storage, none of which the attribute mechanism charges for.

## Open questions

### attr-question-01

**Is the attribute registry a flat global namespace, or is it module-scoped?**

The built registry is a flat global `const` table, which is the right answer for a fixed built-in vocabulary and no answer at all for user-declared schemas.
Once a user may declare a schema, the question is whether an attribute name is imported like a value.
It couples to the data-declaration surface and to the manifest vocabulary, which is the one consumer already large enough to want its own namespace.

_Disposition:_ **carried** — open, and it gates user-declared schemas.

### attr-question-02

**How deep does a code-generation attribute's quoting go?**

Does such an attribute carry type-class _names_ — a shallow quote — or full syntax trees, and how does the choice interact with the **phase index**, the natural number recording how many stages of quotation a piece of syntax sits under?
The answer decides whether the quoted-syntax payload needs the phase tower or only a name list.

_Disposition:_ **carried** — open, and owned jointly with the code-generation lane and the metaprogramming stages.

### attr-question-03

**When a semantic attribute is "reflected into the core IR", what exactly is reflected?**

Two shapes are available: a term the entity _contains_, or a canonical elaboration-input record hashed _beside_ the term.
The constraint that decides it is that the differential and checkpoint oracles must be able to see the reflection; the exact shape is a construction question for the semantic-tier milestone.

_Disposition:_ **carried** — open, and it is the construction half of the risk named above.

### attr-question-04

**Does editing an inert attribute invalidate the entity's typing checkpoint?**

It must not: an inert attribute is hash-neutral, so re-typing on an inert edit is work done for nothing.
But tooling reading the projected attribute field _does_ need refreshing, so the pipeline must route an inert-attribute edit to the projection without re-typing.

_Disposition:_ **carried** — open, and it is checkable against the incremental reuse keys rather than a design choice.

### attr-question-05

**Is the sigil settled, or still triggered?**

[[#attr-decision-01]] records `@[…]` as adopted and `#[…]` as a reversal target with a stated trigger.
The trigger has not fired, and the owner may prefer to settle the question outright rather than leave a reversal standing.

_Disposition:_ **carried** — open as a decision to be taken, not as a design gap; the semantics are sigil-independent either way.

## Source and confidence

This document is written against the pre-reboot entity-attributes design record including its as-built section; the prior programme's tracker rows for the design pass and the implementation; and the tree — `gandr-surface-grammar`'s `surface::term` module for the marker, `gandr-surface-engine`'s `attributes`, `lower`, and `diag` modules for the registry, collection, checking, storage, and projection, and `crates/surface-corpus` for the runnable examples.

**Confidence is high on the built account.** Every as-built claim names its module, the registry was read from the table rather than from prose, and the corpus examples were executed.
**Confidence is medium on the reserved tiers** — the semantic tier, the quoted-syntax payload, and the columnar graduation are transferred from the design record and have no implementation to check against.
**Confidence is medium on the single literature attribution**: the identifier is recorded, and the operator-attribute discipline attributed to it was checked against the design record's use of it rather than against the work.

Where the design record and the tree disagree, the tree wins on status and the record wins on payload, and both readings are stated at the claim.
