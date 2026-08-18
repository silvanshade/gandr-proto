# gandr-theory-virtual-doctrines

The reflection face: gandr's rewrite layer read as a virtual double category, with an internal logic for virtual double categories as its language.

The crate is strictly additive.
It reflects the engine rather than re-implementing it: signatures, cells, certificates and composition all stay where they are built, and what lands here is a first-order judgment layer and a query surface stated over them.
A rule that claims an engine fact is validated by replaying the underlying certificate, so the reflection cannot drift from the thing it reflects.

## Current provision

- The virtual-double-category interface (objects, tight arrows, loose arrows, multi-ary cells, with restriction and multicategorical composition) and its instance over the cell store.
- Checked actions and deterministic witnesses for projection, diagonal, and complete product-structure preservation.
- The reflected judgment layer: protypes and proterms, covering the internal logic's four judgment families.
  It is **first-order with no dependent types**, and one proterm constructor embeds an engine derivation directly.
- A bidirectional checker over two-sided contexts.
  Engine-fact rules are validated by replay elaboration.
- Protype isomorphisms as paired replayable witnesses with groupoid laws, composed in the invertible mode.
- The constructor-menu query surface: path induction over rewrite traces, per-overlap seam composition, extension queries, and instantiation tables.
- The directed fragment, and the crate's largest face: variance-carrying contexts, a directed hom whose eliminator is restricted by polarity so that symmetry stays underivable, and ends and coends as quantifiers with the Fubini and coYoneda operations.
  This is exactly what the first-order judgment layer deliberately cannot express.

**The soundness posture is the crate's most important fact, and it is a limit rather than a feature.** gandr implements checkers and property tests here; the theorem-grade claims are not made in Rust.
The syntax-and-semantics biadjunction that says reflection loses nothing, and the groupoid-fragment certificate-composition theorem, both ride the metatheory tree.
What this crate carries is engineering evidence: per-rule property tests, plus replay elaboration on every rule that claims an engine fact.

## Planned but absent

- **A consumer.** No workspace crate depends on this one.
  It is a face over the rewrite stack that nothing above it yet reads.
- Directed univalence, which the directed-fragment source names as its own future work.
  It is a second-order step and is not scoped here.
- The dependent refinement of the judgment layer, which stages behind the levitation ladder's next rung.

Two questions are open rather than settled, and neither mints a decision record here: the granularity of a loose arrow, currently a named relation read as a set of generating cells rather than as a single cell; and the identity model for a reflected cell, currently a derivation tree quotiented by replay-equivalence and elaborated to the engine only through invertible composition.
Both would need pinning only if a later stage made the reflected syntax canonical.

## Using it

Build a context, check a proterm against a protype, and read the witness back.

```rust
use gandr_theory_virtual_doctrines::check::Checker;
use gandr_theory_virtual_doctrines::syntax::Protype;

let checker = Checker::new(&derivations, &cells);
checker.check(&context, &hyps, &proterm, &protype)?;
```

A positive answer from the checker is a claim about the reflected syntax.
Where the rule asserted an engine fact, the answer additionally means the underlying certificate replayed, which is the only sense in which this layer's yes is evidence about the engine.

## Theoretical ideas relied on

Virtual double categories and their internal logic; the syntax-and-semantics biadjunction as the statement that reflection loses nothing; two-sided contexts and bidirectional checking; dinaturality, and directed type theory in which hom is a directed equality whose eliminator is polarity-restricted so symmetry is underivable; ends and coends as quantifiers, with Fubini and coYoneda; replay-equivalence as the identity on derivations.

## Primary references

- Hayato Nasu, _An Internal Logic of Virtual Double Categories_, 2024, arXiv:2410.06792 — the judgment families the reflected syntax layer realizes, and the biadjunction the soundness posture defers to the metatheory.
- Hayato Nasu, _Logical Aspects of Virtual Double Categories_, master's thesis, 2025, arXiv:2501.17869 — the extended development behind the same logic.
- Andrea Laretto, Fosco Loregian and Niccolò Veltri, _Di- Is for Directed: First-Order Directed Type Theory via Dinaturality_, Proceedings of the ACM on Programming Languages 10 (POPL 2026), 1759–1789, `doi:10.1145/3776703` (arXiv:2409.10237) — the directed fragment: the polarity-restricted eliminator, the (co)ends-as-quantifiers reading, and the theorem that dinaturals always compose over groupoids, which is what makes the invertible-mode cut admissible.
