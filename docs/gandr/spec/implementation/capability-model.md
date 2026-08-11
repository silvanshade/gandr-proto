# The runtime-host capability and grant model

Detail behind [[../implementation#The runtime host|the runtime host]]: the grant design owed by [[roadmap|the implementation roadmap's runtime-host bullet]] before the shell language lands — grants as explicit capabilities threaded through handler install and resume, the grant-check point at the driver/handler boundary, the linear zone made non-vacuous, and what deliberately stays ambient.

## The as-built posture

Every claim in this section is verified against the crate, with the module or symbol named at the claim.

**The seam is name-only and representation-independent.** `gandr_core_checker::effect::host` holds the seam's whole public vocabulary — `HostOp` (an owned triple of signature, operation name, and payload), `HostReply` (`Resume` or `Unhandled`), and the `HostHandler` trait with its blanket closure impl — expressed over public `Value`s and the operation _name_, never a machine continuation frame, so the boundary outlives any one evaluator.
The machine's invariant: the host intercepts **exactly** the `perform`s no source-level handler claims (the `PerformNoHandler`-bound ones), including those cut off from an enclosing handler by an intervening `reset` or a non-matching `handle`.
The L seam offers a **name-only signature** — the operation list is erased at the offer (`ShellDriver::handle`'s own note in driver.rs) — so everything downstream keys on names.

**The handler is unconditional.** `runtime-host`'s `ShellHandler` (handler.rs) is a transparent wrapper over a per-run tempdir counter and nothing else; `ShellHandler::dispatch` routes signature-name then operation-name over a closed operation set and every claimed operation runs its syscall and resumes.
No point in the pipeline asks _whether_ an operation should run: install (`run_with_driver` building `ShellDriver { handler, early: None }`) consults nothing, and resume (`HostReply::Resume`) delivers whatever the syscall returned.

**The driver already has the adaptation channel a denial needs.** `ShellDriver` (driver.rs) maps the internal `HostAction` (`Resume` / `Exit` / `Fail` / `Decline`) onto the seam's two-reply vocabulary, capturing the two outcomes the seam cannot express — a run-truncating `Proc::exit` and a fatal abort — out of band in `ShellEarly`, then declining so the machine takes its terminal `PerformNoHandler` step; `run_with_driver` surfaces the record as `ShellOutcome::Exited` / `ShellOutcome::HostFailed` in place of the blamed `Eval`.

**Payload decoding is total and pre-syscall.** Every decoder in codec.rs is total — a shape mismatch is `ShellError::Payload`, never a panic — and `Exec::exec` passes argv element-wise with no shell interpolation, so a metacharacter payload is an inert argument, not an injection vector.
The one flagged trust assumption: `exec_captured` buffers child output whole with no size cap and no wall-clock timeout — sound for the trusted, short-lived gate scripts v0 targets, explicitly not for untrusted programs.

The operation surface, with the risk class a grant model must price:

| signature | operations                                                             | as-built behavior                                                                                                                                                               | risk class                                                                    |
| --------- | ---------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| `Exec`    | `exec`                                                                 | spawn via `std::process::Command`; `captured` buffers output, `inherit` hands the child the parent's terminal; argv element-wise                                                | arbitrary code execution; terminal takeover in `inherit` mode                 |
| `Fs`      | `read`, `write`, `glob`, `stat`, `mkdir`, `tempdir`, `cwd`, `ls_files` | whole-file reads and writes at arbitrary paths; the glob subset forbids upward traversal (`..` matches nothing); `tempdir` is nonce-named and persists; `ls_files` spawns `git` | unrestricted filesystem access; an implicit second exec surface through `git` |
| `Env`     | `get`, `path`                                                          | read-only; an unset variable reads as the empty string                                                                                                                          | environment disclosure                                                        |
| `Proc`    | `exit`                                                                 | truncates the run with a code; never resumes                                                                                                                                    | run termination only                                                          |

**The linear zone is a frozen, vacuous shape.** `gandr_core_checker::ctx::Sigma` is committed in the two-zone context `Ctx` now (the frozen-shape decision, taken precisely to avoid an expensive retrofit) but populated by no typing rule; the crate docs (ctx.rs, stack.rs) name its obligation sources — session endpoints, **held capabilities**, acquired channels — all deferred.
The duplication discipline is typing-side: `stack.rs` permits duplicating a reified stack only when its captured Σ is empty, which today is vacuously always, and a conformance meta-invariant (conformance.rs) asserts Σ stays empty through every run.
Operationally, resumption is **multi-shot**: a captured continuation prefix is reified as a plain stack value with the handler reinstalled (the crate's soundness note in lib.rs), so a captured continuation may be resumed any number of times — sound precisely because Σ is vacuous.

## Grants as explicit capabilities

A **grant** is an atom $g = (S, o)$ naming one operation $o$ of one canonical signature $S$; a **grant set** $G$ is the authority a run is installed with.
The v0 vocabulary is exactly the twelve (signature, operation) pairs of the as-built surface — no payload-shape constraints; that refinement is a recorded challenge in the ambient register below.

Two threading points replace the ambient posture:

- **Install is explicit.** `run_program` / `run_program_with_prelude` take a grant set, and `run_with_driver` stores it on `ShellDriver` beside `handler` and `early`.
  The handler-installation point — today unconditional — becomes the moment authority is fixed.
- **Exercise is checked at resume.** Every offer through `ShellDriver::handle` is tested against the grant set _before_ `ShellHandler::dispatch` runs: a grant is exercised exactly where the ambient handler would have resumed unconditionally.

The algebra is **attenuation-only**: a nested scope (a subshell, a source-level handler installation) may hold $G' subset.eq G$, never more.
Grants are unforgeable because they never cross the seam as `Value`s — the program's payload is data, and authority is not.

Grants in v0 are **durable**, not consumable: exercise does not spend them.
Consumable grants are the challenged refinement whose natural home is the linear zone, not the driver.

## The check point at the driver boundary

A **grant-check point** is the single place where an intercepted operation is still semantic — decoded names, undecoded payload — and no syscall has happened yet.
It sits **between the seam callback and the dispatcher**: inside `ShellDriver::handle`, after the offer is re-packaged as an owned `HostOp`, before `ShellHandler::dispatch`.

| candidate site                             | verdict                                                                                                                                                          |
| ------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| the seam (`core-checker`'s `effect::host`) | **rejected**: the seam is a preserved representation-independent boundary shared by every driver; policy there couples the machine to one host's authority story |
| the codecs (`decode_*`)                    | **rejected**: decoding decides well-formedness, not authority; conflating the two makes a denied operation indistinguishable from a malformed payload            |
| inside each `ShellHandler::dispatch_*` arm | **rejected**: twelve check sites instead of one; the two-level name routing is the dispatcher's job, the grant check is not                                      |
| `ShellDriver::handle`, pre-dispatch        | **adopted**: one choke point; the `HostOp` is already owned there; the out-of-band outcome channel already lives there                                           |

The check order inside `handle`: name routing (is this ours?) → grant check (is it allowed?) → payload decode (is it well-formed?) → syscall.
Denial precedes decode, so a denied operation's payload is never inspected.

## Denial is a third runtime outcome

A denied operation is neither `Decline` (the operation is not ours — let the machine blame an unclaimed `perform`) nor `Fail` (a syscall failed fatally).
It is a **defined refusal**, and the driver already shows how to express one: extend the out-of-band channel with a `ShellEarly::Denied` recording the (signature, operation) pair, decline so the machine terminates, and surface a `ShellOutcome::Denied` from `run_with_driver` — the same adaptation pattern `Exit` and `Fail` already use, with no seam change.
The rejected alternative — mapping denial to plain `Unhandled` and letting `PerformNoHandler` blame — conflates _no handler claims this_ with _the handler refused this_, and loses the missing grant's name from the blame.

## The linear zone, made non-vacuous

The mechanism the frozen shape was committed for: **a grant held across a capture is a Σ entry.** When a continuation prefix is reified under an installed grant scope, the held authority rides the captured Σ, and the typing-side duplication rule (`stack.rs`: duplication only when the captured Σ is empty) bites for the first time — a grant-holding capture is **one-shot**; a grant-free capture stays multi-shot.
The conformance meta-invariant relaxes accordingly, from "Σ stays empty through every run" to "Σ is empty at every capture the program duplicates" — the one cross-crate obligation this design imposes, living in `core-checker`'s typing rules and conformance suite rather than in the runtime.

This is what makes the linear-zone claim non-vacuous rather than deleted: multi-shot resumption remains the default for grant-free code, and linearity is enforced exactly where resumption would replay authority.
A re-resumed grant-holding continuation would re-exercise the grant through the reinstall the machine performs, and the typing discipline is what rules that configuration out for well-typed programs.

## The soundness obligation

The ambient posture was sound vacuously: with Σ empty, re-resumption replays nothing, so always-resume is harmless.
With grants the soundness question has content, and this model names it: **the host is a certified implementation of the canonical signatures only if every resume is backed by a grant the install supplied, and every grant-holding capture resumes at most once.** The certified-implementation criterion of the layered-game-semantics line [@oliveira-vale-mellies-shao-koenig-stefanesco-2022-layered] — the roadmap's named candidate for the soundness note the crate currently lacks — is the shape to state the obligation in: the handler as a strategy whose plays respect the discipline of the layer it implements, certified against the signature's specification rather than merely tested against it.
Stating the obligation is this document's contribution; discharging it is queued work.

## What the shell language needs

The shell language sits in the ordered tail lane; the surface already reserves its fragment (braced parameters distinct from string interpolation, subshell brackets, file-descriptor redirections, host-escape reserved) and the engine lowers `#!{ … }` shell blocks onto the canonical signatures.
Two as-built facts frame the staging: the operation set is deliberately named `Exec`/`Fs`/`Proc`/`Env` and does not appropriate the reserved `Shell` signature name, and the eager OS pipe between external commands is a stopgap, not the typed `Pipe` session (both from the crate's soundness note in lib.rs).
The obligations this model imposes on the shell language, each carried:

1. **A grant-installation form.** A statically scoped surface form installing a grant set for a block; without it there is nothing for the driver to thread and nothing for Σ to record.
2. **Effect-row honesty.** The sealed row on the returner type must name the signatures a block may perform, so install can check the supplied grants _cover_ the row — the static demand against the dynamic authority.
3. **Attenuation at subshell brackets.** A subshell inherits a subset of its parent's grants and the elaborator checks subsetting, so privilege narrowing is syntactically visible where the shell fragment already scopes.
4. **A denial rendering.** `ShellOutcome::Denied` needs a shell-level presentation — a structured error naming the missing grant — distinct from both blame and fatal abort.
5. **Spawn-mode awareness.** The `Exec` payload's `captured`/`inherit` mode is authority-relevant (`inherit` hands the child the terminal); the surface must keep the mode explicit so the challenged payload-shape grant refinement can key on it.

## What stays ambient, by decision

Every decline here is **challenged**, never refuted: each is a representation or staging decision of the current design, recorded with the delta that would reverse it.

| declined                                                                   | why it stands                                                                                                                                                                                                                                                                                                          | the reversal delta                                                                                                                                                                                       |
| -------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| per-path and per-program grant constraints (path allowlists, argv filters) | v0 grants are (signature, operation) atoms; a path constraint needs a canonicalization oracle — symlinks, `..`, and platform aliases (macOS `/var` versus `/private/var`) put "same path" beyond string comparison — and the glob walker's documented containment properties are the only path discipline in the crate | flip: a path-semantics decision plus a normalization oracle; cost: one design and its conformance suite; unlock: fine-grained filesystem policy and the shell language's workspace confinement           |
| per-variable `Env` grants                                                  | environment reads are already name-scoped and read-only, and a determined reader has side channels (`Fs::read` over the same secrets) unless the filesystem is constrained too                                                                                                                                         | flip: a secret-bearing environment convention together with per-path `Fs` constraints, against untrusted corpus programs; cost: small once path grants exist; unlock: credential hygiene in gate scripts |
| mid-run revocation                                                         | the grant set is fixed at install; revocation needs mutable authority state with a happens-before story across multi-shot resumes                                                                                                                                                                                      | flip: the linear zone tracking grant _versions_ rather than presence; cost: a revocation semantics across capture and resume; unlock: dynamic sandboxing and least-authority pipelines                   |
| consumable (use-once) grants                                               | v0 grants are durable; spending needs the Σ machinery active, which is exactly the non-vacuous-zone obligation above                                                                                                                                                                                                   | flip: the linear-zone activation landing; cost: small on top of it; unlock: one-shot authority — single-spawn licences, one-time writes                                                                  |
| capability passing through the seam itself                                 | grants live runtime-side; the seam stays representation-independent and grant-free so any driver can bind it                                                                                                                                                                                                           | flip: source-level handler-installation syntax plus the Σ story, so programs sandbox their own sub-computations; cost: a language feature, not a runtime one; unlock: in-language privilege separation   |
| an OS-level sandbox (seatbelt, landlock, seccomp)                          | the grant check is a language-level boundary; OS enforcement is a per-platform backend with a portability story the project has not needed — v0 targets trusted gate scripts, and the `exec_captured` buffering assumption already scopes untrusted programs out                                                       | flip: an untrusted-program execution target; cost: platform backends plus a portability matrix; unlock: running adversarial code under grants the OS makes real                                          |

`Fs::cwd` and the `Env` pair stay ambient outright: the cwd reveals one path the process already knows, and the read-only environment pair is priced in the register above.

## The verdict

Evidence **for** adopting the design as staged:

- the choke point exists today — every host-intercepted operation funnels through `ShellDriver::handle` before `ShellHandler::dispatch`, with the payload already owned;
- the denial channel exists today — the `ShellEarly`/`ShellOutcome` pair is precisely the out-of-band adaptation a third outcome needs, and adding a variant touches no seam type;
- the linear zone has the named hook — the crate docs already list held capabilities as a deferred Σ-obligation source, and the duplication rule that would enforce one-shotness is already written, waiting for a population rule;
- the shell language is staged but not blocked — every obligation above is a surface feature on top of this model, none a redesign of it.

Evidence **against**:

- the seam offers a name-only signature, so payload-shape constraints (the challenged refinements) must decode inside the check or move post-codec — the check point as staged prices atoms, not predicates;
- one-shot enforcement is typing-side while grants are runtime-side: the bridge (the install form plus the Σ population rule) must land before the zone is non-vacuous in an actual run, so the model's linearity half is obligation, not mechanism;
- the reinstall-on-resume behavior lives in the machine (the crate's soundness note), outside the runtime crate's reach — the runtime alone cannot enforce one-shotness; only the typing discipline can.

**Net: adopt.** The grant model lands entirely behind the existing driver boundary with no seam change and a defined-denial outcome the driver's own pattern predicts; the linear-zone activation is the one cross-crate obligation, and it is priced and placed rather than discovered.
