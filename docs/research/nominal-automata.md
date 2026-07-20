# Nominal automata with name allocation & deallocation — a theory crate for gandr's Σ-lifecycle

_Research deliverable for gandr-fcw.6 (wayfinder epic gandr-fcw)._ _Primary sources deep-read: the two Erlangen name-allocation-automata papers, read against the ADR-41 `gandr-nominal` substrate and the wyrd-notes nominal corpus._ _Every claim is cited to the source that owns it._ _Paper theorem numbers are stated verbatim so the coordinator's plan can cite them directly._

Primary sources (aliases):

* **NDA** = `papers:2603.24468v1.pdf` — _Nominal Automata with Name Deallocation_, Simon Prucker, Stefan Milius, Lutz Schröder, arXiv:2603.24468v1, 25 Mar 2026 (26 pp; §§1–9 + Appendix A).
* **RNTA** = `papers:LIPIcs.CONCUR.2024.35.pdf` — _Nominal Tree Automata with Name Allocation_, Simon Prucker, Lutz Schröder, CONCUR 2024, LIPIcs vol. 311 art. 35, DOI 10.4230/LIPIcs.CONCUR.2024.35 (17 pp; full version arXiv:2405.14272).

Substrate & context: `wyrd@failed-refactor:crates/gandr-nominal/src/lib.rs` (ADR-41 atom/gensym), `wyrd@failed-refactor:docs/adr/0046-*.md` (capstone: wheeled-polarity-graded-nominal-PROP), `wyrd@failed-refactor:docs/adr/0007-*.md` (multiparty sessions), `…/0005-*.md` (linear zone Σ), `wyrd@failed-refactor:docs/gandr/spec/effects-control-shell.md` (POSIX shell DSL), `wyrd@failed-refactor:docs/gandr/spec/typing-machine.md` (Σ linear consumption), `wyrd-notes:archive/digest/props-successors-handoff.md`, `wyrd-notes:archive/digest/props-deepread-verified.md`.

---

## 1. Executive summary

1. **Three models, one family, one engineering trick.** RNNA (words, allocation only; the FoSSaCS 2017 foundation) is extended in two orthogonal directions: **NDA** adds _deallocation_ to words (malloc/free, non-nested scopes), and **RNTA** adds _tree/term_ structure to allocation.
   All three achieve _elementary_ decision procedures by the **same** template: close the literal language under α-equivalence (a _name-dropping modification_), restrict to a bounded finite name set, and reduce to **classical finite-alphabet NFA / NFTA**, where decisions are standard.
   This template is the crate's spine — a nominal automaton is a finite handle whose decision problems delegate to classical automata over a bounded alphabet (RNTA §5–6; NDA §6–7).

2. **NDA is the resource-lifecycle model, and it _determinizes_.** NDA's alphabet is literally `⟦a` (allocate/open `a`), `a⟧` (deallocate/close `a`), `⟦a⟧` (allocate-and-immediately-free) — the C `malloc`/`free` idiom, with interleaved _non-nested_ scopes (NDA §1, §3).
   Its headline surprise: **NDA can be determinized (Theorem 8.14)**, which the authors call "quite unusual"/"a rare feature" for nominal/register automata — normally deterministic models are strictly weaker.
   Explicit deallocation is _exactly_ what buys determinizability.
   This is the single most valuable property for gandr's **runtime-monitoring** use case (a forward monitor must be deterministic).

3. **Deallocation adds discipline & determinism, not expressivity.** Under local-freshness semantics NDA and RNNA are **equiexpressive (Proposition 5.11)**.
   So the payoff of deallocation is not a bigger language class; it is (a) determinizability (Thm 8.14, Cor 8.15), (b) a memory-safety discipline (_disciplined words_ = "free each resource as soon as it is dead", NDA Def 4.1), and (c) a Kleene-style **regular deallocation expression** algebra (Thm 7.19/7.20).

4. **RNTA is the term/session model.** It processes α-equivalence classes of trees carrying bound names (λ- and π-calculus terms, XML-like structured data).
   Under **global/branchwise** freshness it **generalizes session automata**; under **local** freshness it is a _lossiness-characterized subclass of nondeterministic register tree automata_ (RNTA §4 intro, Remark 5.6).

5. **Complexity is parametrized by degree = number of registers = number of concurrently-live names.** RNTA inclusion is **2-EXPTIME, and singly-exponential-parametrized in the degree** (Thm 6.3/6.6).
   NDA inclusion is **EXPSPACE, and PSPACE-parametrized in the degree** (Thm 5.12).
   Both are _elementary_ — the whole point, since full register/alternating models are usually non-elementary or undecidable (RNTA §1; NDA §1).
   For gandr this means: decisions stay tractable whenever the number of simultaneously-open endpoints/capabilities is bounded — the realistic session/shell regime.

6. **The crate's historically recorded scope was the Σ-class.** Per `wyrd@failed-refactor:docs/adr/0046-*.md` (line 54), a name-allocation automaton "genuinely unifies **only** the Σ-class (channels/caps/worlds/ roles) — it is _not_ the universal fabric", with macro-hygiene supports and CMTT-Ψ on their own machines.
   **That confinement is superseded as a standing posture** (owner direction at PLAN review, `bd comments gandr-fcw.14` amendment 2): nominal-automata adoption is re-evaluated **opportunistically at every build-out stage** as substantially new, potentially performance- and capability-enabling technology — explicitly including whether the automata machinery can **generalize** to encompass what ADR-46 left to separate machines (macro-hygiene supports, CMTT-Ψ).
   The theory-nominal-automata crate is the decision-procedure engine for the Σ / session / resource-lifecycle axis, built _on top of_ `gandr-nominal`'s `Atom`/`Gensym`.

7. **What it needs from substrates:** an **order-theory** layer (finite support as `BTreeSet<Atom>` with native `⊆`-poset; the term/word ordering `⊑` for local-freshness downward closure) and a **graph-theory** layer (orbit-finite state sets and transition relations are labeled graphs; the classical NFA/NFTA back-end needs reachability = emptiness, product/subset = inclusion, powerset = determinization).
   These align with gandr's already-planned `gandr-graph` (petgraph) and the supported-sets representation adopted in ADR-46.

**One contradiction to flag up front:** `wyrd-notes:archive/digest/props-successors-handoff.md` (line 29) annotates RNTA with "(the 'RNTA' acronym is the dialogue's invention)."
**The primary source refutes this** — `papers:LIPIcs.CONCUR.2024.35.pdf` (abstract, §4 Def 4.1) itself defines and uses "regular nominal tree automata (RNTA)".
The acronym is the authors', not a hallucination.

---

## 2. Shared foundations (both papers, `papers:*`)

### 2.1 Nominal-sets recap (RNTA §2; NDA §2)

* Fix a countably infinite set `𝔸` of _names_; `G = Perm(𝔸)` = finite permutations, generated by transpositions `(a b)`.
  A `G`-set has a group action; a map is _equivariant_ if `f(π·x)=π·f(x)`.
* `S ⊆ 𝔸` **supports** `x` if `Fix(S) ⊆ fix(x)` (every permutation fixing `S` fixes `x`).
  A **nominal set** = every element has a finite support; the least one is `supp(x)`.
  `a # x` ("`a` fresh for `x`") ⟺ `a ∉ supp(x)` (RNTA §2; NDA §2).
* **Orbit-finite** = finitely many orbits.
  This is the nominal analogue of "finite" and is what makes automata decidable. (RNTA §2; NDA §2, Lemma 2.1.)
* **Abstraction functor** `[𝔸](−)`: `[𝔸]X = (𝔸×X)/∼`, class `⟨a⟩x`, models α-equivalent binding; `⟨a⟩x` renaming into `b` is **blocked** when `b ∈ supp(x)` (RNTA §2, last ¶; NDA §2).
* **Strong nominal set** (used for the finitary representation): equivalently `X = Σ_{i∈I} 𝔸^{#Xᵢ}` where `Xᵢ` finite and `𝔸^{#Xᵢ}` = total **injective** maps `Xᵢ → 𝔸` (register stores with duplicate-free contents).
  "States = control state `i` + a store assigning names to registers in a duplicate-free manner" (NDA §5, before Lemma 5.1; RNTA §5, before Lemma 5.1).
  Orbit-finite strong nominal sets are exactly coproducts `⊔ⱼ 𝔸^{#nⱼ}` (NDA §5).

### 2.2 The name-allocation paradigm & RNNA (the common ancestor)

* **RNNA** = _Regular Nondeterministic Nominal Automata_, Schröder–Kozen–Milius–Wißmann, FoSSaCS 2017 (arXiv:1603.01455) — reference **[34] in RNTA / [21] in NDA**.
  It is the shared foundation of the whole Erlangen alloc/dealloc line (`wyrd-notes:archive/digest/props-successors-handoff.md` line 30).
  Both papers cast their model as an **extension of RNNA**.
* The paradigm: notions of freshness are based on **α-equivalence** (renaming bound names into names that have free occurrences later is _blocked_).
  In register terms this is a **lossiness** condition — "at every point, register contents may nondeterministically be erased (freeing the register)" (RNTA §1, ¶2).
  Name-allocating models impose **finite branching**.
  Inclusion is typically **elementary**, with low parametrized complexity in the **degree** (= number of registers) (RNTA §1, ¶2).
* **NOFA** (nondeterministic orbit-finite automaton) = the plain lift of NFA to `Nom`; equivalent to nondeterministic register automata with nondeterministic reassignment.
  Both nondeterminism and nondeterministic reassignment strictly _increase_ expressive power, and the class is **not closed under complement** (RNTA §2, "Nominal automata and register automata").
  RNNA/NDA/RNTA trade some of this register expressivity _for_ elementary inclusion checking.

---

## 3. Paper A — RNTA (`papers:LIPIcs.CONCUR.2024.35.pdf`)

### 3.1 What it is

A **nominal non-deterministic top-down tree automaton** following the name-allocation paradigm, for **data-tree languages** (XML documents, terms with binders).
Main result: RNTA inclusion checking is **elementary** (parametrized singly-exponential in the degree), giving "an efficiently decidable formalism for the specification of data words that admits full non-determinism and unboundedly many registers" (RNTA Abstract, §1 ¶3).

### 3.2 Terms, freshness semantics (§3)

* **Def 3.1 (Terms).** `T_𝔸(Σ)` nominal Σ-terms: `t ::= a.f(t₁,…,tₙ) | νa.f(t₁,…,tₙ)`, `f/n ∈ Σ`, `a ∈ 𝔸`.
  `a.f(…)` = node with a **free name**; `νa.f(…)` = node **allocating** a new bound name `a` scoped over the children (as in π-calculus / nominal Kleene algebra).
  Words are the unary- signature special case (RNTA Def 3.1, Remark 3.2).
* **Def 3.4.** `FN(t)` free names; α-equivalence `≡_α`; _clean_ (bound names mutually distinct & not free) and _non-shadowing_ terms.
  `dν(t)` strips all `ν` → **data trees**.
* **Three data-tree semantics** on an alphatic language `L` (RNTA §3, after Def 3.4):
  + `N(L) = {dν(t) | t clean, [t]_α ∈ L}` — **global** freshness (bound name fresh w.r.t. the whole tree);
  + `B(L) = {dν(t) | t non-shadowing, [t]_α ∈ L}` — **branchwise** freshness (fresh on its branch; siblings may reuse);
  + `D(L) = {dν(t) | [t]_α ∈ L}` — **local** freshness (fresh only where α-renaming is blocked, i.e. w.r.t. currently-stored names — "in the spirit of register automata", RNTA §3 after Ex 3.6).
* **Lemma 3.7.** `N` and `B` **preserve and reflect** language inclusion: for alphatic `L₁,L₂`, `L₁ ⊆ L₂ ⟺ N(L₁) ⊆ N(L₂) ⟺ B(L₁) ⊆ B(L₂)`. (So global/branchwise inclusion = alphatic inclusion; only local freshness `D` needs separate treatment — Thm 6.6.)

### 3.3 The automaton (§4)

* **Def 4.1 (RNTA).** `A = (Q, Δ, q₀)`: `Q` an **orbit-finite** nominal set of states, `q₀` initial, `Δ` an **equivariant** set of rewrite rules `q(γ.f(x₁,…,xₙ)) → γ.f(q₁(x₁),…,qₙ(xₙ))`, `γ ∈ Ā`.
  Two imposed properties: **α-invariance** and **finite branching up to α-equivalence**.
  `L(A)=L(q₀)` literal, `L_α(A)=L_α(q₀)` alphatic; `A` accepts data tree `s` under global/branchwise/ local freshness iff `s ∈ N(L_α(A))` / `B(L_α(A))` / `D(L_α(A))`.
* **Degree** `= max cardinality of supports of states` — "corresponds morally to the number of registers."
  States are written `q(a₁,…,aₙ)` = orbit `q` + stored names (RNTA §4, after Def 4.1; p.35:9).
  _This is the exact analogue of a gandr Σ-state: a control point plus the set of live endpoint atoms._
* **Lemma 4.2 (support monotonicity).** (1) if `q(a.f(…)) → a.f(q₁(x₁),…)` then `supp(qᵢ) ∪ {a} ⊆ supp(q)`; (2) if `q(νa.f(…)) → νa.f(…)` then `supp(qᵢ) ⊆ supp(q) ∪ {a}`.
* **Corollary 4.3.** If `q` accepts `t`, then `FN(t) ⊆ supp(q)`. (Free names of an accepted term are bounded by the state's register contents.) Remark 4.4 restricts to empty-support initial states ⇒ accepted language is **closed**.
* **Relation to models (§4 intro):** global/branchwise ⇒ RNTA **generalizes session automata** ([6] = Bollig–Habermehl–Leucker–Monmege, LMCS 2014); local ⇒ RNTA = a **subclass of nondeterministic register tree automata** ([21] = Kaminski–Tan) characterized by a _lossiness_ condition (Remark 5.6).
  Trades register expressivity for elementary inclusion (RNTA §1, Ex 3.6).
* **Examples:** universal data-tree language, "root letter reappears in all leaves" (Ex 4.5); **XML-like structured data** with `⟦elem⟧…eof` nesting (Ex 4.6); **π-calculus** channel allocation/reading (Ex 4.7). λ-calculus terms in Ex 3.5.
* **Remark 4.8 (coalgebra).** RNTAs are coalgebras for `F X = P_ufs( Σ_{f/n∈Σ} (𝔸 × Xⁿ + [𝔸]Xⁿ) )` — `P_ufs` = uniformly-finitely-supported powerset (nondeterminism); `𝔸×Xⁿ` = free-name transitions; `[𝔸]Xⁿ` = bound/allocating transitions.
  _This gives the crate a generic categorical shape; the authors flag coalgebraic generality (weighted/ probabilistic branching) as future work (§7)._

### 3.4 Name dropping (§5) — closing the literal language under α

The literal language of a name-allocating automaton must be **closed under α-equivalence** so only boundedly many names matter for inclusion; this does **not** hold in general and must be _engineered_ (RNTA §5 opening).
The fix: add states that **drop** some names from the support of predecessor states.

* **Lemma 5.1.** Every RNTA has an equivalent RNTA whose state set is a **strong** nominal set.
* **Def 5.2/5.3.** Restriction to **partial injective** maps `r: Xᵢ ⇀ 𝔸` (registers may be empty), written `𝔸^{$X}`; the **name-dropping modification** `A_⊥ = (Q_⊥, Δ_⊥, q₀)`.
* **Theorem 5.5.** _For each RNTA `A`, the name-dropping modification `A_⊥` is an RNTA that accepts the **closure of the literal tree language of `A` under α-equivalence**, hence the **same alphatic tree language**. `A_⊥` has the same degree `d`, and its number of orbits exceeds that of `A` by at most a factor `2^d`._
  (The `2^d` = the number of ways to delete names from a support of size `d`.)
* **Remark 5.6 (Lossiness).** In the automata↔register correspondence, name-dropping is a **lossiness** property: during any transition, letters may be nondeterministically lost from the registers.
  Distinctness of the current letter from an earlier one can be enforced _only if the earlier one is expected to be seen again_ (RNTA Remark 5.6, Ex 3.6/4.5).

### 3.5 Inclusion checking (§6) — the theorems

* **Def 6.1.** `T_S(Σ) = {t ∈ T_𝔸(Σ) | supp(t) ⊆ S}` (terms whose names all lie in finite `S`).
* **Lemma 6.2.** For RNTA of degree `d_A`, max symbol arity `n_ar`, and any `S` with `|S| = d_A·n_ar + 1`: if `A` accepts `t`, it accepts some `t' ∈ T_S(Σ)` with `t' ≡_α t`. (**A bounded name alphabet suffices** to witness every α-class.)
* **Theorem 6.3 (main).** _Alphatic tree language inclusion `L_α(A) ⊆ L_α(B)` of RNTAs `A,B` of degrees `d_A,d_B`, over the fixed signature Σ, is decidable in **doubly exponential time**, and in fact in **parametrized singly exponential time with the degree as the parameter**, i.e. exponential in a function that depends exponentially on `d_A + d_B` and polynomially on the size of `A,B`._
  (Recall NFTA inclusion is **EXPTIME-complete** — [35] Seidl 1990 — so the finite-alphabet floor is already EXPTIME; RNTA §5 end.)
  + _Proof shape (RNTA §6):_ reduce to NFTA inclusion.
    Take `S` with `|S|=d_A·n_ar+1` (Lemma 6.2), build the name-dropping modification `B_⊥` (Thm 5.5), then **S-restrict** `A` and `B_⊥` to finite NFTAs `A_S`, `B_S` over `S̄ × Σ`; classical NFTA inclusion applies.
    Sizes: `#states(A_S) =` `#orbits(A) × (singly-exp in degree)`; name-dropping adds an exponential factor in `d_B` but leaves the degree unchanged; each orbit of support size `m` has ≤ `m!` elements with a fixed support.
* **Def 6.4/Lemma 6.5.** A term/word ordering `⊑` on `T_Ā(Σ)` (via `a ≤ νa`) with downward closure `↓L`, needed for the local-freshness case.
* **Theorem 6.6 (local freshness).** _Language inclusion `D(L_α(A)) ⊆ D(L_α(B))` under **local freshness** of RNTAs `A,B` of degrees `d_A,d_B` over fixed Σ is decidable in **doubly exponential time**, and in fact in **parametrized singly exponential time with the degree as the parameter**._ (Same bound as Thm 6.3; via Lemma 6.5 and downward-closure of the NFTA in step 2.) From Lemma 3.7, the **global** (`N`) and **branchwise** (`B`) freshness inclusion problems inherit the Thm 6.3 bound directly (RNTA p.35:14, after Thm 6.3).

### 3.6 Conclusions / gaps (§7)

RNTAs = a species of nondeterministic top-down nominal tree automata; less expressive than the full register model but admit **elementary** inclusion with unbounded registers + unrestricted nondeterminism; native name allocation lets them process λ- and π-calculus terms.
**Future work:** name allocation over **infinite** trees (nominal μ-calculus [23] Klin–Łełyk); coalgebraic generality (weighted/probabilistic branching).
**Not treated:** top-down **determinization** (Van Heerdt et al. [41] do bottom-up determinization but it "produces orbit-infinite automata"); **emptiness** is not a headline theorem (see §5.3 below).

---

## 4. Paper B — NDA (`papers:2603.24468v1.pdf`)

### 4.1 What it is & why it matters

"Data words with binders formalize concurrently allocated memory."
Most binding mechanisms (λ-calculus) enforce **nested** scoping; stateful languages with `malloc`/`free` **interleave** the scopes of allocated memory regions **in any order** (NDA Abstract; §1 shows the C `int *num1 = malloc(...); … free(num1);` fragment as the motivating idiom).
**NDA** = non- deterministic **deallocation** automata, extending RNNA by **deallocating transitions**.
Three headline contributions (NDA §1):

1. a **name-dropping modification** closing the language under α-equivalence (§6);
2. a **Kleene theorem**: NDA ≡ **regular deallocation expressions** (§7);
3. a **determinization** construction preserving local-freshness semantics (**§8**) — "a quite unusual phenomenon in the realm of regular and nominal automata, where non-deterministic models typically have strictly higher expressivity than their deterministic restrictions." _All that needs to be added to RNNA to allow determinization is explicit deallocation._

### 4.2 Explicit deallocation (§3)

* Extended alphabet `Â = { ⟦a, a⟧, ⟦a⟧ | a ∈ 𝔸 }` over binders `⟦·` and markers `·⟧` (NDA §3):
  + `⟦a` — **opening / allocating** name `a` (`malloc`);
  + `a⟧` — **closing / deallocating** name `a` (`free`);
  + `⟦a⟧` — **allocating and immediately deallocating** `a`.
* Sets `RC(w)` (**right-closed**: last appearance of `a` has the marker), `LO(w)` (**left-open**: first appearance lacks the binder), `LC(w)` (**left-closed**: first appearance has the binder), defined recursively in **Table 1** (NDA §3).
  `RNS(Â)` = **right-non-shadowing** words.
* **Fact 3.6.** For `w ∈ RNS(Â)`, `supp([w]_α) = LO(w)` — the α-class support = the left-open names.
* α-equivalence `≡_α` = renaming of bound names (Def 3.5).
  Allocating the same resource twice in a row without an intervening free is "bad style" but technically possible (`⟦a⟦a` is right-non-shadowing); **deallocating** the same resource twice, or using a resource after deallocation (`a⟧a`), is "technically impossible" — those are right-*shadowing* and excluded (NDA §3, Ex 3.4).

### 4.3 Semantics & memory-safety discipline (§4)

* **Debracketing** `db: RNS(Â) → 𝔸*` erases delimiters → the data word.
  **Local freshness** `D(L_α) = { db(w) | [w]_α ∈ L_α }` (NDA §4).
* **Def 4.1 (disciplined words).** `w` is _disciplined_ if for every split `w = uv`, whenever a name is free/left-open across the split it is right-closed at its last use — i.e. **every resource is deallocated as soon as it is no longer needed** (memory-safe; `⟦a⟦bb⟧` _leaks_ `a`; the disciplined form is `⟦a⟧⟦bb⟧`).
  `disc(w)` = disciplined rewrite.
* **Lemma 4.4.** `D([w]_α) = D([disc(w)]_α)` — **the memory-safe rewrite does not change the local- freshness data language.** _This is gandr's "linearity errors are static, not runtime crashes" story at the automata level: the safe program and the leaky program denote the same data language, and the discipline is a checkable structural property._

### 4.4 The automaton (§5) — the theorems

* **Def 5.1 (NDA).** `A = (Q, Δ, i, F)`: `Q` **orbit-finite** nominal set of states, `Δ ⊆ Q × Â × Q` **equivariant** transition relation, `i` initial, `F` equivariant final.
  Three conditions: **left α-invariance**; **name erasure** (if `q →⟦a⟧ q'` or `q →a⟧ q'` then `a # q'` — _deallocated names are actually forgotten_); **finite branching**.
  Degree `deg(A)=deg(Q)= max_{x∈Q}|supp(x)|`.
* **Example 5.6 (sessions!).** An NDA accepting **valid logs of sessions with ≤ 2 participants (a, b) and an admin (c)**, accepting all logs where all users except the admin are logged out at the end.
  `⟦a` = user `a` logs in (allocates), `a⟧` = logs out (deallocates); `c` (admin) is a free transition (acts without login); a blocklisted user `d` can never log in or out (its `⟦d` / `d⟧` is blocked everywhere).
  _This is directly a multi-party-session lifecycle monitor._
  (NDA §5, Fig. 2.)
* **Lemma 5.7 (Support lemma).** For all `q →γ q'`: (1) `q →a q'` ⇒ `supp(q') ∪ {a} ⊆ supp(q)` (free move: `a` must be in memory); (2) `q →⟦a q'` ⇒ `supp(q') ⊆ supp(q) ∪ {a}` (allocate: adds `a`); (3) `q →a⟧ q'` ⇒ `supp(q') ⊆ supp(q) \ {a}`, `a ∉ supp(q')` (deallocate: removes `a`); (4) `q →⟦a⟧ q'` ⇒ `supp(q') ⊆ supp(q) \ {a}`.
  "Analogy to the register paradigm: transitions for `a` or `a⟧` can only be taken if `a` is in memory; `⟦a` adds `a`; `a⟧`/`⟦a⟧` erase `a`." (NDA §5.)
* **Proposition 5.8.** Every word accepted by an NDA is **right non-shadowing**.
* **Lemma 5.9 (ε-elimination).** ε-transitions can be removed (standard).
* **Proposition 5.11.** _Under local freshness semantics, **NDA and RNNA are equiexpressive**._ (Every NDA has a data-language-equivalent RNNA by replacing `a⟧ → a`, `⟦a → |a`.) ⇒ NDA inherit RNNA's tractability.
* **Theorem 5.12 (inclusion).** _Under local freshness semantics, **language inclusion of NDAs is decidable in exponential space, in fact parametrized polynomial space, with the degree as the parameter**._
  (Via the RNNA translation + RNNA inclusion, [21, Cor 7.4]; degree = number of registers.)

### 4.5 Name dropping & closure under α (§6)

* **Prop 6.2.** Every NDA has an equivalent NDA with a **strong** nominal state set.
* **Construction 6.3 (Name-dropping modification).** `A_⊥ = (Q_⊥, Δ_⊥, i, F_⊥)` over partial injective register maps `𝔸^{$nⱼ}`, adding transitions that drop names from supports.
* **Theorem 6.9.** _The name-dropping modification **closes the language of an NDA under α-equivalence**._
  (Literal language of `A_⊥` is α-closed, and its alphatic language coincides with `A`'s — Lemmas 6.6, 6.8.)

### 4.6 Kleene theorem (§7) — regular deallocation expressions

* **Def 7.1 (regular deallocation expression).** `r ::= ∅ | ε | ⟦a | a⟧ | ⟦a⟧ | ? | r₁·r₂ | r + r | (r₃)*` with side conditions `RC(r₁) ∩ LO(r₂) = ∅` (concatenation) and `RC(r₃) ∩ LO(r₃) = ∅` (star) — i.e. syntactic guards keeping every generated word **right-non-shadowing**.
  `?` = the "unknown"/free letter.
  **Lemma 7.2:** deciding whether a classical regexp over `Â` is a regular deallocation expression is decidable.
* **Def 7.4 (D-NFA).** A classical NFA over `Â` with, per state `q`, disjoint `RC_A(q) ∩ LO_A(q) = ∅` — the finite-alphabet shadow of an NDA.
* **Theorem 7.19.** _For every regular deallocation expression `r`, there is a D-NFA `A` with `L_α(r) = L_α(A)`._
  (Kleene constructions + ε-elimination.)
* **Theorem 7.20.** _For every D-NFA `A`, there is a regular deallocation expression `r` with `L(r) = L(A)`._ (State-elimination.)
* ⇒ **Kleene theorem:** regular deallocation expressions ≡ D-NFA; and via Constructions 7.8 (nominalization) / 7.11 (S-restriction) both translate to/from NDA (NDA Fig. 3).
  _This gives the crate a **surface syntax** for alloc/dealloc protocols (a regex-like DSL) that compiles to the automaton — directly usable as a session/resource-protocol specification language._

### 4.7 Determinization (§8) — the standout result

* **Example 8.1.** **RNNA cannot be determinized** (the language "some letter has occurred before" needs unbounded support in a deterministic RNNA).
  This is the usual nominal-automata state of affairs.
* **Def 8.2 (DDA).** A **deterministic deallocation automaton** = an NDA whose `Δ` is (partially) deterministic: for each `q, γ`, at most one `q'` with `q →γ q'`.
* The enabling **discipline** (NDA §8, before Def 8.2): require the automaton to _drop names only when the transition label already forgets them_ — `⟦a` always allocs, `a⟧` always deallocs, a free `a` keeps `a` in memory, `⟦a⟧` allocs+immediately frees.
  "This discipline is the key to enabling determinization … it is particularly striking that explicit deallocation is all one needs."
* **Construction 8.3 (Disciplined S-restriction)** + **Lemma 8.7** (nominalization of a disciplined D-DFA is a DDA).
* **Theorem 8.14 (Determinizability of NDA).** _For every NDA `A`, there exists a DDA `A_DDA` such that `L_D(A_DDA) = D(L_α(A))`_ — determinization **preserving the data language under local- freshness semantics**.
  Procedure (5 steps): name-dropping mod `A_⊥` (Constr 6.3) → S-restriction `A_S` with `|S| = deg(A_⊥)+1` (Constr 7.11) → disciplined S-restriction `A_D` (Constr 8.3) → classical **powerset** DFA of `A_D` → **nominalize** (Constr 7.8).
* **Corollary 8.15.** _An RNNA whose initial state has empty support is determinizable **as an NDA** regarding data languages._ _For gandr: an allocation-only session/protocol spec (RNNA) can be cast to NDA and determinized into a runnable forward monitor._
* **Remark 8.8.** The powerset construction **fails** for RNNA directly (closing the transition relation under α-invariance re-introduces nondeterminism).
  Determinization is _specific to the deallocation model._

### 4.8 Conclusions (§9)

NDA extend RNNA with deallocating + unknown transitions; NFA-type representation gives a Kleene theorem (regular deallocation expressions, equiexpressive under literal-language semantics); NDA can be **determinized under local freshness while preserving the data language** — "a rare feature in the realm of nominal automata."
Future: algebraic/coalgebraic semantics, complexity of related decision problems, infinite words.

---

## 5. The unifying engineering template (both papers)

Both papers reach elementary decidability by the **same three-step reduction** — this _is_ the crate's core algorithm and should be a shared module, not duplicated per model:

```text
nominal automaton (orbit-infinite literal language)
  │  ① name-dropping modification         RNTA Thm 5.5 / NDA Constr 6.3, Thm 6.9
  ▼      (close literal language under α; orbit blowup ≤ 2^degree)
α-closed nominal automaton
  │  ② S-restriction to a bounded name set  RNTA Lemma 6.2 (|S|=d·n_ar+1) / NDA Constr 7.11 (|S|=deg+1)
  ▼
classical finite-alphabet automaton over S̄×Σ  (NFTA for RNTA; NFA/D-NFA for NDA)
  │  ③ classical decision procedure
  ▼      inclusion (product/complement), emptiness (reachability), determinization (powerset)
answer
```

Consequences for the crate:

* The **classical finite automaton back-end is load-bearing** and reusable across all three models: NFA (words) and NFTA (trees), each with emptiness (reachability), inclusion (product + complement/subset), and — for words — determinization (powerset).
  RNTA §6 reduces to **NFTA inclusion (EXPTIME-complete, [35] Seidl)**; NDA §7–8 reduce to **NFA / DFA**.
* **Degree = number of registers = number of concurrently-live names** is _the_ complexity knob.
  Everything is parametrized by it. gandr should surface degree as a first-class budget on Σ-states.
* The whole approach is the concrete realization of the **supported-sets** thesis adopted in `wyrd@failed-refactor:docs/adr/0046-*.md` (line 43) and `wyrd-notes:archive/digest/props-deepread-verified.md` (§Supported sets): a _finite handle_ (bounded name set `S`, orbit counts, symmetry groups) on an orbit-infinite object.

---

## 6. What a `theory-nominal-automata` crate should contain

### 6.1 Layering on `gandr-nominal` (ADR-41)

`wyrd@failed-refactor:crates/gandr-nominal/src/lib.rs` provides only `Atom<S>` (sort-tagged machine-minted name) and the monotone `Gensym<S>` allocator, plus the `is_unifiable` atom-vs-variable boundary.
Its module doc _explicitly reserves_ (lines 50–59) the exact pieces these papers need: `Perm` (swapping-list permutation + action + freshness `#`), the finite-**support** skin (`BTreeSet<Atom>` with native `⊆`), nominal unification, and — verbatim — "the explicit-dealloc **automaton** that reclaims atoms (arrives with `Σ` sessions; the M1 allocator here is the monotone counter that never reclaims)." **The theory-nominal-automata crate is that reserved automaton layer.** It must build, in order:

1. **`Perm`** = finite permutation as a swapping list / transposition codes (matches the Urban–Pitts–Gabbay representation cited in the `gandr-nominal` doc and the Agda/Rocq mechanizations, `wyrd-notes:archive/digest/props-successors-handoff.md` line 31; keeps MGUs **unitary** per the `is_unifiable` boundary).
2. **`Support`** = `BTreeSet<Atom<S>>`, freshness `a # x`, equivariance — the finite serializable support skin (widens `Sort` with `Ord`/`Hash`, as the ADR-41 doc anticipates).
3. **Orbit-finite / strong nominal state sets** (see §6.4).
4. **The three automata models** (§6.2) sharing the reduction template of §5.
5. **Classical NFA/NFTA back-end** (§6.3) — the decision engine.

### 6.2 Automata models to include

| Model                                         | Data            | Names               | Determinizable?        | gandr role                                             |
| --------------------------------------------- | --------------- | ------------------- | ---------------------- | ------------------------------------------------------ |
| **RNNA** (foundation, [34]/[21])              | words           | alloc only          | **No** (NDA Ex 8.1)    | allocation-only protocol specs; ancestor               |
| **NDA** (`papers:2603.24468v1.pdf`)           | words           | **alloc + dealloc** | **Yes** (Thm 8.14)     | **resource lifecycle / malloc-free; runtime monitors** |
| **RNTA** (`papers:LIPIcs.CONCUR.2024.35.pdf`) | **trees/terms** | alloc               | not treated (top-down) | **session protocols; term/AST/XML data; π/λ**          |

Include all three.
RNNA is the shared base type; NDA and RNTA are its word-with-deallocation and tree-with-allocation extensions respectively. (A _tree-with-deallocation_ model is **not yet in the literature** — see §9 hazards; both papers list infinite-tree / richer branching as future work, RNTA §7, NDA §9.)

### 6.3 Decision procedures to include

* **Inclusion** — the flagship for both papers.
  + RNTA alphatic/global/branchwise: **Thm 6.3** — 2-EXPTIME, singly-exp-parametrized in degree.
  + RNTA local freshness: **Thm 6.6** — same bound.
  + NDA local freshness: **Thm 5.12** — EXPSPACE, PSPACE-parametrized in degree.
* **Determinization** — **NDA only: Thm 8.14** (DDA), **Cor 8.15** (RNNA-with-empty-support-init → NDA).
  RNNA is _not_ determinizable (Ex 8.1); RNTA top-down determinization is _not treated_ (bottom- up produces orbit-infinite automata, [41]).
  Expose determinization as an **NDA-specific capability** and document the RNNA/RNTA impossibility/absence honestly.
* **Emptiness** — _neither paper states an emptiness theorem._ It is **obtainable via the §5 template**: after name-dropping + S-restriction the problem is emptiness of a classical NFA/NFTA (reachability of a final state), which is standard and cheap (NFTA emptiness in P; NFA emptiness in NL).
  **State this as an engineering inference, not a cited theorem** — the papers focus on inclusion, determinization, Kleene.
* **Kleene / expression compiler** — **NDA: Thm 7.19/7.20** — a regular-deallocation-expression surface syntax (`⟦a`, `a⟧`, `⟦a⟧`, `?`, `·`, `+`, `*` with non-shadowing guards) ⇄ D-NFA ⇄ NDA.
  Ship this as the **protocol/spec DSL** for Σ-lifecycles.
* **Membership / run** — accept a data word/tree (forward simulation; on a **DDA** this is deterministic and online — the runtime-monitor path).

### 6.4 Representation choices for orbit-finite sets

Take these _directly from the papers_ (they are the finitary representations that make the code finite):

* **Strong nominal set** `X = Σ_{i∈I} 𝔸^{#Xᵢ}` (`Xᵢ` finite; `𝔸^{#Xᵢ}` = total injective register stores) — a state = `(control index i, injective register assignment)` (NDA §5; RNTA §5, Lemma 5.1).
  Orbit-finite strong nominal sets are exactly `⊔ⱼ 𝔸^{#nⱼ}` (NDA §5).
* **Partial injective maps** `𝔸^{$X}` (`r: X ⇀ 𝔸`, registers may be **empty**) for the **name- dropping** modification (RNTA Def 5.2; NDA §6).
  Orbit count grows by `≤ 2^degree` (RNTA Thm 5.5).
* **Finitary orbit encoding**: "standard finitary representations of orbit-finite nominal sets … enumerate the support sizes and symmetry groups of the orbits" (RNTA §6, after Thm 6.3).
  Each orbit of support size `m` has `≤ m!` elements with a fixed support (RNTA §6 proof).
* **Bounded name alphabet** `S` with `|S| = degree·n_ar + 1` (RNTA Lemma 6.2) or `degree + 1` (NDA Constr 7.11) — the finite name pool the reduction runs over.
* **Support as `BTreeSet<Atom>`** with native `⊆` (the ADR-46-adopted supported-sets skin; `wyrd@failed-refactor:docs/adr/0046-*.md` line 43, 53).
  _Serialize the support only; the renaming action is a derived monad, reconstructed not stored_ (`wyrd-notes:archive/digest/ props-deepread-verified.md` §3 "N-pillar re-cut").

---

## 7. Application mapping — how the crate serves gandr

### 7.1 Multi-party sessions (`…/0007-*.md`, `…/0005-*.md`)

* gandr adopts **global types + projection + role-indexed local types + `MCut`** (ADR-7); the linear zone **Σ** holds session endpoints, delegated endpoints, and world capabilities, all with **no weakening/contraction** (ADR-5).
  Binary sessions = the 2-role special case.
* **RNTA under global/branchwise freshness generalizes session automata** (RNTA §4 intro, [6]) — so the crate can represent a projected local type / session protocol as an RNTA and **check protocol refinement/subtyping as language inclusion** (Thm 6.3).
  Roles/participants are allocated names (`νa`), giving _fresh-participant_ protocols natively.
* **NDA Example 5.6 is literally a session-lifecycle monitor**: users log in (`⟦a` alloc) / out (`a⟧` dealloc), an admin acts freely, blocklisted principals are permanently blocked — "all users except the admin logged out at the end" is a Σ-drain condition.
  Endpoint **fork/spawn = allocation**, **close = deallocation**; **degree = number of concurrently-open endpoints**.

### 7.2 Resource lifecycle — alloc/dealloc à la malloc/free (`…/typing-machine.md`)

* **NDA is the purpose-built model.** Its alphabet is malloc/free (`⟦a`/`a⟧`, NDA §1, §3), scopes are **interleaved and non-nested** (unlike λ-binding) — exactly gandr's Σ where endpoints/capabilities are opened and consumed in arbitrary order.
* **Disciplined words = memory safety** (NDA Def 4.1): "deallocate each resource as soon as it is no longer needed"; `⟦a⟦bb⟧` _leaks_ `a`.
  This is the automata-level twin of gandr's `typing-machine.md:306` rule — _"leftover linear resources at `Done` are a `LinearityError` listing the unconsumed endpoints … fidelity violations surface as **partial derivations** rather than crashes."_ **Lemma 4.4** guarantees the safe rewrite `disc(w)` denotes the **same** data language — i.e. discipline is a checkable structural constraint, not an expressivity change.
* The **PLACES'19 adjoint-logic reading** already in the corpus (`wyrd-notes:archive/digest/ props-deepread-verified.md` §Dealloc, lines 91–94): `spawn`(cut)=alloc, `close`(terminal)=normal dealloc, `drop`(weakening)= **cancellation = "a logically justified form of garbage collection"**.
  The `⟦a⟧` (alloc-and-immediately-free) letter models the drop/cancellation case.

### 7.3 Runtime monitoring (VISION typed-trace/audit goal)

* gandr's VISION frames auditability as " **what happened as a typed trace, not a log to grep**" and makes the runtime the governance surface (`wyrd@failed-refactor:docs/gandr/VISION.md` lines 59, 116; cited via grep).
  A **forward runtime monitor must be deterministic** to consume a live event trace.
* **NDA determinization (Thm 8.14) is the enabling result** and is _unique to the deallocation model_ (RNNA can't be determinized, Ex 8.1; NDA §8).
  So: specify a resource/session protocol as an NDA (or a regular deallocation expression, §4.6), **determinize to a DDA**, and run it as an online monitor over the alloc/dealloc event stream.
  **Corollary 8.15** lets an allocation-only RNNA spec be cast to NDA and determinized.
  _Caveat (record honestly): "runtime monitoring" is a **gandr-side design inference** — neither paper uses the phrase; the papers supply the determinization theorem, gandr supplies the monitoring application._ The temporal-logic direction (nominal μ-calculus [23]; linear- time nominal μ-calculus with name allocation, NDA [16] = Hausmann–Milius–Schröder MFCS 2021) is the natural next layer for _property_ monitors, flagged by both papers as future work.

### 7.4 Shell layer POSIX modeling (`…/effects-control-shell.md`)

The shell spec (§3 table) elaborates POSIX onto Σ constructs — every row is a named-resource lifecycle the crate can model/verify:

| POSIX construct           | gandr construct (spec §3)                                                          | Automata model                                                                  |
| ------------------------- | ---------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| process / `exec`          | `Shell` op spawning a computation at a job world; **stdio = three endpoints in Σ** | NDA alloc of 3 endpoint names                                                   |
| pipe `p \| q`             | `fork` of byte-stream session `Pipe = μX.⊕{chunk:!Bytes.X, eof:end}`               | RNTA session protocol / NDA endpoint pair                                       |
| redirection `>f`, `2>&1`  | **endpoint delegation** — "rebinding which channel a job's stdio names refer to"   | **nominal renaming** (`Perm` action)                                            |
| subshell                  | child world (or `fork`)                                                            | scope allocation                                                                |
| `ssh w -- cmd`            | `migrate_w(shell cmd)`                                                             | (worlds; capability move)                                                       |
| word expansion / globbing | macro-phase (phase 0)                                                              | _outside_ the automata scope (macro hygiene keeps its own machine, ADR-46 l.54) |

* **Redirection = endpoint delegation = the nominal renaming action** — this is precisely why the Σ-lifecycle wants a _nominal_ substrate: rebinding a name is `Perm`/α-renaming, and "every shell footgun lands on a static discipline: dangling pipes → linear Σ" (spec §3, obs. 2).
  Open/close of fds and pipe endpoints = NDA `⟦a`/`a⟧`; **degree = number of simultaneously-open fds/endpoints**.
* The spec's own claim — "POSIX job control **literally is** first-class one-shot stacks; a terminated job with open pipes = exactly the unwind-obligation discipline of §2.4" — is the same memory-safety/leak-freedom property NDA's _disciplined words_ formalize (§7.2).
* **Scope boundary:** word-expansion/globbing is macro-phase (phase 0) and is **not** the automata crate's job today — ADR-46's (l.54) original Σ-class confinement; whether the automata machinery should absorb such sites is part of the standing per-phase re-evaluation (finding 6).

---

## 8. Graph- and order-theory substrate needs

**Order theory:**

* **Support as a `⊆`-poset.** `supp` is a `BTreeSet<Atom>`; scope-set resolution is `⊆`-maximal selection (`wyrd@failed-refactor:docs/adr/0046-*.md` l.53: "`P_f(𝔸)` is a nominal set already in classical Pitts, Ex 2.10; supported sets buy the engineering _fit_").
  Lemma 4.2 (RNTA) / Lemma 5.7 (NDA) are **monotonicity laws over this poset** (support shrinks on dealloc/free-move, grows on alloc) — the crate's invariants are poset inequalities.
* **The term/word order `⊑`** (RNTA Def 6.4, via `a ≤ νa`) with **downward closure `↓L`**, required for the local-freshness inclusion procedure (Thm 6.6, Lemma 6.5).
  The crate needs a downward-closure operator over this order.
* **Degree** is an integer budget ordering the complexity of every procedure (§5).
* (Adjacent, Grove pillar) merge-correctness is a **join-semilattice / 2P-Set CmRDT** order (`wyrd-notes:archive/digest/props-deepread-verified.md` G1/G4) — a _distinct_ order layer from Σ linear removal; keep them separate (ADR-46 l.54; digest §2.5).

**Graph theory:**

* **The automata are labeled graphs**: orbit-finite state set + equivariant transition relation `Δ ⊆ Q × Â × Q` (NDA Def 5.1) / rewrite rules (RNTA Def 4.1).
  Storing, traversing, and quotienting these by orbit is graph work — aligns with gandr's planned **`gandr-graph` (petgraph)** (`wyrd@failed-refactor:docs/gandr/spec/proposal-graph-core.md`).
* **The classical back-end (§5 step ③) is pure finite-automata graph algorithmics:** emptiness = **reachability** of a final state; inclusion = **product + complement/subset**; determinization = **powerset** (NDA §8) over the state graph; tree case = **NFTA** product/emptiness (RNTA §6).
* **Coalgebraic shape** (RNTA Remark 4.8): `F X = P_ufs(Σ_{f/n}(𝔸×Xⁿ + [𝔸]Xⁿ))` — a functor over a graph-of-states; a generic coalgebra layer would let the crate host the authors' future weighted/ probabilistic branching without rework.
* **Σ-IR alignment:** ADR-46 pillar 2 is a "polarity-sorted/graded/wheeled **string-diagram Σ-IR** (cospan of monogamous hypergraphs)" (`…/0046-*.md` l.45) — the automata's transition graph and the Σ-IR hypergraph are the same substrate viewed two ways; the crate's graph layer should be shareable with the Σ-IR.

---

## 9. Hazards, surprises, contradictions, adjacent discoveries

1. **CONTRADICTION (notes vs primary source).** `wyrd-notes:archive/digest/ props-successors-handoff.md` l.29 says "(the 'RNTA' acronym is the dialogue's invention)".
   **False:** `papers:LIPIcs.CONCUR.2024.35.pdf` (abstract, §4 Def 4.1) itself coins and uses "regular nominal tree automata (RNTA)".
   Correct the note.
2. **SURPRISE / KEY ASSET.** NDA determinizability (Thm 8.14) is explicitly flagged by the authors as "quite unusual"/"a rare feature" — deterministic nominal models are normally _weaker_.
   Explicit deallocation is the _sole_ addition that unlocks it (NDA §8, Rmk 8.8).
   This is the crate's highest- value property for gandr and should anchor the runtime-monitoring plan.
3. **SUBTLETY.** Deallocation adds **no data-language expressivity** under local freshness (NDA Prop 5.11: NDA ≡ RNNA).
   Its value is determinism + memory-safety discipline + a Kleene algebra — _not_ a larger language class.
   Do not sell NDA as "more expressive than RNNA"; sell it as "the determinizable, disciplinable presentation of the same class."
4. **GAP in the literature (both papers).** There is **no tree-with-deallocation** model yet: RNTA is allocation-only (RNTA §7 future work = infinite trees / nominal μ-calculus); NDA is words-only. gandr's Σ is _both_ tree-structured (session protocols, process trees) _and_ dealloc-heavy (close/free).
   Combining them (an "RNTA + deallocation" / determinizable tree model) is **open research** — flag as a construction obligation / potential publishable contribution, not something to assume exists.
5. **EMPTINESS is not a stated theorem** in either paper (both center on _inclusion_, _determinization_, _Kleene_).
   It follows from the finite-alphabet reduction (§5) but the plan must cite it as an inference, not as a paper theorem.
6. **RNTA top-down determinization is not treated** (RNTA §1 related work: Van Heerdt et al. [41] do _bottom-up_, and "determinization produces orbit-infinite automata").
   Do not assume RNTA determinizes; the determinization capability is NDA-only.
7. **"Runtime monitoring" and "POSIX modeling" are gandr-side framings**, not claims from the papers.
   The papers own the models and decision procedures; the application mapping (§7) is design synthesis citing the gandr specs (`…/effects-control-shell.md`, `…/typing-machine.md`, VISION).
   Keep the provenance boundary clean in the plan.
8. **ADJACENT (not in the two papers, from the corpus).** The **alternation** paper (arXiv:2408.03658, Frank–Hausmann–Milius–Schröder–Urbat) gives _elementary_ non-emptiness + inclusion **even with unbounded registers** (EXPSPACE global / 2EXPSPACE local, per `wyrd-notes:archive/digest/ props-deepread-verified.md` l.89–90).
   If gandr wants fixpoint/temporal properties over names, that line — plus the **nominal μ-calculus** ([16] linear-time, [23] scalar/vectorial) both papers point to — is the next corpus to deep-read.
   Flagged, not chased.
9. **Scope posture updated.** ADR-46 (l.54) recorded the allocation automaton as unifying **only** the Σ-class, with CMTT-Ψ / macro-hygiene on their own machines; the owner's standing direction (PLAN review, `bd comments gandr-fcw.14` amendment 2) replaces that confinement with **opportunistic re-evaluation at every build-out stage**, explicitly open to generalizing the automata machinery over those sites.
   What the papers themselves establish is narrower and stands as evidence, not as a boundary: RNTA/NDA as published model _data languages / resource lifecycles_ and ship no apparatus for typed contextual metavariables or overlapping macro scopes — so any generalization is design work to be argued at the phase that attempts it, not a free import.
10. **Representation lock-step opportunity.** The papers' strong-nominal-set representation (`Σ 𝔸^{#Xᵢ}`, injective register stores) is the same shape as the ADR-41 `Atom`/`Gensym` + `BTreeSet<Atom>` support the corpus already commits to, and matches the Agda/Rocq `FinPerm` transposition rep (`wyrd-notes:archive/digest/props-successors-handoff.md` l.31, l.49) — a Rust↔Agda lock-step asset for the eventual metatheory.

---

## 10. Precise theorem catalogue (for direct citation)

### RNTA (`papers:LIPIcs.CONCUR.2024.35.pdf`)

* **Lemma 3.7** — `N`,`B` preserve & reflect inclusion: `L₁⊆L₂ ⟺ N(L₁)⊆N(L₂) ⟺ B(L₁)⊆B(L₂)`.
* **Def 4.1** — RNTA = `(Q orbit-finite nominal, Δ equivariant rewrite rules, q₀)` with α-invariance
  + finite branching up to α.
    Degree = max support cardinality of states.
* **Lemma 4.2 / Corollary 4.3** — support monotonicity; `q` accepts `t` ⇒ `FN(t) ⊆ supp(q)`.
* **Remark 4.8** — coalgebra for `F X = P_ufs(Σ_{f/n∈Σ}(𝔸×Xⁿ + [𝔸]Xⁿ))`.
* **Lemma 5.1** — every RNTA ≡ one with a strong nominal state set.
* **Theorem 5.5** — name-dropping modification `A_⊥` accepts the α-closure of `A`'s literal language (same alphatic language), same degree `d`, orbit count `≤ 2^d ×` that of `A`.
* **Remark 5.6** — lossiness: registers may nondeterministically lose letters.
* **Lemma 6.2** — `|S| = d_A·n_ar + 1` names suffice to witness every accepted α-class.
* **Theorem 6.3** — alphatic (= global = branchwise, via Lemma 3.7) inclusion decidable in **2-EXPTIME**, **parametrized singly-exponential in degree** (exp in a fn exp in `d_A+d_B`, poly in `|A|,|B|`).
  Floor: NFTA inclusion is **EXPTIME-complete** ([35] Seidl 1990).
* **Theorem 6.6** — **local-freshness** inclusion `D(L_α(A)) ⊆ D(L_α(B))`: same **2-EXPTIME / degree-parametrized singly-exponential** bound.

### NDA (`papers:2603.24468v1.pdf`)

* **Def 5.1** — NDA = `(Q orbit-finite nominal, Δ ⊆ Q×Â×Q equivariant, i, F)` with left α-invariance, name erasure, finite branching.
  Degree = max support cardinality.
* **Lemma 5.7** — support lemma: `a` moves need `a` in memory; `⟦a` adds, `a⟧`/`⟦a⟧` erase.
* **Proposition 5.8** — every accepted word is right non-shadowing.
* **Proposition 5.11** — under local freshness, **NDA ≡ RNNA** (equiexpressive).
* **Theorem 5.12** — NDA inclusion (local freshness) decidable in **EXPSPACE**, **parametrized PSPACE in the degree** (via RNNA, [21, Cor 7.4]).
* **Construction 6.3 / Theorem 6.9** — name-dropping modification closes the NDA language under α-equivalence.
* **Def 7.1** — regular deallocation expression (`⟦a | a⟧ | ⟦a⟧ | ? | · | + | *`, non-shadowing side-conditions).
* **Theorem 7.19 / 7.20** — regular deallocation expressions ≡ D-NFA (Kleene theorem); via Constr 7.8/7.11 ⇄ NDA.
* **Example 8.1** — RNNA is **not** determinizable.
* **Def 8.2** — DDA = deterministic deallocation automaton.
* **Theorem 8.14** — **every NDA is determinizable to a DDA preserving the data language under local freshness** (`L_D(A_DDA) = D(L_α(A))`).
* **Corollary 8.15** — an RNNA with empty-support initial state is determinizable **as an NDA** (data-language sense).

### Foundation & neighbours (references, for the bibliography)

* **RNNA** — Schröder, Kozen, Milius, Wißmann, _Nominal Automata with Name Binding_, FoSSaCS 2017, arXiv:1603.01455 (RNTA [34] / NDA [21]).
* **Session automata** — Bollig, Habermehl, Leucker, Monmege, _A robust class of data languages…_, LMCS 2014 (RNTA/NDA [6]).
* **NFTA inclusion EXPTIME-complete** — Seidl, _Deciding equivalence of finite tree automata_, SIAM J. Comput. 1990 (RNTA [35]).
* **Nominal register-tree automata** — Kaminski, Tan, _Tree automata over infinite alphabets_, 2008 (RNTA [21]).
* **Supported sets** (representation) — Wißmann, _Supported Sets — A New Foundation for Nominal Sets and Automata_, CSL 2023 / arXiv:2201.09825 (adopted by `…/0046-*.md` l.43).
* **Nominal unification** (unitary MGUs, the `is_unifiable` boundary) — Urban–Pitts–Gabbay, TCS 323, 2004 (cited in `crates/gandr-nominal/src/lib.rs` l.38).
* **Adjacent temporal/fixpoint** — Hausmann–Milius–Schröder, _Linear-time nominal μ-calculus with name allocation_, MFCS 2021 (NDA [16]); Klin–Łełyk, _Scalar and vectorial μ-calculus with atoms_ (RNTA [23]); alternation: arXiv:2408.03658 (corpus-cited, not deep-read here).
