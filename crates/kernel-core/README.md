# gandr-kernel-core

`gandr-kernel-core` is the trusted, minimal certified kernel: a closed polarized CBPV term language, an append-only environment with one checked admission choke point, and a canonical v1 export/readback format.

## Current provision

- `Environment::add_decl` rechecks declarations without trusting elaborator output; `read` replays canonical bytes through that same choke point.
- `write_segmented` emits canonical bytes plus admission-ordered declaration framing.
- `SegmentedArtifact::segment_spans` exposes header and declaration byte ranges as a reader byproduct for untrusted storage layers.
  Spans carry no hashes and declaration segments are not independently replayable because sharing may cross segment boundaries.
- `convertible_values_with_sink` and `convertible_computations_with_sink` emit the shared decision vocabulary through a statically dispatched sink; `replay_values` and `replay_computations` re-execute the kernel worklist and compare decision kinds plus the claimed verdict.

## Trace boundary

The kernel consumes the dependency-free `gandr-kernel-conversion-trace` seam with kernel-local arena identities.
Replay is an in-process session check: it has no persistence format, identity translation layer, strategy policy, or term dependency on the untrusted engine.

## Using it

Use `write` for a complete canonical artifact and `read` for validating replay.
Use `write_segmented` when an outer storage layer needs declaration-granular records while preserving whole-artifact replay.

## Theoretical ideas relied on

The implementation follows the re-checkable export and maximal-sharing representation described by the massive-term design, with replay as the inner validity wall and content addressing kept outside the trusted base.

## Primary references

- Paul Blain Levy, "Call-by-Push-Value: A Subsuming Paradigm", in _Typed Lambda Calculi and Applications_, Lecture Notes in Computer Science, Springer, 1999, 228–243.
  DOI [10.1007/3-540-48959-2_17](https://doi.org/10.1007/3-540-48959-2_17).
- David Gries, _The Science of Programming_, Springer, 1981.
  Edition and ISBN unverified against a publisher record; more than one printing is in circulation.

The re-checkable-export and maximal-sharing representation this crate implements has no verified primary source recorded here.
