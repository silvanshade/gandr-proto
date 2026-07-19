#![allow(dead_code)]
#![allow(unconditional_recursion)]

#[repr(transparent)]
#[derive(Clone, Copy)]
struct Count(u32);

fn missing_direct(value: Count) -> Count
{
    missing_direct(value)
}

/// # Termination
/// - reason: missing measure and boundedness.
fn incomplete_direct(value: Count) -> Count
{
    incomplete_direct(value)
}

fn mutual_left(value: Count) -> Count
{
    mutual_right(value)
}

fn mutual_right(value: Count) -> Count
{
    mutual_left(value)
}

enum Fuel
{
    One,
    Zero,
}

fn compliant_bounded() -> Count
{
    compliant_bounded_with_fuel(Fuel::One)
}

/// # Termination
/// - reason: recursion consumes the private closed Fuel enum.
/// - measure: remaining Fuel variants before Zero.
/// - boundedness: One strictly recurses to Zero, and Zero returns.
/// - input recursion: none.
fn compliant_bounded_with_fuel(fuel: Fuel) -> Count
{
    match fuel {
        | Fuel::One => compliant_bounded_with_fuel(Fuel::Zero),
        | Fuel::Zero => Count(0),
    }
}

/// # Termination
/// - reason: direct recursion falsely claims not to carry caller input.
/// - measure: finite wrapper depth.
/// - boundedness: each recursive call descends.
/// - input recursion: none.
fn false_direct_none(value: Count) -> Count
{
    false_direct_none(value)
}

/// # Termination
/// - reason: mutual recursion falsely claims not to carry caller input.
/// - measure: finite wrapper depth.
/// - boundedness: each recursive call descends.
/// - input recursion: none.
fn false_mutual_left_none(value: Count) -> Count
{
    false_mutual_right_none(value)
}

/// # Termination
/// - reason: mutual recursion falsely claims not to carry caller input.
/// - measure: finite wrapper depth.
/// - boundedness: each recursive call descends.
/// - input recursion: none.
fn false_mutual_right_none(value: Count) -> Count
{
    false_mutual_left_none(value)
}

/// # Termination
/// - reason: let-derived recursion falsely claims not to carry caller input.
/// - measure: finite wrapper depth.
/// - boundedness: each recursive call descends.
/// - input recursion: none.
fn false_let_none(value: Count) -> Count
{
    let child = value;
    false_let_none(child)
}

/// # Termination
/// - reason: match-derived recursion falsely claims not to carry caller input.
/// - measure: finite wrapper depth.
/// - boundedness: each recursive call descends.
/// - input recursion: none.
fn false_match_none(value: Option<Count>) -> Count
{
    match value {
        | Some(child) => false_match_none(Some(child)),
        | None => Count(0),
    }
}

struct Receiver;

impl Receiver
{
    /// # Termination
    /// - reason: method recursion falsely claims not to carry caller input.
    /// - measure: finite receiver chain.
    /// - boundedness: each recursive call descends.
    /// - input recursion: none.
    fn false_self_none(&self)
    {
        self.false_self_none();
    }
}

/// # Termination
/// - reason: ordinary APIs must not recurse over caller input.
/// - measure: finite wrapper depth.
/// - boundedness: each recursive call descends.
/// - input recursion: structural descent over the input Count.
fn ordinary_input_recursion(value: Count) -> Count
{
    ordinary_input_recursion(value)
}

#[allow(
    recursive_function_needs_termination,
    reason = "callers explicitly opt out at the narrowest function scope"
)]
fn allowed_input_recursion(value: Count) -> Count
{
    allowed_input_recursion(value)
}

fn acyclic(value: Count) -> Count
{
    value
}

fn main()
{
}
