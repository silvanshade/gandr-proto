//! **Type formation**: the judgement that says what classifier a type is
//! formed at, and refuses when it is formed at none.
//!
//! Every value type and every computation type has a rule here, and the rule
//! answers with a `Classifier` — a sort and a level — or with a named
//! `FormationError`. Nothing falls through, and nothing guesses: a former
//! outside the admitted fragment is a named refusal, not a silent
//! `ValueType::Unknown`, which is the gradual hole an author wrote and has a
//! rule of its own.
//!
//! # Why it is one interface
//!
//! Before this module, a caller that needed a type's level inferred a sort
//! from an enum name and then guessed a level beside it. That is two
//! judgements in the caller's head and neither of them is checked. Formation
//! is where both are decided, once, so universe lifting, level joins, family
//! telescope checking, and cumulativity are one implementation rather than a
//! convention.
//!
//! # The one level algebra
//!
//! Every successor and every join goes through `gandr_kernel_strata::Level`.
//! There is no arithmetic on levels in this module and there will not be: the
//! rule that decides a level and the oracle that orders one are the same code,
//! which is what makes a formation answer checkable against the kernel's.

pub mod context;
pub mod rules;

pub use context::FamilySignature;
pub use context::FormationContext;
pub use rules::FormType;
