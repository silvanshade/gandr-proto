//! Value and computation types of core CBPV (`type-system.md` §"Types").
//!
//! The polarity split is structural: value types (positive) classify values,
//! computation types (negative) classify computations, and the `U`/`F`
//! adjunction mediates. Later stages extend both sorts (unions, `@w`,
//! intersections, `∀`); the enums are `non_exhaustive` to keep that road open.

use alloc::collections::BTreeMap;
use alloc::rc::Rc;
use alloc::vec::Vec;
use core::fmt;

use crate::boundary::BinderName;
use crate::boundary::DataTypeName;
use crate::boundary::SealComponentName;
use crate::boundary::SealDeclarationName;
use crate::boundary::TypeAtomName;
use crate::boundary::TypeSerial;
use crate::effect::EffectRow;
use crate::grade::Grade;
use crate::syntax::Value;

/// The **core-local nominal identity** of a declared datatype (ADR-80).
///
/// The minted 0-cell a `data D(ā) { … }` declaration stands for (ADR-80
/// Decision 4), keyed by a monotone `serial` (declaration order within one
/// elaboration) plus the datatype's `name`.
///
/// It is **structurally identical** to `gandr_desc::NominalId` (serial + name),
/// but a **separate, core-local** type: `gandr_desc` depends on `gandr_core`
/// (for [`Grade`]), so the core cannot name `gandr_desc::NominalId` without a
/// dependency cycle. `gandr_pipeline` — which depends on both — is the single
/// seam that maps the desc id to this core id when lowering a declaration into
/// the runtime (ADR-80 Decision 4). Two datatypes with structurally-identical
/// carriers but distinct declarations mint distinct ids, giving generativity
/// (`Celsius` and `Fahrenheit` do not interchange) as a first-class type
/// comparison — the "third identity discipline" ADR-50 Decision B names, keyed
/// on the minted 0-cell.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DataId
{
    /// The monotone serial assigned in declaration order within one
    /// elaboration.
    serial: TypeSerial,
    /// The datatype's declared name.
    name: String,
}

impl DataId
{
    /// Mints a core-local nominal id with the given `serial` and `name`.
    ///
    /// # Contract
    /// - ensures: preserves `serial` and `name` exactly; two ids are equal iff
    ///   both fields are.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn new<'source, S, N>(
        serial: S,
        name: N,
    ) -> Self
    where
        S: Into<TypeSerial>,
        N: Into<DataTypeName<'source>>,
    {
        Self {
            serial: serial.into(),
            name: name.into().as_ref().to_owned(),
        }
    }

    /// Return the declaration-order serial.
    #[inline]
    #[must_use]
    pub fn serial(&self) -> TypeSerial
    {
        self.serial
    }

    /// Return the declared datatype name.
    #[inline]
    #[must_use]
    pub fn name(&self) -> DataTypeName<'_>
    {
        DataTypeName::from(self.name.as_str())
    }
}

/// Where one sealing happened: the declaration whose opaque ascription ran, and
/// the abstract type component it sealed.
///
/// The pair is the minting key, so it is also the thing a replay re-derives. It
/// is deliberately made of **source-level names** rather than positions: a
/// position would change under an edit that does not change what was sealed,
/// and a re-minting check that is sensitive to unrelated edits refutes the
/// wrong thing.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SealSite
{
    /// The module declaration whose opaque ascription minted the atom.
    declaration: String,
    /// The abstract type component of the signature the atom stands for.
    component: String,
}

impl SealSite
{
    /// The site at `component` of `declaration`.
    #[inline]
    #[must_use]
    pub fn new<'source, D, C>(
        declaration: D,
        component: C,
    ) -> Self
    where
        D: Into<SealDeclarationName<'source>>,
        C: Into<SealComponentName<'source>>,
    {
        Self {
            declaration: declaration.into().as_ref().to_owned(),
            component: component.into().as_ref().to_owned(),
        }
    }

    /// The declaration whose ascription minted here.
    #[inline]
    #[must_use]
    pub fn declaration(&self) -> SealDeclarationName<'_>
    {
        SealDeclarationName::from(self.declaration.as_str())
    }

    /// The abstract type component sealed here.
    #[inline]
    #[must_use]
    pub fn component(&self) -> SealComponentName<'_>
    {
        SealComponentName::from(self.component.as_str())
    }
}

impl fmt::Display for SealSite
{
    #[inline]
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        write!(f, "{}.{}", self.declaration, self.component)
    }
}

/// The **core-local nominal identity of a sealed abstract type**: the fresh
/// atom one opaque ascription mints for one abstract type component.
///
/// It is [`DataId`]'s sibling and deliberately so — the declared-data handle
/// already compares a minted id *before* its structural carrier, and that is
/// abstract-type behaviour. The difference is what each one carries: a `DataId`
/// names a type whose carrier lives in a decl table, and a `SealId` names a
/// type that **has no carrier recorded anywhere**. There is nothing to look up,
/// which is what opacity is.
///
/// # Identity is the site, and that is what makes freshness checkable
///
/// A seal's identity is its **site** — which declaration sealed it, and which
/// abstract type component of the signature it stands for — plus the monotone
/// `serial` minting assigned. The site is what makes re-minting
/// *deterministic*: elaborating the same program again meets the same sites in
/// the same order and mints the same serials, so "these atoms are fresh" is a
/// claim `gandr_core_checker::judgements::seal::SealTable` can re-derive rather
/// than one the elaborator asserts.
///
/// Two seals of the same component in two declarations are distinct, and two
/// seals of the same declaration's component are the *same* — which is exactly
/// declaration-granular sealing seen from the identity side.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
/// # Why the site is shared rather than copied
///
/// A `SealId` rides inside [`ValueType::Sealed`], which is cloned along every
/// type it appears in, and a value type is the widest-cloned object in the
/// checker. Holding the site behind an [`Rc`] keeps the identity two words, so
/// a sealed type costs no more to carry than an atom name — and keeps the value
/// -type enum from widening, which would grow every `Result` the checker
/// returns.
pub struct SealId
{
    /// The monotone serial assigned in minting order within one elaboration.
    serial: TypeSerial,
    /// Where this atom was minted, shared rather than copied.
    site: Rc<SealSite>,
}

impl SealId
{
    /// Mint a sealed-atom identity at `serial` for `component` of
    /// `declaration`.
    ///
    /// # Contract
    /// - requires: nothing; uniqueness is
    ///   `gandr_core_checker::judgements::seal::SealTable`'s obligation, not
    ///   this constructor's.
    /// - ensures: preserves all three fields exactly; two ids are equal iff all
    ///   three are.
    /// - provides: the identity an opaque ascription binds an abstract type
    ///   component to.
    /// - fails: never.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn new<'source, S, D, C>(
        serial: S,
        declaration: D,
        component: C,
    ) -> Self
    where
        S: Into<TypeSerial>,
        D: Into<SealDeclarationName<'source>>,
        C: Into<SealComponentName<'source>>,
    {
        Self {
            serial: serial.into(),
            site: Rc::new(SealSite::new(declaration, component)),
        }
    }

    /// Pair a serial with an already-shared site.
    #[inline]
    #[must_use]
    pub fn at_site<S>(
        serial: S,
        site: Rc<SealSite>,
    ) -> Self
    where
        S: Into<TypeSerial>,
    {
        Self {
            serial: serial.into(),
            site,
        }
    }

    /// Where this atom was minted.
    #[inline]
    #[must_use]
    pub fn site(&self) -> &SealSite
    {
        self.site.as_ref()
    }

    /// The minting-order serial.
    #[inline]
    #[must_use]
    pub fn serial(&self) -> TypeSerial
    {
        self.serial
    }

    /// The declaration whose opaque ascription minted this atom.
    #[inline]
    #[must_use]
    pub fn declaration(&self) -> SealDeclarationName<'_>
    {
        self.site.declaration()
    }

    /// The abstract type component this atom stands for.
    #[inline]
    #[must_use]
    pub fn component(&self) -> SealComponentName<'_>
    {
        self.site.component()
    }
}

/// A value (positive) type `A`.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum ValueType
{
    /// A rigid base type or type variable `X` (e.g. `Int`, `A`).
    ///
    /// Stage 1 has no binding forms for type variables, so atoms are opaque
    /// names compared by equality.
    Atom(
        /// The atom's name.
        String,
    ),
    /// The unit type `1`.
    Unit,
    /// The eager product `A × A′`.
    Prod(
        /// The first component's type.
        Rc<Self>,
        /// The second component's type.
        Rc<Self>,
    ),
    /// The tagged sum (coproduct) `A + A′`.
    Sum(
        /// The left summand.
        Rc<Self>,
        /// The right summand.
        Rc<Self>,
    ),
    /// The homogeneous list (array) `List A` — the value-model ladder's list
    /// rung (`proposal-shell-usage-surface.md` §2, ADR-40), the first rung to
    /// change the frozen §2 value-type grammar.
    ///
    /// A genuine parameterized positive former, **not** a rigid atom (the
    /// string / numeric scalars of ADR-38/39 occupy the [`Atom`] slot): the
    /// element type is its parameter. Subtyping is **covariant** in the element
    /// (`List A <: List A′` iff `A <: A′`), the same positive-former
    /// decomposition as [`Prod`] / [`Sum`]
    /// (`gandr_core_checker::discipline::subtype`). Its value is the
    /// flat-vector [`crate::syntax::Value::List`]; its eliminator the
    /// structural [`crate::syntax::Comp::ListCase`]. General value-level
    /// `μ⁺` recursive types are the deferred later rung.
    ///
    /// [`Atom`]: ValueType::Atom
    /// [`Prod`]: ValueType::Prod
    /// [`Sum`]: ValueType::Sum
    List(
        /// The element type `A`.
        Rc<Self>,
    ),
    /// The closed record `{ℓᵢ:Aᵢ}` — the value-model ladder's record rung
    /// (`proposal-shell-usage-surface.md` §2, ADR-45), a labeled, n-ary
    /// generalization of the eager product [`Prod`].
    ///
    /// A genuine positive value-type former (records are positive by analogy to
    /// products; the intersection-encoding is polarity-refuted, ∩ being
    /// negative-only). The fields are held in a [`BTreeMap`] keyed by label, so
    /// the representation is **canonical in field order** — `{a:A, b:B}` and
    /// `{b:B, a:A}` are the *same* type — and structural equality compares the
    /// label→type maps directly. Subtyping is structural **width + depth**:
    /// `{ℓᵢ:Aᵢ} <: {mⱼ:Bⱼ}` iff every supertype field `mⱼ:Bⱼ` is present in the
    /// subtype with `Aⱼ <: Bⱼ` (a record with *more* fields is a subtype —
    /// width — and each shared field is covariant — depth); see
    /// `gandr_core_checker::discipline::subtype`. Its value is the
    /// [`crate::syntax::Value::Record`] literal; its eliminator the
    /// field projection [`crate::syntax::Comp::RecordProj`]. The
    /// row-typed open record `{ℓ:A | ρ}` is the bracketed `+poly` upgrade
    /// (a closed record is the empty-tail special case — a refinement, not
    /// a retrofit).
    ///
    /// [`Prod`]: ValueType::Prod
    Record(
        /// The fields `ℓᵢ ↦ Aᵢ`, keyed by label (canonical, name-ordered).
        BTreeMap<String, Rc<Self>>,
    ),
    /// The graded thunk `U_r B` of a computation.
    Thunk(
        /// The usage grade `r`.
        Grade,
        /// The thunked computation type `B`.
        Rc<CompType>,
    ),
    /// The reified stack `Stk(B, C)` — a value-typed evaluation context
    /// consuming a `B`-computation and delivering an answer `C`
    /// (`effects-control-shell.md` §2.1; contract §6.3). Subtyping is
    /// contravariant in `B` and covariant in `C` (ADR-33 D6), matching the
    /// continuation reading. The `stk K` term that inhabits it lands with A3
    /// control.
    Stk(
        /// The consumed computation type `B`.
        Rc<CompType>,
        /// The delivered answer type `C`.
        Rc<CompType>,
    ),
    /// The identity (path) type `Path A x y` — gandr's **first dependent value
    /// type former** (ADR-76; `proposal-identity-univalence.md` §4). Terms
    /// occur inside a type: given a value type `A` and two values `x y : A`,
    /// `Path A x y` classifies proofs that `x` and `y` are equal.
    ///
    /// Introduced by [`crate::syntax::Value::Here`] (`here(v) : Path A v
    /// v`) and eliminated by the full Martin-Löf eliminator
    /// [`crate::syntax::Comp::Walk`] (motive over both endpoints and the
    /// path), whose sole computation rule is the definitional β on `here`
    /// (`walk(here(v), C, (x). c) ↦ c[v/x]`). There is **no K, no UIP, no
    /// definitional proof-irrelevance, and no η** — the without-K
    /// discipline (ADR-76 §2).
    ///
    /// Subtyping is **invariant** in the carrier `A` and in both endpoints
    /// (covariant widening is unsound without transport): `Path A x y <: Path
    /// A′ x′ y′` iff `A ≡ A′` and `x ≡ᵥ x′` and `y ≡ᵥ y′`. Both `≡ᵥ` on the
    /// endpoints and `≡` on the carrier are **definitional equality**, decided
    /// by the normalizer (`gandr_core_checker::nbe::conv::converts` and
    /// `gandr_core_checker::nbe::conv::type_converts`); the two-way subtyping
    /// pass that stood in for the carrier, and the structural no-reduction
    /// equality that stood in for the endpoints, are both retired. So `≡ᵥ`
    /// now carries Walk-β, congruence, and the substitution laws, and
    /// reduction inside a type is admitted.
    Path
    {
        /// The carrier value type `A`.
        ty: Rc<Self>,
        /// The left endpoint `x : A`.
        lhs: Rc<Value>,
        /// The right endpoint `y : A`.
        rhs: Rc<Value>,
    },
    /// The **declared-data nominal handle** `Data { id, args }` — a generative,
    /// nominal value-type former over a structural sum-of-records carrier
    /// (ADR-80 Decision 1; ADR-54 §3.4). The one frozen-core touch the
    /// 1-cell data-declarations MVP spends.
    ///
    /// Mirrors the built-in sum former [`ValueType::Sum`], but positive-nominal
    /// rather than positive-structural: its nominal `id` (a [`DataId`], the
    /// minted 0-cell) is compared **before** its structural carrier, so two
    /// structurally-identical declarations are **distinct** types
    /// (generativity). `args` are the type arguments `ā` in `D(ā)` (`Maybe`'s
    /// `a` instantiated to `Integer` in `Maybe(Integer)`); the carrier
    /// (`Maybe(a)` is `1 + #{x:a}`) is **not** stored here — it lives in the
    /// decl table the pipeline holds, so the core carries only the nominal tag
    /// (ADR-80 Decision 5).
    ///
    /// Introduced by [`crate::syntax::Value::Ctor`] (constructor-tagged
    /// values) and eliminated by [`crate::syntax::Comp::DataCase`] (a
    /// case over the tag). Subtyping compares the nominal `id` first
    /// (equality — generativity), then `args` **covariantly** and
    /// positionally (the same positive-former decomposition as
    /// [`Sum`]/[`List`]; parameter-variance tracking is a later rung — MVP
    /// datatypes are covariant). An **empty** `args` on either side is
    /// treated as consistent with any instantiation (the value-level
    /// carries the nominal tag, not the instantiation; see
    /// `gandr_core_checker::discipline::subtype`).
    ///
    /// [`Sum`]: ValueType::Sum
    /// [`List`]: ValueType::List
    Data
    {
        /// The datatype's minted nominal identity (serial + name).
        id: DataId,
        /// The type arguments `ā` in `D(ā)` (empty for a parameterless `D`).
        args: Vec<Rc<Self>>,
    },
    /// The predicative **code universe** `Type` — the levitation stage-1
    /// universe bump (ADR-67 D7 feature 1; ADR-81, `proposal-levitation.md`
    /// §6). The sort whose elements are the small value types the description
    /// decoder (`gandr_desc::decode`) produces: `decode : Desc → Type`, so
    /// `Universe` is the decoder's codomain and the classifier at which the
    /// dependent [`Sigma`](ValueType::Sigma) former binds its head.
    ///
    /// **One predicative bump, reconciled with ADR-78.** ADR-78 fixes the
    /// judgmental level algebra `{0, +1, max}` with `U_l : U_m ⟺ l < m`, no
    /// cumulativity, no `imax`, no constraints — living in the certified
    /// `gandr-kernel-levels` crate. The stage-1 bill asks for exactly *one*
    /// predicative bump (codes one level above the types they describe, "no
    /// `Type : Type`"). That is the `{0, +1}` sub-fragment of ADR-78's algebra
    /// (value types at level 0, `Universe` at level +1) using none of its
    /// `max` / landmark / `imax` machinery — so the two are consistent, not in
    /// conflict. The frozen interpreter core has **no** first-class
    /// `Γ ⊢ A : U_l` type-formation judgment (that judgmental universe typing
    /// is ADR-78's certified-kernel S2 job); it carries only the *former* — the
    /// single bump — with predicativity holding **by construction of the
    /// first-order fragment** (the decoder produces no type-valued field, so no
    /// `Σ` ever binds a `Universe` head and `Universe : Universe` is never
    /// ascribed). Subtyping is **reflexive only** (no cumulativity):
    /// `Universe <: Universe`, and `Universe` relates to no other former.
    ///
    /// [`Sigma`]: ValueType::Sigma
    Universe,
    /// The **dependent pair** type `Σ(x : A). B` — the levitation stage-1
    /// dependent value former (ADR-67 D7 feature 2; ADR-81,
    /// `proposal-levitation.md` §6). The dependent generalization of the eager
    /// product [`Prod`](ValueType::Prod): the tail type `B` may mention the
    /// value bound to the head, so a `Σ` genuinely puts a *value* in a *type*
    /// (like [`Path`](ValueType::Path) does through its endpoints, but under a
    /// binder).
    ///
    /// Introduced by the eager pair [`crate::syntax::Value::Pair`]
    /// **checked** against a `Sigma` (the intro is direction-driven: an
    /// inferred pair is a [`Prod`](ValueType::Prod), a pair *checked*
    /// against `Σ(x:A).B` checks its first component against `A` and its
    /// second against `B[v₁/x]` — the value-into-type substitution of
    /// [`crate::identity::subst_valuetype`]). Eliminated by the
    /// product / dependent-pair eliminator
    /// [`crate::syntax::Comp::Split`]: a `split v as (p, q) [z. M] in
    /// t` over a `v ⇑ Σ(x:A).B` binds `p : A` and `q : B[p/x]`, and — with
    /// an explicit dependent motive `(z). M` — checks the body against
    /// `M[(p, q)/z]` and delivers `M[v/z]` (rule `SplitMotive`⇑, ADR-82);
    /// the motive-less split is check-only (rule Split⇓). The machine runs
    /// a `Σ`-typed pair / split exactly as a product one — the runtime is
    /// type-erased for these nodes (the motive is inert), so no new
    /// evaluation rule is needed (ADR-81/82).
    ///
    /// **Predicative head.** The head `A` is a *small* value type (level 0); a
    /// `Σ` over [`Universe`](ValueType::Universe) (a type-parametric `Σ`, i.e.
    /// `Π`/families territory) is **outside** the stage-1 fragment and the
    /// checker's `Σ` rules never fire for it — this is where predicativity is
    /// enforced (no `Type : Type`). Subtyping is **invariant** (structural
    /// equality up to α-renaming of the binder, the conservative
    /// [`Path`](ValueType::Path) precedent): covariant `Σ` subtyping under the
    /// binder is a deferred refinement, not needed at stage 1.
    ///
    /// [`Prod`]: ValueType::Prod
    /// [`Path`]: ValueType::Path
    Sigma
    {
        /// The head type `A` — the bound variable's (small) value type.
        fst: Rc<Self>,
        /// The bound value-variable name `x`, in scope in `snd`.
        binder: String,
        /// The dependent tail type `B`, which may mention `binder`.
        snd: Rc<Self>,
    },
    /// A **sealed abstract type** `Sealed(a)` — the fresh nominal atom an
    /// opaque ascription mints for one abstract type component.
    ///
    /// The one positive former with **no structure at all**, and that absence
    /// is the content. [`Data`](ValueType::Data) is nominal *over* a
    /// carrier the decl table holds; a seal is nominal over nothing,
    /// because a client that could reach the representation is a client the
    /// seal failed against.
    ///
    /// **Opacity is a property of what is not here.** No field records a
    /// representation, so no code path can unfold one, and a leak is not a
    /// check that has to succeed — it is a value that cannot be written.
    /// Subtyping compares the [`SealId`] and relates a seal to nothing
    /// else, not even to the type it was sealed from
    /// (`gandr_core_checker::discipline::subtype`): sealing is *generative*, so
    /// two ascriptions of structurally identical implementations yield
    /// types that do not interchange, exactly as two `data` declarations
    /// do.
    ///
    /// It has no introduction and no eliminator of its own. Values of a sealed
    /// type are the values the sealed module exports, at the types the
    /// signature gave them — which is why sealing needs no new term forms
    /// and adds none.
    Sealed(
        /// The atom's minted nominal identity.
        SealId,
    ),
    /// A **first-class module package** `Package_r ⟨ᾱ⟩ A` — the module layer's
    /// one addition to the frozen value-type grammar.
    ///
    /// It internalizes a module at the core value level: the abstract type
    /// components `ᾱ` of the module's signature become **binders**, and the
    /// payload `A` is the signature's remaining content — in practice the
    /// thunked module returner `U_s (F^ε {ℓᵢ:Aᵢ})` the landed record-module
    /// lowering already produces, with each abstract component appearing inside
    /// it as an [`Atom`](ValueType::Atom) naming its binder.
    ///
    /// # It is the type-level Σ the predicative fence keeps out of `Sigma`
    ///
    /// [`Sigma`](ValueType::Sigma) binds a **value** and is fenced away from
    /// [`Universe`](ValueType::Universe) heads, so `Σ(T : Type). A(T)` — the
    /// shape every module signature with an abstract type component has — is
    /// outside its fragment by construction. `Package` is that shape, admitted
    /// at exactly one place: the module boundary, where **both directions are
    /// annotated** so no rule ever guesses it. Packing supplies its witness
    /// types explicitly ([`Value::Pack`], check-only) and unpacking supplies
    /// the signature it eliminates at
    /// ([`Comp::Unpack`](crate::syntax::Comp::Unpack), check-only).
    /// Checking a given large type is easy; guessing one is fenced, and the
    /// fence is what keeps the frozen core's decidability where it was.
    ///
    /// # Binders are names, and abstraction happens at the elimination
    ///
    /// `abstracts` holds the signature's abstract type component labels in
    /// signature order — they are ordinary binders, not minted atoms, because
    /// **minting happens at unpack**. An `Unpack` mints one fresh
    /// [`Sealed`](ValueType::Sealed) atom per binder and substitutes it into
    /// the payload, so two eliminations of one package yield abstract types
    /// that do not interchange, and a client can never reach the
    /// representation the packer supplied. That is where abstraction safety
    /// is bought; the type itself only records what is abstracted over.
    ///
    /// The kinds are arity-only (every binder classifies at `Type`), so the
    /// binder list is flat rather than a telescope: no binder can depend on an
    /// earlier one, and a `Vec` is the complete representation rather than a
    /// simplification of one.
    ///
    /// # It is not a graded thunk, and that is deliberate
    ///
    /// A package carries a usage grade and internalizes a thunked computation,
    /// which is exactly [`Thunk`](ValueType::Thunk)'s job description — and it
    /// is nonetheless a **separate former with a separate eliminator and no
    /// coercion in either direction**.
    /// [`Comp::Force`](crate::syntax::Comp::Force) refuses a package and
    /// [`Comp::Unpack`](crate::syntax::Comp::Unpack) refuses a thunk;
    /// `gandr_core_checker::discipline::subtype` relates a package only to a
    /// package. Forcing a package would hand a client the payload at the
    /// packer's witness types with no atom minted anywhere — the
    /// abstraction leak with no symptom — so the two formers stay apart at
    /// the representation, and the shared grade annotation is an annotation
    /// rather than a shared representation.
    ///
    /// Subtyping is **invariant in the payload** (up to positional α-renaming
    /// of the binders, the [`Sigma`](ValueType::Sigma) precedent) and
    /// **contravariant in the grade** (`hi ⊑ lo`, the
    /// [`Thunk`](ValueType::Thunk) leg): a package usable more often is a
    /// subtype of one usable less often. Two packages of different arity relate
    /// to nothing.
    ///
    /// [`Atom`]: ValueType::Atom
    Package
    {
        /// The usage grade `r` — how many times this package may be unpacked.
        ///
        /// [`Comp::Unpack`](crate::syntax::Comp::Unpack) requires `1 ⊑
        /// r`, exactly as
        /// [`Comp::Force`](crate::syntax::Comp::Force) does of a
        /// thunk, so a `Package_0` is a package that may be transported and
        /// never opened.
        grade: Grade,
        /// The abstract type component labels `ᾱ`, in signature order.
        ///
        /// These are binders: an occurrence inside `payload` is an
        /// [`Atom`](ValueType::Atom) of the same name, and the whole list is
        /// discharged in one simultaneous substitution at pack and at unpack
        /// (`gandr_core_checker::judgements::package`).
        abstracts: Vec<String>,
        /// The payload value type `A`, in whose scope every name in
        /// `abstracts` is bound.
        payload: Rc<Self>,
    },
    /// The unknown (hole) type `?` of value sort (A2.2 holes extension;
    /// `incremental-pipeline.md` §"Holes", `A2-PLAN.md` D5 — the Hazelnut hole
    /// type).
    ///
    /// `Unknown` is what a hole *infers* — the spec's "fresh α; no constraint
    /// emitted", degenerated honestly: Stage 1 has no σ, so the fresh
    /// variable that would carry no constraints collapses to one unknown.
    /// Subsumption treats it as **consistent in both directions** (a
    /// bidirectional wildcard, not a top or bottom type); see
    /// `gandr_core_checker::discipline::subtype` for the recorded decision tree
    /// and the transitivity caveat. Elimination forms whose principal
    /// premise infers `Unknown` succeed with `Unknown`-filled components
    /// (the Hazelnut *matched type* operation, degenerated likewise).
    Unknown,
}

impl ValueType
{
    /// Builds an [`ValueType::Atom`] from a name.
    #[inline]
    #[must_use]
    pub fn atom<'source, N>(name: N) -> Self
    where
        N: Into<TypeAtomName<'source>>,
    {
        Self::Atom(name.into().as_ref().to_owned())
    }

    /// The rigid atom `Integer`, the type of integer literals
    /// ([`crate::syntax::Value::Int`]; A2.1 literals extension).
    ///
    /// Both the checker and the machine type literals with this constructor,
    /// so it is the single point of truth for the atom's spelling.
    #[inline]
    #[must_use]
    pub fn integer() -> Self
    {
        Self::atom("Integer")
    }

    /// The rigid atom `String`, the type of string literals
    /// ([`crate::syntax::Value::Str`]; the value-model ladder's first
    /// scalar rung, ADR-38).
    ///
    /// Like [`ValueType::integer`], the string type is a rigid [`Atom`], not a
    /// dedicated former — Stage 1's base scalars are nominal atoms (the value
    /// type whose `String` spelling a `: String` annotation also lowers to).
    /// A dedicated `ValueType::Str` variant is the reserved, reversal-triggered
    /// upgrade, needed only if records/codecs later want string-specific type
    /// rules. Both the checker and the machine type literals with this
    /// constructor, so it is the single point of truth for the atom's spelling.
    ///
    /// [`Atom`]: ValueType::Atom
    #[inline]
    #[must_use]
    pub fn string() -> Self
    {
        Self::atom("String")
    }

    /// The rigid atom `u32` — one of the six numeric primitives (value-model
    /// ladder, ADR-39).
    ///
    /// Like [`ValueType::integer`] / [`ValueType::string`], the numeric
    /// primitives are rigid [`Atom`]s, not dedicated formers — the single point
    /// of truth for each spelling, the value type its suffixed literal
    /// (`…u32`) synthesizes and its `: u32` annotation lowers to. Distinct
    /// numeric atoms are nominal and pairwise incomparable (no implicit
    /// widening, ADR-39 D5).
    ///
    /// [`Atom`]: ValueType::Atom
    #[inline]
    #[must_use]
    pub fn u32() -> Self
    {
        Self::atom("u32")
    }

    /// The rigid atom `u64` (numeric primitive; see [`ValueType::u32`]).
    #[inline]
    #[must_use]
    pub fn u64() -> Self
    {
        Self::atom("u64")
    }

    /// The rigid atom `i32` (numeric primitive; see [`ValueType::u32`]).
    #[inline]
    #[must_use]
    pub fn i32() -> Self
    {
        Self::atom("i32")
    }

    /// The rigid atom `i64` (numeric primitive; see [`ValueType::u32`]).
    #[inline]
    #[must_use]
    pub fn i64() -> Self
    {
        Self::atom("i64")
    }

    /// The rigid atom `f32` (numeric primitive; see [`ValueType::u32`]).
    #[inline]
    #[must_use]
    pub fn f32() -> Self
    {
        Self::atom("f32")
    }

    /// The rigid atom `f64` (numeric primitive; see [`ValueType::u32`]).
    #[inline]
    #[must_use]
    pub fn f64() -> Self
    {
        Self::atom("f64")
    }

    /// Builds an eager product `fst × snd`.
    #[inline]
    #[must_use]
    pub fn prod(
        fst: Self,
        snd: Self,
    ) -> Self
    {
        Self::Prod(Rc::new(fst), Rc::new(snd))
    }

    /// Builds a tagged sum `lhs + rhs`.
    #[inline]
    #[must_use]
    pub fn sum(
        lhs: Self,
        rhs: Self,
    ) -> Self
    {
        Self::Sum(Rc::new(lhs), Rc::new(rhs))
    }

    /// Builds a list type `List element` (ADR-40).
    #[inline]
    #[must_use]
    pub fn list(element: Self) -> Self
    {
        Self::List(Rc::new(element))
    }

    /// Builds a closed record type `{ℓᵢ:Aᵢ}` from its labeled fields (ADR-45).
    ///
    /// Fields collect into a [`BTreeMap`], so the result is canonical in field
    /// order; a duplicated label keeps the last-supplied type.
    #[inline]
    #[must_use]
    pub fn record<I>(fields: I) -> Self
    where
        I: IntoIterator<Item = (String, Self)>,
    {
        Self::Record(
            fields
                .into_iter()
                .map(|(label, field)| (label, Rc::new(field)))
                .collect(),
        )
    }

    /// Builds a graded thunk `U_grade body`.
    #[inline]
    #[must_use]
    pub fn thunk(
        grade: Grade,
        body: CompType,
    ) -> Self
    {
        Self::Thunk(grade, Rc::new(body))
    }

    /// Builds a reified-stack type `Stk(consumes, delivers)`.
    #[inline]
    #[must_use]
    pub fn stk(
        consumes: CompType,
        delivers: CompType,
    ) -> Self
    {
        Self::Stk(Rc::new(consumes), Rc::new(delivers))
    }

    /// Builds a declared-data nominal handle `Data { id, args }` (ADR-80
    /// Decision 1) from its minted [`DataId`] and its type arguments.
    #[inline]
    #[must_use]
    pub fn data(
        id: DataId,
        args: Vec<Self>,
    ) -> Self
    {
        Self::Data {
            id,
            args: args.into_iter().map(Rc::new).collect(),
        }
    }

    /// Builds an identity type `Path ty lhs rhs` (ADR-76; `Path-Form`).
    #[inline]
    #[must_use]
    pub fn path(
        ty: Self,
        lhs: Value,
        rhs: Value,
    ) -> Self
    {
        Self::Path {
            ty: Rc::new(ty),
            lhs: Rc::new(lhs),
            rhs: Rc::new(rhs),
        }
    }

    /// The predicative code universe `Type` (ADR-81 feature 1;
    /// [`ValueType::Universe`]).
    #[inline]
    #[must_use]
    pub const fn universe() -> Self
    {
        Self::Universe
    }

    /// Builds a dependent pair type `Σ(binder : fst). snd` (ADR-81 feature 2;
    /// [`ValueType::Sigma`]). The tail `snd` may mention `binder`.
    #[inline]
    #[must_use]
    pub fn sigma<'source, B>(
        fst: Self,
        binder: B,
        snd: Self,
    ) -> Self
    where
        B: Into<BinderName<'source>>,
    {
        Self::Sigma {
            fst: Rc::new(fst),
            binder: binder.into().as_ref().to_owned(),
            snd: Rc::new(snd),
        }
    }

    /// Builds a package type `Package_grade ⟨abstracts⟩ payload`
    /// ([`ValueType::Package`]).
    ///
    /// The binder labels arrive as [`SealComponentName`]s because they are the
    /// abstract type components of a signature — the same role the sealing rung
    /// already names — and an occurrence of one inside `payload` is an
    /// [`Atom`](ValueType::Atom) of that name.
    ///
    /// # Contract
    /// - requires: nothing; a duplicate label is a defect the typing rules
    ///   refuse (`gandr_core_checker::judgements::package::instantiate`), not
    ///   this constructor's business.
    /// - ensures: preserves the grade, the binder order, and the payload
    ///   exactly.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn package<'source, I, C>(
        grade: Grade,
        abstracts: I,
        payload: Self,
    ) -> Self
    where
        I: IntoIterator<Item = C>,
        C: Into<SealComponentName<'source>>,
    {
        Self::Package {
            grade,
            abstracts: abstracts
                .into_iter()
                .map(|label| label.into().as_ref().to_owned())
                .collect(),
            payload: Rc::new(payload),
        }
    }
}

/// A computation (negative) type `B`.
///
/// `Debug` is implemented by hand (not derived) so the effect-graded returner
/// `F^ε A` renders as `F(A)` when its row is empty — preserving the
/// pure-returner spelling byte-for-byte (ADR-33 D1); see the `Debug` impl
/// below.
#[derive(Clone, Eq, Hash, PartialEq)]
pub enum CompType
{
    /// The effect-graded returner `F^ε A`: a computation that produces an `A`,
    /// performing the effects in the row `ε` (`effects-control-shell.md` §1;
    /// contract §5). `F A ≡ F^⟨⟩ A` is the empty-row case (the pure returner),
    /// built by [`Self::returner`]; the bottom-up row arithmetic over `ε` lands
    /// with the `+effects` terms.
    F(
        /// The produced value type `A`.
        Rc<ValueType>,
        /// The effect row `ε` the returner performs (empty for a pure `F A`).
        EffectRow,
    ),
    /// The function type `A → B` (value argument, computation result).
    Arrow(
        /// The argument's value type `A`.
        Rc<ValueType>,
        /// The result's computation type `B`.
        Rc<Self>,
    ),
    /// The lazy product ("with") `B & B′`.
    With(
        /// The first component.
        Rc<Self>,
        /// The second component.
        Rc<Self>,
    ),
    /// The unknown (hole) type `?` of computation sort (A2.2 holes
    /// extension; see [`ValueType::Unknown`] — the same decision, per sort,
    /// so the machine's polarity invariant stays sharp).
    Unknown,
}

impl CompType
{
    /// Builds a pure returner `F of ≡ F^⟨⟩ of` (the empty-row case).
    #[inline]
    #[must_use]
    pub fn returner(of: ValueType) -> Self
    {
        Self::F(Rc::new(of), EffectRow::EMPTY)
    }

    /// Builds an effect-graded returner `F^row of`.
    #[inline]
    #[must_use]
    pub fn returner_eff(
        of: ValueType,
        row: EffectRow,
    ) -> Self
    {
        Self::F(Rc::new(of), row)
    }

    /// Builds a function type `arg → res`.
    #[inline]
    #[must_use]
    pub fn arrow(
        arg: ValueType,
        res: Self,
    ) -> Self
    {
        Self::Arrow(Rc::new(arg), Rc::new(res))
    }

    /// Builds a lazy product `fst & snd`.
    #[inline]
    #[must_use]
    pub fn with(
        fst: Self,
        snd: Self,
    ) -> Self
    {
        Self::With(Rc::new(fst), Rc::new(snd))
    }
}

impl core::fmt::Debug for CompType
{
    /// Reproduces `derive(Debug)` exactly for every variant, with one
    /// refinement: the effect-graded returner renders as `F(A)` when its row is
    /// empty (`F^⟨⟩ A ≡ F A`), so the pure-returner spelling stays
    /// byte-identical to the pre-effects code, and as `F(A, ⟨…⟩)` when the
    /// row is non-empty (ADR-33 D1). Built through the
    /// [`core::fmt::Formatter`] tuple builder so no `{:?}` appears (the
    /// carrier renders only through `Debug`).
    #[inline]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        match *self {
            | Self::F(ref of, ref row) => {
                let mut tuple = f.debug_tuple("F");
                tuple.field(of);
                if !bool::from(row.is_empty()) {
                    tuple.field(row);
                }
                tuple.finish()
            },
            | Self::Arrow(ref arg, ref res) => {
                f.debug_tuple("Arrow").field(arg).field(res).finish()
            },
            | Self::With(ref fst, ref snd) => f.debug_tuple("With").field(fst).field(snd).finish(),
            | Self::Unknown => f.write_str("Unknown"),
        }
    }
}

/// A type of either polarity, as carried by `Return` control states and
/// errors.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Ty
{
    /// A value type.
    Value(
        /// The value type.
        ValueType,
    ),
    /// A computation type.
    Comp(
        /// The computation type.
        CompType,
    ),
}
