//! The corpus walker (ADR-84): every `.gandr` example under
//! `examples/` parses its `//@` directives, runs in its declared mode, and
//! meets its expectations; model examples must additionally open with a
//! literate comment header (the learn-by-example discipline).

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

    /// The frozen numbered/root corpus remains exactly 29 model programs and
    /// 27 pathological programs; feature subtrees are independently registered.
    #[test]
    fn frozen_root_fixture_cardinality_is_29_and_27()
    {
        assert_eq!(
            29,
            direct_gandr_files(&crate_root().join(MODEL_DIR)).len(),
            "frozen model root"
        );
        assert_eq!(
            27,
            direct_gandr_files(&crate_root().join(PATHOLOGICAL_DIR)).len(),
            "frozen pathological root"
        );
    }

    #[test]
    fn surface_tree_is_populated_literate_and_firewalled()
    {
        // The W4d surface fold-in tree (`wyrd-ku0f`) is PBG-only and firewalled
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
            // precede it), not with code and not with a bare directive: the
            // literate discipline of ADR-52 Decision C.
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
