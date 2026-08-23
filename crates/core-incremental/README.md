# gandr-core-incremental

`gandr-core-incremental` re-types an editing session's program after each edit by re-typing only the region the edit reached, and re-validating rather than reusing the rest.
Its granularity is the top-level item.

The seam is deliberately parser-agnostic: an item is a name, an ascription, and a lowered core term, so a front end's spans, origin tables, and node identities cannot reach the reuse decision.

## Current provision

- `region` — the item seam: one lowered top-level item, the ordered items of one revision, and the trait a front end implements to produce them.
- `footprint` — an item's dependency footprint, over-approximated by construction: a core node it cannot represent as a read set marks the footprint opaque, which costs reuse and never soundness.
- `checkpoint` — the validated-resume engine.
  An item's cached typing is adopted exactly when its identity — name, ascription, and lowered term together — is unchanged and no name in its footprint changed binding.
  The footprint tested is rescanned from the edited item's current term, never read back from the stored checkpoint, so a stale persisted footprint cannot license reuse.
  Item identity survives inserts and deletes because an edit is spliced onto an order-maintenance structure rather than keyed on position.
- `persistence` — content-addressed checkpoint sets behind one store trait, with an in-memory and a file backend.
  Both are failure-atomic: a rejected store leaves the store exactly as it was.
- `session` — one-call submission over a store, carrying persistence, invalidation, and synthesis.
- `stream` — ordered, resumable synthesis events over validated checkpoints.
- `boundary` — the semantic wrappers those signatures carry in place of bare primitives.

## Planned but absent

Granularity below the item — a within-item dirty frontier, or per-term-node checkpoints coupled to a solver — is not built.
Neither is evaluation: a checkpoint records only what re-typing must reproduce, and running the program is a driver's concern.

**Adoption is unsound for an item whose typing consults a definition's value rather than only its type**, which happens where a type carries a value position and the comparison is definitional — an identity type's endpoints, today.
Two gaps combine: a footprint is scanned from the item's term and never from its ascription, and the changed-binding set compares bound types and never definitional unfoldings.
`gandr-t8j6` carries the mechanism, two reproductions, and the design alternatives; `tests/incremental.rs`'s `divergence` module pins both reproductions as live witnesses that fail when the defect is fixed.

## Using it

`cargo nextest run -p gandr-core-incremental --features=full` runs the crate suite, whose standing obligation is the differential in `tests/incremental.rs`: for every edit, resuming from a checkpoint set yields exactly the typings a from-scratch run computes.
The differential is driven through the item seam by an in-tree test double, so it needs no parser.

Every resume the differential performs discharges four obligations, so a fixed witness and a generated edit are held to one standard: the resumed typings equal a from-scratch re-type; every adopted item's footprint is non-opaque, matches an independent re-scan of its term, and reads only names bound identically at the item's new and old positions; the resulting checkpoint set survives the persistence codec unchanged, and an edit sequence resumes from the _decoded_ set so persistence sits inside the loop; and there is one adoption flag per typing.

The generated coverage is programs of one to six statements under single edits and under chains of up to four, with a resume at every step — replacements, insertions, deletions, coordinated renames that rewrite a definition together with all its readers, independent swaps, and ascription changes.
The `teeth` module proves the differential can fail, by seeding corruptions it must catch: a stale cached typing that rides through an adoption, and a suppressed invalidation signal that produces over-adoption.

## Theoretical ideas relied on

Item-granular incremental typing with validated resume rather than blind reuse; conservative dependency footprints; order maintenance as the carrier of item identity across edits; content addressing as the outer integrity wall over persisted checkpoints, with re-typing as the inner validity wall.

## Primary references

- Paul Dietz and Daniel Sleator, "Two algorithms for maintaining order in a list", in _Proceedings of the Nineteenth Annual ACM Conference on Theory of Computing_ (STOC '87), ACM, 1987, 365–372.
  DOI [10.1145/28395.28434](https://doi.org/10.1145/28395.28434).
