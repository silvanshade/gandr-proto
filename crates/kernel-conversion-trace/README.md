# gandr-kernel-conversion-trace

The dependency-free conversion-decision seam shared by `gandr-core-nbe` and `gandr-kernel-core`.

## Current provision

- `ConversionDecision<Id>` is one decision vocabulary for unfold, postpone, force, and shared comparison.
- `TraceSink<Id>` is statically dispatched; consumers own the identifier type and any storage policy.
- `NullSink` is an empty `no_std` implementation.
  The default conversion paths instantiate it, so recording adds no state or dynamic dispatch.

This crate contains no term types, checker judgments, strategy policy, persistence, wire encoding, certificate tags, allocation, or replay engine.
Traces are session artifacts; the kernel owns replay.

## Using it

```toml
[dependencies]
gandr-kernel-conversion-trace.workspace = true
```

The engine and kernel are the only intended consumers.
Keep identifiers local to the owning arena and do not treat `ConversionDecision` as a serialized format.
