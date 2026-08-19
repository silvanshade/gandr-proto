//! The flagship verification harness: what the higher-cells instances actually
//! elaborate to, and the one property their acceptance would have to rest on.
//!
//! # Why a check one level below the law
//!
//! A law field states an identity between applications of the shape's
//! operations. Every check the flagship carries — the stated carrier, the
//! stated endpoints, the corpus expectation over a definition's type — is aimed
//! at **the law's own type**. None of them looks at the type of the
//! **operation the law is about**.
//!
//! That level had no check on it, and it was wrong: the composition operation's
//! function-typed parameters were written as bare arrows, which are computation
//! types where value types are required, so they degraded to the gradual
//! unknown while the law above them read clean. **Proving a law about an
//! operation whose own argument types are unknown is not proving the law.**
//!
//! So this module asserts the operations are fully written, and prints every
//! item's elaborated type beside it, because diagnosing the next failure needs
//! the types rather than a verdict.

#[cfg(test)]
mod tests
{
    use std::fs;

    use gandr_surface_engine::session::ItemOutcome;
    use gandr_surface_engine::session::Session;

    /// The setoid instance, relative to the corpus crate root.
    const SETOIDS: &str = "examples/model/higher-cells/cat-shape-setoids.gandr";

    /// Elaborates `path` and returns each definition's name beside the debug
    /// rendering of its type, printing the whole submission for diagnosis.
    fn elaborated_definitions(path: &str) -> Vec<(String, String)>
    {
        let text = fs::read_to_string(path).expect("example must be readable");
        let mut session = Session::new();
        let submission = session
            .submit(text.as_str())
            .expect("lowering must be total");
        let mut definitions = Vec::new();
        println!("=== {path} ===");
        for (index, outcome) in submission.outcomes.iter().enumerate() {
            match *outcome {
                | ItemOutcome::Definition {
                    ref name, ref ty, ..
                } => {
                    println!("[{index}] def {name}\n      {ty:?}");
                    definitions.push((name.clone(), format!("{ty:?}")));
                },
                | ItemOutcome::Expression { ref ty, .. } => println!("[{index}] expr {ty:?}"),
                | ItemOutcome::TypeError { ref error } => println!("[{index}] refused {error:?}"),
                | ItemOutcome::Holey => println!("[{index}] holey"),
            }
        }
        definitions
    }

    /// The shape's operations elaborate with **no gradual unknown**, which is
    /// what makes an acceptance of a law about them evidence rather than an
    /// artefact of the unknown being consistent with everything.
    ///
    /// This asserts the durable half only. It deliberately does **not** pin how
    /// the law fields currently fail: that is expected to change the moment
    /// conversion can compute across the operations, and a test asserting
    /// today's refusal would become a false claim at exactly that moment.
    ///
    /// The check is over the debug rendering rather than the type structure,
    /// which is crude and is the honest interim: it is the by-eye check
    /// mechanised, and it should become the structural predicate once that is
    /// available to this crate.
    #[test]
    fn the_setoid_operations_are_fully_written()
    {
        for (name, rendered) in elaborated_definitions(SETOIDS) {
            if name != "id" && name != "comp" {
                continue;
            }
            assert!(
                !rendered.contains("Unknown"),
                "the operation `{name}` must elaborate with no gradual unknown, because a law \
                 proved about it would otherwise rest on a type its own source does not state; \
                 got {rendered}"
            );
        }
    }
}
