//! The interned subsumption rows, driven by the shared free generators in
//! `gandr-core-checker-tools`.
//!
//! Only the generator-driven rows live here. The deterministic rows stay inline
//! beside `discipline::subtype`, where the relation is defined, and the
//! interner's own content-addressing rows stay inline in `gandr-core-term`.
//!
//! These rows are here for the reason `marking.rs` records: the generators sit
//! one tier above this crate, so an inline `cfg(test)` module could not unify
//! types with them.

use gandr_core_checker::discipline::subtype::comp_subtype;
use gandr_core_checker::discipline::subtype::interned_subtype;
use gandr_core_checker::discipline::subtype::value_subtype;
use gandr_core_checker_tools::strategies::arb_comp_type;
use gandr_core_checker_tools::strategies::arb_value_type;
use gandr_core_term::ctx::Ctx;
use gandr_core_term::intern::TypeInterner;
use gandr_core_term::types::Ty;
use proptest::prelude::*;

proptest! {
    /// The interned [`interned_subtype`] verdict equals the structural
    /// [`value_subtype`] on the same pair — the id short-circuit is a pure
    /// optimization, agreeing with structural descent on every pair. `lo`
    /// and `hi` are independent random types, so most pairs take the
    /// structural fallback (distinct ids) and a coincidentally-equal pair
    /// takes the O(1) id hit; either way the verdicts must agree.
    #[test]
    fn interned_subtype_agrees_with_structural_value(
        lo in arb_value_type(3_u32),
        hi in arb_value_type(3_u32),
    ) {
        let mut interner = TypeInterner::new();
        let lo_id = interner.intern(&Ty::Value(lo.clone()));
        let hi_id = interner.intern(&Ty::Value(hi.clone()));
        prop_assert_eq!(interned_subtype(&Ctx::new(), &interner, lo_id, hi_id), value_subtype(&Ctx::new(), &lo, &hi));
    }

    /// The computation-sort analogue of
    /// [`interned_subtype_agrees_with_structural_value`].
    #[test]
    fn interned_subtype_agrees_with_structural_comp(
        lo in arb_comp_type(3_u32),
        hi in arb_comp_type(3_u32),
    ) {
        let mut interner = TypeInterner::new();
        let lo_id = interner.intern(&Ty::Comp(lo.clone()));
        let hi_id = interner.intern(&Ty::Comp(hi.clone()));
        prop_assert_eq!(interned_subtype(&Ctx::new(), &interner, lo_id, hi_id), comp_subtype(&Ctx::new(), &lo, &hi));
    }
}
