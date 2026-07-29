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
  using (refl)

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
  -- The DISCRETE SETOID ON THE IDENTITY TYPE, which is `disc` shifted one
  -- dimension: the elements, their equality as 1-cells, and nothing above.
  --
  -- This is how a `Set`-level structure presents as a `Category` — the hom at a
  -- pair of objects is `≡° (H x y)`, so the 2-cells are `f ≡ g` and `homˢ` is
  -- the identity setoid. `Category`'s fields all land at or below that level:
  -- `idn₁`/`seq₁`/`inv₁` are `refl`/`trans`/`sym`, and `seq↕`, `mon-λ`, `mon-ρ`
  -- and `mon-α` are `≡`-cells. The record is LAWLESS AT ITS LAST DIMENSION —
  -- it states the laws and imposes no coherence among them — which is exactly
  -- what this presentation supports.
  --
  -- ── WHY `𝟘` ABOVE, AND NOT `Id` AND NOT `𝟙` ────────────────────────────────
  -- Three choices for what sits above the last dimension that carries content,
  -- and conflating them is the failure mode:
  --
  --   * `𝟘` — the correct one. It says there are no cells there. It asserts
  --     nothing and discharges nothing, and if the structure later turns out to
  --     carry genuine 2-cells the dimension simply opens up.
  --   * `𝟙` — forbidden by default. A terminal hom makes every coherence above
  --     hold automatically, silently discharging obligations nobody checked.
  --     That is what "do not truncate prematurely" is about; use it only with a
  --     stated reason, as `Setoids`' homotopies do.
  --   * `Id` — honest but not minimal. It continues with the identity type at
  --     every dimension, so it offers a whole tower no consumer asks for; `≡°`
  --     is `Id` stopped at dimension 1 by `𝟘` rather than by `𝟙`.
  --
  -- Using `_≡_` as a structure's 1-cells carries NO UIP claim, and this is
  -- worth stating because it reads like one: nothing above the 1-cells is
  -- asserted, so no two proofs of `f ≡ g` are ever identified. Where a result
  -- genuinely needs set-ness it takes `UIP` as a signature parameter, which is
  -- what the grafting unit laws do.
  -- ═══════════════════════════════════════════════════════════════════════════

  ≡° : ∀ {ℓ} → Set ℓ → ∞Graph ℓ
  ≡° A .ϵ° = A
  ≡° A .δ° x y = disc (x ≡ y)

  -- ═══════════════════════════════════════════════════════════════════════════
  -- THE SAME CHOICE ONE DIMENSION UP: objects and a HOM FAMILY, with
  -- propositional equality of morphisms as the 2-cells and nothing above.
  --
  -- `≡°` gives a `Set` the shape a `Set`-level SETOID needs — content at
  -- dimensions 0 and 1, which is exactly `Setoid`'s reach. A `Set`-level
  -- CATEGORY needs one dimension more, and its 1-cells are not equalities of
  -- objects but a family indexed by a parallel pair of them. So the hom at
  -- `(x , y)` is `≡° (H x y)`: the morphisms, their equalities, and `𝟘` above.
  --
  -- Every `Set`-level category in this tree goes through this former, which is
  -- why it is named once here rather than inlined at the first instance. Its
  -- REGION for `Category` is `Only⋆` — content at dimensions 0–2, and none at
  -- any address above the carrier — so a consumer certifies with `at⋆` and
  -- `Everywhere Category` is refuted (`Gandr.Category.ℂ.homs°-not-everywhere`).
  --
  -- ── THE NAME IS DESCRIPTIVE, AND DELIBERATELY CLAIMS NOTHING ────────────────
  -- `H` is whatever family the caller puts at the homs. Whether it composes,
  -- and what category it is when it does, is the `Category` instance's business
  -- and its header's; nothing is asserted by the carrier.
  -- ═══════════════════════════════════════════════════════════════════════════

  homs° : ∀ {ℓ} (O : Set ℓ) (H : O → O → Set ℓ) → ∞Graph ℓ
  homs° O H .ϵ° = O
  homs° O H .δ° x y = ≡° (H x y)

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
--
-- ── WHY THE ADDRESS IS A CODE, AND NOT A SPINE OF PROJECTIONS ─────────────────
-- A cell's type is written `Ξ .δ° a b .δ° f g .ϵ°`, and that spine grows with
-- dimension. It cannot be abstracted over by an implicit argument, and this is
-- a fact about the unifier rather than an ergonomic complaint: recovering the
-- prefix from a cell's type poses a constraint `ϵ° ?Ξ ≈ ϵ° Ξ₀`, a metavariable
-- under a projection, which Agda solves only by eta-expanding the metavariable
-- — and COINDUCTIVE RECORDS HAVE NO ETA. Anything that tries to infer a carrier
-- or a boundary from a cell is stuck, always, and no rearrangement of the
-- record changes it.
--
-- So the address is REIFIED. `Disc` is inductive with injective constructors:
-- it can be matched on, recursed over, and bound as an ordinary parameter. A
-- statement generic in the dimension binds one `Θ` and reads `⟦Disc⟧ Θ`; the
-- interpretation computes away at any concrete telescope, so such a statement
-- instantiates back to the hand-written spine DEFINITIONALLY rather than up to
-- a transport. The telescope form is not a second way of saying things — it is
-- the same signature with the depth abstracted.
--
-- This is the sort discipline the workflow document states once: what must be
-- discriminated, matched, or recursed over is a CODE; what must be witnessed or
-- transported along is a CERTIFICATE; forcing either into the other's column is
-- what the known failure modes of higher-dimensional formalization are.
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
-- Exported, unqualified, on the same footing as `_▸ᵍ_⇴_`: the telescope is
-- ADDRESSING VOCABULARY for the carrier, not one of the operations that stay
-- behind `𝔾.` to keep a layer's mirror names free.
open Base public

-- ══════════════════════════════════════════════════════════════════════════════
-- CERTIFYING A STRUCTURE AT EVERY DIMENSION, and reading it off at an address.
--
-- `Everywhere 𝒮 Ξ` carries an `𝒮` at `Ξ` and the same certification over every
-- parallel pair — so one former serves `Setoid`, `Category`, `Groupoid` and
-- every doctrine above them, rather than each tower being rebuilt by hand.
-- The point is what it does for STATEMENTS: a law or a reasoning combinator is
-- written once against the structure at a bound `Θ` and holds at every
-- dimension, instead of being restated per dimension or reached only through a
-- spine nothing can abstract over.
--
-- It asserts a structure at each dimension and NOTHING RELATING THE DIMENSIONS.
-- That is weaker than an ω-structure, deliberately and by the naming rule: what
-- is here is dimensionwise certification, so it is not called an ω-anything.
-- Interchange and the cross-dimensional coherences enter with the structures
-- that provide them, as their own fields, where they can be witnessed.
--
-- Note what it does NOT hold of, because the failure is informative: `≡°` puts
-- `𝟘` above its 1-cells, so `Everywhere Setoid (≡° A)` is UNINHABITED — there
-- are no 2-cells to be reflexive at. `𝔾.Id` is the carrier that does carry it,
-- and `Gandr.Setoid.Idˢ` is the instance. That is the `𝟘`-versus-`Id` choice
-- showing up as a theorem rather than as a preference.
-- ══════════════════════════════════════════════════════════════════════════════

record Everywhere {ℓ} (𝒮 : ∞Graph ℓ → Set ℓ) (Ξ : ∞Graph ℓ) : Set ℓ where
  coinductive
  field
    -- the structure at this dimension
    here° : 𝒮 Ξ
    -- and the same certification over each parallel pair
    up° : (x y : Ξ .ϵ°) → Everywhere 𝒮 (Ξ ▸ᵍ x ⇴ y)
open Everywhere public

-- Walking a telescope down to the address it names. The recursion is on the
-- CODE — inductive, injective constructors — which is exactly what the spine of
-- projections is not, and is the whole reason the code is there.
walk° : ∀ {ℓ} {𝒮 : ∞Graph ℓ → Set ℓ} {Ξ : ∞Graph ℓ}
  → Everywhere 𝒮 Ξ
  → (Θ : Disc Ξ)
  → Everywhere 𝒮 (⟦Disc⟧ Ξ Θ)
walk° 𝒮° ⋆ = 𝒮°
walk° 𝒮° (Θ ▸ᵈ x ⇴ y) = walk° 𝒮° Θ .up° x y

-- and reading the structure off there. At the empty telescope this is the
-- carrier-level structure on the nose, so nothing is paid for going through the
-- address.
at° : ∀ {ℓ} {𝒮 : ∞Graph ℓ → Set ℓ} {Ξ : ∞Graph ℓ}
  → Everywhere 𝒮 Ξ
  → (Θ : Disc Ξ)
  → 𝒮 (⟦Disc⟧ Ξ Θ)
at° 𝒮° Θ = walk° 𝒮° Θ .here°

-- THE PIN THAT MAKES THE ADDRESS FREE, and the reason a generic statement costs
-- a consumer nothing. At the empty telescope the lookup is the carrier-level
-- structure ON THE NOSE — both the walk and the interpretation compute away — so
-- anything parameterized by `at° 𝒮° Θ` agrees with its hand-written form at
-- `Θ = ⋆` definitionally, with no transport to insert and no use site to edit.
-- This is what bounds the migration to the telescope form: it is the same
-- signature with the depth abstracted, not a second way of saying things.
at°-⋆ : ∀ {ℓ} {𝒮 : ∞Graph ℓ → Set ℓ} {Ξ : ∞Graph ℓ}
  → (𝒮° : Everywhere 𝒮 Ξ)
  → at° 𝒮° ⋆ ≡ 𝒮° .here°
at°-⋆ 𝒮° = refl

-- ══════════════════════════════════════════════════════════════════════════════
-- CERTIFYING OVER A REGION OF ADDRESSES — which is what a structure whose
-- content STOPS actually has.
--
-- `Everywhere` demands the structure at every address, forever. That is the
-- right hypothesis for a carrier with genuine cells all the way up — `Id` and
-- the doctrines over it — and the wrong one for a structure whose content
-- stops, which is every `Set`-level structure in this tree.
--
-- The general form makes the REGION a parameter: `At 𝒮 Ξ P` certifies `𝒮` at
-- exactly the addresses satisfying `P`. `Everywhere` is the total region, a
-- `Set`-level structure is the singleton region `{⋆}`, and a bounded depth sits
-- between. One vocabulary covers all three, and which one a structure has is
-- stated in its signature rather than discovered.
--
-- ── WHY THIS GATES RATHER THAN LABELS, and it is the code that does it ───────
-- Out of region there is no structure to apply a combinator through, and the
-- author does not have to arrange that. `Disc` is a CODE with injective
-- constructors, so an out-of-region address is discharged by CONSTRUCTOR
-- DISJOINTNESS: `at⋆`'s second clause is `()`. Reasoning above a declared
-- region is therefore not merely unwise and not merely unmarked — the region
-- witness has no inhabitant, so the suite CANNOT BE FORMED there.
--
-- That is a second dividend from reifying the address. The code was introduced
-- so a statement could BIND its dimension; it also lets a statement REFUSE one.
--
-- ── WHAT THIS REPLACES, RECORDED SO IT IS NOT RE-PROPOSED ────────────────────
-- The alternative was to extend every `Set`-level structure upward until
-- `Everywhere` held. Two ways, both rejected on evidence:
--
--   * with `𝟙` — the certification above the content becomes a VACUOUS
--     category whose laws are `tt`, and the whole combinator suite is then
--     available at those addresses returning `tt`, with nothing at the use site
--     marking an informative application apart from an empty one. That is the
--     silent discharge the truncation rule exists to prevent, arriving through
--     the door uniformity opens.
--   * with `Id` — honest, and it does hold (`ℂ.Id°`), but it leaves "the
--     identity tower is the intended content" and "the tower is filler for a
--     region we do not observe" indistinguishable in the type.
--
-- Neither is needed. The region says which addresses carry content, so `≡°`
-- keeps its `𝟘` and nothing is extended, truncated, or marked.
-- ══════════════════════════════════════════════════════════════════════════════

-- The certification of `𝒮` over the addresses of `Ξ` that `P` admits.
At : ∀ {ℓ} (𝒮 : ∞Graph ℓ → Set ℓ) (Ξ : ∞Graph ℓ) (P : Disc Ξ → Set ℓ) → Set ℓ
At 𝒮 Ξ P = (Θ : Disc Ξ) → P Θ → 𝒮 (⟦Disc⟧ Ξ Θ)

-- The two ends of the range, named so a signature reads as a claim about where
-- a structure has content.

-- every address
Total : ∀ {ℓ} {Ξ : ∞Graph ℓ} → Disc Ξ → Set ℓ
Total _ = 𝕊.⊤

-- the carrier, and nothing above it
Only⋆ : ∀ {ℓ} {Ξ : ∞Graph ℓ} → Disc Ξ → Set ℓ
Only⋆ Θ = Θ ≡ ⋆

-- A BARE STRUCTURE IS THE CERTIFICATION OVER `Only⋆`, and the second clause is
-- the gate: there is no address above `⋆` for which a witness can be supplied,
-- so nothing can ask this structure for content it does not have.
at⋆ : ∀ {ℓ} {𝒮 : ∞Graph ℓ → Set ℓ} {Ξ : ∞Graph ℓ}
  → 𝒮 Ξ
  → At 𝒮 Ξ Only⋆
at⋆ 𝒮₀ ⋆ refl = 𝒮₀
at⋆ 𝒮₀ (Θ ▸ᵈ x ⇴ y) ()

-- AND A DIMENSION-WISE CERTIFICATION IS THE CERTIFICATION OVER `Total`, so
-- nothing statable against `Everywhere` is lost by stating it against `At`.
everywhere→At : ∀ {ℓ} {𝒮 : ∞Graph ℓ → Set ℓ} {Ξ : ∞Graph ℓ}
  → Everywhere 𝒮 Ξ
  → At 𝒮 Ξ Total
everywhere→At 𝒮° Θ _ = at° 𝒮° Θ

-- and reading either back at `⋆` is the structure on the nose, as `at°-⋆` is
-- one door up: the region costs a consumer at the carrier nothing.
at⋆-⋆ : ∀ {ℓ} {𝒮 : ∞Graph ℓ → Set ℓ} {Ξ : ∞Graph ℓ}
  → (𝒮₀ : 𝒮 Ξ)
  → at⋆ {𝒮 = 𝒮} {Ξ = Ξ} 𝒮₀ ⋆ refl ≡ 𝒮₀
at⋆-⋆ 𝒮₀ = refl
