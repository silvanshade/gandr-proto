//! The type interner's property rows, driven by the shared free generators in
//! `gandr-core-checker-tools`.
//!
//! Only the generator-driven rows live here. The interner's unit tests stay
//! inline beside `intern.rs`, because one of them constructs an id past the
//! minted range — a state no public caller can reach, so it is not an
//! integration-level observation.
//!
//! These rows are here for the reason `marking.rs` records: the generators sit
//! one tier above this crate, so an inline `cfg(test)` module could not unify
//! types with them.

use gandr_core_checker::intern::TypeInterner;
use gandr_core_checker::subtype::comp_subtype;
use gandr_core_checker::subtype::value_subtype;
use gandr_core_checker::types::Ty;
use gandr_core_checker_tools::strategies::arb_comp_type;
use gandr_core_checker_tools::strategies::arb_value_type;
use proptest::prelude::*;

proptest! {
    /// The interned [`TypeInterner::subtype`] verdict equals the structural
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
        prop_assert_eq!(interner.subtype(lo_id, hi_id), value_subtype(&lo, &hi));
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
        prop_assert_eq!(interner.subtype(lo_id, hi_id), comp_subtype(&lo, &hi));
    }
}
