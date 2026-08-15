#[cfg(test)]
mod tests
{
    //! Integration coverage for structural graph-boundary diagnostics.

    use core::error::Error;
    use core::sync::atomic::AtomicU64;
    use core::sync::atomic::Ordering;
    use std::path::Path;
    use std::path::PathBuf;

    use gandr_workflow_gates::Finding;
    use gandr_workflow_gates::GateError;
    use gandr_workflow_gates::graph_boundary::run;

    /// Test result alias used by all integration cases.
    type TestResult<T = ()> = Result<T, Box<dyn Error>>;

    gandr_workflow_gates::semantic_str!(pub(crate) struct NameText);
    gandr_workflow_gates::semantic_str!(pub(crate) struct SourceText);
    gandr_workflow_gates::semantic_str!(pub(crate) struct ValueText);
    gandr_workflow_gates::semantic_copy!(pub struct NodeCountCount(u32));
    gandr_workflow_gates::semantic_copy!(pub struct PublicAlgorithmCount(u32));

    /// Per-process suffix keeping concurrently-created graph-boundary fixtures
    /// disjoint.
    static NEXT_TEMP_ROOT: AtomicU64 = AtomicU64::new(0);

    /// Stable detail for direct dependency violations.
    const DIRECT_DEPENDENCY_DETAIL: &str =
        "petgraph/fixedbitset may be direct dependencies only of gandr-theory-graphs";
    /// Stable detail for non-graph source references to graph-stack crates.
    const OUTSIDE_SOURCE_DETAIL: &str =
        "source outside gandr-theory-graphs must not mention petgraph/fixedbitset graph-stack APIs";
    /// Stable detail for public `gandr-theory-graphs` API exposure violations.
    const PUBLIC_API_DETAIL: &str = "public gandr-theory-graphs declarations must not expose petgraph/fixedbitset graph-stack APIs";

    /// Temporary workspace fixture plus metadata path.
    struct Fixture
    {
        /// Temporary workspace root.
        root: PathBuf,
        /// Fixture Cargo metadata JSON path.
        metadata_path: PathBuf,
    }

    impl Fixture
    {
        /// Create a temporary workspace containing the requested packages.
        fn create(packages: &[PackageFixture]) -> TestResult<Self>
        {
            let root = unique_root();
            gandr_workflow_gates::support::HOST_FILESYSTEM.create_dir_all(&root)?;

            for package in packages {
                let package_root = root.join("crates").join(package.name);
                gandr_workflow_gates::support::HOST_FILESYSTEM
                    .create_dir_all(package_root.join("src"))?;
                gandr_workflow_gates::support::HOST_FILESYSTEM.write(
                    package_root.join("Cargo.toml"),
                    "[package]\nname = \"fixture\"\n",
                )?;
                gandr_workflow_gates::support::HOST_FILESYSTEM
                    .write(package_root.join("src/lib.rs"), package.source)?;
            }

            let metadata_path = root.join("metadata.json");
            gandr_workflow_gates::support::HOST_FILESYSTEM
                .write(&metadata_path, metadata_json(&root, packages))?;

            Ok(Self {
                root,
                metadata_path,
            })
        }
    }

    impl Drop for Fixture
    {
        /// Remove the temporary workspace best-effort.
        fn drop(&mut self)
        {
            drop(gandr_workflow_gates::support::HOST_FILESYSTEM.remove_dir_all(&self.root));
        }
    }

    /// Package fixture used to synthesize Cargo metadata and source.
    struct PackageFixture
    {
        /// Cargo package name.
        name: &'static str,
        /// Rust source written to `src/lib.rs`.
        source: &'static str,
        /// Direct dependency names emitted in metadata.
        dependencies: &'static [&'static str],
    }

    /// Clean graph owner dependencies and clean app sources produce no
    /// findings.
    #[test]
    fn clean_metadata_and_sources_have_no_findings() -> TestResult
    {
        let fixture = Fixture::create(&[
            PackageFixture {
                name: "gandr-theory-graphs",
                source: "pub trait EdgeSource { fn node_count(&self) -> impl Into<NodeCountCount>; }\n",
                dependencies: &["petgraph"],
            },
            PackageFixture {
                name: "gandr-app",
                source: "pub fn count() -> u32 { 1 }\n",
                dependencies: &[],
            },
        ])?;

        let findings = run_ok(&fixture)?;

        assert_finding_lines(&findings, &[]);
        Ok(())
    }

    /// Non-owner packages may not directly depend on graph-stack crates.
    #[test]
    fn direct_petgraph_and_fixedbitset_dependencies_are_rejected() -> TestResult
    {
        let fixture = Fixture::create(&[
            PackageFixture {
                name: "gandr-theory-graphs",
                source: "pub fn graph_boundary_owner() {}\n",
                dependencies: &["petgraph"],
            },
            PackageFixture {
                name: "gandr-app",
                source: "pub fn app() {}\n",
                dependencies: &["fixedbitset", "petgraph"],
            },
        ])?;

        let findings = run_ok(&fixture)?;

        assert_finding_lines(&findings, &[
            format!(
                "kind=direct-dependency package=gandr-app path=crates/gandr-app/Cargo.toml declaration=fixedbitset detail={DIRECT_DEPENDENCY_DETAIL}"
            ),
            format!(
                "kind=direct-dependency package=gandr-app path=crates/gandr-app/Cargo.toml declaration=petgraph detail={DIRECT_DEPENDENCY_DETAIL}"
            ),
        ]);
        Ok(())
    }

    /// Private outside-crate graph-stack source references are rejected.
    #[test]
    fn private_outside_crate_graph_stack_use_is_rejected() -> TestResult
    {
        let fixture = Fixture::create(&[
            PackageFixture {
                name: "gandr-theory-graphs",
                source: "pub fn graph_boundary_owner() {}\n",
                dependencies: &["petgraph"],
            },
            PackageFixture {
                name: "gandr-app",
                source: r#"
fn private_path() {
    let _size = core::mem::size_of::<petgraph::graph::NodeIndex>();
}

use fixedbitset::FixedBitSet as Bits;
"#,
                dependencies: &[],
            },
        ])?;

        let findings = run_ok(&fixture)?;

        assert_finding_lines(&findings, &[
            format!(
                "kind=source-declaration package=gandr-app path=crates/gandr-app/src/lib.rs declaration=graph-stack path: fixedbitset detail={OUTSIDE_SOURCE_DETAIL}"
            ),
            format!(
                "kind=source-declaration package=gandr-app path=crates/gandr-app/src/lib.rs declaration=graph-stack path: petgraph detail={OUTSIDE_SOURCE_DETAIL}"
            ),
        ]);
        Ok(())
    }

    /// Grouped and renamed public uses preserve structural boundary coverage.
    #[test]
    fn multiline_nested_grouped_and_renamed_public_uses_are_rejected() -> TestResult
    {
        let fixture = Fixture::create(&[PackageFixture {
            name: "gandr-theory-graphs",
            source: r#"
pub use petgraph::{
    graph::{
        Graph as PgGraph,
        NodeIndex,
    },
    visit::EdgeRef,
};
pub use fixedbitset::FixedBitSet as Bits;
"#,
            dependencies: &["petgraph"],
        }])?;

        let findings = run_ok(&fixture)?;

        assert_finding_lines(&findings, &[
            format!(
                "kind=public-graph-boundary package=gandr-theory-graphs path=crates/gandr-theory-graphs/src/lib.rs declaration=pub use fixedbitset::FixedBitSet as Bits: fixedbitset detail={PUBLIC_API_DETAIL}"
            ),
            format!(
                "kind=public-graph-boundary package=gandr-theory-graphs path=crates/gandr-theory-graphs/src/lib.rs declaration=pub use petgraph::{{graph::{{Graph as PgGraph, NodeIndex}}, visit::EdgeRef}}: petgraph detail={PUBLIC_API_DETAIL}"
            ),
        ]);
        Ok(())
    }

    /// Public signatures expose graph-stack roots while private methods do not.
    #[test]
    fn public_signature_type_and_trait_leaks_are_rejected() -> TestResult
    {
        let fixture = Fixture::create(&[PackageFixture {
            name: "gandr-theory-graphs",
            source: r#"
pub type PublicGraph = petgraph::Graph<u32, u32>;
pub const PUBLIC_BITS: fixedbitset::FixedBitSet = fixedbitset::FixedBitSet::new();
pub struct PublicStruct {
    pub node: petgraph::graph::NodeIndex,
    hidden: petgraph::Graph<(), ()>,
}
pub enum PublicEnum {
    Graph(petgraph::Graph<(), ()>),
}
pub trait PublicTrait {
    type GraphLike: Into<petgraph::Graph<(), ()>>;
    fn edge(&self) -> fixedbitset::FixedBitSet;
}
pub struct Owner;
impl Owner {
    pub fn build(input: petgraph::Graph<(), ()>) -> u32 { 0 }
    fn private_method(input: fixedbitset::FixedBitSet) -> u32 { 0 }
}
"#,
            dependencies: &["petgraph"],
        }])?;

        let findings = run_ok(&fixture)?;

        assert_finding_lines(&findings, &[
            format!(
                "kind=public-graph-boundary package=gandr-theory-graphs path=crates/gandr-theory-graphs/src/lib.rs declaration=impl: petgraph detail={PUBLIC_API_DETAIL}"
            ),
            format!(
                "kind=public-graph-boundary package=gandr-theory-graphs path=crates/gandr-theory-graphs/src/lib.rs declaration=pub const PUBLIC_BITS: fixedbitset detail={PUBLIC_API_DETAIL}"
            ),
            format!(
                "kind=public-graph-boundary package=gandr-theory-graphs path=crates/gandr-theory-graphs/src/lib.rs declaration=pub enum PublicEnum: petgraph detail={PUBLIC_API_DETAIL}"
            ),
            format!(
                "kind=public-graph-boundary package=gandr-theory-graphs path=crates/gandr-theory-graphs/src/lib.rs declaration=pub struct PublicStruct: petgraph detail={PUBLIC_API_DETAIL}"
            ),
            format!(
                "kind=public-graph-boundary package=gandr-theory-graphs path=crates/gandr-theory-graphs/src/lib.rs declaration=pub trait PublicTrait: fixedbitset detail={PUBLIC_API_DETAIL}"
            ),
            format!(
                "kind=public-graph-boundary package=gandr-theory-graphs path=crates/gandr-theory-graphs/src/lib.rs declaration=pub trait PublicTrait: petgraph detail={PUBLIC_API_DETAIL}"
            ),
            format!(
                "kind=public-graph-boundary package=gandr-theory-graphs path=crates/gandr-theory-graphs/src/lib.rs declaration=pub type PublicGraph: petgraph detail={PUBLIC_API_DETAIL}"
            ),
        ]);
        Ok(())
    }

    /// Private graph-stack implementation details inside `gandr-theory-graphs`
    /// are allowed.
    #[test]
    fn private_implementation_inside_gandr_graph_does_not_leak() -> TestResult
    {
        let fixture = Fixture::create(&[PackageFixture {
            name: "gandr-theory-graphs",
            source: r#"
use petgraph::Graph;

struct PrivateField {
    graph: petgraph::Graph<(), ()>,
}

pub struct PublicOpaque {
    graph: petgraph::Graph<(), ()>,
}

pub fn public_algorithm() -> impl Into<PublicAlgorithmCount> {
    let _size = core::mem::size_of::<petgraph::Graph<(), ()>>();
    0
}

mod private_mod {
    pub fn still_private(input: fixedbitset::FixedBitSet) -> u32 { 0 }
}

struct Private;
impl petgraph::visit::GraphBase for Private {
    type NodeId = u32;
    type EdgeId = u32;
}
"#,
            dependencies: &["petgraph"],
        }])?;

        let findings = run_ok(&fixture)?;

        assert_finding_lines(&findings, &[]);
        Ok(())
    }

    /// Only declared public modules are inspected as external graph API.
    #[test]
    fn public_module_graph_controls_api_analysis() -> TestResult
    {
        let fixture = Fixture::create(&[PackageFixture {
            name: "gandr-theory-graphs",
            source: "mod hidden;\npub mod api;\n",
            dependencies: &["petgraph"],
        }])?;
        let src = fixture.root.join("crates/gandr-theory-graphs/src");
        gandr_workflow_gates::support::HOST_FILESYSTEM.write(
            src.join("hidden.rs"),
            "pub type HiddenGraph = petgraph::Graph<(), ()>;\n",
        )?;
        gandr_workflow_gates::support::HOST_FILESYSTEM.write(
            src.join("api.rs"),
            "pub type ApiGraph = petgraph::Graph<(), ()>;\n",
        )?;

        let findings = run_ok(&fixture)?;

        assert_finding_lines(&findings, &[format!(
            "kind=public-graph-boundary package=gandr-theory-graphs path=crates/gandr-theory-graphs/src/api.rs declaration=pub type ApiGraph: petgraph detail={PUBLIC_API_DETAIL}"
        )]);
        Ok(())
    }

    /// Inline module directories are advanced before nested out-of-line
    /// modules are resolved.
    #[test]
    fn inline_module_directories_resolve_nested_out_of_line_modules() -> TestResult
    {
        let fixture = Fixture::create(&[PackageFixture {
            name: "gandr-theory-graphs",
            source: "pub mod outer { pub mod api; }\n",
            dependencies: &["petgraph"],
        }])?;
        let src = fixture.root.join("crates/gandr-theory-graphs/src");
        gandr_workflow_gates::support::HOST_FILESYSTEM.create_dir_all(src.join("outer"))?;
        gandr_workflow_gates::support::HOST_FILESYSTEM.write(
            src.join("outer/api.rs"),
            "pub type NestedApiGraph = petgraph::Graph<(), ()>;\n",
        )?;

        let findings = run_ok(&fixture)?;

        assert_finding_lines(&findings, &[format!(
            "kind=public-graph-boundary package=gandr-theory-graphs path=crates/gandr-theory-graphs/src/outer/api.rs declaration=pub type NestedApiGraph: petgraph detail={PUBLIC_API_DETAIL}"
        )]);
        Ok(())
    }

    /// Path-overridden modules resolve relative to the effective current
    /// module directory.
    #[test]
    fn path_overridden_modules_resolve_from_effective_module_directory() -> TestResult
    {
        let fixture = Fixture::create(&[PackageFixture {
            name: "gandr-theory-graphs",
            source: "pub mod outer { #[path = \"renamed.rs\"] pub mod api; }\n",
            dependencies: &["petgraph"],
        }])?;
        let src = fixture.root.join("crates/gandr-theory-graphs/src");
        gandr_workflow_gates::support::HOST_FILESYSTEM.create_dir_all(src.join("outer"))?;
        gandr_workflow_gates::support::HOST_FILESYSTEM.write(
            src.join("outer/renamed.rs"),
            "pub type OverrideApiGraph = petgraph::Graph<(), ()>;\n",
        )?;

        let findings = run_ok(&fixture)?;

        assert_finding_lines(&findings, &[format!(
            "kind=public-graph-boundary package=gandr-theory-graphs path=crates/gandr-theory-graphs/src/outer/renamed.rs declaration=pub type OverrideApiGraph: petgraph detail={PUBLIC_API_DETAIL}"
        )]);
        Ok(())
    }

    /// Ambiguous public module file layouts fail closed with a typed
    /// operational error instead of silently skipping a public module.
    #[test]
    fn ambiguous_public_module_sources_are_operational_errors() -> TestResult
    {
        let fixture = Fixture::create(&[PackageFixture {
            name: "gandr-theory-graphs",
            source: "pub mod api;\n",
            dependencies: &["petgraph"],
        }])?;
        let src = fixture.root.join("crates/gandr-theory-graphs/src");
        gandr_workflow_gates::support::HOST_FILESYSTEM
            .write(src.join("api.rs"), "pub fn flat() {}\n")?;
        gandr_workflow_gates::support::HOST_FILESYSTEM.create_dir_all(src.join("api"))?;
        gandr_workflow_gates::support::HOST_FILESYSTEM
            .write(src.join("api/mod.rs"), "pub fn nested() {}\n")?;

        let result = run(&fixture.root, Some(&fixture.metadata_path));

        assert!(
            matches!(result, Err(GateError::Operational { .. })),
            "ambiguous public module source should fail closed, got {result:?}"
        );
        Ok(())
    }

    /// Restricted visibility and private adapter impls remain implementation
    /// details, even when they mention graph-stack crates.
    #[test]
    fn restricted_visibility_and_private_adapters_are_not_public_api() -> TestResult
    {
        let fixture = Fixture::create(&[PackageFixture {
            name: "gandr-theory-graphs",
            source: r#"
pub(crate) type CrateGraph = petgraph::Graph<(), ()>;
pub mod scoped {
    pub(super) type SuperGraph = petgraph::Graph<(), ()>;
    pub(in crate) type InGraph = fixedbitset::FixedBitSet;
}
struct PrivateAdapter;
impl PrivateAdapter {
    pub fn adapt(input: petgraph::Graph<(), ()>) -> u32 { 0 }
}
"#,
            dependencies: &["petgraph"],
        }])?;

        let findings = run_ok(&fixture)?;

        assert_finding_lines(&findings, &[]);
        Ok(())
    }

    /// Private renamed imports and type aliases cannot hide public graph-stack
    /// signatures, including generic, trait, const, and associated-type impl
    /// surfaces.
    #[test]
    fn alias_and_impl_surfaces_are_rejected() -> TestResult
    {
        let fixture = Fixture::create(&[PackageFixture {
            name: "gandr-theory-graphs",
            source: r#"
extern crate petgraph as pg;
use fixedbitset::FixedBitSet as Bits;
use petgraph::graph::NodeIndex as Id;
pub type PublicId = Id;
pub struct Owner;
impl<T> Owner where T: Into<Bits> {
    pub const BITS: Option<Bits> = None;
    pub fn build(input: Id) -> Option<pg::Graph<(), ()>> { None }
}
impl pg::visit::GraphBase for Owner {
    type NodeId = Id;
    type EdgeId = Id;
}
"#,
            dependencies: &["petgraph"],
        }])?;

        let findings = run_ok(&fixture)?;

        assert_finding_lines(&findings, &[
            format!(
                "kind=public-graph-boundary package=gandr-theory-graphs path=crates/gandr-theory-graphs/src/lib.rs declaration=impl: fixedbitset detail={PUBLIC_API_DETAIL}"
            ),
            format!(
                "kind=public-graph-boundary package=gandr-theory-graphs path=crates/gandr-theory-graphs/src/lib.rs declaration=impl: petgraph detail={PUBLIC_API_DETAIL}"
            ),
            format!(
                "kind=public-graph-boundary package=gandr-theory-graphs path=crates/gandr-theory-graphs/src/lib.rs declaration=impl: petgraph detail={PUBLIC_API_DETAIL}"
            ),
            format!(
                "kind=public-graph-boundary package=gandr-theory-graphs path=crates/gandr-theory-graphs/src/lib.rs declaration=pub type PublicId: petgraph detail={PUBLIC_API_DETAIL}"
            ),
        ]);
        Ok(())
    }

    /// Forbidden private glob imports cannot hide graph-stack names used by
    /// public signatures.
    #[test]
    fn private_forbidden_glob_cannot_hide_public_signature_root() -> TestResult
    {
        let fixture = Fixture::create(&[PackageFixture {
            name: "gandr-theory-graphs",
            source: r#"
use petgraph::graph::*;
pub fn identity<T>(x: T) -> T { x }
pub fn expose(x: NodeIndex) -> NodeIndex { x }
"#,
            dependencies: &["petgraph"],
        }])?;

        let findings = run_ok(&fixture)?;

        assert_finding_lines(&findings, &[format!(
            "kind=public-graph-boundary package=gandr-theory-graphs path=crates/gandr-theory-graphs/src/lib.rs declaration=pub fn expose: petgraph detail={PUBLIC_API_DETAIL}"
        )]);
        Ok(())
    }

    /// Forbidden glob imports are allowed when confined to an effectively
    /// private module.
    #[test]
    fn forbidden_glob_inside_private_module_does_not_leak() -> TestResult
    {
        let fixture = Fixture::create(&[PackageFixture {
            name: "gandr-theory-graphs",
            source: r#"
mod private {
    use petgraph::graph::*;
    pub fn expose(x: NodeIndex) -> NodeIndex { x }
}
"#,
            dependencies: &["petgraph"],
        }])?;

        let findings = run_ok(&fixture)?;

        assert_finding_lines(&findings, &[]);
        Ok(())
    }

    /// Trait impls on private self types are not externally nameable API.
    #[test]
    fn private_self_trait_impl_is_not_public_api() -> TestResult
    {
        let fixture = Fixture::create(&[PackageFixture {
            name: "gandr-theory-graphs",
            source: r#"
struct Private;
impl petgraph::visit::GraphBase for Private {
    type NodeId = u32;
    type EdgeId = u32;
}
"#,
            dependencies: &["petgraph"],
        }])?;

        let findings = run_ok(&fixture)?;

        assert_finding_lines(&findings, &[]);
        Ok(())
    }

    /// Trait impls on public self types are externally visible API.
    #[test]
    fn public_self_trait_impl_is_public_api() -> TestResult
    {
        let fixture = Fixture::create(&[PackageFixture {
            name: "gandr-theory-graphs",
            source: r#"
pub struct Public;
impl petgraph::visit::GraphBase for Public {
    type NodeId = u32;
    type EdgeId = u32;
}
"#,
            dependencies: &["petgraph"],
        }])?;

        let findings = run_ok(&fixture)?;

        assert_finding_lines(&findings, &[format!(
            "kind=public-graph-boundary package=gandr-theory-graphs path=crates/gandr-theory-graphs/src/lib.rs declaration=impl: petgraph detail={PUBLIC_API_DETAIL}"
        )]);
        Ok(())
    }

    /// Additional public type-bearing and re-exporting item forms are
    /// inspected.
    #[test]
    fn additional_public_type_bearing_forms_are_rejected() -> TestResult
    {
        let fixture = Fixture::create(&[PackageFixture {
            name: "gandr-theory-graphs",
            source: r#"
pub extern crate fixedbitset;
pub union PublicUnion {
    pub node: core::mem::ManuallyDrop<petgraph::Graph<(), ()>>,
}
unsafe extern "C" {
    pub fn foreign(input: fixedbitset::FixedBitSet);
}
pub trait GraphAlias = petgraph::visit::GraphBase;
"#,
            dependencies: &["petgraph"],
        }])?;

        let findings = run_ok(&fixture)?;

        assert_finding_lines(&findings, &[
            format!(
                "kind=public-graph-boundary package=gandr-theory-graphs path=crates/gandr-theory-graphs/src/lib.rs declaration=pub extern crate fixedbitset: fixedbitset detail={PUBLIC_API_DETAIL}"
            ),
            format!(
                "kind=public-graph-boundary package=gandr-theory-graphs path=crates/gandr-theory-graphs/src/lib.rs declaration=pub foreign fn foreign: fixedbitset detail={PUBLIC_API_DETAIL}"
            ),
            format!(
                "kind=public-graph-boundary package=gandr-theory-graphs path=crates/gandr-theory-graphs/src/lib.rs declaration=pub trait GraphAlias: petgraph detail={PUBLIC_API_DETAIL}"
            ),
            format!(
                "kind=public-graph-boundary package=gandr-theory-graphs path=crates/gandr-theory-graphs/src/lib.rs declaration=pub union PublicUnion: petgraph detail={PUBLIC_API_DETAIL}"
            ),
        ]);
        Ok(())
    }

    /// Symlinked source children are skipped rather than followed outside the
    /// package root or into an ancestor cycle.
    #[cfg(unix)]
    #[test]
    fn symlinked_source_entries_are_bounded() -> TestResult
    {
        let fixture = Fixture::create(&[
            PackageFixture {
                name: "gandr-theory-graphs",
                source: "pub fn graph_boundary_owner() {}\n",
                dependencies: &["petgraph"],
            },
            PackageFixture {
                name: "gandr-app",
                source: "pub fn app() {}\n",
                dependencies: &[],
            },
        ])?;
        let app_src = fixture.root.join("crates/gandr-app/src");
        let external = unique_root();
        gandr_workflow_gates::support::HOST_FILESYSTEM.create_dir_all(&external)?;
        gandr_workflow_gates::support::HOST_FILESYSTEM
            .write(external.join("leak.rs"), "use petgraph::Graph;\n")?;
        gandr_workflow_gates::support::HOST_FILESYSTEM
            .symlink(&fixture.root, app_src.join("ancestor"))?;
        gandr_workflow_gates::support::HOST_FILESYSTEM
            .symlink(&external, app_src.join("external"))?;

        let findings = run_ok(&fixture)?;

        drop(gandr_workflow_gates::support::HOST_FILESYSTEM.remove_dir_all(external));
        assert_finding_lines(&findings, &[]);
        Ok(())
    }

    /// Relative manifest paths resolve against the supplied workspace root,
    /// even when that path exists nowhere below the process current directory.
    #[test]
    fn relative_manifest_paths_resolve_from_workspace_root() -> TestResult
    {
        let fixture = Fixture::create(&[PackageFixture {
            name: "gandr-theory-graphs",
            source: "pub fn graph_boundary_owner() {}\n",
            dependencies: &["petgraph"],
        }])?;
        let nested_workspace = fixture.root.join("fixture-only");
        gandr_workflow_gates::support::HOST_FILESYSTEM.create_dir_all(&nested_workspace)?;
        gandr_workflow_gates::support::HOST_FILESYSTEM
            .rename(fixture.root.join("crates"), nested_workspace.join("crates"))?;
        gandr_workflow_gates::support::HOST_FILESYSTEM.write(&fixture.metadata_path, "{\"packages\":[{\"id\":\"gandr-theory-graphs\",\"name\":\"gandr-theory-graphs\",\"manifest_path\":\"crates/gandr-theory-graphs/Cargo.toml\",\"dependencies\":[{\"name\":\"petgraph\"}]}],\"workspace_members\":[\"gandr-theory-graphs\"]}")?;

        let result = run(&nested_workspace, Some(&fixture.metadata_path));

        let findings = result
            .map_err(|error| GateError::operational(format!("graph-boundary failed: {error:?}")))?;
        assert_finding_lines(&findings, &[]);
        Ok(())
    }

    /// Rust parse failures are returned as operational gate errors.
    #[test]
    fn rust_parse_errors_are_operational_errors() -> TestResult
    {
        let fixture = Fixture::create(&[PackageFixture {
            name: "gandr-app",
            source: "pub fn broken( {\n",
            dependencies: &[],
        }])?;

        let result = run(&fixture.root, Some(&fixture.metadata_path));

        assert!(
            matches!(result, Err(GateError::RustParse { .. })),
            "invalid Rust source should be a RustParse error, got {result:?}"
        );
        let Err(GateError::RustParse { path, .. }) = result
        else {
            return Ok(());
        };
        assert_eq!(
            path,
            fixture.root.join("crates/gandr-app/src/lib.rs"),
            "invalid Rust source should report the parsed source path"
        );
        Ok(())
    }

    /// Repeated graph-boundary analysis emits deterministic finding order.
    #[test]
    fn findings_are_deterministically_ordered() -> TestResult
    {
        let fixture = Fixture::create(&[
            PackageFixture {
                name: "zeta-app",
                source: "use petgraph::Graph;\n",
                dependencies: &["petgraph"],
            },
            PackageFixture {
                name: "gandr-theory-graphs",
                source: "pub type Zed = petgraph::Graph<(), ()>;\n",
                dependencies: &["petgraph"],
            },
            PackageFixture {
                name: "alpha-app",
                source: "use fixedbitset::FixedBitSet;\n",
                dependencies: &["fixedbitset"],
            },
        ])?;

        let first_output = run_ok(&fixture)?;
        let second_output = run_ok(&fixture)?;
        let first = finding_lines(&first_output);
        let second = finding_lines(&second_output);
        let mut sorted = first.clone();
        sorted.sort();

        assert_eq!(first, second, "repeated analysis should be deterministic");
        assert_eq!(first, sorted, "findings should be emitted in sorted order");
        assert_eq!(
            vec![
                format!(
                    "kind=direct-dependency package=alpha-app path=crates/alpha-app/Cargo.toml declaration=fixedbitset detail={DIRECT_DEPENDENCY_DETAIL}"
                ),
                format!(
                    "kind=direct-dependency package=zeta-app path=crates/zeta-app/Cargo.toml declaration=petgraph detail={DIRECT_DEPENDENCY_DETAIL}"
                ),
                format!(
                    "kind=public-graph-boundary package=gandr-theory-graphs path=crates/gandr-theory-graphs/src/lib.rs declaration=pub type Zed: petgraph detail={PUBLIC_API_DETAIL}"
                ),
                format!(
                    "kind=source-declaration package=alpha-app path=crates/alpha-app/src/lib.rs declaration=graph-stack path: fixedbitset detail={OUTSIDE_SOURCE_DETAIL}"
                ),
                format!(
                    "kind=source-declaration package=zeta-app path=crates/zeta-app/src/lib.rs declaration=graph-stack path: petgraph detail={OUTSIDE_SOURCE_DETAIL}"
                ),
            ],
            first,
            "finding order should follow the stable emitted fields"
        );
        Ok(())
    }

    /// Render findings into stable strings for ordering assertions.
    fn finding_lines(findings: &[Finding]) -> Vec<String>
    {
        findings
            .iter()
            .map(|finding| {
                format!(
                    "kind={} package={} path={} declaration={} detail={}",
                    finding.kind,
                    finding.package,
                    finding.path,
                    finding.declaration,
                    finding.detail
                )
            })
            .collect()
    }

    /// Run the graph-boundary gate and convert errors into test failures.
    fn run_ok(fixture: &Fixture) -> TestResult<Vec<Finding>>
    {
        match run(&fixture.root, Some(&fixture.metadata_path)) {
            | Ok(findings) => Ok(findings),
            | Err(error) => Err(Box::new(std::io::Error::other(format!(
                "graph-boundary failed: {error:?}"
            )))),
        }
    }

    /// Build a unique temporary workspace root for one fixture.
    fn unique_root() -> PathBuf
    {
        let suffix = NEXT_TEMP_ROOT.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "gandr-workflow-gates-graph-boundary-{}-{suffix}",
            std::process::id()
        ))
    }

    /// Assert exact rendered findings in emitted order.
    fn assert_finding_lines(
        findings: &[Finding],
        expected: &[String],
    )
    {
        assert_eq!(finding_lines(findings).as_slice(), expected);
    }

    /// Render minimal Cargo metadata JSON for the fixture packages.
    fn metadata_json(
        root: &Path,
        packages: &[PackageFixture],
    ) -> String
    {
        let package_rows = packages
            .iter()
            .map(|package| package_json(root, package))
            .collect::<Vec<_>>()
            .join(",");
        let member_rows = packages
            .iter()
            .map(|package| format!("\"{}\"", json_escape(package.name)))
            .collect::<Vec<_>>()
            .join(",");

        format!("{{\"packages\":[{package_rows}],\"workspace_members\":[{member_rows}]}}")
    }

    /// Render one package object for the fixture metadata JSON.
    fn package_json(
        root: &Path,
        package: &PackageFixture,
    ) -> String
    {
        let manifest_path = root
            .join("crates")
            .join(package.name)
            .join("Cargo.toml")
            .to_string_lossy()
            .into_owned();
        let dependencies = package
            .dependencies
            .iter()
            .map(|&dependency| format!("{{\"name\":\"{}\"}}", json_escape(dependency)))
            .collect::<Vec<_>>()
            .join(",");

        format!(
            "{{\"id\":\"{}\",\"name\":\"{}\",\"manifest_path\":\"{}\",\"dependencies\":[{}]}}",
            json_escape(package.name),
            json_escape(package.name),
            json_escape(&manifest_path),
            dependencies
        )
    }

    /// Escape a string for the limited fixture JSON renderer.
    fn json_escape<'semantic, Value>(value: Value) -> String
    where
        Value: Into<ValueText<'semantic>>,
    {
        let value = value.into().0;
        value.replace('\\', "\\\\").replace('"', "\\\"")
    }
}
