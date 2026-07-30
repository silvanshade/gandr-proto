//! **Bidirectional checking** of the reflected judgment layer over two-sided
//! contexts (`proposal-vdc-reflection.md` §5.2; ADR-68 D3).
//!
//! Checking is bidirectional over the `FVDblTT` proterm judgment shape
//! verbatim: `Γ # Δ ⊢ α protype` (protype well-formedness against a
//! **two-sided** object context) and `Φ ⊢ μ : β` (a proterm against a
//! seam-composable chain `Φ` of protype hypotheses). It is **first-order** — no
//! dependent types.
//!
//! # The soundness posture (§8; ADR-68 D4)
//!
//! The engine-fact rules are validated by **replay elaboration**, not trusted:
//! a [`Proterm::Cert`] checks against an engine-backed protype only when the
//! embedded derivation **replays** in the store
//! ([`crate::Derivation::replays`], ADR-69). The deeper metatheoretic claims —
//! that this checker is *complete* w.r.t. `CellStoreVdc` (the syntax–semantics
//! biadjunction) — are **not** made in Rust; they ride the Agda face. What is
//! realized here is a total,
//! structural, replay-anchored checker: it rejects refl off the diagonal,
//! non-replaying certificates, shape-mismatched eliminators, and terms over
//! undeclared object variables.

use alloc::boxed::Box;
use alloc::vec::Vec;

use gandr_theory_computads::CellId;
use gandr_theory_computads::CellStore;
use gandr_theory_levitation::Name;
use gandr_theory_levitation::NameRef;

use crate::boundary::CheckContextDeclaration;
use crate::boundary::DerivationIndex;
use crate::syntax::DerivationId;
use crate::syntax::ProVar;
use crate::syntax::Proterm;
use crate::syntax::Protype;
use crate::vdc::Derivation;
use crate::vdc::RelationRef;
use crate::vdc::SignatureRef;
use crate::vdc::TermRef;

/// The **two-sided object context** `Γ # Δ` — domain-side and codomain-side
/// object variables with their signatures (`proposal-vdc-reflection.md` §5.2).
///
/// A protype's framing terms range over the object variables declared here; the
/// two sides are the `FVDblTT` source/target contexts a protype is framed
/// between.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Context
{
    /// The domain-side (`Γ`) object variables.
    pub dom: Vec<(Name, SignatureRef)>,
    /// The codomain-side (`Δ`) object variables.
    pub cod: Vec<(Name, SignatureRef)>,
}

impl Context
{
    /// An empty two-sided context.
    #[inline]
    #[must_use]
    pub fn new() -> Self
    {
        Self::default()
    }

    /// Extend the domain side with an object variable.
    #[inline]
    #[must_use]
    pub fn with_dom(
        mut self,
        name: NameRef<'_>,
        sig: SignatureRef,
    ) -> Self
    {
        self.dom.push((Name::from(name), sig));
        self
    }

    /// Extend the codomain side with an object variable.
    #[inline]
    #[must_use]
    pub fn with_cod(
        mut self,
        name: NameRef<'_>,
        sig: SignatureRef,
    ) -> Self
    {
        self.cod.push((Name::from(name), sig));
        self
    }

    /// Whether `name` is declared on either side.
    ///
    /// # Contract
    /// - ensures: `true` iff `name` appears in `dom` or `cod`.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn declares(
        &self,
        name: NameRef<'_>,
    ) -> CheckContextDeclaration
    {
        for declaration in self.dom.iter().chain(self.cod.iter()) {
            if declaration.0.as_ref() == name.as_ref() {
                return CheckContextDeclaration::from(true);
            }
        }
        CheckContextDeclaration::from(false)
    }
}

/// Why a protype or proterm failed to check (`proposal-vdc-reflection.md`
/// §5.2).
///
/// Each variant is a distinct, testable rejection — the exact-variant oracle
/// the per-rule property tests assert against (ADR-71).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum CheckError
{
    /// A proterm variable is not bound in the hypothesis chain `Φ`.
    UnboundVar(ProVar),
    /// A [`Proterm::Cert`] references a derivation absent from the environment.
    UnboundDerivation(DerivationId),
    /// A term references an object variable not declared in the two-sided
    /// context.
    UndeclaredTermVar(Name),
    /// A relation protype (or tabulator) names a generating cell absent from
    /// the store — a dangling loose-arrow generator.
    UnknownRelationGenerator(CellId),
    /// `refl` was checked against a path whose endpoints differ, or whose
    /// signature disagrees with the `refl` term's.
    ReflOffDiagonal,
    /// A [`Proterm::Cert`] was checked against a protype that is not
    /// engine-backed (only [`Protype::Path`] / [`Protype::Rel`] are).
    NotEngineBacked,
    /// The embedded derivation of a [`Proterm::Cert`] does not replay (ADR-69).
    CertDoesNotReplay(DerivationId),
    /// A [`Protype::Compose`]'s seam signatures disagree (`tgt(l) != mid` or
    /// `src(r) != mid`).
    ComposeSeamMismatch,
    /// A synthesized or looked-up protype disagreed with the expected one.
    Mismatch
    {
        /// The protype the checker expected.
        expected: Box<Protype>,
        /// The protype it found.
        found: Box<Protype>,
    },
    /// A ⊙-introduction (`Pair`) was checked against a
    /// non-[`Protype::Compose`].
    ExpectedCompose,
    /// A ⊲/⊳-introduction (`Lam`) was checked against a non-extension protype.
    ExpectedExtension,
    /// A product introduction/projection met a non-[`Protype::Product`].
    ExpectedProduct,
    /// A path eliminator (`PathInd`) met a non-[`Protype::Path`] scrutinee.
    ExpectedPath,
    /// The unit proterm was checked against a non-[`Protype::Unit`] protype.
    ExpectedUnit,
    /// A seam eliminator (`SeamInd`) met a non-seam scrutinee.
    NotASeam,
    /// Type synthesis was requested for a proterm form that only checks.
    CannotSynthesize,
}

/// The **checker** — an environment of embedded derivations and the cell store
/// they replay against (`proposal-vdc-reflection.md` §5.2).
#[derive(Clone, Copy, Debug)]
pub struct Checker<'env>
{
    /// The embedded engine derivations, indexed by [`DerivationId`].
    pub derivations: &'env [Derivation],
    /// The cell store the certificates replay against.
    pub cells: &'env CellStore,
}

/// A hypothesis chain `Φ` — a sequence of `(variable, protype)` bindings.
type Hyps = [(ProVar, Protype)];

/// One protype-walk work-stack entry (ADR-47): a protype to check, or a
/// `Compose` whose halves have been checked and whose shared middle signature
/// must now join.
enum ProtypeFrame<'protype>
{
    /// Check a protype's structure.
    Enter(&'protype Protype),
    /// Join a checked `Compose`: require the left half's target signature and
    /// the right half's source signature to both equal the middle.
    FinishCompose
    {
        /// The left half (already checked).
        left: &'protype Protype,
        /// The shared middle signature.
        mid: &'protype SignatureRef,
        /// The right half (already checked).
        right: &'protype Protype,
    },
}

/// One bidirectional-checking work-stack entry (ADR-47): a judgment to
/// discharge, or a result-stack discipline to enforce once a synthesis lands.
enum Judgment<'term>
{
    /// Check `term` against `expected` under `hyps` (the checking mode).
    Check
    {
        /// The hypothesis chain in scope.
        hyps: Vec<(ProVar, Protype)>,
        /// The term being checked.
        term: &'term Proterm,
        /// The protype it must equal.
        expected: Protype,
    },
    /// Synthesize `term`'s protype under `hyps`, pushing it onto the result
    /// stack (the synthesis mode).
    Synth
    {
        /// The hypothesis chain in scope.
        hyps: Vec<(ProVar, Protype)>,
        /// The term whose protype is synthesized.
        term: &'term Proterm,
    },
    /// Pop a synthesized protype and require it to equal `expected` (the
    /// check-via-synth reduction for applications and projections).
    ExpectSynth
    {
        /// The protype the synthesized one must equal.
        expected: Protype,
    },
    /// Pop a synthesized protype and require it to be a `Path` (the path
    /// induction scrutinee discipline).
    ExpectPath,
    /// Pop a synthesized protype and require it to be a `Compose` (the seam
    /// induction scrutinee discipline).
    ExpectCompose,
    /// Pop a synthesized `Product` protype and push its left factor.
    ProjectLeft,
    /// Pop a synthesized `Product` protype and push its right factor.
    ProjectRight,
    /// Pop a synthesized extension (`ExtendL` / `ExtendR`) protype, then check
    /// the argument against its domain and finish with its codomain.
    Apply
    {
        /// The hypothesis chain in scope.
        hyps: Vec<(ProVar, Protype)>,
        /// The application argument.
        arg: &'term Proterm,
    },
    /// Push an application's codomain once its argument has checked.
    FinishApp
    {
        /// The codomain protype.
        cod: Protype,
    },
}

impl<'env> Checker<'env>
{
    /// A checker over a derivation environment and a cell store.
    #[inline]
    #[must_use]
    pub fn new(
        derivations: &'env [Derivation],
        cells: &'env CellStore,
    ) -> Self
    {
        Self { derivations, cells }
    }

    /// Check that a **protype** is well-formed in the two-sided context `ctx`.
    ///
    /// # Contract
    /// - ensures: `Ok(())` iff every framing term's free variables are declared
    ///   in `ctx`, every [`Protype::Compose`] seam agrees (`tgt(l) == mid ==
    ///   src(r)`), and every sub-protype is well-formed.
    /// - fails: [`CheckError::UndeclaredTermVar`] for a term over an undeclared
    ///   object variable; [`CheckError::ComposeSeamMismatch`] for a disagreeing
    ///   seam.
    /// - panics: none.
    ///
    /// # Errors
    /// See the `- fails:` clause.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — the seam law and the declared-variable guard are
    ///   separated by boundary inputs (a seam whose `mid` mismatches; a term
    ///   over an undeclared variable) each asserting the exact [`CheckError`]
    ///   variant.
    /// - witness: `laws::tests::compose_protype_requires_seam_agreement`
    /// - witness: `laws::tests::a_term_over_an_undeclared_object_variable_is_rejected`
    #[inline]
    pub fn check_protype(
        &self,
        ctx: &Context,
        protype: &Protype,
    ) -> Result<(), CheckError>
    {
        let mut frames = alloc::vec![ProtypeFrame::Enter(protype)];
        while let Some(frame) = frames.pop() {
            match frame {
                | ProtypeFrame::Enter(current) => match *current {
                    | Protype::Path {
                        ref lhs, ref rhs, ..
                    } => {
                        term_vars_declared(ctx, lhs)?;
                        term_vars_declared(ctx, rhs)?;
                    },
                    | Protype::Rel {
                        ref rel,
                        ref lhs,
                        ref rhs,
                    } => {
                        term_vars_declared(ctx, lhs)?;
                        term_vars_declared(ctx, rhs)?;
                        self.relation_generators_present(rel)?;
                    },
                    | Protype::Compose {
                        ref l,
                        ref mid,
                        ref r,
                    } => {
                        frames.push(ProtypeFrame::FinishCompose {
                            left: l,
                            mid,
                            right: r,
                        });
                        frames.push(ProtypeFrame::Enter(r));
                        frames.push(ProtypeFrame::Enter(l));
                    },
                    | Protype::ExtendR { ref dom, ref cod }
                    | Protype::ExtendL { ref cod, ref dom } => {
                        frames.push(ProtypeFrame::Enter(cod));
                        frames.push(ProtypeFrame::Enter(dom));
                    },
                    | Protype::Product(ref a, ref b) => {
                        frames.push(ProtypeFrame::Enter(b));
                        frames.push(ProtypeFrame::Enter(a));
                    },
                    | Protype::Tabulate { ref rel } => {
                        self.relation_generators_present(rel)?;
                    },
                    | Protype::Unit => {},
                },
                | ProtypeFrame::FinishCompose { left, mid, right } => {
                    if protype_tgt_sig(left).as_ref() != Some(mid)
                        || protype_src_sig(right).as_ref() != Some(mid)
                    {
                        return Err(CheckError::ComposeSeamMismatch);
                    }
                },
            }
        }
        Ok(())
    }

    /// Check that every generating cell of a relation is present in the store.
    ///
    /// # Contract
    /// - ensures: `Ok(())` iff each generator [`CellId`] of `rel` resolves in
    ///   the store.
    /// - fails: [`CheckError::UnknownRelationGenerator`] naming the first
    ///   dangling generator.
    /// - panics: none.
    #[inline]
    fn relation_generators_present(
        &self,
        rel: &RelationRef,
    ) -> Result<(), CheckError>
    {
        for &id in &rel.generators {
            if self.cells.get(id).is_none() {
                return Err(CheckError::UnknownRelationGenerator(id));
            }
        }
        Ok(())
    }

    /// Check a **proterm** against an expected protype (the checking direction
    /// of the bidirectional judgment `Φ ⊢ μ : β`).
    ///
    /// # Contract
    /// - ensures: `Ok(())` iff `term` inhabits `expected` under `ctx` and
    ///   `hyps` by the constructor menu's rules — a hypothesis variable whose
    ///   protype is `expected`; `refl` only on the diagonal path; a `Cert` only
    ///   against an engine-backed protype whose derivation **replays**; and
    ///   each introducer/eliminator only against its matching protype former,
    ///   with the immediate sub-proterms checked against the derived
    ///   sub-protypes.
    /// - fails: a [`CheckError`] naming the exact rejection.
    /// - panics: none.
    ///
    /// # Errors
    /// See [`CheckError`].
    ///
    /// # Adequacy
    /// - hypothesis: L1 evidence for the `Cert` rule (the replay validator
    ///   kills any mutant that admits a non-replaying certificate) plus L3
    ///   pointwise for the shape rules (each eliminator/introducer met by a
    ///   mismatched protype asserts its exact [`CheckError`]).
    /// - witness: `laws::tests::a_replaying_certificate_checks_against_its_relation`
    /// - witness: `laws::tests::a_non_replaying_certificate_is_rejected`
    /// - witness: `laws::tests::refl_off_the_diagonal_is_rejected`
    /// - witness: `laws::tests::a_pair_checks_against_a_seam_composite`
    #[inline]
    pub fn check(
        &self,
        ctx: &Context,
        hyps: &Hyps,
        term: &Proterm,
        expected: &Protype,
    ) -> Result<(), CheckError>
    {
        self.run_judgment(ctx, Judgment::Check {
            hyps: hyps.to_vec(),
            term,
            expected: expected.clone(),
        })
        .map(|_| ())
    }

    /// Synthesize the protype of a proterm (the inference direction); only the
    /// forms that carry enough structure synthesize.
    ///
    /// # Contract
    /// - ensures: the protype of a hypothesis variable, a `refl` (the diagonal
    ///   path), a product projection (from the synthesized product), or an
    ///   extension application (the extension's codomain).
    /// - fails: [`CheckError::CannotSynthesize`] for a checking-only form; the
    ///   relevant shape error for a mis-shaped subterm.
    /// - panics: none.
    ///
    /// # Errors
    /// See [`CheckError`].
    #[inline]
    pub fn synth(
        &self,
        ctx: &Context,
        hyps: &Hyps,
        term: &Proterm,
    ) -> Result<Protype, CheckError>
    {
        let synthesized = self.run_judgment(ctx, Judgment::Synth {
            hyps: hyps.to_vec(),
            term,
        })?;
        synthesized.ok_or(CheckError::CannotSynthesize)
    }

    /// Drives the bidirectional judgment work stack to completion, returning
    /// the protype the root judgment synthesized (if any).
    ///
    /// # Contract
    /// - requires: `initial` is the root judgment.
    /// - ensures: `Ok(Some(p))` when the root synthesized `p`; `Ok(None)` for a
    ///   checking-mode root; `Err` on the first failing rule.
    /// - fails: the relevant [`CheckError`] of the first violated rule.
    /// - panics: none.
    fn run_judgment(
        &self,
        _ctx: &Context,
        initial: Judgment<'_>,
    ) -> Result<Option<Protype>, CheckError>
    {
        let mut work = alloc::vec![initial];
        let mut synthesized = Vec::new();
        while let Some(judgment) = work.pop() {
            match judgment {
                | Judgment::Check {
                    hyps,
                    term,
                    expected,
                } => match *term {
                    | Proterm::Var(ref v) => {
                        let found = lookup_hyp(&hyps, v)
                            .ok_or_else(|| CheckError::UnboundVar(v.clone()))?;
                        expect_equal(&expected, found)?;
                    },
                    | Proterm::Refl {
                        ref sig,
                        term: ref refl_term,
                    } => check_refl(sig, refl_term, &expected)?,
                    | Proterm::Cert(id) => self.check_cert(id, &expected)?,
                    | Proterm::PathInd {
                        ref motive,
                        ref base,
                        ref scrut,
                    } => {
                        expect_equal(&expected, motive)?;
                        work.push(Judgment::Check {
                            hyps: hyps.clone(),
                            term: base,
                            expected: (**motive).clone(),
                        });
                        work.push(Judgment::ExpectPath);
                        work.push(Judgment::Synth { hyps, term: scrut });
                    },
                    | Proterm::Pair { ref l, ref r, .. } => match expected {
                        | Protype::Compose { l: pl, r: pr, .. } => {
                            work.push(Judgment::Check {
                                hyps: hyps.clone(),
                                term: r,
                                expected: *pr,
                            });
                            work.push(Judgment::Check {
                                hyps,
                                term: l,
                                expected: *pl,
                            });
                        },
                        | _ => return Err(CheckError::ExpectedCompose),
                    },
                    | Proterm::SeamInd { ref scrut, ref arm } => {
                        work.push(Judgment::Check {
                            hyps: hyps.clone(),
                            term: arm,
                            expected,
                        });
                        work.push(Judgment::ExpectCompose);
                        work.push(Judgment::Synth { hyps, term: scrut });
                    },
                    | Proterm::Lam { ref hyp, ref body } => {
                        let (Protype::ExtendR { dom, cod } | Protype::ExtendL { cod, dom }) =
                            expected
                        else {
                            return Err(CheckError::ExpectedExtension);
                        };
                        let mut extended = hyps;
                        extended.push((hyp.clone(), *dom));
                        work.push(Judgment::Check {
                            hyps: extended,
                            term: body,
                            expected: *cod,
                        });
                    },
                    | Proterm::App { .. } | Proterm::ProjL(_) | Proterm::ProjR(_) => {
                        work.push(Judgment::ExpectSynth { expected });
                        work.push(Judgment::Synth { hyps, term });
                    },
                    | Proterm::ProdIntro { ref l, ref r } => match expected {
                        | Protype::Product(a, b) => {
                            work.push(Judgment::Check {
                                hyps: hyps.clone(),
                                term: r,
                                expected: *b,
                            });
                            work.push(Judgment::Check {
                                hyps,
                                term: l,
                                expected: *a,
                            });
                        },
                        | _ => return Err(CheckError::ExpectedProduct),
                    },
                    | Proterm::UnitTerm => match expected {
                        | Protype::Unit => {},
                        | _ => return Err(CheckError::ExpectedUnit),
                    },
                },
                | Judgment::Synth { hyps, term } => match *term {
                    | Proterm::Var(ref v) => {
                        let found = lookup_hyp(&hyps, v)
                            .ok_or_else(|| CheckError::UnboundVar(v.clone()))?;
                        synthesized.push(found.clone());
                    },
                    | Proterm::Refl {
                        ref sig,
                        term: ref refl_term,
                    } => synthesized.push(Protype::Path {
                        sig: sig.clone(),
                        lhs: refl_term.clone(),
                        rhs: refl_term.clone(),
                    }),
                    | Proterm::ProjL(ref p) => {
                        work.push(Judgment::ProjectLeft);
                        work.push(Judgment::Synth { hyps, term: p });
                    },
                    | Proterm::ProjR(ref p) => {
                        work.push(Judgment::ProjectRight);
                        work.push(Judgment::Synth { hyps, term: p });
                    },
                    | Proterm::App { ref f, ref arg } => {
                        work.push(Judgment::Apply {
                            hyps: hyps.clone(),
                            arg,
                        });
                        work.push(Judgment::Synth { hyps, term: f });
                    },
                    | _ => return Err(CheckError::CannotSynthesize),
                },
                | Judgment::ExpectSynth { expected } => {
                    let found = synthesized.pop().ok_or(CheckError::CannotSynthesize)?;
                    expect_equal(&expected, &found)?;
                },
                | Judgment::ExpectPath => {
                    let found = synthesized.pop().ok_or(CheckError::CannotSynthesize)?;
                    match found {
                        | Protype::Path { .. } => {},
                        | _ => return Err(CheckError::ExpectedPath),
                    }
                },
                | Judgment::ExpectCompose => {
                    let found = synthesized.pop().ok_or(CheckError::CannotSynthesize)?;
                    match found {
                        | Protype::Compose { .. } => {},
                        | _ => return Err(CheckError::NotASeam),
                    }
                },
                | Judgment::ProjectLeft => {
                    let found = synthesized.pop().ok_or(CheckError::CannotSynthesize)?;
                    match found {
                        | Protype::Product(a, _) => synthesized.push(*a),
                        | _ => return Err(CheckError::ExpectedProduct),
                    }
                },
                | Judgment::ProjectRight => {
                    let found = synthesized.pop().ok_or(CheckError::CannotSynthesize)?;
                    match found {
                        | Protype::Product(_, b) => synthesized.push(*b),
                        | _ => return Err(CheckError::ExpectedProduct),
                    }
                },
                | Judgment::Apply { hyps, arg } => {
                    let found = synthesized.pop().ok_or(CheckError::CannotSynthesize)?;
                    match found {
                        | Protype::ExtendR { dom, cod } | Protype::ExtendL { cod, dom } => {
                            work.push(Judgment::FinishApp { cod: *cod });
                            work.push(Judgment::Check {
                                hyps,
                                term: arg,
                                expected: *dom,
                            });
                        },
                        | _ => return Err(CheckError::ExpectedExtension),
                    }
                },
                | Judgment::FinishApp { cod } => synthesized.push(cod),
            }
        }
        Ok(synthesized.pop())
    }

    /// The `Cert` checking rule — engine-backed protype plus replay (ADR-69).
    ///
    /// # Contract
    /// - ensures: `Ok(())` iff `expected` is engine-backed ([`Protype::Path`]
    ///   or [`Protype::Rel`]) and the embedded derivation replays in the store.
    /// - fails: [`CheckError::UnboundDerivation`] for an absent id;
    ///   [`CheckError::NotEngineBacked`] for a non-engine protype;
    ///   [`CheckError::CertDoesNotReplay`] for a certificate that fails replay.
    /// - panics: none.
    #[inline]
    fn check_cert(
        &self,
        id: DerivationId,
        expected: &Protype,
    ) -> Result<(), CheckError>
    {
        let index = usize::from(DerivationIndex::from(id));
        let derivation = self
            .derivations
            .get(index)
            .ok_or(CheckError::UnboundDerivation(id))?;
        match *expected {
            | Protype::Path { .. } | Protype::Rel { .. } => {},
            | _ => return Err(CheckError::NotEngineBacked),
        }
        if bool::from(derivation.replays(self.cells)) {
            Ok(())
        }
        else {
            Err(CheckError::CertDoesNotReplay(id))
        }
    }
}

/// The `refl` checking rule — inhabits only the diagonal path.
///
/// # Contract
/// - ensures: `Ok(())` iff `expected` is a [`Protype::Path`] whose signature
///   equals `sig` and whose endpoints both equal `term`.
/// - fails: [`CheckError::ReflOffDiagonal`] otherwise.
/// - panics: none.
#[inline]
fn check_refl(
    sig: &SignatureRef,
    term: &TermRef,
    expected: &Protype,
) -> Result<(), CheckError>
{
    match *expected {
        | Protype::Path {
            sig: ref esig,
            ref lhs,
            ref rhs,
        } if esig == sig && lhs == term && rhs == term => Ok(()),
        | _ => Err(CheckError::ReflOffDiagonal),
    }
}

/// Check that every free variable of a term is declared in the two-sided
/// context.
///
/// # Contract
/// - ensures: `Ok(())` iff each free-term variable leaf of `term` is declared
///   in `ctx`.
/// - fails: [`CheckError::UndeclaredTermVar`] naming the first undeclared
///   variable.
/// - panics: none.
#[inline]
fn term_vars_declared(
    ctx: &Context,
    term: &TermRef,
) -> Result<(), CheckError>
{
    for var in term.term().collect_vars() {
        if !bool::from(ctx.declares(NameRef::from(var.as_ref()))) {
            return Err(CheckError::UndeclaredTermVar(var));
        }
    }
    Ok(())
}

/// Look a proterm variable up in the hypothesis chain.
///
/// # Contract
/// - ensures: the protype bound to the **last** matching occurrence of `v`
///   (innermost binding), or `None` when `v` is unbound.
/// - panics: none.
#[inline]
fn lookup_hyp<'hyps>(
    hyps: &'hyps Hyps,
    v: &ProVar,
) -> Option<&'hyps Protype>
{
    for hypothesis in hyps.iter().rev() {
        if hypothesis.0 == *v {
            return Some(&hypothesis.1);
        }
    }
    None
}

/// Require two protypes to be equal, reporting a [`CheckError::Mismatch`]
/// otherwise.
///
/// # Contract
/// - ensures: `Ok(())` iff `expected == found`.
/// - fails: [`CheckError::Mismatch`] carrying both protypes.
/// - panics: none.
#[inline]
fn expect_equal(
    expected: &Protype,
    found: &Protype,
) -> Result<(), CheckError>
{
    if expected == found {
        Ok(())
    }
    else {
        Err(CheckError::Mismatch {
            expected: Box::new(expected.clone()),
            found: Box::new(found.clone()),
        })
    }
}

/// The source signature of a protype, when it has one (for the seam law).
///
/// # Contract
/// - ensures: the framing source signature for a [`Protype::Path`] /
///   [`Protype::Rel`] / [`Protype::Compose`]; `None` for the forms without a
///   single first-order source signature.
/// - panics: none.
#[inline]
fn protype_src_sig(mut protype: &Protype) -> Option<SignatureRef>
{
    loop {
        match *protype {
            | Protype::Path { ref sig, .. } => return Some(sig.clone()),
            | Protype::Rel { ref rel, .. } => return Some(rel.src.clone()),
            | Protype::Compose { ref l, .. } => protype = l,
            | Protype::ExtendR { .. }
            | Protype::ExtendL { .. }
            | Protype::Product(..)
            | Protype::Tabulate { .. }
            | Protype::Unit => return None,
        }
    }
}

/// The target signature of a protype, when it has one (for the seam law).
///
/// # Contract
/// - ensures: the framing target signature for a [`Protype::Path`] /
///   [`Protype::Rel`] / [`Protype::Compose`]; `None` otherwise.
/// - panics: none.
#[inline]
fn protype_tgt_sig(mut protype: &Protype) -> Option<SignatureRef>
{
    loop {
        match *protype {
            | Protype::Path { ref sig, .. } => return Some(sig.clone()),
            | Protype::Rel { ref rel, .. } => return Some(rel.tgt.clone()),
            | Protype::Compose { ref r, .. } => protype = r,
            | Protype::ExtendR { .. }
            | Protype::ExtendL { .. }
            | Protype::Product(..)
            | Protype::Tabulate { .. }
            | Protype::Unit => return None,
        }
    }
}
