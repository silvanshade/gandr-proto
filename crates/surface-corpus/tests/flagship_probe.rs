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
    /// Tests the returner's right unit directly: is `run y <- M; ret y` equal
    /// to `M`? That is the rule the right unit law of a category bottoms out
    /// at, and it is the dual of the thunk eta on the other side of the same
    /// adjunction.
    #[test]
    fn probe_the_returner_right_unit()
    {
        let source = concat!(
            "def piped(a: Type, f: U[\u{3c9}] (a -> F a), x: a) -> F a { run y <- f(x); ret y }\n",
            "def direct(a: Type, f: U[\u{3c9}] (a -> F a), x: a) -> F a { f(x) }\n",
            "def same(a: Type, f: U[\u{3c9}] (a -> F a)) -> F(Path((U(a -> F a)), thunk { piped(a, \
             f) }, thunk { direct(a, f) })) { ret here(thunk { direct(a, f) }) }\n",
        );
        let mut session = Session::new();
        let submission = session.submit(source).expect("lowering must be total");
        println!("=== returner right unit ===");
        for (index, outcome) in submission.outcomes.iter().enumerate() {
            match *outcome {
                | ItemOutcome::Definition { ref name, .. } => println!("[{index}] def {name}"),
                | ItemOutcome::TypeError { ref error } => println!("[{index}] refused {error:?}"),
                | _ => println!("[{index}] other"),
            }
        }
    }

    /// Isolates the law from the module: same operations, same law, at the top
    /// level. If this accepts and the module-nested one does not, the module is
    /// the variable rather than the conversion relation.
    #[test]
    fn probe_the_law_outside_a_module()
    {
        // Split the composition: a SATURATED definition returning the binder
        // exercises delta, beta and the embedding with no eta at all.
        let saturated = concat!(
            "def pick(a: Type, f: U[\u{3c9}] (a -> F a)) -> F (U(a -> F a)) { ret f }\n",
            "def law(a: Type, f: U[\u{3c9}] (a -> F a)) -> F(Path((U(a -> F a)), pick(a, f), f)) \
             { ret here(f) }\n",
        );
        let mut split = Session::new();
        let split_submission = split.submit(saturated).expect("lowering must be total");
        // Same two definitions, submitted SEPARATELY rather than as one source.
        let mut staged = Session::new();
        let first = staged
            .submit("def pick(a: Type, f: U[\u{3c9}] (a -> F a)) -> F (U(a -> F a)) { ret f }")
            .expect("lowering must be total");
        let second = staged
            .submit(
                "def law(a: Type, f: U[\u{3c9}] (a -> F a)) -> F(Path((U(a -> F a)), pick(a, f), \
                 f)) { ret here(f) }",
            )
            .expect("lowering must be total");
        println!("=== staged, one item per submission ===");
        for outcome in first.outcomes.iter().chain(second.outcomes.iter()) {
            match *outcome {
                | ItemOutcome::Definition {
                    ref name, ref ty, ..
                } => println!("  def {name}\n        {ty:?}"),
                | ItemOutcome::TypeError { ref error } => println!("  refused {error:?}"),
                | _ => println!("  other"),
            }
        }
        println!("=== saturated, no eta needed ===");
        for (index, outcome) in split_submission.outcomes.iter().enumerate() {
            match *outcome {
                | ItemOutcome::Definition { ref name, .. } => println!("[{index}] def {name}"),
                | ItemOutcome::TypeError { ref error } => println!("[{index}] refused {error:?}"),
                | _ => println!("[{index}] other"),
            }
        }

        let source = concat!(
            "def id(a: Type, x: a) -> F a { ret x }\n",
            "def comp(a: Type, b: Type, c: Type, f: U[\u{3c9}] (a -> F b), g: U[\u{3c9}] (b -> F c), \
             x: a) -> F c { run y <- f(x); g(y) }\n",
            "def unitL(a: Type, b: Type, f: U[\u{3c9}] (a -> F b)) -> F(Path((U(a -> F b)), thunk { \
             comp(a, b, b, thunk { id(a) }, f) }, f)) { ret here(f) }\n",
        );
        let mut session = Session::new();
        let submission = session.submit(source).expect("lowering must be total");
        println!("=== standalone ===");
        for (index, outcome) in submission.outcomes.iter().enumerate() {
            match *outcome {
                | ItemOutcome::Definition {
                    ref name, ref ty, ..
                } => println!("[{index}] def {name}\n      {ty:?}"),
                | ItemOutcome::Expression { ref ty, .. } => println!("[{index}] expr {ty:?}"),
                | ItemOutcome::TypeError { ref error } => println!("[{index}] refused {error:?}"),
                | ItemOutcome::Holey => println!("[{index}] holey"),
            }
        }
    }

    /// **The left unit law of the category of setoids is a bound definition.**
    ///
    /// This is the durable half and the reason the harness exists. The law
    /// states an identity between an application of a defined operation and a
    /// variable, and it is *checked* — delta across the definition, beta
    /// through the application spine, thunk eta at the end, and the definition
    /// chain reaching the item that needs it, all composing.
    ///
    /// It deliberately does **not** assert anything about the right unit law,
    /// which refuses today for a reason that is expected to stop holding: it
    /// bottoms out at the returner's right unit, and a test pinning today's
    /// refusal would become a false claim the moment that rule lands.
    #[test]
    fn the_left_unit_law_is_checked()
    {
        let bound = elaborated_definitions(SETOIDS)
            .into_iter()
            .any(|(name, _rendered)| name == "unitL");
        assert!(
            bound,
            "the left unit law must be a bound definition rather than a type error"
        );
    }

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
