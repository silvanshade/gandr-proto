//! Integration-test registration: a `tests/` file that no target compiles.
//!
//! Thirty-one workspace crates set `autotests = false` and declare one explicit
//! `[[test]]` target. In those crates Cargo discovers no test file on its own,
//! so a new `tests/something.rs` is compiled only when a `mod something;` line
//! is added by hand to the declared root. Without that line the file is never
//! built, its tests never run, and the wall stays green — the failure is
//! silent by construction, and a selector naming a test inside it runs zero
//! tests and exits zero, so the count reads legitimately green too.
//!
//! The other fourteen crates leave autodiscovery on, where a bare file works.
//! The two halves behave oppositely and nothing at the file marks which half it
//! is in, so a seat that learns the permissive half first learns the wrong
//! rule.

use alloc::collections::BTreeSet;
use alloc::vec::Vec;
use std::path::Path;

use crate::Finding;
use crate::GateResult;

crate::semantic_str!(struct ManifestText);
crate::semantic_str!(struct RootSourceText);

/// A `tests/NAME.rs` stem, with the `.rs` and the directory stripped.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct TestStem(String);

/// Finding kind for a `tests/` file no declared target reaches.
const UNREGISTERED: &str = "unregistered-test-file";

/// Report `tests/*.rs` files that no declared test target compiles.
///
/// # Contract
/// - requires: `workspace_root` contains a `crates/` directory of packages.
/// - ensures: reports one finding per `tests/*.rs` file in a crate that sets
///   `autotests = false` and whose stem is reached by no `mod` declaration in
///   any declared `[[test]]` root, excluding the roots themselves; crates that
///   leave autodiscovery on are not governed and yield nothing.
/// - provides: the registration half of the adequacy story — a witness that
///   exists, compiles, and is never built is indistinguishable from one that
///   passes.
/// - fails: returns an operational gate error when a crate directory cannot be
///   read.
/// - panics: none.
/// - intension: crates are visited in sorted order and findings are emitted in
///   sorted path order.
///
/// # Errors
/// Returns an operational error when the workspace tree cannot be walked.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — a governed crate with an unregistered file, a
///   governed crate that is fully registered, and an ungoverned crate with a
///   bare file are separated by exact finding sets.
/// - witness: `test_registration::tests::an_unregistered_file_in_a_governed_crate_is_reported`
/// - witness: `test_registration::tests::a_fully_registered_governed_crate_is_quiet`
/// - witness: `test_registration::tests::an_ungoverned_crate_is_not_governed`
#[inline]
pub fn check_test_registration(workspace_root: &Path) -> GateResult
{
    let crates = workspace_root.join("crates");
    let mut entries: Vec<_> = std::fs::read_dir(&crates)
        .map_err(|error| {
            crate::GateError::operational(format!("cannot read {}: {error}", crates.display()))
        })?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect();
    entries.sort();
    let mut findings = Vec::new();
    for crate_dir in entries {
        let manifest = crate_dir.join("Cargo.toml");
        let Ok(manifest_text) = std::fs::read_to_string(&manifest)
        else {
            continue;
        };
        let tests_dir = crate_dir.join("tests");
        let Ok(read) = std::fs::read_dir(&tests_dir)
        else {
            continue;
        };
        let mut present: Vec<TestStem> = read
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "rs"))
            .filter_map(|path| {
                path.file_stem()
                    .map(|stem| TestStem(stem.to_string_lossy().into_owned()))
            })
            .collect();
        present.sort();
        let mut roots = Vec::new();
        for stem in declared_root_stems(ManifestText(manifest_text.as_str())) {
            let root = tests_dir.join(format!("{}.rs", stem.0));
            let text = std::fs::read_to_string(&root).unwrap_or_default();
            roots.push((stem, text));
        }
        let crate_name = crate_dir
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        for stem in unregistered_stems(ManifestText(manifest_text.as_str()), &present, &roots) {
            findings.push(Finding::new(
                UNREGISTERED,
                &crate_name,
                format!("crates/{crate_name}/tests/{}.rs", stem.0),
                "",
                format!(
                    "no declared [[test]] root declares `mod {};`, so this file is never \
                     compiled and its tests never run",
                    stem.0
                ),
            ));
        }
    }
    Ok(findings)
}

/// Stems of the `tests/NAME.rs` roots a manifest declares as `[[test]]` paths.
///
/// # Contract
/// - ensures: returns each `path = "tests/NAME.rs"` stem in manifest order.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L1 pointwise — one declared root is recovered from a real
///   manifest fragment.
/// - witness: `test_registration::tests::an_unregistered_file_in_a_governed_crate_is_reported`
#[must_use]
fn declared_root_stems(manifest_text: ManifestText<'_>) -> Vec<TestStem>
{
    manifest_text
        .0
        .lines()
        .filter_map(|line| line.trim().strip_prefix("path"))
        .filter_map(|rest| rest.split('"').nth(1))
        .filter_map(|value| value.strip_prefix("tests/"))
        .filter_map(|value| value.strip_suffix(".rs"))
        .map(|value| TestStem(value.to_owned()))
        .collect()
}

/// Test-file stems a governed crate declares in no root, roots excluded.
///
/// # Contract
/// - requires: `roots` pairs each declared root stem with that root's source.
/// - ensures: returns the sorted stems present on disk that no root declares as
///   a `mod`, excluding the roots; an ungoverned manifest returns nothing.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — governed-with-gap, governed-and-complete, and
///   ungoverned are separated by exact returned sets.
/// - witness: `test_registration::tests::an_unregistered_file_in_a_governed_crate_is_reported`
/// - witness: `test_registration::tests::a_fully_registered_governed_crate_is_quiet`
/// - witness: `test_registration::tests::an_ungoverned_crate_is_not_governed`
#[must_use]
fn unregistered_stems(
    manifest_text: ManifestText<'_>,
    present: &[TestStem],
    roots: &[(TestStem, String)],
) -> Vec<TestStem>
{
    if !manifest_text.0.lines().any(|line| {
        line.split('#')
            .next()
            .unwrap_or_default()
            .contains("autotests = false")
    }) {
        return Vec::new();
    }
    let mut reached: BTreeSet<&str> = BTreeSet::new();
    for entry in roots {
        let (stem, text) = (&entry.0, &entry.1);
        reached.insert(stem.0.as_str());
        for line in text.lines() {
            let trimmed = line.trim();
            let Some(rest) = trimmed.strip_prefix("mod ").or_else(|| {
                trimmed
                    .strip_prefix("pub mod ")
                    .or_else(|| trimmed.strip_prefix("pub(crate) mod "))
            })
            else {
                continue;
            };
            if let Some(name) = rest.split(';').next() {
                reached.insert(name.trim());
            }
        }
    }
    present
        .iter()
        .filter(|stem| !reached.contains(stem.0.as_str()))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests
{
    use super::ManifestText;
    use super::TestStem;
    use super::declared_root_stems;
    use super::unregistered_stems;

    /// A governed manifest, in the shape the workspace actually uses.
    ///
    /// Copied from a real governed crate rather than minimised: a fixture is an
    /// ablation of the corpus, and a shape no author produces separates nothing
    /// about the shapes they do.
    const GOVERNED: &str = r#"
[package]
name = "gandr-surface-engine"
autotests = false

[[test]]
name = "engine"
path = "tests/lib.rs"
"#;

    /// The same manifest with autodiscovery left on.
    const UNGOVERNED: &str = r#"
[package]
name = "gandr-core-term"

[[test]]
name = "term"
path = "tests/lib.rs"
"#;

    /// A declared root that registers two of the files beside it.
    const ROOT: &str = "#![allow(clippy::pedantic)]\n\n#[cfg(test)]\nmod acceptance;\n#[cfg(test)]\nmod session;\n";

    /// THE CAN-FIRE WITNESS. A coverage check reports zero for its whole life,
    /// so a green means nothing unless the check is known to still be able to
    /// speak. This asserts that on every run rather than once in the session
    /// that built it.
    ///
    /// Ablation: delete the `!` from the `reached.contains` filter in
    /// [`unregistered_stems`] and this goes red naming `orphan`.
    #[test]
    fn an_unregistered_file_in_a_governed_crate_is_reported()
    {
        let present = [
            TestStem(String::from("acceptance")),
            TestStem(String::from("lib")),
            TestStem(String::from("orphan")),
            TestStem(String::from("session")),
        ];
        let roots = [(TestStem(String::from("lib")), String::from(ROOT))];
        assert_eq!(
            vec![TestStem(String::from("orphan"))],
            unregistered_stems(ManifestText(GOVERNED), &present, &roots),
            "a tests/ file no root declares is never compiled and must be reported"
        );
    }

    /// The negative side of the same mechanism, so a check that reported
    /// everything would fail here rather than passing the witness above.
    #[test]
    fn a_fully_registered_governed_crate_is_quiet()
    {
        let present = [
            TestStem(String::from("acceptance")),
            TestStem(String::from("lib")),
            TestStem(String::from("session")),
        ];
        let roots = [(TestStem(String::from("lib")), String::from(ROOT))];
        assert_eq!(
            Vec::<TestStem>::new(),
            unregistered_stems(ManifestText(GOVERNED), &present, &roots),
            "every file is declared, and the root itself is not an orphan"
        );
    }

    /// The boundary: fourteen crates leave autodiscovery on, where a bare file
    /// is compiled and this check must stay silent. Same inputs as the can-fire
    /// witness, differing only in the manifest.
    #[test]
    fn an_ungoverned_crate_is_not_governed()
    {
        let present = [
            TestStem(String::from("lib")),
            TestStem(String::from("orphan")),
        ];
        let roots = [(TestStem(String::from("lib")), String::from(ROOT))];
        assert_eq!(
            Vec::<TestStem>::new(),
            unregistered_stems(ManifestText(UNGOVERNED), &present, &roots),
            "autodiscovery compiles a bare file, so it is not unregistered"
        );
    }

    /// A commented-out switch does not govern a crate.
    #[test]
    fn a_commented_autotests_switch_does_not_govern()
    {
        let manifest = "[package]\nname = \"x\"\n# autotests = false\n";
        let present = [TestStem(String::from("orphan"))];
        assert_eq!(
            Vec::<TestStem>::new(),
            unregistered_stems(ManifestText(manifest), &present, &[]),
            "a commented switch leaves autodiscovery on"
        );
    }

    #[test]
    fn declared_roots_are_recovered_from_the_manifest()
    {
        assert_eq!(
            vec![TestStem(String::from("lib"))],
            declared_root_stems(ManifestText(GOVERNED))
        );
    }
}
