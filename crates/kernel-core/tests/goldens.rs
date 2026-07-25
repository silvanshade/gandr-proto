//! Kernel-native **C5 goldens** (gandr-wvd.2, B2.3 deliverable 2): the
//! levelled-universe and explicit-lift fixtures.
//!
//! These forms have **no core-checker counterpart** — core CBPV carries no
//! universe levels and no explicit lift terms (its `Universe` is the
//! un-levelled ADR-81 code universe, which the bridge rejects), so their
//! goldens are authored **directly in kernel-core terms** through the
//! arena-independent spec trees and the iterative materializer of [`common`],
//! never lowered through the B2.3 bridge.
//!
//! Each golden admits a universe/lift declaration through the choke point and
//! witnesses that it **round-trips byte-identically** through the K5 export
//! (`write ∘ read ∘ write` is a fixed point), exercising the level-signature,
//! universe-constant, universe-variable, and lift serialization the bridge-fed
//! corpus never reaches.

mod common;

/// The C5 lift/universe golden round-trips.
#[cfg(test)]
mod tests
{
    use gandr_kernel_core::Environment;
    use gandr_kernel_core::LevelParamCount;
    use gandr_kernel_core::LevelSignature;
    use gandr_kernel_core::read;
    use gandr_kernel_core::write;
    use gandr_kernel_strata::Level;
    use gandr_kernel_strata::LevelConstant;
    use gandr_kernel_strata::LevelVar;
    use gandr_kernel_strata::LevelVarIndex;

    use crate::common::CompTypeSpec;
    use crate::common::ValueSpec;
    use crate::common::ValueTypeSpec;
    use crate::common::stage_axiom;
    use crate::common::stage_def;

    /// The constant level `value`.
    fn constant(value: LevelConstant) -> Level
    {
        Level::constant(value)
    }

    /// The level of the prenex variable `index`.
    fn level_var(index: LevelVarIndex) -> Level
    {
        Level::var(LevelVar::new(index))
    }

    /// A monomorphic level signature (no prenex parameters).
    fn mono() -> LevelSignature
    {
        LevelSignature::monomorphic()
    }

    /// Assert `write ∘ read ∘ write` is byte-identical on `environment`.
    fn assert_round_trips(environment: &Environment)
    {
        let bytes = write(environment);
        let reread =
            read(bytes.as_ref().into()).expect("the C5 golden re-reads through the choke point");
        assert_eq!(
            bytes,
            write(&reread),
            "the C5 golden round-trips byte-identically"
        );
    }

    #[test]
    fn universe_constant_axiom_round_trips()
    {
        // Axiom : U_0. The universe classifies value types; at S1 no value
        // inhabits it, so the golden is an axiom over the universe type.
        let mut environment = Environment::new();
        let declaration = stage_axiom(
            &mut environment,
            mono(),
            &ValueTypeSpec::Universe(constant(0_u64.into())),
        );
        environment
            .add_decl(declaration)
            .expect("U_0 is a well-formed declared type");
        assert_round_trips(&environment);
    }

    #[test]
    fn universe_variable_axiom_round_trips()
    {
        // Over one prenex level parameter x₀, Axiom : U_{x₀} — a level-
        // polymorphic universe, exercising the level-signature and the
        // variable-atom serialization.
        let mut environment = Environment::new();
        let levels = LevelSignature::new(LevelParamCount::from(1_u32), Vec::new());
        let declaration = stage_axiom(
            &mut environment,
            levels,
            &ValueTypeSpec::Universe(level_var(0_u32.into())),
        );
        environment
            .add_decl(declaration)
            .expect("U_{x0} is well-formed under one prenex parameter");
        assert_round_trips(&environment);
    }

    #[test]
    fn lift_type_axiom_round_trips()
    {
        // Axiom : Lift Unit 1. Unit is at level 0, strictly below the target 1,
        // so the explicit lift is well-formed.
        let mut environment = Environment::new();
        let lifted = ValueTypeSpec::Lift(Box::new(ValueTypeSpec::Unit), constant(1_u64.into()));
        let declaration = stage_axiom(&mut environment, mono(), &lifted);
        environment
            .add_decl(declaration)
            .expect("Lift Unit 1 is a well-formed declared type (0 < 1)");
        assert_round_trips(&environment);
    }

    #[test]
    fn lift_value_definition_round_trips()
    {
        // Def (Lift Unit 1) = lift_1 unit. The explicit lift value inhabits the
        // lift type: `unit : Unit` (level 0) lifted to target 1.
        let mut environment = Environment::new();
        let declared = ValueTypeSpec::Lift(Box::new(ValueTypeSpec::Unit), constant(1_u64.into()));
        let body = ValueSpec::Lift(constant(1_u64.into()), Box::new(ValueSpec::Unit));
        let declaration = stage_def(&mut environment, mono(), &declared, &body);
        environment
            .add_decl(declaration)
            .expect("lift_1 unit inhabits Lift Unit 1");
        assert_round_trips(&environment);
    }

    #[test]
    fn universe_in_arrow_domain_round_trips()
    {
        // Axiom : U (U_0 → F Unit) — a universe nested as an arrow domain,
        // exercising the universe embedded inside a larger type former.
        let mut environment = Environment::new();
        let declared = ValueTypeSpec::Thunk(Box::new(CompTypeSpec::Arrow(
            Box::new(ValueTypeSpec::Universe(constant(0_u64.into()))),
            Box::new(CompTypeSpec::Returner(Box::new(ValueTypeSpec::Unit))),
        )));
        let declaration = stage_axiom(&mut environment, mono(), &declared);
        environment
            .add_decl(declaration)
            .expect("U (U_0 -> F Unit) is a well-formed declared type");
        assert_round_trips(&environment);
    }

    #[test]
    fn a_universe_and_lift_environment_round_trips()
    {
        // A whole environment of C5 goldens — an axiom stack of universes and
        // lifts plus a lift definition — round-trips byte-identically, so the
        // admission-ordered multi-declaration artifact is a fixed point too.
        let mut environment = Environment::new();
        let universe = stage_axiom(
            &mut environment,
            mono(),
            &ValueTypeSpec::Universe(constant(0_u64.into())),
        );
        environment.add_decl(universe).expect("U_0 admits");
        let lift_type = stage_axiom(
            &mut environment,
            mono(),
            &ValueTypeSpec::Lift(Box::new(ValueTypeSpec::Unit), constant(2_u64.into())),
        );
        environment.add_decl(lift_type).expect("Lift Unit 2 admits");
        let lift_value = stage_def(
            &mut environment,
            mono(),
            &ValueTypeSpec::Lift(Box::new(ValueTypeSpec::Unit), constant(1_u64.into())),
            &ValueSpec::Lift(constant(1_u64.into()), Box::new(ValueSpec::Unit)),
        );
        environment
            .add_decl(lift_value)
            .expect("lift_1 unit admits");
        assert_round_trips(&environment);
    }
}
