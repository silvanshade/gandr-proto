---
name: find-best-typescript-packages
description: Research, compare, and select TypeScript/JavaScript npm packages using ecosystem standards, maintenance signals, dependency footprint, compatibility, native/binary scrutiny, and security risk.
condition: Use before adding a nontrivial TypeScript/JavaScript dependency or when a coding task needs an npm package recommendation.
---

# Find Best TypeScript Packages

Use this skill before adding a nontrivial npm dependency, or when a coding task needs a TypeScript/JavaScript package recommendation.

## Discovery sources

Check ecosystem and metadata sources before plain keyword search:

- npm registry metadata via the project package manager, `npm view`, or registry UI: versions, dist-tags, publish cadence, maintainers, license, exports, dependency fields, engines, lifecycle scripts, provenance, and deprecation.
- Existing declared dependency graph and package exports.
  Prefer an existing public dependency over adding another package; never rely on undeclared transitives.
- Upstream repository health: issues, release cadence, contributors, CI, lockfiles, changelog, test matrix, TypeScript support, Node/browser support, and maintainer responsiveness.
- Ecosystem-standard sources where relevant: official framework/runtime docs, Node.js, TypeScript, Vite/Vitest, Playwright, Testing Library, OpenJS, and high-signal curated lists as discovery input only.
- Security and supply-chain checks after dependency changes: create or refresh an SBOM with `syft` and scan it with `grype` through the configured project task.

## Selection procedure

1. State the job the package must do and the coupling the repo can accept.
2. Prefer no new dependency when a small local implementation is cheaper, safer, and maintainable.
3. Find candidates from metadata, existing declared packages, official docs, and upstream repositories.
4. Compare at least the strongest two candidates unless one choice is a clear ecosystem standard.
5. Inspect metadata and source for health, fit, Node/runtime support, browser assumptions, ESM/CJS compatibility, type quality, install/build behavior, license, and security posture.
6. Prefer narrow packages that preserve loose coupling, backend flexibility, and compatibility with current Node and TypeScript.
7. Avoid heavyweight peers, global hooks, generated code, native builds, and lifecycle scripts unless the task truly requires them.
8. Use the narrowest correct dependency scope:
   - `devDependencies` for tests, build, lint, docs, and harness tooling;
   - production dependencies only for shipped runtime code;
   - `peerDependencies` only for intentional caller-provided integrations;
   - `optionalDependencies` only for truly optional platform accelerators.
9. Record the decision and rejected alternatives in the task report or existing project documentation when future maintenance will depend on the choice.
10. After adding dependencies, run the configured install/check flow and the `syft`/`grype` scan before closing.
    If a check cannot run, report the exact blocker and complete the remaining local verification.

## Relative quality scoring

Score candidates relative to the task, not as absolute endorsements.

Uprank packages with:

- strong task fit and a small API surface;
- first-class TypeScript types or high-quality bundled declarations;
- current Node, TypeScript, and ESM support matching the repo;
- strong downloads and downstream use for their domain;
- multiple maintainers or a credible contributor base;
- recent compatibility maintenance and responsive issue handling;
- primary contributors with strong ecosystem reputations and a track record of maintaining other successful packages;
- clear docs, examples, changelog, and semver discipline;
- minimal transitive dependencies and no unnecessary peers;
- optional integrations instead of forced framework/runtime coupling;
- no lifecycle scripts, native builds, or binary downloads unless essential and well documented;
- a compatible, explicit license.

Downrank packages with:

- low downloads for their age and domain;
- deprecated, unmaintained, or abandoned status;
- more than 6 months between updates unless mature, feature-complete, and widely established;
- age under 3 months unless specialized and an exact fit;
- a single maintainer with few contributors;
- many required or non-optional transitive dependencies;
- large install, binary, transitive, or bundle-size impact;
- native builds, postinstall downloads, or platform-specific binaries for non-runtime tooling unless the coverage gain justifies the cost;
- multiple recent high-profile or high-risk CVEs, known supply-chain compromises, or slow security response history;
- forced coupling to frameworks, test runners, transpilers, package managers, or runtimes when package independence matters;
- outdated API assumptions, CommonJS-only surfaces, weak ESM interop, or stale TypeScript declarations;
- heavy code generation, decorators, transforms, monkeypatching, or global state where a simpler API would work;
- supersession by a better-maintained fork or successor.

## Native, PTY/TTY, and binary scrutiny

Treat native modules, PTY/TTY/pseudo-terminal packages, GPU/PTX bindings, browser engines, and binary-downloading packages as high-friction dependencies.
Before selecting one, verify:

- why pure TypeScript/JavaScript is insufficient;
- supported Node ABI and target platforms;
- prebuild availability and source-build fallback;
- lifecycle scripts, download hosts, checksums, and provenance;
- install determinism and CI cache behavior;
- macOS/Linux/Windows behavior when cross-platform use matters;
- whether the dependency is dev-only or affects shipped runtime code.

For in-harness terminal tests, prefer one maintained dev-only PTY driver, plus a terminal parser/emulator only when raw ANSI output must be interpreted.
Do not import a transitive PTY package unless it is declared directly or exposed by a supported public API.

## Output format

Report:

- recommended package and version strategy;
- dependency scope (`devDependencies`, production, optional, or peer) and why;
- strongest rejected alternatives and why they lost;
- maintenance, downloads, downstream use, and maintainer/contributor signals;
- Node/TypeScript/ESM and platform compatibility risks;
- transitive footprint, native-build, lifecycle-script, binary, and bundle-size risks;
- `syft`/`grype` result, or the exact blocker to running it.
