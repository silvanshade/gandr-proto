use gandr_core::checker;
use gandr_core::control::Dir;
use gandr_core::machine;
use gandr_core::syntax::Term;
use gandr_pipeline::lower::lower_source_total;
use gandr_pipeline::prelude_ctx;

fn main()
{
    // Lowering then typing arbitrary bytes must not panic, and the recursive
    // checker and the typing machine must agree (event-for-event in their result)
    // on every lowered item — the byte-level extension of the conformance property.
    afl::fuzz!(|data: &[u8]| {
        let source = String::from_utf8_lossy(data);
        let Ok(lowered) = lower_source_total(&source)
        else {
            return;
        };
        for item in &lowered.items {
            match &item.term {
                | Term::Value(value) => {
                    let (rec, _) = checker::run_value(prelude_ctx(), value.clone(), Dir::Infer);
                    let (mach, _) = machine::run_value(prelude_ctx(), value.clone(), Dir::Infer);
                    assert_eq!(rec, mach, "checker and machine disagree on {item:?}");
                },
                | Term::Comp(comp) => {
                    let (rec, _) = checker::run_comp(prelude_ctx(), comp.clone(), Dir::Infer);
                    let (mach, _) = machine::run_comp(prelude_ctx(), comp.clone(), Dir::Infer);
                    assert_eq!(rec, mach, "checker and machine disagree on {item:?}");
                },
                | _ => {},
            }
        }
    });
}
