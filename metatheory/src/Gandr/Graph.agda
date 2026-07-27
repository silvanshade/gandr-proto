{-# OPTIONS --safe --guardedness --without-K --hidden-argument-puns #-}

------------------------------------------------------------------------------
-- Gandr.Graph — the ambient category of ∞-graphs.
--
-- Every structure in this tree is built on ∞-graphs: setoids, categories,
-- profunctors, the arity kits, and the cell complexes. An ∞-graph is the
-- coinductive presentation of a globular type — a presheaf on the globe
-- category, read internally — and an ∞-map is its morphism. This module is the
-- ambient category the rest works inside: initial and terminal objects,
-- coproducts, products, the discrete and codiscrete inclusions of `Set`, the
-- globes, the exponential, and the disc telescopes.
--
-- ── WHY THE CARRIER IS GLOBULAR, AND WHERE MULTI-ARITY GOES INSTEAD ─────────
-- gandr's cell shape is multi-in/multi-out (BUILD-MAP C1), so it is tempting
-- to make the coboundary `δ°` many-to-many at every dimension. That is the
-- wrong place for it, for a reason worth recording once:
--
-- Multi-arity is needed at EXACTLY ONE dimension — the cell shape. Above it
-- sit rules and coherence fillers, whose boundaries are PARALLEL PAIRS of the
-- level below. Parallel pairs are what `δ°` already is. Pushing lists through
-- every dimension would pay for a generality that only the base consumes, and
-- would commit the carrier to a shape where every direction composes — which
-- is not the virtual line this tree is on.
--
-- So the carrier stays globular and multi-arity is confined to the ARITY KIT
-- that the cell complex is parameterized by: linear (free-category) arities
-- recover the ordinary/virtual-double case, many-to-many arities give the
-- properadic one. Both are instances of one interface; this module is what
-- they are both built over.
--
-- ── ON `opaque` ─────────────────────────────────────────────────────────────
-- Deliberately absent here, unlike Gandr.Arena.Offset. `opaque` is for
-- definitions whose UNFOLDING is a cost to be controlled. The definitions in
-- this module exist to be unfolded: every consumer meets them through
-- copattern matching on `ϵ°`/`δ°`, and sealing them would sever the
-- definitional equalities the whole tower is built from. The discipline
-- resumes at the first module that proves something.
------------------------------------------------------------------------------

module Gandr.Graph where

open import Level
  using (Level)
  using (_⊔_)
  renaming (suc to lsuc)

-- The `Set`-level operations the ∞-graph layer mirrors one dimension up.
-- Repackaged rather than imported flat: the layer's job is to name the SAME
-- universal properties one dimension up (`𝟘`/`𝟙`/`⊕`/`⊗` and their maps), so
-- the two levels must read alike. `seq` is DIAGRAMMATIC — `seq f g` runs `f`
-- then `g` — which is the order every composition in this tree uses.
private
  module 𝕊 where
    open import Data.Empty.Polymorphic public
      using (⊥)
      renaming (⊥-elim to ¡)
    open import Data.Unit.Polymorphic public
      using (⊤)
      using (tt)
    open import Data.Sum public
      using (_⊎_)
      using ([_,_])
      renaming (inj₁ to inl)
      renaming (inj₂ to inr)
    open import Data.Product public
      using (_×_)
      using (_,_)
      renaming (proj₁ to fst)
      renaming (proj₂ to snd)
      renaming (<_,_> to ⟨_,_⟩)
    open import Function.Base public
      using (flip)
      renaming (id to idn)
      renaming (_∘′_ to _∘_)

    -- Diagrammatic composition: `seq f g` is `g ∘ f`.
    seq : ∀ {a b c} {A : Set a} {B : Set b} {C : Set c}
      → (A → B)
      → (B → C)
      → (A → C)
    seq f g = g ∘ f

    -- The unique map into the terminal object.
    ! : ∀ {a ℓ} {A : Set a} → A → ⊤ {ℓ}
    ! _ = tt

  open 𝕊
    using (tt)

open import Data.Nat
  using (ℕ)
  using (zero)
  renaming (suc to succ)
open import Relation.Binary.PropositionalEquality.Core
  using (_≡_)

-- The layer letter is 𝔾: a wrapper module mirroring 𝕊 one dimension up, so
-- later layers requalify as 𝔾.idn / 𝔾.seq rather than minting fresh names.
module 𝔾 where

  -- ═══════════════════════════════════════════════════════════════════════════
  -- The carrier. `ϵ°` names the cells at the current dimension; the coboundary
  -- `δ°` hands back, over each parallel pair, the ∞-graph one dimension up.
  -- `δ°` is the primitive — the boundary is a derived projection, never a
  -- field, so nothing downstream can observe a boundary the carrier did not
  -- offer.
  -- ═══════════════════════════════════════════════════════════════════════════

  record ∞Graph (ℓ : Level) : Set (lsuc ℓ) where
    coinductive
    field
      -- the cells at this dimension
      ϵ° : Set ℓ
      -- over each parallel pair, the ∞-graph one dimension up
      δ° : (x y : ϵ°) → ∞Graph ℓ

    -- Postfix sugar for the cells, so a carrier reads as a type.
    infix 0 _϶
    _϶ : Set ℓ
    _϶ = ϵ°

    -- Postfix sugar for the coboundary, so descent reads left to right.
    infixl 1 _▸ᵍ_⇴_
    _▸ᵍ_⇴_ : (x y : ϵ°) → ∞Graph ℓ
    _▸ᵍ_⇴_ = δ°
  open ∞Graph public
  {-# DISPLAY ϵ° Ξ = Ξ ϶ #-}
  {-# DISPLAY δ° Ξ x y = Ξ ▸ᵍ x ⇴ y #-}

  -- A morphism of ∞-graphs: a map on cells at every dimension, respecting
  -- boundaries BY CONSTRUCTION — `δ↬` is typed to land over the image pair, so
  -- there is no separate boundary-preservation law to state or discharge.
  record ∞Map {ℓ} (A B : ∞Graph ℓ) : Set ℓ where
    coinductive
    field
      -- the action on cells at this dimension
      ϵ↬ : ϵ° A → ϵ° B
      -- the action one dimension up, over the image pair
      δ↬ : ∀ {x y} → ∞Map {ℓ} (A .δ° x y) (B .δ° (ϵ↬ x) (ϵ↬ y))

    -- Infix sugar for application.
    infixl 2 _·_
    _·_ : ϵ° A → ϵ° B
    _·_ = ϵ↬

    -- Prefix sugar for ascending a dimension.
    infixr 3 ↗_
    ↗_ : ∀ {x y} → ∞Map {ℓ} (A .δ° x y) (B .δ° (ϵ↬ x) (ϵ↬ y))
    ↗_ = δ↬
  open ∞Map public
  {-# DISPLAY ϵ↬ F x = F · x #-}
  {-# DISPLAY δ↬ F = ↗ F #-}

  -- ═══════════════════════════════════════════════════════════════════════════
  -- The category structure on ∞-graphs. Identity and composition are the
  -- Set-level ones at every dimension, corecursively.
  -- ═══════════════════════════════════════════════════════════════════════════

  -- The identity ∞-map.
  idn : ∀ {ℓ} {A : ∞Graph ℓ} → ∞Map A A
  idn .ϵ↬ = 𝕊.idn
  idn .δ↬ = idn

  -- Composition of ∞-maps, diagrammatic: `seq F G` runs `F` then `G`.
  seq : ∀ {ℓ} {A B C : ∞Graph ℓ} → ∞Map A B → ∞Map B C → ∞Map A C
  seq F G .ϵ↬ = 𝕊.seq (ϵ↬ F) (ϵ↬ G)
  seq F G .δ↬ = seq (δ↬ F) (δ↬ G)

  -- The identity-type ∞-graph on a `Set`: cells are the elements, and the
  -- coboundary is the identity type, all the way up. This is the ∞-graph a
  -- `Set` presents when its equality is taken proof-relevantly.
  Id : ∀ {ℓ} → (A : Set ℓ) → ∞Graph ℓ
  Id A .ϵ° = A
  Id A .δ° x y = Id (x ≡ y)

  -- ═══════════════════════════════════════════════════════════════════════════
  -- The initial object: no cells at any dimension.
  -- ═══════════════════════════════════════════════════════════════════════════

  𝟘 : ∀ {ℓ} → ∞Graph ℓ
  𝟘 .ϵ° = 𝕊.⊥
  𝟘 .δ° ()

  -- The unique map out of the initial object.
  ¡ : ∀ {ℓ} {A : ∞Graph ℓ} → ∞Map 𝟘 A
  ¡ .ϵ↬ = 𝕊.¡
  ¡ .δ↬ {()}

  -- ═══════════════════════════════════════════════════════════════════════════
  -- Coproducts. The boundary constraint is carried IN THE CONSTRUCTORS: a cell
  -- over an inl/inl pair comes from `A`, over inr/inr from `B`, and the mixed
  -- homs are empty — level-wise disjointness. A purpose-built record rather
  -- than a raw sum of sums, so the two summands are named at every dimension.
  -- ═══════════════════════════════════════════════════════════════════════════

  module Σ⊕δ where
    data Σ⊕δ {ℓ} (A B : ∞Graph ℓ) : (x y : A .ϵ° 𝕊.⊎ B .ϵ°) → Set ℓ where
      inl : ∀ {a₀ a₁} → A .δ° a₀ a₁ .ϵ° → Σ⊕δ A B (𝕊.inl a₀) (𝕊.inl a₁)
      inr : ∀ {b₀ b₁} → B .δ° b₀ b₁ .ϵ° → Σ⊕δ A B (𝕊.inr b₀) (𝕊.inr b₁)
  open Σ⊕δ public
    hiding (module Σ⊕δ)
    using (Σ⊕δ)

  infixr 1 _⊕_
  _⊕_ : ∀ {ℓ} → ∞Graph ℓ → ∞Graph ℓ → ∞Graph ℓ
  _⊕_ A B .ϵ° = A .ϵ° 𝕊.⊎ B .ϵ°
  _⊕_ A B .δ° x y .ϵ° = Σ⊕δ A B x y
  _⊕_ A B .δ° x y .δ° (Σ⊕δ.inl f₀) (Σ⊕δ.inl f₁) = A .δ° _ _ .δ° f₀ f₁
  _⊕_ A B .δ° x y .δ° (Σ⊕δ.inr g₀) (Σ⊕δ.inr g₁) = B .δ° _ _ .δ° g₀ g₁

  -- The left injection.
  inl : ∀ {ℓ} {A B : ∞Graph ℓ} → ∞Map A (A ⊕ B)
  inl .ϵ↬ = 𝕊.inl
  inl .δ↬ .ϵ↬ = Σ⊕δ.inl
  inl .δ↬ .δ↬ = idn

  -- The right injection.
  inr : ∀ {ℓ} {A B : ∞Graph ℓ} → ∞Map B (A ⊕ B)
  inr .ϵ↬ = 𝕊.inr
  inr .δ↬ .ϵ↬ = Σ⊕δ.inr
  inr .δ↬ .δ↬ = idn

  -- The copairing: the coproduct's universal property.
  [_,_] : ∀ {ℓ} {A B X : ∞Graph ℓ} → ∞Map A X → ∞Map B X → ∞Map (A ⊕ B) X
  [_,_] F G .ϵ↬ = 𝕊.[ ϵ↬ F , ϵ↬ G ]
  [_,_] F G .δ↬ .ϵ↬ (Σ⊕δ.inl f) = F .δ↬ .ϵ↬ f
  [_,_] F G .δ↬ .ϵ↬ (Σ⊕δ.inr g) = G .δ↬ .ϵ↬ g
  [_,_] F G .δ↬ .δ↬ {x = Σ⊕δ.inl f₀} {y = Σ⊕δ.inl f₁} = F .δ↬ .δ↬
  [_,_] F G .δ↬ .δ↬ {x = Σ⊕δ.inr g₀} {y = Σ⊕δ.inr g₁} = G .δ↬ .δ↬

  -- The coproduct's functorial action.
  [_⊕_] : ∀ {ℓ} {A B X Y : ∞Graph ℓ} → ∞Map A X → ∞Map B Y → ∞Map (A ⊕ B) (X ⊕ Y)
  [_⊕_] F G = [ seq F inl , seq G inr ]

  -- The coproduct interchange.
  ⊕-xchg : ∀ {ℓ} {A B C D : ∞Graph ℓ} → ∞Map ((A ⊕ B) ⊕ (C ⊕ D)) ((A ⊕ C) ⊕ (B ⊕ D))
  ⊕-xchg = [ [ inl ⊕ inl ] , [ inr ⊕ inr ] ]

  -- ═══════════════════════════════════════════════════════════════════════════
  -- The terminal object: one cell over every parallel pair, at every dimension.
  -- ═══════════════════════════════════════════════════════════════════════════

  𝟙 : ∀ {ℓ} → ∞Graph ℓ
  𝟙 .ϵ° = 𝕊.⊤
  𝟙 .δ° x y = 𝟙

  -- The unique map into the terminal object.
  ! : ∀ {ℓ} {A : ∞Graph ℓ} → ∞Map A 𝟙
  ! .ϵ↬ = 𝕊.!
  ! .δ↬ = !

  -- ═══════════════════════════════════════════════════════════════════════════
  -- Products: cells are pairs, level-wise. Unlike the coproduct no constructor
  -- is needed — a parallel pair of pairs already determines the two component
  -- parallel pairs.
  -- ═══════════════════════════════════════════════════════════════════════════

  infixr 2 _⊗_
  _⊗_ : ∀ {ℓ} → ∞Graph ℓ → ∞Graph ℓ → ∞Graph ℓ
  _⊗_ A B .ϵ° = A .ϵ° 𝕊.× B .ϵ°
  _⊗_ A B .δ° (a₀ 𝕊., b₀) (a₁ 𝕊., b₁) = A .δ° a₀ a₁ ⊗ B .δ° b₀ b₁

  -- The left projection.
  fst : ∀ {ℓ} {A B : ∞Graph ℓ} → ∞Map (A ⊗ B) A
  fst .ϵ↬ = 𝕊.fst
  fst .δ↬ = fst

  -- The right projection.
  snd : ∀ {ℓ} {A B : ∞Graph ℓ} → ∞Map (A ⊗ B) B
  snd .ϵ↬ = 𝕊.snd
  snd .δ↬ = snd

  -- The pairing: the product's universal property.
  ⟨_,_⟩ : ∀ {ℓ} {X A B : ∞Graph ℓ} → ∞Map X A → ∞Map X B → ∞Map X (A ⊗ B)
  ⟨_,_⟩ F G .ϵ↬ = 𝕊.⟨ ϵ↬ F , ϵ↬ G ⟩
  ⟨_,_⟩ F G .δ↬ = ⟨ δ↬ F , δ↬ G ⟩

  -- The product's functorial action.
  ⟨_⊗_⟩ : ∀ {ℓ} {X Y A B : ∞Graph ℓ} → ∞Map X A → ∞Map Y B → ∞Map (X ⊗ Y) (A ⊗ B)
  ⟨_⊗_⟩ F G = ⟨ seq fst F , seq snd G ⟩

  -- The product interchange.
  ⊗-xchg : ∀ {ℓ} {A B C D : ∞Graph ℓ} → ∞Map ((A ⊗ B) ⊗ (C ⊗ D)) ((A ⊗ C) ⊗ (B ⊗ D))
  ⊗-xchg = ⟨ ⟨ fst ⊗ fst ⟩ , ⟨ snd ⊗ snd ⟩ ⟩

  -- ═══════════════════════════════════════════════════════════════════════════
  -- The discrete and codiscrete inclusions of `Set` — left and right adjoint,
  -- respectively, to the 0-cells projection `ϵ°`. `disc A` has no cells above
  -- dimension 0; `codisc A` has exactly one over every parallel pair. `𝟘` and
  -- `𝟙` are their images of the empty and unit types.
  -- ═══════════════════════════════════════════════════════════════════════════

  disc : ∀ {ℓ} → Set ℓ → ∞Graph ℓ
  disc A .ϵ° = A
  disc A .δ° x y = 𝟘

  codisc : ∀ {ℓ} → Set ℓ → ∞Graph ℓ
  codisc A .ϵ° = A
  codisc A .δ° x y = 𝟙

  -- ═══════════════════════════════════════════════════════════════════════════
  -- The globes. `𝕪 n` is the n-globe — the representable presheaf at `n`,
  -- rendered coinductively: two poles at every dimension below `n`, one cell at
  -- `n`, nothing above. These are the definable internal intervals — an n-cell
  -- of an exponential is a cylinder out of `𝕪 n ⊗ A` — so no interval
  -- primitive is needed in the trusted surface.
  -- ═══════════════════════════════════════════════════════════════════════════

  module 𝕪 where
    -- The two poles of a positive-dimensional globe, where the derived boundary
    -- lands. Purpose-built with informative constructors rather than `⊤ ⊎ ⊤`.
    data Pole {ℓ} : Set ℓ where
      ∂⁻ : Pole
      ∂⁺ : Pole

    𝕪 : ∀ {ℓ} → ℕ → ∞Graph ℓ
    𝕪 zero = disc 𝕊.⊤
    𝕪 (succ n) .ϵ° = Pole
    𝕪 (succ n) .δ° ∂⁻ ∂⁻ = 𝟘
    𝕪 (succ n) .δ° ∂⁻ ∂⁺ = 𝕪 n
    𝕪 (succ n) .δ° ∂⁺ ∂⁻ = 𝟘
    𝕪 (succ n) .δ° ∂⁺ ∂⁺ = 𝟘
  open 𝕪 public

  -- ═══════════════════════════════════════════════════════════════════════════
  -- The internal hom. `[ A ⇒ B ]` is the presheaf exponential, right adjoint to
  -- `_⊗_`, and Yoneda forces its shape: an n-cell is a map out of the n-globe
  -- cylinder `𝕪 n ⊗ A → B`. So the 0-cells are BARE FUNCTIONS `A .ϵ° → B .ϵ°`
  -- (because `𝕪 0 ⊗ A` is discrete), NOT ∞-maps — the ∞-maps are the GLOBAL
  -- POINTS `∞Map 𝟙 [ A ⇒ B ]` (`name`/`unname` below); and an (n+1)-cell over
  -- (φ, ψ) is a CYLINDER, sending each (n+1)-cell of `A` lying over a parallel
  -- pair (u, u′) to an (n+1)-cell of `B` over (φ u, ψ u′) — no object
  -- components, no composition. The components-style tower genuinely needs
  -- composition and arrives with the structure layer.
  --
  -- The whole tower is ONE guarded corecursion, the family former `⟨_⋔_⟩`: the
  -- corecursive case closes only after generalizing from a pair of graphs to an
  -- I-indexed FAMILY of pairs, the index growing one parallel pair per level.
  -- `Σ⋔δ` is that index step. The indices reachable from `[ A ⇒ B ]` are
  -- exactly `A`'s sphere telescopes — a disc plus a parallel pair. The
  -- reindexing steps `⋔-dom`/`⋔-cod` are NAMED definitions on purpose: distinct
  -- use-sites must share them definitionally, and pattern-lambdas would mint a
  -- fresh definition per site.
  -- ═══════════════════════════════════════════════════════════════════════════

  module Σ⋔δ where
    record Σ⋔δ {ℓ} {I : Set ℓ} (𝐀 : I → ∞Graph ℓ) : Set ℓ where
      constructor _∂_⇒_
      field
        pos : I
        src : 𝐀 pos .ϵ°
        tgt : 𝐀 pos .ϵ°
  open Σ⋔δ public
    hiding (module Σ⋔δ)

  -- Reindex the domain family one level up.
  ⋔-dom : ∀ {ℓ} {I : Set ℓ} (𝐀 : I → ∞Graph ℓ) → Σ⋔δ 𝐀 → ∞Graph ℓ
  ⋔-dom 𝐀 (i ∂ u ⇒ u′) = 𝐀 i .δ° u u′

  -- Reindex the codomain family one level up, over the image pair.
  ⋔-cod : ∀ {ℓ} {I : Set ℓ} (𝐀 𝐁 : I → ∞Graph ℓ)
    → (φ ψ : ∀ i → 𝐀 i .ϵ° → 𝐁 i .ϵ°)
    → Σ⋔δ 𝐀 → ∞Graph ℓ
  ⋔-cod 𝐀 𝐁 φ ψ (i ∂ u ⇒ u′) = 𝐁 i .δ° (φ i u) (ψ i u′)

  -- The family former: the single guarded corecursion the exponential is.
  ⟨_⋔_⟩ : ∀ {ℓ} {I : Set ℓ} (𝐀 𝐁 : I → ∞Graph ℓ) → ∞Graph ℓ
  ⟨ 𝐀 ⋔ 𝐁 ⟩ .ϵ° = ∀ i → 𝐀 i .ϵ° → 𝐁 i .ϵ°
  ⟨ 𝐀 ⋔ 𝐁 ⟩ .δ° φ ψ = ⟨ ⋔-dom 𝐀 ⋔ ⋔-cod 𝐀 𝐁 φ ψ ⟩

  -- Evaluation at an index.
  ev : ∀ {ℓ} {I : Set ℓ} (𝐀 𝐁 : I → ∞Graph ℓ) (i : I) → ∞Map (⟨ 𝐀 ⋔ 𝐁 ⟩ ⊗ 𝐀 i) (𝐁 i)
  ev 𝐀 𝐁 i .ϵ↬ (φ 𝕊., a) = φ i a
  ev 𝐀 𝐁 i .δ↬ {φ 𝕊., a} {ψ 𝕊., b} = ev (⋔-dom 𝐀) (⋔-cod 𝐀 𝐁 φ ψ) (i ∂ a ⇒ b)

  -- Abstraction into the family former.
  lam : ∀ {ℓ} {I : Set ℓ} {X : ∞Graph ℓ} (𝐀 𝐁 : I → ∞Graph ℓ)
    → (∀ i → ∞Map (X ⊗ 𝐀 i) (𝐁 i))
    → ∞Map X ⟨ 𝐀 ⋔ 𝐁 ⟩
  lam 𝐀 𝐁 F .ϵ↬ x i a = F i .ϵ↬ (x 𝕊., a)
  lam 𝐀 𝐁 F .δ↬ {x} {y} =
    lam (⋔-dom 𝐀)
        (⋔-cod 𝐀 𝐁 (λ i a → F i .ϵ↬ (x 𝕊., a)) (λ i a → F i .ϵ↬ (y 𝕊., a)))
        (λ { (i ∂ a ⇒ b) → F i .δ↬ {x 𝕊., a} {y 𝕊., b} })

  -- A global point of the family former is a family of ∞-maps.
  unlam : ∀ {ℓ} {I : Set ℓ} (𝐀 𝐁 : I → ∞Graph ℓ)
    → ∞Map 𝟙 ⟨ 𝐀 ⋔ 𝐁 ⟩ → ∀ i → ∞Map (𝐀 i) (𝐁 i)
  unlam 𝐀 𝐁 P i .ϵ↬ a = P .ϵ↬ tt i a
  unlam 𝐀 𝐁 P i .δ↬ {x} {y} =
    unlam (⋔-dom 𝐀) (⋔-cod 𝐀 𝐁 (P .ϵ↬ tt) (P .ϵ↬ tt)) (P .δ↬) (i ∂ x ⇒ y)

  -- The exponential's own hom-index: the 0-spheres — ordered parallel pairs of
  -- 0-cells — of `A`. Again purpose-built over a raw pair.
  module Σ⇒δ where
    record Σ⇒δ {ℓ} (A : ∞Graph ℓ) : Set ℓ where
      constructor _⇒_
      field
        src : A .ϵ°
        tgt : A .ϵ°
  open Σ⇒δ public
    hiding (module Σ⇒δ)

  -- Reindex the exponential's domain one level up.
  ⇒-dom : ∀ {ℓ} (A : ∞Graph ℓ) → Σ⇒δ A → ∞Graph ℓ
  ⇒-dom A (x ⇒ y) = A .δ° x y

  -- Reindex the exponential's codomain one level up, over the image pair.
  ⇒-cod : ∀ {ℓ} (A B : ∞Graph ℓ) (h k : A .ϵ° → B .ϵ°) → Σ⇒δ A → ∞Graph ℓ
  ⇒-cod A B h k (x ⇒ y) = B .δ° (h x) (k y)

  -- The exponential.
  [_⇒_] : ∀ {ℓ} → ∞Graph ℓ → ∞Graph ℓ → ∞Graph ℓ
  [ A ⇒ B ] .ϵ° = A .ϵ° → B .ϵ°
  [ A ⇒ B ] .δ° h k = ⟨ ⇒-dom A ⋔ ⇒-cod A B h k ⟩

  -- Application: the counit of the adjunction.
  apply : ∀ {ℓ} {A B : ∞Graph ℓ} → ∞Map ([ A ⇒ B ] ⊗ A) B
  apply .ϵ↬ (h 𝕊., a) = h a
  apply {A} {B} .δ↬ {h 𝕊., a} {k 𝕊., b} = ev (⇒-dom A) (⇒-cod A B h k) (a ⇒ b)

  -- Currying: the adjunction's transpose.
  curry : ∀ {ℓ} {X A B : ∞Graph ℓ} → ∞Map (X ⊗ A) B → ∞Map X [ A ⇒ B ]
  curry F .ϵ↬ x a = F .ϵ↬ (x 𝕊., a)
  curry {A} {B} F .δ↬ {x} {y} =
    lam (⇒-dom A)
        (⇒-cod A B (λ a → F .ϵ↬ (x 𝕊., a)) (λ a → F .ϵ↬ (y 𝕊., a)))
        (λ { (a ⇒ b) → F .δ↬ {x 𝕊., a} {y 𝕊., b} })

  -- Uncurrying: the inverse transpose.
  uncurry : ∀ {ℓ} {X A B : ∞Graph ℓ} → ∞Map X [ A ⇒ B ] → ∞Map (X ⊗ A) B
  uncurry G = seq ⟨ seq fst G , snd ⟩ apply

  -- The global points of the exponential ARE the ∞-maps — the adjunction at
  -- `X = 𝟙`. An ∞-map unfolds into exactly the diagonal point-tower.
  name : ∀ {ℓ} {A B : ∞Graph ℓ} → ∞Map A B → ∞Map 𝟙 [ A ⇒ B ]
  name F = curry (seq snd F)

  unname : ∀ {ℓ} {A B : ∞Graph ℓ} → ∞Map 𝟙 [ A ⇒ B ] → ∞Map A B
  unname {A} {B} P .ϵ↬ a = P .ϵ↬ tt a
  unname {A} {B} P .δ↬ {x} {y} =
    unlam (⇒-dom A) (⇒-cod A B (P .ϵ↬ tt) (P .ϵ↬ tt)) (P .δ↬) (x ⇒ y)

  -- The symmetry of the product.
  swap : ∀ {ℓ} {A B : ∞Graph ℓ} → ∞Map (A ⊗ B) (B ⊗ A)
  swap = ⟨ snd , fst ⟩

  -- Distribution of the product over the coproduct, both handednesses. Derived
  -- from the exponential rather than defined by cases: `⊗` is a left adjoint,
  -- so it preserves the coproduct, and that is the proof.
  ⊗-dist-λ : ∀ {ℓ} {A B C : ∞Graph ℓ} → ∞Map (A ⊗ (B ⊕ C)) ((A ⊗ B) ⊕ (A ⊗ C))
  ⊗-dist-λ = seq swap (uncurry [ curry (seq swap inl) , curry (seq swap inr) ])

  ⊗-dist-ρ : ∀ {ℓ} {A B C : ∞Graph ℓ} → ∞Map ((A ⊕ B) ⊗ C) ((A ⊗ C) ⊕ (B ⊗ C))
  ⊗-dist-ρ = uncurry [ curry inl , curry inr ]

-- Only the carrier vocabulary is unqualified at the top level; the operations
-- stay behind `𝔾.` so later layers can mirror them one dimension up without
-- collision.
open 𝔾 public
  hiding (module ∞Graph)
  using (∞Graph)
  using (ϵ°)
  using (δ°)
  using (_϶)
  using (_▸ᵍ_⇴_)
  using (∞Map)
  using (ϵ↬)
  using (δ↬)
  using (↗_)
  using (_·_)

-- ══════════════════════════════════════════════════════════════════════════════
-- Discs: the addresses. A disc `Θ` is a telescope of parallel pairs walked down
-- from a carrier `Ξ`, and `⟦Disc⟧` interprets it as the ∞-graph it lands in —
-- the generic cell position. The structure towers' descent and the cell
-- complex's spheres index over this.
-- ══════════════════════════════════════════════════════════════════════════════

module Base {ℓ} (Ξ : ∞Graph ℓ) where
  infixl 0 _▸ᵈ_⇴_
  mutual
    data Disc : Set ℓ where
      -- the empty telescope: the carrier itself
      ⋆ : Disc
      -- extend by a parallel pair of the positions reached so far
      _▸ᵈ_⇴_ : (Θ : Disc) (x y : ⟦Disc⟧ Θ .ϵ°) → Disc

    ⟦Disc⟧ : Disc → ∞Graph ℓ
    ⟦Disc⟧ ⋆ = Ξ
    ⟦Disc⟧ (Θ ▸ᵈ x ⇴ y) = ⟦Disc⟧ Θ ▸ᵍ x ⇴ y
open Base
