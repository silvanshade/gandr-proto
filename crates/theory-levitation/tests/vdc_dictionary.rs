//! # The F0 VDC dictionary suite — verdict
//!
//! A property-test suite checking the virtual-double-category laws
//! (`docs/gandr/spec/proposal-vdc-reflection.md` §3; ADR-68 D2) on the **real
//! stage-0 structures** of `gandr-theory-levitation`, with a per-law verdict
//! recorded here.
//!
//! ## Real structure vs the F0 stand-in
//!
//! Objects ([`harness::SigObj`] over real
//! [`gandr_theory_levitation::DataDesc`]s), tight arrows (renamings of real
//! ctor / op symbols, checked against real [`gandr_theory_levitation::Code`] /
//! `BridgeArity`), and restriction (recomputing real
//! [`gandr_theory_levitation::CellFace`]s through the real
//! [`gandr_theory_levitation::wellformed::derive_cell_var_meta`] and validated
//! by the real [`gandr_theory_levitation::check_desc`]) are built **directly on
//! the landed crate**. The multi-ary **cell / derivation machinery**
//! ([`harness::Cell`], [`harness::replay`], [`harness::graft`]) is a deliberate
//! **test-side stand-in**: `gandr-polygraph` does not exist yet (L2). The
//! future `gandr-vdc` crate (F2) **inherits this harness split** — its
//! `Tracelet` replaces [`harness::CellKind`] and its replay checker replaces
//! [`harness::replay`], while the object / tight-arrow / restriction
//! realization carries over unchanged.
//!
//! ## Verdict table
//!
//! | Law | Verdict | Strict or up-to-replay | Located invariant / obligation | Witnessing tests |
//! | --- | --- | --- | --- | --- |
//! | **1 — tight category** | PASS | strict (structural) | none | `law1::*` (composition assoc/unit, contravariant action functoriality, product β + terminal uniqueness, diagonal, `check_morphism` accept/reject) |
//! | **2 — multicategorical cells** | PASS | up-to-replay (on the F0 stand-in L2's `Tracelet` inherits) | none; symbolic `graft` is partial (unital + linear single-clause chain) — other shapes decline and defer to replay-level composition | `law2::*` (grafting unital, associative, `replay(graft) = replay∘replay`, unsupported-shape decline) |
//! | **3 — restriction** | PASS | (a) strict on the real `CellFace`; (b) four Def 3.2.6 equalities strict **by construction** (Lemma 3.2.8's tuple form realized); (c) split fibrationality free from the representation; (d) strict, real-structure | none | `law3::*` (split face action, `α[id#id]`, `α[s#t][s′#t′]=α[s∘s′#t∘t′]`, `⊤[s#t]=⊤`, `(α∧β)[s#t]` distribution, globular factorization, variance invariance, well-formedness preservation, boundary reads) |
//! | **4 — cartesian** | PASS | β / pairing up-to-replay; local `⊤`/`∧` strict via 3(b) | none; the pairing-uniqueness spot-check is **bounded to `Pair`-shaped ρ** (recorded honestly) | `law4::*` (projection/pairing bijection, uniqueness spot-check, unique cell into `⊤`, products preserved by restriction) |
//! | **5 — units (path induction)** | **PARTIAL** | β holds (strict on `refl`); the unit universal property's **bijectivity is not total** | **`PathInd` declines on a non-empty path**: the base cell's shared middle variable requires the chain endpoints to meet, so a path between them cannot be absorbed by any generating instance. The obligation: **instances must be closed under path action — a saturation invariant on the future cell store (instances-as-modules over the rewrite-path relation).** The saturation probe shows this invariant is *sufficient*, not merely necessary. | `law5::*` (real path formation, β on `refl`, non-empty decline, saturation probe, distinctness) |
//!
//! Net: laws 1–4 PASS (1 and 3 strict / by construction; 2 and 4 up to
//! replay-equivalence, the F3 certificate-identity notion); law 5 is the
//! informative PARTIAL, and its decline **locates a precise cell-store
//! invariant** rather than a defect in the suite.
//!
//! ## Located obligations (verdict notes)
//!
//! * **H1** — `gandr-theory-levitation` ships **no production substitution /
//!   matching** on [`gandr_theory_levitation::FreeTerm`]; the first-order
//!   matcher, substitution, rewriter, and bounded path search are supplied
//!   test-side ([`harness`]). An F2 / L2 obligation. Witness:
//!   `law3::matching_and_substitution_are_supplied_test_side`.
//! * **H2** — [`gandr_theory_levitation::CellFace`] /
//!   [`gandr_theory_levitation::check_desc`] is **single-signature
//!   (homogeneous)**: heterogeneous loose arrows (`I ≠ J`) have no
//!   well-formedness support, and the `UnboundRhsVariable` rule (rhs vars ⊆ lhs
//!   vars) is a **rewrite discipline**, not the condition for a two-sided
//!   relation interface. Witnesses:
//!   `law3::check_desc_is_single_signature_homogeneous`,
//!   `law3::unbound_rhs_rule_is_a_rewrite_discipline_not_a_relation_condition`.
//! * **H3** — there is **no production signature-morphism type**; `SigMorphism`
//!   and its `check_morphism` checker live entirely in [`harness`]. Witness:
//!   `law1::check_morphism_accepts_a_role_matched_renaming` (and the rejection
//!   test).
//!
//! ## The split-form note (Law 3(b))
//!
//! All four Def 3.2.6 equalities hold **strictly by construction**, because the
//! [`harness::FormalRestriction`] representation (`base[left # right]`) never
//! touches the base under restriction — it only composes into the tight frames:
//! `α[id#id]=α` because composing with the identity tight arrow is the
//! identity; `α[s#t][s′#t′]=α[s∘s′#t∘t′]` because tight composition is
//! associative (Law 1); `⊤[s#t]=⊤` because the empty `∧`-tuple has no factors
//! to restrict; and `(α∧β)[s#t]=α[s#t]∧β[s#t]` because restriction maps over
//! the tuple. Law 3(a) carries the **real-structure content**: the face action
//! is split on the landed [`gandr_theory_levitation::CellFace`] type itself.

mod fixtures;
mod harness;
mod law1_tight;
mod law2_cells;
mod law3_restriction;
mod law4_cartesian;
mod law5_units;
