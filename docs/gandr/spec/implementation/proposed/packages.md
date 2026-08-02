# The package and build manager

**Proposed.
No package manager, resolver, cache, or lockfile exists in this tree.** What exists is a content-addressed storage tier serving a different consumer, and a manifest vocabulary that is registered and type-checked but has no resolution behind it.

The design fixes how a gandr program **names, fetches, verifies, ascribes, caches, and distributes** the units it is built from, and how a build is orchestrated over them.
Its distinguishing commitment is that the manager introduces **no semantics of its own**: an import lowers to an ordinary module value, an identity is a content address, and version compatibility is a typed question rather than a heuristic one.

## What is built, and what this document describes

**Built, and verified against the tree at write time.**

* **The manifest vocabulary is registered and checked.** `gandr-surface-engine`'s `attributes` module registers `package`, `dependency`, `toolchain`, `name`, `license`, and `authors` as inert attribute schemas in the same registry as `doc` and `deprecated`, typed by the same checker path ([[../../surface-language/attributes#The registry]]).
  `dependency` is repeatable, one per dependency; the rest are single-valued.
  The runnable example is `crates/surface-corpus`'s `examples/model/attributes/module-metadata.gandr`.
* **The manifest has no unit root to attach to.** A module declaration takes no leading attribute block, so the schemas validate on a top-level definition as the unit-root stand-in ([[../../surface-language/proposed/modules#module-question-04]]).
* **A content-addressed store exists, and it serves a different consumer.** `gandr-storage-prolly-trees` is an ordered-record Merkle search tree whose node identity is a BLAKE3 hash of the encoded node, with membership, non-membership, and range proofs and a block-store interface; `gandr-storage-chunker` is a deterministic record-safe boundary detector with an 85-byte parameter commitment; `gandr-storage-artifact` binds the two into an artifact identity.
  **Their consumer is the kernel's export artifacts, not build units** ([[../../implementation#Storage — content addressing, canonicalize-before-address]]).
* **There is no persistent backend.** The block store that exists is in-memory, and the absence of a persistent one is a self-declared deficit of the storage tier rather than an oversight.
* **`import "URI" as name ;` parses and is never lowered.** No resolver consumes it ([[../../surface-language/proposed/modules#What is built, and what this document describes]]).

**Designed, and not built.** Everything else in this document: the import lowering, the address function over a unit's canonicalized inputs, the cache and hydration, endpoint distribution, typed ascription of a fetched unit, the build graph, distributed builds, the lock record, and the manifest's identity-bearing fields.

**A naming collision that already exists in this tree, and it will mislead a reader who does not know about it.** The word _manifest_ now denotes three unrelated things: the **package manifest** this document specifies; the **artifact manifest** of the storage tier, whose hash binds a chunker parameter commitment, a record count, a root hash, and an inner format version; and the **corpus manifest** that registers the specification documents themselves and is watched by a drift gate.
None of the three is a generalization of another.
This document says _manifest_ for the first sense only and qualifies the other two wherever they appear.

## The two senses of "package"

The word carries two distinct senses, and this document owns exactly one.

| sense                           | what it is                                                                                                        | home                                                             |
| ------------------------------- | ----------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------- |
| **first-class module value**    | `Package σ ≜ ∃β̄. U_r (F σ)` — a runtime value carrying a sealed, thunked module, packed and unpacked in a program | [[../../surface-language/proposed/modules#First-class packages]] |
| **build and distribution unit** | the addressable artifact a build resolves, fetches, verifies, and links                                           | this document                                                    |

The first is a **language** construct: a value in the core calculus.
The second is a **toolchain** construct: an artifact identity plus the resolution and distribution machinery around it.

Where this document says "package" unqualified it means the **build and distribution unit**, and the corresponding qualification holds in the other direction over there.
Recording the collision in both places is deliberate: an unqualified use in a quotation carried somewhere else is exactly how the two get conflated.

## The import surface and its lowering

A source unit names its dependencies through an **import surface** that lowers to ordinary module values, so imports add no new core construct.

* The import surface is a **declaration form** naming an external build unit and binding it to a local name.
* Lowering resolves each import to a **first-class module value** and threads it into scope — either as a package the body unpacks, or as a directly bound structure.
* Resolution is **content-addressed**: an import names a unit by a content address, optionally through a human-facing alias that a lockfile pins to that address.
* The surface stays **layout-insignificant and grammar-local**, consistent with the rest of the surface ([[../../surface-language#The design stance]]); it adds a surface form and no core-IR construct.

**The point of lowering to module values is that the package manager introduces no semantics of its own.** Once an import is resolved, the body sees an ordinary gandr module, type-checked by the existing rules.
Nothing downstream of resolution knows that a module arrived through an import.

### pkg-decision-01

**Resolution lives outside the trusted core, and a hermetic fixture resolver comes before any network behaviour.**

The resolver supplies source and module data to the elaborator; the elaborator checks the declared signature.
Package identity, path and error diagnostics, and deterministic resolution are specified **before** registry or network behaviour is added, and the acceptance instrument is a fixture resolver that imports a package as a typed module **with no network access at all** — successful selection evaluating through the shared linker, and a missing package or a signature mismatch producing structured diagnostics.

_Status:_ adopted as a binding constraint on the construction, and unbuilt.
It is the toolchain half of [[../../surface-language/proposed/modules#module-question-03]], and the reason it is stated as a decision rather than a preference is that the ordering is what makes the manager auditable: a resolver that reached the network before its identity story was fixed would be untestable at exactly the point where reproducibility is the product.

## Content-addressed identity, cache, and hydration

Build units are identified by **content address** — canonicalize, then hash, with BLAKE3 — which is the discipline the storage tier already runs for its own consumer, down to the ordering of the two steps ([[../../implementation#Storage — content addressing, canonicalize-before-address]]).

**Identity.** A unit's address is a hash over its **canonicalized inputs**: its source, its own resolved dependency addresses, and the relevant toolchain and version metadata.
The address is therefore reproducible, and a dependency edge is an **address** edge — a Merkle graph, not a name-and-hope reference.

**Cache.** Resolved units, checked core IR, and build outputs are stored keyed by content address.
A rebuild is a cache lookup, and equal inputs share one entry.

**Hydration.** A build **hydrates** the closure of addresses it needs, fetching only the missing blocks from a cache or a remote endpoint, rather than materializing a full dependency tree eagerly.

### pkg-decision-02

**The cache is authored behind a store trait so its backend is a swap rather than a rewrite.**

The design was written when no persistent traversable block store existed, and it declined to wait for one: the cache sits behind a store abstraction with a filesystem or embedded-database implementation now, so that re-hosting it on a content-addressed store later is a lossless swap.

_Status:_ adopted, and the ground under it has shifted in the design's favour.
A content-addressed block store **does** now exist in this tree, with proofs and a parameter-commitment discipline — but it is in-memory, and its consumer is kernel export artifacts.
So the store trait is still the right shape, and the candidate implementation behind it is closer than the design assumed.
Whether the storage tier's tree is the right substrate for a build cache, rather than merely an available one, is [[#pkg-question-04]].

**This is the seam where the package facet meets the compilation facet.** The compilation backend's artifact-identity constraints and this cache are **one identity discipline, not two**: a checked-core-IR address is simultaneously the cache key and the compiler's reproducible-artifact identity.
The compilation backend's own design record has not been absorbed into this corpus, so that constraint is stated here as the seam it is and not as a summary of a document a reader cannot open.

## Distribution over endpoint coordinates

Distribution uses **endpoint coordinates**: a remote build unit is reachable by its content address plus a coordinate that locates a provider able to serve the missing blocks.
The design names a specific peer-to-peer endpoint system for this role; that is a **vendor selection carried from the design record**, and this corpus holds no locator for it and no in-tree dependency on it.

**Address against location, and they are orthogonal.** The content address is _what_ — an identity, verified on receipt.
The endpoint is _where_ — a provider to hydrate from.
The same unit can therefore be served from many endpoints, and a received block is **always re-verified against its address**.

**Peer-to-peer hydration follows.** Because units are content-addressed, distribution is direct block transfer between endpoints: there is **no central registry acting as a trust root**.
Trust is in the hash; availability is in the endpoints.

**The worlds seam.** Endpoint coordinates align with the language's world and capability direction: _where_ a unit may be fetched or run is capability-gated, and an endpoint coordinate is the toolchain-level analogue of a world handle ([[../../surface-language/proposed/modules#Worlds and distribution]]).
This is a construction obligation, not a built connection, and the capability model it would gate through is itself a design ([[../capability-model]]).

## Typed ascription is the version-compatibility check

**A fetched unit is not trusted by name.** It is checked against an expected signature using the module system's ascription machinery.

* An import may carry an **expected signature** `σ`, and the resolved module is ascribed to it by the existing rules — transparent ascription preserving identities, or opaque sealing hiding them ([[../../surface-language/proposed/modules#The typing rules]]).
* A first-class package fetched at build or load time is **unpacked with a dynamic signature match**, so a version whose actual signature does not satisfy the expected `σ` fails **at the boundary, before its body runs**.
* Therefore the manager's version-compatibility question is a **typed** question: "does this address satisfy the signature this import expects" is signature matching, not a version-number heuristic.

Version aliases may still pin addresses in a lockfile, and they are a convenience for humans.
**The load-bearing check is ascription**, and the alias is not evidence about anything.

## Distributed builds

The build is orchestrated over the content-addressed graph and may be distributed across endpoints.

**The build graph** is a directed acyclic graph whose nodes are addressed units and whose edges are resolved dependencies.
A node is rebuilt only when its address is absent from the cache.

**Distribution is a consequence, not an addition.** Because a node's inputs and outputs are content-addressed and reproducible, a node may be built remotely and its output hydrated back: the same identity discipline that makes the cache sound makes a remote build output trustable on receipt.

**Reproducibility is what pays for both.** Deterministic inputs plus content-addressed outputs give the reproducibility the compilation backend requires **without the build manager needing to trust any particular builder** — which is the property that makes remote building acceptable rather than merely possible.

## The staged cut

The direction is decided; the construction is staged, and only the first slice is near-term.

| slice        | what it contains                                                                                                                                                                             | what gates it                                                                          |
| ------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| **first**    | a minimal **local** package-import path: the import surface lowering to module values, a local content-addressed cache behind the store trait, and typed ascription against the module layer | the module rung above the built record modules, and the toolchain surface it runs from |
| **advanced** | endpoint distribution, peer-to-peer hydration, distributed builds, the re-host of the cache onto a persistent content-addressed store, and capability gating of endpoints                    | additionally: a persistent store, and the compilation backend's artifact-identity work |

The first slice is deliberately a **single-machine dependency story**, and its whole job is to prove two things: that imports lower to modules, and that units are cache-keyed by address.
No distribution, no distributed builds.

## The manifest

The machinery above fixes resolution, cache, distribution, and ascription, and leaves one gap: **a unit carries no place for user-facing metadata about itself** — its name, its exposed version aliases, its dependencies' coordinates, the signature it advertises, the capabilities it needs, the toolchain it wants.

The load-bearing stance, stated once: **a manifest is not a new file format.
It is typed gandr data attached to the unit's root declaration through the entity-attribute layer** ([[../../surface-language/attributes]]).

The manifest therefore inherits, for free, three things it would otherwise have had to build: a typed payload checked by the ordinary bidirectional checker, storage keyed by a stable entity identity, and read-back through the plain-data report projection.

The one design obligation **unique to packaging** is the **content-address boundary**: which manifest fields may, and may not, perturb the unit's address — because that address is the cache key, the lockfile pin, and the compiler's reproducible-artifact identity all at once.

### The design space

Four axes fix the manifest, and each is decided below.

| axis                    | the question                                                                            | decided as                                                                                       |
| ----------------------- | --------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| **attachment site**     | where does the manifest live — root attributes, a manifest module, or an external file? | leading `@[…]` attributes on the unit's root declaration                                         |
| **field identity role** | does a field change what the unit **is**, or only describe it?                          | a per-field taxonomy: inert, participates-via-resolution, participates-via-syntax, location hint |
| **registry scope**      | are manifest attribute names built-in globals, or user- and module-scoped?              | built-in prelude-bound globals for the first slice; user schemas are growth                      |
| **staged cut**          | what ships first, and what is designed in but unbuilt?                                  | inert descriptive and coordinate fields now; identity-bearing fields gate on the semantic tier   |

### pkg-decision-03

**The manifest is written as leading attribute blocks on the unit's root declaration, not as a dedicated format.**

```text
@[package(#{ name = "acme/parser", version = "1.4.0" })]
@[dependency(#{ name = "acme/lexer", alias = "lexer", constraint = "^2.1" })]
@[toolchain(#{ gandr = ">=0.9" })]
module Parser { … }
```

Two alternatives are declined for the first slice.
A **dedicated manifest module** — a source file whose body is one record value — and an **external manifest file** are each exactly the per-consumer bespoke slot the attribute layer exists to prevent: each would need its own parser, its own typing path, and its own storage, none of which the attribute mechanism charges for ([[../../surface-language/attributes#The named dead-ends]]).

Hosting on attributes also puts the manifest **inside the source unit whose syntax is content-addressed**, so the manifest and the code it describes share one identity discipline rather than two.

**The host does not exist yet, and the example above does not parse in this tree.** A module declaration takes no leading attribute block, so the manifest's attachment point is a top-level definition, and the module root remains the intended host ([[../../surface-language/proposed/modules#module-question-04]]).
For a build unit whose root is a non-gandr input with no in-language declaration at all, the external-file manifest is the **reserved reversal target** rather than a dead end ([[#pkg-question-05]]).

_Status:_ adopted for gandr-rooted units and built as far as its schemas; the attachment site is unbuilt.

### The field-role taxonomy

Every manifest field is classified by its **identity role** — whether it changes what the unit is — because that classification, not the field's surface, decides content-address participation.

| field                     | schema                                                        | identity role                                                               | slice    |
| ------------------------- | ------------------------------------------------------------- | --------------------------------------------------------------------------- | -------- |
| `name`                    | `String`                                                      | inert                                                                       | first    |
| version aliases           | a list of alias records                                       | inert coordinate; the **resolved** pin participates via the lock record     | first    |
| dependency coordinates    | `[ #{ name : String, alias : String, constraint : String } ]` | inert coordinate; the **resolved address** participates via the lock record | first    |
| exposed signature pointer | a reference to a signature `σ`                                | **participates via syntax** — sealing is generative                         | growth   |
| required capabilities     | an effect and capability row                                  | **participates via syntax** — a gate on what type-checks                    | growth   |
| toolchain constraints     | `#{ gandr : String, … }`                                      | participates via the toolchain-metadata hash input                          | growth   |
| `doc`, `license`, authors | `String` and `[String]`                                       | inert                                                                       | first    |
| endpoint hint             | a coordinate `String`                                         | inert **location** hint, never identity                                     | advanced |

The taxonomy has three participation classes, matched to the attribute layer's tiers.

**Inert** covers the descriptive fields, the location hints, and the human-facing dependency and version **coordinates as written**.
These live only in the side table, so two units identical in source but differing in an inert field have the **same** content address.

**Participates via resolution** covers the **resolved** dependency addresses and version pins.
The coordinate is inert; resolution turns it into an address that enters the hash through the lock record, **not through the coordinate's own bytes**.

**Participates via syntax** covers the exposed-signature pointer and the required-capabilities gate.
These change what the unit type-checks _as_, so they are semantic attributes — modeled as syntax and hash-participating — and they gate on the semantic tier, which is designed and unbuilt ([[../../surface-language/attributes#attr-decision-03]]).

### pkg-decision-04

**The manifest attribute is hash-neutral: its bytes never perturb the unit's content address.
Build metadata nonetheless participates in the hash, through three channels that do not run through the manifest attribute.**

Both halves are true at once, and holding them together is the whole content-address boundary.

1. **Resolved dependency addresses** enter through the lock and hydration record, which is a hash input.
   Editing a coordinate changes the address only by changing _what resolution pins_, never by the coordinate's own bytes.
2. **Toolchain and version metadata** enters through the toolchain-metadata hash input directly.
3. **The two genuinely identity-bearing fields** — the exposed signature and the capability gate — participate by being **modeled as syntax**, which is the attribute layer's own semantic-tier carve-out, and they gate on that tier landing.

**This refines the attribute layer's advice rather than overturning it.** That layer's guidance was to choose the inert tier for all module, package, and build metadata and confirm hash-neutrality.
This design **exercises the layer's own semantic carve-out** for the two fields that must change what the unit is, flagging them semantic exactly as the layer instructs such fields be flagged.
The manifest attribute stays inert and hash-neutral; identity-bearing participation is routed to the three channels above, so the manifest never has to be "a semantic attribute" as a whole.

_Status:_ adopted; the inert half is built, and all three participation channels are unbuilt.

### How the manifest feeds the toolchain

The manifest is the **input** to the machinery fixed above.
It introduces no new resolution semantics of its own, and that is the property to preserve.

**Endpoint coordinates.** An optional per-dependency endpoint hint records _where_ to hydrate from.
By the address-against-location orthogonality it is **never** part of identity, so it is inert and two units differing only in endpoint hints hash equal.

**The lock and hydration record.** Resolution consumes the manifest's dependency coordinates and version-alias constraints and produces the lock record — a mapping from alias to resolved address — whose resolved addresses are the hash inputs.
A version alias is a human-facing name that the lock pins to a content address: **the alias string is inert, and the pin participates**.

**Typed ascription.** The exposed-signature field is the interface the unit **advertises**; a consumer's import may carry an expected `σ`, checked against the resolved unit by the existing rules.
So "is this version compatible" is signature matching, and semver aliases pin addresses without ever being the check.

**The registry scope, for now.** The manifest attribute names are **built-in, prelude-bound globals**, which sidesteps the global-against-module-scoped registry question for exactly the toolchain vocabulary ([[../../surface-language/attributes#attr-question-01]]).
User-declared manifest fields are the growth path and reopen it.

### The manifest's staged cut

| concern    | first slice, gated on the module root                                                             | growth, designed in                                                     |
| ---------- | ------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------- |
| attachment | `@[…]` on the module or package root                                                              | a build-unit or external-file manifest for non-gandr roots              |
| fields     | inert descriptive fields plus dependency and version **coordinates**                              | exposed signature, required capabilities, toolchain constraints         |
| identity   | inert only, manifest hash-neutral; resolved-address participation through a **local** lock record | semantic-tier fields modeled as syntax; full toolchain-metadata hashing |
| resolution | a local content-addressed cache behind the store trait, keyed by source and resolved address      | endpoints, peer-to-peer hydration, distributed builds                   |
| ascription | none — coordinate matching only                                                                   | typed `σ`-ascription of the exposed signature                           |
| registry   | built-in global manifest schemas                                                                  | user-declared manifest fields                                           |

**Against this cut, what the tree has is the fields column of the first slice and nothing else.** The inert schemas are registered and typed; the module root, the lock record, and the cache are not built.

### Interactions

Each of these is a seam this design must not foreclose.

* **Entity attributes** — the host mechanism ([[../../surface-language/attributes]]); the manifest is its module and package consumer, and the refinement above is the one place this design departs from its guidance.
* **Modules and packaging** — the module layer supplies the root the manifest attaches to and the signatures the exposed-signature field points at ([[../../surface-language/proposed/modules]]).
* **Build configuration** — build-target configuration is the **sibling** attribute registry on build targets: the same typed-schema mechanism, a different entity and a different registry, so the manifest and build configuration do not collide.
* **The foreign interface** — the manifest's required-capabilities field is the **aggregate** of the unit's `extern` capability gates, which are themselves semantic attributes; the package-level field is a checkable summary of that surface ([[../foreign-interface]]).
* **Code generation** — the sibling first semantic consumer of the attribute layer; the manifest's identity-bearing fields join it in exercising the semantic tier once it lands.
* **Readable errors** — manifest diagnostics (an unknown field, an ill-typed coordinate, a duplicate `@[package(…)]`, a signature mismatch) render through the ordinary diagnostic adapter, localized by the pipeline's provenance machinery.
* **Self-hosting** — a staged self-hosting toolchain needs a manifest for its gandr-in-gandr packages, and is the capstone consumer of this section.
* **Value semantics** — the functional record-update stance informs an eventual manifest-editing surface ([[../../surface-language/value-semantics#Functional record update]]); off the near path.

### The corpus examples plan

Once implemented, the feature lands executable examples under `crates/surface-corpus`, split between a model subtree and a pathological one.

**Model examples**, none of which exists:

* a **module manifest** — a unit root with an inert `@[package(…)]`, the canonical first read, paired with a projection fixture showing that the renderer reads and never re-resolves;
* **dependency coordinates** — a manifest declaring coordinates and version aliases, with an expected **lock-record** fixture showing the coordinate-to-address flow;
* a **hash-neutral doc field** — two units identical modulo an inert `doc` field, asserted to have the **same** content address: the default made executable;
* an **exposed signature** — a manifest naming a signature that a consumer's expected `σ` ascribes against; growth-tier, activated by the module signature rung and the semantic tier together.

**Pathological examples**, none of which exists:

* an **ill-typed coordinate** — a payload violating the schema, pinning the localized ordinary type error;
* an **unknown manifest field** — pinning the did-you-mean over the manifest registry;
* a **duplicate `@[package(…)]`** — pinning the single-valued duplicate diagnostic;
* a **resolved-address hash** — two units identical in source and inert metadata but pinning **different** resolved dependency addresses, asserted to hash distinct: the participates-via-resolution leg, growth-tier;
* a **signature mismatch** — an importer expecting `σ` against a unit whose exposed signature does not satisfy it, pinning the failure **at the boundary, before the body runs**; growth-tier.

**Three of these are writable against the built schemas today** — the ill-typed coordinate, the unknown field, and the duplicate — because they exercise the attribute registry rather than the resolver, and the existing manifest example already proves the shape.
The rest wait on the lock record, the module root, or the semantic tier.

## Why this shape

**Maximal reuse.** The manifest is typed attribute data: schemas ride the record former, storage rides the side table, read-back rides the report projection.
The first slice adds field schemas and a taxonomy, not a format.

**The content-address boundary becomes precise and executable.** Inert manifest bytes never hash; resolved addresses hash through the lock record; and two corpus examples — the hash-neutral doc field and the resolved-address hash — pin both legs rather than leaving them as prose.

**Version compatibility is typed.** Aliases pin addresses and ascription is the real check, which replaces a heuristic with a decision procedure the checker already has.

**One mechanism, sibling consumers.** Build configuration and code generation attach to the same typed layer on different entities, so packaging does not accrete a bespoke slot.

### The risks

**The identity-bearing story is entirely deferred**, and that is the honest cost.
The exposed signature, the capabilities, and toolchain hashing all gate on the semantic tier, which the attribute layer only reserves — so the first-slice manifest is genuinely inert-only and **cannot yet answer "is this version compatible" by ascription**, which is the design's own headline claim.
If the semantic-tier reflection turns out not to compose with the address function, **the boundary moves rather than merely extends** — a risk inherited whole from the attribute layer ([[../../surface-language/attributes#The risk, and it is one risk]]).

**The host does not exist**, so this section is designed against an unbuilt substrate on two axes at once: the module root it attaches to and the semantic tier its identity-bearing half needs.

### The named dead-ends

Three alternatives are rejected, each with the reason that binds and, where it applies, the condition that reopens it.

**An external manifest file** for gandr-rooted units.
Rejected: it fragments identity into two disciplines and needs its own parser and typing path — the exact bespoke-format cost the attribute layer avoids.
**Kept as the reversal target for non-gandr roots**, which have no in-language declaration to attach to.

**Endpoint coordinates in the content address.** Rejected: it conflates _where_ with _what_, and would make the same content served from two providers hash distinct, which breaks the address-against-location split that peer-to-peer hydration rests on.

**The manifest as a whole-unit semantic attribute.** Rejected: it would make every doc-string edit perturb the address, defeating the cache the manifest exists to feed.

## Open questions

### pkg-question-01

**Are manifest schemas globally named, or module-scoped?**

The first slice's manifest attributes are built-in globals.
User-declared manifest fields reopen the global-against-module-scoped registry question, coupled to the data-declaration surface.

_Disposition:_ **carried** — open, and it is the same question the attribute layer carries ([[../../surface-language/attributes#attr-question-01]]), asked of one specific vocabulary.

### pkg-question-02

**Is the exposed signature a transparent advertisement, or an opaque seal?**

The transparent reading makes it derivable from source, hence redundant for identity, hence able to stay **inert**.
The sealed reading is generative and therefore genuinely identity-bearing, which forces participation via syntax.
Only one of the two readings costs the semantic tier.

_Disposition:_ **carried** — open, and it decides a row of the field-role taxonomy.

### pkg-question-03

**Is the package-level capability field a summary, or an upper bound?**

Is it a checkable **summary** of the unit's per-`extern` capability gates, or an independent **upper bound** the checker enforces?
The two differ in what a mismatch means: a summary that disagrees with its members is a stale summary, and an upper bound that is exceeded is a violation.

_Disposition:_ **carried** — open, and it is owned jointly with the foreign-interface capability work ([[../foreign-interface]]).

### pkg-question-04

**Does the coordinate or the resolved address enter the address, and is the storage tier the right substrate for the cache?**

Two halves of one question about what the cache is made of.
The taxonomy routes participation through the **resolved address**, not the coordinate, and that must be confirmed against the canonicalization so that re-aliasing a dependency to the same address is genuinely a no-op on the unit's address.
Separately, a content-addressed ordered-record tree with proofs now exists in this tree, and whether it is the right substrate for a build cache — rather than merely an available one — is unmeasured; its stated deficits are that it is two-level, in-memory, and canonical by construction rather than by a named theorem ([[../../implementation#Storage — content addressing, canonicalize-before-address]]).

_Disposition:_ **carried** — open; the first half is a check against the address function, the second is a design comparison that has not been run.

### pkg-question-05

**Where does a build unit's manifest live when its root is not a gandr declaration?**

For the build and distribution sense whose root is a non-gandr input, there is no in-language declaration to attach attributes to.
The external-file manifest is the reserved reversal for exactly this case, and exercising it is a decision rather than a fallback.

_Disposition:_ **carried** — open, and it is the one case in which a rejected alternative is the intended answer.

## Source and confidence

This document is written against the pre-reboot package and build-manager design record including its manifest section; the prior programme's tracker rows for the manifest design pass, for the deferred typed-package-import work, and for the consolidated platform backlog; and the tree — `gandr-surface-engine`'s `attributes` module for the registered manifest schemas, `crates/surface-corpus` for the runnable manifest example, and the storage-tier crates for the content-addressed substrate that exists.

**Confidence is high on the built account**, which is small: the schemas, their arities, their example, and the storage tier's consumer and deficits, each read from a named module or from the implementation track's own account.
**Confidence is medium on the design payload**, which is transferred from the design record and has had no independent pass.
**Confidence is low on one point and it is marked at the claim**: the specific peer-to-peer endpoint system named for distribution is a vendor selection carried across, with no locator in this corpus's reference register and no in-tree dependency, so nothing here should be read as a measurement of its current capabilities.

Where the design record and the tree disagree, the tree wins on status and the record wins on payload, and both readings are stated at the claim.
