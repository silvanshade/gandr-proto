//! The gate suite's bounded parser facade must not panic on arbitrary bytes.
//!
//! `exercise_fuzz_input` is parser-only by construction — it runs no command,
//! touches no filesystem, and shares its whole implementation with the CLI
//! parser — so the property under test is exactly the parser's totality.
//!
//! Seeds: `fuzz/corpus/gates/`.

fn main()
{
    afl::fuzz!(|data: &[u8]| {
        gandr_workflow_gates::fuzzing::exercise_fuzz_input(data);
    });
}
