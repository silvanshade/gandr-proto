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
//! Hom(a, c))`
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
//! ⟦comp(id(a), f)⟧, f))`
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
mod tests
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
    /// **The type variables are named `t u v` and `p q` rather than `a b c`,
    /// and that is a workaround rather than a style.** Type substitution is not
    /// capture-avoiding, so a caller whose variables collide with an
    /// operation's own binders is instantiated wrongly; the composition's
    /// binders are `a b c`, and a category's laws written the obvious way
    /// collide with all three. `gandr-ijdw`. When it lands, the names here
    /// should go back to the design's spelling, and the fact that they *can* is
    /// part of that repair's witness.
    pub const SETOID_CAT: &str = r#"module SetoidCat : #{
  type Ob = Type,
  type Hom(a : Ob, b : Ob) = U[ω] (a -> F b),
  id : U[ω] ((a : Ob) -> F Hom(a, a)),
  comp : U[ω] ((a : Ob) -> (b : Ob) -> (c : Ob) -> Hom(a, b) -> Hom(b, c) -> F Hom(a, c)),
  unitL : U[ω] ((a : Ob) -> (b : Ob) -> (f : Hom(a, b)) -> F Path(Hom(a, b), comp(a, a, b, id(a), f), f)),
  unitR : U[ω] ((a : Ob) -> (b : Ob) -> (f : Hom(a, b)) -> F Path(Hom(a, b), comp(a, b, b, f, id(b)), f))
} {
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
}"#;

    /// The signature elaborates, and no field's type mentions the gradual
    /// unknown.
    ///
    /// The unknown clause is part of the claim rather than hygiene beside it: a
    /// field elaborated at a type mentioning an unknown is consistent with
    /// everything, so a signature carrying one states less than it appears to.
    #[test]
    fn the_model_signature_elaborates_without_an_unknown()
    {
        let _ = (MODEL_CAT_SHAPE, SETOID_CAT);
        todo!("gandr-0ika")
    }

    /// The indexed sort survives as a **family of arity two**, not as a single
    /// function type.
    ///
    /// `type Hom : Ob -> Ob -> Type` and `type Hom = Ob -> Ob -> Type` are
    /// different declarations, and reading the first as the second would bind
    /// `Hom` to a type the source does not state.
    #[test]
    fn the_indexed_sort_is_a_family_of_arity_two()
    {
        todo!("gandr-wvd.6.2")
    }

    /// The discrete-setoid instance matches the signature, and both unit laws
    /// are members of the resulting module.
    ///
    /// Members rather than top-level definitions: the instance is a module, so
    /// the claim is about its components, which is what the member-level corpus
    /// expectations exist to state.
    #[test]
    fn the_setoid_instance_matches_the_model_signature()
    {
        todo!("gandr-0ika")
    }

    /// The law fields' endpoints name the **model's own** operations.
    ///
    /// This is the claim that separates a law of the model from a law about the
    /// instance's private helpers. It is stated as its own witness because the
    /// two spellings elaborate to different terms while both type-check once
    /// the embedding resolves, so nothing downstream would notice the
    /// substitution.
    #[test]
    fn the_law_endpoints_name_the_models_own_operations()
    {
        todo!("gandr-rson")
    }

    /// No index or type in the claim path is **misrepresented**: none mentions
    /// the gradual unknown, and none holds a type former where the declaration
    /// puts a value.
    ///
    /// Two clauses because one token is not the invariant. An unknown is
    /// visible to `Ty::mentions_unknown`; a type atom standing in a value
    /// index position reads as a clean successful elaboration to every
    /// instrument in the tree, which is the sibling that recruits a false
    /// record rather than its own correction.
    #[test]
    fn no_index_in_the_claim_path_is_misrepresented()
    {
        todo!("gandr-0ika")
    }
}
