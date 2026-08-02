# Ornaments over the description universe — what the sort index must supply, and what can be built before it

**Proposed.
Nothing this document proposes is adopted, and the construction it describes does not exist.** It carries the ornament machinery of the datatype-cosmology line — ornaments as an intensional characterisation of Cartesian container morphisms, ornamental algebras and generated forgetful maps, algebraic ornaments and reornaments, and the functional-ornament layer that transports operations across refinements — as a proposal held against machinery gandr is committed to and has not built.

It exists because the owner ordered the question answered from a full read of the source, and because the answer turned out to be a schedule rather than a verdict: **the gate the ornament machinery waits on is the same gate the planned n-sorted signature surface (the `sign` block) waits on — the sort-indexed description universe — so the gate is paid once**, and the requirements below are design inputs for that single payment.

* Status: **proposed, and deferred on the sort index.** The deferral is not a decline and not a scope cut; it is an import order shared with the signature generalization.
  The gating substrate is [[#The gating substrate, and what the deferral is deferred on]], and every requirement it must satisfy is at [[#The requirements ledger]].
* **The read's confidence is carried and never upgraded.** Every claim about the source names the statement it rests on, and the grades are collected at [[#Source and confidence]].
* **A citation-convention note, because the source's numbers are unusual.** The thesis numbers paragraphs and formal statements on one shared per-chapter counter, so a numbered paragraph and a numbered definition are neighbours in a single sequence.
  Here "¶ n.m" cites a numbered paragraph and "def/thm/lem/rem/ex/prop/fig n.m" a numbered statement; a bare section number would not resolve.
* **Two constructions in this import are owed, not imported.** The source names them and does not build them; they are marked at [[#ornament-requirement-07]] and [[#ornament-requirement-08]] so nobody reads them as ports.

## The gating substrate, and what the deferral is deferred on

**The gate is the sort-indexed description universe, plus a propositional equality the constructions can insert.**

The source's ornament universe is parameterised over an indexed description and a reindexing — `Orn (D : IDesc K) [u : I → K]` [@dagand-2013-cosmology, def 8.17] — and the source says directly that on non-indexed types "ornaments are of lesser practical interest" [ibid., rem 8.13].
The index is what an ornament refines; without it the machinery has nothing to be defined over.

**gandr's description universe is single-sort in the strong sense, verified at the symbol.** `Code::Var` is the only former that counts as a recursive occurrence, and it is nullary — it carries no sort; `Code::is_recursive` reports whether a code _contains_ a `Code::Var` anywhere, per its documented contract, and nothing else can stand for one (`theory-levitation`'s `code` module).
A reference from one declared datatype to another is a `Code::Field` over a symbolic `ValueTypeRef::Ctor`, documented as a **non**-recursive leaf, so a cross-sort recursive occurrence is indistinguishable from an unrelated external type reference.
The one sort vocabulary the code grammar does carry is the binder's — `Code::Bind` names an atom sort, a binder's sort rather than a datatype's — and whether the index set `I` reuses, collides with, or stays disjoint from that atom-sort vocabulary is itself a design input for the gate.
The as-built universe therefore cannot express an n-sorted mutually-recursive signature at all — which is the same limitation the source records for its own non-indexed universe, listing mutual inductive definitions among what that universe does not support [ibid., rem 4.12, ex 5.34].

**The same gate is the `sign` block's gate, and the corpus already holds the position this document ratifies.** [[../../metatheory#Cellular data — descriptions, cells, and computads]] records that the multi-output term face forces the indexed description universe and that the signature universe should be container-based precisely because sorts are arities.
The source is the ratifying artifact: the indexed universe takes `var i` at a sort [ibid., def 5.8], multi-sortedness is a currying `func I J = J → IDesc I` [ibid., def 5.12], the index-as-sort identification is made in so many words [ibid., ¶ 5.4], and an n-sorted signature is one description indexed by an enumeration of size n [ibid., ex 5.34] — never a grouping of per-sort _descriptions_ and never a signature stratum above the universe, though the per-sort **surface** survives: the source supplies grouping as notation over the one indexed description [ibid., rem 5.46, def 5.44], which is the exact shape the ruled `sign` block wants.

**The second half of the gate is an equality former.** The algebraic ornament inserts a constraint `α xs = x` into a description [ibid., def 8.41], and the source is explicit that its universe cannot _introduce_ a propositional equality — one has to pre-exist [ibid., rem 5.37]. gandr's first-order code fragment has no such former today, and [[#ornament-requirement-03]] records the genuine design tension there rather than treating it as a missing feature.

**So the deferral has a concrete condition, and it is shared.** The ornament machinery becomes statable when the description universe grows its sort index and an equality-constraint former, which is the same landing the n-sorted `sign` block requires.
Until then this document is a requirements ledger and a buildable-now list, and nothing in it is a licence.

## What the import gives once the machinery lands

**A genuine algebra of cross-signature relationships, and generated rather than hand-written reuse.**

**Ornaments are morphisms of signatures, proved.** An ornament is an intensional characterisation of the Cartesian morphisms of containers, `orn D u v ≅ ICont(−, ⟨D⟩)_{u,v}` [@dagand-2013-cosmology, prop 8.85], with descriptions and containers equivalent by the two translations and their inverse property [ibid., def 5.74, def 5.75, ¶ 5.76, prop 5.71].
They compose in both directions — identity [ibid., def 8.103], vertical composition collapsing refinement chains [ibid., ¶ 8.104], horizontal composition along functor composition [ibid., ¶ 8.106] — and every pair of ornaments over a common base has a pullback [ibid., ¶ 8.113, prop 8.114], whose merging of two independently-added indexing disciplines is the source's own reading, worked for bounded lists [ibid., rem 8.115, ex 8.112].
Ornamentation is stable under the derivative for containers differentiable in the index [ibid., prop 8.121, def 8.117], so ornamenting and deriving commute — stated unqualified by the source's own remark [ibid., rem 8.122].

**The sort-level operations a signature import wants come from the frame structure.** Base-change and cobase-change containers along sort maps [ibid., def 8.76, def 8.78] give sort renaming and sort restriction as ornaments — machinery the source itself attributes to the polynomial-functor line [ibid., ¶ 8.74, ¶ 8.75] [@gambino-kock-2013-polynomial]; the worked cobase-change example restricts the index set along an embedding, which under the index-as-sort identification [ibid., ¶ 5.4] is the restriction of a signature to a sub-sort-set [ibid., ex 8.109].

**The forgetful direction is generated, not written.** The projection from an ornamented functor to its base is defined once by cases on ornament codes [ibid., def 8.32]; post-composed with the initial algebra it is the ornamental algebra [ibid., def 8.34], and its catamorphism is the forgetful map [ibid., def 8.35] — worked as `length` for the list ornament and cardinality for the finite-set ornament [ibid., ex 8.36, ex 8.37].

**Algebraic ornaments and reornaments internalise "indexed by the result of a fold".** For an algebra α, the algebraic ornament indexes a description by the result of its catamorphism, with `µ D^α (k,x) ≅ (t : µ D k) × ⦇α⦈ t = x` [ibid., ¶ 8.38, def 8.41]; vectors are lists algebraically ornamented by length [ibid., ex 8.44].
Categorically this is the refinement functor of [@atkey-johann-ghani-2012-refining], as the source's own statement titles attribute it [ibid., def 8.95, thm 8.97].
The reornament — extension separated from structure, duplicated data deleted — is what makes the transported constructors trivial to supply [ibid., def 8.59, def 8.61, def 8.62, rem 9.74].

**The functional layer transports operations, asking only for the new information.** A patch type internalises the coherence square `f ∘ forget = forget ∘ f⁺` by indexing, so no coherence proof is written [ibid., def 9.41], and is isomorphic to the pair of a lifting and its proof [ibid., lem 9.45]; recursion patterns transport — `lift-fold` and `lift-ind` over their coherent-algebra prerequisites, and `lift-case` [ibid., def 9.62, def 9.63, def 9.66, def 9.67, def 9.68] — and constructors transport by supplying only what the refinement added [ibid., def 9.73].
This is the reusability payoff the source's subtitle names.

**The sharp limit, stated because it decides what ornaments are _for_.** A Cartesian morphism preserves arity exactly — operations can be extended, arity cannot change [ibid., ¶ 8.81, def 8.70] — and the non-example is decisive: binary trees cannot ornament natural numbers, because arity 2 cannot map to arity 1 [ibid., non-ex 8.11].
**An ornament therefore carries cross-signature _refinement_, never cross-signature _interpretation_**: it cannot send an operation to a derived term of another signature.
The general container morphism that would relax this is named by the source and declined [ibid., rem 8.71]; that remark co-names the polynomial-functor line [@gambino-kock-2013-polynomial] and the containers line's paper "∂ for Data: Differentiating Data Structures" (Abbott, Altenkirch, McBride, and Ghani; Fundamenta Informaticae 65(1–2), 2005 — not yet in the register, and not the registered three-author containers paper), and theory-to-theory translation proper is the multiversal line's subject [@fiore-hamana-2013-multiversal].
A later pass that wants interpretation must import from those sources; nothing in this document supplies it.

**And a levitation caveat travels with the whole import.** Ornaments are ordinary programs over descriptions _because_ descriptions levitate — the source's ornament chapters need "no change or adaptation to the meta-theory" [ibid., rem 8.14] — but the levitation theorem itself is proved at the non-indexed universe [ibid., thm 6.28, thm 6.96], while for the indexed universe the manoeuvre is sketched, in two retained variants, and not proved [ibid., ¶ 6.32].
Since ornaments live on the indexed universe [ibid., rem 8.13], "ornaments compose with levitation" is theorem-grade one rung below where the composition is needed, and this document does not upgrade it.

## The requirements ledger

**Each row states one construct, the machinery it requires at the statement that demands it, and what breaks or weakens without it.** The rows are written to be consumed by the signature-generalization lane, and each has a stable anchored identifier so a decision there can cite one without quoting it.
**Numbering is stable**: retiring a row leaves its number unused.
Every "where gandr stands" clause was verified against the tree at write time at the named module and symbol.

### ornament-requirement-01

**A sort-indexed description universe.**

_The construct._ `IDesc I` with the sorted recursive occurrence `var i` [@dagand-2013-cosmology, def 5.8, fig 5.1] and the multi-sorted currying `func I J = J → IDesc I` [ibid., def 5.12].

_What breaks without it._ The ornament universe has nothing to be defined over [ibid., def 8.17] — and, independently, the n-sorted `sign` block cannot be expressed at all [ibid., rem 4.12, ex 5.34].
This row is the shared gate.

_Where gandr stands today._ `Code::Var` is nullary and is the only former `Code::is_recursive` counts as a recursive occurrence — the method reports whether a code contains one (`theory-levitation`'s `code` module); cross-datatype references are non-recursive `Code::Field` leaves over `ValueTypeRef::Ctor`.

### ornament-requirement-02

**A reindexing function with its inverse image.**

_The construct._ The `u : I → K` frame and the `var (i : u⁻¹ k)` refinement code [ibid., def 8.17]; the inverse image is `f⁻¹ b = (a : A) × f a = b` [ibid., rem 5.67].

_What breaks without it._ Index refinement — the half of ornamentation that makes `Fin` from `Nat` [ibid., ex 8.26] — is unavailable, leaving extension only.

_A dependency inside the ledger._ The inverse image is itself an equality-indexed Σ, so this row already consumes the equality former of [[#ornament-requirement-06]], in a source that otherwise stays agnostic about which equality it is [ibid., rem 5.37].

### ornament-requirement-03

**A Σ code with an arbitrary set domain and a dependent continuation.**

_The construct._ `insert (S : SET)(D⁺ : S → Orn D u)`, interpreting to `Σ S λs. ⟦D⁺ s⟧` [ibid., def 8.17, fig 8.1b].

_What breaks without it._ Nothing can be _added_ by an ornament; the extension half dies with it.

_Where gandr stands today, and why this row is a tension rather than a gap._ `Code::Field` is a non-dependent leaf over a symbolic value type, and the first-order fragment is chosen deliberately so that code equality stays decidable (`theory-levitation`'s crate-level contract; `CodeInterner` and `CodeId` key content-addressed interning on that decidability).
`insert` over an arbitrary `S : SET` with a dependent continuation is exactly what the fragment excludes.
The source flags the corresponding price from its own side [ibid., rem 4.7, rem 5.9, ex 7.96], so the fragment choice is ratified and this row is its cost, to be decided — not discovered — at the gate.

### ornament-requirement-04

**Index computation, for `delete`.**

_The construct._ `delete (s : S)(T⁺ : Orn (T s) u)` — dropping a field whose value the index already determines [ibid., def 8.17]; this internalises Brady forcing and detagging [ibid., def 5.42, def 5.43, ex 8.29, ex 8.30].
The `delete` code is the source's own addition over the original ornament universe [ibid., ¶ 8.16, ¶ 8.28] — an importer will not find it in the wider ornament literature.

_What weakens without it._ Reornaments carry spurious equality constraints and duplicated data — the source says explicitly that this is what the naive construction does [ibid., ¶ 8.57].
Weakens rather than breaks.

### ornament-requirement-05

**The catamorphism and the canonical lifting, as programs over descriptions.**

_The construct._ The canonical lifting `□D` [ibid., def 5.23] and the catamorphism derived from induction via `replace` [ibid., def 6.44, ¶ 6.45], both consumed by the algebraic ornament [ibid., def 8.41].

_What breaks without it._ No algebraic ornament, hence no reornament [ibid., def 8.62], hence no patch type [ibid., def 9.41] — the entire functional layer.

_Where gandr stands today._ The generic layer holds host-side generic programs — `generic_eq`, `serialize_value`, `serialize_desc` (`theory-levitation`'s `generic` module) — and no catamorphism and no canonical lifting.

### ornament-requirement-06

**A propositional equality the algebraic ornament can insert.**

_The construct._ The constraint `α xs = x` inserted by `D^α` [ibid., def 8.41] and the coherence isomorphism it justifies [ibid., ¶ 8.38, thm 8.97].

_What breaks without it._ The coherence property is unstatable, and with it the whole patch story.
The source requires the equality to pre-exist and stays agnostic about which one it is [ibid., rem 5.37]; choosing it is gandr's decision at the gate.

### ornament-requirement-07

**A tagged variant of the ornament universe, so constructor names survive. — An owed construction, not an import.**

_The construct._ **The source does not build this.** It records that ornamentation loses constructor names and proposes the fix without executing it: a universe of ornaments specialised to tagged descriptions, asking for an enumeration of constructor tags in bijection with the original [ibid., rem 8.23].

_What breaks without it._ gandr's constructor identity is nominal — `CtorDesc.name` and the minted identity discipline (`theory-levitation`'s `desc` module) — so every ornamentation would destroy exactly the datum gandr keys identity on.

### ornament-requirement-08

**An elaboration of ornaments from surface syntax. — An owed construction, not an import.**

_The construct._ **The source does not build this either.** Its ornament notation is introduced informally with the translation declared out of scope [ibid., rem 8.21], and further work confirms the datatype elaborator would have to be extended to ornaments [ibid., ¶ 10.7]; the functional layer's lifting commands are in the same position [ibid., rem 9.65].
Datatype elaboration itself _is_ specified and proved [ibid., def 7.5, thm 7.29, def 7.37, thm 7.78], so the asymmetry is real and specific.

_What this means for gandr._ An ornament surface for the `sign` block is gandr's own design work, priced accordingly — with the source's elaboration chapter as the model to extend, not a port.

### ornament-requirement-09

**Polarity. — An absence in the source, not a gap in gandr.**

_The construct._ None.
Coinduction is out of scope [ibid., rem 4.64]; greatest fixpoints and the lifting of coinductive definitions are further work [ibid., ¶ 10.6, ¶ 10.10].

_What this bounds._ Ornaments over gandr's `DeclPolarity::Codata` descriptions have no source statement, and this source is not the one to cite for the ν decoder.
Mixed-polarity signatures additionally require _internal_ fixpoint codes — the source's µ is an external operator applied to a whole description [ibid., def 4.38, def 5.16], and it says alternating fixpoints would need the internalisation [ibid., ¶ 10.6] — a universe change, not a decoder change, recorded here as a design input for the signature generalization's mixed-polarity ruling.

### ornament-requirement-10

**A universe of function types, for the functional layer.**

_The construct._ The deliberately minimal `Type` universe of first-order function types the functional ornaments decorate [ibid., def 9.24, def 9.29], with its declined extensions named: non-inductive parameters, dependent quantifiers, and higher-order functions — the last declined because it would force a covariant-against-contravariant distinction in ornamentation that a first-order universe can overlook [ibid., ¶ 9.25, ¶ 10.10].

_Where gandr stands today, and where it is ahead._ The nearest analogue is the operation layer's `BridgeArity` (`theory-levitation`'s `arity` module), which is an arity, not a coded function type — and which carries multi-output structure that has no counterpart in the source's `Type` at all.
This row is where gandr would extend the source rather than adopt it.

## What can be built before the machinery lands

**Verified against the tree at write time at the named modules and symbols; what is verified is that the symbols exist with the stated shape, not that any body behaves as documented.**

* **The tagged-description normal form is already the as-built shape, and it was the right call for reasons the source states.** The source's tagged form `tagDesc = (E : EnumU) × π E (λ_. Desc)` [@dagand-2013-cosmology, def 4.23] is `DataDesc`'s constructor table — `CtorDesc { name, code, result, attrs }` (`theory-levitation`'s `desc` module) — and the source's rationale for enforcing constructor form, that it "ease[s] the implementation of datatype transformations" [ibid., ¶ 4.24], is the ornament argument banked in advance.
* **The operation layer is already container-shaped, and the constructor layer is not — the `sign` block should make them agree.** `BridgeArity` carries inputs, per-monomial factor counts, a source map, a destination map, and outputs (`theory-levitation`'s `arity` module) — a container in the source's sense [ibid., def 5.53], with the correspondence exhibited rather than asserted: the destination map's fibres give the operations over the output ports, the factor counts give each operation's arity set, and the source map is the sort map into the input ports — while the constructor layer has no sort map, only the waiting output-sort slot `CtorDesc.result`.
  This asymmetry is a design input for the signature generalization, recorded here because the ornament read is what exposed it.
* **An extension-only forgetful fold is host-implementable today — stated as buildable, not as recommended.** An `insert`-only ornament over the current fragment is a per-constructor field deletion, and the forgetful projection on the non-indexed codes is a structural fold over `Code` [ibid., def 8.32].
  Building it before [[#ornament-requirement-01]] lands would fix an interface that [[#ornament-requirement-02]] then changes; it buys only the unindexed examples [ibid., ex 8.22, ex 8.25].
* **The derivable-property mechanism is buildable now and is the principled home for gandr's derive story.** Declare a sub-universe of codes, a decidable membership test, and a derive function [ibid., def 7.93, def 7.94, ex 7.96, fig 7.7]; over gandr's `Code` and the well-formedness pass `check_desc` (`theory-levitation`'s `wellformed` module) this needs no dependent machinery at all.
* **Decidable code equality and content-addressed interning are ratified, not challenged.** The source flags exactly this property as what first-order codes buy [ibid., rem 4.7, rem 5.9, ex 7.96]; gandr's derived `Code` equality (the `code` module) and the `CodeInterner`/`CodeId` interning keyed on it (the `intern` module) are that purchase under the crate-level first-order contract, and [[#ornament-requirement-03]] is its price.

## What stays out of scope regardless

* **Induction-recursion and W-type modelling.** The source uses them only to model levitation in a meta-meta-theory [ibid., rem 6.79, def 6.82, thm 6.96]; they justify self-description on paper, are not a kernel requirement, and the model does not even capture the eliminator gadget [ibid., rem 6.98].
* **Extensional reasoning.** The indexed-family semantics is developed in a model with equality reflection excluded from the type theory itself [ibid., ¶ 5.50, rem 5.51]; the computational content is intensional and only the proofs are extensional.
* **Coq-completeness results** [ibid., lem 7.32, thm 7.34, conj 7.80] — they measure the source against Coq's `Inductive`, which is not a gandr obligation.
* **Polykinded programming**, declined by the source itself [ibid., ¶ 6.106].

## Source and confidence

**Written against a single primary source, read in full for the signature-generalization sweep on 2026-08-02, with three register entries cited for routing only.**

The primary [@dagand-2013-cosmology] was read cover to cover — 263 pages — at graded depth: theorem grade on the inductive-types, inductive-families, bootstrapping, ornaments, and functional-ornaments chapters; theorem grade on the elaboration chapter's numbered statements with its intermediate lemma run read as a block for shape; ergonomics grade on the notation chapter; orientation grade on the introduction; and, in the type-theory chapter, theorem grade on the enumeration universe only, with the remaining metatheory at triage grade; the conclusion chapter was read in full, with no depth grade recorded by the read.
Every statement number above was read in the held PDF; none is recalled.
The grades are the read's and are not upgraded here.

The three routing citations are not reads: [@gambino-kock-2013-polynomial] is cited as the named home of the general container morphism the source declines [@dagand-2013-cosmology, rem 8.71] and as the attributed source of the frame structure [ibid., ¶ 8.74, ¶ 8.75], its locator transcribed from the source's bibliography and verified against the publisher record; [@atkey-johann-ghani-2012-refining] is cited only as the attributed home of the refinement functor, its locator transcribed the same way and verified against the publisher and arXiv records; [@fiore-hamana-2013-multiversal] was read at theorem grade in the same sweep and is cited only as where theory-to-theory translation lives.

**What is verified against the tree at write time.** The as-built claims name their modules and symbols in `theory-levitation`: the `code` module's `Code::Var` and `Code::is_recursive`, the `desc` module's `CtorDesc` and `DataDesc` with `DeclPolarity`, the `arity` module's `BridgeArity`, the `intern` module's `CodeInterner` and `CodeId`, the `generic` module's generic programs, and the `wellformed` module's `check_desc`.
What is verified is each symbol's existence, shape, and documented contract — not that any body decides what its documentation says it decides.

**What is not verified, and is load-bearing for the ledger.** No requirement row has been checked against an implementation of ornaments, because none exists anywhere in gandr; each row is derived from the source's definition at its own statement number.
A row's "what breaks" clause is a structural consequence of the cited definition, not a failure anyone has exhibited.

**One as-built claim rests on read code rather than read prose, and it is the load-bearing one.** That the as-built universe cannot express an n-sorted signature is exhibited by `Code::Var`'s nullarity and by `Code::is_recursive`'s contract — containment of a `Code::Var` is the only recursion the method reports — not inferred from documentation.

**No recorded corpus claim is contradicted by this document.** The container position of [[../../metatheory#Cellular data — descriptions, cells, and computads]] is ratified by the source rather than revised, and the polarity rows record absences in the source, not judgements against gandr's recorded codata stance.
