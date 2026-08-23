//! Goldens pinning representative core-state forms at two page widths.
//!
//! The binding invariant under test: every document's **flattened image is
//! byte-for-byte the engine's flat spelling**, so each fixture pins its wide
//! rendering against an inline literal (the parity witness) and stores both
//! widths as golden files under `tests/golden/`. A form breaks only when its
//! flattened image exceeds the page width, continuations indent two columns.
//!
//! Regenerate with `GANDR_SURFACE_PRETTY_BLESS=1 cargo nextest run -p
//! gandr-surface-pretty`, then review `git diff tests/golden/`.

#![allow(
    unknown_lints,
    primitive_signature,
    reason = "golden fixtures construct representative semantic values from compact primitive witnesses"
)]

extern crate alloc;

use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use gandr_core_term::effect::EffectRow;
use gandr_core_term::grade::Grade;
use gandr_core_term::syntax::Side;
use gandr_core_term::syntax::Value;
use gandr_core_term::types::CompType;
use gandr_core_term::types::DataId;
use gandr_core_term::types::Ty;
use gandr_core_term::types::ValueType;
use gandr_surface_layout::units::PageWidth;

/// Every fixture helper fails as a plain message.
type Fixture = Result<(), String>;

fn narrow() -> PageWidth
{
    gandr_surface_layout::units::PageWidth::from(40u32)
}

fn wide() -> PageWidth
{
    gandr_surface_layout::units::PageWidth::from(100u32)
}

/// One value-type atom.
fn atom(name: &str) -> ValueType
{
    ValueType::Atom(String::from(name))
}

/// A pure returner `F A`.
fn f_of(payload: ValueType) -> CompType
{
    CompType::F(Rc::new(payload), EffectRow::EMPTY)
}

/// A non-dependent arrow `A → B`.
fn arrow(
    arg: ValueType,
    res: CompType,
) -> CompType
{
    CompType::Arrow {
        binder: None,
        arg: Rc::new(arg),
        res: Rc::new(res),
    }
}

/// A dependent arrow `Π(x : A). B`.
fn dependent(
    binder: &str,
    arg: ValueType,
    res: CompType,
) -> CompType
{
    CompType::Arrow {
        binder: Some(String::from(binder)),
        arg: Rc::new(arg),
        res: Rc::new(res),
    }
}

/// The lazy product `B & B′`.
fn with(
    fst: CompType,
    snd: CompType,
) -> CompType
{
    CompType::With(Rc::new(fst), Rc::new(snd))
}

/// The graded thunk type `U B`.
fn thunk_type(body: CompType) -> ValueType
{
    ValueType::Thunk(Grade::OMEGA, Rc::new(body))
}

/// The stack type `Stk(B, C)`.
fn stack(
    consumes: CompType,
    delivers: CompType,
) -> ValueType
{
    ValueType::Stk(Rc::new(consumes), Rc::new(delivers))
}

/// A declared-data application `Name(args…)`.
fn data_application(
    name: &str,
    args: Vec<ValueType>,
) -> ValueType
{
    ValueType::Data {
        id: DataId::new(7u64, name),
        args: args.into_iter().map(Rc::new).collect(),
    }
}

/// An injection value.
fn injection(
    side: Side,
    payload: Value,
) -> Value
{
    Value::Inj(side, Rc::new(payload))
}

/// A right-nested chain of `depth` lists around one integer leaf.
fn nested_lists(depth: usize) -> Value
{
    let mut current = Value::int(7);
    for _ in 0 .. depth {
        current = Value::list(vec![current]);
    }
    current
}

/// Writes both widths when blessing, else asserts them against the golden
/// files.
fn check_or_bless(
    name: &str,
    narrow_text: &str,
    wide_text: &str,
) -> Fixture
{
    let dir = alloc::format!("{}/tests/golden", env!("CARGO_MANIFEST_DIR"));
    let blessing = std::env::var_os("GANDR_SURFACE_PRETTY_BLESS").is_some();
    if blessing {
        std::fs::create_dir_all(&dir)
            .map_err(|error| alloc::format!("golden directory {dir} must create: {error}"))?;
    }
    for (kind, text) in [("narrow", narrow_text), ("wide", wide_text)] {
        let path = alloc::format!("{dir}/{name}.{kind}.txt");
        if blessing {
            std::fs::write(&path, text)
                .map_err(|error| alloc::format!("blessing {path} must write: {error}"))?;
            continue;
        }
        let expected = std::fs::read_to_string(&path).map_err(|error| {
            alloc::format!("golden {path} must be committed: {error}; bless and review")
        })?;
        assert_eq!(
            expected, text,
            "golden mismatch for {name}.{kind}; bless and review the diff"
        );
    }
    Ok(())
}

/// Presents a type at both widths and pins it.
fn pinned_type(
    name: &str,
    ty: &Ty,
    wide_flat: &str,
) -> Fixture
{
    let narrow_text = gandr_surface_pretty::present_type(ty, narrow())
        .map_err(|error| alloc::format!("{name} must present at 40: {error}"))?;
    let wide_text = gandr_surface_pretty::present_type(ty, wide())
        .map_err(|error| alloc::format!("{name} must present at 100: {error}"))?;
    assert_eq!(
        wide_flat, wide_text,
        "the flattened image must be exactly the flat spelling"
    );
    check_or_bless(name, &narrow_text, &wide_text)
}

/// Presents a value at both widths and pins it.
fn pinned_value(
    name: &str,
    value: &Value,
    wide_flat: &str,
) -> Fixture
{
    let narrow_text = gandr_surface_pretty::present_value(value, narrow())
        .map_err(|error| alloc::format!("{name} must present at 40: {error}"))?;
    let wide_text = gandr_surface_pretty::present_value(value, wide())
        .map_err(|error| alloc::format!("{name} must present at 100: {error}"))?;
    assert_eq!(
        wide_flat, wide_text,
        "the flattened image must be exactly the flat spelling"
    );
    check_or_bless(name, &narrow_text, &wide_text)
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn dependent_function_type_breaks_before_codomain() -> Fixture
    {
        // Π(x : Integer). F Integer — short enough to hold either way; the
        // golden pins that small dependent types never break.
        let ty = Ty::Comp(dependent("x", atom("Integer"), f_of(atom("Integer"))));
        pinned_type("pi_dependent", &ty, "Π(x : Integer). F Integer")
    }

    #[test]
    fn long_dependent_function_type_breaks_at_the_narrow_page() -> Fixture
    {
        let inner = dependent(
            "rest",
            atom("Accumulator"),
            f_of(data_application("Sequence", vec![
                atom("Digit"),
                atom("Carry"),
            ])),
        );
        let ty = Ty::Comp(dependent(
            "input",
            data_application("Sequence", vec![atom("Digit")]),
            inner,
        ));
        let wide_flat = concat!(
            "Π(input : Sequence(Digit)). ",
            "Π(rest : Accumulator). F Sequence(Digit, Carry)"
        );
        pinned_type("pi_long", &ty, wide_flat)
    }

    #[test]
    fn arrow_chain_breaks_before_each_continuation() -> Fixture
    {
        let inner = arrow(atom("String"), f_of(ValueType::Unit));
        let ty = Ty::Comp(arrow(atom("Integer"), inner));
        pinned_type("arrow_chain", &ty, "(Integer → (String → F Unit))")
    }

    #[test]
    fn lazy_product_and_thunk_types() -> Fixture
    {
        let ty = Ty::Value(thunk_type(with(
            f_of(atom("Integer")),
            f_of(atom("String")),
        )));
        pinned_type("thunk_with", &ty, "U (F Integer & F String)")
    }

    #[test]
    fn stack_type_pins_bracketed_pair_notation() -> Fixture
    {
        let ty = Ty::Value(stack(f_of(atom("Unit")), f_of(atom("Integer"))));
        pinned_type("stack_type", &ty, "Stk(F Unit, F Integer)")
    }

    #[test]
    fn declared_data_application_breaks_arguments() -> Fixture
    {
        let ty = Ty::Value(data_application("Map", vec![
            atom("Name"),
            data_application("Set", vec![atom("Account")]),
            atom("Balance"),
        ]));
        pinned_type("data_application", &ty, "Map(Name, Set(Account), Balance)")
    }

    #[test]
    fn record_value_breaks_fields_at_the_narrow_page() -> Fixture
    {
        let value = Value::record([
            (String::from("machine"), Value::string("analytical engine")),
            (String::from("name"), Value::string("ada lovelace")),
            (String::from("year"), Value::int(1843)),
        ]);
        pinned_value(
            "record_value",
            &value,
            "#{machine = \"analytical engine\", name = \"ada lovelace\", year = 1843}",
        )
    }

    #[test]
    fn nested_list_value_stays_grouped() -> Fixture
    {
        let value = Value::list(vec![
            Value::list(vec![Value::int(1), Value::int(2)]),
            Value::list(vec![Value::int(3), Value::int(4)]),
            Value::list(vec![Value::int(5), Value::int(6)]),
        ]);
        pinned_value("nested_list_value", &value, "[[1, 2], [3, 4], [5, 6]]")
    }

    #[test]
    fn pair_of_injections_pins_sum_notation() -> Fixture
    {
        let value = Value::pair(
            injection(Side::Fst, Value::int(1)),
            injection(Side::Snd, Value::string("two")),
        );
        pinned_value("pair_injections", &value, "(Inl(1), Inr(\"two\"))")
    }

    #[test]
    fn annotations_are_transparent() -> Fixture
    {
        let value = Value::annot(Value::int(42), thunk_type(f_of(atom("Unit"))));
        pinned_value("annotated_value", &value, "42")
    }

    #[test]
    fn here_witness_pins_identity_notation() -> Fixture
    {
        let value = Value::here(Value::pair(Value::int(2), Value::int(2)));
        pinned_value("here_witness", &value, "here((2, 2))")
    }

    #[test]
    fn beyond_the_depth_limit_renders_deep() -> Fixture
    {
        // The flat renderer descends bracketing as it goes, so a value deeper
        // than the limit renders as the outer brackets around one `<deep>`
        // leaf — not a bare `<deep>`. Pin exactly that shape.
        let limit = gandr_surface_pretty::DEPTH_LIMIT;
        let value = nested_lists(limit + 8);
        let wide_flat = alloc::format!("{}<deep>{}", "[".repeat(limit), "]".repeat(limit));
        pinned_value("deep_value", &value, wide_flat.as_str())
    }
}
