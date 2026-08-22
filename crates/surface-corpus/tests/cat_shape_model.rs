//! `Model(CatShape)` written out as a module signature, and the category of
//! setoids as an instance of it.
//!
//! # What this file is for
//!
//! The higher-cells flagship is the weak category: two sorts, one of them
//! indexed, three laws, two coherences. The staged design gives `Model(S)` as a
//! signature expression **computed by elaboration** from a `sign` block. This
//! file carries the **hand-written** signature instead, and the instance is
//! checked against it.
//!
//! That order is deliberate. Deriving the signature from the `sign` block first
//! would put a second, unverified translation between the flagship's claim and
//! its witness: the instance would be witnessing agreement with a derivation
//! nothing had checked. Written by hand, the signature is an artifact a reader
//! can compare against the design clause by clause, and the derivation's own
//! acceptance becomes definitional agreement **with this signature**. So this
//! is not scaffolding for the derivation; it is the derivation's oracle.
//!
//! **What the flagship claim covers, and what it does not.** The witness here
//! exercises the instance against the *stated* signature. It does **not**
//! exercise the `sign`-block-to-`Model(S)` derivation, which is a separate
//! rung. A reader who wants to know whether the derivation works must look at
//! that rung, not at this file.
//!
//! # The correspondence, clause by clause
//!
//! The staged design's shape, quoted in the spelling it uses:
//!
//! ```text
//! sign CatShape {
//!   sort Ob : Type
//!   sort Hom(dom: Ob, cod: Ob) : Type
//!
//!   oper id : (a : Ob) --> Hom(a, a)
//!   oper comp : (f : Hom(a, b), g : Hom(b, c)) --> Hom(a, c)
//!
//!   rule unitL : comp(id(a), f) ==> f
//!   rule unitR : comp(f, id(b)) ==> f
//!   rule assoc : comp(comp(f, g), h) ==> comp(f, comp(g, h))
//!
//!   rule triangle : (assoc(f, id(b), g) then comp(f, unitL(g))) ==> comp(unitR(f), g)
//!   rule pentagon : …
//! }
//! ```
//!
//! and its four `Model(S)` clauses, by dimension:
//!
//! | member of `S`                                 | field of `Model(S)`                                        |
//! | --------------------------------------------- | ---------------------------------------------------------- |
//! | `sort X(Δ)`                                   | `type X : Δ → Type`                                         |
//! | `oper f : T̄ --> X`                            | `val f : U_ω (T̄ → F X)`                                     |
//! | `rule r : l ==> t` at sort `X`                | `val r : U_ω (Π Γ_r → F Path(X, ⟦l⟧, ⟦t⟧))`                 |
//! | `rule m : ρ ==> ρ′` at rule-sorted endpoints  | `val m : U_ω (Π Γ_m → F Path(Path(X, ⟦l⟧, ⟦t⟧), ⟦ρ⟧, ⟦ρ′⟧))` |
//!
//! Applied to `CatShape`, member by member. Each row states the design clause
//! it comes from and, where this signature makes a choice the design leaves
//! open, says so and says why — because the derivation rung will be held to
//! these choices.
//!
//! ## `sort Ob : Type` → `type Ob : Type`
//!
//! A nullary kinded component. Abstract rather than manifest: an instance
//! supplies it, and `Model(CatShape)` states only its kind.
//!
//! ## `sort Hom(dom: Ob, cod: Ob) : Type` → `type Hom : Ob -> Ob -> Type`
//!
//! The indexed sort, and the reason the weak category is a stage-1 flagship
//! rather than a near-term one: `Δ → Type` for a non-empty `Δ` is a type
//! **family**, so the signature grammar needs a kinded component and universe
//! formation together.
//!
//! **The design's `Δ → Type` is spelled as an arrow spine, and the binder names
//! are dropped.** `sort Hom(dom: Ob, cod: Ob)` names its parameters; the kind
//! `Ob -> Ob -> Type` does not. That is a real loss and it is deliberate: at
//! this rung a kind is an arrow spine ending in `Type`, nothing in the field
//! types below refers to a sort parameter by name, and carrying names into an
//! abstract kind would invent a binding form the grammar does not have. **A
//! later dependent kind — one whose codomain mentions an earlier parameter —
//! cannot be written this way, and the derivation rung inherits that limit.**
//!
//! ## `oper id : (a : Ob) --> Hom(a, a)` → `val id : U_ω (Π(a : Ob) F Hom(a, a))`
//!
//! **The design's `T̄ → F X` clause does not say where an indexed sort's own
//! variables are bound, and this signature binds them.** `id`'s declaration
//! mentions `a : Ob` explicitly, so `a` is a `Π`-bound parameter of the field.
//!
//! ## `oper comp : (f : Hom(a, b), g : Hom(b, c)) --> Hom(a, c)`
//!
//! → `val comp : U_ω (Π(a : Ob) Π(b : Ob) Π(c : Ob) Hom(a, b) -> Hom(b, c) -> F
//! Hom(a, c))`.
//!
//! **Here the design is silent and the choice is load-bearing.** `comp`'s
//! declaration mentions `a`, `b`, and `c` and binds none of them: they are the
//! indices its argument sorts carry. This signature binds every free sort
//! variable of an operation's declaration as a `Π` parameter, **in order of
//! first occurrence left to right through the argument list and then the
//! result**. For `comp` that is `a`, `b`, `c`.
//!
//! Order of first occurrence rather than alphabetical, because it is the order
//! a reader recovers from the declaration without a convention, and because it
//! degrades correctly for an operation whose result introduces an index its
//! arguments do not.
//!
//! ## `rule unitL : comp(id(a), f) ==> f`
//!
//! → `val unitL : U_ω (Π(a : Ob) Π(b : Ob) Π(f : Hom(a, b)) F Path(Hom(a, b),
//! ⟦comp(id(a), f)⟧, f))`.
//!
//! The carrier is the sort the rule is stated at — `Hom(a, b)`, the sort of
//! both endpoints. `Γ_r` is the rule's own variable context: the sort variables
//! its endpoints mention, plus the 1-cell variables, in first-occurrence order.
//!
//! **`⟦l⟧` is the model reading, and it names the model's own fields.** The
//! design is explicit that a rule's endpoint composite is definable *because
//! rules and operations are named fields*, so `⟦comp(id(a), f)⟧` mentions this
//! signature's `comp` and `id` and nothing else. An endpoint spelled in terms
//! of an instance's private helper would make the law a statement about the
//! helper rather than about the model, so it is not an available spelling here.
//!
//! **The implicit index arguments are supplied.** `comp` takes three sort
//! parameters, so the endpoint is `comp(a, a, b, id(a), f)`: `id(a) : Hom(a,
//! a)` fixes the middle index at `a`. The mirrored law takes `comp(a, b, b, f,
//! id(b))`. Those two assignments are the ones the endpoint checker admits and
//! the ones it refuses a permutation of.
//!
//! ## `rule assoc`, `rule triangle`, `rule pentagon`
//!
//! `assoc` follows the same clause one dimension along the composite.
//! `triangle` and `pentagon` are the fourth clause — `Path` over a `Path`, both
//! faces composites of the law fields themselves. They are stated in the
//! signature because the shape states them; whether the instance's coherence
//! fields are inhabited at grade `0` is the instance's business, not the
//! signature's.
//!
//! # The instance
//!
//! The category of **discrete** setoids: an object is a type, a hom is a
//! function, and equality is `Path`. Naming it plainly, because the
//! discreteness is the part a later reader would otherwise have to rediscover —
//! a discrete setoid's equivalence is `Path` itself, so "respects the relation"
//! is congruence and every function satisfies it. A setoid with a *chosen*
//! equivalence needs `El : Setoid → Type`, a large elimination the surface has
//! no path to; that is the reason this instance is the discrete one and not a
//! decision to revisit casually.

#[cfg(test)]
pub mod tests
{
    /// `Model(CatShape)`, as the module signature grammar spells it.
    ///
    /// Held as one string so the correspondence above can be read against a
    /// single artifact, and so the derivation rung's acceptance test has
    /// exactly one thing to agree with.
    ///
    /// **`triangle` and `pentagon` are absent, by decision.** Their faces are
    /// composites in the boundary language — `then` and `cong` over the law
    /// fields — and an identity-type endpoint has no spelling for either, so
    /// writing them would need a form the surface does not carry. Stating them
    /// as anything else would state a weaker theorem under their names, which
    /// is the one thing a signature meant as an oracle may not do. The
    /// derivation rung inherits the same gap and should refuse them by name
    /// rather than emit a degraded field.
    pub const MODEL_CAT_SHAPE: &str = r#"#{
  type Ob : Type,
  type Hom : Ob -> Ob -> Type,
  id : U[ω] ((a : Ob) -> F Hom(a, a)),
  comp : U[ω] ((a : Ob) -> (b : Ob) -> (c : Ob) -> Hom(a, b) -> Hom(b, c) -> F Hom(a, c)),
  unitL : U[ω] ((a : Ob) -> (b : Ob) -> (f : Hom(a, b)) -> F Path(Hom(a, b), comp(a, a, b, id(a), f), f)),
  unitR : U[ω] ((a : Ob) -> (b : Ob) -> (f : Hom(a, b)) -> F Path(Hom(a, b), comp(a, b, b, f, id(b)), f)),
  assoc : U[ω] ((a : Ob) -> (b : Ob) -> (c : Ob) -> (d : Ob) -> (f : Hom(a, b)) -> (g : Hom(b, c)) -> (h : Hom(c, d)) -> F Path(Hom(a, d), comp(a, c, d, comp(a, b, c, f, g), h), comp(a, b, d, f, comp(b, c, d, g, h))))
}"#;

    /// The discrete-setoid instance, ascribed to `MODEL_CAT_SHAPE` with its two
    /// type components made manifest.
    ///
    /// `Ob = Type` and `Hom(a, b) = U[ω] (a -> F b)`: an object is a type, a
    /// hom is a function. The manifest spelling is what lets the law fields
    /// reduce — under the abstract signature `Hom` has no structure for
    /// conversion to see — so this is transparent ascription, and opacity
    /// is sealing's question rather than this instance's.
    ///
    /// **The type variables are named `t u v` and `p q` rather than `a b c`.**
    /// Type substitution now renames a capturing type binder apart
    /// (`gandr-ijdw`), so the design's own spelling is admissible here.
    ///
    /// **The spelling is retained because restoring it would witness nothing
    /// *here*, and that was measured rather than assumed.** Rewriting this
    /// module and the corpus entry to `a b c` and running them against an
    /// `identity.rs` with the rename ablated leaves both green, bracketed
    /// baseline / ablated / restored under one run.
    ///
    /// **The instrument is not blind; this program does not exercise the
    /// path.** The same bracket's positive control — renaming a law member —
    /// turns the corpus walker red on this very file, so the walker is live
    /// over it. And a corpus witness written *for* the capture path does
    /// separate the two sides: refused without the repair, accepted with it.
    /// The claim here is only that **these two spellings of this module** are
    /// indistinguishable, which is a fact about the program rather than about
    /// the surface.
    pub const SETOID_CAT_SIGNATURE: &str = r#"#{
  type Ob = Type,
  type Hom(a : Ob, b : Ob) = U[ω] (a -> F b),
  id : U[ω] ((a : Ob) -> F Hom(a, a)),
  comp : U[ω] ((a : Ob) -> (b : Ob) -> (c : Ob) -> Hom(a, b) -> Hom(b, c) -> F Hom(a, c)),
  unitL : U[ω] ((a : Ob) -> (b : Ob) -> (f : Hom(a, b)) -> F Path(Hom(a, b), comp(a, a, b, id(a), f), f)),
  unitR : U[ω] ((a : Ob) -> (b : Ob) -> (f : Hom(a, b)) -> F Path(Hom(a, b), comp(a, b, b, f, id(b)), f))
}"#;

    /// The instance's body, held apart from its ascription so the two modules
    /// below share it as a **fact** rather than as a claim.
    ///
    /// **The divergence is unrepresentable rather than checked.** A mitigation
    /// that is two artifacts required to hold identical text is the same defect
    /// one level out: an edit reaching one and not the other leaves an
    /// unknown-scrutiny witness reading a body nobody claims anything about,
    /// beside a matching claim over a body the witness no longer mirrors, and
    /// both stay green. Deriving both modules from this one string means there
    /// is no second copy to drift, so nothing has to notice.
    ///
    /// **Why there are two modules.** A member-level type check over a signed
    /// module reads the *ascribed* field types, so it witnesses the signature
    /// and says nothing about what the bodies elaborated to. And that gap
    /// cannot be argued away by saying the body must have matched: the gradual
    /// unknown is consistent with everything, so a body that degraded to
    /// `Unknown` still matches an ascription naming a real type. Left alone,
    /// the unknown clause of the flagship claim would have passed by
    /// witnessing that the signature — written by hand, on this branch — has
    /// no unknown in it.
    ///
    /// So the claim splits over two artifacts, and the split is stated rather
    /// than presented as design: the **signed** module carries the matching
    /// claim, the **unsigned twin** carries the unknown claim, because an
    /// unsigned module's reported record type is exactly what its bodies
    /// elaborated to. `gandr-64oy` carries what would replace the twin.
    ///
    /// **The type variables are named `t u v` and `p q` rather than `a b c`.**
    /// Type substitution now renames a capturing type binder apart
    /// (`gandr-ijdw`), so the design's own spelling is admissible here.
    ///
    /// **The spelling is retained because restoring it would witness nothing
    /// *here*, and that was measured rather than assumed.** Rewriting this
    /// module and the corpus entry to `a b c` and running them against an
    /// `identity.rs` with the rename ablated leaves both green, bracketed
    /// baseline / ablated / restored under one run.
    ///
    /// **The instrument is not blind; this program does not exercise the
    /// path.** The same bracket's positive control — renaming a law member —
    /// turns the corpus walker red on this very file, so the walker is live
    /// over it. And a corpus witness written *for* the capture path does
    /// separate the two sides: refused without the repair, accepted with it.
    /// The claim here is only that **these two spellings of this module** are
    /// indistinguishable, which is a fact about the program rather than about
    /// the surface.
    pub const SETOID_CAT_BODY: &str = r#"
  def ident(t : Type, x : t) -> F t { ret x }

  def compose(t : Type, u : Type, v : Type, f : U[ω] (t -> F u), g : U[ω] (u -> F v), x : t) -> F v {
    run y <- f(x);
    g(y)
  }

  def id(t : Type) -> F (U[ω] (t -> F t)) { ret thunk { ident(t) } }

  def comp(t : Type, u : Type, v : Type, f : U[ω] (t -> F u), g : U[ω] (u -> F v)) -> F (U[ω] (t -> F v)) {
    ret thunk { compose(t, u, v, f, g) }
  }

  def unitL(p : Type, q : Type, f : U[ω] (p -> F q)) -> F Path((U[ω] (p -> F q)), comp(p, p, q, id(p), f), f) {
    ret here(f)
  }

  def unitR(p : Type, q : Type, f : U[ω] (p -> F q)) -> F Path((U[ω] (p -> F q)), comp(p, q, q, f, id(q)), f) {
    ret here(f)
  }
"#;

    /// Nothing here is asserted in Rust, and that is deliberate.
    ///
    /// **What is witnessed lives where it can be observed**: the corpus entry
    /// `examples/model/higher-cells/cat-shape-setoids.gandr` states, through
    /// member-level directives, that the module's operations and both unit laws
    /// are present and carry no gradual unknown; and
    /// `flagship_probe::both_unit_laws_check_in_model_faithful_form` states
    /// that their endpoints name the model's own operations, paired with
    /// the helper spelling that must not move with them. Two witnesses for
    /// one claim drift, so this file carries neither a second copy.
    ///
    /// **What is owed is owed on something specific.** The signature above
    /// cannot be parsed at all: its operation clause binds every free sort
    /// variable as a dependent parameter, and the surface type grammar has no
    /// dependent function type — `gandr-3jus`. Until then an instance has
    /// nothing to be an instance *of*, which is why the corpus witnesses a
    /// module that presents the category of setoids rather than an instance of
    /// anything.
    ///
    /// **And the misrepresentation claim's second clause is vacuous here rather
    /// than inspected.** No index in the claim path holds a type former where a
    /// value belongs, because this module has no value indices at all: its
    /// objects are types and its homs expand by substitution. A check that
    /// cannot fire is not a check, and a claim leaning on one is weaker than it
    /// reads.
    #[test]
    fn the_signature_is_read_rather_than_asserted()
    {
        assert!(
            MODEL_CAT_SHAPE.contains("type Hom : Ob -> Ob -> Type"),
            "the oracle states the indexed sort as a kinded component"
        );
        assert!(
            SETOID_CAT_SIGNATURE.contains("type Hom(a : Ob, b : Ob)"),
            "the instance supplies it as a manifest family"
        );
        assert!(
            !SETOID_CAT_BODY.contains("assoc"),
            "the third law is absent, and the corpus entry says on what"
        );
    }
}
