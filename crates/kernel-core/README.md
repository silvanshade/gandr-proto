# kernel-core

`gandr-kernel-core` is the trusted, minimal certified kernel: a closed polarized CBPV term language, an append-only environment with one checked admission choke point, and a canonical v1 export/readback format.

## Current provision

- `Environment::add_decl` rechecks declarations without trusting elaborator output; `read` replays canonical bytes through that same choke point.
- `write_segmented` emits canonical bytes plus admission-ordered declaration framing.
- `SegmentedArtifact::segment_spans` exposes header and declaration byte ranges as a reader byproduct for untrusted storage layers.
  Spans carry no hashes and declaration segments are not independently replayable because sharing may cross segment boundaries.
- Decode rejects truncation, unknown or reserved vocabulary, structural violations, noncanonical encodings, and exceeded expanded-work budgets.

## Planned but absent

The kernel does not provide effects, handlers, general recursion, data declarations, identity types, holes, or a persistent storage backend.
Cross-process sealing uniqueness and the O(1) strata variable-plus-offset constructor remain separate follow-ups.

## Usage

Use `write` for a complete canonical artifact and `read` for validating replay.
Use `write_segmented` when an outer storage layer needs declaration-granular records while preserving whole-artifact replay.

## Named theoretical ideas

The implementation follows the re-checkable export and maximal-sharing representation described by the massive-term design, with replay as the inner validity wall and content addressing kept outside the trusted base.

## Primary references

- _Dependent Types in Programming_, Thorsten Altenkirch, Simon McBride, and James McKinna, 2007, DOI: 10.1007/978-3-540-74407-8_2.
- _The Science of Programming_, David Gries, 1981, ISBN: 978-3540961565.
