//! The engine stays inside this crate.
//!
//! Assessment is not adoption. While the question is open, nothing in the
//! shipping tree may acquire a dependency on the engine — and a rule that lives
//! only in prose survives exactly until someone adds a line to a manifest.

use gandr_core_incremental_assessment::boundary::ManifestText;
use gandr_core_incremental_assessment::manifests;

#[test]
fn the_engine_reaches_exactly_one_member_manifest()
{
    let members = manifests::members_naming_the_engine().expect("the workspace is readable");
    assert_eq!(
        members,
        vec![manifests::confined_to()],
        "the engine under assessment is named by exactly one member manifest; any other entry here is an adoption nobody decided on"
    );
}

#[test]
fn the_scan_recognizes_this_crate_own_declaration()
{
    // Teeth for the scan: a check that cannot fire is not a check. The
    // workspace's three declaration forms are recognized, and a crate whose
    // name merely begins the same way is not.
    let inherited = "[dependencies]\nsalsa.workspace = true\n";
    let pinned = "[dependencies]\nsalsa = \"0.28.2\"\n";
    let sectioned = "[workspace.dependencies.salsa]\nversion = \"0.28.2\"\n";
    let unrelated = "[dependencies]\nsalsa20.workspace = true\n";
    let commented = "# salsa.workspace = true\n";

    for text in [inherited, pinned, sectioned] {
        assert!(
            bool::from(manifests::names_the_engine(ManifestText::from(text))),
            "a declaration is recognized: {text:?}"
        );
    }
    assert!(
        !bool::from(manifests::names_the_engine(ManifestText::from(unrelated))),
        "a differently-named crate is not a match"
    );
    assert!(
        !bool::from(manifests::names_the_engine(ManifestText::from(commented))),
        "a commented-out line is not a declaration"
    );
}
