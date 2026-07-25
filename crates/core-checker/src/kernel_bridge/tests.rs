//! Unit witnesses for the kernel bridge.
//!
//! Successful lowering of the S1 stock, name resolution (de Bruijn and
//! cross-declaration constant), grade-op and annotation erasure, exact-variant
//! structural rejection of one representative per exclusion class, and the
//! value-polarity declaration round-trips.

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;

use gandr_kernel_core::ConstantIndex;
use gandr_kernel_core::Environment;
use gandr_kernel_core::LevelSignature;
use gandr_kernel_core::read;
use gandr_kernel_core::write;

use super::BridgeContext;
use super::BridgeRejection;
use super::lower_comp;
use super::lower_comp_type;
use super::lower_computation_definition;
use super::lower_value;
use super::lower_value_definition;
use super::lower_value_type;
use crate::effect::EffectSig;
use crate::prim::NativePrim;
use crate::syntax::Comp;
use crate::syntax::Value;
use crate::types::CompType;
use crate::types::DataId;
use crate::types::ValueType;

/// An empty naming environment (the closed-program case).
fn closed() -> BridgeContext
{
    BridgeContext::new()
}

/// Assert that `write ∘ read ∘ write` on the environment is byte-identical (the
/// export round-trip through the choke point).
fn assert_round_trips(environment: &Environment)
{
    let bytes = write(environment);
    let reread = read(&bytes).expect("a bridged environment re-reads through the choke point");
    let rebytes = write(&reread);
    assert_eq!(
        bytes, rebytes,
        "the bridged declaration round-trips byte-identically"
    );
}

// ----- Successful lowering of the S1 stock -----

#[test]
fn pair_of_literals_lowers()
{
    let mut arena = gandr_kernel_core::TermArena::new();
    let value = Value::pair(Value::int(1), Value::string("hi"));
    assert!(
        lower_value(&closed(), &mut arena, &value).is_ok(),
        "a pair of an integer and a string literal lowers into the S1 stock"
    );
}

#[test]
fn returning_a_literal_lowers()
{
    let mut arena = gandr_kernel_core::TermArena::new();
    let comp = Comp::Ret(Rc::new(Value::int(7)));
    assert!(
        lower_comp(&closed(), &mut arena, &comp).is_ok(),
        "returning an integer literal lowers into the S1 stock"
    );
}

#[test]
fn returner_type_lowers()
{
    let mut arena = gandr_kernel_core::TermArena::new();
    let comp_type = CompType::returner(ValueType::integer());
    assert!(
        lower_comp_type(&mut arena, &comp_type).is_ok(),
        "the pure returner F Integer lowers"
    );
}

#[test]
fn product_type_lowers()
{
    let mut arena = gandr_kernel_core::TermArena::new();
    let value_type = ValueType::prod(ValueType::integer(), ValueType::string());
    assert!(
        lower_value_type(&mut arena, &value_type).is_ok(),
        "the product Integer × String lowers"
    );
}

// ----- Erasure (C4) -----

#[test]
fn annotation_is_erased()
{
    // `(1 : Integer)` lowers (the ascription is peeled, operationally
    // transparent), and an annotation does not shield an out-of-S1 interior:
    // `(?hole : Integer)` still rejects on the hole.
    let mut arena = gandr_kernel_core::TermArena::new();
    let annotated = Value::annot(Value::int(1), ValueType::integer());
    assert!(
        lower_value(&closed(), &mut arena, &annotated).is_ok(),
        "a type ascription is erased; `(1 : Integer)` lowers as the bare literal"
    );
    let shielding = Value::annot(Value::Hole(0), ValueType::integer());
    assert_eq!(
        lower_value(&closed(), &mut arena, &shielding),
        Err(BridgeRejection::ValueHole),
        "erasing the ascription still lowers its interior, so an inner hole rejects"
    );
}

#[test]
fn drop_erases_to_return_unit()
{
    // `drop (thunk (ret unit))` lowers (the thunk body is validated, the drop
    // itself erases to `return ()`).
    let mut arena = gandr_kernel_core::TermArena::new();
    let thunk = Value::thunk(crate::grade::Grade::OMEGA, Comp::Ret(Rc::new(Value::Unit)));
    let dropped = Comp::Drop(Rc::new(thunk));
    assert!(
        lower_comp(&closed(), &mut arena, &dropped).is_ok(),
        "drop erases to return ()"
    );
}

#[test]
fn dup_erases_to_return_pair()
{
    let mut arena = gandr_kernel_core::TermArena::new();
    let thunk = Value::thunk(crate::grade::Grade::OMEGA, Comp::Ret(Rc::new(Value::Unit)));
    let duplicated = Comp::Dup(Rc::new(thunk));
    assert!(
        lower_comp(&closed(), &mut arena, &duplicated).is_ok(),
        "dup erases to return (v, v)"
    );
}

// ----- Name resolution -----

#[test]
fn a_bound_variable_resolves_to_a_de_bruijn_index()
{
    // `λx. ret x` — the body's `x` resolves to de Bruijn index 0.
    let mut arena = gandr_kernel_core::TermArena::new();
    let lambda = Comp::Abs(
        "x".to_owned(),
        None,
        Rc::new(Comp::Ret(Rc::new(Value::var("x")))),
    );
    assert!(
        lower_comp(&closed(), &mut arena, &lambda).is_ok(),
        "a lambda-bound variable resolves to a de Bruijn index"
    );
}

#[test]
fn a_free_name_is_unbound()
{
    let mut arena = gandr_kernel_core::TermArena::new();
    let free = Value::var("greeting");
    assert_eq!(
        lower_value(&closed(), &mut arena, &free),
        Err(BridgeRejection::UnboundName(String::from("greeting"))),
        "a free name with no binder and no constant mapping is unbound"
    );
}

#[test]
fn a_free_name_resolves_to_a_constant_when_mapped()
{
    // With `first ↦ admission 0`, a free `first` lowers to a constant reference.
    let mut arena = gandr_kernel_core::TermArena::new();
    let context = BridgeContext::new().with_constant("first", ConstantIndex::from(0_usize));
    let reference = Value::var("first");
    assert!(
        lower_value(&context, &mut arena, &reference).is_ok(),
        "a mapped free name lowers to a Value::Constant admission reference"
    );
}

// ----- Value-polarity declarations + round-trip -----

#[test]
fn a_lowered_value_definition_admits()
{
    let value = Value::pair(Value::int(1), Value::int(2));
    let declared = ValueType::prod(ValueType::integer(), ValueType::integer());
    let mut environment = Environment::new();
    let mut builder = environment.stage();
    let (declared_id, body_id) =
        lower_value_definition(&closed(), builder.arena(), &value, &declared).expect("lowers");
    let declaration = builder.def(LevelSignature::monomorphic(), declared_id, body_id);
    assert!(
        environment.add_decl(declaration).is_ok(),
        "a lowered closed value definition re-admits through the choke point"
    );
    assert_round_trips(&environment);
}

#[test]
fn a_lowered_computation_definition_admits()
{
    // `ret "hello"` : F String enters as the value declaration U (F String)
    // with body `thunk (ret "hello")`.
    let comp = Comp::Ret(Rc::new(Value::string("hello")));
    let declared = CompType::returner(ValueType::string());
    let mut environment = Environment::new();
    let mut builder = environment.stage();
    let (declared_id, body_id) =
        lower_computation_definition(&closed(), builder.arena(), &comp, &declared).expect("lowers");
    let declaration = builder.def(LevelSignature::monomorphic(), declared_id, body_id);
    assert!(
        environment.add_decl(declaration).is_ok(),
        "a lowered computation definition enters as a thunk and admits"
    );
    assert_round_trips(&environment);
}

#[test]
fn a_constant_reference_across_declarations_admits()
{
    // First admit `Def Unit = unit`; then `Def Unit = <ref to the first>`.
    let mut environment = Environment::new();
    let mut builder = environment.stage();
    let (declared_id, body_id) =
        lower_value_definition(&closed(), builder.arena(), &Value::Unit, &ValueType::Unit)
            .expect("the first definition lowers");
    let first = builder.def(LevelSignature::monomorphic(), declared_id, body_id);
    let first = environment.add_decl(first).expect("the first admits");

    let context = BridgeContext::new().with_constant("first", first.position());
    let reference = Value::var("first");
    let mut builder = environment.stage();
    let (declared_id, body_id) =
        lower_value_definition(&context, builder.arena(), &reference, &ValueType::Unit)
            .expect("the reference lowers to a constant");
    let second = builder.def(LevelSignature::monomorphic(), declared_id, body_id);
    assert!(
        environment.add_decl(second).is_ok(),
        "a definition referencing a prior one by constant re-admits"
    );
    assert_round_trips(&environment);
}

// ----- Exact-variant structural rejection, one per exclusion class -----

#[test]
fn hole_unknown_class_rejects_exactly()
{
    let mut arena = gandr_kernel_core::TermArena::new();
    assert_eq!(
        lower_value(&closed(), &mut arena, &Value::Hole(0)),
        Err(BridgeRejection::ValueHole),
        "a value hole rejects with the exact ValueHole variant"
    );
    assert_eq!(
        lower_comp(&closed(), &mut arena, &Comp::Hole(0)),
        Err(BridgeRejection::ComputationHole),
        "a computation hole rejects with the exact ComputationHole variant"
    );
    assert_eq!(
        lower_value_type(&mut arena, &ValueType::Unknown),
        Err(BridgeRejection::UnknownValueType),
        "the unknown value type rejects exactly"
    );
    assert_eq!(
        lower_comp_type(&mut arena, &CompType::Unknown),
        Err(BridgeRejection::UnknownComputationType),
        "the unknown computation type rejects exactly"
    );
}

#[test]
fn effects_control_class_rejects_exactly()
{
    let mut arena = gandr_kernel_core::TermArena::new();
    let reset = Comp::Reset(Rc::new(Comp::Ret(Rc::new(Value::Unit))));
    assert_eq!(
        lower_comp(&closed(), &mut arena, &reset),
        Err(BridgeRejection::Reset),
        "a control `reset` rejects with the exact Reset variant"
    );
    let perform = Comp::Perform(
        Box::new(EffectSig::new("E".into(), Vec::new())),
        String::from("op"),
        Rc::new(Value::Unit),
    );
    assert_eq!(
        lower_comp(&closed(), &mut arena, &perform),
        Err(BridgeRejection::Perform),
        "an effect `perform` rejects with the exact Perform variant"
    );
}

#[test]
fn native_class_rejects_exactly()
{
    let mut arena = gandr_kernel_core::TermArena::new();
    let native = Comp::Native {
        prim: NativePrim::RecordUpdate,
        args: Vec::new(),
    };
    assert_eq!(
        lower_comp(&closed(), &mut arena, &native),
        Err(BridgeRejection::Native),
        "a native primitive rejects with the exact Native variant"
    );
}

#[test]
fn declared_data_class_rejects_exactly()
{
    let mut arena = gandr_kernel_core::TermArena::new();
    let ctor = Value::Ctor {
        id: DataId::new(0_u64, "Maybe"),
        tag: 0,
        payload: Rc::new(Value::Unit),
    };
    assert_eq!(
        lower_value(&closed(), &mut arena, &ctor),
        Err(BridgeRejection::DataConstructor),
        "a declared-data constructor rejects with the exact DataConstructor variant"
    );
}

#[test]
fn structural_stock_class_rejects_exactly()
{
    let mut arena = gandr_kernel_core::TermArena::new();
    assert_eq!(
        lower_value(&closed(), &mut arena, &Value::List(Vec::new())),
        Err(BridgeRejection::ListValue),
        "a list value rejects with the exact ListValue variant"
    );
    assert_eq!(
        lower_value(&closed(), &mut arena, &Value::Record(BTreeMap::new())),
        Err(BridgeRejection::RecordValue),
        "a record value rejects with the exact RecordValue variant"
    );
}

#[test]
fn sigma_split_class_rejects_exactly()
{
    let mut arena = gandr_kernel_core::TermArena::new();
    let sigma = ValueType::sigma(ValueType::Unit, "x", ValueType::Unit);
    assert_eq!(
        lower_value_type(&mut arena, &sigma),
        Err(BridgeRejection::SigmaType),
        "a dependent-pair type rejects with the exact SigmaType variant"
    );
}

#[test]
fn identity_class_rejects_exactly()
{
    let mut arena = gandr_kernel_core::TermArena::new();
    let here = Value::here(Value::Unit);
    assert_eq!(
        lower_value(&closed(), &mut arena, &here),
        Err(BridgeRejection::HereProof),
        "a reflexivity proof rejects with the exact HereProof variant"
    );
}

#[test]
fn universe_class_rejects_exactly()
{
    let mut arena = gandr_kernel_core::TermArena::new();
    assert_eq!(
        lower_value_type(&mut arena, &ValueType::Universe),
        Err(BridgeRejection::UniverseType),
        "the un-levelled code universe rejects with the exact UniverseType variant"
    );
}

#[test]
fn machine_numeric_class_rejects_exactly()
{
    let mut arena = gandr_kernel_core::TermArena::new();
    assert_eq!(
        lower_value(&closed(), &mut arena, &Value::u32(1_u32)),
        Err(BridgeRejection::MachineNumericLiteral),
        "a machine-numeric literal rejects with the exact MachineNumericLiteral variant"
    );
    assert_eq!(
        lower_value_type(&mut arena, &ValueType::atom("u64")),
        Err(BridgeRejection::UnsupportedBaseAtom(String::from("u64"))),
        "a machine-numeric atom rejects with the exact UnsupportedBaseAtom variant"
    );
}

#[test]
fn every_rejection_has_a_stable_exclusion_class()
{
    // The class tags partition the vocabulary into the corpus manifest's
    // rationale classes.
    for (rejection, class) in [
        (
            BridgeRejection::UnboundName(String::new()),
            "open-free-name",
        ),
        (BridgeRejection::ValueHole, "hole-unknown"),
        (BridgeRejection::Perform, "effects-control"),
        (BridgeRejection::Native, "native"),
        (BridgeRejection::DataType, "declared-data"),
        (BridgeRejection::ListType, "structural-stock"),
        (BridgeRejection::SigmaType, "sigma-split"),
        (BridgeRejection::PathType, "identity"),
        (BridgeRejection::UniverseType, "universe"),
        (BridgeRejection::MachineNumericLiteral, "machine-numeric"),
    ] {
        assert_eq!(
            rejection.exclusion_class().as_ref(),
            class,
            "{rejection:?} belongs to the {class} class"
        );
    }
}
