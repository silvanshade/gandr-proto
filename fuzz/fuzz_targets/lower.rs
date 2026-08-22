//! Total lowering must terminate without panicking on arbitrary bytes.
//!
//! Every editor state lowers to a `Lowered`: out-of-fragment constructs and
//! error regions become holes carrying a note and their elided byte range, and
//! the only failures are the two input-independent infrastructure errors. So a
//! panic here is always a defect and never a rejected program.
//!
//! Seeds: `fuzz/corpus/lower/`.

use gandr_surface_engine::boundary::PipelineSource;
use gandr_surface_engine::lower::lower_source_total;

fn main()
{
    afl::fuzz!(|data: &[u8]| {
        let source = String::from_utf8_lossy(data);
        let _ = lower_source_total(PipelineSource::from(source.as_ref()));
    });
}
