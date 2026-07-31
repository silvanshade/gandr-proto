# Implementation roadmap

What remains on the Rust side, beyond the phase table in [[../implementation|the implementation track]].
The phase plan itself is `PLAN.html` plus the tracker; this file carries the standing constraints and residuals a phase author must not rediscover.

## The anticipation register

Nine binding forward-compatibility constraints on rungs not yet built, each preventing a named failure.
The two most load-bearing:

* **Never conflate certificate equivalence with type-level identity.** The certificate normal-form relation is a quotient on derivation _data_, never a type former; conflating it with `Path`/`Flow` is a metatheoretic category error that would surface inside the kernel — the implementation-side twin of "the normal form is a cost fast path, never a decidability result".
* **The overlap enumerator emits seam data**, span-level overlap descriptions, not just boolean or reduced results — a cell store that can answer overlaps but cannot describe them starves the convolution face (landed as the renamed-right-leg accessor).

Also standing: content-addressed identity is structural only — no arena addresses, generation stamps, or session state in hashed term identity.

## The engine↔metatheory statement contract

The dovetail between the Rust fast paths and what the Agda side must prove: identity is replay-equivalence; laws are witnessed cells at value-setoid grade; composition is strict on data and coherent on behaviour; loose composites need not exist.
The tractability witness (`VTractableAt`-shaped: a record per tractability reason, with the normal-form/shift-equivalence decision as its first inhabitant) **does not exist yet** and is where the `cells_equal` fast path's soundness certificate is born; the fast path is **TCB-adjacent** — a guard plus the witness, never documentation — and the off-TCB framing applies to the enumerator only.
The tractability axis (convergent-fragment versus certificate-carried) must stay separable from the invertible/directed mode axis; they coincide in today's two-band design and will not once a convergent directed fragment exists.

## Phase residuals worth pinning

* **Kernel-replay** (the certificates phase): an independent reader-side framing walk against the format specification, never shared code; the annotation plane (variance/directedness) must be parseable by both checkers, which is why its slot is reserved early.
* **Modules**: primitive modules with coercive matching, abstract-type sealing with export replay, generative functors, first-class modules with the package Σ.
* **The alphabet-polymorphic enumerator** (shared with the metatheory roadmap's top spike): lift `theory-computads` over its cell alphabet so the shape-layer and directed rule layers can use the completion machinery; the monomorphism is currently intentional (a review tripwire), so the lift is a design change, not a refactor.
* **Runtime-host capability model**: currently ambient always-resume with no sandbox and a multi-shot vacuous linear zone; the handler soundness note should cite the certified-implementation criterion, and a capability/grant design is owed before the shell language lands.
* **The engines' first external consumer**: nothing outside their own tests reaches `theory-computads` or `theory-virtual-doctrines`; the integration phase is a demo obligation, not construction.
* **Incrementality lane**: the standing gate is incremental ≡ from-scratch on every landed increment.
* **Coverage floor seeding**: enforcement is per production file, and the floor table is currently unseeded.
* **Storage**: persistent backend, deeper tree, and boundary-grinding hardening are declared deficits; wire compatibility is unclaimed.

## Stale-documentation repairs owed

Found by inventory against the live tree; each is a one-line-to-small fix in its owning artifact:

* the checker crate's status file still says the CEK evaluator is retained as the differential oracle — it is retired and removed; differentials compare frozen snapshots;
* `ARCHITECTURE.md` counts twenty-two members, omits four crates from its domain table, counts a parked crate, and stops its tier ladder one tier short;
* `AGENTS.md` and `docs/workflow/specs.md` route specification work to a validated-corpus entry point that does not exist while the `.gfd` pipeline is parked; the routing should name this markdown corpus (this reboot updated the Agda workflow's done-rule already);
* `PLAN.html`'s current-state section and its kernel-core row predate the build-out and should be read through the tracker;
* whether the spec tracks register in `docs/gandr/MANIFEST.yml` (and so come under the manifest-drift gate) is a deliberate decision to take, not an accident to inherit.
