fn main()
{
    // Total lowering must terminate without panicking on arbitrary bytes: every
    // editor state lowers to a `Lowered` (out-of-fragment regions become holes).
    afl::fuzz!(|data: &[u8]| {
        let source = String::from_utf8_lossy(data);
        let _ = gandr_pipeline::lower::lower_source_total(&source);
    });
}
