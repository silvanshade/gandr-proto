//! The flagship verification harness: what the higher-cells instances actually
//! elaborate to, and the one property their acceptance would have to rest on.
//!
//! # Why a check one level below the law
//!
//! A law field states an identity between applications of the shape's
//! operations. Every check the flagship carries — the stated carrier, the
//! endpoint spines, the corpus expectation over a definition's type, and the
//! operation declarations those spines invoke — is aimed at **the law's own
//! type** and its dependent arguments.
//!
//! The endpoint former is the boundary: it carries terms inside a type, so
//! conversion can compare the resulting values while the checker validates
//! each application argument against the operation's dependent signature.
//! The operations' type parameters are explicit dependent products, allowing
//! corrected index assignments to instantiate the signature before the
//! endpoint conversion runs.

#[cfg(test)]
mod tests
{

    use gandr_surface_engine::session::ItemOutcome;
    use gandr_surface_engine::session::Session;

    /// The category of discrete setoids: an object is a type, a hom is a
    /// function, and equality is `Path`.
    ///
    /// The source is carried here rather than read from the corpus because the
    /// corpus gate admits only programs the molder can mold, and these forms
    /// are not among them. The verification does not depend on where the text
    /// lives.
    ///
    /// **The index arguments are checked against `comp`'s dependent
    /// declaration.** The left unit law is `comp(a, a, b, id(a), f)`, because
    /// `id(a) : a -> F a` fixes the middle index; the right is
    /// `comp(a, b, b, f, id(b))`.
    const FLAGSHIP: &str = concat!(
        "def id(a: Type, x: a) -> F a { ret x }\n",
        "def comp(a: Type, b: Type, c: Type, f: U[\u{3c9}] (a -> F b), g: U[\u{3c9}] (b -> F c), \
         x: a) -> F c { run y <- f(x); g(y) }\n",
        "def unitL(a: Type, b: Type, f: U[\u{3c9}] (a -> F b)) -> F(Path((U(a -> F b)), thunk { \
         comp(a, a, b, thunk { id(a) }, f) }, f)) { ret here(f) }\n",
        "def unitR(a: Type, b: Type, f: U[\u{3c9}] (a -> F b)) -> F(Path((U(a -> F b)), thunk { \
         comp(a, b, b, f, thunk { id(b) }) }, f)) { ret here(f) }\n",
    );

    /// Elaborates `path` and returns each definition's name beside the debug
    /// rendering of its type, printing the whole submission for diagnosis.
    fn elaborated_definitions() -> Vec<(String, String)>
    {
        let mut session = Session::new();
        let submission = session.submit(FLAGSHIP).expect("lowering must be total");
        let mut definitions = Vec::new();
        println!("=== the setoid instance ===");
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
        println!("--- goals: {} ---", submission.report.goals.len());
        for goal in &submission.report.goals {
            println!("    item {} note {:?}", goal.item, goal.note);
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

    /// **Both unit laws of the category of setoids are bound definitions.**
    ///
    /// This is the durable half and the reason the harness exists. Each law
    /// states an identity between an application of a defined operation and a
    /// variable, and both are *checked*: delta across the definition, beta
    /// through the application spine, thunk eta at the `U` side, the returner's
    /// right unit at the `F` side, and the definition chain reaching the item
    /// that needs it, all composing.
    ///
    /// The two laws are not symmetric in what they require, which is why both
    /// are asserted. The left law puts the identity first, so its bind fires on
    /// a returner and reduces without the `F` side at all. The right law puts
    /// it second, leaving `M >>= ret`, so it needs the returner's right unit —
    /// and needs that rule to read a sequence's triviality from its normal form
    /// rather than its stored body, because the triviality is only visible
    /// after a definition unfolds.
    ///
    /// The endpoint spine must check its indices against `comp`'s declaration.
    ///
    /// The body ignores type indices, so conversion alone accepts this
    /// deliberately absurd `comp(a, b, b, ...)` endpoint: `id(a)` has the
    /// wrong codomain for the first function slot. The formation-side endpoint
    /// check must refuse it while the corrected unit laws remain accepted.
    #[test]
    fn absurd_endpoint_index_assignment_is_refused()
    {
        let source = concat!(
            "def id(a: Type, x: a) -> F a { ret x }\n",
            "def comp(a: Type, b: Type, c: Type, f: U[\u{3c9}] (a -> F b), \
             g: U[\u{3c9}] (b -> F c), x: a) -> F c { run y <- f(x); g(y) }\n",
            "def absurd(a: Type, b: Type, f: U[\u{3c9}] (a -> F b)) -> \
             F(Path((U(a -> F b)), thunk { comp(a, b, b, thunk { id(a) }, f) }, f)) \
             { ret here(f) }\n",
        );
        let mut session = Session::new();
        let submission = session.submit(source).expect("lowering must be total");
        assert!(
            matches!(submission.outcomes[2], ItemOutcome::TypeError { .. }),
            "the absurd endpoint index assignment must be refused: {:?}",
            submission.outcomes
        );
    }
    /// A path body remains subject to ordinary term checking. Associativity
    /// deliberately applies `mul` in the witness body, so it must not become
    /// accepted merely because its endpoint indices are now checked.
    #[test]
    fn associativity_witness_application_is_refused()
    {
        let source = concat!(
            "def unit = 0;\n",
            "def mul(x: Integer, y: Integer) -> F Integer { ret x + y }\n",
            "def assoc(x: Integer, y: Integer, z: Integer) -> F Path(Integer, \
             mul(mul(x, y), z), mul(x, mul(y, z))) { ret here(mul(mul(x, y), z)) }\n",
        );
        let mut session = Session::new();
        let submission = session.submit(source).expect("lowering must be total");
        assert!(
            matches!(submission.outcomes[2], ItemOutcome::TypeError { .. }),
            "the associativity witness application must be refused: {:?}",
            submission.outcomes
        );
    }

    #[test]
    fn both_unit_laws_are_checked()
    {
        let bound: Vec<String> = elaborated_definitions()
            .into_iter()
            .map(|(name, _rendered)| name)
            .collect();
        for law in ["unitL", "unitR"] {
            assert!(
                bound.iter().any(|name| name == law),
                "the unit law `{law}` must be a bound definition rather than a type error; bound: \
                 {bound:?}"
            );
        }
    }

    /// No item of the instance carries a gradual unknown.
    ///
    /// The unknown is consistent with everything, so a law accepted at a type
    /// mentioning one is inhabited trivially rather than proved. An earlier
    /// spelling of this instance did exactly that: it accepted with a result
    /// type of `F(Unknown)`.
    #[test]
    fn the_instance_carries_no_gradual_unknown()
    {
        for (name, rendered) in elaborated_definitions() {
            assert!(
                !rendered.contains("Unknown"),
                "`{name}` must elaborate with no gradual unknown, because anything checked at a \
                 type mentioning one is consistent with everything; got {rendered}"
            );
        }
    }

    #[test]
    fn the_setoid_operations_are_fully_written()
    {
        for (name, rendered) in elaborated_definitions() {
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
