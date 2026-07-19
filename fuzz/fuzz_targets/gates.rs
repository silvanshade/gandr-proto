fn main()
{
    afl::fuzz!(|data: &[u8]| {
        wyrd_rust_gates::fuzzing::exercise_fuzz_input(data);
    });
}
