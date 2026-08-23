//! The corpus walker: every `.gandr` example under `examples/` parses its
//! `//@` directives, runs in its declared mode, and meets its expectations;
//! model examples must additionally open with a literate comment header (the
//! learn-by-example discipline).

/// Corpus walker tests.
#[cfg(test)]
mod tests
{
    use std::fs;
    use std::path::Path;
    use std::path::PathBuf;

    use gandr_surface_corpus::MODEL_DIR;
    use gandr_surface_corpus::PATHOLOGICAL_DIR;
    use gandr_surface_corpus::SURFACE_DIR;
    use gandr_surface_corpus::check_case;
    use gandr_surface_corpus::parse_case;

    /// Relative path of one executable corpus example tree.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct CorpusTree<'tree>(&'tree str);

    impl<'tree> From<&'tree str> for CorpusTree<'tree>
    {
        #[inline]
        fn from(value: &'tree str) -> Self
        {
            Self(value)
        }
    }

    impl AsRef<Path> for CorpusTree<'_>
    {
        #[inline]
        fn as_ref(&self) -> &Path
        {
            Path::new(self.0)
        }
    }

    impl core::fmt::Display for CorpusTree<'_>
    {
        #[inline]
        fn fmt(
            &self,
            f: &mut core::fmt::Formatter<'_>,
        ) -> core::fmt::Result
        {
            self.0.fmt(f)
        }
    }

    #[test]
    fn model_examples_meet_their_expectations()
    {
        let failures = tree_failures(MODEL_DIR);
        assert!(
            failures.is_empty(),
            "model corpus failures:\n{}",
            failures.join("\n")
        );
    }

    #[test]
    fn pathological_examples_meet_their_expectations()
    {
        let failures = tree_failures(PATHOLOGICAL_DIR);
        assert!(
            failures.is_empty(),
            "pathological corpus failures:\n{}",
            failures.join("\n")
        );
    }

    /// Every executable example submits **whole** through
    /// `Session::submit` without panicking.
    ///
    /// The expectation walkers above slice a file into items before running
    /// it, so a defect that only fires on whole-file submission is invisible
    /// to them: `module-missing-component.gandr` carried a lowering whose
    /// origin shadow tree was one level shallower than its term, the goals
    /// pass never registered the repair hole, and `Session::submit` — the
    /// REPL / LSP / driver front end — panicked at a `debug_assert!` the
    /// walker could not reach (`gandr-w0lg`). This sweep is that property,
    /// asserted for every example: total lowering accepts or recovers, and
    /// the goals machinery stays consistent with the terms it annotates.
    #[test]
    fn whole_file_submissions_never_panic_across_the_corpus()
    {
        use gandr_surface_engine::session::Session;

        let mut files = gandr_files(&crate_root().join(MODEL_DIR));
        files.extend(gandr_files(&crate_root().join(PATHOLOGICAL_DIR)));
        assert!(
            files.len() >= 100,
            "the executable trees are populated ({} files)",
            files.len()
        );
        let mut failures = Vec::new();
        for file in &files {
            let source = fs::read_to_string(file)
                .unwrap_or_else(|error| panic!("cannot read `{}`: {error}", file.display()));
            // Mirror `check_case`'s gate: an example asking for a feature this
            // build lacks is skipped by the walkers, so the sweep skips it too.
            let Ok(case) = parse_case(&source)
            else {
                failures.push(format!("{}: directive error", file.display()));
                continue;
            };
            if case.required_features.iter().any(|&feature| match feature {
                | gandr_surface_corpus::RequiredFeature::Regex => !cfg!(feature = "regex"),
                | gandr_surface_corpus::RequiredFeature::Ffi => !cfg!(feature = "ffi"),
            }) {
                continue;
            }
            let path = file.display().to_string();
            let submitted = std::panic::catch_unwind(move || {
                let mut session = Session::new();
                drop(session.submit(&source));
            });
            if submitted.is_err() {
                failures.push(format!("{path}: whole-file submission panicked"));
            }
        }
        assert!(
            failures.is_empty(),
            "whole-file submission failures:\n{}",
            failures.join("\n")
        );
    }

    /// The numbered/root corpus is exactly 31 model programs and 40
    /// pathological programs; feature subtrees are independently registered.
    ///
    /// The package rung moved both counts: one model program for the three
    /// package forms, and four failure goldens — the abstraction leak, the
    /// uninferable `pack`, the grade-zero opening, and the payload whose shape
    /// leaves the package no grade to read.
    ///
    /// The eliminator answer type and the `run` bind annotation moved both
    /// again: one model program for the two annotated surfaces, and four
    /// failure goldens — a value type in each of the two slots, an answer type
    /// the branches cannot check against (which is what shows the annotation is
    /// checked rather than recorded), and an `else if` chain annotated on a
    /// tail rung instead of its head.
    ///
    /// Executable pattern holes moved both a third time: one model program for
    /// the `?` pattern atom in a whole arm and inside a constructor, and four
    /// failure goldens — the later arm the hole shadows, the constructor no arm
    /// covers that the hole holds in front of, the payload hole that leaves its
    /// head test alone, and the first-arm hole that settles nothing for any
    /// scrutinee. The surface tree lost its `pattern-holes.gandr` reservation
    /// in the same change, which is what promotion means.
    ///
    /// Overlapping arms register in the `data/` subtrees rather than at either
    /// root: `model/data/data-matched-arms.gandr` is the one model program for
    /// the three shapes the pattern-matrix compiler took — a top-level
    /// catch-all, an or-pattern with distinguishable alternatives, and two arms
    /// at one constructor head — and `pathological/data/
    /// literal-column-declined.gandr` is the failure golden for the shape it
    /// did not take, a literal column, whose decline must stay observable as a
    /// goal rather than regressing into a dropped arm.
    ///
    /// Dependent instantiation capture moved the pathological root alone, by
    /// two, and it takes two files rather than one because the fault has two
    /// halves demanding opposite outcomes. `dependent-instantiation-capture`
    /// is the refusing half: a caller whose binder names collide with the
    /// callee's must check, and its renamed twin must check to the same type.
    /// `dependent-instantiation-capture-accepts` is the silent half, and it is
    /// a refutation — every type in it is the one a capturing instantiation
    /// computes, so a capturing engine accepts it and a correct engine refuses
    /// it. One file cannot carry both, because the first must run clean and
    /// the second must not.
    #[test]
    fn frozen_root_fixture_cardinality_is_31_and_42()
    {
        assert_eq!(
            31,
            direct_gandr_files(&crate_root().join(MODEL_DIR)).len(),
            "frozen model root"
        );
        assert_eq!(
            42,
            direct_gandr_files(&crate_root().join(PATHOLOGICAL_DIR)).len(),
            "frozen pathological root"
        );
    }

    #[test]
    fn surface_tree_is_populated_literate_and_firewalled()
    {
        // The surface reservation tree is PBG-only and firewalled
        // from execution: the corpus walker runs the model and pathological
        // trees only (never `SURFACE_DIR`), so these fixtures never lower or
        // evaluate — their gate is the PBG parser's zero-obligation sweep. This
        // test asserts the tree is populated, each fixture opens with a literate
        // graduation header, and none carries a `//@` run directive (they are
        // parse-only, never handed to `check_case`).
        let root = crate_root().join(SURFACE_DIR);
        let files = gandr_files(&root);
        assert!(
            files.len() >= 9,
            "the surface tree is populated ({} files)",
            files.len()
        );
        let mut violations = Vec::new();
        for file in &files {
            let source = fs::read_to_string(file)
                .unwrap_or_else(|error| panic!("cannot read `{}`: {error}", file.display()));
            let mut lines = source.lines().peekable();
            if lines.peek().is_some_and(|line| line.starts_with("#!/")) {
                let _shebang = lines.next();
            }
            let opens_with_prose = lines
                .next()
                .is_some_and(|line| line.starts_with("//") && !line.starts_with("//@"));
            if !opens_with_prose {
                violations.push(format!(
                    "{}: surface fixtures must open with a literate graduation comment",
                    file.display()
                ));
            }
            if source.contains("//@") {
                violations.push(format!(
                    "{}: surface fixtures are parse-only and must carry no `//@` run directive",
                    file.display()
                ));
            }
        }
        assert!(
            violations.is_empty(),
            "surface-tree violations:\n{}",
            violations.join("\n")
        );
    }

    #[test]
    fn model_examples_are_literate()
    {
        let root = crate_root().join(MODEL_DIR);
        let mut violations = Vec::new();
        for file in gandr_files(&root) {
            let source = fs::read_to_string(&file)
                .unwrap_or_else(|error| panic!("cannot read `{}`: {error}", file.display()));
            // A model example opens with prose commentary (a shebang line may
            // precede it), not with code and not with a bare directive. The
            // model tree is pedagogy and the pathological tree is testing, and
            // the literate header is what keeps the two apart on sight.
            let mut lines = source.lines().peekable();
            if lines.peek().is_some_and(|line| line.starts_with("#!/")) {
                let _shebang = lines.next();
            }
            let opens_with_prose = lines
                .next()
                .is_some_and(|line| line.starts_with("//") && !line.starts_with("//@"));
            if !opens_with_prose {
                violations.push(format!(
                    "{}: model examples must open with a literate `//` header",
                    file.display()
                ));
            }
        }
        assert!(
            violations.is_empty(),
            "literate-header violations:\n{}",
            violations.join("\n")
        );
    }

    /// Runs every example in `tree` and returns the per-file failures.
    fn tree_failures<'tree>(tree: impl Into<CorpusTree<'tree>>) -> Vec<String>
    {
        let tree = tree.into();
        let root = crate_root().join(tree);
        let files = gandr_files(&root);
        assert!(
            !files.is_empty(),
            "the `{tree}` tree must contain at least one example"
        );
        let mut failures = Vec::new();
        for file in &files {
            let source = fs::read_to_string(file)
                .unwrap_or_else(|error| panic!("cannot read `{}`: {error}", file.display()));
            for failure in check_case(&source) {
                failures.push(format!("{}: {failure}", file.display()));
            }
        }
        failures
    }

    /// The crate root (where the `examples/` trees live).
    fn crate_root() -> PathBuf
    {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    /// Direct `.gandr` children without folding in feature subtrees.
    fn direct_gandr_files(dir: &Path) -> Vec<PathBuf>
    {
        let mut files: Vec<PathBuf> = fs::read_dir(dir)
            .unwrap_or_else(|error| panic!("cannot read `{}`: {error}", dir.display()))
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.is_file()
                    && path
                        .extension()
                        .is_some_and(|extension| extension == "gandr")
            })
            .collect();
        files.sort();
        files
    }

    /// Collects every `.gandr` file under `dir`, recursively, sorted.
    fn gandr_files(dir: &Path) -> Vec<PathBuf>
    {
        let mut files = Vec::new();
        let mut pending = vec![dir.to_path_buf()];
        while let Some(current) = pending.pop() {
            let entries = fs::read_dir(&current)
                .unwrap_or_else(|error| panic!("cannot read `{}`: {error}", current.display()));
            for entry in entries {
                let path = entry.expect("directory entry").path();
                if path.is_dir() {
                    pending.push(path);
                }
                else if path.extension().is_some_and(|ext| ext == "gandr") {
                    files.push(path);
                }
            }
        }
        files.sort();
        files
    }
}
